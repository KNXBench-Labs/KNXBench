//! Count operation-local reuse at the real retained-source reconstruction seam.
use super::*;
use crate::system2_validation_preparation::{
    count_reconstructions, prepare_retained_project_hardware_source, HardwareObservation,
    ProjectHardwareRecord, RecoveryTrack,
};
#[path = "../tests/it/system2_validation_preparation/fixture.rs"]
mod fixture;

fn bundle() -> PreparedBundle {
    let (_dir, conn, project, _profile) = fixture::configured_inputs();
    let source = prepare_retained_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        "reuse-operation",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    // Offline synthetic observed record: these tests never contact a transport.
    let mut bundle = source.preview().clone();
    bundle.format = 3;
    bundle.binding.simulated = false;
    bundle.binding.serial = [0, 1, 2, 3, 4, 5];
    bundle.hardware_preparation = Some(Box::new(ProjectHardwareRecord {
        pointer_width_policy: None,
        revision: 1,
        operation: "reuse-operation".into(),
        recovery_track: RecoveryTrack::KnxBenchBestEffort,
        recovery_basis:
            "experimental-same-application-source-shape-not-observed-allocator-history-v1".into(),
        observation: HardwareObservation {
            target: bundle.binding.target.clone(),
            serial: bundle.binding.serial,
            mask: bundle.binding.mask,
            manufacturer: bundle.binding.manufacturer,
            hardware: bundle.binding.hardware,
            application: bundle.binding.application,
            observed_application: None,
        },
        source_preparation_sha256: bundle
            .project_preparation
            .as_ref()
            .unwrap()
            .sha256()
            .unwrap(),
    }));
    rebind_records(&mut bundle);
    bundle.plan().unwrap();
    bundle
}

fn rebind_records(bundle: &mut PreparedBundle) {
    let source = bundle
        .project_preparation
        .as_ref()
        .unwrap()
        .sha256()
        .unwrap();
    let hardware = bundle.hardware_preparation.as_mut().unwrap();
    hardware.source_preparation_sha256 = source;
    bundle.binding.project_preparation_sha256 = Some(hardware.sha256().unwrap());
}

