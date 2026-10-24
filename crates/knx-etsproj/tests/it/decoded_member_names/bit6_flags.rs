//! Contradictory bit-6-only flags refuse before project payload interpretation.

use super::*;
use knx_testsupport::ZIPCRYPTO_MINIMAL_PASSWORD;

const REASON: &str = "strong-encryption flag (bit 6) requires encryption flag (bit 0)";
const PATH: &str = "P-0001/BinaryData/répertoire.bin";

fn refused(mut m: Member, nested: bool, expected: &str) {
    m.flags |= 64;
    let source = if nested {
        protected_with(vec![m])
    } else {
        project_with(vec![m])
    };
    let result = if nested {
        Container::open_with_password(source, ZIPCRYPTO_MINIMAL_PASSWORD)
    } else {
        Container::open(source)
    };
    let error = match result {
        Err(error) => error,
        Ok(_) => panic!("bit-6-only entry must refuse at container admission"),
    };
    assert_eq!(
        error,
        ContainerError::InconsistentRecord {
            path: expected.into(),
            reason: REASON
        }
    );
}

macro_rules! cases {
    ($($name:ident: $nested:literal, $deflated:literal, $content:expr;)*) => {
        $(#[test]
        fn $name() {
            let mut m = if $deflated { deflated(PATH, $content) } else { member(PATH, $content) };
            m.flags |= UTF8_FLAG;
            refused(m, $nested, PATH);
        })*
    };
}

cases! {
    outer_stored_file: false, false, b"public nonempty payload";
    nested_stored_file: true, false, b"public nonempty payload";
    outer_deflated_file: false, true, b"public nonempty payload";
    nested_deflated_file: true, true, b"public nonempty payload";
    outer_empty_stored_file: false, false, b"";
    nested_empty_stored_file: true, false, b"";
    outer_empty_deflated_file: false, true, b"";
    nested_empty_deflated_file: true, true, b"";
}

#[test]
fn outer_unicode_override_leaf() {
    let mut m = member("raw.bin", b"payload");
    m.extra = unicode_path(&m.name, PATH);
    refused(m, false, PATH);
}

#[test]
fn nested_cp437_leaf() {
    let mut m = member("raw.bin", b"payload");
    m.name = b"P-0001/BinaryData/r\x82pertoire.bin".to_vec();
    refused(m, true, PATH);
}

#[test]
fn outer_unicode_override_directory() {
    let mut m = member("raw", b"");
    m.extra = unicode_path(&m.name, "répertoire/");
    refused(m, false, "répertoire/");
}

#[test]
fn nested_cp437_directory() {
    let mut m = member("raw", b"");
    m.name = b"r\x82pertoire/".to_vec();
    refused(m, true, "répertoire/");
}
