//! Explicit unsupported-feature boundary, not directory decryption or ZIP validity.
//!
//! Original nested short-header acceptance is deliberately superseded; plaintext
//! and ordinary-file controls retain their existing behavior.

use super::ordinary_file_stream_characterization::encrypt;
use super::*;
use knx_testsupport::ZIPCRYPTO_MINIMAL_PASSWORD;
use std::io::{Cursor, Read};

const DIR: &str = "P-0001/Directory/";
const FILE: &str = "P-0001/BinaryData/encrypted.dat";
const CONTENT: &[u8] = b"public fixture payload";
const WRONG_PASSWORD: &str = "intentionally-wrong-public-fixture-password";
const UNSUPPORTED: &str = "encrypted directory records are unsupported";

#[derive(Clone, Copy)]
enum Layer {
    Outer,
    Nested,
}

#[derive(Clone, Copy)]
enum Shape {
    Absent,
    Raw0300,
    OneByte,
    ElevenBytes,
    ArbitraryHeader,
    GenuineStored,
    GenuineDeflate,
    AesEmpty,
    AesMetadata28,
    StrongAbsent,
    StrongRaw0300,
    StrongGenuine,
}

#[derive(Clone, Copy)]
enum Trailer {
    None,
    SignedZero,
    UnsignedZero,
    SignedTruthful,
    UnsignedTruthful,
}

fn descriptor(mut m: Member, trailer: Trailer) -> Member {
    if !matches!(trailer, Trailer::None) {
        m = with_descriptor(
            m,
            matches!(trailer, Trailer::SignedZero | Trailer::SignedTruthful),
        );
        m.descriptor.as_mut().unwrap().keep_local =
            matches!(trailer, Trailer::SignedTruthful | Trailer::UnsignedTruthful);
    }
    m
}

fn shaped(shape: Shape, trailer: Trailer) -> Member {
    let (method, payload): (u16, &[u8]) = match shape {
        Shape::Raw0300 | Shape::GenuineDeflate | Shape::StrongRaw0300 => (8, &[3, 0]),
        Shape::OneByte => (0, &[0x5a]),
        Shape::ElevenBytes => (0, &[0x5a; 11]),
        Shape::ArbitraryHeader => (0, &[0x5a; 12]),
        Shape::AesEmpty => (99, &[]),
        Shape::AesMetadata28 => (99, &[0; 28]),
        _ => (0, &[]),
    };
    let mut m = descriptor(directory(DIR, method, payload, 0), trailer);
    if matches!(
        shape,
        Shape::GenuineStored | Shape::GenuineDeflate | Shape::StrongGenuine
    ) {
        // Reuse the existing independent, deterministic fixture cipher rather
        // than duplicating a cipher implementation for directory fixtures.
        encrypt(&mut m);
        independently_decodes(&m, b"");
    }
    m.flags |= 1;
    if matches!(
        shape,
        Shape::StrongAbsent | Shape::StrongRaw0300 | Shape::StrongGenuine
    ) {
        m.flags |= 64;
    }
    if matches!(shape, Shape::AesEmpty | Shape::AesMetadata28) {
        // Well-formed AE-2/vendor AE/strength 3/underlying Stored metadata,
        // not authenticated AES ciphertext and not a decryption claim.
        m.extra = vec![1, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 0, 0];
    }
    m
}

fn independently_decodes(m: &Member, expected: &[u8]) {
    let source = zip(std::slice::from_ref(m));
    let mut archive = zip::ZipArchive::new(Cursor::new(source)).unwrap();
    let mut entry = archive
        .by_index_decrypt(0, ZIPCRYPTO_MINIMAL_PASSWORD.as_bytes())
        .unwrap();
    assert!(entry.encrypted());
    assert_eq!(entry.size(), expected.len() as u64);
    let mut plain = Vec::new();
    entry.read_to_end(&mut plain).unwrap();
    assert_eq!(plain, expected, "independent zip cipher/codec control");
}

fn archive(layer: Layer, m: Member) -> Vec<u8> {
    match layer {
        Layer::Outer => project_with(vec![m]),
        Layer::Nested => protected_with(vec![m]),
    }
}

fn open(layer: Layer, source: Vec<u8>, password: &str) -> Result<Container, ContainerError> {
    match layer {
        Layer::Outer => Container::open(source),
        Layer::Nested => Container::open_with_password(source, password),
    }
}

fn error(result: Result<Container, ContainerError>) -> ContainerError {
    match result {
        Err(error) => error,
        Ok(_) => panic!("expected directory admission refusal"),
    }
}