#[test]
fn hardware_execution_context_bounds_retained_reconstruction_and_preserves_barrier() {
    let bundle = bundle();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    let (_, count) = count_reconstructions(|| {
        let context = ValidatedHardwareExecution::new(&bundle, &mut store).unwrap();
        assert_eq!(context.preconditions.original_plan, context.plan);
        assert_eq!(context.preconditions.serial, bundle.binding.serial);
        assert_eq!(context.bundle(), &bundle);
        let operation = bundle.consent_phrase();
        let hypothesis = bundle.acceptance_phrase();
        let download = knx_core::commissioning::mutation::required_confirmation_phrase(
            context.plan().target(),
            WriteScope::Download,
        );
        let risk = knx_net::commissioning::hardware_release::required_release_phrase(
            context.plan().target(),
        );
        let confirmations = crate::system2_validation_preparation::HardwareConfirmations {
            operation: &operation,
            hypothesis: &hypothesis,
            download: &download,
            risk: &risk,
        };
        context.check_confirmations(&confirmations).unwrap();
        for wrong in 0..4 {
            let bad = crate::system2_validation_preparation::HardwareConfirmations {
                operation: if wrong == 0 { "wrong" } else { &operation },
                hypothesis: if wrong == 1 { "wrong" } else { &hypothesis },
                download: if wrong == 2 { "wrong" } else { &download },
                risk: if wrong == 3 { "wrong" } else { &risk },
            };
            assert!(context.check_confirmations(&bad).is_err());
        }
        // This is the actual callback used by fresh hardware execution, not a
        // classification copy. Repeated accesses must not reconstruct sources.
        for index in 0..2 {
            let step = &context.plan.steps()[index];
            hardware_event(
                context.store,
                context.bundle,
                &context.plan,
                PropertyJournalEvent::OperationStarting {
                    operation_index: index,
                    source_index: step.source_index,
                    mutation_may_have_started: false,
                },
                false,
            )
            .unwrap();
            hardware_event(
                context.store,
                context.bundle,
                &context.plan,
                PropertyJournalEvent::OperationCompleted {
                    operation_index: index,
                    source_index: step.source_index,
                    mutation_may_have_started: false,
                },
                false,
            )
            .unwrap();
        }
        let before = std::fs::read(path.join("HEAD")).unwrap();
        let mutation_index = 2;
        assert!(matches!(
            context.plan.steps()[mutation_index].operation,
            PropertyOperation::LoadControl { .. }
        ));
        for (operation_index, source_index) in [
            (
                mutation_index,
                context.plan.steps()[mutation_index].source_index,
            ),
            (mutation_index, usize::MAX),
            (usize::MAX, usize::MAX),
        ] {
            assert!(hardware_event(
                context.store,
                context.bundle,
                &context.plan,
                PropertyJournalEvent::OperationStarting {
                    operation_index,
                    source_index,
                    mutation_may_have_started: false,
                },
                false,
            )
            .is_err());
            assert_eq!(std::fs::read(path.join("HEAD")).unwrap(), before);
        }
    });
    assert_eq!(
        count, 1,
        "one actual retained-source reconstruction per fresh execution context"
    );
    assert!(!store.mutation_possible());
    assert!(store.recovery().is_none());
    assert_eq!(store.progress().completed_steps, 2);
    // Public validators stay independent; no bundle/global/persistent cache.
    let (_, count) = count_reconstructions(|| {
        bundle.plan().unwrap();
        bundle.preconditions().unwrap();
    });
    assert_eq!(count, 2);
    let before = std::fs::read(path.join("HEAD")).unwrap();
    assert!(
        ValidatedHardwareExecution::new(&bundle, &mut store).is_err(),
        "no reuse after any execution checkpoint"
    );
    assert_eq!(std::fs::read(path.join("HEAD")).unwrap(), before);
}

#[test]
fn hardware_execution_context_refuses_rebound_source_plan_and_other_operation() {
    let bundle = bundle();
    let dir = tempfile::tempdir().unwrap();
    for fault in ["source-bytes", "source-request", "plan", "operation"] {
        let mut bad = bundle.clone();
        match fault {
            "source-bytes" => {
                let crate::system2_validation_preparation::ProjectProductInput::RetainedSourceMembers { member_bytes, .. } =
                    &mut bad.project_preparation.as_mut().unwrap().product_input
                else { panic!("retained source fixture required"); };
                member_bytes[1][0] ^= 1;
            }
            "source-request" => {
                bad.project_preparation
                    .as_mut()
                    .unwrap()
                    .request
                    .values
                    .clear();
            }
            "plan" => {
                let write = bad
                    .normalized_plan
                    .steps
                    .iter_mut()
                    .find_map(|step| {
                        if let crate::property_validation_bundle::OperationRecord::WriteMemory {
                            data,
                            ..
                        } = &mut step.operation
                        {
                            Some(data)
                        } else {
                            None
                        }
                    })
                    .unwrap();
                write[0] ^= 1;
                bad.binding.plan_sha256 = bad.normalized_plan.sha256().unwrap();
                bad.project_preparation.as_mut().unwrap().plan_sha256 =
                    bad.binding.plan_sha256.clone();
            }
            _ => bad
                .hardware_preparation
                .as_mut()
                .unwrap()
                .operation
                .push_str("-other"),
        }
        // Recompute enclosing hashes deliberately: retained bytes/reconstruction
        // must reject the source/plan faults; exact store binding rejects the last.
        rebind_records(&mut bad);
        let expected = if fault == "operation" {
            &bundle.binding
        } else {
            &bad.binding
        };
        let path = dir.path().join(fault);
        let mut store = OperationStore::create(&path, expected.clone()).unwrap();
        let head = std::fs::read(path.join("HEAD")).unwrap();
        let (refused, count) =
            count_reconstructions(|| ValidatedHardwareExecution::new(&bad, &mut store).is_err());
        assert!(refused, "mismatched {fault} admitted");
        assert_eq!(
            count, 1,
            "refusal must reach actual retained-source equivalence validation"
        );
        assert_eq!(std::fs::read(path.join("HEAD")).unwrap(), head);
        assert_eq!(store.progress().completed_steps, 0);
        assert!(!store.mutation_possible());
        assert!(store.recovery().is_none());
    }
}

