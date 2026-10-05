//! Synthetic ZIP count, declared-size and raw-input bounds; no limit or compatibility expansion.
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

const DECLARED_MEMBER_BYTES: u32 = 64 * 1024 * 1024;
const DECLARED_TOTAL_BYTES: u64 = 256 * 1024 * 1024;

fn declared_baggage_path(index: usize) -> String {
    format!("M-0001/Baggages/declared-{index}.bin")
}

// APPNOTE 6.3.10 sections 4.3.7/4.3.12; ordinary headers, no descriptor/ZIP64.
// Only declarations are large: each real baggage body remains one byte.
fn archive_with_declared_baggage_sizes(sizes: &[u32]) -> Vec<u8> {
    assert!(!sizes.is_empty() && sizes.len() <= 4);
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for (name, payload) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(payload).unwrap();
    }
    for index in 0..sizes.len() {
        writer
            .start_file(declared_baggage_path(index), options)
            .unwrap();
        writer.write_all(b"x").unwrap();
    }
    let mut bytes = writer.finish().unwrap().into_inner();
    assert!(
        bytes.len() < 8 * 1024,
        "fixture must not allocate its declared size"
    );
    let offsets = {
        let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        (0..sizes.len())
            .map(|index| {
                let path = declared_baggage_path(index);
                let member = archive.by_name(&path).unwrap();
                assert_eq!(member.size(), 1);
                assert_eq!(member.compression(), zip::CompressionMethod::Deflated);
                (
                    usize::try_from(member.header_start()).unwrap(),
                    usize::try_from(member.central_header_start()).unwrap(),
                )
            })
            .collect::<Vec<_>>()
    };
    for ((local, central), size) in offsets.into_iter().zip(sizes) {
        assert_eq!(&bytes[local..local + 4], b"PK\x03\x04");
        assert_eq!(&bytes[central..central + 4], b"PK\x01\x02");
        // Bit 3 would put sizes in a data descriptor, outside this fixture.
        for flag_offset in [local + 6, central + 8] {
            let flags = u16::from_le_bytes(bytes[flag_offset..flag_offset + 2].try_into().unwrap());
            assert_eq!(flags & 8, 0);
        }
        const LOCAL_UNCOMPRESSED_SIZE: usize = 22;
        const CENTRAL_UNCOMPRESSED_SIZE: usize = 24;
        for offset in [
            local + LOCAL_UNCOMPRESSED_SIZE,
            central + CENTRAL_UNCOMPRESSED_SIZE,
        ] {
            assert_eq!(
                u32::from_le_bytes(bytes[offset..offset + 4].try_into().unwrap()),
                1
            );
            bytes[offset..offset + 4].copy_from_slice(&size.to_le_bytes());
        }
    }
    let mut archive = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    assert_eq!(archive.len(), 2 + sizes.len());
    let mut declared_total = 0_u64;
    for index in 0..archive.len() {
        declared_total = declared_total
            .checked_add(archive.by_index_raw(index).unwrap().size())
            .unwrap();
    }
    assert_eq!(
        declared_total,
        u64::try_from(MASTER.len() + HARDWARE.len()).unwrap()
            + sizes.iter().map(|size| u64::from(*size)).sum::<u64>()
    );
    for (index, size) in sizes.iter().enumerate() {
        assert_eq!(
            archive
                .by_name(&declared_baggage_path(index))
                .unwrap()
                .size(),
            u64::from(*size)
        );
    }
    drop(archive);
    bytes
}

fn refused_package_preserves_seed(source_name: &str, bytes: &[u8]) -> PackageError {
    let (_directory, connection) = db();
    let seed = archive_with_entries(2);
    let seed_report = install_package(&connection, "byte-seed.knxprod", &seed).unwrap();
    assert_eq!(retained_archive(&connection, &seed_report.sha256), seed);
    let before = contents(&connection);
    let error = install_package(&connection, source_name, bytes).unwrap_err();
    assert_eq!(
        contents(&connection),
        before,
        "byte admission must preserve every seeded value"
    );
    assert_eq!(retained_archive(&connection, &seed_report.sha256), seed);
    error
}

