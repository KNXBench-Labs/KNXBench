//! Covers native admission, encoded copies, producer fields, and finding propagation.

use super::super::*;
use super::fixture::*;
use std::path::Path;

fn native_copy_case(copy: &'static str, duplicate: Duplicate) {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::Field("Password", "123"),
        duplicate,
        Extra::None,
    );
    let ordinary = ordinary_native(&fixture.native);
    match duplicate {
        Duplicate::Body(_) => {
            let store = reopened(&ordinary.bytes, dir.path());
            let rows = knx_store::load_opaque(&store.conn).unwrap();
            assert!(rows.iter().any(|row| row.bytes == copy.as_bytes()));
        }
        Duplicate::SourcePath(_) | Duplicate::Xpath(_) => {
            assert!(serde_json::to_string(&ordinary).unwrap().contains(copy));
        }
        Duplicate::None => unreachable!(),
    }
    assert_bundle_unavailable(native_project_for_bundle(&fixture.native));
    fixture.assert_sources_unchanged();
}

macro_rules! native_copy_test {
    ($name:ident, $copy:literal, $place:expr) => {
        #[test]
        fn $name() {
            native_copy_case($copy, $place);
        }
    };
}

native_copy_test!(
    bundle_admission_native_body_literal_password123_refuses,
    "123",
    Duplicate::Body("123")
);
native_copy_test!(
    bundle_admission_native_body_hex_password123_refuses,
    "313233",
    Duplicate::Body("313233")
);
native_copy_test!(
    bundle_admission_native_body_base64_password123_refuses,
    "MTIz",
    Duplicate::Body("MTIz")
);
native_copy_test!(
    bundle_admission_native_body_percent_password123_refuses,
    "%31%32%33",
    Duplicate::Body("%31%32%33")
);
native_copy_test!(
    bundle_admission_native_source_path_literal_password123_refuses,
    "123",
    Duplicate::SourcePath("123")
);
native_copy_test!(
    bundle_admission_native_source_path_hex_password123_refuses,
    "313233",
    Duplicate::SourcePath("313233")
);
native_copy_test!(
    bundle_admission_native_source_path_base64_password123_refuses,
    "MTIz",
    Duplicate::SourcePath("MTIz")
);
native_copy_test!(
    bundle_admission_native_source_path_percent_password123_refuses,
    "%31%32%33",
    Duplicate::SourcePath("%31%32%33")
);
native_copy_test!(
    bundle_admission_native_xpath_literal_password123_refuses,
    "123",
    Duplicate::Xpath("123")
);
native_copy_test!(
    bundle_admission_native_xpath_hex_password123_refuses,
    "313233",
    Duplicate::Xpath("313233")
);
native_copy_test!(
    bundle_admission_native_xpath_base64_password123_refuses,
    "MTIz",
    Duplicate::Xpath("MTIz")
);
native_copy_test!(
    bundle_admission_native_xpath_percent_password123_refuses,
    "%31%32%33",
    Duplicate::Xpath("%31%32%33")
);

fn native_field_case(name: &'static str) {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::Field(name, "123"),
        Duplicate::None,
        Extra::None,
    );
    ordinary_native(&fixture.native);
    assert_bundle_unavailable(native_project_for_bundle(&fixture.native));
    fixture.assert_sources_unchanged();
}

macro_rules! native_field_test {
    ($name:ident, $field:literal) => {
        #[test]
        fn $name() {
            native_field_case($field);
        }
    };
}

native_field_test!(bundle_admission_native_key_refuses, "Key");
native_field_test!(bundle_admission_native_tool_key_refuses, "ToolKey");
native_field_test!(bundle_admission_native_fdsk_refuses, "FDSK");
native_field_test!(
    bundle_admission_native_backbone_key_refuses,
    "IPRoutingBackboneKey"
);
native_field_test!(bundle_admission_native_password_refuses, "Password");
native_field_test!(
    bundle_admission_native_password_hash_refuses,
    "PasswordHash"
);
native_field_test!(
    bundle_admission_native_project_password_refuses,
    "ProjectPassword"
);
native_field_test!(
    bundle_admission_native_authentication_code_refuses,
    "DeviceAuthenticationCode"
);
native_field_test!(
    bundle_admission_native_authentication_code_hash_refuses,
    "DeviceAuthenticationCodeHash"
);
native_field_test!(
    bundle_admission_native_management_password_refuses,
    "DeviceManagementPassword"
);
native_field_test!(
    bundle_admission_native_management_password_hash_refuses,
    "DeviceManagementPasswordHash"
);
native_field_test!(bundle_admission_native_bcu_key_refuses, "BCUKey");
native_field_test!(
    bundle_admission_native_loaded_tool_key_refuses,
    "LoadedToolKey"
);
native_field_test!(
    bundle_admission_native_loaded_backbone_key_refuses,
    "LoadedIPRoutingBackboneKey"
);
native_field_test!(
    bundle_admission_native_loaded_authentication_hash_refuses,
    "LoadedDeviceAuthenticationCodeHash"
);
native_field_test!(
    bundle_admission_native_loaded_management_hash_refuses,
    "LoadedDeviceManagementPasswordHash"
);

fn native_bus_access_case(part: &'static str) {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::BusAccess(part, "123"),
        Duplicate::None,
        Extra::None,
    );
    ordinary_native(&fixture.native);
    assert_bundle_unavailable(native_project_for_bundle(&fixture.native));
    fixture.assert_sources_unchanged();
}

