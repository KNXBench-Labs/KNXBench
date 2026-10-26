//! Synthetic application admission regressions for contradictory or encrypted directories.
//!
//! Closed DELETE-mode snapshots prove this pre-persistence refusal boundary,
//! not general cross-store atomicity, live-WAL behavior or crash recovery.

use std::io::{Cursor, Read};

use super::stream;
use knx_etsproj::opaque::sha256_hex;
use knx_store::project_history::NativeSnapshot;
use knx_store::{load_manufacturer_refs, load_opaque, open_and_migrate, Connection};

const DIRECTORY: &str = "répertoire/";
const REASON: &str = "encrypted directory records are unsupported";
const WITNESS_PATH: &str = "directory-policy-witness.bin";
const NESTED_WITNESS_PATH: &str = "P-0001/directory-policy-witness.bin";
const WITNESS: &[u8] = b"synthetic directory admission persistence witness";
/// A well-formed WinZip AES field (AE-2, vendor `AE`, strength 3, Stored).
const AES_FIELD: [u8; 11] = [1, 0x99, 7, 0, 2, 0, b'A', b'E', 3, 0, 0];
const AES_REASON: &str = "a WinZip AES field (0x9901) requires compression method 99";

#[derive(Clone, Copy)]
enum Layer {
    Outer,
    DirectInner,
}

#[derive(Clone, Copy)]
enum Shape {
    StoredAbsentHeader,
    Raw0300ShortHeader,
}

fn new_manufacturer() -> Vec<u8> {
    stream::manufacturer("directory-policy-new")
}

fn with_directory(
    members: &[stream::Member],
    shape: Shape,
    added_flags: u16,
    aes: bool,
) -> Vec<u8> {
    let payload: &[u8] = match shape {
        Shape::StoredAbsentHeader => b"",
        Shape::Raw0300ShortHeader => &[0x03, 0x00],
    };
    // Reuse the existing ZIP writer/offset builder. Patch both headers only for
    // this directory: the raw 03 00 bytes are not a traditional cipher header.
    let mut bytes = stream::archive(members);
    let mut zip = zip::ZipArchive::new(Cursor::new(&bytes)).unwrap();
    let directory_index = zip.len() - 1;
    let entry = zip.by_index_raw(directory_index).unwrap();
    assert_eq!(entry.name(), DIRECTORY);
    assert!(entry.is_dir());
    assert_eq!(entry.compressed_size(), payload.len() as u64);
    let local = entry.header_start() as usize;
    let central = entry.central_header_start() as usize;
    drop(entry);
    drop(zip);
    for (offset, flags_at, method_at, crc_at, size_at) in
        [(local, 6, 8, 14, 22), (central, 8, 10, 16, 24)]
    {
        let flags = u16::from_le_bytes(
            bytes[offset + flags_at..offset + flags_at + 2]
                .try_into()
                .unwrap(),
        );
        assert_eq!(flags & 1, 0);
        bytes[offset + flags_at..offset + flags_at + 2]
            .copy_from_slice(&(flags | added_flags).to_le_bytes());
        let method: u16 = match shape {
            Shape::StoredAbsentHeader => 0,
            Shape::Raw0300ShortHeader => 8,
        };
        bytes[offset + method_at..offset + method_at + 2].copy_from_slice(&method.to_le_bytes());
        bytes[offset + crc_at..offset + crc_at + 4].copy_from_slice(&0u32.to_le_bytes());
        bytes[offset + size_at..offset + size_at + 4].copy_from_slice(&0u32.to_le_bytes());
    }
    if aes {
        insert_aes_field(&mut bytes, local, central);
    }
    bytes
}

/// Gives the last record (local and central) a WinZip AES field: shifts only
/// the central directory, then its size, in the end record without comment.
fn insert_aes_field(bytes: &mut Vec<u8>, local: usize, central: usize) {
    let field = AES_FIELD.len();
    for (header, name_len_at, extra_len_at, fixed) in
        [(local, 26, 28, 30), (central + field, 28, 30, 46)]
    {
        let u16_at = |at: usize| u16::from_le_bytes([bytes[header + at], bytes[header + at + 1]]);
        assert_eq!(
            u16_at(extra_len_at),
            0,
            "fixture record carries no extra field"
        );
        let name_end = header + fixed + usize::from(u16_at(name_len_at));
        bytes[header + extra_len_at..header + extra_len_at + 2]
            .copy_from_slice(&(field as u16).to_le_bytes());
        bytes.splice(name_end..name_end, AES_FIELD);
    }
    let end = bytes.len() - 22;
    assert_eq!(&bytes[end..end + 4], b"PK\x05\x06");
    for (at, delta) in [(end + 12, field), (end + 16, field)] {
        let value = u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) + delta as u32;
        bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
}

