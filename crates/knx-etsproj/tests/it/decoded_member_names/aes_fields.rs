//! Contradictory WinZip AES metadata refuses by name in outer and nested project layers.

use super::*;
use knx_testsupport::ZIPCRYPTO_MINIMAL_PASSWORD;

/// AE-2, vendor `AE`, strength 3, underlying method Stored.
const AE_STORED: [u8; 11] = [1, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 0, 0];
/// The same, naming DEFLATE as the underlying method.
const AE_DEFLATED: [u8; 11] = [1, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 8, 0];
const PATH: &str = "P-0001/BinaryData/chiffré.bin";
const DIR: &str = "P-0001/Répertoire/";
const CONTENT: &[u8] = b"public nonempty payload";

const FIELDS: &str = "its local and central WinZip AES fields (0x9901) differ";
const METHOD: &str = "a WinZip AES field (0x9901) requires compression method 99";
const COUNT: &str =
    "compression method 99 (WinZip AES) requires exactly one WinZip AES field (0x9901)";
const BIT0: &str = "compression method 99 (WinZip AES) requires encryption flag (bit 0)";

fn open(nested: bool, m: Member) -> Result<Container, ContainerError> {
    if nested {
        Container::open_with_password(protected_with(vec![m]), ZIPCRYPTO_MINIMAL_PASSWORD)
    } else {
        Container::open(project_with(vec![m]))
    }
}

fn refused(nested: bool, m: Member, path: &str, reason: &'static str) {
    match open(nested, m) {
        Err(error) => assert_eq!(
            error,
            ContainerError::InconsistentRecord {
                path: path.into(),
                reason
            }
        ),
        Ok(_) => panic!("contradictory AES metadata must refuse at container admission"),
    }
}

/// `name` is non-ASCII, so every fixture flags its name as UTF-8.
fn utf8(name: &str, data: &[u8]) -> Member {
    let mut m = member(name, data);
    m.flags = UTF8_FLAG;
    m
}

fn stored_with(extra: &[u8]) -> Member {
    let mut m = utf8(PATH, CONTENT);
    m.extra = extra.to_vec();
    m
}

fn deflate_carrier_named_stored() -> Member {
    // A real DEFLATE stream behind raw method 0: `zip` follows the field and
    // inflates it, a reader following the raw method sees the stream bytes.
    let mut m = deflated(PATH, CONTENT);
    m.flags = UTF8_FLAG;
    m.method = 0;
    m.extra = AE_DEFLATED.to_vec();
    m
}

fn method99_plain() -> Member {
    let mut m = stored_with(&AE_STORED);
    m.method = 99;
    m
}

fn local_only() -> Member {
    let mut m = utf8(PATH, CONTENT);
    m.local = |f| f.extra = AE_STORED.to_vec();
    m
}

fn central_only() -> Member {
    let mut m = utf8(PATH, CONTENT);
    m.central = |f| f.extra = AE_STORED.to_vec();
    m
}

fn local_field_differs() -> Member {
    let mut m = encrypted_aes(&AE_STORED);
    m.local = |f| f.extra = AE_DEFLATED.to_vec();
    m
}

fn encrypted_aes(extra: &[u8]) -> Member {
    // Opaque bytes, not AES ciphertext: admission must refuse before any codec.
    let mut m = utf8(PATH, &[0x5a; 40]);
    m.flags |= 1;
    m.method = 99;
    m.extra = extra.to_vec();
    m
}

fn encrypted_field_without_method99() -> Member {
    let mut m = encrypted_aes(&AE_STORED);
    m.method = 0;
    m
}

fn deflated_with_field() -> Member {
    let mut m = deflated(PATH, CONTENT);
    m.flags = UTF8_FLAG;
    m.extra = AE_DEFLATED.to_vec();
    m
}

fn plaintext_directory_with_field() -> Member {
    let mut m = utf8(DIR, b"");
    m.extra = AE_STORED.to_vec();
    m
}

macro_rules! refusals {
    ($($name:ident: $nested:literal, $build:expr, $path:expr, $reason:expr;)*) => {
        $(#[test]
        fn $name() {
            refused($nested, $build, $path, $reason);
        })*
    };
}

