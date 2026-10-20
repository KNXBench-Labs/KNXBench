//! Strict normalized graph admission before any lossy dictionary projection.
use super::budget::MAX_DEPTH;
use knx_core::*;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Debug;

pub(super) struct Selection<'a> {
    pub installation: &'a Installation,
    pub devices: BTreeMap<DeviceId, &'a DeviceInstance>,
    pub addresses: BTreeMap<GroupAddressId, &'a GroupAddressEntry>,
    pub spaces: BTreeMap<BuildingPartId, &'a BuildingPart>,
    pub ranges: BTreeMap<GroupRangeId, &'a GroupRange>,
    pub functions: Vec<&'a Function>,
}
fn unique<T: Ord + Copy + Debug>(set: &mut BTreeSet<T>, id: T, kind: &str) -> Result<(), String> {
    if !set.insert(id) {
        return Err(format!("duplicate {kind} ID/reference {id:?}"));
    }
    Ok(())
}
fn own<T: Ord + Copy + Debug>(
    owners: &mut BTreeMap<T, InstallationId>,
    id: T,
    owner: InstallationId,
) -> Result<(), String> {
    if owners.insert(id, owner).is_some_and(|old| old != owner) {
        return Err(format!("conflicting ownership reference {id:?}"));
    }
    Ok(())
}

/// Check both sides, reachability, and the inclusive depth bound iteratively.
/// Parent walking is bounded by 65 steps, even for a completely rootless cycle.
fn hierarchy<T: Ord + Copy + Debug>(nodes: &BTreeMap<T, (Option<T>, &[T])>) -> Result<(), String> {
    let mut backlinks = BTreeMap::new();
    for (&id, (_, children)) in nodes {
        let mut seen = BTreeSet::new();
        for &child in *children {
            unique(&mut seen, child, "hierarchy child")?;
            let node = nodes.get(&child).ok_or("dangling hierarchy reference")?;
            if node.0 != Some(id) || backlinks.insert(child, id).is_some() {
                return Err("inconsistent hierarchy parent/children".into());
            }
        }
    }
    for (&id, (parent, _)) in nodes {
        if let Some(parent) = parent {
            if !nodes.contains_key(parent) {
                return Err("dangling hierarchy reference".into());
            }
            if backlinks.get(&id) != Some(parent) {
                return Err("inconsistent hierarchy parent/children".into());
            }
        }
        let mut cursor = Some(id);
        let mut path = BTreeSet::new();
        let mut depth = 0;
        while let Some(current) = cursor {
            if !path.insert(current) {
                return Err("cyclic hierarchy".into());
            }
            depth += 1;
            if depth > MAX_DEPTH {
                return Err("hierarchy depth limit exceeded (64)".into());
            }
            cursor = nodes.get(&current).ok_or("dangling hierarchy reference")?.0;
        }
    }
    Ok(())
}