fn project(layer: Layer, shape: Shape, flags: u16, aes: bool) -> Vec<u8> {
    let mut template =
        zip::ZipArchive::new(Cursor::new(knx_testsupport::minimal_knxproj_bytes())).unwrap();
    let mut outer = Vec::new();
    let mut inner = Vec::new();
    for index in 0..template.len() {
        let mut file = template.by_index(index).unwrap();
        let mut plain = Vec::new();
        file.read_to_end(&mut plain).unwrap();
        if matches!(layer, Layer::DirectInner) && file.name().starts_with("P-0001/") {
            inner.push(stream::stored(
                file.name().strip_prefix("P-0001/").unwrap(),
                &plain,
            ));
        } else {
            outer.push(stream::stored(file.name(), &plain));
        }
    }
    // New source identity even in an already populated product database; the
    // XML comes from the maintained minimal fixture with only a comment added.
    outer.push(stream::stored(stream::EARLY, &new_manufacturer()));
    let payload: &[u8] = match shape {
        Shape::StoredAbsentHeader => b"",
        Shape::Raw0300ShortHeader => &[0x03, 0x00],
    };
    match layer {
        Layer::Outer => {
            outer.push(stream::stored(WITNESS_PATH, WITNESS));
            outer.push(stream::stored(DIRECTORY, payload));
            with_directory(&outer, shape, flags, aes)
        }
        Layer::DirectInner => {
            inner.push(stream::stored(WITNESS_PATH, WITNESS));
            inner.push(stream::stored(DIRECTORY, payload));
            outer.push(stream::stored(
                "P-0001.zip",
                &with_directory(&inner, shape, flags, aes),
            ));
            stream::archive(&outer)
        }
    }
}

fn application_case(layer: Layer, shape: Shape, populated: bool, encrypted: bool) {
    application_case_flags(layer, shape, populated, u16::from(encrypted));
}

fn application_case_flags(layer: Layer, shape: Shape, populated: bool, flags: u16) {
    application_case_full(layer, shape, populated, flags, false);
}

