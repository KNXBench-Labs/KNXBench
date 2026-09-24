//! Verifies atomic standalone product-package ingestion and ZIP safety boundaries.

use std::io::{Cursor, Read, Write};
use std::path::PathBuf;

use knx_productdb::{install_package, open_and_migrate, PackageError};
use rusqlite::Connection;
use zip::write::{FullFileOptions, SimpleFileOptions};

const MASTER: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Example"/></Manufacturers></MasterData></KNX>"#;
const HARDWARE: &[u8] = br#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1" Name="Example"><Products><Product Id="P-1" Text="Example"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn db() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

#[test]
fn known_single_file_ingest_still_skips_inside_a_callers_transaction() {
    let (_dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-0001/Hardware.xml", HARDWARE).unwrap();
    let tx = conn.unchecked_transaction().unwrap();
    assert!(matches!(
        knx_productdb::ingest_file(&tx, "M-0001/Hardware.xml", HARDWARE),
        Ok(knx_productdb::IngestOutcome::Skipped { .. })
    ));
}

#[test]
fn a_raw_member_cannot_poison_the_manufacturer_parse_cache() {
    let (_dir, conn) = db();
    let later = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("H-1", "H-2")
        .replace("P-1", "P-2");
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("notes.xml", later.as_bytes()),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    install_package(&conn, "good.knxprod", &bytes).unwrap();
    knx_productdb::ingest_file(&conn, "M-0001/Hardware-later.xml", later.as_bytes()).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM hardware", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn retries_keep_conflicts_and_unknown_paths_cannot_supply_parsed_rows() {
    let (_dir, conn) = db();
    install_package(
        &conn,
        "first.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
        ]),
    )
    .unwrap();
    let changed = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("Example", "Conflicting");
    let arbitrary = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("H-1", "H-2")
        .replace("P-1", "P-2");
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", changed.as_bytes()),
        ("notes.xml", arbitrary.as_bytes()),
        ("M-0001/Baggages/data.xml", arbitrary.as_bytes()),
    ]);
    let first = install_package(&conn, "second.knxprod", &bytes).unwrap();
    assert!(!first.conflicts.is_empty());
    install_package(
        &conn,
        "third.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", changed.as_bytes()),
        ]),
    )
    .unwrap();
    assert_eq!(
        install_package(&conn, "retry.knxprod", &bytes)
            .unwrap()
            .conflicts,
        first.conflicts
    );
    assert_eq!(
        conn.query_row("SELECT count(*) FROM hardware WHERE id = 'H-2'", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
}

#[test]
fn manufacturer_partition_and_ref_id_must_agree() {
    let (_dir, conn) = db();
    for path in ["M-anything/Hardware.xml", "M-0002/Hardware.xml"] {
        assert!(install_package(
            &conn,
            "bad.knxprod",
            &archive(&[("knx_master.xml", MASTER), (path, HARDWARE)])
        )
        .is_err());
        assert_eq!(counts(&conn), vec![0; 9]);
    }
    let fake_root = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("KNX", "UnknownRoot");
    assert!(install_package(
        &conn,
        "bad.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", fake_root.as_bytes())
        ])
    )
    .is_err());
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn excessive_declared_entry_count_is_rejected_before_loading_zip_metadata() {
    let (_dir, conn) = db();
    let mut bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let eocd = bytes.len() - 22;
    bytes[eocd + 8..eocd + 10].copy_from_slice(&4097_u16.to_le_bytes());
    bytes[eocd + 10..eocd + 12].copy_from_slice(&4097_u16.to_le_bytes());
    let result = install_package(&conn, "bad.knxprod", &bytes);
    assert!(
        matches!(result, Err(PackageError::SizeLimit { .. })),
        "{result:?}"
    );
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn aggregate_central_comments_stop_at_the_metadata_budget() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    for index in 0..400 {
        zip.start_file(
            format!("M-0001/Baggages/{index:04x}.bin"),
            SimpleFileOptions::default(),
        )
        .unwrap();
    }
    let bytes = inflate_central_comments(zip.finish().unwrap().into_inner(), u16::MAX);
    let (_dir, conn) = db();

    assert!(matches!(
        install_package(&conn, "comment-heavy.knxprod", &bytes),
        Err(PackageError::SizeLimit { ref path }) if path == "central directory metadata"
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn a_maximum_length_legacy_name_stays_within_the_decoded_budget() {
    let ascii_component = "x".repeat(60_000);
    let ascii_name = format!("M-0001/Baggages/{ascii_component}.bin");
    let mut raw_name = b"M-0001/Baggages/".to_vec();
    raw_name.extend(std::iter::repeat_n(0x81, 60_000));
    raw_name.extend_from_slice(b".bin");
    let bytes = legacy_member_names(
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            (&ascii_name, b"x"),
        ]),
        &[(ascii_name.as_str(), raw_name.as_slice())],
    );
    let (_dir, conn) = db();

    let report = install_package(&conn, "long-name.knxprod", &bytes).unwrap();
    assert!(report
        .members
        .iter()
        .any(|member| member.path.len() > 100_000));
}

#[test]
fn maximum_member_count_with_long_shared_prefixes_remains_bounded() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("M-0001/Hardware.xml", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(HARDWARE).unwrap();
    let shared = "a".repeat(1024);
    for index in 0..4095 {
        let name = format!("M-0001/Baggages/{shared}-{index:04x}.bin");
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(b"x").unwrap();
    }
    let bytes = zip.finish().unwrap().into_inner();
    let (_dir, conn) = db();

    assert!(matches!(
        install_package(&conn, "maximum-members.knxprod", &bytes),
        Err(PackageError::MissingMaster)
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn maximum_member_count_with_progressively_nested_paths_remains_bounded() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    let mut nested = String::from("M-0001/Baggages/");
    for _ in 0..4094 {
        nested.push_str("a/");
        zip.add_directory(&nested, SimpleFileOptions::default())
            .unwrap();
    }
    let bytes = zip.finish().unwrap().into_inner();
    let (_dir, conn) = db();

    let report = install_package(&conn, "nested-maximum.knxprod", &bytes).unwrap();
    assert_eq!(report.members.len(), 2);
}

#[test]
fn disjoint_deep_paths_stop_at_the_component_node_budget() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("M-0001/Hardware.xml", SimpleFileOptions::default())
        .unwrap();
    zip.write_all(HARDWARE).unwrap();
    for member in 0..4095 {
        let suffix = (0..17)
            .map(|depth| format!("{member:04x}-{depth:02x}"))
            .collect::<Vec<_>>()
            .join("/");
        let name = format!("M-0001/Baggages/{suffix}.bin");
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(b"x").unwrap();
    }
    let bytes = zip.finish().unwrap().into_inner();
    let (_dir, conn) = db();

    assert!(matches!(
        install_package(&conn, "deep-disjoint.knxprod", &bytes),
        Err(PackageError::SizeLimit { ref path })
            if path == "decoded path component budget"
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

fn archive(members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in members {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}

fn archive_with_symlink(name: &str) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (path, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ] {
        zip.start_file(path, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.start_file(
        name,
        SimpleFileOptions::default().unix_permissions(0o120777),
    )
    .unwrap();
    zip.write_all(b"target").unwrap();
    let mut bytes = zip.finish().unwrap().into_inner();
    for offset in 0..bytes.len().saturating_sub(46) {
        if bytes.get(offset..offset + 4) == Some(b"PK\x01\x02") {
            let name_len = u16::from_le_bytes([bytes[offset + 28], bytes[offset + 29]]) as usize;
            if bytes.get(offset + 46..offset + 46 + name_len) == Some(name.as_bytes()) {
                bytes[offset + 5] = 3;
                bytes[offset + 38..offset + 42]
                    .copy_from_slice(&(0o120777_u32 << 16).to_le_bytes());
            }
        }
    }
    bytes
}

fn archive_with_unicode_path(
    raw_name: &str,
    version: u8,
    crc32: u32,
    unicode_name: &[u8],
    duplicate: bool,
) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    let mut payload = vec![version];
    payload.extend_from_slice(&crc32.to_le_bytes());
    payload.extend_from_slice(unicode_name);
    let mut options = FullFileOptions::default();
    for _ in 0..=usize::from(duplicate) {
        options.add_extra_data(0xA11E, &payload, false).unwrap();
        options.add_extra_data(0xA11E, &payload, true).unwrap();
    }
    zip.start_file(raw_name, options).unwrap();
    zip.write_all(b"unicode path").unwrap();
    let mut bytes = zip.finish().unwrap().into_inner();
    for index in 0..bytes.len().saturating_sub(1) {
        if bytes[index..index + 2] == 0xA11E_u16.to_le_bytes() {
            bytes[index..index + 2].copy_from_slice(&0x7075_u16.to_le_bytes());
        }
    }
    bytes
}

fn archive_with_malformed_aes_extra() -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    let mut options = FullFileOptions::default();
    options.add_extra_data(0xA11E, [1], false).unwrap();
    options.add_extra_data(0xA11E, [1], true).unwrap();
    zip.start_file("M-0001/Baggages/extra.bin", options)
        .unwrap();
    zip.write_all(b"payload").unwrap();
    let mut bytes = zip.finish().unwrap().into_inner();
    for offset in 0..bytes.len().saturating_sub(1) {
        if bytes[offset..offset + 2] == 0xA11E_u16.to_le_bytes() {
            bytes[offset..offset + 2].copy_from_slice(&0x9901_u16.to_le_bytes());
        }
    }
    bytes
}

fn append_rebased_archive(mut prefix: Vec<u8>, mut archive: Vec<u8>) -> Vec<u8> {
    let base = prefix.len() as u32;
    for central in central_headers(&archive) {
        let local = u32::from_le_bytes(archive[central + 42..central + 46].try_into().unwrap());
        archive[central + 42..central + 46].copy_from_slice(&(local + base).to_le_bytes());
    }
    let eocd = eocd_offset(&archive);
    let directory = u32::from_le_bytes(archive[eocd + 16..eocd + 20].try_into().unwrap());
    archive[eocd + 16..eocd + 20].copy_from_slice(&(directory + base).to_le_bytes());
    prefix.extend(archive);
    prefix
}

fn insert_fallback_eocd_in_central_comment(mut bytes: Vec<u8>) -> Vec<u8> {
    let headers = central_headers(&bytes);
    assert!(headers.len() >= 3);
    let old_eocd = eocd_offset(&bytes);
    let first_record_size = (headers[1] - headers[0]) as u32;
    let insertion = headers[2];
    assert_eq!(&bytes[headers[1] + 32..headers[1] + 34], &[0, 0]);

    let mut fake_eocd = Vec::with_capacity(22);
    fake_eocd.extend_from_slice(b"PK\x05\x06");
    fake_eocd.extend_from_slice(&0_u16.to_le_bytes());
    fake_eocd.extend_from_slice(&0_u16.to_le_bytes());
    fake_eocd.extend_from_slice(&1_u16.to_le_bytes());
    fake_eocd.extend_from_slice(&1_u16.to_le_bytes());
    fake_eocd.extend_from_slice(&first_record_size.to_le_bytes());
    fake_eocd.extend_from_slice(&0_u32.to_le_bytes());
    fake_eocd.extend_from_slice(&0_u16.to_le_bytes());
    assert_eq!(fake_eocd.len(), 22);

    bytes[headers[1] + 32..headers[1] + 34].copy_from_slice(&22_u16.to_le_bytes());
    bytes.splice(insertion..insertion, fake_eocd);
    let new_eocd = old_eocd + 22;
    let old_size = u32::from_le_bytes(bytes[new_eocd + 12..new_eocd + 16].try_into().unwrap());
    bytes[new_eocd + 12..new_eocd + 16].copy_from_slice(&(old_size + 22).to_le_bytes());
    bytes
}

fn insert_fallback_eocd_in_final_comment(mut bytes: Vec<u8>) -> Vec<u8> {
    let headers = central_headers(&bytes);
    let eocd = eocd_offset(&bytes);
    let first_record_size = (headers[1] - headers[0]) as u32;
    assert_eq!(&bytes[eocd + 20..eocd + 22], &[0, 0]);

    bytes[eocd + 20..eocd + 22].copy_from_slice(&23_u16.to_le_bytes());
    bytes.extend_from_slice(b"PK\x05\x06");
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.extend_from_slice(&first_record_size.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u16.to_le_bytes());
    // Keep the nested record from terminating at EOF; the selected EOCD owns
    // all 23 comment bytes, while `zip` otherwise accepts trailing bytes.
    bytes.push(b'x');
    bytes
}

fn mutate_first_name(mut bytes: Vec<u8>, from: &[u8], to: &[u8]) -> Vec<u8> {
    assert_eq!(from.len(), to.len());
    let start = bytes
        .windows(from.len())
        .position(|window| window == from)
        .unwrap();
    bytes[start..start + to.len()].copy_from_slice(to);
    bytes
}

fn mutate_local_metadata(mut bytes: Vec<u8>, name: &[u8], field: usize) -> Vec<u8> {
    let name_start = bytes
        .windows(name.len())
        .position(|window| window == name)
        .unwrap();
    bytes[name_start - 30 + field] ^= 1;
    bytes
}

fn alias_second_central_local_offset(mut bytes: Vec<u8>) -> Vec<u8> {
    let headers = bytes
        .windows(4)
        .enumerate()
        .filter_map(|(offset, value)| (value == b"PK\x01\x02").then_some(offset))
        .collect::<Vec<_>>();
    let first = bytes[headers[0] + 42..headers[0] + 46].to_vec();
    bytes[headers[1] + 42..headers[1] + 46].copy_from_slice(&first);
    bytes
}

fn eocd_offset(bytes: &[u8]) -> usize {
    bytes
        .windows(4)
        .rposition(|value| value == b"PK\x05\x06")
        .unwrap()
}

fn central_headers(bytes: &[u8]) -> Vec<usize> {
    let eocd = eocd_offset(bytes);
    let count = u16::from_le_bytes([bytes[eocd + 10], bytes[eocd + 11]]) as usize;
    let mut offset = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let mut headers = Vec::with_capacity(count);
    for _ in 0..count {
        assert_eq!(
            bytes.get(offset..offset + 4),
            Some(b"PK\x01\x02".as_slice())
        );
        headers.push(offset);
        let name = u16::from_le_bytes([bytes[offset + 28], bytes[offset + 29]]) as usize;
        let extra = u16::from_le_bytes([bytes[offset + 30], bytes[offset + 31]]) as usize;
        let comment = u16::from_le_bytes([bytes[offset + 32], bytes[offset + 33]]) as usize;
        offset += 46 + name + extra + comment;
    }
    headers
}

fn inflate_central_comments(mut bytes: Vec<u8>, comment_len: u16) -> Vec<u8> {
    let eocd = eocd_offset(&bytes);
    let directory = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    let mut rebuilt = Vec::new();
    for offset in central_headers(&bytes) {
        let name = u16::from_le_bytes([bytes[offset + 28], bytes[offset + 29]]) as usize;
        let extra = u16::from_le_bytes([bytes[offset + 30], bytes[offset + 31]]) as usize;
        let old_comment = u16::from_le_bytes([bytes[offset + 32], bytes[offset + 33]]) as usize;
        let end = offset + 46 + name + extra + old_comment;
        let record_start = rebuilt.len();
        rebuilt.extend_from_slice(&bytes[offset..end - old_comment]);
        rebuilt[record_start + 32..record_start + 34].copy_from_slice(&comment_len.to_le_bytes());
        rebuilt.resize(rebuilt.len() + comment_len as usize, b'c');
    }
    let mut footer = bytes[eocd..].to_vec();
    footer[12..16].copy_from_slice(&(rebuilt.len() as u32).to_le_bytes());
    bytes.truncate(directory);
    bytes.extend(rebuilt);
    bytes.extend(footer);
    bytes
}

fn add_data_descriptor_to_last_member(
    mut bytes: Vec<u8>,
    with_signature: bool,
    corrupt_crc: bool,
) -> Vec<u8> {
    let old_eocd = eocd_offset(&bytes);
    let old_directory =
        u32::from_le_bytes(bytes[old_eocd + 16..old_eocd + 20].try_into().unwrap()) as usize;
    let central = *central_headers(&bytes).last().unwrap();
    let local = u32::from_le_bytes(bytes[central + 42..central + 46].try_into().unwrap()) as usize;
    let crc = u32::from_le_bytes(bytes[central + 16..central + 20].try_into().unwrap());
    let compressed = u32::from_le_bytes(bytes[central + 20..central + 24].try_into().unwrap());
    let uncompressed = u32::from_le_bytes(bytes[central + 24..central + 28].try_into().unwrap());
    let local_name = u16::from_le_bytes([bytes[local + 26], bytes[local + 27]]) as usize;
    let local_extra = u16::from_le_bytes([bytes[local + 28], bytes[local + 29]]) as usize;
    let data_start = local + 30 + local_name + local_extra;
    assert_eq!(data_start + compressed as usize, old_directory);

    bytes[local + 6] |= 1 << 3;
    bytes[local + 14..local + 26].fill(0);
    let mut descriptor = Vec::with_capacity(16);
    if with_signature {
        descriptor.extend_from_slice(b"PK\x07\x08");
    }
    descriptor.extend_from_slice(&(crc ^ u32::from(corrupt_crc)).to_le_bytes());
    descriptor.extend_from_slice(&compressed.to_le_bytes());
    descriptor.extend_from_slice(&uncompressed.to_le_bytes());
    let descriptor_len = descriptor.len();
    bytes.splice(old_directory..old_directory, descriptor);

    let central = central + descriptor_len;
    let eocd = old_eocd + descriptor_len;
    bytes[central + 8] |= 1 << 3;
    bytes[eocd + 16..eocd + 20]
        .copy_from_slice(&((old_directory + descriptor_len) as u32).to_le_bytes());
    bytes
}

fn make_first_local_record_overlap_next(mut bytes: Vec<u8>) -> Vec<u8> {
    let headers = central_headers(&bytes);
    let first = headers[0];
    let second = headers[1];
    let first_local =
        u32::from_le_bytes(bytes[first + 42..first + 46].try_into().unwrap()) as usize;
    let second_local =
        u32::from_le_bytes(bytes[second + 42..second + 46].try_into().unwrap()) as usize;
    let name = u16::from_le_bytes([bytes[first_local + 26], bytes[first_local + 27]]) as usize;
    let extra = u16::from_le_bytes([bytes[first_local + 28], bytes[first_local + 29]]) as usize;
    let data_start = first_local + 30 + name + extra;
    let overlapping = (second_local - data_start + 1) as u32;
    bytes[first_local + 18..first_local + 22].copy_from_slice(&overlapping.to_le_bytes());
    bytes[first + 20..first + 24].copy_from_slice(&overlapping.to_le_bytes());
    bytes
}

fn make_last_local_record_overlap_directory(mut bytes: Vec<u8>) -> Vec<u8> {
    let central = *central_headers(&bytes).last().unwrap();
    let local = u32::from_le_bytes(bytes[central + 42..central + 46].try_into().unwrap()) as usize;
    let compressed = u32::from_le_bytes(bytes[central + 20..central + 24].try_into().unwrap());
    let overlapping = compressed + 1;
    bytes[local + 18..local + 22].copy_from_slice(&overlapping.to_le_bytes());
    bytes[central + 20..central + 24].copy_from_slice(&overlapping.to_le_bytes());
    bytes
}

fn legacy_member_names(mut bytes: Vec<u8>, replacements: &[(&str, &[u8])]) -> Vec<u8> {
    for (utf8_name, legacy_name) in replacements {
        assert_eq!(utf8_name.len(), legacy_name.len());
        let mut replaced = 0;
        let mut offset = 0;
        while offset + utf8_name.len() <= bytes.len() {
            let Some(relative) = bytes[offset..]
                .windows(utf8_name.len())
                .position(|window| window == utf8_name.as_bytes())
            else {
                break;
            };
            let start = offset + relative;
            bytes[start..start + legacy_name.len()].copy_from_slice(legacy_name);
            replaced += 1;
            offset = start + legacy_name.len();
        }
        assert_eq!(replaced, 2, "local and central ZIP names must both change");
    }
    bytes
}

fn mark_names_as_utf8(mut bytes: Vec<u8>) -> Vec<u8> {
    for offset in 0..bytes.len().saturating_sub(46) {
        if bytes.get(offset..offset + 4) == Some(b"PK\x03\x04") {
            bytes[offset + 7] |= 0x08;
        } else if bytes.get(offset..offset + 4) == Some(b"PK\x01\x02") {
            bytes[offset + 9] |= 0x08;
        }
    }
    bytes
}

#[test]
fn utf8_and_legacy_cp437_member_names_are_retained_as_unicode() {
    for bytes in [
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/Baggages/Grüsse.txt", b"utf8"),
        ]),
        legacy_member_names(
            archive(&[
                ("knx_master.xml", MASTER),
                ("M-0001/Hardware.xml", HARDWARE),
                ("M-0001/Baggages/Grusse.txt", b"cp437"),
            ]),
            &[(
                "M-0001/Baggages/Grusse.txt",
                b"M-0001/Baggages/Gr\x81sse.txt",
            )],
        ),
    ] {
        let (_dir, conn) = db();
        let report = install_package(&conn, "names.knxprod", &bytes).unwrap();
        assert!(report
            .members
            .iter()
            .any(|member| member.path == "M-0001/Baggages/Grüsse.txt"));
    }
}

#[test]
fn decoded_legacy_names_still_reject_traversal_and_normalized_collisions() {
    let traversal = legacy_member_names(
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/Baggages/../Grusse.txt", b"bad"),
        ]),
        &[(
            "M-0001/Baggages/../Grusse.txt",
            b"M-0001/Baggages/../Gr\x81sse.txt",
        )],
    );
    let (_dir, conn) = db();
    assert!(matches!(
        install_package(&conn, "traversal.knxprod", &traversal),
        Err(PackageError::UnsafeMember { .. })
    ));

    let collision = legacy_member_names(
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/Baggages/Grusse", b"file"),
            ("M-0001/Baggages/Grusse/", b""),
        ]),
        &[
            ("M-0001/Baggages/Grusse/", b"M-0001/Baggages/Gr\x81sse/"),
            ("M-0001/Baggages/Grusse", b"M-0001/Baggages/Gr\x81sse"),
        ],
    );
    let (_dir, conn) = db();
    assert!(matches!(
        install_package(&conn, "collision.knxprod", &collision),
        Err(PackageError::DuplicateMember { .. })
    ));
}