// Synthetic old bytes over the existing retained-source fixture's exact shape.
// This is callback/state-machine coverage, not an observed device backup.
fn recovery_store(path: &std::path::Path, bundle: &PreparedBundle) -> OperationStore {
    use crate::property_validation_bundle::{RegionRecord, ResourceRecord};
    let plan = bundle.plan().unwrap();
    let preconditions = bundle.preconditions().unwrap();
    let recovery = RecoveryRecord {
        version: 1,
        original_bundle: bundle.clone(),
        old_device_control: 0,
        resources: preconditions
            .source_pointers
            .iter()
            .map(|p| ResourceRecord {
                object: p.object,
                state: knx_core::commissioning::load_state::LoadState::Loaded.octet(),
                pointer: p.value.into(),
                pointer_refused: false,
            })
            .collect(),
        allocations: plan
            .steps()
            .iter()
            .enumerate()
            .filter_map(|(operation_index, step)| {
                let PropertyOperation::LoadControl { object, payload } = &step.operation else {
                    return None;
                };
                let bytes = payload.octets();
                if bytes[0] != 3 || bytes[1] > 1 {
                    return None;
                }
                Some(RegionRecord {
                    operation_index,
                    source_index: step.source_index,
                    object: *object,
                    address: u16::from_be_bytes([bytes[2], bytes[3]]),
                    data: vec![0x5a; usize::from(u16::from_be_bytes([bytes[4], bytes[5]]))],
                })
            })
            .collect(),
        regions: plan
            .steps()
            .iter()
            .enumerate()
            .filter_map(|(operation_index, step)| {
                let PropertyOperation::WriteMemory {
                    object,
                    address,
                    data,
                } = &step.operation
                else {
                    return None;
                };
                Some(RegionRecord {
                    operation_index,
                    source_index: step.source_index,
                    object: *object,
                    address: *address,
                    data: vec![0x5a; data.len()],
                })
            })
            .collect(),
        prestate_observations: Vec::new(),
        observed_serial: None,
        observed_program_version: None,
        prestate_unreadable_allocations: Vec::new(),
    };
    let mut store = OperationStore::create(path, bundle.binding.clone()).unwrap();
    store
        .checkpoint(Checkpoint::RecoveryAcquired { recovery })
        .unwrap();
    store.checkpoint(Checkpoint::Cancelled).unwrap();
    drop(store);
    OperationStore::open(path, &bundle.binding).unwrap()
}

fn journal_bytes(path: &std::path::Path) -> Vec<(std::ffi::OsString, Vec<u8>)> {
    let mut files: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (entry.file_name(), std::fs::read(entry.path()).unwrap())
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    files
}

