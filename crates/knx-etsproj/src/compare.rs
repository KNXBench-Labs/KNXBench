//! The declared semantic-equality relation (ADR-0007, IMPORT_EXPORT §9):
//! what "the roundtrip didn't change anything" means in this repository.
//!
//! `semantic_view` keys every entity by its `SourceRef::ets_id`, sorts
//! every collection by that key (or, for a communication object, by its
//! `number` — the plan's own choice, not the `ets_id`/`RefId`, since a
//! `RefId`'s format is schema-version-specific (RESEARCH §3.3) while the
//! object number is not), and keeps every field that is an actual modelled
//! attribute on the corresponding `knx_core` struct. What it deliberately
//! drops: every synthetic `*Id` (an id allocated fresh on each import, so
//! comparing two imports' ids would always "differ" for no real reason),
//! and any relationship that is not itself a field on the entity being
//! described — a device's line/area placement, for instance, is
//! `Topology`'s data, not `DeviceInstance`'s, so it is not part of
//! `SemanticDevice` (matching the plan's own literal field list for that
//! type) even though it is part of `SemanticLine`'s equivalent for
//! `GroupRange`'s `parent` and `GroupAddressEntry`'s `range` — both of
//! which genuinely are fields on those structs.
//!
//! Changing this type changes what fidelity means in this repository,
//! which is why it is a declared type here and not a bag of ad-hoc
//! assertions inside a test.

use std::collections::BTreeMap;

