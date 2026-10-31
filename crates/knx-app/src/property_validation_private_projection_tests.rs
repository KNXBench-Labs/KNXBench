//! Real Net producer, journal and fresh replay feed the private projection; sentinels never leak.
use super::*;
use crate::property_validation::{
    self as app, Checkpoint, FinalStepCategory, FinalStepRecord, FinalStepStage, LocalOutcome,
    OperationStore,
};
use crate::property_validation_bundle::{PlanRecord, PreparedBundle};
use knx_core::commissioning::{
    absolute_load_control::{absolute_data, AbsolutePropertySegment},
    authorisation::AuthorisationPlan,
    load_control::event_payload,
    load_control_memory::SegmentMemoryType,
    load_state::{LoadEvent, LoadState, MaskVersion},
    mutation::{required_confirmation_phrase, TargetKind, WriteAuthorisation, WriteScope},
    properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_PROGRAM_VERSION},
    property_download::{PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation as Op},
};
use knx_core::IndividualAddress;
use knx_net::{
    cemi::{ApplicationService, Destination, Tpci},
    client::{BusError, TunnelEvent},
    commissioning::{
        hardware_release::required_release_phrase,
        property_download::run_simulated_property_plan,
        simulator::{SimulatedDevice, SimulatorConfig},
        SessionTiming,
    },
    management::ManagementTransport,
};
use serde_json::{json, Value};
use std::{
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};
use tokio::sync::broadcast;

#[path = "property_validation_private_projection_tests_more.rs"]
mod more;

const SERIAL: [u8; 6] = [0, 2, 1, 2, 3, 4];
const VERSION: [u8; 5] = [0, 2, 0x12, 0x34, 1];
/// Allocation 3C00h..3C08h as the device holds it before the download (the captured prestate).
pub(super) const PRESTATE: [u8; 8] = [0x5a, 0xc3, 0x91, 0x7e, 0x2d, 0xb8, 0x46, 0xf0];
/// What the plan writes at 3C02h (offsets 2..6): the written/template side.
pub(super) const TEMPLATE: [u8; 4] = [0x1b, 0x4d, 0x8f, 0xe2];
/// Unexpected final readbacks: in the unwritten tail, in the written head, during restore.
pub(super) const READ_UNWRITTEN: (u16, [u8; 2]) = (6, [0xd7, 0x3e]);
pub(super) const READ_WRITTEN: (u16, [u8; 2]) = (2, [0x9c, 0x6b]);
pub(super) const READ_RESTORE: (u16, [u8; 2]) = (3, [0x64, 0xa9]);

pub(super) fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
/// Every sentinel byte run the projection must never carry, in all encodings a lazy
/// serializer might pick: hex in both cases and separators, `0x`, decimal lists, base64
/// (every alignment, interior characters only) and the raw octets.
pub(super) fn forbidden_forms(bytes: &[u8]) -> Vec<Vec<u8>> {
    let join = |sep: &str, f: &dyn Fn(u8) -> String| {
        bytes.iter().map(|b| f(*b)).collect::<Vec<_>>().join(sep)
    };
    let mut forms: Vec<String> = vec![hex(bytes), hex(bytes).to_uppercase()];
    for sep in [" ", ":", "-", ",", ", ", "."] {
        forms.push(join(sep, &|b| format!("{b:02x}")));
        forms.push(join(sep, &|b| format!("{b:02X}")));
        forms.push(join(sep, &|b| format!("0x{b:02x}")));
        forms.push(join(sep, &|b| b.to_string()));
    }
    forms.push(format!("[{}]", join(",", &|b| b.to_string())));
    forms.push(format!("[{}]", join(", ", &|b| b.to_string())));
    let mut out: Vec<Vec<u8>> = forms.into_iter().map(String::into_bytes).collect();
    for pad in 0..3 {
        let mut padded = vec![0u8; pad];
        padded.extend_from_slice(bytes);
        for alphabet in [B64_STD, B64_URL] {
            let text = base64(&padded, alphabet);
            let trimmed = text.trim_end_matches('=');
            // Drop the characters that mix in padding or the neighbour; keep the interior.
            let interior = &trimmed[(pad * 4).div_ceil(3).min(trimmed.len())..];
            if interior.len() > 2 {
                out.push(interior.as_bytes()[..interior.len() - 1].to_vec());
            }
        }
    }
    out.push(bytes.to_vec());
    out
}
const B64_STD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const B64_URL: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
fn base64(bytes: &[u8], alphabet: &[u8; 64]) -> String {
    let mut out = String::new();
    for chunk in bytes.chunks(3) {
        let n = chunk
            .iter()
            .enumerate()
            .fold(0u32, |n, (i, b)| n | u32::from(*b) << (16 - 8 * i));
        for i in 0..=chunk.len() {
            out.push(char::from(alphabet[(n >> (18 - 6 * i) & 63) as usize]));
        }
    }
    out
}
const PROJECTION_SCHEMA_KEYS: &[&str] = &[
    "format",
    "kind",
    "runs",
    "run",
    "allocationMismatch",
    "classification",
    "reason",
    "loss",
];

