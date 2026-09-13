//! `products.sqlite`'s own schema and migration chain, keyed off SQLite's
//! `user_version` pragma.
//!
//! Deliberately not `knx-store`'s chain: a project-schema bump must not
//! force a product-database migration, or the other way round (ADR-0005,
//! ADR-0011). The two databases have different lifetimes — a project file
//! is per project, this one is shared across all of them.

use std::fmt;
use std::path::{Path, PathBuf};

use rusqlite::{params, Connection};

use crate::ingest::{classify, FileKind};
use crate::parse::translation::{ingest_translations, TranslationScope};
use crate::report::insert_unknown;

/// The product-database schema version this build writes.
pub const CURRENT_PRODUCTDB_VERSION: i64 = 6;

#[derive(Debug)]
pub enum ProductDbError {
    Sqlite(rusqlite::Error),
    Xml { source_path: String, cause: String },
    FutureVersion { found: i64, supported: i64 },
}

impl std::error::Error for ProductDbError {}

impl fmt::Display for ProductDbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProductDbError::Sqlite(e) => write!(f, "{e}"),
            ProductDbError::Xml { source_path, cause } => {
                write!(f, "{source_path}: {cause}")
            }
            ProductDbError::FutureVersion { found, supported } => write!(
                f,
                "product database is version {found}, this build supports up to {supported} — no downgrade path exists"
            ),
        }
    }
}

impl From<rusqlite::Error> for ProductDbError {
    fn from(e: rusqlite::Error) -> Self {
        ProductDbError::Sqlite(e)
    }
}

