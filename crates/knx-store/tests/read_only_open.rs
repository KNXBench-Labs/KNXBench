//! Read-only opening never changes a project file, whatever its schema version.
//
// ADR-0090: an agent's question must never rewrite the user's file. Every
// case compares the file's bytes before and after the open and load.

use std::path::Path;

use knx_core::string_table::Language;
use knx_core::Project;
use knx_store::{
    load_project, open_and_migrate, open_existing_read_only, save_project, MigrationError,
    CURRENT_SCHEMA_VERSION,
};

fn saved_project(path: &Path, name: &str) {
    let conn = open_and_migrate(path).unwrap();
    let mut project = Project::new(Language("en".into()));
    project.info.name = name.into();
    save_project(&conn, &project).unwrap();
}

/// Rewrites a current-version store into the v9 layout that
/// `migrate_v9_to_v10` upgrades, keeping its saved project.
fn downgrade_to_v9(path: &Path) {
    let conn = knx_store::Connection::open(path).unwrap();
    conn.execute_batch(
        "ALTER TABLE group_address DROP COLUMN dpt_state;
         ALTER TABLE group_address DROP COLUMN dpt_value;
         ALTER TABLE group_address DROP COLUMN dpt_layer;
         ALTER TABLE project_info DROP COLUMN unlifted_group_address_dpt_declarations;
         PRAGMA user_version = 9;",
    )
    .unwrap();
}

fn sibling_files(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_current_store_opens_loads_and_stays_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.knxdb");
    saved_project(&path, "Current");
    let before = std::fs::read(&path).unwrap();

    let store = open_existing_read_only(&path).unwrap();
    assert_eq!(store.migrated_from, None);
    assert_eq!(load_project(&store.conn).unwrap().info.name, "Current");
    drop(store);

    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(sibling_files(dir.path()), vec!["p.knxdb".to_string()]);
}

#[test]
fn a_read_only_connection_refuses_writes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.knxdb");
    saved_project(&path, "Current");
    let before = std::fs::read(&path).unwrap();

    let store = open_existing_read_only(&path).unwrap();
    let write = store
        .conn
        .execute("UPDATE project_info SET name = 'Changed'", []);
    assert!(write.is_err(), "a read-only store must refuse an UPDATE");
    drop(store);

    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn an_older_store_is_migrated_in_memory_and_the_file_keeps_its_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.knxdb");
    saved_project(&path, "Older");
    downgrade_to_v9(&path);
    let before = std::fs::read(&path).unwrap();

    let store = open_existing_read_only(&path).unwrap();
    assert_eq!(store.migrated_from, Some(9));
    let version: i64 = store
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, CURRENT_SCHEMA_VERSION);
    assert_eq!(load_project(&store.conn).unwrap().info.name, "Older");
    drop(store);

    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(sibling_files(dir.path()), vec!["old.knxdb".to_string()]);
    let file = knx_store::Connection::open(&path).unwrap();
    let version: i64 = file
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, 9, "the file itself must not be upgraded");
}

#[test]
fn a_newer_store_is_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("new.knxdb");
    saved_project(&path, "Future");
    knx_store::Connection::open(&path)
        .unwrap()
        .pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
        .unwrap();
    let before = std::fs::read(&path).unwrap();

    let error = open_existing_read_only(&path).err().unwrap();
    assert!(matches!(
        error,
        MigrationError::FutureSchemaVersion { found, .. } if found == CURRENT_SCHEMA_VERSION + 1
    ));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_foreign_database_is_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("foreign.sqlite");
    knx_store::Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE other (x INTEGER); PRAGMA user_version = 3;")
        .unwrap();
    let before = std::fs::read(&path).unwrap();

    let error = open_existing_read_only(&path).err().unwrap();
    assert!(matches!(error, MigrationError::ForeignDatabase));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_store_without_a_saved_project_is_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("empty.knxdb");
    drop(open_and_migrate(&path).unwrap());
    let before = std::fs::read(&path).unwrap();

    let error = open_existing_read_only(&path).err().unwrap();
    assert!(matches!(error, MigrationError::NothingSaved));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_missing_path_is_not_found_and_nothing_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent.knxdb");

    let error = open_existing_read_only(&path).err().unwrap();
    assert!(matches!(error, MigrationError::NotFound));
    assert!(!path.exists());
    assert!(sibling_files(dir.path()).is_empty());
}

#[test]
fn a_file_that_is_not_sqlite_is_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("text.knxdb");
    std::fs::write(&path, b"definitely not a database, just vibes").unwrap();
    let before = std::fs::read(&path).unwrap();

    assert!(open_existing_read_only(&path).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn an_open_during_a_save_waits_for_the_lock_instead_of_failing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.knxdb");
    saved_project(&path, "Busy");

    // A writer holding the exclusive lock, as a save's commit does.
    let writer = knx_store::Connection::open(&path).unwrap();
    writer.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        writer.execute_batch("COMMIT").unwrap();
    });

    let store = open_existing_read_only(&path).expect("waits for the writer");
    assert_eq!(load_project(&store.conn).unwrap().info.name, "Busy");
    release.join().unwrap();
}
