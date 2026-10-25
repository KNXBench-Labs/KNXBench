//! Mounted System-2 cancellation and original recovery over persistent test hardware.
//!
//! Opt-in mounted worker proof. Test hardware provenance, independent persistent
//! software state, no socket/bus/device and no simulator-bundle promotion.
use axum::{body::Body, http::Request, Router};
use knx_app::{
    property_validation::{read_bundle, Checkpoint, LocalOutcome, OperationStore},
    property_validation_bundle::{OperationRecord, RecoveryRecord},
};
use knx_core::{commissioning::load_state::LoadState, DeviceId, IndividualAddress};
use knx_net::{
    commissioning::{
        simulator::{
            SimulatedAbsoluteAllocation, SimulatedDevice, SimulatedSystem2Object,
            SimulatedSystem2ObjectState, SimulatedSystem2Snapshot, SimulatedTaskControl2,
            SimulatedTaskSegment, SimulatorConfig,
        },
        SessionTiming,
    },
    management::ManagementTransport,
    ApplicationService, BusError, Destination, Tpci, TunnelEvent,
};
use serde_json::{json, Value};
use std::{
    future::Future,
    net::SocketAddrV4,
    path::{Path, PathBuf},
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::{broadcast, Notify};
use tower::ServiceExt;

const PACKAGE_SHA256: &str = "520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f";
const CATALOG_ITEM: &str = "M-0001_H-hp.5F00295-2_HP-9205-04-9270_CI-5WG1.20258.2D2DB12-1";
const GATEWAY: &str = "203.0.113.250:3671";
const TARGET: &str = "1.1.25";
const SERIAL: [u8; 6] = [0, 1, 2, 3, 4, 5];
const EXTENTS: [(u16, u16); 4] = [(278, 330), (608, 416), (1024, 512), (15360, 256)];
const PIR_REFERENCE: &str = "M-0001_A-9205-04-9270_P-900_R-900";
const PIR_RAW: &str = "4";
const PIR_ADDRESS: u16 = 15610; // Source segment 15360 + offset 250, bit 0 / 8.

// Opt-in private evidence only: no tokens, names, complete panels or snapshots.
// The caller supplies an existing private directory; never overwrite evidence.
fn private_diagnostic(name: &str, value: Value) {
    let Some(directory) = std::env::var_os("KNXBENCH_MOUNTED_ACCEPTANCE_DIAGNOSTICS") else {
        return;
    };
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    assert!(bytes.len() <= 16 * 1024, "bounded private diagnostic");
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(PathBuf::from(directory).join(name))
        .expect("create private diagnostic without overwriting prior evidence");
    std::io::Write::write_all(&mut file, &bytes).unwrap();
}

fn pir_fields(panel: &Value) -> Vec<&Value> {
    panel["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|section| section["fields"].as_array().unwrap())
        .filter(|field| field["etsId"] == PIR_REFERENCE)
        .collect()
}

// OLD is never constructed by executing NEW, nor inferred from its resulting
// memory. These source-shape extents match the existing pinned public App proof.
fn old_device() -> Arc<SimulatedDevice> {
    let device = SimulatedDevice::with_config_at(
        TARGET.parse().unwrap(),
        SimulatorConfig {
            mask_version: 0x0025,
            serial_number: Some(SERIAL),
            ..Default::default()
        },
    );
    device.preset_property(
        0,
        knx_core::commissioning::properties::PID_MANUFACTURER_ID,
        &[0, 1],
    );
    device.preset_property(
        0,
        knx_core::commissioning::properties::PID_HARDWARE_TYPE,
        &[0, 1, 0, 0, 0, 91],
    );
    device.preset_property(
        3,
        knx_core::commissioning::properties::PID_PROGRAM_VERSION,
        &[0, 1, 0x92, 5, 4],
    );
    for (owner, extents, pointer, control) in [
        (
            SimulatedSystem2Object::AddressTable,
            &EXTENTS[..1],
            278,
            false,
        ),
        (
            SimulatedSystem2Object::AssociationTable,
            &EXTENTS[1..2],
            608,
            false,
        ),
        (SimulatedSystem2Object::Application, &EXTENTS[2..], 0, true),
    ] {
        let allocations = extents
            .iter()
            .map(|&(start, length)| {
                device.preset_memory(u32::from(start), &vec![0xa5; usize::from(length)]);
                SimulatedAbsoluteAllocation {
                    subtype: 0,
                    start,
                    length,
                    access: 0x33,
                    memory_type: 3,
                    attributes: 0,
                }
            })
            .collect();
        device.preset_system2_object(
            owner,
            SimulatedSystem2ObjectState {
                load_state: LoadState::Loaded,
                table_reference: pointer,
                allocations,
                task_segment: Some(SimulatedTaskSegment {
                    start: pointer as u16,
                    pei_type: 0,
                    manufacturer: 1,
                    application: 0x9205,
                    version: 4,
                }),
                task_control_2: control.then_some(SimulatedTaskControl2 {
                    callback: 0,
                    communication_objects: 1024,
                    segment_1: 0,
                    segment_2: 0,
                }),
            },
        );
    }
    Arc::new(device)
}

struct TestHardware {
    observation: Arc<super::super::test_observation::Observation>,
    transport_observation: Mutex<TransportObservation>,
    device: Arc<SimulatedDevice>,
    old: SimulatedSystem2Snapshot,
    data: PathBuf,
    connects: AtomicUsize,
    disconnects: AtomicUsize,
    writes: AtomicUsize,
    backup_checks: AtomicUsize,
    checked_connection: AtomicUsize,
    first_backup: Mutex<Option<RecoveryRecord>>,
    pause_data_write: AtomicBool,
    data_written: Notify,
    release_write: Notify,
}
impl TestHardware {
    fn new(data: &Path) -> Arc<Self> {
        let device = old_device();
        Arc::new(Self {
            observation: Arc::new(Default::default()),
            transport_observation: Mutex::new(Default::default()),
            old: device.system2_snapshot(),
            device,
            data: data.into(),
            connects: AtomicUsize::new(0),
            disconnects: AtomicUsize::new(0),
            writes: AtomicUsize::new(0),
            backup_checks: AtomicUsize::new(0),
            checked_connection: AtomicUsize::new(0),
            first_backup: Mutex::new(None),
            pause_data_write: AtomicBool::new(true),
            data_written: Notify::new(),
            release_write: Notify::new(),
        })
    }
    // Observe bytes from the real worker's committed journal while it owns its
    // exclusive OperationStore lock. No second opener, checkpoint injection or
    // progress boolean stands in for HEAD/hash-chain/actual-prestate readback.
    fn check_durable_backup_before_send(&self) {
        let root = self.data.join(super::super::ROOT_NAME);
        let id = std::fs::read_to_string(root.join("selected")).unwrap();
        let operation = root.join(&id);
        let bundle = read_bundle(&operation.join("observed.json")).unwrap();
        assert_eq!(bundle.format, 3);
        assert!(!bundle.binding.simulated);
        assert_eq!(bundle.binding.serial, SERIAL);
        assert_eq!(bundle.binding.target, TARGET);
        assert_eq!(bundle.hardware_preparation.as_ref().unwrap().operation, id);
        let source = bundle.project_preparation.as_ref().unwrap();
        assert_eq!(source.request.program_id, "M-0001_A-9205-04-9270");
        assert_eq!(
            source.request.values.get(PIR_REFERENCE).map(String::as_str),
            Some(PIR_RAW)
        );
        let fields: Vec<_> = source
            .coverage
            .iter()
            .filter(|field| {
                field.parameter_id == "M-0001_A-9205-04-9270_P-900"
                    && field.active_reference.as_deref() == Some(PIR_REFERENCE)
            })
            .collect();
        assert_eq!(fields.len(), 1, "one exact retained configured field");
        assert_eq!(
            fields[0].segment.as_deref(),
            Some("M-0001_A-9205-04-9270_AS-3C00")
        );
        assert_eq!(fields[0].field, Some([250, 0, 8]));
        assert_eq!(fields[0].source_default, "3");
        assert_eq!(fields[0].disposition, "active-overlay");
        let planned: Vec<_> = bundle
            .normalized_plan
            .steps
            .iter()
            .filter_map(|step| {
                if let OperationRecord::WriteMemory { address, data, .. } = &step.operation {
                    PIR_ADDRESS
                        .checked_sub(*address)
                        .and_then(|offset| data.get(usize::from(offset)))
                        .copied()
                } else {
                    None
                }
            })
            .collect();
        // This is the retained NEW plan, not a claim that cancellation reaches
        // this byte: the worker deliberately pauses at its first data write.
        assert_eq!(
            planned,
            [4],
            "exact source-admitted nondefault byte in retained plan"
        );
        if self.first_backup.lock().unwrap().is_none() {
            private_diagnostic(
                "r1-retained-request.json",
                json!({
                    "programId": source.request.program_id,
                    "configurationSha256": source.sha256().unwrap(),
                    "planSha256": bundle.binding.plan_sha256,
                    "parameterValues": source.request.values.len(),
                    "reference": PIR_REFERENCE,
                    "raw": source.request.values.get(PIR_REFERENCE),
                    "field": fields[0].field,
                    "address": PIR_ADDRESS,
                    "planned": planned
                }),
            );
        }
        let journal = operation.join("journal");
        let head: Value =
            serde_json::from_slice(&std::fs::read(journal.join("HEAD")).unwrap()).unwrap();
        assert_eq!(head["format"], 1);
        let sequence = head["sequence"].as_u64().unwrap();
        let mut previous = String::new();
        let mut backups = Vec::new();
        for index in 0..=sequence {
            let bytes = std::fs::read(journal.join(format!("checkpoint-{index:08}.json"))).unwrap();
            let envelope: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(envelope["format"], 1);
            assert_eq!(envelope["kind"], "knxbench-property-validation-protocol");
            assert_eq!(envelope["sequence"], index);
            assert_eq!(envelope["previous_sha256"], previous);
            let checkpoint: Checkpoint =
                serde_json::from_value(envelope["checkpoint"].clone()).unwrap();
            match checkpoint {
                Checkpoint::Created { binding } => assert_eq!(binding, bundle.binding),
                Checkpoint::RecoveryAcquired { recovery } => backups.push(recovery),
                _ => {}
            }
            previous = knx_productdb::sha256_hex(&bytes);
        }
        assert_eq!(head["sha256"], previous);
        assert_eq!(
            backups.len(),
            1,
            "actual backup must be committed before ANY mutation, including VerifyMode"
        );
        let backup = backups.pop().unwrap();
        assert_eq!(backup.original_bundle, bundle);
        assert_eq!(backup.allocations.len(), EXTENTS.len());
        for (region, &(address, length)) in backup.allocations.iter().zip(&EXTENTS) {
            assert_eq!(region.address, address);
            assert_eq!(region.data, vec![0xa5; usize::from(length)]);
        }
        let mut first = self.first_backup.lock().unwrap();
        if let Some(original) = first.as_ref() {
            assert_eq!(
                &backup, original,
                "recovery must retain the ORIGINAL backup"
            );
        } else {
            assert_eq!(
                self.device.system2_snapshot(),
                self.old,
                "backup precedes setup, not just data writes"
            );
            *first = Some(backup);
        }
        self.backup_checks.fetch_add(1, Ordering::SeqCst);
    }
}

// This is explicitly a test HARDWARE provenance adapter, like configured_hardware
// in knx-app. It forwards every frame to independent persistent software state.
// Ordinary simulated connector provenance is neither changed nor admitted.
struct TestHardwareConnector(Arc<TestHardware>);
struct TestHardwareTunnel(Arc<TestHardware>);
#[derive(Default)]
struct TransportObservation {
    entered: usize,
    device_completed: usize,
    device_errors: usize,
    adapter_returned: usize,
    notified: bool,
    last: Option<Value>,
}
impl TestHardware {
    fn transport_entered(
        &self,
        destination: Destination,
        transport: Tpci,
        service: &ApplicationService,
    ) {
        if let Ok(mut observation) = self.transport_observation.try_lock() {
            observation.entered += 1;
            observation.last = Some(json!({
                "connectionOrdinal":self.connects.load(Ordering::SeqCst),
                "targetMatches":matches!(destination, Destination::Individual(address) if address == TARGET.parse::<IndividualAddress>().unwrap()),
                "service":match service {
                    ApplicationService::NoApplicationPdu => "no-application-pdu",
                    ApplicationService::MemoryRead { .. } => "memory-read",
                    ApplicationService::UserMemoryRead { .. } => "user-memory-read",
                    ApplicationService::PropertyValueRead { .. } => "property-read",
                    ApplicationService::DeviceDescriptorRead { .. } => "descriptor-read",
                    ApplicationService::MemoryWrite { .. } => "memory-write",
                    ApplicationService::UserMemoryWrite { .. } => "user-memory-write",
                    ApplicationService::PropertyValueWrite { .. } => "property-write",
                    _ => "other",
                },
                "tpci":match transport {
                    Tpci::UnnumberedData => "unnumbered-data",
                    Tpci::NumberedData { .. } => "numbered-data",
                    Tpci::Connect => "connect", Tpci::Disconnect => "disconnect",
                    Tpci::Ack { .. } => "ack", Tpci::Nak { .. } => "nak", Tpci::Unknown(_) => "unknown",
                },
                "sendEntered":true,"deviceSendReturned":false,"deviceSendSucceeded":false,"adapterReturned":false,"errorClass":Value::Null,
            }));
        }
    }
    fn transport_result(&self, result: &Result<(), BusError>) {
        if let Ok(mut observation) = self.transport_observation.try_lock() {
            if result.is_ok() {
                observation.device_completed += 1;
            } else {
                observation.device_errors += 1;
                observation.adapter_returned += 1;
            }
            if let Some(last) = observation.last.as_mut() {
                last["deviceSendReturned"] = true.into();
                last["deviceSendSucceeded"] = result.is_ok().into();
                last["adapterReturned"] = result.is_err().into();
                last["ioKind"] = match result {
                    Err(BusError::Io(error)) => json!(format!("{:?}", error.kind())),
                    _ => Value::Null,
                };
                last["errorClass"] = match result {
                    Ok(()) => Value::Null,
                    Err(error) => json!(match error {
                        BusError::Io(_) => "io",
                        BusError::Timeout => "timeout",
                        BusError::ConnectionRefused(_) => "connection-refused",
                        BusError::NotImplemented => "not-implemented",
                        BusError::Protocol(_) => "protocol",
                        BusError::NotMulticast(_) => "not-multicast",
                    }),
                };
            }
        }
    }
    fn transport_snapshot(&self) -> Value {
        match self.transport_observation.try_lock() {
            Ok(o) => {
                json!({"entered":o.entered,"deviceSucceeded":o.device_completed,"deviceErrors":o.device_errors,
                "adapterReturned":o.adapter_returned,"dataNotificationReached":o.notified,"last":o.last,
                "connects":self.connects.load(Ordering::SeqCst),"disconnects":self.disconnects.load(Ordering::SeqCst),
                "mutations":self.writes.load(Ordering::SeqCst),"durableBackupChecks":self.backup_checks.load(Ordering::SeqCst)})
            }
            Err(_) => json!({"unavailable":"transport-observation-lock-busy-or-poisoned"}),
        }
    }
}
impl crate::GatewayConnector for TestHardwareConnector {
    #[allow(clippy::type_complexity)]
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Box<dyn crate::BusTunnel>, crate::BusSessionError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async move {
            assert_eq!(gateway, GATEWAY.parse::<SocketAddrV4>().unwrap());
            self.0.connects.fetch_add(1, Ordering::SeqCst);
            Ok(Box::new(TestHardwareTunnel(self.0.clone())) as Box<dyn crate::BusTunnel>)
        })
    }
    #[allow(clippy::type_complexity)]
    fn discover(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Vec<knx_net::DiscoveredGateway>, crate::BusSessionError>>
                + Send
                + '_,
        >,
    > {
        Box::pin(async { panic!("mounted System-2 must not discover a network") })
    }
}
impl crate::BusTunnel for TestHardwareTunnel {
    fn diagnostics_are_simulated(&self) -> bool {
        false
    }
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.0.device.as_ref())
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.0.device.as_ref())
    }
    fn send(
        &self,
        _: Destination,
        _: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), crate::BusSessionError>> + Send + '_>> {
        Box::pin(async { panic!("System-2 must use its actual management-frame worker") })
    }
    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        Box::pin(async move {
            self.0.transport_entered(destination, transport, &service);
            let mutation = matches!(
                &service,
                ApplicationService::MemoryWrite { .. }
                    | ApplicationService::UserMemoryWrite { .. }
                    | ApplicationService::PropertyValueWrite { .. }
            );
            if mutation {
                // Read the complete committed chain at the FIRST mutation of
                // each tunnel (download and explicit original recovery).
                let connection = self.0.connects.load(Ordering::SeqCst);
                if self.0.checked_connection.swap(connection, Ordering::SeqCst) != connection {
                    self.0.check_durable_backup_before_send();
                }
                self.0.writes.fetch_add(1, Ordering::SeqCst);
            }
            let data_write = matches!(
                &service,
                ApplicationService::MemoryWrite { .. } | ApplicationService::UserMemoryWrite { .. }
            );
            let result = ManagementTransport::send_frame(
                self.0.device.as_ref(),
                destination,
                transport,
                service,
            )
            .await;
            self.0.transport_result(&result);
            result?;
            if data_write && self.0.pause_data_write.swap(false, Ordering::SeqCst) {
                if let Ok(mut observation) = self.0.transport_observation.try_lock() {
                    observation.notified = true;
                }
                self.0.data_written.notify_one();
                self.0.release_write.notified().await;
            }
            if let Ok(mut observation) = self.0.transport_observation.try_lock() {
                observation.adapter_returned += 1;
                if let Some(last) = observation.last.as_mut() {
                    last["adapterReturned"] = true.into();
                }
            }
            Ok(())
        })
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), crate::BusSessionError>> + Send>> {
        Box::pin(async move {
            self.0.disconnects.fetch_add(1, Ordering::SeqCst);
            Ok(())
        })
    }
}

