//! `knx-app`'s tests cannot reach `knx-etsproj`'s `tests/support` module —
//! a crate's integration tests are private to it — so this uses
//! `knx-testsupport`, the shared dev-dependency every crate's tests can
//! reach, instead of a public test-only API being added to `knx-etsproj`
//! for their benefit.

use std::path::{Path, PathBuf};

use knx_app::import_ets_project;
use knx_etsproj::opaque::sha256_hex;
use knx_store::{load_opaque, open_and_migrate, Connection};

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

fn migrated_store() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    (dir, conn)
}

#[test]
fn synthetic_default_lines_and_device_local_objects_survive_native_reopen() {
    for version in [11, 21, 23] {
        for default_line in [None, Some("P-0001-0_L-3"), Some(""), Some("P-0001-0_L-99")] {
            for use_products in [false, true] {
                let dir = tempfile::tempdir().unwrap();
                let source_path = dir.path().join("synthetic-mapping.knxproj");
                let native_path = dir.path().join("mapping.knxdb");
                let ref_id = if version == 11 {
                    "M-0001_A-1_O-7_R-1"
                } else {
                    "O-7_R-1"
                };
                let source_bytes = knx_testsupport::mapping_boundary_knxproj_bytes(
                    version,
                    default_line,
                    default_line,
                    ref_id,
                );
                std::fs::write(&source_path, &source_bytes).unwrap();
                let store = open_and_migrate(&native_path).unwrap();
                let products =
                    knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
                let imported = knx_app::import_ets_project_with(
                    &source_path,
                    &store,
                    knx_app::ImportOptions {
                        product_db: use_products.then_some(&products),
                    },
                )
                .unwrap();
                let unresolved = matches!(default_line, Some("") | Some("P-0001-0_L-99"));
                assert_eq!(imported.report.errors.len(), if unresolved { 2 } else { 0 });
                let installation = &imported.project.installations[0];
                let expected_line = (default_line == Some("P-0001-0_L-3"))
                    .then_some(installation.topology.lines[1].id);
                assert_eq!(installation.default_line, expected_line);
                assert_eq!(installation.buildings[0].default_line, expected_line);
                assert_eq!(imported.project.devices.iter().count(), 2);
                assert_eq!(imported.project.devices.com_objects().count(), 4);
                for device in imported.project.devices.iter() {
                    let object = imported
                        .project
                        .devices
                        .com_object(device.com_objects[0])
                        .unwrap();
                    assert_eq!(object.device, device.id);
                    assert_eq!(object.number, 7);
                    assert_eq!(object.source.ets_id, ref_id);
                }
                knx_store::save_project(&store, &imported.project).unwrap();
                let expected_opaque = load_opaque(&store).unwrap();
                let note = expected_opaque
                    .iter()
                    .find(|entry| entry.source_path == "note.txt")
                    .unwrap();
                assert_eq!(note.bytes, b"opaque mapping witness");
                assert_eq!(note.sha256, sha256_hex(b"opaque mapping witness"));
                drop(store);
                let reopened = open_and_migrate(&native_path).unwrap();
                // Entire model: identity, links, flags/layers, both hierarchies and counters.
                assert!(knx_store::load_project(&reopened).unwrap() == imported.project);
                assert_eq!(load_opaque(&reopened).unwrap(), expected_opaque);
                assert_eq!(std::fs::read(&source_path).unwrap(), source_bytes);
                // Unresolved source line tokens are reported, not invented as LineIds.
                // This does not add import-report persistence or ETS export.
            }
        }
    }
}

