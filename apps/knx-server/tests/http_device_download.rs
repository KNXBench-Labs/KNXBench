//! K5: a download to a device, started and watched through the web API, landed in the simulator.
//!
//! The saved K3 project (`KNXBench 1.1.67 option C.knxdb`) is opened through
//! `/api/project/open`, planned through `/api/device-download/plan`,
//! started with the phrase the plan names and polled through
//! `/api/device-download/status`. The gateway is a [`SimTunnel`]: the
//! production route, session and executor, with the bus replaced by a
//! simulated mask-`0701h` device answering at `1.1.67`. No socket anywhere.
//!
//! Refusals (wrong phrase, stale plan, edited project, monitor running) are
//! checked to happen *before* the gateway is asked for a tunnel: the
//! connector counts its calls, and the simulator counts its writes.
//!
//! `#[ignore]`d: it needs the private product corpus and the saved K3
//! project (`KNXBENCH_PRODUCT_CORPUS`, `KNXBENCH_K3_PROJECT`, or the
//! gitignored `OriginalData/`).

use std::future::Future;
use std::net::SocketAddrV4;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::properties::{
    ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
};
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

const PACKAGE: &str = "MDT_KP_BE_01_Push_Button_V15a.knxprod";
const PROJECT: &str = "KNXBench 1.1.67 option C.knxdb";
const GATEWAY: &str = "192.0.2.10:3671";

fn original_data() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData")
}

/// The server's tunnel, backed by one simulated device.
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
        panic!("a download sends no group telegram")
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

/// Hands out the one simulated device as a tunnel and counts every ask.
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

/// A simulated mask-`0701h` MDT device at `1.1.67`, as the CLI's K4 tests
/// set one up.
fn mdt(config: SimulatorConfig) -> Arc<SimulatedDevice> {
    let device = SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0701,
            ..config
        },
    );
    device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x83]);
    device.preset_property(0, PID_HARDWARE_TYPE, &[0, 0, 0, 0, 0x01, 0x27]);
    device.preset_memory(0x4000, &[0x05]);
    device.preset_memory(0x4001, &[0x11, 0x43]);
    for machine in [1, 2, 3] {
        device.preset_load_state(ObjectIndex::new(machine), LoadState::Loaded);
    }
    Arc::new(device)
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
        post_restart_disconnect_wait: Duration::from_millis(5),
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

/// An app with the product file installed and the saved K3 project open,
/// the gateway answering with a simulated MDT device.
async fn harness(config: SimulatorConfig) -> Harness {
    harness_with_device(mdt(config)).await
}

async fn harness_with_device(device: Arc<SimulatedDevice>) -> Harness {
    harness_timed(device, fast()).await
}

