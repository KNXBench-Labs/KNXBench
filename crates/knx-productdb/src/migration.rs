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
pub const CURRENT_PRODUCTDB_VERSION: i64 = 22;

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
        migrate_v6_to_v7,
        migrate_v7_to_v8,
        migrate_v8_to_v9,
        migrate_v9_to_v10,
        migrate_v10_to_v11,
        migrate_v11_to_v12,
        migrate_v12_to_v13,
        migrate_v13_to_v14,
        migrate_v14_to_v15,
        migrate_v15_to_v16,
        migrate_v16_to_v17,
        migrate_v17_to_v18,
        migrate_v18_to_v19,
        migrate_v19_to_v20,
        migrate_v20_to_v21,
        migrate_v21_to_v22,
    ]
}

/// v21 -> v22 (ADR-0094, legacy EX-IM product databases). Additive DDL only.
///
/// A published legacy file is keyed by its decrypted payload. Both the
/// payload and the original file stay in `source_file`, byte for byte; the
/// password is stored nowhere. `legacy_program` marks every application
/// program that came from such a file: its `source_sha256` names EX-IM text,
/// not XML, and consumers that read program XML (the download path) must
/// refuse it by name. `legacy_diagnostic` keeps the publication report.
fn migrate_v21_to_v22(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE legacy_source (
             payload_sha256 TEXT PRIMARY KEY,
             namespace TEXT NOT NULL UNIQUE,
             member_name TEXT NOT NULL,
             member_kind TEXT NOT NULL,
             format_version TEXT,
             exported_at TEXT,
             producer TEXT,
             charset TEXT NOT NULL
         ) STRICT;
         CREATE TABLE legacy_source_file (
             payload_sha256 TEXT NOT NULL REFERENCES legacy_source (payload_sha256),
             original_sha256 TEXT NOT NULL,
             source_name TEXT NOT NULL,
             encrypted INTEGER NOT NULL,
             PRIMARY KEY (payload_sha256, original_sha256, source_name)
         ) STRICT;
         CREATE TABLE legacy_program (
             program_id TEXT PRIMARY KEY,
             payload_sha256 TEXT NOT NULL REFERENCES legacy_source (payload_sha256),
             exim_program_id TEXT NOT NULL
         ) STRICT;
         CREATE TABLE legacy_diagnostic (
             payload_sha256 TEXT NOT NULL REFERENCES legacy_source (payload_sha256),
             ordinal INTEGER NOT NULL,
             kind TEXT NOT NULL,
             detail TEXT NOT NULL,
             PRIMARY KEY (payload_sha256, ordinal)
         ) STRICT;",
    )?;
    Ok(())
}

/// v20 -> v21 (ADR-0081, KNOWN_LIMITATIONS §156). `Parameter` and
/// `ParameterRef` now report every attribute they do not store. No DDL; per
/// stored blob that classifies as an `ApplicationProgram` and whose bytes
/// still match their key, in its own savepoint,
/// `parse::program::backfill_parameter_attribute_unknowns` adds the rows a
/// current ingest writes, for the keys the blob lacks, once per package
/// member that re-parsed it (or once for a standalone blob): the v16 rule. Scheme-evidence reconciliation runs in the mode ingest used: the
/// package mode for a blob some scheme-21/23 package carries.
///
/// Every measured package report then gains its members' new rows, merged
/// as install merges them, and `package.unknown_count` the distinct rows
/// each `ApplicationProgram` member gained — the v16/v18 rule, not v15's
/// "historical" one: the old report under-states what the parser met, and
/// these rows are pure functions of the retained bytes. A report with a
/// member that was not carried forward (missing, damaged or unparseable
/// bytes), or whose rewrite does not validate, is downgraded to
/// `unavailable` with an `InstallReportBackfillError`; the blob itself gets
/// a `ParameterAttributeBackfillError`.
fn migrate_v20_to_v21(conn: &Connection) -> Result<(), ProductDbError> {
    let blobs = conn
        .prepare(
            "SELECT s.sha256, s.source_path,
                    EXISTS (SELECT 1 FROM package_member AS m JOIN package AS p
                            ON p.sha256 = m.package_sha256
                            WHERE m.source_sha256 = s.sha256 AND p.scheme IN (21, 23)),
                    (SELECT count(*) FROM package_member AS m
                     WHERE m.source_sha256 = s.sha256 AND m.role = 'ApplicationProgram')
             FROM source_file AS s ORDER BY s.sha256",
        )?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    // Blob -> the rows it gained. A program blob missing from this map was
    // not carried forward, and neither is any report that counts it.
    let mut gained: std::collections::HashMap<String, Vec<crate::report::UnknownConstruct>> =
        std::collections::HashMap::new();
    for (sha256, source_path, extended, members) in blobs {
        let failure = |cause: &str| ProductDbError::Xml {
            source_path: source_path.clone(),
            cause: format!("v21 backfill: {cause}"),
        };
        // One copy per package that re-parsed it, or one for a blob only a
        // standalone ingest parsed (the v16 rule).
        let copies = usize::try_from(members.max(1)).unwrap_or(1);
        let Some(bytes) = crate::load_source_file(conn, &sha256)? else {
            continue;
        };
        let refused = if classify(&bytes) != FileKind::ApplicationProgram {
            // A damaged blob can stop classifying as a program altogether;
            // its owner or member row still says this migration owes it rows.
            let was_program: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM application_program WHERE source_sha256 = ?1)
                     OR EXISTS(SELECT 1 FROM package_member WHERE source_sha256 = ?1
                               AND role = 'ApplicationProgram')",
                [&sha256],
                |r| r.get(0),
            )?;
            if !was_program {
                continue;
            }
            failure("stored program no longer classifies as an ApplicationProgram")
        } else if crate::sha256_hex(&bytes) != sha256 {
            failure("stored bytes do not match their SHA-256 key")
        } else {
            conn.execute_batch("SAVEPOINT v21_parameter_attributes;")?;
            match crate::parse::program::backfill_parameter_attribute_unknowns(
                conn,
                &sha256,
                &source_path,
                &bytes,
                extended,
                copies,
            ) {
                Ok(rows) => {
                    conn.execute_batch("RELEASE SAVEPOINT v21_parameter_attributes;")?;
                    gained.insert(sha256, rows);
                    continue;
                }
                Err(ProductDbError::Sqlite(error)) => return Err(ProductDbError::Sqlite(error)),
                Err(error) => {
                    conn.execute_batch(
                        "ROLLBACK TO SAVEPOINT v21_parameter_attributes;
                         RELEASE SAVEPOINT v21_parameter_attributes;",
                    )?;
                    error
                }
            }
        };
        record_backfill_failure(
            conn,
            &sha256,
            &source_path,
            "ParameterAttributeBackfillError",
            "backfill_parameter_attribute_unknowns",
            &refused,
        )?;
    }
    add_parameter_attributes_to_reports(conn, &gained)
}

/// The package half of `migrate_v20_to_v21`, the additive mirror of
/// `retire_channel_number_from_reports`.
fn add_parameter_attributes_to_reports(
    conn: &Connection,
    gained: &std::collections::HashMap<String, Vec<crate::report::UnknownConstruct>>,
) -> Result<(), ProductDbError> {
    let packages = conn
        .prepare("SELECT sha256 FROM package ORDER BY sha256")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for package in packages {
        let members = conn
            .prepare(
                "SELECT source_sha256 FROM package_member
                 WHERE package_sha256 = ?1 AND role = 'ApplicationProgram'
                 ORDER BY ordinal",
            )?
            .query_map([&package], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let overflow = || ProductDbError::Xml {
            source_path: package.clone(),
            cause: "v21 backfill: unknown counter overflow".into(),
        };
        let added = members.iter().try_fold(0u64, |total, m| {
            let rows = gained.get(m).map_or(0, Vec::len);
            total
                .checked_add(u64::try_from(rows).map_err(|_| overflow())?)
                .ok_or_else(overflow)
        })?;
        let added = i64::try_from(added).map_err(|_| overflow())?;
        conn.execute(
            "UPDATE package SET unknown_count = unknown_count + ?2 WHERE sha256 = ?1",
            params![package, added],
        )?;

        let stale_member = members.iter().find(|m| !gained.contains_key(*m));
        let additions: Vec<_> = members
            .iter()
            .filter_map(|m| gained.get(m))
            .flatten()
            .cloned()
            .collect();
        conn.execute_batch("SAVEPOINT v21_parameter_report;")?;
        let outcome = match stale_member {
            Some(member) => Ok(Err(crate::package::ReportNotUpgradable(format!(
                "member {member} was not carried forward"
            )))),
            None => crate::package::add_report_unknowns(conn, &package, &additions),
        };
        let why = match outcome {
            Ok(Ok(())) => None,
            Ok(Err(why)) => Some(why.0),
            Err(ProductDbError::Sqlite(error)) => return Err(ProductDbError::Sqlite(error)),
            Err(error) => Some(error.to_string()),
        };
        match why {
            None => conn.execute_batch("RELEASE SAVEPOINT v21_parameter_report;")?,
            Some(why) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT v21_parameter_report;
                     RELEASE SAVEPOINT v21_parameter_report;",
                )?;
                crate::package::mark_report_unavailable(conn, &package)?;
                record_backfill_failure(
                    conn,
                    &package,
                    &package,
                    "InstallReportBackfillError",
                    "parameter_attribute_report_backfill",
                    &ProductDbError::Xml {
                        source_path: package.clone(),
                        cause: format!("v21 backfill: {why}"),
                    },
                )?;
            }
        }
    }
    Ok(())
}

/// v19 -> v20 (ADR-0080). `parameter_ref` gains `access`, a new
/// `parameter_calculation_ref` table indexes which refs a
/// `ParameterCalculation` names, and `application_program` gains
/// `write_authority_recorded`, filled from each retained program blob in its
/// own savepoint by the same pass ingest runs.
///
/// A blob that no longer re-reads — bytes that do not match their key, a
/// blob that no longer classifies as a program, or one whose XML fails — is
/// rolled back and recorded as `WriteAuthorityBackfillError`. Its programs
/// keep `write_authority_recorded = 0`, which the server treats as
/// read-only: fail closed, never silently writable. No unknown row and no
/// install report changes: `ParameterRef/@Access` was never reported and the
/// calculation elements stay reported as they were.
fn migrate_v19_to_v20(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "ALTER TABLE parameter_ref ADD COLUMN access TEXT;
         ALTER TABLE application_program
             ADD COLUMN write_authority_recorded INTEGER NOT NULL DEFAULT 0;
         CREATE TABLE parameter_calculation_ref (
             program_id       TEXT NOT NULL,
             calculation_id   TEXT NOT NULL,
             side             TEXT NOT NULL CHECK (side IN ('L', 'R')),
             parameter_ref_id TEXT NOT NULL,
             PRIMARY KEY (program_id, calculation_id, side, parameter_ref_id)
         ) STRICT;
         CREATE INDEX parameter_calculation_ref_member
             ON parameter_calculation_ref (program_id, parameter_ref_id);",
    )?;
    let blobs = conn
        .prepare(
            "SELECT DISTINCT s.sha256, s.source_path FROM source_file s
             JOIN application_program p ON p.source_sha256 = s.sha256
             ORDER BY s.sha256",
        )?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    for (sha256, source_path) in blobs {
        let failure = |cause: &str| ProductDbError::Xml {
            source_path: source_path.clone(),
            cause: format!("v20 backfill: {cause}"),
        };
        let Some(bytes) = crate::load_source_file(conn, &sha256)? else {
            record_backfill_failure(
                conn,
                &sha256,
                &source_path,
                "WriteAuthorityBackfillError",
                "record_write_authority",
                &failure("stored program bytes are missing"),
            )?;
            continue;
        };
        let refused = if crate::sha256_hex(&bytes) != sha256 {
            Some(failure("stored bytes do not match their SHA-256 key"))
        } else if classify(&bytes) != FileKind::ApplicationProgram {
            Some(failure(
                "stored program no longer classifies as an ApplicationProgram",
            ))
        } else {
            None
        };
        if let Some(error) = refused {
            record_backfill_failure(
                conn,
                &sha256,
                &source_path,
                "WriteAuthorityBackfillError",
                "record_write_authority",
                &error,
            )?;
            continue;
        }
        conn.execute_batch("SAVEPOINT v20_write_authority;")?;
        match crate::parse::write_authority::record_write_authority(
            conn,
            &sha256,
            &source_path,
            &bytes,
        ) {
            Ok(()) => conn.execute_batch("RELEASE SAVEPOINT v20_write_authority;")?,
            Err(ProductDbError::Sqlite(error)) => return Err(ProductDbError::Sqlite(error)),
            Err(error) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT v20_write_authority;
                     RELEASE SAVEPOINT v20_write_authority;",
                )?;
                record_backfill_failure(
                    conn,
                    &sha256,
                    &source_path,
                    "WriteAuthorityBackfillError",
                    "record_write_authority",
                    &error,
                )?;
            }
        }
    }
    Ok(())
}

/// Byte-only Languages evidence. Historical install snapshots and normalized
/// master entities remain untouched; explicit rebuild shares the same path.
fn migrate_v18_to_v19(conn: &Connection) -> Result<(), ProductDbError> {
    crate::master_evidence::rederive_master_language_evidence(conn)?;
    Ok(())
}

/// The two `ingest_unknown` xpaths the dynamic pass (`dynamic::parse::
/// insert_node`) reports a `Channel` attribute under. Only that pass writes
/// an unqualified `Number` row there: the `Static` pass skips `Dynamic`
/// whole, and scheme evidence reconciliation only adds targeted names (none
/// on `Channel`) and namespaced ones (`x:Number`, never `Number`).
const RETIRED_CHANNEL_NUMBER_XPATHS: [&str; 2] = [
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Dynamic/Channel",
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/ModuleDefs/ModuleDef/Dynamic/Channel",
];

