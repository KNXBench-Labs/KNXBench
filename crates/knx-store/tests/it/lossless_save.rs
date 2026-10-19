//! A save is exact or refused — never lossy (ADR-0074).
//!
//! The `.knxdb` schema keys entities by id, holds one area per line and
//! rebuilds `children` lists from `parent_id`. Before this check, each case
//! below saved "successfully" and reopened different (duplicates collapsed,
//! an orphaned line vanished, doubled or missing child entries were
//! rewritten). Each must now be refused with a named finding and leave the
//! file on disk untouched; representable variations must reopen exactly.

use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, CompletionStatus, DeviceId, GroupAddress,
    GroupRangeId, IdAllocators, Installation, InstallationId, LineId, Project, Topology,
};
use knx_store::representable::{EntityKind, RepresentationIssue};
use knx_store::{load_project, open_and_migrate, save_project, StoreError};

fn base() -> Project {
    let mut project = knx_etsproj::import_knxproj_bytes(
        knx_testsupport::minimal_knxproj_bytes(),
        "synthetic.knxproj",
    )
    .unwrap()
    .project;
    // Fixture entities below use ids up to 99; keep the counters above them
    // so a reload does not need to repair them (ADR-0039 D7).
    project.ids.raise_to(&IdAllocators::from_counts(
        100, 100, 100, 100, 100, 100, 100, 100, 100,
    ));
    project
}

fn part(project: &Project, id: u32, devices: Vec<DeviceId>) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: project.installations[0].topology.areas[0].source.clone(),
        name: format!("B{id}"),
        number: None,
        kind: BuildingPartType::Room,
        default_line: None,
        completion: CompletionStatus::Undefined,
        children: vec![],
        devices,
        parent: None,
    }
}

fn first_device(project: &Project) -> DeviceId {
    project.installations[0].topology.lines[0].devices[0]
}

/// Saves `clean`, then tries `changed`: must be refused with exactly
/// `expected`, and the file must still reopen as `clean`.
fn assert_refused(changed: impl FnOnce(&mut Project), expected: Vec<RepresentationIssue>) {
    let clean = base();
    let mut project = clean.clone();
    changed(&mut project);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.knxdb");
    let conn = open_and_migrate(&path).unwrap();
    save_project(&conn, &clean).unwrap();
    let before = files(&path);
    match save_project(&conn, &project) {
        Err(StoreError::Unrepresentable(issues)) => assert_eq!(issues, expected),
        other => panic!("expected Unrepresentable({expected:?}), got {other:?}"),
    }
    assert_eq!(files(&path), before, "a refused save writes no byte");
    assert_eq!(load_project(&conn).unwrap(), clean);
}

/// The SQLite main file and its WAL, read separately (atomicity evidence,
/// not a byte-exact export claim).
fn files(path: &std::path::Path) -> (Vec<u8>, Option<Vec<u8>>) {
    let wal = path.with_file_name(format!(
        "{}-wal",
        path.file_name().unwrap().to_string_lossy()
    ));
    (std::fs::read(path).unwrap(), std::fs::read(wal).ok())
}

fn assert_exact(changed: impl FnOnce(&mut Project)) {
    let mut project = base();
    changed(&mut project);
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    save_project(&conn, &project).unwrap();
    assert_eq!(load_project(&conn).unwrap(), project);
}

#[test]
fn duplicate_ids_are_refused_per_kind() {
    assert_refused(
        |p| {
            let mut area = p.installations[0].topology.areas[0].clone();
            area.address = 9;
            area.lines.clear();
            p.installations[0].topology.areas.push(area);
        },
        vec![RepresentationIssue::DuplicateId {
            kind: EntityKind::Area,
            id: 1,
        }],
    );
    assert_refused(
        |p| {
            let mut entry = p.installations[0].group_addresses[0].clone();
            entry.address = GroupAddress::from_raw(5);
            p.installations[0].group_addresses.push(entry);
        },
        vec![RepresentationIssue::DuplicateId {
            kind: EntityKind::GroupAddress,
            id: 1,
        }],
    );
    assert_refused(
        |p| {
            let mut range = p.installations[0].group_ranges[0].clone();
            range.name = "copy".into();
            p.installations[0].group_ranges.push(range);
        },
        vec![RepresentationIssue::DuplicateId {
            kind: EntityKind::GroupRange,
            id: 2,
        }],
    );
    assert_refused(
        |p| {
            let first = part(p, 50, vec![]);
            let mut second = first.clone();
            second.name = "other".into();
            p.installations[0].buildings.extend([first, second]);
        },
        vec![RepresentationIssue::DuplicateId {
            kind: EntityKind::BuildingPart,
            id: 50,
        }],
    );
}

