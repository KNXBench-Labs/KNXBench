//! Checks before every save that the native schema can hold the project exactly.
//!
//! The `.knxdb` tables key every entity by its id (`ON CONFLICT(id) DO UPDATE`),
//! store one placement per device and one area per line (`line.area_id NOT
//! NULL`), and rebuild `children` lists from `parent_id` on load. A project
//! outside those limits used to save "successfully" and reopen different —
//! the last duplicate won, an orphaned line vanished, a doubled child entry
//! collapsed. Such a project is refused here before any write, with typed
//! findings, so nothing is lost silently (ADR-0071, ADR-0074).

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use knx_core::ids::{AreaId, BuildingPartId, DeviceId, LineId};
use knx_core::project::Project;

use crate::StoreError;

/// Entity kinds whose ids are table primary keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EntityKind {
    Area,
    Line,
    BuildingPart,
    GroupRange,
    GroupAddress,
    ParameterInstance,
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            EntityKind::Area => "area",
            EntityKind::Line => "line",
            EntityKind::BuildingPart => "building part",
            EntityKind::GroupRange => "group range",
            EntityKind::GroupAddress => "group address",
            EntityKind::ParameterInstance => "parameter instance",
        })
    }
}

/// One reason the schema cannot hold the project exactly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum RepresentationIssue {
    /// The same id is used by several entities of one kind (anywhere in the
    /// project); the table would keep only the last one.
    DuplicateId { kind: EntityKind, id: u32 },
    /// A line no area lists; `line.area_id` cannot be empty.
    OrphanLine(LineId),
    /// An area lists a line that is not in its own installation's topology.
    UnknownLineReference { area: AreaId, line: LineId },
    /// A building part or group range whose `parent` and its parent's
    /// `children` disagree (missing, doubled, or in another installation);
    /// reload rebuilds `children` from `parent` and would differ.
    HierarchyMismatch { kind: EntityKind, id: u32 },
    /// A device listed twice in one building part.
    DeviceRepeatedInBuildingPart {
        part: BuildingPartId,
        device: DeviceId,
    },
}

impl fmt::Display for RepresentationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RepresentationIssue::DuplicateId { kind, id } => {
                write!(f, "{kind} id {id} is used more than once")
            }
            RepresentationIssue::OrphanLine(line) => {
                write!(f, "line {line} belongs to no area")
            }
            RepresentationIssue::UnknownLineReference { area, line } => write!(
                f,
                "area {area} lists line {line}, which is not in its installation"
            ),
            RepresentationIssue::HierarchyMismatch { kind, id } => {
                write!(f, "{kind} {id}: parent and child lists disagree")
            }
            RepresentationIssue::DeviceRepeatedInBuildingPart { part, device } => {
                write!(f, "device {device} is listed twice in building part {part}")
            }
        }
    }
}

/// Refuses a project the schema cannot hold exactly. Placement ambiguity
/// keeps its own error (MODEL-02 repair commands exist for it); every other
/// finding is reported together as [`StoreError::Unrepresentable`].
pub(crate) fn check_representable(project: &Project) -> Result<(), StoreError> {
    check_unambiguous_topology(project)?;
    let mut issues = BTreeSet::new();
    duplicate_ids(project, &mut issues);
    line_references(project, &mut issues);
    for installation in &project.installations {
        hierarchy(
            EntityKind::BuildingPart,
            installation.buildings.iter().map(|p| {
                (
                    p.id.0,
                    p.parent.map(|x| x.0),
                    p.children.iter().map(|c| c.0).collect(),
                )
            }),
            &mut issues,
        );
        hierarchy(
            EntityKind::GroupRange,
            installation.group_ranges.iter().map(|r| {
                (
                    r.id.0,
                    r.parent.map(|x| x.0),
                    r.children.iter().map(|c| c.0).collect(),
                )
            }),
            &mut issues,
        );
        for part in &installation.buildings {
            let mut seen = BTreeSet::new();
            for device in &part.devices {
                if !seen.insert(*device) {
                    issues.insert(RepresentationIssue::DeviceRepeatedInBuildingPart {
                        part: part.id,
                        device: *device,
                    });
                }
            }
        }
    }
    if issues.is_empty() {
        Ok(())
    } else {
        Err(StoreError::Unrepresentable(issues.into_iter().collect()))
    }
}

