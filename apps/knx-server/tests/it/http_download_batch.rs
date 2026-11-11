//! Batch download (ADR-0172) through the web API, on simulated devices and fake tunnels only.
//!
//! Synthetic throughout ([`knx_testsupport::vendor_plugin`]'s program, plus a
//! System-2 and a check-less variant of it): every *not in this batch*
//! reason, the refusal of a whole batch on one wrong phrase, the run that
//! starts each device exactly as the single start does and stops at the
//! first doubt, stop after this device, server stop and the shown-plan slot
//! shared with the single download. The connector hands out one simulated
//! device per connect, in address order, and counts every connect: no socket,
//! no gateway, no bus.

use std::collections::VecDeque;
use std::future::Future;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, Weak};
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::commissioning::device_backup::written_regions;
use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::memory_download::{machines, MemoryDownloadPlan, MemoryDownloadStep};
use knx_core::commissioning::partial_memory_download::application_identity;
use knx_core::commissioning::properties::{
    ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
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
use knx_testsupport::vendor_plugin::{program_xml, PROGRAM_ID};
use serde_json::{json, Value};
use tokio::sync::{broadcast, Notify};
use tower::ServiceExt;

const GATEWAY: &str = "192.0.2.10:3671";
/// The regular synthetic program, a System-2 copy and one without a task segment.
const SYSTEM2_PROGRAM: &str = "M-FFF6_A-0002-01-0000";
const NO_CHECK_PROGRAM: &str = "M-FFF6_A-0003-01-0000";

const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-FFF6"><Hardware>
<Hardware Id="M-FFF6_H-1" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1"><Products><Product Id="M-FFF6_H-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="M-FFF6_H-1_HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-FFF6_A-0001-01-0000" /></Hardware2Program></Hardware2Programs></Hardware>
<Hardware Id="M-FFF6_H-2" Name="Synthetic S2" SerialNumber="synthetic2" VersionNumber="1"><Products><Product Id="M-FFF6_H-2_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="M-FFF6_H-2_HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-FFF6_A-0002-01-0000" /></Hardware2Program></Hardware2Programs></Hardware>
<Hardware Id="M-FFF6_H-3" Name="Synthetic no check" SerialNumber="synthetic3" VersionNumber="1"><Products><Product Id="M-FFF6_H-3_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="M-FFF6_H-3_HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="M-FFF6_A-0003-01-0000" /></Hardware2Program></Hardware2Programs></Hardware>
</Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// Which synthetic product a project device references.
#[derive(Clone, Copy)]
enum Program {
    Regular,
    System2,
    NoCheck,
    Unknown,
}

impl Program {
    fn refs(self) -> (&'static str, &'static str) {
        match self {
            Program::Regular => ("M-FFF6_H-1_P-1", "M-FFF6_H-1_HP-1"),
            Program::System2 => ("M-FFF6_H-2_P-1", "M-FFF6_H-2_HP-1"),
            Program::NoCheck => ("M-FFF6_H-3_P-1", "M-FFF6_H-3_HP-1"),
            Program::Unknown => ("M-FFF6_H-9_P-1", "M-FFF6_H-9_HP-1"),
        }
    }
}

/// Flags with the given parts pending; the medium configuration loaded.
fn flags(address: bool, application: bool, parameters: bool, tables: bool) -> CommissioningState {
    CommissioningState {
        individual_address_loaded: !address,
        application_program_loaded: !application,
        parameters_loaded: !parameters,
        communication_part_loaded: !tables,
        medium_config_loaded: true,
        ..CommissioningState::default()
    }
}

fn app_pending() -> CommissioningState {
    flags(false, true, false, false)
}

struct Device {
    id: u32,
    address: Option<&'static str>,
    program: Program,
    flags: CommissioningState,
}

fn project(devices: &[Device]) -> Project {
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
            unassigned: devices.iter().map(|device| DeviceId(device.id)).collect(),
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    for device in devices {
        let (product, program) = device.program.refs();
        project.devices.insert(DeviceInstance {
            id: DeviceId(device.id),
            source: SourceRef {
                path: "synthetic".into(),
                ets_id: format!("DEV-{}", device.id),
            },
            name: format!("Device {}", device.id),
            description: None,
            address: device.address.map(|address| address.parse().unwrap()),
            product_ref: product.into(),
            program_ref: program.into(),
            commissioning: device.flags.clone(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    project
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

/// A simulator at `address` that answers like the regular program's device.
fn simulated(
    address: &str,
    plan: &MemoryDownloadPlan,
    config: SimulatorConfig,
) -> Arc<SimulatedDevice> {
    let device = SimulatedDevice::with_config_at(
        address.parse().unwrap(),
        SimulatorConfig {
            mask_version: plan.mask.0,
            ..config
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
    for machine in machines(plan) {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
    }
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Loaded);
    }
    for (start, length) in written_regions(plan) {
        device.preset_memory(u32::from(start), &vec![0x5A; length]);
    }
    Arc::new(device)
}

type Hook = Box<dyn FnOnce() + Send>;

/// What one connect does: optional hook and pause, then a device or a refusal.
#[derive(Default)]
struct Connect {
    device: Option<Arc<SimulatedDevice>>,
    before: Option<Hook>,
    /// Notified on arrival; the connect waits for the second.
    pause: Option<(Arc<Notify>, Arc<Notify>)>,
    /// Runs once, right after the first memory write went to the device.
    after_first_write: Option<Hook>,
}

struct Tunnel {
    device: Arc<SimulatedDevice>,
    after_first_write: Mutex<Option<Hook>>,
}

impl BusTunnel for Tunnel {
    fn assigned_address(&self) -> IndividualAddress {
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
        panic!("a download sends no group telegram")
    }

    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move {
            let result = ManagementTransport::send_frame(
                self.device.as_ref(),
                destination,
                transport,
                service,
            )
            .await;
            if self
                .device
                .seen()
                .iter()
                .any(|seen| matches!(seen, Seen::MemoryWrite { .. }))
            {
                if let Some(hook) = self.after_first_write.lock().unwrap().take() {
                    hook();
                }
            }
            result
        })
    }

    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}

struct Connector {
    plan: Arc<Mutex<VecDeque<Connect>>>,
    calls: Arc<AtomicUsize>,
}

impl GatewayConnector for Connector {
    fn connect_tunnel(
        &self,
        _gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.calls.fetch_add(1, Ordering::SeqCst);
        let next = self.plan.lock().unwrap().pop_front().unwrap_or_default();
        Box::pin(async move {
            if let Some(hook) = next.before {
                hook();
            }
            if let Some((arrived, release)) = next.pause {
                arrived.notify_one();
                release.notified().await;
            }
            match next.device {
                Some(device) => {
                    let tunnel: Box<dyn BusTunnel> = Box::new(Tunnel {
                        device,
                        after_first_write: Mutex::new(next.after_first_write),
                    });
                    Ok(tunnel)
                }
                None => Err(BusSessionError::Transport(BusError::Timeout)),
            }
        })
    }

    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async { Ok(Vec::new()) })
    }
}

struct Harness {
    _dir: tempfile::TempDir,
    state: Arc<knx_server::AppState>,
    app: axum::Router,
    plan: MemoryDownloadPlan,
    connects: Arc<Mutex<VecDeque<Connect>>>,
    calls: Arc<AtomicUsize>,
}

impl Harness {
    fn weak(&self) -> Weak<knx_server::AppState> {
        Arc::downgrade(&self.state)
    }

    /// Queues one connect that hands out a fresh simulator at `address`.
    fn device(&self, address: &str) -> Arc<SimulatedDevice> {
        self.device_with(address, SimulatorConfig::default())
    }

    fn device_with(&self, address: &str, config: SimulatorConfig) -> Arc<SimulatedDevice> {
        let device = simulated(address, &self.plan, config);
        self.queue(Connect {
            device: Some(Arc::clone(&device)),
            ..Connect::default()
        });
        device
    }

    fn queue(&self, connect: Connect) {
        self.connects.lock().unwrap().push_back(connect);
    }

    fn flags(&self, id: u32) -> CommissioningState {
        self.state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .devices
            .get(DeviceId(id))
            .unwrap()
            .commissioning
            .clone()
    }
}

fn harness(devices: &[Device]) -> Harness {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let regular = program_xml("");
    let system2 = regular
        .replace(PROGRAM_ID, SYSTEM2_PROGRAM)
        .replace("ApplicationNumber=\"1\"", "ApplicationNumber=\"2\"")
        .replace("MV-0701", "MV-0025");
    let no_check = regular
        .replace(PROGRAM_ID, NO_CHECK_PROGRAM)
        .replace("ApplicationNumber=\"1\"", "ApplicationNumber=\"3\"")
        .replace("<LdCtrlTaskSegment LsmIdx=\"3\" Address=\"17408\" />", "");
    assert_ne!(
        no_check.len(),
        regular.len(),
        "the task segment was removed"
    );
    for (file, xml) in [
        ("M-FFF6/A1.xml", regular.as_str()),
        ("M-FFF6/A2.xml", system2.as_str()),
        ("M-FFF6/A3.xml", no_check.as_str()),
        ("M-FFF6/Hardware.xml", HARDWARE),
    ] {
        knx_productdb::ingest_file(&products, file, xml.as_bytes()).unwrap();
    }
    let plan = knx_app::download_support::offline_plan(&products, PROGRAM_ID).unwrap();
    let connects = Arc::new(Mutex::new(VecDeque::new()));
    let calls = Arc::new(AtomicUsize::new(0));
    let mut state = knx_server::AppState::new(dir.path().to_path_buf());
    state.product_db = Some(
        knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
    );
    state.connector = Box::new(Connector {
        plan: Arc::clone(&connects),
        calls: Arc::clone(&calls),
    });
    state.device_download_timing = fast();
    *state.project.lock().unwrap() = Some(project(devices));
    let state = Arc::new(state);
    let app = knx_server::app(Arc::clone(&state), None);
    Harness {
        _dir: dir,
        state,
        app,
        plan,
        connects,
        calls,
    }
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes)
            .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned())),
    )
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

