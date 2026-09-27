//! Confirms the projection did not drop or duplicate anything the importer
//! produced, against the same reference project and the same golden counts
//! Session 3's import golden test already established
//! (crates/knx-etsproj/tests/golden_reference_project.rs).

use std::path::PathBuf;

fn reference_ets4_path() -> PathBuf {
    knx_testsupport::reference_ets4_path()
}

#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn opening_the_reference_project_yields_the_measured_counts() {
    assert!(
        reference_ets4_path().exists(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    let tree = knx_server::open_project_impl(&reference_ets4_path()).unwrap();

    // The reference project's golden import has no real errors: `warnings`
    // is non-zero only because of the two documented vendor-baggage
    // capability gaps (see knx-etsproj's own
    // `vendor_baggage_is_reported_as_unsupported` test) — a genuine
    // capability gap, not a data loss (`ImportReport::has_losses()`
    // deliberately excludes it), so it must never surface as `errors`.
    assert_eq!(tree.errors, 0);
    assert_eq!(tree.warnings, 2);

    assert_eq!(tree.installations.len(), 1);
    let inst = &tree.installations[0];

    assert_eq!(inst.topology.len(), 1); // one area
    assert_eq!(inst.topology[0].lines.len(), 1);

    let device_count: usize = inst
        .topology
        .iter()
        .flat_map(|a| a.lines.iter())
        .map(|l| l.devices.len())
        .sum();
    assert_eq!(device_count, 35); // 35 on the line
    assert_eq!(inst.unassigned.len(), 1); // plus the one unassigned device

    assert_eq!(count_buildings(&inst.buildings), 22);
}

fn count_buildings(nodes: &[knx_projection::BuildingNode]) -> usize {
    nodes.iter().map(|n| 1 + count_buildings(&n.children)).sum()
}
