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
        app: knx_server::app(state, None),
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
async fn a_button_pressed_while_waiting_gets_the_address() {
    let h = harness(SimulatorConfig::default());
    let original = h.device.address();
    let (status, body) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::OK, "{body}");

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
    assert_eq!(h.device.address().to_string(), NEW);

    // Only changes are logged: "nobody" once, then the device, then the find.
    let kinds: Vec<&str> = events.iter().map(|e| e["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["round", "round", "found"], "{events:?}");
    assert_eq!(events[0]["inProgrammingMode"], json!([]));
    assert_eq!(
        events[1]["inProgrammingMode"],
        json!([original.to_string()])
    );
    assert_eq!(events[2]["currentAddress"], original.to_string());
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn nobody_pressing_gives_up_and_writes_nothing() {
    let h = harness(SimulatorConfig::default());
    let original = h.device.address();
    let (status, _) = start(&h, request(PHRASE, 1)).await;
    assert_eq!(status, StatusCode::OK);
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
    let (_, body) = start(&h, request(PHRASE, 30)).await;
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
    let (status, _) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::OK);
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

#[tokio::test]
async fn written_but_silent_at_the_new_address_is_unconfirmed() {
    // Step 4's connects to the new address go unanswered.
    let h = harness(SimulatorConfig {
        programming_mode: true,
        unanswered_connects: Some(1..3),
        ..SimulatorConfig::default()
    });
    let (status, _) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::OK);
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
async fn the_gateway_serves_one_tunnel_in_both_directions() {
    // A monitor runs: no programming starts.
    let h = harness(SimulatorConfig::default());
    let (status, body) = send(
        &h.app,
        post("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("monitor"), "{body}");
    assert_eq!(h.calls.load(Ordering::SeqCst), 1, "only the monitor's");

    // A programming waits: no monitor, scan or second programming starts.
    let h = harness(SimulatorConfig::default());
    let (status, body) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
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
    let (status, _) = start(&h, request(PHRASE, 30)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(h.calls.load(Ordering::SeqCst), 1, "only the programming's");

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
