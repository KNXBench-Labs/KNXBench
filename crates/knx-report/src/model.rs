//! The derived model `render.rs` (Task 3) walks: building forests, group
//! range nesting, the group-address → communication-object inverse index,
//! and every orphan and dangling reference the walk finds along the way.
//!
//! Pure data in, pure data out — no HTML, no formatting beyond what a
//! [`crate::ReportWarning`]'s location string needs. `build` never reads
//! the system clock and never touches a `HashMap` where output order is
//! observable: every `Vec` here is either in the model's own stored order
//! or in the id order a `BTreeMap` gives for free.
//!
//! `render_html`'s `render.rs` is this module's only intended caller.

use std::collections::{BTreeMap, HashMap, HashSet};

use knx_core::{
    BuildingPart, BuildingPartId, ComObjectInstanceId, GroupAddress, GroupAddressEntry,
    GroupAddressId, GroupAddressStyle, GroupRange, GroupRangeId, Installation, Project,
};

use crate::ReportWarning;

/// Everything `render.rs` needs, one `InstallationModel` per
/// `project.installations` entry in stored order.
pub(crate) struct ReportModel {
    pub installations: Vec<InstallationModel>,
    /// Communication object instances owned by no device, or by a device
    /// that does not list them — from the one pass over
    /// `project.devices.com_objects()` that enumerates every instance,
    /// orphans included.
    pub orphan_com_objects: Vec<ComObjectInstanceId>,
    pub warnings: Vec<ReportWarning>,
    pub counts: Counts,
}

/// The Summary section's row of counts, each a plain total across the whole
/// project. `com_objects` counts every instance `Devices` holds, including
/// orphans — the same enumeration the inverse index walks.
pub(crate) struct Counts {
    pub installations: usize,
    pub areas: usize,
    pub lines: usize,
    pub devices: usize,
    pub com_objects: usize,
    pub group_ranges: usize,
    pub group_addresses: usize,
    pub building_parts: usize,
    pub parameter_values: usize,
}

/// The derived shape of one `Installation`: buildings resolved into
/// roots/orphans, group ranges resolved into roots/children, addresses
/// filed under their innermost containing range (or not), and the inverse
/// link index restricted to this installation's own group addresses.
pub(crate) struct InstallationModel {
    pub building_roots: Vec<BuildingPartId>,
    /// A building part whose `parent` names an id this installation does
    /// not hold. Distinct from a root (`parent: None`) — conflating the two
    /// would hide a real data problem behind ordinary top-level state.
    pub orphan_building_parts: Vec<BuildingPartId>,
    pub range_roots: Vec<GroupRangeId>,
    /// Keyed by the parent range's id; present only for a range that
    /// actually has children.
    pub range_children: BTreeMap<GroupRangeId, Vec<GroupRangeId>>,
    /// A group address filed under the narrowest range that contains it —
    /// a middle range wins over the main range enclosing it.
    pub addresses_by_range: BTreeMap<GroupRangeId, Vec<GroupAddressId>>,
    pub addresses_without_range: Vec<GroupAddressId>,
    /// Every communication object instance linked to a given address, in
    /// `ComObjectInstanceId` order. An address with no links is **absent**
    /// from this map, never present with an empty `Vec` — the renderer
    /// treats a missing key as "no links".
    pub links_by_address: BTreeMap<GroupAddressId, Vec<ComObjectInstanceId>>,
}

pub(crate) fn build(project: &Project) -> ReportModel {
    let style = project.info.group_address_style;
    let mut warnings = Vec::new();

    let mut installations: Vec<InstallationModel> = project
        .installations
        .iter()
        .map(|installation| build_installation(installation, style, &mut warnings))
        .collect();

    for installation in &project.installations {
        for device_id in &installation.topology.unassigned {
            warnings.push(ReportWarning {
                location: format!("device {device_id}"),
                detail: "assigned to no line (Topology::unassigned)".to_string(),
            });
        }
    }

    let ga_installation = index_group_addresses_by_installation(project);
    let orphan_com_objects =
        index_com_objects(project, &ga_installation, &mut installations, &mut warnings);

    let counts = compute_counts(project);

    ReportModel {
        installations,
        orphan_com_objects,
        warnings,
        counts,
    }
}

