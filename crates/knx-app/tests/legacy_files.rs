//! Legacy EX-IM files opened with the user's password: right, wrong, missing, empty, redacted.

use knx_app::legacy::{import_legacy_file, inspect_legacy_file, open_legacy_file, LegacyPassword};
use knx_productdb::legacy::LegacyPublishError;
use knx_productdb::legacy::{ExImContent, LegacyError, LegacyMemberKind};

const PASSWORD: &str = "marvin-synthetic";

fn fixture(path: &str) -> Vec<u8> {
    let full = knx_testsupport::workspace_root()
        .join("crates/knx-productdb/fixtures/legacy")
        .join(path);
    std::fs::read(&full).unwrap_or_else(|e| panic!("fixture {full:?} unreadable: {e}"))
}

fn password() -> LegacyPassword {
    LegacyPassword::new(PASSWORD)
}

#[test]
fn the_right_password_yields_the_exact_plaintext_payload() {
    let payload = open_legacy_file(&fixture("marvin-encrypted.vd4"), Some(&password())).unwrap();
    assert_eq!(payload.bytes(), fixture("src-vd/MARVIN/ets.vd_").as_slice());
    assert_eq!(
        payload.container().member_kind,
        LegacyMemberKind::ProductDatabase
    );
    assert!(payload.container().encrypted);
}

#[test]
fn a_missing_or_empty_password_is_a_named_requirement() {
    let bytes = fixture("marvin-encrypted.vd4");
    assert!(matches!(
        open_legacy_file(&bytes, None),
        Err(LegacyError::PasswordRequired)
    ));
    // An empty password is treated as absent, so an always-sending client is
    // asked for one rather than told it is wrong.
    assert!(matches!(
        open_legacy_file(&bytes, Some(&LegacyPassword::new(""))),
        Err(LegacyError::PasswordRequired)
    ));
}

#[test]
fn wrong_passwords_never_produce_a_payload() {
    // ZipCrypto's check byte accepts roughly one wrong password in 128 (both
    // conventions tried), so a few hundred attempts also exercise the
    // inflate/CRC refusal after a falsely passed check byte.
    let bytes = fixture("marvin-encrypted.vd4");
    let (mut at_check_byte, mut after_check_byte) = (0, 0);
    for attempt in 0..400 {
        let wrong = LegacyPassword::new(format!("canary-wrong-{attempt}"));
        match open_legacy_file(&bytes, Some(&wrong)) {
            Err(LegacyError::WrongPassword) => at_check_byte += 1,
            Err(LegacyError::WrongPasswordOrCorrupt) => after_check_byte += 1,
            other => panic!("attempt {attempt}: expected a wrong-password refusal, got {other:?}"),
        }
    }
    assert!(
        at_check_byte > 300,
        "only {at_check_byte} of 400 refused at the check byte"
    );
    assert!(
        after_check_byte > 0,
        "no wrong password passed the check byte; the false-accept path went untested"
    );
}

#[test]
fn a_password_on_an_unencrypted_file_changes_nothing() {
    let plain = fixture("marvin-plain.vd4");
    let without = open_legacy_file(&plain, None).unwrap();
    let with = open_legacy_file(&plain, Some(&password())).unwrap();
    assert_eq!(without.bytes(), with.bytes());
    assert!(!without.container().encrypted);
}

#[test]
fn the_password_never_appears_in_debug_or_error_text() {
    let secret = LegacyPassword::new("canary-secret-password");
    assert!(!format!("{secret:?}").contains("canary-secret-password"));
    let error = open_legacy_file(&fixture("marvin-encrypted.vd4"), Some(&secret)).unwrap_err();
    assert!(!error.to_string().contains("canary-secret-password"));
    assert!(!format!("{error:?}").contains("canary-secret-password"));
    let inspection =
        inspect_legacy_file(&fixture("marvin-encrypted.vd4"), Some(&password())).unwrap();
    assert!(!format!("{inspection:?}").contains(PASSWORD));
}

#[test]
fn inspection_runs_end_to_end_for_both_kinds() {
    let database =
        inspect_legacy_file(&fixture("marvin-encrypted.vd4"), Some(&password())).unwrap();
    assert_eq!(database.content, ExImContent::ProductDatabase);
    assert_eq!(database.products.len(), 2);
    let project = inspect_legacy_file(&fixture("marvin-project.pr5"), Some(&password())).unwrap();
    assert_eq!(project.content, ExImContent::ProjectExport);
    assert_eq!(project.member_kind, LegacyMemberKind::ProjectExport);
}

#[test]
fn a_crc_mismatch_after_a_passed_check_byte_is_wrong_password_or_corrupt() {
    // Flip a low byte of the CRC-32 in both headers. Its high byte is the
    // password check byte and stays, so the right password still passes the
    // check and only the CRC of the inflated payload disagrees.
    let mut bytes = fixture("marvin-encrypted.vd4");
    let eocd = bytes
        .windows(4)
        .rposition(|w| w == b"PK\x05\x06")
        .expect("end record");
    let central = u32::from_le_bytes(bytes[eocd + 16..eocd + 20].try_into().unwrap()) as usize;
    for at in [14, central + 16] {
        bytes[at] ^= 0x5A;
    }
    assert!(matches!(
        open_legacy_file(&bytes, Some(&password())),
        Err(LegacyError::WrongPasswordOrCorrupt)
    ));
}

fn products() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    (dir, conn)
}

fn rows(conn: &knx_productdb::Connection, table: &str) -> i64 {
    conn.query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
        .unwrap()
}