fn assert_no_sentinel_bytes(label: &str, haystack: &[u8], sentinels: &[&[u8]]) {
    for run in sentinels {
        for form in forbidden_forms(run) {
            assert!(
                !haystack.windows(form.len()).any(|w| w == form.as_slice()),
                "{label}: leaked {:?}",
                String::from_utf8_lossy(&form)
            );
        }
    }
}

fn assert_no_sentinel_value(label: &str, value: &Value, sentinels: &[&[u8]]) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if !PROJECTION_SCHEMA_KEYS.contains(&key.as_str()) {
                    assert_no_sentinel_bytes(label, key.as_bytes(), sentinels);
                }
                assert_no_sentinel_value(label, value, sentinels);
            }
        }
        Value::Array(values) => {
            if values
                .iter()
                .all(|value| !value.is_array() && !value.is_object())
            {
                assert_no_sentinel_bytes(label, &serde_json::to_vec(value).unwrap(), sentinels);
            }
            for value in values {
                assert_no_sentinel_value(label, value, sentinels);
            }
        }
        Value::String(text) => {
            assert_no_sentinel_bytes(label, text.as_bytes(), sentinels);
            assert_no_sentinel_bytes(label, &serde_json::to_vec(value).unwrap(), sentinels);
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {
            assert_no_sentinel_bytes(label, &serde_json::to_vec(value).unwrap(), sentinels);
        }
    }
}

/// Panics if parsed JSON carries a sentinel outside exact fixed structural key names.
pub(super) fn assert_no_sentinel(label: &str, haystack: &[u8], sentinels: &[&[u8]]) {
    let value: Value = serde_json::from_slice(haystack).unwrap();
    assert_no_sentinel_value(label, &value, sentinels);
}
/// Every sentinel run and the whole-allocation strings of one scenario.
pub(super) fn all_sentinels(extra: &[Vec<u8>]) -> Vec<Vec<u8>> {
    let mut all = vec![PRESTATE.to_vec(), TEMPLATE.to_vec()];
    all.extend(
        [
            &PRESTATE[..4],
            &PRESTATE[4..],
            &PRESTATE[..2],
            &PRESTATE[6..],
        ]
        .map(<[u8]>::to_vec),
    );
    all.extend([READ_UNWRITTEN.1, READ_WRITTEN.1, READ_RESTORE.1].map(|r| r.to_vec()));
    all.extend(extra.iter().cloned());
    all
}

