//! A settled run reaches the working journal; reopened undo and redo bring no stale claim back.

use knx_core::Command;

use super::project_edits::{command, edit_time, install_rich_project, TIME_ORIGINAL};
use super::*;
use crate::domain;

/// A server holding the project of `f` as a saved synthetic `.knxdb`.
fn saved(f: &Fixture) -> std::path::PathBuf {
    install_rich_project(&f.state);
    let path = f._dir.path().join("project.knxdb");
    domain::save_project_as(&f.state, &path).expect("synthetic project saves");
    path
}

/// A fresh server that opens `path` the way the open route does.
fn reopen(path: &Path) -> SharedState {
    let state = Arc::new(AppState::default());
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, path);
    domain::open_native_project(&state, path, true, &progress).expect("saved project reopens");
    state
}

fn time(state: &SharedState) -> String {
    let project = state.project.lock().unwrap();
    project.as_ref().unwrap().installations[0].parameters[0]
        .raw
        .clone()
}

fn memory(parts: ProgrammingParts) -> ProgrammingParts {
    ProgrammingParts {
        address: false,
        ..parts
    }
}

/// History: everything marked programmed, then the parameter edited, whose
/// undo step claims the parameters programmed again.
fn claim_history(f: &Fixture) {
    command(
        &f.state,
        Command::SetDeviceProgramming {
            device: DeviceId(1),
            change: ProgrammingChange::mark(ProgrammingParts::ALL, true),
        },
    );
    edit_time(&f.state);
}

/// Undoes and redoes the whole history; no step may bring a memory claim back.
fn walk_history(state: &SharedState) {
    let mut seen = Vec::new();
    domain::undo_impl(state).expect("undo of the parameter edit");
    seen.push(belief(state));
    assert_eq!(time(state), TIME_ORIGINAL);
    domain::undo_impl(state).expect("undo of the mark");
    seen.push(belief(state));
    domain::redo_impl(state).expect("redo of the mark");
    seen.push(belief(state));
    domain::redo_impl(state).expect("redo of the parameter edit");
    seen.push(belief(state));
    for believed in seen {
        assert_eq!(
            memory(believed),
            NONE,
            "a stale memory claim came back: {believed:?}"
        );
    }
}

#[test]
fn undo_and_redo_after_a_source_changed_run_cannot_resurrect_memory() {
    let f = fixture(&OLD, &new_base());
    saved(&f);
    claim_history(&f);
    assert_eq!(full(&f, true), ADDRESS, "live state after settling");
    walk_history(&f.state);
}

#[test]
fn a_source_changed_run_is_persisted_by_the_settlement_itself_and_reopens_settled() {
    let f = fixture(&OLD, &new_base());
    let path = saved(&f);
    claim_history(&f);
    let settled = full(&f, true);
    assert_eq!(settled, ADDRESS, "live state after settling");

    // No explicit save: settlement wrote the recoverable working journal of
    // the saved file, and a fresh server recovers it on open.
    let reopened = reopen(&path);
    assert_eq!(belief(&reopened), settled, "reopened state");
    assert_eq!(time(&reopened), time(&f.state));
    walk_history(&reopened);
}

#[test]
fn a_partial_run_leaves_untouched_flags_and_their_history_alone_after_reopening() {
    let f = fixture(&OLD, &new_base());
    let path = saved(&f);
    claim_history(&f);
    let before = belief(&f.state);
    assert!(before.address && before.application && before.group_addresses);
    let after = partial(&f, true, false, true);
    assert!(!after.parameters, "the written parameters are pending");
    assert_eq!(
        ProgrammingParts {
            parameters: false,
            ..after
        },
        ProgrammingParts {
            parameters: false,
            ..before
        },
        "parts the run never wrote keep their flags"
    );
    let reopened = reopen(&path);
    assert_eq!(belief(&reopened), after);
    domain::undo_impl(&reopened).expect("undo of the parameter edit");
    let undone = belief(&reopened);
    assert!(!undone.parameters, "the parameter claim is gone");
    assert_eq!(
        ProgrammingParts {
            parameters: false,
            ..undone
        },
        ProgrammingParts {
            parameters: false,
            ..before
        },
        "undo preserves the untouched address, application and table claims"
    );
}
