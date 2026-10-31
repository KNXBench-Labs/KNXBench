//! Covers report admission, short credentials, encoded copies, and generic refusal.

use super::super::*;
use super::fixture::*;
use serde_json::{json, Value};

fn push_synthetic_digestless_report_row(report: &mut Value, name: &str, xpath: &str, sample: &str) {
    report["unknown"].as_array_mut().unwrap().push(json!({
        "source_path": "synthetic-bundle-admission.xml",
        "xpath": xpath,
        "kind": "Attribute",
        "name": name,
        "occurrences": 1,
        "sample": sample,
    }));
}

fn report_copy_case(copy: &'static str, duplicate: Duplicate) {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::Field("Password", "123"),
        duplicate,
        Extra::None,
    );
    let before = fixture.report.clone();
    let ordinary = ordinary_report(&fixture.report);
    match duplicate {
        Duplicate::Body(_) => assert!(String::from_utf8_lossy(&ordinary.bytes).contains(copy)),
        Duplicate::SourcePath(_) | Duplicate::Xpath(_) => {
            assert!(serde_json::to_string(&ordinary).unwrap().contains(copy));
        }
        Duplicate::None => unreachable!(),
    }
    assert_bundle_unavailable(import_report_for_bundle(&fixture.report));
    assert_eq!(fixture.report, before);
    fixture.assert_sources_unchanged();
}

macro_rules! report_copy_test {
    ($name:ident, $copy:literal, $place:expr) => {
        #[test]
        fn $name() {
            report_copy_case($copy, $place);
        }
    };
}

report_copy_test!(
    bundle_admission_report_body_literal_password123_refuses,
    "123",
    Duplicate::Body("123")
);
report_copy_test!(
    bundle_admission_report_body_hex_password123_refuses,
    "313233",
    Duplicate::Body("313233")
);
report_copy_test!(
    bundle_admission_report_body_base64_password123_refuses,
    "MTIz",
    Duplicate::Body("MTIz")
);
report_copy_test!(
    bundle_admission_report_body_percent_password123_refuses,
    "%31%32%33",
    Duplicate::Body("%31%32%33")
);
report_copy_test!(
    bundle_admission_report_source_path_literal_password123_refuses,
    "123",
    Duplicate::SourcePath("123")
);
report_copy_test!(
    bundle_admission_report_source_path_hex_password123_refuses,
    "313233",
    Duplicate::SourcePath("313233")
);
report_copy_test!(
    bundle_admission_report_source_path_base64_password123_refuses,
    "MTIz",
    Duplicate::SourcePath("MTIz")
);
report_copy_test!(
    bundle_admission_report_source_path_percent_password123_refuses,
    "%31%32%33",
    Duplicate::SourcePath("%31%32%33")
);
report_copy_test!(
    bundle_admission_report_xpath_literal_password123_refuses,
    "123",
    Duplicate::Xpath("123")
);
report_copy_test!(
    bundle_admission_report_xpath_hex_password123_refuses,
    "313233",
    Duplicate::Xpath("313233")
);
report_copy_test!(
    bundle_admission_report_xpath_base64_password123_refuses,
    "MTIz",
    Duplicate::Xpath("MTIz")
);
report_copy_test!(
    bundle_admission_report_xpath_percent_password123_refuses,
    "%31%32%33",
    Duplicate::Xpath("%31%32%33")
);

fn report_field_case(name: &'static str) {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::Field(name, "123"),
        Duplicate::None,
        Extra::None,
    );
    let before = fixture.report.clone();
    ordinary_report(&fixture.report);
    assert_bundle_unavailable(import_report_for_bundle(&fixture.report));
    assert_eq!(fixture.report, before);
    fixture.assert_sources_unchanged();
}

macro_rules! report_field_test {
    ($name:ident, $field:literal) => {
        #[test]
        fn $name() {
            report_field_case($field);
        }
    };
}

report_field_test!(bundle_admission_report_key_refuses, "Key");
report_field_test!(bundle_admission_report_tool_key_refuses, "ToolKey");
report_field_test!(bundle_admission_report_fdsk_refuses, "FDSK");
report_field_test!(
    bundle_admission_report_backbone_key_refuses,
    "IPRoutingBackboneKey"
);
report_field_test!(bundle_admission_report_password_refuses, "Password");
report_field_test!(
    bundle_admission_report_password_hash_refuses,
    "PasswordHash"
);
report_field_test!(
    bundle_admission_report_project_password_refuses,
    "ProjectPassword"
);
report_field_test!(
    bundle_admission_report_authentication_code_refuses,
    "DeviceAuthenticationCode"
);
report_field_test!(
    bundle_admission_report_authentication_code_hash_refuses,
    "DeviceAuthenticationCodeHash"
);
report_field_test!(
    bundle_admission_report_management_password_refuses,
    "DeviceManagementPassword"
);
report_field_test!(
    bundle_admission_report_management_password_hash_refuses,
    "DeviceManagementPasswordHash"
);
report_field_test!(bundle_admission_report_bcu_key_refuses, "BCUKey");
report_field_test!(
    bundle_admission_report_loaded_tool_key_refuses,
    "LoadedToolKey"
);
report_field_test!(
    bundle_admission_report_loaded_backbone_key_refuses,
    "LoadedIPRoutingBackboneKey"
);
report_field_test!(
    bundle_admission_report_loaded_authentication_hash_refuses,
    "LoadedDeviceAuthenticationCodeHash"
);
report_field_test!(
    bundle_admission_report_loaded_management_hash_refuses,
    "LoadedDeviceManagementPasswordHash"
);