pub(super) fn select<'a>(
    p: &'a Project,
    requested: Option<InstallationId>,
) -> Result<Selection<'a>, String> {
    let chosen = match requested {
        Some(id) => p
            .installations
            .iter()
            .find(|i| i.id == id)
            .ok_or("unknown installation")?,
        None => match p.installations.as_slice() {
            [i] => i,
            _ => return Err("select exactly one installation explicitly".into()),
        },
    };
    let mut installations = BTreeSet::new();
    let mut areas = BTreeSet::new();
    let mut lines = BTreeSet::new();
    let mut ranges = BTreeSet::new();
    let mut spaces = BTreeSet::new();
    let mut gas = BTreeSet::new();
    let mut owners = BTreeMap::new();
    let mut ga_owners = BTreeMap::new();
    let mut space_owners = BTreeMap::new();
    for i in &p.installations {
        unique(&mut installations, i.id, "installation")?;
        let mut area_addresses = BTreeSet::new();
        let mut attached_lines = BTreeSet::new();
        let line_ids: BTreeSet<_> = i.topology.lines.iter().map(|l| l.id).collect();
        for a in &i.topology.areas {
            unique(&mut areas, a.id, "area")?;
            unique(&mut area_addresses, a.address, "area address")?;
            if a.address > 15 {
                return Err("area address exceeds 15".into());
            }
            let mut local_addresses = BTreeSet::new();
            for &id in &a.lines {
                unique(&mut attached_lines, id, "line placement")?;
                let line = i
                    .topology
                    .line(id)
                    .ok_or("dangling topology line reference")?;
                unique(&mut local_addresses, line.address, "line address")?;
            }
        }
        for l in &i.topology.lines {
            unique(&mut lines, l.id, "line")?;
            if !attached_lines.contains(&l.id) {
                return Err("line ownership reference missing".into());
            }
            if l.address > 15 {
                return Err("line address exceeds 15".into());
            }
            let mut placement = BTreeSet::new();
            for &d in &l.devices {
                unique(&mut placement, d, "device topology placement")?;
                own(&mut owners, d, i.id)?;
            }
        }
        let mut unassigned = BTreeSet::new();
        for &d in &i.topology.unassigned {
            unique(&mut unassigned, d, "unassigned device placement")?;
            own(&mut owners, d, i.id)?;
        }
        if i.default_line.is_some_and(|id| !line_ids.contains(&id)) {
            return Err("dangling installation default line reference".into());
        }
        let mut space_nodes = BTreeMap::new();
        let mut sibling_names = BTreeSet::new();
        for s in &i.buildings {
            unique(&mut spaces, s.id, "space")?;
            space_owners.insert(s.id, i.id);
            if !sibling_names.insert((s.parent, s.name.as_str())) {
                return Err("duplicate sibling space name".into());
            }
            if s.default_line.is_some_and(|id| !line_ids.contains(&id)) {
                return Err("dangling space default line reference".into());
            }
            space_nodes.insert(s.id, (s.parent, s.children.as_slice()));
            let mut local_devices = BTreeSet::new();
            for &d in &s.devices {
                unique(&mut local_devices, d, "space device")?;
                own(&mut owners, d, i.id)?;
            }
            if i.id == chosen.id && s.kind == BuildingPartType::RoomPart {
                return Err("unsupported space kind RoomPart".into());
            }
        }
        hierarchy(&space_nodes)?;
        let mut range_nodes = BTreeMap::new();
        let mut range_map = BTreeMap::new();
        for r in &i.group_ranges {
            unique(&mut ranges, r.id, "range")?;
            range_nodes.insert(r.id, (r.parent, r.children.as_slice()));
            range_map.insert(r.id, r);
            if r.start > r.end {
                return Err("invalid range bounds".into());
            }
        }
        hierarchy(&range_nodes)?;
        for r in &i.group_ranges {
            if let Some(parent) = r.parent {
                let parent = range_map.get(&parent).ok_or("dangling range reference")?;
                if !parent.contains(r.start) || !parent.contains(r.end) {
                    return Err("range hierarchy bounds inconsistent".into());
                }
            }
        }
        let mut raw_addresses = BTreeSet::new();
        for g in &i.group_addresses {
            unique(&mut gas, g.id, "group address")?;
            unique(&mut raw_addresses, g.address, "group address key")?;
            ga_owners.insert(g.id, i.id);
            if let Some(r) = g.range {
                let r = range_map.get(&r).ok_or("dangling group range reference")?;
                if !r.contains(g.address) {
                    return Err("group address outside range bounds".into());
                }
            }
        }
    }
    for &d in owners.keys() {
        let device = p.devices.get(d).ok_or("dangling device reference")?;
        if device.id != d {
            return Err("device ID reference mismatch".into());
        }
    }
    let mut device_ids = BTreeSet::new();
    let mut object_ids = BTreeSet::new();
    let mut addresses = BTreeSet::new();
    for d in p.devices.iter() {
        unique(&mut device_ids, d.id, "device")?;
        let owner = owners.get(&d.id).ok_or(
            "device ownership reference missing; individual address does not establish ownership",
        )?;
        if let Some(address) = d.address {
            unique(&mut addresses, (*owner, address), "individual address key")?;
        }
        for &id in &d.com_objects {
            unique(&mut object_ids, id, "object")?;
            let o = p
                .devices
                .com_object(id)
                .ok_or("dangling communication object reference")?;
            if o.id != id {
                return Err("communication object ID reference mismatch".into());
            }
            if o.device != d.id {
                return Err("communication object owner reference mismatch".into());
            }
        }
    }
    let mut actual_object_ids = BTreeSet::new();
    for o in p.devices.com_objects() {
        unique(&mut actual_object_ids, o.id, "actual object")?;
        if !object_ids.contains(&o.id) {
            return Err("orphan communication object reference".into());
        }
        let owner = owners
            .get(&o.device)
            .ok_or("dangling object device reference")?;
        let mut links = BTreeSet::new();
        for link in &o.links {
            let ga_owner = ga_owners
                .get(&link.ga)
                .ok_or("dangling group address link reference")?;
            if owner != ga_owner {
                return Err("cross-installation group address reference".into());
            }
            if !links.insert((
                link.ga,
                match link.direction {
                    Direction::Send => 0,
                    Direction::Receive => 1,
                },
            )) {
                return Err("duplicate directional group address reference".into());
            }
        }
        if let Some(id) = o.module_instance {
            let m = p
                .devices
                .module_instance(id)
                .ok_or("dangling module reference")?;
            if m.device != o.device {
                return Err("cross-owner module reference".into());
            }
        }
    }
    let mut functions = BTreeSet::new();
    let mut members = BTreeSet::new();
    for f in &p.functions {
        unique(&mut functions, f.id, "function")?;
        let owner = space_owners
            .get(&f.location)
            .ok_or("dangling function location reference")?;
        let mut local_members = BTreeSet::new();
        for m in &f.members {
            unique(&mut members, m.id, "function member")?;
            unique(
                &mut local_members,
                m.group_address,
                "function member address",
            )?;
            if ga_owners.get(&m.group_address) != Some(owner) {
                return Err("dangling or cross-installation function member reference".into());
            }
        }
    }
    // Retained normalized references are checked too, even though their values
    // are excluded. They must not manufacture a device's installation owner.
    for i in &p.installations {
        for param in &i.parameters {
            if owners.get(&param.device) != Some(&i.id) {
                return Err("parameter device ownership reference mismatch".into());
            }
        }
    }
    for m in p.devices.module_instances() {
        if !owners.contains_key(&m.device) {
            return Err("dangling module device reference".into());
        }
    }
    Ok(Selection {
        installation: chosen,
        devices: p
            .devices
            .iter()
            .filter(|d| owners.get(&d.id) == Some(&chosen.id))
            .map(|d| (d.id, d))
            .collect(),
        addresses: chosen.group_addresses.iter().map(|g| (g.id, g)).collect(),
        spaces: chosen.buildings.iter().map(|s| (s.id, s)).collect(),
        ranges: chosen.group_ranges.iter().map(|r| (r.id, r)).collect(),
        functions: p
            .functions
            .iter()
            .filter(|f| space_owners.get(&f.location) == Some(&chosen.id))
            .collect(),
    })
}