#[test]
fn the_encrypted_and_the_plain_file_share_one_publication() {
    let (_dir, conn) = products();
    let encrypted = fixture("marvin-program.vd4");
    let first = import_legacy_file(&conn, "marvin.vd4", &encrypted, Some(&password())).unwrap();
    assert!(!first.skipped);
    assert_eq!(first.programs.len(), 1);
    let plain = fixture("marvin-program-plain.vd4");
    let second = import_legacy_file(&conn, "marvin-plain.vd4", &plain, None).unwrap();
    assert!(second.skipped);
    assert_eq!(second.payload_sha256, first.payload_sha256);
    assert_ne!(second.original_sha256, first.original_sha256);
    assert_eq!(rows(&conn, "application_program"), 1);
    assert_eq!(rows(&conn, "legacy_source_file"), 2);
}

/// The installer-tree layout of the measured `.vd5`: an encrypted mask
/// image beside the encrypted payload. The payload decrypts to the very
/// bytes of `marvin-program.vd4`'s, so both are one publication; the mask
/// is not read, stays in the stored original and is reported every time.
#[test]
fn an_installer_tree_vd5_imports_its_payload_and_reports_the_other_member() {
    let bytes = fixture("marvin-installer.vd5");
    let payload = open_legacy_file(&bytes, Some(&password())).unwrap();
    assert_eq!(
        payload.bytes(),
        fixture("src-vd-program/MARVIN/ets.vd_").as_slice()
    );
    let others = &payload.container().other_members;
    assert_eq!(others.len(), 1, "{others:?}");
    assert_eq!(
        others[0].name,
        "Program Files (x86)/Common Files/MARVIN sc/MASK/mask4242.bin"
    );
    assert!(others[0].encrypted);
    let inspection = inspect_legacy_file(&bytes, Some(&password())).unwrap();
    assert_eq!(&inspection.other_members, others);

    let unread = |report: &knx_productdb::legacy::LegacyPublishReport| {
        report
            .diagnostics
            .iter()
            .filter(|(kind, _)| kind == "unread-member")
            .map(|(_, detail)| detail.clone())
            .collect::<Vec<_>>()
    };
    let (_dir, conn) = products();
    let first = import_legacy_file(&conn, "marvin.vd5", &bytes, Some(&password())).unwrap();
    assert!(!first.skipped);
    assert_eq!(first.programs.len(), 1);
    let details = unread(&first);
    assert_eq!(details.len(), 1, "{details:?}");
    assert!(
        details[0].contains("mask4242.bin (14 bytes, encrypted)"),
        "{details:?}"
    );
    // The file, not the payload, has the member: no stored diagnostic row.
    let stored: i64 = conn
        .query_row(
            "SELECT count(*) FROM legacy_diagnostic WHERE kind = 'unread-member'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(stored, 0);
    // The original, mask included, is stored byte for byte.
    let original: Vec<u8> = conn
        .query_row(
            "SELECT bytes FROM source_file WHERE sha256 = ?1",
            [&first.original_sha256],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(original, bytes);

    // The single-member file with the same payload is the same publication
    // and reports no other member; the installer file again reports it.
    let single = import_legacy_file(
        &conn,
        "marvin.vd4",
        &fixture("marvin-program.vd4"),
        Some(&password()),
    )
    .unwrap();
    assert!(single.skipped);
    assert_eq!(single.payload_sha256, first.payload_sha256);
    assert!(unread(&single).is_empty());
    let again = import_legacy_file(&conn, "again.vd5", &bytes, Some(&password())).unwrap();
    assert!(again.skipped);
    assert_eq!(unread(&again), details);
    assert_eq!(rows(&conn, "application_program"), 1);
}

#[test]
fn the_password_is_stored_nowhere() {
    let (dir, conn) = products();
    let secret = "canary-import-password";
    // The fixture's password is public; plant a canary by importing the
    // plain file with it as an (ignored) password, then the encrypted one.
    import_legacy_file(
        &conn,
        "plain.vd4",
        &fixture("marvin-program-plain.vd4"),
        Some(&LegacyPassword::new(secret)),
    )
    .unwrap();
    import_legacy_file(
        &conn,
        "marvin.vd4",
        &fixture("marvin-program.vd4"),
        Some(&password()),
    )
    .unwrap();
    drop(conn);
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        for needle in [secret, PASSWORD] {
            assert!(
                !bytes.windows(needle.len()).any(|w| w == needle.as_bytes()),
                "{needle} found in the product database files"
            );
        }
    }
}

#[test]
fn a_wrong_password_writes_nothing() {
    let (_dir, conn) = products();
    let error = import_legacy_file(
        &conn,
        "marvin.vd4",
        &fixture("marvin-program.vd4"),
        Some(&LegacyPassword::new("wrong")),
    )
    .unwrap_err();
    assert!(matches!(error, LegacyPublishError::Legacy(_)), "{error:?}");
    for table in ["source_file", "legacy_source", "application_program"] {
        assert_eq!(rows(&conn, table), 0, "{table}");
    }
}

#[test]
fn secret_column_values_of_an_encrypted_file_are_stored_nowhere() {
    let (dir, conn) = products();
    import_legacy_file(
        &conn,
        "marvin.vd4",
        &fixture("marvin-program.vd4"),
        Some(&password()),
    )
    .unwrap();
    drop(conn);
    for entry in std::fs::read_dir(dir.path()).unwrap() {
        let bytes = std::fs::read(entry.unwrap().path()).unwrap();
        for needle in ["Zaphod42", "Beeblebrox"] {
            assert!(
                !bytes.windows(needle.len()).any(|w| w == needle.as_bytes()),
                "{needle} found in the product database files"
            );
        }
    }
}