#[test]
fn unsupported_master_metadata_is_reported_and_its_bytes_survive_native_reopen() {
    let mut template =
        knx_etsproj::Container::open(knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let topology = template.read("P-0001/0.xml").unwrap();
    let info = template.read("P-0001/Project.xml").unwrap();
    let manufacturer = template.read("M-0001/M-0001_A-1.xml").unwrap();
    let master = br#"<KNX xmlns="urn:MASTER_METADATA_SENTINEL/11" CreatedBy="raw producer" ToolVersion="raw version lexeme">
      <MasterData><DatapointTypes>
        <DatapointType Id="DPT-42" Number="42" Name="untrusted master type"/>
      </DatapointTypes></MasterData>
    </KNX>"#;
    let expected = knx_etsproj::report::UnsupportedFeature {
        what: "knx_master.xml root metadata".to_string(),
        consequence: "unsupported KNX project namespace; master namespace comparison unavailable; original master bytes retained without interpreting root metadata".to_string(),
    };

    for use_products in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let native_path = dir.path().join("project.knxdb");
        let store = open_and_migrate(&native_path).unwrap();
        let products =
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
        let source_path = dir.path().join("unsupported-master.knxproj");
        std::fs::write(
            &source_path,
            knx_testsupport::zip_with_entries(&[
                ("P-0001.signature", b"x"),
                ("P-0001/0.xml", &topology),
                ("P-0001/Project.xml", &info),
                ("knx_master.xml", master),
                ("M-0001/M-0001_A-1.xml", &manufacturer),
            ]),
        )
        .unwrap();
        let source_before = std::fs::read(&source_path).unwrap();
        let imported = knx_app::import_ets_project_with(
            &source_path,
            &store,
            knx_app::ImportOptions {
                product_db: use_products.then_some(&products),
            },
        )
        .unwrap();
        let mut expected_features = vec![expected.clone()];
        if !use_products {
            expected_features.push(knx_etsproj::report::UnsupportedFeature {
                what: "manufacturer semantic diagnostics".into(),
                consequence: "unavailable without a product database; source files are retained, not semantically installed".into(),
            });
        }
        assert_eq!(imported.report.unsupported, expected_features);
        assert!(!imported
            .report
            .to_json()
            .contains("MASTER_METADATA_SENTINEL"));
        let dpt_rows: i64 = products
            .query_row("SELECT COUNT(*) FROM datapoint_type", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            dpt_rows, 0,
            "unsupported master roots must not create typed DPT rows"
        );
        knx_store::save_project(&store, &imported.project).unwrap();
        let expected_opaque = load_opaque(&store).unwrap();
        drop(store);

        let reopened = open_and_migrate(&native_path).unwrap();
        assert!(knx_store::load_project(&reopened).unwrap() == imported.project);
        let opaque = load_opaque(&reopened).unwrap();
        assert_eq!(opaque, expected_opaque);
        let retained: Vec<_> = opaque
            .iter()
            .filter(|entry| entry.source_path == "knx_master.xml")
            .collect();
        assert_eq!(retained.len(), 1);
        assert_eq!(retained[0].kind, "MasterData");
        assert_eq!(retained[0].bytes, master);
        assert_eq!(retained[0].sha256, sha256_hex(master));
        assert_eq!(std::fs::read(&source_path).unwrap(), source_before);
        // The store preserves source evidence, not a newly persisted import
        // report. Report availability after reopening is a separate contract.
    }
}

