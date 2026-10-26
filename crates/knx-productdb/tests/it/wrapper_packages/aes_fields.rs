//! Contradictory WinZip AES metadata at direct, wrapper and nested product install boundaries.

use super::*;
use flate2::{write::DeflateEncoder, Compression};
use std::collections::BTreeMap;

const AE_STORED: [u8; 11] = [1, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 0, 0];
const AE_DEFLATED: [u8; 11] = [1, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 8, 0];
const FILE: &str = "chiffré/witness.bin";
const DIR: &str = "chiffré/";
const INNER: &str = "second.knxprod";
const PAYLOAD: &[u8] = b"public payload";

const FIELDS: &str = "its local and central WinZip AES fields (0x9901) differ";
const METHOD: &str = "a WinZip AES field (0x9901) requires compression method 99";
const COUNT: &str =
    "compression method 99 (WinZip AES) requires exactly one WinZip AES field (0x9901)";
const BIT0: &str = "compression method 99 (WinZip AES) requires encryption flag (bit 0)";

#[derive(Clone, Copy)]
enum Layer {
    Direct,
    Wrapper,
    Inner,
}

/// One hand-written record; `deflate` stores a DEFLATE stream whatever `method` says.
struct Record<'a> {
    name: &'a str,
    plain: &'a [u8],
    flags: u16,
    method: u16,
    deflate: bool,
    local_extra: &'a [u8],
    central_extra: &'a [u8],
}

fn plain<'a>(name: &'a str, plain: &'a [u8]) -> Record<'a> {
    Record {
        name,
        plain,
        flags: 0,
        method: 0,
        deflate: false,
        local_extra: &[],
        central_extra: &[],
    }
}

fn raw_zip(records: &[Record]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut central = Vec::new();
    for r in records {
        let data = if r.deflate {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(r.plain).unwrap();
            encoder.finish().unwrap()
        } else {
            r.plain.to_vec()
        };
        let offset = bytes.len() as u32;
        let crc = crc32fast::hash(r.plain);
        bytes.extend_from_slice(b"PK\x03\x04");
        for value in [20u16, r.flags | 0x0800, r.method, 0, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [crc, data.len() as u32, r.plain.len() as u32] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [r.name.len() as u16, r.local_extra.len() as u16] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(r.name.as_bytes());
        bytes.extend_from_slice(r.local_extra);
        bytes.extend_from_slice(&data);
        central.extend_from_slice(b"PK\x01\x02");
        for value in [20u16, 20, r.flags | 0x0800, r.method, 0, 0] {
            central.extend_from_slice(&value.to_le_bytes());
        }
        for value in [crc, data.len() as u32, r.plain.len() as u32] {
            central.extend_from_slice(&value.to_le_bytes());
        }
        for value in [r.name.len() as u16, r.central_extra.len() as u16, 0, 0, 0] {
            central.extend_from_slice(&value.to_le_bytes());
        }
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(r.name.as_bytes());
        central.extend_from_slice(r.central_extra);
    }
    let start = bytes.len() as u32;
    bytes.extend_from_slice(&central);
    bytes.extend_from_slice(b"PK\x05\x06");
    for value in [0u16, 0, records.len() as u16, records.len() as u16] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&(central.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&start.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}

fn fixture(layer: Layer, shaped: &dyn Fn() -> Record<'static>) -> Vec<u8> {
    let program = program(0x12);
    let hardware = hardware(0x12);
    let inner = raw_zip(&[
        plain("knx_master.xml", MASTER),
        plain("M-0001/Program.xml", program.as_bytes()),
        plain("M-0001/Hardware.xml", hardware.as_bytes()),
        shaped(),
    ]);
    match layer {
        Layer::Direct => inner,
        Layer::Wrapper => raw_zip(&[plain("first.knxprod", &package(0x11)), shaped()]),
        Layer::Inner => raw_zip(&[plain("first.knxprod", &package(0x11)), plain(INNER, &inner)]),
    }
}

fn snapshot(dir: &std::path::Path) -> BTreeMap<std::path::PathBuf, Vec<u8>> {
    std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().into(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}

fn refused(layer: Layer, shaped: &dyn Fn() -> Record<'static>, reason: &str) {
    let leaf = shaped().name;
    let (dir, conn) = fresh();
    conn.execute_batch("PRAGMA journal_mode=DELETE").unwrap();
    install_package(&conn, "seed.knxprod", &package(0x10)).unwrap();
    let retained = stored_packages(&conn);
    drop(conn);
    let source = fixture(layer, shaped);
    let before = snapshot(dir.path());
    let database = dir.path().join("products.sqlite");
    let conn = Connection::open(&database).unwrap();
    let result = match layer {
        Layer::Direct => install_package(&conn, "test.knxprod", &source).map(|_| ()),
        _ => {
            install_product_file(&conn, "wrapper.zip", &source, PackageLimits::STANDARD).map(|_| ())
        }
    };
    let mut error = match result {
        Err(error) => error,
        Ok(()) => panic!("contradictory AES metadata must refuse before persistence"),
    };
    if matches!(layer, Layer::Inner) {
        error = match error {
            PackageError::WrapperMember { path, error } => {
                assert_eq!(path, INNER);
                *error
            }
            other => panic!("expected named wrapper member refusal, got {other:?}"),
        };
    }
    match error {
        PackageError::InvalidZip { cause } => assert_eq!(cause, format!("{leaf}: {reason}")),
        other => panic!("expected named typed AES consistency refusal, got {other:?}"),
    }
    assert_eq!(stored_packages(&conn), retained);
    drop(conn);
    assert_eq!(
        snapshot(dir.path()),
        before,
        "refusal must preserve database bytes"
    );
    let reopened = open_and_migrate(&database).unwrap();
    assert_eq!(stored_packages(&reopened), retained);
}

fn stored_field() -> Record<'static> {
    Record {
        central_extra: &AE_STORED,
        local_extra: &AE_STORED,
        ..plain(FILE, PAYLOAD)
    }
}

fn deflate_carrier_named_stored() -> Record<'static> {
    Record {
        deflate: true,
        central_extra: &AE_DEFLATED,
        local_extra: &AE_DEFLATED,
        ..plain(FILE, PAYLOAD)
    }
}

fn method99_plain() -> Record<'static> {
    Record {
        method: 99,
        ..stored_field()
    }
}

