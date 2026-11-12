//! Exact native snapshot codec and versioned semantic fingerprints.

use super::{HistoryError, MAX_CONTEXT_BYTES, MAX_IMAGE_BYTES};
use crate::{ManufacturerRef, StoredOpaqueEntry};
use knx_core::Project;
use rusqlite::{Connection, MAIN_DB};
use sha2::{Digest, Sha256};

#[cfg(test)]
thread_local! {
    /// In-memory native stores built on this thread, as (carrying a retained
    /// context, model only): what a write encoded or hashed afresh.
    pub(super) static BUILT_STORES: std::cell::Cell<(usize, usize)> =
        const { std::cell::Cell::new((0, 0)) };
}

/// A complete native semantic project, not a global catalogue or hardware backup.
#[derive(Debug, Clone, PartialEq)]
pub struct NativeSnapshot {
    pub project: Project,
    pub opaque: Vec<StoredOpaqueEntry>,
    pub manufacturer_refs: Vec<ManufacturerRef>,
}

impl NativeSnapshot {
    pub fn read(conn: &Connection) -> Result<Self, HistoryError> {
        Ok(Self {
            project: crate::load_project(conn)?,
            opaque: crate::load_opaque(conn)?,
            manufacturer_refs: crate::load_manufacturer_refs(conn)?,
        })
    }

    fn memory_store(&self) -> Result<Connection, HistoryError> {
        if self.project.schema_version != knx_core::project::CURRENT_SCHEMA_VERSION {
            return Err(HistoryError::Invalid("unsupported model version"));
        }
        #[cfg(test)]
        BUILT_STORES.with(|built| {
            let (context, model) = built.get();
            built.set(
                if self.opaque.is_empty() && self.manufacturer_refs.is_empty() {
                    (context, model + 1)
                } else {
                    (context + 1, model)
                },
            );
        });
        let conn = crate::open_and_migrate_in_memory()
            .map_err(|_| HistoryError::Invalid("snapshot store initialization failed"))?;
        // The fresh store has no preceding project/history. Plain-save hooks
        // must leave these history tables empty to prevent recursive snapshots.
        crate::save_project_with_passthrough(
            &conn,
            &self.project,
            &self.opaque,
            &self.manufacturer_refs,
        )?;
        if Self::read(&conn)? != *self {
            return Err(HistoryError::Invalid("native snapshot would not reopen exactly; repair allocator metadata or unsupported model state first"));
        }
        Ok(conn)
    }

    pub fn encode(&self) -> Result<Vec<u8>, HistoryError> {
        let conn = self.memory_store()?;
        let bytes = conn.serialize(MAIN_DB)?;
        super::image_codec::encode(&bytes, MAX_IMAGE_BYTES)
    }

    pub(super) fn encode_with_identity(&self) -> Result<(Vec<u8>, String), HistoryError> {
        let conn = self.memory_store()?;
        let identity = semantic_store_hash(&conn)?;
        let bytes = conn.serialize(MAIN_DB)?;
        let image = super::image_codec::encode(&bytes, MAX_CONTEXT_BYTES)?;
        Ok((image, identity))
    }

    /// Canonical semantic identity, independent of SQLite page layout and
    /// writer-library header bytes. Only a freshly constructed internal store
    /// supplies SQL identifiers; user database schema is never interpolated.
    pub fn semantic_hash(&self) -> Result<String, HistoryError> {
        semantic_store_hash(&self.memory_store()?)
    }

    pub fn decode(bytes: &[u8], expected_hash: &str) -> Result<Self, HistoryError> {
        Self::decode_with_limit(bytes, expected_hash, MAX_IMAGE_BYTES)
    }

    pub(super) fn decode_context(bytes: &[u8], expected_hash: &str) -> Result<Self, HistoryError> {
        Self::decode_with_limit(bytes, expected_hash, MAX_CONTEXT_BYTES)
    }