async fn harness_timed(device: Arc<SimulatedDevice>, timing: SessionTiming) -> Harness {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| original_data().join("ProductDatabases"));
    let package = knx_testsupport::find_corpus_file(&root, PACKAGE)
        .unwrap_or_else(|| panic!("{PACKAGE} not under {}", root.display()));
    let project = std::env::var_os("KNXBENCH_K3_PROJECT")
        .map(PathBuf::from)
        .unwrap_or_else(|| original_data().join("DemoProjects").join(PROJECT));
    assert!(project.exists(), "{} not present", project.display());

    let dir = tempfile::tempdir().unwrap();
    // A copy: opening may migrate the file in place.
    std::fs::copy(&project, dir.path().join("k3.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    knx_productdb::install_package(&products, PACKAGE, &std::fs::read(package).unwrap()).unwrap();

    let calls = Arc::new(AtomicUsize::new(0));
    let state = Arc::new(knx_server::AppState {
        product_db: Some(Mutex::new(products)),
        data_dir: dir.path().to_path_buf(),
        connector: Box::new(SimConnector {
            device: Arc::clone(&device),
            calls: Arc::clone(&calls),
        }),
        device_download_timing: timing,
        ..Default::default()
    });
    let app = knx_server::app(Arc::clone(&state), None);
    let (status, body) = send(
        &app,
        post("/api/project/open", json!({ "path": "k3.knxdb" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    Harness {
        _dir: dir,
        app,
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

async fn plan(h: &Harness) -> Value {
    let (status, body) = send(
        &h.app,
        post("/api/device-download/plan", json!({ "address": "1.1.67" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

async fn start(h: &Harness, plan_id: &Value, confirmation: &str) -> (StatusCode, Value) {
    send(
        &h.app,
        post(
            "/api/device-download/start",
            json!({ "planId": plan_id, "gateway": GATEWAY, "confirmation": confirmation }),
        ),
    )
    .await
}

/// Polls until the run ends, collecting every event exactly once.
async fn finish(h: &Harness) -> (Value, Vec<Value>) {
    let mut since = 0;
    let mut events = Vec::new();
    for _ in 0..2000 {
        let (status, body) = send(
            &h.app,
            get(&format!("/api/device-download/status?since={since}")),
        )
        .await;
        if status == StatusCode::CONFLICT {
            tokio::time::sleep(Duration::from_millis(5)).await;
            continue;
        }
        assert_eq!(status, StatusCode::OK, "{body}");
        events.extend(body["events"].as_array().unwrap().iter().cloned());
        since = body["nextSince"].as_u64().unwrap() as usize;
        if body["status"]["state"] != "running" {
            return (body["status"].clone(), events);
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the download did not end");
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn the_ui_route_writes_the_saved_project_and_shows_every_block() {
    let h = harness(SimulatorConfig::default()).await;
    let plan = plan(&h).await;
    assert_eq!(plan["address"], "1.1.67");
    assert_eq!(plan["dataOctets"], 1416);
    assert_eq!(plan["steps"].as_array().unwrap().len(), 25);
    assert_eq!(plan["confirmationPhrase"], "I confirm download to 1.1.67");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0, "a plan opens no tunnel");
    assert!(!h.device.memory_was_written());

    let (status, body) = start(&h, &plan["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (end, events) = finish(&h).await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(end["written"], "yes");
    assert_eq!(end["restart"], "acknowledged");

    let started: Vec<u64> = events
        .iter()
        .filter(|e| e["kind"] == "stepStarted")
        .map(|e| e["number"].as_u64().unwrap())
        .collect();
    assert_eq!(
        started,
        (1..=25).collect::<Vec<_>>(),
        "every step, once, in order"
    );
    let blocks: Vec<&Value> = events
        .iter()
        .filter(|e| e["kind"] == "dataWritten")
        .collect();
    assert!(!blocks.is_empty());
    let shown: usize = blocks
        .iter()
        .map(|b| b["octets"].as_array().unwrap().len())
        .sum();
    assert_eq!(shown, 1416, "every written octet is shown once");
    assert_eq!(blocks.last().unwrap()["written"], 1416);
    assert_eq!(blocks.last().unwrap()["of"], 1416);
    assert!(
        blocks.iter().any(|b| b["address"] == 0x4003),
        "the block after the address is shown"
    );
    // Each block is shown with the octets that are now in the device.
    for block in &blocks {
        let address = block["address"].as_u64().unwrap() as u32;
        let octets: Vec<Option<u8>> = block["octets"]
            .as_array()
            .unwrap()
            .iter()
            .map(|o| Some(o.as_u64().unwrap() as u8))
            .collect();
        assert_eq!(
            h.device.memory(address, octets.len()),
            octets,
            "block {address:04X}h"
        );
    }
    assert_eq!(
        h.device.memory(0x4001, 2),
        vec![Some(0x11), Some(0x43)],
        "address kept"
    );
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);

    // A plan is written once; the same id is refused afterwards.
    let (status, _) = start(&h, &plan["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn an_unanswered_restart_is_written_but_unconfirmed() {
    let h = harness(SimulatorConfig {
        restart_unanswered: true,
        ..SimulatorConfig::default()
    })
    .await;
    let plan = plan(&h).await;
    let (status, body) = start(&h, &plan["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (end, _) = finish(&h).await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(end["written"], "yes");
    assert_eq!(end["restart"], "unconfirmed");
    assert!(
        end["restartNote"].as_str().is_some_and(|n| !n.is_empty()),
        "{end}"
    );
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn a_connection_lost_mid_download_says_partially_and_where() {
    let h = harness(SimulatorConfig {
        drop_connection_after: Some(12),
        ..SimulatorConfig::default()
    })
    .await;
    let plan = plan(&h).await;
    let (status, _) = start(&h, &plan["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::OK);
    let (end, _) = finish(&h).await;
    assert_eq!(end["state"], "failed", "{end}");
    assert_eq!(end["written"], "partially");
    assert!(end["stoppedInStep"].as_u64().is_some(), "{end}");
    assert!(end["error"].as_str().is_some_and(|e| !e.is_empty()));
}

/// A device locked above its free level. Without a key in the project the
/// route stops with a hint and sends no key; with the project's
/// `Installation/@BCUKey` the same route unlocks it through MP §3.5.2 and
/// the key appears in no response.
#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn a_locked_device_takes_the_projects_key_and_no_response_shows_it() {
    let locked = SimulatorConfig {
        free_access_level: 3,
        key: Some(0x0BAD_CAFE),
        key_level: 1,
        write_requires_level: 2,
        ..SimulatorConfig::default()
    };

    let h = harness(locked.clone()).await;
    let plan_body = plan(&h).await;
    assert_eq!(
        plan_body["accessKey"], "none (the device's free access level)",
        "{plan_body}"
    );
    let (status, _) = start(&h, &plan_body["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::OK);
    let (end, events) = finish(&h).await;
    assert_eq!(end["state"], "failed", "{end}");
    assert!(
        end["hint"]
            .as_str()
            .is_some_and(|hint| hint.contains("never guess")),
        "{end}"
    );
    assert!(events
        .iter()
        .any(|e| e["kind"] == "authorised" && e["level"].is_null()));
    assert_eq!(h.device.authorize_requests(), 0, "nothing was guessed");

    let h = harness(locked).await;
    h.state
        .opaque
        .lock()
        .unwrap()
        .push(knx_store::StoredOpaqueEntry {
            source_path: "P-0001/0.xml".into(),
            xpath: "/KNX/Project/Installations/Installation".into(),
            kind: "RetainedAttribute".into(),
            name: "BCUKey".into(),
            bytes: b"195939070".to_vec(),
            sha256: String::new(),
        });
    let plan_body = plan(&h).await;
    assert_eq!(
        plan_body["accessKey"],
        "from the project (Installation/@BCUKey)"
    );
    let (status, _) = start(&h, &plan_body["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::OK);
    let (end, events) = finish(&h).await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(h.device.authorize_requests(), 2, "free key, then the key");
    assert!(events
        .iter()
        .any(|e| e["kind"] == "authorised" && e["level"] == 1));
    let everything = format!("{plan_body}{end}{}", Value::Array(events));
    for shown in ["195939070", "0BADCAFE", "0badcafe"] {
        assert!(!everything.contains(shown), "the key leaked");
    }
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn another_device_type_stops_before_the_first_write() {
    // Answers at 1.1.67, but with mask 0705h and no MDT identity.
    let h = harness_with_device(Arc::new(SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0705,
            ..SimulatorConfig::default()
        },
    )))
    .await;
    let plan = plan(&h).await;
    let (status, _) = start(&h, &plan["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::OK);
    let (end, _) = finish(&h).await;
    assert_eq!(end["state"], "failed", "{end}");
    assert_eq!(end["written"], "no");
    assert!(!h.device.memory_was_written());
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn refusals_happen_before_any_tunnel_opens() {
    let h = harness(SimulatorConfig::default()).await;
    let plan = plan(&h).await;

    // The phrase for another device.
    let (status, body) = start(&h, &plan["planId"], "I confirm download to 1.1.68").await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    // No phrase at all.
    let (status, _) = start(&h, &plan["planId"], "").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // A plan id that was never shown.
    let (status, _) = start(&h, &json!(9999), "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::CONFLICT);
    // A newer plan replaces the shown one.
    let newer = self::plan(&h).await;
    let (status, _) = start(&h, &plan["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::CONFLICT);

    // The project changes after the plan was shown: the linked object
    // gets a second, listening group address through the link route. The
    // configuration stays valid, and the plan it gives is a different one.
    let linked = {
        let project = h.state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let object = project
            .devices
            .com_objects()
            .find(|o| !o.links.is_empty())
            .expect("the saved project links one object");
        object.id.0
    };
    let (status, tree) = send(
        &h.app,
        post(
            "/api/group-addresses",
            json!({ "name": "K5 edited after the plan", "address": "2/0/54" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let ga = {
        let project = h.state.project.lock().unwrap();
        project.as_ref().unwrap().installations[0]
            .group_addresses
            .iter()
            .find(|g| g.name == "K5 edited after the plan")
            .expect("created")
            .id
            .0
    };
    let (status, body) = send(
        &h.app,
        post(
            "/api/group-links",
            json!({ "comObjectId": linked, "gaId": ga, "direction": "Receive" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = start(&h, &newer["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("changed"), "{body}");

    // An excluded address cannot even be planned.
    let (status, _) = send(
        &h.app,
        post("/api/device-download/plan", json!({ "address": "1.1.220" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    assert_eq!(h.calls.load(Ordering::SeqCst), 0, "no tunnel was asked for");
    assert!(!h.device.memory_was_written());
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn the_gateway_serves_one_tunnel_in_both_directions() {
    // A monitor is running: no download starts, and no tunnel is asked for.
    let h = harness(SimulatorConfig::default()).await;
    let (status, body) = send(
        &h.app,
        post("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let shown = self::plan(&h).await;
    let (status, body) = start(&h, &shown["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("monitor"), "{body}");
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        1,
        "only the monitor's tunnel"
    );
    assert!(!h.device.memory_was_written());

    // A download is running (a silent device, a two-second connect
    // time-out): neither a monitor nor a scan starts.
    let silent = Arc::new(SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            silent: true,
            ..SimulatorConfig::default()
        },
    ));
    let h = harness_timed(
        silent,
        SessionTiming {
            connection_timeout: Duration::from_secs(2),
            ..fast()
        },
    )
    .await;
    let shown = self::plan(&h).await;
    let (status, body) = start(&h, &shown["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(
        &h.app,
        post("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("download"), "{body}");
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
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        1,
        "only the download's tunnel"
    );
    let (end, _) = finish(&h).await;
    assert_eq!(end["state"], "failed", "{end}");
    assert_eq!(end["written"], "no");
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn a_waiting_address_programming_holds_off_the_download() {
    let h = harness(SimulatorConfig::default()).await;
    let shown = self::plan(&h).await;
    let (status, body) = send(
        &h.app,
        post(
            "/api/device-address/start",
            json!({
                "address": "1.1.30",
                "gateway": GATEWAY,
                "confirmation": "I confirm individual-address programming to 1.1.30",
                "waitSeconds": 30,
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = start(&h, &shown["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("programming"), "{body}");
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        1,
        "only the programming's tunnel"
    );
    assert!(!h.device.memory_was_written());
    let (status, _) = send(
        &h.app,
        post(
            "/api/device-address/stop",
            json!({ "programmingId": body_id(&h).await }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
}

async fn body_id(h: &Harness) -> Value {
    let (status, body) = send(&h.app, get("/api/device-address/status")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["programmingId"].clone()
}

/// K15 over HTTP: a partial plan is shown as one, the start runs exactly
/// the shown partial steps, and the application is never unloaded.
#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn a_partial_download_runs_the_partial_plan_that_was_shown() {
    let h = harness(SimulatorConfig::default()).await;
    h.device
        .preset_property(3, PID_PROGRAM_VERSION, &[0x00, 0x83, 0x00, 0x27, 0x15]);
    let (status, plan) = send(
        &h.app,
        post(
            "/api/device-download/plan",
            json!({
                "address": "1.1.67",
                "partial": { "parameters": true, "groupAddresses": false }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(plan["partial"], true);
    assert_eq!(plan["dataOctets"], 394);
    let steps: Vec<String> = plan["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|step| step.as_str().unwrap().to_string())
        .collect();
    assert_eq!(steps.len(), 11, "{steps:?}");
    assert!(
        steps.iter().all(|step| !step.contains("unload")),
        "{steps:?}"
    );
    assert!(
        steps.contains(&"compare property 3/13 with 00 83 00 27 15".to_string()),
        "{steps:?}"
    );

    let (status, body) = start(&h, &plan["planId"], "I confirm download to 1.1.67").await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (end, events) = finish(&h).await;
    assert_eq!(end["state"], "finished", "{end}");
    assert_eq!(end["written"], "yes");
    let started = events.iter().filter(|e| e["kind"] == "stepStarted").count();
    assert_eq!(started, 11, "the shown steps, no more");
    for machine in [1, 2, 3] {
        assert_eq!(
            h.device.load_state(ObjectIndex::new(machine)),
            LoadState::Loaded
        );
    }
}

/// Without a `partial` field the plan stays the complete one: the field is
/// an addition, not a change of the existing request.
#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn a_plan_without_partial_is_still_the_complete_download() {
    let h = harness(SimulatorConfig::default()).await;
    let plan = plan(&h).await;
    assert_eq!(plan["partial"], false);
    assert_eq!(plan["dataOctets"], 1416);
    assert_eq!(plan["notWritten"], json!([]));
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn a_partial_plan_with_nothing_selected_is_refused() {
    let h = harness(SimulatorConfig::default()).await;
    let (status, body) = send(
        &h.app,
        post(
            "/api/device-download/plan",
            json!({
                "address": "1.1.67",
                "partial": { "parameters": false, "groupAddresses": false }
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
}