#[test]
fn an_orphaned_line_and_a_foreign_line_reference_are_refused() {
    assert_refused(
        |p| {
            let mut line = p.installations[0].topology.lines[0].clone();
            line.id = LineId(77);
            line.devices.clear();
            p.installations[0].topology.lines.push(line);
        },
        vec![RepresentationIssue::OrphanLine(LineId(77))],
    );
    // An area of a second installation lists installation 0's only line,
    // which it does not own; the line itself is still listed at home.
    assert_refused(
        |p| {
            let mut area = p.installations[0].topology.areas[0].clone();
            area.id = knx_core::AreaId(90);
            area.lines = vec![LineId(1)];
            p.installations[0].topology.areas[0].lines.clear();
            p.installations.push(Installation {
                id: InstallationId(1),
                topology: Topology {
                    areas: vec![area],
                    lines: vec![],
                    unassigned: vec![],
                },
                buildings: vec![],
                group_ranges: vec![],
                group_addresses: vec![],
                parameters: vec![],
                ..p.installations[0].clone()
            });
        },
        vec![
            RepresentationIssue::OrphanLine(LineId(1)),
            RepresentationIssue::UnknownLineReference {
                area: knx_core::AreaId(90),
                line: LineId(1),
            },
        ],
    );
}

#[test]
fn inconsistent_parent_and_child_lists_are_refused() {
    let parent_with_child = |p: &Project| {
        p.installations[0]
            .group_ranges
            .iter()
            .position(|r| !r.children.is_empty())
            .unwrap()
    };
    assert_refused(
        |p| {
            let index = parent_with_child(p);
            let child = p.installations[0].group_ranges[index].children[0];
            p.installations[0].group_ranges[index].children.push(child);
        },
        vec![RepresentationIssue::HierarchyMismatch {
            kind: EntityKind::GroupRange,
            id: 1,
        }],
    );
    assert_refused(
        |p| {
            let index = parent_with_child(p);
            p.installations[0].group_ranges[index].children.clear();
        },
        vec![RepresentationIssue::HierarchyMismatch {
            kind: EntityKind::GroupRange,
            id: 2,
        }],
    );
    assert_refused(
        |p| {
            let mut parent = part(p, 60, vec![]);
            parent.children = vec![BuildingPartId(61)];
            let child = part(p, 61, vec![]); // parent pointer missing
            p.installations[0].buildings.extend([parent, child]);
        },
        vec![RepresentationIssue::HierarchyMismatch {
            kind: EntityKind::BuildingPart,
            id: 60,
        }],
    );
}

#[test]
fn a_device_twice_in_one_building_part_is_refused() {
    assert_refused(
        |p| {
            let device = first_device(p);
            let doubled = part(p, 50, vec![device, device]);
            p.installations[0].buildings.push(doubled);
        },
        vec![RepresentationIssue::DeviceRepeatedInBuildingPart {
            part: BuildingPartId(50),
            device: DeviceId(1),
        }],
    );
}

#[test]
fn representable_variations_reopen_exactly() {
    assert_exact(|_| {});
    // A device in building parts of two installations is representable.
    assert_exact(|p| {
        let device = first_device(p);
        let home = part(p, 60, vec![device]);
        let away = part(p, 61, vec![device]);
        p.installations[0].buildings.push(home);
        p.installations.push(Installation {
            id: InstallationId(1),
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![away],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
            ..p.installations[0].clone()
        });
    });
    // A consistent nested building hierarchy.
    assert_exact(|p| {
        let mut parent = part(p, 60, vec![]);
        parent.children = vec![BuildingPartId(62), BuildingPartId(61)];
        let mut first = part(p, 61, vec![]);
        first.parent = Some(BuildingPartId(60));
        let mut second = part(p, 62, vec![]);
        second.parent = Some(BuildingPartId(60));
        p.installations[0].buildings.extend([parent, first, second]);
    });
}

#[test]
fn a_dangling_range_pointer_is_still_refused_not_written() {
    let clean = base();
    let mut project = clean.clone();
    project.installations[0].group_addresses[0].range = Some(GroupRangeId(999));
    let dir = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    save_project(&conn, &clean).unwrap();
    // Not a representation limit but a dangling reference: the deferred
    // foreign-key check still rejects it at commit.
    assert!(matches!(
        save_project(&conn, &project),
        Err(StoreError::Sqlite(_))
    ));
    assert_eq!(load_project(&conn).unwrap(), clean);
}