/// v0 -> v1. Extended in place while this plan runs, because the database
/// has no released state yet; frozen once the plan's last task lands.
fn migrate_v0_to_v1(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE schema_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
         INSERT INTO schema_meta (key, value) VALUES ('created_by', 'knx-productdb');
         CREATE TABLE source_file (
             sha256          TEXT PRIMARY KEY,
             source_path     TEXT NOT NULL,
             manufacturer_id TEXT,
             len             INTEGER NOT NULL,
             bytes           BLOB NOT NULL
         ) STRICT;
         CREATE INDEX source_file_manufacturer ON source_file (manufacturer_id);
         CREATE TABLE ingest_unknown (
             id            INTEGER PRIMARY KEY,
             source_sha256 TEXT NOT NULL,
             program_id    TEXT,
             xpath         TEXT NOT NULL,
             kind          TEXT NOT NULL,
             name          TEXT NOT NULL,
             occurrences   INTEGER NOT NULL,
             sample        TEXT
         ) STRICT;
         CREATE INDEX ingest_unknown_source ON ingest_unknown (source_sha256);
         CREATE TABLE manufacturer (
             id   TEXT PRIMARY KEY,
             name TEXT
         ) STRICT;
         CREATE TABLE catalog_section (
             id                  TEXT PRIMARY KEY,
             manufacturer_id     TEXT NOT NULL,
             parent_id           TEXT,
             name                TEXT,
             number              TEXT,
             visible_description TEXT,
             default_language    TEXT,
             source_sha256       TEXT NOT NULL
         ) STRICT;
         CREATE TABLE catalog_item (
             id                      TEXT PRIMARY KEY,
             manufacturer_id         TEXT NOT NULL,
             section_id              TEXT NOT NULL,
             name                    TEXT,
             number                  TEXT,
             visible_description     TEXT,
             product_ref_id          TEXT,
             hardware2program_ref_id TEXT,
             default_language        TEXT,
             source_sha256           TEXT NOT NULL
         ) STRICT;
         CREATE INDEX catalog_item_section ON catalog_item (section_id);
         CREATE TABLE hardware (
             id                      TEXT PRIMARY KEY,
             manufacturer_id         TEXT NOT NULL,
             name                    TEXT,
             serial_number           TEXT,
             version_number          TEXT,
             bus_current             TEXT,
             has_individual_address  INTEGER,
             has_application_program INTEGER,
             is_accessory            INTEGER,
             is_coupler              INTEGER,
             is_power_supply         INTEGER,
             is_ip_enabled           INTEGER,
             is_power_line_repeater  INTEGER,
             original_manufacturer   TEXT,
             source_sha256           TEXT NOT NULL
         ) STRICT;
         CREATE TABLE product (
             id                  TEXT PRIMARY KEY,
             manufacturer_id     TEXT NOT NULL,
             hardware_id         TEXT NOT NULL,
             text                TEXT,
             order_number        TEXT,
             is_rail_mounted     INTEGER,
             width_in_millimeter TEXT,
             default_language    TEXT,
             hash                TEXT,
             registration_status TEXT,
             source_sha256       TEXT NOT NULL
         ) STRICT;
         CREATE INDEX product_hardware ON product (hardware_id);
         CREATE TABLE hardware2program (
             id                      TEXT PRIMARY KEY,
             manufacturer_id         TEXT NOT NULL,
             hardware_id             TEXT NOT NULL,
             application_program_ref TEXT,
             medium_types            TEXT,
             hash                    TEXT,
             registration_number     TEXT,
             registration_status     TEXT,
             registration_signature  TEXT,
             source_sha256           TEXT NOT NULL
         ) STRICT;
         CREATE INDEX hardware2program_program ON hardware2program (application_program_ref);
         CREATE TABLE application_program (
             id                    TEXT PRIMARY KEY,
             manufacturer_id       TEXT NOT NULL,
             name                  TEXT,
             application_number    TEXT,
             application_version   TEXT,
             program_type          TEXT,
             mask_version          TEXT,
             pei_type              TEXT,
             load_procedure_style  TEXT,
             default_language      TEXT,
             hash                  TEXT,
             linkable              INTEGER,
             original_manufacturer TEXT,
             source_sha256         TEXT NOT NULL
         ) STRICT;
         CREATE TABLE parameter_type (
             program_id     TEXT NOT NULL,
             id             TEXT NOT NULL,
             name           TEXT,
             kind           TEXT NOT NULL,
             size_in_bit    INTEGER,
             base           TEXT,
             min_inclusive  TEXT,
             max_inclusive  TEXT,
             number_type    TEXT,
             PRIMARY KEY (program_id, id)
         ) STRICT;
         CREATE TABLE parameter_type_enum (
             program_id        TEXT NOT NULL,
             parameter_type_id TEXT NOT NULL,
             id                TEXT NOT NULL,
             value             TEXT,
             text              TEXT,
             display_order     INTEGER,
             PRIMARY KEY (program_id, id)
         ) STRICT;
         CREATE INDEX parameter_type_enum_type ON parameter_type_enum (program_id, parameter_type_id);
         CREATE TABLE parameter (
             program_id        TEXT NOT NULL,
             id                TEXT NOT NULL,
             name              TEXT,
             text              TEXT,
             parameter_type_id TEXT,
             access            TEXT,
             value             TEXT,
             suffix            TEXT,
             code_segment      TEXT,
             offset            INTEGER,
             bit_offset        INTEGER,
             union_id          INTEGER,
             union_size_in_bit INTEGER,
             PRIMARY KEY (program_id, id)
         ) STRICT;
         CREATE TABLE parameter_ref (
             program_id    TEXT NOT NULL,
             id            TEXT NOT NULL,
             parameter_id  TEXT NOT NULL,
             display_order INTEGER,
             tag           TEXT,
             text          TEXT,
             value         TEXT,
             PRIMARY KEY (program_id, id)
         ) STRICT;
         CREATE INDEX parameter_ref_program ON parameter_ref (program_id);
         CREATE TABLE com_object (
             program_id          TEXT NOT NULL,
             id                  TEXT NOT NULL,
             number              INTEGER,
             name                TEXT,
             text                TEXT,
             function_text       TEXT,
             visible_description TEXT,
             object_size         TEXT,
             priority            TEXT,
             dpt_list            TEXT,
             read_flag           TEXT,
             write_flag          TEXT,
             transmit_flag       TEXT,
             update_flag         TEXT,
             communication_flag  TEXT,
             read_on_init_flag   TEXT,
             PRIMARY KEY (program_id, id)
         ) STRICT;
         CREATE TABLE com_object_ref (
             program_id          TEXT NOT NULL,
             id                  TEXT NOT NULL,
             com_object_id       TEXT NOT NULL,
             tag                 TEXT,
             text                TEXT,
             function_text       TEXT,
             visible_description TEXT,
             object_size         TEXT,
             priority            TEXT,
             dpt_list            TEXT,
             read_flag           TEXT,
             write_flag          TEXT,
             transmit_flag       TEXT,
             update_flag         TEXT,
             communication_flag  TEXT,
             read_on_init_flag   TEXT,
             PRIMARY KEY (program_id, id)
         ) STRICT;
         CREATE INDEX com_object_ref_program ON com_object_ref (program_id);
         CREATE INDEX com_object_ref_object ON com_object_ref (program_id, com_object_id);
         CREATE TABLE translation (
             program_id     TEXT NOT NULL,
             language       TEXT NOT NULL,
             ref_id         TEXT NOT NULL,
             attribute_name TEXT NOT NULL,
             text           TEXT,
             PRIMARY KEY (program_id, language, ref_id, attribute_name)
         ) STRICT;
         CREATE INDEX translation_lookup ON translation (program_id, language, ref_id);
         CREATE TABLE datapoint_type (
             id   TEXT PRIMARY KEY,
             main INTEGER NOT NULL,
             sub  INTEGER,
             name TEXT,
             text TEXT
         ) STRICT;",
    )?;
    Ok(())
}

