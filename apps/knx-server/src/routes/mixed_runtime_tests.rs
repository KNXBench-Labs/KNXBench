//! Mixed bus/project routes wait for the project off the runtime, bus work unchanged (#36).
//!
//! Each case holds the project lock on a plain thread at one distinct wait of a
//! route that also reserves the gateway or reads a device. On this
//! single-threaded runtime a wait taken inline would block the timer until the
//! holder gives up. Every case also fails when the request answers while the
//! lock is still held, so a refusal before the lock cannot pass as coverage,
//! and checks the exact answer after release. The bus is an in-memory
//! simulated device behind a fake connector; no socket is opened.

use std::future::Future;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::properties::{ObjectIndex, PID_MANUFACTURER_ID};
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::{
    ApplicationService, BusError, Destination, DiscoveredGateway, ManagementTransport,
    SessionTiming, Tpci, TunnelEvent,
};
use serde_json::{json, Value};
use tokio::sync::{broadcast, Notify};
use tower::ServiceExt;

use super::editor_runtime_tests::{project_with_device, request};
use crate::{AppState, BusSessionError, BusTunnel, GatewayConnector};

const GATEWAY: &str = "203.0.113.250:3671";
const HANG_GUARD: Duration = Duration::from_secs(10);

/// Where the test takes the project lock.
#[derive(Clone, Copy, Debug)]
enum Stage {
    /// Before the request starts: the first project wait.
    Immediately,
    /// While the request holds its gateway reservations (programming, monitor
    /// and scan); the test holds the scan lock until the project is held.
    UnderReservation,
    /// After the device read, once the tunnel was disconnected.
    AfterTheRead,
}

/// Lets a test hold the tunnel open and see when it is closed again.
#[derive(Clone, Default)]
struct Gate {
    entered: Arc<Notify>,
    open: Arc<Notify>,
    disconnected: Arc<Notify>,
}

struct SimTunnel {
    device: Arc<SimulatedDevice>,
    disconnected: Arc<Notify>,
}

impl BusTunnel for SimTunnel {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        ManagementTransport::assigned_address(self.device.as_ref())
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device.as_ref())
    }

    fn send(
        &self,
        _destination: Destination,
        _service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        panic!("these routes send no group telegram")
    }

    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move {
            ManagementTransport::send_frame(self.device.as_ref(), destination, transport, service)
                .await
        })
    }

    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        self.disconnected.notify_one();
        Box::pin(async { Ok(()) })
    }
}

/// Opens a tunnel to the simulated device once the test opens the gate.
struct GatedConnector {
    device: Arc<SimulatedDevice>,
    gate: Gate,
}

impl GatewayConnector for GatedConnector {
    fn connect_tunnel(
        &self,
        _gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        let gate = self.gate.clone();
        let device = Arc::clone(&self.device);
        Box::pin(async move {
            gate.entered.notify_one();
            gate.open.notified().await;
            let tunnel: Box<dyn BusTunnel> = Box::new(SimTunnel {
                device,
                disconnected: gate.disconnected,
            });
            Ok(tunnel)
        })
    }

    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}

/// Refuses every tunnel: the table rows must never reach the bus.
struct NoTunnel;

impl GatewayConnector for NoTunnel {
    fn connect_tunnel(
        &self,
        _gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        panic!("a table row reached the bus")
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
        post_restart_disconnect_wait: Duration::from_millis(5),
        programming_mode_broadcast_timeout: Duration::from_millis(20),
    }
}

/// Sends `request` with the project lock taken at `stage`, checks that the
/// runtime stayed free and that the request waited for the lock, and
/// returns the answer given after release.
async fn witness(
    state: &Arc<AppState>,
    request: Request<Body>,
    stage: Stage,
    gate: &Gate,
) -> (StatusCode, Value) {
    let router = crate::app(Arc::clone(state), None);
    let label = format!("{} {} at {stage:?}", request.method(), request.uri());
    if !matches!(stage, Stage::AfterTheRead) {
        gate.open.notify_one();
    }
    let scan = match stage {
        Stage::UnderReservation => Some(Arc::clone(&state.line_scan_session).lock_owned().await),
        _ => None,
    };
    let mut held =
        matches!(stage, Stage::Immediately).then(|| super::tests::hold_project_lock(state));
    let call = tokio::spawn(async move {
        let response = router.oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body = serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
        (status, body)
    });
    match stage {
        Stage::Immediately => {}
        Stage::UnderReservation => {
            // The programming reservation is taken after the first project
            // wait and before the scan lock this test still holds.
            tokio::time::timeout(HANG_GUARD, async {
                while state.address_programming.try_lock().is_ok() {
                    tokio::time::sleep(Duration::from_millis(1)).await;
                }
            })
            .await
            .unwrap_or_else(|_| panic!("{label}: the reservation was never taken"));
            held = Some(super::tests::hold_project_lock(state));
            drop(scan);
        }
        Stage::AfterTheRead => {
            tokio::time::timeout(HANG_GUARD, gate.entered.notified())
                .await
                .unwrap_or_else(|_| panic!("{label}: no tunnel was asked for"));
            held = Some(super::tests::hold_project_lock(state));
            gate.open.notify_one();
            tokio::time::timeout(HANG_GUARD, gate.disconnected.notified())
                .await
                .unwrap_or_else(|_| panic!("{label}: the tunnel was never closed"));
        }
    }
    let (release, holder) = held.expect("the project lock is held");
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    let early = call.is_finished();
    let _ = release.send(());
    // A blocked runtime also finishes the call late, so check the holder first.
    assert!(holder.join().unwrap(), "{label}: blocked the runtime");
    assert!(!early, "{label}: answered without waiting for the project");
    call.await.unwrap()
}