/// The Net evidence fixture: allocation 3C00h..3C08h holding `PRESTATE`, plan writes `TEMPLATE`.
async fn eight_bytes() -> (SimulatedDevice, PreparedBundle) {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        serial_number: Some(SERIAL),
        ..Default::default()
    });
    device.preset_property(0, PID_HARDWARE_TYPE, &[1; 6]);
    device.preset_property(3, PID_PROGRAM_VERSION, &VERSION);
    device.preset_load_state(ObjectIndex::new(3), LoadState::Loaded);
    let step = |source_index, operation| PropertyDownloadStep {
        source_index,
        operation,
    };
    let control = |i, payload| step(i, Op::LoadControl { object: 3, payload });
    let allocation = absolute_data(AbsolutePropertySegment {
        start: 0x3c00,
        length: 8,
        access: 0xff,
        memory_type: SegmentMemoryType::Eeprom,
        checksum_disabled: true,
    })
    .unwrap();
    let plan = PropertyDownloadPlan::new(
        device.address(),
        MaskVersion(0x0025),
        2,
        vec![
            step(0, Op::Connect),
            step(1, Op::HardwareCheck { expected: [1; 6] }),
            control(2, event_payload(LoadEvent::Unload)),
            control(3, event_payload(LoadEvent::StartLoading)),
            control(4, allocation),
            step(
                4,
                Op::WriteMemory {
                    object: 3,
                    address: 0x3c02,
                    data: TEMPLATE.to_vec(),
                },
            ),
            control(5, event_payload(LoadEvent::LoadCompleted)),
            step(6, Op::Disconnect),
        ],
    )
    .unwrap();
    let record = PlanRecord::from_plan(&plan);
    let binding = app::Binding {
        target: plan.target().to_string(),
        mask: plan.mask().0,
        manufacturer: plan.manufacturer(),
        serial: SERIAL,
        hardware: [1; 6],
        application: VERSION,
        profile_id: "synthetic-projection-defaults-v3".into(),
        profile_version: 3,
        profile_sha256: "a".repeat(64),
        source_sha256: "b".repeat(64),
        plan_sha256: record.sha256().unwrap(),
        simulated: true,
        project_preparation_sha256: None,
    };
    let bundle = PreparedBundle::new(binding, record).unwrap();
    run_simulated_property_plan(
        &device,
        &bundle.plan().unwrap(),
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap(),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .unwrap();
    device.preset_memory(0x3c00, &PRESTATE);
    (device, bundle.rebind_hardware(SERIAL).unwrap())
}
fn timing() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(20),
        response_timeout: Duration::from_millis(5),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(30),
        programming_delay: Duration::ZERO,
        ..Default::default()
    }
}