type Migration = fn(&Connection) -> Result<(), ProductDbError>;

fn migrations() -> Vec<Migration> {
    vec![
        migrate_v0_to_v1,
        migrate_v1_to_v2,
        migrate_v2_to_v3,
        migrate_v3_to_v4,
        migrate_v4_to_v5,
        migrate_v5_to_v6,
    ]
}

/// v5 -> v6. One more `package` counter, same shape and same reasoning as
/// `migrate_v4_to_v5`'s four: `dropped_datapoint_type_count` records how
/// many `datapoint_type` rows this package's own `knx_master.xml` declared
/// that `INSERT OR IGNORE` dropped because the id already belonged to an
/// earlier package (KNOWN_LIMITATIONS.md §84). Defaults to `0` for a
/// package installed before this column existed, for the same honesty
/// reason `migrate_v4_to_v5` gives: re-deriving the true count would mean
/// re-parsing bytes this migration does not have.
fn migrate_v5_to_v6(conn: &Connection) -> Result<(), ProductDbError> {
    if !column_exists(conn, "package", "dropped_datapoint_type_count")? {
        conn.execute_batch(
            "ALTER TABLE package ADD COLUMN dropped_datapoint_type_count INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    Ok(())
}

/// v4 -> v5. Gives `package` four new per-scope counters so a re-opened
/// (already-installed) package can still report how many `translation` rows
/// it contributed (R3), without re-parsing the archive just to answer that.
/// Each defaults to `0`: a package already installed under schema v4 has no
/// record of what it wrote at install time (that install pre-dates the
/// columns), and re-deriving the true count would mean re-parsing bytes this
/// migration has no access to — so a pre-existing package reports zero
/// rather than a guess, honestly naming the gap instead of inventing a
/// number. A package installed from here on always gets its real count.
///
/// Guarded column-by-column rather than as one `execute_batch`: a database
/// whose `user_version` says v4 but whose `package` table already carries
/// one or more of these columns (a hand-rolled test fixture rolling other
/// tables back to an earlier shape without touching `package`, for
/// instance — see `dynamic_tree.rs`'s backfill tests) must not fail this
/// migration with SQLite's "duplicate column name" just because the schema
/// is ahead of the version pragma for this one table. Each `ALTER TABLE` is
/// skipped if its column is already there, applied if not — so running
/// this migration twice against the same `package` table is always safe.
fn migrate_v4_to_v5(conn: &Connection) -> Result<(), ProductDbError> {
    for column in [
        "translation_program_count",
        "translation_catalog_count",
        "translation_hardware_count",
        "translation_master_count",
    ] {
        if !column_exists(conn, "package", column)? {
            conn.execute_batch(&format!(
                "ALTER TABLE package ADD COLUMN {column} INTEGER NOT NULL DEFAULT 0;"
            ))?;
        }
    }
    Ok(())
}

/// True if `table` already has a column named `column`. Used where an
/// `ALTER TABLE ... ADD COLUMN` migration must tolerate being replayed
/// against a table that already has it (see `migrate_v4_to_v5`).
fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool, ProductDbError> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get("name")?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// v3 -> v4. Widens `translation` from a program-only table to one that can
/// hold translations for anything an ingest pass wants to attach a language
/// override to (`Catalog`/`Hardware`/`Master`, added by later tasks in this
/// plan): `program_id` becomes the generic `(scope, scope_id)` pair, `scope`
/// naming which table `scope_id` refers into. SQLite cannot widen a primary
/// key in place, so the table is rebuilt: a new `translation_v4` is created,
/// every existing row is copied across with `scope = 'Program'` and
/// `scope_id` set to the old `program_id`, then the old table is dropped and
/// the new one renamed into its place.
///
/// `scope_id` is `NOT NULL` with `''` as the sentinel for a master-scope row
/// (one that is not attached to any particular program, catalog item or
/// piece of hardware) rather than nullable, for exactly the reason
/// `migrate_v2_to_v3`'s `dynamic_node.module_def_id` sentinel exists: SQLite
/// treats NULLs in a non-`INTEGER` `PRIMARY KEY` as pairwise distinct, which
/// would defeat the uniqueness constraint for exactly the rows ingested once
/// per package rather than once per program/catalog item/hardware entry.
fn migrate_v3_to_v4(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE translation_v4 (
            scope          TEXT NOT NULL,
            scope_id       TEXT NOT NULL,
            language       TEXT NOT NULL,
            ref_id         TEXT NOT NULL,
            attribute_name TEXT NOT NULL,
            text           TEXT,
            PRIMARY KEY (scope, scope_id, language, ref_id, attribute_name)
        ) STRICT;
        INSERT INTO translation_v4 (scope, scope_id, language, ref_id, attribute_name, text)
            SELECT 'Program', program_id, language, ref_id, attribute_name, text FROM translation;
        DROP INDEX translation_lookup;
        DROP TABLE translation;
        ALTER TABLE translation_v4 RENAME TO translation;
        CREATE INDEX translation_lookup ON translation (scope, scope_id, language, ref_id);",
    )?;
    backfill_shared_translations(conn)?;
    Ok(())
}

/// A product database that reached v3 before `Catalog`/`Hardware`/`Master`
/// scoped translations existed has `source_file` blobs whose own
/// `Languages` block was never read — `ingest_translations` is a second
/// pass over bytes an entity parser already consumed (`ingest.rs`), and
/// installation's content-hash idempotence (`source_parse_evidence`) means
/// an already-installed blob is never revisited by the ordinary path.
/// Modelled directly on `backfill_dynamic_nodes` above (read that one
/// first): this replays every stored blob that classifies as `Catalog`,
/// `Hardware` or `MasterData` through `ingest_translations`, after the
/// table rebuild above, inside the same migration transaction
/// `open_and_migrate` already holds.
///
/// Each blob gets its own `SAVEPOINT`, released on success and rolled back
/// to on error before `record_backfill_failure` runs, for the identical
/// reason `backfill_dynamic_nodes` does: a parse error partway through a
/// blob must not leave partial rows behind, but must also not stop the loop
/// from reaching the next blob or abort the migration outright — a database
/// that refuses to open is worse than one with a gap.
fn backfill_shared_translations(conn: &Connection) -> Result<(), ProductDbError> {
    let mut stmt = conn.prepare("SELECT sha256, source_path, bytes FROM source_file")?;
    let blobs: Vec<(String, String, Vec<u8>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    for (sha256, source_path, bytes) in blobs {
        // `Program`-scope translations are already covered by
        // `application_program`'s own `Languages` parse; only the scopes
        // that ride a second pass over bytes need replaying here. Anything
        // else — baggage, unrecognized content — is skipped without being
        // parsed at all, exactly as the ordinary ingest path dispatches.
        let scope = match classify(&bytes) {
            FileKind::Catalog => TranslationScope::Catalog,
            FileKind::Hardware => TranslationScope::Hardware,
            FileKind::MasterData => TranslationScope::Master,
            _ => continue,
        };
        conn.execute_batch("SAVEPOINT translation_backfill_blob;")?;
        match ingest_translations(conn, scope, &source_path, &bytes) {
            Ok(_) => {
                conn.execute_batch("RELEASE SAVEPOINT translation_backfill_blob;")?;
            }
            Err(error) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT translation_backfill_blob;
                     RELEASE SAVEPOINT translation_backfill_blob;",
                )?;
                record_backfill_failure(
                    conn,
                    &sha256,
                    &source_path,
                    "TranslationBackfillError",
                    "ingest_translations",
                    &error,
                )?;
            }
        }
    }
    Ok(())
}