#[test]
fn every_documented_decoded_path_hazard_remains_rejected() {
    let nul = legacy_member_names(
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/Baggages/nul-x", b"nul"),
        ]),
        &[("M-0001/Baggages/nul-x", b"M-0001/Baggages/nul\0x")],
    );
    for bytes in [
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("/absolute", b"bad"),
        ]),
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("C:/drive", b"bad"),
        ]),
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001\\backslash", b"bad"),
        ]),
        nul,
        archive_with_symlink("M-0001/Baggages/link"),
    ] {
        let (_dir, conn) = db();
        assert!(matches!(
            install_package(&conn, "unsafe.knxprod", &bytes),
            Err(PackageError::UnsafeMember { .. })
        ));
        assert_eq!(counts(&conn), vec![0; 9]);
    }
}

#[test]
fn invalid_bytes_claiming_to_be_utf8_are_not_treated_as_cp437() {
    let bytes = mark_names_as_utf8(legacy_member_names(
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/Baggages/Grusse.txt", b"bad flag"),
        ]),
        &[(
            "M-0001/Baggages/Grusse.txt",
            b"M-0001/Baggages/Gr\x81sse.txt",
        )],
    ));
    let (_dir, conn) = db();

    let error = install_package(&conn, "invalid-utf8.knxprod", &bytes).unwrap_err();
    assert!(matches!(
        error,
        PackageError::InvalidZip { ref cause }
            if cause == "member name has invalid flagged UTF-8"
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn local_and_central_member_identity_must_agree() {
    let bytes = mutate_first_name(
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/Baggages/one.txt", b"identity"),
        ]),
        b"M-0001/Baggages/one.txt",
        b"M-0001/Baggages/two.txt",
    );
    let (_dir, conn) = db();

    let error = install_package(&conn, "split-identity.knxprod", &bytes).unwrap_err();
    assert!(matches!(
        error,
        PackageError::InvalidZip { ref cause }
            if cause == "local and central member names differ"
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn local_and_central_metadata_and_offsets_must_agree() {
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    for (bytes, cause) in [
        (
            mutate_local_metadata(valid.clone(), b"knx_master.xml", 6),
            "local and central member metadata differ",
        ),
        (
            mutate_local_metadata(valid.clone(), b"knx_master.xml", 8),
            "local and central member metadata differ",
        ),
        (
            alias_second_central_local_offset(valid.clone()),
            "multiple members reference one local header",
        ),
    ] {
        let (_dir, conn) = db();
        let error = install_package(&conn, "metadata.knxprod", &bytes).unwrap_err();
        assert!(matches!(
            error,
            PackageError::InvalidZip { cause: ref actual } if actual == cause
        ));
        assert_eq!(counts(&conn), vec![0; 9]);
    }
}

#[test]
fn zip64_entry_metadata_and_nonzero_entry_disks_are_rejected() {
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let mut other_disk = valid.clone();
    let central = central_headers(&other_disk)[0];
    other_disk[central + 34..central + 36].copy_from_slice(&1_u16.to_le_bytes());

    let raw = "M-0001/Baggages/zip64.txt";
    let mut zip64_extra = archive_with_unicode_path(
        raw,
        2,
        crc32fast::hash(raw.as_bytes()),
        b"ignored-version",
        false,
    );
    for offset in 0..zip64_extra.len().saturating_sub(1) {
        if zip64_extra[offset..offset + 2] == 0x7075_u16.to_le_bytes() {
            zip64_extra[offset..offset + 2].copy_from_slice(&0x0001_u16.to_le_bytes());
        }
    }

    for (bytes, cause) in [
        (other_disk, "central member starts on another disk"),
        (zip64_extra, "ZIP64 member metadata is unsupported"),
    ] {
        let (_dir, conn) = db();
        let error = install_package(&conn, "unsupported.knxprod", &bytes).unwrap_err();
        assert!(matches!(
            error,
            PackageError::InvalidZip { cause: ref actual } if actual == cause
        ));
        assert_eq!(counts(&conn), vec![0; 9]);
    }
}

#[test]
fn zip_parser_cannot_backtrack_to_an_unchecked_earlier_directory() {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in [
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    for index in 0..390 {
        zip.start_file(
            format!("M-0001/Baggages/{index:04x}.bin"),
            SimpleFileOptions::default(),
        )
        .unwrap();
    }
    // This earlier directory is itself larger than the accepted metadata
    // budget. The parser must still report the selected final directory's
    // malformed AES field rather than scanning or allocating from this one.
    let earlier = inflate_central_comments(zip.finish().unwrap().into_inner(), u16::MAX);
    let bytes = append_rebased_archive(earlier, archive_with_malformed_aes_extra());
    let (_dir, conn) = db();

    let error = install_package(&conn, "two-directories.knxprod", &bytes).unwrap_err();
    assert!(matches!(
        error,
        PackageError::InvalidZip { ref cause }
            if cause == "unsupported Zip archive: AES extra data field has an unsupported length"
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn zip_parser_cannot_fall_back_to_an_eocd_inside_a_central_comment() {
    let bytes = insert_fallback_eocd_in_central_comment(archive_with_malformed_aes_extra());
    let (_dir, conn) = db();

    let error = install_package(&conn, "comment-eocd.knxprod", &bytes).unwrap_err();
    assert!(matches!(
        error,
        PackageError::InvalidZip { ref cause }
            if cause == "unsupported Zip archive: AES extra data field has an unsupported length"
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn zip_parser_cannot_select_an_eocd_inside_the_final_comment() {
    let bytes = insert_fallback_eocd_in_final_comment(archive_with_malformed_aes_extra());
    let (_dir, conn) = db();

    let error = install_package(&conn, "final-comment-eocd.knxprod", &bytes).unwrap_err();
    assert!(matches!(
        error,
        PackageError::InvalidZip { ref cause }
            if cause == "unsupported Zip archive: AES extra data field has an unsupported length"
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn decoded_file_prefix_collisions_and_directory_payloads_are_rejected() {
    for bytes in [
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/Baggages/node", b"file"),
            ("M-0001/Baggages/node/child", b"child"),
        ]),
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("M-0001/Baggages/node/child", b"child"),
            ("M-0001/Baggages/node", b"file"),
        ]),
    ] {
        let (_dir, conn) = db();
        assert!(matches!(
            install_package(&conn, "prefix.knxprod", &bytes),
            Err(PackageError::DuplicateMember { .. })
        ));
        assert_eq!(counts(&conn), vec![0; 9]);
    }

    let (_dir, conn) = db();
    let payload = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages/directory/", b"hidden payload"),
    ]);
    assert!(matches!(
        install_package(&conn, "directory-payload.knxprod", &payload),
        Err(PackageError::UnsafeMember { .. })
    ));
    assert_eq!(counts(&conn), vec![0; 9]);

    let (_dir, conn) = db();
    let ordinary_directory = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001/Baggages/directory/", b""),
        ("M-0001/Baggages/directory/child", b"child"),
    ]);
    assert!(install_package(&conn, "directory.knxprod", &ordinary_directory).is_ok());
}

#[test]
fn central_extent_local_ranges_and_crc_size_metadata_must_agree() {
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let mut wrong_directory_size = valid.clone();
    let eocd = eocd_offset(&wrong_directory_size);
    let declared = u32::from_le_bytes(
        wrong_directory_size[eocd + 12..eocd + 16]
            .try_into()
            .unwrap(),
    );
    wrong_directory_size[eocd + 12..eocd + 16].copy_from_slice(&(declared + 1).to_le_bytes());

    for (bytes, cause) in [
        (
            mutate_local_metadata(valid.clone(), b"knx_master.xml", 14),
            "local and central CRC or sizes differ",
        ),
        (
            mutate_local_metadata(valid.clone(), b"knx_master.xml", 22),
            "local and central CRC or sizes differ",
        ),
        (
            make_first_local_record_overlap_next(valid.clone()),
            "overlapping local file records",
        ),
        (
            make_last_local_record_overlap_directory(valid.clone()),
            "local file record overlaps central directory",
        ),
        (
            wrong_directory_size,
            "central directory size or extent mismatch",
        ),
    ] {
        let (_dir, conn) = db();
        let error = install_package(&conn, "inconsistent.knxprod", &bytes).unwrap_err();
        assert!(matches!(
            error,
            PackageError::InvalidZip { cause: ref actual } if actual == cause
        ));
        assert_eq!(counts(&conn), vec![0; 9]);
    }
}

#[test]
fn data_descriptors_are_checked_against_the_central_directory() {
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    for with_signature in [true, false] {
        let (_dir, conn) = db();
        assert!(install_package(
            &conn,
            "descriptor.knxprod",
            &add_data_descriptor_to_last_member(valid.clone(), with_signature, false),
        )
        .is_ok());
    }

    let (_dir, conn) = db();
    let error = install_package(
        &conn,
        "bad-descriptor.knxprod",
        &add_data_descriptor_to_last_member(valid, true, true),
    )
    .unwrap_err();
    assert!(matches!(
        error,
        PackageError::InvalidZip { ref cause } if cause == "data descriptor differs from central directory"
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn a_hash_retry_still_revalidates_the_stored_archive_boundary() {
    let (_dir, conn) = db();
    let malformed = mutate_local_metadata(
        archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
        ]),
        b"knx_master.xml",
        14,
    );
    let sha256 = knx_productdb::sha256_hex(&malformed);
    conn.execute(
        "INSERT INTO package (sha256, source_name, scheme, size, bytes, unknown_count)
         VALUES (?1, 'legacy.knxprod', 11, ?2, ?3, 0)",
        rusqlite::params![sha256, malformed.len() as i64, malformed],
    )
    .unwrap();

    let error = install_package(&conn, "retry.knxprod", &malformed).unwrap_err();
    assert!(matches!(
        error,
        PackageError::InvalidZip { ref cause } if cause == "local and central CRC or sizes differ"
    ));
    assert_eq!(
        conn.query_row("SELECT count(*) FROM package", [], |row| row
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn unicode_path_extra_fields_have_an_explicit_fallback_and_rejection_policy() {
    let raw = "M-0001/Baggages/Grusse.txt";
    let crc = crc32fast::hash(raw.as_bytes());

    for (bytes, expected) in [
        (
            archive_with_unicode_path(raw, 1, crc, "M-0001/Baggages/Grüsse.txt".as_bytes(), false),
            "M-0001/Baggages/Grüsse.txt",
        ),
        (
            archive_with_unicode_path(raw, 1, crc ^ 1, b"ignored-bad-crc", false),
            raw,
        ),
        (
            archive_with_unicode_path(raw, 2, crc, b"ignored-version", false),
            raw,
        ),
    ] {
        let (_dir, conn) = db();
        let report = install_package(&conn, "unicode-path.knxprod", &bytes).unwrap();
        assert!(report.members.iter().any(|member| member.path == expected));
    }

    for (bytes, expected) in [
        (
            archive_with_unicode_path(raw, 1, crc, b"M-0001/Baggages/Gr\xFFsse.txt", false),
            "invalid Unicode path UTF-8",
        ),
        (
            archive_with_unicode_path(raw, 1, crc, b"../escape", false),
            "unsafe decoded path",
        ),
        (
            archive_with_unicode_path(raw, 1, crc, b"M-0001/Baggages/Gruesse.txt", true),
            "duplicate Unicode path field",
        ),
    ] {
        let (_dir, conn) = db();
        let error = install_package(&conn, "unicode-path.knxprod", &bytes).unwrap_err();
        match expected {
            "unsafe decoded path" => {
                assert!(matches!(error, PackageError::UnsafeMember { .. }))
            }
            cause => assert!(matches!(
                error,
                PackageError::InvalidZip { cause: ref actual } if actual == cause
            )),
        }
        assert_eq!(counts(&conn), vec![0; 9]);
    }
}

fn counts(conn: &Connection) -> Vec<i64> {
    [
        "package",
        "package_member",
        "source_file",
        "manufacturer",
        "hardware",
        "product",
        "ingest_unknown",
        "catalog_item",
        "application_program",
    ]
    .iter()
    .map(|table| {
        conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    })
    .collect()
}

#[test]
fn installs_the_readable_corpus() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        });
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    for name in [
        "MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod",
        "Dummy_Applikation_Secure.knxprod",
        "646704-04_ETS4_2012_47_DE_EN.knxprod",
        "Weinzierl_730_KNX_IP_Interface_ETS4.knxprod",
    ] {
        let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| panic!("corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to OriginalData/ProductDatabases"));
        let (_dir, conn) = db();
        let report = install_package(&conn, name, &bytes).unwrap();
        assert!(!report.skipped);
        assert!([11, 20].contains(&report.scheme));
        let stored: Vec<u8> = conn
            .query_row(
                "SELECT bytes FROM package WHERE sha256 = ?1",
                [&report.sha256],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(stored, bytes);
        let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
        let mut member_count = 0;
        for i in 0..zip.len() {
            let mut member = zip.by_index(i).unwrap();
            if member.is_dir() {
                continue;
            }
            member_count += 1;
            let mut raw = Vec::new();
            member.read_to_end(&mut raw).unwrap();
            let (sha, size): (String, i64) = conn.query_row("SELECT source_sha256, size FROM package_member WHERE package_sha256 = ?1 AND path = ?2", [&report.sha256, member.name()], |r| Ok((r.get(0)?, r.get(1)?))).unwrap();
            assert_eq!(size as usize, raw.len());
            assert_eq!(
                knx_productdb::load_source_file(&conn, &sha)
                    .unwrap()
                    .unwrap(),
                raw
            );
        }
        assert_eq!(report.members.len(), member_count);
        assert!(knx_productdb::verify(&conn).unwrap().is_empty());
        assert!(
            !knx_productdb::query::catalog_items(&conn, None, None, None)
                .unwrap()
                .is_empty()
        );
        let before = counts(&conn);
        assert!(install_package(&conn, name, &bytes).unwrap().skipped);
        assert_eq!(counts(&conn), before);
    }
}

#[test]
fn rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        });
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let name = "Weinzierl_730_KNX_IP_Interface_ETS2-3.vd2";
    let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| {
        panic!("corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to OriginalData/ProductDatabases")
    });
    let (_dir, conn) = db();
    let err = install_package(&conn, name, &bytes).unwrap_err();
    let (sha256, len) = match &err {
        PackageError::LegacyVd2 { sha256, len } => (sha256.clone(), *len),
        other => panic!("expected PackageError::LegacyVd2, got {other:?}"),
    };
    assert_eq!(len, bytes.len());
    assert_eq!(sha256.len(), 64);
    assert!(sha256
        .chars()
        .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    assert_eq!(sha256, knx_productdb::sha256_hex(&bytes));
    let rendered = err.to_string();
    assert!(rendered.contains(&sha256), "{rendered}");
    assert!(rendered.contains(&len.to_string()), "{rendered}");
    assert_eq!(counts(&conn), vec![0; 9]);
    eprintln!(
        "rejects_the_real_legacy_vd2_corpus_file_with_its_hash_and_size: sha256={sha256} len={len}"
    );
}

#[test]
fn a_small_vd2_still_reports_hash_and_length() {
    let (_dir, conn) = db();
    let bytes = vec![1u8, 2, 3];
    let err = install_package(&conn, "legacy.vd2", &bytes).unwrap_err();
    match err {
        PackageError::LegacyVd2 { sha256, len } => {
            assert_eq!(len, 3);
            assert_eq!(sha256, knx_productdb::sha256_hex(&bytes));
        }
        other => panic!("expected PackageError::LegacyVd2, got {other:?}"),
    }
}

#[test]
fn failures_preserve_an_existing_install_including_reports() {
    let (_dir, conn) = db();
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
    ]);
    let report = install_package(&conn, "valid.knxprod", &valid).unwrap();
    let before = counts(&conn);
    let changed = String::from_utf8(HARDWARE.to_vec())
        .unwrap()
        .replace("Example", "Conflicting");
    let malformed = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", changed.as_bytes()),
        ("M-0001/new.xml", b"<KNX><Unknown>"),
    ]);
    assert!(install_package(&conn, "bad.knxprod", &malformed).is_err());
    assert_eq!(counts(&conn), before);
    assert_eq!(
        conn.query_row("SELECT name FROM hardware WHERE id = 'H-1'", [], |r| r
            .get::<_, String>(
            0
        ))
        .unwrap(),
        "Example"
    );
    assert_eq!(
        install_package(&conn, "renamed.knxprod", &valid)
            .unwrap()
            .members,
        report.members
    );
}

