//! A synthetic HTTP download on a simulator: the durable backup holds the serial, no body does.

use std::future::Future;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::commissioning::device_backup::written_regions;
use knx_core::commissioning::device_backup::ExemplarEvidence;
use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::memory_download::{
    machines, source_procedure_descriptions, ExemplarSerial, MemoryDownloadStep,
};
use knx_core::commissioning::partial_memory_download::application_identity;
use knx_core::commissioning::properties::{
    ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION, PID_SERIAL_NUMBER,
};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, IndividualAddress,
    Installation, InstallationId, Language, Project, SourceRef, Topology,
};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::{
    ApplicationService, BusError, Destination, DiscoveredGateway, ManagementTransport,
    SessionTiming, Tpci, TunnelEvent,
};
use knx_server::{BusSessionError, BusTunnel, GatewayConnector};
use knx_testsupport::vendor_plugin::{
    program_xml, HARDWARE2PROGRAM_REF, HARDWARE_XML, PRODUCT_REF, PROGRAM_ID, TARGET,
};
use serde_json::{json, Value};
use tokio::sync::broadcast;
use tower::ServiceExt;

const SERIAL: [u8; 6] = [0x00, 0x83, 0xAB, 0xCD, 0xEF, 0x12];

mod customer_adjustable;
mod inactive_selection;

struct Tunnel(Arc<SimulatedDevice>);

impl BusTunnel for Tunnel {
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

struct Connector(Arc<SimulatedDevice>);

impl GatewayConnector for Connector {
    fn connect_tunnel(
        &self,
        _gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        let tunnel: Box<dyn BusTunnel> = Box::new(Tunnel(Arc::clone(&self.0)));
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
        post_restart_disconnect_wait: Duration::from_millis(5),
        programming_mode_broadcast_timeout: Duration::from_millis(20),
    }
}

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-1".into(),
        },
        name: "Synthetic actuator".into(),
        description: None,
        address: Some(TARGET.parse().unwrap()),
        product_ref: PRODUCT_REF.into(),
        program_ref: HARDWARE2PROGRAM_REF.into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

/// A simulator at `TARGET` that answers like the plan's product: identity, loaded machines, old memory.
fn device_for(
    plan: &knx_core::commissioning::memory_download::MemoryDownloadPlan,
) -> SimulatedDevice {
    let device = SimulatedDevice::with_config_at(
        TARGET.parse().unwrap(),
        SimulatorConfig {
            mask_version: plan.mask.0,
            ..SimulatorConfig::default()
        },
    );
    device.preset_property(0, PID_MANUFACTURER_ID, &plan.manufacturer.to_be_bytes());
    for step in &plan.steps {
        if let MemoryDownloadStep::CompareProperty {
            object_index,
            property_id,
            inline_data,
        } = step
        {
            let width = if *property_id == PID_HARDWARE_TYPE {
                6
            } else {
                inline_data.len()
            };
            device.preset_property(*object_index, *property_id, &inline_data[..width]);
        }
    }
    if let Some(identity) = application_identity(plan) {
        device.preset_property(3, PID_PROGRAM_VERSION, &identity);
    }
    device.preset_property(0, PID_SERIAL_NUMBER, &SERIAL);
    for machine in machines(plan) {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
    }
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
    }
    for (address, length) in written_regions(plan) {
        device.preset_memory(u32::from(address), &vec![0x5A; length]);
    }
    device
}

async fn call(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    let body = serde_json::from_slice(&bytes).unwrap_or(Value::String(text.clone()));
    (status, body, text)
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

#[tokio::test]
async fn an_http_download_keeps_the_serial_in_the_private_backup_and_in_no_body() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", program_xml("").as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    let plan = knx_app::download_support::offline_plan(&products, PROGRAM_ID).unwrap();
    let device = Arc::new(device_for(&plan));
    let mut state = knx_server::AppState::new(dir.path().to_path_buf());
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    state.connector = Box::new(Connector(Arc::clone(&device)));
    state.device_download_timing = fast();
    *state.project.lock().unwrap() = Some(project());
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, shown, shown_text) = call(
        &app,
        post("/api/device-download/plan", json!({ "address": TARGET })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{shown}");
    let phrase = shown["confirmationPhrase"]
        .as_str()
        .expect("a phrase")
        .to_owned();
    let (status, started, started_text) = call(
        &app,
        post(
            "/api/device-download/start",
            json!({
                "planId": shown["planId"],
                "gateway": "192.0.2.10:3671",
                "confirmation": phrase,
                "acceptUntested": format!("I accept an untested download to {TARGET}"),
            }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{started}");

    let mut last = Value::Null;
    let mut events_text = String::new();
    for _ in 0..2000 {
        let (status, body, text) = call(&app, get("/api/device-download/status?since=0")).await;
        if status == StatusCode::OK {
            events_text = text;
            last = body["status"].clone();
            if last["state"] != "running" {
                break;
            }
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    assert_eq!(last["state"], "finished", "{last}");

    // The durable private file: format 4, uppercase hex, the observed serial.
    let (_, now, status_now) = call(&app, get("/api/device-download/status")).await;
    let file = now["backupFile"]
        .as_str()
        .expect("a backup file")
        .to_owned();
    let path = dir.path().join(&file);
    let on_disk = std::fs::read_to_string(&path).unwrap();
    assert!(on_disk.contains("\"format\": 4"), "{on_disk}");
    assert!(
        on_disk.contains("\"exemplar_serial\": \"0083ABCDEF12\""),
        "{on_disk}"
    );
    let stored = knx_app::device_backup::read_backup(&path).unwrap();
    assert_eq!(
        stored.backup.exemplar_serial,
        ExemplarEvidence::Recorded(ExemplarSerial::new(SERIAL))
    );
    assert_eq!(stored.plan_steps, source_procedure_descriptions(&plan));
    // The download itself read the serial once, before the first write.
    let seen = device.seen();
    let read = seen
        .iter()
        .position(|s| {
            matches!(
                s,
                Seen::PropertyRead {
                    object_index: 0,
                    property_id: PID_SERIAL_NUMBER
                }
            )
        })
        .expect("the serial was read");
    let write = seen
        .iter()
        .position(|s| matches!(s, Seen::MemoryWrite { .. }))
        .expect("the download wrote");
    assert!(read < write);

    // No HTTP body carries it, in any rendering.
    let (_, _, history) = call(&app, get("/api/bus/history")).await;
    let (_, _, report) = call(&app, get("/api/device-download/report")).await;
    assert!(
        history.contains("deviceDownload"),
        "the run is in the history: {history}"
    );
    assert!(report.contains("outcome"), "a report exists: {report}");
    for (name, body) in [
        ("plan", &shown_text),
        ("start", &started_text),
        ("events", &events_text),
        ("status", &status_now),
        ("history", &history),
        ("report", &report),
    ] {
        let upper = body.to_uppercase();
        for needle in [
            "0083ABCDEF12",
            "00 83 AB CD EF 12",
            "ABCDEF12",
            "171, 205, 239",
        ] {
            assert!(
                !upper.contains(needle),
                "{name} body carries the serial: {body}"
            );
        }
    }
}