    fn decode_with_limit(
        bytes: &[u8],
        expected_hash: &str,
        stored_limit: usize,
    ) -> Result<Self, HistoryError> {
        if bytes.len() > stored_limit {
            return Err(HistoryError::ImageLimit(
                "encoded native snapshot exceeds its model/context budget",
            ));
        }
        if image_hash(bytes) != expected_hash {
            return Err(HistoryError::Invalid(
                "snapshot length, header or SHA-256 mismatch",
            ));
        }
        let decoded = super::image_codec::decode(bytes, stored_limit)?;
        let bytes = decoded.as_ref();
        if bytes.len() < 100 || !bytes.starts_with(b"SQLite format 3\0") {
            return Err(HistoryError::Invalid(
                "invalid decoded SQLite snapshot header",
            ));
        }
        let mut conn = Connection::open_in_memory()?;
        // All conversion is confined to this owned in-memory database. Original
        // bytes/hash are verified above; unknown shape must refuse before DDL.
        conn.deserialize_read_exact(MAIN_DB, bytes, bytes.len(), false)?;
        conn.execute_batch("PRAGMA trusted_schema = OFF; PRAGMA foreign_keys = ON;")?;
        let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if !ADMITTED_VERSIONS.contains(&version) {
            return Err(HistoryError::Invalid("unsupported snapshot schema version"));
        }
        let creator: String = conn.query_row(
            "SELECT value FROM schema_meta WHERE key = 'created_by'",
            [],
            |r| r.get(0),
        )?;
        if creator != "knx-store" {
            return Err(HistoryError::Invalid("snapshot is not a native project"));
        }
        let integrity: String = conn.query_row("PRAGMA quick_check", [], |r| r.get(0))?;
        if integrity != "ok" {
            return Err(HistoryError::Invalid("snapshot SQLite integrity failure"));
        }
        let fk_failure = conn.prepare("PRAGMA foreign_key_check")?.exists([])?;
        if fk_failure {
            return Err(HistoryError::Invalid("snapshot has broken references"));
        }
        for table in [
            "project_history_context",
            "project_history_state",
            "project_history_stack",
            "project_history_version",
        ] {
            let count: i64 =
                conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?;
            if count != 0 {
                return Err(HistoryError::Invalid("recursive history snapshot"));
            }
        }
        // Carried manufacturer files belong to the file, never to a
        // snapshot (ADR-0128): an image holding any is not one we wrote.
        if version >= 16 {
            let carried: i64 =
                conn.query_row("SELECT count(*) FROM manufacturer_file", [], |r| r.get(0))?;
            if carried != 0 {
                return Err(HistoryError::Invalid(
                    "snapshot carries file-level manufacturer files",
                ));
            }
        }
        // Parameter baselines belong to the file, never to a snapshot
        // (ADR-0173): an image holding any is not one we wrote.
        if version >= 20 {
            for table in if version >= 21 {
                &crate::parameter_baseline::TABLES[..]
            } else {
                &crate::parameter_baseline::TABLES_V20[..]
            } {
                let rows: i64 =
                    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))?;
                if rows != 0 {
                    return Err(HistoryError::Invalid(
                        "snapshot carries file-level parameter baselines",
                    ));
                }
            }
        }
        check_schema_version(&conn, version)?;
        if version < crate::CURRENT_SCHEMA_VERSION {
            crate::migration::migrate(&conn)
                .map_err(|_| HistoryError::Invalid("predecessor snapshot upgrade refused"))?;
        }
        conn.execute_batch("PRAGMA query_only = ON;")?;
        let snapshot = Self::read(&conn)?;
        crate::representable::check_representable(&snapshot.project)?;
        Ok(snapshot)
    }
}

/// Native schema versions a stored image may have been written at.
const ADMITTED_VERSIONS: [i64; 10] = [
    12,
    13,
    14,
    15,
    16,
    17,
    18,
    19,
    20,
    crate::CURRENT_SCHEMA_VERSION,
];

pub fn image_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Compare with the actual current native migration, never an external-format
/// guess. Extra data objects/columns cannot silently disappear in a typed image.
pub(super) fn check_schema(conn: &Connection) -> Result<(), HistoryError> {
    check_schema_version(conn, crate::CURRENT_SCHEMA_VERSION)
}

