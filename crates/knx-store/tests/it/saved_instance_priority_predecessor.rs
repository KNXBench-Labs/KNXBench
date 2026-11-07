//! Admits authentic predecessor files without deriving captured facts from old retained attributes.

use knx_core::{CommandStack, SavedInstancePriority};
use knx_store::project_history::{self as history};
use rusqlite::{types::Value, Connection, OpenFlags};
use std::path::{Path, PathBuf};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/native16-before-instance-priority.sqlite")
}
fn history_rows(conn: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
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
        let rows = stmt
            .query_map([], |row| {
                (0..columns)
                    .map(|i| row.get::<_, Value>(i))
                    .collect::<Result<Vec<_>, _>>()
            })
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        (table.into(), rows)
    })
    .collect()
}
fn not_captured(project: &knx_core::Project) {
    let objects: Vec<_> = project.devices.com_objects().collect();
    assert!(
        !objects.is_empty(),
        "an object-free predecessor cannot witness this migration"
    );
    for object in objects {
        assert_eq!(
            object.saved_instance_priority,
            SavedInstancePriority::default()
        );
    }
}
fn copy_fixture() -> (tempfile::TempDir, PathBuf, Vec<u8>) {
    let bytes = std::fs::read(fixture())
        .expect("attach the verified real V16 writer fixture before running this test");
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("predecessor.knxdb");
    std::fs::write(&path, &bytes).unwrap();
    (dir, path, bytes)
}

#[test]
fn saved_instance_priority_native16_predecessor_root_working_stacks_named_and_read_only() {
    let (_dir, path, bytes) = copy_fixture();
    let original = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        original
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        16
    );
    let rows = history_rows(&original);
    let opaque = knx_store::load_opaque(&original).unwrap();
    drop(original);
    assert!(opaque
        .iter()
        .any(|e| e.name == "Priority" && e.bytes.is_empty()));
    assert!(opaque
        .iter()
        .any(|e| e.name == "Priority" && e.xpath.is_empty()));
    let store = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(store.migrated_from, Some(16));
    assert_eq!(
        store
            .conn
            .query_row("SELECT count(*) FROM com_object_saved_priority", [], |r| {
                r.get::<_, i64>(0)
            })
            .unwrap(),
        0
    );
    assert_eq!(history_rows(&store.conn), rows);
    assert_eq!(knx_store::load_opaque(&store.conn).unwrap(), opaque);
    let root = knx_store::load_project(&store.conn).unwrap();
    not_captured(&root);
    let recovered = history::load_editor(&store.conn).unwrap().unwrap();
    not_captured(&recovered.baseline.project);
    not_captured(&recovered.working.project);
    assert_ne!(recovered.baseline.project, recovered.working.project);
    assert_eq!(recovered.generation, 3);
    assert_eq!(recovered.undo.len(), 2);
    assert_eq!(recovered.redo.len(), 1);
    for project in recovered.undo.iter().chain(&recovered.redo) {
        not_captured(project);
    }
    assert_eq!(recovered.baseline.opaque, opaque);
    assert_eq!(recovered.working.opaque, opaque);
    let versions = history::list_versions(&store.conn).unwrap();
    let named = versions
        .iter()
        .filter(|v| v.reason == "named")
        .collect::<Vec<_>>();
    assert_eq!(named.len(), 1);
    let named_id = named[0].id;
    let version = history::read_version(&store.conn, named_id).unwrap();
    not_captured(&version.project);
    assert_eq!(version.opaque, opaque);
    let expected_stack = (recovered.undo.clone(), recovered.redo.clone());
    let mut project = recovered.working.project.clone();
    let mut stack = CommandStack::from_snapshot_states(expected_stack.0, expected_stack.1);
    stack.undo(&mut project).unwrap();
    not_captured(&project);
    stack.redo(&mut project).unwrap();
    assert!(project == recovered.working.project);
    stack.redo(&mut project).unwrap();
    not_captured(&project);
    drop(store);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    let original = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    assert_eq!(
        original
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        16
    );
    drop(original);
    // A later owned write migrates and preserves original fact/context semantics.
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(history_rows(&conn), rows);
    let g = history::save_editor(
        &conn,
        &recovered.working,
        &CommandStack::from_snapshot_states(recovered.undo, recovered.redo),
        Some(recovered.generation),
        false,
    )
    .unwrap();
    let result = history::change_history(
        &conn,
        &recovered.working,
        g,
        history::HistoryAction::Restore(named_id),
    )
    .unwrap();
    let restored = result.restored.unwrap();
    not_captured(&restored.working.project);
    assert!(restored.working == version);
    assert_eq!(knx_store::load_opaque(&conn).unwrap(), opaque);
}

