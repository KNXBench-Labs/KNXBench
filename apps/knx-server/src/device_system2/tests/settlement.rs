//! Automatic project-belief settlement of a System-2 run through the real domain seam.
use crate::device_system2::dto::{Binding, Metadata};
use crate::device_system2::settlement::{settle, Receipt, Run};
use crate::{domain, AppState, SharedState};
use knx_app::property_validation::{
    DownloadFacts, DownloadResult, LocalOutcome, OperationProgress,
};
use knx_core::commissioning::pending::{ProgrammingChange, ProgrammingParts};
use knx_core::{Command, DeviceId, DeviceInstance, Language, Project, SourceRef};
use std::sync::{atomic::Ordering, Arc};

const ADDRESS: &str = "1.1.67";

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "t".into(),
            ets_id: "D-1".into(),
        },
        name: "Synthetic".into(),
        description: None,
        address: Some(ADDRESS.parse().unwrap()),
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.info.project_id = "P-0001".into();
    project.installations.push(knx_core::Installation {
        id: knx_core::InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::FinishedDesign,
        topology: knx_core::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.ids.next_device_id().unwrap();
    project
}
fn fresh() -> SharedState {
    let state = Arc::new(AppState::default());
    *state.project.lock().unwrap() = Some(project());
    state
}
fn meta(state: &AppState) -> Metadata {
    Metadata {
        schema_version: 1,
        operation_id: "0123456789abcdef0123456789abcdef".into(),
        binding: Binding {
            server_incarnation: state.server_incarnation.clone(),
            project_incarnation: state.project_incarnation.load(Ordering::SeqCst).to_string(),
            project_revision: "0".into(),
            device_id: 1,
        },
        address: ADDRESS.into(),
        device_name: "Synthetic".into(),
    }
}
fn progress(outcome: LocalOutcome, result: DownloadResult, mutation: bool) -> OperationProgress {
    let mut p = crate::device_system2::initial(outcome);
    p.mutation_possible = mutation;
    p.download = DownloadFacts {
        result,
        outcome,
        completed_steps: 26,
        total_steps: Some(27),
    };
    p
}
fn belief(state: &SharedState) -> ProgrammingParts {
    let guard = state.project.lock().unwrap();
    let c = &guard
        .as_ref()
        .unwrap()
        .devices
        .get(DeviceId(1))
        .unwrap()
        .commissioning;
    ProgrammingParts {
        address: c.individual_address_loaded,
        application: c.application_program_loaded,
        parameters: c.parameters_loaded,
        group_addresses: c.communication_part_loaded,
    }
}
fn mark_all(state: &SharedState, programmed: bool) {
    let binding = domain::SnapshotBinding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::SeqCst),
        snapshot_revision: domain::current_project_revision(state),
    };
    let command = Command::SetDeviceProgramming {
        device: DeviceId(1),
        change: ProgrammingChange::mark(ProgrammingParts::ALL, programmed),
    };
    domain::apply_bound_prepared(state, &binding, "Test", command).unwrap();
}
const NONE: ProgrammingParts = ProgrammingParts::NONE;
fn failed() -> OperationProgress {
    progress(
        LocalOutcome::Failed,
        DownloadResult::TransferCompleteVerificationUnfinished,
        true,
    )
}
fn verified() -> OperationProgress {
    progress(LocalOutcome::Finished, DownloadResult::Verified, true)
}

#[test]
fn a_possible_mutation_invalidates_memory_parts_but_not_the_address_and_survives_undo_redo() {
    let state = fresh();
    mark_all(&state, true);
    let receipt = settle(&state, &meta(&state), &failed(), Run::Download, |_, _| {
        ProgrammingParts::ALL
    });
    assert_eq!(receipt, Receipt::SessionOnly);
    assert_eq!(
        belief(&state),
        ProgrammingParts {
            address: true,
            ..NONE
        }
    );
    // History is invalidated: undoing the mark or redoing it cannot bring a memory claim back.
    domain::undo_impl(&state).unwrap();
    assert!(
        !belief(&state).application
            && !belief(&state).parameters
            && !belief(&state).group_addresses
    );
    domain::redo_impl(&state).unwrap();
    let after = belief(&state);
    assert!(!after.application && !after.parameters && !after.group_addresses);
}