/// v2 -> v3. Adds `dynamic_node` (design D2,
/// `docs/superpowers/specs/2026-09-11-dynamic-tree-parse-and-evaluate-design.md`):
/// one row per element of every `ApplicationProgram`/`ModuleDef` `Dynamic`
/// tree, stored losslessly and unevaluated. `module_def_id` is `NOT NULL`
/// with `''` as the sentinel for the program's own tree rather than
/// nullable, because SQLite treats NULLs in a non-`INTEGER` `PRIMARY KEY`
/// as pairwise distinct, which would silently defeat the uniqueness
/// constraint for exactly the common case.
///
/// `extra` (every attribute not captured by a dedicated column, as
/// `"name=value"` pairs, sorted, newline-joined) is a human-readable audit
/// trail, not a re-parseable encoding: it cannot be split unambiguously
/// back apart when a value itself contains `=` or a newline.
///
/// This is the first migration in this crate that runs Rust rather than
/// plain SQL — see `backfill_dynamic_nodes` below, which is the payoff
/// ADR-0011's blob store was designed for: a file's bytes are kept
/// specifically so a later parser can read what an earlier one skipped,
/// without asking the user to feed the file in again.
fn migrate_v2_to_v3(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE dynamic_node (
            program_id    TEXT NOT NULL,
            module_def_id TEXT NOT NULL,
            node_id       INTEGER NOT NULL,
            parent_id     INTEGER,
            position      INTEGER NOT NULL,
            kind          TEXT NOT NULL,
            element_id    TEXT,
            ref_id        TEXT,
            test          TEXT,
            is_default    INTEGER,
            text          TEXT,
            extra         TEXT,
            PRIMARY KEY (program_id, module_def_id, node_id)
        ) STRICT;
        CREATE UNIQUE INDEX dynamic_node_sibling
            ON dynamic_node (program_id, module_def_id, parent_id, position);",
    )?;
    backfill_dynamic_nodes(conn)?;
    Ok(())
}