/// No sockets: persistent simulator behind the hardware-kind App entry. Corrupts only on a
/// final read, after LoadCompleted, so every individual write readback succeeds.
pub(super) struct FinalRead<'a> {
    device: &'a SimulatedDevice,
    corrupt: Option<(u16, [u8; 2])>,
    short_prestate: bool,
    written: AtomicBool,
    final_phase: AtomicBool,
}
impl<'a> FinalRead<'a> {
    pub(super) fn new(
        device: &'a SimulatedDevice,
        corrupt: Option<(u16, [u8; 2])>,
        short_prestate: bool,
    ) -> Self {
        Self {
            device,
            corrupt,
            short_prestate,
            written: AtomicBool::new(false),
            final_phase: AtomicBool::new(false),
        }
    }
}
impl ManagementTransport for FinalRead<'_> {
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
        mut service: ApplicationService,
    ) -> Result<(), BusError> {
        match &mut service {
            ApplicationService::PropertyValueWrite {
                property_id, data, ..
            } => {
                self.written.store(true, Ordering::SeqCst);
                if *property_id == 5 && data.first() == Some(&2) {
                    self.final_phase.store(true, Ordering::SeqCst);
                }
            }
            ApplicationService::MemoryRead { number, .. } => {
                if self.short_prestate && !self.written.load(Ordering::SeqCst) && *number > 1 {
                    *number -= 1;
                }
                if self.final_phase.load(Ordering::SeqCst) {
                    if let Some((offset, bytes)) = self.corrupt {
                        self.device
                            .preset_memory(u32::from(0x3c00 + offset), &bytes);
                    }
                }
            }
            _ => {}
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

/// A real failed (or finished) download on disk, its device and bundle.
pub(super) struct Journal {
    pub(super) _temp: tempfile::TempDir,
    pub(super) path: PathBuf,
    pub(super) device: SimulatedDevice,
    pub(super) bundle: PreparedBundle,
}
pub(super) async fn download(
    corrupt: Option<(u16, [u8; 2])>,
    short_prestate: bool,
    cancelled: bool,
) -> Journal {
    let (device, bundle) = eight_bytes().await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    let transport = FinalRead::new(&device, corrupt, short_prestate);
    let result = app::execute_hardware(
        &transport,
        &bundle,
        &mut store,
        timing(),
        &required_confirmation_phrase(device.address(), WriteScope::Download),
        &required_release_phrase(device.address()),
        &AtomicBool::new(cancelled),
    )
    .await;
    assert_eq!(result.is_err(), corrupt.is_some() || cancelled);
    drop(store);
    Journal {
        _temp: temp,
        path,
        device,
        bundle,
    }
}
/// Explicit restore of the failed download from a fresh open, as the App does.
pub(super) async fn restore(journal: &Journal, corrupt: Option<(u16, [u8; 2])>) {
    let mut store = OperationStore::open(&journal.path, &journal.bundle.binding).unwrap();
    let transport = FinalRead::new(&journal.device, corrupt, false);
    let result = app::restore_hardware(
        &transport,
        &journal.bundle,
        &mut store,
        timing(),
        &journal.bundle.restore_phrase(),
        &required_confirmation_phrase(journal.device.address(), WriteScope::Download),
        &required_release_phrase(journal.device.address()),
        &AtomicBool::new(false),
    )
    .await;
    assert_eq!(result.is_err(), corrupt.is_some());
}

pub(super) fn journal_bytes(path: &Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
    let mut files: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (e.file_name(), std::fs::read(e.path()).unwrap())
        })
        .collect();
    files.sort();
    files
}
fn last_checkpoint(path: &Path) -> Value {
    let files = journal_bytes(path);
    let (_, bytes) = files
        .iter()
        .rfind(|(name, _)| name.to_string_lossy().starts_with("checkpoint-"))
        .unwrap();
    serde_json::from_slice(bytes).unwrap()
}
/// What a scenario's read looked like, printed before any assertion so a RED shows it.
pub(super) struct Observed {
    pub(super) snapshot: PrivateSnapshot,
    pub(super) json: Option<Value>,
    pub(super) text: String,
}
pub(super) fn observe(label: &str, path: &Path, binding: &app::Binding) -> Observed {
    let snapshot = read_snapshot(path, binding);
    let json = snapshot
        .projection()
        .map(|p| serde_json::to_value(p).unwrap());
    let text = json.as_ref().map(Value::to_string).unwrap_or_default();
    eprintln!("[{label}] snapshot={snapshot:?}\n[{label}] projection={text}");
    Observed {
        snapshot,
        json,
        text,
    }
}
pub(super) fn projection(runs: Value) -> Value {
    json!({"format": 1, "kind": "knxbench-system2-private-projection", "runs": runs})
}
pub(super) fn withheld(run: &str) -> Value {
    json!({"run": run, "allocationMismatch": {
        "classification": "recordedBytesWithheld",
        "reason": "unclassifiedMemory",
        "loss": "recordedBytesNotExported"}})
}
pub(super) fn run_class(run: &str, class: &str) -> Value {
    json!({"run": run, "allocationMismatch": {"classification": class}})
}
/// The journal's own recorded whole-allocation hex, as the private checkpoint stores it.
fn recorded(path: &Path) -> (Value, String) {
    let last = last_checkpoint(path);
    eprintln!(
        "[fixture] event={} final_step={} allocation_evidence_present={}",
        last["checkpoint"]["event"],
        last["checkpoint"]["final_step"],
        !last["checkpoint"]["allocation_evidence"].is_null()
    );
    let evidence = last["checkpoint"]["allocation_evidence"].clone();
    (evidence, serde_json::to_string(&last).unwrap())
}
fn splice(base: &[u8; 8], at: usize, bytes: &[u8]) -> Vec<u8> {
    let mut out = base.to_vec();
    out[at..at + bytes.len()].copy_from_slice(bytes);
    out
}
/// Whole-allocation expectation of the download: prestate with the template over 2..6.
fn download_expected() -> Vec<u8> {
    splice(&PRESTATE, 2, &TEMPLATE)
}

