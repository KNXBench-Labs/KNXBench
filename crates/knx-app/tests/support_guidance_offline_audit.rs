//! Audits offline planning with aggregate output and explicit unknown extension coverage.

use knx_app::download_support::{coverage, shipped_evidence, CoverageSummary};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[test]
#[ignore = "requires explicit permitted product corpus and output path; offline audit only"]
fn offline_support_guidance_audit_preserves_unknown_dependency_scope() {
    let root = PathBuf::from(
        std::env::var_os("KNXBENCH_SUPPORT_AUDIT_CORPUS")
            .expect("set KNXBENCH_SUPPORT_AUDIT_CORPUS explicitly"),
    );
    let output = PathBuf::from(
        std::env::var_os("KNXBENCH_SUPPORT_AUDIT_OUTPUT")
            .expect("set KNXBENCH_SUPPORT_AUDIT_OUTPUT explicitly"),
    );
    assert!(root.is_dir(), "configured permitted corpus is missing");
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let mut packages = 0usize;
    for path in knx_testsupport::walk_corpus_files(&root) {
        if path.extension().and_then(|value| value.to_str()) != Some("knxprod") {
            continue;
        }
        let bytes = std::fs::read(&path).expect("permitted input could not be read");
        let package_name = path.file_name().unwrap().to_string_lossy();
        knx_productdb::install_package_with_limits(
            &products,
            &package_name,
            &bytes,
            knx_productdb::PackageLimits::LARGE,
        )
        .unwrap_or_else(|_| panic!("permitted package {} refused installation", packages + 1));
        packages += 1;
    }
    assert!(packages > 0, "no permitted product packages inspected");
    let rows = coverage(&products, None, &shipped_evidence().unwrap())
        .expect("production coverage failed");
    let summary = CoverageSummary::of(&rows);
    assert!(summary.programs > 0, "no installed applications inspected");
    let refused: BTreeMap<&str, usize> = summary
        .unsupported
        .iter()
        .map(|(category, count)| (category.code(), *count))
        .collect();
    assert_eq!(
        summary.programs,
        summary.verified + summary.untested + refused.values().sum::<usize>()
    );
    let named_masks: BTreeMap<&str, serde_json::Value> = ["MV-0701", "MV-0705", "MV-07B0"]
        .into_iter()
        .map(|mask| {
            let (programs, plannable) = summary.by_mask.get(mask).copied().unwrap_or_default();
            (mask, json!({"programs": programs, "plannable": plannable}))
        })
        .collect();
    let document = json!({
        "kind": "knxbench-support-guidance-offline-audit",
        "format": 1,
        "scope": "installed-product-defaults-no-links",
        "packages": packages,
        "programs": summary.programs,
        "verified": summary.verified,
        "untested": summary.untested,
        "refusedByCategory": refused,
        "selectedMaskAggregates": named_masks,
        "executableDependencies": {
            "knownAbsent": 0,
            "knownRequired": 0,
            "unassessed": summary.programs,
            "reason": "no reviewed dependency census; filename absence is not evidence"
        },
        "newHardwareVerification": false,
        "concreteProjectConfigurationCovered": false
    });
    let encoded = serde_json::to_string_pretty(&document).unwrap() + "\n";
    let temporary = output.with_extension("tmp");
    std::fs::write(&temporary, encoded.as_bytes()).expect("audit output could not be written");
    std::fs::rename(&temporary, &output).expect("audit output could not be published atomically");
    let reread: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&output).unwrap()).unwrap();
    assert_eq!(reread, document);
    // No per-item identifiers, source hashes, raw errors or private input paths leave here.
    assert_eq!(
        document["executableDependencies"]["unassessed"],
        document["programs"]
    );
}