/// v17 -> v18 (ADR-0052). `dynamic_node` gains `name` and `number`, filled
/// for `Channel` from each stored `ApplicationProgram` blob, and
/// `Channel/@Number` stops being an unknown attribute.
///
/// Per `ApplicationProgram` blob whose bytes still match their key, inside
/// its own savepoint:
/// * the `dynamic_node`/`module_def_argument` rows of the programs it owns
///   are cleared and re-parsed, as `reparse_dynamic_trees` does for v11
///   (`reparse_owned_trees`);
/// * the re-parse's unknown rows are **discarded**. Unlike v11, the stored
///   `…/Dynamic/…` rows are not rewritten from the parse, because since v13
///   scheme evidence reconciliation adds and subtracts rows there that the
///   dynamic pass alone cannot reproduce. Only the retired
///   `Channel/@Number` rows are deleted, for every such blob: the dynamic
///   pass reports an attribute even for a program it does not store;
/// * a blob that no longer re-parses is rolled back to its v17 rows and
///   recorded as `ChannelNameBackfillError`, as is one whose bytes no
///   longer match their key.
///
/// Every measured package report then drops the same retired rows, and
/// `package.unknown_count` the distinct rows its members lost. A report
/// with a member that was not carried forward, or that does not validate,
/// is downgraded to `unavailable` with a recorded
/// `InstallReportBackfillError`, as in v14 and v16.
fn migrate_v17_to_v18(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "ALTER TABLE dynamic_node ADD COLUMN name TEXT;
         ALTER TABLE dynamic_node ADD COLUMN number TEXT;",
    )?;
    let blobs = conn
        .prepare("SELECT sha256, source_path FROM source_file ORDER BY sha256")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    // Blob -> how many distinct retired rows it had: what one install of it
    // added to `package.unknown_count` for them. A blob that is missing
    // from this map was not carried forward, and neither is any report
    // that counts it.
    let mut retired = std::collections::HashMap::new();
    for (sha256, source_path) in blobs {
        let Some(bytes) = crate::load_source_file(conn, &sha256)? else {
            continue;
        };
        let kind = classify(&bytes);
        if kind != FileKind::ApplicationProgram {
            // A damaged blob can stop classifying as a program altogether.
            // Its old package/member or owner row is still evidence that this
            // migration owes it a tree; never silently skip that failure.
            let was_program: bool = conn.query_row(
                "SELECT EXISTS(SELECT 1 FROM application_program WHERE source_sha256 = ?1)
                     OR EXISTS(SELECT 1 FROM package_member WHERE source_sha256 = ?1
                               AND role = 'ApplicationProgram')",
                [&sha256],
                |r| r.get(0),
            )?;
            if was_program {
                record_backfill_failure(
                    conn,
                    &sha256,
                    &source_path,
                    "ChannelNameBackfillError",
                    "parse_dynamic_trees",
                    &ProductDbError::Xml {
                        source_path: source_path.clone(),
                        cause: "v18 backfill: stored program no longer classifies as an ApplicationProgram".into(),
                    },
                )?;
            }
            continue;
        }
        if crate::sha256_hex(&bytes) != sha256 {
            record_backfill_failure(
                conn,
                &sha256,
                &source_path,
                "ChannelNameBackfillError",
                "parse_dynamic_trees",
                &ProductDbError::Xml {
                    source_path: source_path.clone(),
                    cause: "v18 backfill: stored bytes do not match their SHA-256 key".into(),
                },
            )?;
            continue;
        }
        conn.execute_batch("SAVEPOINT v18_channel_blob;")?;
        match reparse_owned_trees(conn, &sha256, &source_path, &bytes) {
            Ok(()) => {
                let distinct = retire_channel_number_unknowns(conn, &sha256)?;
                conn.execute_batch("RELEASE SAVEPOINT v18_channel_blob;")?;
                retired.insert(sha256, distinct);
            }
            Err(ProductDbError::Sqlite(error)) => return Err(ProductDbError::Sqlite(error)),
            Err(error) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT v18_channel_blob;
                     RELEASE SAVEPOINT v18_channel_blob;",
                )?;
                record_backfill_failure(
                    conn,
                    &sha256,
                    &source_path,
                    "ChannelNameBackfillError",
                    "parse_dynamic_trees",
                    &error,
                )?;
            }
        }
    }
    retire_channel_number_from_reports(conn, &retired)
}

/// Re-parses the trees of the programs one blob owns. A program another
/// blob owns is skipped by the parser itself (`program_should_be_skipped`),
/// so a blob that lost every id conflict gains no trees. The parse's
/// unknown rows are discarded (`migrate_v17_to_v18`).
fn reparse_owned_trees(
    conn: &Connection,
    sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<(), ProductDbError> {
    clear_program_trees(conn, sha256)?;
    crate::dynamic::parse::parse_dynamic_trees(conn, sha256, source_path, bytes)?;
    Ok(())
}

/// Deletes the `dynamic_node` and `module_def_argument` rows of the
/// programs one blob owns, so `parse_dynamic_trees` writes them again
/// instead of skipping a program that already has rows. The blob's
/// `ingest_unknown` rows are left alone (`migrate_v17_to_v18`).
fn clear_program_trees(conn: &Connection, sha256: &str) -> Result<(), ProductDbError> {
    const OWNED_PROGRAMS: &str = "SELECT id FROM application_program WHERE source_sha256 = ?1";
    conn.execute(
        &format!("DELETE FROM dynamic_node WHERE program_id IN ({OWNED_PROGRAMS})"),
        [sha256],
    )?;
    conn.execute(
        &format!("DELETE FROM module_def_argument WHERE program_id IN ({OWNED_PROGRAMS})"),
        [sha256],
    )?;
    Ok(())
}

/// Deletes one blob's retired `Channel/@Number` rows and returns how many
/// of the retired xpaths had any. `ingest_unknown` has no unique key and a
/// package install re-records a blob it re-parses, so the rows themselves
/// may be duplicated; the distinct xpaths are what one parse reported.
fn retire_channel_number_unknowns(conn: &Connection, sha256: &str) -> Result<u64, ProductDbError> {
    let mut distinct = 0;
    for xpath in RETIRED_CHANNEL_NUMBER_XPATHS {
        let deleted = conn.execute(
            "DELETE FROM ingest_unknown
             WHERE source_sha256 = ?1 AND xpath = ?2 AND kind = 'Attribute' AND name = 'Number'",
            params![sha256, xpath],
        )?;
        if deleted > 0 {
            distinct += 1;
        }
    }
    Ok(distinct)
}

/// The package half of `migrate_v17_to_v18`. `package.unknown_count`
/// loses, per `ApplicationProgram` member, the distinct rows that member
/// had retired, exactly what one install of it added. The measured report
/// is then rewritten without the retired rows, in its own savepoint. If a
/// member was not carried forward (missing, damaged or unparseable), or the
/// rewritten report does not validate, the report is downgraded to
/// `unavailable` instead.
fn retire_channel_number_from_reports(
    conn: &Connection,
    retired: &std::collections::HashMap<String, u64>,
) -> Result<(), ProductDbError> {
    let packages = conn
        .prepare("SELECT sha256 FROM package ORDER BY sha256")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for package in packages {
        let members = conn
            .prepare(
                "SELECT source_sha256 FROM package_member
                 WHERE package_sha256 = ?1 AND role = 'ApplicationProgram'
                 ORDER BY ordinal",
            )?
            .query_map([&package], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        let lost = members.iter().try_fold(0u64, |total, m| {
            total
                .checked_add(retired.get(m).copied().unwrap_or(0))
                .ok_or_else(|| ProductDbError::Xml {
                    source_path: package.clone(),
                    cause: "v18 backfill: unknown counter overflow".into(),
                })
        })?;
        let lost = i64::try_from(lost).map_err(|_| ProductDbError::Xml {
            source_path: package.clone(),
            cause: "v18 backfill: unknown counter overflow".into(),
        })?;
        let updated = conn.execute(
            "UPDATE package SET unknown_count = unknown_count - ?2
             WHERE sha256 = ?1 AND unknown_count >= ?2",
            params![package, lost],
        )?;
        if updated != 1 {
            return Err(ProductDbError::Xml {
                source_path: package.clone(),
                cause: "v18 backfill: package unknown counter would underflow".into(),
            });
        }

        let stale_member = members.iter().find(|m| !retired.contains_key(*m));
        conn.execute_batch("SAVEPOINT v18_channel_report;")?;
        let outcome = match stale_member {
            Some(member) => Ok(Err(crate::package::ReportNotUpgradable(format!(
                "member {member} was not carried forward"
            )))),
            None => crate::package::retire_report_unknowns(
                conn,
                &package,
                &RETIRED_CHANNEL_NUMBER_XPATHS,
                "Number",
            ),
        };
        let why = match outcome {
            Ok(Ok(())) => None,
            Ok(Err(why)) => Some(why.0),
            Err(ProductDbError::Sqlite(error)) => return Err(ProductDbError::Sqlite(error)),
            Err(error) => Some(error.to_string()),
        };
        match why {
            None => conn.execute_batch("RELEASE SAVEPOINT v18_channel_report;")?,
            Some(why) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT v18_channel_report;
                     RELEASE SAVEPOINT v18_channel_report;",
                )?;
                crate::package::mark_report_unavailable(conn, &package)?;
                record_backfill_failure(
                    conn,
                    &package,
                    &package,
                    "InstallReportBackfillError",
                    "channel_number_report_backfill",
                    &ProductDbError::Xml {
                        source_path: package.clone(),
                        cause: format!("v18 backfill: {why}"),
                    },
                )?;
            }
        }
    }
    Ok(())
}

/// v16 -> v17 (PDB-11, ADR-0043). Adds the package identity tables.
///
/// The backfill seeds `package_source_name` from `package.source_name`,
/// scans every parsed blob that classifies as a catalogue, hardware or
/// program file (exactly the blobs a domain parser read, so a raw-stored
/// member never becomes a candidate), and extracts producer facts from
/// every stored blob. A blob whose stored bytes no longer match its key,
/// that the scan cannot read, or whose historical rows disagree with its
/// scan is recorded `unavailable` with the reason and the upgrade
/// continues. It also indexes the six identity tables by `source_sha256`
/// for the agreement check. Winners and `package_conflict` rows stay as they
/// were: the original install order is not recoverable.
fn migrate_v16_to_v17(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE package_source_name (
            package_sha256 TEXT NOT NULL REFERENCES package(sha256),
            source_name    TEXT NOT NULL,
            PRIMARY KEY (package_sha256, source_name)
        ) STRICT;
        CREATE TABLE source_identity_scan (
            source_sha256 TEXT PRIMARY KEY REFERENCES source_file(sha256),
            status        TEXT NOT NULL CHECK (status IN ('measured','unavailable')),
            reason        TEXT,
            scanner       INTEGER NOT NULL,
            CHECK ((status = 'unavailable') = (reason IS NOT NULL))
        ) STRICT;
        CREATE TABLE source_identity (
            source_sha256 TEXT NOT NULL REFERENCES source_identity_scan(source_sha256),
            table_name    TEXT NOT NULL CHECK (table_name IN ('catalog_section','catalog_item','hardware','product','hardware2program','application_program')),
            logical_id    TEXT NOT NULL,
            occurrence    INTEGER NOT NULL CHECK (occurrence >= 1),
            digest        TEXT NOT NULL CHECK (length(digest) = 64),
            PRIMARY KEY (source_sha256, table_name, logical_id, occurrence)
        ) STRICT;
        CREATE INDEX source_identity_by_id ON source_identity (table_name, logical_id);
        CREATE TABLE source_producer (
            source_sha256  TEXT PRIMARY KEY REFERENCES source_file(sha256),
            root_namespace TEXT,
            created_by     TEXT,
            tool_version   TEXT
        ) STRICT;
        CREATE INDEX IF NOT EXISTS catalog_section_source ON catalog_section (source_sha256);
        CREATE INDEX IF NOT EXISTS catalog_item_source ON catalog_item (source_sha256);
        CREATE INDEX IF NOT EXISTS hardware_source ON hardware (source_sha256);
        CREATE INDEX IF NOT EXISTS product_source ON product (source_sha256);
        CREATE INDEX IF NOT EXISTS hardware2program_source ON hardware2program (source_sha256);
        CREATE INDEX IF NOT EXISTS application_program_source ON application_program (source_sha256);
        INSERT INTO package_source_name (package_sha256, source_name)
            SELECT sha256, source_name FROM package;",
    )?;
    let blobs = conn
        .prepare(
            "SELECT s.sha256, s.source_path, EXISTS (SELECT 1 FROM source_parse_evidence AS e WHERE e.sha256 = s.sha256)
             FROM source_file AS s ORDER BY s.sha256",
        )?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (sha, path, parsed) in blobs {
        let Some(bytes) = crate::load_source_file(conn, &sha)? else {
            continue;
        };
        let intact = crate::sha256_hex(&bytes) == sha;
        if intact {
            crate::identity::record_producer(conn, &sha, &bytes)?;
        }
        if !parsed {
            continue;
        }
        if !intact {
            // Damaged bytes cannot be trusted to classify either: any parsed
            // blob that no longer matches its key is recorded unmeasured.
            crate::identity::record_unavailable(
                conn,
                &sha,
                "v17 backfill: stored bytes do not match their SHA-256 key",
            )?;
        } else if matches!(
            classify(&bytes),
            FileKind::Catalog | FileKind::Hardware | FileKind::ApplicationProgram
        ) {
            crate::identity::backfill_scan(conn, &sha, &path, &bytes)?;
        }
    }
    Ok(())
}

