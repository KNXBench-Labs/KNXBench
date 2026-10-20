//! Synthetic application admission regressions for bit-0 project directories.
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

fn with_directory(members: &[stream::Member], shape: Shape, encrypted: bool) -> Vec<u8> {
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
            .copy_from_slice(&(flags | u16::from(encrypted)).to_le_bytes());
        let method: u16 = match shape {
            Shape::StoredAbsentHeader => 0,
            Shape::Raw0300ShortHeader => 8,
        };
        bytes[offset + method_at..offset + method_at + 2].copy_from_slice(&method.to_le_bytes());
        bytes[offset + crc_at..offset + crc_at + 4].copy_from_slice(&0u32.to_le_bytes());
        bytes[offset + size_at..offset + size_at + 4].copy_from_slice(&0u32.to_le_bytes());
    }
    bytes
}

fn project(layer: Layer, shape: Shape, encrypted: bool) -> Vec<u8> {
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
            with_directory(&outer, shape, encrypted)
        }
        Layer::DirectInner => {
            inner.push(stream::stored(WITNESS_PATH, WITNESS));
            inner.push(stream::stored(DIRECTORY, payload));
            outer.push(stream::stored(
                "P-0001.zip",
                &with_directory(&inner, shape, encrypted),
            ));
            stream::archive(&outer)
        }
    }
}

fn application_case(layer: Layer, shape: Shape, populated: bool, encrypted: bool) {
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
    let source = project(layer, shape, encrypted);
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
                assert_eq!(reason, REASON, "refusal must state the unsupported policy");
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