fn two_fields() -> Record<'static> {
    const TWO: [u8; 22] = {
        let mut both = [0u8; 22];
        let mut i = 0;
        while i < 22 {
            both[i] = AE_STORED[i % 11];
            i += 1;
        }
        both
    };
    Record {
        flags: 1,
        method: 99,
        central_extra: &TWO,
        local_extra: &TWO,
        ..plain(FILE, &[0x5a; 40])
    }
}

fn local_only() -> Record<'static> {
    Record {
        local_extra: &AE_STORED,
        ..plain(FILE, PAYLOAD)
    }
}

fn central_only() -> Record<'static> {
    Record {
        central_extra: &AE_STORED,
        ..plain(FILE, PAYLOAD)
    }
}

fn local_field_differs() -> Record<'static> {
    Record {
        flags: 1,
        method: 99,
        central_extra: &AE_STORED,
        local_extra: &AE_DEFLATED,
        ..plain(FILE, &[0x5a; 40])
    }
}

fn encrypted_field_without_method99() -> Record<'static> {
    Record {
        flags: 1,
        central_extra: &AE_STORED,
        local_extra: &AE_STORED,
        ..plain(FILE, &[0x5a; 40])
    }
}

fn directory_field() -> Record<'static> {
    Record {
        central_extra: &AE_STORED,
        local_extra: &AE_STORED,
        ..plain(DIR, b"")
    }
}

macro_rules! refusals {
    ($($name:ident: $layer:ident, $shape:ident, $reason:ident;)*) => {
        $(#[test] fn $name() { refused(Layer::$layer, &$shape, $reason); })*
    };
}

refusals! {
    direct_stored_field: Direct, stored_field, METHOD;
    wrapper_stored_field: Wrapper, stored_field, METHOD;
    inner_stored_field: Inner, stored_field, METHOD;
    direct_deflate_carrier_named_stored: Direct, deflate_carrier_named_stored, METHOD;
    wrapper_deflate_carrier_named_stored: Wrapper, deflate_carrier_named_stored, METHOD;
    inner_deflate_carrier_named_stored: Inner, deflate_carrier_named_stored, METHOD;
    direct_method99_without_bit0: Direct, method99_plain, BIT0;
    wrapper_method99_without_bit0: Wrapper, method99_plain, BIT0;
    inner_method99_without_bit0: Inner, method99_plain, BIT0;
    direct_two_fields: Direct, two_fields, COUNT;
    wrapper_two_fields: Wrapper, two_fields, COUNT;
    inner_two_fields: Inner, two_fields, COUNT;
    direct_local_only: Direct, local_only, FIELDS;
    wrapper_local_only: Wrapper, local_only, FIELDS;
    inner_local_only: Inner, local_only, FIELDS;
    direct_central_only: Direct, central_only, FIELDS;
    wrapper_central_only: Wrapper, central_only, FIELDS;
    inner_central_only: Inner, central_only, FIELDS;
    direct_local_field_differs: Direct, local_field_differs, FIELDS;
    wrapper_local_field_differs: Wrapper, local_field_differs, FIELDS;
    inner_local_field_differs: Inner, local_field_differs, FIELDS;
    direct_encrypted_field_without_method99: Direct, encrypted_field_without_method99, METHOD;
    wrapper_encrypted_field_without_method99: Wrapper, encrypted_field_without_method99, METHOD;
    inner_encrypted_field_without_method99: Inner, encrypted_field_without_method99, METHOD;
    direct_directory_field: Direct, directory_field, METHOD;
    wrapper_directory_field: Wrapper, directory_field, METHOD;
    inner_directory_field: Inner, directory_field, METHOD;
}

#[test]
fn consistent_aes_keeps_the_encrypted_member_refusal() {
    let (_dir, conn) = fresh();
    let consistent = || Record {
        flags: 1,
        method: 99,
        central_extra: &AE_STORED,
        local_extra: &AE_STORED,
        ..plain(FILE, &[0x5a; 40])
    };
    let error = install_package(&conn, "test.knxprod", &fixture(Layer::Direct, &consistent))
        .expect_err("consistent AES stays unsupported");
    assert!(
        matches!(&error, PackageError::Encrypted { path } if path == FILE),
        "{error:?}"
    );
}

#[test]
fn an_unrelated_unknown_extra_field_stays_accepted() {
    const OTHER: [u8; 11] = [2, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 8, 0];
    let (_dir, conn) = fresh();
    let unrelated = || Record {
        central_extra: &OTHER,
        local_extra: &OTHER,
        ..plain(FILE, PAYLOAD)
    };
    install_package(&conn, "test.knxprod", &fixture(Layer::Direct, &unrelated)).unwrap();
}