/// v15 -> v16 (PDB-10, ADR-0042). Adds the baggage inventory tables and
/// swaps the `unsupported-baggage-index` diagnostic kind for
/// `unresolved-baggage-declaration` and `undeclared-baggage-payload`
/// (the CHECK list is rebuilt, rows copied verbatim).
///
/// Every installed package then gets an inventory re-derived from its own
/// retained member bytes by install's own functions, never inferred from
/// counts. `ingest_unknown` gets the index parser's unknown rows as a fresh
/// ingest writes them: once per package member (package install re-parses
/// its members, `parse_existing`), and once per parsed index blob that no
/// package carries (standalone `ingest_file`, which parses a blob once).
///
/// A measured install report is rewritten into its v16 shape by
/// `upgrade_report_for_baggage`; an `unavailable` report stays so. A
/// package whose retained bytes no longer parse or whose report disagrees
/// with them gets an `unavailable` inventory and an `unavailable` report,
/// never an invented zero, and a recorded `InstallReportBackfillError`.
fn migrate_v15_to_v16(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE package_baggage_inventory (
            package_sha256 TEXT PRIMARY KEY REFERENCES package(sha256),
            status TEXT NOT NULL CHECK (status IN ('measured','unavailable'))
        ) STRICT;
        CREATE TABLE package_baggage_payload (
            package_sha256 TEXT NOT NULL REFERENCES package_baggage_inventory(package_sha256),
            member_path TEXT NOT NULL,
            sha256 TEXT NOT NULL REFERENCES source_file(sha256),
            size INTEGER NOT NULL CHECK (size >= 0),
            media_class TEXT NOT NULL CHECK (media_class IN ('empty','png','jpeg','gif','bmp','pdf','zip','pe-executable','ole2-compound','xml','unknown')),
            declarations INTEGER NOT NULL CHECK (declarations >= 0),
            nested_status TEXT NOT NULL CHECK (nested_status IN ('not-archive','unreadable','read')),
            nested_entries INTEGER CHECK (nested_entries >= 0),
            nested_expanded_size INTEGER CHECK (nested_expanded_size >= 0),
            nested_encrypted_entries INTEGER CHECK (nested_encrypted_entries >= 0),
            nested_archive_names INTEGER CHECK (nested_archive_names >= 0),
            PRIMARY KEY (package_sha256, member_path),
            CHECK ((nested_status = 'read') = (nested_entries IS NOT NULL)),
            CHECK ((nested_entries IS NULL) = (nested_expanded_size IS NULL)),
            CHECK ((nested_entries IS NULL) = (nested_encrypted_entries IS NULL)),
            CHECK ((nested_entries IS NULL) = (nested_archive_names IS NULL)),
            CHECK ((media_class = 'zip') = (nested_status <> 'not-archive'))
        ) STRICT;
        CREATE TABLE package_baggage_declaration (
            package_sha256 TEXT NOT NULL REFERENCES package_baggage_inventory(package_sha256),
            index_path TEXT NOT NULL,
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            baggage_id TEXT,
            name TEXT,
            target_path TEXT,
            install_on_import TEXT,
            time_info TEXT,
            file_version TEXT,
            resolution TEXT NOT NULL CHECK (resolution IN ('resolved','missing','invalid')),
            member_path TEXT,
            detail TEXT,
            PRIMARY KEY (package_sha256, index_path, ordinal),
            CHECK ((resolution = 'resolved') = (member_path IS NOT NULL)),
            CHECK ((resolution = 'resolved') = (detail IS NULL))
        ) STRICT;
        CREATE TABLE package_install_diagnostic_v16 (
            package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            kind TEXT NOT NULL CHECK (kind IN ('unsupported-master-section','unsupported-master-subtree','unresolved-baggage-declaration','undeclared-baggage-payload')),
            archive_path TEXT NOT NULL,
            xml_path TEXT NOT NULL,
            detail TEXT NOT NULL,
            occurrences INTEGER NOT NULL CHECK (occurrences > 0),
            PRIMARY KEY (package_sha256, ordinal),
            UNIQUE (package_sha256, kind, archive_path, xml_path, detail)
        ) STRICT;
        INSERT INTO package_install_diagnostic_v16
            SELECT * FROM package_install_diagnostic WHERE kind <> 'unsupported-baggage-index';
        DROP TABLE package_install_diagnostic;
        ALTER TABLE package_install_diagnostic_v16 RENAME TO package_install_diagnostic;",
    )?;
    // Parsed blobs outside every package: standalone ingests. Only these
    // need classifying; package members carry their role.
    let standalone = conn
        .prepare(
            "SELECT s.sha256, s.source_path FROM source_file AS s
             JOIN source_parse_evidence AS e ON e.sha256 = s.sha256
             WHERE NOT EXISTS (SELECT 1 FROM package_member AS m WHERE m.source_sha256 = s.sha256)
             ORDER BY s.sha256",
        )?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    for (sha, path) in standalone {
        let Some(bytes) = crate::load_source_file(conn, &sha)? else {
            continue;
        };
        if classify(&bytes) != FileKind::Baggages {
            continue;
        }
        match crate::parse::baggage::parse_baggage_index(&path, &bytes) {
            Ok(index) => insert_unknown(conn, &sha, &index.unknown)?,
            Err(error @ ProductDbError::Xml { .. }) => {
                crate::ingest::record_unreadable_baggage_index(conn, &sha, &path, &error)?
            }
            Err(error) => return Err(error),
        }
    }
    let packages = conn
        .prepare("SELECT sha256 FROM package ORDER BY sha256")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for package in packages {
        conn.execute_batch("SAVEPOINT v16_baggage")?;
        let failure = match measure_baggage(conn, &package)? {
            Err(failure) => Some(failure),
            Ok((inventory, unknowns)) => {
                // A fresh install adds each re-parsed member's distinct
                // unknown rows to `package.unknown_count` (`InstallReport::
                // unknown`); v15 parsed indexes without reporting any.
                let mut added: i64 = 0;
                for (sha, rows) in &unknowns {
                    insert_unknown(conn, sha, rows)?;
                    added = i64::try_from(rows.len())
                        .ok()
                        .and_then(|n| added.checked_add(n))
                        .ok_or_else(|| ProductDbError::Xml {
                            source_path: package.clone(),
                            cause: "v16 backfill: unknown counter overflow".into(),
                        })?;
                }
                conn.execute(
                    "UPDATE package SET unknown_count = unknown_count + ?2 WHERE sha256 = ?1",
                    rusqlite::params![package, added],
                )?;
                let unknowns = unknowns.into_iter().flat_map(|(_, rows)| rows).collect();
                crate::baggage::persist(conn, &package, &inventory)?;
                // A report that cannot be read or re-validated downgrades
                // just this package to `unavailable`, as v13 -> v14 does; only
                // a database failure aborts the migration.
                match crate::package::upgrade_report_for_baggage(
                    conn, &package, &inventory, unknowns,
                ) {
                    Ok(Ok(())) => None,
                    Ok(Err(why)) => Some((
                        package.clone(),
                        package.clone(),
                        ProductDbError::Xml {
                            source_path: package.clone(),
                            cause: format!("v16 backfill: {}", why.0),
                        },
                    )),
                    Err(ProductDbError::Sqlite(error)) => {
                        return Err(ProductDbError::Sqlite(error))
                    }
                    Err(error) => Some((package.clone(), package.clone(), error)),
                }
            }
        };
        match failure {
            None => conn.execute_batch("RELEASE v16_baggage")?,
            Some((source_sha, path, error)) => {
                conn.execute_batch("ROLLBACK TO v16_baggage; RELEASE v16_baggage")?;
                crate::baggage::persist_unavailable(conn, &package)?;
                // A package with no report row (only a damaged v15 database)
                // has nothing to downgrade; that must not stop it opening.
                crate::package::mark_report_unavailable(conn, &package)?;
                record_backfill_failure(
                    conn,
                    &source_sha,
                    &path,
                    "InstallReportBackfillError",
                    "baggage_inventory_backfill",
                    &error,
                )?;
                // The rollback also took the index unknowns, but they describe
                // each index blob, not this package's report: keep those of
                // every index that still parses. The report stays
                // `unavailable`, so `package.unknown_count` is not raised.
                for (sha, rows) in parseable_index_unknowns(conn, &package)? {
                    insert_unknown(conn, &sha, &rows)?;
                }
            }
        }
    }
    Ok(())
}

/// The unknowns of each of `package`'s retained `Baggages.xml` members that
/// is present, matches its hash and parses. Anything else was already
/// recorded as the package's backfill failure.
fn parseable_index_unknowns(
    conn: &Connection,
    package: &str,
) -> Result<Vec<(String, Vec<crate::report::UnknownConstruct>)>, ProductDbError> {
    let members = conn
        .prepare(
            "SELECT path, source_sha256 FROM package_member
             WHERE package_sha256 = ?1 AND role = 'Baggages' ORDER BY ordinal",
        )?
        .query_map([package], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::new();
    for (path, sha) in members {
        let Some(bytes) = crate::load_source_file(conn, &sha)? else {
            continue;
        };
        if crate::sha256_hex(&bytes) != sha {
            continue;
        }
        match crate::baggage::parse_index(&path, &bytes) {
            Ok((_, unknown)) => out.push((sha, unknown)),
            Err(ProductDbError::Sqlite(error)) => return Err(ProductDbError::Sqlite(error)),
            Err(_) => {}
        }
    }
    Ok(out)
}

/// The inventory plus each index member's unknowns, keyed by blob hash.
type MeasuredBaggage = (
    crate::baggage::BaggageInventory,
    Vec<(String, Vec<crate::report::UnknownConstruct>)>,
);

/// Re-derives one package's inventory, and the index-parser unknowns its
/// install report merges, from retained member bytes. The outer `Result` is
/// a database failure; the inner `Err` names the blob that cannot be read.
fn measure_baggage(
    conn: &Connection,
    package: &str,
) -> Result<Result<MeasuredBaggage, UnmeasurableMaster>, ProductDbError> {
    let members = conn
        .prepare(
            "SELECT path, role, source_sha256 FROM package_member
             WHERE package_sha256 = ?1 AND role IN ('Baggages', 'Baggage') ORDER BY ordinal",
        )?
        .query_map([package], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut indexes = Vec::new();
    let mut payloads = Vec::new();
    let mut unknowns = Vec::new();
    for (path, role, sha) in members {
        let Some(bytes) = crate::load_source_file(conn, &sha)? else {
            let error = ProductDbError::Xml {
                source_path: path.clone(),
                cause: "v16 backfill: retained baggage member is missing".into(),
            };
            return Ok(Err((sha, path, error)));
        };
        if crate::sha256_hex(&bytes) != sha {
            let error = ProductDbError::Xml {
                source_path: path.clone(),
                cause: "v16 backfill: retained baggage member does not match its hash".into(),
            };
            return Ok(Err((sha, path, error)));
        }
        if role == "Baggage" {
            payloads.push(crate::baggage::BaggagePayload::measure(path, sha, &bytes));
            continue;
        }
        match crate::baggage::parse_index(&path, &bytes) {
            Ok((index, unknown)) => {
                unknowns.push((sha, unknown));
                indexes.push(index);
            }
            Err(error) => return Ok(Err((sha, path, error))),
        }
    }
    Ok(Ok((
        crate::baggage::BaggageInventory::resolve(indexes, payloads),
        unknowns,
    )))
}

/// v14 -> v15 (PDB-9). `TypeColor` and `TypeTime` get their own
/// `parameter_type.kind` (`Color`, `Time`) instead of `Other`; no DDL is
/// needed because `kind` has no CHECK list. Every blob that still carries a
/// v14-era unknown-`Element` row for either child is re-read with
/// `parse::program::backfill_color_time_kinds` (ADR-0020), which re-kinds
/// the winning rows and swaps the stale element row for the attribute rows
/// a fresh ingest writes. Package install reports are historical encounter
/// records and are left as measured, the v12 -> v13 precedent. Per-blob
/// `SAVEPOINT`; a failure is recorded, not fatal — a database that refuses
/// to open is worse than one with a named gap.
fn migrate_v14_to_v15(conn: &Connection) -> Result<(), ProductDbError> {
    // `scheme21`: whether ingest ran this blob through the scheme-21
    // package reconciliation (it does for an application program installed
    // as a member of a scheme-21 package, ingest.rs), so the backfill
    // reconciles its new attribute rows the same way.
    let sources = conn
        .prepare(
            "SELECT DISTINCT u.source_sha256, s.source_path,
                    EXISTS (SELECT 1 FROM package_member AS m JOIN package AS p
                            ON p.sha256 = m.package_sha256
                            WHERE m.source_sha256 = u.source_sha256 AND p.scheme = 21)
             FROM ingest_unknown AS u JOIN source_file AS s ON s.sha256 = u.source_sha256
             WHERE u.kind = 'Element' AND u.name IN ('TypeColor', 'TypeTime')
             ORDER BY u.source_sha256",
        )?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, bool>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (sha, path, scheme21) in sources {
        let bytes: Vec<u8> = conn.query_row(
            "SELECT bytes FROM source_file WHERE sha256 = ?1",
            [&sha],
            |r| r.get(0),
        )?;
        conn.execute_batch("SAVEPOINT color_time_backfill")?;
        match crate::parse::program::backfill_color_time_kinds(conn, &sha, &path, &bytes, scheme21)
        {
            Ok(_) => conn.execute_batch("RELEASE color_time_backfill")?,
            Err(error) => {
                conn.execute_batch("ROLLBACK TO color_time_backfill; RELEASE color_time_backfill")?;
                record_backfill_failure(
                    conn,
                    &sha,
                    &path,
                    "ParameterKindBackfillError",
                    "backfill_color_time_kinds",
                    &error,
                )?;
            }
        }
    }
    Ok(())
}

/// v13 -> v14 (PDB-8). Adds the `master_subtree` count category and the
/// `unsupported-master-subtree` diagnostic kind. SQLite cannot widen a
/// CHECK in place, so both tables are rebuilt with identical columns and
/// their rows copied verbatim.
///
/// Every *measured* report then needs its new required row. It is measured,
/// not assumed: the package's own retained `knx_master.xml` bytes are
/// re-scanned with the same pure function install uses
/// (`uninterpreted_master_subtrees`), and a package without a Master member
/// honestly measures zero. `unavailable` reports stay unavailable.
///
/// A retained master that no longer scans (bit rot, tampering) cannot be
/// measured, so that one package's report is downgraded to `unavailable` —
/// the same honest "not measured" marker pre-v12 packages carry, never an
/// invented zero — and the failure is recorded through
/// `record_backfill_failure`, as v12 -> v13 does. The database stays
/// openable; reinstalling the package bytes measures it again.
fn migrate_v13_to_v14(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE package_install_count_v14 (
            package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            category TEXT NOT NULL CHECK (category IN ('archive_member','product','application_program','parameter','communication_object','dynamic_node','module','baggage_index','baggage','unknown_construct','master_section','master_subtree','datapoint_type')),
            disposition TEXT NOT NULL CHECK (disposition IN ('read','stored','deduplicated','retained-but-uninterpreted','unsupported','dropped')),
            count INTEGER NOT NULL CHECK (count >= 0),
            PRIMARY KEY (package_sha256, ordinal),
            UNIQUE (package_sha256, category, disposition)
        ) STRICT;
        INSERT INTO package_install_count_v14 SELECT * FROM package_install_count;
        DROP TABLE package_install_count;
        ALTER TABLE package_install_count_v14 RENAME TO package_install_count;
        CREATE TABLE package_install_diagnostic_v14 (
            package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            kind TEXT NOT NULL CHECK (kind IN ('unsupported-master-section','unsupported-master-subtree','unsupported-baggage-index')),
            archive_path TEXT NOT NULL,
            xml_path TEXT NOT NULL,
            detail TEXT NOT NULL,
            occurrences INTEGER NOT NULL CHECK (occurrences > 0),
            PRIMARY KEY (package_sha256, ordinal),
            UNIQUE (package_sha256, kind, archive_path, xml_path, detail)
        ) STRICT;
        INSERT INTO package_install_diagnostic_v14 SELECT * FROM package_install_diagnostic;
        DROP TABLE package_install_diagnostic;
        ALTER TABLE package_install_diagnostic_v14 RENAME TO package_install_diagnostic;",
    )?;
    let measured = conn
        .prepare(
            "SELECT package_sha256 FROM package_install_report
             WHERE status = 'measured' ORDER BY package_sha256",
        )?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for package in measured {
        match measure_master_subtrees(conn, &package)? {
            Ok((total, diagnostics)) => {
                persist_master_subtrees(conn, &package, total, diagnostics)?
            }
            Err((source_sha, path, error)) => {
                conn.execute_batch("SAVEPOINT v14_unavailable")?;
                for table in [
                    "package_install_count",
                    "package_install_unknown",
                    "package_install_diagnostic",
                ] {
                    conn.execute(
                        &format!("DELETE FROM {table} WHERE package_sha256 = ?1"),
                        [&package],
                    )?;
                }
                conn.execute(
                    "UPDATE package_install_report
                     SET status = 'unavailable', unknown_distinct = 0, unknown_occurrences = 0
                     WHERE package_sha256 = ?1",
                    [&package],
                )?;
                record_backfill_failure(
                    conn,
                    &source_sha,
                    &path,
                    "InstallReportBackfillError",
                    "master_subtree_backfill",
                    &error,
                )?;
                conn.execute_batch("RELEASE v14_unavailable")?;
            }
        }
    }
    Ok(())
}

type MeasuredSubtrees = (u64, Vec<crate::package::InstallDiagnostic>);
type UnmeasurableMaster = (String, String, ProductDbError);

/// Scans every Master member of `package`. The outer `Result` is a database
/// failure (aborts the migration); the inner `Err` names a master whose
/// retained bytes cannot be measured (downgrades just this report).
fn measure_master_subtrees(
    conn: &Connection,
    package: &str,
) -> Result<Result<MeasuredSubtrees, UnmeasurableMaster>, ProductDbError> {
    let masters = conn
        .prepare(
            "SELECT path, source_sha256 FROM package_member
             WHERE package_sha256 = ?1 AND role = 'Master' ORDER BY path",
        )?
        .query_map([package], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut total = 0u64;
    let mut diagnostics = Vec::new();
    for (path, source_sha) in masters {
        let Some(bytes) = crate::load_source_file(conn, &source_sha)? else {
            let error = ProductDbError::Xml {
                source_path: path.clone(),
                cause: "v14 backfill: retained master blob is missing".into(),
            };
            return Ok(Err((source_sha, path, error)));
        };
        let subtrees = match crate::parse::master::uninterpreted_master_subtrees(&bytes) {
            Ok(subtrees) => subtrees,
            Err(error) => return Ok(Err((source_sha, path, error))),
        };
        for subtree in subtrees {
            total = total
                .checked_add(subtree.occurrences)
                .ok_or_else(|| ProductDbError::Xml {
                    source_path: path.clone(),
                    cause: "v14 backfill: master-subtree counter overflow".into(),
                })?;
            diagnostics.push(crate::package::master_subtree_diagnostic(
                path.clone(),
                &subtree,
            )?);
        }
    }
    Ok(Ok((total, diagnostics)))
}

fn persist_master_subtrees(
    conn: &Connection,
    package: &str,
    total: u64,
    diagnostics: Vec<crate::package::InstallDiagnostic>,
) -> Result<(), ProductDbError> {
    let too_large = |what: &str| ProductDbError::Xml {
        source_path: package.to_string(),
        cause: format!("v14 backfill: {what} exceeds SQLite INTEGER"),
    };
    let count_ordinal: i64 = conn.query_row(
        "SELECT coalesce(max(ordinal) + 1, 0) FROM package_install_count WHERE package_sha256 = ?1",
        [package],
        |r| r.get(0),
    )?;
    conn.execute(
        "INSERT INTO package_install_count VALUES (?1, ?2, 'master_subtree', 'unsupported', ?3)",
        params![
            package,
            count_ordinal,
            i64::try_from(total).map_err(|_| too_large("master-subtree count"))?
        ],
    )?;
    let first_ordinal: i64 = conn.query_row(
        "SELECT coalesce(max(ordinal) + 1, 0) FROM package_install_diagnostic WHERE package_sha256 = ?1",
        [package],
        |r| r.get(0),
    )?;
    for (ordinal, row) in (first_ordinal..).zip(diagnostics) {
        conn.execute(
            "INSERT INTO package_install_diagnostic VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                package,
                ordinal,
                row.kind().as_str(),
                row.archive_path(),
                row.xml_path(),
                row.detail(),
                i64::try_from(row.occurrences()).map_err(|_| too_large("occurrences"))?
            ],
        )?;
    }
    Ok(())
}

/// v12 -> v13. These nullable columns hold verbatim catalogue lexemes;
/// migration of old source bytes is handled below, never inferred from
/// product IDs or package-level installation totals.
fn migrate_v12_to_v13(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "ALTER TABLE application_program ADD COLUMN is_secure_enabled TEXT;
         ALTER TABLE application_program ADD COLUMN max_security_group_key_table_entries TEXT;
         ALTER TABLE application_program ADD COLUMN max_security_individual_address_entries TEXT;
         ALTER TABLE application_program ADD COLUMN max_security_p2p_key_table_entries TEXT;
         ALTER TABLE application_program ADD COLUMN max_tunneling_user_entries TEXT;
         ALTER TABLE application_program ADD COLUMN max_user_entries TEXT;
         ALTER TABLE application_program ADD COLUMN min_ets_version TEXT;
         ALTER TABLE application_program ADD COLUMN replaces_versions TEXT;",
    )?;
    let mut stmt = conn.prepare(
        "SELECT sha256, source_path FROM source_file AS s
         WHERE EXISTS (SELECT 1 FROM application_program WHERE source_sha256 = s.sha256)
         ORDER BY sha256",
    )?;
    let sources = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);
    // Collect only identities, never all blobs. Release the read cursor before
    // savepoint rollback: SQLite aborts an active iterator on ROLLBACK TO.
    for (sha, path) in sources {
        let bytes: Vec<u8> = conn.query_row(
            "SELECT bytes FROM source_file WHERE sha256 = ?1",
            [&sha],
            |r| r.get(0),
        )?;
        conn.execute_batch("SAVEPOINT catalog_metadata_backfill")?;
        match crate::parse::program::backfill_catalog_metadata(conn, &sha, &path, &bytes) {
            Ok(_) => conn.execute_batch("RELEASE catalog_metadata_backfill")?,
            Err(error) => {
                conn.execute_batch(
                    "ROLLBACK TO catalog_metadata_backfill; RELEASE catalog_metadata_backfill",
                )?;
                record_backfill_failure(
                    conn,
                    &sha,
                    &path,
                    "CatalogMetadataBackfillError",
                    "backfill_catalog_metadata",
                    &error,
                )?;
            }
        }
    }
    Ok(())
}

