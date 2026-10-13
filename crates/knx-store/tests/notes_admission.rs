//! Verifies notes in versioned native storage and authentic predecessor history.

use knx_core::note::{ProjectNote, ProjectNoteTarget as Target};
use knx_core::{Command, DeviceId, Language, Project, ProjectNoteId};
use knx_store::project_history::{self as history, NativeSnapshot};

fn note(id: u32, target: Target) -> ProjectNote {
    ProjectNote::new(
        ProjectNoteId(id),
        target,
        " why Grüße🙂 ".into(),
        "Literal\r\n<img src=\"https://untrusted.invalid/never\">\n\t".into(),
    )
    .unwrap()
    .with_documentation(true)
}
#[test]
fn notes_order_target_text_inclusion_and_highwater_survive_native_reopen_and_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let mut project = Project::new(Language("en".into()));
    Command::InsertProjectNote {
        note: note(5, Target::Project),
        position: 0,
    }
    .apply(&mut project)
    .unwrap();
    Command::InsertProjectNote {
        note: note(2, Target::Project).with_documentation(false),
        position: 1,
    }
    .apply(&mut project)
    .unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);
    let snapshot = NativeSnapshot::read(&conn).unwrap();
    let bytes = snapshot.encode().unwrap();
    assert_eq!(
        NativeSnapshot::decode(&bytes, &history::image_hash(&bytes)).unwrap(),
        snapshot
    );
    drop(conn);
    let reopened = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(knx_store::load_project(&reopened.conn).unwrap(), project);
    assert_eq!(project.ids.peek_project_note(), 5);
    assert_eq!(
        reopened
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
}
#[test]
fn removed_notes_do_not_rewind_persisted_allocator_or_semantic_identity() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let project = Project::new(Language("en".into()));
    knx_store::save_project(&conn, &project).unwrap();
    let original = NativeSnapshot::read(&conn)
        .unwrap()
        .semantic_hash()
        .unwrap();
    let mut edited = project;
    edited.ids = edited.ids.with_project_note_count(5);
    knx_store::save_project(&conn, &edited).unwrap();
    let mut reopened = knx_store::load_project(&conn).unwrap();
    assert_eq!(
        reopened.ids.next_project_note_id().unwrap(),
        ProjectNoteId(6)
    );
    assert_ne!(
        NativeSnapshot::read(&conn)
            .unwrap()
            .semantic_hash()
            .unwrap(),
        original
    );
}
#[test]
fn invalid_note_target_refuses_without_changing_native_bytes_or_saved_project() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let project = Project::new(Language("en".into()));
    knx_store::save_project(&conn, &project).unwrap();
    let bytes = conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
    let mut invalid = project.clone();
    invalid.notes.push(note(1, Target::Device(DeviceId(99))));
    invalid.ids = invalid.ids.with_project_note_count(1);
    assert!(knx_store::save_project(&conn, &invalid).is_err());
    assert_eq!(conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec(), bytes);
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);
}
#[test]
fn authentic_native13_saved_and_working_both_stacks_migrate_without_rewriting_originals() {
    for (name, bytes) in [
        (
            "saved",
            include_bytes!("fixtures/native13-before-notes-saved.sqlite").as_slice(),
        ),
        (
            "working",
            include_bytes!("fixtures/native13-before-notes-working.sqlite").as_slice(),
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("predecessor.knxdb");
        std::fs::write(&path, bytes).unwrap();
        let read = knx_store::open_existing_read_only(&path).unwrap();
        assert_eq!(read.migrated_from, Some(13));
        let root = NativeSnapshot::read(&read.conn).unwrap();
        assert!(root.project.notes.is_empty());
        assert_eq!(root.project.ids.peek_project_note(), 0);
        let editor = history::load_editor(&read.conn).unwrap().unwrap();
        assert!(editor.working.project.notes.is_empty());
        assert_eq!(editor.working.project.ids.peek_project_note(), 0);
        if name == "working" {
            assert_eq!(editor.undo.len(), 3);
            assert_eq!(editor.redo.len(), 1);
            assert_eq!(
                editor.working.project.instructions[&DeviceId(1)].runs()[0].text,
                "Working two Grüße🙂"
            );
        }
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
        drop(read);
        let migrated = knx_store::open_existing_and_migrate(&path).unwrap();
        assert_eq!(
            migrated
                .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            15
        );
        assert_eq!(
            history::load_editor(&migrated).unwrap().unwrap().working,
            editor.working
        );
        assert_eq!(NativeSnapshot::read(&migrated).unwrap(), root);
    }
}

#[test]
fn malformed_note_rows_refuse_without_guessing_or_repairing_authored_text() {
    for mutation in [
        "UPDATE project_note SET position=10",
        "UPDATE project_note SET target_kind='unknown'",
        "UPDATE project_note SET target_kind='device',target_id=99",
        "UPDATE project_note SET title='   '",
        "UPDATE project_note SET body=char(0)",
        "UPDATE project_note SET include_in_documentation=9",
        "UPDATE project_note SET body=printf('%.*c',65537,'x')",
        "DELETE FROM project_note_state",
    ] {
        let conn = knx_store::open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(Language("en".into()));
        Command::InsertProjectNote {
            note: note(1, Target::Project),
            position: 0,
        }
        .apply(&mut project)
        .unwrap();
        knx_store::save_project(&conn, &project).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints=ON")
            .unwrap();
        conn.execute_batch(mutation).unwrap();
        let bytes = conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
        assert!(knx_store::load_project(&conn).is_err(), "{mutation}");
        assert_eq!(conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec(), bytes);
    }
}
#[test]
fn stale_note_counter_uses_existing_explicit_allocator_repair_report() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let mut project = Project::new(Language("en".into()));
    Command::InsertProjectNote {
        note: note(5, Target::Project),
        position: 0,
    }
    .apply(&mut project)
    .unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    conn.execute("UPDATE project_note_state SET last_id=1", [])
        .unwrap();
    let bytes = conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
    let (mut loaded, report) = knx_store::load_project_reporting(&conn).unwrap();
    let repair = report.unwrap();
    assert_eq!(repair.stored.peek_project_note(), 1);
    assert_eq!(repair.repaired.peek_project_note(), 5);
    assert_eq!(loaded.notes, project.notes);
    assert_eq!(loaded.ids.next_project_note_id().unwrap(), ProjectNoteId(6));
    assert_eq!(conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec(), bytes);
}
#[test]
fn note_working_edits_and_both_stacks_survive_reopen_and_undo_redo() {
    use knx_core::CommandStack;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("notes-history.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let mut working = NativeSnapshot {
        project: Project::new(Language("en".into())),
        opaque: vec![],
        manufacturer_refs: vec![],
    };
    knx_store::save_project(&conn, &working.project).unwrap();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut working.project,
            Command::InsertProjectNote {
                note: note(1, Target::Project),
                position: 0,
            },
        )
        .unwrap();
    stack
        .do_command(
            &mut working.project,
            Command::ReplaceProjectNote {
                note: ProjectNote::new(
                    ProjectNoteId(1),
                    Target::Project,
                    "edited".into(),
                    "literal Grüße🙂".into(),
                )
                .unwrap(),
            },
        )
        .unwrap();
    stack.undo(&mut working.project).unwrap();
    let generation = history::save_editor(&conn, &working, &stack, None, false).unwrap();
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let loaded = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(loaded.generation, generation);
    assert_eq!(loaded.working, working);
    assert_eq!(loaded.undo.len(), 1);
    assert_eq!(loaded.redo.len(), 1);
    let mut current = loaded.working;
    let mut stack = CommandStack::from_snapshot_states(loaded.undo, loaded.redo);
    stack.undo(&mut current.project).unwrap();
    assert!(current.project.notes.is_empty());
    assert_eq!(current.project.ids.peek_project_note(), 1);
    stack.redo(&mut current.project).unwrap();
    assert_eq!(current, working);
    stack.redo(&mut current.project).unwrap();
    assert_eq!(current.project.notes[0].title(), "edited");
    history::save_editor(&conn, &current, &stack, Some(generation), true).unwrap();
    assert_eq!(NativeSnapshot::read(&conn).unwrap(), current);
}