#[test]
fn initially_clean_flags_become_pending_after_a_failure_that_may_have_written() {
    let state = fresh();
    mark_all(&state, true);
    let revision = domain::current_project_revision(&state);
    settle(&state, &meta(&state), &failed(), Run::Download, |_, _| NONE);
    assert!(domain::current_project_revision(&state) > revision);
    assert!(!belief(&state).application);
}

#[test]
fn a_run_that_could_not_write_leaves_flags_stack_and_revision_alone() {
    let state = fresh();
    mark_all(&state, true);
    let (revision, before) = (domain::current_project_revision(&state), belief(&state));
    for p in [
        progress(LocalOutcome::Failed, DownloadResult::NothingWritten, false),
        progress(
            LocalOutcome::Cancelled,
            DownloadResult::NothingWritten,
            false,
        ),
    ] {
        assert_eq!(
            settle(&state, &meta(&state), &p, Run::Download, |_, _| {
                ProgrammingParts::ALL
            }),
            Receipt::NoMutation
        );
    }
    assert_eq!(
        (domain::current_project_revision(&state), belief(&state)),
        (revision, before)
    );
}

#[test]
fn only_a_verified_current_download_earns_programmed() {
    let state = fresh();
    assert_eq!(
        settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::SessionOnly
    );
    assert_eq!(belief(&state), ProgrammingParts::ALL);
    // The same verified run against a changed project/source: memory pending, address not claimed.
    let state = fresh();
    settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
        NONE
    });
    assert_eq!(belief(&state), NONE);
    mark_all(&state, true);
    settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
        NONE
    });
    assert_eq!(
        belief(&state),
        ProgrammingParts {
            address: true,
            ..NONE
        }
    );
}