fn application_case_full(layer: Layer, shape: Shape, populated: bool, flags: u16, aes: bool) {
    // `encrypted` names every refused case, including the plaintext AES-field one.
    let encrypted = flags != 0 || aes;
    let dir = tempfile::tempdir().unwrap();
    let native_path = dir.path().join("project.knxdb");
    let products_path = dir.path().join("products.sqlite");
    assert!(!native_path.exists() && !products_path.exists());
    let store = open_and_migrate(&native_path).unwrap();
    let products = knx_productdb::open_and_migrate(&products_path).unwrap();
    for connection in [&store, &products] {
        let mode: String = connection
            .query_row("PRAGMA journal_mode=DELETE", [], |row| row.get(0))
            .unwrap();
        assert_eq!(mode, "delete");
    }
    let expected_native = if populated {
        let seed_path = knx_testsupport::write_minimal_knxproj(dir.path());
        let seed = knx_app::import_ets_project_with(
            &seed_path,
            &store,
            knx_app::ImportOptions {
                product_db: Some(&products),
            },
        )
        .unwrap();
        assert!(seed.manufacturer_ingested > 0);
        assert_eq!(seed.project.devices.iter().count(), 1);
        knx_store::save_project(&store, &seed.project).unwrap();
        Some(NativeSnapshot::read(&store).unwrap())
    } else {
        assert!(knx_store::load_project(&store).is_err());
        assert!(load_opaque(&store).unwrap().is_empty());
        assert!(load_manufacturer_refs(&store).unwrap().is_empty());
        None
    };
    let seed_sources: Vec<_> = load_manufacturer_refs(&store)
        .unwrap()
        .into_iter()
        .map(|reference| {
            let bytes = knx_productdb::load_source_file(&products, &reference.sha256)
                .unwrap()
                .unwrap();
            assert_eq!(sha256_hex(&bytes), reference.sha256);
            (reference.sha256, bytes)
        })
        .collect();
    assert_eq!(seed_sources.is_empty(), !populated);
    let manufacturer = new_manufacturer();
    let new_hash = sha256_hex(&manufacturer);
    assert!(!knx_productdb::has_source_file(&products, &new_hash).unwrap());
    assert!(!load_opaque(&store)
        .unwrap()
        .iter()
        .any(|entry| entry.source_path.ends_with(WITNESS_PATH)));
    let source = project(layer, shape, flags, aes);
    let source_path = dir.path().join("directory-policy.knxproj");
    std::fs::write(&source_path, &source).unwrap();
    drop(store);
    drop(products);
    let before = stream::disk_snapshot(dir.path());
    for database in ["project.knxdb", "products.sqlite"] {
        for suffix in ["-journal", "-wal", "-shm"] {
            assert!(!before.contains_key(std::path::Path::new(&format!("{database}{suffix}"))));
        }
    }
    // Creation and migrations are outside this boundary. These are ordinary
    // connections to closed, already migrated, exclusively owned test stores.
    let store = Connection::open(&native_path).unwrap();
    let products = knx_productdb::Connection::open(&products_path).unwrap();
    let password = stream::password(stream::Layer::PlainNested).unwrap();
    let result = knx_app::import_ets_project_with_password(
        &source_path,
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
        Some(&password),
        &(),
    );
    let expected_after = if encrypted {
        match result {
            Err(knx_app::AppError::Import(knx_etsproj::ImportFailure::Container(
                knx_etsproj::ContainerError::InconsistentRecord { path, reason },
            ))) => {
                assert_eq!(path, DIRECTORY, "refusal must name the decoded directory");
                let expected = if aes {
                    AES_REASON
                } else if flags == 64 {
                    "strong-encryption flag (bit 6) requires encryption flag (bit 0)"
                } else {
                    REASON
                };
                assert_eq!(reason, expected, "refusal must state the flag policy");
            }
            Err(other) => {
                panic!("expected explicit encrypted-directory InconsistentRecord, got {other}")
            }
            Ok(_) => {
                panic!("encrypted-directory policy must refuse before application persistence")
            }
        }
        assert!(!knx_productdb::has_source_file(&products, &new_hash).unwrap());
        expected_native
    } else {
        let imported = result.unwrap();
        assert_eq!(imported.project.info.name, "Minimal");
        assert_eq!(imported.project.devices.iter().count(), 1);
        assert!(imported.manufacturer_ingested > 0);
        assert_eq!(
            knx_productdb::load_source_file(&products, &new_hash)
                .unwrap()
                .unwrap(),
            manufacturer
        );
        assert!(load_manufacturer_refs(&store)
            .unwrap()
            .iter()
            .any(
                |reference| reference.source_path == stream::EARLY && reference.sha256 == new_hash
            ));
        let witness_path = match layer {
            Layer::Outer => WITNESS_PATH,
            Layer::DirectInner => NESTED_WITNESS_PATH,
        };
        let opaque = load_opaque(&store).unwrap();
        let witness = opaque
            .iter()
            .find(|entry| entry.source_path == witness_path)
            .unwrap();
        assert_eq!(witness.bytes, WITNESS);
        assert_eq!(witness.sha256, sha256_hex(WITNESS));
        assert!(!imported
            .report
            .to_json()
            .contains(knx_testsupport::ZIPCRYPTO_MINIMAL_PASSWORD));
        knx_store::save_project(&store, &imported.project).unwrap();
        assert!(knx_store::load_project(&store).unwrap() == imported.project);
        Some(NativeSnapshot::read(&store).unwrap())
    };
    drop(store);
    drop(products);
    let after = stream::disk_snapshot(dir.path());
    if encrypted {
        assert!(
            after == before,
            "refusal changed source, database or sidecar paths/bytes/absence"
        );
    } else {
        assert!(
            after != before,
            "positive import must actually mutate the destination stores"
        );
        for database in ["project.knxdb", "products.sqlite"] {
            assert!(
                after[std::path::Path::new(database)] != before[std::path::Path::new(database)]
            );
        }
    }
    assert_eq!(std::fs::read(&source_path).unwrap(), source);
    let reopened = open_and_migrate(&native_path).unwrap();
    if let Some(expected) = expected_after {
        assert!(NativeSnapshot::read(&reopened).unwrap() == expected);
        assert_eq!(
            knx_store::load_project(&reopened).unwrap().info.name,
            "Minimal"
        );
    } else {
        assert!(knx_store::load_project(&reopened).is_err());
        assert!(load_opaque(&reopened).unwrap().is_empty());
        assert!(load_manufacturer_refs(&reopened).unwrap().is_empty());
    }
    let products = knx_productdb::open_and_migrate(&products_path).unwrap();
    assert_eq!(
        knx_productdb::has_source_file(&products, &new_hash).unwrap(),
        !encrypted
    );
    if !encrypted {
        assert_eq!(
            knx_productdb::load_source_file(&products, &new_hash)
                .unwrap()
                .unwrap(),
            manufacturer
        );
    }
    for (hash, bytes) in seed_sources {
        assert_eq!(
            knx_productdb::load_source_file(&products, &hash)
                .unwrap()
                .unwrap(),
            bytes
        );
    }
    drop(reopened);
    drop(products);
    assert!(
        stream::disk_snapshot(dir.path()) == after,
        "native reopen changed the closed-file snapshot"
    );
}

