//! Remaps topology and hierarchical dependencies without importing unrelated devices.

use super::{
    merge::{mapping, unique},
    scoped, IdMapping,
};
use knx_core::*;
use std::collections::{BTreeMap, BTreeSet};

type PlacementMaps = (
    BTreeMap<LineId, LineId>,
    BTreeMap<BuildingPartId, BuildingPartId>,
);

pub(super) fn ancestors<T: Copy + Ord>(
    start: T,
    included: &mut BTreeSet<T>,
    mut parent: impl FnMut(T) -> Result<Option<T>, String>,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut next = Some(start);
    while let Some(id) = next {
        if !seen.insert(id) {
            return Err("cyclic source dependency".into());
        }
        if seen.len() > 1024 {
            return Err("source dependency depth exceeds 1024".into());
        }
        included.insert(id);
        next = parent(id)?;
    }
    Ok(())
}

pub(super) struct PlacementRequest<'a> {
    pub source: &'a Installation,
    pub target_index: usize,
    pub lines: &'a BTreeSet<LineId>,
    pub buildings: &'a BTreeSet<BuildingPartId>,
    pub devices: &'a BTreeMap<DeviceId, DeviceId>,
    pub prefix: &'a str,
}

pub(super) fn topology_and_buildings(
    target: &mut Project,
    request: PlacementRequest<'_>,
    mappings: &mut Vec<IdMapping>,
    notes: &mut Vec<String>,
) -> Result<PlacementMaps, String> {
    let PlacementRequest {
        source,
        target_index,
        lines,
        buildings,
        devices,
        prefix,
    } = request;
    let initial_line_count = target.installations[target_index].topology.lines.len();
    let initial_area_count = target.installations[target_index].topology.areas.len();
    let initial_children: BTreeMap<_, _> = target.installations[target_index]
        .topology
        .areas
        .iter()
        .map(|a| (a.id, a.lines.clone()))
        .collect();
    let mut line_map = BTreeMap::new();
    let mut area_map = BTreeMap::new();
    for old in lines {
        let line = unique(
            source.topology.lines.iter().filter(|l| l.id == *old),
            "source line",
        )?;
        let area = unique(
            source
                .topology
                .areas
                .iter()
                .filter(|a| a.lines.contains(old)),
            "source line owner",
        )?;
        let new_area = if let Some(id) = area_map.get(&area.id) {
            *id
        } else {
            let candidates: Vec<_> = target.installations[target_index]
                .topology
                .areas
                .iter()
                .filter(|a| a.address == area.address)
                .collect();
            let (id, reused) = match candidates.as_slice() {
                [] => {
                    let id = target.ids.next_area_id().map_err(|e| e.to_string())?;
                    let mut copy = area.clone();
                    copy.id = id;
                    copy.source = scoped(&area.source, prefix);
                    copy.lines.clear();
                    target.installations[target_index].topology.areas.push(copy);
                    (id, false)
                }
                [existing] => {
                    if existing.name != area.name || existing.completion != area.completion {
                        notes.push(format!("Existing area {} metadata retained; source metadata remains in the original archive.",area.address));
                    }
                    (existing.id, true)
                }
                _ => return Err("ambiguous target area address".into()),
            };
            area_map.insert(area.id, id);
            mappings.push(mapping("area", area.id.0, id.0, reused));
            id
        };
        let candidates: Vec<_> = target.installations[target_index]
            .topology
            .lines
            .iter()
            .filter(|l| {
                l.address == line.address
                    && target.installations[target_index]
                        .topology
                        .areas
                        .iter()
                        .any(|a| a.id == new_area && a.lines.contains(&l.id))
            })
            .collect();
        let (id, reused) = match candidates.as_slice() {
            [] => {
                let id = target.ids.next_line_id().map_err(|e| e.to_string())?;
                let mut copy = line.clone();
                copy.id = id;
                copy.source = scoped(&line.source, prefix);
                copy.devices.clear();
                target.installations[target_index].topology.lines.push(copy);
                target.installations[target_index]
                    .topology
                    .areas
                    .iter_mut()
                    .find(|a| a.id == new_area)
                    .ok_or("target area missing")?
                    .lines
                    .push(id);
                (id, false)
            }
            [existing] => {
                if existing.medium_ref != line.medium_ref
                    || existing.domain_address != line.domain_address
                    || existing.domain_address_is_checked != line.domain_address_is_checked
                    || existing.ip_routing_multicast_address != line.ip_routing_multicast_address
                    || existing.multicast_ttl != line.multicast_ttl
                {
                    return Err("target line configuration conflicts with the source line".into());
                }
                if existing.name != line.name || existing.completion != line.completion {
                    notes.push(format!("Existing line {}.{} metadata retained; source metadata remains in the original archive.",area.address,line.address));
                }
                (existing.id, true)
            }
            _ => return Err("ambiguous target line address".into()),
        };
        line_map.insert(*old, id);
        mappings.push(mapping("line", old.0, id.0, reused));
    }
    // Newly created vectors retain source order; reused destination vectors keep theirs.
    let source_line_positions: BTreeMap<_, _> = source
        .topology
        .lines
        .iter()
        .enumerate()
        .filter_map(|(position, line)| line_map.get(&line.id).map(|id| (*id, position)))
        .collect();
    target.installations[target_index].topology.lines[initial_line_count..]
        .sort_by_key(|l| source_line_positions[&l.id]);
    let source_area_positions: BTreeMap<_, _> = source
        .topology
        .areas
        .iter()
        .enumerate()
        .filter_map(|(position, area)| area_map.get(&area.id).map(|id| (*id, position)))
        .collect();
    target.installations[target_index].topology.areas[initial_area_count..]
        .sort_by_key(|a| source_area_positions[&a.id]);
    for area in &source.topology.areas {
        if let Some(mapped) = area_map.get(&area.id) {
            let mut children = initial_children.get(mapped).cloned().unwrap_or_default();
            for id in area.lines.iter().filter_map(|id| line_map.get(id)) {
                if !children.contains(id) {
                    children.push(*id);
                }
            }
            target.installations[target_index]
                .topology
                .areas
                .iter_mut()
                .find(|a| a.id == *mapped)
                .ok_or("target area missing")?
                .lines = children;
        }
    }
    let mut building_map = BTreeMap::new();
    for part in source
        .buildings
        .iter()
        .filter(|b| buildings.contains(&b.id))
    {
        let id = target
            .ids
            .next_building_part_id()
            .map_err(|e| e.to_string())?;
        if building_map.insert(part.id, id).is_some() {
            return Err("ambiguous source building identity".into());
        }
        mappings.push(mapping("building_part", part.id.0, id.0, false));
    }
    for part in source
        .buildings
        .iter()
        .filter(|b| buildings.contains(&b.id))
    {
        let mut copy = part.clone();
        copy.id = building_map[&part.id];
        copy.source = scoped(&part.source, prefix);
        copy.parent = part
            .parent
            .map(|p| {
                building_map
                    .get(&p)
                    .copied()
                    .ok_or("building parent missing")
            })
            .transpose()?;
        copy.default_line = part
            .default_line
            .map(|l| {
                line_map
                    .get(&l)
                    .copied()
                    .ok_or("building default line missing")
            })
            .transpose()?;
        copy.children = part
            .children
            .iter()
            .filter_map(|id| building_map.get(id).copied())
            .collect();
        copy.devices = part
            .devices
            .iter()
            .filter_map(|id| devices.get(id).copied())
            .collect();
        target.installations[target_index].buildings.push(copy);
    }
    Ok((line_map, building_map))
}