#[test]
fn a_supported_master_root_still_allows_typed_master_ingestion() {
    let mut template =
        knx_etsproj::Container::open(knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let topology = template.read("P-0001/0.xml").unwrap();
    let info = template.read("P-0001/Project.xml").unwrap();
    let master = br#"<KNX xmlns="http://knx.org/xml/project/11">
      <MasterData><DatapointTypes>
        <DatapointType Id="DPT-1" Number="1" Name="declared master type"/>
      </DatapointTypes></MasterData>
    </KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let store = open_and_migrate(&dir.path().join("project.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let source_path = dir.path().join("supported-master.knxproj");
    std::fs::write(
        &source_path,
        knx_testsupport::zip_with_entries(&[
            ("P-0001.signature", b"x"),
            ("P-0001/0.xml", &topology),
            ("P-0001/Project.xml", &info),
            ("knx_master.xml", master),
        ]),
    )
    .unwrap();
    let imported = knx_app::import_ets_project_with(
        &source_path,
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();
    assert_eq!(imported.report.unsupported, vec![]);
    let rows: Vec<(String, i64, String)> = products
        .prepare("SELECT id, main, name FROM datapoint_type ORDER BY id")
        .unwrap()
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    assert_eq!(
        rows,
        vec![("DPT-1".to_string(), 1, "declared master type".to_string())]
    );
}

#[test]
fn namespace_refusal_preserves_existing_project_and_product_databases() {
    // These are synthetic archives, not private or independent ETS samples.
    let mut template =
        knx_etsproj::Container::open(knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let topology = String::from_utf8(template.read("P-0001/0.xml").unwrap()).unwrap();
    let info = String::from_utf8(template.read("P-0001/Project.xml").unwrap()).unwrap();
    let master = template.read("knx_master.xml").unwrap();
    let manufacturer = template.read("M-0001/M-0001_A-1.xml").unwrap();
    let canonical = "http://knx.org/xml/project/11";

    for use_products in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let store_path = dir.path().join("project.sqlite");
        let products_path = dir.path().join("products.sqlite");
        let store = open_and_migrate(&store_path).unwrap();
        let products = knx_productdb::open_and_migrate(&products_path).unwrap();
        // Seed both databases, including the retained source bytes, so an
        // unchanged empty database cannot make this test vacuously green.
        let seed_path = knx_testsupport::write_minimal_knxproj(dir.path());
        let seed = knx_app::import_ets_project_with(
            &seed_path,
            &store,
            knx_app::ImportOptions {
                product_db: Some(&products),
            },
        )
        .unwrap();
        assert!(seed.opaque_entries > 0);
        assert!(seed.manufacturer_ingested > 0);
        assert!(!seed.project.installations.is_empty());
        assert!(seed.project.devices.iter().next().is_some());
        knx_store::save_project(&store, &seed.project).unwrap();
        assert!(
            knx_store::load_project(&store).unwrap() == seed.project,
            "refusal fixture must contain the persisted nonempty seed project"
        );
        assert!(!knx_store::load_manufacturer_refs(&store)
            .unwrap()
            .is_empty());

        // Compare the durable SQLite files AND their WALs. Lock bookkeeping
        // in -shm is not project/product data and is deliberately excluded.
        let snapshot = || {
            [&store_path, &products_path]
                .into_iter()
                .flat_map(|path| {
                    [
                        path.clone(),
                        PathBuf::from(format!("{}-wal", path.display())),
                    ]
                })
                .map(|path| match std::fs::read(&path) {
                    Ok(bytes) => Some(bytes),
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                    Err(error) => panic!("cannot read synthetic database: {error}"),
                })
                .collect::<Vec<_>>()
        };
        let before = snapshot();
        // Filename-only legacy refusal must propagate through both application
        // modes without touching the seeded main files, WALs or source bytes.
        for extension in [
            "vd2", "vd3", "vd4", "vd5", "pr3", "pr4", "pr5", "VD4", "Pr3",
        ] {
            let rejected_path = dir.path().join(format!("rejected.{extension}"));
            std::fs::copy(&seed_path, &rejected_path).unwrap();
            let source_before = std::fs::read(&rejected_path).unwrap();
            let result = knx_app::import_ets_project_with(
                &rejected_path,
                &store,
                knx_app::ImportOptions {
                    product_db: use_products.then_some(&products),
                },
            );
            match result {
                Err(knx_app::AppError::Import(
                    knx_etsproj::ImportFailure::UnsupportedLegacyFormat { extension: refused },
                )) => assert_eq!(refused, extension),
                _ => panic!("application did not propagate the typed legacy refusal"),
            }
            assert!(
                snapshot() == before,
                "legacy refusal changed durable database bytes"
            );
            assert!(knx_store::load_project(&store).unwrap() == seed.project);
            assert_eq!(std::fs::read(&rejected_path).unwrap(), source_before);
        }
        for (topology_ns, info_ns, entry, mismatch) in [
            ("urn:foreign/11", canonical, "P-0001/0.xml", false),
            (canonical, "urn:foreign/11", "P-0001/Project.xml", false),
            (
                canonical,
                "http://knx.org/xml/project/21",
                "P-0001/Project.xml",
                true,
            ),
        ] {
            let changed_topology = topology.replace(canonical, topology_ns);
            let changed_info = info.replace(canonical, info_ns);
            let rejected_path = dir.path().join("rejected.knxproj");
            std::fs::write(
                &rejected_path,
                knx_testsupport::zip_with_entries(&[
                    ("P-0001.signature", b"synthetic signature"),
                    ("P-0001/0.xml", changed_topology.as_bytes()),
                    ("P-0001/Project.xml", changed_info.as_bytes()),
                    ("knx_master.xml", &master),
                    ("M-0001/M-0001_A-1.xml", &manufacturer),
                ]),
            )
            .unwrap();
            let result = knx_app::import_ets_project_with(
                &rejected_path,
                &store,
                knx_app::ImportOptions {
                    product_db: use_products.then_some(&products),
                },
            );
            match result {
                Err(knx_app::AppError::Import(knx_etsproj::ImportFailure::Detect(error))) => {
                    use knx_etsproj::detect::DetectError;
                    match error {
                        DetectError::NamespaceMismatch {
                            entry: rejected,
                            expected,
                            actual,
                        } if mismatch => {
                            assert_eq!(rejected, entry);
                            assert_eq!(expected, canonical);
                            assert_eq!(actual, info_ns);
                        }
                        DetectError::UnsupportedNamespace {
                            entry: rejected,
                            namespace,
                        } if !mismatch => {
                            assert_eq!(rejected, entry);
                            assert_eq!(namespace, "urn:foreign/11");
                        }
                        _ => panic!("wrong structured namespace-refusal diagnostic"),
                    }
                }
                _ => panic!("application import did not refuse the namespace"),
            }
            assert!(
                snapshot() == before,
                "refusal modified durable database bytes"
            );
            assert!(knx_store::load_project(&store).unwrap() == seed.project);
        }
    }
}

/// Edits an archive given the offsets of one record's local header and
/// central record.
type Damage = fn(&mut [u8], usize, usize);

/// The synthetic minimal project plus an empty `P-0001/BinaryData/`
/// directory record, then `damage` applied to the directory's local header
/// and its central record alike, so the two still agree.
fn minimal_with_directory(damage: Damage) -> Vec<u8> {
    use std::io::{Cursor, Read, Write};
    const DIRECTORY: &[u8] = b"P-0001/BinaryData/";
    let mut template =
        zip::ZipArchive::new(Cursor::new(knx_testsupport::minimal_knxproj_bytes())).unwrap();
    let stored =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for i in 0..template.len() {
        let mut entry = template.by_index(i).unwrap();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        writer.start_file(entry.name(), stored).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer
        .add_directory(std::str::from_utf8(DIRECTORY).unwrap(), stored)
        .unwrap();
    let mut bytes = writer.finish().unwrap().into_inner();
    let find = |signature: &[u8], name_at: usize| {
        (0..bytes.len())
            .find(|&at| {
                bytes[at..].starts_with(signature) && bytes[at + name_at..].starts_with(DIRECTORY)
            })
            .expect("the directory record")
    };
    let local = find(b"PK\x03\x04", 30);
    let central = find(b"PK\x01\x02", 46);
    damage(&mut bytes, local, central);
    bytes
}

#[test]
fn a_malformed_empty_directory_refuses_without_changing_existing_databases() {
    // WP-16: both headers agree, so only the payload rule can refuse it.
    let crc = |bytes: &mut [u8], local: usize, central: usize| {
        bytes[local + 14..local + 18].copy_from_slice(&0xdead_beefu32.to_le_bytes());
        bytes[central + 16..central + 20].copy_from_slice(&0xdead_beefu32.to_le_bytes());
    };
    let no_stream = |bytes: &mut [u8], local: usize, central: usize| {
        bytes[local + 8..local + 10].copy_from_slice(&8u16.to_le_bytes());
        bytes[central + 10..central + 12].copy_from_slice(&8u16.to_le_bytes());
    };
    let damaged: [(Damage, &str); 2] = [
        (crc, "CRC-32"),
        (no_stream, "no complete compressed stream"),
    ];

    for use_products in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let options = |products| knx_app::ImportOptions {
            product_db: use_products.then_some(products),
        };
        // The undamaged archive imports, so a refusal below is the damage.
        let control_path = dir.path().join("control.knxproj");
        std::fs::write(&control_path, minimal_with_directory(|_, _, _| {})).unwrap();
        let control_store = open_and_migrate(&dir.path().join("control.sqlite")).unwrap();
        let control_products =
            knx_productdb::open_and_migrate(&dir.path().join("control-products.sqlite")).unwrap();
        knx_app::import_ets_project_with(&control_path, &control_store, options(&control_products))
            .unwrap();

        let store_path = dir.path().join("project.sqlite");
        let products_path = dir.path().join("products.sqlite");
        let store = open_and_migrate(&store_path).unwrap();
        let products = knx_productdb::open_and_migrate(&products_path).unwrap();
        let seed_path = knx_testsupport::write_minimal_knxproj(dir.path());
        let seed = knx_app::import_ets_project_with(
            &seed_path,
            &store,
            knx_app::ImportOptions {
                product_db: Some(&products),
            },
        )
        .unwrap();
        knx_store::save_project(&store, &seed.project).unwrap();
        assert!(!seed.project.installations.is_empty());
        assert!(!load_opaque(&store).unwrap().is_empty());
        let snapshot = || {
            [&store_path, &products_path]
                .into_iter()
                .flat_map(|path| {
                    [
                        path.clone(),
                        PathBuf::from(format!("{}-wal", path.display())),
                    ]
                })
                .map(|path| std::fs::read(path).ok())
                .collect::<Vec<_>>()
        };
        let before = snapshot();
        for (damage, why) in damaged {
            let rejected_path = dir.path().join("rejected.knxproj");
            std::fs::write(&rejected_path, minimal_with_directory(damage)).unwrap();
            match knx_app::import_ets_project_with(&rejected_path, &store, options(&products)) {
                Err(knx_app::AppError::Import(knx_etsproj::ImportFailure::Container(
                    knx_etsproj::ContainerError::InconsistentRecord { path, reason },
                ))) => {
                    assert_eq!(path, "P-0001/BinaryData/");
                    assert!(reason.contains(why), "{reason}");
                }
                _ => panic!("the damaged directory was not refused ({why})"),
            }
            assert!(
                snapshot() == before,
                "refusal modified durable database bytes"
            );
            assert!(knx_store::load_project(&store).unwrap() == seed.project);
        }
    }
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn importing_persists_the_opaque_entries() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let (_dir, conn) = migrated_store();
    let imported = import_ets_project(&reference_ets4_path(), &conn).unwrap();
    assert_eq!(imported.project.installations.len(), 1);
    assert!(imported.opaque_entries > 0);
    assert_eq!(load_opaque(&conn).unwrap().len(), imported.opaque_entries);
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_persisted_bytes_are_the_bytes_that_were_read() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let (_dir, conn) = migrated_store();
    import_ets_project(&reference_ets4_path(), &conn).unwrap();
    let stored = load_opaque(&conn).unwrap();
    let dll = stored
        .iter()
        .find(|e| e.source_path.ends_with("econEts3.dll"))
        .expect("the baggage entry is persisted, not executed");
    assert_eq!(sha256_hex(&dll.bytes), dll.sha256);
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn a_failed_import_leaves_the_store_untouched() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let (_dir, conn) = migrated_store();
    assert!(import_ets_project(Path::new("/nonexistent.knxproj"), &conn).is_err());
    assert_eq!(load_opaque(&conn).unwrap().len(), 0);
}

#[test]
fn master_identity_diagnostics_reach_the_project_import_report() {
    let mut template =
        knx_etsproj::Container::open(knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let topology = template.read("P-0001/0.xml").unwrap();
    let info = template.read("P-0001/Project.xml").unwrap();
    let manufacturer = template.read("M-0001/M-0001_A-1.xml").unwrap();
    let master=br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><SpaceUsage Id="U"/><SpaceUsage Id="U"/><FunctionPoint/><DatapointType/></MasterData></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("master-diagnostics.knxproj");
    std::fs::write(
        &path,
        knx_testsupport::zip_with_entries(&[
            ("P-0001.signature", b"x"),
            ("P-0001/0.xml", &topology),
            ("P-0001/Project.xml", &info),
            ("M-0001/M-0001_A-1.xml", &manufacturer),
            ("knx_master.xml", master),
        ]),
    )
    .unwrap();
    let native = knx_store::open_and_migrate(&dir.path().join("project.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let imported = knx_app::import_ets_project_with(
        &path,
        &native,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();
    let diagnostics = imported
        .report
        .unsupported
        .iter()
        .filter(|d| d.what.starts_with("master provenance: "))
        .collect::<Vec<_>>();
    assert_eq!(diagnostics.len(), 3);
    assert_eq!(
        diagnostics
            .iter()
            .filter(|d| d.what.contains("master-identity-collision"))
            .count(),
        1
    );
    assert_eq!(
        diagnostics
            .iter()
            .filter(|d| d.what.contains("malformed-master-declaration"))
            .count(),
        2
    );
    for d in diagnostics {
        assert!(d.consequence.contains(&sha256_hex(master)));
        assert!(d.consequence.contains("declaration"));
    }
}

// WP-16: these tests establish pre-persistence refusal only. Later service
// failures do not have a cross-database transaction/whole-import atomicity claim.
use super::project_stream_fixture as stream;

fn stream_refusal_preserves_destinations(
    layer: stream::Layer,
    seeded: bool,
    use_products: bool,
    manufacturer: bool,
) {
    use knx_store::project_history::NativeSnapshot;
    let dir = tempfile::tempdir().unwrap();
    let native_path = dir.path().join("project.knxdb");
    let product_path = dir.path().join("products.sqlite");
    assert!(!native_path.exists() && !product_path.exists());
    let store = open_and_migrate(&native_path).unwrap();
    let products = knx_productdb::open_and_migrate(&product_path).unwrap();
    store.execute_batch("PRAGMA journal_mode=DELETE").unwrap();
    products
        .execute_batch("PRAGMA journal_mode=DELETE")
        .unwrap();
    let expected = if seeded {
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
        assert!(seed.project.devices.iter().next().is_some());
        knx_store::save_project(&store, &seed.project).unwrap();
        Some(NativeSnapshot::read(&store).unwrap())
    } else {
        assert!(load_opaque(&store).unwrap().is_empty());
        assert!(knx_store::load_manufacturer_refs(&store)
            .unwrap()
            .is_empty());
        None
    };
    let expected_product_sources: Vec<_> = knx_store::load_manufacturer_refs(&store)
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
    assert_eq!(expected_product_sources.is_empty(), !seeded);
    let early_hash = sha256_hex(&stream::manufacturer("early-new"));
    assert!(
        !knx_productdb::has_source_file(&products, &early_hash).unwrap(),
        "early manufacturer must be truly new, not a hash skip"
    );
    let source = stream::project(layer, true, manufacturer);
    let source_path = dir.path().join("fixed-source.knxproj");
    std::fs::write(&source_path, &source).unwrap();
    // Inventory order is the actual collector order, not just ZIP write order.
    let inventory = knx_etsproj::Container::open_outer(source.clone()).unwrap();
    let early = inventory
        .entries()
        .iter()
        .position(|e| e.path == stream::EARLY)
        .unwrap();
    let late = match layer {
        stream::Layer::Outer => {
            if manufacturer {
                stream::LATE_MANUFACTURER
            } else {
                stream::LATE_OPAQUE
            }
        }
        _ => "P-0001.zip",
    };
    let late_index = inventory
        .entries()
        .iter()
        .position(|e| e.path == late)
        .unwrap();
    assert!(
        early < late_index,
        "new manufacturer must precede damage in logical inventory"
    );
    drop(inventory);
    drop(store);
    drop(products);
    let before = stream::disk_snapshot(dir.path());
    // Already migrated connections: DB creation/migration is deliberately outside
    // the refusal boundary, and fresh-store sidecar absence is snapshotted too.
    let store = Connection::open(&native_path).unwrap();
    let products = knx_productdb::Connection::open(&product_path).unwrap();
    let password = stream::password(layer);
    let result = knx_app::import_ets_project_with_password(
        &source_path,
        &store,
        knx_app::ImportOptions {
            product_db: use_products.then_some(&products),
        },
        password.as_ref(),
        &(),
    );
    let error = match result {
        Err(knx_app::AppError::Import(knx_etsproj::ImportFailure::Container(error))) => error,
        Err(other) => panic!("expected container refusal, got {other}"),
        Ok(_) => panic!("stream policy must refuse before application persistence"),
    };
    match (layer, error) {
        (stream::Layer::ZipCrypto, knx_etsproj::ContainerError::WrongPassword { nested_entry }) => {
            assert_eq!(nested_entry, "P-0001.zip")
        }
        (stream::Layer::ZipCrypto, other) => {
            panic!("correct-credential encrypted damage changed existing mapper: {other}")
        }
        (_, knx_etsproj::ContainerError::Read { path, cause }) => {
            let expected_path = if matches!(layer, stream::Layer::PlainNested) {
                stream::NESTED_LEAF
            } else {
                late
            };
            assert_eq!(path, expected_path);
            assert!(
                !cause.is_empty(),
                "stream refusal needs its technical cause"
            );
        }
        (_, other) => panic!("expected named Read refusal, got {other}"),
    }
    assert!(!knx_productdb::has_source_file(&products, &early_hash).unwrap());
    drop(store);
    drop(products);
    assert!(
        stream::disk_snapshot(dir.path()) == before,
        "refusal changed source, DB paths/bytes or sidecar bytes/absence"
    );
    assert_eq!(std::fs::read(&source_path).unwrap(), source);
    let reopened = open_and_migrate(&native_path).unwrap();
    if let Some(expected) = expected {
        assert!(
            NativeSnapshot::read(&reopened).unwrap() == expected,
            "seeded native data changed on reopen"
        );
    } else {
        assert!(knx_store::load_project(&reopened).is_err());
        assert!(load_opaque(&reopened).unwrap().is_empty());
        assert!(knx_store::load_manufacturer_refs(&reopened)
            .unwrap()
            .is_empty());
    }
    let products = knx_productdb::open_and_migrate(&product_path).unwrap();
    assert!(!knx_productdb::has_source_file(&products, &early_hash).unwrap());
    for (hash, bytes) in expected_product_sources {
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
        stream::disk_snapshot(dir.path()) == before,
        "native reopen changed closed-file snapshot"
    );
}

#[test]
fn wp16_outer_opaque_refusal_fresh_with_products() {
    stream_refusal_preserves_destinations(stream::Layer::Outer, false, true, false);
}
#[test]
fn wp16_outer_opaque_refusal_fresh_without_products() {
    stream_refusal_preserves_destinations(stream::Layer::Outer, false, false, false);
}
#[test]
fn wp16_outer_opaque_refusal_seeded_with_products() {
    stream_refusal_preserves_destinations(stream::Layer::Outer, true, true, false);
}
#[test]
fn wp16_outer_opaque_refusal_seeded_without_products() {
    stream_refusal_preserves_destinations(stream::Layer::Outer, true, false, false);
}
#[test]
fn wp16_outer_manufacturer_refusal_fresh_with_products() {
    stream_refusal_preserves_destinations(stream::Layer::Outer, false, true, true);
}
#[test]
fn wp16_outer_manufacturer_refusal_fresh_without_products() {
    stream_refusal_preserves_destinations(stream::Layer::Outer, false, false, true);
}
#[test]
fn wp16_outer_manufacturer_refusal_seeded_with_products() {
    stream_refusal_preserves_destinations(stream::Layer::Outer, true, true, true);
}
#[test]
fn wp16_outer_manufacturer_refusal_seeded_without_products() {
    stream_refusal_preserves_destinations(stream::Layer::Outer, true, false, true);
}
#[test]
fn wp16_plain_nested_refusal_fresh_without_products() {
    stream_refusal_preserves_destinations(stream::Layer::PlainNested, false, false, false);
}
#[test]
fn wp16_plain_nested_refusal_seeded_with_products() {
    stream_refusal_preserves_destinations(stream::Layer::PlainNested, true, true, false);
}
#[test]
fn wp16_correct_password_zipcrypto_refusal_fresh_with_products() {
    stream_refusal_preserves_destinations(stream::Layer::ZipCrypto, false, true, false);
}
#[test]
fn wp16_correct_password_zipcrypto_refusal_seeded_without_products() {
    stream_refusal_preserves_destinations(stream::Layer::ZipCrypto, true, false, false);
}
#[test]
fn wp16_compressed_nested_carrier_refusal_seeded_with_products() {
    stream_refusal_preserves_destinations(stream::Layer::CompressedCarrier, true, true, false);
}

fn stream_control_reopens(layer: stream::Layer) {
    use knx_store::project_history::NativeSnapshot;
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("control.knxproj");
    let native_path = dir.path().join("control.knxdb");
    let source = stream::project(layer, false, false);
    std::fs::write(&source_path, &source).unwrap();
    let store = open_and_migrate(&native_path).unwrap();
    let product_path = dir.path().join("products.sqlite");
    let products = knx_productdb::open_and_migrate(&product_path).unwrap();
    let password = stream::password(layer);
    let imported = knx_app::import_ets_project_with_password(
        &source_path,
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
        password.as_ref(),
        &(),
    )
    .unwrap();
    assert_eq!(imported.project.info.name, "Minimal");
    assert_eq!(imported.project.devices.iter().count(), 1);
    let early = stream::manufacturer("early-new");
    let hash = sha256_hex(&early);
    assert_eq!(
        knx_productdb::load_source_file(&products, &hash)
            .unwrap()
            .unwrap(),
        early
    );
    let refs = knx_store::load_manufacturer_refs(&store).unwrap();
    assert!(refs
        .iter()
        .any(|r| r.source_path == stream::EARLY && r.sha256 == hash));
    let opaque = load_opaque(&store).unwrap();
    let leaf_path = if matches!(layer, stream::Layer::Outer) {
        stream::LATE_OPAQUE
    } else {
        stream::NESTED_LEAF
    };
    let witness = opaque.iter().find(|e| e.source_path == leaf_path).unwrap();
    assert_eq!(witness.bytes, stream::WITNESS);
    assert_eq!(witness.sha256, sha256_hex(stream::WITNESS));
    if matches!(layer, stream::Layer::ZipCrypto) {
        // Independently decode the actual encrypted leaf with zip's cipher.
        use std::io::{Cursor, Read};
        let mut outer = zip::ZipArchive::new(Cursor::new(&source)).unwrap();
        let mut nested = Vec::new();
        outer
            .by_name("P-0001.zip")
            .unwrap()
            .read_to_end(&mut nested)
            .unwrap();
        let mut inner = zip::ZipArchive::new(Cursor::new(nested)).unwrap();
        let mut leaf = inner
            .by_name_decrypt(
                stream::LATE_OPAQUE,
                knx_testsupport::ZIPCRYPTO_MINIMAL_PASSWORD.as_bytes(),
            )
            .unwrap();
        assert!(leaf.encrypted());
        assert_eq!(
            leaf.compressed_size(),
            stream::WITNESS.len() as u64 + 5 + 12
        );
        let mut decoded = Vec::new();
        leaf.read_to_end(&mut decoded).unwrap();
        assert_eq!(decoded, stream::WITNESS);
    }
    assert!(!imported
        .report
        .to_json()
        .contains(knx_testsupport::ZIPCRYPTO_MINIMAL_PASSWORD));
    if !matches!(layer, stream::Layer::Outer) {
        assert!(
            imported.report.unsupported.iter().any(|feature| {
                feature.what == "project password protection (P-0001.zip, ZipCrypto)"
                    && feature.consequence == "the project part was decrypted for import and is kept unprotected from here on (in memory, in the retained original files and in a saved .knxdb); the password protection itself is not preserved"
            }),
            "prepared nested imports retain the exact existing protection-loss warning"
        );
    }
    knx_store::save_project(&store, &imported.project).unwrap();
    let expected = NativeSnapshot::read(&store).unwrap();
    drop(store);
    drop(products);
    let reopened = open_and_migrate(&native_path).unwrap();
    assert!(NativeSnapshot::read(&reopened).unwrap() == expected);
    let products = knx_productdb::open_and_migrate(&product_path).unwrap();
    assert_eq!(
        knx_productdb::load_source_file(&products, &hash)
            .unwrap()
            .unwrap(),
        early
    );
    assert_eq!(std::fs::read(source_path).unwrap(), source);
}

#[test]
fn wp16_valid_alternate_outer_stream_reopens_exact_bytes() {
    stream_control_reopens(stream::Layer::Outer);
}
#[test]
fn wp16_valid_alternate_plain_nested_stream_reopens_exact_bytes() {
    stream_control_reopens(stream::Layer::PlainNested);
}
#[test]
fn wp16_valid_alternate_zipcrypto_stream_reopens_exact_bytes() {
    stream_control_reopens(stream::Layer::ZipCrypto);
}
#[test]
fn wp16_valid_alternate_compressed_carrier_reopens_exact_bytes() {
    stream_control_reopens(stream::Layer::CompressedCarrier);
}

#[test]
fn wp16_correct_password_invalid_zipcrypto_stream_keeps_existing_wrong_password_contract() {
    let dir = tempfile::tempdir().unwrap();
    let source_path = dir.path().join("correct-password-damaged.knxproj");
    let source = stream::invalid_zipcrypto_project();
    std::fs::write(&source_path, &source).unwrap();
    let native_path = dir.path().join("empty.knxdb");
    let store = open_and_migrate(&native_path).unwrap();
    store.execute_batch("PRAGMA journal_mode=DELETE").unwrap();
    drop(store);
    let before = stream::disk_snapshot(dir.path());
    let store = Connection::open(&native_path).unwrap();
    let password = stream::password(stream::Layer::ZipCrypto).unwrap();
    let result = knx_app::import_ets_project_with_password(
        &source_path,
        &store,
        knx_app::ImportOptions::default(),
        Some(&password),
        &(),
    );
    match result {
        Err(knx_app::AppError::Import(knx_etsproj::ImportFailure::Container(
            knx_etsproj::ContainerError::WrongPassword { nested_entry },
        ))) => assert_eq!(nested_entry, "P-0001.zip"),
        Err(other) => panic!("existing correct-password corruption identity changed: {other}"),
        Ok(_) => panic!("invalid encrypted DEFLATE must fail under the correct credential"),
    }
    drop(store);
    assert!(stream::disk_snapshot(dir.path()) == before);
    let reopened = open_and_migrate(&native_path).unwrap();
    assert!(load_opaque(&reopened).unwrap().is_empty());
    assert!(knx_store::load_manufacturer_refs(&reopened)
        .unwrap()
        .is_empty());
    assert!(knx_store::load_project(&reopened).is_err());
    assert!(!dir.path().join("products.sqlite").exists());
    assert_eq!(std::fs::read(source_path).unwrap(), source);
}

#[test]
fn wp16_valid_alternate_late_manufacturer_stream_installs_and_reopens_exact_bytes() {
    let dir = tempfile::tempdir().unwrap();
    let source = stream::project(stream::Layer::Outer, false, true);
    let source_path = dir.path().join("valid-manufacturer.knxproj");
    std::fs::write(&source_path, &source).unwrap();
    let native_path = dir.path().join("valid.knxdb");
    let product_path = dir.path().join("products.sqlite");
    let store = open_and_migrate(&native_path).unwrap();
    let products = knx_productdb::open_and_migrate(&product_path).unwrap();
    let imported = knx_app::import_ets_project_with(
        &source_path,
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();
    let bytes = stream::manufacturer("late-new");
    let hash = sha256_hex(&bytes);
    assert_eq!(
        knx_productdb::load_source_file(&products, &hash)
            .unwrap()
            .unwrap(),
        bytes
    );
    assert!(knx_store::load_manufacturer_refs(&store)
        .unwrap()
        .iter()
        .any(|r| r.source_path == stream::LATE_MANUFACTURER && r.sha256 == hash));
    knx_store::save_project(&store, &imported.project).unwrap();
    let expected = knx_store::project_history::NativeSnapshot::read(&store).unwrap();
    drop(store);
    drop(products);
    let reopened = open_and_migrate(&native_path).unwrap();
    assert!(knx_store::project_history::NativeSnapshot::read(&reopened).unwrap() == expected);
    let products = knx_productdb::open_and_migrate(&product_path).unwrap();
    assert_eq!(
        knx_productdb::load_source_file(&products, &hash)
            .unwrap()
            .unwrap(),
        bytes
    );
    assert_eq!(std::fs::read(source_path).unwrap(), source);
}
