//! Shared `#[cfg(test)]` fixtures for `model.rs`: a minimal but
//! structurally complete `Project`, so every test builds on the same
//! ground instead of re-deriving one ad hoc — the same reason
//! `knx-csv/src/testutil.rs` exists. Hand-built values only, no corpus, no
//! import.

#![cfg(test)]

use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance, ComObjectInstanceId,
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Direction, GroupAddress,
    GroupAddressEntry, GroupAddressId, GroupAddressStyle, GroupLink, GroupRange, GroupRangeId,
    Installation, InstallationId, Language, Override, Project, ResolvedFlags, SourceRef, Topology,
};

pub(crate) fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.to_string(),
        ets_id: tag.to_string(),
    }
}

/// A project with one empty installation — callers push what their test
/// needs onto `project.installations[0]` or `project.devices`.
pub(crate) fn empty_project(style: GroupAddressStyle) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.group_address_style = style;
    project.installations.push(empty_installation());
    project
}

pub(crate) fn empty_installation() -> Installation {
    Installation {
        id: InstallationId(0),
        name: "Installation".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::default(),
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    }
}

pub(crate) fn building_part(
    id: u32,
    name: &str,
    kind: BuildingPartType,
    parent: Option<u32>,
    children: &[u32],
) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: source(&format!("BP-{id}")),
        name: name.to_string(),
        number: None,
        kind,
        default_line: None,
        completion: CompletionStatus::default(),
        children: children.iter().map(|c| BuildingPartId(*c)).collect(),
        devices: vec![],
        parent: parent.map(BuildingPartId),
    }
}

pub(crate) fn range(
    id: u32,
    name: &str,
    start: u16,
    end: u16,
    parent: Option<u32>,
    children: &[u32],
) -> GroupRange {
    GroupRange {
        id: GroupRangeId(id),
        source: source(&format!("GR-{id}")),
        name: name.to_string(),
        start: GroupAddress::from_raw(start),
        end: GroupAddress::from_raw(end),
        parent: parent.map(GroupRangeId),
        children: children.iter().map(|c| GroupRangeId(*c)).collect(),
    }
}

pub(crate) fn entry(id: u32, address: u16, name: &str) -> GroupAddressEntry {
    GroupAddressEntry {
        id: GroupAddressId(id),
        source: source(&format!("GA-{id}")),
        name: name.to_string(),
        address: GroupAddress::from_raw(address),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Default::default(),
    }
}

/// A device owning exactly the communication object ids in `com_objects`
/// (which may deliberately omit one that `Devices` still holds, to build an
/// orphan fixture).
pub(crate) fn device(id: u32, com_objects: &[u32]) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: source(&format!("D-{id}")),
        name: "Dev".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: com_objects
            .iter()
            .map(|c| ComObjectInstanceId(*c))
            .collect(),
        binary_data: vec![],
    }
}

/// A communication object instance linked (`Direction::Send`) to `ga`. Not
/// inserted into any `DeviceInstance`'s own `com_objects` list by this
/// helper — callers do that separately with [`device`] when they want it
/// owned, and skip it when they want an orphan.
pub(crate) fn linked_com_object(id: u32, device: u32, ga: u32) -> ComObjectInstance {
    ComObjectInstance {
        id: ComObjectInstanceId(id),
        source: source(&format!("CO-{id}")),
        device: DeviceId(device),
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga: GroupAddressId(ga),
            direction: Direction::Send,
        }],
        module_instance: None,
    }
}

/// Like [`linked_com_object`], but with no links at all — for building an
/// orphan fixture without accidentally also exercising the inverse index.
pub(crate) fn unlinked_com_object(id: u32, device: u32) -> ComObjectInstance {
    ComObjectInstance {
        links: vec![],
        ..linked_com_object(id, device, 0)
    }
}