/// A state whose bus is refused and whose project has device 1 (no address).
fn plain_state(products: bool) -> Arc<AppState> {
    let product_db = products.then(|| {
        let dir = tempfile::tempdir().unwrap();
        let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        // The open connection keeps the file usable once the directory is gone.
        drop(dir);
        crate::CatalogueOwner::adopt_opened(conn, Default::default()).unwrap()
    });
    let state = Arc::new(AppState {
        product_db,
        connector: Box::new(NoTunnel),
        ..Default::default()
    });
    project_with_device(&state);
    state
}

/// Each row reaches its first project wait and is refused under the lock.
#[tokio::test(flavor = "current_thread")]
async fn first_project_waits_of_mixed_routes_leave_the_runtime_free() {
    let serial_write = json!({
        "address": "1.1.9",
        "gateway": GATEWAY,
        "confirmation": "I confirm individual-address programming to 1.1.9",
        "device": "KB-DEV-1",
    })
    .to_string();
    let preview = json!({
        "deviceId": 1,
        "gateway": GATEWAY,
        "profile": "generic-identity-v1",
        "context": {"serverIncarnation": "x", "projectIncarnation": 0, "revision": 0},
    })
    .to_string();
    let partial = json!({
        "address": "1.1.7",
        "gateway": GATEWAY,
        "partial": {"parameters": true, "groupAddresses": false},
    })
    .to_string();
    let full = json!({"address": "1.1.7", "gateway": GATEWAY}).to_string();
    let find = format!("/api/device-address/find-serial?gateway={GATEWAY}&device=KB-DEV-1");
    let rows: &[(bool, &str, &str, &str, StatusCode, &str)] = &[
        // Full comparison: the binding is decided under the project lock.
        (
            true,
            "POST",
            "/api/device-compare",
            &full,
            StatusCode::CONFLICT,
            "deviceCompareNotComparable",
        ),
        // Partial comparison: the binding is read before the product check.
        (
            false,
            "POST",
            "/api/device-compare",
            &partial,
            StatusCode::CONFLICT,
            "downloadProductDataMissing",
        ),
        (
            false,
            "POST",
            "/api/device-diagnostics/preview",
            &preview,
            StatusCode::BAD_REQUEST,
            "diagnostics-address-missing",
        ),
        (
            false,
            "POST",
            "/api/device-address/by-serial",
            &serial_write,
            StatusCode::UNPROCESSABLE_ENTITY,
            "records no serial number",
        ),
        (
            false,
            "GET",
            &find,
            "",
            StatusCode::UNPROCESSABLE_ENTITY,
            "records no serial number",
        ),
    ];
    for &(products, method, uri, body, expected, fragment) in rows {
        let state = plain_state(products);
        let (status, answer) = witness(
            &state,
            request(method, uri, body),
            Stage::Immediately,
            &Gate::default(),
        )
        .await;
        assert_eq!(status, expected, "{method} {uri}: {answer}");
        assert!(
            answer.to_string().contains(fragment),
            "{method} {uri}: {answer}"
        );
    }
}

/// The corpus-free plug-in fixture of `http_device_compare.rs`: a product
/// database with one program and a simulated device that holds it.
fn compare_fixture(dir: &tempfile::TempDir, gate: &Gate) -> Arc<AppState> {
    use knx_testsupport::vendor_plugin as fixture;
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(
        &products,
        "M-FFF6/A.xml",
        fixture::program_xml(&fixture::declaration()).as_bytes(),
    )
    .unwrap();
    knx_productdb::ingest_file(
        &products,
        "M-FFF6/Hardware.xml",
        fixture::HARDWARE_XML.as_bytes(),
    )
    .unwrap();
    let device = Arc::new(SimulatedDevice::with_config_at(
        fixture::TARGET.parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0701,
            ..Default::default()
        },
    ));
    device.preset_property(0, PID_MANUFACTURER_ID, &[0xFF, 0xF6]);
    device.preset_property(
        3,
        knx_core::commissioning::properties::PID_PROGRAM_VERSION,
        &[0xFF, 0xF6, 0x00, 0x01, 0x01],
    );
    device.preset_memory(0x4000, &[0; 8]);
    device.preset_memory(0x4400, &[1, 2, 3, 4]);
    for machine in [1, 3] {
        device.preset_load_state(ObjectIndex::new(machine), LoadState::Loaded);
    }
    let state = Arc::new(AppState {
        product_db: Some(
            crate::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
        ),
        connector: Box::new(GatedConnector {
            device,
            gate: gate.clone(),
        }),
        device_download_timing: fast(),
        ..AppState::new(dir.path().to_path_buf())
    });
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-1".into(),
        },
        name: "Synthetic plug-in witness".into(),
        description: None,
        address: Some(fixture::TARGET.parse().unwrap()),
        product_ref: fixture::PRODUCT_REF.into(),
        program_ref: fixture::HARDWARE2PROGRAM_REF.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    *state.project.lock().unwrap() = Some(project);
    state
}

