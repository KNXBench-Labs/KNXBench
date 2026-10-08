//! Removes the schema-v20 objects so an older-version fixture is genuine.
//!
//! Removes the schema-v20 objects (ADR-0080) so an older-version fixture is
//! genuine: `migrate_v19_to_v20` would otherwise fail on a column that is
//! already there. Idempotent, because some fixtures rewind through more than
//! one helper. Every rewind below v20 is also below v22, so the v22 objects
//! go first.

use rusqlite::Connection;

/// Removes the schema-v22 legacy provenance tables (ADR-0094), so that
/// `migrate_v21_to_v22` finds a genuine v21 database.
#[allow(dead_code)]
pub fn drop_v22_objects(conn: &Connection) {
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