#[test]
fn master_namespace_must_be_exact_and_xml_complete() {
    let (_dir, conn) = db();
    for master in [
        br#"<KNX xmlns="http://knx.org/xml/project/110"/>"#.as_slice(),
        br#"<KNX xmlns="https://knx.org/xml/project/11"/>"#,
        br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData>"#,
        br#"<KNX xmlns="http://knx.org/xml/project/11"/><KNX/>"#,
    ] {
        assert!(install_package(
            &conn,
            "bad.knxprod",
            &archive(&[
                ("knx_master.xml", master),
                ("M-0001/Hardware.xml", HARDWARE)
            ])
        )
        .is_err());
        assert_eq!(counts(&conn), vec![0; 9]);
    }
    let prefixed = br#"<k:KNX xmlns:k="http://knx.org/xml/project/20"><k:MasterData/></k:KNX>"#;
    assert_eq!(
        install_package(
            &conn,
            "good.knxprod",
            &archive(&[
                ("knx_master.xml", prefixed),
                ("M-0001/Hardware.xml", HARDWARE)
            ])
        )
        .unwrap()
        .scheme,
        20
    );
}

#[test]
fn validates_unknown_xml_payloads_without_rejecting_valid_references() {
    let (_dir, conn) = db();
    for payload in [
        br#"<![CDATA[bad]]><R/>"#.as_slice(),
        br#"<R bad=>ok</R>"#,
        br#"<R>&undefined;</R>"#,
        br#"&amp;<R/>"#,
        br#"<R>&#0;</R>"#,
        br#"<R>&#+65;</R>"#,
    ] {
        let bytes = archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
            ("notes.xml", payload),
        ]);
        assert!(install_package(&conn, "malformed.knxprod", &bytes).is_err());
        assert_eq!(counts(&conn), vec![0; 9]);
    }
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("notes.xml", br#"<R>&#65;&#x41;</R>"#),
    ]);
    assert!(install_package(&conn, "valid.knxprod", &valid).is_ok());
}

