//! Refuses unpublished note state before native notes storage is admitted.

use knx_core::note::{ProjectNote, ProjectNoteTarget};
use knx_core::{Language, Project, ProjectNoteId};
#[test]
fn native13_cannot_silently_discard_new_authored_note_state() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let project = Project::new(Language("en".into()));
    knx_store::save_project(&conn, &project).unwrap();
    let bytes = conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
    let mut edited = project.clone();
    edited.notes.push(
        ProjectNote::new(
            ProjectNoteId(1),
            ProjectNoteTarget::Project,
            "why".into(),
            "reason".into(),
        )
        .unwrap(),
    );
    assert!(knx_store::save_project(&conn, &edited).is_err());
    assert_eq!(conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec(), bytes);
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);
}
#[test]
fn native13_cannot_silently_rewind_the_new_note_allocator() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let project = Project::new(Language("en".into()));
    knx_store::save_project(&conn, &project).unwrap();
    let bytes = conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
    let mut edited = project.clone();
    edited.ids = edited.ids.with_project_note_count(5);
    assert!(knx_store::save_project(&conn, &edited).is_err());
    assert_eq!(conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec(), bytes);
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);
}
