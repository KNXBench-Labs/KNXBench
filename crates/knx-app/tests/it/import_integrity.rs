//! Independent synthetic source/report/persistence witnesses.

#[test]
#[ignore = "requires an explicitly configured private source; no private fixture is distributed"]
fn configured_private_source_roundtrips_without_changing_retained_bytes() {
    let source = std::env::var_os("KNXBENCH_PRIVATE_IMPORT_SOURCE")
        .expect("explicit private source required");
    let source = std::path::PathBuf::from(source);
    let original = std::fs::read(&source).expect("private source available");
    let original_digest = knx_etsproj::opaque::sha256_hex(&original);
    let dir = tempfile::tempdir().unwrap();
    let native = dir.path().join("private.knxdb");
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let conn = knx_store::open_and_migrate(&native).unwrap();
    let first = knx_app::import_ets_project_with(
        &source,
        &conn,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .expect("production import must reach the model");
    assert!(
        first.report.error_count() == 0,
        "error-level diagnostics require classification"
    );
    knx_store::save_project(&conn, &first.project).unwrap();
    let opaque = knx_store::load_opaque(&conn).unwrap();
    let manifest = knx_store::load_manufacturer_refs(&conn).unwrap();
    assert!(opaque
        .iter()
        .all(|entry| entry.sha256 == knx_etsproj::opaque::sha256_hex(&entry.bytes)));
    assert!(knx_productdb::verify(&products).unwrap().is_empty());
    for entry in &manifest {
        let bytes = knx_productdb::load_source_file(&products, &entry.sha256)
            .unwrap()
            .expect("manifest source available");
        assert!(knx_productdb::sha256_hex(&bytes) == entry.sha256);
    }
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&native).unwrap();
    assert!(knx_store::load_project(&conn).unwrap() == first.project);
    assert!(knx_store::load_opaque(&conn).unwrap() == opaque);
    knx_store::save_project(&conn, &first.project).unwrap();
    assert!(knx_store::load_opaque(&conn).unwrap() == opaque);
    let retry_conn = knx_store::open_and_migrate(&dir.path().join("retry.knxdb")).unwrap();
    let retry = knx_app::import_ets_project_with(
        &source,
        &retry_conn,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();
    assert!(retry.manufacturer_ingested == 0);
    assert!(retry.report.unknown == first.report.unknown);
    assert!(retry.report.errors == first.report.errors);
    assert!(knx_etsproj::opaque::sha256_hex(&std::fs::read(&source).unwrap()) == original_digest);
}
use knx_testsupport::zip_with_entries;
#[test]
fn corrupt_cached_diagnostics_are_refused_before_new_product_writes() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let bad = b"synthetic cached opaque payload";
    let hash = knx_productdb::sha256_hex(bad);
    knx_productdb::ingest_file(&products, "M-0001/Baggages/bad.dat", bad).unwrap();
    products.execute("INSERT INTO ingest_unknown (source_sha256,program_id,xpath,kind,name,occurrences,sample) VALUES (?1,NULL,'/KNX','FutureKind','Synthetic',1,NULL)", [&hash]).unwrap();
    let mut template =
        knx_etsproj::Container::open(knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let topology = template.read("P-0001/0.xml").unwrap();
    let info = template.read("P-0001/Project.xml").unwrap();
    let input = dir.path().join("synthetic.knxproj");
    let hardware = br#"<KNX xmlns="http://knx.org/xml/project/23"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-2" Name="Synthetic"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    std::fs::write(
        &input,
        zip_with_entries(&[
            ("P-0001.signature", b""),
            ("P-0001/0.xml", &topology),
            ("P-0001/Project.xml", &info),
            ("M-0001/Hardware.xml", hardware),
            ("M-0001/Baggages/bad.dat", bad),
        ]),
    )
    .unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("native.knxdb")).unwrap();
    assert!(knx_app::import_ets_project_with(
        &input,
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products)
        }
    )
    .is_err());
    assert_eq!(
        products
            .query_row("SELECT count(*) FROM source_file", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(knx_store::load_opaque(&store).unwrap().is_empty());
}

#[test]
fn disabled_manufacturer_installation_is_unavailable_not_measured_zero() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("synthetic.knxproj");
    std::fs::write(&input, knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("native.knxdb")).unwrap();
    let imported = knx_app::import_ets_project(&input, &store).unwrap();
    assert!(imported
        .report
        .unsupported
        .iter()
        .any(|u| u.what == "manufacturer semantic diagnostics"
            && u.consequence.contains("unavailable")));
}

