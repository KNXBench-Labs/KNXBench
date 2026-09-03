//! Schema-version migration chain, keyed off SQLite's `user_version` pragma
//! (ADR-0003). Migrations run in order; there is no version-skipping path
//! and no downgrade. Entity tables arrive with Session 3's importer, once
//! there is data to store — this chain currently only proves the version
//! mechanics via a `schema_meta` marker table.

use std::fmt;
use std::path::Path;

use rusqlite::Connection;

/// Matches `knx_core::project::CURRENT_SCHEMA_VERSION`.
pub const CURRENT_SCHEMA_VERSION: i64 = 3;

#[derive(Debug)]
pub enum MigrationError {
    Sqlite(rusqlite::Error),
    FutureSchemaVersion { found: i64, supported: i64 },
}

impl std::error::Error for MigrationError {}

impl fmt::Display for MigrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MigrationError::Sqlite(e) => write!(f, "{e}"),
            MigrationError::FutureSchemaVersion { found, supported } => write!(
                f,
                "project file is schema version {found}, this build supports up to {supported} — no downgrade path exists"
            ),
        }
    }
}

impl From<rusqlite::Error> for MigrationError {
    fn from(e: rusqlite::Error) -> Self {
        MigrationError::Sqlite(e)
    }
}

/// v0 -> v1: creates the `schema_meta` marker table. Nothing else exists at
/// this schema version.
fn migrate_v0_to_v1(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
         INSERT INTO schema_meta (key, value) VALUES ('created_by', 'knx-store');",
    )?;
    Ok(())
}

/// v1 -> v2: creates the opaque passthrough table (Task 12/13) — everything
/// the domain model does not carry, kept as bytes plus a hash so export can
/// write it back unchanged.
fn migrate_v1_to_v2(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE opaque_entry (
             id          INTEGER PRIMARY KEY,
             source_path TEXT NOT NULL,
             xpath       TEXT NOT NULL,
             kind        TEXT NOT NULL,
             name        TEXT NOT NULL,
             bytes       BLOB NOT NULL,
             sha256      TEXT NOT NULL
         ) STRICT;
         CREATE INDEX opaque_entry_source_path ON opaque_entry (source_path);",
    )?;
    Ok(())
}

/// v2 -> v3: the manufacturer manifest. Manufacturer data itself now lives
/// in the shared product database (ADR-0005); this table is what lets a
/// project name the files it was imported with even when that database is
/// absent — a nameable gap instead of silent loss.
fn migrate_v2_to_v3(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE manufacturer_ref (
             id          INTEGER PRIMARY KEY,
             source_path TEXT NOT NULL,
             sha256      TEXT NOT NULL,
             len         INTEGER NOT NULL,
             kind        TEXT NOT NULL
         ) STRICT;
         CREATE INDEX manufacturer_ref_sha256 ON manufacturer_ref (sha256);",
    )?;
    Ok(())
}

type Migration = fn(&Connection) -> Result<(), MigrationError>;

/// Ordered chain; index `i` migrates `user_version` `i` to `i + 1`.
fn migrations() -> Vec<Migration> {
    vec![migrate_v0_to_v1, migrate_v1_to_v2, migrate_v2_to_v3]
}

/// Opens (creating if absent) the SQLite file at `path`, runs every pending
/// migration in order, and returns the connection at
/// `CURRENT_SCHEMA_VERSION`.
pub fn open_and_migrate(path: &Path) -> Result<Connection, MigrationError> {
    let conn = Connection::open(path)?;
    migrate(&conn)?;
    Ok(conn)
}

/// Opens an in-memory database and runs every migration in order — the same
/// chain as [`open_and_migrate`], but with nothing written to disk and
/// nothing left behind when the connection is dropped. For callers that
/// need a `Connection` to satisfy an API built around persistence (the
/// opaque store, the manifest table) without wanting a project file of
/// their own — e.g. the desktop app importing a `.knxproj` purely to
/// display it, with no save/reload feature yet (Session 5 cycle 1).
pub fn open_and_migrate_in_memory() -> Result<Connection, MigrationError> {
    let conn = Connection::open_in_memory()?;
    migrate(&conn)?;
    Ok(conn)
}