macro_rules! native_bus_access_test {
    ($name:ident, $part:literal) => {
        #[test]
        fn $name() {
            native_bus_access_case($part);
        }
    };
}

native_bus_access_test!(
    bundle_admission_native_bus_access_password_refuses,
    "UserPassword"
);
native_bus_access_test!(bundle_admission_native_bus_access_key_refuses, "TunnelKey");
native_bus_access_test!(
    bundle_admission_native_bus_access_authentication_code_refuses,
    "DeviceAuthenticationCode"
);
native_bus_access_test!(
    bundle_admission_native_bus_access_secret_refuses,
    "SessionSecret"
);

fn assert_native_admitted(fixture: &Produced, dir: &Path) {
    let expected = expected_parts(ordinary_native(&fixture.native));
    let actual = admitted(native_project_for_bundle(&fixture.native));
    assert_eq!(actual, expected);
    let path = dir.join("admitted.knxdb");
    std::fs::write(&path, &actual.0).unwrap();
    let store = knx_store::open_existing_read_only(&path).unwrap();
    knx_store::load_project(&store.conn).unwrap();
    let opaque = knx_store::load_opaque(&store.conn).unwrap();
    assert!(crate::import_evidence::read(&opaque).unwrap().is_some());
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_native_credential_free_project_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(dir.path(), Credential::None, Duplicate::None, Extra::None);
    assert_native_admitted(&fixture, dir.path());
}

#[test]
fn bundle_admission_native_free_access_bcu_key_project_reopens() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::FreeAccessBcuKey,
        Duplicate::None,
        Extra::None,
    );
    assert_native_admitted(&fixture, dir.path());
}

#[test]
fn bundle_admission_native_equal_length_secrets_have_the_same_generic_refusal() {
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
    ordinary_native(&first.native);
    ordinary_native(&second.native);

    assert_bundle_unavailable(native_project_for_bundle(&first.native));
    assert_bundle_unavailable(native_project_for_bundle(&second.native));
    first.assert_sources_unchanged();
    second.assert_sources_unchanged();
}

#[test]
fn bundle_admission_native_nested_child_credential_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::None,
        Duplicate::None,
        Extra::NestedCredential,
    );
    ordinary_native(&fixture.native);
    assert_bundle_unavailable(native_project_for_bundle(&fixture.native));
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_native_discarded_child_credential_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::None,
        Duplicate::None,
        Extra::DiscardedCredential,
    );
    let ordinary = ordinary_native(&fixture.native);
    assert!(ordinary
        .withheld
        .iter()
        .any(|note| note.contains("discarded.xml")));
    assert_bundle_unavailable(native_project_for_bundle(&fixture.native));
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_native_withheld_carried_credential_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::None,
        Duplicate::None,
        Extra::WithheldCarriedCredential,
    );
    let ordinary = ordinary_native(&fixture.native);
    assert!(ordinary
        .withheld
        .iter()
        .any(|note| note.contains("carried product file")));
    assert_bundle_unavailable(native_project_for_bundle(&fixture.native));
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_ordinary_native_withholds_duplicate_of_undecodable_credential_raw_value() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::None,
        Duplicate::None,
        Extra::UndecodableCredentialWithRawDuplicate,
    );

    let ordinary = ordinary_native(&fixture.native);

    assert!(ordinary
        .withheld
        .iter()
        .any(|note| note.contains("synthetic-undecodable-credential.xml")));
    let store = reopened(&ordinary.bytes, dir.path());
    let rows = knx_store::load_opaque(&store.conn).unwrap();
    assert!(!rows.iter().any(|row| row.bytes == b"&undefined;"));
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_native_undecodable_withheld_credential_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::None,
        Duplicate::None,
        Extra::UndecodableCredentialWithRawDuplicate,
    );

    let ordinary = ordinary_native(&fixture.native);
    assert!(ordinary
        .withheld
        .iter()
        .any(|note| note.contains("synthetic-undecodable-credential.xml")));
    assert_bundle_unavailable(native_project_for_bundle(&fixture.native));
    fixture.assert_sources_unchanged();
}

fn synthetic_nonempty_native_value_refuses(extra: Extra) {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(dir.path(), Credential::None, Duplicate::None, extra);

    ordinary_native(&fixture.native);
    assert_bundle_unavailable(native_project_for_bundle(&fixture.native));
    fixture.assert_sources_unchanged();
}

#[test]
fn bundle_admission_native_nonempty_whitespace_attribute_refuses() {
    synthetic_nonempty_native_value_refuses(Extra::WhitespaceAttribute);
}

#[test]
fn bundle_admission_native_nonempty_whitespace_element_refuses() {
    synthetic_nonempty_native_value_refuses(Extra::WhitespaceElement);
}

#[test]
fn bundle_admission_native_nonempty_whitespace_bus_access_component_refuses() {
    synthetic_nonempty_native_value_refuses(Extra::WhitespaceBusAccess);
}

#[test]
fn bundle_admission_native_truly_empty_recognized_values_are_admitted() {
    let dir = tempfile::tempdir().unwrap();
    let fixture = produce(
        dir.path(),
        Credential::None,
        Duplicate::None,
        Extra::EmptyRecognizedValues,
    );

    assert_native_admitted(&fixture, dir.path());
}