#[test]
fn malformed_and_unsupported_packages_leave_no_rows() {
    let (_dir, conn) = db();
    let cases = [
        ("legacy.vd2", vec![1, 2, 3], "legacy"),
        ("bad.knxprod", vec![1, 2, 3], "ZIP"),
        (
            "missing.knxprod",
            archive(&[("M-0001/Hardware.xml", HARDWARE)]),
            "master",
        ),
        (
            "unsafe.knxprod",
            archive(&[("knx_master.xml", MASTER), ("../escape", b"x")]),
            "unsafe",
        ),
        (
            "unknown.knxprod",
            archive(&[
                (
                    "knx_master.xml",
                    br#"<KNX xmlns="http://knx.org/xml/project/21"/>"#,
                ),
                ("M-0001/Hardware.xml", HARDWARE),
            ]),
            "namespace",
        ),
        (
            "project.knxprod",
            archive(&[("knx_master.xml", MASTER), ("P-0001/0.xml", b"<KNX/>")]),
            "project",
        ),
    ];
    for (name, bytes, message) in cases {
        let err = install_package(&conn, name, &bytes).unwrap_err();
        assert!(err.to_string().contains(message), "{name}: {err}");
        assert_eq!(counts(&conn), vec![0; 9]);
    }
}

#[test]
fn a_late_xml_failure_rolls_back_every_package_write() {
    let (_dir, conn) = db();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        (
            "M-0001/Broken.xml",
            b"<KNX><ManufacturerData><ApplicationPrograms><ApplicationProgram Id=\"broken\">",
        ),
    ]);
    assert!(matches!(
        install_package(&conn, "bad.knxprod", &bytes),
        Err(PackageError::Database(_))
    ));
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn unknown_members_are_retained_and_reported() {
    let (_dir, conn) = db();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001.signature", b"signature"),
        ("notes.txt", b"abc"),
    ]);
    let report = install_package(&conn, "example.knxprod", &bytes).unwrap();
    let member = report
        .members
        .iter()
        .find(|m| m.path == "notes.txt")
        .unwrap();
    assert_eq!(
        member.sha256,
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(member.role, "Unrecognized");
    assert_eq!(member.size, 3);
    assert!(report.unknown > 0);
}