fn refused_declared_byte_fixture_preserves_seed(sizes: &[u32]) -> PackageError {
    let bytes = archive_with_declared_baggage_sizes(sizes);
    refused_package_preserves_seed("declared-size.knxprod", &bytes)
}

// Large only in raw slice length: intentionally invalid, no ZIP expansion.
const RAW_INPUT_BYTES: usize = 256 * 1024 * 1024;

#[test]
fn raw_input_at_package_limit_reaches_named_zip_preflight_error_without_mutation() {
    let bytes = vec![0xA5_u8; RAW_INPUT_BYTES];
    assert_eq!(bytes.len(), RAW_INPUT_BYTES);
    let error = refused_package_preserves_seed("raw-input-at-cap.knxprod", &bytes);
    assert!(
        matches!(error, PackageError::InvalidZip { ref cause } if cause == "missing complete end-of-directory record"),
        "inclusive raw-input bound must reach the named ZIP preflight error, got {error}"
    );
}

#[test]
fn raw_input_one_over_package_limit_preserves_caller_name_and_every_seeded_value() {
    let length = RAW_INPUT_BYTES.checked_add(1).unwrap();
    let bytes = vec![0xA5_u8; length];
    assert_eq!(bytes.len(), length);
    let source_name = "raw-input-over-cap.knxprod";
    let error = refused_package_preserves_seed(source_name, &bytes);
    assert!(
        matches!(error, PackageError::SizeLimit { ref path } if path == source_name),
        "raw-input over the existing bound must fail with the exact caller filename, got {error}"
    );
}

#[test]
fn declared_byte_member_at_limit_reaches_named_payload_mismatch_without_mutation() {
    let error = refused_declared_byte_fixture_preserves_seed(&[DECLARED_MEMBER_BYTES]);
    assert!(
        matches!(error, PackageError::InvalidZip { ref cause } if cause == &format!("size mismatch for {}", declared_baggage_path(0))),
        "inclusive declared member bound must reach the later named payload mismatch, got {error}"
    );
}

#[test]
fn declared_byte_member_one_over_limit_is_typed_and_preserves_every_seeded_value() {
    let error = refused_declared_byte_fixture_preserves_seed(&[DECLARED_MEMBER_BYTES + 1]);
    assert!(
        matches!(error, PackageError::SizeLimit { ref path } if path == &declared_baggage_path(0)),
        "declared member above the existing bound must fail at the named size limit, got {error}"
    );
}

fn declared_baggage_total_at_limit() -> [u32; 4] {
    let xml_sizes = u32::try_from(MASTER.len() + HARDWARE.len()).unwrap();
    let mut sizes = [DECLARED_MEMBER_BYTES; 4];
    sizes[3] = sizes[3].checked_sub(xml_sizes).unwrap();
    assert_eq!(
        u64::from(xml_sizes) + sizes.iter().map(|size| u64::from(*size)).sum::<u64>(),
        DECLARED_TOTAL_BYTES
    );
    sizes
}

#[test]
fn declared_byte_total_at_limit_reaches_named_payload_mismatch_without_mutation() {
    let error = refused_declared_byte_fixture_preserves_seed(&declared_baggage_total_at_limit());
    assert!(
        matches!(error, PackageError::InvalidZip { ref cause } if cause == &format!("size mismatch for {}", declared_baggage_path(0))),
        "inclusive declared total bound must reach the later named payload mismatch, got {error}"
    );
}

#[test]
fn declared_byte_total_one_over_limit_is_typed_and_preserves_every_seeded_value() {
    let mut sizes = declared_baggage_total_at_limit();
    sizes[3] = sizes[3].checked_add(1).unwrap();
    assert!(sizes.iter().all(|size| *size <= DECLARED_MEMBER_BYTES));
    let error = refused_declared_byte_fixture_preserves_seed(&sizes);
    assert!(
        matches!(error, PackageError::SizeLimit { ref path } if path == &declared_baggage_path(3)),
        "declared aggregate above the existing bound must fail at its last member, got {error}"
    );
}
