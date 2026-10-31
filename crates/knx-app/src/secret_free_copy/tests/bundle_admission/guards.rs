//! Pins generic bundle refusal and admitted withholding at existing helper guards.

use super::super::*;
use super::fixture::*;
use std::path::PathBuf;

#[test]
fn bundle_admission_report_malformed_input_is_generically_unavailable() {
    let report = serde_json::json!("not an import report");
    assert!(import_report_without_secrets(&report).is_err());
    assert_bundle_unavailable(import_report_for_bundle(&report));
}

#[test]
fn bundle_admission_report_depth_budget_is_generically_unavailable() {
    let mut report = serde_json::json!({"name": "safe"});
    for _ in 0..65 {
        report = serde_json::json!({"child": report});
    }
    assert!(import_report_without_secrets(&report).is_err());
    assert_bundle_unavailable(import_report_for_bundle(&report));
}

#[test]
fn bundle_admission_native_sidecar_is_generically_unavailable_and_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(dir.path(), Credential::None, Duplicate::None, Extra::None);
    let mut sidecar = fixture.native.as_os_str().to_os_string();
    sidecar.push("-journal");
    let sidecar = PathBuf::from(sidecar);
    std::fs::write(&sidecar, b"synthetic active journal").unwrap();
    let native_before = std::fs::read(&fixture.native).unwrap();
    let sidecar_before = std::fs::read(&sidecar).unwrap();

    assert!(native_project_without_secrets(&fixture.native).is_err());
    assert_bundle_unavailable(native_project_for_bundle(&fixture.native));

    assert!(std::fs::read(&fixture.native).unwrap() == native_before);
    assert!(std::fs::read(&sidecar).unwrap() == sidecar_before);
}

#[test]
fn bundle_admission_native_uninspectable_row_is_withheld_and_copy_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::None,
        Duplicate::None,
        Extra::WideUninspectable,
    );
    let ordinary = ordinary_native(&fixture.native);
    assert!(ordinary
        .withheld
        .iter()
        .any(|note| note.contains("wide.xml")));
    let expected = expected_parts(ordinary);

    let actual = admitted(native_project_for_bundle(&fixture.native));

    assert_eq!(actual, expected);
    let path = dir.path().join("uninspectable-admitted.knxdb");
    std::fs::write(&path, &actual.0).unwrap();
    let store = knx_store::open_existing_read_only(&path).unwrap();
    let rows = knx_store::load_opaque(&store.conn).unwrap();
    assert!(!rows.iter().any(|row| row.source_path == "wide.xml"));
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_native_nonproject_is_generically_unavailable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("malformed.knxdb");
    std::fs::write(&path, b"not a native project").unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(native_project_without_secrets(&path).is_err());
    assert_bundle_unavailable(native_project_for_bundle(&path));
    assert!(std::fs::read(path).unwrap() == before);
}
