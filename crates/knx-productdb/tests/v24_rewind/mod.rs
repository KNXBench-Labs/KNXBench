//! Rewinds a v25 install of the `placement_columns` fixture to a genuine v24 database.
//!
//! v25 stores union-member `Offset`/`BitOffset`, `Parameter/Property` placements
//! and four `Enumeration` attributes in columns, retires their unknown rows,
//! and starts reporting the `Enumeration` attributes no column stores. For the
//! fixture's one program blob, a v24 parser instead reported:
//! - `Offset` and `BitOffset` on both union members;
//! - the `Property` element on two parameters, and all its attributes
//!   (`PropertyId` twice, first value `78`, not only the unreadable `x51`);
//! - nothing on `Enumeration` (so no `Fancy` row).
//!
//! The rewind drops the columns, applies exactly that difference to
//! `ingest_unknown` and to the measured report, and raises
//! `package.unknown_count` by the distinct rows it adds.

use rusqlite::{params, Connection};

const STATIC: &str =
    "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static";

/// `(xpath below Static, kind, name, occurrences, sample)`.
type Row = (
    &'static str,
    &'static str,
    &'static str,
    i64,
    Option<&'static str>,
);

/// Rows a v24 parser wrote that v25 does not, with v25's own `PropertyId`
/// row replaced by v24's.
const V24_ONLY: &[Row] = &[
    (
        "/Parameters/Union/Parameter",
        "Attribute",
        "Offset",
        2,
        Some("0"),
    ),
    (
        "/Parameters/Union/Parameter",
        "Attribute",
        "BitOffset",
        2,
        Some("0"),
    ),
    ("/Parameters/Parameter", "Element", "Property", 2, None),
    (
        "/Parameters/Parameter/Property",
        "Attribute",
        "ObjectIndex",
        1,
        Some("0"),
    ),
    (
        "/Parameters/Parameter/Property",
        "Attribute",
        "ObjectType",
        1,
        Some("9"),
    ),
    (
        "/Parameters/Parameter/Property",
        "Attribute",
        "PropertyId",
        2,
        Some("78"),
    ),
    (
        "/Parameters/Parameter/Property",
        "Attribute",
        "Offset",
        2,
        Some("0"),
    ),
    (
        "/Parameters/Parameter/Property",
        "Attribute",
        "BitOffset",
        2,
        Some("0"),
    ),
    (
        "/Parameters/Parameter/Property",
        "Attribute",
        "Occurrence",
        1,
        Some("0"),
    ),
];

/// Rows v25 writes that a v24 parser did not.
const V25_ONLY: &[(&str, &str, &str)] = &[
    (
        "/ParameterTypes/ParameterType/TypeRestriction/Enumeration",
        "Attribute",
        "Fancy",
    ),
    ("/Parameters/Parameter/Property", "Attribute", "PropertyId"),
];

fn delete_v25_only(conn: &Connection, table: &str, key_column: &str, key: &str) -> i64 {
    let mut deleted = 0;
    for (xpath, kind, name) in V25_ONLY {
        deleted += conn
            .execute(
                &format!(
                    "DELETE FROM {table} WHERE {key_column} = ?1 AND xpath = ?2
                     AND kind = ?3 AND name = ?4"
                ),
                params![key, format!("{STATIC}{xpath}"), kind, name],
            )
            .unwrap() as i64;
    }
    deleted
}

#[allow(dead_code)]
pub fn rewind_to_v24(conn: &Connection) {
    super::v20_rewind::drop_v25_objects(conn);
    let blob: String = conn
        .query_row(
            "SELECT source_sha256 FROM application_program WHERE id = 'A-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    delete_v25_only(conn, "ingest_unknown", "source_sha256", &blob);
    for (xpath, kind, name, occurrences, sample) in V24_ONLY {
        conn.execute(
            "INSERT INTO ingest_unknown (source_sha256, program_id, xpath, kind, name, occurrences, sample)
             VALUES (?1, NULL, ?2, ?3, ?4, ?5, ?6)",
            params![blob, format!("{STATIC}{xpath}"), kind, name, occurrences, sample],
        )
        .unwrap();
    }

    let package: String = conn
        .query_row(
            "SELECT package_sha256 FROM package_member WHERE source_sha256 = ?1",
            [&blob],
            |r| r.get(0),
        )
        .unwrap();
    let removed = delete_v25_only(conn, "package_install_unknown", "package_sha256", &package);
    conn.execute(
        "UPDATE package SET unknown_count = unknown_count + ?2 WHERE sha256 = ?1",
        params![package, V24_ONLY.len() as i64 - removed],
    )
    .unwrap();
    let mut rows: Vec<(String, String, String, i64, Option<String>)> = conn
        .prepare(
            "SELECT xpath, kind, name, occurrences, sample FROM package_install_unknown
             WHERE package_sha256 = ?1",
        )
        .unwrap()
        .query_map([&package], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    for (xpath, kind, name, occurrences, sample) in V24_ONLY {
        rows.push((
            format!("{STATIC}{xpath}"),
            kind.to_string(),
            name.to_string(),
            *occurrences,
            sample.map(str::to_string),
        ));
    }
    rows.sort_by(|a, b| (&a.0, &a.1, &a.2).cmp(&(&b.0, &b.1, &b.2)));
    conn.execute(
        "DELETE FROM package_install_unknown WHERE package_sha256 = ?1",
        [&package],
    )
    .unwrap();
    for (ordinal, (xpath, kind, name, occurrences, sample)) in rows.iter().enumerate() {
        conn.execute(
            "INSERT INTO package_install_unknown VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                package,
                ordinal as i64,
                xpath,
                kind,
                name,
                occurrences,
                sample
            ],
        )
        .unwrap();
    }
    let distinct = rows.len() as i64;
    let total: i64 = rows.iter().map(|r| r.3).sum();
    conn.execute(
        "UPDATE package_install_report SET unknown_distinct = ?2, unknown_occurrences = ?3
         WHERE package_sha256 = ?1",
        params![package, distinct, total],
    )
    .unwrap();
    for (disposition, count) in [("read", total), ("stored", distinct)] {
        conn.execute(
            "UPDATE package_install_count SET count = ?3
             WHERE package_sha256 = ?1 AND category = 'unknown_construct' AND disposition = ?2",
            params![package, disposition, count],
        )
        .unwrap();
    }
    conn.execute_batch("PRAGMA user_version = 24;").unwrap();
}