#[test]
fn duplicate_encrypted_truncated_and_oversized_members_are_rejected() {
    let (_dir, conn) = db();
    let valid = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("one.txt", b"x"),
        ("two.txt", b"x"),
    ]);
    let mut duplicate = valid.clone();
    for i in 0..duplicate.len() - 7 {
        if &duplicate[i..i + 7] == b"two.txt" {
            duplicate[i..i + 7].copy_from_slice(b"one.txt");
        }
    }
    let result = install_package(&conn, "bad.knxprod", &duplicate);
    assert!(
        matches!(result, Err(PackageError::DuplicateMember { .. })),
        "{result:?}"
    );
    let mut encrypted = valid.clone();
    let mut oversized = valid.clone();
    for i in 0..valid.len() - 46 {
        if &valid[i..i + 4] == b"PK\x01\x02" {
            encrypted[i + 8] |= 1;
            oversized[i + 24..i + 28].copy_from_slice(&(64_u32 * 1024 * 1024 + 1).to_le_bytes());
        }
        if &valid[i..i + 4] == b"PK\x03\x04" {
            encrypted[i + 6] |= 1;
            oversized[i + 22..i + 26].copy_from_slice(&(64_u32 * 1024 * 1024 + 1).to_le_bytes());
        }
    }
    assert!(matches!(
        install_package(&conn, "bad.knxprod", &encrypted),
        Err(PackageError::Encrypted { .. })
    ));
    assert!(matches!(
        install_package(&conn, "bad.knxprod", &oversized),
        Err(PackageError::SizeLimit { .. })
    ));
    assert!(install_package(&conn, "bad.knxprod", &valid[..valid.len() / 2]).is_err());
    assert_eq!(counts(&conn), vec![0; 9]);
}

