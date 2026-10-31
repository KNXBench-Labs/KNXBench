//! A real System-2 allocation mismatch reaches the bundle; neither byte side reaches any ZIP entry.
//!
//! DIAG-03 Original14 X2 (with S1/S2): the App executor runs the configured synthetic
//! project against an independent OLD simulator over a hardware-kind transport. The
//! final allocation read answers distinct sentinel octets, so the journal records both
//! the expected side (captured prestate) and the actual side. The same exchange, byte
//! for byte, becomes System-2 recordings. Bundle preview, section texts and every ZIP
//! entry must carry neither side in any encoding; the operation still arrives as its
//! closed, metadata-only projection and typed report.

use super::{fixture, post, preview, section, section_text};
use axum::body::to_bytes;
use knx_app::operation_recording::{OperationKind, OperationStart, RecordedOutcome};
use knx_app::system2_validation_preparation::{
    admit_current_project_hardware, execute_project_hardware, observe_project_hardware,
    prepare_retained_project_hardware_source, HardwareConfirmations, RecoveryTrack,
};
use knx_core::commissioning::mutation::{required_confirmation_phrase, TargetKind, WriteScope};
use knx_core::IndividualAddress;
use knx_net::{
    cemi::{self, ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci},
    client::{BusError, TunnelEvent},
    commissioning::{hardware_release::required_release_phrase, simulator, SessionTiming},
    management::ManagementTransport,
    wire_tap::WireDirection,
};
use serde_json::{json, Value};
use std::{
    io::Read,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::broadcast;

#[path = "../../../../../crates/knx-app/tests/it/system2_validation_preparation/old_device.rs"]
mod old_device;
#[path = "../../../../../crates/knx-app/src/sentinel_forms_tests.rs"]
mod sentinel_forms;

/// The address table allocation (16 octets); the configured plan writes offsets 0..7.
const ALLOCATION: u16 = 278;
/// What the plan writes there: only the expected side keeps all of it, once the final
/// read lies over its head.
const TEMPLATE: [u8; 7] = [0x03, 0x12, 0x34, 0x10, 0x35, 0x10, 0x36];
/// Captured, unwritten prestate at offsets 8..12: both sides hold it. Memory is read in
/// 12-octet pieces, so it lies inside one response and a recording holds it contiguously.
const PRESTATE: [u8; 4] = [0x5a, 0xc3, 0x91, 0x7e];
const PRESTATE_OFFSET: u16 = 8;
/// What the final read answers at offsets 0..4: only the actual side holds it.
const READBACK: [u8; 4] = [0xd7, 0x3e, 0x9c, 0x6b];
const SENTINELS: [&[u8]; 3] = [&TEMPLATE, &PRESTATE, &READBACK];

/// Hardware-kind transport over the simulator that keeps every exchanged frame as a
/// tunnel datagram and, after the plan's last LoadCompleted, lies on memory reads.
struct Lying<'a> {
    device: &'a simulator::SimulatedDevice,
    received: Mutex<broadcast::Receiver<TunnelEvent>>,
    datagrams: Mutex<Vec<(WireDirection, Vec<u8>)>>,
    completions: AtomicUsize,
    last_completion: usize,
}
impl<'a> Lying<'a> {
    fn new(device: &'a simulator::SimulatedDevice, last_completion: usize) -> Self {
        Self {
            device,
            received: Mutex::new(ManagementTransport::subscribe(device)),
            datagrams: Mutex::new(Vec::new()),
            completions: AtomicUsize::new(0),
            last_completion,
        }
    }
    fn keep(&self, direction: WireDirection, frame: &LDataFrame) {
        let datagram = knx_net::frame::encode_frame(
            knx_net::tunnelling::TUNNELLING_REQUEST,
            &knx_net::tunnelling::encode_tunnelling_request(
                1,
                0,
                &cemi::encode_l_data(frame).unwrap(),
            ),
        );
        self.datagrams.lock().unwrap().push((direction, datagram));
    }
    /// Keeps every telegram the device answered so far; a lag would lose one, so it fails.
    fn drain(&self) {
        let mut received = self.received.lock().unwrap();
        loop {
            match received.try_recv() {
                Ok(TunnelEvent::Telegram(frame)) => self.keep(WireDirection::Received, &frame),
                Ok(TunnelEvent::Closed) => {}
                Err(broadcast::error::TryRecvError::Empty) => return,
                Err(error) => panic!("the recording would miss a telegram: {error:?}"),
            }
        }
    }
    fn datagrams(&self) -> Vec<(WireDirection, Vec<u8>)> {
        self.drain();
        self.datagrams.lock().unwrap().clone()
    }
}
impl ManagementTransport for Lying<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device)
    }
    fn target_kind(&self) -> TargetKind {
        TargetKind::Hardware
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        self.drain();
        match &service {
            ApplicationService::PropertyValueWrite {
                property_id, data, ..
            } if *property_id == 5 && data.first() == Some(&2) => {
                self.completions.fetch_add(1, Ordering::SeqCst);
            }
            ApplicationService::MemoryRead { .. }
                if self.completions.load(Ordering::SeqCst) >= self.last_completion =>
            {
                self.device.preset_memory(u32::from(ALLOCATION), &READBACK);
            }
            _ => {}
        }
        self.keep(
            WireDirection::Sent,
            &LDataFrame {
                kind: LDataMessageKind::Request,
                source: self.assigned_address(),
                destination,
                transport,
                service: service.clone(),
                control: None,
            },
        );
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

fn timing() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(30),
        response_timeout: Duration::from_millis(5),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(30),
        programming_delay: Duration::ZERO,
        ..Default::default()
    }
}

