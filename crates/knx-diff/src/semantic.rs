//! Per-entity keys, field snapshots, and the extraction functions that turn
//! a `knx_core` entity into one of each. This is the direct, independent
//! reimplementation of `crates/knx-etsproj/src/compare.rs`'s field-
//! resolution techniques (`semantic_text`/`semantic_dpt`/`semantic_flag`,
//! `compare.rs:356-393`) against `knx_core` directly — never a call into
//! `knx-etsproj`, which `knx-diff` must never depend on (design spec §6).
//!
//! Nothing here calls `key::match_entities` — that is Task 3's job, once
//! these types are wired into `diff.rs`'s `EntityTable`s. A `*Key` type is
//! `Ord` so it can serve as `match_entities`' `NK` type parameter and so
//! output lists can be sorted deterministically (design spec §4); a
//! `*Fields` type is not, since nothing sorts by field content.
//!
use std::collections::BTreeMap;
use std::net::Ipv4Addr;

use knx_core::{
    Area, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance, CommissioningState,
    CompletionStatus, DeviceId, DeviceInstance, Direction, DptRef, GroupAddressEntry,
    GroupAddressId, GroupAddressStyle, GroupRange, GroupRangeId, Line, ModuleInstanceId, Override,
    ParameterInstance, StringTable, Text, Topology,
};

/// Compares `left`/`right` field by field, in the exact order given, and
/// returns the names of the fields that differ — the same order every time
/// (design spec §4), so nothing downstream needs its own sort step.
macro_rules! changed_fields {
    ($left:expr, $right:expr, [$($field:ident),+ $(,)?]) => {{
        let mut changed: Vec<&'static str> = Vec::new();
        $(
            if $left.$field != $right.$field {
                changed.push(stringify!($field));
            }
        )+
        changed
    }};
}

// ---------------------------------------------------------------------
// Area
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct AreaKey {
    pub address: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AreaFields {
    pub name: String,
    pub completion: CompletionStatus,
}

pub fn area_key(area: &Area) -> AreaKey {
    AreaKey {
        address: area.address,
    }
}

pub fn area_fields(area: &Area) -> AreaFields {
    AreaFields {
        name: area.name.clone(),
        completion: area.completion,
    }
}

pub fn area_changed_fields(left: &AreaFields, right: &AreaFields) -> Vec<&'static str> {
    changed_fields!(left, right, [name, completion])
}

// ---------------------------------------------------------------------
// Line
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct LineKey {
    pub area_address: u8,
    pub line_address: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LineFields {
    pub name: String,
    pub medium_ref: String,
    pub domain_address: Option<String>,
    pub domain_address_is_checked: Option<bool>,
    pub ip_routing_multicast_address: Option<Ipv4Addr>,
    pub multicast_ttl: Option<u8>,
    pub completion: CompletionStatus,
    /// `None` only if the line is unreachable from any `Area` in its
    /// installation's topology — should not occur for well-formed data,
    /// but this stays honest about it rather than panicking or guessing.
    pub area: Option<AreaKey>,
}

/// `None` when `line` is not referenced by any `Area` in `topology` — see
/// `LineFields::area`'s own doc comment for why that is reported rather
/// than assumed away.
pub fn line_key(line: &Line, topology: &Topology) -> Option<LineKey> {
    let area = topology.area_of(line.id)?;
    Some(LineKey {
        area_address: area.address,
        line_address: line.address,
    })
}

pub fn line_fields(line: &Line, topology: &Topology) -> LineFields {
    LineFields {
        name: line.name.clone(),
        medium_ref: line.medium_ref.clone(),
        domain_address: line.domain_address.clone(),
        domain_address_is_checked: line.domain_address_is_checked,
        ip_routing_multicast_address: line.ip_routing_multicast_address,
        multicast_ttl: line.multicast_ttl,
        completion: line.completion,
        area: topology.area_of(line.id).map(area_key),
    }
}

pub fn line_changed_fields(left: &LineFields, right: &LineFields) -> Vec<&'static str> {
    changed_fields!(
        left,
        right,
        [
            name,
            medium_ref,
            domain_address,
            domain_address_is_checked,
            ip_routing_multicast_address,
            multicast_ttl,
            completion,
            area,
        ]
    )
}

/// The line, if any, whose `devices` contains `device`. `None` means the
/// device is presumed to live in `Topology::unassigned` — valid project
/// state, not an error (`topology.rs:44`).
pub fn line_of_device(device: DeviceId, topology: &Topology) -> Option<LineKey> {
    let line = topology
        .lines
        .iter()
        .find(|l| l.devices.contains(&device))?;
    line_key(line, topology)
}