/// Runs every pending migration against an already-open connection and
/// brings its `user_version` to `CURRENT_SCHEMA_VERSION`. Shared by the
/// file-backed and in-memory entry points so the two can never drift.
fn migrate(conn: &Connection) -> Result<(), MigrationError> {
    let found: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;

    if found > CURRENT_SCHEMA_VERSION {
        return Err(MigrationError::FutureSchemaVersion {
            found,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }

    let pending = &migrations()[found as usize..CURRENT_SCHEMA_VERSION as usize];
    for migration in pending {
        migration(conn)?;
    }

    if found < CURRENT_SCHEMA_VERSION {
        conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_file_migrates_to_current_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.sqlite");
        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        let marker: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'created_by'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(marker, "knx-store");
    }

    #[test]
    fn reopening_an_already_migrated_file_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.sqlite");
        open_and_migrate(&path).unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn a_file_from_a_newer_schema_version_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("p.sqlite");
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION + 1)
                .unwrap();
        }
        assert!(matches!(
            open_and_migrate(&path),
            Err(MigrationError::FutureSchemaVersion { .. })
        ));
    }

    #[test]
    fn the_frozen_v1_fixture_still_opens() {
        // Copied, not opened in place: a migration test must not mutate its
        // fixture — `open_and_migrate` would otherwise rewrite the committed
        // v1 file to v2 on disk.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v1-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn a_fresh_file_migrates_to_the_current_version_and_has_the_opaque_table() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
        assert_eq!(crate::opaque::load_opaque(&conn).unwrap(), vec![]);
    }

    #[test]
    fn opaque_bytes_survive_a_round_trip_through_sqlite_unchanged() {
        use crate::opaque::{insert_opaque, load_opaque, StoredOpaqueEntry};

        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let entry = StoredOpaqueEntry {
            source_path: "M-0008/Baggages/econEts3.dll".into(),
            xpath: String::new(),
            kind: "Baggage".into(),
            name: String::new(),
            bytes: vec![0x4d, 0x5a, 0x00, 0xff, 0x00],
            sha256: "abc".into(),
        };
        insert_opaque(&conn, std::slice::from_ref(&entry)).unwrap();
        assert_eq!(load_opaque(&conn).unwrap(), vec![entry]);
    }

    #[test]
    fn the_frozen_v1_fixture_migrates_forward_to_the_current_version() {
        // Copied, not opened in place: a migration test must not mutate its fixture.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v1.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v1-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
        assert_eq!(crate::opaque::load_opaque(&conn).unwrap(), vec![]);
    }

    // `the_frozen_v2_fixture_still_opens` is deliberately not kept: v2 is no
    // longer `CURRENT_SCHEMA_VERSION`, so opening that fixture by its literal
    // path (rather than a copy) would migrate it forward and rewrite the
    // committed file on disk. `the_frozen_v2_fixture_migrates_forward_to_v3`
    // below covers the same fixture safely, and `the_frozen_v3_fixture_still_opens`
    // covers the "still opens as a no-op" guarantee for the version that is
    // current now.

    #[test]
    fn a_fresh_file_migrates_to_version_three_and_has_the_manifest_table() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 3);
        assert_eq!(
            crate::manifest::load_manufacturer_refs(&conn).unwrap(),
            vec![]
        );
    }

    #[test]
    fn the_frozen_v2_fixture_migrates_forward_to_v3() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v2.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v2-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 3);
        // The v2 opaque table survives the migration with its data intact.
        assert_eq!(crate::opaque::load_opaque(&conn).unwrap(), vec![]);
    }

    #[test]
    fn the_frozen_v3_fixture_still_opens() {
        let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v3-empty.sqlite");
        let conn = open_and_migrate(Path::new(fixture)).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 3);
    }

    #[test]
    fn in_memory_connection_migrates_to_current_version() {
        let conn = open_and_migrate_in_memory().unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        // The opaque table exists, same as a fresh file-backed connection.
        let _count: i64 = conn
            .query_row("SELECT COUNT(*) FROM opaque_entry", [], |row| row.get(0))
            .unwrap();
    }
}
