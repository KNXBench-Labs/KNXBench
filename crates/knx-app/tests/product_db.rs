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
fn export_is_byte_identical_with_and_without_the_product_database() {
    if !reference_project_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    // The proof that routing manufacturer data through the product
    // database changes nothing about what we write back (spec §9).
    let dir = tempfile::tempdir().unwrap();
    let with_store = knx_store::open_and_migrate(&dir.path().join("with.knxdb")).unwrap();
    let without_store = knx_store::open_and_migrate(&dir.path().join("without.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();

    let with = knx_app::import_ets_project_with(
        &reference_project_path(),
        &with_store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();
    let without = knx_app::import_ets_project_with(
        &reference_project_path(),
        &without_store,
        knx_app::ImportOptions { product_db: None },
    )
    .unwrap();

    let a = knx_app::export_ets_project(&with.project, &with_store, Some(&products)).unwrap();
    let b = knx_app::export_ets_project(&without.project, &without_store, None).unwrap();
    assert_eq!(a.bytes, b.bytes);
}

#[test]
fn a_project_opens_and_names_its_gap_when_the_product_database_is_gone() {
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

    // Exporting without the product database: every manifest entry is
    // named as missing, and the export still happens.
    let outcome = knx_app::export_ets_project(
        &knx_app::import_ets_project_with(
            &reference_project_path(),
            &knx_store::open_and_migrate(&dir.path().join("q.knxdb")).unwrap(),
            knx_app::ImportOptions { product_db: None },
        )
        .unwrap()
        .project,
        &store,
        None,
    )
    .unwrap();

    let missing = outcome
        .warnings
        .iter()
        .filter(|w| {
            matches!(
                w,
                knx_etsproj::export::ExportWarning::MissingManufacturerData { .. }
            )
        })
        .count();
    assert_eq!(
        missing,
        knx_store::load_manufacturer_refs(&store).unwrap().len()
    );
    assert!(!outcome.bytes.is_empty());
}
