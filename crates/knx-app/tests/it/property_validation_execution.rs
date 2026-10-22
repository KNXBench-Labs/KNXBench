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
        old_device_control: 0,
        resources: vec![
            knx_net::commissioning::property_recovery::PropertyRecoveryResource {
                object: 3,
                state: LoadState::Loaded,
                pointer: [0, 0, 0x3c, 0],
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
    let (device, bundle) = fixture();
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