#[test]
fn hardware_recovery_callbacks_reuse_original_plan_and_preserve_original_barrier() {
    let bundle = bundle();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation");
    let mut store = recovery_store(&path, &bundle);
    check_bundle(&bundle, &store, false).unwrap();
    store.require_restore_available().unwrap();
    let snapshot = store.recovery().unwrap().to_snapshot(&bundle).unwrap();
    let (plan, count) = count_reconstructions(|| bundle.plan().unwrap());
    assert_eq!(count, 1, "fully reconstruct the original plan before reuse");
    assert_ne!(snapshot.recovery_plan(&plan).unwrap(), plan);
    store.checkpoint(Checkpoint::RestoreStarted).unwrap();

    let before = journal_bytes(&path);
    let progress = store.progress();
    // Wrong in-range order, wrong source position and out-of-range position.
    for (index, source, reason) in [
        (
            1,
            plan.steps()[1].source_index,
            "step start is out of order or operation already ended",
        ),
        (0, usize::MAX, "journal source position differs"),
        (
            usize::MAX,
            usize::MAX,
            "journal operation is outside exact plan",
        ),
    ] {
        let (result, count) = count_reconstructions(|| {
            hardware_event(
                &mut store,
                &bundle,
                &plan,
                PropertyJournalEvent::OperationStarting {
                    operation_index: index,
                    source_index: source,
                    mutation_may_have_started: false,
                },
                true,
            )
        });
        assert_eq!(result, Err(reason.into()));
        assert_eq!(count, 0);
        assert_eq!(journal_bytes(&path), before);
        assert_eq!(store.progress(), progress);
    }

    for (index, step) in plan.steps().iter().enumerate() {
        // All original operation classes, including writes and Disconnect, use
        // the actual restoring callback. No counter scope crosses an await.
        let previous_mutation = matches!(step.operation, PropertyOperation::ToolMetadata { .. });
        let (_, count) = count_reconstructions(|| {
            hardware_event(
                &mut store,
                &bundle,
                &plan,
                PropertyJournalEvent::OperationStarting {
                    operation_index: index,
                    source_index: step.source_index,
                    mutation_may_have_started: previous_mutation,
                },
                true,
            )
            .unwrap()
        });
        assert_eq!(
            count, 0,
            "operation-start must not reconstruct retained sources"
        );
        let head: serde_json::Value =
            serde_json::from_slice(&std::fs::read(path.join("HEAD")).unwrap()).unwrap();
        let sequence = head["sequence"].as_u64().unwrap();
        let checkpoint: serde_json::Value = serde_json::from_slice(
            &std::fs::read(path.join(format!("checkpoint-{sequence:08}.json"))).unwrap(),
        )
        .unwrap();
        assert_eq!(
            checkpoint["checkpoint"],
            serde_json::json!({
                "event": "step_started",
                "index": index,
                "mutation": previous_mutation || matches!(step.operation,
                    PropertyOperation::LoadControl { .. }
                    | PropertyOperation::WriteMemory { .. }
                    | PropertyOperation::Disconnect),
            })
        );
        if matches!(step.operation, PropertyOperation::HardwareCheck { .. }) {
            let before = journal_bytes(&path);
            let progress = store.progress();
            let mut other = snapshot.clone();
            other.old_device_control ^= 1;
            // Structurally valid evidence is still NOT the durable original.
            RecoveryRecord::from_snapshot(&other, &bundle).unwrap();
            let (result, count) = count_reconstructions(|| {
                hardware_event(
                    &mut store,
                    &bundle,
                    &plan,
                    PropertyJournalEvent::RecoveryAcquired(&other),
                    true,
                )
            });
            assert_eq!(
                result,
                Err("restore snapshot differs from durable original".into())
            );
            assert_eq!(count, 4, "original snapshot validation stays independent");
            assert_eq!(journal_bytes(&path), before);
            assert_eq!(store.progress(), progress);
            let (_, count) = count_reconstructions(|| {
                hardware_event(
                    &mut store,
                    &bundle,
                    &plan,
                    PropertyJournalEvent::RecoveryAcquired(&snapshot),
                    true,
                )
                .unwrap()
            });
            assert_eq!(count, 4);
            assert_eq!(
                journal_bytes(&path),
                before,
                "restore compares, never appends a new backup"
            );
        }
        hardware_event(
            &mut store,
            &bundle,
            &plan,
            PropertyJournalEvent::OperationCompleted {
                operation_index: index,
                source_index: step.source_index,
                mutation_may_have_started: false,
            },
            true,
        )
        .unwrap();
    }
    assert_eq!(store.progress().completed_steps, plan.steps().len());
    assert_eq!(
        store.recovery().unwrap().to_snapshot(&bundle).unwrap(),
        snapshot
    );
    // These are synthetic journal transitions only, never a device verdict.
    store.checkpoint(Checkpoint::Restored).unwrap();
    drop(store);
    let mut store = OperationStore::open(&path, &bundle.binding).unwrap();
    let before = journal_bytes(&path);
    assert!(store.require_restore_available().is_err());
    assert!(store.checkpoint(Checkpoint::RestoreStarted).is_err());
    assert_eq!(journal_bytes(&path), before);
}