#[test]
fn migrating_v1_preserves_existing_rows_and_blobs() {
    let (dir, conn) = db();
    knx_productdb::ingest_file(&conn, "M-0001/Hardware.xml", HARDWARE).unwrap();
    // `translation` is also rolled back to its pre-Task-1 (v0-v1) shape: `db()`
    // already ran the full chain up to v4, so `translation` already has
    // `scope`/`scope_id`, and rerunning `migrate_v3_to_v4`'s rebuild against a
    // table that is already in its own target shape would fail looking for
    // the `program_id` column it expects to migrate away from.
    conn.execute_batch(
        "DROP TABLE package_install_diagnostic;
         DROP TABLE package_install_unknown;
         DROP TABLE package_install_count;
         DROP TABLE package_install_report;
         DROP TABLE package_conflict; DROP TABLE package_member; DROP TABLE source_parse_evidence;
         DROP TABLE package; DROP TABLE dynamic_node;
         DROP TABLE module_def_argument;
         DROP INDEX translation_lookup;
         DROP TABLE translation;
         CREATE TABLE translation (
             program_id     TEXT NOT NULL,
             language       TEXT NOT NULL,
             ref_id         TEXT NOT NULL,
             attribute_name TEXT NOT NULL,
             text           TEXT,
             PRIMARY KEY (program_id, language, ref_id, attribute_name)
         ) STRICT;
         CREATE INDEX translation_lookup ON translation (program_id, language, ref_id);
         PRAGMA user_version = 1;",
    )
    .unwrap();
    drop(conn);
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        knx_productdb::CURRENT_PRODUCTDB_VERSION
    );
    assert_eq!(
        knx_productdb::load_source_file(&conn, &knx_productdb::sha256_hex(HARDWARE))
            .unwrap()
            .unwrap(),
        HARDWARE
    );
    install_package(
        &conn,
        "example.knxprod",
        &archive(&[
            ("knx_master.xml", MASTER),
            ("M-0001/Hardware.xml", HARDWARE),
        ]),
    )
    .unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM hardware", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn a_failed_v1_to_v2_migration_rolls_back_its_ddl_and_version() {
    let (dir, conn) = db();
    // `translation` is also rolled back to its pre-Task-1 (v0-v1) shape, for
    // the same reason `migrating_v1_preserves_existing_rows_and_blobs` does:
    // `db()` already ran the full chain up to v4, so `migrate_v3_to_v4`'s
    // rebuild must find `program_id` still there to migrate away from.
    conn.execute_batch(
        "DROP TABLE package_install_diagnostic;
         DROP TABLE package_install_unknown;
         DROP TABLE package_install_count;
         DROP TABLE package_install_report;
         DROP TABLE package_conflict; DROP TABLE package_member; DROP TABLE source_parse_evidence;
         DROP TABLE package; DROP TABLE dynamic_node;
         DROP TABLE module_def_argument;
         DROP INDEX translation_lookup;
         DROP TABLE translation;
         CREATE TABLE translation (
             program_id     TEXT NOT NULL,
             language       TEXT NOT NULL,
             ref_id         TEXT NOT NULL,
             attribute_name TEXT NOT NULL,
             text           TEXT,
             PRIMARY KEY (program_id, language, ref_id, attribute_name)
         ) STRICT;
         CREATE INDEX translation_lookup ON translation (program_id, language, ref_id);
         CREATE TABLE package_conflict (marker INTEGER); PRAGMA user_version = 1;",
    )
    .unwrap();
    drop(conn);
    assert!(open_and_migrate(&dir.path().join("products.sqlite")).is_err());
    let conn = Connection::open(dir.path().join("products.sqlite")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(conn.prepare("SELECT * FROM package").is_err());
    conn.execute_batch("DROP TABLE package_conflict;").unwrap();
    drop(conn);
    assert_eq!(
        open_and_migrate(&dir.path().join("products.sqlite"))
            .unwrap()
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        knx_productdb::CURRENT_PRODUCTDB_VERSION
    );
}