/// v11 -> v12. Install reports are deliberately separate normalized evidence,
/// rather than counters inferred from final catalog totals. Existing packages
/// have no encounter/write ledger, so they receive explicit `unavailable`
/// markers instead of fabricated measured zeroes. These names cannot exist in
/// a legitimate v11 schema, so any collision is an error and the outer
/// migration transaction rolls the whole step back.
fn migrate_v11_to_v12(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE package_install_report (
            package_sha256 TEXT PRIMARY KEY REFERENCES package(sha256),
            report_version INTEGER NOT NULL CHECK (report_version = 1),
            status TEXT NOT NULL CHECK (status IN ('measured','unavailable')),
            unknown_distinct INTEGER NOT NULL CHECK (unknown_distinct >= 0),
            unknown_occurrences INTEGER NOT NULL CHECK (unknown_occurrences >= 0),
            CHECK (status = 'measured' OR (unknown_distinct = 0 AND unknown_occurrences = 0))
        ) STRICT;
        CREATE TABLE package_install_count (
            package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            category TEXT NOT NULL CHECK (category IN ('archive_member','product','application_program','parameter','communication_object','dynamic_node','module','baggage_index','baggage','unknown_construct','master_section','datapoint_type')),
            disposition TEXT NOT NULL CHECK (disposition IN ('read','stored','deduplicated','retained-but-uninterpreted','unsupported','dropped')),
            count INTEGER NOT NULL CHECK (count >= 0),
            PRIMARY KEY (package_sha256, ordinal),
            UNIQUE (package_sha256, category, disposition)
        ) STRICT;
        CREATE TABLE package_install_unknown (
            package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            xpath TEXT NOT NULL,
            kind TEXT NOT NULL CHECK (kind IN ('Element','Attribute')),
            name TEXT NOT NULL,
            occurrences INTEGER NOT NULL CHECK (occurrences > 0),
            sample TEXT,
            PRIMARY KEY (package_sha256, ordinal),
            UNIQUE (package_sha256, xpath, kind, name)
        ) STRICT;
        CREATE TABLE package_install_diagnostic (
            package_sha256 TEXT NOT NULL REFERENCES package_install_report(package_sha256),
            ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
            kind TEXT NOT NULL CHECK (kind IN ('unsupported-master-section','unsupported-baggage-index')),
            archive_path TEXT NOT NULL,
            xml_path TEXT NOT NULL,
            detail TEXT NOT NULL,
            occurrences INTEGER NOT NULL CHECK (occurrences > 0),
            PRIMARY KEY (package_sha256, ordinal),
            UNIQUE (package_sha256, kind, archive_path, xml_path, detail)
        ) STRICT;
        INSERT INTO package_install_report
            (package_sha256, report_version, status, unknown_distinct, unknown_occurrences)
            SELECT sha256, 1, 'unavailable', 0, 0 FROM package;",
    )?;
    Ok(())
}

fn migrate_v9_to_v10(conn: &Connection) -> Result<(), ProductDbError> {
    // `IF NOT EXISTS` throughout: this migration, uniquely among the ones in
    // this file, is exercised by tests that roll a fully-migrated database's
    // `user_version` pragma back below 10 without dropping the tables a first
    // pass through this same file already created — an unguarded `CREATE
    // TABLE` would then fail on the physically-still-there table. Every
    // other structural migration here (`migrate_v2_to_v3`'s `dynamic_node`,
    // `migrate_v1_to_v2`'s `package`) never had to survive that, because no
    // later migration reintroduced their tables' names; this one does.
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS function_type (
             id     TEXT PRIMARY KEY,
             number INTEGER,
             text   TEXT,
             status TEXT
         ) STRICT;
         CREATE TABLE IF NOT EXISTS function_point (
             id               TEXT PRIMARY KEY,
             function_type_id TEXT NOT NULL,
             datapoint_type   TEXT,
             role             TEXT,
             characteristics  TEXT,
             text             TEXT
         ) STRICT;
         CREATE INDEX IF NOT EXISTS function_point_function_type ON function_point (function_type_id);
         CREATE TABLE IF NOT EXISTS space_usage (
             id     TEXT PRIMARY KEY,
             number INTEGER,
             text   TEXT
         ) STRICT;",
    )?;
    backfill_function_and_space_data(conn)?;
    Ok(())
}

/// A product database that reached v9 before `function_type`/
/// `function_point`/`space_usage` existed has `source_file` blobs whose
/// `knx_master.xml` was already parsed for `Manufacturers`/`DatapointTypes`/
/// `Languages` but never for `FunctionTypes`/`SpaceUsages` — installation's
/// content-hash idempotence (`source_parse_evidence`) means an
/// already-installed blob is never revisited by the ordinary path.
/// Modelled on `backfill_shared_translations` above: every blob that
/// classifies as `MasterData` is replayed through `ingest_master_data` in
/// full, inside the same migration transaction `open_and_migrate` already
/// holds. Replaying the whole function rather than a second, narrower
/// parser is safe because every write it makes is `INSERT OR IGNORE` or
/// `ON CONFLICT DO UPDATE SET name = excluded.name` against a blob's own
/// unchanged bytes — that reproduces first ingest's result whenever no
/// master blob was installed by more than one package, and only the three
/// new tables actually gain anything. That qualifier is not automatic and
/// is not decoration: see the `ORDER BY rowid` comment below for the
/// `store_source_file` short-circuit that makes install order and
/// `source_file` rowid order two different things, and for the measured
/// case where it matters.
///
/// That argument covers `ingest_master_data`'s own writes and stops there.
/// The one write this function makes on its own behalf — `insert_unknown` —
/// is a plain `INSERT` into a table with no unique key, and the collector it
/// would be handed is file-wide, so replaying it whole would duplicate every
/// unknown construct `install_package` already recorded for this blob. It is
/// therefore filtered to the two xpath prefixes this migration's three new
/// element families live under; see the comment at the call site.
fn backfill_function_and_space_data(conn: &Connection) -> Result<(), ProductDbError> {
    // `ORDER BY rowid`: `Manufacturer`'s `ON CONFLICT DO UPDATE SET name =
    // excluded.name` inside the replayed `ingest_master_data` is genuinely
    // last-writer-wins, and the corpus's own master files disagree about a
    // manufacturer's display name often enough to matter — 36 manufacturer
    // ids disagree across the corpus's five master files, 132 disagreeing
    // file pairs in total, measured (KNOWN_LIMITATIONS.md §88); `M-0052` is
    // `Theodor HEIMEIER Metallwerk` in one master and `IMI Hydronic
    // Engineering` in another. Without an explicit order this table scan
    // happens to come back in insertion order today, which is the only
    // reason replaying it reproduces the original install's result — true
    // by accident, not by anything SQLite promises. `rowid` pins it to
    // what `source_file` actually promises: the order distinct blobs were
    // first written, by construction (ADR-0020 §E2 — this value depends on
    // install history, not only on stored bytes, so the migration may not
    // re-derive a *different* one).
    //
    // "The order distinct blobs were first written" is *not* the same
    // thing as "install order", and that gap is real, not theoretical.
    // `store_source_file` (`blob.rs`) returns `false` without inserting a
    // row when a blob's sha256 is already on record, but `install_package`
    // (`package.rs`) calls `ingest_master_data` on that blob regardless —
    // so a master blob installed by two different packages is genuinely
    // ingested twice, at two different points in real install history,
    // while `source_file` only ever gives it one rowid. This scan then
    // replays that blob once, at the position its *first* install
    // occupies, which can differ from the position its *last* install
    // (the one whose `Manufacturer` names actually won, under last-writer-
    // wins) occupied. Measured [V]: install Weinzierl 730 ETS4, then MDT KP
    // AMI/AMS 03, then the Weinzierl archive repacked with one XML comment
    // appended to `M-00C5/Catalog.xml` (package hash differs,
    // `knx_master.xml` byte-identical to the first install — so the third
    // install's `ingest_master_data` call replays the *same* manufacturer
    // rows a second time, at a rowid that still reflects only the first
    // install). Roll back to `user_version = 9`, drop the three v10 tables,
    // reopen through `open_and_migrate`: 21 of 799 manufacturer display
    // names change relative to the pre-rollback database. `M-0002` goes
    // from `ABB` to `ABB AG - STOTZ-KONTAKT`, `M-0007` from `Busch-Jaeger
    // Elektro` to `ABB AG - BUSCH-JAEGER`, `M-000A` from `INSTA ELEKTRO` to
    // `Insta GmbH`; `translation`, `datapoint_type` and `ingest_unknown`
    // counts are unchanged. This flip happens with or without `ORDER BY
    // rowid` — the ordering is still the right one to hold, since it is
    // still the *only* order `source_file` actually records — but it does
    // mean this function's result is not guaranteed to equal first
    // install's result in every database, only in the (typical) one where
    // no master blob was ever installed by more than one package.
    // KNOWN_LIMITATIONS.md §88 records the residual beside its existing
    // first-winner/last-winner pair.
    let mut stmt =
        conn.prepare("SELECT sha256, source_path, bytes FROM source_file ORDER BY rowid")?;
    let blobs: Vec<(String, String, Vec<u8>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    for (sha256, source_path, bytes) in blobs {
        // Only `MasterData` content can carry `FunctionTypes`/`SpaceUsages`;
        // everything else is skipped without being parsed at all, exactly
        // as the ordinary ingest path already dispatches by `classify`.
        if classify(&bytes) != FileKind::MasterData {
            continue;
        }
        conn.execute_batch("SAVEPOINT function_space_backfill_blob;")?;
        match crate::parse::master::ingest_master_data(conn, &bytes) {
            Ok(outcome) => {
                conn.execute_batch("RELEASE SAVEPOINT function_space_backfill_blob;")?;
                // Only the unknowns belonging to the three element families
                // this migration newly parses. `ingest_master_data`'s
                // collector is file-wide, and every blob reaching this
                // backfill was already ingested once — by `install_package`,
                // which called `insert_unknown` on that same file-wide set.
                // `ingest_unknown` has no unique key and `insert_unknown` is
                // a plain `INSERT`, so handing it the whole set again would
                // silently double each `Manufacturer`/`DatapointType`
                // unknown that install already recorded: a count that
                // depends on install history rather than on bytes, which is
                // exactly what ADR-0020's E2 warns a re-derivation must not
                // touch. `FunctionType`/`FunctionPoint`/`SpaceUsage`
                // unknowns are the opposite case — the old parser never
                // looked at those elements, so nothing about them was ever
                // recorded and there is nothing to double.
                let newly_parsed: Vec<_> = outcome
                    .unknown
                    .into_iter()
                    .filter(|u| {
                        u.xpath.starts_with("/KNX/MasterData/FunctionTypes/")
                            || u.xpath.starts_with("/KNX/MasterData/SpaceUsages/")
                    })
                    .collect();
                insert_unknown(conn, &sha256, &newly_parsed)?;
            }
            Err(error) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT function_space_backfill_blob;
                     RELEASE SAVEPOINT function_space_backfill_blob;",
                )?;
                record_backfill_failure(
                    conn,
                    &sha256,
                    &source_path,
                    "FunctionSpaceBackfillError",
                    "ingest_master_data",
                    &error,
                )?;
            }
        }
    }
    Ok(())
}