#[test]
fn hardware_recovery_reuse_keeps_independent_original_store_binding_admission() {
    let bundle = bundle();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("operation");
    let store = recovery_store(&path, &bundle);
    let mut other = bundle.clone();
    other
        .hardware_preparation
        .as_mut()
        .unwrap()
        .operation
        .push_str("-other");
    rebind_records(&mut other);
    let plan = other.plan().unwrap();
    let recovery = other.restore_phrase();
    let download = knx_core::commissioning::mutation::required_confirmation_phrase(
        plan.target(),
        WriteScope::Download,
    );
    let risk = knx_net::commissioning::hardware_release::required_release_phrase(plan.target());
    let confirmations = crate::system2_validation_preparation::HardwareRecoveryConfirmations {
        recovery: &recovery,
        download: &download,
        risk: &risk,
    };
    let before = journal_bytes(&path);
    let progress = store.progress();
    let (result, count) = count_reconstructions(|| check_bundle(&other, &store, false));
    assert_eq!(
        result.unwrap_err().to_string(),
        "operation-store binding differs from bundle kind"
    );
    assert_eq!(
        count, 2,
        "inner recovery source validation stays independent"
    );
    let (result, count) = count_reconstructions(|| {
        crate::system2_validation_preparation::validate_project_hardware_recovery(
            &other,
            &store,
            &confirmations,
        )
    });
    assert_eq!(
        result.unwrap_err().to_string(),
        "separate configured hardware preparation and matching operation required"
    );
    assert_eq!(count, 1);
    assert_eq!(journal_bytes(&path), before);
    assert_eq!(store.progress(), progress);
    store.require_restore_available().unwrap();
}

#[test]
fn genuine_pointer_observations_reuse_the_validated_original_snapshot() {
    use crate::property_validation_bundle::{PointerObservationRecord, PointerRecord};
    let bundle = bundle();
    let dir = tempfile::tempdir().unwrap();
    let recovery = recovery_store(&dir.path().join("seed"), &bundle)
        .recovery()
        .unwrap()
        .clone();
    let (object, expected) = {
        let source = &bundle.preconditions().unwrap().source_pointers[0];
        (source.object, source.value)
    };
    let difference = || Checkpoint::FinalPointerObservations {
        observations: vec![PointerObservationRecord {
            object,
            expected,
            observed: Some(PointerRecord::Four([9, 9, 9, 9])),
            state: knx_core::commissioning::load_state::LoadState::Loaded.octet(),
        }],
    };
    let path = dir.path().join("live");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    let (acquired, initial) =
        count_reconstructions(|| store.checkpoint(Checkpoint::RecoveryAcquired { recovery }));
    acquired.unwrap();
    assert!(initial >= 1, "the one validation of the original recovery");
    store
        .checkpoint(Checkpoint::StepStarted {
            index: 0,
            mutation: true,
        })
        .unwrap();
    let (_, repeated) = count_reconstructions(|| {
        for _ in 0..3 {
            store.checkpoint(difference()).unwrap();
        }
    });
    assert_eq!(repeated, 0, "observations reuse the validated snapshot");
    drop(store);
    // Reopen validates the recovery once, like the live acquisition, then reuses it.
    let (reopened, on_open) =
        count_reconstructions(|| OperationStore::open(&path, &bundle.binding));
    let mut reopened = reopened.unwrap();
    assert_eq!(
        on_open, initial,
        "replay of three observations adds no reconstruction"
    );
    let (_, after) = count_reconstructions(|| reopened.checkpoint(difference()).unwrap());
    assert_eq!(after, 0);
    assert_eq!(reopened.progress().pointer_observations.len(), 4);
}