fn build_installation(
    installation: &Installation,
    style: GroupAddressStyle,
    warnings: &mut Vec<ReportWarning>,
) -> InstallationModel {
    let (building_roots, orphan_building_parts) =
        build_building_forest(&installation.buildings, warnings);
    let (range_roots, range_children) = build_range_forest(&installation.group_ranges, warnings);
    let (addresses_by_range, addresses_without_range) = place_addresses(
        &installation.group_addresses,
        &installation.group_ranges,
        style,
        warnings,
    );

    InstallationModel {
        building_roots,
        orphan_building_parts,
        range_roots,
        range_children,
        addresses_by_range,
        addresses_without_range,
        links_by_address: BTreeMap::new(),
    }
}

/// Building parts are stored flat, linked by `parent`/`children` ids
/// (`building.rs:25`). Roots are the parts with `parent: None`, in stored
/// order; a part whose `parent` names an id the lookup does not contain is
/// an orphan, not a root — those are different findings and must not be
/// conflated. Mirrors `build_building_forest`'s technique
/// (`knx-projection/src/lib.rs:396-428`), minus the recursive node build:
/// `render.rs` walks the nesting itself from each root's own `.children`.
fn build_building_forest(
    parts: &[BuildingPart],
    warnings: &mut Vec<ReportWarning>,
) -> (Vec<BuildingPartId>, Vec<BuildingPartId>) {
    let by_id: HashMap<BuildingPartId, &BuildingPart> = parts.iter().map(|p| (p.id, p)).collect();

    let mut roots = Vec::new();
    let mut orphans = Vec::new();
    for part in parts {
        match part.parent {
            None => roots.push(part.id),
            Some(parent_id) if !by_id.contains_key(&parent_id) => {
                orphans.push(part.id);
                warnings.push(ReportWarning {
                    location: format!("building part {}", part.id),
                    detail: format!("parent building part {parent_id} does not exist"),
                });
            }
            Some(_) => {
                // An ordinary child: neither a root nor an orphan. Reached
                // through its resolvable parent's own `children` list.
            }
        }
    }
    (roots, orphans)
}

/// `GroupRange` already stores both directions of its own nesting
/// (`group.rs:12-24`), so the roots/children split is mostly a direct read
/// — plus a presence check, because a `parent`/`children` id can dangle
/// (name a range this installation does not hold) the same way a
/// `BuildingPart::parent` can. Unlike the building case there is no
/// separate orphan list to put the finding in: a range with a dangling
/// `parent` still needs somewhere to hang in the document, so it becomes a
/// root instead, with a warning explaining why; a dangling `children`
/// entry is simply dropped, with a warning, since there is nothing for
/// `render.rs` to descend into. CLAUDE.md: never silently discard a
/// structural oddity.
fn build_range_forest(
    ranges: &[GroupRange],
    warnings: &mut Vec<ReportWarning>,
) -> (Vec<GroupRangeId>, BTreeMap<GroupRangeId, Vec<GroupRangeId>>) {
    let present: HashSet<GroupRangeId> = ranges.iter().map(|r| r.id).collect();

    let mut roots = Vec::new();
    for r in ranges {
        match r.parent {
            None => roots.push(r.id),
            Some(parent_id) if !present.contains(&parent_id) => {
                roots.push(r.id);
                warnings.push(ReportWarning {
                    location: format!("group range {}", r.id),
                    detail: format!(
                        "parent group range {parent_id} does not exist in this installation; treated as a root"
                    ),
                });
            }
            Some(_) => {
                // An ordinary child: reached through its resolvable
                // parent's own (filtered) `children` entry below.
            }
        }
    }

    let mut children = BTreeMap::new();
    for r in ranges {
        let mut kept = Vec::with_capacity(r.children.len());
        for &child_id in &r.children {
            if present.contains(&child_id) {
                kept.push(child_id);
            } else {
                warnings.push(ReportWarning {
                    location: format!("group range {}", r.id),
                    detail: format!(
                        "child group range {child_id} does not exist in this installation; omitted"
                    ),
                });
            }
        }
        if !kept.is_empty() {
            children.insert(r.id, kept);
        }
    }
    (roots, children)
}

/// The `GroupRange` with the narrowest `start..=end` that contains
/// `address` — a middle range wins over the main range enclosing it, the
/// same rule `knx-csv`'s planner uses (`plan.rs:188-199`).
fn innermost_containing_range(
    ranges: &[GroupRange],
    address: GroupAddress,
) -> Option<GroupRangeId> {
    ranges
        .iter()
        .filter(|r| r.contains(address))
        .min_by_key(|r| r.end.raw() - r.start.raw())
        .map(|r| r.id)
}