/// Design D5: a product database that reached v2 before `dynamic_node`
/// existed has `source_file` blobs and `application_program` rows, but
/// nothing in `dynamic_node` for them — installation is content-hash
/// idempotent (`source_parse_evidence`), so a file already on record is
/// never re-parsed, and without this backfill such a database would stay
/// empty forever with no visible sign of why. This replays every stored
/// blob that looks like `ApplicationProgram` content through Task 1's
/// `dynamic::parse::parse_dynamic_trees`, inside the same migration
/// transaction `open_and_migrate` already holds.
///
/// `parse_dynamic_trees`'s own `program_should_be_skipped` check is *not*
/// keyed on `application_program.source_sha256` matching (see its doc
/// comment in `dynamic/parse.rs`) precisely so this call is not a silent
/// no-op: every program already in `application_program` at this point has
/// a long-since-stored `source_sha256` that trivially matches its own
/// blob, and skipping on that basis would backfill nothing at all.
///
/// A single blob's parse failure is recorded into `ingest_unknown`
/// (`kind = 'DynamicBackfillError'`) and does not abort the migration or
/// undo the blobs already processed in this same pass — a database that
/// refuses to open is worse than one with a gap.
///
/// `parse_dynamic_trees` inserts `dynamic_node` rows incrementally as it
/// walks the XML event stream, so a malformed blob (e.g. a mismatched end
/// tag) can leave a handful of rows behind before the error is even raised
/// — quick-xml only notices the mismatch once it reaches the offending end
/// tag, by which point every element opened before it is already inserted.
/// Each blob therefore gets its own `SAVEPOINT`, released on success and
/// rolled back to on error, *before* `record_backfill_failure` runs — this
/// is the same "a parse error partway through leaves the database exactly
/// as it was" invariant `ingest.rs`'s doc comment already promises for the
/// ordinary install path (`ingest.rs:33-37`), now honoured here too. It is
/// purely an inner boundary: the outer migration transaction
/// `open_and_migrate` holds around this whole function is untouched, and a
/// bad blob still does not stop the loop from reaching the next one.
fn backfill_dynamic_nodes(conn: &Connection) -> Result<(), ProductDbError> {
    let mut stmt = conn.prepare("SELECT sha256, source_path, bytes FROM source_file")?;
    let blobs: Vec<(String, String, Vec<u8>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    for (sha256, source_path, bytes) in blobs {
        // Only `ApplicationProgram` content can carry a `Dynamic` tree;
        // everything else (catalog, hardware, baggage, ...) is skipped
        // without being parsed at all, exactly as the ordinary ingest path
        // already dispatches by `classify`.
        if classify(&bytes) != FileKind::ApplicationProgram {
            continue;
        }
        conn.execute_batch("SAVEPOINT dynamic_backfill_blob;")?;
        match crate::dynamic::parse::parse_dynamic_trees(conn, &sha256, &source_path, &bytes) {
            Ok(outcome) => {
                conn.execute_batch("RELEASE SAVEPOINT dynamic_backfill_blob;")?;
                insert_unknown(conn, &sha256, &outcome.unknown)?;
            }
            Err(error) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT dynamic_backfill_blob;
                     RELEASE SAVEPOINT dynamic_backfill_blob;",
                )?;
                record_backfill_failure(
                    conn,
                    &sha256,
                    &source_path,
                    "DynamicBackfillError",
                    "parse_dynamic_trees",
                    &error,
                )?;
            }
        }
    }
    Ok(())
}

