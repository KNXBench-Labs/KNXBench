//! Source-bound preparation keeps experimental defaults distinct from physical admission.

use knx_app::system2_validation_preparation::{prepare_simulated_candidate, ValidationProfile};
use knx_core::IndividualAddress;
use std::path::PathBuf;

const PROFILE: &[u8] =
    include_bytes!("../../../../data/system2-validation/up258d12-920504-v1.json");

#[test]
fn profile_requires_all_explicit_rules_and_rejects_unknown_or_unbounded_configuration() {
    let profile = ValidationProfile::from_json(PROFILE).unwrap();
    assert_eq!(profile.program_id, "M-0001_A-9205-04-9270");
    for (old, new) in [
        ("complete-defaults-no-links-v1", "any-project"),
        ("strong-hypothesis-simulator-validation-only", "verified"),
        ("declared-counts-zero-flags-zero-table-identity-v1", "guess"),
    ] {
        let text = String::from_utf8(PROFILE.to_vec())
            .unwrap()
            .replace(old, new);
        assert!(ValidationProfile::from_json(text.as_bytes()).is_err());
    }
    let mut value: serde_json::Value = serde_json::from_slice(PROFILE).unwrap();
    value["unknown_rule"] = true.into();
    assert!(ValidationProfile::from_json(&serde_json::to_vec(&value).unwrap()).is_err());
}

#[test]
fn preparation_refuses_an_unknown_application_without_creating_or_mutating_products() {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let before = conn.total_changes();
    let result = prepare_simulated_candidate(&conn, PROFILE, IndividualAddress::from_raw(0x0119));
    assert!(result.is_err());
    assert_eq!(conn.total_changes(), before);
}

#[test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
fn exact_public_package_prepares_full_default_candidate_not_a_handcrafted_plan() {
    let path = std::env::var_os("KNXBENCH_PUBLIC_UP258")
        .map(PathBuf::from)
        .expect("set KNXBENCH_PUBLIC_UP258");
    let bytes = std::fs::read(path).unwrap();
    assert_eq!(
        knx_productdb::sha256_hex(&bytes),
        "520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f"
    );
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package_with_limits(
        &conn,
        "public.knxprod",
        &bytes,
        knx_productdb::PackageLimits::LARGE,
    )
    .unwrap();
    let before = conn.total_changes();
    let result =
        prepare_simulated_candidate(&conn, PROFILE, IndividualAddress::from_raw(0x0119)).unwrap();
    assert_eq!(result.source_positions, 21);
    assert_eq!(result.allocation_octets, 1514);
    assert_eq!(result.written_octets, 760);
    assert_eq!(result.preserved_octets, 754);
    assert!(result.bundle.binding.simulated);
    assert!(result.inferences.iter().all(|s| !s.is_empty()));
    assert_eq!(conn.total_changes(), before);
    assert_eq!(result.bundle.plan().unwrap().source_positions(), 21);
}

use knx_app::property_validation::{self as app, LocalOutcome, OperationStore};
use knx_app::property_validation_bundle::PreparedBundle;
use knx_core::commissioning::{
    load_state::LoadState,
    properties::{PID_HARDWARE_TYPE, PID_PROGRAM_VERSION},
};
use knx_net::commissioning::{
    simulator::{
        Interruption, SimulatedAbsoluteAllocation, SimulatedDevice, SimulatedSystem2Object,
        SimulatedSystem2ObjectState, SimulatedTaskControl2, SimulatedTaskSegment, SimulatorConfig,
    },
    SessionTiming,
};
use std::{sync::atomic::AtomicBool, time::Duration};

fn prepared_public() -> PreparedBundle {
    let bytes = std::fs::read(
        std::env::var_os("KNXBENCH_PUBLIC_UP258").expect("set KNXBENCH_PUBLIC_UP258"),
    )
    .unwrap();
    assert_eq!(
        knx_productdb::sha256_hex(&bytes),
        "520d3fcee5b63d079ac7d0bd5e7ce9467ff84cb5cde06ac341b77aba47e3ee0f"
    );
    let dir = tempfile::tempdir().unwrap();
    let db = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::install_package_with_limits(
        &db,
        "public.knxprod",
        &bytes,
        knx_productdb::PackageLimits::LARGE,
    )
    .unwrap();
    prepare_simulated_candidate(&db, PROFILE, IndividualAddress::from_raw(0x0119))
        .unwrap()
        .bundle
}

