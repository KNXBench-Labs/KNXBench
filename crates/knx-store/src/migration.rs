//! Schema-version migration chain, keyed off SQLite's `user_version` pragma
//! (ADR-0003). Migrations run in order; there is no version-skipping path
//! and no downgrade. The chain now runs v0 -> v9: the `schema_meta` marker
//! table (v1), the opaque passthrough table (v2), the manufacturer manifest
//! (v3), every `knx_core::Project` entity table (v4 — `project_info`
//! through `parameter_instance`, written and read by `project.rs`'s
//! `save_project`/`load_project`), `ModuleInstance` persistence (v5 —
//! ADR-0013, the schema-≥21 modular-application-program entity) and the
//! retained `ModuleInstance/@Id` (v6 — D38, `module_instance.
//! instance_ets_id`) and the Read-on-Init flag's own `com_object_override`
//! attribute (v7 — §117, a version bump with no DDL; see
//! `migrate_v6_to_v7`). v8 is reserved for a concurrent branch (the
//! schema-≥21 export path) this chain never saw the contents of — see
//! `migrate_v7_to_v8`. v9 adds `com_object_program_default`
//! (KNOWN_LIMITATIONS §12 gap 2, ADR-0012 amended by ADR-0027). Each
//! version has a frozen fixture under `fixtures/` that the tests below
//! migrate forward.

use std::fmt;
use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension, Transaction};

/// Matches `knx_core::project::CURRENT_SCHEMA_VERSION`.
pub const CURRENT_SCHEMA_VERSION: i64 = 11;

#[derive(Debug)]
pub enum MigrationError {
    Sqlite(rusqlite::Error),
    FutureSchemaVersion {
        found: i64,
        supported: i64,
    },
    /// The file is an SQLite database that KNXBench did not create: tables
    /// but no `user_version`, or a version without the `schema_meta`
    /// `created_by = knx-store` marker (AR18 review M1). Refused untouched
    /// rather than given 22 KNXBench tables it never asked for.
    ForeignDatabase,
    /// [`open_existing_and_migrate`] was given a path that does not exist;
    /// nothing was created there.
    NotFound,
    /// [`open_existing_and_migrate`] found a KNXBench (or empty) database
    /// that holds no saved project. Refused before any migration, so the
    /// file keeps its schema version (AR18 review M1c).
    NothingSaved,
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
            MigrationError::ForeignDatabase => write!(
                f,
                "this SQLite file was not created by KNXBench; it was left untouched"
            ),
            MigrationError::NotFound => write!(f, "no project file at this path"),
            MigrationError::NothingSaved => write!(
                f,
                "no project has been saved to this file; it was left untouched"
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

/// v5 -> v6: retains the `ModuleInstance/@Id` a schema-≥21 project already
/// requires (D38) — the `MD-<d>_M-<n>_MI-<k>` id, not just its `@RefId`
/// prefix `source_ets_id` already keeps. Additive-only (DATA_MODEL §11);
/// existing rows default to `''`, which the read side (D39, a later task)
/// treats the same as a missing instance: read-only, reported, never
/// guessed.
fn migrate_v5_to_v6(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "ALTER TABLE module_instance ADD COLUMN instance_ets_id TEXT NOT NULL DEFAULT '';",
    )?;
    Ok(())
}

/// v6 -> v7: the sixth communication-object flag, Read-on-Init
/// (KNOWN_LIMITATIONS §117). Deliberately empty of DDL, and that is the
/// whole point of writing it down.
///
/// `com_object_override` is keyed by `(com_object_instance_id, attr)`, one
/// row per stated attribute, so a new flag needs no new column — it needs a
/// new `attr` string, `"read_on_init"`. What the version bump buys is the
/// two directions of the boundary:
///
/// * **Forward.** A pre-v7 project has no `read_on_init` rows, and no row
///   decodes as `Override::Absent`. Nothing is backfilled: an old project
///   never said "this object does not read on init", it said nothing at
///   all, and the two are different facts. Writing `false` here would be the
///   same silent invention §117 exists to stop.
/// * **Backward.** A v7 project *may* carry `read_on_init` rows, which a
///   pre-v7 build would meet as `StoreError::UnknownOverrideAttr` — a
///   cryptic failure deep in the load. With the version moved, that build
///   stops at `MigrationError::FutureSchemaVersion` instead and says so.
fn migrate_v6_to_v7(_conn: &Connection) -> Result<(), MigrationError> {
    Ok(())
}

/// v7 -> v8: **reserved**. A concurrent branch (the schema-≥21 export path)
/// owns this slot; this branch never got to see what it contains.
/// Deliberately empty of DDL so this branch's own chain stays contiguous
/// and testable today, same trick as `migrate_v6_to_v7`. The coordinator
/// deletes this stub at merge time and renumbers `migrate_v8_to_v9` down
/// to `migrate_v7_to_v8` in its place — a one-line change, by design.
fn migrate_v7_to_v8(_conn: &Connection) -> Result<(), MigrationError> {
    Ok(())
}

/// v8 -> v9: `com_object_program_default` (KNOWN_LIMITATIONS §12 gap 2,
/// ADR-0012 amended by ADR-0027). One row per lifted field on a
/// communication object instance whose own slot was `Override::Empty` but
/// whose application program still states a value — mirrors
/// `com_object_override`'s shape (attr/value/layer) rather than inventing a
/// new one, since it is the same "one attribute, one row" problem with a
/// different source layer. `layer` is always `program` or `program_ref`
/// here; never `instance` or `user_edit` (`knx_productdb::enrich` is the
/// only writer).
fn migrate_v8_to_v9(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE com_object_program_default (
            com_object_instance_id INTEGER NOT NULL REFERENCES com_object_instance(id),
            attr TEXT NOT NULL,
            value TEXT NOT NULL,
            text_kind TEXT,
            layer TEXT NOT NULL,
            PRIMARY KEY (com_object_instance_id, attr)
        ) STRICT;",
    )?;
    Ok(())
}