use knx_core::{
    Area, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance, CommissioningState,
    CompletionStatus, DeviceId, DeviceInstance, Direction, DptRef, GroupAddressEntry,
    GroupAddressStyle, GroupRange, GroupRangeId, Installation, Language, Line, LineId, Override,
    ParameterInstance, Project, StringTable, Text,
};

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticProject {
    pub info: SemanticInfo,
    /// Keyed by installation index, which is stable: installations are
    /// written in source order and read back in the same order.
    pub installations: Vec<SemanticInstallation>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticInfo {
    pub project_id: String,
    pub name: String,
    pub project_number: Option<String>,
    pub group_address_style: GroupAddressStyle,
    pub completion: CompletionStatus,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticInstallation {
    /// Sorted by `ets_id`. Every collection below is likewise sorted, so
    /// ordering differences are normalized away and content differences
    /// are not.
    pub areas: Vec<(String, u8, String)>,
    pub lines: Vec<SemanticLine>,
    pub devices: Vec<SemanticDevice>,
    pub group_ranges: Vec<SemanticGroupRange>,
    pub group_addresses: Vec<SemanticGroupAddress>,
    pub buildings: Vec<SemanticBuildingPart>,
    pub parameters: Vec<SemanticParameter>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticLine {
    pub ets_id: String,
    pub name: String,
    pub address: u8,
    pub medium_ref: String,
    pub domain_address: Option<String>,
    pub domain_address_is_checked: Option<bool>,
    pub ip_routing_multicast_address: Option<String>,
    pub multicast_ttl: Option<u8>,
    pub completion: CompletionStatus,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticDevice {
    pub ets_id: String,
    pub name: String,
    pub address: Option<String>,
    pub product_ref: Option<String>,
    pub program_ref: Option<String>,
    pub commissioning: CommissioningState,
    pub binary_data: Vec<String>,
    pub com_objects: Vec<SemanticComObject>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticComObject {
    pub number: u16,
    /// `None` where the attribute was absent, `Some("")` where it was
    /// present and empty — the distinction Task 1 exists to preserve.
    pub text: Option<String>,
    pub description: Option<String>,
    pub dpt: Option<String>,
    /// Read, write, transmit, update, communication, in that order.
    pub flags: [Option<bool>; 5],
    /// `(group address ets_id, direction)`, sorted.
    pub links: Vec<(String, Direction)>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticGroupRange {
    pub ets_id: String,
    pub name: String,
    pub start: String,
    pub end: String,
    pub parent: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticGroupAddress {
    pub ets_id: String,
    pub name: String,
    pub address: String,
    pub central: bool,
    pub unfiltered: bool,
    pub range: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticBuildingPart {
    pub ets_id: String,
    pub name: String,
    pub number: Option<String>,
    pub kind: BuildingPartType,
    pub default_line: Option<String>,
    pub completion: CompletionStatus,
    pub parent: Option<String>,
    pub devices: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct SemanticParameter {
    pub ets_id: String,
    pub device: String,
    pub raw: String,
}

pub fn semantic_view(project: &Project) -> SemanticProject {
    let info = SemanticInfo {
        project_id: project.info.project_id.clone(),
        name: project.info.name.clone(),
        project_number: project.info.project_number.clone(),
        group_address_style: project.info.group_address_style,
        completion: project.info.completion,
    };

    let installations = project
        .installations
        .iter()
        .map(|inst| semantic_installation(project, inst))
        .collect();

    SemanticProject {
        info,
        installations,
    }
}

fn semantic_installation(project: &Project, inst: &Installation) -> SemanticInstallation {
    let mut areas: Vec<(String, u8, String)> = inst
        .topology
        .areas
        .iter()
        .map(|a: &Area| (a.source.ets_id.clone(), a.address, a.name.clone()))
        .collect();
    areas.sort();

    let mut lines: Vec<SemanticLine> = inst.topology.lines.iter().map(semantic_line).collect();
    lines.sort_by(|a, b| a.ets_id.cmp(&b.ets_id));

    let mut device_ids: Vec<DeviceId> = inst
        .topology
        .lines
        .iter()
        .flat_map(|l| l.devices.iter().copied())
        .collect();
    device_ids.extend(inst.topology.unassigned.iter().copied());
    let mut devices: Vec<SemanticDevice> = device_ids
        .iter()
        .filter_map(|&id| project.devices.get(id))
        .map(|d| semantic_device(project, d))
        .collect();
    devices.sort_by(|a, b| a.ets_id.cmp(&b.ets_id));

    let range_by_id: BTreeMap<GroupRangeId, &GroupRange> =
        inst.group_ranges.iter().map(|r| (r.id, r)).collect();
    let mut group_ranges: Vec<SemanticGroupRange> = inst
        .group_ranges
        .iter()
        .map(|r| semantic_group_range(&range_by_id, r))
        .collect();
    group_ranges.sort_by(|a, b| a.ets_id.cmp(&b.ets_id));

    let mut group_addresses: Vec<SemanticGroupAddress> = inst
        .group_addresses
        .iter()
        .map(|g| semantic_group_address(&range_by_id, g))
        .collect();
    group_addresses.sort_by(|a, b| a.ets_id.cmp(&b.ets_id));

    let line_by_id: BTreeMap<LineId, &Line> =
        inst.topology.lines.iter().map(|l| (l.id, l)).collect();
    let building_by_id: BTreeMap<BuildingPartId, &BuildingPart> =
        inst.buildings.iter().map(|b| (b.id, b)).collect();
    let mut buildings: Vec<SemanticBuildingPart> = inst
        .buildings
        .iter()
        .map(|b| semantic_building_part(project, b, &building_by_id, &line_by_id))
        .collect();
    buildings.sort_by(|a, b| a.ets_id.cmp(&b.ets_id));

    let mut parameters: Vec<SemanticParameter> = inst
        .parameters
        .iter()
        .map(|p| semantic_parameter(project, p))
        .collect();
    parameters.sort_by(|a, b| a.ets_id.cmp(&b.ets_id));

    SemanticInstallation {
        areas,
        lines,
        devices,
        group_ranges,
        group_addresses,
        buildings,
        parameters,
    }
}

fn semantic_line(l: &Line) -> SemanticLine {
    SemanticLine {
        ets_id: l.source.ets_id.clone(),
        name: l.name.clone(),
        address: l.address,
        medium_ref: l.medium_ref.clone(),
        domain_address: l.domain_address.clone(),
        domain_address_is_checked: l.domain_address_is_checked,
        ip_routing_multicast_address: l.ip_routing_multicast_address.map(|a| a.to_string()),
        multicast_ttl: l.multicast_ttl,
        completion: l.completion,
    }
}

fn semantic_device(project: &Project, d: &DeviceInstance) -> SemanticDevice {
    let mut com_objects: Vec<SemanticComObject> = d
        .com_objects
        .iter()
        .filter_map(|&id| project.devices.com_object(id))
        .map(|c| semantic_com_object(project, c))
        .collect();
    com_objects.sort_by_key(|c| c.number);

    let mut binary_data: Vec<String> = d.binary_data.iter().map(|b| b.id.clone()).collect();
    binary_data.sort();

    SemanticDevice {
        ets_id: d.source.ets_id.clone(),
        name: d.name.clone(),
        address: d.address.map(|a| a.to_string()),
        product_ref: Some(d.product_ref.clone()).filter(|s| !s.is_empty()),
        program_ref: Some(d.program_ref.clone()).filter(|s| !s.is_empty()),
        commissioning: d.commissioning.clone(),
        binary_data,
        com_objects,
    }
}

fn semantic_com_object(project: &Project, c: &ComObjectInstance) -> SemanticComObject {
    let mut links: Vec<(String, Direction)> = c
        .links
        .iter()
        .filter_map(|link| {
            project
                .installations
                .iter()
                .flat_map(|i| &i.group_addresses)
                .find(|g| g.id == link.ga)
                .map(|g| (g.source.ets_id.clone(), link.direction))
        })
        .collect();
    links.sort_by(|a, b| {
        a.0.cmp(&b.0)
            .then(direction_rank(a.1).cmp(&direction_rank(b.1)))
    });

    SemanticComObject {
        number: c.number,
        text: semantic_text(&c.text, &project.strings),
        description: semantic_text(&c.description, &project.strings),
        dpt: semantic_dpt(&c.dpt),
        flags: [
            semantic_flag(&c.flags.read),
            semantic_flag(&c.flags.write),
            semantic_flag(&c.flags.transmit),
            semantic_flag(&c.flags.update),
            semantic_flag(&c.flags.communication),
        ],
        links,
    }
}

fn direction_rank(d: Direction) -> u8 {
    match d {
        Direction::Send => 0,
        Direction::Receive => 1,
    }
}

/// `Override::Absent` -> `None`, `Override::Empty` -> `Some("")`; a
/// `Value` renders only if its layer is exported — matching exactly what a
/// roundtrip through `export_knxproj` actually writes, since that is what
/// this comparison exists to check.
fn semantic_text(o: &Override<Text>, strings: &StringTable) -> Option<String> {
    match o {
        Override::Absent => None,
        Override::Empty => Some(String::new()),
        Override::Value(r) if r.layer.is_exported() => {
            let language = Language("en".to_string());
            Some(
                strings
                    .text(&r.value, &language)
                    .unwrap_or_default()
                    .to_string(),
            )
        }
        Override::Value(_) => None,
    }
}

fn semantic_dpt(o: &Override<DptRef>) -> Option<String> {
    match o {
        Override::Absent => None,
        Override::Empty => Some(String::new()),
        Override::Value(r) if r.layer.is_exported() => Some(r.value.to_string()),
        Override::Value(_) => None,
    }
}

/// `Option<bool>` has no third state, so `Override::Empty` collapses into
/// `None` alongside `Override::Absent` — a real, if narrow, loss of
/// distinction in this comparison type specifically. None of the
/// reference project's flag attributes are ever empty (measured: each of
/// the five is exclusively `"Enabled"` or `"Disabled"`), so it does not
/// affect this repository's actual roundtrip test.
fn semantic_flag(o: &Override<bool>) -> Option<bool> {
    match o {
        Override::Value(r) if r.layer.is_exported() => Some(r.value),
        _ => None,
    }
}

fn semantic_group_range(
    by_id: &BTreeMap<GroupRangeId, &GroupRange>,
    r: &GroupRange,
) -> SemanticGroupRange {
    SemanticGroupRange {
        ets_id: r.source.ets_id.clone(),
        name: r.name.clone(),
        start: r.start.raw().to_string(),
        end: r.end.raw().to_string(),
        parent: r
            .parent
            .and_then(|id| by_id.get(&id))
            .map(|p| p.source.ets_id.clone()),
    }
}

fn semantic_group_address(
    by_id: &BTreeMap<GroupRangeId, &GroupRange>,
    g: &GroupAddressEntry,
) -> SemanticGroupAddress {
    SemanticGroupAddress {
        ets_id: g.source.ets_id.clone(),
        name: g.name.clone(),
        address: g.address.raw().to_string(),
        central: g.central,
        unfiltered: g.unfiltered,
        range: g
            .range
            .and_then(|id| by_id.get(&id))
            .map(|r| r.source.ets_id.clone()),
    }
}

fn semantic_building_part(
    project: &Project,
    part: &BuildingPart,
    by_id: &BTreeMap<BuildingPartId, &BuildingPart>,
    line_by_id: &BTreeMap<LineId, &Line>,
) -> SemanticBuildingPart {
    let mut devices: Vec<String> = part
        .devices
        .iter()
        .filter_map(|&id| project.devices.get(id))
        .map(|d| d.source.ets_id.clone())
        .collect();
    devices.sort();

    SemanticBuildingPart {
        ets_id: part.source.ets_id.clone(),
        name: part.name.clone(),
        number: part.number.clone(),
        kind: part.kind,
        default_line: part
            .default_line
            .and_then(|id| line_by_id.get(&id))
            .map(|l| l.source.ets_id.clone()),
        completion: part.completion,
        parent: part
            .parent
            .and_then(|id| by_id.get(&id))
            .map(|p| p.source.ets_id.clone()),
        devices,
    }
}

fn semantic_parameter(project: &Project, p: &ParameterInstance) -> SemanticParameter {
    SemanticParameter {
        ets_id: p.source.ets_id.clone(),
        device: project
            .devices
            .get(p.device)
            .map(|d| d.source.ets_id.clone())
            .unwrap_or_default(),
        raw: p.raw.clone(),
    }
}

/// Returns the first difference between `a` and `b` as human-readable
/// text, or `None` if they are semantically equal. A failing roundtrip
/// test that says only "not equal" costs an hour; this says which field,
/// which index, and both values.
pub fn describe_difference(a: &SemanticProject, b: &SemanticProject) -> Option<String> {
    if a.info != b.info {
        return Some(format!(
            "project info differs:\n  left:  {:?}\n  right: {:?}",
            a.info, b.info
        ));
    }
    if a.installations.len() != b.installations.len() {
        return Some(format!(
            "installation count differs: {} vs {}",
            a.installations.len(),
            b.installations.len()
        ));
    }
    for (i, (ia, ib)) in a.installations.iter().zip(&b.installations).enumerate() {
        if let Some(diff) = describe_installation_difference(i, ia, ib) {
            return Some(diff);
        }
    }
    None
}

fn describe_installation_difference(
    i: usize,
    a: &SemanticInstallation,
    b: &SemanticInstallation,
) -> Option<String> {
    if a.areas != b.areas {
        return Some(describe_vec_difference(
            &format!("installations[{i}].areas"),
            &a.areas,
            &b.areas,
        ));
    }
    if a.lines != b.lines {
        return Some(describe_vec_difference(
            &format!("installations[{i}].lines"),
            &a.lines,
            &b.lines,
        ));
    }
    if a.devices != b.devices {
        return Some(describe_vec_difference(
            &format!("installations[{i}].devices"),
            &a.devices,
            &b.devices,
        ));
    }
    if a.group_ranges != b.group_ranges {
        return Some(describe_vec_difference(
            &format!("installations[{i}].group_ranges"),
            &a.group_ranges,
            &b.group_ranges,
        ));
    }
    if a.group_addresses != b.group_addresses {
        return Some(describe_vec_difference(
            &format!("installations[{i}].group_addresses"),
            &a.group_addresses,
            &b.group_addresses,
        ));
    }
    if a.buildings != b.buildings {
        return Some(describe_vec_difference(
            &format!("installations[{i}].buildings"),
            &a.buildings,
            &b.buildings,
        ));
    }
    if a.parameters != b.parameters {
        return Some(describe_vec_difference(
            &format!("installations[{i}].parameters"),
            &a.parameters,
            &b.parameters,
        ));
    }
    None
}

fn describe_vec_difference<T: std::fmt::Debug + PartialEq>(
    label: &str,
    a: &[T],
    b: &[T],
) -> String {
    if a.len() != b.len() {
        return format!("{label}: length differs ({} vs {})", a.len(), b.len());
    }
    for (idx, (x, y)) in a.iter().zip(b).enumerate() {
        if x != y {
            return format!("{label}[{idx}] differs:\n  left:  {x:?}\n  right: {y:?}");
        }
    }
    format!("{label}: differs, but no element-wise difference was found (order?)")
}