#[test]
fn bit6_only_outer_stored_preserves_populated_stores() {
    application_case_flags(Layer::Outer, Shape::StoredAbsentHeader, true, 64);
}

#[test]
fn bit6_only_outer_deflated_preserves_populated_stores() {
    application_case_flags(Layer::Outer, Shape::Raw0300ShortHeader, true, 64);
}

#[test]
fn bit6_only_nested_stored_preserves_populated_stores() {
    application_case_flags(Layer::DirectInner, Shape::StoredAbsentHeader, true, 64);
}

#[test]
fn plaintext_aes_field_outer_preserves_populated_stores() {
    application_case_full(Layer::Outer, Shape::StoredAbsentHeader, true, 0, true);
}

#[test]
fn plaintext_aes_field_nested_preserves_populated_stores() {
    application_case_full(Layer::DirectInner, Shape::StoredAbsentHeader, true, 0, true);
}

#[test]
fn plaintext_aes_field_nested_refuses_fresh_destinations() {
    application_case_full(
        Layer::DirectInner,
        Shape::StoredAbsentHeader,
        false,
        0,
        true,
    );
}

#[test]
fn bit6_only_nested_deflated_preserves_populated_stores() {
    application_case_flags(Layer::DirectInner, Shape::Raw0300ShortHeader, true, 64);
}

#[test]
fn outer_stored_absent_header_refuses_fresh_destinations() {
    application_case(Layer::Outer, Shape::StoredAbsentHeader, false, true);
}
#[test]
fn outer_stored_absent_header_refuses_populated_destinations() {
    application_case(Layer::Outer, Shape::StoredAbsentHeader, true, true);
}
#[test]
fn outer_raw0300_short_header_refuses_fresh_destinations() {
    application_case(Layer::Outer, Shape::Raw0300ShortHeader, false, true);
}
#[test]
fn outer_raw0300_short_header_refuses_populated_destinations() {
    application_case(Layer::Outer, Shape::Raw0300ShortHeader, true, true);
}
#[test]
fn direct_inner_stored_absent_header_refuses_fresh_destinations() {
    application_case(Layer::DirectInner, Shape::StoredAbsentHeader, false, true);
}
#[test]
fn direct_inner_stored_absent_header_refuses_populated_destinations() {
    application_case(Layer::DirectInner, Shape::StoredAbsentHeader, true, true);
}
#[test]
fn direct_inner_raw0300_short_header_refuses_fresh_destinations() {
    application_case(Layer::DirectInner, Shape::Raw0300ShortHeader, false, true);
}
#[test]
fn direct_inner_raw0300_short_header_refuses_populated_destinations() {
    application_case(Layer::DirectInner, Shape::Raw0300ShortHeader, true, true);
}
#[test]
fn plaintext_outer_stored_directory_imports_fresh_destinations() {
    application_case(Layer::Outer, Shape::StoredAbsentHeader, false, false);
}
#[test]
fn plaintext_outer_stored_directory_imports_populated_destinations() {
    application_case(Layer::Outer, Shape::StoredAbsentHeader, true, false);
}
#[test]
fn plaintext_outer_deflated_directory_imports_fresh_destinations() {
    application_case(Layer::Outer, Shape::Raw0300ShortHeader, false, false);
}
#[test]
fn plaintext_outer_deflated_directory_imports_populated_destinations() {
    application_case(Layer::Outer, Shape::Raw0300ShortHeader, true, false);
}
#[test]
fn plaintext_direct_inner_stored_directory_imports_fresh_destinations() {
    application_case(Layer::DirectInner, Shape::StoredAbsentHeader, false, false);
}
#[test]
fn plaintext_direct_inner_stored_directory_imports_populated_destinations() {
    application_case(Layer::DirectInner, Shape::StoredAbsentHeader, true, false);
}
#[test]
fn plaintext_direct_inner_deflated_directory_imports_fresh_destinations() {
    application_case(Layer::DirectInner, Shape::Raw0300ShortHeader, false, false);
}
#[test]
fn plaintext_direct_inner_deflated_directory_imports_populated_destinations() {
    application_case(Layer::DirectInner, Shape::Raw0300ShortHeader, true, false);
}