fn refused_member(layer: Layer, m: Member, path: &str, reason: &'static str, password: &str) {
    assert_eq!(
        error(open(layer, archive(layer, m), password)),
        ContainerError::InconsistentRecord {
            path: path.into(),
            reason,
        }
    );
}

fn refused(layer: Layer, shape: Shape, trailer: Trailer) {
    refused_member(
        layer,
        shaped(shape, trailer),
        DIR,
        UNSUPPORTED,
        ZIPCRYPTO_MINIMAL_PASSWORD,
    );
}

// Separate registered names are intentional: an original-source failure in one
// shape must not prevent execution of another descriptor, cipher or layer case.
macro_rules! refusal_cases {
    ($($name:ident: $layer:ident, $shape:ident, $trailer:ident;)*) => {
        $(#[test]
        fn $name() {
            refused(Layer::$layer, Shape::$shape, Trailer::$trailer);
        })*
    };
}

refusal_cases! {
    outer_absent_header: Outer, Absent, None;
    nested_absent_header: Nested, Absent, None;
    outer_absent_header_signed_zero: Outer, Absent, SignedZero;
    nested_absent_header_signed_zero: Nested, Absent, SignedZero;
    outer_absent_header_unsigned_zero: Outer, Absent, UnsignedZero;
    nested_absent_header_unsigned_zero: Nested, Absent, UnsignedZero;
    outer_absent_header_signed_truthful: Outer, Absent, SignedTruthful;
    nested_absent_header_signed_truthful: Nested, Absent, SignedTruthful;
    outer_absent_header_unsigned_truthful: Outer, Absent, UnsignedTruthful;
    nested_absent_header_unsigned_truthful: Nested, Absent, UnsignedTruthful;
    outer_raw0300_short_header: Outer, Raw0300, None;
    nested_raw0300_short_header: Nested, Raw0300, None;
    outer_raw0300_signed_zero: Outer, Raw0300, SignedZero;
    nested_raw0300_signed_zero: Nested, Raw0300, SignedZero;
    outer_raw0300_unsigned_zero: Outer, Raw0300, UnsignedZero;
    nested_raw0300_unsigned_zero: Nested, Raw0300, UnsignedZero;
    outer_raw0300_signed_truthful: Outer, Raw0300, SignedTruthful;
    nested_raw0300_signed_truthful: Nested, Raw0300, SignedTruthful;
    outer_raw0300_unsigned_truthful: Outer, Raw0300, UnsignedTruthful;
    nested_raw0300_unsigned_truthful: Nested, Raw0300, UnsignedTruthful;
    outer_genuine_stored_empty: Outer, GenuineStored, None;
    nested_genuine_stored_empty: Nested, GenuineStored, None;
    outer_genuine_stored_signed_zero: Outer, GenuineStored, SignedZero;
    nested_genuine_stored_signed_zero: Nested, GenuineStored, SignedZero;
    outer_genuine_stored_unsigned_zero: Outer, GenuineStored, UnsignedZero;
    nested_genuine_stored_unsigned_zero: Nested, GenuineStored, UnsignedZero;
    outer_genuine_stored_signed_truthful: Outer, GenuineStored, SignedTruthful;
    nested_genuine_stored_signed_truthful: Nested, GenuineStored, SignedTruthful;
    outer_genuine_stored_unsigned_truthful: Outer, GenuineStored, UnsignedTruthful;
    nested_genuine_stored_unsigned_truthful: Nested, GenuineStored, UnsignedTruthful;
    outer_genuine_deflate_empty: Outer, GenuineDeflate, None;
    nested_genuine_deflate_empty: Nested, GenuineDeflate, None;
    outer_genuine_deflate_signed_zero: Outer, GenuineDeflate, SignedZero;
    nested_genuine_deflate_signed_zero: Nested, GenuineDeflate, SignedZero;
    outer_genuine_deflate_unsigned_zero: Outer, GenuineDeflate, UnsignedZero;
    nested_genuine_deflate_unsigned_zero: Nested, GenuineDeflate, UnsignedZero;
    outer_genuine_deflate_signed_truthful: Outer, GenuineDeflate, SignedTruthful;
    nested_genuine_deflate_signed_truthful: Nested, GenuineDeflate, SignedTruthful;
    outer_genuine_deflate_unsigned_truthful: Outer, GenuineDeflate, UnsignedTruthful;
    nested_genuine_deflate_unsigned_truthful: Nested, GenuineDeflate, UnsignedTruthful;
    outer_one_byte_header: Outer, OneByte, None;
    nested_one_byte_header: Nested, OneByte, None;
    outer_eleven_byte_header: Outer, ElevenBytes, None;
    nested_eleven_byte_header: Nested, ElevenBytes, None;
    outer_arbitrary_twelve_byte_header: Outer, ArbitraryHeader, None;
    nested_arbitrary_twelve_byte_header: Nested, ArbitraryHeader, None;
    outer_aes_metadata_empty: Outer, AesEmpty, None;
    nested_aes_metadata_empty: Nested, AesEmpty, None;
    outer_aes_metadata_28bytes: Outer, AesMetadata28, None;
    nested_aes_metadata_28bytes: Nested, AesMetadata28, None;
    outer_bits0_and6_absent_header: Outer, StrongAbsent, None;
    nested_bits0_and6_absent_header: Nested, StrongAbsent, None;
    outer_bits0_and6_raw0300: Outer, StrongRaw0300, None;
    nested_bits0_and6_raw0300: Nested, StrongRaw0300, None;
    outer_bits0_and6_genuine_stored: Outer, StrongGenuine, None;
    nested_bits0_and6_genuine_stored: Nested, StrongGenuine, None;
}