#[test]
fn a_recovery_never_earns_programmed_and_invalidates_memory() {
    let state = fresh();
    mark_all(&state, true);
    let mut p = verified();
    p.outcome = LocalOutcome::Restored;
    p.restore_attempted = true;
    assert_eq!(
        settle(&state, &meta(&state), &p, Run::Recovery, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::SessionOnly
    );
    assert_eq!(
        belief(&state),
        ProgrammingParts {
            address: true,
            ..NONE
        }
    );
    // A recovery that was refused before it started changes nothing.
    let state2 = fresh();
    mark_all(&state2, true);
    let mut idle = verified();
    idle.restore_attempted = false;
    assert_eq!(
        settle(&state2, &meta(&state2), &idle, Run::Recovery, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::NoMutation
    );
    assert_eq!(belief(&state2), ProgrammingParts::ALL);
}

#[test]
fn a_verified_run_is_not_current_after_a_restore_was_attempted() {
    let state = fresh();
    mark_all(&state, true);
    let mut p = verified();
    p.restore_attempted = true;
    settle(&state, &meta(&state), &p, Run::Download, |_, _| {
        ProgrammingParts::ALL
    });
    assert_eq!(
        belief(&state),
        ProgrammingParts {
            address: true,
            ..NONE
        }
    );
}

#[test]
fn another_project_a_missing_device_or_a_reused_address_gets_nothing() {
    let state = fresh();
    mark_all(&state, true);
    let m = meta(&state);
    let (revision, before) = (domain::current_project_revision(&state), belief(&state));
    // Another project incarnation.
    state.project_incarnation.fetch_add(1, Ordering::SeqCst);
    assert_eq!(
        settle(&state, &m, &failed(), Run::Download, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::OtherProject
    );
    state.project_incarnation.fetch_sub(1, Ordering::SeqCst);
    // The address now belongs to a different device id; the bound device moved away.
    state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .address = Some("1.1.68".parse().unwrap());
    assert_eq!(
        settle(&state, &m, &failed(), Run::Download, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::DeviceMissing
    );
    // The device is gone.
    let mut other = m.clone();
    other.binding.device_id = 9;
    assert_eq!(
        settle(&state, &other, &failed(), Run::Download, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::DeviceMissing
    );
    assert_eq!(domain::current_project_revision(&state), revision);
    assert_eq!(belief(&state), before);
    // No project at all.
    *state.project.lock().unwrap() = None;
    assert_eq!(
        settle(&state, &m, &failed(), Run::Download, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::NoProject
    );
}

#[test]
fn a_failed_persist_applies_nothing_and_says_so() {
    let state = fresh();
    mark_all(&state, true);
    let dir = tempfile::tempdir().unwrap();
    state
        .store_path
        .lock()
        .unwrap()
        .set(Some(dir.path().join("missing/dir/project.knxdb")));
    let (revision, before) = (domain::current_project_revision(&state), belief(&state));
    assert_eq!(
        settle(&state, &meta(&state), &failed(), Run::Download, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::PersistenceFailed
    );
    assert_eq!(
        (domain::current_project_revision(&state), belief(&state)),
        (revision, before)
    );
}

#[test]
fn a_saved_project_is_persisted_before_the_receipt_says_so() {
    let state = fresh();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.knxdb");
    domain::save_project_as(&state, &path).unwrap();
    assert_eq!(
        settle(&state, &meta(&state), &failed(), Run::Download, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::Persisted
    );
}

#[test]
fn a_historical_recovery_from_another_server_session_cannot_settle_a_coincident_project() {
    let state = fresh();
    mark_all(&state, true);
    // The numeric project incarnation and the address coincide after a restart; the server does not.
    let mut historical = meta(&state);
    historical.binding.server_incarnation = "a-previous-server-session".into();
    let (revision, before) = (domain::current_project_revision(&state), belief(&state));
    let mut p = verified();
    p.restore_attempted = true;
    for (run, progress) in [(Run::Recovery, p), (Run::Download, failed())] {
        assert_eq!(
            settle(&state, &historical, &progress, run, |_, _| {
                ProgrammingParts::ALL
            }),
            Receipt::OtherProject
        );
    }
    assert_eq!(
        (domain::current_project_revision(&state), belief(&state)),
        (revision, before)
    );
    // The history is untouched too: undo still reaches the earlier claim.
    domain::undo_impl(&state).unwrap();
    assert!(!belief(&state).application);
}

const ADR_APP: ProgrammingParts = ProgrammingParts {
    address: true,
    application: true,
    parameters: false,
    group_addresses: false,
};
const ADR_APP_PAR: ProgrammingParts = ProgrammingParts {
    parameters: true,
    ..ADR_APP
};
fn memory_claims(state: &SharedState) -> bool {
    let b = belief(state);
    b.parameters || b.group_addresses
}

#[test]
fn s2_per_part_a_verified_run_credits_only_the_current_parts_and_undo_redo_cannot_restore_more() {
    let state = fresh();
    assert_eq!(
        settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
            ADR_APP_PAR
        }),
        Receipt::SessionOnly
    );
    assert_eq!(belief(&state), ADR_APP_PAR);
    let state = fresh();
    mark_all(&state, true);
    settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
        ADR_APP
    });
    assert_eq!(belief(&state), ADR_APP);
    domain::undo_impl(&state).unwrap();
    assert!(!memory_claims(&state), "undo must not restore stale claims");
    domain::redo_impl(&state).unwrap();
    assert!(!memory_claims(&state), "redo must not restore stale claims");
}

#[test]
fn s2_per_part_an_unverified_or_recovery_end_never_consults_the_current_parts() {
    let mut restored = verified();
    restored.restore_attempted = true;
    let mut interrupted = verified();
    interrupted.outcome = LocalOutcome::Interrupted;
    let cancelled = progress(
        LocalOutcome::Cancelled,
        DownloadResult::TransferIncomplete,
        true,
    );
    for (run, p) in [
        (Run::Download, failed()),
        (Run::Download, cancelled),
        (Run::Download, interrupted),
        (Run::Download, restored.clone()),
        (Run::Recovery, restored),
    ] {
        let state = fresh();
        mark_all(&state, true);
        settle(&state, &meta(&state), &p, run, |_, _| {
            unreachable!("only a verified download may ask for current parts")
        });
        assert_eq!(
            belief(&state),
            ProgrammingParts {
                address: true,
                ..NONE
            }
        );
    }
}

#[test]
fn s2_per_part_a_failed_persist_keeps_flags_history_and_revision() {
    let state = fresh();
    mark_all(&state, true);
    let dir = tempfile::tempdir().unwrap();
    state
        .store_path
        .lock()
        .unwrap()
        .set(Some(dir.path().join("missing/dir/project.knxdb")));
    let (revision, before) = (domain::current_project_revision(&state), belief(&state));
    assert_eq!(
        settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
            ADR_APP
        }),
        Receipt::PersistenceFailed
    );
    assert_eq!(
        (domain::current_project_revision(&state), belief(&state)),
        (revision, before)
    );
    // The history is untouched: once a store works again, undo reaches the state before the mark.
    state.store_path.lock().unwrap().set(None);
    domain::undo_impl(&state).unwrap();
    assert_eq!(belief(&state), NONE);
}

#[test]
fn s2_per_part_a_reopened_project_keeps_the_partial_credit_and_no_stale_claim() {
    let state = fresh();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.knxdb");
    domain::save_project_as(&state, &path).unwrap();
    mark_all(&state, true);
    assert_eq!(
        settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
            ADR_APP
        }),
        Receipt::Persisted
    );
    let reopened: SharedState = Arc::new(AppState::default());
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    domain::open_native_project(&reopened, &path, true, &progress).expect("reopens");
    assert_eq!(belief(&reopened), ADR_APP);
    for _ in 0..2 {
        let _ = domain::undo_impl(&reopened);
        assert!(
            !memory_claims(&reopened),
            "reopened undo must not restore claims"
        );
    }
    for _ in 0..2 {
        let _ = domain::redo_impl(&reopened);
        assert!(
            !memory_claims(&reopened),
            "reopened redo must not restore claims"
        );
    }
}

