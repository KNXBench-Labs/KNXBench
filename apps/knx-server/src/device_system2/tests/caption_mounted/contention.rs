//! Actual terminal workers contend with managed writers while holding one project candidate.
use super::owned;
use super::*;
use crate::device_system2::settlement_observation::{Event, Observation};
use knx_app::property_validation::DownloadResult;
use knx_productdb::managed_catalogue::{LegacyMutationKind, LegacySettlementHint};
use std::{sync::mpsc, time::Instant};

async fn next(receiver: &Arc<Mutex<mpsc::Receiver<Event>>>) -> Event {
    let receiver = receiver.clone();
    tokio::task::spawn_blocking(move || {
        receiver
            .lock()
            .unwrap()
            .recv_timeout(Duration::from_secs(60))
            .expect("actual terminal rendezvous")
    })
    .await
    .unwrap()
}
pub(super) fn binding(state: &crate::SharedState) -> crate::domain::SnapshotBinding {
    crate::domain::SnapshotBinding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::SeqCst),
        snapshot_revision: crate::domain::current_project_revision(state),
    }
}
pub(super) fn command(state: &crate::SharedState, command: knx_core::Command) {
    crate::domain::apply_bound_prepared(
        state,
        &binding(state),
        "Caption contention fixture",
        command,
    )
    .unwrap();
}
pub(super) fn project_flags(project: &knx_core::Project) -> [bool; 5] {
    let c = &project.devices.get(f::DEVICE).unwrap().commissioning;
    [
        c.individual_address_loaded,
        c.application_program_loaded,
        c.parameters_loaded,
        c.communication_part_loaded,
        c.medium_config_loaded,
    ]
}
pub(super) fn configuration(project: &knx_core::Project) -> knx_core::Project {
    let mut copy = project.clone();
    let c = &mut copy.devices.get_mut(f::DEVICE).unwrap().commissioning;
    c.individual_address_loaded = false;
    c.application_program_loaded = false;
    c.parameters_loaded = false;
    c.communication_part_loaded = false;
    c.last_download = None;
    copy
}