macro_rules! wrong_password_cases {
    ($($name:ident: $layer:ident, $shape:ident;)*) => {
        $(#[test]
        fn $name() {
            assert_eq!(
                error(Container::open_with_password(
                    archive(Layer::$layer, shaped(Shape::$shape, Trailer::None)),
                    WRONG_PASSWORD,
                )),
                ContainerError::InconsistentRecord { path: DIR.into(), reason: UNSUPPORTED },
            );
        })*
    };
}

wrong_password_cases! {
    outer_absent_header_wrong_fixture_password: Outer, Absent;
    nested_absent_header_wrong_fixture_password: Nested, Absent;
    outer_raw0300_wrong_fixture_password: Outer, Raw0300;
    nested_raw0300_wrong_fixture_password: Nested, Raw0300;
}

fn decoded_directory(layer: Layer, unicode: bool, backslash: bool) {
    let mut m = shaped(Shape::Absent, Trailer::None);
    let path = if backslash {
        "P-0001/é\\"
    } else {
        "P-0001/é/"
    };
    if unicode {
        // Raw name is not a directory: the validated decoded name decides.
        m.name = b"P-0001/raw.bin".to_vec();
        m.extra = unicode_path(&m.name, path);
    } else {
        m.name = if backslash {
            b"P-0001/\x82\\"
        } else {
            b"P-0001/\x82/"
        }
        .to_vec();
    }
    refused_member(layer, m, path, UNSUPPORTED, ZIPCRYPTO_MINIMAL_PASSWORD);
}

macro_rules! decoded_cases {
    ($($name:ident: $layer:ident, $unicode:literal, $backslash:literal;)*) => {
        $(#[test]
        fn $name() { decoded_directory(Layer::$layer, $unicode, $backslash); })*
    };
}

decoded_cases! {
    outer_unicode_slash_directory: Outer, true, false;
    nested_unicode_slash_directory: Nested, true, false;
    outer_unicode_backslash_directory: Outer, true, true;
    nested_unicode_backslash_directory: Nested, true, true;
    outer_cp437_slash_directory: Outer, false, false;
    nested_cp437_slash_directory: Nested, false, false;
    outer_cp437_backslash_directory: Outer, false, true;
    nested_cp437_backslash_directory: Nested, false, true;
}

fn accepted(layer: Layer, m: Member) {
    let mut c = open(layer, archive(layer, m), ZIPCRYPTO_MINIMAL_PASSWORD).unwrap();
    assert_eq!(c.read("P-0001/0.xml").unwrap(), INSTALLATION);
    assert_eq!(c.read("P-0001/Project.xml").unwrap(), PROJECT_INFO);
    assert_eq!(c.entries().len(), 3);
    assert!(c.find(DIR).is_none());
    assert!(c.entries().iter().all(|e| !e.path.ends_with(['/', '\\'])));
}

macro_rules! plaintext_cases {
    ($($name:ident: $layer:ident, $method:literal, $payload:expr, $trailer:ident, $flags:literal;)*) => {
        $(#[test]
        fn $name() {
            let mut m = descriptor(directory(DIR, $method, $payload, 0), Trailer::$trailer);
            m.flags |= $flags;
            accepted(Layer::$layer, m);
        })*
    };
}