async fn review(h: &Harness) -> Value {
    let (status, body) = send(&h.app, post("/api/device-download/batch/plan", json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

/// The start entry of every reviewed device in the batch, phrase and
/// acknowledgement as the review names them.
fn entries(review: &Value) -> Vec<Value> {
    review["devices"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|device| device["status"] == "inBatch")
        .map(|device| {
            json!({
                "address": device["address"],
                "confirmation": device["confirmationPhrase"],
                "acceptUntested": device["untestedAcknowledgement"],
            })
        })
        .collect()
}

async fn start(h: &Harness, review: &Value, devices: Vec<Value>) -> (StatusCode, Value) {
    send(
        &h.app,
        post(
            "/api/device-download/batch/start",
            json!({ "batchId": review["batchId"], "gateway": GATEWAY, "devices": devices }),
        ),
    )
    .await
}

/// Polls the batch until it has ended.
async fn finish(h: &Harness) -> Value {
    for _ in 0..4000 {
        let (status, body) = send(&h.app, get("/api/device-download/batch/status")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        if body["state"] == "ended" {
            return body;
        }
        tokio::time::sleep(Duration::from_millis(5)).await;
    }
    panic!("the batch did not end");
}

fn states(status: &Value) -> Vec<(String, String)> {
    status["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|device| {
            (
                device["address"].as_str().unwrap_or("-").to_string(),
                device["state"].as_str().unwrap().to_string(),
            )
        })
        .collect()
}

fn pair(address: &str, state: &str) -> (String, String) {
    (address.to_string(), state.to_string())
}

fn writes(device: &SimulatedDevice) -> usize {
    device
        .seen()
        .iter()
        .filter(|seen| matches!(seen, Seen::MemoryWrite { .. }))
        .count()
}

async fn history_downloads(h: &Harness) -> Vec<Value> {
    let (status, body) = send(&h.app, get("/api/bus/history")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["entries"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|entry| entry["kind"] == "deviceDownload")
        .cloned()
        .collect()
}

fn three_regular() -> Vec<Device> {
    ["1.1.8", "1.1.9", "1.1.10"]
        .into_iter()
        .enumerate()
        .map(|(index, address)| Device {
            id: index as u32 + 1,
            address: Some(address),
            program: Program::Regular,
            flags: app_pending(),
        })
        .collect()
}

#[tokio::test]
async fn the_review_lists_every_pending_device_with_its_reason_and_sends_nothing() {
    let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
    let excluded: &'static str = Box::leak(excluded.into_boxed_str());
    let h = harness(&[
        Device {
            id: 1,
            address: None,
            program: Program::Regular,
            flags: app_pending(),
        },
        Device {
            id: 2,
            address: Some(excluded),
            program: Program::Regular,
            flags: app_pending(),
        },
        Device {
            id: 3,
            address: Some("1.1.3"),
            program: Program::Regular,
            flags: flags(true, false, false, false),
        },
        Device {
            id: 4,
            address: Some("1.1.4"),
            program: Program::Regular,
            flags: flags(true, true, false, false),
        },
        Device {
            id: 5,
            address: Some("1.1.5"),
            program: Program::System2,
            flags: app_pending(),
        },
        Device {
            id: 6,
            address: Some("1.1.6"),
            program: Program::Unknown,
            flags: app_pending(),
        },
        Device {
            id: 7,
            address: Some("1.1.7"),
            program: Program::NoCheck,
            flags: app_pending(),
        },
        Device {
            id: 8,
            address: Some("1.1.10"),
            program: Program::Regular,
            flags: flags(false, false, true, false),
        },
        Device {
            id: 9,
            address: Some("1.1.9"),
            program: Program::Regular,
            flags: app_pending(),
        },
        Device {
            id: 10,
            address: Some("1.1.11"),
            program: Program::Regular,
            flags: flags(false, false, false, false),
        },
    ]);
    let body = review(&h).await;
    let rows = body["devices"].as_array().unwrap();
    let summary: Vec<(Value, Value, Value)> = rows
        .iter()
        .map(|row| {
            (
                row["address"].clone(),
                row["status"].clone(),
                row["reason"]["code"].clone(),
            )
        })
        .collect();
    let mut addressed = vec![
        (
            json!("1.1.3"),
            json!("notInBatch"),
            json!("nothingToDownload"),
        ),
        (json!("1.1.4"), json!("notInBatch"), json!("addressPending")),
        (json!("1.1.5"), json!("notInBatch"), json!("system2")),
        (json!("1.1.6"), json!("notInBatch"), json!("unsupported")),
        (
            json!("1.1.7"),
            json!("notInBatch"),
            json!("noPostDownloadCheck"),
        ),
        (json!("1.1.9"), json!("inBatch"), Value::Null),
        (json!("1.1.10"), json!("inBatch"), Value::Null),
        (json!(excluded), json!("notInBatch"), json!("excluded")),
    ];
    // The pending block's order: no address first, then by address (1.1.10
    // after 1.1.9); 1.1.11 has nothing pending and is not listed.
    addressed.sort_by_key(|(address, _, _)| {
        address
            .as_str()
            .unwrap()
            .parse::<IndividualAddress>()
            .unwrap()
    });
    let mut expected = vec![(Value::Null, json!("notInBatch"), json!("noAddress"))];
    expected.extend(addressed);
    assert_eq!(summary, expected, "{body}");

    let unsupported = &rows.iter().find(|row| row["address"] == "1.1.6").unwrap()["reason"];
    assert!(unsupported["category"].is_string(), "{unsupported}");
    assert!(unsupported["detail"].is_string(), "{unsupported}");
    let complete = rows.iter().find(|row| row["address"] == "1.1.9").unwrap();
    assert_eq!(complete["scope"], "complete", "{complete}");
    assert_eq!(complete["support"], "untested", "{complete}");
    assert_eq!(
        complete["confirmationPhrase"],
        "I confirm download to 1.1.9"
    );
    assert_eq!(
        complete["untestedAcknowledgement"],
        "I accept an untested download to 1.1.9"
    );
    let partial = rows.iter().find(|row| row["address"] == "1.1.10").unwrap();
    assert_eq!(partial["scope"], "parameters", "{partial}");
    assert_ne!(partial["planId"], complete["planId"]);
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        0,
        "a review opens no tunnel"
    );
    assert!(history_downloads(&h).await.is_empty());
}

#[tokio::test]
async fn one_wrong_phrase_or_acknowledgement_refuses_the_whole_batch_before_any_tunnel() {
    let h = harness(&three_regular());
    let body = review(&h).await;
    let good = entries(&body);
    assert_eq!(good.len(), 3);
    let mut wrong_phrase = good.clone();
    wrong_phrase[1]["confirmation"] = json!("I confirm download to 1.1.8");
    let mut no_acknowledgement = good.clone();
    no_acknowledgement[2]["acceptUntested"] = Value::Null;
    let mut unknown = good.clone();
    unknown.push(json!({ "address": "1.1.42", "confirmation": "I confirm download to 1.1.42" }));
    let mut twice = good.clone();
    twice.push(good[0].clone());
    for (devices, kind) in [
        (wrong_phrase, "downloadConfirmationRefused"),
        (no_acknowledgement, "downloadConfirmationRefused"),
        (unknown, "batchDeviceNotInBatch"),
        (twice, "batchDeviceRepeated"),
        (vec![], "batchEmpty"),
    ] {
        let (status, refused) = start(&h, &body, devices).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
        assert_eq!(refused["kind"], kind, "{refused}");
    }
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
    assert!(history_downloads(&h).await.is_empty());
    let (status, _) = send(&h.app, get("/api/device-download/batch/status")).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "no batch ever started");
}

#[tokio::test]
async fn every_device_runs_as_a_single_download_and_the_batch_goes_on_only_after_a_full_match() {
    let h = harness(&three_regular());
    let simulators: Vec<_> = ["1.1.8", "1.1.9", "1.1.10"]
        .into_iter()
        .map(|address| h.device(address))
        .collect();
    let body = review(&h).await;
    let (status, started) = start(&h, &body, entries(&body)).await;
    assert_eq!(status, StatusCode::OK, "{started}");
    assert_eq!(started["batchId"], body["batchId"]);
    let status = finish(&h).await;
    assert_eq!(
        states(&status),
        vec![
            pair("1.1.8", "done"),
            pair("1.1.9", "done"),
            pair("1.1.10", "done")
        ],
        "{status}"
    );
    for device in &status["devices"].as_array().unwrap()[..] {
        assert_eq!(device["written"], "yes", "{device}");
        assert_eq!(
            device["postDownloadCheck"]["observation"]["reach"]["identity"]["planLoadVerdict"],
            "matchesPlan",
            "{device}"
        );
    }
    for simulator in &simulators {
        assert!(writes(simulator) > 0, "every device was written");
    }
    assert_eq!(h.calls.load(Ordering::SeqCst), 3, "one tunnel per device");
    // One history row per device, each its own download id.
    let rows = history_downloads(&h).await;
    assert_eq!(rows.len(), 3, "{rows:?}");
    // One recording per device, each linked to the batch.
    assert!(h.state.recordings.wait_until_idle(Duration::from_secs(10)));
    let recordings = h.state.recordings.list().unwrap();
    let downloads: Vec<_> = recordings
        .iter()
        .filter(|summary| summary.operation_kind.as_deref() == Some("download"))
        .collect();
    assert_eq!(downloads.len(), 3, "{recordings:?}");
    for summary in downloads {
        let text = String::from_utf8(h.state.recordings.read(&summary.id).unwrap()).unwrap();
        let first: Value = serde_json::from_str(text.lines().next().unwrap()).unwrap();
        assert_eq!(
            first["related"]["batchId"],
            body["batchId"].to_string(),
            "{first}"
        );
    }
    // The flags of every finished device settled, its own only.
    for id in 1..=3 {
        assert!(
            h.flags(id).application_program_loaded,
            "device {id} settled"
        );
    }
    // The batch is over: the gateway is free again (the next connect is the
    // connector's default refusal, not a busy reservation).
    let (status, free) = send(
        &h.app,
        post("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_ne!(status, StatusCode::CONFLICT, "{free}");
}

/// Every outcome short of a full match stops the batch: here an unanswered
/// restart (`unconfirmed`), then a gateway that does not connect (`failed`).
#[tokio::test]
async fn a_doubtful_device_stops_the_batch_and_the_rest_is_not_started() {
    let h = harness(&three_regular());
    let first = h.device("1.1.8");
    // A second device whose restart goes unanswered.
    let second = h.device_with(
        "1.1.9",
        SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        },
    );
    let third = h.device("1.1.10");
    let body = review(&h).await;
    let (status, _) = start(&h, &body, entries(&body)).await;
    assert_eq!(status, StatusCode::OK);
    let status = finish(&h).await;
    let rows = states(&status);
    assert_eq!(rows[0], pair("1.1.8", "done"), "{status}");
    assert_eq!(rows[1], pair("1.1.9", "unconfirmed"), "{status}");
    assert_eq!(
        status["devices"][1]["doubt"], "restartUnconfirmed",
        "{status}"
    );
    assert_eq!(rows[2], pair("1.1.10", "notStarted"), "{status}");
    assert_eq!(status["devices"][2]["reason"], "earlierDevice", "{status}");
    assert!(writes(&first) > 0 && writes(&second) > 0);
    assert_eq!(
        writes(&third),
        0,
        "nothing reached the device after the doubt"
    );
    assert_eq!(h.calls.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn a_gateway_that_does_not_connect_fails_that_device_with_nothing_written() {
    let h = harness(&three_regular());
    h.device("1.1.8");
    h.queue(Connect::default());
    let body = review(&h).await;
    let (status, _) = start(&h, &body, entries(&body)).await;
    assert_eq!(status, StatusCode::OK);
    let status = finish(&h).await;
    assert_eq!(
        states(&status),
        vec![
            pair("1.1.8", "done"),
            pair("1.1.9", "failed"),
            pair("1.1.10", "notStarted")
        ],
        "{status}"
    );
    let failed = &status["devices"][1];
    assert_eq!(failed["written"], "no", "{failed}");
    assert_eq!(failed["kind"], "gatewayTimeout", "{failed}");
    assert!(!h.flags(2).application_program_loaded);
    assert!(!h.flags(3).application_program_loaded);
}

/// ADR-0172 §4: a project edit that changes a later device's plan stops the
/// batch before that device; the batch's own settlement of an earlier device
/// does not (the full run above settles each device and goes on).
#[tokio::test]
async fn a_project_edit_between_two_devices_stops_the_batch_before_the_second() {
    let h = harness(&three_regular());
    let first = h.device("1.1.8");
    let weak = h.weak();
    // While the first device connects, the second device's pending parts change.
    h.connects.lock().unwrap()[0].before = Some(Box::new(move || {
        let state = weak.upgrade().unwrap();
        let mut project = state.project.lock().unwrap();
        project
            .as_mut()
            .unwrap()
            .devices
            .get_mut(DeviceId(2))
            .unwrap()
            .commissioning = flags(false, false, true, false);
    }));
    let body = review(&h).await;
    let (status, _) = start(&h, &body, entries(&body)).await;
    assert_eq!(status, StatusCode::OK);
    let status = finish(&h).await;
    assert_eq!(
        states(&status),
        vec![
            pair("1.1.8", "done"),
            pair("1.1.9", "failed"),
            pair("1.1.10", "notStarted")
        ],
        "{status}"
    );
    assert_eq!(
        status["devices"][1]["kind"], "downloadPlanStale",
        "{status}"
    );
    assert_eq!(status["devices"][1]["written"], "no", "{status}");
    assert!(writes(&first) > 0);
    assert_eq!(
        h.calls.load(Ordering::SeqCst),
        1,
        "no tunnel for the changed device"
    );
    assert_eq!(history_downloads(&h).await.len(), 1);
}

#[tokio::test]
async fn stop_after_this_device_lets_it_finish_and_starts_no_other() {
    let h = harness(&three_regular());
    let first = h.device("1.1.8");
    let arrived = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    h.connects.lock().unwrap()[0].pause = Some((Arc::clone(&arrived), Arc::clone(&release)));
    let second = h.device("1.1.9");
    let body = review(&h).await;
    let (status, _) = start(&h, &body, entries(&body)).await;
    assert_eq!(status, StatusCode::OK);
    arrived.notified().await;

    // While the batch runs: busy routes name it, `/plan` leaves it alone.
    let (status, busy) = send(
        &h.app,
        post("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{busy}");
    assert_eq!(busy["holder"]["kind"], "deviceDownloadBatch", "{busy}");
    assert_eq!(busy["holder"]["batchId"], body["batchId"], "{busy}");
    assert_eq!(busy["holder"]["address"], "1.1.8", "{busy}");
    let (status, single) = send(
        &h.app,
        post("/api/device-download/plan", json!({ "address": "1.1.10" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{single}");
    let (status, refused) = send(
        &h.app,
        post(
            "/api/device-download/start",
            json!({ "planId": single["planId"], "gateway": GATEWAY,
                "confirmation": "I confirm download to 1.1.10",
                "acceptUntested": "I accept an untested download to 1.1.10" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert_eq!(
        refused["holder"]["kind"], "deviceDownloadBatch",
        "{refused}"
    );
    let (status, another) = start(&h, &body, entries(&body)).await;
    assert_eq!(
        status,
        StatusCode::CONFLICT,
        "a started batch left the slot: {another}"
    );

    let (status, stopping) = send(
        &h.app,
        post(
            "/api/device-download/batch/stop",
            json!({ "batchId": body["batchId"] }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{stopping}");
    assert_eq!(stopping["stop"], "afterThisDevice");
    release.notify_one();
    let status = finish(&h).await;
    assert_eq!(status["stopRequested"], true);
    assert_eq!(
        states(&status),
        vec![
            pair("1.1.8", "done"),
            pair("1.1.9", "notStarted"),
            pair("1.1.10", "notStarted")
        ],
        "{status}"
    );
    assert_eq!(status["devices"][1]["reason"], "stopRequest", "{status}");
    assert!(writes(&first) > 0, "the device in progress ran to its end");
    assert_eq!(writes(&second), 0);
}

#[tokio::test]
async fn a_server_stop_stops_the_current_device_before_its_first_change_and_starts_no_other() {
    let h = harness(&three_regular());
    let first = h.device("1.1.8");
    let weak = h.weak();
    h.connects.lock().unwrap()[0].before = Some(Box::new(move || {
        weak.upgrade().unwrap().stop_admission.close();
    }));
    let body = review(&h).await;
    let (status, _) = start(&h, &body, entries(&body)).await;
    assert_eq!(status, StatusCode::OK);
    let status = finish(&h).await;
    assert_eq!(
        states(&status),
        vec![
            pair("1.1.8", "stopped"),
            pair("1.1.9", "notStarted"),
            pair("1.1.10", "notStarted")
        ],
        "{status}"
    );
    assert_eq!(status["devices"][1]["reason"], "serverStop", "{status}");
    assert_eq!(
        writes(&first),
        0,
        "ADR-0153: nothing changed after the stop"
    );
    assert_eq!(h.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_server_stop_after_the_first_change_lets_that_device_finish_and_starts_no_other() {
    let h = harness(&three_regular());
    let first = h.device("1.1.8");
    let weak = h.weak();
    h.connects.lock().unwrap()[0].after_first_write = Some(Box::new(move || {
        weak.upgrade().unwrap().stop_admission.close();
    }));
    let body = review(&h).await;
    let (status, _) = start(&h, &body, entries(&body)).await;
    assert_eq!(status, StatusCode::OK);
    let status = finish(&h).await;
    assert_eq!(
        states(&status),
        vec![
            pair("1.1.8", "done"),
            pair("1.1.9", "notStarted"),
            pair("1.1.10", "notStarted")
        ],
        "{status}"
    );
    assert_eq!(status["devices"][1]["reason"], "serverStop", "{status}");
    assert!(writes(&first) > 0);
}

#[tokio::test]
async fn a_deselected_device_is_reported_and_skipped() {
    let h = harness(&three_regular());
    let first = h.device("1.1.8");
    let third = h.device("1.1.10");
    let body = review(&h).await;
    let mut devices = entries(&body);
    devices.remove(1);
    let (status, _) = start(&h, &body, devices).await;
    assert_eq!(status, StatusCode::OK);
    let status = finish(&h).await;
    assert_eq!(
        states(&status),
        vec![
            pair("1.1.8", "done"),
            pair("1.1.9", "deselected"),
            pair("1.1.10", "done")
        ],
        "{status}"
    );
    assert!(writes(&first) > 0 && writes(&third) > 0);
    assert!(
        !h.flags(2).application_program_loaded,
        "a deselected device keeps its flags"
    );
}

/// The slot holds one single plan or one batch; each replaces the other.
#[tokio::test]
async fn a_single_plan_and_a_batch_replace_each_other() {
    let h = harness(&three_regular());
    let batch = review(&h).await;
    let (status, single) = send(
        &h.app,
        post("/api/device-download/plan", json!({ "address": "1.1.8" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{single}");
    let (status, refused) = start(&h, &batch, entries(&batch)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert_eq!(refused["kind"], "downloadPlanStale", "{refused}");

    let _batch = review(&h).await;
    let (status, refused) = send(
        &h.app,
        post(
            "/api/device-download/start",
            json!({ "planId": single["planId"], "gateway": GATEWAY,
                "confirmation": "I confirm download to 1.1.8",
                "acceptUntested": "I accept an untested download to 1.1.8" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{refused}");
    assert_eq!(refused["kind"], "downloadPlanStale", "{refused}");
    assert_eq!(h.calls.load(Ordering::SeqCst), 0);
}