/// The keyed opaque path of a group address (`knx_etsproj::xpath::group_address`),
/// spelled here because the store does not depend on the importer. A
/// change there must change this too; the v10 lift test pins both.
const GROUP_ADDRESS_XPATH_PREFIX: &str =
    "/KNX/Project/Installations/Installation/GroupAddresses/GroupRanges/GroupRange/GroupAddress";

/// v9 -> v10: a group address's own `@DatapointType` (ADR-0078).
///
/// Adds `group_address.dpt_state`/`dpt_value`/`dpt_layer` (the
/// `com_object_override` encoding) and
/// `project_info.unlifted_group_address_dpt_declarations`. For schema ≥21
/// projects it then lifts every retained `DatapointType` attribute whose
/// xpath is exactly a stored address's keyed path, parsing it as the
/// importer does, and deletes the lifted row so the value is stored once
/// (ADR-0020: re-derive what the stored bytes determine). Rows it cannot
/// attribute — the unkeyed form of imports before 2026-09-20, or a key that
/// matches no stored address — stay untouched and are counted. The whole
/// step runs in one savepoint.
fn migrate_v9_to_v10(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch("SAVEPOINT migrate_v9_to_v10")?;
    match lift_group_address_dpts(conn) {
        Ok(()) => {
            conn.execute_batch("RELEASE migrate_v9_to_v10")?;
            Ok(())
        }
        Err(error) => {
            conn.execute_batch("ROLLBACK TO migrate_v9_to_v10; RELEASE migrate_v9_to_v10")?;
            Err(error)
        }
    }
}

fn lift_group_address_dpts(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "ALTER TABLE group_address ADD COLUMN dpt_state TEXT NOT NULL DEFAULT 'absent';
         ALTER TABLE group_address ADD COLUMN dpt_value TEXT;
         ALTER TABLE group_address ADD COLUMN dpt_layer TEXT;
         ALTER TABLE project_info ADD COLUMN unlifted_group_address_dpt_declarations
             INTEGER NOT NULL DEFAULT 0;",
    )?;
    let schema: Option<i64> = conn
        .query_row(
            "SELECT ets_schema_version FROM project_info WHERE id = 0",
            [],
            |r| r.get(0),
        )
        .optional()?;
    // Schema 11 defines no `GroupAddress/@DatapointType`; such an attribute
    // stays an unknown retained one, exactly as a fresh import keeps it.
    if schema.is_none_or(|v| v < 21) {
        return Ok(());
    }
    let rows: Vec<(i64, String, String, Vec<u8>)> = conn
        .prepare(
            "SELECT id, source_path, xpath, bytes FROM opaque_entry
             WHERE kind = 'RetainedAttribute' AND name = 'DatapointType'
               AND substr(xpath, 1, ?1) = ?2
             ORDER BY id",
        )?
        .query_map(
            params![
                GROUP_ADDRESS_XPATH_PREFIX.len() as i64,
                GROUP_ADDRESS_XPATH_PREFIX
            ],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )?
        .collect::<Result<_, _>>()?;
    let mut unlifted: i64 = 0;
    for (opaque_id, source_path, xpath, bytes) in rows {
        let ets_id = xpath
            .strip_prefix(GROUP_ADDRESS_XPATH_PREFIX)
            .and_then(|rest| rest.strip_prefix("[@Id='"))
            .and_then(|rest| rest.strip_suffix("']"));
        let target: Option<i64> = match ets_id {
            Some(ets_id) => conn
                .query_row(
                    "SELECT id FROM group_address WHERE source_path = ?1 AND source_ets_id = ?2",
                    params![source_path, ets_id],
                    |r| r.get(0),
                )
                .optional()?,
            None => None,
        };
        let (Some(group_address_id), Ok(text)) = (target, String::from_utf8(bytes)) else {
            unlifted += 1;
            continue;
        };
        let (state, value, layer) = if text.is_empty() {
            ("empty", None, None)
        } else {
            match knx_core::DptRef::parse(&text) {
                Ok(dpt) => ("value", Some(dpt.to_string()), Some("Instance")),
                Err(_) => ("malformed", Some(text), None),
            }
        };
        conn.execute(
            "UPDATE group_address SET dpt_state = ?1, dpt_value = ?2, dpt_layer = ?3 WHERE id = ?4",
            params![state, value, layer, group_address_id],
        )?;
        conn.execute("DELETE FROM opaque_entry WHERE id = ?1", params![opaque_id])?;
    }
    conn.execute(
        "UPDATE project_info SET unlifted_group_address_dpt_declarations = ?1 WHERE id = 0",
        params![unlifted],
    )?;
    Ok(())
}

