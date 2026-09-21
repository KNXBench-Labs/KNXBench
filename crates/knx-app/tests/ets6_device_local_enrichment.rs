//! Pins product-database enrichment of ETS 6's device-local communication-object ids.
//!
//! All three corpus projects, so a fix for one cannot move another.
//!
//! The import-side half of this defect lives in
//! `knx-etsproj/tests/device_local_com_object_refs.rs`. This is the other
//! half: even once the mapper reads `O-<n>_R-<m>` correctly, the id is not
//! the fully-qualified `ComObjectRef/@Id` the database is keyed by, so
//! every lookup missed. The schema-23 reference project used to enrich 0
//! of its 867 communication objects and report 867
//! `ComObjectRefMissing` issues; it now enriches all 867 and reports none.
//!
//! Gated by the standard `corpus_available()` pattern: `OriginalData/` is
//! the maintainer's own installation, gitignored, so CI and every
//! contributor without a copy skip rather than fail.

use knx_productdb::EnrichmentIssue;

/// Imports one corpus project with a freshly built product database and
/// hands back its enrichment report. The database is populated from the
/// project's own manufacturer files, exactly as an ordinary import does.
fn enrichment_of(path: &std::path::Path) -> knx_productdb::EnrichmentReport {
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let imported = knx_app::import_ets_project_with(
        path,
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .unwrap();
    imported
        .enrichment
        .expect("a product database was supplied, so there is a report")
}

/// The headline: 867 of 867, up from 0 of 867. The 107 `AmbiguousDpt`
/// entries left over are the pre-existing, deliberate refusal to pick one
/// datapoint type out of a list of alternatives (RESEARCH §4.2) — a
/// reported non-guess, not a lookup failure, and they only became visible
/// once the lookups started hitting.
#[test]
fn the_ets6_project_enriches_every_device_local_com_object() {
    if !knx_testsupport::corpus_available() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let report = enrichment_of(&knx_testsupport::reference_ets6_path());

    assert!(report.available);
    assert_eq!(report.devices_resolved, 35);
    assert_eq!(report.devices_unresolved, 0);
    assert_eq!(
        report.com_objects_enriched, 867,
        "was 0 before the device-local lookup id existed"
    );

    let missing = report
        .issues
        .iter()
        .filter(|i| matches!(i, EnrichmentIssue::ComObjectRefMissing { .. }))
        .count();
    assert_eq!(missing, 0, "was 867 — one per communication object");

    let ambiguous = report
        .issues
        .iter()
        .filter(|i| matches!(i, EnrichmentIssue::AmbiguousDpt { .. }))
        .count();
    assert_eq!(ambiguous, 107, "reported alternatives, never guessed at");
    assert_eq!(report.issues.len(), ambiguous);
}

/// The schema-21 project goes through the module hop instead, and must not
/// have moved: the device-local branch is reached only for an id that is
/// neither module-based nor already fully qualified.
#[test]
fn the_schema21_project_still_enriches_through_the_module_hop() {
    if !knx_testsupport::corpus_available() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let report = enrichment_of(&knx_testsupport::reference_kv_schema21_path());
    assert_eq!(report.devices_resolved, 4);
    assert_eq!(report.com_objects_enriched, 75);
    assert_eq!(report.issues, vec![]);
}