// ---------------------------------------------------------------------
// Building part
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct BuildingPartKey {
    /// Root-first names, e.g. `["Building A", "Floor 1", "Room 3"]`.
    pub path: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BuildingPartFields {
    pub name: String,
    pub number: Option<String>,
    pub kind: BuildingPartType,
    pub completion: CompletionStatus,
    pub default_line: Option<LineKey>,
}

/// Walks `parent` from `id` to the root through `by_id`, collecting names,
/// then reverses so the result reads root-first — the same technique
/// `compare.rs:429-459`'s `semantic_building_part` uses for one parent hop,
/// extended here to the full chain. A dangling parent id (not present in
/// `by_id`) simply ends the walk early rather than panicking.
pub fn building_part_path(
    id: BuildingPartId,
    by_id: &BTreeMap<BuildingPartId, &BuildingPart>,
) -> Vec<String> {
    let mut names = Vec::new();
    let mut current = Some(id);
    while let Some(current_id) = current {
        let Some(part) = by_id.get(&current_id) else {
            break;
        };
        names.push(part.name.clone());
        current = part.parent;
    }
    names.reverse();
    names
}

pub fn building_part_key(
    part: &BuildingPart,
    by_id: &BTreeMap<BuildingPartId, &BuildingPart>,
) -> BuildingPartKey {
    BuildingPartKey {
        path: building_part_path(part.id, by_id),
    }
}

pub fn building_part_fields(part: &BuildingPart, topology: &Topology) -> BuildingPartFields {
    BuildingPartFields {
        name: part.name.clone(),
        number: part.number.clone(),
        kind: part.kind,
        completion: part.completion,
        default_line: part
            .default_line
            .and_then(|id| topology.line(id))
            .and_then(|line| line_key(line, topology)),
    }
}

pub fn building_part_changed_fields(
    left: &BuildingPartFields,
    right: &BuildingPartFields,
) -> Vec<&'static str> {
    changed_fields!(left, right, [name, number, kind, completion, default_line])
}

// ---------------------------------------------------------------------
// Device
// ---------------------------------------------------------------------

// Spelled out verbatim from design spec §6 — do not add fields.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DeviceKey {
    pub ets_id: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeviceFields {
    pub name: String,
    pub description: Option<String>,
    /// Formatted `IndividualAddress`, when present.
    pub address: Option<String>,
    pub product_ref: String,
    pub program_ref: String,
    pub commissioning: CommissioningState,
    pub line: Option<LineKey>,
    pub building: Option<BuildingPartKey>,
}

/// `ets_id` collapses an empty `SourceRef::ets_id` to `None`, the same
/// `Some(..).filter(|s| !s.is_empty())` idiom `compare.rs:295-296` already
/// uses for `product_ref`/`program_ref` — an empty id is not a usable
/// identity any more than an absent one.
pub fn device_key(device: &DeviceInstance) -> DeviceKey {
    DeviceKey {
        ets_id: Some(device.source.ets_id.clone()).filter(|s| !s.is_empty()),
        address: device.address.map(|a| a.to_string()),
    }
}

pub fn device_fields(
    device: &DeviceInstance,
    line: Option<LineKey>,
    building: Option<BuildingPartKey>,
) -> DeviceFields {
    DeviceFields {
        name: device.name.clone(),
        description: device.description.clone(),
        address: device.address.map(|a| a.to_string()),
        product_ref: device.product_ref.clone(),
        program_ref: device.program_ref.clone(),
        commissioning: device.commissioning.clone(),
        line,
        building,
    }
}

pub fn device_changed_fields(left: &DeviceFields, right: &DeviceFields) -> Vec<&'static str> {
    changed_fields!(
        left,
        right,
        [
            name,
            description,
            address,
            product_ref,
            program_ref,
            commissioning,
            line,
            building,
        ]
    )
}

// ---------------------------------------------------------------------
// Group range
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupRangeKey {
    pub start: u16,
    pub end: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupRangeFields {
    pub name: String,
    pub start: u16,
    pub end: u16,
    pub parent: Option<GroupRangeKey>,
}

pub fn group_range_key(range: &GroupRange) -> GroupRangeKey {
    GroupRangeKey {
        start: range.start.raw(),
        end: range.end.raw(),
    }
}

pub fn group_range_fields(
    range: &GroupRange,
    ranges_by_id: &BTreeMap<GroupRangeId, &GroupRange>,
) -> GroupRangeFields {
    GroupRangeFields {
        name: range.name.clone(),
        start: range.start.raw(),
        end: range.end.raw(),
        parent: range
            .parent
            .and_then(|id| ranges_by_id.get(&id))
            .map(|r| group_range_key(r)),
    }
}

pub fn group_range_changed_fields(
    left: &GroupRangeFields,
    right: &GroupRangeFields,
) -> Vec<&'static str> {
    changed_fields!(left, right, [name, start, end, parent])
}

