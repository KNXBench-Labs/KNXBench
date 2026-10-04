//! Synthetic existing ZIP member-count boundaries; no limit or compatibility expansion.
use std::io::{Cursor, Write};

use knx_productdb::{install_package, open_and_migrate, PackageError};
use rusqlite::{types::Value, Connection};
use zip::write::SimpleFileOptions;

// Public contract of the existing product-package preflight, not a proposed raise.
const MEMBER_LIMIT: usize = 4096;
const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"><Products><Product Id="P-1" Text="Example"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn archive_with_entries(count: usize) -> Vec<u8> {
    assert!(count >= 2);
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ] {
        writer
            .start_file(path, SimpleFileOptions::default())
            .unwrap();
        writer.write_all(bytes).unwrap();
    }
    for index in 2..count {
        writer
            .add_directory(format!("empty-{index}/"), SimpleFileOptions::default())
            .unwrap();
    }
    let bytes = writer.finish().unwrap().into_inner();
    assert_eq!(
        zip::ZipArchive::new(Cursor::new(&bytes)).unwrap().len(),
        count,
        "fixture must contain the actual boundary entry count"
    );
    bytes
}

fn db() -> (tempfile::TempDir, Connection) {
    let directory = tempfile::tempdir().unwrap();
    let connection = open_and_migrate(&directory.path().join("products.sqlite")).unwrap();
    (directory, connection)
}

fn contents(connection: &Connection) -> Vec<(String, Vec<Vec<Value>>)> {
    let tables = connection
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    tables
        .into_iter()
        .map(|table| {
            let query = format!("SELECT * FROM \"{}\"", table.replace('"', "\"\""));
            let statement = connection.prepare(&query).unwrap();
            let width = statement.column_count();
            let ordering = (1..=width)
                .map(|index| index.to_string())
                .collect::<Vec<_>>()
                .join(",");
            let mut statement = connection
                .prepare(&format!("{query} ORDER BY {ordering}"))
                .unwrap();
            let rows = statement
                .query_map([], |row| {
                    (0..width)
                        .map(|index| row.get::<_, Value>(index))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (table, rows)
        })
        .collect()
}

fn retained_archive(connection: &Connection, digest: &str) -> Vec<u8> {
    connection
        .query_row(
            "SELECT bytes FROM package WHERE sha256=?1",
            [digest],
            |row| row.get(0),
        )
        .unwrap()
}

#[test]
fn exact_member_count_cap_installs_and_retains_original_archive_on_replay() {
    let (_directory, connection) = db();
    let bytes = archive_with_entries(MEMBER_LIMIT);
    let report = install_package(&connection, "exact-cap.knxprod", &bytes)
        .expect("existing member-count cap is inclusive");
    assert_eq!(report.scheme, 11);
    assert_eq!(
        report.members.len(),
        2,
        "empty directories are not payloads"
    );
    assert_eq!(retained_archive(&connection, &report.sha256), bytes);
    let before = contents(&connection);
    install_package(&connection, "exact-cap.knxprod", &bytes).unwrap();
    assert_eq!(
        contents(&connection),
        before,
        "replay must not alter any row"
    );
    assert_eq!(retained_archive(&connection, &report.sha256), bytes);
}

#[test]
fn one_entry_over_member_count_cap_preserves_every_seeded_database_value() {
    let (_directory, connection) = db();
    let seed = archive_with_entries(2);
    let seed_report = install_package(&connection, "seed.knxprod", &seed).unwrap();
    assert_eq!(retained_archive(&connection, &seed_report.sha256), seed);
    let before = contents(&connection);
    let bytes = archive_with_entries(MEMBER_LIMIT + 1);
    let error = install_package(&connection, "over-cap.knxprod", &bytes).unwrap_err();
    assert!(
        matches!(error, PackageError::SizeLimit { .. }),
        "expected the entry-count resource boundary, got {error}"
    );
    assert_eq!(
        contents(&connection),
        before,
        "refusal must preserve all rows"
    );
    assert_eq!(retained_archive(&connection, &seed_report.sha256), seed);
}
