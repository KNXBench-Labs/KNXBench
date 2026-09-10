//! Shared `#[cfg(test)]` fixtures for the writer and planner tests: a
//! minimal but structurally complete `Project`, so every test builds on the
//! same ground instead of re-deriving one ad hoc — the same reason
//! `knx-etsproj/src/testutil.rs` exists.

#![cfg(test)]

use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CompletionStatus, DeviceId, Direction, DptRef,
    GroupAddress, GroupAddressEntry, GroupAddressId, GroupAddressStyle, GroupLink, GroupRange,
    GroupRangeId, Installation, InstallationId, Language, Layer, Override, Project, Resolved,
    ResolvedFlags, SourceRef, Topology,
};

pub(crate) fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.to_string(),
        ets_id: tag.to_string(),
    }
}

/// A project with one installation and nothing in it yet — callers push
/// what their test needs onto `project.installations[0]`.
pub(crate) fn empty_project(style: GroupAddressStyle) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.group_address_style = style;
    project.installations.push(Installation {
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
    });
    project
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
    }
}

/// Like [`entry`], but with `central`/`unfiltered` set explicitly instead of
/// defaulting to `false` — needed to tell "an empty cell applied `false`"
/// apart from "an empty cell left the existing `true` alone", which would
/// otherwise look identical against an always-`false` fixture.
pub(crate) fn entry_with_flags(
    id: u32,
    address: u16,
    name: &str,
    central: bool,
    unfiltered: bool,
) -> GroupAddressEntry {
    GroupAddressEntry {
        central,
        unfiltered,
        ..entry(id, address, name)
    }
}

pub(crate) fn range(id: u32, name: &str, start: u16, end: u16, parent: Option<u32>) -> GroupRange {
    GroupRange {
        id: GroupRangeId(id),
        source: source(&format!("GR-{id}")),
        name: name.to_string(),
        start: GroupAddress::from_raw(start),
        end: GroupAddress::from_raw(end),
        parent: parent.map(GroupRangeId),
        children: vec![],
    }
}

/// A communication object instance linked (`Direction::Send`) to `ga`, with
/// `dpt` as its resolved (`Layer::Instance`) datapoint type, or
/// `Override::Absent` when `dpt` is `None`. Not owned by any
/// `DeviceInstance` — `Devices::com_objects` does not require one
/// (`devices.rs`'s own test makes the same choice).
pub(crate) fn linked_com_object(
    id: u32,
    device: u32,
    ga: u32,
    dpt: Option<DptRef>,
) -> ComObjectInstance {
    ComObjectInstance {
        id: ComObjectInstanceId(id),
        source: source(&format!("CO-{id}")),
        device: DeviceId(device),
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: match dpt {
            Some(value) => Override::Value(Resolved {
                value,
                layer: Layer::Instance,
            }),
            None => Override::Absent,
        },
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