fn report_bus_access_case(part: &'static str) {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::BusAccess(part, "123"),
        Duplicate::None,
        Extra::None,
    );
    let report = fixture.synthetic_supplementary_bus_access_report();
    let before = report.clone();
    ordinary_report(&report);
    assert_bundle_unavailable(import_report_for_bundle(&report));
    assert_eq!(report, before);
    fixture.assert_sources_unchanged();
}

macro_rules! report_bus_access_test {
    ($name:ident, $part:literal) => {
        #[test]
        fn $name() {
            report_bus_access_case($part);
        }
    };
}

report_bus_access_test!(
    bundle_admission_report_bus_access_password_refuses,
    "UserPassword"
);
report_bus_access_test!(bundle_admission_report_bus_access_key_refuses, "TunnelKey");
report_bus_access_test!(
    bundle_admission_report_bus_access_authentication_code_refuses,
    "DeviceAuthenticationCode"
);
report_bus_access_test!(
    bundle_admission_report_bus_access_secret_refuses,
    "SessionSecret"
);

#[test]
fn bundle_admission_report_credential_free_producer_is_admitted() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(dir.path(), Credential::None, Duplicate::None, Extra::None);
    let ordinary = ordinary_report(&fixture.report);
    let expected = expected_parts(ordinary);

    let actual = admitted(import_report_for_bundle(&fixture.report));

    assert_eq!(actual, expected);
    assert_eq!(
        serde_json::from_slice::<Value>(&actual.0).unwrap(),
        fixture.report
    );
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_report_free_access_bcu_key_is_admitted() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::FreeAccessBcuKey,
        Duplicate::None,
        Extra::None,
    );
    let mut report = fixture.report.clone();
    report["unknown"].as_array_mut().unwrap().push(json!({
        "source_path": "P-0001/0.xml",
        "xpath": "/KNX/Project/Installations/Installation",
        "kind": "Attribute",
        "name": "BCUKey",
        "occurrences": 1,
        "sample": "4294967295",
        "sha256": knx_store::project_history::image_hash(b"4294967295"),
    }));
    let expected = expected_parts(ordinary_report(&report));
    let before = report.clone();

    let actual = admitted(import_report_for_bundle(&report));

    assert_eq!(actual, expected);
    assert_eq!(report, before);
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_report_equal_length_secrets_have_the_same_generic_refusal() {
    let first_dir = tempfile::tempdir().unwrap();
    let second_dir = tempfile::tempdir().unwrap();
    let first = produce(
        first_dir.path(),
        Credential::Field("Password", "123"),
        Duplicate::None,
        Extra::None,
    );
    let second = produce(
        second_dir.path(),
        Credential::Field("Password", "789"),
        Duplicate::None,
        Extra::None,
    );
    let first_before = first.report.clone();
    let second_before = second.report.clone();
    ordinary_report(&first.report);
    ordinary_report(&second.report);

    assert_bundle_unavailable(import_report_for_bundle(&first.report));
    assert_bundle_unavailable(import_report_for_bundle(&second.report));
    assert_eq!(first.report, first_before);
    assert_eq!(second.report, second_before);
    first.assert_sources_unchanged();
    second.assert_sources_unchanged();
}

#[test]
fn bundle_admission_report_nonempty_whitespace_password_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(dir.path(), Credential::None, Duplicate::None, Extra::None);
    let mut report = fixture.report.clone();
    push_synthetic_digestless_report_row(
        &mut report,
        "Password",
        "/KNX/Project/DeviceInstance",
        " ",
    );
    let before = report.clone();

    ordinary_report(&report);
    assert_bundle_unavailable(import_report_for_bundle(&report));
    assert_eq!(report, before);
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_report_nonempty_whitespace_bus_access_component_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(dir.path(), Credential::None, Duplicate::None, Extra::None);
    let mut report = fixture.report.clone();
    push_synthetic_digestless_report_row(
        &mut report,
        "Parameter",
        "/KNX/Project/BusAccess",
        "Target=203.0.113.250;UserPassword= ;Port=3671",
    );
    let before = report.clone();

    ordinary_report(&report);
    assert_bundle_unavailable(import_report_for_bundle(&report));
    assert_eq!(report, before);
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_report_truly_empty_recognized_values_are_admitted() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(dir.path(), Credential::None, Duplicate::None, Extra::None);
    let mut report = fixture.report.clone();
    push_synthetic_digestless_report_row(
        &mut report,
        "Password",
        "/KNX/Project/DeviceInstance",
        "",
    );
    push_synthetic_digestless_report_row(
        &mut report,
        "Parameter",
        "/KNX/Project/BusAccess",
        "Target=203.0.113.250;UserPassword=;Port=3671",
    );
    let before = report.clone();
    let expected = expected_parts(ordinary_report(&report));

    let actual = admitted(import_report_for_bundle(&report));

    assert_eq!(actual, expected);
    assert_eq!(report, before);
    fixture.assert_sources_unchanged();
}