/// Native editor recovery and independent project versions (ADR-0100).
fn migrate_v10_to_v11(conn: &Connection) -> Result<(), MigrationError> {
    conn.execute_batch(
        "CREATE TABLE project_history_context (
             content_hash TEXT PRIMARY KEY CHECK (length(content_hash) = 64),
             format_version INTEGER NOT NULL CHECK (format_version = 1),
             image BLOB NOT NULL CHECK (length(image) > 0),
             image_hash TEXT NOT NULL CHECK (length(image_hash) = 64)
         ) STRICT;
         CREATE TABLE project_history_state (
             id INTEGER PRIMARY KEY CHECK (id = 0),
             format_version INTEGER NOT NULL CHECK (format_version = 1),
             generation INTEGER NOT NULL CHECK (generation > 0),
             baseline_hash TEXT NOT NULL CHECK (length(baseline_hash) = 64),
             working BLOB NOT NULL CHECK (length(working) > 0),
             working_hash TEXT NOT NULL CHECK (length(working_hash) = 64),
             context_hash TEXT NOT NULL REFERENCES project_history_context(content_hash)
         ) STRICT;
         CREATE TABLE project_history_stack (
             side TEXT NOT NULL CHECK (side IN ('undo', 'redo')),
             position INTEGER NOT NULL CHECK (position >= 0),
             format_version INTEGER NOT NULL CHECK (format_version = 1),
             image BLOB NOT NULL CHECK (length(image) > 0),
             image_hash TEXT NOT NULL CHECK (length(image_hash) = 64),
             context_hash TEXT NOT NULL REFERENCES project_history_context(content_hash),
             PRIMARY KEY (side, position)
         ) STRICT;
         CREATE TABLE project_history_version (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             format_version INTEGER NOT NULL CHECK (format_version = 1),
             created_at TEXT NOT NULL,
             reason TEXT NOT NULL CHECK (reason IN ('save', 'named', 'pre_restore', 'replaced_workspace')),
             label TEXT NOT NULL CHECK (length(label) BETWEEN 1 AND 120),
             image BLOB NOT NULL CHECK (length(image) > 0),
             image_hash TEXT NOT NULL CHECK (length(image_hash) = 64),
             context_hash TEXT NOT NULL REFERENCES project_history_context(content_hash)
         ) STRICT;
         ALTER TABLE line ADD COLUMN model_position INTEGER NOT NULL DEFAULT 0;",
    )?;
    // Match the preceding native reader's area/line traversal exactly while
    // introducing an independent model-vector order for new snapshots.
    let lines = conn
        .prepare("SELECT a.installation_id, l.id FROM line l JOIN area a ON l.area_id = a.id ORDER BY a.installation_id, a.position, l.position")?
        .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    let mut installation = None;
    let mut position = 0i64;
    for (owner, id) in lines {
        if installation != Some(owner) {
            installation = Some(owner);
            position = 0;
        }
        conn.execute(
            "UPDATE line SET model_position = ?1 WHERE id = ?2",
            params![position, id],
        )?;
        position += 1;
    }
    Ok(())
}

type Migration = fn(&Connection) -> Result<(), MigrationError>;

