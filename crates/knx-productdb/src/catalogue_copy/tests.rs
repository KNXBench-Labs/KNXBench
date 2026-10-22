//! Tests of the bounded copy primitive against schema-28 synthetic data.

use super::*;
use crate::operation::PublicationGate;

fn catalogue_with_row() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("catalogue.db");
    let conn = crate::open_and_migrate(&path).unwrap();
    conn.execute(
        "INSERT INTO manufacturer (id, name) VALUES ('m1', 'Example Co')",
        [],
    )
    .unwrap();
    drop(conn);
    (dir, path)
}

/// A pinned read-only source with an active read snapshot.
fn pinned_source(path: &std::path::Path) -> crate::migration::ReadOnlyProductDb {
    let ro = crate::open_read_only(path).unwrap();
    // Begin DEFERRED, then a real main read so the snapshot is pinned.
    ro.conn.execute_batch("BEGIN").unwrap();
    let n: i64 = ro
        .conn
        .query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))
        .unwrap();
    assert!(n > 0);
    assert!(!ro.conn.is_autocommit());
    ro
}

fn fresh_destination(dir: &tempfile::TempDir, page_size: i64) -> rusqlite::Connection {
    let path = dir.path().join("copy.db");
    let conn = rusqlite::Connection::open(&path).unwrap();
    conn.pragma_update(None, "page_size", page_size).unwrap();
    conn
}

#[test]
fn copy_observes_done_and_the_fresh_zero_cookie_rule() {
    let (_dir, path) = catalogue_with_row();
    let ro = pinned_source(&path);
    let source_page_size: i64 = ro
        .conn
        .query_row("PRAGMA page_size", [], |r| r.get(0))
        .unwrap();
    let mut dest = fresh_destination(&_dir, source_page_size);

    let done = copy_pinned_current(&ro.conn, &mut dest, &PublicationGate::new()).unwrap();
    assert_eq!(done.cookie_before, 0);
    assert_eq!(done.cookie_after, 1);
    assert_eq!(done.page_size, source_page_size as u64);
    assert!(done.page_count > 0);
    assert!(done.steps >= 1);

    // The destination now carries the copied rows.
    let copied: i64 = dest
        .query_row("SELECT count(*) FROM manufacturer", [], |r| r.get(0))
        .unwrap();
    assert_eq!(copied, 1);
}

#[test]
fn copy_refuses_a_write_capable_source() {
    let (_dir, path) = catalogue_with_row();
    let conn = crate::open_and_migrate(&path).unwrap();
    let mut dest = fresh_destination(&_dir, 4096);
    assert!(matches!(
        copy_pinned_current(&conn, &mut dest, &PublicationGate::new()),
        Err(CopyError::NotReadOnly)
    ));
}

#[test]
fn copy_refuses_a_source_without_an_active_snapshot() {
    let (_dir, path) = catalogue_with_row();
    let ro = crate::open_read_only(&path).unwrap();
    // No BEGIN/read: the connection is still autocommit.
    assert!(ro.conn.is_autocommit());
    let mut dest = fresh_destination(&_dir, 4096);
    assert!(matches!(
        copy_pinned_current(&ro.conn, &mut dest, &PublicationGate::new()),
        Err(CopyError::NoActiveSnapshot)
    ));
}

#[test]
fn copy_refuses_a_non_fresh_destination() {
    let (_dir, path) = catalogue_with_row();
    let ro = pinned_source(&path);
    let mut dest = fresh_destination(&_dir, 4096);
    dest.execute_batch("CREATE TABLE scratch(x)").unwrap();
    assert!(matches!(
        copy_pinned_current(&ro.conn, &mut dest, &PublicationGate::new()),
        Err(CopyError::DestinationNotFresh(_))
    ));
}

#[test]
fn copy_refuses_a_page_size_mismatch() {
    let (_dir, path) = catalogue_with_row();
    let ro = pinned_source(&path);
    let source_page_size: i64 = ro
        .conn
        .query_row("PRAGMA page_size", [], |r| r.get(0))
        .unwrap();
    // A different supported page size than the source.
    let other = if source_page_size == 4096 { 8192 } else { 4096 };
    let mut dest = fresh_destination(&_dir, other);
    assert!(matches!(
        copy_pinned_current(&ro.conn, &mut dest, &PublicationGate::new()),
        Err(CopyError::PageSizeMismatch { .. })
    ));
}

#[test]
fn precancelled_copy_returns_typed_stop() {
    let (_dir, path) = catalogue_with_row();
    let ro = pinned_source(&path);
    let mut dest = fresh_destination(&_dir, 4096);
    let gate = PublicationGate::new();
    assert_eq!(gate.try_cancel(), crate::operation::CancelOutcome::Accepted);
    assert!(matches!(
        copy_pinned_current(&ro.conn, &mut dest, &gate),
        Err(CopyError::Stop(crate::operation::Stop::CancelAccepted))
    ));
}

#[test]
fn supported_page_size_helper_accepts_only_powers_of_two() {
    for size in [512, 1024, 2048, 4096, 8192, 16384, 32768, 65536] {
        assert!(is_supported_page_size(size), "{size}");
    }
    for size in [0, 256, 768, 4097, 100_000, -4096] {
        assert!(!is_supported_page_size(size), "{size}");
    }
}
