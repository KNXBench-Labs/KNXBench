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
    let receipt = settle(&state, &meta(&state), &failed(), Run::Download, |_, _| true);
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
    settle(&state, &meta(&state), &failed(), Run::Download, |_, _| {
        false
    });
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
            settle(&state, &meta(&state), &p, Run::Download, |_, _| true),
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
            true
        }),
        Receipt::SessionOnly
    );
    assert_eq!(belief(&state), ProgrammingParts::ALL);
    // The same verified run against a changed project/source: memory pending, address not claimed.
    let state = fresh();
    settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
        false
    });
    assert_eq!(belief(&state), NONE);
    mark_all(&state, true);
    settle(&state, &meta(&state), &verified(), Run::Download, |_, _| {
        false
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
        settle(&state, &meta(&state), &p, Run::Recovery, |_, _| true),
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
        settle(&state2, &meta(&state2), &idle, Run::Recovery, |_, _| true),
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
    settle(&state, &meta(&state), &p, Run::Download, |_, _| true);
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
        settle(&state, &m, &failed(), Run::Download, |_, _| true),
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
        settle(&state, &m, &failed(), Run::Download, |_, _| true),
        Receipt::DeviceMissing
    );
    // The device is gone.
    let mut other = m.clone();
    other.binding.device_id = 9;
    assert_eq!(
        settle(&state, &other, &failed(), Run::Download, |_, _| true),
        Receipt::DeviceMissing
    );
    assert_eq!(domain::current_project_revision(&state), revision);
    assert_eq!(belief(&state), before);
    // No project at all.
    *state.project.lock().unwrap() = None;
    assert_eq!(
        settle(&state, &m, &failed(), Run::Download, |_, _| true),
        Receipt::NoProject
    );
}

#[test]
fn a_failed_persist_applies_nothing_and_says_so() {
    let state = fresh();
    mark_all(&state, true);
    let dir = tempfile::tempdir().unwrap();
    *state.store_path.lock().unwrap() = Some(dir.path().join("missing/dir/project.knxdb"));
    let (revision, before) = (domain::current_project_revision(&state), belief(&state));
    assert_eq!(
        settle(&state, &meta(&state), &failed(), Run::Download, |_, _| true),
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
        settle(&state, &meta(&state), &failed(), Run::Download, |_, _| true),
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
            settle(&state, &historical, &progress, run, |_, _| true),
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