/// v10 -> v11. `Module` argument interpretation (goal-completion task 12,
/// design D47): the two pieces of an argument binding that the evaluator
/// needs, and that nothing stored before v11 held in a readable form.
///
/// * `module_def_argument` — one row per `ModuleDef/Arguments/Argument`.
///   The declaration's `@Name` is the only key by which a `{{Name}}`
///   placeholder inside that `ModuleDef`'s own `Dynamic` tree can be
///   resolved, and it lived nowhere at all before this: `Arguments` sits
///   outside `Dynamic`, so the `Dynamic` pass never saw it, and the
///   `Static` pass reported it as an unmodelled construct and moved on.
/// * `dynamic_node.value` — `NumericArg`/`TextArg`'s `@Value`. The value
///   *was* stored before v11, but only inside `extra`, which design D2's
///   own schema comment declares is a human-readable audit trail and
///   explicitly **not** re-parseable (a `@Text` value containing `=` or a
///   newline splits it wrong). Re-deriving a column from a stored blob is
///   cheaper than teaching something to parse a format documented as
///   unparseable, which is the ADR-0020 check — "is it already stored?" —
///   answered honestly: stored, yes; readable, no.
///
/// [ADR-0020](../../../docs/adr/0020-migrations-may-rederive-from-stored-bytes.md)
/// E1 permits the backfill: every value re-derived here is a pure function
/// of `source_file.bytes`, with no dependence on install order or install
/// history. `migrate_v2_to_v3`'s own `backfill_dynamic_nodes` is the direct
/// precedent — same parser, same blobs, same transaction.
///
/// Renumbered from v9->v10 to v10->v11 (goal-completion task 12 renumber):
/// T13's `FunctionType`/`FunctionPoint`/`SpaceUsage` migration landed on
/// `main` first and kept the v9->v10 slot; this one runs after it, not
/// before, so it appears second in `migrations()` too.
fn migrate_v10_to_v11(conn: &Connection) -> Result<(), ProductDbError> {
    conn.execute_batch(
        "CREATE TABLE module_def_argument (
            program_id    TEXT NOT NULL,
            module_def_id TEXT NOT NULL,
            id            TEXT NOT NULL,
            name          TEXT,
            arg_type      TEXT,
            allocates     INTEGER,
            position      INTEGER NOT NULL,
            extra         TEXT,
            PRIMARY KEY (program_id, module_def_id, id)
        ) STRICT;
        CREATE INDEX module_def_argument_scope
            ON module_def_argument (program_id, module_def_id);
        ALTER TABLE dynamic_node ADD COLUMN value TEXT;",
    )?;
    reparse_dynamic_trees(conn)
}

/// Replays every stored `ApplicationProgram` blob through
/// `dynamic::parse::parse_dynamic_trees` again, after clearing what the
/// previous parse of the *same* blob wrote.
///
/// Clearing first is what makes this different from
/// `backfill_dynamic_nodes`, and it is not optional:
/// `parse_dynamic_trees` deliberately skips a program that already has
/// `dynamic_node` rows, so without the `DELETE` this function would be an
/// elaborate no-op on precisely the databases it exists for. The delete is
/// scoped to the programs this blob owns — `application_program.source_sha256
/// = this sha` — so a program that lost an id conflict to an earlier,
/// different file keeps the winner's rows, exactly as it does on the
/// ordinary ingest path.
///
/// The `ingest_unknown` rows the previous `Dynamic` pass wrote are deleted
/// alongside, matched on that pass's own xpath shape (`.../Dynamic/...`),
/// and rewritten from the fresh parse. Without that, a database migrated to
/// v11 would keep claiming `NumericArg/@Value` is an unmodelled attribute
/// long after it acquired a column — `backfill_linkable`'s stale-row
/// retirement, applied to the attributes this slice starts modelling. Rows
/// from the `Static` pass are untouched: that pass still does not model
/// `ModuleDef/Arguments`, and still says so, because its memory-allocation
/// facet (`@Allocates`, `Memory/@BaseOffset`, `ComObject/@BaseNumber`)
/// genuinely stays unmodelled here.
///
/// Per-blob `SAVEPOINT` and a recorded failure rather than an aborted
/// migration, for the reason every backfill above gives: a database that
/// refuses to open is worse than one with a gap.
fn reparse_dynamic_trees(conn: &Connection) -> Result<(), ProductDbError> {
    let mut stmt = conn.prepare("SELECT sha256, source_path, bytes FROM source_file")?;
    let blobs: Vec<(String, String, Vec<u8>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    for (sha256, source_path, bytes) in blobs {
        if classify(&bytes) != FileKind::ApplicationProgram {
            continue;
        }
        conn.execute_batch("SAVEPOINT dynamic_reparse_blob;")?;
        let cleared = clear_dynamic_pass_output(conn, &sha256);
        let outcome = cleared.and_then(|()| {
            crate::dynamic::parse::parse_dynamic_trees(conn, &sha256, &source_path, &bytes)
        });
        match outcome {
            Ok(outcome) => {
                insert_unknown(conn, &sha256, &outcome.unknown)?;
                conn.execute_batch("RELEASE SAVEPOINT dynamic_reparse_blob;")?;
            }
            Err(error) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT dynamic_reparse_blob;
                     RELEASE SAVEPOINT dynamic_reparse_blob;",
                )?;
                record_backfill_failure(
                    conn,
                    &sha256,
                    &source_path,
                    "DynamicReparseError",
                    "parse_dynamic_trees",
                    &error,
                )?;
            }
        }
    }
    Ok(())
}

/// Deletes everything the `Dynamic` pass wrote for the programs owned by
/// one blob: their `dynamic_node` rows, their `module_def_argument` rows
/// (empty on the way into v11, non-empty on a re-run), and the
/// `ingest_unknown` rows that pass recorded — identified by the xpath shape
/// `dynamic::parse::insert_node` builds and nothing else does.
fn clear_dynamic_pass_output(conn: &Connection, sha256: &str) -> Result<(), ProductDbError> {
    const OWNED_PROGRAMS: &str = "SELECT id FROM application_program WHERE source_sha256 = ?1";
    conn.execute(
        &format!("DELETE FROM dynamic_node WHERE program_id IN ({OWNED_PROGRAMS})"),
        [sha256],
    )?;
    conn.execute(
        &format!("DELETE FROM module_def_argument WHERE program_id IN ({OWNED_PROGRAMS})"),
        [sha256],
    )?;
    conn.execute(
        "DELETE FROM ingest_unknown
         WHERE source_sha256 = ?1 AND xpath LIKE '%/Dynamic/%'",
        [sha256],
    )?;
    Ok(())
}

/// v8 -> v9. The second instance of §87's class, not the third — `linkable`
/// (v8) was the first. Re-derives `parameter_type.min_inclusive`/
/// `max_inclusive`/`size_in_bit` for the rows a pre-2026-09-14 ingest left
/// `NULL` on a `Float` or `Text` kind (KNOWN_LIMITATIONS.md §87), reading
/// `TypeFloat/@minInclusive`/`@maxInclusive` and `TypeText/@SizeInBit` back
/// out of the `source_file` blob each row's owning `ApplicationProgram`
/// came from. Same shape as `backfill_linkable` for the same reason
/// [ADR-0020](../../../docs/adr/0020-migrations-may-rederive-from-stored-bytes.md)
/// gives: both attributes are pure functions of bytes this database
/// already holds, with no dependence on install order.
///
/// Unlike `linkable`, there is no stale `ingest_unknown` row to retire —
/// the old parser did not read-and-reject these attributes, it never asked
/// `insert_parameter_type` for them at all, so nothing was ever reported
/// about them either way. A backfilled row therefore differs from one a
/// fresh v9 ingest would produce in one respect this migration does not
/// close: T18 fix round 1's `report_unknown_attrs` call in
/// `insert_parameter_type` means a fresh ingest also records `TypeFloat`'s
/// unmodelled `Encoding`/`Increment`/`DisplayFormat` as `ingest_unknown`
/// rows, and this backfill does not reach for that — bounds only, matching
/// what this migration exists to fix. Named, not silently left different:
/// see KNOWN_LIMITATIONS.md §87's own note on this narrower gap.
fn migrate_v8_to_v9(conn: &Connection) -> Result<(), ProductDbError> {
    backfill_parameter_type_bounds(conn)
}

/// Scoped by the defect, the same way `backfill_linkable` is scoped: a blob
/// is read only if it is the `source_file` behind an `application_program`
/// row that in turn owns a `parameter_type` row still missing its bounds —
/// `Float`'s `min_inclusive`/`max_inclusive` both `NULL` together (the old
/// parser always wrote that pair together, never one alone) or `Text`'s
/// `size_in_bit` `NULL`. `parameter_type` carries no `source_sha256` of its
/// own, so the join through `application_program` is what stands in for
/// the direct column `backfill_linkable` reads.
///
/// Per-blob `SAVEPOINT`, and a failure recorded rather than an aborted
/// migration, for the reason `backfill_linkable` gives in full: a database
/// that refuses to open is worse than one with a gap.
fn backfill_parameter_type_bounds(conn: &Connection) -> Result<(), ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT sf.sha256, sf.source_path, sf.bytes
         FROM source_file sf
         JOIN application_program ap ON ap.source_sha256 = sf.sha256
         JOIN parameter_type pt ON pt.program_id = ap.id
         WHERE (pt.kind = 'Float' AND pt.min_inclusive IS NULL AND pt.max_inclusive IS NULL)
            OR (pt.kind = 'Text' AND pt.size_in_bit IS NULL)",
    )?;
    let blobs: Vec<(String, String, Vec<u8>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    for (sha256, source_path, bytes) in blobs {
        conn.execute_batch("SAVEPOINT parameter_type_bounds_backfill_blob;")?;
        match crate::parse::program::backfill_parameter_type_bounds(
            conn,
            &sha256,
            &source_path,
            &bytes,
        ) {
            Ok(_) => {
                conn.execute_batch("RELEASE SAVEPOINT parameter_type_bounds_backfill_blob;")?;
            }
            Err(error) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT parameter_type_bounds_backfill_blob;
                     RELEASE SAVEPOINT parameter_type_bounds_backfill_blob;",
                )?;
                record_backfill_failure(
                    conn,
                    &sha256,
                    &source_path,
                    "ParameterTypeBoundsBackfillError",
                    "backfill_parameter_type_bounds",
                    &error,
                )?;
            }
        }
    }
    Ok(())
}

/// v7 -> v8. The first step in this chain that adds no structure at all: it
/// re-derives `application_program.linkable` for the rows a pre-2026-09-13
/// ingest left `NULL` (KNOWN_LIMITATIONS.md §87), reading
/// `ApplicationProgram/@Linkable` back out of the `source_file` blob each row
/// came from.
///
/// [ADR-0020](../../../docs/adr/0020-migrations-may-rederive-from-stored-bytes.md)
/// is the decision that permits it, and the line it draws is why this is not
/// the same request `migrate_v4_to_v5` and `migrate_v5_to_v6` refused:
/// `Linkable` is a pure function of bytes this database already holds, while
/// their counters count what one `INSERT OR IGNORE` changed at one moment of
/// one database's history and cannot be recovered by replaying anything.
///
/// `user_version` is the entire mechanism. There is no column to guard on
/// with `column_exists`, so what makes this run exactly once per database is
/// the version bump `open_and_migrate` performs once the step returns.
fn migrate_v7_to_v8(conn: &Connection) -> Result<(), ProductDbError> {
    backfill_linkable(conn)
}

/// Scoped by the defect rather than by the database: only blobs that actually
/// produced an `application_program` row with `linkable IS NULL` are read, so
/// a database with none — every one ingested after 2026-09-13, and every
/// fresh one — pays a single query and touches no blob. That filter is also
/// why this backfill needs no `classify` call, unlike `backfill_dynamic_nodes`
/// and `backfill_shared_translations` above: a blob that produced an
/// `application_program` row is application-program content by construction,
/// so asking its bytes what kind they are would be asking a question already
/// answered.
///
/// Per-blob `SAVEPOINT`, and a failure recorded rather than an aborted
/// migration, for the reason those two give in full: a database that refuses
/// to open is worse than one with a gap.
fn backfill_linkable(conn: &Connection) -> Result<(), ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT sha256, source_path, bytes FROM source_file
         WHERE sha256 IN (
             SELECT source_sha256 FROM application_program WHERE linkable IS NULL
         )",
    )?;
    let blobs: Vec<(String, String, Vec<u8>)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    for (sha256, source_path, bytes) in blobs {
        conn.execute_batch("SAVEPOINT linkable_backfill_blob;")?;
        match crate::parse::program::backfill_linkable(conn, &sha256, &source_path, &bytes) {
            Ok(_) => {
                conn.execute_batch("RELEASE SAVEPOINT linkable_backfill_blob;")?;
            }
            Err(error) => {
                conn.execute_batch(
                    "ROLLBACK TO SAVEPOINT linkable_backfill_blob;
                     RELEASE SAVEPOINT linkable_backfill_blob;",
                )?;
                record_backfill_failure(
                    conn,
                    &sha256,
                    &source_path,
                    "LinkableBackfillError",
                    "backfill_linkable",
                    &error,
                )?;
            }
        }
    }
    Ok(())
}

/// v6 -> v7. `package_conflict` gains `occurrence`, `first_winner`'s own
/// per-parse-call repeat count (KNOWN_LIMITATIONS.md §86): `1` for the
/// cross-file conflicts this table has always stored, greater than `1`
/// for a same-file duplicate id, now that `first_winner` can tell the two
/// apart. Defaults to `1` for a `package_conflict` row written before this
/// column existed — the same honest convention `migrate_v5_to_v6` and
/// `migrate_v4_to_v5` use, and correct here besides: every conflict
/// `first_winner` could record before this task closed §86 *was* a
/// cross-file one, so `1` is not a guess for those rows, it is what
/// `first_winner` would have written itself.
fn migrate_v6_to_v7(conn: &Connection) -> Result<(), ProductDbError> {
    if !column_exists(conn, "package_conflict", "occurrence")? {
        conn.execute_batch(
            "ALTER TABLE package_conflict ADD COLUMN occurrence INTEGER NOT NULL DEFAULT 1;",
        )?;
    }
    Ok(())
}

