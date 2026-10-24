//! Bit-6 flag consistency at direct, wrapper and nested product install boundaries.

use super::*;
use flate2::{write::DeflateEncoder, Compression};
use std::collections::BTreeMap;

const REASON: &str = "strong-encryption flag (bit 6) requires encryption flag (bit 0)";
const FILE: &str = "répertoire/witness.bin";
const DIR: &str = "répertoire/";
const INNER: &str = "second.knxprod";

#[derive(Clone, Copy)]
enum Layer {
    Direct,
    Wrapper,
    Inner,
}

// Hand-written records keep methods, flags and directory payloads independent.
fn raw_zip(members: &[(&str, &[u8], u16, u16)]) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut central = Vec::new();
    for &(name, plain, flags, method) in members {
        let data = if method == 8 {
            let mut encoder = DeflateEncoder::new(Vec::new(), Compression::default());
            encoder.write_all(plain).unwrap();
            encoder.finish().unwrap()
        } else {
            plain.to_vec()
        };
        let offset = bytes.len() as u32;
        let crc = crc32fast::hash(plain);
        bytes.extend_from_slice(b"PK\x03\x04");
        for value in [20u16, flags | 0x0800, method, 0, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [crc, data.len() as u32, plain.len() as u32] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        for value in [name.len() as u16, 0] {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes.extend_from_slice(name.as_bytes());
        bytes.extend_from_slice(&data);
        central.extend_from_slice(b"PK\x01\x02");
        for value in [20u16, 20, flags | 0x0800, method, 0, 0] {
            central.extend_from_slice(&value.to_le_bytes());
        }
        for value in [crc, data.len() as u32, plain.len() as u32] {
            central.extend_from_slice(&value.to_le_bytes());
        }
        for value in [name.len() as u16, 0, 0, 0, 0] {
            central.extend_from_slice(&value.to_le_bytes());
        }
        central.extend_from_slice(&0u32.to_le_bytes());
        central.extend_from_slice(&offset.to_le_bytes());
        central.extend_from_slice(name.as_bytes());
    }
    let start = bytes.len() as u32;
    bytes.extend_from_slice(&central);
    bytes.extend_from_slice(b"PK\x05\x06");
    for value in [0u16, 0, members.len() as u16, members.len() as u16] {
        bytes.extend_from_slice(&value.to_le_bytes());
    }
    bytes.extend_from_slice(&(central.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&start.to_le_bytes());
    bytes.extend_from_slice(&0u16.to_le_bytes());
    bytes
}

fn fixture(layer: Layer, directory: bool, method: u16, flags: u16) -> Vec<u8> {
    let leaf = if directory { DIR } else { FILE };
    let payload: &[u8] = if directory { b"" } else { b"public payload" };
    let program = program(0x12);
    let hardware = hardware(0x12);
    let inner = raw_zip(&[
        ("knx_master.xml", MASTER, 0, 0),
        ("M-0001/Program.xml", program.as_bytes(), 0, 0),
        ("M-0001/Hardware.xml", hardware.as_bytes(), 0, 0),
        (leaf, payload, flags, method),
    ]);
    match layer {
        Layer::Direct => inner,
        Layer::Wrapper => raw_zip(&[
            ("first.knxprod", &package(0x11), 0, 0),
            (leaf, payload, flags, method),
        ]),
        Layer::Inner => raw_zip(&[
            ("first.knxprod", &package(0x11), 0, 0),
            (INNER, &inner, 0, 0),
        ]),
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

fn case(layer: Layer, directory: bool, method: u16, flags: u16) {
    let (dir, conn) = fresh();
    conn.execute_batch("PRAGMA journal_mode=DELETE").unwrap();
    let seed = package(0x10);
    install_package(&conn, "seed.knxprod", &seed).unwrap();
    let retained = stored_packages(&conn);
    drop(conn);
    let source = fixture(layer, directory, method, flags);
    let source_path = dir.path().join("source.zip");
    std::fs::write(&source_path, &source).unwrap();
    let before = snapshot(dir.path());
    let database = dir.path().join("products.sqlite");
    let conn = Connection::open(&database).unwrap();
    let result = match layer {
        Layer::Direct => install_package(&conn, "test.knxprod", &source).map(|_| ()),
        _ => {
            install_product_file(&conn, "wrapper.zip", &source, PackageLimits::STANDARD).map(|_| ())
        }
    };
    if flags == 0 {
        result.unwrap();
        assert!(stored_packages(&conn).len() > retained.len());
    } else {
        let mut error = match result {
            Err(error) => error,
            Ok(()) => panic!("bit-6-only product entry must refuse before persistence"),
        };
        if matches!(layer, Layer::Inner) {
            error = match error {
                PackageError::WrapperMember { path, error } => {
                    assert_eq!(path, INNER);
                    *error
                }
                _ => panic!("expected named wrapper member refusal"),
            };
        }
        let leaf = if directory { DIR } else { FILE };
        if flags == 64 {
            match error {
                PackageError::InvalidZip { cause } => {
                    assert_eq!(cause, format!("{leaf}: {REASON}"))
                }
                _ => panic!("expected named typed flag consistency refusal"),
            }
        } else {
            assert!(matches!(error, PackageError::Encrypted { path } if path == leaf));
        }
        assert_eq!(stored_packages(&conn), retained);
    }
    drop(conn);
    let after = snapshot(dir.path());
    if flags != 0 {
        assert_eq!(
            after, before,
            "refusal must preserve database and sidecar bytes/absence"
        );
    }
    assert_eq!(std::fs::read(source_path).unwrap(), source);
    let reopened = open_and_migrate(&database).unwrap();
    if flags != 0 {
        assert_eq!(stored_packages(&reopened), retained);
    }
    drop(reopened);
    assert_eq!(
        snapshot(dir.path()),
        after,
        "native reopen must preserve quiescent bytes"
    );
}

macro_rules! cases {
    ($($name:ident: $layer:ident, $directory:literal, $method:literal, $flags:literal;)*) => {
        $(#[test] fn $name() { case(Layer::$layer, $directory, $method, $flags); })*
    };
}

cases! {
    direct_stored_file: Direct, false, 0, 64;
    direct_deflated_file: Direct, false, 8, 64;
    direct_stored_directory: Direct, true, 0, 64;
    direct_deflated_directory: Direct, true, 8, 64;
    wrapper_stored_file: Wrapper, false, 0, 64;
    wrapper_deflated_file: Wrapper, false, 8, 64;
    wrapper_stored_directory: Wrapper, true, 0, 64;
    wrapper_deflated_directory: Wrapper, true, 8, 64;
    inner_stored_file: Inner, false, 0, 64;
    inner_deflated_file: Inner, false, 8, 64;
    inner_stored_directory: Inner, true, 0, 64;
    inner_deflated_directory: Inner, true, 8, 64;
    plain_direct_file: Direct, false, 0, 0;
    plain_direct_directory: Direct, true, 8, 0;
    plain_wrapper_file: Wrapper, false, 0, 0;
    plain_wrapper_directory: Wrapper, true, 8, 0;
    plain_inner_file: Inner, false, 0, 0;
    plain_inner_directory: Inner, true, 8, 0;
    bits0_and6_direct: Direct, false, 0, 65;
    bits0_and6_wrapper: Wrapper, false, 0, 65;
    bits0_and6_inner: Inner, false, 0, 65;
}

#[test]
fn bit6_only_wrapper_package_carrier() {
    let (_dir, conn) = fresh();
    let bytes = raw_zip(&[(INNER, &package(0x12), 64, 0)]);
    let error = match install_product_file(&conn, "wrapper.zip", &bytes, PackageLimits::STANDARD) {
        Err(error) => error,
        Ok(_) => panic!("bit-6-only package carrier must refuse"),
    };
    assert!(
        matches!(error, PackageError::InvalidZip { cause } if cause == format!("{INNER}: {REASON}"))
    );
    assert!(stored_packages(&conn).is_empty());
}
