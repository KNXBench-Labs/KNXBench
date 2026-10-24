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