/// v5 -> v6. One more `package` counter, same shape and same reasoning as
/// `migrate_v4_to_v5`'s four: `dropped_datapoint_type_count` records how
/// many `datapoint_type` rows this package's own `knx_master.xml` declared
/// that `INSERT OR IGNORE` dropped because the id already belonged to an
/// earlier package (KNOWN_LIMITATIONS.md §86). Defaults to `0` for a
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
    // `ORDER BY rowid`: `ingest_translations`' write is `INSERT OR IGNORE`
    // (`parse/translation.rs`) against `(scope, scope_id, language, ref_id,
    // attribute_name)` — first-writer-wins, so which text survives a
    // conflicting re-declaration depends on scan order. This is a real
    // hazard, not a hypothetical one: measured across the corpus's five
    // master files [V], 13 `(language, RefId, AttributeName)` keys carry
    // conflicting text — `de-DE/DPT-18/Text` is `Szenensteuerung` in one
    // file and `Szenen Kontrolle` in another, `de-DE/DPST-9-7/Text` is
    // `Feuchtigkeit (%)` in one and `Prozent (%)` in another. `rowid` pins
    // the scan to the order distinct blobs were first written — the same
    // qualifier `backfill_function_and_space_data` above carries and for
    // the same reason (ADR-0020 §E2): a blob installed by two different
    // packages is ingested twice but occupies one rowid, so this order
    // matches true install order only when no `Catalog`/`Hardware`/
    // `MasterData` blob was installed by more than one package. This hazard
    // pre-dates T13's `d10-language-data` branch — `backfill_shared_translations`
    // has been unordered since it was added in `migrate_v3_to_v4` — it is
    // named and fixed here because the same measurement pass that found
    // finding A's manufacturer-name flip found this one too.
    let mut stmt =
        conn.prepare("SELECT sha256, source_path, bytes FROM source_file ORDER BY rowid")?;
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
    conn.pragma_update(None, "foreign_keys", "ON")?;
    let found: i64 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found > CURRENT_PRODUCTDB_VERSION {
        return Err(ProductDbError::FutureVersion {
            found,
            supported: CURRENT_PRODUCTDB_VERSION,
        });
    }
    if found < CURRENT_PRODUCTDB_VERSION {
        migrate_from(&conn, found)?;
    }
    Ok(conn)
}

