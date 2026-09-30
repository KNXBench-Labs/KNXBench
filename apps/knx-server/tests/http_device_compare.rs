//! `/api/device-compare`: what a download would change on a device, read only, over HTTP.
//!
//! The same simulated gateway as `http_device_download.rs`: the production
//! route and read path, with the bus replaced by a simulated mask-`0701h`
//! device at `1.1.67`. No socket anywhere. The route may read and must never
//! write: every test checks the device saw no write of any kind.
//!
//! The corpus-free tests pin the refusals that happen before any tunnel.
//! The others are `#[ignore]`d: they need the private product corpus and the
//! saved K3 project (`KNXBENCH_PRODUCT_CORPUS`, `KNXBENCH_K3_PROJECT`, or the
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
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID};
use knx_core::IndividualAddress;
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
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
        panic!("a compare sends no group telegram")
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

fn empty_app() -> (axum::Router, Arc<AtomicUsize>) {
    let calls = Arc::new(AtomicUsize::new(0));
    // The product database is set explicitly, never picked up from the
    // machine (as in `http_device_readiness.rs`).
    let state = Arc::new(knx_server::AppState {
        product_db: None,
        connector: Box::new(SimConnector {
            device: mdt(SimulatorConfig::default()),
            calls: Arc::clone(&calls),
        }),
        ..Default::default()
    });
    (knx_server::app(state, None), calls)
}

async fn compare(app: &axum::Router, body: Value) -> (StatusCode, Value) {
    send(app, post("/api/device-compare", body)).await
}

fn no_writes(device: &SimulatedDevice) -> bool {
    !device.seen().into_iter().any(|seen| {
        matches!(
            seen,
            Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
        )
    })
}

