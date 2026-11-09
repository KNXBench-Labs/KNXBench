//! The background journal writer answers first, coalesces, blocks on failure and flushes.

use std::time::{Duration, Instant};

use crate::journal_writer::JournalStatus;
use crate::{domain, AppState};
use knx_store::project_history as history;

/// A saved project on a server whose journal writer runs in the background.
fn background(dir: &tempfile::TempDir) -> (AppState, std::path::PathBuf, u8) {
    let path = dir.path().join("project.knxdb");
    let state = AppState::default();
    let tree = domain::new_project_impl(&state, None, None, None, None, true).unwrap();
    domain::save_project_as(&state, &path).unwrap();
    state.journal().run_in_background();
    (state, path, tree.installations[0].id)
}

fn stored_name(path: &std::path::Path) -> String {
    let conn = knx_store::open_existing_and_migrate(path).unwrap();
    history::load_editor(&conn)
        .unwrap()
        .unwrap()
        .working
        .project
        .installations[0]
        .name
        .clone()
}

fn writes(state: &AppState) -> usize {
    state.journal_writes.lock().unwrap().records().count()
}

fn wait_until(what: &str, done: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(30);
    while !done() {
        assert!(Instant::now() < deadline, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn an_edit_is_answered_before_its_write_and_is_in_the_file_after_a_flush() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = background(&dir);
    // Hold the generation so the writer cannot finish, then edit.
    let held = state.history_generation.value.lock().unwrap();
    let tree = domain::rename_installation_impl(&state, installation, "Answered".into()).unwrap();
    assert_eq!(tree.installations[0].name, "Answered");
    assert_eq!(state.journal().status(), JournalStatus::Saving);
    drop(held);
    state.journal().flush();
    assert_eq!(state.journal().status(), JournalStatus::Saved);
    assert_eq!(stored_name(&path), "Answered");
}

#[test]
fn edits_during_a_running_write_become_one_write_of_the_newest_state() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = background(&dir);
    let held = state.history_generation.value.lock().unwrap();
    domain::rename_installation_impl(&state, installation, "First".into()).unwrap();
    wait_until("the writer to start", || state.journal().is_busy());
    for name in ["Second", "Third", "Fourth"] {
        domain::rename_installation_impl(&state, installation, name.into()).unwrap();
    }
    drop(held);
    state.journal().flush();
    assert_eq!(
        writes(&state),
        2,
        "the running write and one coalesced write"
    );
    assert_eq!(stored_name(&path), "Fourth");
    // The coalesced journal still reopens to the full undo history.
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(history::load_editor(&conn).unwrap().unwrap().undo.len(), 4);
}

#[test]
fn a_failed_write_refuses_further_edits_until_a_save_as_succeeds() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = background(&dir);
    std::fs::remove_file(&path).unwrap();
    let tree = domain::rename_installation_impl(&state, installation, "Lost file".into()).unwrap();
    assert_eq!(tree.installations[0].name, "Lost file");
    state.journal().flush();
    assert!(matches!(
        state.journal().status(),
        JournalStatus::Failed { .. }
    ));
    let refused = domain::rename_installation_impl(&state, installation, "Refused".into());
    assert!(refused.unwrap_err().contains("could not be written"));
    // Nothing shown was discarded: Save As writes it and clears the failure.
    let rescued = dir.path().join("rescued.knxdb");
    domain::save_project_as(&state, &rescued).unwrap();
    assert_eq!(state.journal().status(), JournalStatus::Saved);
    assert_eq!(stored_name(&rescued), "Lost file");
    domain::rename_installation_impl(&state, installation, "Editing again".into()).unwrap();
    state.journal().flush();
    assert_eq!(stored_name(&rescued), "Editing again");
}

#[test]
fn a_retry_writes_the_newest_state_once_the_file_is_back() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = background(&dir);
    let copy = dir.path().join("copy.knxdb");
    std::fs::copy(&path, &copy).unwrap();
    std::fs::remove_file(&path).unwrap();
    domain::rename_installation_impl(&state, installation, "Pending".into()).unwrap();
    state.journal().flush();
    assert!(matches!(
        state.journal().status(),
        JournalStatus::Failed { .. }
    ));
    std::fs::rename(&copy, &path).unwrap();
    // Meanwhile another writer changed the project in memory (a note, for
    // example): the retry writes that current state, not the failed job.
    state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .installations[0]
        .name = "Changed meanwhile".into();
    domain::retry_journal(&state);
    state.journal().flush();
    assert_eq!(state.journal().status(), JournalStatus::Saved);
    assert_eq!(stored_name(&path), "Changed meanwhile");
}