fn check_schema_version(conn: &Connection, version: i64) -> Result<(), HistoryError> {
    type Columns = Vec<(String, String, i64, i64, i64)>;
    type Shape = Vec<(String, String, Columns)>;
    fn shape(conn: &Connection) -> Result<Shape, rusqlite::Error> {
        let objects = conn
            .prepare("SELECT type, name FROM sqlite_schema WHERE type IN ('table', 'view', 'trigger') AND name NOT GLOB 'sqlite_*' ORDER BY type, name")?
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        objects.into_iter().map(|(kind, name)| {
            let columns = conn
                .prepare("SELECT name, type, \"notnull\", pk, hidden FROM pragma_table_xinfo(?1) ORDER BY cid")?
                .query_map([&name], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)))?
                .collect::<Result<Columns, _>>()?;
            Ok((kind, name, columns))
        }).collect()
    }
    let expected = crate::open_and_migrate_in_memory()
        .map_err(|_| HistoryError::Invalid("native schema admission initialization failed"))?;
    if !ADMITTED_VERSIONS.contains(&version) {
        return Err(HistoryError::Invalid("unsupported native schema shape"));
    }
    if version < 21 {
        expected.execute_batch("DROP TABLE device_object_flags_baseline; DROP TABLE device_link_baseline_link; DROP TABLE device_link_baseline; ALTER TABLE device_parameter_baseline DROP COLUMN object_flags_known;")?;
    }
    if version < 20 {
        expected.execute_batch(
            "DROP TABLE device_parameter_baseline_value; DROP TABLE device_parameter_baseline;",
        )?;
    }
    if version < 19 {
        expected.execute_batch("DROP TABLE group_address_description;")?;
    }
    if version < 18 {
        expected.execute_batch("DROP TABLE com_object_priority_edit;")?;
    }
    if version < 17 {
        expected.execute_batch("DROP TABLE com_object_saved_priority;")?;
    }
    if version < 16 {
        expected.execute_batch("DROP TABLE manufacturer_file;")?;
    }
    if version < 15 {
        expected.execute_batch(
            "DROP TABLE project_function_member; DROP TABLE project_function; \
             DROP TABLE project_function_state;",
        )?;
    }
    if version < 14 {
        expected.execute_batch("DROP TABLE project_note; DROP TABLE project_note_state;")?;
    }
    if version == 12 {
        expected.execute_batch("DROP TABLE instruction_run; DROP TABLE instruction_document;")?;
    }
    if shape(conn)? != shape(&expected)? {
        return Err(HistoryError::Invalid(
            "unsupported native tables, views, triggers or columns",
        ));
    }
    Ok(())
}

fn hash_bytes(hash: &mut Sha256, bytes: &[u8]) {
    hash.update((bytes.len() as u64).to_be_bytes());
    hash.update(bytes);
}

