//! Native 18 user priority edits: authentic v17 migration, persistence and refusal.

use knx_core::{
    ComObjectInstanceId, Command, CommandStack, ObjectPriority, Project, SavedInstancePriority,
};
use knx_store::project_history::{self as history, NativeSnapshot};
use knx_store::StoreError;
use rusqlite::{types::Value, Connection, OpenFlags};
use std::path::{Path, PathBuf};

/// Semantic identities the unchanged v17 writer recorded for its own images
/// (`native17-before-priority-edit.provenance.json`, `semantic_sha256`).
const BASELINE_COMPLETE: &str = "b3b66cf29a3e4c96e80bd3eaa92c133faa22d15babfe40c2f831c79d9db222ff";
const WORKING_COMPLETE: &str = "e0295ff79d1aa53169431bae5c4415f43074e493d8d734f17b34f04df44c378d";
const UNDO: [&str; 2] = [
    "a3fe3ac3584dbb6a11a36a38839c0235ecb1fbf87e51b722c8b663f1b7ee0435",
    "7b96e6cdf238b43996da21d4991a0dfcee2e60c025da41b6d09a8e6c53f33456",
];
const REDO: &str = "5d60f69983d9fb34c087597c7cb65787c11276df2acc5272a5654a212523c2ed";

fn copy_fixture() -> (tempfile::TempDir, PathBuf, Vec<u8>) {
    let source = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native17-before-priority-edit.sqlite");
    let bytes = std::fs::read(source).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("predecessor.knxdb");
    std::fs::write(&path, &bytes).unwrap();
    (dir, path, bytes)
}

fn user_version(conn: &Connection) -> i64 {
    conn.query_row("PRAGMA user_version", [], |r| r.get(0))
        .unwrap()
}

fn edit_rows(conn: &Connection) -> i64 {
    conn.query_row("SELECT count(*) FROM com_object_priority_edit", [], |r| {
        r.get(0)
    })
    .unwrap()
}

fn history_rows(conn: &Connection) -> Vec<Vec<Vec<Value>>> {
    [
        "project_history_context",
        "project_history_state",
        "project_history_stack",
        "project_history_version",
    ]
    .into_iter()
    .map(|table| {
        let mut stmt = conn
            .prepare(&format!("SELECT * FROM {table} ORDER BY 1,2"))
            .unwrap();
        let columns = stmt.column_count();
        stmt.query_map([], |row| {
            (0..columns)
                .map(|i| row.get::<_, Value>(i))
                .collect::<Result<Vec<_>, _>>()
        })
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap()
    })
    .collect()
}

/// Every edit is `None` and the importer's captured declarations are intact.
fn unedited_with_captured_facts(project: &Project) {
    let mut declared = vec![];
    let mut absent = 0;
    for object in project.devices.com_objects() {
        assert_eq!(object.priority_edit, None, "migration invented an edit");
        match &object.saved_instance_priority {
            SavedInstancePriority::Declared(d) => declared.push(d.raw.clone()),
            SavedInstancePriority::Absent(_) => absent += 1,
            other => panic!("captured fact changed: {other:?}"),
        }
    }
    declared.sort();
    assert_eq!(declared, ["", "Alert", "Future-\u{3a9}", "High"]);
    assert_eq!(absent, 2);
}

fn model(project: &Project) -> NativeSnapshot {
    NativeSnapshot {
        project: project.clone(),
        opaque: vec![],
        manufacturer_refs: vec![],
    }
}

fn first_object(project: &Project) -> ComObjectInstanceId {
    project.devices.com_objects().next().unwrap().id
}

