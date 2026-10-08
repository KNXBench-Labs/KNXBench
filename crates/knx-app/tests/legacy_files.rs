//! Legacy EX-IM files opened with the user's password: right, wrong, missing, empty, redacted.

use knx_app::legacy::{inspect_legacy_file, open_legacy_file, LegacyPassword};
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
