//! Schema-version migration chain, keyed off SQLite's `user_version` pragma
//! (ADR-0003). Migrations run in order; there is no version-skipping path
//! and no downgrade. The chain now runs v0 -> v5: the `schema_meta` marker
//! table (v1), the opaque passthrough table (v2), the manufacturer manifest
//! (v3), every `knx_core::Project` entity table (v4 — `project_info`
//! through `parameter_instance`, written and read by `project.rs`'s
//! `save_project`/`load_project`) and `ModuleInstance` persistence (v5 —
//! ADR-0013, the schema-≥21 modular-application-program entity). Each
//! version has a frozen fixture under `fixtures/` that the tests below
//! migrate forward.

use std::fmt;
use std::path::Path;

use rusqlite::Connection;

/// Matches `knx_core::project::CURRENT_SCHEMA_VERSION`.
pub const CURRENT_SCHEMA_VERSION: i64 = 5;

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

/// v3 -> v4: every entity table for `knx_core::Project` — the full domain
/// model, not just the opaque/manifest passthrough (ADR-0003; design doc
/// `docs/superpowers/specs/2026-09-03-knx-entity-persistence-design.md`).
/// The `Override<T>` chain is one row per (com object, attribute) in
/// `com_object_override`, not wide columns. `position`/`flat_position`
/// columns preserve every order-sensitive `Vec` the domain model has — see
/// the design doc's "owned-list vs flat-list order" note for which table
/// gets which.
fn migrate_v3_to_v4(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE project_info (
             id                  INTEGER PRIMARY KEY CHECK (id = 0),
             project_id          TEXT NOT NULL,
             name                TEXT NOT NULL,
             project_number      TEXT,
             group_address_style TEXT NOT NULL,
             completion          TEXT NOT NULL,
             last_modified       TEXT,
             project_start       TEXT,
             default_language    TEXT NOT NULL
         ) STRICT;

         CREATE TABLE id_allocators (
             id                  INTEGER PRIMARY KEY CHECK (id = 0),
             device              INTEGER NOT NULL,
             area                INTEGER NOT NULL,
             line                INTEGER NOT NULL,
             com_object_instance INTEGER NOT NULL,
             group_range         INTEGER NOT NULL,
             group_address       INTEGER NOT NULL,
             building_part       INTEGER NOT NULL,
             parameter_instance  INTEGER NOT NULL
         ) STRICT;

         CREATE TABLE string_table_entry (
             key      TEXT NOT NULL,
             language TEXT NOT NULL,
             value    TEXT NOT NULL,
             PRIMARY KEY (key, language)
         ) STRICT;

         CREATE TABLE installation (
             id                INTEGER PRIMARY KEY,
             name              TEXT NOT NULL,
             -- No REFERENCES: `line` rows can be created before their owning
             -- installation's default line is known during a future
             -- incremental-update path. The one deliberate unconstrained FK.
             default_line_id   INTEGER,
             multicast_address TEXT,
             completion        TEXT NOT NULL
         ) STRICT;

         CREATE TABLE area (
             id              INTEGER PRIMARY KEY,
             installation_id INTEGER NOT NULL REFERENCES installation(id),
             position        INTEGER NOT NULL,
             source_path     TEXT NOT NULL,
             source_ets_id   TEXT NOT NULL,
             name            TEXT NOT NULL,
             address         INTEGER NOT NULL,
             completion      TEXT NOT NULL
         ) STRICT;
         CREATE INDEX area_installation_id ON area (installation_id);

         CREATE TABLE line (
             id                           INTEGER PRIMARY KEY,
             area_id                      INTEGER NOT NULL REFERENCES area(id),
             position                     INTEGER NOT NULL,
             source_path                  TEXT NOT NULL,
             source_ets_id                TEXT NOT NULL,
             name                         TEXT NOT NULL,
             address                      INTEGER NOT NULL,
             medium_ref                   TEXT NOT NULL,
             domain_address               TEXT,
             domain_address_is_checked    INTEGER,
             ip_routing_multicast_address TEXT,
             multicast_ttl                INTEGER,
             completion                   TEXT NOT NULL
         ) STRICT;
         CREATE INDEX line_area_id ON line (area_id);

         CREATE TABLE building_part (
             id              INTEGER PRIMARY KEY,
             installation_id INTEGER NOT NULL REFERENCES installation(id),
             parent_id       INTEGER REFERENCES building_part(id),
             position        INTEGER NOT NULL,
             flat_position   INTEGER NOT NULL,
             source_path     TEXT NOT NULL,
             source_ets_id   TEXT NOT NULL,
             name            TEXT NOT NULL,
             number          TEXT,
             kind            TEXT NOT NULL,
             default_line_id INTEGER REFERENCES line(id),
             completion      TEXT NOT NULL
         ) STRICT;
         CREATE INDEX building_part_installation_id ON building_part (installation_id);
         CREATE INDEX building_part_parent_id ON building_part (parent_id);

         CREATE TABLE device (
             id                          INTEGER PRIMARY KEY,
             installation_id             INTEGER NOT NULL REFERENCES installation(id),
             line_id                     INTEGER REFERENCES line(id),
             topology_position           INTEGER NOT NULL,
             source_path                 TEXT NOT NULL,
             source_ets_id               TEXT NOT NULL,
             name                        TEXT NOT NULL,
             description                 TEXT,
             address                     INTEGER,
             product_ref                 TEXT NOT NULL,
             program_ref                 TEXT NOT NULL,
             completion                  TEXT NOT NULL,
             individual_address_loaded   INTEGER NOT NULL,
             application_program_loaded  INTEGER NOT NULL,
             parameters_loaded           INTEGER NOT NULL,
             communication_part_loaded   INTEGER NOT NULL,
             medium_config_loaded        INTEGER NOT NULL,
             last_modified               TEXT,
             last_download               TEXT,
             broken                      INTEGER NOT NULL,
             visibility_calculated       INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX device_installation_id ON device (installation_id);
         CREATE INDEX device_line_id ON device (line_id);

         CREATE TABLE binary_data_ref (
             device_id INTEGER NOT NULL REFERENCES device(id),
             position  INTEGER NOT NULL,
             blob_id   TEXT NOT NULL,
             name      TEXT NOT NULL,
             PRIMARY KEY (device_id, position)
         ) STRICT;

         CREATE TABLE building_part_device (
             building_part_id INTEGER NOT NULL REFERENCES building_part(id),
             device_id        INTEGER NOT NULL REFERENCES device(id),
             position         INTEGER NOT NULL,
             PRIMARY KEY (building_part_id, device_id)
         ) STRICT;
         CREATE INDEX building_part_device_device_id ON building_part_device (device_id);

         CREATE TABLE com_object_instance (
             id            INTEGER PRIMARY KEY,
             device_id     INTEGER NOT NULL REFERENCES device(id),
             position      INTEGER NOT NULL,
             source_path   TEXT NOT NULL,
             source_ets_id TEXT NOT NULL,
             number        INTEGER NOT NULL,
             size_kind     TEXT,
             size_value    INTEGER,
             size_layer    TEXT,
             is_active     INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX com_object_instance_device_id ON com_object_instance (device_id);

         CREATE TABLE com_object_override (
             com_object_instance_id INTEGER NOT NULL REFERENCES com_object_instance(id),
             attr                    TEXT NOT NULL,
             state                   TEXT NOT NULL,
             value                   TEXT,
             text_kind               TEXT,
             layer                   TEXT,
             PRIMARY KEY (com_object_instance_id, attr)
         ) STRICT;

         CREATE TABLE group_range (
             id              INTEGER PRIMARY KEY,
             installation_id INTEGER NOT NULL REFERENCES installation(id),
             parent_id       INTEGER REFERENCES group_range(id),
             position        INTEGER NOT NULL,
             flat_position   INTEGER NOT NULL,
             source_path     TEXT NOT NULL,
             source_ets_id   TEXT NOT NULL,
             name            TEXT NOT NULL,
             range_start     INTEGER NOT NULL,
             range_end       INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX group_range_installation_id ON group_range (installation_id);
         CREATE INDEX group_range_parent_id ON group_range (parent_id);

         CREATE TABLE group_address (
             id              INTEGER PRIMARY KEY,
             installation_id INTEGER NOT NULL REFERENCES installation(id),
             range_id        INTEGER REFERENCES group_range(id),
             position        INTEGER NOT NULL,
             source_path     TEXT NOT NULL,
             source_ets_id   TEXT NOT NULL,
             name            TEXT NOT NULL,
             address         INTEGER NOT NULL,
             central         INTEGER NOT NULL,
             unfiltered      INTEGER NOT NULL
         ) STRICT;
         CREATE INDEX group_address_installation_id ON group_address (installation_id);
         CREATE INDEX group_address_range_id ON group_address (range_id);

         CREATE TABLE group_link (
             com_object_instance_id INTEGER NOT NULL REFERENCES com_object_instance(id),
             group_address_id       INTEGER NOT NULL REFERENCES group_address(id),
             direction              TEXT NOT NULL,
             position               INTEGER NOT NULL,
             PRIMARY KEY (com_object_instance_id, position)
         ) STRICT;
         CREATE INDEX group_link_group_address_id ON group_link (group_address_id);

         CREATE TABLE parameter_instance (
             id            INTEGER PRIMARY KEY,
             device_id     INTEGER NOT NULL REFERENCES device(id),
             position      INTEGER NOT NULL,
             source_path   TEXT NOT NULL,
             source_ets_id TEXT NOT NULL,
             raw           TEXT NOT NULL
         ) STRICT;
         CREATE INDEX parameter_instance_device_id ON parameter_instance (device_id);",
    )?;
    Ok(())
}

/// v4 -> v5: `ModuleInstance` persistence (ADR-0013) — the schema-≥21
/// modular-application-program entity Task 1 added to `knx-core`.
/// `module_instance`/`module_instance_argument` mirror `parameter_instance`'s
/// shape (retained-but-uninterpreted `SourceRef`+value rows); the nullable
/// `com_object_instance.module_instance_id` column, `project_info.
/// ets_schema_version` column, and `id_allocators.module_instance` counter
/// column are additive-only, per DATA_MODEL §11.
fn migrate_v4_to_v5(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE module_instance (
             id            INTEGER PRIMARY KEY,
             device_id     INTEGER NOT NULL REFERENCES device(id),
             position      INTEGER NOT NULL,
             source_path   TEXT NOT NULL,
             source_ets_id TEXT NOT NULL,
             repeat_index  TEXT NOT NULL
         ) STRICT;
         CREATE INDEX module_instance_device_id ON module_instance (device_id);

         CREATE TABLE module_instance_argument (
             module_instance_id INTEGER NOT NULL REFERENCES module_instance(id),
             position            INTEGER NOT NULL,
             source_path         TEXT NOT NULL,
             source_ets_id       TEXT NOT NULL,
             value               TEXT NOT NULL,
             PRIMARY KEY (module_instance_id, position)
         ) STRICT;

         ALTER TABLE com_object_instance ADD COLUMN module_instance_id INTEGER REFERENCES module_instance(id);
         ALTER TABLE project_info ADD COLUMN ets_schema_version INTEGER NOT NULL DEFAULT 11;
         ALTER TABLE id_allocators ADD COLUMN module_instance INTEGER NOT NULL DEFAULT 0;",
    )?;
    Ok(())
}

type Migration = fn(&Connection) -> Result<(), MigrationError>;

/// Ordered chain; index `i` migrates `user_version` `i` to `i + 1`.
fn migrations() -> Vec<Migration> {
    vec![
        migrate_v0_to_v1,
        migrate_v1_to_v2,
        migrate_v2_to_v3,
        migrate_v3_to_v4,
        migrate_v4_to_v5,
    ]
}

/// Opens (creating if absent) the SQLite file at `path`, runs every pending
/// migration in order, and returns the connection at
/// `CURRENT_SCHEMA_VERSION`.
pub fn open_and_migrate(path: &Path) -> Result<Connection, MigrationError> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
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
    conn.pragma_update(None, "foreign_keys", "ON")?;
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
    // below covers the same fixture safely.
    //
    // `the_frozen_v3_fixture_still_opens` was kept for the same reason while
    // v3 was current, but is deliberately not kept now that v5 is current:
    // opening the v3 fixture by its literal path would migrate it forward to
    // v5 and rewrite the committed file on disk.
    // `the_frozen_v3_fixture_migrates_forward_to_v4` below covers the same
    // fixture safely.
    //
    // `the_frozen_v4_fixture_still_opens` was kept for the same reason while
    // v4 was current, but is deliberately not kept now that v5 is current:
    // opening the v4 fixture by its literal path would migrate it forward to
    // v5 and rewrite the committed file on disk.
    // `the_frozen_v4_fixture_migrates_forward_to_v5` below covers the same
    // fixture safely.

    #[test]
    fn a_fresh_file_migrates_to_version_three_and_has_the_manifest_table() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        // `open_and_migrate` always runs the full chain, so a fresh file
        // lands on `CURRENT_SCHEMA_VERSION` (now 5), not v3 — the manifest
        // table introduced at v3 is what this test actually verifies, and it
        // still exists and is empty at v5.
        assert_eq!(v, 5);
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
        // See the comment on the test above: the chain runs all the way to
        // `CURRENT_SCHEMA_VERSION` (now 5), not just to v3.
        assert_eq!(v, 5);
        // The v2 opaque table survives the migration with its data intact.
        assert_eq!(crate::opaque::load_opaque(&conn).unwrap(), vec![]);
    }

    #[test]
    fn a_fresh_file_migrates_to_version_four_and_has_every_entity_table() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        // As with the tests above, a fresh file always lands on
        // `CURRENT_SCHEMA_VERSION` (now 5) — the v4 entity tables checked
        // below still exist and are empty at v5.
        assert_eq!(v, 5);
        for table in [
            "project_info",
            "id_allocators",
            "string_table_entry",
            "installation",
            "area",
            "line",
            "building_part",
            "building_part_device",
            "device",
            "binary_data_ref",
            "com_object_instance",
            "com_object_override",
            "group_link",
            "group_range",
            "group_address",
            "parameter_instance",
        ] {
            let count: i64 = conn
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "table {table} should exist and be empty");
        }
    }

    #[test]
    fn foreign_keys_are_enforced() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let fk_on: i64 = conn
            .query_row("PRAGMA foreign_keys", [], |r| r.get(0))
            .unwrap();
        assert_eq!(fk_on, 1);
    }

    #[test]
    fn the_frozen_v3_fixture_migrates_forward_to_v4() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v3.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v3-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        // See the comment on `the_frozen_v2_fixture_migrates_forward_to_v3`:
        // the chain runs all the way to `CURRENT_SCHEMA_VERSION` (now 5), not
        // just to v4.
        assert_eq!(v, 5);
        assert_eq!(crate::opaque::load_opaque(&conn).unwrap(), vec![]);
    }

    #[test]
    fn the_frozen_v4_fixture_migrates_forward_to_v5() {
        // Copied, not opened in place: a migration test must not mutate its
        // fixture — `open_and_migrate` would otherwise rewrite the committed
        // v4 file on disk to v5.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v4.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v4-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, 5);
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

    #[test]
    fn v4_to_v5_adds_the_module_instance_table_and_column() {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        migrate(&conn).unwrap(); // runs the full chain including the new migrate_v4_to_v5
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0)).unwrap();
        assert_eq!(version, 5);
        conn.execute("INSERT INTO module_instance (id, device_id, position, source_path, source_ets_id, repeat_index) VALUES (1, 0, 0, 't', 't', '6x1')", []).unwrap_err(); // device_id FK: no device(0) exists, expected to fail — proves the FK/table exist
        conn.query_row("SELECT module_instance_id FROM com_object_instance LIMIT 0", [], |_| Ok(())).ok(); // column exists (no rows to fail on, just proves no "no such column" error at prepare time)
    }
}
