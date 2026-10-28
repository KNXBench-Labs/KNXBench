//! K6: programming an individual address through the web API, on the button loop, in the simulator.
//!
//! The gateway is a [`SimTunnel`]: the production route, session, loop and
//! MP §2.3 procedure, with the bus replaced by one simulated device whose
//! programming button the test presses and releases. No socket anywhere.
//! Refusals are checked to happen before the gateway is asked for a
//! tunnel: the connector counts its calls.

use std::future::Future;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::IndividualAddress;
use knx_net::commissioning::programming_button_wait::{
    AddressProgrammingAuthorisation, ButtonWait,
};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::{
    ApplicationService, BusError, Destination, DiscoveredGateway, ManagementTransport,
    SessionTiming, Tpci, TunnelEvent,
};
use knx_server::{BusSessionError, BusTunnel, GatewayConnector};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

const GATEWAY: &str = "192.0.2.10:3671";
const NEW: &str = "1.1.30";
const PHRASE: &str = "I confirm individual-address programming to 1.1.30";

struct SimTunnel(Arc<SimulatedDevice>);

impl BusTunnel for SimTunnel {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.0.as_ref())
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.0.as_ref())
    }

    fn send(
        &self,
        _destination: Destination,
        _service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        panic!("address programming sends no group telegram")
    }

    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move {
            ManagementTransport::send_frame(self.0.as_ref(), destination, transport, service).await
        })
    }

    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}

struct SimConnector {
    device: Arc<SimulatedDevice>,
    calls: Arc<AtomicUsize>,
}

impl GatewayConnector for SimConnector {
    fn connect_tunnel(
        &self,
        _gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let tunnel: Box<dyn BusTunnel> = Box::new(SimTunnel(Arc::clone(&self.device)));
        Box::pin(async move { Ok(tunnel) })
    }

    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}

fn fast() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(50),
        response_timeout: Duration::from_millis(50),
        answer_timeout: None,
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(40),
        programming_delay: Duration::from_millis(0),
        restart_basic_t1: Duration::from_millis(1),
        restart_responsive_again: Duration::from_millis(5),
        post_restart_disconnect_wait: Duration::from_millis(60),
        programming_mode_broadcast_timeout: Duration::from_millis(20),
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    app: axum::Router,
    state: Arc<knx_server::AppState>,
    device: Arc<SimulatedDevice>,
    calls: Arc<AtomicUsize>,
}

fn harness(config: SimulatorConfig) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let device = Arc::new(SimulatedDevice::with_config(config));
    let calls = Arc::new(AtomicUsize::new(0));
    let state = Arc::new(knx_server::AppState {
        data_dir: dir.path().to_path_buf(),
        connector: Box::new(SimConnector {
            device: Arc::clone(&device),
            calls: Arc::clone(&calls),
        }),
        address_programming_timing: fast(),
        address_programming_pause: Duration::from_millis(10),
        ..Default::default()
    });
    Harness {
        _dir: dir,
        app: knx_server::app(Arc::clone(&state), None),
        state,
        device,
        calls,
    }
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, body)
}

async fn start(h: &Harness, body: Value) -> (StatusCode, Value) {
    send(&h.app, post("/api/device-address/start", body)).await
}

/// Exercise the background session and HTTP status/stop endpoints in the
/// simulator without bypassing the production start route's recovery gate.
async fn start_simulated(h: &Harness, wait_seconds: u64) -> Value {
    let address: IndividualAddress = NEW.parse().unwrap();
    let authorisation = AddressProgrammingAuthorisation::for_hardware(address, PHRASE).unwrap();
    let id = h
        .state
        .next_address_programming_id
        .fetch_add(1, Ordering::SeqCst);
    let session = knx_server::AddressProgrammingSession::start(
        id,
        Box::new(SimTunnel(Arc::clone(&h.device))),
        address,
        authorisation,
        fast(),
        ButtonWait {
            give_up_after: Duration::from_secs(wait_seconds),
            pause_between_rounds: Duration::from_millis(10),
        },
        knx_server::address_programming_settler(&h.state, address),
    );
    assert!(h
        .state
        .address_programming
        .lock()
        .await
        .replace(session)
        .is_none());
    json!({ "programmingId": id })
}

