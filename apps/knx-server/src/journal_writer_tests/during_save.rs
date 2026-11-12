//! Edits made while a Save writes its file: answered at once, journaled once it finishes (#36).

use std::path::Path;
use std::sync::{mpsc, Arc};
use std::time::Duration;

use super::stored_name;
use crate::domain::{self, DURING_SAVE_WRITE};
use crate::journal_writer::JournalStatus;
use crate::AppState;

/// Generous: a passing run answers in milliseconds, a loaded gate host is slow.
const ANSWER: Duration = Duration::from_secs(10);

/// Installs a hook on this (the saving) thread that renames installation
/// `installation` from another thread while the save writes, and reports
/// whether that edit was answered before the write went on.
fn edit_during_the_write(
    state: &Arc<AppState>,
    installation: u8,
    name: &'static str,
) -> mpsc::Receiver<bool> {
    let (answered_tx, answered_rx) = mpsc::channel();
    let editing = Arc::clone(state);
    DURING_SAVE_WRITE.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            let (done_tx, done_rx) = mpsc::channel();
            std::thread::spawn(move || {
                let renamed = domain::rename_installation_impl(&editing, installation, name.into());
                let _ = done_tx.send(renamed.is_ok());
            });
            // A regression waits here for the deadline, then lets the save go
            // on; the blocked edit finishes after it, so nothing stays stuck.
            let answered = done_rx.recv_timeout(ANSWER).unwrap_or(false);
            answered_tx.send(answered).unwrap();
        }));
    });
    answered_rx
}

fn assert_journaled(state: &AppState, file: &Path, name: &str) {
    state.journal().flush();
    assert_eq!(state.journal().status(), JournalStatus::Saved);
    assert_eq!(
        stored_name(file),
        name,
        "the edit is not in {}",
        file.display()
    );
}

/// A plain Save: the edit is answered while the file is written and reaches
/// the same file after the save, still unsaved in the editor.
#[test]
fn an_edit_during_a_save_is_answered_and_journaled_after_it() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = super::background(&dir);
    let state = Arc::new(state);
    let answered = edit_during_the_write(&state, installation, "Edited during save");
    domain::save_project(&state).unwrap();
    assert!(
        answered.try_recv().expect("the hook ran"),
        "the edit waited for the save write"
    );
    assert_journaled(&state, &path, "Edited during save");
    assert!(
        domain::current_project_tree(&state)
            .unwrap()
            .tree
            .is_modified
    );
}

/// Save As moves the project while the edit waits for its journal write: the
/// edit goes to the new file at its generation, never to the file it left,
/// including the first Save As of a project that had no file yet.
#[test]
fn an_edit_during_save_as_is_journaled_into_the_new_file() {
    for saved_before in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let old = dir.path().join("project.knxdb");
        let (state, installation) = if saved_before {
            let (state, path, installation) = super::background(&dir);
            assert_eq!(path, old);
            (state, installation)
        } else {
            let state = AppState::default();
            let tree = domain::new_project_impl(&state, None, None, None, None, true).unwrap();
            state.journal().run_in_background();
            (state, tree.installations[0].id)
        };
        let state = Arc::new(state);
        let before = saved_before.then(|| stored_name(&old));
        let new = dir.path().join("moved.knxdb");
        let answered = edit_during_the_write(&state, installation, "Edited during Save As");
        domain::save_project_as(&state, &new).unwrap();
        assert!(
            answered.try_recv().expect("the hook ran"),
            "saved_before={saved_before}: the edit waited for the save write"
        );
        assert_journaled(&state, &new, "Edited during Save As");
        if let Some(before) = before {
            assert_eq!(stored_name(&old), before, "the edit reached the old file");
        }
    }
}

/// A Save As that fails leaves the project where it was: an edit made during
/// it is journaled into the project's own file, or nowhere without one.
#[test]
fn an_edit_during_a_failed_save_as_stays_with_the_project() {
    for saved_before in [true, false] {
        let dir = tempfile::tempdir().unwrap();
        let (state, installation) = if saved_before {
            let (state, _, installation) = super::background(&dir);
            (state, installation)
        } else {
            let state = AppState::default();
            let tree = domain::new_project_impl(&state, None, None, None, None, true).unwrap();
            state.journal().run_in_background();
            (state, tree.installations[0].id)
        };
        let state = Arc::new(state);
        let writes_before = super::writes(&state);
        let answered = edit_during_the_write(&state, installation, "Edited during failure");
        let unwritable = dir.path().join("missing").join("moved.knxdb");
        assert!(domain::save_project_as(&state, &unwritable).is_err());
        assert!(answered.try_recv().expect("the hook ran"));
        state.journal().flush();
        assert_eq!(state.journal().status(), JournalStatus::Saved);
        if saved_before {
            assert_journaled(
                &state,
                &dir.path().join("project.knxdb"),
                "Edited during failure",
            );
        } else {
            assert_eq!(state.store_path.known(), None);
            assert_eq!(super::writes(&state), writes_before, "a journal write ran");
        }
    }
}

/// The save publishes acquired manifest refs after the edit was handed over.
/// Pin that seam independently: inject a stale in-memory manifest during the
/// save hook, then let the real save restore its frozen manifest and let the
/// genuine journal writer finish. Reopen must keep the save's manifest, not
/// the stale refs frozen in the edit's job.
#[test]
fn an_edit_during_a_save_keeps_the_saves_manifest() {
    let dir = tempfile::tempdir().unwrap();
    let (state, path, installation) = super::background(&dir);
    let saved_ref = knx_store::ManufacturerRef {
        source_path: "M-0001/Hardware.xml".into(),
        sha256: "saved-source-hash".into(),
        len: 7,
        kind: "Hardware".into(),
    };
    *state.manufacturer_refs.lock().unwrap() = vec![saved_ref.clone()];
    let state = Arc::new(state);
    let editing = Arc::clone(&state);
    let (answer_tx, answer_rx) = mpsc::channel();
    DURING_SAVE_WRITE.with(|hook| {
        *hook.borrow_mut() = Some(Box::new(move || {
            // Save already acquired saved_ref, but has not published it yet.
            editing.manufacturer_refs.lock().unwrap().clear();
            let (done_tx, done_rx) = mpsc::channel();
            std::thread::spawn(move || {
                let result = domain::rename_installation_impl(
                    &editing,
                    installation,
                    "Manifest edit".into(),
                );
                let _ = done_tx.send(result.is_ok());
            });
            answer_tx
                .send(done_rx.recv_timeout(ANSWER).unwrap_or(false))
                .unwrap();
        }));
    });
    domain::save_project(&state).unwrap();
    state.journal().flush();
    assert!(answer_rx.try_recv().unwrap(), "the edit waited for Save");
    assert_eq!(state.journal().status(), JournalStatus::Saved);
    let connection = knx_store::open_existing_and_migrate(&path).unwrap();
    let editor = knx_store::project_history::load_editor(&connection)
        .unwrap()
        .unwrap();
    assert_eq!(
        editor.working.project.installations[0].name,
        "Manifest edit"
    );
    assert_eq!(editor.working.manufacturer_refs, vec![saved_ref]);
}