pub(super) fn groups(
    target: &mut Project,
    source: &Installation,
    target_index: usize,
    addresses: &BTreeSet<GroupAddressId>,
    prefix: &str,
    mappings: &mut Vec<IdMapping>,
) -> Result<BTreeMap<GroupAddressId, GroupAddressId>, String> {
    let mut ranges = BTreeSet::new();
    for id in addresses {
        let ga = unique(
            source.group_addresses.iter().filter(|g| g.id == *id),
            "selected group address",
        )?;
        if !ga.address.fits_style(target.info.group_address_style) {
            return Err("selected group address does not fit the destination address style".into());
        }
        if let Some(range) = ga.range {
            ancestors(range, &mut ranges, |id| {
                unique(
                    source.group_ranges.iter().filter(|r| r.id == id),
                    "group range",
                )
                .map(|r| r.parent)
            })?;
        }
    }
    let initial_range_count = target.installations[target_index].group_ranges.len();
    let initial_children: BTreeMap<_, _> = target.installations[target_index]
        .group_ranges
        .iter()
        .map(|r| (r.id, r.children.clone()))
        .collect();
    let mut range_map = BTreeMap::new();
    let mut pending = ranges;
    while !pending.is_empty() {
        let available: Vec<_> = pending
            .iter()
            .copied()
            .filter(|id| {
                source
                    .group_ranges
                    .iter()
                    .find(|r| r.id == *id)
                    .is_some_and(|r| r.parent.is_none_or(|p| range_map.contains_key(&p)))
            })
            .collect();
        if available.is_empty() {
            return Err("missing or cyclic group-range dependency".into());
        }
        for old in available {
            let range = unique(
                source.group_ranges.iter().filter(|r| r.id == old),
                "group range",
            )?;
            if !range.start.fits_style(target.info.group_address_style)
                || !range.end.fits_style(target.info.group_address_style)
            {
                return Err("source group range does not fit the destination address style".into());
            }
            let parent = range.parent.map(|p| range_map[&p]);
            let candidates: Vec<_> = target.installations[target_index]
                .group_ranges
                .iter()
                .filter(|r| r.parent == parent && r.start == range.start && r.end == range.end)
                .collect();
            let (id, reused) = match candidates.as_slice() {
                [] => {
                    let siblings = target.installations[target_index]
                        .group_ranges
                        .iter()
                        .filter(|r| r.parent == parent);
                    let id = target
                        .ids
                        .next_group_range_id()
                        .map_err(|e| e.to_string())?;
                    knx_core::validation::check_no_overlapping_group_range(
                        siblings,
                        id,
                        range.start,
                        range.end,
                    )
                    .map_err(|e| e.to_string())?;
                    let mut copy = range.clone();
                    copy.id = id;
                    copy.source = scoped(&range.source, prefix);
                    copy.parent = parent;
                    copy.children.clear();
                    target.installations[target_index].group_ranges.push(copy);
                    if let Some(parent) = parent {
                        target.installations[target_index]
                            .group_ranges
                            .iter_mut()
                            .find(|r| r.id == parent)
                            .ok_or("group parent missing")?
                            .children
                            .push(id);
                    }
                    (id, false)
                }
                [existing] if existing.name == range.name => (existing.id, true),
                _ => {
                    return Err(
                        "group range metadata conflicts or target range is ambiguous".into(),
                    );
                }
            };
            range_map.insert(old, id);
            pending.remove(&old);
            mappings.push(mapping("group_range", old.0, id.0, reused));
        }
    }
    let source_positions: BTreeMap<_, _> = source
        .group_ranges
        .iter()
        .enumerate()
        .filter_map(|(position, range)| range_map.get(&range.id).map(|id| (*id, position)))
        .collect();
    target.installations[target_index].group_ranges[initial_range_count..]
        .sort_by_key(|r| source_positions[&r.id]);
    for range in &source.group_ranges {
        if let Some(mapped) = range_map.get(&range.id) {
            let mut children = initial_children.get(mapped).cloned().unwrap_or_default();
            for id in range.children.iter().filter_map(|id| range_map.get(id)) {
                if !children.contains(id) {
                    children.push(*id);
                }
            }
            target.installations[target_index]
                .group_ranges
                .iter_mut()
                .find(|r| r.id == *mapped)
                .ok_or("group parent missing")?
                .children = children;
        }
    }
    let mut group_map = BTreeMap::new();
    for old in addresses {
        let ga = unique(
            source.group_addresses.iter().filter(|g| g.id == *old),
            "selected group address",
        )?;
        if source
            .group_addresses
            .iter()
            .filter(|g| g.address == ga.address)
            .count()
            != 1
        {
            return Err("duplicate selected group address value".into());
        }
        let range = ga.range.map(|r| range_map[&r]);
        let candidates: Vec<_> = target.installations[target_index]
            .group_addresses
            .iter()
            .filter(|g| g.address == ga.address)
            .collect();
        let (id, reused) = match candidates.as_slice() {
            [] => {
                let id = target
                    .ids
                    .next_group_address_id()
                    .map_err(|e| e.to_string())?;
                let mut copy = ga.clone();
                copy.id = id;
                copy.source = scoped(&ga.source, prefix);
                copy.range = range;
                target.installations[target_index]
                    .group_addresses
                    .push(copy);
                (id, false)
            }
            [existing]
                if existing.name == ga.name
                    && existing.central == ga.central
                    && existing.unfiltered == ga.unfiltered
                    && existing.declared_dpt == ga.declared_dpt
                    && existing.range == range =>
            {
                (existing.id, true)
            }
            _ => {
                return Err(format!(
                    "group address {:?} conflicts with existing metadata",
                    ga.address
                ));
            }
        };
        group_map.insert(*old, id);
        mappings.push(mapping("group_address", old.0, id.0, reused));
    }
    Ok(group_map)
}