#[test]
fn a_package_reports_how_many_translations_it_actually_wrote_by_scope() {
    // R3: one `Translation` planted in each of the four scopes a package can
    // carry one in, then the report is checked against what `translation`
    // actually holds afterwards — not against what the parser merely walked.
    let master = br#"<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Example"/>
    </Manufacturers>
  </MasterData>
  <Languages>
    <Language Identifier="de-DE">
      <TranslationUnit RefId="LOC-1">
        <TranslationElement RefId="LOC-1">
          <Translation AttributeName="Text" Text="Uebersetzt"/>
        </TranslationElement>
      </TranslationUnit>
    </Language>
  </Languages>
</KNX>"#;
    let hardware = br#"<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-0001">
      <Hardware>
        <Hardware Id="H-1" Name="Example">
          <Products><Product Id="P-1" Text="Example"/></Products>
        </Hardware>
      </Hardware>
      <Languages>
        <Language Identifier="de-DE">
          <TranslationUnit RefId="H-1">
            <TranslationElement RefId="H-1">
              <Translation AttributeName="Text" Text="Beispiel"/>
            </TranslationElement>
          </TranslationUnit>
        </Language>
      </Languages>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;
    let catalog = br#"<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-0001">
      <Catalog>
        <CatalogSection Id="M-0001_CG-1" Name="Sensors" Number="1">
          <CatalogItem Id="M-0001_CI-1" Name="Sensor" Number="1"/>
        </CatalogSection>
      </Catalog>
      <Languages>
        <Language Identifier="de-DE">
          <TranslationUnit RefId="M-0001_CI-1">
            <TranslationElement RefId="M-0001_CI-1">
              <Translation AttributeName="Name" Text="Sensor DE"/>
            </TranslationElement>
          </TranslationUnit>
        </Language>
      </Languages>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;
    let program = br#"<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-0001">
      <ApplicationPrograms>
        <ApplicationProgram Id="A-1" Name="P" ApplicationVersion="22" MaskVersion="MV-0701">
          <Static>
            <ComObjectTable>
              <ComObject Id="A-1_O-0" Number="0" Text="Output" ObjectSize="1 Bit"/>
            </ComObjectTable>
          </Static>
          <Languages>
            <Language Identifier="de-DE">
              <TranslationUnit RefId="A-1">
                <TranslationElement RefId="A-1_O-0">
                  <Translation AttributeName="Text" Text="Ausgang"/>
                </TranslationElement>
              </TranslationUnit>
            </Language>
          </Languages>
        </ApplicationProgram>
      </ApplicationPrograms>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;
    let (_dir, conn) = db();
    let bytes = archive(&[
        ("knx_master.xml", master),
        ("M-0001/Hardware.xml", hardware),
        ("M-0001/Catalog.xml", catalog),
        ("M-0001/Program.xml", program),
    ]);
    let report = install_package(&conn, "four-scopes.knxprod", &bytes).unwrap();

    assert_eq!(report.translations.master, 1);
    assert_eq!(report.translations.hardware, 1);
    assert_eq!(report.translations.catalog, 1);
    assert_eq!(report.translations.program, 1);
    assert_eq!(report.translations.total(), 4);

    let actual: i64 = conn
        .query_row("SELECT count(*) FROM translation", [], |r| r.get(0))
        .unwrap();
    assert_eq!(actual, 4, "the report must match what was actually written");

    // A retried install reports the counts recorded at the original
    // install, not zero and not a re-parse.
    let retry = install_package(&conn, "four-scopes.knxprod", &bytes).unwrap();
    assert!(retry.skipped);
    assert_eq!(retry.translations, report.translations);
}