// ---------------------------------------------------------------------
// Group address
// ---------------------------------------------------------------------

// Spelled out verbatim from design spec §6.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct GroupAddressKey {
    pub ets_id: Option<String>,
    /// Formatted per the project's own `GroupAddressStyle` (design spec
    /// §6's "one address-formatting note").
    pub address: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupAddressFields {
    pub name: String,
    pub central: bool,
    pub unfiltered: bool,
    pub range: Option<GroupRangeKey>,
}

pub fn group_address_key(entry: &GroupAddressEntry, style: GroupAddressStyle) -> GroupAddressKey {
    GroupAddressKey {
        ets_id: Some(entry.source.ets_id.clone()).filter(|s| !s.is_empty()),
        address: entry.address.format(style),
    }
}

pub fn group_address_fields(
    entry: &GroupAddressEntry,
    ranges_by_id: &BTreeMap<GroupRangeId, &GroupRange>,
) -> GroupAddressFields {
    GroupAddressFields {
        name: entry.name.clone(),
        central: entry.central,
        unfiltered: entry.unfiltered,
        range: entry
            .range
            .and_then(|id| ranges_by_id.get(&id))
            .map(|r| group_range_key(r)),
    }
}

pub fn group_address_changed_fields(
    left: &GroupAddressFields,
    right: &GroupAddressFields,
) -> Vec<&'static str> {
    changed_fields!(left, right, [name, central, unfiltered, range])
}

// ---------------------------------------------------------------------
// Override resolution — reimplemented directly against `knx_core`, not
// called from `knx-etsproj::compare` (module header note).
// ---------------------------------------------------------------------

/// `Absent` -> `None`; `Empty` -> `Some(String::new())`; `Value` where
/// `layer.is_exported()` -> `Some(resolved text)`; `Value` otherwise ->
/// `None`; `Malformed(raw)` -> `Some(raw.clone())`. Matches
/// `compare.rs:356-369`'s `semantic_text` exactly.
pub fn semantic_text(text: &Override<Text>, strings: &StringTable) -> Option<String> {
    match text {
        Override::Absent => None,
        Override::Empty => Some(String::new()),
        Override::Value(r) if r.layer.is_exported() => Some(
            strings
                .text(&r.value, strings.default_language())
                .unwrap_or_default()
                .to_string(),
        ),
        Override::Value(_) => None,
        Override::Malformed(raw) => Some(raw.clone()),
    }
}

/// Same rule as [`semantic_text`], for a datapoint type reference. Matches
/// `compare.rs:371-379`'s `semantic_dpt` exactly.
pub fn semantic_dpt(dpt: &Override<DptRef>) -> Option<String> {
    match dpt {
        Override::Absent => None,
        Override::Empty => Some(String::new()),
        Override::Value(r) if r.layer.is_exported() => Some(r.value.to_string()),
        Override::Value(_) => None,
        Override::Malformed(raw) => Some(raw.clone()),
    }
}

/// `Value` where `layer.is_exported()` -> `Some(value)`; every other case
/// (`Absent`, `Empty`, `Value` not exported, `Malformed`) -> `None` — the
/// same narrow collapse `compare.rs:389-393` documents and accepts for its
/// own `semantic_flag`, because `Option<bool>` has no third state to hold
/// the "present but unreadable/empty" distinction in.
pub fn semantic_flag(flag: &Override<bool>) -> Option<bool> {
    match flag {
        Override::Value(r) if r.layer.is_exported() => Some(r.value),
        _ => None,
    }
}