fn request(confirmation: &str, wait_seconds: u64) -> Value {
    json!({
        "address": NEW,
        "gateway": GATEWAY,
        "confirmation": confirmation,
        "waitSeconds": wait_seconds,
    })
}

/// Polls until the state is not `waiting`/`programming`, calling `during`
/// with every status seen on the way. Collects each event once.
async fn finish(h: &Harness, mut during: impl FnMut(&Value)) -> (Value, Vec<Value>) {
    let mut since = 0;
    let mut events = Vec::new();
    for _ in 0..4000 {
        let (status, body) = send(
            &h.app,
            get(&format!("/api/device-address/status?since={since}")),
        )
        .await;
        if status == StatusCode::CONFLICT {
            tokio::time::sleep(Duration::from_millis(2)).await;
            continue;
        }
        assert_eq!(status, StatusCode::OK, "{body}");
        events.extend(body["events"].as_array().unwrap().iter().cloned());
        since = body["nextSince"].as_u64().unwrap() as usize;
        let state = body["status"]["state"].as_str().unwrap().to_string();
        if state != "waiting" && state != "programming" {
            return (body["status"].clone(), events);
        }
        during(&body["status"]);
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    panic!("the programming did not end");
}

#[tokio::test]
async fn the_phrase_route_names_the_address_and_sends_nothing() {
    let h = harness(SimulatorConfig::default());
    let (status, body) = send(&h.app, get("/api/device-address/phrase?address=1.1.30")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["confirmationPhrase"], PHRASE);
    assert_eq!(body["defaultWaitSeconds"], 120);
    assert_eq!(body["maxWaitSeconds"], 600);
    // An excluded address has no phrase.
    let (status, _) = send(&h.app, get("/api/device-address/phrase?address=1.1.220")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn availability_reports_the_same_recovery_block_as_confirmed_start_without_a_tunnel() {
    let h = harness(SimulatorConfig {
        programming_mode: true,
        ..SimulatorConfig::default()
    });
    let previous = h.device.address();
    let (status, availability) = send(&h.app, get("/api/device-address/availability")).await;
    assert_eq!(status, StatusCode::OK, "{availability}");
    assert_eq!(availability["startAvailable"], false);
    assert!(availability["reason"]
        .as_str()
        .is_some_and(|reason| reason.contains("no verified durable pre-write backup")));

    let (status, refused) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED, "{refused}");
    assert!(refused
        .to_string()
        .contains(availability["reason"].as_str().unwrap()));
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    assert_eq!(h.device.address(), previous);
}

#[tokio::test]
async fn confirmed_start_needs_durable_recovery_before_any_tunnel() {
    let h = harness(SimulatorConfig {
        programming_mode: true,
        ..SimulatorConfig::default()
    });
    let previous = h.device.address();
    let (status, body) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED, "{body}");
    assert!(body
        .to_string()
        .contains("no verified durable pre-write backup"));
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    assert_eq!(h.device.address(), previous);
}

#[tokio::test]
async fn a_button_pressed_while_waiting_gets_the_address() {
    let h = harness(SimulatorConfig::default());
    let original = h.device.address();
    let body = start_simulated(&h, 30).await;
    let id = body["programmingId"].as_u64().unwrap();
    let (status, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(status, StatusCode::OK, "{activity}");
    assert_eq!(activity["coverage"], "partial");
    assert_eq!(activity["sessions"][0]["kind"], "addressProgramming");
    assert_eq!(activity["sessions"][0]["id"], id);
    assert_eq!(activity["sessions"][0]["address"], NEW);
    assert_eq!(activity["sessions"][0]["state"], "waiting");
    assert!(activity["sessions"][0]["rounds"].is_u64());

    // The operator presses the button once the UI has shown two empty
    // rounds.
    let device = Arc::clone(&h.device);
    let (end, events) = finish(&h, |status| {
        if status["state"] == "waiting" && status["rounds"].as_u64().unwrap() >= 2 {
            device.set_programming_mode(true);
        }
    })
    .await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(end["written"], "yes");
    assert_eq!(end["previousAddress"], original.to_string());
    assert_eq!(end["wasFree"], true);
    assert_eq!(end["restartConfirmed"], true, "{end}");
    assert_eq!(h.device.address().to_string(), NEW);
    let (status, activity) = send(&h.app, get("/api/bus/activity")).await;
    assert_eq!(status, StatusCode::OK, "{activity}");
    assert_eq!(activity["sessions"][0]["id"], id);
    assert_eq!(activity["sessions"][0]["state"], "finished");
    assert!(activity["sessions"][0].get("rounds").is_none());

    // Only changes are logged: "nobody" once, then the device, then the find.
    let kinds: Vec<&str> = events.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["round", "round", "found"], "{events:?}");
    assert_eq!(events[0]["inProgrammingMode"], json!([]));
    assert_eq!(
        events[1]["inProgrammingMode"],
        json!([original.to_string()])
    );
    assert_eq!(events[2]["currentAddress"], original.to_string());
    assert_eq!(h.calls.load(Ordering::SeqCst), 0, "no public start tunnel");
}

/// A project with one device per octet in `octets` on line 1.1, none of
/// them with its address programmed.
fn project_with_devices(octets: &[u8]) -> knx_core::Project {
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    for (index, octet) in octets.iter().enumerate() {
        let id = index as u32 + 1;
        project.devices.insert(knx_core::DeviceInstance {
            id: knx_core::DeviceId(id),
            source: knx_core::SourceRef {
                path: "t".into(),
                ets_id: format!("d{id}"),
            },
            name: format!("Device {id}"),
            description: None,
            address: Some(format!("1.1.{octet}").parse().unwrap()),
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: knx_core::CommissioningState {
                application_program_loaded: true,
                parameters_loaded: true,
                communication_part_loaded: true,
                ..knx_core::CommissioningState::default()
            },
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    project
}

fn loaded_addresses(h: &Harness) -> Vec<bool> {
    let project = h.state.project.lock().unwrap();
    project
        .as_ref()
        .unwrap()
        .devices
        .iter()
        .map(|device| device.commissioning.individual_address_loaded)
        .collect()
}

async fn press_and_finish(h: &Harness) -> Value {
    start_simulated(h, 30).await;
    let device = Arc::clone(&h.device);
    let (end, _) = finish(h, |status| {
        if status["state"] == "waiting" {
            device.set_programming_mode(true);
        }
    })
    .await;
    end
}

#[test]
fn address_settlement_does_not_follow_an_address_to_a_different_project_device() {
    for octets in [&[29, 30][..], &[30, 30][..]] {
        let h = harness(SimulatorConfig::default());
        *h.state.project.lock().unwrap() = Some(project_with_devices(octets));
        let settle = knx_server::address_programming_settler(&h.state, NEW.parse().unwrap());
        {
            let mut project = h.state.project.lock().unwrap();
            let project = project.as_mut().unwrap();
            project
                .devices
                .get_mut(knx_core::DeviceId(1))
                .unwrap()
                .address = Some("1.1.30".parse().unwrap());
            project
                .devices
                .get_mut(knx_core::DeviceId(2))
                .unwrap()
                .address = Some("1.1.29".parse().unwrap());
        }
        settle(&knx_server::AddressProgrammingStatus::Finished {
            written: knx_server::AddressWritten::Yes,
            previous_address: "15.15.255".into(),
            was_free: true,
            restart_confirmed: true,
        });
        assert_eq!(
            loaded_addresses(&h),
            [false, false],
            "neither a replacement nor a previously ambiguous device is confirmed"
        );
    }
}

/// ADR-0129: programming the individual address clears the Address part of
/// the one project device that has it, outside the undo history, and says
/// so in the log. Nothing else of the device changes.
#[tokio::test]
async fn a_programmed_address_is_no_longer_pending_on_its_project_device() {
    let h = harness(SimulatorConfig::default());
    *h.state.project.lock().unwrap() = Some(project_with_devices(&[29, 30]));
    let revision = h.state.project_revision.load(Ordering::SeqCst);
    let end = press_and_finish(&h).await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(
        loaded_addresses(&h),
        [false, true],
        "only the device at 1.1.30"
    );
    let project = h.state.project.lock().unwrap().clone().unwrap();
    let device = project.devices.get(knx_core::DeviceId(2)).unwrap();
    assert!(device.commissioning.parameters_loaded);
    assert_eq!(device.commissioning.last_download, None);
    assert_eq!(
        h.state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert!(h.state.project_revision.load(Ordering::SeqCst) > revision);
    let (_, log) = send(&h.app, get("/api/log")).await;
    assert!(
        log.to_string()
            .contains("Individual address 1.1.30 programmed into “Device 2” (1.1.30): address marked programmed"),
        "{log}"
    );
}

/// S2-04 (issue #30): the settler can save the native project while the
/// tunnel is still open. One runtime worker, as in a one-CPU container: a
/// synchronous stall before the real settler stands in for a slow fsync, a
/// spawned ticker for the tunnel's receive task, which must keep running.
#[tokio::test(flavor = "multi_thread", worker_threads = 1)]
async fn a_blocking_address_settler_leaves_tunnel_tasks_running() {
    const STALL: Duration = Duration::from_millis(300);
    let h = harness(SimulatorConfig {
        programming_mode: true,
        ..SimulatorConfig::default()
    });
    *h.state.project.lock().unwrap() = Some(project_with_devices(&[29, 30]));
    let ticks = Arc::new(AtomicUsize::new(0));
    let ticker = tokio::spawn({
        let ticks = Arc::clone(&ticks);
        async move {
            loop {
                tokio::time::sleep(Duration::from_millis(5)).await;
                ticks.fetch_add(1, Ordering::SeqCst);
            }
        }
    });
    while ticks.load(Ordering::SeqCst) == 0 {
        tokio::time::sleep(Duration::from_millis(1)).await;
    }
    let address: IndividualAddress = NEW.parse().unwrap();
    let window = Arc::new(std::sync::Mutex::new(None));
    let settle = knx_server::address_programming_settler(&h.state, address);
    let on_end: knx_server::OnProgrammingEnd = Box::new({
        let (ticks, window) = (Arc::clone(&ticks), Arc::clone(&window));
        move |status| {
            let before = ticks.load(Ordering::SeqCst);
            std::thread::sleep(STALL);
            *window.lock().unwrap() = Some((before, ticks.load(Ordering::SeqCst)));
            settle(status);
        }
    });
    let mut session = knx_server::AddressProgrammingSession::start(
        1,
        Box::new(SimTunnel(Arc::clone(&h.device))),
        address,
        AddressProgrammingAuthorisation::for_hardware(address, PHRASE).unwrap(),
        fast(),
        ButtonWait {
            give_up_after: Duration::from_secs(30),
            pause_between_rounds: Duration::from_millis(10),
        },
        on_end,
    );
    session.join().await;
    ticker.abort();
    assert!(
        matches!(
            session.status(),
            knx_server::AddressProgrammingStatus::Finished {
                written: knx_server::AddressWritten::Yes,
                ..
            }
        ),
        "{:?}",
        session.status()
    );
    let (before, after) = window.lock().unwrap().expect("the settler ran");
    // About 60 nominal; inline on the only worker it is at most one.
    assert!(
        after - before >= 10,
        "only {} ticks while the settler blocked",
        after - before
    );
    assert_eq!(
        loaded_addresses(&h),
        [false, true],
        "the real settler still settles the device at 1.1.30"
    );
}

/// The project open when the programming started is the only one it may
/// settle: after another project was opened, nothing is applied.
#[tokio::test]
async fn a_project_opened_meanwhile_is_left_alone() {
    let h = harness(SimulatorConfig::default());
    *h.state.project.lock().unwrap() = Some(project_with_devices(&[30]));
    start_simulated(&h, 30).await;
    // What opening, creating or importing another project does.
    h.state.project_incarnation.fetch_add(1, Ordering::SeqCst);
    let device = Arc::clone(&h.device);
    let (end, _) = finish(&h, |status| {
        if status["state"] == "waiting" {
            device.set_programming_mode(true);
        }
    })
    .await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(loaded_addresses(&h), [false]);
    let (_, log) = send(&h.app, get("/api/log")).await;
    assert!(log.to_string().contains("another project is open"), "{log}");
}

#[tokio::test]
async fn an_address_two_project_devices_share_changes_neither() {
    let h = harness(SimulatorConfig::default());
    *h.state.project.lock().unwrap() = Some(project_with_devices(&[30, 30]));
    let end = press_and_finish(&h).await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(loaded_addresses(&h), [false, false]);
    let (_, log) = send(&h.app, get("/api/log")).await;
    assert!(
        log.to_string()
            .contains("no unchanged, uniquely identified project device"),
        "{log}"
    );
}

#[tokio::test]
async fn nobody_pressing_gives_up_and_writes_nothing() {
    let h = harness(SimulatorConfig::default());
    let original = h.device.address();
    start_simulated(&h, 1).await;
    let (end, _) = finish(&h, |_| {}).await;
    assert_eq!(end["state"], "failed", "{end}");
    assert_eq!(end["written"], "no");
    assert!(end["step"].is_null(), "the procedure never started: {end}");
    assert!(end["error"]
        .as_str()
        .unwrap()
        .contains("nothing was written"));
    assert_eq!(h.device.address(), original);
}

#[tokio::test]
async fn stop_ends_the_wait_and_is_refused_once_the_procedure_runs() {
    let h = harness(SimulatorConfig::default());
    let original = h.device.address();
    let body = start_simulated(&h, 30).await;
    let id = body["programmingId"].clone();
    // Let it wait a little, then stop.
    tokio::time::sleep(Duration::from_millis(80)).await;
    let (status, body) = send(
        &h.app,
        post("/api/device-address/stop", json!({ "programmingId": id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (end, _) = finish(&h, |_| {}).await;
    assert_eq!(end["state"], "stopped", "{end}");
    assert_eq!(h.device.address(), original);

    // After the end there is nothing left to stop.
    let (status, _) = send(
        &h.app,
        post("/api/device-address/stop", json!({ "programmingId": id })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    // An unknown id is not this programming.
    let (status, _) = send(
        &h.app,
        post("/api/device-address/stop", json!({ "programmingId": 999 })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn several_buttons_are_asked_to_release_all_but_one() {
    let other: IndividualAddress = "1.1.40".parse().unwrap();
    let h = harness(SimulatorConfig {
        programming_mode: true,
        other_programming_mode_devices: vec![other],
        ..SimulatorConfig::default()
    });
    let original = h.device.address();
    start_simulated(&h, 30).await;
    let device = Arc::clone(&h.device);
    let (end, events) = finish(&h, |status| {
        if status["state"] == "waiting"
            && status["inProgrammingMode"].as_array().unwrap().len() == 2
        {
            device.set_other_programming_mode_devices(Vec::new());
        }
    })
    .await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(
        events[0]["inProgrammingMode"],
        json!([original.to_string(), other.to_string()]),
        "{events:?}"
    );
    assert_eq!(h.device.address().to_string(), NEW);
}

/// The live case of 2026-09-29: the device answered at the new address,
/// only the closing Basic Restart went unacknowledged. Finished, with the
/// restart flagged, not "failed" with the address "unconfirmed".
#[tokio::test]
async fn an_unacknowledged_restart_finishes_with_the_restart_unconfirmed() {
    let h = harness(SimulatorConfig {
        programming_mode: true,
        restart_unanswered: true,
        ..SimulatorConfig::default()
    });
    start_simulated(&h, 30).await;
    let (end, _) = finish(&h, |_| {}).await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(end["written"], "yes");
    assert_eq!(end["restartConfirmed"], false, "{end}");
    assert_eq!(h.device.address().to_string(), NEW);
}

#[tokio::test]
async fn written_but_silent_at_the_new_address_is_unconfirmed() {
    // Step 4's connects to the new address go unanswered.
    let h = harness(SimulatorConfig {
        programming_mode: true,
        unanswered_connects: Some(1..3),
        ..SimulatorConfig::default()
    });
    start_simulated(&h, 30).await;
    let (end, _) = finish(&h, |_| {}).await;
    assert_eq!(end["state"], "failed", "{end}");
    assert_eq!(end["written"], "unconfirmed");
    assert_eq!(end["step"], 4);
    assert_eq!(h.device.address().to_string(), NEW, "the write did happen");
}

#[tokio::test]
async fn refusals_happen_before_any_tunnel_opens() {
    let h = harness(SimulatorConfig {
        programming_mode: true,
        ..SimulatorConfig::default()
    });
    let original = h.device.address();
    for (body, expected) in [
        // Another address's phrase.
        (
            request("I confirm individual-address programming to 1.1.31", 30),
            StatusCode::BAD_REQUEST,
        ),
        // The download phrase for the same address.
        (
            request("I confirm download to 1.1.30", 30),
            StatusCode::BAD_REQUEST,
        ),
        // The restart phrase alone.
        (
            request("I confirm restart to 1.1.30", 30),
            StatusCode::BAD_REQUEST,
        ),
        (request("", 30), StatusCode::BAD_REQUEST),
        // Waits out of bounds.
        (request(PHRASE, 0), StatusCode::BAD_REQUEST),
        (request(PHRASE, 601), StatusCode::BAD_REQUEST),
        // An excluded address, with its own phrase.
        (
            json!({
                "address": "1.1.220",
                "gateway": GATEWAY,
                "confirmation": "I confirm individual-address programming to 1.1.220",
            }),
            StatusCode::BAD_REQUEST,
        ),
        // Not a gateway.
        (
            json!({ "address": NEW, "gateway": "nowhere", "confirmation": PHRASE }),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let (status, reply) = start(&h, body.clone()).await;
        assert_eq!(status, expected, "{body} -> {reply}");
    }
    assert_eq!(h.calls.load(Ordering::SeqCst), 0, "no tunnel was asked for");
    assert_eq!(h.device.address(), original);
}

#[tokio::test]
async fn a_simulated_session_excludes_other_tunnels_while_public_start_refuses() {
    // A monitor runs: the public programming start still refuses before a
    // second tunnel (the missing recovery proof takes precedence).
    let h = harness(SimulatorConfig::default());
    let (status, body) = send(
        &h.app,
        post("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED, "{body}");
    assert!(
        body.to_string().contains("durable pre-write backup"),
        "{body}"
    );
    assert_eq!(h.calls.load(Ordering::SeqCst), 1, "only the monitor's");

    // A simulated programming waits: no monitor or scan starts.
    let h = harness(SimulatorConfig::default());
    let body = start_simulated(&h, 30).await;
    let id = body["programmingId"].clone();
    let (status, body) = send(
        &h.app,
        post("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("programming"), "{body}");
    let (status, body) = send(
        &h.app,
        post(
            "/api/bus/scan/start",
            json!({
                "gateway": GATEWAY,
                "area": 1,
                "line": 1,
                "firstDevice": 1,
                "lastDevice": 2,
                "responseTimeoutMs": 100,
                "interProbePauseMs": 0
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    let (status, body) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::PRECONDITION_FAILED, "{body}");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0, "simulated session only");

    let (status, _) = send(
        &h.app,
        post("/api/device-address/stop", json!({ "programmingId": id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (end, _) = finish(&h, |_| {}).await;
    assert_eq!(end["state"], "stopped", "{end}");
    // Over: the monitor may start again.
    let (status, body) = send(
        &h.app,
        post("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