/// Runs every migration from `found` to `CURRENT_PRODUCTDB_VERSION` in one
/// transaction. Shared by [`open_and_migrate`] and [`open_read_only`]'s
/// in-memory copy so the two can never drift.
fn migrate_from(conn: &Connection, found: i64) -> Result<(), ProductDbError> {
    conn.execute_batch("BEGIN IMMEDIATE")?;
    let result = (|| {
        for migration in &migrations()[found as usize..CURRENT_PRODUCTDB_VERSION as usize] {
            migration(conn)?;
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
    Ok(())
}

/// How long [`open_read_only`]'s connection waits for a writer's lock.
pub const READ_ONLY_BUSY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

/// A product database opened by [`open_read_only`].
#[derive(Debug)]
pub struct ReadOnlyProductDb {
    /// At `CURRENT_PRODUCTDB_VERSION`. Either the file itself, opened
    /// `SQLITE_OPEN_READ_ONLY` with `query_only` on, or an in-memory copy.
    pub conn: Connection,
    /// The file's own version when it was older than this build's, in which
    /// case `conn` is an in-memory copy that was migrated instead of the
    /// file (ADR-0090). `None` when the file was already current.
    pub migrated_from: Option<i64>,
}

/// Opens an existing product database without ever writing to it
/// (ADR-0090). Unlike [`open_and_migrate`] it creates neither the file nor
/// its parent directory: a missing path is an SQLite open error. A newer
/// version is refused. An older one is copied into memory with SQLite's
/// online backup API and only the copy is migrated, so the file keeps its
/// bytes and its version.
pub fn open_read_only(path: &Path) -> Result<ReadOnlyProductDb, ProductDbError> {
    use rusqlite::OpenFlags;
    let flags = OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX;
    let file = Connection::open_with_flags(path, flags)?;
    file.pragma_update(None, "query_only", true)?;
    // An install holds the exclusive lock while it commits; a reader waits
    // for it rather than failing. Explicit, not left to the library default.
    file.busy_timeout(READ_ONLY_BUSY_TIMEOUT)?;
    let found: i64 = file.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if found > CURRENT_PRODUCTDB_VERSION {
        return Err(ProductDbError::FutureVersion {
            found,
            supported: CURRENT_PRODUCTDB_VERSION,
        });
    }
    if found == CURRENT_PRODUCTDB_VERSION {
        return Ok(ReadOnlyProductDb {
            conn: file,
            migrated_from: None,
        });
    }
    let mut memory = Connection::open_in_memory()?;
    {
        // All pages in one step; a busy source is retried after a pause.
        let backup = rusqlite::backup::Backup::new(&file, &mut memory)?;
        backup.run_to_completion(i32::MAX, std::time::Duration::from_millis(20), None)?;
    }
    drop(file);
    memory.pragma_update(None, "foreign_keys", "ON")?;
    migrate_from(&memory, found)?;
    Ok(ReadOnlyProductDb {
        conn: memory,
        migrated_from: Some(found),
    })
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
    fn v12_catalogue_backfill_uses_winning_source_and_rejects_truncated_blob() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        {
            let conn = Connection::open(&path).unwrap();
            for migrate in migrations().iter().take(12) {
                migrate(&conn).unwrap();
            }
            conn.pragma_update(None, "user_version", 12).unwrap();
            conn.execute("INSERT INTO manufacturer (id) VALUES ('M-0001')", [])
                .unwrap();
            for (sha, bytes) in [
                ("winner", b"<KNX><ApplicationProgram Id='A-win' IsSecureEnabled='false' MinEtsVersion=''/></KNX>".as_slice()),
                ("loser", b"<KNX><ApplicationProgram Id='A-win' IsSecureEnabled='true'/></KNX>".as_slice()),
                ("truncated", b"<KNX><ApplicationProgram Id='A-bad' IsSecureEnabled='true'/>".as_slice()),
                ("trailing", b"<KNX><ApplicationProgram Id='A-trailing' IsSecureEnabled='true'/></KNX>garbage".as_slice()),
                ("entity", b"<KNX><Unknown>&undeclared;</Unknown><ApplicationProgram Id='A-entity' IsSecureEnabled='true'/></KNX>".as_slice()),
                ("attrs", b"<KNX><Unknown x='1' x='2'/><ApplicationProgram Id='A-attrs' IsSecureEnabled='true'/></KNX>".as_slice()),
                ("good", b"<KNX><ApplicationProgram Id='A-good' ReplacesVersions='1,2'/></KNX>".as_slice()),
            ] {
                conn.execute(
                    "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes) VALUES (?1, ?2, 'M-0001', ?3, ?4)",
                    params![sha, format!("M-0001/{sha}.xml"), bytes.len() as i64, bytes],
                )
                .unwrap();
            }
            for (id, sha) in [
                ("A-win", "winner"),
                ("A-bad", "truncated"),
                ("A-trailing", "trailing"),
                ("A-entity", "entity"),
                ("A-attrs", "attrs"),
                ("A-good", "good"),
            ] {
                conn.execute(
                    "INSERT INTO application_program (id, manufacturer_id, source_sha256) VALUES (?1, 'M-0001', ?2)",
                    params![id, sha],
                )
                .unwrap();
            }
            conn.execute(
                "INSERT INTO ingest_unknown (source_sha256, xpath, kind, name, occurrences) VALUES ('winner', '/KNX/ApplicationProgram', 'Attribute', 'MinEtsVersion', 2)",
                [],
            )
            .unwrap();
        }
        let conn = open_and_migrate(&path).unwrap();
        let win: (Option<String>, Option<String>) = conn
            .query_row("SELECT is_secure_enabled, min_ets_version FROM application_program WHERE id='A-win'", [], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap();
        assert_eq!(win, (Some("false".into()), Some(String::new())));
        let good: String = conn
            .query_row(
                "SELECT replaces_versions FROM application_program WHERE id='A-good'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(good, "1,2");
        let bad: Option<String> = conn
            .query_row(
                "SELECT is_secure_enabled FROM application_program WHERE id='A-bad'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bad, None);
        for id in ["A-trailing", "A-entity", "A-attrs"] {
            let value: Option<String> = conn
                .query_row(
                    "SELECT is_secure_enabled FROM application_program WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .unwrap();
            assert_eq!(value, None, "malformed source {id} filled metadata");
        }
        let diagnostics: i64 = conn
            .query_row(
                "SELECT count(*) FROM ingest_unknown WHERE kind='CatalogMetadataBackfillError'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(diagnostics, 4);
        let history: i64 = conn.query_row("SELECT occurrences FROM ingest_unknown WHERE source_sha256='winner' AND name='MinEtsVersion'", [], |r| r.get(0)).unwrap();
        assert_eq!(history, 2);
        drop(conn);
        let conn = open_and_migrate(&path).unwrap();
        let repeat: i64 = conn
            .query_row(
                "SELECT count(*) FROM ingest_unknown WHERE kind='CatalogMetadataBackfillError'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(repeat, 4);
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
    fn returned_connections_enforce_foreign_keys_when_fresh_and_reopened() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");

        let fresh = open_and_migrate(&path).unwrap();
        let enabled: i64 = fresh
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(enabled, 1);
        drop(fresh);

        let reopened = open_and_migrate(&path).unwrap();
        let enabled: i64 = reopened
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .unwrap();
        assert_eq!(enabled, 1);
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

    /// One `ApplicationProgram`, spelled the way a `.knxprod` spells it —
    /// `Linkable="false"`, not `"0"` — which is the spelling KNOWN_LIMITATIONS
    /// §87's `bool_flag` could not read. `{LINKABLE}` is substituted per test.
    const PROGRAM_TEMPLATE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/20">
  <ManufacturerData>
    <Manufacturer RefId="M-0083">
      <ApplicationPrograms>
        <ApplicationProgram Id="M-0083_A-0318-31-DB39" Name="Switch Actuator"
                            ApplicationNumber="792" ApplicationVersion="31"
                            MaskVersion="MV-0701"{LINKABLE}>
          <Static />
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    const PROGRAM_ID: &str = "M-0083_A-0318-31-DB39";

    /// The xpath the ingest path computes for that element's attributes, and
    /// therefore the key the `ingest_unknown` row a pre-fix ingest wrote is
    /// under. Spelled out rather than derived, so that a change to either
    /// side of the pairing has to come here and be looked at.
    const LINKABLE_XPATH: &str =
        "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram";

    fn program_xml(linkable: &str) -> String {
        PROGRAM_TEMPLATE.replace("{LINKABLE}", linkable)
    }

    /// Older migration fixtures use the current parser but must still have
    /// their original schema when the migration under test opens them.
    fn with_catalogue_columns_for_fixture(conn: &Connection, ingest: impl FnOnce()) {
        const COLUMNS: &[&str] = &[
            "is_secure_enabled",
            "max_security_group_key_table_entries",
            "max_security_individual_address_entries",
            "max_security_p2p_key_table_entries",
            "max_tunneling_user_entries",
            "max_user_entries",
            "min_ets_version",
            "replaces_versions",
        ];
        for name in COLUMNS {
            conn.execute_batch(&format!(
                "ALTER TABLE application_program ADD COLUMN {name} TEXT"
            ))
            .unwrap();
        }
        ingest();
        for name in COLUMNS {
            conn.execute_batch(&format!(
                "ALTER TABLE application_program DROP COLUMN {name}"
            ))
            .unwrap();
        }
    }

    /// Builds the thing this migration exists for: a database at
    /// `user_version` 6 holding a program blob, the program's row, and
    /// `linkable` `NULL` — plus, when `stale_report` is set, the
    /// `ingest_unknown` row the old `bool_flag` wrote when it met a spelling
    /// it could not read. Returns the blob's sha256.
    ///
    /// The rollback is done by ingesting with the *current* parser and then
    /// undoing exactly the two effects the 2026-09-13 fix has — the stored
    /// value and the retired report — rather than by hand-writing an
    /// `application_program` row, so the fixture cannot drift away from the
    /// table's real shape.
    fn v6_database_with_a_null_linkable(
        path: &Path,
        xml: &str,
        stale_report: Option<&str>,
    ) -> String {
        let bytes = xml.as_bytes();
        let sha = crate::sha256_hex(bytes);
        let conn = Connection::open(path).unwrap();
        for migration in &migrations()[0..6] {
            migration(&conn).unwrap();
        }
        conn.execute(
            "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![sha, "M-0083/A.xml", "M-0083", bytes.len() as i64, bytes],
        )
        .unwrap();
        with_catalogue_columns_for_fixture(&conn, || {
            crate::parse::program::ingest_program(&conn, &sha, "M-0083/A.xml", bytes).unwrap();
        });
        conn.execute("UPDATE application_program SET linkable = NULL", [])
            .unwrap();
        if let Some(sample) = stale_report {
            conn.execute(
                "INSERT INTO ingest_unknown
                 (source_sha256, program_id, xpath, kind, name, occurrences, sample)
                 VALUES (?1, NULL, ?2, 'Attribute', 'Linkable', 1, ?3)",
                params![sha, LINKABLE_XPATH, sample],
            )
            .unwrap();
        }
        conn.pragma_update(None, "user_version", 6i64).unwrap();
        sha
    }

    fn stored_linkable(conn: &Connection) -> Option<i64> {
        conn.query_row(
            "SELECT linkable FROM application_program WHERE id = ?1",
            [PROGRAM_ID],
            |r| r.get(0),
        )
        .unwrap()
    }

    fn stale_linkable_reports(conn: &Connection) -> i64 {
        conn.query_row(
            "SELECT count(*) FROM ingest_unknown
             WHERE kind = 'Attribute' AND name = 'Linkable'",
            [],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn a_v6_database_backfills_the_linkable_its_blob_already_held() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        v6_database_with_a_null_linkable(&path, &program_xml(" Linkable=\"false\""), Some("false"));

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            CURRENT_PRODUCTDB_VERSION
        );
        assert_eq!(
            stored_linkable(&conn),
            Some(0),
            "`Linkable=\"false\"` is stored as 0, re-read from the blob alone"
        );
        assert_eq!(
            stale_linkable_reports(&conn),
            0,
            "the report that said the attribute was not understood is retired, \
             because it now is"
        );
    }

    #[test]
    fn a_v6_database_backfills_a_true_linkable_as_one() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        v6_database_with_a_null_linkable(&path, &program_xml(" Linkable=\"true\""), Some("true"));

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(stored_linkable(&conn), Some(1));
        assert_eq!(stale_linkable_reports(&conn), 0);
    }

    #[test]
    fn a_program_whose_file_never_stated_linkable_stays_null() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        v6_database_with_a_null_linkable(&path, &program_xml(""), None);

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            stored_linkable(&conn),
            None,
            "NULL means the file did not state it, and the backfill must not \
             invent a value it never read"
        );
        assert_eq!(
            conn.query_row("SELECT count(*) FROM ingest_unknown", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0,
            "and it must not report anything either — an absent attribute is \
             nothing to report"
        );
    }

    #[test]
    fn a_linkable_an_ingest_already_determined_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        v6_database_with_a_null_linkable(&path, &program_xml(" Linkable=\"false\""), Some("false"));
        {
            // The blob says `false`; the row says `true`. Only an ingest can
            // have put a non-NULL value there, and ADR-0020 rule 2 says a
            // migration does not argue with it.
            let conn = Connection::open(&path).unwrap();
            conn.execute("UPDATE application_program SET linkable = 1", [])
                .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(stored_linkable(&conn), Some(1));
        assert_eq!(
            stale_linkable_reports(&conn),
            1,
            "and the report stays too, since nothing was filled"
        );
    }

    #[test]
    fn a_blob_does_not_backfill_a_row_that_came_from_a_different_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let sha = v6_database_with_a_null_linkable(
            &path,
            &program_xml(" Linkable=\"false\""),
            Some("false"),
        );
        {
            // The id-conflict shape of ADR-0011: this program id's row was won
            // by some *other* file, so this blob must not write into it — even
            // though the blob is still read, because a second row of its own
            // (which it did win) is NULL and pulls it into the selection.
            let conn = Connection::open(&path).unwrap();
            conn.execute(
                "INSERT INTO application_program (id, manufacturer_id, source_sha256)
                 VALUES ('M-0083_A-OTHER', 'M-0083', ?1)",
                [&sha],
            )
            .unwrap();
            conn.execute(
                "UPDATE application_program SET source_sha256 = 'deadbeef' WHERE id = ?1",
                [PROGRAM_ID],
            )
            .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            stored_linkable(&conn),
            None,
            "the winning row belongs to another file's bytes"
        );
        assert_eq!(stale_linkable_reports(&conn), 1);
    }

    #[test]
    fn a_v6_blob_that_fails_to_parse_records_itself_and_does_not_stop_the_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let xml = program_xml(" Linkable=\"false\"");
        let good_sha = v6_database_with_a_null_linkable(&path, &xml, Some("false"));
        // Truncated past the last end tag, which `quick-xml` rejects as "tag
        // not closed" rather than accepting as `Eof` — the same shape the
        // translation backfill's equivalent test uses. Given an
        // `application_program` row of its own so the v8 filter selects it.
        let bad = &xml.as_bytes()[..xml.len() - 20];
        let bad_sha = crate::sha256_hex(bad);
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![bad_sha, "M-BAD/A.xml", "M-BAD", bad.len() as i64, bad],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO application_program (id, manufacturer_id, source_sha256)
                 VALUES ('M-BAD_A-1', 'M-BAD', ?1)",
                [&bad_sha],
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 6i64).unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            CURRENT_PRODUCTDB_VERSION,
            "one blob's parse failure must not abort the migration"
        );
        assert_eq!(
            stored_linkable(&conn),
            Some(0),
            "the good blob must still be backfilled"
        );
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM ingest_unknown
                 WHERE source_sha256 = ?1 AND kind = 'LinkableBackfillError'",
                [&bad_sha],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM ingest_unknown
                 WHERE source_sha256 = ?1 AND kind = 'Attribute' AND name = 'Linkable'",
                [&good_sha],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0,
            "and the good blob's own stale report is still retired"
        );
    }

    #[test]
    fn a_database_with_nothing_to_backfill_reads_no_blob_at_all() {
        // The v8 step is scoped by `linkable IS NULL`, so a database whose
        // programs all have a value — every one ingested after 2026-09-13 —
        // must not be dragged through its own blobs. Proven by giving it a
        // blob that cannot be parsed at all: if v8 read it, the migration
        // would record a `LinkableBackfillError`.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let xml = program_xml(" Linkable=\"false\"");
        {
            let conn = Connection::open(&path).unwrap();
            for migration in &migrations()[0..6] {
                migration(&conn).unwrap();
            }
            let bytes = xml.as_bytes();
            let sha = crate::sha256_hex(bytes);
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![sha, "M-0083/A.xml", "M-0083", bytes.len() as i64, bytes],
            )
            .unwrap();
            with_catalogue_columns_for_fixture(&conn, || {
                crate::parse::program::ingest_program(&conn, &sha, "M-0083/A.xml", bytes).unwrap();
            });
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES ('feedface', 'M-BAD/A.xml', 'M-BAD', 7, ?1)",
                [b"<KNX><".as_slice()],
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 6i64).unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(stored_linkable(&conn), Some(0));
        assert_eq!(
            conn.query_row("SELECT count(*) FROM ingest_unknown", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0,
            "no blob was read, so no blob could fail"
        );
    }

    // --- v8 -> v9: parameter_type bounds backfill -------------------------

    const PARAM_PROGRAM_TEMPLATE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-006A">
      <ApplicationPrograms>
        <ApplicationProgram Id="M-006A_A-0001-22-26C0-O0079" Name="Presence"
                             ApplicationNumber="1" ApplicationVersion="22"
                             MaskVersion="MV-0701">
          <Static>
            <ParameterTypes>
              <ParameterType Id="PT-Float" Name="threshold">
                <TypeFloat Encoding="DPT 9"{FLOAT} />
              </ParameterType>
              <ParameterType Id="PT-Text" Name="label">
                <TypeText{TEXT} />
              </ParameterType>
            </ParameterTypes>
            <Parameters />
            <ParameterRefs />
          </Static>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

    const PARAM_PROGRAM_ID: &str = "M-006A_A-0001-22-26C0-O0079";

    fn param_program_xml(float_attrs: &str, text_attrs: &str) -> String {
        PARAM_PROGRAM_TEMPLATE
            .replace("{FLOAT}", float_attrs)
            .replace("{TEXT}", text_attrs)
    }

    /// Builds the thing v9 exists for: a database at `user_version` 8 (every
    /// migration through `linkable`'s, none of `parameter_type`'s bounds)
    /// holding a program blob whose `Float`/`Text` parameter types are
    /// `NULL` on the columns a pre-2026-09-14 ingest never read — built the
    /// same way `v6_database_with_a_null_linkable` is: ingest for real with
    /// the *current* parser (which does read these attributes), then null
    /// exactly the columns the old parser never wrote, so the fixture cannot
    /// drift from the table's real shape. Returns the blob's sha256.
    fn v8_database_with_null_parameter_type_bounds(path: &Path, xml: &str) -> String {
        let bytes = xml.as_bytes();
        let sha = crate::sha256_hex(bytes);
        let conn = Connection::open(path).unwrap();
        for migration in &migrations()[0..8] {
            migration(&conn).unwrap();
        }
        conn.execute(
            "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![sha, "M-006A/A.xml", "M-006A", bytes.len() as i64, bytes],
        )
        .unwrap();
        with_catalogue_columns_for_fixture(&conn, || {
            crate::parse::program::ingest_program(&conn, &sha, "M-006A/A.xml", bytes).unwrap();
        });
        conn.execute(
            "UPDATE parameter_type SET min_inclusive = NULL, max_inclusive = NULL
             WHERE kind = 'Float'",
            [],
        )
        .unwrap();
        conn.execute(
            "UPDATE parameter_type SET size_in_bit = NULL WHERE kind = 'Text'",
            [],
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 8i64).unwrap();
        sha
    }

    fn stored_float_bounds(conn: &Connection) -> (Option<String>, Option<String>) {
        conn.query_row(
            "SELECT min_inclusive, max_inclusive FROM parameter_type
             WHERE program_id = ?1 AND id = 'PT-Float'",
            [PARAM_PROGRAM_ID],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap()
    }

    fn stored_text_size(conn: &Connection) -> Option<i64> {
        conn.query_row(
            "SELECT size_in_bit FROM parameter_type
             WHERE program_id = ?1 AND id = 'PT-Text'",
            [PARAM_PROGRAM_ID],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn a_v8_database_backfills_float_bounds_and_text_size_from_its_blob() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        v8_database_with_null_parameter_type_bounds(
            &path,
            &param_program_xml(
                " minInclusive=\"-100\" maxInclusive=\"200\"",
                " SizeInBit=\"240\"",
            ),
        );

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            CURRENT_PRODUCTDB_VERSION
        );
        assert_eq!(
            stored_float_bounds(&conn),
            (Some("-100".to_string()), Some("200".to_string())),
            "bounds re-read from the blob alone"
        );
        assert_eq!(
            stored_text_size(&conn),
            Some(240),
            "size re-read from the blob alone"
        );
    }

    #[test]
    fn a_program_whose_file_never_stated_bounds_stays_null() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        v8_database_with_null_parameter_type_bounds(&path, &param_program_xml("", ""));

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            stored_float_bounds(&conn),
            (None, None),
            "NULL means the file did not state it, and the backfill must not \
             invent a value it never read"
        );
        assert_eq!(stored_text_size(&conn), None);
    }

    #[test]
    fn a_bound_an_ingest_already_determined_is_not_overwritten() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        v8_database_with_null_parameter_type_bounds(
            &path,
            &param_program_xml(
                " minInclusive=\"-100\" maxInclusive=\"200\"",
                " SizeInBit=\"240\"",
            ),
        );
        {
            // The blob says -100/200; the row says 0/50. Only an ingest can
            // have put a non-NULL value there, and ADR-0020 rule 2 says a
            // migration does not argue with it.
            let conn = Connection::open(&path).unwrap();
            conn.execute(
                "UPDATE parameter_type SET min_inclusive = '0', max_inclusive = '50'
                 WHERE kind = 'Float'",
                [],
            )
            .unwrap();
            conn.execute(
                "UPDATE parameter_type SET size_in_bit = 64 WHERE kind = 'Text'",
                [],
            )
            .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            stored_float_bounds(&conn),
            (Some("0".to_string()), Some("50".to_string()))
        );
        assert_eq!(stored_text_size(&conn), Some(64));
    }

    #[test]
    fn a_parameter_type_bounds_blob_does_not_backfill_a_row_from_a_different_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let sha = v8_database_with_null_parameter_type_bounds(
            &path,
            &param_program_xml(
                " minInclusive=\"-100\" maxInclusive=\"200\"",
                " SizeInBit=\"240\"",
            ),
        );
        {
            // The id-conflict shape of ADR-0011: this program id's row was
            // won by some *other* file, so this blob must not write into
            // its parameter_type rows — even though the blob is still read,
            // because a second program (which it did win) is still NULL
            // and pulls it into the selection.
            let conn = Connection::open(&path).unwrap();
            conn.execute(
                "INSERT INTO application_program (id, manufacturer_id, source_sha256)
                 VALUES ('M-006A_A-OTHER', 'M-006A', ?1)",
                [&sha],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO parameter_type (program_id, id, name, kind)
                 VALUES ('M-006A_A-OTHER', 'PT-Float', 'other', 'Float')",
                [],
            )
            .unwrap();
            conn.execute(
                "UPDATE application_program SET source_sha256 = 'deadbeef' WHERE id = ?1",
                [PARAM_PROGRAM_ID],
            )
            .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            stored_float_bounds(&conn),
            (None, None),
            "the winning row belongs to another file's bytes"
        );
        assert_eq!(stored_text_size(&conn), None);
    }

    #[test]
    fn a_v8_blob_that_fails_to_parse_records_itself_and_does_not_stop_the_migration() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let xml = param_program_xml(
            " minInclusive=\"-100\" maxInclusive=\"200\"",
            " SizeInBit=\"240\"",
        );
        v8_database_with_null_parameter_type_bounds(&path, &xml);
        // Truncated past the last end tag, same shape the linkable
        // backfill's own parse-failure test uses.
        let bad = &xml.as_bytes()[..xml.len() - 20];
        let bad_sha = crate::sha256_hex(bad);
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![bad_sha, "M-BAD/A.xml", "M-BAD", bad.len() as i64, bad],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO application_program (id, manufacturer_id, source_sha256)
                 VALUES ('M-BAD_A-1', 'M-BAD', ?1)",
                [&bad_sha],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO parameter_type (program_id, id, name, kind)
                 VALUES ('M-BAD_A-1', 'PT-Float', 'bad', 'Float')",
                [],
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 8i64).unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            CURRENT_PRODUCTDB_VERSION,
            "one blob's parse failure must not abort the migration"
        );
        assert_eq!(
            stored_float_bounds(&conn),
            (Some("-100".to_string()), Some("200".to_string())),
            "the good blob must still be backfilled"
        );
        assert_eq!(stored_text_size(&conn), Some(240));
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM ingest_unknown
                 WHERE source_sha256 = ?1 AND kind = 'ParameterTypeBoundsBackfillError'",
                [&bad_sha],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn a_database_with_no_parameter_type_bounds_to_backfill_reads_no_blob_at_all() {
        // The v9 step is scoped by the bounds columns being NULL, so a
        // database whose `parameter_type` rows all already have a value —
        // every one ingested after 2026-09-14 — must not be dragged through
        // its own blobs. Proven by giving it a blob that cannot be parsed
        // at all: if v9 read it, the migration would record a
        // `ParameterTypeBoundsBackfillError`.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let xml = param_program_xml(
            " minInclusive=\"-100\" maxInclusive=\"200\"",
            " SizeInBit=\"240\"",
        );
        {
            let conn = Connection::open(&path).unwrap();
            for migration in &migrations()[0..8] {
                migration(&conn).unwrap();
            }
            let bytes = xml.as_bytes();
            let sha = crate::sha256_hex(bytes);
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![sha, "M-006A/A.xml", "M-006A", bytes.len() as i64, bytes],
            )
            .unwrap();
            with_catalogue_columns_for_fixture(&conn, || {
                crate::parse::program::ingest_program(&conn, &sha, "M-006A/A.xml", bytes).unwrap();
            });
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES ('feedface', 'M-BAD/A.xml', 'M-BAD', 7, ?1)",
                [b"<KNX><".as_slice()],
            )
            .unwrap();
            conn.pragma_update(None, "user_version", 8i64).unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            stored_float_bounds(&conn),
            (Some("-100".to_string()), Some("200".to_string()))
        );
        assert_eq!(stored_text_size(&conn), Some(240));
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM ingest_unknown WHERE kind = 'ParameterTypeBoundsBackfillError'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0,
            "no blob was read, so no blob could fail"
        );
    }

    /// A `knx_master.xml` carrying `FunctionTypes`/`SpaceUsages` plus
    /// `Master`-scope translations for both — the same shape
    /// `parse/master.rs`'s own fixture uses, kept here rather than shared so
    /// this file's frozen-database tests stay self-contained the way its
    /// neighbours already do.
    const MASTER_WITH_FUNCTIONS_AND_LANGUAGES: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Siemens" />
    </Manufacturers>
    <FunctionTypes>
      <FunctionType Id="FT-1" Number="1" Text="Switch" Status="Certified">
        <FunctionPoint Id="FP-1_DR-1" Text="Switch" DatapointType="DPST-1-1" Role="Control" Characteristics="W" />
      </FunctionType>
    </FunctionTypes>
    <SpaceUsages>
      <SpaceUsage Id="SU-1" Number="1" Text="Office" />
    </SpaceUsages>
  </MasterData>
  <Languages>
    <Language Identifier="de-DE">
      <TranslationUnit RefId="FT-1">
        <TranslationElement RefId="FT-1">
          <Translation AttributeName="Text" Text="Schalten" />
        </TranslationElement>
      </TranslationUnit>
      <TranslationUnit RefId="SU-1">
        <TranslationElement RefId="SU-1">
          <Translation AttributeName="Text" Text="Büro" />
        </TranslationElement>
      </TranslationUnit>
    </Language>
  </Languages>
