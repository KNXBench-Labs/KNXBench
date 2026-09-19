//! Proves an observed import announces the real stages, in order, across both crates.
//!
//! The point of these tests is the thing ADR-0023 says a progress
//! indicator may not do: invent. Every assertion below is against the
//! sequence an actual import emitted, over a fixture small enough that
//! each expected value can be read off `knx_testsupport`'s source — so a
//! stage that stops being emitted, or starts being emitted in the wrong
//! place, fails here rather than in somebody's browser.

use knx_app::{import_ets_project_observed, ImportOptions, LoadObserver, LoadStage};
use knx_etsproj::ImportStage;
use knx_store::{open_and_migrate, Connection};
use std::sync::Mutex;

#[derive(Default)]
struct Recorder {
    stages: Mutex<Vec<LoadStage>>,
    items: Mutex<Vec<(LoadStage, u64, u64)>>,
}

impl LoadObserver for Recorder {
    fn stage(&self, stage: LoadStage) {
        self.stages.lock().unwrap().push(stage);
    }

    fn items(&self, completed: u64, total: u64) {
        let current = *self
            .stages
            .lock()
            .unwrap()
            .last()
            .expect("items() is only ever reported inside a stage");
        self.items.lock().unwrap().push((current, completed, total));
    }
}

fn migrated_store(dir: &std::path::Path) -> Connection {
    open_and_migrate(&dir.join("p.knxdb")).unwrap()
}

#[test]
fn an_import_without_a_product_database_reports_parse_stages_then_the_insert() {
    let dir = tempfile::tempdir().unwrap();
    let conn = migrated_store(dir.path());
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let recorder = Recorder::default();

    import_ets_project_observed(&path, &conn, ImportOptions::default(), &recorder).unwrap();

    assert_eq!(
        *recorder.stages.lock().unwrap(),
        vec![
            LoadStage::Parse(ImportStage::OpenContainer),
            LoadStage::Parse(ImportStage::DetectSchema),
            LoadStage::Parse(ImportStage::ParseTopology),
            LoadStage::Parse(ImportStage::ParseProjectInfo),
            LoadStage::Parse(ImportStage::Validate),
            LoadStage::Parse(ImportStage::Map),
            LoadStage::Parse(ImportStage::InferDatapointTypes),
            LoadStage::Parse(ImportStage::CollectContainerEntries),
            LoadStage::PersistOpaque,
        ],
        "no product database means no ingest and no enrichment — and the \
         stage list has to say so rather than report work nobody did"
    );
}

#[test]
fn an_import_with_a_product_database_reports_the_ingest_and_enrichment_stages_too() {
    let dir = tempfile::tempdir().unwrap();
    let conn = migrated_store(dir.path());
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.knxdb")).unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let recorder = Recorder::default();

    import_ets_project_observed(
        &path,
        &conn,
        ImportOptions {
            product_db: Some(&products),
        },
        &recorder,
    )
    .unwrap();

    let stages = recorder.stages.lock().unwrap().clone();
    assert_eq!(
        stages.iter().skip(8).copied().collect::<Vec<_>>(),
        vec![
            LoadStage::IngestManufacturerData,
            LoadStage::IngestMasterData,
            LoadStage::EnrichFromProductDatabase,
            LoadStage::PersistOpaque,
        ]
    );
    assert_eq!(
        stages[0],
        LoadStage::Parse(ImportStage::OpenContainer),
        "the parse stages still come first, unchanged"
    );
}

#[test]
fn every_reported_count_is_a_real_count_of_something_that_happened() {
    let dir = tempfile::tempdir().unwrap();
    let conn = migrated_store(dir.path());
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.knxdb")).unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let recorder = Recorder::default();

    let imported = import_ets_project_observed(
        &path,
        &conn,
        ImportOptions {
            product_db: Some(&products),
        },
        &recorder,
    )
    .unwrap();

    let items = recorder.items.lock().unwrap().clone();
    // Exactly two stages in the whole pipeline have a total known before
    // their loop starts. If a third ever appears, this assertion is the
    // place to argue for it — with the measurement that justifies it.
    let counted: Vec<LoadStage> = {
        let mut seen: Vec<LoadStage> = Vec::new();
        for (stage, ..) in &items {
            if !seen.contains(stage) {
                seen.push(*stage);
            }
        }
        seen
    };
    assert_eq!(
        counted,
        vec![
            LoadStage::Parse(ImportStage::CollectContainerEntries),
            LoadStage::IngestManufacturerData,
        ]
    );

    for (stage, completed, total) in &items {
        assert!(
            completed <= total && *total > 0,
            "{stage:?} reported {completed}/{total}, which is not a measurement"
        );
    }

    let manufacturer_counts: Vec<(u64, u64)> = items
        .iter()
        .filter(|(stage, ..)| *stage == LoadStage::IngestManufacturerData)
        .map(|(_, completed, total)| (*completed, *total))
        .collect();
    // The fixture carries one `M-0001/*.xml` and one baggage file; both
    // count as manufacturer files, and both were ingested or skipped.
    assert_eq!(manufacturer_counts, vec![(1, 2), (2, 2)]);
    assert_eq!(
        imported.manufacturer_ingested + imported.manufacturer_skipped,
        2,
        "the reported total is the number of files the import really handled"
    );
}

#[test]
fn a_failed_import_stops_announcing_stages_where_it_failed() {
    let dir = tempfile::tempdir().unwrap();
    let conn = migrated_store(dir.path());
    let path = dir.path().join("not-an-archive.knxproj");
    std::fs::write(&path, b"this is not a zip file").unwrap();
    let recorder = Recorder::default();

    let failure = import_ets_project_observed(&path, &conn, ImportOptions::default(), &recorder);

    assert!(failure.is_err());
    assert_eq!(
        *recorder.stages.lock().unwrap(),
        vec![LoadStage::Parse(ImportStage::OpenContainer)],
        "a stage is announced before its work, so the last one announced \
         names the stage that failed"
    );
}
