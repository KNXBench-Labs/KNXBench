//! Rewinds a v16 database to a genuine v15 one, for migration tests.
//!
//! Drops the PDB-10 baggage inventory tables and restores the v15 report
//! shape: the `baggage_index`/`unsupported` count row, the
//! `unsupported-baggage-index` diagnostic per index member (its v15 path,
//! detail and ordinal), and the v15 diagnostic CHECK list. The v15 index
//! parser reported no unknowns, so a fixture whose index has any is refused
//! rather than half-rewound.
//!
//! A rewind starts at the current schema, so the v17 identity tables
//! (ADR-0043) are dropped first; v15 and v16 had neither.

use rusqlite::Connection;

/// The v17 tables, gone before any v16-or-older shape is restored.
fn drop_v17_tables(conn: &Connection) {
    conn.execute_batch(
        "DROP TABLE IF EXISTS source_identity;
         DROP TABLE IF EXISTS source_identity_scan;
         DROP TABLE IF EXISTS source_producer;
         DROP TABLE IF EXISTS package_source_name;",
    )
    .unwrap();
}

#[allow(dead_code)]
pub const V15_BAGGAGE_DETAIL: &str =
    "baggage index declarations are counted but not typed until PDB-10";

#[allow(dead_code)]
pub fn rewind_to_v15(conn: &Connection) {
    drop_v17_tables(conn);
    let index_unknowns: i64 = conn
        .query_row(
            "SELECT (SELECT count(*) FROM ingest_unknown WHERE xpath LIKE '/KNX/ManufacturerData/Manufacturer/Baggages%')
                  + (SELECT count(*) FROM package_install_unknown WHERE xpath LIKE '/KNX/ManufacturerData/Manufacturer/Baggages%')",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        index_unknowns, 0,
        "fixture has index unknowns v15 never wrote"
    );
    conn.execute_batch(&format!(
        "CREATE TABLE package_install_diagnostic_v15 (
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
        INSERT INTO package_install_diagnostic_v15
            SELECT * FROM package_install_diagnostic
            WHERE kind NOT IN ('unresolved-baggage-declaration', 'undeclared-baggage-payload');
        -- v15 sorted `unsupported-baggage-index` last, after every
        -- surviving kind, ordered by archive path: append in that order.
        INSERT INTO package_install_diagnostic_v15
            SELECT d.package_sha256,
                   (SELECT count(*) FROM package_install_diagnostic_v15 AS o
                     WHERE o.package_sha256 = d.package_sha256)
                   + (SELECT count(DISTINCT e.index_path) FROM package_baggage_declaration AS e
                       WHERE e.package_sha256 = d.package_sha256 AND e.index_path < d.index_path),
                   'unsupported-baggage-index', d.index_path,
                   '/KNX/ManufacturerData/Manufacturer/Baggages/Baggage', '{V15_BAGGAGE_DETAIL}',
                   count(*)
            FROM package_baggage_declaration AS d
            JOIN package_install_report AS r
              ON r.package_sha256 = d.package_sha256 AND r.status = 'measured'
            GROUP BY d.package_sha256, d.index_path;
        DROP TABLE package_install_diagnostic;
        ALTER TABLE package_install_diagnostic_v15 RENAME TO package_install_diagnostic;
        UPDATE package_install_count SET disposition = 'unsupported'
            WHERE category = 'baggage_index' AND disposition = 'stored';
        DROP TABLE package_baggage_declaration;
        DROP TABLE package_baggage_payload;
        DROP TABLE package_baggage_inventory;
        PRAGMA user_version = 15;"
    ))
    .unwrap();
}

/// For fixtures rolled back far below v15, whose report tables are dropped
/// wholesale: only the v16 tables need removing.
#[allow(dead_code)]
pub fn drop_baggage_inventory_tables(conn: &Connection) {
    drop_v17_tables(conn);
    conn.execute_batch(
        "DROP TABLE package_baggage_declaration;
         DROP TABLE package_baggage_payload;
         DROP TABLE package_baggage_inventory;",
    )
    .unwrap();
}