#[test]
fn manufacturer_findings_survive_first_import_and_idempotent_retry() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("synthetic.knxproj");
    let mut template =
        knx_etsproj::Container::open(knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let topology = template.read("P-0001/0.xml").unwrap();
    let info = template.read("P-0001/Project.xml").unwrap();
    let hardware = br#"<KNX xmlns="http://knx.org/xml/project/23"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="M-0001_H-1" Name="Synthetic" Fancy="synthetic-marker"/><Hardware Id="M-0001_H-1" Name="Synthetic second"/></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    let master = br#"<KNX xmlns="http://knx.org/xml/project/11"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Synthetic" FutureMaster="synthetic"/></Manufacturers></MasterData></KNX>"#;
    std::fs::write(
        &input,
        zip_with_entries(&[
            ("P-0001.signature", b""),
            ("P-0001/0.xml", &topology),
            ("P-0001/Project.xml", &info),
            ("knx_master.xml", master),
            ("M-0001/Hardware.xml", hardware),
            (
                "M-0001/Baggages/fake.png",
                b"BM\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",
            ),
        ]),
    )
    .unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let mut reports = Vec::new();
    for n in 0..2 {
        let store = knx_store::open_and_migrate(&dir.path().join(format!("p{n}.knxdb"))).unwrap();
        let imported = knx_app::import_ets_project_with(
            &input,
            &store,
            knx_app::ImportOptions {
                product_db: Some(&products),
            },
        )
        .unwrap();
        let finding = imported
            .report
            .unknown
            .iter()
            .find(|u| u.name == "Fancy")
            .expect("manufacturer unknown must reach project report");
        assert_eq!(finding.source_path, "M-0001/Hardware.xml");
        assert_eq!(finding.occurrences, 1);
        assert!(imported
            .report
            .unknown
            .iter()
            .any(|u| u.name == "FutureMaster" && u.source_path == "knx_master.xml"));
        assert!(imported
            .report
            .errors
            .iter()
            .any(|e| e.stage == "manufacturer" && e.detail.contains("IdConflict")));
        let report_json: serde_json::Value =
            serde_json::from_str(&imported.report.to_json()).unwrap();
        let conflict = report_json["errors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["stage"] == "manufacturer")
            .unwrap();
        assert_eq!(conflict["source_path"], "M-0001/Hardware.xml");
        assert_eq!(conflict["xpath"], "");
        let baggage = imported
            .report
            .unsupported
            .iter()
            .find(|u| u.what.ends_with("fake.png"))
            .unwrap();
        assert!(baggage.consequence.contains("bmp"));
        assert!(!baggage.consequence.contains("plugin code"));
        let opaque_summary = imported
            .report
            .opaque
            .iter()
            .find(|e| e.source_path.ends_with("fake.png"))
            .unwrap();
        assert!(opaque_summary.reason.contains("bmp"));
        assert!(!opaque_summary.reason.contains("plugin binary"));
        let bytes_store =
            knx_store::open_and_migrate(&dir.path().join(format!("bytes{n}.knxdb"))).unwrap();
        let by_bytes = knx_app::import_ets_project_bytes(
            std::fs::read(&input).unwrap(),
            "synthetic.knxproj",
            &bytes_store,
            knx_app::ImportOptions {
                product_db: Some(&products),
            },
        )
        .unwrap();
        assert_eq!(by_bytes.report.opaque, imported.report.opaque);
        assert_eq!(by_bytes.report.unsupported, imported.report.unsupported);
        knx_store::save_project(&store, &imported.project).unwrap();
        let opaque = knx_store::load_opaque(&store).unwrap();
        drop(store);
        let reopened =
            knx_store::open_and_migrate(&dir.path().join(format!("p{n}.knxdb"))).unwrap();
        assert!(knx_store::load_project(&reopened).unwrap() == imported.project);
        assert_eq!(knx_store::load_opaque(&reopened).unwrap(), opaque);
        knx_store::save_project(&reopened, &imported.project).unwrap();
        assert_eq!(knx_store::load_opaque(&reopened).unwrap(), opaque);
        reports.push((imported.report.unknown, imported.report.errors));
    }
    assert_eq!(reports[0], reports[1]);
}
