//! The golden regression test for the whole import pipeline.
//!
//! Every count here comes from ROADMAP Session 3 and RESEARCH §3, and each
//! was re-measured against the reference project directly (see the Session
//! 3 SDD ledger's per-task notes) rather than assumed from the plan alone.
//! If any stage starts dropping or miscounting entities, one of these
//! numbers moves — that is what this test exists to catch.

use std::path::PathBuf;

use knx_core::{BuildingPartType, Direction, Override};
use knx_etsproj::import_knxproj;

fn workspace_root() -> PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .expect("crate lives at <root>/crates/<name>")
        .to_path_buf()
}

fn reference_ets4_path() -> PathBuf {
    workspace_root().join("OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj")
}

#[test]
fn the_reference_project_imports_with_the_measured_counts() {
    let out = import_knxproj(&reference_ets4_path()).unwrap();
    let p = &out.project;
    let inst = &p.installations[0];

    assert_eq!(p.installations.len(), 1);
    assert_eq!(inst.topology.areas.len(), 1);
    assert_eq!(inst.topology.lines.len(), 1);

    // 35 devices on the line plus the one unassigned device xknxproject loses.
    assert_eq!(p.devices.iter().count(), 36);
    assert_eq!(inst.topology.unassigned.len(), 1);

    assert_eq!(inst.group_ranges.len(), 35);
    assert_eq!(
        inst.group_ranges
            .iter()
            .filter(|r| r.parent.is_none())
            .count(),
        7
    );
    assert_eq!(inst.group_addresses.len(), 514);
    assert_eq!(inst.group_addresses.iter().filter(|g| g.central).count(), 2);
    assert_eq!(
        inst.group_addresses.iter().filter(|g| g.unfiltered).count(),
        2
    );

    assert_eq!(inst.buildings.len(), 22);
    assert_eq!(
        inst.buildings
            .iter()
            .filter(|b| b.kind == BuildingPartType::Room)
            .count(),
        14
    );
    assert_eq!(
        inst.buildings
            .iter()
            .map(|b| b.devices.len())
            .sum::<usize>(),
        29
    );

    assert_eq!(inst.parameters.len(), 1390);
    assert_eq!(
        inst.parameters
            .iter()
            .filter(|p| p.source.ets_id.contains("_UP-"))
            .count(),
        216
    );

    let coms: Vec<_> = p
        .devices
        .iter()
        .flat_map(|d| d.com_objects.clone())
        .filter_map(|id| p.devices.com_object(id))
        .collect();
    assert_eq!(coms.len(), 907);

    let (send, receive) =
        coms.iter()
            .flat_map(|c| c.links.iter())
            .fold((0, 0), |(s, r), l| match l.direction {
                Direction::Send => (s + 1, r),
                Direction::Receive => (s, r + 1),
            });
    assert_eq!((send, receive), (569, 27));

    assert_eq!(
        coms.iter()
            .filter(|c| matches!(c.dpt, Override::Value(_)))
            .count(),
        261
    );
    assert_eq!(
        coms.iter()
            .filter(|c| matches!(c.dpt, Override::Empty))
            .count(),
        497
    );
    assert_eq!(
        coms.iter().filter(|c| c.description.is_present()).count(),
        691
    );
    assert_eq!(coms.iter().filter(|c| c.text.is_present()).count(), 121);
    assert_eq!(
        coms.iter().filter(|c| c.flags.read.is_present()).count(),
        39
    );
    assert_eq!(
        coms.iter().filter(|c| c.flags.update.is_present()).count(),
        30
    );
    assert_eq!(
        coms.iter()
            .filter(|c| c.flags.transmit.is_present())
            .count(),
        27
    );
    assert_eq!(
        coms.iter().filter(|c| c.flags.write.is_present()).count(),
        18
    );
    assert_eq!(
        coms.iter()
            .filter(|c| c.flags.communication.is_present())
            .count(),
        8
    );

    assert_eq!(
        p.devices.iter().flat_map(|d| d.binary_data.iter()).count(),
        3
    );
}

#[test]
fn nothing_in_the_reference_project_is_unknown_or_lost() {
    let out = import_knxproj(&reference_ets4_path()).unwrap();
    assert_eq!(out.report.unknown, vec![]);
    assert_eq!(out.report.errors, vec![]);
    assert!(!out.report.has_losses());
    for row in &out.report.counts.rows {
        assert_eq!(
            row.read, row.mapped,
            "{} lost entities in mapping",
            row.entity
        );
    }
}
