//! Round-trips the reference project through knx-desktop's own `.knxdb`
//! save/load path (distinct from the ETS `.knxproj` import path covered by
//! `open_reference_project.rs`): import once, save to a fresh `.knxdb` file,
//! reopen it, and confirm the projection carries the same golden counts.

use std::path::PathBuf;

fn workspace_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate lives at <root>/apps/knx-server")
        .to_path_buf()
}

fn reference_ets4_path() -> PathBuf {
    workspace_root().join("OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj")
}

#[test]
fn saving_then_reopening_a_native_project_round_trips_the_golden_counts() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let imported = knx_app::import_ets_project_with(
        &reference_ets4_path(),
        &conn,
        knx_app::ImportOptions::default(),
    )
    .unwrap();

    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roundtrip.knxdb");

    knx_server::save_project_as_impl(&db_path, &imported.project).unwrap();
    let tree = knx_server::open_native_project_impl(&db_path).unwrap();

    // A native load has no `ImportReport` — nothing was reinterpreted from
    // an external format, so both counts are genuinely zero, not merely
    // unmeasured.
    assert_eq!(tree.errors, 0);
    assert_eq!(tree.warnings, 0);

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