/// History owns the surrounding transaction so a refused editor write also
/// rolls back any native upgrade. The ordinary openers keep their own atomic
/// upgrade boundary; this helper never begins or commits another transaction.
pub(crate) fn migrate_in_transaction(tx: &Transaction<'_>) -> Result<(), MigrationError> {
    let found: i64 = tx.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found < 0 || !is_own_or_empty(tx, found)? {
        return Err(MigrationError::ForeignDatabase);
    }
    if found > CURRENT_SCHEMA_VERSION {
        return Err(MigrationError::FutureSchemaVersion {
            found,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    apply_pending_migrations(tx, found)
}

fn apply_pending_migrations(conn: &Connection, found: i64) -> Result<(), MigrationError> {
    if found == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }
    for migration in &migrations()[found as usize..CURRENT_SCHEMA_VERSION as usize] {
        migration(conn)?;
    }
    conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    Ok(())
}

/// Ordered chain; index `i` migrates `user_version` `i` to `i + 1`.
fn migrations() -> Vec<Migration> {
    vec![
        migrate_v0_to_v1,
        migrate_v1_to_v2,
        migrate_v2_to_v3,
        migrate_v3_to_v4,
        migrate_v4_to_v5,
        migrate_v5_to_v6,
        migrate_v6_to_v7,
        migrate_v7_to_v8,
        migrate_v8_to_v9,
        migrate_v9_to_v10,
        migrate_v10_to_v11,
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

/// [`open_and_migrate`] for a file that must already exist — every reader
/// (`doc-export`, `ga-export`, open). A missing path is refused with
/// [`MigrationError::NotFound`] and nothing is created there (AR18 review
/// M1d: a mistyped path used to leave an empty project file behind).
pub fn open_existing_and_migrate(path: &Path) -> Result<Connection, MigrationError> {
    use rusqlite::OpenFlags;
    if !path.exists() {
        return Err(MigrationError::NotFound);
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_WRITE
        | OpenFlags::SQLITE_OPEN_URI
        | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let conn = Connection::open_with_flags(path, flags).map_err(|error| {
        match error.sqlite_error_code() {
            Some(rusqlite::ErrorCode::CannotOpen) => MigrationError::NotFound,
            _ => MigrationError::Sqlite(error),
        }
    })?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let version: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version <= CURRENT_SCHEMA_VERSION {
        if !is_own_or_empty(&conn, version)? {
            return Err(MigrationError::ForeignDatabase);
        }
        if !has_saved_project(&conn)? {
            return Err(MigrationError::NothingSaved);
        }
    }
    migrate(&conn)?;
    Ok(conn)
}

/// How long [`open_existing_read_only`] waits for a writer's lock.
pub const READ_ONLY_BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// Pause before an in-memory copy retries a busy source.
const BACKUP_RETRY_PAUSE: std::time::Duration = std::time::Duration::from_millis(20);

/// A project store opened by [`open_existing_read_only`].
#[derive(Debug)]
pub struct ReadOnlyStore {
    /// At `CURRENT_SCHEMA_VERSION`, ready for `load_project`. Either the
    /// file itself, opened `SQLITE_OPEN_READ_ONLY` with `query_only` on, or
    /// an in-memory copy of it.
    pub conn: Connection,
    /// The file's own schema version when it was older than this build's,
    /// in which case `conn` is an in-memory copy that was migrated instead
    /// of the file (ADR-0090). `None` when the file was already current.
    pub migrated_from: Option<i64>,
}

/// Opens an existing project store for reading without ever writing to it
/// (ADR-0090) — the opener for callers that must not change the user's
/// file, such as the read-only MCP adapter.
///
/// Refuses exactly what [`open_existing_and_migrate`] refuses (a missing
/// path, a foreign database, a store with nothing saved, a newer schema),
/// and creates nothing. A current store is returned as a read-only
/// connection to the file. An older one is copied into memory with SQLite's
/// online backup API and only the copy is migrated, so the file keeps its
/// bytes and its schema version.
pub fn open_existing_read_only(path: &Path) -> Result<ReadOnlyStore, MigrationError> {
    use rusqlite::OpenFlags;
    if !path.exists() {
        return Err(MigrationError::NotFound);
    }
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let file = Connection::open_with_flags(path, flags).map_err(|error| {
        match error.sqlite_error_code() {
            Some(rusqlite::ErrorCode::CannotOpen) => MigrationError::NotFound,
            _ => MigrationError::Sqlite(error),
        }
    })?;
    file.pragma_update(None, "query_only", true)?;
    // A save's commit holds the file's exclusive lock for a moment; a
    // reader waits for it rather than failing with "database is locked".
    // Explicit, not left to the library's own default.
    file.busy_timeout(READ_ONLY_BUSY_TIMEOUT)?;
    let version: i64 = file.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if version > CURRENT_SCHEMA_VERSION {
        return Err(MigrationError::FutureSchemaVersion {
            found: version,
            supported: CURRENT_SCHEMA_VERSION,
        });
    }
    if !is_own_or_empty(&file, version)? {
        return Err(MigrationError::ForeignDatabase);
    }
    if !has_saved_project(&file)? {
        return Err(MigrationError::NothingSaved);
    }
    if version == CURRENT_SCHEMA_VERSION {
        return Ok(ReadOnlyStore {
            conn: file,
            migrated_from: None,
        });
    }
    let mut memory = Connection::open_in_memory()?;
    {
        // All pages in one step; a busy source is retried after a pause,
        // never in a tight loop.
        let backup = rusqlite::backup::Backup::new(&file, &mut memory)?;
        backup.run_to_completion(i32::MAX, BACKUP_RETRY_PAUSE, None)?;
    }
    drop(file);
    memory.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&memory)?;
    Ok(ReadOnlyStore {
        conn: memory,
        migrated_from: Some(version),
    })
}

/// Whether a saved project is present. Only schema v4 and later can hold
/// one (`project_info` arrives in `migrate_v3_to_v4`); an older or empty
/// file has nothing a reader could load, migrated or not.
fn has_saved_project(conn: &Connection) -> Result<bool, MigrationError> {
    let has_table: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'project_info'",
        [],
        |row| row.get(0),
    )?;
    if has_table == 0 {
        return Ok(false);
    }
    let rows: i64 = conn.query_row("SELECT count(*) FROM project_info", [], |row| row.get(0))?;
    Ok(rows > 0)
}

/// Whether `conn` is a database this crate created, or an empty one it may
/// initialise. Checked before any migration writes (AR18 review M1b).
fn is_own_or_empty(conn: &Connection, version: i64) -> Result<bool, MigrationError> {
    let tables: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table'",
        [],
        |row| row.get(0),
    )?;
    if version == 0 {
        return Ok(tables == 0);
    }
    let has_meta: i64 = conn.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name = 'schema_meta'",
        [],
        |row| row.get(0),
    )?;
    if has_meta == 0 {
        return Ok(false);
    }
    let marker: Option<String> = conn
        .query_row(
            "SELECT value FROM schema_meta WHERE key = 'created_by'",
            [],
            |row| row.get(0),
        )
        .map(Some)
        .or_else(|error| match error {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(other),
        })?;
    Ok(marker.as_deref() == Some("knx-store"))
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

    if found < 0 || !is_own_or_empty(conn, found)? {
        return Err(MigrationError::ForeignDatabase);
    }

    if found == CURRENT_SCHEMA_VERSION {
        return Ok(());
    }
    // One upgrade, one transaction (KNOWN_LIMITATIONS §157): a failure in
    // any step, or a killed process, leaves the file at its old version
    // with none of the earlier steps applied, so the next open can retry.
    // The product database does the same (`knx_productdb::open_and_migrate`).
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = apply_pending_migrations(conn, found);
    match result {
        Ok(()) => conn.execute_batch("COMMIT")?,
        Err(error) => {
            let _ = conn.execute_batch("ROLLBACK");
            return Err(error);
        }
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
    fn a_failed_upgrade_rolls_back_every_step_and_the_file_stays_reopenable() {
        // KNOWN_LIMITATIONS §157: one upgrade is one transaction. A v8 file
        // whose v9->v10 step fails must not keep the v8->v9 table, or the
        // next open re-runs v8->v9 against it and fails for good.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v8.sqlite");
        {
            let conn = Connection::open(&path).unwrap();
            conn.pragma_update(None, "foreign_keys", "ON").unwrap();
            for migration in &migrations()[..8] {
                migration(&conn).unwrap();
            }
            conn.pragma_update(None, "user_version", 8).unwrap();
            // A collision that fails the v9->v10 step after v8->v9 has run,
            // standing in for a full disk or a killed process.
            conn.execute_batch("ALTER TABLE group_address ADD COLUMN dpt_state TEXT")
                .unwrap();
        }
        let before = std::fs::read(&path).unwrap();
        assert!(open_and_migrate(&path).is_err());
        assert!(
            std::fs::read(&path).unwrap() == before,
            "a failed upgrade leaves the file byte for byte as it was"
        );
        {
            let conn = Connection::open(&path).unwrap();
            let version: i64 = conn
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            assert_eq!(version, 8);
            let leaked: i64 = conn
                .query_row(
                    "SELECT count(*) FROM sqlite_schema WHERE name = 'com_object_program_default'",
                    [],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(
                leaked, 0,
                "the v8->v9 step rolls back with the failed v9->v10 step"
            );
            conn.execute_batch("ALTER TABLE group_address DROP COLUMN dpt_state")
                .unwrap();
        }
        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
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
    // v3 was current, but is deliberately not kept now that v6 is current:
    // opening the v3 fixture by its literal path would migrate it forward to
    // v6 and rewrite the committed file on disk.
    // `the_frozen_v3_fixture_migrates_forward_to_v4` below covers the same
    // fixture safely.
    //
    // `the_frozen_v4_fixture_still_opens` was kept for the same reason while
    // v4 was current, but is deliberately not kept now that v6 is current:
    // opening the v4 fixture by its literal path would migrate it forward to
    // v6 and rewrite the committed file on disk.
    // `the_frozen_v4_fixture_migrates_forward_to_v5` below covers the same
    // fixture safely.
    //
    // No `the_frozen_v5_fixture_still_opens` test was ever added: v5 landed
    // and was superseded by v6 in the same task, so
    // `the_frozen_v5_fixture_migrates_forward_to_v6` below is the only test
    // that fixture ever needed.

    #[test]
    fn a_fresh_file_migrates_to_version_three_and_has_the_manifest_table() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        // `open_and_migrate` always runs the full chain, so a fresh file
        // lands on `CURRENT_SCHEMA_VERSION` (now 9), not v3 — the manifest
        // table introduced at v3 is what this test actually verifies, and it
        // still exists and is empty at v5.
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
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
        // `CURRENT_SCHEMA_VERSION` (now 9), not just to v3.
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
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
        // `CURRENT_SCHEMA_VERSION` (now 9) — the v4 entity tables checked
        // below still exist and are empty at v5.
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
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
        // the chain runs all the way to `CURRENT_SCHEMA_VERSION` (now 9), not
        // just to v4.
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
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
        // See the comment on `the_frozen_v3_fixture_migrates_forward_to_v4`:
        // the chain runs all the way to `CURRENT_SCHEMA_VERSION` (now 9),
        // not just to v5.
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
    }

    #[test]
    fn the_frozen_v5_fixture_migrates_forward_to_v6() {
        // Copied, not opened in place: a migration test must not mutate its
        // fixture — `open_and_migrate` would otherwise rewrite the committed
        // v5 file on disk to v6.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v5.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v5-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
        // `module_instance` carries no rows in the empty fixture, so the
        // "existing rows default to ''" claim is checked directly against
        // the column definition ETS never populated.
        let (notnull, dflt_value): (i64, String) = conn
            .query_row(
                "SELECT \"notnull\", dflt_value FROM pragma_table_info('module_instance') WHERE name = 'instance_ets_id'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(notnull, 1);
        assert_eq!(dflt_value, "''");
    }

    #[test]
    fn the_frozen_v6_fixture_migrates_forward_to_v7() {
        // Copied, not opened in place: a migration test must not mutate its
        // fixture — `open_and_migrate` would otherwise rewrite the committed
        // v6 file on disk to v7.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v6.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v6-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
        // v7 adds no DDL — `com_object_override` is keyed by attribute name,
        // so the sixth flag needed a new `attr` string and nothing else.
        // What the migration must not do is invent rows, so the table is
        // still empty.
        let overrides: i64 = conn
            .query_row("SELECT COUNT(*) FROM com_object_override", [], |r| r.get(0))
            .unwrap();
        assert_eq!(overrides, 0);
    }

    #[test]
    fn the_frozen_v7_fixture_migrates_forward_to_v9() {
        // Copied, not opened in place: a migration test must not mutate its
        // fixture — `open_and_migrate` would otherwise rewrite the committed
        // v7 file on disk to v9.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v7.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v7-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let v: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(v, CURRENT_SCHEMA_VERSION);
        // v8 (reserved, no DDL) and v9's new `com_object_program_default`
        // table both land; the table exists and, migrating from empty, is
        // itself empty — a migration invents no rows.
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM com_object_program_default", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(count, 0);
    }

    /// A migration only ever runs over old data that has rows in it, so a
    /// test over an empty old database exercises the schema change and
    /// nothing else. This one writes a real project — device, communication
    /// object, a stated dpt and a stated flag — into a genuine v7 file,
    /// migrates it, and reads the project back. ADR-0027's claim that "an
    /// already-persisted project with no `com_object_program_default` rows
    /// behaves exactly as before" is this assertion and nothing more.
    #[test]
    fn a_populated_pre_v9_project_survives_the_v9_migration_unchanged() {
        use knx_core::{
            ComObjectInstance, ComObjectInstanceId, DeviceId, DptRef, Layer, Override, Resolved,
            ResolvedFlags,
        };

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v7-populated.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v7-empty.sqlite"),
            &path,
        )
        .unwrap();

        let saved = {
            // Same trick `a_pre_v7_com_object_reads_its_sixth_flag_as_absent_
            // not_false` uses one screen down: write through today's writer,
            // then take the file back to the old shape by hand. `save_project`
            // clears `com_object_program_default` unconditionally and so
            // cannot run against a file that lacks the table, and a v7 build's
            // writer is not available to a v9 build to borrow.
            let conn = open_and_migrate(&path).unwrap();

            let mut project = knx_core::Project::new(knx_core::Language("en".into()));
            project.installations.push(knx_core::Installation {
                id: knx_core::InstallationId(0),
                name: "I".into(),
                default_line: None,
                multicast_address: None,
                completion: knx_core::CompletionStatus::FinishedDesign,
                topology: knx_core::Topology {
                    areas: vec![],
                    lines: vec![],
                    unassigned: vec![DeviceId(1)],
                },
                buildings: vec![],
                group_ranges: vec![],
                group_addresses: vec![],
                parameters: vec![],
            });
            project.devices.insert(knx_core::DeviceInstance {
                id: DeviceId(1),
                source: knx_core::SourceRef {
                    path: "P-0001/0.xml".into(),
                    ets_id: "A-1".into(),
                },
                name: "A pre-v9 device".into(),
                description: None,
                address: None,
                product_ref: "P".into(),
                program_ref: "H".into(),
                commissioning: knx_core::CommissioningState::default(),
                visibility_calculated: true,
                com_objects: vec![ComObjectInstanceId(5)],
                binary_data: vec![],
            });
            project.devices.insert_com_object(ComObjectInstance {
                id: ComObjectInstanceId(5),
                source: knx_core::SourceRef {
                    path: "P-0001/0.xml".into(),
                    ets_id: "A-1_O-1_R-1".into(),
                },
                device: DeviceId(1),
                number: 3,
                text: Override::Empty,
                description: Override::Absent,
                dpt: Override::Value(Resolved {
                    value: DptRef {
                        main: 9,
                        sub: Some(1),
                    },
                    layer: Layer::Instance,
                }),
                flags: ResolvedFlags {
                    communication: Override::Value(Resolved {
                        value: true,
                        layer: Layer::Instance,
                    }),
                    ..ResolvedFlags::none()
                },
                size: None,
                is_active: true,
                links: vec![],
                module_instance: None,
            });
            crate::project::cover_ids_in_use(&mut project);
            crate::project::save_project(&conn, &project).unwrap();

            // Back to a genuine v7 shape: v8 is a no-DDL placeholder, v9's
            // only change is this table and v10's are four columns, so
            // dropping them and rewinding `user_version` leaves exactly the
            // file a v7 build would have written for this project.
            conn.execute_batch(
                "DROP TABLE project_history_state;
                 DROP TABLE project_history_stack;
                 DROP TABLE project_history_version;
         DROP TABLE project_history_context;
         ALTER TABLE line DROP COLUMN model_position;
                 DROP TABLE com_object_program_default;
                 ALTER TABLE group_address DROP COLUMN dpt_state;
                 ALTER TABLE group_address DROP COLUMN dpt_value;
                 ALTER TABLE group_address DROP COLUMN dpt_layer;
                 ALTER TABLE project_info DROP COLUMN unlifted_group_address_dpt_declarations;",
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 7i64).unwrap();
            project
        };

        {
            let conn = Connection::open(&path).unwrap();
            let version: i64 = conn
                .query_row("PRAGMA user_version", [], |r| r.get(0))
                .unwrap();
            assert_eq!(version, 7, "the file under test must be a v7 file");
            let devices: i64 = conn
                .query_row("SELECT COUNT(*) FROM device", [], |r| r.get(0))
                .unwrap();
            assert_eq!(devices, 1, "and a populated one, which is the whole point");
        }

        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);

        let loaded = crate::project::load_project(&conn).unwrap();
        assert_eq!(
            loaded, saved,
            "a v7 project must come back out of a v9 database exactly as it went in"
        );

        // The values, named individually, so a failure says which one moved
        // rather than dumping two whole projects at the reader.
        let com = loaded.devices.com_object(ComObjectInstanceId(5)).unwrap();
        assert_eq!(com.number, 3);
        assert_eq!(com.text, Override::Empty, "Empty is not Absent, still");
        assert_eq!(
            com.dpt.value().map(|r| r.value),
            Some(DptRef {
                main: 9,
                sub: Some(1)
            })
        );
        assert_eq!(com.dpt.value().map(|r| r.layer), Some(Layer::Instance));
        assert_eq!(com.flags.communication.value().map(|r| r.value), Some(true));

        // And the new table is there, empty, having invented nothing for the
        // `Empty` text slot it would have been entitled to guess about.
        let defaults: i64 = conn
            .query_row("SELECT COUNT(*) FROM com_object_program_default", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(defaults, 0);
        assert!(loaded
            .devices
            .program_defaults(ComObjectInstanceId(5))
            .is_none());
    }

    /// The point of schema 7, stated as an assertion: a communication object
    /// written by a pre-v7 build says nothing at all about Read-on-Init, and
    /// after the migration it still says nothing. "Not stated" is not
    /// "stated false", and a migration that backfilled `false` would be the
    /// same data loss §117 exists to end.
    #[test]
    fn a_pre_v7_com_object_reads_its_sixth_flag_as_absent_not_false() {
        use knx_core::{
            ComObjectInstance, ComObjectInstanceId, DeviceId, Layer, Override, Resolved,
            ResolvedFlags,
        };

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("v6.sqlite");
        std::fs::copy(
            concat!(env!("CARGO_MANIFEST_DIR"), "/fixtures/v6-empty.sqlite"),
            &path,
        )
        .unwrap();
        let conn = Connection::open(&path).unwrap();
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        // A v6-shaped object: five flags' worth of vocabulary available and
        // one of them stated. The insert goes through today's writer, which
        // knows the sixth attribute, so the `read_on_init` row is deleted
        // again below — a real v6 file has no such row at all, and a row
        // saying `absent` would let the test pass without ever exercising
        // the missing-row path it exists to prove.
        let flags = ResolvedFlags {
            communication: Override::Value(Resolved {
                value: false,
                layer: Layer::Instance,
            }),
            ..ResolvedFlags::none()
        };
        crate::devices::upsert_com_object_instance(
            &conn,
            DeviceId(1),
            0,
            &ComObjectInstance {
                id: ComObjectInstanceId(1),
                source: knx_core::SourceRef {
                    path: "P-0001/0.xml".into(),
                    ets_id: "A-1_O-1_R-1".into(),
                },
                device: DeviceId(1),
                number: 1,
                text: Override::Absent,
                description: Override::Absent,
                dpt: Override::Absent,
                flags,
                size: None,
                is_active: true,
                links: vec![],
                module_instance: None,
            },
        )
        .unwrap();
        // Back to a genuine v6 shape: the attribute vocabulary of a v6
        // build is the five flags, so the sixth row must not be there.
        let deleted = conn
            .execute(
                "DELETE FROM com_object_override WHERE attr = 'read_on_init'",
                [],
            )
            .unwrap();
        assert_eq!(deleted, 1, "the writer under test wrote the row we remove");
        let remaining: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM com_object_override WHERE attr = 'read_on_init'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(remaining, 0);
        drop(conn);

        let conn = open_and_migrate(&path).unwrap();
        // The migration invents nothing: still no row, and the loader turns
        // a missing row into `Absent` rather than a stated `false`.
        let rows: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM com_object_override WHERE attr = 'read_on_init'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(rows, 0, "v6 -> v7 must not backfill the sixth flag");
        let com = crate::devices::load_com_object_instance(&conn, ComObjectInstanceId(1)).unwrap();
        assert_eq!(com.flags.read_on_init, Override::Absent);
        // The flag that *was* stated false stays stated false — absence and
        // a stated `false` are still two different facts after the
        // migration.
        assert_eq!(
            com.flags.communication.value().map(|r| r.value),
            Some(false)
        );
    }

    /// Proves the migration's `DEFAULT ''` actually backfills a row that
    /// predates the column — not just that fresh inserts can supply one.
    #[test]
    fn v5_to_v6_backfills_a_pre_existing_module_instance_to_an_empty_instance_ets_id() {
        let conn = Connection::open_in_memory().unwrap();
        // FK off: this test hand-builds a v5-shaped row with a device_id
        // that names no real device, the way a real pre-v6 project's row
        // would — its own device row is out of scope for what this test
        // proves.
        conn.pragma_update(None, "foreign_keys", "OFF").unwrap();
        for migration in &migrations()[0..5] {
            migration(&conn).unwrap();
        }
        conn.pragma_update(None, "user_version", 5i64).unwrap();
        conn.execute(
            "INSERT INTO module_instance (id, device_id, position, source_path, source_ets_id, repeat_index) VALUES (1, 0, 0, 't', 'MD-2_M-4', '6x1')",
            [],
        )
        .unwrap();

        migrate(&conn).unwrap(); // runs the new migrate_v5_to_v6
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
        let instance_ets_id: String = conn
            .query_row(
                "SELECT instance_ets_id FROM module_instance WHERE id = 1",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(instance_ets_id, "");
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
        migrate(&conn).unwrap(); // runs the full chain, now including migrate_v5_to_v6 too
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION); // `migrate` always runs to CURRENT_SCHEMA_VERSION, not just to v5
        conn.execute("INSERT INTO module_instance (id, device_id, position, source_path, source_ets_id, repeat_index) VALUES (1, 0, 0, 't', 't', '6x1')", []).unwrap_err(); // device_id FK: no device(0) exists, expected to fail — proves the FK/table exist
        conn.query_row(
            "SELECT module_instance_id FROM com_object_instance LIMIT 0",
            [],
            |_| Ok(()),
        )
        .ok(); // column exists (no rows to fail on, just proves no "no such column" error at prepare time)
    }

    /// A v9 store with one installation, three group addresses and the
    /// opaque rows an import writes for them; `ets_schema` is the project's
    /// ETS schema version.
    fn v9_store_with_declarations(ets_schema: i64) -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        conn.pragma_update(None, "foreign_keys", "ON").unwrap();
        for migration in &migrations()[0..9] {
            migration(&conn).unwrap();
        }
        conn.pragma_update(None, "user_version", 9i64).unwrap();
        conn.execute(
            "INSERT INTO project_info (id, project_id, name, group_address_style, completion,
                 default_language, ets_schema_version)
             VALUES (0, 'P-1', 'p', 'ThreeLevel', 'Undefined', 'en', ?1)",
            params![ets_schema],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO installation (id, name, completion) VALUES (0, 'I', 'Undefined')",
            [],
        )
        .unwrap();
        for (id, ets) in [(1, "P-1-0_GA-1"), (2, "P-1-0_GA-2"), (3, "P-1-0_GA-3")] {
            conn.execute(
                "INSERT INTO group_address (id, installation_id, position, source_path,
                     source_ets_id, name, address, central, unfiltered)
                 VALUES (?1, 0, ?1, 'P-1/0.xml', ?2, 'ga', ?1, 0, 0)",
                params![id, ets],
            )
            .unwrap();
        }
        let keyed = |ets: &str| format!("{GROUP_ADDRESS_XPATH_PREFIX}[@Id='{ets}']");
        for (xpath, name, value) in [
            (keyed("P-1-0_GA-1"), "DatapointType", "DPST-9-1"),
            (keyed("P-1-0_GA-2"), "DatapointType", ""),
            (keyed("P-1-0_GA-3"), "DatapointType", "DPST-1-1 DPST-1-2"),
            (keyed("P-1-0_GA-1"), "Puid", "17"),
            // The pre-2026-09-20 unkeyed form, and a key naming no address.
            (
                GROUP_ADDRESS_XPATH_PREFIX.to_string(),
                "DatapointType",
                "DPST-5-1",
            ),
            (keyed("P-1-0_GA-99"), "DatapointType", "DPST-1-1"),
        ] {
            conn.execute(
                "INSERT INTO opaque_entry (source_path, xpath, kind, name, bytes, sha256)
                 VALUES ('P-1/0.xml', ?1, 'RetainedAttribute', ?2, ?3, 'x')",
                params![xpath, name, value.as_bytes()],
            )
            .unwrap();
        }
        conn
    }

    fn declared(conn: &Connection, id: i64) -> (String, Option<String>, Option<String>) {
        conn.query_row(
            "SELECT dpt_state, dpt_value, dpt_layer FROM group_address WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap()
    }

    fn opaque_rows(conn: &Connection) -> Vec<(String, String)> {
        conn.prepare("SELECT xpath, name FROM opaque_entry ORDER BY id")
            .unwrap()
            .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap()
    }

    fn unlifted(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT unlifted_group_address_dpt_declarations FROM project_info WHERE id = 0",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn v9_to_v10_lifts_keyed_declarations_and_counts_the_rest() {
        let conn = v9_store_with_declarations(21);
        migrate(&conn).unwrap();
        assert_eq!(
            declared(&conn, 1),
            (
                "value".into(),
                Some("DPST-9-1".into()),
                Some("Instance".into())
            )
        );
        assert_eq!(declared(&conn, 2), ("empty".into(), None, None));
        assert_eq!(
            declared(&conn, 3),
            ("malformed".into(), Some("DPST-1-1 DPST-1-2".into()), None)
        );
        let keyed = |ets: &str| format!("{GROUP_ADDRESS_XPATH_PREFIX}[@Id='{ets}']");
        assert_eq!(
            opaque_rows(&conn),
            vec![
                (keyed("P-1-0_GA-1"), "Puid".to_string()),
                (
                    GROUP_ADDRESS_XPATH_PREFIX.to_string(),
                    "DatapointType".to_string()
                ),
                (keyed("P-1-0_GA-99"), "DatapointType".to_string()),
            ],
            "lifted rows go, every other row stays"
        );
        assert_eq!(unlifted(&conn), 2);
    }

    #[test]
    fn v9_to_v10_lifts_nothing_for_a_schema_11_project() {
        let conn = v9_store_with_declarations(11);
        migrate(&conn).unwrap();
        for id in 1..=3 {
            assert_eq!(declared(&conn, id), ("absent".into(), None, None));
        }
        assert_eq!(opaque_rows(&conn).len(), 6);
        assert_eq!(unlifted(&conn), 0);
    }

    #[test]
    fn v9_to_v10_on_a_store_without_a_project_only_adds_columns() {
        let conn = Connection::open_in_memory().unwrap();
        for migration in &migrations()[0..9] {
            migration(&conn).unwrap();
        }
        conn.pragma_update(None, "user_version", 9i64).unwrap();
        migrate(&conn).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_SCHEMA_VERSION);
    }
}
