//! Simulator-only vertical path uses Net acquisition and fresh-disk explicit restore.
use knx_app::property_validation::{
    self as app, Binding, Checkpoint, LocalOutcome, OperationStore,
};
use knx_app::property_validation_bundle::{PlanRecord, PreparedBundle, RecoveryRecord};
use knx_core::commissioning::{
    absolute_load_control::{absolute_data, AbsolutePropertySegment},
    authorisation::AuthorisationPlan,
    load_control::event_payload,
    load_control_memory::SegmentMemoryType,
    load_state::{LoadEvent, LoadState, MaskVersion},
    mutation::{WriteAuthorisation, WriteScope},
    properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_PROGRAM_VERSION},
    property_download::{PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation as Op},
};
use knx_net::commissioning::{
    property_download::run_simulated_property_plan,
    property_recovery::ExperimentalPropertyPreconditions,
    simulator::{SimulatedDevice, SimulatorConfig},
    SessionTiming,
};
use std::{sync::atomic::AtomicBool, time::Duration};

mod send_boundary;
mod superseded_defaults_profile;

const SERIAL: [u8; 6] = [0, 2, 1, 2, 3, 4];
const VERSION: [u8; 5] = [0, 2, 0x12, 0x34, 1];
fn fixture() -> (SimulatedDevice, PreparedBundle) {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        serial_number: Some(SERIAL),
        ..Default::default()
    });
    device.preset_property(0, PID_HARDWARE_TYPE, &[1; 6]);
    device.preset_property(3, PID_PROGRAM_VERSION, &VERSION);
    device.preset_load_state(ObjectIndex::new(3), LoadState::Loaded);
    let op = |source_index, operation| PropertyDownloadStep {
        source_index,
        operation,
    };
    let ctrl = |i, payload| op(i, Op::LoadControl { object: 3, payload });
    let plan = PropertyDownloadPlan::new(
        device.address(),
        MaskVersion(0x0025),
        2,
        vec![
            op(0, Op::Connect),
            op(1, Op::HardwareCheck { expected: [1; 6] }),
            ctrl(2, event_payload(LoadEvent::Unload)),
            ctrl(3, event_payload(LoadEvent::StartLoading)),
            ctrl(
                4,
                absolute_data(AbsolutePropertySegment {
                    start: 0x3c00,
                    length: 4,
                    access: 0xff,
                    memory_type: SegmentMemoryType::Eeprom,
                    checksum_disabled: true,
                })
                .unwrap(),
            ),
            op(
                4,
                Op::WriteMemory {
                    object: 3,
                    address: 0x3c00,
                    data: vec![1, 2, 3, 4],
                },
            ),
            ctrl(5, event_payload(LoadEvent::LoadCompleted)),
            op(6, Op::Disconnect),
        ],
    )
    .unwrap();
    let record = PlanRecord::from_plan(&plan);
    let binding = Binding {
        target: plan.target().to_string(),
        mask: plan.mask().0,
        manufacturer: plan.manufacturer(),
        serial: SERIAL,
        hardware: [1; 6],
        application: VERSION,
        profile_id: "synthetic-v1".into(),
        profile_version: 1,
        profile_sha256: "a".repeat(64),
        source_sha256: "b".repeat(64),
        plan_sha256: record.sha256().unwrap(),
        simulated: true,
        project_preparation_sha256: None,
    };
    let bundle = PreparedBundle::new(binding, record).unwrap();
    (device, bundle)
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
async fn establish(device: &SimulatedDevice, bundle: &PreparedBundle) {
    let plan = bundle.plan().unwrap();
    run_simulated_property_plan(
        device,
        &plan,
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap(),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .unwrap();
    device.preset_memory(0x3c00, &[9, 8, 7, 6]);
}
#[tokio::test]
async fn property_validation_restore_cannot_finish_without_executing_complete_procedure() {
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    drop(store);
    let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    assert!(fresh.checkpoint(Checkpoint::RestoreStarted).is_ok());
    assert!(
        fresh.checkpoint(Checkpoint::Restored).is_err(),
        "unexecuted recovery must not be declared restored"
    );
}
#[tokio::test]
async fn property_validation_execution_persists_complete_prewrite_then_restores_from_fresh_disk() {
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert_eq!(
        device.memory(0x3c00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    let recovery = store.recovery().unwrap();
    assert_eq!(recovery.regions[0].data, [9, 8, 7, 6]);
    assert_eq!(
        recovery.to_snapshot(&bundle).unwrap().original_plan,
        bundle.plan().unwrap()
    );
    drop(store);
    let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    app::restore_simulated(
        &device,
        &bundle,
        &mut fresh,
        timing(),
        &AtomicBool::new(false),
        &bundle.restore_phrase(),
    )
    .await
    .unwrap();
    assert_eq!(fresh.outcome(), LocalOutcome::Restored);
    assert_eq!(
        device.memory(0x3c00, 4),
        [Some(9), Some(8), Some(7), Some(6)]
    );
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loaded);
}
#[test]
fn property_validation_bundle_tamper_and_unknown_fields_refuse() {
    let (_, bundle) = fixture();
    let mut value = serde_json::to_value(&bundle).unwrap();
    value["binding"]["source_sha256"] = "not-a-digest".into();
    let changed: PreparedBundle = serde_json::from_value(value).unwrap();
    assert!(changed.plan().is_err());
    let mut value = serde_json::to_value(&bundle).unwrap();
    value["key"] = "secret".into();
    assert!(serde_json::from_value::<PreparedBundle>(value).is_err());
    assert_ne!(bundle.acceptance_phrase(), bundle.restore_phrase());
    assert!(bundle
        .acceptance_phrase()
        .contains(&bundle.binding.plan_sha256));
}
#[tokio::test]
async fn property_validation_execution_cancellation_is_persisted_without_backup_or_mutation() {
    let (device, bundle) = fixture();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    assert!(app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(true)
    )
    .await
    .is_err());
    drop(store);
    let fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    assert_eq!(fresh.outcome(), LocalOutcome::Cancelled);
    assert!(!fresh.mutation_possible());
    assert!(fresh.recovery().is_none());
    assert!(device.seen().is_empty());
}
#[tokio::test]
async fn property_validation_execution_persistence_failure_has_no_first_write() {
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let before = device.seen().len();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    // New immutable checkpoint collision: never overwrite or continue on failure.
    std::fs::write(path.join("checkpoint-00000003.json"), b"collision").unwrap();
    assert!(app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    use knx_net::commissioning::simulator::Seen;
    assert!(!device.seen()[before..]
        .iter()
        .any(|s| matches!(s, Seen::PropertyWrite { .. } | Seen::MemoryWrite { .. })));
}
#[tokio::test]
async fn property_validation_execution_restore_refuses_wrong_confirmation_and_keeps_failed_restore()
{
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    drop(store);
    let mut store = OperationStore::open(&path, &bundle.binding).unwrap();
    let before = device.seen().len();
    assert!(app::restore_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
        "yes"
    )
    .await
    .is_err());
    assert_eq!(device.seen().len(), before);
    assert!(app::restore_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(true),
        &bundle.restore_phrase()
    )
    .await
    .is_err());
    drop(store);
    let mut store = OperationStore::open(&path, &bundle.binding).unwrap();
    assert_eq!(store.outcome(), LocalOutcome::RestoreFailed);
    assert!(store.recovery().is_some());
    assert!(
        app::restore_simulated(
            &device,
            &bundle,
            &mut store,
            timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase()
        )
        .await
        .is_err(),
        "failed restore cannot automatically retry"
    );
}
#[tokio::test]
async fn property_validation_failure_details_stay_in_private_protocol() {
    let (device, bundle) = fixture();
    device.preset_property(0, PID_HARDWARE_TYPE, &[9; 6]);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    assert!(app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    let protocol = std::fs::read_dir(&path)
        .unwrap()
        .filter_map(|e| {
            let e = e.unwrap();
            e.file_name()
                .to_string_lossy()
                .starts_with("checkpoint-")
                .then(|| std::fs::read_to_string(e.path()).unwrap())
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        protocol.contains("HardwareMismatch"),
        "typed failure lacks private raw local diagnostic"
    );
    assert!(!serde_json::to_string(&store.shareable_report())
        .unwrap()
        .contains("HardwareMismatch"));
}
#[test]
fn property_validation_recovery_record_refuses_partial_regions() {
    let (_, bundle) = fixture();
    let plan = bundle.plan().unwrap();
    let snapshot = knx_net::commissioning::property_recovery::PropertyRecoverySnapshot {
        version: 1,
        original_plan: plan.clone(),
        preconditions: ExperimentalPropertyPreconditions::same_application_reinstallation(
            &plan, SERIAL, VERSION,
        )
        .unwrap(),
        observed_serial: SERIAL,
        observed_program_version: VERSION.to_vec(),
        old_device_control: 0,
        unreadable_allocations: Vec::new(),
        resources: vec![
            knx_net::commissioning::property_recovery::PropertyRecoveryResource {
                object: 3,
                state: LoadState::Loaded,
                pointer: Some([0, 0, 0x3c, 0]),
            },
        ],
        regions: vec![],
        allocations: vec![],
    };
    assert!(RecoveryRecord::from_snapshot(&snapshot, &bundle).is_err());
    let temp = tempfile::tempdir().unwrap();
    let mut store =
        OperationStore::create(&temp.path().join("op"), bundle.binding.clone()).unwrap();
    assert!(store
        .checkpoint(Checkpoint::StepStarted {
            index: 0,
            mutation: true
        })
        .is_err());
}
#[test]
fn property_validation_rebind_hardware_swaps_serial_and_marks_physical() {
    let (_, bundle) = fixture();
    assert!(bundle.binding.simulated);
    let real = [0, 7, 0, 1, 2, 3];
    let rebound = bundle.rebind_hardware(real).unwrap();
    assert!(!rebound.binding.simulated);
    assert_eq!(rebound.binding.serial, real);
    assert_eq!(rebound.binding.plan_sha256, bundle.binding.plan_sha256);
    assert_eq!(rebound.binding.source_sha256, bundle.binding.source_sha256);
    // The plan stays identical; only the recovery premise serial differs.
    assert_eq!(rebound.plan().unwrap(), bundle.plan().unwrap());
    assert_ne!(
        rebound.preconditions().unwrap(),
        bundle.preconditions().unwrap()
    );
}
#[tokio::test]
async fn property_validation_execute_hardware_refuses_wrong_phrases_before_any_frame() {
    let (device, mut bundle) = fixture();
    // Profile v2: the superseded v1 refusal must not mask the phrase checks.
    bundle.binding.profile_version = 2;
    bundle.binding.profile_id = "synthetic-defaults-v2".into();
    let bundle = PreparedBundle::new(bundle.binding, bundle.normalized_plan).unwrap();
    let rebound = bundle.rebind_hardware([0, 7, 0, 1, 2, 3]).unwrap();
    let target = rebound.plan().unwrap().target();
    let download = knx_core::commissioning::mutation::required_confirmation_phrase(
        target,
        WriteScope::Download,
    );
    let release = knx_net::commissioning::hardware_release::required_release_phrase(target);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, rebound.binding.clone()).unwrap();
    assert!(
        app::execute_hardware(
            &device,
            &rebound,
            &mut store,
            timing(),
            &download,
            "wrong release",
            &AtomicBool::new(false),
        )
        .await
        .is_err(),
        "a wrong best-effort release phrase must refuse before any frame"
    );
    assert!(device.seen().is_empty());
    assert!(
        app::execute_hardware(
            &device,
            &rebound,
            &mut store,
            timing(),
            "wrong download",
            &release,
            &AtomicBool::new(false),
        )
        .await
        .is_err(),
        "a wrong download confirmation must refuse before any frame"
    );
    assert!(device.seen().is_empty());
}
fn checkpoint_text(path: &std::path::Path) -> String {
    let mut files: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap())
        .filter(|e| e.file_name().to_string_lossy().starts_with("checkpoint-"))
        .collect();
    files.sort_by_key(|e| e.file_name());
    files
        .iter()
        .map(|e| std::fs::read_to_string(e.path()).unwrap())
        .collect::<Vec<_>>()
        .join("\n")
}
#[tokio::test]
async fn property_validation_prestate_pointer_mismatch_finishes_and_blocks_original_state_recovery()
{
    // Issue #30: no shape-establishing run; the device answers PID 7 = 0 while
    // the source record says 3C00h. The owner ruling allows the download.
    let (device, bundle) = fixture();
    device.preset_memory(0x3c00, &[9, 8, 7, 6]);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .expect("prestate pointer mismatch is an observation, not a refusal");
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert_eq!(
        device.memory(0x3c00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    let recovery = store.recovery().unwrap();
    assert_eq!(recovery.resources[0].pointer, [0; 4]);
    let journal = checkpoint_text(&path);
    assert!(journal.contains("prestate_observations"), "{journal}");
    drop(store);
    let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    assert!(!fresh.progress().restore_available);
    assert!(fresh.progress().backup_available);
    let before = checkpoint_text(&path);
    let error = app::restore_simulated(
        &device,
        &bundle,
        &mut fresh,
        timing(),
        &AtomicBool::new(false),
        &bundle.restore_phrase(),
    )
    .await
    .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("original-state recovery unavailable"),
        "{error}"
    );
    assert_eq!(checkpoint_text(&path), before, "refusal appends nothing");
    assert_eq!(
        device.memory(0x3c00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
}
#[tokio::test]
async fn property_validation_alpha16_recovery_record_without_observations_still_loads() {
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    // The Alpha 16 RecoveryRecord grammar: exactly these keys, nothing else.
    let value = serde_json::to_value(store.recovery().unwrap()).unwrap();
    let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "allocations",
            "old_device_control",
            "original_bundle",
            "regions",
            "resources",
            "version"
        ]
    );
    let old: RecoveryRecord = serde_json::from_value(value).unwrap();
    let snapshot = old.to_snapshot(&bundle).unwrap();
    assert!(snapshot.recovery_plan(&bundle.plan().unwrap()).is_ok());
    drop(store);
    let fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    assert!(fresh.progress().restore_available);
}
#[tokio::test]
async fn property_validation_differing_serial_is_recorded_and_recovery_stays_bound_to_it() {
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let observed = [0, 2, 9, 9, 9, 9];
    device.preset_property(0, 11, &observed);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .expect("serial is an observation, not download admission");
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert_eq!(store.recovery().unwrap().observed_serial, Some(observed));
    assert!(checkpoint_text(&path).contains("observed_serial"));
    drop(store);
    // A third device of the same type now answers: the captured bytes are foreign to it.
    device.preset_property(0, 11, &[0, 2, 7, 7, 7, 7]);
    let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    assert!(fresh.progress().restore_available);
    let error = app::restore_simulated(
        &device,
        &bundle,
        &mut fresh,
        timing(),
        &AtomicBool::new(false),
        &bundle.restore_phrase(),
    )
    .await
    .unwrap_err();
    assert!(
        error.to_string().contains("explicit restore failed"),
        "{error}"
    );
    let journal = checkpoint_text(&path);
    assert!(journal.contains("serial number differs"), "{journal}");
    assert_eq!(fresh.outcome(), LocalOutcome::RestoreFailed);
    assert_eq!(
        device.memory(0x3c00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
}
#[tokio::test]
async fn property_validation_different_prior_program_version_record_blocks_original_state_recovery()
{
    // The simulator plan writes no identity record, so build the evidence from a
    // matching acquisition and mark the prior application as different.
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    let mut record = store.recovery().unwrap().clone();
    drop(store);
    record.observed_program_version = Some(vec![0, 2, 0x12, 0x34, 9]);
    let snapshot = record.to_snapshot(&bundle).expect("evidence still loads");
    assert_eq!(
        snapshot.observed_program_version,
        [0, 2, 0x12, 0x34, 9].to_vec()
    );
    let reason = match snapshot.recovery_plan(&bundle.plan().unwrap()) {
        Err(failure) => format!("{failure:?}"),
        Ok(_) => panic!("recovery must be unavailable"),
    };
    assert!(reason.contains("different application"), "{reason}");
    // Equal values are never recorded twice (one canonical Alpha 16 encoding).
    record.observed_program_version = Some(VERSION.to_vec());
    assert!(record.to_snapshot(&bundle).is_err());
    let path = temp.path().join("op2");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    record.observed_program_version = Some(Vec::new());
    store
        .checkpoint(Checkpoint::RecoveryAcquired { recovery: record })
        .unwrap();
    store
        .checkpoint(Checkpoint::Failed {
            code: app::FailureCode::Executor,
            local_diagnostic: "synthetic".into(),
        })
        .unwrap();
    drop(store);
    let fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    let progress = fresh.progress();
    assert!(progress.prestate_differs);
    assert!(!progress.restore_available);
    assert_eq!(
        progress.program_version_observation,
        Some((VERSION, Vec::new()))
    );
}
#[tokio::test]
async fn property_validation_refused_pointer_and_in_procedure_observations_persist() {
    use knx_app::property_validation::PointerObservationPhase;
    use knx_app::property_validation_bundle::PointerObservationRecord;
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let temp = tempfile::tempdir().unwrap();
    let mut store =
        OperationStore::create(&temp.path().join("op"), bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    let alpha16 = store.recovery().unwrap().clone();
    // Alpha 16 resource grammar: exactly object, pointer, state.
    let value = serde_json::to_value(&alpha16.resources[0]).unwrap();
    let mut keys: Vec<_> = value.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(keys, ["object", "pointer", "state"]);
    // A refused PID 7 read is persisted explicitly and blocks recovery.
    let mut snapshot = alpha16.to_snapshot(&bundle).unwrap();
    snapshot.resources[0].pointer = None;
    let record = RecoveryRecord::from_snapshot(&snapshot, &bundle).unwrap();
    let json = serde_json::to_string(&record).unwrap();
    assert!(json.contains("\"pointer_refused\":true"), "{json}");
    assert!(json.contains("\"observed\":null"), "{json}");
    assert_eq!(record.to_snapshot(&bundle).unwrap(), snapshot);
    let reason = format!("{:?}", snapshot.recovery_plan(&bundle.plan().unwrap()));
    assert!(reason.contains("observed refused"), "{reason}");
    let mut forged = record.clone();
    forged.resources[0].pointer = [0, 0, 0x3c, 0];
    assert!(
        forged.to_snapshot(&bundle).is_err(),
        "refused carries no value"
    );
    // In-procedure observations are a durable checkpoint bound to the active
    // allocation step (operation 4 is the 03 00 record of object 3).
    let path = temp.path().join("op2");
    let observation = PointerObservationRecord {
        object: 3,
        expected: [0, 0, 0x3c, 0],
        observed: Some([0, 0, 0x3d, 0]),
        state: LoadState::Loading.octet(),
    };
    let mut store = store_at_step(&path, &bundle, alpha16, 4);
    assert!(store
        .checkpoint(Checkpoint::InProcedurePointerObservation {
            operation_index: 3,
            observation: observation.clone(),
        })
        .is_err());
    store
        .checkpoint(Checkpoint::InProcedurePointerObservation {
            operation_index: 4,
            observation: observation.clone(),
        })
        .unwrap();
    drop(store);
    let fresh = OperationStore::open(&path, &bundle.binding).unwrap();
    let progress = fresh.progress();
    assert_eq!(progress.pointer_observations.len(), 1);
    assert_eq!(
        progress.pointer_observations[0].phase,
        PointerObservationPhase::InProcedure
    );
    assert_eq!(progress.pointer_observations[0].operation_index, Some(4));
    assert_eq!(progress.pointer_observations[0].record, observation);
    // An in-procedure difference alone leaves the original prestate usable.
    assert!(!progress.prestate_differs);
}
// Drive a fresh store to an active step `index` of the fixture plan.
fn store_at_step(
    path: &std::path::Path,
    bundle: &PreparedBundle,
    recovery: RecoveryRecord,
    index: usize,
) -> OperationStore {
    let mut store = OperationStore::create(path, bundle.binding.clone()).unwrap();
    store
        .checkpoint(Checkpoint::RecoveryAcquired { recovery })
        .unwrap();
    for step in 0..=index {
        store
            .checkpoint(Checkpoint::StepStarted {
                index: step,
                mutation: true,
            })
            .unwrap();
        if step < index {
            store
                .checkpoint(Checkpoint::StepFinished { index: step })
                .unwrap();
        }
    }
    store
}
async fn executed_record() -> (PreparedBundle, RecoveryRecord) {
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let temp = tempfile::tempdir().unwrap();
    let mut store =
        OperationStore::create(&temp.path().join("op"), bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    let record = store.recovery().unwrap().clone();
    (bundle, record)
}
#[tokio::test]
async fn property_validation_in_procedure_observation_must_match_an_allocation_step() {
    use knx_app::property_validation_bundle::PointerObservationRecord;
    let (bundle, record) = executed_record().await;
    let observation = |object| PointerObservationRecord {
        object,
        expected: [0, 0, 0x3c, 0],
        observed: Some([0, 0, 0x3d, 0]),
        state: LoadState::Loading.octet(),
    };
    let temp = tempfile::tempdir().unwrap();
    // Step 0 is Connect, not an allocation record.
    let mut store = store_at_step(&temp.path().join("a"), &bundle, record.clone(), 0);
    assert!(store
        .checkpoint(Checkpoint::InProcedurePointerObservation {
            operation_index: 0,
            observation: observation(3),
        })
        .is_err());
    // Step 4 is the 03 00 allocation of object 3: another object refuses.
    let mut store = store_at_step(&temp.path().join("b"), &bundle, record, 4);
    assert!(store
        .checkpoint(Checkpoint::InProcedurePointerObservation {
            operation_index: 4,
            observation: observation(1),
        })
        .is_err());
    store
        .checkpoint(Checkpoint::InProcedurePointerObservation {
            operation_index: 4,
            observation: observation(3),
        })
        .unwrap();
}
#[tokio::test]
async fn property_validation_download_and_restore_observations_are_both_kept() {
    use knx_app::property_validation_bundle::PointerObservationRecord;
    let (bundle, record) = executed_record().await;
    let observation = |observed| PointerObservationRecord {
        object: 3,
        expected: [0, 0, 0x3c, 0],
        observed: Some(observed),
        state: LoadState::Loaded.octet(),
    };
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = store_at_step(&path, &bundle, record, 0);
    store
        .checkpoint(Checkpoint::FinalPointerObservations {
            observations: vec![observation([0, 0, 0x3d, 0])],
        })
        .unwrap();
    store
        .checkpoint(Checkpoint::StepFinished { index: 0 })
        .unwrap();
    store
        .checkpoint(Checkpoint::Failed {
            code: app::FailureCode::Executor,
            local_diagnostic: "synthetic".into(),
        })
        .unwrap();
    drop(store);
    let mut store = OperationStore::open(&path, &bundle.binding).unwrap();
    store.checkpoint(Checkpoint::RestoreStarted).unwrap();
    store
        .checkpoint(Checkpoint::StepStarted {
            index: 0,
            mutation: true,
        })
        .unwrap();
    store
        .checkpoint(Checkpoint::FinalPointerObservations {
            observations: vec![observation([0, 0, 0x3e, 0])],
        })
        .unwrap();
    let progress = store.progress();
    assert_eq!(
        progress.pointer_observations.len(),
        2,
        "{:?}",
        progress.pointer_observations
    );
    use knx_app::property_validation::PointerObservationRun;
    assert_eq!(
        progress
            .pointer_observations
            .iter()
            .map(|o| (o.run, o.record.observed))
            .collect::<Vec<_>>(),
        [
            (PointerObservationRun::Download, Some([0, 0, 0x3d, 0])),
            (PointerObservationRun::Restore, Some([0, 0, 0x3e, 0])),
        ]
    );
    // Reopening replays the journal to the same two entries.
    drop(store);
    let reopened = OperationStore::open(&path, &bundle.binding).unwrap();
    assert_eq!(reopened.progress().pointer_observations.len(), 2);
}
#[tokio::test]
async fn property_validation_unreadable_prestate_is_listed_and_never_a_restorable_backup() {
    let (bundle, record) = executed_record().await;
    let mut snapshot = record.to_snapshot(&bundle).unwrap();
    snapshot.allocations[0].data.fill(0);
    snapshot.regions[0].data.fill(0);
    snapshot.unreadable_allocations = vec![0];
    let record = RecoveryRecord::from_snapshot(&snapshot, &bundle).unwrap();
    assert_eq!(record.prestate_unreadable_allocations, [0]);
    let temp = tempfile::tempdir().unwrap();
    let mut store =
        OperationStore::create(&temp.path().join("op"), bundle.binding.clone()).unwrap();
    store
        .checkpoint(Checkpoint::RecoveryAcquired { recovery: record })
        .unwrap();
    let progress = store.progress();
    assert_eq!(
        progress.prestate_unreadable_allocations,
        [app::UnreadableAllocation {
            object: 3,
            address: 0x3c00,
            length: 4,
        }]
    );
    assert!(progress.prestate_differs);
    assert!(!progress.restore_available);
    assert!(progress.recovery_phrases.is_none());
}