fn place_addresses(
    entries: &[GroupAddressEntry],
    ranges: &[GroupRange],
    style: GroupAddressStyle,
    warnings: &mut Vec<ReportWarning>,
) -> (
    BTreeMap<GroupRangeId, Vec<GroupAddressId>>,
    Vec<GroupAddressId>,
) {
    let mut by_range: BTreeMap<GroupRangeId, Vec<GroupAddressId>> = BTreeMap::new();
    let mut without_range = Vec::new();

    for entry in entries {
        match innermost_containing_range(ranges, entry.address) {
            Some(range_id) => by_range.entry(range_id).or_default().push(entry.id),
            None => {
                without_range.push(entry.id);
                warnings.push(ReportWarning {
                    location: format!("group address {}", entry.address.format(style)),
                    detail: "not inside any group range".to_string(),
                });
            }
        }
    }

    (by_range, without_range)
}

/// A lookup index only — never iterated to emit output, so it does not
/// violate the "no `HashMap` drives output order" rule. Group address ids
/// are project-unique, so one pass over every installation's
/// `group_addresses` is enough to route a `GroupLink` to its owning
/// installation, the same problem `find_group_address_entry`
/// (`knx-projection/src/lib.rs:372-380`) solves by linear scan.
fn index_group_addresses_by_installation(project: &Project) -> HashMap<GroupAddressId, usize> {
    let mut index = HashMap::new();
    for (i, installation) in project.installations.iter().enumerate() {
        for entry in &installation.group_addresses {
            index.insert(entry.id, i);
        }
    }
    index
}

/// One pass over `project.devices.com_objects()` (`devices.rs:64`), which
/// enumerates every instance including orphans — that is exactly why
/// orphan detection is possible here, and must not be replaced by a walk
/// over each device's own `com_objects` list. Builds the inverse
/// group-address → communication-object index in the same pass.
fn index_com_objects(
    project: &Project,
    ga_installation: &HashMap<GroupAddressId, usize>,
    installations: &mut [InstallationModel],
    warnings: &mut Vec<ReportWarning>,
) -> Vec<ComObjectInstanceId> {
    let mut orphans = Vec::new();

    for com in project.devices.com_objects() {
        let owned = match project.devices.get(com.device) {
            None => {
                warnings.push(ReportWarning {
                    location: format!("communication object {}", com.id),
                    detail: format!("device {} does not exist", com.device),
                });
                false
            }
            Some(device) => {
                if device.com_objects.contains(&com.id) {
                    true
                } else {
                    warnings.push(ReportWarning {
                        location: format!("communication object {}", com.id),
                        detail: format!(
                            "device {} does not list this communication object",
                            com.device
                        ),
                    });
                    false
                }
            }
        };
        if !owned {
            orphans.push(com.id);
        }

        for link in &com.links {
            match ga_installation.get(&link.ga) {
                Some(&idx) => installations[idx]
                    .links_by_address
                    .entry(link.ga)
                    .or_default()
                    .push(com.id),
                None => warnings.push(ReportWarning {
                    location: format!("communication object {}", com.id),
                    detail: format!(
                        "links to group address {}, which no installation holds",
                        link.ga
                    ),
                }),
            }
        }
    }

    orphans
}

fn compute_counts(project: &Project) -> Counts {
    let mut counts = Counts {
        installations: 0,
        areas: 0,
        lines: 0,
        devices: project.devices.iter().count(),
        com_objects: project.devices.com_objects().count(),
        group_ranges: 0,
        group_addresses: 0,
        building_parts: 0,
        parameter_values: 0,
    };

    for installation in &project.installations {
        counts.installations += 1;
        counts.areas += installation.topology.areas.len();
        counts.lines += installation.topology.lines.len();
        counts.group_ranges += installation.group_ranges.len();
        counts.group_addresses += installation.group_addresses.len();
        counts.building_parts += installation.buildings.len();
        counts.parameter_values += installation.parameters.len();
    }

    counts
}

#[cfg(test)]
mod tests {
    use knx_core::{BuildingPartType, DeviceId, GroupAddressStyle, IndividualAddress};

    use super::*;
    use crate::testutil::{
        building_part, device, entry, linked_com_object, range, unlinked_com_object,
    };