#[test]
fn saved_instance_priority_predecessor_post_migration_failure_rolls_back_version_and_images() {
    let (_dir, path, bytes) = copy_fixture();
    let read = knx_store::open_existing_read_only(&path).unwrap();
    let recovered = history::load_editor(&read.conn).unwrap().unwrap();
    drop(read);
    let conn = Connection::open(&path).unwrap();
    conn.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let before = history_rows(&conn);
    let stale = history::save_editor(
        &conn,
        &recovered.working,
        &CommandStack::new(),
        Some(recovered.generation + 99),
        false,
    );
    assert!(stale.is_err());
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        16
    );
    assert_eq!(history_rows(&conn), before);
    drop(conn);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    // A later named-restore refusal must share the same owned upgrade transaction.
    let conn = Connection::open(&path).unwrap();
    let before = history_rows(&conn);
    assert!(history::change_history(
        &conn,
        &recovered.working,
        recovered.generation,
        history::HistoryAction::Restore(-999)
    )
    .is_err());
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        16
    );
    assert_eq!(history_rows(&conn), before);
    drop(conn);
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
}

#[test]
fn saved_instance_priority_predecessor_history_hash_corruption_is_not_repaired() {
    for table in [
        "project_history_state",
        "project_history_context",
        "project_history_stack",
        "project_history_version",
    ] {
        let (_dir, path, _) = copy_fixture();
        let conn = Connection::open(&path).unwrap();
        let hash = if table == "project_history_state" {
            "working_hash"
        } else {
            "image_hash"
        };
        conn.execute(&format!("UPDATE {table} SET {hash}=?1"), ["0".repeat(64)])
            .unwrap();
        drop(conn);
        let corrupt = std::fs::read(&path).unwrap();
        let store = knx_store::open_existing_read_only(&path).unwrap();
        let result = if table == "project_history_version" {
            history::list_versions(&store.conn).map(|_| ())
        } else {
            history::load_editor(&store.conn).map(|_| ())
        };
        assert!(result.is_err(), "corrupt {table} was admitted");
        drop(store);
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
    }
}

#[test]
fn saved_instance_priority_authentic_v12_v13_empty_overlay_and_original_hashes() {
    for (name, version) in [
        ("fixtures/project_v12_instruction_history.sqlite", 12),
        ("fixtures/project_v12_instruction_history_device.sqlite", 12),
        ("tests/fixtures/native13-before-notes-saved.sqlite", 13),
        ("tests/fixtures/native13-before-notes-working.sqlite", 13),
    ] {
        let source = Path::new(env!("CARGO_MANIFEST_DIR")).join(name);
        let bytes = std::fs::read(&source).unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("old.knxdb");
        std::fs::write(&path, &bytes).unwrap();
        let original =
            Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let rows = history_rows(&original);
        drop(original);
        let store = knx_store::open_existing_read_only(&path).unwrap();
        assert_eq!(store.migrated_from, Some(version));
        assert_eq!(history_rows(&store.conn), rows);
        assert_eq!(
            store
                .conn
                .query_row("SELECT count(*) FROM com_object_saved_priority", [], |r| {
                    r.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
        let recovered = history::load_editor(&store.conn).unwrap().unwrap();
        for project in std::iter::once(&recovered.baseline.project)
            .chain(std::iter::once(&recovered.working.project))
            .chain(&recovered.undo)
            .chain(&recovered.redo)
        {
            for object in project.devices.com_objects() {
                assert_eq!(
                    object.saved_instance_priority,
                    SavedInstancePriority::default()
                );
            }
        }
        drop(store);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[test]
fn saved_instance_priority_old_native_never_backfills_even_matching_or_corrupt_opaque_hashes() {
    for corrupt in [false, true] {
        let (_dir, path, _) = copy_fixture();
        if corrupt {
            let conn = Connection::open(&path).unwrap();
            let changed = conn
                .execute(
                    "UPDATE opaque_entry SET sha256=?1 WHERE name='Priority'",
                    ["0".repeat(64)],
                )
                .unwrap();
            assert_eq!(changed, 5);
        }
        let before = std::fs::read(&path).unwrap();
        let original =
            Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
        let retained = knx_store::load_opaque(&original).unwrap();
        drop(original);
        let store = knx_store::open_existing_read_only(&path).unwrap();
        not_captured(&knx_store::load_project(&store.conn).unwrap());
        assert_eq!(knx_store::load_opaque(&store.conn).unwrap(), retained);
        assert_eq!(
            store
                .conn
                .query_row("SELECT count(*) FROM com_object_saved_priority", [], |r| {
                    r.get::<_, i64>(0)
                })
                .unwrap(),
            0
        );
        drop(store);
        assert_eq!(std::fs::read(&path).unwrap(), before);
    }
}