/// The schema stores one placement per device (`device.line_id`) and one
/// area per line (`line.area_id`). A project that places a device twice or
/// lists a line under several area entries would be collapsed by the save
/// upserts — last write wins, silently. Refused instead; the user repairs it
/// explicitly first (MODEL-02 `RepairDevicePlacement` / `RepairLineOwner`).
fn check_unambiguous_topology(project: &Project) -> Result<(), StoreError> {
    let mut device_counts: std::collections::BTreeMap<DeviceId, usize> = Default::default();
    let mut line_counts: std::collections::BTreeMap<LineId, usize> = Default::default();
    for installation in &project.installations {
        let topology = &installation.topology;
        for device in topology
            .unassigned
            .iter()
            .chain(topology.lines.iter().flat_map(|line| &line.devices))
        {
            *device_counts.entry(*device).or_default() += 1;
        }
        for line in topology.areas.iter().flat_map(|area| &area.lines) {
            *line_counts.entry(*line).or_default() += 1;
        }
    }
    let devices: Vec<DeviceId> = device_counts
        .into_iter()
        .filter(|&(_, count)| count > 1)
        .map(|(id, _)| id)
        .collect();
    let lines: Vec<LineId> = line_counts
        .into_iter()
        .filter(|&(_, count)| count > 1)
        .map(|(id, _)| id)
        .collect();
    if devices.is_empty() && lines.is_empty() {
        Ok(())
    } else {
        Err(StoreError::AmbiguousTopology { devices, lines })
    }
}

fn duplicate_ids(project: &Project, issues: &mut BTreeSet<RepresentationIssue>) {
    let mut seen: BTreeSet<(EntityKind, u32)> = BTreeSet::new();
    let mut note = |kind: EntityKind, id: u32| {
        if !seen.insert((kind, id)) {
            issues.insert(RepresentationIssue::DuplicateId { kind, id });
        }
    };
    for installation in &project.installations {
        installation
            .topology
            .areas
            .iter()
            .for_each(|a| note(EntityKind::Area, a.id.0));
        installation
            .topology
            .lines
            .iter()
            .for_each(|l| note(EntityKind::Line, l.id.0));
        installation
            .buildings
            .iter()
            .for_each(|p| note(EntityKind::BuildingPart, p.id.0));
        installation
            .group_ranges
            .iter()
            .for_each(|r| note(EntityKind::GroupRange, r.id.0));
        installation
            .group_addresses
            .iter()
            .for_each(|g| note(EntityKind::GroupAddress, g.id.0));
        installation
            .parameters
            .iter()
            .for_each(|p| note(EntityKind::ParameterInstance, p.id.0));
    }
}

/// Every line must be listed by an area of its own installation; every area
/// reference must name a line of that installation.
fn line_references(project: &Project, issues: &mut BTreeSet<RepresentationIssue>) {
    for installation in &project.installations {
        let topology = &installation.topology;
        let own: BTreeSet<LineId> = topology.lines.iter().map(|l| l.id).collect();
        let mut listed = BTreeSet::new();
        for area in &topology.areas {
            for line in &area.lines {
                if own.contains(line) {
                    listed.insert(*line);
                } else {
                    issues.insert(RepresentationIssue::UnknownLineReference {
                        area: area.id,
                        line: *line,
                    });
                }
            }
        }
        for line in own.difference(&listed) {
            issues.insert(RepresentationIssue::OrphanLine(*line));
        }
    }
}

/// `items` are `(id, parent, children)` of one installation's hierarchy.
/// Representable iff each parent's `children` lists exactly the items that
/// name it as parent, each once, and every parent exists here.
fn hierarchy(
    kind: EntityKind,
    items: impl Iterator<Item = (u32, Option<u32>, Vec<u32>)>,
    issues: &mut BTreeSet<RepresentationIssue>,
) {
    let items: Vec<_> = items.collect();
    let parents: BTreeMap<u32, Option<u32>> =
        items.iter().map(|(id, parent, _)| (*id, *parent)).collect();
    let children: BTreeMap<u32, &Vec<u32>> = items
        .iter()
        .map(|(id, _, children)| (*id, children))
        .collect();
    for (id, parent, listed) in &items {
        let mut seen = BTreeSet::new();
        for child in listed {
            let consistent = seen.insert(*child) && parents.get(child) == Some(&Some(*id));
            if !consistent {
                issues.insert(RepresentationIssue::HierarchyMismatch { kind, id: *id });
            }
        }
        if let Some(parent) = parent {
            let listed_by_parent = children
                .get(parent)
                .is_some_and(|siblings| siblings.contains(id));
            if !listed_by_parent {
                issues.insert(RepresentationIssue::HierarchyMismatch { kind, id: *id });
            }
        }
    }
}
