//! Real caption workers skip source reads after project replacement, removal or cancellation.
use super::contention::{binding, command};
use super::owned;
use super::*;
use crate::device_system2::settlement_observation::Observation;
use knx_app::property_validation::DownloadResult;

#[derive(Debug, Clone, Copy)]
enum End {
    OtherProject,
    Removed,
    ReusedAddress,
    Cancelled,
}
impl End {
    fn receipt(self) -> &'static str {
        match self {
            Self::OtherProject => "otherProject",
            Self::Removed | Self::ReusedAddress => "deviceMissing",
            Self::Cancelled => "sessionOnly",
        }
    }
}
fn without_memory_claims(project: &knx_core::Project) -> knx_core::Project {
    let mut expected = project.clone();
    let commissioning = &mut expected.devices.get_mut(f::DEVICE).unwrap().commissioning;
    commissioning.application_program_loaded = false;
    commissioning.parameters_loaded = false;
    commissioning.communication_part_loaded = false;
    expected
}

async fn run(namespace: u8, retained: bool, end: End, marked: bool) {
    let input = f::inputs(namespace, retained);
    let data = input.dir.path();
    let mut project = input.project.clone();
    let d = project.devices.get_mut(f::DEVICE).unwrap();
    d.address = Some(TARGET.parse().unwrap());
    d.commissioning.medium_config_loaded = true;
    d.commissioning.last_download = Some("2001-02-03T04:05:06Z".parse().unwrap());
    let hardware = Arc::new(Hardware {
        device: old_device(),
        paused: AtomicBool::new(true),
        reached: Notify::new(),
        release: Notify::new(),
    });
    drop(input.conn);
    let products = data.join("caption-products.sqlite");
    let state = state_for(data, products.clone(), project, &hardware);
    let router = crate::app(state.clone(), None);
    let (observation, _events) = Observation::new();
    *state.device_system2.settlement_observer.lock().unwrap() = Some(observation.clone());
    let mut ownership = owned::Owned::new(&state, &hardware, &observation);
    let result = owned::catch(async {
        // Old programmed claims in both initial-belief populations make forgetting observable.
        mark(&state);
        if !marked {
            command(
                &state,
                knx_core::Command::SetDeviceProgramming {
                    device: f::DEVICE,
                    change: ProgrammingChange::mark(ProgrammingParts::ALL, false),
                },
            );
        }
        command(&state, knx_core::Command::RenameDevice { device: f::DEVICE, name: "Lifecycle redo".into() });
        crate::domain::undo_impl(&state).unwrap();
        let preparation = preview(&state, &router, &input.profile, retained).await;
        http(
            &router,
            "/api/device-system2/start",
            Some(start(&preparation)),
            200,
        )
        .await;
        tokio::time::timeout(Duration::from_secs(60), hardware.reached.notified())
            .await
            .unwrap();
        let root = data.join(crate::device_system2::ROOT_NAME);
        let id = std::fs::read_to_string(root.join("selected")).unwrap();
        let operation = root.join(&id);
        let observed = std::fs::read(operation.join("observed.json")).unwrap();
        let original = read_bundle(&operation.join("observed.json")).unwrap();
        let backup = owned::backup_at_pause(&operation, &original);
        let metadata = std::fs::read(operation.join("metadata.json")).unwrap();
        let replacement = data.join("lifecycle-replacement.sqlite");
        f::replacement(
            &replacement,
            &f::caption(&input.package),
            &input.package,
            false,
        );
        state
            .product_db
            .as_ref()
            .unwrap()
            .restore_legacy(&products, &replacement)
            .unwrap();
        {
            let project = state.project.lock().unwrap();
            state
                .product_db
                .as_ref()
                .unwrap()
                .with_read(|conn| {
                    assert_eq!(
                        knx_app::system2_validation_preparation::current_project_hardware_parts(
                            conn,
                            project.as_ref().unwrap(),
                            f::DEVICE,
                            &original
                        )
                        .unwrap(),
                        ProgrammingParts::ALL
                    );
                    Ok::<_, std::io::Error>(())
                })
                .unwrap()
                .unwrap();
        }
        match end {
            End::OtherProject => {
                let mut same_id = state
                    .project
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .devices
                    .get(f::DEVICE)
                    .unwrap()
                    .clone();
                same_id.com_objects.clear();
                crate::domain::new_project_impl(
                    &state,
                    Some("Another synthetic project".into()),
                    None,
                    None,
                    None,
                    true,
                )
                .unwrap();
                crate::domain::apply_bound_command(
                    &state,
                    &binding(&state),
                    "Replacement project device",
                    String::new(),
                    |project| {
                        same_id.id = project.ids.next_device_id().map_err(|e| e.to_string())?;
                        assert_eq!(
                            same_id.id,
                            f::DEVICE,
                            "same numeric id/address, different incarnation"
                        );
                        Ok(knx_core::Command::CreateDevice {
                            device: same_id,
                            com_objects: vec![],
                            program_defaults: vec![],
                            installation: None,
                            line: None,
                        })
                    },
                )
                .unwrap();
            }
            End::Removed | End::ReusedAddress => {
                let mut replacement = state
                    .project
                    .lock()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .devices
                    .get(f::DEVICE)
                    .unwrap()
                    .clone();
                let unlink = {
                    let lock = state.project.lock().unwrap();
                    let project = lock.as_ref().unwrap();
                    replacement
                        .com_objects
                        .iter()
                        .flat_map(|id| {
                            project
                                .devices
                                .com_object(*id)
                                .unwrap()
                                .links
                                .iter()
                                .map(move |l| knx_core::Command::UnlinkComObject {
                                    com_object: *id,
                                    ga: l.ga,
                                    direction: l.direction,
                                })
                        })
                        .collect::<Vec<_>>()
                };
                for unlink in unlink {
                    command(&state, unlink);
                }
                // DeleteDevice deliberately refuses dependent data; remove this fixture's
                // parameter rows through their validated undoable inverse before deletion.
                let parameters = {
                    let project = state.project.lock().unwrap();
                    project.as_ref().unwrap().installations[0]
                        .parameters
                        .iter()
                        .filter(|parameter| parameter.device == f::DEVICE)
                        .map(|parameter| knx_core::Command::RestoreParameterValue {
                            id: parameter.id,
                            device: f::DEVICE,
                            ets_id: parameter.source.ets_id.clone(),
                            raw: None,
                        })
                        .collect::<Vec<_>>()
                };
                assert!(!parameters.is_empty(), "non-vacuous dependent-data fixture");
                for parameter in parameters {
                    command(&state, parameter);
                }
                command(&state, knx_core::Command::DeleteDevice { id: f::DEVICE });
                assert!(state.project.lock().unwrap().as_ref().unwrap().devices.get(f::DEVICE).is_none());
                if matches!(end, End::ReusedAddress) {
                    replacement.com_objects.clear();
                    crate::domain::apply_bound_command(
                        &state,
                        &binding(&state),
                        "Reused target",
                        String::new(),
                        |project| {
                            replacement.id = project.ids.next_device_id().map_err(|e| e.to_string())?;
                            assert_ne!(replacement.id, f::DEVICE);
                            Ok(knx_core::Command::CreateDevice {
                                device: replacement,
                                com_objects: vec![],
                                program_defaults: vec![],
                                installation: None,
                                line: None,
                            })
                        },
                    )
                    .unwrap();
                }
            }
            End::Cancelled => {
                http(
                    &router,
                    "/api/device-system2/cancel",
                    Some(json!({"operationId": id})),
                    200,
                )
                .await;
            }
        }
        let before = state.project.lock().unwrap().as_ref().unwrap().clone();
        let stack_before = state.command_stack.lock().unwrap().clone();
        let revision = crate::domain::current_project_revision(&state);
        hardware.release.notify_one();
        let cancelled = matches!(end, End::Cancelled);
        super::super::failure_diagnostic::settled_slow(
            &state,
            &router,
            if cancelled { "cancelled" } else { "finished" },
        )
        .await;
        let counts = observation.counts();
        assert_eq!(
            (
                counts.callbacks,
                counts.attempts,
                counts.leases,
                counts.proofs
            ),
            (0, 0, 0, 0),
            "{end:?}: actual worker must not consult source"
        );
        let after = state.project.lock().unwrap().as_ref().unwrap().clone();
        let stack_after = state.command_stack.lock().unwrap().clone();
        if cancelled {
            assert_eq!(flags(&state), [marked, false, false, false, true]);
            assert_eq!(after, without_memory_claims(&before),
                "exact current snapshot: only MEMORY belief may change; address/medium/timestamps/config preserved");
            assert_eq!(crate::domain::current_project_revision(&state), revision + 1);
            assert_eq!(stack_after.history_lengths(), stack_before.history_lengths());
            assert_eq!(stack_after.comparison_revision(f::DEVICE), stack_before.comparison_revision(f::DEVICE) + 1);
            let (old_undo, old_redo) = stack_before.snapshot_states(&before).unwrap();
            assert!(!old_undo.is_empty() && !old_redo.is_empty(), "both histories must be observable");
            let (undo, redo) = stack_after.snapshot_states(&after).unwrap();
            assert_eq!(undo, old_undo.iter().map(without_memory_claims).collect::<Vec<_>>(),
                "every undo snapshot preserves all fields except stale MEMORY claims");
            assert_eq!(redo, old_redo.iter().map(without_memory_claims).collect::<Vec<_>>(),
                "every redo snapshot preserves all fields except stale MEMORY claims");
        } else {
            assert_eq!(
                after, before,
                "{end:?}: complete post-command project preserved"
            );
            assert_eq!(
                stack_after, stack_before,
                "{end:?}: complete post-command history preserved"
            );
            assert_eq!(crate::domain::current_project_revision(&state), revision);
        }
        let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
        let progress = store.progress();
        assert_eq!(
            progress.outcome,
            if cancelled {
                LocalOutcome::Cancelled
            } else {
                LocalOutcome::Finished
            }
        );
        assert_eq!(progress.download.outcome, progress.outcome);
        assert_eq!(
            progress.download.result,
            if cancelled {
                DownloadResult::TransferIncomplete
            } else {
                DownloadResult::Verified
            }
        );
        assert!(progress.mutation_possible && !progress.restore_attempted);
        assert_eq!(store.recovery(), Some(&backup), "complete original recovery preserved across settlement");
        drop(store);
        for _ in 0..2 {
            let status = http(&router, "/api/device-system2/status", None, 200).await;
            assert_eq!(status["projectSettlement"], end.receipt());
            let report = http(
                &router,
                &format!("/api/device-system2/report?operationId={id}"),
                None,
                200,
            )
            .await;
            assert_eq!(
                report["download"]["result"],
                if cancelled {
                    "transferIncomplete"
                } else {
                    "verified"
                }
            );
            assert_eq!(
                report["download"]["completedSteps"],
                progress.download.completed_steps
            );
            let reopened = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
            assert_eq!(reopened.progress(), progress);
            assert_eq!(reopened.recovery(), Some(&backup), "complete original recovery preserved across status/report");
        }
        let selected = http(
            &router,
            "/api/device-system2/select",
            Some(json!({"operationId": id})),
            200,
        )
        .await;
        assert_eq!(
            selected["projectSettlement"], "notRecorded",
            "selection reattaches facts without settlement"
        );
        let reopened = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
        assert_eq!(
            reopened.progress(),
            progress,
            "selection/reopen never rewrite durable facts"
        );
        assert_eq!(reopened.recovery(), Some(&backup), "complete original recovery preserved across select/reopen");
        drop(reopened);
        assert_eq!(observation.counts(), counts);
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap(), &after);
        assert_eq!(*state.command_stack.lock().unwrap(), stack_after);
        assert_eq!(
            std::fs::read(operation.join("observed.json")).unwrap(),
            observed
        );
        assert_eq!(
            std::fs::read(operation.join("metadata.json")).unwrap(),
            metadata
        );
        eprintln!("E14_E16_LIFECYCLE family={} namespace={namespace} end={end:?} marked={marked} callbacks=0 leases=0 proofs=0", if retained { "retained" } else { "standalone" });
    }).await;
    let cleanup = ownership.finish(result.is_err()).await;
    owned::propagate(result, cleanup);
}

#[tokio::test]
async fn caption_lifecycle_retained_target_gates_and_cancel_skip_actual_source_reads() {
    for namespace in [20, 23] {
        for end in [
            End::OtherProject,
            End::Removed,
            End::ReusedAddress,
            End::Cancelled,
        ] {
            for marked in [false, true] {
                run(namespace, true, end, marked).await;
            }
        }
    }
}
#[tokio::test]
async fn caption_lifecycle_standalone_target_gates_and_cancel_skip_actual_source_reads() {
    for namespace in [20, 23] {
        for end in [
            End::OtherProject,
            End::Removed,
            End::ReusedAddress,
            End::Cancelled,
        ] {
            for marked in [false, true] {
                run(namespace, false, end, marked).await;
            }
        }
    }
}