async fn http(router: &Router, path: &str, body: Option<Value>, expected: u16) -> Value {
    let mut request = Request::builder()
        .method(if body.is_some() { "POST" } else { "GET" })
        .uri(path);
    let body = if let Some(value) = body {
        request = request.header("Content-Type", "application/json");
        Body::from(serde_json::to_vec(&value).unwrap())
    } else {
        Body::empty()
    };
    let response = router
        .clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status().as_u16(), expected, "{path}");
    // The newly exercised parameter API returns a full source panel, not the
    // small System-2 summary. Keep the existing summary limit unchanged.
    let body_limit = if path.ends_with("/parameters") {
        4 * 1024 * 1024
    } else {
        128 * 1024
    };
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), body_limit)
            .await
            .unwrap(),
    )
    .unwrap()
}
async fn prepare(router: &Router, device: DeviceId) -> Value {
    http(
        router,
        "/api/device-system2/prepare",
        Some(json!({"deviceId":device.0,"sourceMode":"retainedSources"})),
        200,
    )
    .await
}
fn start(preview: &Value) -> Value {
    json!({
        "preparationId":preview["preparationId"], "gateway":GATEWAY,
        "confirmation":preview["confirmationPhrase"],
        "hypothesisConfirmation":preview["hypothesisPhrase"],
        "downloadConfirmation":preview["downloadPhrase"],
        "releaseConfirmation":preview["releasePhrase"],
        "recoveryTrack":"knxbenchBestEffort"
    })
}
async fn settled(state: &crate::SharedState, router: &Router, outcome: &str) -> Value {
    settled_with_timeout(state, router, outcome, || {}).await
}
async fn settled_with_timeout(
    state: &crate::SharedState,
    router: &Router,
    outcome: &str,
    on_timeout: impl FnOnce(),
) -> Value {
    let result = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let finished = state
                .device_system2
                .inner
                .lock()
                .unwrap()
                .current
                .as_ref()
                .and_then(|r| r.task.as_ref())
                .is_some_and(tokio::task::JoinHandle::is_finished);
            if finished {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await;
    if result.is_err() {
        on_timeout();
    }
    result.expect("actual mounted worker must finish and disconnect");
    state.device_system2.join_finished().await;
    let status = http(router, "/api/device-system2/status", None, 200).await;
    assert_eq!(status["state"], outcome);
    status
}
fn state(
    data: &Path,
    products: Option<PathBuf>,
    hardware: &Arc<TestHardware>,
) -> crate::SharedState {
    let mut state = crate::AppState::with_product_db_at(data.into(), products);
    Arc::get_mut(&mut state.device_system2)
        .expect("new fixture exclusively owns its System-2 service")
        .observation = Some(hardware.observation.clone());
    state.connector = Box::new(TestHardwareConnector(hardware.clone()));
    let leases = data.join("gateway-leases");
    std::fs::create_dir_all(&leases).unwrap();
    state.diagnostic_lease_dir = Some(leases);
    state.device_download_timing = SessionTiming {
        connection_timeout: Duration::from_millis(30),
        response_timeout: Duration::from_millis(5),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(30),
        programming_delay: Duration::ZERO,
        ..Default::default()
    };
    Arc::new(state)
}

#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn pinned_public_http_worker_cancellation_reopens_original_once_only_recovery() {
    let package = std::fs::read(
        std::env::var_os("KNXBENCH_PUBLIC_UP258")
            .expect("set KNXBENCH_PUBLIC_UP258 to the existing read-only public fixture"),
    )
    .unwrap();
    assert_eq!(knx_productdb::sha256_hex(&package), PACKAGE_SHA256);
    let directory = tempfile::tempdir().unwrap();
    let products = directory.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&products).unwrap();
    knx_productdb::install_package_with_limits(
        &conn,
        "public-up258.knxprod",
        &package,
        knx_productdb::PackageLimits::LARGE,
    )
    .unwrap();
    drop(conn);
    let hardware = TestHardware::new(directory.path());
    let state = state(directory.path(), Some(products), &hardware);
    let router = crate::app(state.clone(), None);
    http(&router, "/api/project/new", Some(json!({})), 200).await;
    http(
        &router,
        "/api/devices",
        Some(json!({"catalogItemId":CATALOG_ITEM,"name":"Mounted System-2 selected device"})),
        200,
    )
    .await;
    let device = {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        let ids: Vec<_> = project.devices.iter().map(|d| d.id).collect();
        assert_eq!(ids.len(), 1);
        project.devices.get_mut(ids[0]).unwrap().address = Some(TARGET.parse().unwrap());
        ids[0]
    };
    // A newly created catalogue device has no explicit project parameters.
    // Configure the public source's nondefault PIR sensitivity through the real
    // panel write authority; a refusal here must remain a refusal, not a model edit.
    let path = format!("/api/device/{}/parameters", device.0);
    let panel = http(&router, &path, None, 200).await;
    assert_eq!(panel["programId"], "M-0001_A-9205-04-9270");
    let fields = pir_fields(&panel);
    assert_eq!(fields.len(), 1, "one active source PIR field");
    let field = fields[0];
    assert_eq!(field["value"], "3");
    assert_eq!(field["editable"], true);
    assert_eq!(field["writeEtsId"], PIR_REFERENCE);
    assert!(field["enumOptions"]
        .as_array()
        .unwrap()
        .iter()
        .any(|option| option["value"] == PIR_RAW));
    let tree = http(&router, "/api/project", None, 200).await;
    assert!(tree["server_incarnation"].is_string());
    assert!(tree["project_incarnation"].is_u64());
    assert!(tree["snapshot_revision"].is_u64());
    let edited = http(
        &router,
        &path,
        Some(json!({
            "etsId": field["writeEtsId"],
            "raw": PIR_RAW,
            "serverIncarnation": tree["server_incarnation"],
            "projectIncarnation": tree["project_incarnation"],
            "snapshotRevision": tree["snapshot_revision"],
            "expectedValue": field["value"]
        })),
        200,
    )
    .await;
    let fields = pir_fields(&edited);
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0]["value"], PIR_RAW);
    let stored = {
        let project = state.project.lock().unwrap();
        project
            .as_ref()
            .unwrap()
            .installations
            .iter()
            .flat_map(|installation| &installation.parameters)
            .filter(|parameter| parameter.device == device)
            .map(|parameter| (parameter.source.ets_id.clone(), parameter.raw.clone()))
            .collect::<Vec<_>>()
    };
    assert_eq!(stored, [(PIR_REFERENCE.to_string(), PIR_RAW.to_string())]);
    let preview = prepare(&router, device).await;
    private_diagnostic(
        "r1-preparation.json",
        json!({
            "programId": preview["programId"],
            "configurationSha256": preview["configurationSha256"],
            "planSha256": preview["planSha256"],
            "parameterValues": preview["parameterValues"],
            "stored": stored
        }),
    );
    assert_eq!(preview["address"], TARGET);
    assert_eq!(preview["programId"], "M-0001_A-9205-04-9270");
    assert!(preview["parameterValues"].as_u64().unwrap() > 0);
    assert_eq!(
        hardware.connects.load(Ordering::SeqCst),
        0,
        "prepare is offline"
    );
    for field in [
        "confirmation",
        "hypothesisConfirmation",
        "downloadConfirmation",
        "releaseConfirmation",
    ] {
        let mut wrong = start(&preview);
        wrong[field] = format!("{} ", wrong[field].as_str().unwrap()).into();
        let refusal = http(&router, "/api/device-system2/start", Some(wrong), 409).await;
        assert_eq!(refusal["kind"], "system2ConfirmationsRequired");
    }
    let mut wrong_target = start(&preview);
    wrong_target["downloadConfirmation"] =
        knx_core::commissioning::mutation::required_confirmation_phrase(
            "1.1.26".parse().unwrap(),
            knx_core::commissioning::mutation::WriteScope::Download,
        )
        .into();
    assert_eq!(
        http(
            &router,
            "/api/device-system2/start",
            Some(wrong_target),
            409
        )
        .await["kind"],
        "system2ConfirmationsRequired"
    );
    // Even drift that has not bumped the adapter revision must fail the real
    // retained-source/current-project check, not merely the HTTP token check.
    {
        let mut project = state.project.lock().unwrap();
        project
            .as_mut()
            .unwrap()
            .devices
            .get_mut(device)
            .unwrap()
            .name
            .push_str(" changed");
    }
    assert_eq!(
        http(
            &router,
            "/api/device-system2/start",
            Some(start(&preview)),
            409
        )
        .await["kind"],
        "system2PreparationStale"
    );
    assert_eq!(hardware.connects.load(Ordering::SeqCst), 0);
    assert_eq!(hardware.writes.load(Ordering::SeqCst), 0);
    assert_eq!(hardware.device.system2_snapshot(), hardware.old);

    // An addressed but WRONG physical identity reaches the actual observation
    // worker, then refuses before setup, backup creation or any mutation.
    hardware.device.preset_property(
        0,
        knx_core::commissioning::properties::PID_HARDWARE_TYPE,
        &[0, 1, 0, 0, 0, 92],
    );
    let wrong_old = hardware.device.system2_snapshot();
    let wrong_preview = prepare(&router, device).await;
    http(
        &router,
        "/api/device-system2/start",
        Some(start(&wrong_preview)),
        200,
    )
    .await;
    let wrong_status = settled(&state, &router, "failed").await;
    assert_eq!(wrong_status["backupAvailable"], false);
    assert_eq!(hardware.connects.load(Ordering::SeqCst), 1);
    assert_eq!(hardware.disconnects.load(Ordering::SeqCst), 1);
    assert_eq!(hardware.writes.load(Ordering::SeqCst), 0);
    assert_eq!(hardware.device.system2_snapshot(), wrong_old);
    hardware.device.preset_property(
        0,
        knx_core::commissioning::properties::PID_HARDWARE_TYPE,
        &[0, 1, 0, 0, 0, 91],
    );

    let preview = prepare(&router, device).await;
    let accepted = http(
        &router,
        "/api/device-system2/start",
        Some(start(&preview)),
        200,
    )
    .await;
    let operation_id = accepted["operationId"].as_str().unwrap().to_owned();
    assert_eq!(operation_id, preview["preparationId"]);
    assert_eq!(accepted["state"], "running");
    let data_written =
        tokio::time::timeout(Duration::from_secs(10), hardware.data_written.notified()).await;
    if data_written.is_err() {
        // Retain actual worker/transport/committed evidence BEFORE panic/TempDir
        // teardown. Do not await the worker, release the pause or extend its budget.
        super::super::test_observation::retain_timeout(
            &state,
            &hardware.observation,
            directory.path(),
            &operation_id,
            &preview,
            hardware.transport_snapshot(),
        );
    }
    data_written.expect("worker must reach an actual data write after durable backup");
    let running = http(&router, "/api/device-system2/status", None, 200).await;
    assert_eq!(running["operationId"], operation_id);
    assert_eq!(running["state"], "running");
    assert_eq!(running["backupAvailable"], true);
    assert!(
        running["recoveryPhrase"].is_null(),
        "no recovery while the real task owns transport"
    );
    assert!(hardware.backup_checks.load(Ordering::SeqCst) > 0);
    assert_ne!(hardware.device.system2_snapshot(), hardware.old);
    assert_eq!(
        http(
            &router,
            "/api/device-system2/cancel",
            Some(json!({"operationId":"ffffffffffffffffffffffffffffffff"})),
            409
        )
        .await["kind"],
        "system2OperationUnknown"
    );
    let cancel = http(
        &router,
        "/api/device-system2/cancel",
        Some(json!({"operationId":operation_id})),
        200,
    )
    .await;
    assert_eq!(cancel["operationId"], operation_id);
    hardware.release_write.notify_one();
    let cancelled = settled(&state, &router, "cancelled").await;
    assert_eq!(hardware.connects.load(Ordering::SeqCst), 2);
    assert_eq!(hardware.disconnects.load(Ordering::SeqCst), 2);
    assert_ne!(
        hardware.device.system2_snapshot(),
        hardware.old,
        "cancel is not rollback"
    );
    let interrupted = hardware.device.system2_snapshot();
    let operation = directory
        .path()
        .join(super::super::ROOT_NAME)
        .join(&operation_id);
    let bundle = read_bundle(&operation.join("observed.json")).unwrap();
    let original = hardware.first_backup.lock().unwrap().clone().unwrap();
    let reopened = OperationStore::open(&operation.join("journal"), &bundle.binding).unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Cancelled);
    assert_eq!(reopened.recovery(), Some(&original));
    drop(reopened);
    state.catalogue_jobs.close_admission_and_request_stop();
    assert_eq!(
        state
            .catalogue_jobs
            .drain(Duration::from_secs(5), std::future::pending())
            .await
            .end,
        crate::catalogue_jobs::DrainEnd::Finished
    );
    drop(router);
    drop(state);

    // New server incarnation, no project and no catalogue. Only the ORIGINAL
    // selected disk evidence supplies the target, backup and acknowledgement.
    let fresh = self::state(directory.path(), None, &hardware);
    assert!(fresh.project.lock().unwrap().is_none());
    assert!(fresh.product_db.is_none());
    let router = crate::app(fresh.clone(), None);
    let status = http(&router, "/api/device-system2/status", None, 200).await;
    assert_eq!(status["operationId"], operation_id);
    assert_eq!(status["binding"], cancelled["binding"]);
    assert_ne!(
        status["binding"]["serverIncarnation"],
        fresh.server_incarnation
    );
    assert_eq!(status["address"], TARGET);
    assert_eq!(status["backupAvailable"], true);
    for field in ["recoveryPhrase", "downloadPhrase", "releasePhrase"] {
        assert_eq!(status[field], cancelled[field]);
        assert!(status[field].as_str().is_some());
    }
    let recovery = json!({
        "operationId":operation_id, "gateway":GATEWAY,
        "recoveryConfirmation":status["recoveryPhrase"],
        "downloadConfirmation":status["downloadPhrase"],
        "releaseConfirmation":status["releasePhrase"]
    });
    for field in [
        "recoveryConfirmation",
        "downloadConfirmation",
        "releaseConfirmation",
    ] {
        let mut wrong = recovery.clone();
        wrong[field] = format!("{} ", wrong[field].as_str().unwrap()).into();
        assert_eq!(
            http(&router, "/api/device-system2/recover", Some(wrong), 409).await["kind"],
            "system2RecoveryRefused"
        );
    }
    assert!(
        !operation.join("recovery-command").exists(),
        "refused acknowledgements cannot consume recovery"
    );
    assert_eq!(hardware.connects.load(Ordering::SeqCst), 2);
    assert_eq!(hardware.device.system2_snapshot(), interrupted);
    // Fixture-wide counters include the wrong-identity control and download.
    // Keep their pre-correct-recovery baseline, never label them recovery-only.
    let recovery_transport_baseline = hardware.transport_snapshot();
    let restoring = http(
        &router,
        "/api/device-system2/recover",
        Some(recovery.clone()),
        200,
    )
    .await;
    assert_eq!(restoring["operationId"], operation_id);
    let restored = settled_with_timeout(&fresh, &router, "restored", || {
        super::super::test_observation::retain_recovery_timeout(
            &fresh,
            &hardware.observation,
            directory.path(),
            &operation_id,
            &bundle,
            (&status, &restoring),
            json!({
                "counterScope":"cumulative-fixture; not-recovery-only",
                "baselineBoundary":"before-correct-recovery-http-admission",
                "baseline":recovery_transport_baseline,
                "atTimeout":hardware.transport_snapshot(),
                "firstDataWritePauseArmed":hardware.pause_data_write.load(Ordering::SeqCst),
                "bestEffortNonAtomicObservation":true,
            }),
        );
    })
    .await;
    assert_eq!(restored["backupAvailable"], true);
    assert!(restored["recoveryPhrase"].is_null());
    assert_eq!(hardware.device.system2_snapshot(), hardware.old);
    assert_eq!(hardware.connects.load(Ordering::SeqCst), 3);
    assert_eq!(hardware.disconnects.load(Ordering::SeqCst), 3);
    assert_eq!(
        hardware.backup_checks.load(Ordering::SeqCst),
        2,
        "both workers must read the same durable original before their first mutation"
    );
    assert_eq!(
        std::fs::read_to_string(operation.join("recovery-command")).unwrap(),
        operation_id
    );
    let writes = hardware.writes.load(Ordering::SeqCst);
    assert_eq!(
        http(
            &router,
            "/api/device-system2/recover",
            Some(recovery.clone()),
            409
        )
        .await["kind"],
        "system2RecoveryConsumed"
    );
    let reopened = OperationStore::open(&operation.join("journal"), &bundle.binding).unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    assert_eq!(reopened.recovery(), Some(&original));
    drop(reopened);
    drop(router);
    drop(fresh);
    let restarted = self::state(directory.path(), None, &hardware);
    let router = crate::app(restarted, None);
    assert_eq!(
        http(&router, "/api/device-system2/status", None, 200).await["state"],
        "restored"
    );
    assert_eq!(
        http(&router, "/api/device-system2/recover", Some(recovery), 409).await["kind"],
        "system2RecoveryConsumed"
    );
    assert_eq!(hardware.connects.load(Ordering::SeqCst), 3);
    assert_eq!(hardware.writes.load(Ordering::SeqCst), writes);
    assert_eq!(hardware.device.system2_snapshot(), hardware.old);
}