plaintext_cases! {
    outer_plain_stored: Outer, 0, b"", None, 0;
    nested_plain_stored: Nested, 0, b"", None, 0;
    outer_plain_deflate: Outer, 8, &[3, 0], None, 0;
    nested_plain_deflate: Nested, 8, &[3, 0], None, 0;
    outer_plain_alternate_deflate: Outer, 8, &[1, 0, 0, 255, 255], None, 0;
    nested_plain_alternate_deflate: Nested, 8, &[1, 0, 0, 255, 255], None, 0;
    outer_plain_stored_signed_zero: Outer, 0, b"", SignedZero, 0;
    nested_plain_stored_signed_zero: Nested, 0, b"", SignedZero, 0;
    outer_plain_stored_unsigned_zero: Outer, 0, b"", UnsignedZero, 0;
    nested_plain_stored_unsigned_zero: Nested, 0, b"", UnsignedZero, 0;
    outer_plain_stored_signed_truthful: Outer, 0, b"", SignedTruthful, 0;
    nested_plain_stored_signed_truthful: Nested, 0, b"", SignedTruthful, 0;
    outer_plain_stored_unsigned_truthful: Outer, 0, b"", UnsignedTruthful, 0;
    nested_plain_stored_unsigned_truthful: Nested, 0, b"", UnsignedTruthful, 0;
    outer_plain_deflate_signed_zero: Outer, 8, &[3, 0], SignedZero, 0;
    nested_plain_deflate_signed_zero: Nested, 8, &[3, 0], SignedZero, 0;
    outer_plain_deflate_unsigned_zero: Outer, 8, &[3, 0], UnsignedZero, 0;
    nested_plain_deflate_unsigned_zero: Nested, 8, &[3, 0], UnsignedZero, 0;
    outer_plain_deflate_signed_truthful: Outer, 8, &[3, 0], SignedTruthful, 0;
    nested_plain_deflate_signed_truthful: Nested, 8, &[3, 0], SignedTruthful, 0;
    outer_plain_deflate_unsigned_truthful: Outer, 8, &[3, 0], UnsignedTruthful, 0;
    nested_plain_deflate_unsigned_truthful: Nested, 8, &[3, 0], UnsignedTruthful, 0;
    outer_bit6_only_stored: Outer, 0, b"", None, 64;
    nested_bit6_only_stored: Nested, 0, b"", None, 64;
    outer_bit6_only_deflate: Outer, 8, &[3, 0], None, 64;
    nested_bit6_only_deflate: Nested, 8, &[3, 0], None, 64;
}

macro_rules! plain_refusal_cases {
    ($($name:ident: $layer:ident, $method:literal, $payload:expr, $crc:literal, $reason:literal;)*) => {
        $(#[test]
        fn $name() {
            refused_member(Layer::$layer, directory(DIR, $method, $payload, $crc), DIR,
                $reason, ZIPCRYPTO_MINIMAL_PASSWORD);
        })*
    };
}

plain_refusal_cases! {
    outer_plain_stored_data_is_unchanged: Outer, 0, &[0x5a], 0, "a directory record carries data";
    nested_plain_stored_data_is_unchanged: Nested, 0, &[0x5a], 0, "a directory record carries data";
    outer_plain_deflate_truncated_is_unchanged: Outer, 8, &[3], 0, "a compressed directory record holds no complete compressed stream";
    nested_plain_deflate_truncated_is_unchanged: Nested, 8, &[3], 0, "a compressed directory record holds no complete compressed stream";
    outer_plain_crc_is_unchanged: Outer, 0, b"", 1, "an empty directory record declares a CRC-32 other than 0, the CRC-32 of no content";
    nested_plain_crc_is_unchanged: Nested, 0, b"", 1, "an empty directory record declares a CRC-32 other than 0, the CRC-32 of no content";
}

#[derive(Clone, Copy)]
enum Defect {
    Name,
    Flags,
    Bounds,
    Descriptor,
    Layout,
}

fn structural_precedence(layer: Layer, defect: Defect) {
    let mut m = shaped(Shape::Absent, Trailer::None);
    let reason = match defect {
        Defect::Name => {
            m.local_name = Some(b"P-0001/Different/".to_vec());
            "its local header names a different member"
        }
        Defect::Flags => {
            m.local = |f| f.flags ^= 1;
            "its local header disagrees with the central record (flags/method)"
        }
        Defect::Bounds => {
            m.local = |f| f.compressed = 0x00ff_0000;
            m.central = m.local;
            "its local header is missing, truncated or malformed"
        }
        Defect::Descriptor => {
            m = descriptor(m, Trailer::SignedZero);
            m.descriptor.as_mut().unwrap().edit = |f| f.crc = 1;
            "its data descriptor disagrees with the central record (sizes/CRC)"
        }
        Defect::Layout => {
            m.after = vec![0xa5];
            "its local record does not end where the next record begins"
        }
    };
    refused_member(layer, m, DIR, reason, ZIPCRYPTO_MINIMAL_PASSWORD);
}