fn exact_old_device(bundle: &PreparedBundle, mut config: SimulatorConfig) -> SimulatedDevice {
    // Independent OLD fixture: source-declared extents and identity, NOT an execution
    // of the proposed NEW plan. Unknown real firmware history remains a hypothesis.
    config.mask_version = 0x0025;
    config.serial_number = Some(bundle.binding.serial);
    let device = SimulatedDevice::with_config_at(bundle.plan().unwrap().target(), config);
    device.preset_property(
        0,
        knx_core::commissioning::properties::PID_MANUFACTURER_ID,
        &[0, 1],
    );
    device.preset_property(0, PID_HARDWARE_TYPE, &[0, 1, 0, 0, 0, 91]);
    device.preset_property(3, PID_PROGRAM_VERSION, &[0, 1, 0x92, 5, 4]);
    for (owner, extents, pointer, app_identity) in [
        (
            SimulatedSystem2Object::AddressTable,
            vec![(278, 330)],
            278,
            false,
        ),
        (
            SimulatedSystem2Object::AssociationTable,
            vec![(608, 416)],
            608,
            false,
        ),
        (
            SimulatedSystem2Object::Application,
            vec![(1024, 512), (15360, 256)],
            0,
            true,
        ),
    ] {
        let allocations = extents
            .into_iter()
            .map(|(start, length)| {
                device.preset_memory(u32::from(start), &vec![0xA5; usize::from(length)]);
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
                    manufacturer: if app_identity { 1 } else { 0 },
                    application: if app_identity { 0x9205 } else { 0 },
                    version: if app_identity { 4 } else { 0 },
                }),
                task_control_2: app_identity.then_some(SimulatedTaskControl2 {
                    callback: 0,
                    communication_objects: 1024,
                    segment_1: 0,
                    segment_2: 0,
                }),
            },
        );
    }
    device
}

