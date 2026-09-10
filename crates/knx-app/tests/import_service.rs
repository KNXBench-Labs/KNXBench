//! `knx-app`'s tests cannot reach `knx-etsproj`'s `tests/support` module —
//! a crate's integration tests are private to it — so this repeats the
//! four-line path helper rather than a public test-only API being added
//! to `knx-etsproj` for their benefit.

use std::path::{Path, PathBuf};

use knx_app::import_ets_project;
use knx_etsproj::opaque::sha256_hex;
use knx_store::{load_opaque, open_and_migrate, Connection};

fn reference_ets4_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj")
}

fn migrated_store() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    (dir, conn)
}

#[test]
fn importing_persists_the_opaque_entries() {
    let (_dir, conn) = migrated_store();
    let imported = import_ets_project(&reference_ets4_path(), &conn).unwrap();
    assert_eq!(imported.project.installations.len(), 1);
    assert!(imported.opaque_entries > 0);
    assert_eq!(load_opaque(&conn).unwrap().len(), imported.opaque_entries);
}

#[test]
fn the_persisted_bytes_are_the_bytes_that_were_read() {
    let (_dir, conn) = migrated_store();
    import_ets_project(&reference_ets4_path(), &conn).unwrap();
    let stored = load_opaque(&conn).unwrap();
    let dll = stored
        .iter()
        .find(|e| e.source_path.ends_with("econEts3.dll"))
        .expect("the baggage entry is persisted, not executed");
    assert_eq!(sha256_hex(&dll.bytes), dll.sha256);
}

#[test]
fn a_failed_import_leaves_the_store_untouched() {
    let (_dir, conn) = migrated_store();
    assert!(import_ets_project(Path::new("/nonexistent.knxproj"), &conn).is_err());
    assert_eq!(load_opaque(&conn).unwrap().len(), 0);
}
