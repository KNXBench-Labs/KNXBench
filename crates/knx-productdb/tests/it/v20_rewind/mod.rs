//! Removes the schema-v20 objects so an older-version fixture is genuine.
//!
//! Removes the schema-v20 objects (ADR-0080) so an older-version fixture is
//! genuine: `migrate_v19_to_v20` would otherwise fail on a column that is
//! already there. Idempotent, because some fixtures rewind through more than
//! one helper. Every rewind below v20 is also below v22, so the v22 objects
//! go first.

use rusqlite::Connection;

/// Removes the schema-v26 objects (ADR-0146): the two `ingest_unknown`
/// identity indexes, so an older migration may write the repeated rows it
/// always wrote and `migrate_v25_to_v26` can create them again.
#[allow(dead_code)]
pub fn drop_v26_objects(conn: &Connection) {
    conn.execute_batch(
        "DROP INDEX IF EXISTS ingest_unknown_construct_identity;
         DROP INDEX IF EXISTS ingest_unknown_conflict_identity;",
    )
    .unwrap();
}

/// Removes the schema-v25 placement and enumeration columns, so that
/// `migrate_v24_to_v25` finds no column already there. Only the columns: a
/// rewind that needs a genuine v24 report uses `v24_rewind::rewind_to_v24`.
#[allow(dead_code)]
pub fn drop_v25_objects(conn: &Connection) {
    drop_v26_objects(conn);
    for (table, column) in [
        ("parameter", "own_offset"),
        ("parameter", "own_bit_offset"),
        ("parameter", "property_object_index"),
        ("parameter", "property_object_type"),
        ("parameter", "property_id"),
        ("parameter", "property_offset"),
        ("parameter", "property_bit_offset"),
        ("parameter", "property_occurrence"),
        ("parameter_type_enum", "binary_value"),
        ("parameter_type_enum", "name"),
        ("parameter_type_enum", "icon"),
        ("parameter_type_enum", "picture_alignment"),
    ] {
        let present: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
                [table, column],
                |r| r.get(0),
            )
            .unwrap();
        if present {
            conn.execute_batch(&format!("ALTER TABLE {table} DROP COLUMN {column};"))
                .unwrap();
        }
    }
}

/// Removes the schema-v24 `access` and `ui_hint` columns (ADR-0139), and the
/// newer ones first, so that `migrate_v23_to_v24` finds no column already
/// there. Only the columns: a rewind that needs a genuine v23 report uses
/// `v23_rewind::rewind_to_v23`.
#[allow(dead_code)]
pub fn drop_v24_objects(conn: &Connection) {
    drop_v25_objects(conn);
    for column in ["access", "ui_hint"] {
        let present: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info('dynamic_node') WHERE name = ?1)",
                [column],
                |r| r.get(0),
            )
            .unwrap();
        if present {
            conn.execute_batch(&format!("ALTER TABLE dynamic_node DROP COLUMN {column};"))
                .unwrap();
        }
    }
}

/// Removes the schema-v23 `text_parameter_ref_id` columns (ADR-0126), so that
/// `migrate_v22_to_v23` finds no column already there. Only the columns: a
/// rewind that needs a genuine v22 report uses `v22_rewind::rewind_to_v22`.
#[allow(dead_code)]
pub fn drop_v23_objects(conn: &Connection) {
    drop_v24_objects(conn);
    for table in ["dynamic_node", "com_object_ref"] {
        let present: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = 'text_parameter_ref_id')",
                [table],
                |r| r.get(0),
            )
            .unwrap();
        if present {
            conn.execute_batch(&format!(
                "ALTER TABLE {table} DROP COLUMN text_parameter_ref_id;"
            ))
            .unwrap();
        }
    }
}

/// Removes the schema-v22 legacy provenance tables (ADR-0094), so that
/// `migrate_v21_to_v22` finds a genuine v21 database.
#[allow(dead_code)]
pub fn drop_v22_objects(conn: &Connection) {
    drop_v23_objects(conn);
    conn.execute_batch(
        "DROP TABLE IF EXISTS legacy_diagnostic;
         DROP TABLE IF EXISTS legacy_program;
         DROP TABLE IF EXISTS legacy_source_file;
         DROP TABLE IF EXISTS legacy_source;",
    )
    .unwrap();
}

#[allow(dead_code)]
pub fn drop_v20_objects(conn: &Connection) {
    drop_v22_objects(conn);
    conn.execute_batch(
        "DROP INDEX IF EXISTS parameter_calculation_ref_member;
         DROP TABLE IF EXISTS parameter_calculation_ref;",
    )
    .unwrap();
    for (table, column) in [
        ("parameter_ref", "access"),
        ("application_program", "write_authority_recorded"),
    ] {
        let present: bool = conn
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM pragma_table_info(?1) WHERE name = ?2)",
                [table, column],
                |r| r.get(0),
            )
            .unwrap();
        if present {
            conn.execute_batch(&format!("ALTER TABLE {table} DROP COLUMN {column};"))
                .unwrap();
        }
    }
}