/// Records a backfill parse failure through the same `ingest_unknown` table
/// every other diagnostic in this crate lands in (`report::insert_unknown`,
/// `report::insert_conflicts`'s `'IdConflict'` rows follow the identical
/// pattern of a literal `kind` string with no table of its own). Shared by
/// every backfill in this file — `kind` and `name` are the only things that
/// differ between, say, `backfill_dynamic_nodes`'s
/// `('DynamicBackfillError', "parse_dynamic_trees")` and
/// `backfill_shared_translations`'s
/// `('TranslationBackfillError', "ingest_translations")`.
fn record_backfill_failure(
    conn: &Connection,
    sha256: &str,
    source_path: &str,
    kind: &str,
    name: &str,
    error: &ProductDbError,
) -> Result<(), ProductDbError> {
    conn.execute(
        "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
         VALUES (?1, NULL, ?2, ?3, ?4, 1, ?5)",
        params![sha256, source_path, kind, name, error.to_string()],
    )?;
    Ok(())
}

fn migrate_v1_to_v2(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE package (
            sha256 TEXT PRIMARY KEY,
            source_name TEXT NOT NULL,
            scheme INTEGER NOT NULL,
            size INTEGER NOT NULL,
            bytes BLOB NOT NULL,
            unknown_count INTEGER NOT NULL
        ) STRICT;
        CREATE TABLE package_member (
            package_sha256 TEXT NOT NULL REFERENCES package(sha256),
            ordinal INTEGER NOT NULL,
            path TEXT NOT NULL,
            role TEXT NOT NULL,
            source_sha256 TEXT NOT NULL REFERENCES source_file(sha256),
            size INTEGER NOT NULL,
            PRIMARY KEY (package_sha256, path),
            UNIQUE (package_sha256, ordinal)
        ) STRICT;
        CREATE TABLE source_parse_evidence (
            sha256 TEXT PRIMARY KEY REFERENCES source_file(sha256)
        ) STRICT;
        INSERT INTO source_parse_evidence (sha256) SELECT sha256 FROM source_file;
        CREATE TABLE package_conflict (
            package_sha256 TEXT NOT NULL REFERENCES package(sha256),
            ordinal INTEGER NOT NULL,
            table_name TEXT NOT NULL,
            logical_id TEXT NOT NULL,
            kept_sha256 TEXT NOT NULL,
            other_sha256 TEXT NOT NULL,
            PRIMARY KEY (package_sha256, ordinal)
        ) STRICT;",
    )?;
    Ok(())
}

