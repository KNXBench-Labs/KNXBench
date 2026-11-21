//! Offline partial-group-addresses plan probe for M-0006_A-E052-12-3406 (issue #49 evidence).

use knx_app::download_support::offline_download;
use knx_core::commissioning::partial_memory_download::{derive_partial_plan, PartialDownloadParts};

const PROGRAM_ID: &str = "M-0006_A-E052-12-3406";

/// Member of the private example project that holds the hash-pinned program XML.
const PROJECT_FILE: &str = "private example 001.knxproj";
const PROGRAM_MEMBER: &str = "M-0006/M-0006_A-E052-12-3406.xml";

/// The program XML is private; it is read from the corpus project, never embedded.
#[test]
#[ignore = "requires OriginalData private DemoProjects corpus (issue #49 program XML); no hardware"]
fn the_49_report_plan_reproduces_offline() {
    let root = knx_testsupport::workspace_root().join("OriginalData/DemoProjects");
    let project = knx_testsupport::find_corpus_file(&root, PROJECT_FILE)
        .expect("private issue #49 project must exist for this ignored test");
    let file = std::fs::File::open(project).expect("readable private project");
    let mut archive = zip::ZipArchive::new(file).expect("project container");
    let mut xml = Vec::new();
    std::io::Read::read_to_end(
        &mut archive.by_name(PROGRAM_MEMBER).expect("program XML member"),
        &mut xml,
    )
    .expect("read program xml");
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, PROGRAM_MEMBER, &xml).unwrap();
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