fn seed_baseline(state: &SharedState) {
    use knx_core::commissioning::parameter_baseline::{BaselineEdit, ParameterBaseline};
    state
        .parameter_baselines
        .lock()
        .unwrap()
        .apply(&BaselineEdit {
            device: DeviceId(1),
            parameter: knx_core::commissioning::parameter_baseline::BaselineChange::Record(
                ParameterBaseline {
                    object_flags: Some(Default::default()),
                    program_id: "M-0001_A-0001-01-0000".into(),
                    values: Default::default(),
                    at: chrono::DateTime::UNIX_EPOCH,
                },
            ),
            links: knx_core::commissioning::parameter_baseline::LinkBaselineChange::Record(
                knx_core::commissioning::parameter_baseline::LinkBaseline {
                    program_id: "M-0001_A-0001-01-0000".into(),
                    links: vec![],
                    at: chrono::DateTime::UNIX_EPOCH,
                },
            ),
        });
}

fn has_baseline(state: &SharedState) -> bool {
    state
        .parameter_baselines
        .lock()
        .unwrap()
        .get(DeviceId(1))
        .is_some()
}

/// ADR-0173 §3: a System-2 run writes no `ImageRequest`, so every settlement that touches the
/// parameters removes the baseline, the verified run that marks them programmed included.
#[test]
fn s2_every_settlement_that_may_have_mutated_removes_the_parameter_baseline() {
    let mut recovery = verified();
    recovery.outcome = LocalOutcome::Restored;
    recovery.restore_attempted = true;
    for (label, p, run) in [
        ("verified", verified(), Run::Download),
        ("failed", failed(), Run::Download),
        ("recovery", recovery, Run::Recovery),
    ] {
        let state = fresh();
        seed_baseline(&state);
        let receipt = settle(&state, &meta(&state), &p, run, |_, _| ProgrammingParts::ALL);
        assert_eq!(receipt, Receipt::SessionOnly, "{label}");
        assert!(!has_baseline(&state), "{label} kept a baseline");
        assert!(
            state
                .parameter_baselines
                .lock()
                .unwrap()
                .get_links(DeviceId(1))
                .is_none(),
            "{label} kept links"
        );
    }
    let state = fresh();
    seed_baseline(&state);
    settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
        ProgrammingParts::ALL
    });
    assert_eq!(
        belief(&state),
        ProgrammingParts::ALL,
        "programmed, baseline unknown"
    );
}

#[test]
fn s2_a_run_that_could_not_write_keeps_the_parameter_baseline() {
    let state = fresh();
    seed_baseline(&state);
    let p = progress(LocalOutcome::Failed, DownloadResult::NothingWritten, false);
    assert_eq!(
        settle(&state, &meta(&state), &p, Run::Download, |_, _| {
            ProgrammingParts::ALL
        }),
        Receipt::NoMutation
    );
    assert!(has_baseline(&state));
    assert!(state
        .parameter_baselines
        .lock()
        .unwrap()
        .get_links(DeviceId(1))
        .is_some());
}