</KNX>"#;

    /// A v9 database — built the frozen way, running only `migrations()`'s
    /// first nine functions, exactly like every other "previous schema
    /// version" test in this file — already holds the `knx_master.xml` blob
    /// and its `source_parse_evidence` row, but has no `function_type`,
    /// `function_point` or `space_usage` table to have written into, so the
    /// `FunctionType`/`FunctionPoint`/`SpaceUsage` rows and their
    /// `Master`-scope translations never got written on first ingest. This
    /// proves `migrate_v9_to_v10`'s backfill recovers them anyway, closing
    /// `docs/KNOWN_LIMITATIONS.md` §64's last residue for data already on
    /// disk, not only for data ingested from here on.
    #[test]
    fn a_v9_database_backfills_function_and_space_usage_rows_and_their_translations() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let bytes = MASTER_WITH_FUNCTIONS_AND_LANGUAGES.as_bytes();
        let sha = crate::sha256_hex(bytes);
        {
            let conn = Connection::open(&path).unwrap();
            for migration in &migrations()[0..9] {
                migration(&conn).unwrap();
            }
            conn.pragma_update(None, "user_version", 9i64).unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    sha,
                    "knx_master.xml",
                    None::<String>,
                    bytes.len() as i64,
                    bytes
                ],
            )
            .unwrap();
            // Present in `source_parse_evidence`, exactly like a blob a v9
            // build already ingested through `ingest_master_data` — the
            // content-hash skip means the ordinary path would never revisit
            // it, which is the whole reason a backfill exists.
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
        // Not a literal 10 any more: the module-argument migration was
        // renumbered behind this one, so a v9 database now climbs two steps.
        // What this test is about is the backfill below, not where the chain
        // happens to stop.
        assert_eq!(version, CURRENT_PRODUCTDB_VERSION);

        let (number, text, status): (i64, String, String) = conn
            .query_row(
                "SELECT number, text, status FROM function_type WHERE id = 'FT-1'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(
            (number, text.as_str(), status.as_str()),
            (1, "Switch", "Certified")
        );

        let function_type_id: String = conn
            .query_row(
                "SELECT function_type_id FROM function_point WHERE id = 'FP-1_DR-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(function_type_id, "FT-1");

        let space_usage_text: String = conn
            .query_row("SELECT text FROM space_usage WHERE id = 'SU-1'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(space_usage_text, "Office");

        let function_type_translation: String = conn
            .query_row(
                "SELECT text FROM translation
                 WHERE scope = 'Master' AND ref_id = 'FT-1' AND attribute_name = 'Text'
                   AND language = 'de-DE'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(function_type_translation, "Schalten");

        let space_usage_translation: String = conn
            .query_row(
                "SELECT text FROM translation
                 WHERE scope = 'Master' AND ref_id = 'SU-1' AND attribute_name = 'Text'
                   AND language = 'de-DE'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(space_usage_translation, "Büro");
    }

    /// A `knx_master.xml` whose `Manufacturer` and `FunctionType` each carry
    /// one attribute no `*_ATTRS` list names, so `ingest_master_data`'s
    /// file-wide `UnknownCollector` produces one of each.
    const MASTER_WITH_UNKNOWN_ATTRS: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Siemens" KnxAdminMode="true" />
    </Manufacturers>
    <FunctionTypes>
      <FunctionType Id="FT-1" Number="1" Text="Switch" Status="Certified" Obsolete="false">
        <FunctionPoint Id="FP-1_DR-1" Text="Switch" DatapointType="DPST-1-1" Role="Control" Characteristics="W" />
      </FunctionType>
    </FunctionTypes>
    <SpaceUsages>
      <SpaceUsage Id="SU-1" Number="1" Text="Office" />
    </SpaceUsages>
  </MasterData>
</KNX>"#;

    /// The backfill replays the *whole* `ingest_master_data`, whose unknown
    /// collector is file-wide, but the blob it replays was already ingested
    /// once — `install_package` called `insert_unknown` on that same
    /// file-wide set at install time, and `ingest_unknown` has no unique key
    /// to collide on. Recording the whole set a second time would double
    /// every unknown the *old* parser had already seen, turning a count into
    /// a function of how many times a database happened to be migrated.
    /// Only the two element families this migration newly parses may
    /// contribute, because only those were never recorded before.
    #[test]
    fn the_backfill_records_only_the_unknowns_of_the_families_it_newly_parses() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let bytes = MASTER_WITH_UNKNOWN_ATTRS.as_bytes();
        let sha = crate::sha256_hex(bytes);
        {
            let conn = Connection::open(&path).unwrap();
            for migration in &migrations()[0..9] {
                migration(&conn).unwrap();
            }
            conn.pragma_update(None, "user_version", 9i64).unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    sha,
                    "knx_master.xml",
                    None::<String>,
                    bytes.len() as i64,
                    bytes
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO source_parse_evidence (sha256) VALUES (?1)",
                [&sha],
            )
            .unwrap();
            // What the v9 install of this same blob recorded: the
            // `Manufacturer` unknown, and nothing about `FunctionType`,
            // whose element the old parser never entered.
            conn.execute(
                "INSERT INTO ingest_unknown
                     (source_sha256, program_id, xpath, kind, name, occurrences, sample)
                 VALUES (?1, NULL, '/KNX/MasterData/Manufacturers/Manufacturer',
                         'Attribute', 'KnxAdminMode', 1, 'true')",
                [&sha],
            )
            .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();

        let manufacturer_rows: i64 = conn
            .query_row(
                "SELECT count(*) FROM ingest_unknown WHERE name = 'KnxAdminMode'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            manufacturer_rows, 1,
            "the backfill must not re-record an unknown the original ingest already recorded"
        );

        let function_type_rows: i64 = conn
            .query_row(
                "SELECT count(*) FROM ingest_unknown WHERE name = 'Obsolete'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            function_type_rows, 1,
            "an unknown on an element only this migration parses has never been recorded before, \
             so it must be recorded now"
        );
    }

    #[test]
    fn a_v9_blob_that_fails_to_parse_as_master_data_records_itself_and_does_not_stop_the_migration()
    {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let good = MASTER_WITH_FUNCTIONS_AND_LANGUAGES.as_bytes();
        let good_sha = crate::sha256_hex(good);
        // Truncated mid-tag, in the same spirit as the linkable and
        // parameter-type-bounds backfills' own parse-failure tests, but by
        // a smaller, empirically-checked amount: `<MasterData>` opens early
        // enough for `classify` to route this blob into
        // `ingest_master_data` either way, but not every truncation length
        // of this particular fixture leaves `quick-xml` mid-tag rather
        // than at a tag boundary it is willing to read as `Eof` — 8 bytes
        // off the end does, checked against this exact fixture rather than
        // assumed from a sibling's number.
        let bad = &good[..good.len() - 8];
        let bad_sha = crate::sha256_hex(bad);
        {
            let conn = Connection::open(&path).unwrap();
            for migration in &migrations()[0..9] {
                migration(&conn).unwrap();
            }
            conn.pragma_update(None, "user_version", 9i64).unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    good_sha,
                    "knx_master.xml",
                    None::<String>,
                    good.len() as i64,
                    good
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO source_parse_evidence (sha256) VALUES (?1)",
                [&good_sha],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    bad_sha,
                    "M-BAD/knx_master.xml",
                    None::<String>,
                    bad.len() as i64,
                    bad
                ],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO source_parse_evidence (sha256) VALUES (?1)",
                [&bad_sha],
            )
            .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
                .unwrap(),
            CURRENT_PRODUCTDB_VERSION,
            "one blob's parse failure must not abort the migration"
        );

        let function_type_text: String = conn
            .query_row(
                "SELECT text FROM function_type WHERE id = 'FT-1'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            function_type_text, "Switch",
            "the good blob must still be backfilled"
        );

        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM ingest_unknown
                 WHERE source_sha256 = ?1 AND kind = 'FunctionSpaceBackfillError'",
                [&bad_sha],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            1
        );
    }

    #[test]
    fn a_v9_database_with_no_master_data_blob_reads_no_blob_at_all() {
        // `backfill_function_and_space_data` skips every blob that does not
        // classify as `MasterData` before ever handing it to
        // `ingest_master_data`. Proven with a blob that cannot be parsed as
        // XML at all — the same garbage bytes the linkable and
        // parameter-type-bounds backfills' own "nothing to backfill" tests
        // use: if this backfill read it anyway, the migration would record
        // a `FunctionSpaceBackfillError`.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        {
            let conn = Connection::open(&path).unwrap();
            for migration in &migrations()[0..9] {
                migration(&conn).unwrap();
            }
            conn.pragma_update(None, "user_version", 9i64).unwrap();
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES ('feedface', 'M-BAD/A.xml', 'M-BAD', 7, ?1)",
                [b"<KNX><".as_slice()],
            )
            .unwrap();
        }

        let conn = open_and_migrate(&path).unwrap();
        assert_eq!(
            conn.query_row("SELECT count(*) FROM function_type", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            conn.query_row(
                "SELECT count(*) FROM ingest_unknown WHERE kind = 'FunctionSpaceBackfillError'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0,
            "no MasterData blob was read, so no blob could fail"
        );
    }

    const MASTER_OLD_SPELLING: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0042" Name="Old Spelling" />
    </Manufacturers>
  </MasterData>
</KNX>"#;

    const MASTER_NEW_SPELLING: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0042" Name="New Spelling" />
    </Manufacturers>
  </MasterData>
</KNX>"#;

    /// Regression guard for `backfill_function_and_space_data`'s
    /// `ORDER BY rowid` (T13 fix round 2, KNOWN_LIMITATIONS.md §88's
    /// residual). Two `knx_master.xml` blobs disagree about `M-0042`'s
    /// display name; `Manufacturer`'s write is `ON CONFLICT DO UPDATE SET
    /// name = excluded.name` — last-writer-wins — so the surviving name
    /// must be the blob inserted *last*, i.e. the one at the higher
    /// `rowid`. `PRAGMA reverse_unordered_selects` makes this an actual
    /// regression guard rather than a test that would pass with or without
    /// the clause: a plain two-insert setup does not distinguish them,
    /// because SQLite's own unordered table scan already visits a
    /// never-deleted-from table in rowid order in practice — the very
    /// "true by accident" behaviour the `ORDER BY rowid` comment on
    /// `backfill_function_and_space_data` warns about. The pragma is
    /// SQLite's own documented knob for finding exactly this class of
    /// unstated-order assumption: it reverses a table/index scan that has
    /// no explicit `ORDER BY` of its own, and leaves one that does (this
    /// backfill's `ORDER BY rowid`) alone. Calls `migrate_v9_to_v10`
    /// directly, on the same connection the pragma is set on — through
    /// `open_and_migrate` the migration would run on a second, fresh
    /// connection the pragma never reached.
    #[test]
    fn the_function_and_space_backfill_scan_order_determines_which_manufacturer_name_survives() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let old_bytes = MASTER_OLD_SPELLING.as_bytes();
        let new_bytes = MASTER_NEW_SPELLING.as_bytes();
        let old_sha = crate::sha256_hex(old_bytes);
        let new_sha = crate::sha256_hex(new_bytes);
        let conn = Connection::open(&path).unwrap();
        for migration in &migrations()[0..9] {
            migration(&conn).unwrap();
        }
        // Insertion order fixes rowid order: `old_sha` first, `new_sha`
        // second, so a correct last-writer-wins replay must leave
        // "New Spelling" standing.
        for (sha, source_path, bytes) in [
            (&old_sha, "M-0042/A/knx_master.xml", old_bytes),
            (&new_sha, "M-0042/B/knx_master.xml", new_bytes),
        ] {
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![sha, source_path, "M-0042", bytes.len() as i64, bytes],
            )
            .unwrap();
        }
        conn.execute_batch("PRAGMA reverse_unordered_selects = ON;")
            .unwrap();
        migrate_v9_to_v10(&conn).unwrap();
        let name: String = conn
            .query_row(
                "SELECT name FROM manufacturer WHERE id = 'M-0042'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            name, "New Spelling",
            "last-writer-wins requires the backfill to replay blobs in a fixed, \
             recorded order (rowid), not whatever order SQLite's scan happens to prefer"
        );
    }

    const MASTER_TRANSLATION_FIRST: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Siemens" />
    </Manufacturers>
  </MasterData>
  <Languages>
    <Language Identifier="de-DE">
      <TranslationUnit RefId="LOC-1">
        <TranslationElement RefId="LOC-1">
          <Translation AttributeName="Text" Text="First Text" />
        </TranslationElement>
      </TranslationUnit>
    </Language>
  </Languages>
</KNX>"#;

    const MASTER_TRANSLATION_SECOND: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0002" Name="MDT" />
    </Manufacturers>
  </MasterData>
  <Languages>
    <Language Identifier="de-DE">
      <TranslationUnit RefId="LOC-1">
        <TranslationElement RefId="LOC-1">
          <Translation AttributeName="Text" Text="Second Text" />
        </TranslationElement>
      </TranslationUnit>
    </Language>
  </Languages>
</KNX>"#;

    /// Regression guard for `backfill_shared_translations`'s `ORDER BY
    /// rowid` (T13 fix round 2). Two `knx_master.xml` blobs both declare
    /// `Master`-scope text for the same `(language, RefId, AttributeName)`
    /// key; `ingest_translations`' write is `INSERT OR IGNORE` —
    /// first-writer-wins, the mirror image of the manufacturer-name test
    /// above — so the surviving text must be the blob inserted *first*,
    /// i.e. the one at the lower `rowid`. Same
    /// `PRAGMA reverse_unordered_selects` mechanism as that test, and the
    /// same reason it is needed: a plain two-insert setup cannot tell
    /// "with `ORDER BY rowid`" apart from "without" it, because an
    /// unordered table scan already comes back in rowid order in practice
    /// on a table nothing has deleted from. Calls `migrate_v3_to_v4`
    /// directly, on the same connection the pragma is set on.
    #[test]
    fn the_translation_backfill_scan_order_determines_which_conflicting_text_survives() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("products.sqlite");
        let first_bytes = MASTER_TRANSLATION_FIRST.as_bytes();
        let second_bytes = MASTER_TRANSLATION_SECOND.as_bytes();
        let first_sha = crate::sha256_hex(first_bytes);
        let second_sha = crate::sha256_hex(second_bytes);
        let conn = Connection::open(&path).unwrap();
        for migration in &migrations()[0..3] {
            migration(&conn).unwrap();
        }
        // Insertion order fixes rowid order: `first_sha` first,
        // `second_sha` second, so a correct first-writer-wins replay
        // must leave "First Text" standing.
        for (sha, source_path, bytes) in [
            (&first_sha, "M-0001/knx_master.xml", first_bytes),
            (&second_sha, "M-0002/knx_master.xml", second_bytes),
        ] {
            conn.execute(
                "INSERT INTO source_file (sha256, source_path, manufacturer_id, len, bytes)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
                params![sha, source_path, None::<String>, bytes.len() as i64, bytes],
            )
            .unwrap();
        }
        conn.execute_batch("PRAGMA reverse_unordered_selects = ON;")
            .unwrap();
        migrate_v3_to_v4(&conn).unwrap();
        let text: String = conn
            .query_row(
                "SELECT text FROM translation
                 WHERE scope = 'Master' AND ref_id = 'LOC-1' AND attribute_name = 'Text'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            text, "First Text",
            "first-writer-wins requires the backfill to replay blobs in a fixed, \
             recorded order (rowid), not whatever order SQLite's scan happens to prefer"
        );
    }
}