// Issue #30: the REAL owning download worker, called with the refusals that end it
// before any journal or tunnel exists. Opt-in public package, no network or hardware.
fn evidence_tree(data: &Path) -> Vec<(String, String)> {
    fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            let relative = path.strip_prefix(base).unwrap().display().to_string();
            if path.is_dir() {
                out.push((relative, "dir".into()));
                walk(base, &path, out);
            } else {
                out.push((
                    relative,
                    knx_productdb::sha256_hex(&std::fs::read(&path).unwrap()),
                ));
            }
        }
    }
    let mut out = Vec::new();
    walk(data, &data.join(super::super::ROOT_NAME), &mut out);
    out.sort();
    out
}
struct Refusal {
    _directory: tempfile::TempDir,
    data: PathBuf,
    state: crate::SharedState,
    router: Router,
    hardware: Arc<TestHardware>,
    device: DeviceId,
    start: super::super::dto::Start,
}
async fn refusal_fixture(activity_unavailable: bool) -> Refusal {
    let package = std::fs::read(
        std::env::var_os("KNXBENCH_PUBLIC_UP258")
            .expect("set KNXBENCH_PUBLIC_UP258 to the existing read-only public fixture"),
    )
    .unwrap();
    assert_eq!(knx_productdb::sha256_hex(&package), PACKAGE_SHA256);
    let directory = tempfile::tempdir().unwrap();
    let products = directory.path().join("products.sqlite");
    let conn = knx_productdb::open_and_migrate(&products).unwrap();
    knx_productdb::install_package_with_limits(
        &conn,
        "public-up258.knxprod",
        &package,
        knx_productdb::PackageLimits::LARGE,
    )
    .unwrap();
    drop(conn);
    let hardware = TestHardware::new(directory.path());
    let mut state = state(directory.path(), Some(products), &hardware);
    if activity_unavailable {
        // No history path: the log refuses write admission, as a failed store does.
        Arc::get_mut(&mut state).unwrap().one_shot_activity = Arc::default();
    }
    let router = crate::app(state.clone(), None);
    http(&router, "/api/project/new", Some(json!({})), 200).await;
    http(
        &router,
        "/api/devices",
        Some(json!({"catalogItemId":CATALOG_ITEM,"name":"Refusal selected device"})),
        200,
    )
    .await;
    let device = {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        let id = project.devices.iter().map(|d| d.id).next().unwrap();
        project.devices.get_mut(id).unwrap().address = Some(TARGET.parse().unwrap());
        id
    };
    let preview = prepare(&router, device).await;
    let start = super::super::dto::decode(&serde_json::to_vec(&start(&preview)).unwrap()).unwrap();
    // A different, older operation stands in for retained original evidence.
    let root = state.device_system2.root(&state).unwrap();
    root.create(&super::metadata()).unwrap();
    Refusal {
        data: directory.path().into(),
        _directory: directory,
        state,
        router,
        hardware,
        device,
        start,
    }
}
impl Refusal {
    /// Mirrors the start route after admission, then awaits the real worker.
    async fn run(&self) -> Value {
        let preparation = self
            .state
            .device_system2
            .inner
            .lock()
            .unwrap()
            .preparation
            .clone()
            .unwrap();
        let meta = preparation.metadata.clone();
        let lease = self
            .state
            .device_system2
            .control
            .try_admit(&meta.operation_id)
            .unwrap();
        self.state.device_system2.inner.lock().unwrap().current = Some(super::super::Record {
            metadata: meta.clone(),
            progress: super::super::initial(LocalOutcome::Running),
            updates: None,
            task: None,
            control_id: meta.operation_id.clone(),
            message: None,
            refusal: None,
        });
        let root = self.state.device_system2.root(&self.state).unwrap();
        let body = super::super::dto::decode(
            &serde_json::to_vec(&json!({
                "preparationId": self.start.preparation_id,
                "gateway": GATEWAY,
                "confirmation": self.start.confirmation,
                "hypothesisConfirmation": self.start.hypothesis_confirmation,
                "downloadConfirmation": self.start.download_confirmation,
                "releaseConfirmation": self.start.release_confirmation,
                "recoveryTrack": "knxbenchBestEffort"
            }))
            .unwrap(),
        )
        .unwrap();
        super::super::worker::download(
            self.state.clone(),
            root,
            preparation,
            body,
            GATEWAY.parse().unwrap(),
            lease,
        )
        .await;
        http(&self.router, "/api/device-system2/status", None, 200).await
    }
    async fn refused_without_effect(&self, kind: &str) {
        let before = evidence_tree(&self.data);
        let status = self.run().await;
        assert_eq!(status["state"], "failed", "{kind}");
        assert_eq!(status["refusal"], kind);
        assert_eq!(status["mutationMayHaveStarted"], false);
        assert_eq!(status["backupAvailable"], false);
        assert_eq!(self.hardware.connects.load(Ordering::SeqCst), 0, "{kind}");
        assert_eq!(self.hardware.writes.load(Ordering::SeqCst), 0, "{kind}");
        assert_eq!(self.hardware.transport_snapshot()["entered"], 0, "{kind}");
        assert_eq!(
            evidence_tree(&self.data),
            before,
            "{kind}: evidence changed"
        );
        assert_eq!(self.hardware.device.system2_snapshot(), self.hardware.old);
        assert!(matches!(
            self.state.device_system2.control.snapshot().settlement,
            knx_app::property_validation::System2Settlement::Settled {
                outcome: LocalOutcome::Failed,
                ..
            }
        ));
    }
}