fn semantic_store_hash(conn: &Connection) -> Result<String, HistoryError> {
    use rusqlite::types::ValueRef;
    let mut hash = Sha256::new();
    hash.update(b"KNXBench semantic native snapshot v1\0");
    // An additive empty overlay must not invalidate authentic schema12
    // baseline/context fingerprints. A present empty document is NOT absent.
    let instructions: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM instruction_document)",
        [],
        |row| row.get(0),
    )?;
    let notes: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM project_note) OR EXISTS(SELECT 1 FROM project_note_state WHERE last_id!=0)", [], |row|row.get(0))?;
    // Same rule for Functions (ADR-0124): an older image without any keeps
    // its identity after the additive v15 migration.
    let functions: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM project_function) \
         OR EXISTS(SELECT 1 FROM project_function_state \
                   WHERE last_function_id!=0 OR last_member_id!=0)",
        [],
        |row| row.get(0),
    )?;
    let priorities: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM com_object_saved_priority)",
        [],
        |row| row.get(0),
    )?;
    // User priority edits (native 18): a store without any keeps its
    // native17-or-older identity.
    let edits: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM com_object_priority_edit)",
        [],
        |row| row.get(0),
    )?;
    // Group-address descriptions (native 19): same rule.
    let descriptions: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM group_address_description)",
        [],
        |row| row.get(0),
    )?;
    let identity_version: u32 = if descriptions {
        19
    } else if edits {
        18
    } else if priorities {
        17
    } else if functions {
        15
    } else if notes {
        14
    } else if instructions {
        13
    } else {
        12
    };
    hash.update(identity_version.to_be_bytes());
    let tables: Vec<String> = conn
        .prepare(
            "SELECT name FROM sqlite_schema WHERE type = 'table' ORDER BY name COLLATE BINARY",
        )?
        .query_map([], |row| row.get(0))?
        .collect::<Result<_, _>>()?;
    for name in tables {
        if name.starts_with("sqlite_")
            || name.starts_with("project_history_")
            || name == "schema_meta"
            // File-level carried bytes, never snapshot content (ADR-0128).
            || name == "manufacturer_file"
            // File-level download baselines, never snapshot content (ADR-0173).
            || crate::parameter_baseline::TABLES.contains(&name.as_str())
            || (!priorities && name == "com_object_saved_priority")
            || (!edits && name == "com_object_priority_edit")
            || (!descriptions && name == "group_address_description")
            || (!notes && matches!(name.as_str(), "project_note" | "project_note_state"))
            || (!functions
                && matches!(
                    name.as_str(),
                    "project_function" | "project_function_member" | "project_function_state"
                ))
            || (!instructions
                && matches!(name.as_str(), "instruction_document" | "instruction_run"))
        {
            continue;
        }
        hash_bytes(&mut hash, name.as_bytes());
        // Identifiers come exclusively from the fresh internal schema above.
        let quoted = &name;
        let columns = conn
            .prepare(&format!(r#"SELECT * FROM "{quoted}" LIMIT 0"#))?
            .column_count();
        let order = (1..=columns)
            .map(|index| format!("{index} COLLATE BINARY"))
            .collect::<Vec<_>>()
            .join(",");
        let mut stmt = conn.prepare(&format!(r#"SELECT * FROM "{quoted}" ORDER BY {order}"#))?;
        hash.update((columns as u64).to_be_bytes());
        for column in stmt.column_names() {
            hash_bytes(&mut hash, column.as_bytes());
        }
        let mut rows = stmt.query([])?;
        while let Some(row) = rows.next()? {
            hash.update([255]);
            for index in 0..columns {
                match row.get_ref(index)? {
                    ValueRef::Null => hash.update([0]),
                    ValueRef::Integer(value) => {
                        hash.update([1]);
                        hash.update(value.to_be_bytes());
                    }
                    ValueRef::Real(value) => {
                        hash.update([2]);
                        hash.update(value.to_bits().to_be_bytes());
                    }
                    ValueRef::Text(value) => {
                        hash.update([3]);
                        hash_bytes(&mut hash, value);
                    }
                    ValueRef::Blob(value) => {
                        hash.update([4]);
                        hash_bytes(&mut hash, value);
                    }
                }
            }
        }
        hash.update([254]);
    }
    Ok(format!("{:x}", hash.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_instance_priority_empty_overlay_preserves_legacy_identity_and_explicit_v16_shape() {
        let conn = crate::open_and_migrate_in_memory().unwrap();
        let snapshot = NativeSnapshot {
            project: Project::new(knx_core::Language("en".into())),
            opaque: vec![],
            manufacturer_refs: vec![],
        };
        crate::save_project(&conn, &snapshot.project).unwrap();
        let current = semantic_store_hash(&conn).unwrap();
        conn.execute_batch("DROP TABLE device_object_flags_baseline; DROP TABLE device_link_baseline_link; DROP TABLE device_link_baseline; DROP TABLE device_parameter_baseline_value; DROP TABLE device_parameter_baseline; DROP TABLE group_address_description; DROP TABLE com_object_priority_edit; DROP TABLE com_object_saved_priority; PRAGMA user_version=16;")
            .unwrap();
        check_schema_version(&conn, 16).unwrap();
        let bytes = conn.serialize(MAIN_DB).unwrap().to_vec();
        let decoded = NativeSnapshot::decode(&bytes, &image_hash(&bytes)).unwrap();
        assert_eq!(decoded.semantic_hash().unwrap(), current);
        let mut invalid = Connection::open_in_memory().unwrap();
        invalid
            .deserialize_read_exact(MAIN_DB, bytes.as_slice(), bytes.len(), false)
            .unwrap();
        invalid
            .execute_batch("CREATE TABLE unknown_overlay (id INTEGER) STRICT;")
            .unwrap();
        assert!(check_schema_version(&invalid, 16).is_err());
    }

    fn empty_snapshot() -> NativeSnapshot {
        NativeSnapshot {
            project: Project::new(knx_core::Language("en".into())),
            opaque: vec![],
            manufacturer_refs: vec![],
        }
    }

    const BASELINE_ROWS: &str =
        "INSERT INTO device_parameter_baseline (device_id, program_id, at) VALUES (1, 'P', '2026-11-11T10:00:00Z');
         INSERT INTO device_parameter_baseline_value VALUES (1, 'P_R-1', '7');";

    #[test]
    fn a_native19_image_without_baseline_tables_decodes_with_its_identity() {
        let snapshot = empty_snapshot();
        let conn = snapshot.memory_store().unwrap();
        let current = semantic_store_hash(&conn).unwrap();
        conn.execute_batch(
            "DROP TABLE device_object_flags_baseline; DROP TABLE device_link_baseline_link; DROP TABLE device_link_baseline; DROP TABLE device_parameter_baseline_value; DROP TABLE device_parameter_baseline; \
             PRAGMA user_version=19;",
        )
        .unwrap();
        check_schema_version(&conn, 19).unwrap();
        assert!(
            check_schema_version(&conn, crate::CURRENT_SCHEMA_VERSION).is_err(),
            "a native20 image must hold both baseline tables"
        );
        let bytes = conn.serialize(MAIN_DB).unwrap().to_vec();
        let decoded = NativeSnapshot::decode(&bytes, &image_hash(&bytes)).unwrap();
        assert_eq!(decoded, snapshot);
        assert_eq!(decoded.semantic_hash().unwrap(), current);
    }

    #[test]
    fn baseline_rows_never_enter_a_semantic_identity_and_an_image_carrying_them_is_refused() {
        let snapshot = empty_snapshot();
        let conn = snapshot.memory_store().unwrap();
        let empty = semantic_store_hash(&conn).unwrap();
        let clean = conn.serialize(MAIN_DB).unwrap().to_vec();
        conn.execute_batch(BASELINE_ROWS).unwrap();
        assert_eq!(semantic_store_hash(&conn).unwrap(), empty);
        let carrying = conn.serialize(MAIN_DB).unwrap().to_vec();
        assert!(NativeSnapshot::decode(&clean, &image_hash(&clean)).is_ok());
        let refused = NativeSnapshot::decode(&carrying, &image_hash(&carrying));
        assert!(
            matches!(
                refused,
                Err(HistoryError::Invalid(
                    "snapshot carries file-level parameter baselines"
                ))
            ),
            "{refused:?}"
        );
    }

    #[test]
    fn semantic_digest_orders_column_values_not_physical_row_insertion() {
        let snapshot = NativeSnapshot {
            project: Project::new(knx_core::Language("en".into())),
            opaque: vec![],
            manufacturer_refs: vec![],
        };
        let first = snapshot.memory_store().unwrap();
        let second = snapshot.memory_store().unwrap();
        for (conn, rows) in [
            (&first, "('z', 1), ('a', 2)"),
            (&second, "('a', 2), ('z', 1)"),
        ] {
            conn.execute_batch(&format!("CREATE TABLE canonical_witness (value TEXT, position INTEGER); INSERT INTO canonical_witness VALUES {rows};")).unwrap();
        }
        assert_eq!(
            semantic_store_hash(&first).unwrap(),
            semantic_store_hash(&second).unwrap()
        );
    }

    #[test]
    fn semantic_digest_ignores_page_layout_and_sqlite_header_housekeeping() {
        let snapshot = NativeSnapshot {
            project: Project::new(knx_core::Language("en".into())),
            opaque: vec![],
            manufacturer_refs: vec![],
        };
        let mut conn = snapshot.memory_store().unwrap();
        let original = semantic_store_hash(&conn).unwrap();
        let mut bytes = conn.serialize(MAIN_DB).unwrap().to_vec();
        let previous = image_hash(&bytes);
        bytes[99] ^= 1; // The documented SQLite writer-version header field.
        assert_ne!(image_hash(&bytes), previous);
        conn.deserialize_read_exact(MAIN_DB, bytes.as_slice(), bytes.len(), true)
            .unwrap();
        assert_eq!(semantic_store_hash(&conn).unwrap(), original);
        let mut changed = snapshot;
        changed.project.info.name = "Changed content".into();
        assert_ne!(changed.semantic_hash().unwrap(), original);
    }
    #[test]
    fn schema21_flag_and_link_evidence_never_enters_images_or_semantic_hashes() {
        for evidence in [
            "INSERT INTO device_link_baseline VALUES (1,'P','2026-11-12T00:00:00Z');",
            "INSERT INTO device_link_baseline VALUES (1,'P','2026-11-12T00:00:00Z'); INSERT INTO device_link_baseline_link VALUES (1,255,65535,1);",
            "INSERT INTO device_parameter_baseline VALUES (1,'P','2026-11-12T00:00:00Z',1);",
            "INSERT INTO device_parameter_baseline VALUES (1,'P','2026-11-12T00:00:00Z',1); INSERT INTO device_object_flags_baseline VALUES (1,255,1,0,1,0,1,NULL);",
        ] {
            let snapshot=empty_snapshot();
            let conn=snapshot.memory_store().unwrap();
            let before=semantic_store_hash(&conn).unwrap();
            conn.execute_batch(evidence).unwrap();
            assert_eq!(semantic_store_hash(&conn).unwrap(),before);
            let bytes=conn.serialize(MAIN_DB).unwrap().to_vec();
            assert!(matches!(NativeSnapshot::decode(&bytes,&image_hash(&bytes)),Err(HistoryError::Invalid("snapshot carries file-level parameter baselines"))));
        }
    }
}
