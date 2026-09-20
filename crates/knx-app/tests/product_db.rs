use std::path::PathBuf;

fn reference_project_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

#[test]
fn importing_with_a_product_db_ingests_manufacturer_files_and_keeps_them_out_of_the_opaque_store() {
    if !reference_project_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();

    let imported = knx_app::import_ets_project_with(
        &reference_project_path(),
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();

    assert!(imported.manufacturer_ingested > 0);
    let manifest = knx_store::load_manufacturer_refs(&store).unwrap();
    assert_eq!(
        manifest.len(),
        imported.manufacturer_ingested + imported.manufacturer_skipped
    );
    let opaque = knx_store::load_opaque(&store).unwrap();
    assert!(
        opaque
            .iter()
            .all(|e| e.kind != "ManufacturerData" && e.kind != "Baggage"),
        "manufacturer data no longer lives in the project"
    );
}

#[test]
fn a_second_import_into_the_same_product_db_skips_every_file() {
    if !reference_project_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let first_store = knx_store::open_and_migrate(&dir.path().join("a.knxdb")).unwrap();
    let second_store = knx_store::open_and_migrate(&dir.path().join("b.knxdb")).unwrap();

    let first = knx_app::import_ets_project_with(
        &reference_project_path(),
        &first_store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();
    let second = knx_app::import_ets_project_with(
        &reference_project_path(),
        &second_store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();

    assert!(first.manufacturer_ingested > 0);
    assert_eq!(second.manufacturer_ingested, 0);
    assert_eq!(second.manufacturer_skipped, first.manufacturer_ingested);
}

#[test]
fn a_project_names_its_manufacturer_gap_when_the_product_database_is_gone() {
    if !reference_project_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products_path = dir.path().join("products.sqlite");
    {
        let products = knx_productdb::open_and_migrate(&products_path).unwrap();
        knx_app::import_ets_project_with(
            &reference_project_path(),
            &store,
            knx_app::ImportOptions {
                product_db: Some(&products),
            },
        )
        .unwrap();
    }
    std::fs::remove_file(&products_path).unwrap();

    // The manifest is what makes the gap nameable: every manufacturer file
    // the project carried is still listed by path and content hash, so a
    // build with no product database can say exactly which files it cannot
    // reach rather than reporting a smaller project (CLAUDE.md: never
    // silently discard information).
    let manifest = knx_store::load_manufacturer_refs(&store).unwrap();
    assert!(!manifest.is_empty());
    let fresh = knx_productdb::open_and_migrate(&dir.path().join("empty.sqlite")).unwrap();
    for reference in &manifest {
        assert!(!reference.source_path.is_empty());
        assert!(
            knx_productdb::load_source_file(&fresh, &reference.sha256)
                .unwrap()
                .is_none(),
            "an empty product database resolves nothing, so every manifest entry is a named gap"
        );
    }
}
