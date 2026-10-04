//! Password-protected `.knxproj` import through the library entry points (AR08, KL-13).

use knx_etsproj::{ContainerError, ImportFailure, ProjectPassword};
use knx_testsupport::{zipcrypto_minimal_knxproj_bytes, ZIPCRYPTO_MINIMAL_PASSWORD};

fn import(password: Option<&ProjectPassword>) -> Result<knx_etsproj::ImportOutcome, ImportFailure> {
    knx_etsproj::import_knxproj_bytes_with(
        zipcrypto_minimal_knxproj_bytes(),
        "protected.knxproj",
        password,
        &(),
    )
}

#[test]
fn the_right_password_imports_the_protected_project() {
    let password = ProjectPassword::new(ZIPCRYPTO_MINIMAL_PASSWORD);
    let outcome = import(Some(&password)).expect("protected project imports with its password");
    assert_eq!(outcome.project.info.name, "Minimal");
    assert_eq!(outcome.project.devices.iter().count(), 1);
}

#[test]
fn without_a_password_the_import_names_the_protection() {
    let Err(ImportFailure::Container(ContainerError::PasswordProtected { nested_entry })) =
        import(None)
    else {
        panic!("expected PasswordProtected");
    };
    assert_eq!(nested_entry, "P-0001.zip");
}

#[test]
fn a_wrong_password_is_refused_as_such() {
    let wrong = ProjectPassword::new("not-the-password");
    assert!(matches!(
        import(Some(&wrong)),
        Err(ImportFailure::Container(
            ContainerError::WrongPassword { .. }
        ))
    ));
}

#[test]
fn a_password_on_an_unprotected_project_changes_nothing() {
    let password = ProjectPassword::new("irrelevant");
    let with = knx_etsproj::import_knxproj_bytes_with(
        knx_testsupport::minimal_knxproj_bytes(),
        "minimal.knxproj",
        Some(&password),
        &(),
    )
    .unwrap();
    let without = knx_etsproj::import_knxproj_bytes(
        knx_testsupport::minimal_knxproj_bytes(),
        "minimal.knxproj",
    )
    .unwrap();
    assert_eq!(with.project, without.project);
}

#[test]
fn the_password_never_reaches_debug_output_reports_or_errors() {
    let password = ProjectPassword::new(ZIPCRYPTO_MINIMAL_PASSWORD);
    assert!(!format!("{password:?}").contains(ZIPCRYPTO_MINIMAL_PASSWORD));
    let outcome = import(Some(&password)).unwrap();
    assert!(!format!("{:?}", outcome.report).contains(ZIPCRYPTO_MINIMAL_PASSWORD));
    assert!(!format!("{:?}", outcome.project).contains(ZIPCRYPTO_MINIMAL_PASSWORD));
    let wrong = ProjectPassword::new("canary-wrong-password");
    let error = import(Some(&wrong)).unwrap_err();
    assert!(!error.to_string().contains("canary-wrong-password"));
    assert!(!format!("{error:?}").contains("canary-wrong-password"));
}

/// ZipCrypto's check byte lets about 1 in 128 wrong passwords through
/// (KNOWN_LIMITATIONS §13). This one is such a false accept for this
/// fixture: the decrypted bytes are not a valid deflate stream. That is
/// still a wrong password, not "corrupt data" — found by the CLI test.
#[test]
fn a_wrong_password_that_passes_the_check_byte_is_still_reported_as_wrong() {
    let lucky = ProjectPassword::new("canary-wrong-password");
    let result = import(Some(&lucky));
    assert!(
        matches!(
            result,
            Err(ImportFailure::Container(
                ContainerError::WrongPassword { .. }
            ))
        ),
        "{result:?}"
    );
}

/// The opaque store keeps decrypted bytes and drops the ZipCrypto
/// ciphertext, so an export would come back unprotected. That loss is
/// reported, not discovered later (KNOWN_LIMITATIONS §13).
#[test]
fn a_decrypted_import_reports_that_the_protection_is_not_preserved() {
    let password = ProjectPassword::new(ZIPCRYPTO_MINIMAL_PASSWORD);
    let outcome = import(Some(&password)).unwrap();
    let entry = outcome
        .report
        .unsupported
        .iter()
        .find(|u| u.what.contains("password protection"))
        .expect("an unsupported-feature entry for the dropped protection");
    assert!(entry.what.contains("P-0001.zip"), "{entry:?}");
    assert!(entry.consequence.contains("unprotected"), "{entry:?}");

    let plain = knx_etsproj::import_knxproj_bytes(
        knx_testsupport::minimal_knxproj_bytes(),
        "minimal.knxproj",
    )
    .unwrap();
    assert!(!plain
        .report
        .unsupported
        .iter()
        .any(|u| u.what.contains("password protection")));
}
