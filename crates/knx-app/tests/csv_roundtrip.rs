//! A corpus-gated round trip: import the reference `.knxproj`, export its
//! group addresses, re-parse that text, and plan importing it back against
//! the same project. The plan should be entirely `unchanged` — this is the
//! real proof that the writer and the reader agree with each other,
//! including on names containing commas, quotes, and umlauts, which a
//! hand-built fixture cannot exercise realistically.
//!
//! Lives here, not in `knx-csv/tests/`, because it needs `knx-etsproj` to
//! produce a real `Project` to feed the writer, and `knx-csv` itself must
//! never depend on `knx-etsproj` (`xtask check-layering` enforces this, and
//! `cargo metadata`'s resolve graph does not distinguish `[dependencies]`
//! from `[dev-dependencies]`, so even a dev-only edge there trips it).
//! `knx-app` is deliberately the one crate that sees both sides
//! (`xtask/src/main.rs`'s own comment on why it alone reaches both
//! `knx-etsproj` and `knx-store`), so its `tests/` is where a test that
//! needs both `knx-csv` and `knx-etsproj` belongs.
//!
//! `knx-app`'s tests cannot reach `knx-etsproj`'s `tests/support` module —
//! a crate's integration tests are private to it — so this repeats the
//! four-line path helper the same way `import_service.rs` does.
//!
//! Gated by the standard `corpus_available()` pattern: `OriginalData/` is
//! the maintainer's own real KNX installation, gitignored and local-only,
//! so CI (and any contributor without a copy) skips this test rather than
//! failing it.

use std::path::{Path, PathBuf};

fn reference_ets4_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj")
}

#[test]
fn exporting_and_replanning_the_reference_project_is_entirely_unchanged() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }

    let outcome = knx_etsproj::import_knxproj(&reference_ets4_path())
        .expect("the reference project is a known-good fixture");
    let project = outcome.project;

    let export = knx_csv::export_group_addresses(&project);
    assert!(
        export.warnings.is_empty(),
        "expected no export warnings against the reference project, got {:?}",
        export.warnings
    );

    let parsed = knx_csv::parse_group_addresses(&export.text, project.info.group_address_style);
    assert!(
        parsed.problems.is_empty(),
        "expected the writer's own output to re-parse cleanly, got {:?}",
        parsed.problems
    );

    let installation = project
        .installations
        .first()
        .expect("the reference project has an installation");
    assert_eq!(
        parsed.rows.len(),
        installation.group_addresses.len(),
        "expected every group address to round-trip through the CSV text"
    );

    let plan = knx_csv::plan_import(&project, &parsed);

    assert_eq!(plan.command, None, "expected nothing to apply");
    assert!(
        plan.report.problems.is_empty(),
        "expected no problems, got {:?}",
        plan.report.problems
    );
    assert_eq!(plan.report.created, 0);
    assert_eq!(plan.report.updated, 0);
    assert_eq!(
        plan.report.unchanged,
        installation.group_addresses.len(),
        "expected every address to plan as unchanged"
    );
}