#[tokio::test]
async fn unwritten_mismatch_withholds_bytes_that_the_journal_still_records() {
    let journal = download(Some(READ_UNWRITTEN), false, false).await;
    let expected = download_expected();
    let actual = splice(&expected.clone().try_into().unwrap(), 6, &READ_UNWRITTEN.1);
    let (evidence, journal_text) = recorded(&journal.path);
    // Precondition: the private journal really holds both sides, in its own hex form.
    assert_eq!(evidence["expected"], hex(&expected));
    assert_eq!(evidence["actual"], hex(&actual));
    assert!(journal_text.contains(&hex(&TEMPLATE)));
    let before = journal_bytes(&journal.path);
    let seen = observe("unwritten", &journal.path, &journal.bundle.binding);
    assert_eq!(seen.json, Some(projection(json!([withheld("download")]))));
    let sentinels = all_sentinels(&[expected, actual]);
    let refs: Vec<&[u8]> = sentinels.iter().map(Vec::as_slice).collect();
    assert_no_sentinel("unwritten", seen.text.as_bytes(), &refs);
    assert_eq!(journal_bytes(&journal.path), before, "journal untouched");
}

#[tokio::test]
async fn written_mismatch_is_projected_exactly_like_an_unwritten_one() {
    let journal = download(Some(READ_WRITTEN), false, false).await;
    let (evidence, _) = recorded(&journal.path);
    let actual = splice(&download_expected().try_into().unwrap(), 2, &READ_WRITTEN.1);
    assert_eq!(evidence["actual"], hex(&actual));
    let seen = observe("written", &journal.path, &journal.bundle.binding);
    assert_eq!(seen.json, Some(projection(json!([withheld("download")]))));
    let sentinels = all_sentinels(&[download_expected(), actual]);
    let refs: Vec<&[u8]> = sentinels.iter().map(Vec::as_slice).collect();
    assert_no_sentinel("written", seen.text.as_bytes(), &refs);
}

#[tokio::test]
async fn unreadable_prestate_mismatch_is_withheld_and_progress_keeps_the_unreadable_fact() {
    let journal = download(Some(READ_WRITTEN), true, false).await;
    let (evidence, _) = recorded(&journal.path);
    assert_eq!(evidence["prestate_unreadable"], true);
    // Unreadable prestate: the unwritten expectation is a zero placeholder, not PRESTATE.
    let expected = splice(&[0; 8], 2, &TEMPLATE);
    assert_eq!(evidence["expected"], hex(&expected));
    let seen = observe("unreadable", &journal.path, &journal.bundle.binding);
    assert_eq!(seen.json, Some(projection(json!([withheld("download")]))));
    let progress = seen.snapshot.progress().expect("progress with projection");
    eprintln!(
        "[unreadable] unreadable={:?}",
        progress.prestate_unreadable_allocations
    );
    assert_eq!(progress.prestate_unreadable_allocations.len(), 1);
    assert_eq!(progress.outcome, LocalOutcome::Failed);
    let sentinels = all_sentinels(&[expected]);
    let refs: Vec<&[u8]> = sentinels.iter().map(Vec::as_slice).collect();
    assert_no_sentinel("unreadable", seen.text.as_bytes(), &refs);
}

#[tokio::test]
async fn restore_has_its_own_expectation_and_run_attribution() {
    let journal = download(Some(READ_WRITTEN), false, false).await;
    restore(&journal, Some(READ_RESTORE)).await;
    let (evidence, _) = recorded(&journal.path);
    // The restore compares against the ORIGINAL captured bytes, not the downloaded template.
    let restore_actual = splice(&PRESTATE, 3, &READ_RESTORE.1);
    assert_eq!(evidence["expected"], hex(&PRESTATE));
    assert_eq!(evidence["actual"], hex(&restore_actual));
    let seen = observe("restore", &journal.path, &journal.bundle.binding);
    assert_eq!(
        seen.json,
        Some(projection(json!([
            withheld("download"),
            withheld("restore")
        ])))
    );
    assert_eq!(
        seen.snapshot.progress().map(|p| p.outcome),
        Some(LocalOutcome::RestoreFailed)
    );
    assert_eq!(
        seen.snapshot.progress().map(|p| p.restore_attempted),
        Some(true)
    );
    let sentinels = all_sentinels(&[restore_actual, download_expected()]);
    let refs: Vec<&[u8]> = sentinels.iter().map(Vec::as_slice).collect();
    assert_no_sentinel("restore", seen.text.as_bytes(), &refs);
}