struct Mismatch {
    _dir: tempfile::TempDir,
    state: Arc<crate::AppState>,
    operation_id: String,
    private_bytes: Vec<u8>,
    recordings: Vec<String>,
}

/// One real failed download on disk under the server's System-2 root, plus (when
/// enabled) two System-2 recordings holding its exact exchange.
async fn real_mismatch(recordings_enabled: bool) -> Mismatch {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(
            dir.path(),
            !recordings_enabled,
        )),
        ..crate::AppState::new(dir.path().into())
    });
    let (_inputs, products, project, _profile) = fixture::configured_inputs();
    let mut metadata = super::super::metadata();
    let context = crate::diagnostic_bundle_context::ProjectContext::capture(&state);
    metadata.binding.server_incarnation = context.server_incarnation;
    metadata.binding.project_incarnation = context.project_incarnation.to_string();
    metadata.binding.project_revision = context.revision.to_string();
    let source = prepare_retained_project_hardware_source(
        &products,
        &project,
        knx_core::DeviceId(1),
        &metadata.operation_id,
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    // Counted on the untouched preview: the serial below only names the simulator.
    let last_completion = source
        .preview()
        .plan()
        .unwrap()
        .steps()
        .iter()
        .filter(|step| matches!(&step.operation,
            knx_core::commissioning::property_download::PropertyOperation::LoadControl { payload, .. }
                if payload.event_octet() == 2))
        .count();
    let mut identity = source.preview().clone();
    identity.binding.serial = [0, 1, 2, 3, 4, 5];
    let device = old_device::synthetic_old_device(&identity, Default::default());
    old_device::configure_old_device(&device);
    device.preset_memory(u32::from(ALLOCATION + PRESTATE_OFFSET), &PRESTATE);
    let transport = Lying::new(&device, last_completion);
    let cancel = AtomicBool::new(false);
    let bundle = observe_project_hardware(&transport, source, timing(), &cancel)
        .await
        .unwrap();
    let admission =
        admit_current_project_hardware(&products, &project, knx_core::DeviceId(1), &bundle)
            .unwrap();
    metadata.address = bundle.binding.target.clone();
    let root = state.device_system2.root(&state).unwrap();
    root.create(&metadata).unwrap();
    let mut store = root.observed(&metadata.operation_id, &bundle).unwrap();
    let (operation, hypothesis) = (bundle.consent_phrase(), bundle.acceptance_phrase());
    let download = required_confirmation_phrase(device.address(), WriteScope::Download);
    let risk = required_release_phrase(device.address());
    let error = execute_project_hardware(
        &transport,
        &admission,
        &mut store,
        timing(),
        &HardwareConfirmations {
            operation: &operation,
            hypothesis: &hypothesis,
            download: &download,
            risk: &risk,
        },
        &cancel,
    )
    .await
    .expect_err("the lying final read must fail the download");
    drop(store);

    // Preconditions: the journal holds both sides, the typed failure is the mismatch.
    let journal = dir
        .path()
        .join(super::super::super::ROOT_NAME)
        .join(&metadata.operation_id)
        .join("journal");
    let evidence = allocation_evidence(&journal);
    eprintln!("[fixture] error={error} evidence={evidence}");
    assert_eq!(
        evidence["address"], ALLOCATION,
        "fixture: the address table must fail"
    );
    let (expected, actual) = (
        evidence["expected"].as_str().unwrap(),
        evidence["actual"].as_str().unwrap(),
    );
    let hex = sentinel_forms::hex;
    assert!(expected.contains(&hex(&TEMPLATE)) && !actual.contains(&hex(&TEMPLATE)));
    assert!(actual.contains(&hex(&READBACK)) && !expected.contains(&hex(&READBACK)));
    assert!(expected.contains(&hex(&PRESTATE)) && actual.contains(&hex(&PRESTATE)));
    let admitted = root
        .bundle_admission(&metadata.operation_id)
        .unwrap()
        .unwrap();
    let progress = serde_json::to_value(&admitted.progress).unwrap();
    eprintln!("[fixture] error={error} progress={progress}");
    assert_eq!(
        progress["final_step_failures"],
        json!([{"run": "download", "category": "verificationMismatch", "stage": "allocation"}]),
        "fixture: the final allocation verification must be what failed"
    );
    let private_bytes = admitted.projection.bytes().to_vec();

    let datagrams = transport.datagrams();
    let mut recordings = Vec::new();
    if recordings_enabled {
        for edge in ["history", "current"] {
            let recorder = crate::operation_recordings::start(
                &state,
                OperationStart::new(OperationKind::System2Download)
                    .target(json!({
                        "deviceId": metadata.binding.device_id,
                        "individualAddress": metadata.address,
                    }))
                    .gateway("203.0.113.250:3671")
                    .related("operationId", &metadata.operation_id)
                    .related("consumerEdge", edge),
            );
            let tap = recorder.wire_tap().unwrap();
            for (direction, datagram) in &datagrams {
                tap.datagram(*direction, datagram);
            }
            recorder.finish(RecordedOutcome::from_error(&error));
            assert!(state.recordings.wait_until_idle(Duration::from_secs(10)));
            std::thread::sleep(Duration::from_millis(3));
            let id = recorder.id().unwrap().to_string();
            let raw = state.recordings.read(&id).unwrap();
            for side in SENTINELS {
                assert!(
                    sentinel_forms::forbidden_forms(side)
                        .iter()
                        .any(|form| raw.windows(form.len()).any(|w| w == form.as_slice())),
                    "fixture: recording {id} must hold {}",
                    sentinel_forms::hex(side)
                );
            }
            recordings.push(id);
        }
    }
    Mismatch {
        _dir: dir,
        state,
        operation_id: metadata.operation_id,
        private_bytes,
        recordings,
    }
}

