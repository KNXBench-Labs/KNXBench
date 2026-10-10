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
