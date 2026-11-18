//! Offline partial-group-addresses plan probe for M-0006_A-E052-12-3406 (issue #49 evidence).

use knx_app::download_support::offline_download;
use knx_core::commissioning::partial_memory_download::{derive_partial_plan, PartialDownloadParts};

const PROGRAM_ID: &str = "M-0006_A-E052-12-3406";

#[test]
fn the_49_report_plan_reproduces_offline() {
    let xml_path = std::env::var("C49_PROGRAM_XML").expect("C49_PROGRAM_XML path");
    let xml = std::fs::read(&xml_path).expect("read program xml");
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-0006/M-0006_A-E052-12-3406.xml", &xml).unwrap();
    let identity = knx_productdb::query::program_identity(&conn, PROGRAM_ID)
        .unwrap()
        .expect("program identity");
    assert_eq!(
        identity.source_sha256,
        "3cd53544f5cdf1947be75a376352bcc23d8131220a5f14326c76b5ac00c0cd5b"
    );
    let (complete, inferences) =
        offline_download(&conn, PROGRAM_ID).expect("offline complete plan");
    assert!(inferences.is_empty(), "{inferences:?}");
    let parts = PartialDownloadParts {
        parameters: false,
        group_addresses: true,
    };
    let partial = derive_partial_plan(&complete, parts).expect("partial plan");
    assert_eq!(
        partial.plan.steps.len(),
        24,
        "steps: {}",
        partial.plan.steps.len()
    );
    assert_eq!(partial.plan.data_octets(), 1020, "octets");
    assert!(
        partial.ignored_writes.is_empty(),
        "{:?}",
        partial.ignored_writes
    );
    println!(
        "C49-OK complete steps={} octets={} partial-ga steps={} octets={}",
        complete.steps.len(),
        complete.data_octets(),
        partial.plan.steps.len(),
        partial.plan.data_octets()
    );
}