#[test]
fn native17_predecessor_migrates_without_edits_and_keeps_every_identity() {
    let (_dir, path, bytes) = copy_fixture();
    let original = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(user_version(&original), 17);
    let rows = history_rows(&original);
    drop(original);

    let store = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(store.migrated_from, Some(17));
    assert_eq!(edit_rows(&store.conn), 0);
    assert_eq!(history_rows(&store.conn), rows);
    unedited_with_captured_facts(&knx_store::load_project(&store.conn).unwrap());
    let recovered = history::load_editor(&store.conn).unwrap().unwrap();
    assert_eq!(
        recovered.baseline.semantic_hash().unwrap(),
        BASELINE_COMPLETE
    );
    assert_eq!(recovered.working.semantic_hash().unwrap(), WORKING_COMPLETE);
    assert_eq!(recovered.undo.len(), 2);
    for (project, expected) in recovered.undo.iter().zip(UNDO) {
        unedited_with_captured_facts(project);
        assert_eq!(model(project).semantic_hash().unwrap(), expected);
    }
    assert_eq!(model(&recovered.redo[0]).semantic_hash().unwrap(), REDO);
    let named = history::list_versions(&store.conn).unwrap();
    assert_eq!(named.len(), 1);
    unedited_with_captured_facts(
        &history::read_version(&store.conn, named[0].id)
            .unwrap()
            .project,
    );
    drop(store);
    assert_eq!(std::fs::read(&path).unwrap(), bytes, "read-only open wrote");

    // An owned write migrates; an edit then rides the old history.
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(user_version(&conn), 18);
    assert_eq!(history_rows(&conn), rows);
    let mut working = recovered.working.clone();
    let mut stack = CommandStack::from_snapshot_states(recovered.undo, recovered.redo);
    let target = first_object(&working.project);
    stack
        .do_command(
            &mut working.project,
            Command::SetComObjectPriority {
                com_object: target,
                priority: Some(ObjectPriority::Alert),
            },
        )
        .unwrap();
    assert_ne!(working.semantic_hash().unwrap(), WORKING_COMPLETE);
    let generation =
        history::save_editor(&conn, &working, &stack, Some(recovered.generation), false).unwrap();
    drop(conn);

    let store = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(store.migrated_from, None);
    let reopened = history::load_editor(&store.conn).unwrap().unwrap();
    assert_eq!(reopened.generation, generation);
    let edited = reopened.working.project.devices.com_object(target).unwrap();
    assert_eq!(edited.priority_edit, Some(ObjectPriority::Alert));
    let mut project = reopened.working.project.clone();
    let mut stack = CommandStack::from_snapshot_states(reopened.undo, reopened.redo);
    stack.undo(&mut project).unwrap();
    unedited_with_captured_facts(&project);
    assert_eq!(model(&project), model(&recovered.working.project));
}

#[test]
fn edits_save_reload_clear_and_change_the_semantic_identity() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let (_dir, path, _) = copy_fixture();
    let old = knx_store::open_existing_read_only(&path).unwrap();
    let mut project = knx_store::load_project(&old.conn).unwrap();
    drop(old);
    let ids: Vec<_> = project.devices.com_objects().map(|c| c.id).collect();
    let before = model(&project).semantic_hash().unwrap();
    for (id, priority) in [
        (ids[0], ObjectPriority::Low),
        (ids[3], ObjectPriority::High),
    ] {
        Command::SetComObjectPriority {
            com_object: id,
            priority: Some(priority),
        }
        .apply(&mut project)
        .unwrap();
    }
    knx_store::save_project(&conn, &project).unwrap();
    assert_eq!(edit_rows(&conn), 2);
    let raw: Vec<String> = conn
        .prepare("SELECT priority FROM com_object_priority_edit ORDER BY com_object_instance_id")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(raw, ["Low", "High"], "stored as exact schema literals");
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);
    let edited = model(&project);
    assert_ne!(edited.semantic_hash().unwrap(), before);
    let image = edited.encode().unwrap();
    let decoded = NativeSnapshot::decode(&image, &history::image_hash(&image)).unwrap();
    assert_eq!(decoded, edited, "history images carry the edit");

    for id in [ids[0], ids[3]] {
        Command::SetComObjectPriority {
            com_object: id,
            priority: None,
        }
        .apply(&mut project)
        .unwrap();
    }
    knx_store::save_project(&conn, &project).unwrap();
    assert_eq!(edit_rows(&conn), 0);
    assert_eq!(model(&project).semantic_hash().unwrap(), before);
}

#[test]
fn an_orphan_or_non_token_edit_row_is_refused_on_load() {
    let (_dir, path, _) = copy_fixture();
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let project = knx_store::load_project(&conn).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    let check = conn.execute(
        "INSERT INTO com_object_priority_edit VALUES (?1, 'low')",
        [first_object(&project).0],
    );
    assert!(check.is_err(), "the CHECK refuses a non-token");
    conn.execute_batch("PRAGMA foreign_keys=OFF;").unwrap();
    conn.execute(
        "INSERT INTO com_object_priority_edit VALUES (999999, 'Low')",
        [],
    )
    .unwrap();
    assert!(matches!(
        knx_store::load_project(&conn),
        Err(StoreError::PriorityEditIntegrity)
    ));
    conn.execute_batch(
        "DELETE FROM com_object_priority_edit;
         DROP TABLE com_object_priority_edit;
         CREATE TABLE com_object_priority_edit (
             com_object_instance_id INTEGER PRIMARY KEY REFERENCES com_object_instance(id),
             priority TEXT NOT NULL) STRICT;",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO com_object_priority_edit VALUES (?1, 'Urgent')",
        [first_object(&project).0],
    )
    .unwrap();
    assert!(matches!(
        knx_store::load_project(&conn),
        Err(StoreError::PriorityEditIntegrity)
    ));
}