macro_rules! precedence_cases {
    ($($name:ident: $layer:ident, $defect:ident;)*) => {
        $(#[test]
        fn $name() { structural_precedence(Layer::$layer, Defect::$defect); })*
    };
}

precedence_cases! {
    outer_local_name_precedes_encryption: Outer, Name;
    nested_local_name_precedes_encryption: Nested, Name;
    outer_local_flags_precede_encryption: Outer, Flags;
    nested_local_flags_precede_encryption: Nested, Flags;
    outer_bounds_precede_encryption: Outer, Bounds;
    nested_bounds_precede_encryption: Nested, Bounds;
    outer_descriptor_precedes_encryption: Outer, Descriptor;
    nested_descriptor_precedes_encryption: Nested, Descriptor;
    outer_full_layout_precedes_encryption: Outer, Layout;
    nested_full_layout_precedes_encryption: Nested, Layout;
}

macro_rules! payload_precedence_cases {
    ($($name:ident: $layer:ident, $size:literal, $crc:literal;)*) => {
        $(#[test]
        fn $name() {
            let mut m = shaped(Shape::Absent, Trailer::None);
            m.declared = $size;
            m.crc = Some($crc);
            refused_member(Layer::$layer, m, DIR, UNSUPPORTED, ZIPCRYPTO_MINIMAL_PASSWORD);
        })*
    };
}

payload_precedence_cases! {
    outer_encryption_precedes_declared_content: Outer, 1, 0;
    nested_encryption_precedes_declared_content: Nested, 1, 0;
    outer_encryption_precedes_crc: Outer, 0, 1;
    nested_encryption_precedes_crc: Nested, 0, 1;
}

fn ordinary(method: u16) -> Member {
    let mut m = if method == 8 {
        deflated(FILE, CONTENT)
    } else {
        Member {
            crc: Some(crc32(CONTENT)),
            ..member(FILE, CONTENT)
        }
    };
    encrypt(&mut m);
    independently_decodes(&m, CONTENT);
    m
}

#[test]
fn ordinary_stored_correct_password_is_unchanged() {
    let mut c = Container::open_with_password(
        protected_with(vec![ordinary(0)]),
        ZIPCRYPTO_MINIMAL_PASSWORD,
    )
    .unwrap();
    assert_eq!(c.read(FILE).unwrap(), CONTENT);
}

#[test]
fn ordinary_deflate_correct_password_is_unchanged() {
    let mut c = Container::open_with_password(
        protected_with(vec![ordinary(8)]),
        ZIPCRYPTO_MINIMAL_PASSWORD,
    )
    .unwrap();
    assert_eq!(c.read(FILE).unwrap(), CONTENT);
}

#[test]
fn ordinary_stored_wrong_password_is_unchanged() {
    assert_eq!(
        error(Container::open_with_password(
            protected_with(vec![ordinary(0)]),
            WRONG_PASSWORD
        )),
        ContainerError::WrongPassword {
            nested_entry: "P-0001.zip".into()
        }
    );
}

#[test]
fn ordinary_deflate_wrong_password_is_unchanged() {
    assert_eq!(
        error(Container::open_with_password(
            protected_with(vec![ordinary(8)]),
            WRONG_PASSWORD
        )),
        ContainerError::WrongPassword {
            nested_entry: "P-0001.zip".into()
        }
    );
}

#[test]
fn ordinary_missing_password_is_unchanged() {
    assert_eq!(
        error(Container::open(protected_with(vec![ordinary(0)]))),
        ContainerError::PasswordProtected {
            nested_entry: "P-0001.zip".into()
        }
    );
}

#[test]
fn ordinary_outer_encryption_is_still_password_required() {
    match error(Container::open(project_with(vec![ordinary(0)]))) {
        ContainerError::NotAZip(cause) => assert!(cause.contains("Password required"), "{cause}"),
        other => panic!("unexpected outer file error: {other:?}"),
    }
}

#[test]
fn ordinary_nested_aes_retains_wrapper_unsupported_encryption() {
    use knx_etsproj::container::EncryptionScheme;
    let mut m = shaped(Shape::AesMetadata28, Trailer::None);
    m.name = FILE.as_bytes().to_vec();
    assert_eq!(
        error(Container::open_with_password(
            protected_with(vec![m]),
            ZIPCRYPTO_MINIMAL_PASSWORD
        )),
        ContainerError::UnsupportedEncryption {
            nested_entry: "P-0001.zip".into(),
            scheme: EncryptionScheme::Aes
        }
    );
}
