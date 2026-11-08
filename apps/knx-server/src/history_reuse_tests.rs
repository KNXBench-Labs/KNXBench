//! The journal reuse cache describes the stored rows after edits and open, and none elsewhere.

use crate::{domain, AppState};
use knx_store::project_history::{self as history, StackRows};

fn stored(path: &std::path::Path) -> StackRows {
    let conn = knx_store::open_existing_and_migrate(path).expect("saved project opens");
    history::load_editor(&conn)
        .expect("journal loads")
        .expect("journal exists")
        .rows
}

/// Whether the cache knows more than rows: retained context, baseline, working state.
fn knows_context(state: &AppState) -> bool {
    state
        .persisted_history
        .lock()
        .unwrap()
        .as_ref()
        .is_some_and(|cache| {
            cache.trust != history::JournalTrust::from_rows(cache.trust.rows.clone())
        })
}

fn cached(state: &AppState) -> Option<StackRows> {
    state
        .persisted_history
        .lock()
        .unwrap()
        .as_ref()
        .map(|cache| cache.trust.rows.clone())
}

#[test]
fn the_cache_follows_journal_writes_native_open_and_project_replacement() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.knxdb");
    let state = AppState::default();
    let tree = domain::new_project_impl(&state, None, None, None, None, true).unwrap();
    let installation = tree.installations[0].id;
    domain::save_project_as(&state, &path).unwrap();
    for name in ["First", "Second", "Third"] {
        domain::rename_installation_impl(&state, installation, name.into()).unwrap();
        assert_eq!(
            cached(&state),
            Some(stored(&path)),
            "after editing to {name}"
        );
    }
    assert!(
        knows_context(&state),
        "a journal write hands its context on"
    );
    domain::undo_impl(&state).unwrap();
    let rows = stored(&path);
    assert_eq!((rows.undo.len(), rows.redo.len()), (2, 1));
    assert_eq!(cached(&state), Some(rows.clone()));

    // A native open claims exactly the rows it recovered, so its first edit is incremental.
    let reopened = AppState::default();
    let progress = domain::detached_progress(crate::load_progress::LoadKind::Open, &path);
    domain::open_native_project(&reopened, &path, true, &progress).unwrap();
    assert_eq!(cached(&reopened), Some(rows));
    assert!(knows_context(&reopened), "an open hands its context on");
    domain::rename_installation_impl(&reopened, installation, "Fourth".into()).unwrap();
    assert_eq!(cached(&reopened), Some(stored(&path)));

    // Any other replacement has no stored journal to describe.
    domain::new_project_impl(&reopened, None, None, None, None, true).unwrap();
    assert_eq!(cached(&reopened), None);
}

#[test]
fn journal_writes_reach_the_diagnostic_environment_without_project_content() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.knxdb");
    let state = std::sync::Arc::new(AppState::default());
    let tree = domain::new_project_impl(&state, None, None, None, None, true).unwrap();
    let installation = tree.installations[0].id;
    domain::save_project_as(&state, &path).unwrap();
    for name in ["Secret installation name", "Another secret name"] {
        domain::rename_installation_impl(&state, installation, name.into()).unwrap();
    }
    let section = crate::diagnostic_bundle_routes::environment(&state);
    let file = section
        .files
        .iter()
        .find(|file| file.name == "environment.json")
        .unwrap();
    let text = String::from_utf8(file.bytes.clone()).unwrap();
    let environment: serde_json::Value = serde_json::from_str(&text).unwrap();
    let recent = environment["journalWrites"]["recent"].as_array().unwrap();
    assert_eq!(recent.len(), 2);
    let last = &recent[1];
    assert_eq!(last["ok"], true);
    assert_eq!(last["undoStates"], 2);
    assert_eq!(last["contextReused"], true);
    for phase in [
        "storeWait",
        "prepare",
        "preAdmission",
        "admission",
        "context",
        "commit",
        "retain",
    ] {
        assert!(
            last["phasesMs"].get(phase).is_some(),
            "phase {phase} in {last}"
        );
    }
    assert!(!text.contains("Secret"), "no project content: {text}");
    assert!(!text.contains("project.knxdb"), "no path: {text}");
}

fn journal_records(state: &AppState) -> Vec<crate::journal_write_log::JournalWriteRecord> {
    state
        .journal_writes
        .lock()
        .unwrap()
        .records()
        .cloned()
        .collect()
}

#[test]
fn an_unsaved_project_records_no_journal_write() {
    let state = AppState::default();
    let tree = domain::new_project_impl(&state, None, None, None, None, true).unwrap();
    domain::rename_installation_impl(&state, tree.installations[0].id, "Edited".into()).unwrap();
    assert!(journal_records(&state).is_empty());
}

#[test]
fn a_failed_journal_write_is_recorded_as_failed_with_its_phases() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.knxdb");
    let state = AppState::default();
    let tree = domain::new_project_impl(&state, None, None, None, None, true).unwrap();
    domain::save_project_as(&state, &path).unwrap();
    // The file disappears under the server: the read-only pre-admission refuses.
    std::fs::remove_file(&path).unwrap();
    let refused = domain::rename_installation_impl(&state, tree.installations[0].id, "Lost".into());
    assert!(refused.is_err());
    let records = journal_records(&state);
    let last = records.last().expect("the failed write is recorded");
    assert!(!last.ok);
    let names: Vec<_> = last.report.phases.iter().map(|(name, _)| *name).collect();
    assert_eq!(names, ["storeWait", "prepare", "preAdmission"]);
}