/// Opens (creating if absent) the product database at `path`, runs every
/// pending migration in order, and returns the connection at
/// `CURRENT_PRODUCTDB_VERSION`. Creates the parent directory, since the
/// default path lives under a data directory the user may not have yet.
pub fn open_and_migrate(path: &Path) -> Result<Connection, ProductDbError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| ProductDbError::Xml {
                source_path: parent.display().to_string(),
                cause: e.to_string(),
            })?;
        }
    }
    let conn = Connection::open(path)?;
    let found: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found > CURRENT_PRODUCTDB_VERSION {
        return Err(ProductDbError::FutureVersion {
            found,
            supported: CURRENT_PRODUCTDB_VERSION,
        });
    }
    if found < CURRENT_PRODUCTDB_VERSION {
        conn.execute_batch("BEGIN IMMEDIATE")?;
        let result = (|| {
            for migration in &migrations()[found as usize..CURRENT_PRODUCTDB_VERSION as usize] {
                migration(&conn)?;
            }
            conn.pragma_update(None, "user_version", CURRENT_PRODUCTDB_VERSION)?;
            Ok::<(), ProductDbError>(())
        })();
        match result {
            Ok(()) => conn.execute_batch("COMMIT")?,
            Err(error) => {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(error);
            }
        }
    }
    Ok(conn)
}