async fn run(namespace: u8, retained: bool, ambiguous: bool, fail_after_command_block: bool) {
    let input = f::inputs(namespace, retained);
    let data = input.dir.path();
    let mut project = input.project.clone();
    let d = project.devices.get_mut(f::DEVICE).unwrap();
    d.address = Some(TARGET.parse().unwrap());
    d.commissioning.medium_config_loaded = true;
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
    let (observation, receiver) = Observation::new();
    *state.device_system2.settlement_observer.lock().unwrap() = Some(observation.clone());
    let receiver = Arc::new(Mutex::new(receiver));
    let mut ownership = owned::Owned::new(&state, &hardware, &observation);
    let result = owned::catch(async {
        mark(&state);
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
        let metadata = std::fs::read(operation.join("metadata.json")).unwrap();
        let original = read_bundle(&operation.join("observed.json")).unwrap();
        let backup = owned::backup_at_pause(&operation, &original);
        let changed = f::caption(&input.package);
        let replacement = data.join("contention-replacement.sqlite");
        f::replacement(&replacement, &changed, &input.package, false);
        state
            .product_db
            .as_ref()
            .unwrap()
            .restore_legacy(&products, &replacement)
            .unwrap();
        // A real parameter command makes the successful comparison selective, killing credit-ALL.
        let parameter = {
            let project = state.project.lock().unwrap();
            project.as_ref().unwrap().installations[0]
                .parameters
                .iter()
                .find(|p| p.source.ets_id == format!("{}_R", f::PROGRAM))
                .unwrap()
                .clone()
        };
        command(
            &state,
            knx_core::Command::SetParameterValue {
                id: parameter.id,
                device: f::DEVICE,
                ets_id: parameter.source.ets_id,
                raw: "772".into(),
            },
        );
        command(
            &state,
            knx_core::Command::RenameDevice {
                device: f::DEVICE,
                name: "Redo caption".into(),
            },
        );
        crate::domain::undo_impl(&state).unwrap();
        let before = state.project.lock().unwrap().as_ref().unwrap().clone();
        let stack_before = state.command_stack.lock().unwrap().clone();
        let (old_undo, old_redo) = stack_before.snapshot_states(&before).unwrap();
        assert!(!old_undo.is_empty() && !old_redo.is_empty());
        assert!(old_undo.iter().any(|p| project_flags(p)[2]));
        let revision = crate::domain::current_project_revision(&state);
        let stale_binding = binding(&state);
        let (writer_held, held) = mpsc::channel();
        let (release_writer, release) = mpsc::channel();
        ownership.writer_release = Some(release_writer);
        let writer_state = state.clone();
        let second = f::package(&f::members(&changed), "contending actual container");
        assert_ne!(
            knx_productdb::sha256_hex(&second),
            knx_productdb::sha256_hex(&changed)
        );
        ownership.writer = Some(std::thread::spawn(move || {
            writer_state
                .product_db
                .as_ref()
                .unwrap()
                .with_legacy_mutation(
                    LegacyMutationKind::DirectPackage,
                    |conn| {
                        let _ = writer_held.send(());
                        release.recv_timeout(Duration::from_secs(60)).map_err(|e| e.to_string())?;
                        // Mutation completes BEFORE the waiting terminal lease: no mixed inventory.
                        let package = if ambiguous { &second } else { &changed };
                        knx_productdb::install_package(conn, "contending.knxprod", package).map(|_| ()).map_err(|e| e.to_string())
                    },
                    |result| if result.is_ok() { LegacySettlementHint::SuccessfulReturn }
                        else { LegacySettlementHint::PreCommitSemanticRefusal },
                ).map_err(|e| e.to_string())?
        }));
        tokio::task::spawn_blocking(move || held.recv_timeout(Duration::from_secs(60)).unwrap())
            .await
            .unwrap();
        hardware.release.notify_one();
        assert_eq!(next(&receiver).await, Event::LeaseAttempt);
        let counts = observation.counts();
        assert_eq!(
            (
                counts.callbacks,
                counts.attempts,
                counts.leases,
                counts.proofs
            ),
            (1, 1, 0, 0)
        );
        assert!(
            matches!(
                state.project.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ),
            "actual terminal candidate owns project"
        );
        assert!(
            matches!(
                state.command_stack.try_lock(),
                Err(std::sync::TryLockError::WouldBlock)
            ),
            "actual terminal candidate owns stack"
        );
        // The observer fires only after try_lock ACTUALLY fails inside this command.
        let (edit_done, done) = mpsc::channel();
        let editor_state = state.clone();
        let edit_command = knx_core::Command::RenameDevice {
            device: f::DEVICE, name: "Cannot enter compared candidate".into(),
        };
        let expected_command = (stale_binding.server_incarnation.clone(),
            stale_binding.project_incarnation, stale_binding.snapshot_revision, edit_command.clone());
        ownership.editor = Some(std::thread::spawn(move || {
            let start = Instant::now();
            let result = crate::domain::apply_bound_prepared(
                &editor_state, &stale_binding, "Contending project edit", edit_command,
            );
            let _ = edit_done.send((result, start.elapsed()));
            Ok(())
        }));
        assert_eq!(next(&receiver).await, Event::CommandProjectBlocked);
        assert_eq!(observation.blocked_command(), Some(expected_command));
        assert_eq!(observation.counts().command_blocks, 1);
        if fail_after_command_block { panic!("intentional contention assertion cleanup"); }
        assert!(
            matches!(done.try_recv(), Err(mpsc::TryRecvError::Empty)),
            "edit cannot complete while catalogue writer blocks the terminal candidate"
        );
        assert_eq!(
            observation.counts().leases,
            0,
            "writer still owns the catalogue"
        );
        ownership.release_writer();
        assert_eq!(next(&receiver).await, Event::LeaseAcquired);
        assert_eq!(next(&receiver).await, Event::Settled);
        let (edit_result, edit_wait) =
            tokio::task::spawn_blocking(move || done.recv_timeout(Duration::from_secs(60)).unwrap())
                .await
                .unwrap();
        assert!(
            matches!(edit_result, Err(crate::domain::BoundCommandError::Stale)),
            "settlement advances revision before the waiting edit can publish"
        );
        super::super::failure_diagnostic::settled_slow(&state, &router, "finished").await;
        let counts = observation.counts();
        assert_eq!(
            (
                counts.callbacks,
                counts.attempts,
                counts.leases,
                counts.proofs
            ),
            (1, 1, 1, 1)
        );
        assert!(!counts.wait.is_zero() && !counts.proof.is_zero());
        assert!(counts.candidate.is_some());
        assert_eq!(counts.compared_candidate, counts.candidate, "same locked candidate enters comparison");
        assert_eq!(counts.command_blocks, 1);
        let refused = ambiguous && !retained;
        assert_eq!(
            counts.parts,
            if refused {
                None
            } else {
                Some(ProgrammingParts {
                    address: true,
                    application: true,
                    parameters: false,
                    group_addresses: false,
                })
            }
        );
        let expected = if refused {
            [true, false, false, false, true]
        } else {
            [true, true, false, false, true]
        };
        assert_eq!(flags(&state), expected);
        assert_eq!(
            crate::domain::current_project_revision(&state),
            revision + 1
        );
        let after = state.project.lock().unwrap().as_ref().unwrap().clone();
        let stack_after = state.command_stack.lock().unwrap().clone();
        assert_eq!(configuration(&after), configuration(&before));
        assert_eq!(
            stack_after.history_lengths(),
            stack_before.history_lengths()
        );
        assert_eq!(
            stack_after.comparison_revision(f::DEVICE),
            stack_before.comparison_revision(f::DEVICE) + 1
        );
        let (undo, redo) = stack_after.snapshot_states(&after).unwrap();
        for (new, old) in undo.iter().zip(&old_undo).chain(redo.iter().zip(&old_redo)) {
            assert_eq!(configuration(new), configuration(old));
            assert!(
                !project_flags(new)[2] && !project_flags(new)[3],
                "old touched claims never resurrect"
            );
            if refused {
                assert!(!project_flags(new)[1]);
            }
        }
        let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
        let progress = store.progress();
        assert_eq!(progress.outcome, LocalOutcome::Finished);
        assert_eq!(progress.download.outcome, LocalOutcome::Finished);
        assert_eq!(progress.download.result, DownloadResult::Verified);
        assert!(progress.download.completed_steps > 0);
        assert_eq!(
            Some(progress.download.completed_steps),
            progress.download.total_steps
        );
        assert_eq!(store.recovery(), Some(&backup), "complete captured prestate survives settlement");
        drop(store);
        for _ in 0..2 {
            let status = http(&router, "/api/device-system2/status", None, 200).await;
            assert_eq!(status["projectSettlement"], "sessionOnly");
            let report = http(
                &router,
                &format!("/api/device-system2/report?operationId={id}"),
                None,
                200,
            )
            .await;
            assert_eq!(report["download"]["result"], "verified");
            assert_eq!(
                report["download"]["completedSteps"],
                progress.download.completed_steps
            );
            let reopened = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
            assert_eq!(reopened.progress(), progress);
            assert_eq!(reopened.recovery(), Some(&backup), "complete prestate survives status/report");
        }
        assert_eq!(
            observation.counts(),
            counts,
            "GET/report never re-enter the terminal proof"
        );
        assert_eq!(
            std::fs::read(operation.join("observed.json")).unwrap(),
            observed
        );
        assert_eq!(
            std::fs::read(operation.join("metadata.json")).unwrap(),
            metadata
        );
        let selected = http(&router, "/api/device-system2/select", Some(json!({"operationId": id})), 200).await;
        assert_eq!(selected["projectSettlement"], "notRecorded");
        let reopened = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
        assert_eq!(reopened.progress(), progress);
        assert_eq!(reopened.recovery(), Some(&backup), "complete prestate survives select/reopen");
        drop(reopened);
        assert_eq!(observation.counts(), counts);
        for expected in undo.iter().rev() {
            crate::domain::undo_impl(&state).unwrap();
            assert_eq!(state.project.lock().unwrap().as_ref().unwrap(), expected);
        }
        for _ in &undo {
            crate::domain::redo_impl(&state).unwrap();
        }
        assert_eq!(state.project.lock().unwrap().as_ref().unwrap(), &after);
        crate::domain::redo_impl(&state).unwrap();
        assert_eq!(
            state.project.lock().unwrap().as_ref().unwrap(),
            redo.last().unwrap()
        );
        eprintln!("E17_CONTENTION family={} namespace={namespace} ambiguous={ambiguous} callbacks={} lease_attempts={} acquired_reads={} proofs={} command_blocks={} same_candidate={} catalogue_wait_us={} proof_us={} project_command_us={}",
            if retained { "retained" } else { "standalone" }, counts.callbacks, counts.attempts,
            counts.leases, counts.proofs, counts.command_blocks, counts.candidate == counts.compared_candidate, counts.wait.as_micros(), counts.proof.as_micros(), edit_wait.as_micros());
    }).await;
    let cleanup = ownership.finish(result.is_err()).await;
    if fail_after_command_block {
        if let Err(panic) = &result {
            if owned::panic_text(panic.as_ref()) == "intentional contention assertion cleanup" {
                assert!(
                    cleanup.is_empty(),
                    "injected assertion cleanup: {cleanup:?}"
                );
                assert!(ownership.writer.is_none() && ownership.editor.is_none());
                assert_eq!(
                    ownership.joined_threads, 2,
                    "both real thread joins completed"
                );
                assert!(
                    ownership.service_joined,
                    "retained supervisor join completed"
                );
                assert_eq!(
                    observation.worker_finished(),
                    Some(true),
                    "actual child completed"
                );
                assert!(
                    state
                        .device_system2
                        .inner
                        .lock()
                        .unwrap()
                        .current
                        .as_ref()
                        .unwrap()
                        .task
                        .is_none(),
                    "actual service supervisor and child joined before fixture retirement"
                );
                return;
            }
        }
        owned::propagate(result, cleanup);
        panic!("injected contention assertion did not execute");
    }
    owned::propagate(result, cleanup);
}

#[tokio::test]
async fn caption_contention_standalone20_actual_terminal_lease() {
    for ambiguous in [false, true] {
        run(20, false, ambiguous, false).await;
    }
}
#[tokio::test]
async fn caption_contention_standalone23_actual_terminal_lease() {
    for ambiguous in [false, true] {
        run(23, false, ambiguous, false).await;
    }
}
#[tokio::test]
async fn caption_contention_retained20_actual_terminal_lease() {
    for ambiguous in [false, true] {
        run(20, true, ambiguous, false).await;
    }
}
#[tokio::test]
async fn caption_contention_retained23_actual_terminal_lease() {
    for ambiguous in [false, true] {
        run(23, true, ambiguous, false).await;
    }
}

#[tokio::test]
async fn caption_contention_assertion_failure_releases_and_joins_owned_work() {
    run(20, false, false, true).await;
}
