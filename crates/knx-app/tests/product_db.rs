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

/// The manifest above proves the gap is *nameable*. This one proves
/// somebody actually names it.
///
/// Importing against a product database that holds no application program
/// leaves every device's product unresolved. Until T27 that fact reached
/// the user as `ExportWarning::MissingManufacturerData`; `.knxproj` export
/// is gone (ADR-0028) and the import's own report is the only surface
/// left. `EnrichmentReport::available` alone is not enough — it says the
/// database is empty, not how much of *this* project that costs — and
/// `devices_resolved`, `com_objects_enriched` and `issues` are all zero on
/// this path, which is indistinguishable from a project with no devices.
/// Uses the in-memory minimal fixture rather than the corpus, because the
/// corpus project carries real manufacturer XML: ingesting it is what
/// makes the database non-empty, so a "no application program anywhere"
/// run cannot be staged with it. The fixture's `M-0001_A-1.xml` is an
/// empty `<ManufacturerData/>`, which is exactly the shape that leaves the
/// database with nothing in it.
#[test]
fn an_empty_product_database_reports_how_many_devices_it_could_not_resolve() {
    let dir = tempfile::tempdir().unwrap();
    let project = knx_testsupport::write_minimal_knxproj(dir.path());
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();

    let imported = knx_app::import_ets_project_with(
        &project,
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();

    let enrichment = imported.enrichment.expect("a product database was supplied");
    assert!(
        !enrichment.available,
        "an empty <ManufacturerData/> ingests no application program"
    );
    // The three figures that look like a clean run, and the one that is
    // not fooled.
    assert_eq!(enrichment.devices_resolved, 0);
    assert_eq!(enrichment.com_objects_enriched, 0);
    assert_eq!(enrichment.issues, vec![]);
    assert_eq!(
        enrichment.devices_unresolved,
        imported.project.devices.iter().count(),
        "every device in the project is unresolved, and the count says so"
    );
    assert!(enrichment.devices_unresolved > 0);
}

/// The other side of the same figure: a database that *does* hold the
/// project's application programs resolves every device, so the gap is
/// zero. Without this, a `devices_unresolved` wired to a constant would
/// pass the test above.
#[test]
fn a_populated_product_database_leaves_no_unresolved_device() {
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

    let enrichment = imported.enrichment.expect("a product database was supplied");
    assert!(enrichment.available);
    assert_eq!(enrichment.devices_unresolved, 0);
    assert_eq!(
        enrichment.devices_resolved,
        imported.project.devices.iter().count()
    );
}