// ---------------------------------------------------------------------
// Communication object
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ComObjectKey {
    pub device: DeviceKey,
    pub number: u16,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComObjectFields {
    pub text: Option<String>,
    pub description: Option<String>,
    pub dpt: Option<String>,
    pub read: Option<bool>,
    pub write: Option<bool>,
    pub transmit: Option<bool>,
    pub update: Option<bool>,
    pub communication: Option<bool>,
    /// The sixth flag (§117). `None` where nothing stated it, which is most
    /// of the time — it is normally a product-database value.
    pub read_on_init: Option<bool>,
    /// Sorted by group address key, then direction.
    pub links: Vec<(GroupAddressKey, Direction)>,
    /// The linked module instance's own `ets_id`, `None` for a
    /// schema-11-shaped device.
    pub module_instance: Option<String>,
}

pub fn com_object_key(com: &ComObjectInstance, device: DeviceKey) -> ComObjectKey {
    ComObjectKey {
        device,
        number: com.number,
    }
}

pub fn com_object_fields(
    com: &ComObjectInstance,
    strings: &StringTable,
    ga_keys: &BTreeMap<GroupAddressId, GroupAddressKey>,
    module_ets_ids: &BTreeMap<ModuleInstanceId, String>,
) -> ComObjectFields {
    let mut links: Vec<(GroupAddressKey, Direction)> = com
        .links
        .iter()
        .filter_map(|link| {
            ga_keys
                .get(&link.ga)
                .map(|key| (key.clone(), link.direction))
        })
        .collect();
    links.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(direction_rank(a.1).cmp(&direction_rank(b.1)))
    });

    ComObjectFields {
        text: semantic_text(&com.text, strings),
        description: semantic_text(&com.description, strings),
        dpt: semantic_dpt(&com.dpt),
        read: semantic_flag(&com.flags.read),
        write: semantic_flag(&com.flags.write),
        transmit: semantic_flag(&com.flags.transmit),
        update: semantic_flag(&com.flags.update),
        communication: semantic_flag(&com.flags.communication),
        read_on_init: semantic_flag(&com.flags.read_on_init),
        links,
        module_instance: com
            .module_instance
            .and_then(|id| module_ets_ids.get(&id))
            .cloned(),
    }
}

fn direction_rank(direction: Direction) -> u8 {
    match direction {
        Direction::Send => 0,
        Direction::Receive => 1,
    }
}

pub fn com_object_changed_fields(
    left: &ComObjectFields,
    right: &ComObjectFields,
) -> Vec<&'static str> {
    changed_fields!(
        left,
        right,
        [
            text,
            description,
            dpt,
            read,
            write,
            transmit,
            update,
            communication,
            read_on_init,
            links,
            module_instance,
        ]
    )
}

// ---------------------------------------------------------------------
// Parameter instance
// ---------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ParameterKey {
    pub device: DeviceKey,
    pub ets_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ParameterFields {
    pub raw: String,
}

pub fn parameter_key(param: &ParameterInstance, device: DeviceKey) -> ParameterKey {
    ParameterKey {
        device,
        ets_id: param.source.ets_id.clone(),
    }
}

pub fn parameter_fields(param: &ParameterInstance) -> ParameterFields {
    ParameterFields {
        raw: param.raw.clone(),
    }
}