// Journal-only derivation of the download result: no message, count or free text is read.
mod download_result {
    use super::*;
    use crate::property_validation::{DownloadResult, LocalOutcome};

    fn failed() -> Checkpoint {
        Checkpoint::Failed {
            code: crate::property_validation::FailureCode::IdentityOrRecovery,
            local_diagnostic: "private text that must never decide anything".into(),
            pointer_failure: None,
        }
    }

    /// Journal with `finished` completed steps, optionally the next one started, then `end`.
    fn run(
        dir: &std::path::Path,
        bundle: &PreparedBundle,
        acquired: bool,
        finished: usize,
        started_next: bool,
        end: Option<Checkpoint>,
    ) -> OperationStore {
        let mut store = OperationStore::create(dir, bundle.binding.clone()).unwrap();
        if acquired {
            let recovery = recovery_store(&dir.with_extension("seed"), bundle)
                .recovery()
                .unwrap()
                .clone();
            store
                .checkpoint(Checkpoint::RecoveryAcquired { recovery })
                .unwrap();
        }
        for index in 0..finished {
            store
                .checkpoint(Checkpoint::StepStarted {
                    index,
                    mutation: acquired,
                })
                .unwrap();
            store
                .checkpoint(Checkpoint::StepFinished { index })
                .unwrap();
        }
        if started_next {
            store
                .checkpoint(Checkpoint::StepStarted {
                    index: finished,
                    mutation: acquired,
                })
                .unwrap();
        }
        if let Some(end) = end {
            store.checkpoint(end).unwrap();
        }
        store
    }

    fn total(bundle: &PreparedBundle) -> usize {
        bundle.plan().unwrap().steps().len()
    }

    #[test]
    fn last_plan_step_hosts_the_verification() {
        let plan = bundle().plan().unwrap();
        assert!(matches!(
            plan.steps().last().unwrap().operation,
            PropertyOperation::Disconnect
        ));
    }

    #[test]
    fn failure_inside_the_final_step_is_transfer_complete_but_unverified() {
        let bundle = bundle();
        let n = total(&bundle);
        let dir = tempfile::tempdir().unwrap();
        let store = run(
            &dir.path().join("j"),
            &bundle,
            true,
            n - 1,
            true,
            Some(failed()),
        );
        let progress = store.progress();
        assert_eq!(progress.outcome, LocalOutcome::Failed);
        assert_eq!(
            (progress.completed_steps, progress.total_steps),
            (n - 1, Some(n))
        );
        assert_eq!(
            progress.download.result,
            DownloadResult::TransferCompleteVerificationUnfinished
        );
    }

    #[test]
    fn cancel_before_the_final_step_starts_never_started_verification() {
        let bundle = bundle();
        let n = total(&bundle);
        let dir = tempfile::tempdir().unwrap();
        let store = run(
            &dir.path().join("j"),
            &bundle,
            true,
            n - 1,
            false,
            Some(Checkpoint::Cancelled),
        );
        assert_eq!(
            store.progress().download.result,
            DownloadResult::TransferCompleteVerificationNotStarted
        );
    }

    #[test]
    fn failure_before_the_last_transfer_step_is_incomplete() {
        let bundle = bundle();
        let n = total(&bundle);
        let dir = tempfile::tempdir().unwrap();
        for done in [2, n / 2, n - 2] {
            let path = dir.path().join(format!("j{done}"));
            let store = run(&path, &bundle, true, done, true, Some(failed()));
            assert_eq!(
                store.progress().download.result,
                DownloadResult::TransferIncomplete,
                "{done}"
            );
        }
    }