fn integrated_timing() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(30),
        response_timeout: Duration::from_millis(5),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(30),
        programming_delay: Duration::ZERO,
        ..Default::default()
    }
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_full_download_and_reopened_recovery_compare_all_modeled_persistent_state() {
    let bundle = prepared_public();
    let device = exact_old_device(&bundle, SimulatorConfig::default());
    let old = device.system2_snapshot();
    let dir = tempfile::tempdir().unwrap();
    let bundle_path = dir.path().join("bundle.json");
    std::fs::write(&bundle_path, serde_json::to_vec(&bundle).unwrap()).unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        integrated_timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    let new = device.system2_snapshot();
    assert_ne!(new.memory, old.memory);
    assert_eq!(
        new.objects, old.objects,
        "independent source-shape/task oracle"
    );
    let backup = store.recovery().unwrap();
    assert_eq!(
        backup
            .allocations
            .iter()
            .map(|a| a.data.len())
            .sum::<usize>(),
        1514
    );
    assert_eq!(
        backup.regions.iter().map(|a| a.data.len()).sum::<usize>(),
        760
    );
    drop(store);
    drop(bundle);
    let reopened_bundle: PreparedBundle =
        serde_json::from_slice(&std::fs::read(&bundle_path).unwrap()).unwrap();
    let mut reopened = OperationStore::open(&path, &reopened_bundle.binding).unwrap();
    app::restore_simulated(
        &device,
        &reopened_bundle,
        &mut reopened,
        integrated_timing(),
        &AtomicBool::new(false),
        &reopened_bundle.restore_phrase(),
    )
    .await
    .unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    assert_eq!(
        device.system2_snapshot(),
        old,
        "bytes, ownership, pointers, identity, task controls and properties must all return"
    );
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_partial_download_failure_reopens_and_explicitly_restores_old_state() {
    let bundle = prepared_public();
    let device = exact_old_device(
        &bundle,
        SimulatorConfig {
            interrupt_at: Some(Interruption::MemoryWriteAt(15360)),
            ..Default::default()
        },
    );
    let old = device.system2_snapshot();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    assert!(app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        integrated_timing(),
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(store.mutation_possible());
    assert_ne!(device.system2_snapshot(), old);
    drop(store);
    let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
    app::restore_simulated(
        &device,
        &bundle,
        &mut reopened,
        integrated_timing(),
        &AtomicBool::new(false),
        &bundle.restore_phrase(),
    )
    .await
    .unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    assert_eq!(device.system2_snapshot(), old);
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_cancellation_and_crash_checkpoint_boundaries_restore_from_disk() {
    use knx_app::property_validation::Checkpoint;
    use knx_app::property_validation_bundle::RecoveryRecord;
    use knx_core::commissioning::{
        authorisation::AuthorisationPlan,
        mutation::{WriteAuthorisation, WriteScope},
        property_download::PropertyOperation,
    };
    use knx_net::commissioning::{
        property_download::run_experimental_property_plan, property_recovery::PropertyJournalEvent,
    };
    use std::sync::atomic::Ordering;
    let bundle = prepared_public();
    for (cut_source, cancelled) in [(2, false), (6, false), (12, false), (13, true)] {
        let device = exact_old_device(&bundle, SimulatorConfig::default());
        let old = device.system2_snapshot();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operation");
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        let cancel = AtomicBool::new(false);
        let plan = bundle.plan().unwrap();
        let result = run_experimental_property_plan(&device,&plan,
            WriteAuthorisation::for_simulator(plan.target(),WriteScope::Download).unwrap(),
            AuthorisationPlan::Skip,integrated_timing(),&bundle.preconditions().unwrap(),
            &mut |event| {
                match event {
                    PropertyJournalEvent::RecoveryAcquired(snapshot) => store.checkpoint(Checkpoint::RecoveryAcquired {recovery: RecoveryRecord::from_snapshot(snapshot,&bundle).map_err(|e|e.to_string())?}),
                    PropertyJournalEvent::OperationStarting {operation_index,mutation_may_have_started,..} => store.checkpoint(Checkpoint::StepStarted {index:operation_index,mutation:mutation_may_have_started || matches!(plan.steps()[operation_index].operation,PropertyOperation::LoadControl{..}|PropertyOperation::WriteMemory{..}|PropertyOperation::Disconnect)}),
                    PropertyJournalEvent::OperationCompleted {operation_index,source_index,..} => {
                        store.checkpoint(Checkpoint::StepFinished {index:operation_index}).map_err(|e|e.to_string())?;
                        if source_index == cut_source {
                            if cancelled { cancel.store(true,Ordering::SeqCst); }
                            else {return Err("deterministic crash-boundary journal cut; caller terminal checkpoint intentionally absent".into());}
                        }
                        Ok(())
                    }
                }.map_err(|e|e.to_string())
            },&||cancel.load(Ordering::SeqCst)).await;
        assert!(result.is_err(), "source {cut_source} must stop");
        if cancelled {
            store.checkpoint(Checkpoint::Cancelled).unwrap();
        }
        drop(store);
        let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
        assert_eq!(
            reopened.outcome(),
            if cancelled {
                LocalOutcome::Cancelled
            } else {
                LocalOutcome::Interrupted
            }
        );
        assert!(reopened.mutation_possible());
        app::restore_simulated(
            &device,
            &bundle,
            &mut reopened,
            integrated_timing(),
            &AtomicBool::new(false),
            &bundle.restore_phrase(),
        )
        .await
        .unwrap();
        assert_eq!(
            device.system2_snapshot(),
            old,
            "source {cut_source} old persistent state"
        );
        assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    }
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_incomplete_identity_or_cancelled_recovery_is_durably_failed_not_restored() {
    let bundle = prepared_public();
    for cancel_restore in [false, true] {
        let device = exact_old_device(&bundle, SimulatorConfig::default());
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("operation");
        let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
        app::execute_simulated(
            &device,
            &bundle,
            &mut store,
            integrated_timing(),
            &AtomicBool::new(false),
        )
        .await
        .unwrap();
        if !cancel_restore {
            device.preset_property(3, PID_PROGRAM_VERSION, &[]);
        }
        drop(store);
        let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
        assert!(app::restore_simulated(
            &device,
            &bundle,
            &mut reopened,
            integrated_timing(),
            &AtomicBool::new(cancel_restore),
            &bundle.restore_phrase()
        )
        .await
        .is_err());
        assert_eq!(reopened.outcome(), LocalOutcome::RestoreFailed);
        drop(reopened);
        let mut fresh = OperationStore::open(&path, &bundle.binding).unwrap();
        assert_eq!(fresh.outcome(), LocalOutcome::RestoreFailed);
        assert!(
            app::restore_simulated(
                &device,
                &bundle,
                &mut fresh,
                integrated_timing(),
                &AtomicBool::new(false),
                &bundle.restore_phrase()
            )
            .await
            .is_err(),
            "failed restore must not automatically retry"
        );
    }
}

#[tokio::test]
#[ignore = "requires pinned public UP258 package; set KNXBENCH_PUBLIC_UP258"]
async fn exact_public_destructive_unload_counterhypothesis_is_not_reported_as_restored() {
    let bundle = prepared_public();
    let device = exact_old_device(
        &bundle,
        SimulatorConfig {
            experimental_destroy_allocation_memory_on_unload: true,
            ..Default::default()
        },
    );
    let old = device.system2_snapshot();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    assert!(
        app::execute_simulated(
            &device,
            &bundle,
            &mut store,
            integrated_timing(),
            &AtomicBool::new(false),
        )
        .await
        .is_err(),
        "destructive Unload must fail the full-allocation verification already during NEW"
    );
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    drop(store);
    let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
    assert!(app::restore_simulated(
        &device,
        &bundle,
        &mut reopened,
        integrated_timing(),
        &AtomicBool::new(false),
        &bundle.restore_phrase()
    )
    .await
    .is_err());
    assert_eq!(reopened.outcome(), LocalOutcome::RestoreFailed);
    assert_ne!(
        device.system2_snapshot(),
        old,
        "masked bytes lost by destructive unload are an unresolved physical rule"
    );
}
