//! Full private-corpus AP1 diagnostics never promote existing download readiness.
use std::collections::BTreeMap;
use std::path::PathBuf;

use knx_app::download_support::{coverage, shipped_evidence, CoverageSummary};
use knx_productdb::procedure_resolution::resolve_package_ap1;

#[test]
#[ignore = "requires the complete private product corpus; explicit opt-in only"]
fn full_corpus_resolution_preserves_sources_and_live_readiness() {
    let root = std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .expect("set KNXBENCH_PRODUCT_CORPUS to the private original corpus");
    assert!(root.is_dir());
    let paths: Vec<_> = knx_testsupport::walk_corpus_files(&root)
        .into_iter()
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("knxprod"))
        .collect();
    assert_eq!(paths.len(), 105);
    let original: Vec<_> = paths
        .iter()
        .map(|p| knx_productdb::sha256_hex(&std::fs::read(p).unwrap()))
        .collect();
    let temp = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&temp.path().join("products.sqlite")).unwrap();
    let mut packages = Vec::new();
    for path in &paths {
        let bytes = std::fs::read(path).unwrap();
        let report = knx_productdb::install_package_with_limits(
            &conn,
            "private-corpus.knxprod",
            &bytes,
            knx_productdb::PackageLimits::LARGE,
        )
        .unwrap();
        packages.push(report.sha256);
    }
    let ids:Vec<String>=conn.prepare("SELECT id FROM application_program WHERE mask_version='MV-07B0' AND load_procedure_style IN ('MergedProcedure','DefaultProcedure') ORDER BY id").unwrap().query_map([],|r|r.get(0)).unwrap().collect::<Result<_,_>>().unwrap();
    let before = conn.total_changes();
    let mut statuses = BTreeMap::<String, usize>::new();
    let mut issues = BTreeMap::<String, usize>::new();
    let mut results = 0;
    for package in packages {
        for id in &ids {
            let owned:bool=conn.query_row("SELECT EXISTS(SELECT 1 FROM application_program a JOIN package_member m ON m.source_sha256=a.source_sha256 WHERE a.id=?1 AND m.package_sha256=?2)",[id,&package],|r|r.get(0)).unwrap();
            if !owned {
                continue;
            }
            let result = resolve_package_ap1(&conn, &package, id).unwrap().unwrap();
            assert!(!result.executable);
            if result.status == "partial" {
                assert!(!result.steps.is_empty());
                assert_eq!(result.sources.len(), 2);
            }
            assert!(result.steps.iter().all(|s| result
                .sources
                .iter()
                .any(|src| src.source == s.origin.source && src.sha256 == s.origin.sha256)));
            *statuses.entry(result.status).or_default() += 1;
            for issue in result.issues {
                *issues.entry(issue.code).or_default() += 1;
            }
            results += 1;
        }
    }
    assert_eq!(ids.len(), 75);
    assert_eq!(results, 75);
    assert_eq!(
        statuses,
        BTreeMap::from([("partial".into(), 69), ("unavailable".into(), 6)])
    );
    assert_eq!(
        issues,
        BTreeMap::from([
            ("empty-fragment".into(), 1),
            ("missing-mandatory-merge".into(), 50),
            ("nested-or-unresolved-merge".into(), 51),
            ("source-byte-limit".into(), 6),
            ("uninterpreted-step".into(), 295),
            ("unused-fragment".into(), 1),
        ])
    );
    assert_eq!(conn.total_changes(), before);
    let rows = coverage(&conn, None, &shipped_evidence().unwrap()).unwrap();
    let summary = CoverageSummary::of(&rows);
    assert_eq!(summary.programs, 467);
    assert_eq!((summary.verified, summary.untested), (1, 98));
    for (path, sha) in paths.iter().zip(&original) {
        assert_eq!(
            &knx_productdb::sha256_hex(&std::fs::read(path).unwrap()),
            sha
        );
    }
    eprintln!("offline_resolution_aggregate: programs={} source_bound_results={results} statuses={statuses:?} issues={issues:?}; all105_originals_unchanged=true; existing_readiness=1_verified_98_untested",ids.len());
    let aggregate = serde_json::json!({"programs":ids.len(),"sourceBoundResults":results,"statuses":statuses,"issues":issues,"originalsUnchanged":true,"existingPrograms":summary.programs,"existingVerified":summary.verified,"existingUntested":summary.untested});
    if let Some(path) = std::env::var_os("KNXBENCH_PROCEDURE_CORPUS_REPORT") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .unwrap();
        file.write_all(&serde_json::to_vec_pretty(&aggregate).unwrap())
            .unwrap();
    }
    assert!(results > 0);
}
