//! Shared `#[cfg(test)]` fixtures for `semantic.rs`: hand-built `Project`
//! pieces, no corpus, no import — the same convention
//! `crates/knx-report/src/testutil.rs` uses for the same reason (small,
//! obviously-correct fixtures beat one more copy of a 2000-line
//! `.knxproj`).

use std::net::Ipv4Addr;

use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
    ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Direction,
    DptRef, GroupAddress, GroupAddressEntry, GroupAddressId, GroupAddressStyle, GroupLink,
    GroupRange, GroupRangeId, IndividualAddress, Language, Layer, Line, LineId, ModuleInstanceId,
    Override, ParameterInstance, ParameterInstanceId, Project, Resolved, ResolvedFlags, SourceRef,
    Text,
};

pub(crate) fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.to_string(),
        ets_id: tag.to_string(),
    }
}

/// A project with one empty installation (id 0) and the default
/// `GroupAddressStyle::ThreeLevel` — callers push whatever their test needs
/// onto `project.installations[0]` / `project.devices` directly.
pub(crate) fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(knx_core::Installation {
        id: knx_core::InstallationId(0),
        name: "Installation".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::default(),
        topology: knx_core::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project
}

/// Adds an `Area` with the given address to the project's one installation
/// and returns its id.
pub(crate) fn add_area(project: &mut Project, id: u32, address: u8) -> AreaId {
    let area_id = AreaId(id);
    project.installations[0].topology.areas.push(Area {
        id: area_id,
        source: source(&format!("A-{id}")),
        name: format!("Area {address}"),
        address,
        completion: CompletionStatus::default(),
        lines: vec![],
    });
    area_id
}

/// Adds a `Line` with the given address under `area` and returns its id.
pub(crate) fn add_line(project: &mut Project, area: AreaId, id: u32, address: u8) -> LineId {
    let line_id = LineId(id);
    project.installations[0].topology.lines.push(Line {
        id: line_id,
        source: source(&format!("L-{id}")),
        name: format!("Line {address}"),
        address,
        medium_ref: "MT-0".into(),
        domain_address: None,
        domain_address_is_checked: None,
        ip_routing_multicast_address: None,
        multicast_ttl: None,
        completion: CompletionStatus::default(),
        devices: vec![],
    });
    if let Some(a) = project.installations[0]
        .topology
        .areas
        .iter_mut()
        .find(|a| a.id == area)
    {
        a.lines.push(line_id);
    }
    line_id
}

/// A `Line` fixture with every optional field populated, not attached to
/// any project — for tests that only need `line_fields`/`line_changed_fields`
/// over a standalone value.
pub(crate) fn full_line(id: u32, address: u8) -> Line {
    Line {
        id: LineId(id),
        source: source(&format!("L-{id}")),
        name: format!("Line {address}"),
        address,
        medium_ref: "MT-0".into(),
        domain_address: Some("0".into()),
        domain_address_is_checked: Some(true),
        ip_routing_multicast_address: Some(Ipv4Addr::new(224, 0, 23, 12)),
        multicast_ttl: Some(16),
        completion: CompletionStatus::FinishedDesign,
        devices: vec![],
    }
}

/// Adds a `GroupRange` and returns its id.
pub(crate) fn add_group_range(
    project: &mut Project,
    id: u32,
    name: &str,
    start: u16,
    end: u16,
    parent: Option<GroupRangeId>,
) -> GroupRangeId {
    let range_id = GroupRangeId(id);
    project.installations[0].group_ranges.push(GroupRange {
        id: range_id,
        source: source(&format!("GR-{id}")),
        name: name.to_string(),
        start: GroupAddress::from_raw(start),
        end: GroupAddress::from_raw(end),
        parent,
        children: vec![],
    });
    range_id
}

/// Adds a `GroupAddressEntry` inside `range` (if any) and returns its id.
pub(crate) fn add_group_address(
    project: &mut Project,
    id: u32,
    address: u16,
    name: &str,
    range: Option<GroupRangeId>,
) -> GroupAddressId {
    let ga_id = GroupAddressId(id);
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id: ga_id,
            source: source(&format!("GA-{id}")),
            name: name.to_string(),
            address: GroupAddress::from_raw(address),
            central: false,
            unfiltered: false,
            range,
            declared_dpt: Default::default(),
        });
    ga_id
}