/// `$XDG_DATA_HOME/knx/products.sqlite`, falling back to
/// `$HOME/.local/share/knx/products.sqlite`. `None` when neither variable
/// is set, which the caller reports rather than guessing a location.
pub fn default_path() -> Option<PathBuf> {
    if let Some(dir) = std::env::var_os("XDG_DATA_HOME").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(dir).join("knx").join("products.sqlite"));
    }
    let home = std::env::var_os("HOME").filter(|v| !v.is_empty())?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("knx")
            .join("products.sqlite"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_file_migrates_to_current_version() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);
        let marker: String = conn
            .query_row(
                "SELECT value FROM schema_meta WHERE key = 'created_by'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(marker, "knx-productdb");
    }

    #[test]
    fn reopening_an_already_migrated_file_is_a_no_op() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        open_and_migrate(&path).unwrap();
        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);
    }

    #[test]
    fn a_file_from_a_newer_version_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        {
            let conn = rusqlite::Connection::open(&path).unwrap();
            conn.pragma_update(None, "user_version", CURRENT_PRODUCTDB_VERSION + 1)
                .unwrap();
        }
        assert!(matches!(
            open_and_migrate(&path),
            Err(ProductDbError::FutureVersion { .. })
        ));
    }

    #[test]
    fn default_path_sits_under_the_xdg_data_directory() {
        // `default_path` reads the environment; assert its shape, not a
        // machine-specific absolute path.
        let p = default_path().expect("HOME or XDG_DATA_HOME is set in CI");
        assert!(p.ends_with("knx/products.sqlite"), "{}", p.display());
    }

    #[test]
    fn a_v3_database_keeps_every_translation_row_through_the_v4_rebuild() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        {
            let conn = Connection::open(&path).unwrap();
            for migration in &migrations()[0..3] {
                migration(&conn).unwrap();
            }
            conn.pragma_update(None, "user_version", 3i64).unwrap();
            conn.execute_batch(
                "INSERT INTO translation (program_id, language, ref_id, attribute_name, text)
                 VALUES ('A-1', 'en-US', 'A-1_O-0', 'Text', 'Output');
                 INSERT INTO translation (program_id, language, ref_id, attribute_name, text)
                 VALUES ('A-1', 'de-DE', 'A-1_O-0', 'Text', 'Ausgang');
                 INSERT INTO translation (program_id, language, ref_id, attribute_name, text)
                 VALUES ('A-2', 'en-US', 'A-2_O-0', 'FunctionText', 'Switch');",
            )
            .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);

        let count: i64 = conn
            .query_row("SELECT count(*) FROM translation", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 3);

        let scopes: i64 = conn
            .query_row(
                "SELECT count(*) FROM translation WHERE scope = 'Program'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(scopes, 3);

        let scope_id: String = conn
            .query_row(
                "SELECT scope_id FROM translation
                 WHERE language = 'en-US' AND ref_id = 'A-1_O-0' AND attribute_name = 'Text'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(scope_id, "A-1");
    }

    #[test]
    fn the_master_scope_id_sentinel_is_the_empty_string_not_null() {
        let dir = tempfile::tempdir().unwrap();
        let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        conn.execute_batch(
            "INSERT OR IGNORE INTO translation (scope, scope_id, language, ref_id, attribute_name, text)
             VALUES ('Master', '', 'en-US', 'M-1', 'Text', 'Foo');
             INSERT OR IGNORE INTO translation (scope, scope_id, language, ref_id, attribute_name, text)
             VALUES ('Master', '', 'en-US', 'M-2', 'Text', 'Bar');
             INSERT OR IGNORE INTO translation (scope, scope_id, language, ref_id, attribute_name, text)
             VALUES ('Master', '', 'en-US', 'M-1', 'Text', 'Duplicate');",
        )
        .unwrap();
        let count: i64 = conn
            .query_row(
                "SELECT count(*) FROM translation WHERE scope = 'Master'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    const CATALOG_WITH_LANGUAGES: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-0083">
      <Catalog>
        <CatalogSection Id="M-0083_CG-1" Name="Sensors" Number="1">
          <CatalogItem Id="M-0083_CI-1" Name="Sensor" Number="1" />
        </CatalogSection>
      </Catalog>
      <Languages>
        <Language Identifier="en-US">
          <TranslationUnit RefId="M-0083_CI-1">
            <TranslationElement RefId="M-0083_CI-1">
              <Translation AttributeName="Name" Text="Sensor" />
            </TranslationElement>
          </TranslationUnit>
        </Language>
      </Languages>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    #[test]
    fn a_v3_database_backfills_the_translations_its_blobs_already_held() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let bytes = CATALOG_WITH_LANGUAGES.as_bytes();
        let sha = crate::sha256_hex(bytes);
        {
            let conn = Connection::open(&path).unwrap();
            for migration in &migrations()[0..3] {
                migration(&conn).unwrap();
            }
            conn.pragma_update(None, "user_version", 3i64).unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    sha,
                    "M-0083/Catalog.xml",
                    "M-0083",
                    bytes.len() as i64,
                    bytes
                ],
            )
            .unwrap();
            // Also present in `source_parse_evidence`, exactly like a blob
            // whose catalog rows were already parsed under the ordinary
            // (pre-shared-translations) path: the content-hash skip in
            // `ingest.rs` would leave this blob alone forever without the
            // backfill.
            conn.execute(
                "INSERT INTO source_parse_evidence (sha256) VALUES (?1)",
                [&sha],
            )
            .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);

        let text: String = conn
            .query_row(
                "SELECT text FROM translation
                 WHERE scope = 'Catalog' AND scope_id = 'M-0083'
                   AND ref_id = 'M-0083_CI-1' AND attribute_name = 'Name'
                   AND language = 'en-US'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(text, "Sensor");
    }

    #[test]
    fn a_blob_that_fails_to_parse_records_itself_and_does_not_stop_the_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let good = CATALOG_WITH_LANGUAGES.as_bytes();
        let good_sha = crate::sha256_hex(good);
        // Truncated 20 bytes before the end: empirically this lands inside
        // `</ManufacturerData>`'s closing tag, which `quick-xml` rejects
        // with "tag not closed" rather than treating as ordinary `Eof` —
        // the same "genuine parse error, not silent truncation" shape
        // `dynamic_tree.rs`'s equivalent backfill test documents.
        let bad = &good[..good.len() - 20];
        let bad_sha = crate::sha256_hex(bad);
        {
            let conn = Connection::open(&path).unwrap();
            for migration in &migrations()[0..3] {
                migration(&conn).unwrap();
            }
            conn.pragma_update(None, "user_version", 3i64).unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    good_sha,
                    "M-0083/Catalog.xml",
                    "M-0083",
                    good.len() as i64,
                    good
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![bad_sha, "M-BAD/Catalog.xml", "M-BAD", bad.len() as i64, bad],
            )
            .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        let version: i64 = conn
            .query_row("PRAGMA user_version", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            version, CURRENT_PRODUCTDB_VERSION,
            "one blob's parse failure must not abort the migration"
        );

        let text: String = conn
            .query_row(
                "SELECT text FROM translation
                 WHERE scope = 'Catalog' AND scope_id = 'M-0083'
                   AND ref_id = 'M-0083_CI-1' AND attribute_name = 'Name'
                   AND language = 'en-US'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(text, "Sensor", "the good blob must still be backfilled");

        let recorded: i64 = conn
            .query_row(
                "SELECT count(*) FROM ingest_unknown
                 WHERE source_sha256 = ?1 AND kind = 'TranslationBackfillError'",
                [&bad_sha],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(recorded, 1);
    }
}
