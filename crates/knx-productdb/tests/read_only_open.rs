//! Opening the product database read-only never changes the file, whatever its version.
//
// ADR-0090: the read-only MCP adapter keeps this connection open for its
// whole lifetime, so it must never migrate or create the user's database.

use std::path::Path;

use knx_productdb::{open_and_migrate, open_read_only, ProductDbError, CURRENT_PRODUCTDB_VERSION};

fn files_in(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_current_database_opens_read_only_and_stays_byte_identical() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    drop(open_and_migrate(&path).unwrap());
    let before = std::fs::read(&path).unwrap();

    let db = open_read_only(&path).unwrap();
    assert_eq!(db.migrated_from, None);
    assert!(knx_productdb::query::manufacturers(&db.conn)
        .unwrap()
        .is_empty());
    assert!(db
        .conn
        .execute("INSERT INTO manufacturer (id) VALUES ('M-0001')", [])
        .is_err());
    drop(db);

    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(files_in(dir.path()), vec!["products.sqlite".to_string()]);
}

#[test]
fn an_older_database_is_migrated_in_memory_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    // v20 -> v21 only re-derives rows from stored blobs; an empty database
    // therefore is a genuine v20 one once its version says so.
    conn.pragma_update(None, "user_version", CURRENT_PRODUCTDB_VERSION - 1)
        .unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();

    let db = open_read_only(&path).unwrap();
    assert_eq!(db.migrated_from, Some(CURRENT_PRODUCTDB_VERSION - 1));
    let version: i64 = db
        .conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .unwrap();
    assert_eq!(version, CURRENT_PRODUCTDB_VERSION);
    drop(db);

    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert_eq!(files_in(dir.path()), vec!["products.sqlite".to_string()]);
}

#[test]
fn a_newer_database_is_refused_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let conn = open_and_migrate(&path).unwrap();
    conn.pragma_update(None, "user_version", CURRENT_PRODUCTDB_VERSION + 1)
        .unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();

    let error = open_read_only(&path).err().unwrap();
    assert!(matches!(error, ProductDbError::FutureVersion { .. }));
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn a_missing_database_is_an_error_and_nothing_is_created() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nested").join("products.sqlite");

    assert!(open_read_only(&path).is_err());
    assert!(
        files_in(dir.path()).is_empty(),
        "not even the parent directory"
    );
}

#[test]
fn a_query_during_an_install_waits_for_the_lock_instead_of_failing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    drop(open_and_migrate(&path).unwrap());
    let db = open_read_only(&path).unwrap();

    let writer = rusqlite::Connection::open(&path).unwrap();
    writer.execute_batch("BEGIN EXCLUSIVE").unwrap();
    let release = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        writer.execute_batch("COMMIT").unwrap();
    });

    let found = knx_productdb::query::manufacturers(&db.conn).expect("waits for the writer");
    assert!(found.is_empty());
    release.join().unwrap();
}