refusals! {
    outer_stored_member_with_deflate_field: false, deflate_carrier_named_stored(), PATH, METHOD;
    nested_stored_member_with_deflate_field: true, deflate_carrier_named_stored(), PATH, METHOD;
    outer_stored_member_with_stored_field: false, stored_with(&AE_STORED), PATH, METHOD;
    nested_stored_member_with_stored_field: true, stored_with(&AE_STORED), PATH, METHOD;
    outer_deflated_member_with_field: false, deflated_with_field(), PATH, METHOD;
    nested_deflated_member_with_field: true, deflated_with_field(), PATH, METHOD;
    outer_encrypted_field_without_method99: false, encrypted_field_without_method99(), PATH, METHOD;
    nested_encrypted_field_without_method99: true, encrypted_field_without_method99(), PATH, METHOD;
    outer_method99_without_bit0: false, method99_plain(), PATH, BIT0;
    nested_method99_without_bit0: true, method99_plain(), PATH, BIT0;
    outer_two_aes_fields: false, encrypted_aes(&[AE_STORED, AE_STORED].concat()), PATH, COUNT;
    nested_two_aes_fields: true, encrypted_aes(&[AE_STORED, AE_STORED].concat()), PATH, COUNT;
    outer_field_in_local_header_only: false, local_only(), PATH, FIELDS;
    nested_field_in_local_header_only: true, local_only(), PATH, FIELDS;
    outer_field_in_central_record_only: false, central_only(), PATH, FIELDS;
    nested_field_in_central_record_only: true, central_only(), PATH, FIELDS;
    outer_local_field_differs: false, local_field_differs(), PATH, FIELDS;
    nested_local_field_differs: true, local_field_differs(), PATH, FIELDS;
    outer_plaintext_directory_with_field: false, plaintext_directory_with_field(), DIR, METHOD;
    nested_plaintext_directory_with_field: true, plaintext_directory_with_field(), DIR, METHOD;
}

#[test]
fn outer_unicode_override_names_the_decoded_leaf() {
    let mut m = member("raw.bin", CONTENT);
    m.extra = [unicode_path(b"raw.bin", PATH), AE_STORED.to_vec()].concat();
    refused(false, m, PATH, METHOD);
}

#[test]
fn nested_cp437_name_names_the_decoded_leaf() {
    let mut m = member("raw.bin", CONTENT);
    m.name = b"P-0001/BinaryData/chiffr\x82.bin".to_vec();
    m.extra = AE_STORED.to_vec();
    match open(true, m) {
        Err(error) => assert_eq!(
            error,
            ContainerError::InconsistentRecord {
                path: PATH.into(),
                reason: METHOD
            }
        ),
        Ok(_) => panic!("contradictory AES metadata must refuse at container admission"),
    }
}

#[test]
fn consistent_aes_keeps_the_unsupported_encryption_refusal() {
    assert_eq!(
        open(true, encrypted_aes(&AE_STORED)).err(),
        Some(ContainerError::UnsupportedEncryption {
            nested_entry: "P-0001.zip".into(),
            scheme: knx_etsproj::EncryptionScheme::Aes,
        })
    );
    let outer = open(false, encrypted_aes(&AE_STORED)).err();
    assert!(
        matches!(&outer, Some(ContainerError::NotAZip(cause)) if cause.contains("Password required")),
        "outer consistent AES keeps its password-required refusal: {outer:?}"
    );
}

#[test]
fn method99_without_any_field_keeps_the_reader_refusal() {
    let mut m = encrypted_aes(&[]);
    m.extra.clear();
    let cause = "invalid Zip archive: AES encryption without AES extra data field";
    let outer = open(false, m).err();
    assert_eq!(outer, Some(ContainerError::NotAZip(cause.into())));
    let mut m = encrypted_aes(&[]);
    m.extra.clear();
    assert_eq!(
        open(true, m).err(),
        Some(ContainerError::Read {
            path: "P-0001.zip".into(),
            cause: cause.into()
        })
    );
}

#[test]
fn an_unrelated_unknown_extra_field_stays_accepted() {
    // 0x9902 is listed by APPNOTE 4.6.1 as "unknown"; only 0x9901 is AES.
    let field = [2, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 8, 0];
    for nested in [false, true] {
        let mut container = open(nested, stored_with(&field)).unwrap();
        assert_eq!(container.read(PATH).unwrap(), CONTENT);
    }
}