#[tokio::test]
async fn refusals_before_any_tunnel_need_no_corpus() {
    let (app, calls) = empty_app();
    let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
    let (status, body) = compare(&app, json!({ "address": excluded, "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, body) = compare(&app, json!({ "address": "1.1.67", "gateway": "nowhere" })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, body) = compare(&app, json!({ "address": "1.1.67", "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("no product database"), "{body}");
    assert_eq!(calls.load(Ordering::SeqCst), 0, "no tunnel was asked for");

    // No phrase, no key: a compare has no field that could write.
    let response = app
        .clone()
        .oneshot(get("/api/device-compare"))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn a_fresh_device_differs_and_nothing_is_written() {
    let h = harness(SimulatorConfig::default()).await;
    let (status, body) = compare(&h.app, json!({ "address": "1.1.67", "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["address"], "1.1.67");
    assert_eq!(body["written"], false);
    assert_eq!(body["same"], false);
    assert_eq!(body["mask"], 0x0701);
    assert_eq!(body["manufacturer"], 0x0083);
    assert_eq!(body["partial"], false);
    let changes = body["changes"].as_array().unwrap();
    assert!(!changes.is_empty());
    let differing: usize = changes
        .iter()
        .map(|c| c["project"].as_array().unwrap().len())
        .sum();
    assert_eq!(body["differingOctets"], differing);
    for change in changes {
        assert_eq!(
            change["device"].as_array().unwrap().len(),
            change["project"].as_array().unwrap().len(),
            "{change}"
        );
        assert!(change["segment"].is_string(), "{change}");
        // `device` is what the device holds; `project` differs from it at
        // both ends of every run (a run is maximal).
        let octets = |key: &str| -> Vec<u8> {
            change[key]
                .as_array()
                .unwrap()
                .iter()
                .map(|o| o.as_u64().unwrap() as u8)
                .collect()
        };
        let (device, project) = (octets("device"), octets("project"));
        let address = change["address"].as_u64().unwrap() as u32;
        // The simulator reads memory nobody preset as `00h`.
        let held: Vec<u8> = h
            .device
            .memory(address, device.len())
            .into_iter()
            .map(|octet| octet.unwrap_or(0))
            .collect();
        assert_eq!(held, device, "{change}");
        assert_ne!(device.first(), project.first(), "{change}");
        assert_ne!(device.last(), project.last(), "{change}");
    }
    assert!(body["loadStates"].as_array().is_some_and(|s| !s.is_empty()));
    assert!(no_writes(&h.device));
    assert!(!h.device.memory_was_written());
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);
    assert_eq!(h.device.seen().last(), Some(&Seen::Disconnect));
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn after_the_download_the_device_compares_the_same() {
    let h = harness(SimulatorConfig::default()).await;
    let (status, plan) = send(
        &h.app,
        post("/api/device-download/plan", json!({ "address": "1.1.67" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    let (status, body) = send(
        &h.app,
        post(
            "/api/device-download/start",
            json!({
                "planId": plan["planId"],
                "gateway": GATEWAY,
                "confirmation": "I confirm download to 1.1.67"
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    for _ in 0..2000 {
        let (status, body) = send(&h.app, get("/api/device-download/status")).await;
        if status == StatusCode::OK && body["status"]["state"] != "running" {
            assert_eq!(body["status"]["state"], "finished", "{body}");
            break;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    let writes = h
        .device
        .seen()
        .into_iter()
        .filter(|seen| matches!(seen, Seen::MemoryWrite { .. }))
        .count();

    let (status, body) = compare(&h.app, json!({ "address": "1.1.67", "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["same"], true, "{body}");
    assert_eq!(body["differingOctets"], 0);
    assert_eq!(body["changes"], json!([]));
    assert_eq!(body["octets"], plan["dataOctets"]);
    let after = h
        .device
        .seen()
        .into_iter()
        .filter(|seen| matches!(seen, Seen::MemoryWrite { .. }))
        .count();
    assert_eq!(after, writes, "the compare wrote nothing");
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn the_gateway_serves_one_tunnel_so_a_running_monitor_blocks_the_compare() {
    let h = harness(SimulatorConfig::default()).await;
    let (status, body) = send(
        &h.app,
        post("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = compare(&h.app, json!({ "address": "1.1.67", "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("monitor"), "{body}");
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        1,
        "only the monitor's tunnel"
    );
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn another_device_type_is_not_compared_and_says_nothing_was_written() {
    let other = Arc::new(SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0705,
            ..SimulatorConfig::default()
        },
    ));
    let h = harness_with_device(Arc::clone(&other)).await;
    let (status, body) = compare(&h.app, json!({ "address": "1.1.67", "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(body.to_string().contains("nothing was written"), "{body}");
    assert!(no_writes(&other));
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn a_partial_compare_reads_only_what_the_partial_download_writes() {
    let h = harness(SimulatorConfig::default()).await;
    let (status, whole) = compare(&h.app, json!({ "address": "1.1.67", "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::OK, "{whole}");
    let (status, part) = compare(
        &h.app,
        json!({
            "address": "1.1.67",
            "gateway": GATEWAY,
            "partial": { "parameters": true, "groupAddresses": false }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{part}");
    assert_eq!(part["partial"], true);
    assert!(
        part["octets"].as_u64().unwrap() < whole["octets"].as_u64().unwrap(),
        "{part}"
    );
    assert!(no_writes(&h.device));
}

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ corpus (product file and the saved K3 project); run with --ignored"]
async fn a_running_download_blocks_the_compare() {
    // A silent device and a two-second connect time-out keep the download
    // running while the compare asks.
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
    let (status, plan) = send(
        &h.app,
        post("/api/device-download/plan", json!({ "address": "1.1.67" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    let (status, body) = send(
        &h.app,
        post(
            "/api/device-download/start",
            json!({
                "planId": plan["planId"],
                "gateway": GATEWAY,
                "confirmation": "I confirm download to 1.1.67"
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = compare(&h.app, json!({ "address": "1.1.67", "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert!(body.to_string().contains("download"), "{body}");
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        1,
        "only the download's tunnel"
    );
}