/// docs/KNOWN_LIMITATIONS.md §64 (D10): a translation living outside any
/// `ApplicationProgram` — `knx_master.xml`'s own `Languages` block
/// (`Master` scope, no owning element) and `Hardware.xml`'s (`Hardware`
/// scope, keyed by the manufacturer partition it was found under) — must
/// survive `install_package` with its actual text intact, not merely be
/// counted. The row-count sibling test above already proves the count;
/// this one reads the text back and checks it against what was planted,
/// so a scope/ref_id mixup that happened to preserve the total row count
/// could not pass silently. It also carries the one end-to-end
/// `install_package` coverage `function_type`/`function_point`/
/// `space_usage` have (§64's T13 paragraph): a `FunctionTypes` and
/// `SpaceUsages` section, each with its own `Master`-scope translation,
/// planted alongside the pre-existing `LOC-1`/`H-1` translations, checked
/// against the three new tables and one join — everything else exercising
/// them so far is `parse/master.rs`'s and `migration.rs`'s own unit tests,
/// which call `ingest_master_data` directly rather than going through a
/// real `.knxprod` archive and `install_package`.
#[test]
fn hardware_and_master_scope_translations_survive_install_with_their_text_intact() {
    let master = br#"<KNX xmlns="http://knx.org/xml/project/11">
  <MasterData>
    <Manufacturers>
      <Manufacturer Id="M-0001" Name="Example"/>
    </Manufacturers>
    <FunctionTypes>
      <FunctionType Id="FT-1" Number="1" Text="Switch" Status="Certified">
        <FunctionPoint Id="FP-1_DR-1" Text="Switch" DatapointType="DPST-1-1" Role="Control" Characteristics="W"/>
      </FunctionType>
    </FunctionTypes>
    <SpaceUsages>
      <SpaceUsage Id="SU-1" Number="1" Text="Office"/>
    </SpaceUsages>
  </MasterData>
  <Languages>
    <Language Identifier="de-DE">
      <TranslationUnit RefId="LOC-1">
        <TranslationElement RefId="LOC-1">
          <Translation AttributeName="Text" Text="Herstellerunabhaengig"/>
        </TranslationElement>
      </TranslationUnit>
      <TranslationUnit RefId="FT-1">
        <TranslationElement RefId="FT-1">
          <Translation AttributeName="Text" Text="Schalten"/>
        </TranslationElement>
      </TranslationUnit>
      <TranslationUnit RefId="SU-1">
        <TranslationElement RefId="SU-1">
          <Translation AttributeName="Text" Text="Buero"/>
        </TranslationElement>
      </TranslationUnit>
    </Language>
  </Languages>
</KNX>"#;
    let hardware = br#"<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-0001">
      <Hardware>
        <Hardware Id="H-1" Name="Example">
          <Products><Product Id="P-1" Text="Example"/></Products>
        </Hardware>
      </Hardware>
      <Languages>
        <Language Identifier="de-DE">
          <TranslationUnit RefId="H-1">
            <TranslationElement RefId="H-1">
              <Translation AttributeName="Text" Text="Beispielgeraet"/>
            </TranslationElement>
          </TranslationUnit>
        </Language>
      </Languages>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;
    let (_dir, conn) = db();
    let bytes = archive(&[
        ("knx_master.xml", master),
        ("M-0001/Hardware.xml", hardware),
    ]);
    install_package(&conn, "outside-a-program.knxprod", &bytes).unwrap();

    // Master scope: no owning element, so `scope_id` is the empty-string
    // sentinel (`migrate_v3_to_v4`'s convention, not `NULL`).
    let master_text: String = conn
        .query_row(
            "SELECT text FROM translation
             WHERE scope = 'Master' AND scope_id = '' AND ref_id = 'LOC-1'
               AND attribute_name = 'Text' AND language = 'de-DE'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(master_text, "Herstellerunabhaengig");

    // Hardware scope: keyed by the manufacturer partition (`M-0001`), the
    // only owner a `Hardware.xml` translation ever has.
    let hardware_text: String = conn
        .query_row(
            "SELECT text FROM translation
             WHERE scope = 'Hardware' AND scope_id = 'M-0001' AND ref_id = 'H-1'
               AND attribute_name = 'Text' AND language = 'de-DE'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(hardware_text, "Beispielgeraet");

    // Schema v10 (T13): `function_type`, `function_point` and
    // `space_usage` each got a real row, not just the translations that
    // point at them.
    let function_type_row: (i64, String, String) = conn
        .query_row(
            "SELECT number, text, status FROM function_type WHERE id = 'FT-1'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )
        .unwrap();
    assert_eq!(
        function_type_row,
        (1, "Switch".to_string(), "Certified".to_string())
    );

    let function_point_type: String = conn
        .query_row(
            "SELECT function_type_id FROM function_point WHERE id = 'FP-1_DR-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(function_point_type, "FT-1");

    let space_usage_text: String = conn
        .query_row("SELECT text FROM space_usage WHERE id = 'SU-1'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(space_usage_text, "Office");

    // And the join docs/KNOWN_LIMITATIONS.md §64 says these tables exist
    // for in the first place: a real package install, not a hand-rolled
    // unit fixture, resolves `FunctionType`'s `Master`-scope translation
    // against the row `function_type` now has for it.
    let function_type_translation: String = conn
        .query_row(
            "SELECT t.text FROM translation t
             JOIN function_type f ON f.id = t.ref_id
             WHERE t.scope = 'Master' AND t.language = 'de-DE' AND t.attribute_name = 'Text'
               AND f.id = 'FT-1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(function_type_translation, "Schalten");
}

/// KNOWN_LIMITATIONS.md §85. A `.signature` member is recognised, given the
/// role `"Signature"`, and its bytes are kept verbatim in `source_file` —
/// exactly like `notes.txt` gets `"Unrecognized"` two tests up. Nothing
/// about that role is a cryptographic claim: no code path in this crate
/// reads a `"Signature"`-role member back out to check it against
/// anything. This test pins that absence, not a feature — if it ever
/// starts failing because verification was added, update §85 before
/// touching this assertion.
#[test]
fn signature_members_are_stored_verbatim_and_never_verified() {
    let (_dir, conn) = db();
    let bytes = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001.signature", b"not a real signature, just bytes"),
    ]);
    let report = install_package(&conn, "signed.knxprod", &bytes).unwrap();

    let member = report
        .members
        .iter()
        .find(|m| m.path == "M-0001.signature")
        .unwrap();
    assert_eq!(member.role, "Signature");

    // Stored byte-for-byte, retrievable only as an opaque blob.
    let stored: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM source_file WHERE sha256 = ?1",
            [&member.sha256],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored, b"not a real signature, just bytes");

    // There is no table, column or flag anywhere that a verification step
    // could have written a verdict into: a `"Signature"`-role member
    // affects `unknown`/`conflicts`/`translations` in no way at all,
    // exactly like any other retained-but-unparsed member.
    assert_eq!(report.unknown, 0);
    assert!(report.conflicts.is_empty());
    assert_eq!(report.translations.total(), 0);

    // A corrupted, truncated or outright wrong "signature" installs
    // exactly as cleanly as a genuine one would — because nothing ever
    // looks at the bytes beyond storing them.
    let corrupted = archive(&[
        ("knx_master.xml", MASTER),
        ("M-0001/Hardware.xml", HARDWARE),
        ("M-0001.signature", b""),
    ]);
    let (_dir2, conn2) = db();
    let report2 = install_package(&conn2, "signed.knxprod", &corrupted).unwrap();
    let member2 = report2
        .members
        .iter()
        .find(|m| m.path == "M-0001.signature")
        .unwrap();
    assert_eq!(member2.role, "Signature");
}

/// ADR-0020's v8 backfill, against the database it exists for rather than
/// against a fixture: every corpus package is installed by the current build,
/// the result is rolled back to exactly the state a pre-2026-09-13 ingest left
/// — `linkable` `NULL`, one "attribute not understood" row per program,
/// `user_version` 6 — and the file is reopened. Every value has to come back
/// out of the stored blobs, with no package reinstalled and no original file
/// touched.
///
/// These are the packages that made KNOWN_LIMITATIONS.md §87 real: all seven
/// of their `ApplicationProgram` elements spell the attribute
/// `Linkable="true"`/`"false"`, which is the spelling the old `bool_flag` read
/// as absent.
#[test]
fn a_v6_corpus_database_gets_its_linkable_back_from_its_own_blobs() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        });
    if !root.exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let expected: Vec<(String, Option<i64>, String)> = {
        let conn = open_and_migrate(&path).unwrap();
        for name in [
            "MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod",
            "Dummy_Applikation_Secure.knxprod",
            "646704-04_ETS4_2012_47_DE_EN.knxprod",
            "Weinzierl_730_KNX_IP_Interface_ETS4.knxprod",
        ] {
            let bytes = std::fs::read(root.join(name)).unwrap_or_else(|e| {
                panic!("corpus fixture {name} unavailable: {e}; set KNXBENCH_PRODUCT_CORPUS to OriginalData/ProductDatabases")
            });
            install_package(&conn, name, &bytes).unwrap();
        }
        let rows = linkable_rows(&conn);
        assert!(
            !rows.is_empty(),
            "sanity: the corpus installed some programs"
        );
        assert!(
            rows.iter().all(|(_, linkable, _)| linkable.is_some()),
            "sanity: the current build stores every corpus Linkable, \
             which is the fix §87 says only reaches future ingests: {rows:?}"
        );
        rows
    };

    // Roll back the two things, and only the two things, the 2026-09-13 parse
    // fix changed: the stored value, and the `ingest_unknown` row the old
    // `bool_flag` wrote when it met a spelling it could not read.
    {
        let conn = Connection::open(&path).unwrap();
        conn.execute("UPDATE application_program SET linkable = NULL", [])
            .unwrap();
        let mut stmt = conn
            .prepare(
                "INSERT INTO ingest_unknown
                 (source_sha256, program_id, xpath, kind, name, occurrences, sample)
                 VALUES (?1, NULL,
                   '/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram',
                   'Attribute', 'Linkable', 1, 'false')",
            )
            .unwrap();
        for (_, _, source_sha256) in &expected {
            stmt.execute([source_sha256]).unwrap();
        }
        drop(stmt);
        // v11/v12's own DDL has to go with the version number: this test
        // rewinds `user_version` without rewinding the schema, which was
        // free while v7-v10 added no structure the module-argument slice
        // needs and stopped being free once later migrations added tables and
        // a column. Drop v12's evidence children before their parent.
        conn.execute_batch(
            "DROP TABLE package_install_diagnostic;
             DROP TABLE package_install_unknown;
             DROP TABLE package_install_count;
             DROP TABLE package_install_report;
             DROP TABLE module_def_argument;
             ALTER TABLE dynamic_node DROP COLUMN value;",
        )
        .unwrap();
        conn.pragma_update(None, "user_version", 6i64).unwrap();
    }

    let conn = open_and_migrate(&path).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        knx_productdb::CURRENT_PRODUCTDB_VERSION
    );
    assert_eq!(
        linkable_rows(&conn),
        expected,
        "every value back, from the blobs alone"
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind = 'Attribute' AND name = 'Linkable'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0,
        "and every report that said the attribute was not understood retired"
    );
    assert_eq!(
        conn.query_row(
            "SELECT count(*) FROM ingest_unknown WHERE kind = 'LinkableBackfillError'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0,
        "no corpus blob failed to re-parse"
    );
    eprintln!(
        "a_v6_corpus_database_gets_its_linkable_back_from_its_own_blobs: {} program(s) refilled",
        expected.len()
    );
}

fn linkable_rows(conn: &Connection) -> Vec<(String, Option<i64>, String)> {
    conn.prepare("SELECT id, linkable, source_sha256 FROM application_program ORDER BY id")
        .unwrap()
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap()
}
