//! Regresses immutable archive snapshots and safe published metadata.

use super::*;

#[test]
fn secret_free_review_r6_findings_metadata_is_safe() {
    let mut findings = Findings::default();
    findings.remember(b"MetaPasswordWitness");
    findings.redact(
        "MetaPasswordWitness.xml:/a".into(),
        "password-MetaPasswordWitness".into(),
    );
    findings.withhold("MetaPasswordWitness.xml", "cannot be decoded");
    let copy = findings.finish(Vec::new()).unwrap();
    assert!(
        !serde_json::to_string(&copy)
            .unwrap()
            .contains("MetaPasswordWitness"),
        "serialized findings leaked removed password"
    );
    assert!(!format!("{copy:?}").contains("MetaPasswordWitness"));
}

#[test]
fn secret_free_review_r6_error_debug_is_safe_independently() {
    let error = SecretFreeCopyError::Sql(knx_store::SqlError::InvalidParameterName(
        "MetaPasswordWitness".into(),
    ));
    assert!(
        !format!("{error:?}").contains("MetaPasswordWitness"),
        "Debug leaked storage text"
    );
    assert!(!error.to_string().contains("MetaPasswordWitness"));
}

#[test]
fn secret_free_review_r5_source_replacement_is_not_consumed_by_second_pass() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    let first = knx_testsupport::zip_with_entries(&[("safe.txt", b"first snapshot")]);
    let second = knx_testsupport::zip_with_entries(&[
        ("secret.xml", br#"<a Password="RacePasswordWitness"/>"#),
        ("notes.txt", b"RacePasswordWitness"),
    ]);
    std::fs::write(&path, &first).unwrap();
    let copy =
        archive::copy_between_passes(&path, || std::fs::write(&path, &second).unwrap()).unwrap();
    let out = members(&copy.bytes);
    assert!(
        !out.values().any(|v| contains(v, "RacePasswordWitness")),
        "pass-two secret survived"
    );
    assert_eq!(
        copy.bytes, first,
        "output must be wholly the admitted snapshot"
    );
    assert_eq!(std::fs::read(&path).unwrap(), second);
}

#[test]
fn secret_free_review_r6_archive_member_names_and_notes() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    std::fs::write(
        &path,
        knx_testsupport::zip_with_entries(&[
            (
                "MetaPasswordWitness.xml",
                br#"<a Password="MetaPasswordWitness"/>"#,
            ),
            ("safe.bin", b"keep binary"),
        ]),
    )
    .unwrap();
    let copy = import_source_without_secrets(&path).unwrap();
    assert_eq!(members(&copy.bytes)["safe.bin"], b"keep binary");
    assert!(
        !members(&copy.bytes)
            .keys()
            .any(|name| name.contains("MetaPasswordWitness")),
        "member name leaked removed password"
    );
    assert!(!serde_json::to_string(&copy)
        .unwrap()
        .contains("MetaPasswordWitness"));
    assert!(!format!("{copy:?}").contains("MetaPasswordWitness"));
    assert!(!copy.withheld.is_empty());
}

fn duplicate_metadata_archive() -> Vec<u8> {
    let mut bytes = knx_testsupport::zip_with_entries(&[
        ("MetaPasswordWitnessA.xml", b"<a/>"),
        ("MetaPasswordWitnessB.xml", b"<a/>"),
    ]);
    let needle = b"MetaPasswordWitnessB.xml";
    for index in 0..bytes.len() - needle.len() {
        if &bytes[index..index + needle.len()] == needle {
            bytes[index + b"MetaPasswordWitness".len()] = b'A';
        }
    }
    bytes
}

#[test]
fn secret_free_review_r6_outer_container_error_does_not_echo_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    std::fs::write(&path, duplicate_metadata_archive()).unwrap();
    let error = import_source_without_secrets(&path).unwrap_err();
    assert!(
        !error.to_string().contains("MetaPasswordWitness"),
        "Display leaked untrusted container metadata"
    );
    assert!(
        !format!("{error:?}").contains("MetaPasswordWitness"),
        "Debug leaked untrusted container metadata"
    );
}

#[test]
fn secret_free_review_r6_nested_container_error_has_safe_reason() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("input.knxproj");
    let bytes = knx_testsupport::zip_with_entries(&[
        ("plugin.zip", duplicate_metadata_archive().as_slice()),
        ("safe.bin", b"keep"),
    ]);
    std::fs::write(&path, bytes).unwrap();
    let copy = import_source_without_secrets(&path).unwrap();
    assert!(!members(&copy.bytes).contains_key("plugin.zip"));
    assert_eq!(members(&copy.bytes)["safe.bin"], b"keep");
    assert!(!serde_json::to_string(&copy)
        .unwrap()
        .contains("MetaPasswordWitness"));
    assert!(!format!("{copy:?}").contains("MetaPasswordWitness"));
}