fn assert_clean(label: &str, bytes: &[u8]) {
    sentinel_forms::assert_no_sentinel_bytes(label, bytes, &SENTINELS);
}

/// The private allocation evidence of the journal's last checkpoint.
fn allocation_evidence(journal: &std::path::Path) -> Value {
    let mut names: Vec<_> = std::fs::read_dir(journal)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("checkpoint-")
        })
        .collect();
    names.sort();
    let last: Value =
        serde_json::from_slice(&std::fs::read(names.last().unwrap()).unwrap()).unwrap();
    last["checkpoint"]["allocation_evidence"].clone()
}

/// Preview, every section text and every ZIP entry; returns the archive.
async fn export_clean(
    fixture: &Mismatch,
    mut query: Value,
    sections: &[(&str, Option<&str>)],
) -> zip::ZipArchive<std::io::Cursor<Vec<u8>>> {
    let shown = preview(&fixture.state, &query).await;
    assert_clean("preview", shown.to_string().as_bytes());
    query["review"] = shown["review"].clone();
    for &(kind, label) in sections {
        let selected = section(&shown, kind, label);
        assert_eq!(selected["files"], json!(["system2-private.json"]), "{kind}");
        assert_eq!(selected["bytes"], fixture.private_bytes.len() as u64);
        let text = section_text(&fixture.state, &query, selected["key"].as_str().unwrap()).await;
        assert_clean(&format!("section text {kind}"), text.to_string().as_bytes());
    }
    query["consent"] = json!(true);
    let response = post(&fixture.state, "/api/diagnostics/bundle", query).await;
    assert_eq!(response.status(), 200);
    let disposition = response
        .headers()
        .get("content-disposition")
        .map(|value| value.as_bytes().to_vec())
        .unwrap_or_default();
    assert_clean("content-disposition", &disposition);
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap()
        .to_vec();
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert!(archive.len() > 2, "the bundle holds its sections");
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().to_owned();
        assert!(
            !name.ends_with("recording.jsonl"),
            "{name} is a raw recording"
        );
        assert_clean(&format!("entry name {name}"), name.as_bytes());
        let mut content = Vec::new();
        entry.read_to_end(&mut content).unwrap();
        assert_clean(&format!("entry {name}"), &content);
    }
    archive
}

