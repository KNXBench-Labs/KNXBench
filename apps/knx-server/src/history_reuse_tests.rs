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
