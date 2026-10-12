//! Which project entities a note may be attached to, decided by the service.
//!
//! The catalogue reads the domain model directly (no display projection, which
//! is the UI layer's job) and keeps only targets the domain itself accepts
//! ([`ProjectNoteTarget::validate`], with the same counting done in one pass
//! for installation-scoped kinds): a choice the user can pick is a choice
//! Apply will not refuse as missing or ambiguous. Labels are locale-neutral
//! parts (addresses, names, object numbers) formatted the way the workbench
//! shows them; the UI adds its own translated kind word. The project target is
//! not listed; the editor always offers it.
use std::collections::{BTreeMap, BTreeSet};

use knx_core::{
    note::ProjectNoteTarget, BuildingPart, BuildingPartId, DeviceInstance, Installation, Project,
};

/// One target a note may be attached to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteTargetOption {
    pub target: ProjectNoteTarget,
    /// The owning device's identifying parts, for a communication object;
    /// empty for every other kind.
    pub owner: Vec<String>,
    /// Identifying parts in reading order (address or number, then name).
    /// Blank parts are left out; an entity without any has no parts.
    pub parts: Vec<String>,
}

/// Every entity a note may target, once per stable ID, grouped by kind:
/// installations, areas, lines, building parts, devices, communication
/// objects, group ranges, group addresses. Installation-owned kinds keep
/// project order (lines under their area, building parts in pre-order).
/// Devices are listed by individual address, unaddressed ones last by ID;
/// each device's objects follow in the device's own object order. IDs the
/// domain cannot resolve to exactly one entity are withheld.
pub fn note_target_catalogue(project: &Project) -> Vec<NoteTargetOption> {
    let mut kinds: [Vec<NoteTargetOption>; 8] = Default::default();
    let [installations, areas, lines, building_parts, devices, objects, ranges, addresses] =
        &mut kinds;
    let style = project.info.group_address_style;
    for installation in &project.installations {
        installations.push(entry(
            ProjectNoteTarget::Installation(installation.id),
            [&installation.name],
        ));
        for area in &installation.topology.areas {
            areas.push(entry(
                ProjectNoteTarget::Area(area.id),
                [&area.address.to_string(), &area.name],
            ));
            let area_lines = area.lines.iter().filter_map(|id| {
                installation
                    .topology
                    .lines
                    .iter()
                    .find(|line| line.id == *id)
            });
            for line in area_lines {
                lines.push(entry(
                    ProjectNoteTarget::Line(line.id),
                    [&format!("{}.{}", area.address, line.address), &line.name],
                ));
            }
        }
        for part in pre_order(installation) {
            building_parts.push(entry(
                ProjectNoteTarget::BuildingPart(part.id),
                [&part.name],
            ));
        }
        for range in &installation.group_ranges {
            let span = format!("{}–{}", range.start.format(style), range.end.format(style));
            ranges.push(entry(
                ProjectNoteTarget::GroupRange(range.id),
                [&span, &range.name],
            ));
        }
        for address in &installation.group_addresses {
            addresses.push(entry(
                ProjectNoteTarget::GroupAddress(address.id),
                [&address.address.format(style), &address.name],
            ));
        }
    }
    let mut device_order: Vec<&DeviceInstance> = project.devices.iter().collect();
    device_order.sort_by_key(|device| (device.address.is_none(), device.address, device.id));
    let language = project.strings.default_language();
    for device in device_order {
        let address = device
            .address
            .map(|address| address.to_string())
            .unwrap_or_default();
        let owner = present([&address, &device.name]);
        devices.push(NoteTargetOption {
            target: ProjectNoteTarget::Device(device.id),
            owner: Vec::new(),
            parts: owner.clone(),
        });
        let listed = device
            .com_objects
            .iter()
            .filter_map(|id| project.devices.com_object(*id));
        for object in listed.filter(|object| object.device == device.id) {
            let name = object
                .text
                .value()
                .and_then(|text| project.strings.text(&text.value, language))
                .unwrap_or_default();
            objects.push(NoteTargetOption {
                target: ProjectNoteTarget::ComObject(object.id),
                owner: owner.clone(),
                parts: present([&object.number.to_string(), name]),
            });
        }
    }
    let occurrences = occurrences(project);
    let mut seen = BTreeSet::new();
    kinds
        .into_iter()
        .flatten()
        .filter(|option| seen.insert(option.target))
        .filter(|option| match occurrences.get(&option.target) {
            Some(count) => *count == 1,
            // Devices and objects are keyed maps: validate is a lookup there.
            None => option.target.validate(project).is_ok(),
        })
        .collect()
}

/// How often each installation-scoped ID occurs, counted over exactly the
/// collections [`ProjectNoteTarget::validate`] scans, in one pass. Calling
/// `validate` per offered entry instead scans those collections once per
/// entry: 20 000 group addresses took 266 ms (release build) under the
/// server's project lock. `tests/note_targets.rs` pins that every offered
/// entry still passes `validate`.
fn occurrences(project: &Project) -> BTreeMap<ProjectNoteTarget, usize> {
    let mut counts = BTreeMap::new();
    let mut count = |target| *counts.entry(target).or_insert(0) += 1;
    for installation in &project.installations {
        count(ProjectNoteTarget::Installation(installation.id));
        installation
            .topology
            .areas
            .iter()
            .for_each(|v| count(ProjectNoteTarget::Area(v.id)));
        installation
            .topology
            .lines
            .iter()
            .for_each(|v| count(ProjectNoteTarget::Line(v.id)));
        installation
            .buildings
            .iter()
            .for_each(|v| count(ProjectNoteTarget::BuildingPart(v.id)));
        installation
            .group_ranges
            .iter()
            .for_each(|v| count(ProjectNoteTarget::GroupRange(v.id)));
        installation
            .group_addresses
            .iter()
            .for_each(|v| count(ProjectNoteTarget::GroupAddress(v.id)));
    }
    counts
}

fn entry<const N: usize>(target: ProjectNoteTarget, parts: [&str; N]) -> NoteTargetOption {
    NoteTargetOption {
        target,
        owner: Vec::new(),
        parts: present(parts),
    }
}

fn present<const N: usize>(parts: [&str; N]) -> Vec<String> {
    parts
        .into_iter()
        .filter(|part| !part.trim().is_empty())
        .map(str::to_string)
        .collect()
}

/// Building parts in pre-order from the roots, iteratively (deep imported
/// hierarchies must not overflow), then any part no root reaches, in stored
/// order, so a broken hierarchy hides nothing.
fn pre_order(installation: &Installation) -> Vec<&BuildingPart> {
    let parts = &installation.buildings;
    let by_id: BTreeMap<BuildingPartId, &BuildingPart> = parts.iter().map(|p| (p.id, p)).collect();
    let find = |id: BuildingPartId| by_id.get(&id).copied();
    let mut visited = BTreeSet::new();
    let mut ordered = Vec::new();
    let mut stack: Vec<&BuildingPart> = parts.iter().filter(|p| p.parent.is_none()).rev().collect();
    while let Some(part) = stack.pop() {
        if !visited.insert(part.id) {
            continue;
        }
        ordered.push(part);
        stack.extend(part.children.iter().rev().filter_map(|id| find(*id)));
    }
    ordered.extend(parts.iter().filter(|part| !visited.contains(&part.id)));
    ordered
}