#[tokio::test]
async fn a_successful_restore_after_a_mismatch_stays_unclassified_without_a_witness() {
    let journal = download(Some(READ_WRITTEN), false, false).await;
    restore(&journal, None).await;
    let seen = observe("restored", &journal.path, &journal.bundle.binding);
    assert_eq!(
        seen.snapshot.progress().map(|p| p.outcome),
        Some(LocalOutcome::Restored)
    );
    // No typed restore-verified result exists; counters and `Restored` are not a witness.
    assert_eq!(
        seen.json,
        Some(projection(json!([
            withheld("download"),
            run_class("restore", "unclassified")
        ])))
    );
}

#[tokio::test]
async fn a_finished_download_remains_unclassified_without_a_separate_typed_witness() {
    let journal = download(None, false, false).await;
    let seen = observe("verified", &journal.path, &journal.bundle.binding);
    assert_eq!(
        seen.snapshot.progress().map(|p| p.outcome),
        Some(LocalOutcome::Finished)
    );
    assert_eq!(
        seen.json,
        Some(projection(json!([run_class("download", "unclassified")])))
    );
    // Cancelled before any step: no mismatch either, but no typed witness of absence.
    let cancelled = download(None, false, true).await;
    let seen = observe("cancelled", &cancelled.path, &cancelled.bundle.binding);
    assert_eq!(
        seen.json,
        Some(projection(json!([run_class("download", "unclassified")])))
    );
}

/// Failed checkpoints on a real recovery record, driven by hand like the existing
/// missing-evidence test: `final_step` decides typed (no record) versus untyped.
fn failed_on_real_recovery(
    journal: &Journal,
    dir: &Path,
    final_step: Option<FinalStepRecord>,
) -> PathBuf {
    let recovery = OperationStore::open(&journal.path, &journal.bundle.binding)
        .unwrap()
        .recovery()
        .unwrap()
        .clone();
    let path = dir.join(if final_step.is_some() {
        "typed"
    } else {
        "untyped"
    });
    let mut store = OperationStore::create(&path, journal.bundle.binding.clone()).unwrap();
    store
        .checkpoint(Checkpoint::RecoveryAcquired { recovery })
        .unwrap();
    let last = final_step.as_ref().map_or(0, |_| 7);
    for index in 0..=last {
        store
            .checkpoint(Checkpoint::StepStarted {
                index,
                mutation: true,
            })
            .unwrap();
        if index < last {
            store
                .checkpoint(Checkpoint::StepFinished { index })
                .unwrap();
        }
    }
    store
        .checkpoint(Checkpoint::Failed {
            code: app::FailureCode::IdentityOrRecovery,
            local_diagnostic: "synthetic caller omitted evidence".into(),
            pointer_failure: None,
            final_step,
            allocation_evidence: None,
        })
        .unwrap();
    path
}

#[tokio::test]
async fn typed_mismatch_without_record_is_not_recorded_and_untyped_failure_is_unclassified() {
    let journal = download(Some(READ_WRITTEN), false, false).await;
    let dir = tempfile::tempdir().unwrap();
    let mismatch = FinalStepRecord {
        category: FinalStepCategory::VerificationMismatch,
        stage: FinalStepStage::Allocation,
    };
    let typed = failed_on_real_recovery(&journal, dir.path(), Some(mismatch));
    let untyped = failed_on_real_recovery(&journal, dir.path(), None);
    let binding = &journal.bundle.binding;
    let typed_seen = observe("not-recorded", &typed, binding);
    let untyped_seen = observe("untyped", &untyped, binding);
    assert_eq!(
        typed_seen.json,
        Some(projection(json!([run_class("download", "notRecorded")])))
    );
    assert_eq!(
        untyped_seen.json,
        Some(projection(json!([run_class("download", "unclassified")])))
    );
}