pub fn parameter_changed_fields(
    left: &ParameterFields,
    right: &ParameterFields,
) -> Vec<&'static str> {
    changed_fields!(left, right, [raw])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::*;
    use knx_core::{BuildingPartType, Language, Layer};

    // -----------------------------------------------------------------
    // *_key tests
    // -----------------------------------------------------------------

    #[test]
    fn area_key_uses_the_raw_address() {
        let mut project = project();
        let area_id = add_area(&mut project, 1, 3);
        let area = project.installations[0]
            .topology
            .areas
            .iter()
            .find(|a| a.id == area_id)
            .unwrap();

        assert_eq!(area_key(area), AreaKey { address: 3 });
    }

    #[test]
    fn area_fields_carries_name_and_completion() {
        let mut project = project();
        let area_id = add_area(&mut project, 1, 3);
        let area = project.installations[0]
            .topology
            .areas
            .iter_mut()
            .find(|a| a.id == area_id)
            .unwrap();
        area.completion = CompletionStatus::Accepted;

        let fields = area_fields(area);
        assert_eq!(fields.name, "Area 3");
        assert_eq!(fields.completion, CompletionStatus::Accepted);
    }

    #[test]
    fn line_key_resolves_via_topology_area_of() {
        let mut project = project();
        let area_id = add_area(&mut project, 1, 1);
        let line_id = add_line(&mut project, area_id, 1, 2);
        let topology = &project.installations[0].topology;
        let line = topology.line(line_id).unwrap();

        assert_eq!(
            line_key(line, topology),
            Some(LineKey {
                area_address: 1,
                line_address: 2,
            })
        );
    }

    #[test]
    fn line_key_is_none_for_a_line_unreachable_from_any_area() {
        let line = full_line(1, 5);
        let topology = Topology {
            areas: vec![],
            lines: vec![line.clone()],
            unassigned: vec![],
        };

        assert_eq!(line_key(&line, &topology), None);
    }

    #[test]
    fn line_fields_resolves_placement_and_medium_configuration() {
        let mut project = project();
        let area_id = add_area(&mut project, 1, 1);
        let line_id = add_line(&mut project, area_id, 1, 2);
        let topology = &project.installations[0].topology;
        let line = topology.line(line_id).unwrap();

        let fields = line_fields(line, topology);
        assert_eq!(fields.name, "Line 2");
        assert_eq!(fields.medium_ref, "MT-0");
        assert_eq!(fields.area, Some(AreaKey { address: 1 }));
    }

    #[test]
    fn line_of_device_finds_the_owning_line() {
        let mut project = project();
        let area_id = add_area(&mut project, 1, 1);
        let line_id = add_line(&mut project, area_id, 1, 4);
        let device_id = add_device(&mut project, 1, Some(line_id));
        let topology = &project.installations[0].topology;

        assert_eq!(
            line_of_device(device_id, topology),
            Some(LineKey {
                area_address: 1,
                line_address: 4,
            })
        );
    }

    #[test]
    fn line_of_device_returns_none_for_an_unassigned_device() {
        let mut project = project();
        let device_id = add_device(&mut project, 1, None);
        let topology = &project.installations[0].topology;

        assert_eq!(line_of_device(device_id, topology), None);
    }

    #[test]
    fn device_key_uses_ets_id_and_formatted_address() {
        let mut project = project();
        add_device(&mut project, 7, None);
        let device = project.devices.get(DeviceId(7)).unwrap();
        let mut device = device.clone();
        device.address = Some(individual_address(1, 1, 5));

        assert_eq!(
            device_key(&device),
            DeviceKey {
                ets_id: Some("D-7".into()),
                address: Some("1.1.5".into()),
            }
        );
    }

    #[test]
    fn device_key_collapses_an_empty_ets_id_to_none() {
        let mut device = {
            let mut project = project();
            add_device(&mut project, 1, None);
            project.devices.get(DeviceId(1)).unwrap().clone()
        };
        device.source.ets_id = String::new();

        assert_eq!(device_key(&device).ets_id, None);
    }

    #[test]
    fn group_range_key_uses_start_and_end() {
        let mut project = project();
        let range_id = add_group_range(&mut project, 1, "Lighting", 100, 200, None);
        let range = project.installations[0]
            .group_ranges
            .iter()
            .find(|r| r.id == range_id)
            .unwrap();

        assert_eq!(
            group_range_key(range),
            GroupRangeKey {
                start: 100,
                end: 200
            }
        );
    }

    #[test]
    fn group_range_fields_resolves_its_parent_via_the_lookup_map() {
        let mut project = project();
        let parent_id = add_group_range(&mut project, 1, "Lighting", 0, 4095, None);
        let child_id = add_group_range(
            &mut project,
            2,
            "Lighting - On/Off",
            0,
            255,
            Some(parent_id),
        );
        let ranges_by_id: BTreeMap<GroupRangeId, &GroupRange> = project.installations[0]
            .group_ranges
            .iter()
            .map(|r| (r.id, r))
            .collect();
        let child = ranges_by_id[&child_id];

        let fields = group_range_fields(child, &ranges_by_id);
        assert_eq!(fields.name, "Lighting - On/Off");
        assert_eq!(
            fields.parent,
            Some(GroupRangeKey {
                start: 0,
                end: 4095
            })
        );
    }

    #[test]
    fn group_address_key_formats_per_the_given_style() {
        let mut project = project();
        add_group_address(&mut project, 1, (4 << 11) | (2 << 8) | 100, "Light", None);
        let entry = &project.installations[0].group_addresses[0];

        assert_eq!(
            group_address_key(entry, style()),
            GroupAddressKey {
                ets_id: Some("GA-1".into()),
                address: "4/2/100".into(),
            }
        );
    }

    #[test]
    fn group_address_fields_resolves_its_range_via_the_lookup_map() {
        let mut project = project();
        let range_id = add_group_range(&mut project, 1, "Lighting", 0, 4095, None);
        add_group_address(&mut project, 1, 100, "Light on", Some(range_id));
        let ranges_by_id: BTreeMap<GroupRangeId, &GroupRange> = project.installations[0]
            .group_ranges
            .iter()
            .map(|r| (r.id, r))
            .collect();
        let entry = &project.installations[0].group_addresses[0];

        let fields = group_address_fields(entry, &ranges_by_id);
        assert_eq!(fields.name, "Light on");
        assert_eq!(
            fields.range,
            Some(GroupRangeKey {
                start: 0,
                end: 4095
            })
        );
    }

    #[test]
    fn building_part_key_builds_root_first_path() {
        let mut project = project();
        let building = add_building_part(
            &mut project,
            1,
            "Building A",
            BuildingPartType::Building,
            None,
        );
        let floor = add_building_part(
            &mut project,
            2,
            "Floor 1",
            BuildingPartType::Floor,
            Some(building),
        );
        let room = add_building_part(
            &mut project,
            3,
            "Room 3",
            BuildingPartType::Room,
            Some(floor),
        );

        let by_id: BTreeMap<BuildingPartId, &BuildingPart> = project.installations[0]
            .buildings
            .iter()
            .map(|p| (p.id, p))
            .collect();
        let part = by_id[&room];

        assert_eq!(
            building_part_key(part, &by_id).path,
            vec![
                "Building A".to_string(),
                "Floor 1".to_string(),
                "Room 3".to_string()
            ]
        );
    }

    #[test]
    fn building_part_path_walks_to_root_and_reverses() {
        let mut project = project();
        let building = add_building_part(
            &mut project,
            1,
            "Building A",
            BuildingPartType::Building,
            None,
        );
        let floor = add_building_part(
            &mut project,
            2,
            "Floor 1",
            BuildingPartType::Floor,
            Some(building),
        );

        let by_id: BTreeMap<BuildingPartId, &BuildingPart> = project.installations[0]
            .buildings
            .iter()
            .map(|p| (p.id, p))
            .collect();

        assert_eq!(
            building_part_path(floor, &by_id),
            vec!["Building A".to_string(), "Floor 1".to_string()]
        );
    }

    #[test]
    fn com_object_key_pairs_device_key_with_number() {
        let device = DeviceKey {
            ets_id: Some("D-1".into()),
            address: None,
        };
        let com = com_object(1, DeviceId(1), 7);

        assert_eq!(
            com_object_key(&com, device.clone()),
            ComObjectKey { device, number: 7 }
        );
    }

    #[test]
    fn parameter_key_pairs_device_key_with_ets_id() {
        let device = DeviceKey {
            ets_id: Some("D-1".into()),
            address: None,
        };
        let mut project = project();
        let device_id = add_device(&mut project, 1, None);
        let param_id = add_parameter(&mut project, 1, device_id, "M-1_P-1_R-1", "42");
        let param = project.installations[0]
            .parameters
            .iter()
            .find(|p| p.id == param_id)
            .unwrap();

        assert_eq!(
            parameter_key(param, device.clone()),
            ParameterKey {
                device,
                ets_id: "M-1_P-1_R-1".into(),
            }
        );
        assert_eq!(parameter_fields(param).raw, "42");
    }

    // -----------------------------------------------------------------
    // Override resolution
    // -----------------------------------------------------------------

    #[test]
    fn override_value_at_program_layer_resolves_to_none_in_com_object_fields() {
        let strings = StringTable::new(Language("en".into()));
        let text = literal_text("hello", Layer::Program);
        let dpt = dpt_value(1, Some(1), Layer::Program);

        assert_eq!(semantic_text(&text, &strings), None);
        assert_eq!(semantic_dpt(&dpt), None);
    }

    #[test]
    fn override_value_at_instance_or_user_edit_layer_resolves_to_some() {
        let strings = StringTable::new(Language("en".into()));

        for layer in [Layer::Instance, Layer::UserEdit] {
            let text = literal_text("hello", layer);
            let dpt = dpt_value(1, Some(1), layer);
            assert_eq!(semantic_text(&text, &strings), Some("hello".to_string()));
            assert_eq!(semantic_dpt(&dpt), Some("DPST-1-1".to_string()));
        }
    }

    #[test]
    fn semantic_flag_resolves_to_some_only_at_an_exported_layer() {
        assert_eq!(
            semantic_flag(&flag_value(true, Layer::Program)),
            None,
            "program-layer flags are not exported"
        );
        assert_eq!(
            semantic_flag(&flag_value(true, Layer::Instance)),
            Some(true)
        );
        assert_eq!(
            semantic_flag(&flag_value(false, Layer::UserEdit)),
            Some(false)
        );
    }

    #[test]
    fn override_empty_resolves_to_some_empty_string_for_text_and_dpt_but_none_for_flags() {
        let strings = StringTable::new(Language("en".into()));
        let text: Override<Text> = Override::Empty;
        let dpt: Override<DptRef> = Override::Empty;
        let flag: Override<bool> = Override::Empty;

        assert_eq!(semantic_text(&text, &strings), Some(String::new()));
        assert_eq!(semantic_dpt(&dpt), Some(String::new()));
        assert_eq!(semantic_flag(&flag), None);
    }

    #[test]
    fn override_malformed_resolves_to_the_raw_string() {
        let strings = StringTable::new(Language("en".into()));
        let text: Override<Text> = Override::Malformed("garbage ets export".into());
        let dpt: Override<DptRef> = Override::Malformed("garbage ets export".into());

        assert_eq!(
            semantic_text(&text, &strings),
            Some("garbage ets export".to_string())
        );
        assert_eq!(semantic_dpt(&dpt), Some("garbage ets export".to_string()));
    }

    // -----------------------------------------------------------------
    // Device fields
    // -----------------------------------------------------------------

    #[test]
    fn device_fields_excludes_com_objects_and_parameters() {
        let mut project = project();
        let device_id = add_device(&mut project, 1, None);
        let device = project.devices.get(device_id).unwrap();

        let fields = device_fields(device, None, None);
        // Named field-by-field so a future accidental field addition to
        // `DeviceFields` that duplicates `EntityTable` data breaks this
        // test by refusing to compile, not just a reviewer's memory.
        let DeviceFields {
            name,
            description,
            address,
            product_ref,
            program_ref,
            commissioning,
            line,
            building,
        } = fields;
        assert_eq!(name, device.name);
        assert_eq!(description, device.description);
        assert_eq!(address, None);
        assert_eq!(product_ref, device.product_ref);
        assert_eq!(program_ref, device.program_ref);
        assert_eq!(commissioning, device.commissioning);
        assert_eq!(line, None);
        assert_eq!(building, None);
    }

    // -----------------------------------------------------------------
    // Com object links
    // -----------------------------------------------------------------

    #[test]
    fn com_object_links_are_sorted_by_group_address_key_then_direction() {
        let mut project = project();
        add_group_address(&mut project, 1, 100, "A", None);
        add_group_address(&mut project, 2, 200, "B", None);

        let ga_keys: BTreeMap<GroupAddressId, GroupAddressKey> = project.installations[0]
            .group_addresses
            .iter()
            .map(|g| (g.id, group_address_key(g, style())))
            .collect();

        let mut com = com_object(1, DeviceId(1), 0);
        com.links = vec![
            link(2, Direction::Send),
            link(1, Direction::Receive),
            link(1, Direction::Send),
        ];

        let strings = StringTable::new(Language("en".into()));
        let module_ets_ids: BTreeMap<ModuleInstanceId, String> = BTreeMap::new();
        let fields = com_object_fields(&com, &strings, &ga_keys, &module_ets_ids);

        let ga1 = group_address_key(&project.installations[0].group_addresses[0], style());
        let ga2 = group_address_key(&project.installations[0].group_addresses[1], style());
        assert_eq!(
            fields.links,
            vec![
                (ga1.clone(), Direction::Send),
                (ga1, Direction::Receive),
                (ga2, Direction::Send),
            ]
        );
    }

    #[test]
    fn com_object_fields_resolves_module_instance_ets_id() {
        let strings = StringTable::new(Language("en".into()));
        let ga_keys: BTreeMap<GroupAddressId, GroupAddressKey> = BTreeMap::new();
        let mut module_ets_ids: BTreeMap<ModuleInstanceId, String> = BTreeMap::new();
        module_ets_ids.insert(module_id(1), "MD-2_M-1".into());

        let mut com = com_object(1, DeviceId(1), 0);
        com.module_instance = Some(module_id(1));

        let fields = com_object_fields(&com, &strings, &ga_keys, &module_ets_ids);
        assert_eq!(fields.module_instance, Some("MD-2_M-1".to_string()));
    }

    // -----------------------------------------------------------------
    // Building part fields
    // -----------------------------------------------------------------

    #[test]
    fn building_part_fields_resolves_default_line() {
        let mut project = project();
        let area_id = add_area(&mut project, 1, 1);
        let line_id = add_line(&mut project, area_id, 1, 1);
        let part_id = add_building_part(
            &mut project,
            1,
            "Building A",
            BuildingPartType::Building,
            None,
        );
        if let Some(part) = project.installations[0]
            .buildings
            .iter_mut()
            .find(|p| p.id == part_id)
        {
            part.default_line = Some(line_id);
        }
        let topology = &project.installations[0].topology;
        let part = project.installations[0]
            .buildings
            .iter()
            .find(|p| p.id == part_id)
            .unwrap();

        assert_eq!(
            building_part_fields(part, topology).default_line,
            Some(LineKey {
                area_address: 1,
                line_address: 1,
            })
        );
    }

    // -----------------------------------------------------------------
    // *_changed_fields tests — one per `*Fields` type, 8 total.
    // -----------------------------------------------------------------

    #[test]
    fn area_changed_fields_reports_every_differing_field_in_order() {
        let left = AreaFields {
            name: "A".into(),
            completion: CompletionStatus::Editing,
        };
        let right = AreaFields {
            name: "B".into(),
            completion: CompletionStatus::Accepted,
        };
        assert_eq!(
            area_changed_fields(&left, &right),
            vec!["name", "completion"]
        );
    }

    #[test]
    fn line_changed_fields_reports_every_differing_field_in_order() {
        let left = LineFields {
            name: "L1".into(),
            medium_ref: "MT-0".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: Some(4),
            completion: CompletionStatus::Editing,
            area: None,
        };
        let right = LineFields {
            name: "L2".into(),
            multicast_ttl: Some(8),
            ..left.clone()
        };
        assert_eq!(
            line_changed_fields(&left, &right),
            vec!["name", "multicast_ttl"]
        );
    }

    #[test]
    fn device_changed_fields_reports_every_differing_field_in_order() {
        let left = DeviceFields {
            name: "D1".into(),
            description: None,
            address: None,
            product_ref: "P-0".into(),
            program_ref: "H-0".into(),
            commissioning: CommissioningState::default(),
            line: None,
            building: None,
        };
        let right = DeviceFields {
            name: "D2".into(),
            program_ref: "H-1".into(),
            ..left.clone()
        };
        assert_eq!(
            device_changed_fields(&left, &right),
            vec!["name", "program_ref"]
        );
    }

    #[test]
    fn group_range_changed_fields_reports_every_differing_field_in_order() {
        let left = GroupRangeFields {
            name: "Lighting".into(),
            start: 0,
            end: 100,
            parent: None,
        };
        let right = GroupRangeFields {
            name: "HVAC".into(),
            end: 200,
            ..left.clone()
        };
        assert_eq!(
            group_range_changed_fields(&left, &right),
            vec!["name", "end"]
        );
    }

    #[test]
    fn group_address_changed_fields_reports_every_differing_field_in_order() {
        let left = GroupAddressFields {
            name: "Light on".into(),
            central: false,
            unfiltered: false,
            range: None,
        };
        let right = GroupAddressFields {
            name: "Light off".into(),
            unfiltered: true,
            ..left.clone()
        };
        assert_eq!(
            group_address_changed_fields(&left, &right),
            vec!["name", "unfiltered"]
        );
    }

    #[test]
    fn building_part_changed_fields_reports_every_differing_field_in_order() {
        let left = BuildingPartFields {
            name: "Room 1".into(),
            number: None,
            kind: BuildingPartType::Room,
            completion: CompletionStatus::Editing,
            default_line: None,
        };
        let right = BuildingPartFields {
            name: "Room 2".into(),
            completion: CompletionStatus::Accepted,
            ..left.clone()
        };
        assert_eq!(
            building_part_changed_fields(&left, &right),
            vec!["name", "completion"]
        );
    }

    #[test]
    fn com_object_changed_fields_reports_every_differing_field_in_order() {
        let left = ComObjectFields {
            text: Some("On".into()),
            description: None,
            dpt: Some("DPST-1-1".into()),
            read: Some(true),
            write: Some(false),
            transmit: Some(true),
            update: Some(false),
            communication: Some(true),
            read_on_init: None,
            links: vec![],
            module_instance: None,
        };
        let right = ComObjectFields {
            text: Some("Off".into()),
            update: Some(true),
            // Absent on the left, stated on the right: the sixth flag
            // reports as a change like any other field (§117).
            read_on_init: Some(true),
            ..left.clone()
        };
        assert_eq!(
            com_object_changed_fields(&left, &right),
            vec!["text", "update", "read_on_init"]
        );
    }

    #[test]
    fn parameter_changed_fields_reports_every_differing_field_in_order() {
        let left = ParameterFields { raw: "1".into() };
        let right = ParameterFields { raw: "2".into() };
        assert_eq!(parameter_changed_fields(&left, &right), vec!["raw"]);
    }
}