// One fixture per case: a settled operation id is never admitted twice.
#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn worker_shared_lock_refusal_reaches_status_without_touching_the_bus() {
    let fixture = refusal_fixture(false).await;
    // Another route holds a shared tunnel lock when the worker reserves.
    let _held = fixture.state.device_download.lock().await;
    fixture.refused_without_effect("gatewayBusy").await;
}

#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn worker_gateway_lease_refusal_reaches_status_without_touching_the_bus() {
    let fixture = refusal_fixture(false).await;
    // A different holder owns the cross-process gateway lease.
    let directory = fixture.state.diagnostic_lease_dir.clone().unwrap();
    let _foreign = knx_app::device_diagnostics::lease_in(&directory).unwrap();
    fixture.refused_without_effect("gatewayBusy").await;
}

#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn worker_source_recheck_refusal_reaches_status_without_touching_the_bus() {
    let fixture = refusal_fixture(false).await;
    // The source drifts after the route's own check; the worker rechecks it.
    fixture
        .state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .devices
        .get_mut(fixture.device)
        .unwrap()
        .name
        .push_str(" changed");
    fixture
        .refused_without_effect("system2PreparationStale")
        .await;
}

#[tokio::test]
#[ignore = "requires digest-pinned public package; set KNXBENCH_PUBLIC_UP258; no network or hardware"]
async fn worker_activity_refusal_reaches_status_without_touching_the_bus() {
    refusal_fixture(true)
        .await
        .refused_without_effect("system2ActivityUnavailable")
        .await;
}