/// Adds a `BuildingPart` under `parent` (root, if `None`) and returns its
/// id. Does not wire `children`/`devices` — tests that need those set them
/// afterwards on the returned part directly, or via [`add_device`].
pub(crate) fn add_building_part(
    project: &mut Project,
    id: u32,
    name: &str,
    kind: BuildingPartType,
    parent: Option<BuildingPartId>,
) -> BuildingPartId {
    let part_id = BuildingPartId(id);
    project.installations[0].buildings.push(BuildingPart {
        id: part_id,
        source: source(&format!("BP-{id}")),
        name: name.to_string(),
        number: None,
        kind,
        default_line: None,
        completion: CompletionStatus::default(),
        children: vec![],
        devices: vec![],
        parent,
    });
    part_id
}

/// Adds a `DeviceInstance` on `line` (unassigned if `None`) and returns its
/// id. Wires `Topology::lines[..].devices`/`Topology::unassigned`
/// accordingly.
pub(crate) fn add_device(project: &mut Project, id: u32, line: Option<LineId>) -> DeviceId {
    let device_id = DeviceId(id);
    project.devices.insert(DeviceInstance {
        id: device_id,
        source: source(&format!("D-{id}")),
        name: format!("Device {id}"),
        description: None,
        address: None,
        product_ref: "P-0".into(),
        program_ref: "H-0".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    match line {
        Some(line_id) => {
            if let Some(l) = project.installations[0]
                .topology
                .lines
                .iter_mut()
                .find(|l| l.id == line_id)
            {
                l.devices.push(device_id);
            }
        }
        None => project.installations[0].topology.unassigned.push(device_id),
    }
    device_id
}

/// Adds a `ParameterInstance` on `device` and returns its id.
pub(crate) fn add_parameter(
    project: &mut Project,
    id: u32,
    device: DeviceId,
    ets_id: &str,
    raw: &str,
) -> ParameterInstanceId {
    let param_id = ParameterInstanceId(id);
    project.installations[0].parameters.push(ParameterInstance {
        id: param_id,
        device,
        source: source(ets_id),
        raw: raw.to_string(),
    });
    param_id
}

/// A `ComObjectInstance` fixture: every override absent, no links, no
/// module. Callers overwrite whatever field their test needs and push it
/// into `project.devices` themselves with [`push_com_object`].
pub(crate) fn com_object(id: u32, device: DeviceId, number: u16) -> ComObjectInstance {
    ComObjectInstance {
        id: ComObjectInstanceId(id),
        source: source(&format!("CO-{id}")),
        device,
        number,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
    }
}

/// A resolved `Override::Value` carrying `text` at `layer`.
pub(crate) fn literal_text(text: &str, layer: Layer) -> Override<Text> {
    Override::Value(Resolved {
        value: Text::Literal(text.to_string()),
        layer,
    })
}

/// A resolved `Override::Value` carrying `dpt` at `layer`.
pub(crate) fn dpt_value(main: u16, sub: Option<u16>, layer: Layer) -> Override<DptRef> {
    Override::Value(Resolved {
        value: DptRef { main, sub },
        layer,
    })
}

/// A resolved `Override::Value` carrying a flag at `layer`.
pub(crate) fn flag_value(value: bool, layer: Layer) -> Override<bool> {
    Override::Value(Resolved { value, layer })
}

/// A `GroupLink` to `ga` in `direction`.
pub(crate) fn link(ga: u32, direction: Direction) -> GroupLink {
    GroupLink {
        ga: GroupAddressId(ga),
        direction,
    }
}

pub(crate) fn individual_address(area: u8, line: u8, device: u8) -> IndividualAddress {
    IndividualAddress::new(area, line, device).expect("valid fixture address")
}

pub(crate) fn style() -> GroupAddressStyle {
    GroupAddressStyle::ThreeLevel
}

pub(crate) fn module_id(n: u32) -> ModuleInstanceId {
    ModuleInstanceId(n)
}
