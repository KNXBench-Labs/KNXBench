//! Legacy EX-IM containers: detection, layout and method checks, bounded inflation and inspection.
//!
//! `knx-productdb` never decrypts and its tests do not either (it must not
//! reach `knx-secure`, not even from tests). Everything that needs the
//! password is tested in `knx-app/tests/legacy_files.rs`.

use knx_productdb::legacy::{
    detect_legacy_container, inspect_payload, read_legacy_member, ExImContent, LegacyError,
    LegacyMemberKind, LegacyPayload,
};
use knx_productdb::{install_package, open_and_migrate, PackageError};

fn fixture(path: &str) -> Vec<u8> {
    let full = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/legacy")
        .join(path);
    std::fs::read(&full).unwrap_or_else(|e| panic!("fixture {full:?} unreadable: {e}"))
}

/// Opens an unencrypted member; an encrypted one is refused as needing a
/// password, unless a container rule refuses it first.
fn open(bytes: &[u8]) -> Result<LegacyPayload, LegacyError> {
    read_legacy_member(bytes)?.open_unencrypted()
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// Offsets of the single member's local and central headers.
fn header_offsets(bytes: &[u8]) -> (usize, usize) {
    let eocd = bytes
        .windows(4)
        .rposition(|w| w == b"PK\x05\x06")
        .expect("end record");
    let central = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    (0, central)
}

/// Writes one field into both the local (`local_at`) and the central
/// (`central_at`) header, so the archive stays self-consistent and only the
/// meaning of the field changes.
fn patch_both(bytes: &mut [u8], local_at: usize, central_at: usize, value: &[u8]) {
    let (local, central) = header_offsets(bytes);
    bytes[local + local_at..local + local_at + value.len()].copy_from_slice(value);
    bytes[central + central_at..central + central_at + value.len()].copy_from_slice(value);
}

const FLAGS: (usize, usize) = (6, 8);
const METHOD: (usize, usize) = (8, 10);
const CRC: (usize, usize) = (14, 16);
const UNCOMPRESSED: (usize, usize) = (22, 24);

fn with_field(name: &str, field: (usize, usize), value: &[u8]) -> Vec<u8> {
    let mut bytes = fixture(name);
    patch_both(&mut bytes, field.0, field.1, value);
    bytes
}

/// A minimal stored ZIP with the given members, written by hand so the test
/// controls names and payloads independently of any ZIP library.
fn stored_zip(members: &[(&str, &[u8])]) -> Vec<u8> {
    stored_zip_after(b"", members)
}

/// [`stored_zip`] behind `prefix`, with every offset pointing at the real
/// position (the way a self-extractor stub would be laid out).
fn stored_zip_after(prefix: &[u8], members: &[(&str, &[u8])]) -> Vec<u8> {
    let mut out = prefix.to_vec();
    let mut central = Vec::new();
    for (name, data) in members {
        let offset = out.len() as u32;
        let crc = crc32fast::hash(data);
        let mut local = Vec::new();
        local.extend_from_slice(b"PK\x03\x04");
        local.extend_from_slice(&[20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        local.extend_from_slice(&crc.to_le_bytes());
        local.extend_from_slice(&(data.len() as u32).to_le_bytes());
        local.extend_from_slice(&(data.len() as u32).to_le_bytes());
        local.extend_from_slice(&(name.len() as u16).to_le_bytes());
        local.extend_from_slice(&0u16.to_le_bytes());
        local.extend_from_slice(name.as_bytes());
        out.extend_from_slice(&local);
        out.extend_from_slice(data);
        central.extend_from_slice(b"PK\x01\x02");
        central.extend_from_slice(&[20, 0, 20, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        central.extend_from_slice(&crc.to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(data.len() as u32).to_le_bytes());
        central.extend_from_slice(&(name.len() as u16).to_le_bytes());
        central.extend_from_slice(&[0; 12]);
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let start = out.len() as u32;
    out.extend_from_slice(&central);
    out.extend_from_slice(b"PK\x05\x06\0\0\0\0");
    out.extend_from_slice(&(members.len() as u16).to_le_bytes());
    out.extend_from_slice(&(members.len() as u16).to_le_bytes());
    out.extend_from_slice(&(central.len() as u32).to_le_bytes());
    out.extend_from_slice(&start.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes());
    out
}

#[test]
fn the_plain_fixture_yields_the_exact_payload() {
    let payload = open(&fixture("marvin-plain.vd4")).unwrap();
    assert_eq!(payload.bytes(), fixture("src-vd/MARVIN/ets.vd_").as_slice());
    let container = payload.container();
    assert_eq!(container.member_kind, LegacyMemberKind::ProductDatabase);
    assert_eq!(container.member_name, "MARVIN/ets.vd_");
    assert!(!container.encrypted);
}

#[test]
fn an_encrypted_member_is_handed_out_undecrypted_with_its_check_bytes() {
    let bytes = fixture("marvin-encrypted.vd4");
    let member = read_legacy_member(&bytes).unwrap();
    let container = member.container().clone();
    assert!(container.encrypted);
    let stream = member.encrypted_stream().expect("encrypted stream");
    assert_eq!(stream.len() as u64, container.compressed_size);
    // The stream is the file's own bytes, ending where the central
    // directory begins: nothing was decrypted or copied.
    let (_, central) = header_offsets(&bytes);
    assert_eq!(&bytes[central - stream.len()..central], stream);
    assert_eq!(
        member.check_bytes().crc32_high_byte,
        (container.crc32 >> 24) as u8
    );
    assert!(matches!(
        member.open_unencrypted(),
        Err(LegacyError::PasswordRequired)
    ));
    let plain = fixture("marvin-plain.vd4");
    assert!(read_legacy_member(&plain)
        .unwrap()
        .encrypted_stream()
        .is_none());
}

#[test]
fn detection_reads_metadata_only_and_needs_no_password() {
    let bytes = fixture("marvin-encrypted.vd4");
    let container = detect_legacy_container(&bytes).expect("legacy container");
    assert_eq!(container.member_kind, LegacyMemberKind::ProductDatabase);
    assert_eq!(container.sha256, sha256_hex(&bytes));
    assert_eq!(container.len, bytes.len());
    let project = detect_legacy_container(&fixture("marvin-project.pr5")).unwrap();
    assert_eq!(project.member_kind, LegacyMemberKind::ProjectExport);
}

#[test]
fn ordinary_and_broken_inputs_are_not_detected_as_legacy() {
    assert!(detect_legacy_container(b"not a zip at all").is_none());
    assert!(detect_legacy_container(&stored_zip(&[("knx_master.xml", b"<KNX/>")])).is_none());
    assert!(detect_legacy_container(&stored_zip(&[("ets.vd_.txt", b"x")])).is_none());
    // A second member makes it something else; it is not opened either.
    let two = stored_zip(&[("ets.vd_", b"EX-IM"), ("readme.txt", b"hi")]);
    assert!(detect_legacy_container(&two).is_none());
    assert!(matches!(
        open(&two),
        Err(LegacyError::NotLegacyContainer { .. })
    ));
}

#[test]
fn member_names_match_case_insensitively_on_their_basename() {
    for name in ["ETS.VD_", "a/b/Ets2.vd_", "dir\\ets.pr_"] {
        let zip = stored_zip(&[(name, b"EX-IM\r\n")]);
        assert!(detect_legacy_container(&zip).is_some(), "{name}");
    }
}

#[test]
fn a_member_that_does_not_start_the_file_is_refused() {
    let payload = fixture("src-vd/MARVIN/ets.vd_");
    let zip = stored_zip_after(b"SFX-stub", &[("MARVIN/ets.vd_", &payload)]);
    assert!(detect_legacy_container(&zip).is_some());
    assert!(matches!(
        open(&zip),
        Err(LegacyError::InvalidContainer { .. })
    ));
}

#[test]
fn strong_encryption_and_aes_are_refused_by_name() {
    let strong = with_field("marvin-encrypted.vd4", FLAGS, &0x0041u16.to_le_bytes());
    assert!(matches!(
        open(&strong),
        Err(LegacyError::UnsupportedEncryption { .. })
    ));
    let aes = with_field("marvin-encrypted.vd4", METHOD, &99u16.to_le_bytes());
    assert!(matches!(
        open(&aes),
        Err(LegacyError::UnsupportedEncryption { .. })
    ));
}

#[test]
fn compression_other_than_stored_or_deflate_is_refused() {
    let bzip2 = with_field("marvin-plain.vd4", METHOD, &12u16.to_le_bytes());
    assert!(matches!(
        open(&bzip2),
        Err(LegacyError::UnsupportedCompression { method: 12 })
    ));
}

#[test]
fn a_stored_member_opens_too() {
    let payload = fixture("src-vd/MARVIN/ets.vd_");
    let zip = stored_zip(&[("MARVIN/ets.vd_", &payload)]);
    assert_eq!(open(&zip).unwrap().bytes(), payload.as_slice());
}

#[test]
fn an_oversized_declared_payload_is_refused_before_allocation() {
    let huge = with_field(
        "marvin-plain.vd4",
        UNCOMPRESSED,
        &(64 * 1024 * 1024 + 1u32).to_le_bytes(),
    );
    assert!(matches!(open(&huge), Err(LegacyError::SizeLimit { .. })));
}

#[test]
fn inflation_beyond_the_declared_size_is_refused() {
    let small = with_field("marvin-plain.vd4", UNCOMPRESSED, &100u32.to_le_bytes());
    match open(&small) {
        Err(LegacyError::Corrupt { reason }) => {
            assert!(reason.contains("declared size"), "{reason}")
        }
        other => panic!("expected LegacyError::Corrupt, got {other:?}"),
    }
}

#[test]
fn a_crc_mismatch_on_an_unencrypted_member_is_corruption() {
    let plain = with_field("marvin-plain.vd4", CRC, &0u32.to_le_bytes());
    match open(&plain) {
        Err(LegacyError::Corrupt { reason }) => assert!(reason.contains("CRC"), "{reason}"),
        other => panic!("expected LegacyError::Corrupt, got {other:?}"),
    }
}

#[test]
fn a_truncated_or_shifted_archive_is_refused_without_panicking() {
    let bytes = fixture("marvin-encrypted.vd4");
    for cut in [0, 10, 30, 100, bytes.len() / 2, bytes.len() - 1] {
        let _ = open(&bytes[..cut]);
        let _ = detect_legacy_container(&bytes[..cut]);
    }
    let mut prefixed = b"garbage".to_vec();
    prefixed.extend_from_slice(&bytes);
    assert!(open(&prefixed).is_err());
}

#[test]
fn inspection_summarises_tables_and_products() {
    let bytes = fixture("marvin-plain.vd4");
    let inspection = inspect_payload(&open(&bytes).unwrap()).unwrap();
    assert_eq!(inspection.source_sha256, sha256_hex(&bytes));
    assert_eq!(
        inspection.payload_sha256,
        sha256_hex(&fixture("src-vd/MARVIN/ets.vd_"))
    );
    assert_eq!(inspection.content, ExImContent::ProductDatabase);
    assert_eq!(inspection.format_version.as_deref(), Some("6.2"));
    assert_eq!(inspection.producer.as_deref(), Some("ETS3"));
    assert_eq!(
        inspection.exported_at.as_deref(),
        Some("2026-10-08 09:00:00")
    );
    assert_eq!(inspection.tables.len(), 7);
    assert_eq!(inspection.total_rows(), 10);
    let catalog = inspection
        .tables
        .iter()
        .find(|t| t.name == "catalog_entry")
        .unwrap();
    assert_eq!((catalog.columns, catalog.rows), (5, 2));
    assert_eq!(inspection.products.len(), 2);
    let second = &inspection.products[1];
    assert_eq!(second.order_number.as_deref(), Some("MT-42-B"));
    assert_eq!(
        second.name.as_deref(),
        Some("Heart of Gold Sensor b\u{e9}ta")
    );
    assert_eq!(second.manufacturer.as_deref(), Some("Marvin Test"));
    assert_eq!(second.program_name.as_deref(), Some("Improbability Drive"));
    assert_eq!(second.program_version.as_deref(), Some("16"));
    assert_eq!(second.mask_version.as_deref(), Some("MV-0701"));
    assert!(inspection.diagnostics.is_empty());
}

#[test]
fn a_project_export_inspects_without_products() {
    let payload = fixture("src-pr/MARVIN/ets.pr_");
    let zip = stored_zip(&[("MARVIN/ets.pr_", &payload)]);
    let inspection = inspect_payload(&open(&zip).unwrap()).unwrap();
    assert_eq!(inspection.content, ExImContent::ProjectExport);
    assert_eq!(inspection.member_kind, LegacyMemberKind::ProjectExport);
    assert!(inspection.products.is_empty());
}

#[test]
fn a_legacy_file_renamed_to_knxprod_is_refused_by_content_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let count = |conn: &knx_productdb::Connection| -> i64 {
        conn.query_row(
            "SELECT (SELECT count(*) FROM package) + (SELECT count(*) FROM source_file)",
            [],
            |r| r.get(0),
        )
        .unwrap()
    };
    let before = count(&conn);
    let bytes = fixture("marvin-encrypted.vd4");
    match install_package(&conn, "renamed.knxprod", &bytes) {
        Err(PackageError::LegacyExIm { kind, sha256, len }) => {
            assert_eq!(kind, LegacyMemberKind::ProductDatabase);
            assert_eq!(sha256, sha256_hex(&bytes));
            assert_eq!(len, bytes.len());
        }
        other => panic!("expected PackageError::LegacyExIm, got {other:?}"),
    }
    let message = install_package(&conn, "renamed.knxprod", &bytes)
        .unwrap_err()
        .to_string();
    assert!(message.contains("legacy ETS3"), "{message}");
    assert_eq!(count(&conn), before);
}