    #[test]
    fn ended_without_acquired_recovery_is_nothing_written() {
        let bundle = bundle();
        let dir = tempfile::tempdir().unwrap();
        let store = run(
            &dir.path().join("j"),
            &bundle,
            false,
            1,
            true,
            Some(failed()),
        );
        assert!(!store.progress().mutation_possible);
        assert_eq!(
            store.progress().download.result,
            DownloadResult::NothingWritten
        );
    }

    #[test]
    fn finished_is_verified_only_when_every_step_ran() {
        let bundle = bundle();
        let n = total(&bundle);
        let dir = tempfile::tempdir().unwrap();
        let store = run(
            &dir.path().join("j"),
            &bundle,
            true,
            n,
            false,
            Some(Checkpoint::Finished),
        );
        assert_eq!(store.progress().download.result, DownloadResult::Verified);
    }

    #[test]
    fn all_steps_done_without_a_finished_checkpoint_stays_unknown() {
        let bundle = bundle();
        let n = total(&bundle);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("j");
        drop(run(&path, &bundle, true, n, false, None));
        let reopened = OperationStore::open(&path, &bundle.binding).unwrap();
        assert_eq!(reopened.progress().outcome, LocalOutcome::Interrupted);
        assert_eq!(reopened.progress().download.result, DownloadResult::Unknown);
    }

    #[test]
    fn running_is_in_progress_and_reopened_running_is_derived_from_steps() {
        let bundle = bundle();
        let n = total(&bundle);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("j");
        let live = run(&path, &bundle, true, n - 1, true, None);
        assert_eq!(live.progress().download.result, DownloadResult::InProgress);
        drop(live);
        let reopened = OperationStore::open(&path, &bundle.binding).unwrap();
        assert_eq!(reopened.progress().outcome, LocalOutcome::Interrupted);
        assert_eq!(
            reopened.progress().download.result,
            DownloadResult::TransferCompleteVerificationUnfinished
        );
    }

    #[test]
    fn restore_keeps_the_download_result_and_survives_reopen() {
        let bundle = bundle();
        let n = total(&bundle);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("j");
        drop(run(&path, &bundle, true, n - 1, true, Some(failed())));
        let mut store = OperationStore::open(&path, &bundle.binding).unwrap();
        store.checkpoint(Checkpoint::RestoreStarted).unwrap();
        assert_eq!(
            store.progress().outcome,
            LocalOutcome::Interrupted,
            "reopened store never claims a live restore"
        );
        assert_eq!(
            store.progress().download.result,
            DownloadResult::TransferCompleteVerificationUnfinished
        );
        // The download keeps its own ended outcome and counters; the recovery has its own.
        let progress = store.progress();
        assert!(progress.restore_attempted);
        assert_eq!(progress.download.outcome, LocalOutcome::Failed);
        assert_eq!(
            (
                progress.download.completed_steps,
                progress.download.total_steps
            ),
            (n - 1, Some(n))
        );
        assert_eq!(
            progress.completed_steps, 0,
            "current counters are the recovery's"
        );
        drop(store);
        let reopened = OperationStore::open(&path, &bundle.binding).unwrap();
        assert_eq!(
            reopened.progress().download.result,
            DownloadResult::TransferCompleteVerificationUnfinished
        );
        assert_eq!(reopened.progress().download.outcome, LocalOutcome::Failed);
        assert!(reopened.progress().restore_attempted);
    }

    #[test]
    fn a_verified_download_stays_verified_in_its_own_facts_after_a_recovery_started() {
        let bundle = bundle();
        let n = total(&bundle);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("j");
        drop(run(
            &path,
            &bundle,
            true,
            n,
            false,
            Some(Checkpoint::Finished),
        ));
        let mut store = OperationStore::open(&path, &bundle.binding).unwrap();
        store.checkpoint(Checkpoint::RestoreStarted).unwrap();
        let p = store.progress();
        assert_eq!(
            (p.download.result, p.download.outcome),
            (DownloadResult::Verified, LocalOutcome::Finished)
        );
        assert!(p.restore_attempted);
    }
}