    #[test]
    fn building_forest_nests_three_levels_with_one_root_and_no_orphans() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        let building = building_part(1, "Building", BuildingPartType::Building, None, &[2]);
        let floor = building_part(2, "Floor", BuildingPartType::Floor, Some(1), &[3]);
        let room = building_part(3, "Room", BuildingPartType::Room, Some(2), &[]);
        project.installations[0].buildings = vec![building, floor, room];

        let model = build(&project);
        let inst = &model.installations[0];

        assert_eq!(inst.building_roots, vec![BuildingPartId(1)]);
        assert!(inst.orphan_building_parts.is_empty());
    }

    #[test]
    fn a_building_part_with_a_dangling_parent_is_orphaned_and_warns() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        let stray = building_part(5, "Stray", BuildingPartType::Room, Some(99), &[]);
        project.installations[0].buildings = vec![stray];

        let model = build(&project);
        let inst = &model.installations[0];

        assert!(inst.building_roots.is_empty());
        assert_eq!(inst.orphan_building_parts, vec![BuildingPartId(5)]);
        assert!(model
            .warnings
            .iter()
            .any(|w| w.location.contains("building part 5") && w.detail.contains("99")));
    }

    #[test]
    fn a_main_range_with_two_middle_ranges_yields_one_root_with_children_in_stored_order() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        let main = range(1, "Main", 0, 4095, None, &[2, 3]);
        let middle_a = range(2, "A", 0, 2047, Some(1), &[]);
        let middle_b = range(3, "B", 2048, 4095, Some(1), &[]);
        project.installations[0].group_ranges = vec![main, middle_a, middle_b];

        let model = build(&project);
        let inst = &model.installations[0];

        assert_eq!(inst.range_roots, vec![GroupRangeId(1)]);
        assert_eq!(
            inst.range_children.get(&GroupRangeId(1)),
            Some(&vec![GroupRangeId(2), GroupRangeId(3)])
        );
    }

    #[test]
    fn a_group_range_with_a_dangling_parent_becomes_a_root_and_warns() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        let stray = range(5, "Stray", 0, 100, Some(99), &[]);
        project.installations[0].group_ranges = vec![stray];

        let model = build(&project);
        let inst = &model.installations[0];

        // Reachable, not merely warned about: it must actually be a root,
        // or `render.rs`'s root-then-descend walk will never visit it.
        assert_eq!(inst.range_roots, vec![GroupRangeId(5)]);
        assert!(model
            .warnings
            .iter()
            .any(|w| w.location.contains("group range 5") && w.detail.contains("99")));
    }

    #[test]
    fn a_group_range_with_a_dangling_child_drops_it_and_warns() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        let main = range(1, "Main", 0, 4095, None, &[2, 99]);
        let real_child = range(2, "Real", 0, 2047, Some(1), &[]);
        project.installations[0].group_ranges = vec![main, real_child];

        let model = build(&project);
        let inst = &model.installations[0];

        assert_eq!(
            inst.range_children.get(&GroupRangeId(1)),
            Some(&vec![GroupRangeId(2)])
        );
        assert!(model
            .warnings
            .iter()
            .any(|w| w.location.contains("group range 1") && w.detail.contains("99")));
    }

    #[test]
    fn an_address_inside_a_middle_range_files_under_the_middle_range_not_the_main_range() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        let main = range(1, "Main", 0, 4095, None, &[2]);
        let middle = range(2, "Middle", 0, 2047, Some(1), &[]);
        project.installations[0].group_ranges = vec![main, middle];
        project.installations[0].group_addresses = vec![entry(1, 100, "GA")];

        let model = build(&project);
        let inst = &model.installations[0];

        assert_eq!(
            inst.addresses_by_range.get(&GroupRangeId(2)),
            Some(&vec![GroupAddressId(1)])
        );
        assert!(!inst.addresses_by_range.contains_key(&GroupRangeId(1)));
        assert!(inst.addresses_without_range.is_empty());
    }

    #[test]
    fn an_address_inside_no_range_lands_in_addresses_without_range_and_warns() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        project.installations[0].group_addresses = vec![entry(1, 100, "GA")];

        let model = build(&project);
        let inst = &model.installations[0];

        assert_eq!(inst.addresses_without_range, vec![GroupAddressId(1)]);
        assert!(model
            .warnings
            .iter()
            .any(
                |w| w.location.contains("group address") && w.detail.contains("no group range")
                    || w.detail.contains("not inside any group range")
            ));
    }

    #[test]
    fn links_by_address_lists_every_linked_com_object_in_id_order_and_omits_unlinked_addresses() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        project.installations[0].group_addresses =
            vec![entry(1, 100, "Linked"), entry(2, 200, "Unlinked")];
        project.devices.insert(device(1, &[10, 11]));
        // Inserted out of id order to prove the map orders by id, not
        // insertion order.
        project
            .devices
            .insert_com_object(linked_com_object(11, 1, 1));
        project
            .devices
            .insert_com_object(linked_com_object(10, 1, 1));

        let model = build(&project);
        let inst = &model.installations[0];

        assert_eq!(
            inst.links_by_address.get(&GroupAddressId(1)),
            Some(&vec![ComObjectInstanceId(10), ComObjectInstanceId(11)])
        );
        assert!(!inst.links_by_address.contains_key(&GroupAddressId(2)));
        assert!(model.orphan_com_objects.is_empty());
    }

    #[test]
    fn a_com_object_whose_device_does_not_exist_is_orphaned_and_warns() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        project
            .devices
            .insert_com_object(unlinked_com_object(1, 99));

        let model = build(&project);

        assert_eq!(model.orphan_com_objects, vec![ComObjectInstanceId(1)]);
        assert!(model
            .warnings
            .iter()
            .any(|w| w.location.contains("communication object 1")
                && w.detail.contains("does not exist")));
    }

    #[test]
    fn a_com_object_its_device_does_not_list_is_orphaned_and_warns() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        project.devices.insert(device(1, &[])); // does not list com object 7
        project.devices.insert_com_object(unlinked_com_object(7, 1));

        let model = build(&project);

        assert_eq!(model.orphan_com_objects, vec![ComObjectInstanceId(7)]);
        assert!(model
            .warnings
            .iter()
            .any(|w| w.location.contains("communication object 7")
                && w.detail.contains("does not list")));
    }

    #[test]
    fn a_group_link_naming_an_unknown_group_address_warns() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        project.devices.insert(device(1, &[1]));
        project
            .devices
            .insert_com_object(linked_com_object(1, 1, 404));

        let model = build(&project);

        assert!(model
            .warnings
            .iter()
            .any(|w| w.detail.contains("404") && w.detail.contains("no installation holds")));
    }

    #[test]
    fn a_topology_unassigned_device_warns_but_an_address_less_device_does_not() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        project.installations[0].topology.unassigned = vec![DeviceId(1)];
        let mut with_address = device(1, &[]);
        with_address.address = Some(IndividualAddress::from_raw(1));
        project.devices.insert(with_address);
        // A second device, never mentioned in `unassigned`, with no
        // individual address at all — valid state, must not warn.
        project.devices.insert(device(2, &[]));

        let model = build(&project);

        assert!(model
            .warnings
            .iter()
            .any(|w| w.location.contains("device 1") && w.detail.contains("unassigned")));
        assert!(!model
            .warnings
            .iter()
            .any(|w| w.location.contains("device 2")));
    }

    #[test]
    fn counts_matches_a_hand_counted_small_project() {
        let mut project = crate::testutil::empty_project(GroupAddressStyle::Free);
        project.installations[0].buildings = vec![
            building_part(1, "Building", BuildingPartType::Building, None, &[]),
            building_part(2, "Room", BuildingPartType::Room, Some(1), &[]),
        ];
        project.installations[0].group_ranges = vec![
            range(1, "Main", 0, 4095, None, &[]),
            range(2, "Mid", 0, 2047, None, &[]),
        ];
        project.installations[0].group_addresses =
            vec![entry(1, 100, "A"), entry(2, 200, "B"), entry(3, 300, "C")];
        project.devices.insert(device(1, &[1]));
        project.devices.insert(device(2, &[]));
        project
            .devices
            .insert_com_object(linked_com_object(1, 1, 1));
        project
            .devices
            .insert_com_object(unlinked_com_object(2, 99)); // orphan, still counted

        let model = build(&project);

        assert_eq!(model.counts.installations, 1);
        assert_eq!(model.counts.areas, 0);
        assert_eq!(model.counts.lines, 0);
        assert_eq!(model.counts.devices, 2);
        assert_eq!(model.counts.com_objects, 2);
        assert_eq!(model.counts.group_ranges, 2);
        assert_eq!(model.counts.group_addresses, 3);
        assert_eq!(model.counts.building_parts, 2);
        assert_eq!(model.counts.parameter_values, 0);
    }
}