#[test]
fn reading_the_generation_waits_for_the_writer() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = background(&dir);
    for name in ["One", "Two", "Three"] {
        domain::rename_installation_impl(&state, installation, name.into()).unwrap();
    }
    // A version action or Save reads the generation: it sees the file current.
    let generation = *state.history_generation.lock().unwrap();
    assert_eq!(state.journal().status(), JournalStatus::Saved);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(
        history::load_editor(&conn).unwrap().unwrap().generation,
        generation
    );
    assert_eq!(stored_name(&path), "Three");
}

#[test]
fn own_commits_skip_the_admission_and_a_foreign_write_restores_it() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = background(&dir);
    let phases = |state: &AppState| -> Vec<&'static str> {
        let writes = state.journal_writes.lock().unwrap();
        let last = writes.records().last().unwrap();
        last.report.phases.iter().map(|(name, _)| *name).collect()
    };
    domain::rename_installation_impl(&state, installation, "Cold".into()).unwrap();
    state.journal().flush();
    assert!(
        phases(&state).contains(&"preAdmission"),
        "{:?}",
        phases(&state)
    );
    domain::rename_installation_impl(&state, installation, "Warm".into()).unwrap();
    state.journal().flush();
    assert!(
        !phases(&state).contains(&"preAdmission"),
        "{:?}",
        phases(&state)
    );
    // Another connection commits (and restores the content, so the file is
    // still valid): the next write admits the file in full again.
    let other = knx_store::open_existing_and_migrate(&path).unwrap();
    other
        .execute("UPDATE project_info SET name = name || ' elsewhere'", [])
        .unwrap();
    other
        .execute(
            "UPDATE project_info SET name = substr(name, 1, length(name) - 10)",
            [],
        )
        .unwrap();
    drop(other);
    domain::rename_installation_impl(&state, installation, "After foreign".into()).unwrap();
    state.journal().flush();
    assert!(
        phases(&state).contains(&"preAdmission"),
        "{:?}",
        phases(&state)
    );
    assert_eq!(stored_name(&path), "After foreign");
}

#[test]
fn a_real_foreign_change_is_refused_instead_of_overwritten() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = background(&dir);
    domain::rename_installation_impl(&state, installation, "Ours".into()).unwrap();
    state.journal().flush();
    let other = knx_store::open_existing_and_migrate(&path).unwrap();
    other
        .execute(
            "UPDATE project_info SET name = 'Saved by another editor'",
            [],
        )
        .unwrap();
    drop(other);
    domain::rename_installation_impl(&state, installation, "Ours again".into()).unwrap();
    state.journal().flush();
    let JournalStatus::Failed { message } = state.journal().status() else {
        panic!(
            "a foreign save must stop the writer: {:?}",
            state.journal().status()
        );
    };
    assert!(message.contains("another editor"), "{message}");
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(
        knx_store::load_project(&conn).unwrap().info.name,
        "Saved by another editor",
        "the other editor's save is kept"
    );
}

#[test]
fn opening_another_project_writes_the_last_edits_first() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = background(&dir);
    domain::rename_installation_impl(&state, installation, "Before switching".into()).unwrap();
    domain::new_project_impl(&state, None, None, None, None, true).unwrap();
    assert_eq!(state.journal().status(), JournalStatus::Saved);
    assert_eq!(stored_name(&path), "Before switching");
}

#[test]
fn a_file_replaced_under_the_same_path_is_admitted_afresh_not_written_blind() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = background(&dir);
    domain::rename_installation_impl(&state, installation, "Kept".into()).unwrap();
    state.journal().flush();
    // Another program replaces the file with a copy (same content, new inode):
    // writing through the kept connection would land in the unlinked old file.
    let replacement = dir.path().join("replacement.knxdb");
    std::fs::copy(&path, &replacement).unwrap();
    std::fs::rename(&replacement, &path).unwrap();
    domain::rename_installation_impl(&state, installation, "Into the new file".into()).unwrap();
    state.journal().flush();
    assert_eq!(state.journal().status(), JournalStatus::Saved);
    assert_eq!(stored_name(&path), "Into the new file");
    let writes = state.journal_writes.lock().unwrap();
    let last = writes.records().last().unwrap();
    assert!(last
        .report
        .phases
        .iter()
        .any(|(name, _)| *name == "preAdmission"));
}
