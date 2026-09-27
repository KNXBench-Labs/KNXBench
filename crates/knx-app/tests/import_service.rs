//! `knx-app`'s tests cannot reach `knx-etsproj`'s `tests/support` module —
//! a crate's integration tests are private to it — so this uses
//! `knx-testsupport`, the shared dev-dependency every crate's tests can
//! reach, instead of a public test-only API being added to `knx-etsproj`
//! for their benefit.

use std::path::{Path, PathBuf};

use knx_app::import_ets_project;
use knx_etsproj::opaque::sha256_hex;
use knx_store::{load_opaque, open_and_migrate, Connection};

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

fn migrated_store() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    (dir, conn)
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn importing_persists_the_opaque_entries() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let (_dir, conn) = migrated_store();
    let imported = import_ets_project(&reference_ets4_path(), &conn).unwrap();
    assert_eq!(imported.project.installations.len(), 1);
    assert!(imported.opaque_entries > 0);
    assert_eq!(load_opaque(&conn).unwrap().len(), imported.opaque_entries);
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_persisted_bytes_are_the_bytes_that_were_read() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
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
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn a_failed_import_leaves_the_store_untouched() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let (_dir, conn) = migrated_store();
    assert!(import_ets_project(Path::new("/nonexistent.knxproj"), &conn).is_err());
    assert_eq!(load_opaque(&conn).unwrap().len(), 0);
}
