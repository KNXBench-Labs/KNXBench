//! Readers and failures leave project files alone, and a save is one transaction.
//
// AR18 independent review, findings M1 and M2 (2026-10-06).

use knx_core::string_table::Language;
use knx_core::Project;
use knx_store::{
    load_manufacturer_refs, load_opaque, load_project, open_and_migrate, open_existing_and_migrate,
    save_project_with_passthrough, Connection, ManufacturerRef, MigrationError, StoredOpaqueEntry,
};

fn project(name: &str) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.name = name.into();
    project
}

fn opaque(tag: &str) -> Vec<StoredOpaqueEntry> {
    vec![StoredOpaqueEntry {
        source_path: format!("P-0001/{tag}.bin"),
        xpath: String::new(),
        kind: "Baggage".into(),
        name: String::new(),
        bytes: tag.as_bytes().to_vec(),
        sha256: format!("sha-{tag}"),
    }]
}

fn refs(tag: &str) -> Vec<ManufacturerRef> {
    vec![ManufacturerRef {
        source_path: format!("M-7FF0/{tag}.xml"),
        sha256: format!("sha-{tag}"),
        len: 1,
        kind: "ManufacturerData".into(),
    }]
}

#[test]
fn a_save_that_fails_in_its_last_table_changes_none_of_the_three() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.knxdb");
    let conn = open_and_migrate(&path).unwrap();
    save_project_with_passthrough(&conn, &project("Old"), &opaque("old"), &refs("old")).unwrap();
    conn.execute_batch(
        "CREATE TEMP TRIGGER refuse BEFORE INSERT ON main.manufacturer_ref
         BEGIN SELECT RAISE(ABORT, 'disk full, say'); END;",
    )
    .unwrap();

    let failed =
        save_project_with_passthrough(&conn, &project("New"), &opaque("new"), &refs("new"));
    assert!(
        failed.unwrap_err().to_string().contains("disk full, say"),
        "the injected last-table failure must actually run"
    );
    assert_eq!(load_project(&conn).unwrap().info.name, "Old");
    assert_eq!(load_opaque(&conn).unwrap(), opaque("old"));
    assert_eq!(load_manufacturer_refs(&conn).unwrap(), refs("old"));
}

#[test]
fn a_successful_save_writes_all_three() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    save_project_with_passthrough(&conn, &project("New"), &opaque("new"), &refs("new")).unwrap();
    assert_eq!(load_project(&conn).unwrap().info.name, "New");
    assert_eq!(load_opaque(&conn).unwrap(), opaque("new"));
    assert_eq!(load_manufacturer_refs(&conn).unwrap(), refs("new"));
}

#[test]
fn a_foreign_sqlite_file_without_a_version_is_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("someone-else.sqlite");
    Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE notes (body TEXT); INSERT INTO notes VALUES ('mine');")
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    for open in [open_and_migrate, open_existing_and_migrate] {
        assert!(matches!(open(&path), Err(MigrationError::ForeignDatabase)));
        assert!(
            std::fs::read(&path).unwrap() == before,
            "the file must stay byte-identical"
        );
    }
}

#[test]
fn a_versioned_sqlite_file_without_the_knxbench_marker_is_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("other-app.sqlite");
    Connection::open(&path)
        .unwrap()
        .execute_batch("CREATE TABLE settings (k TEXT); PRAGMA user_version = 3;")
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        open_and_migrate(&path),
        Err(MigrationError::ForeignDatabase)
    ));
    assert!(std::fs::read(&path).unwrap() == before);
}

#[test]
fn a_schema_meta_table_from_another_application_is_not_the_marker() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("lookalike.sqlite");
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL);
             INSERT INTO schema_meta VALUES ('created_by', 'some-other-tool');
             PRAGMA user_version = 2;",
        )
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        open_and_migrate(&path),
        Err(MigrationError::ForeignDatabase)
    ));
    assert!(std::fs::read(&path).unwrap() == before);
}

#[test]
fn a_current_store_that_never_saved_a_project_is_refused_by_readers() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("initialised.knxdb");
    drop(open_and_migrate(&path).unwrap());
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        open_existing_and_migrate(&path),
        Err(MigrationError::NothingSaved)
    ));
    assert!(std::fs::read(&path).unwrap() == before);
    // Once a project is saved, the reader opens it.
    let conn = open_and_migrate(&path).unwrap();
    save_project_with_passthrough(&conn, &project("Saved"), &[], &[]).unwrap();
    drop(conn);
    let conn = open_existing_and_migrate(&path).unwrap();
    assert_eq!(load_project(&conn).unwrap().info.name, "Saved");
}

#[test]
fn a_writer_still_initialises_an_empty_file_but_a_reader_leaves_it_alone() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("fresh.knxdb");
    std::fs::write(&path, b"").unwrap();
    assert!(matches!(
        open_existing_and_migrate(&path),
        Err(MigrationError::NothingSaved)
    ));
    assert_eq!(std::fs::read(&path).unwrap(), b"");
    assert!(open_and_migrate(&path).is_ok());
    assert!(open_and_migrate(&dir.path().join("brand-new.knxdb")).is_ok());
}

#[test]
fn an_older_store_without_a_saved_project_is_refused_before_migration() {
    // AR18 review M1c: GUI Open of an empty v3 file answered 500 after
    // upgrading it. A KNXBench store at v3 cannot hold a project yet.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.knxdb");
    Connection::open(&path)
        .unwrap()
        .execute_batch(
            "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
             INSERT INTO schema_meta VALUES ('created_by', 'knx-store');
             PRAGMA user_version = 3;",
        )
        .unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(matches!(
        open_existing_and_migrate(&path),
        Err(MigrationError::NothingSaved)
    ));
    assert!(std::fs::read(&path).unwrap() == before);
}

#[test]
fn opening_an_existing_file_never_creates_one_at_a_missing_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("typo.knxdb");
    assert!(matches!(
        open_existing_and_migrate(&path),
        Err(MigrationError::NotFound)
    ));
    assert!(!path.exists());
}