fn entry(archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>, name: &str) -> Vec<u8> {
    let mut bytes = Vec::new();
    archive
        .by_name(name)
        .unwrap_or_else(|_| panic!("missing {name}"))
        .read_to_end(&mut bytes)
        .unwrap();
    bytes
}

fn assert_typed_mismatch(archive: &mut zip::ZipArchive<std::io::Cursor<Vec<u8>>>) {
    let report: Value =
        serde_json::from_slice(&entry(archive, "reports/system2-report.json")).unwrap();
    assert_eq!(
        report["finalStepFailures"],
        json!([{"run": "download", "category": "verificationMismatch", "stage": "allocation"}])
    );
}

#[tokio::test]
async fn a_real_mismatch_without_a_recording_exports_neither_byte_side() {
    let fixture = real_mismatch(false).await;
    let projection: Value = serde_json::from_slice(&fixture.private_bytes).unwrap();
    assert_eq!(
        projection["runs"][0]["allocationMismatch"]["classification"],
        "recordedBytesWithheld"
    );
    let query = json!({
        "problemType": "deviceProgramming",
        "operation": {"relatedKey": "operationId", "relatedValue": fixture.operation_id},
        "sections": ["operation", "legacyReports"],
    });
    let mut archive = export_clean(&fixture, query, &[("operation", None)]).await;
    assert_eq!(
        entry(&mut archive, "operations/system2-private.json"),
        fixture.private_bytes
    );
    assert_typed_mismatch(&mut archive);
}

#[tokio::test]
async fn real_mismatch_recordings_and_history_export_neither_byte_side() {
    let fixture = real_mismatch(true).await;
    let [earlier, current] = [&fixture.recordings[0], &fixture.recordings[1]];
    let query = json!({
        "problemType": "deviceProgramming",
        "operation": {"recordingId": current},
        "earlierRecordingIds": [earlier],
        "sections": ["operation", "earlierOperation", "legacyReports"],
    });
    let mut archive = export_clean(
        &fixture,
        query,
        &[
            ("operation", Some(current.as_str())),
            ("earlierOperation", Some(earlier.as_str())),
        ],
    )
    .await;
    for id in [current, earlier] {
        assert_eq!(
            entry(
                &mut archive,
                &format!("operations/{id}/system2-private.json")
            ),
            fixture.private_bytes
        );
    }
    assert_typed_mismatch(&mut archive);
}