/// The comparison's preparation, its check under the gateway reservations and
/// its result check and retention after the read each wait off the runtime,
/// and the read still yields a retained observation.
#[tokio::test(flavor = "current_thread")]
async fn the_device_comparison_waits_off_the_runtime() {
    for stage in [
        Stage::Immediately,
        Stage::UnderReservation,
        Stage::AfterTheRead,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let gate = Gate::default();
        let state = compare_fixture(&dir, &gate);
        let body = json!({"address": "1.1.7", "gateway": GATEWAY}).to_string();
        let (status, answer) = witness(
            &state,
            request("POST", "/api/device-compare", &body),
            stage,
            &gate,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{stage:?}: {answer}");
        assert_eq!(answer["written"], false, "{stage:?}");
        assert_eq!(answer["outdated"], false, "{stage:?}");
        assert_eq!(answer["deviceId"], 1, "{stage:?}");
        assert!(
            answer["observationToken"].is_string(),
            "{stage:?}: {answer}"
        );
        // The reservations are all released again.
        assert!(state.address_programming.try_lock().is_ok(), "{stage:?}");
        assert!(state.bus_session.try_lock().is_ok(), "{stage:?}");
        assert!(state.line_scan_session.try_lock().is_ok(), "{stage:?}");
    }
}

/// `http_support_diagnostics.rs`'s device: mask 0025h at 1.1.67, device 7.
fn support_fixture(dir: &tempfile::TempDir, gate: &Gate) -> Arc<AppState> {
    let device = Arc::new(SimulatedDevice::with_config_at(
        "1.1.67".parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0025,
            ..Default::default()
        },
    ));
    device.preset_property(0, 12, &[0, 1]);
    device.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5b]);
    let state = Arc::new(AppState {
        connector: Box::new(GatedConnector {
            device,
            gate: gate.clone(),
        }),
        ..AppState::new(dir.path().to_path_buf())
    });
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(7),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-7".into(),
        },
        name: "Support witness".into(),
        description: None,
        address: Some("1.1.67".parse().unwrap()),
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    *state.project.lock().unwrap() = Some(project);
    state
}

/// The support diagnose run's admission check, its recheck under the gateway
/// reservations and its final check and publication after the read each wait
/// off the runtime; the run still publishes its live facts.
#[tokio::test(flavor = "current_thread")]
async fn the_support_diagnose_run_waits_off_the_runtime() {
    for stage in [
        Stage::Immediately,
        Stage::UnderReservation,
        Stage::AfterTheRead,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let gate = Gate::default();
        let state = support_fixture(&dir, &gate);
        let context = json!({
            "serverIncarnation": state.server_incarnation,
            "projectIncarnation": state.project_incarnation.load(std::sync::atomic::Ordering::SeqCst),
            "revision": state.project_revision.load(std::sync::atomic::Ordering::SeqCst),
        });
        let preview = crate::app(Arc::clone(&state), None)
            .oneshot(request(
                "POST",
                "/api/support/preview",
                &json!({"deviceId": 7, "context": context, "input": {"mask": 37}}).to_string(),
            ))
            .await
            .unwrap();
        assert_eq!(preview.status(), StatusCode::OK, "{stage:?}");
        let preview: Value = serde_json::from_slice(
            &axum::body::to_bytes(preview.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        let body =
            json!({"token": preview["token"], "gateway": GATEWAY, "consent": true}).to_string();
        let (status, answer) = witness(
            &state,
            request("POST", "/api/support/diagnose", &body),
            stage,
            &gate,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{stage:?}: {answer}");
        assert_eq!(answer["token"], preview["token"], "{stage:?}");
        let live = answer["report"]["facts"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|fact| fact["origin"] == "live" && fact["field"] == "hardwareType")
            .count();
        assert_eq!(live, 1, "{stage:?}: {answer}");
        assert!(state.address_programming.try_lock().is_ok(), "{stage:?}");
        assert!(state.bus_session.try_lock().is_ok(), "{stage:?}");
        assert!(state.line_scan_session.try_lock().is_ok(), "{stage:?}");
    }
}
