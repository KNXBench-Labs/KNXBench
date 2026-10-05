//! Rewinds a v17 database to a genuine v16 one, for migration tests.
//!
//! Drops the four PDB-11 identity tables and indexes (ADR-0043) and sets
//! `user_version = 16`. v16 kept nothing those tables hold except the
//! first `package.source_name`, which stays in `package`.

use rusqlite::Connection;

/// Removes only the v17 tables, for fixtures rolled back further by their
/// own helpers (which set `user_version` themselves).
#[allow(dead_code)]
pub fn drop_identity_tables(conn: &Connection) {
    super::v20_rewind::drop_v20_objects(conn);
    conn.execute_batch(
        "DROP TABLE source_identity;
         DROP TABLE source_identity_scan;
         DROP TABLE source_producer;
         DROP TABLE package_source_name;
         DROP INDEX catalog_section_source;
         DROP INDEX catalog_item_source;
         DROP INDEX hardware_source;
         DROP INDEX product_source;
         DROP INDEX hardware2program_source;
         DROP INDEX application_program_source;
         ALTER TABLE dynamic_node DROP COLUMN name;
         ALTER TABLE dynamic_node DROP COLUMN number;",
    )
    .unwrap();
}

#[allow(dead_code)]
pub fn rewind_to_v16(conn: &Connection) {
    drop_identity_tables(conn);
    conn.execute_batch("PRAGMA user_version = 16;").unwrap();
}
