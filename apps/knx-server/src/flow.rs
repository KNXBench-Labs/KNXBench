//! Configured telegram-flow participants of one monitor interpretation context.
//!
//! Configured telegram-flow participants: who is linked to which group
//! address, as the open project states it (AR20,
//! `docs/TELEGRAM_FLOW_VISUALIZATION.md` §4 and §9.2).
//!
//! This is **configuration evidence, not observation**. A member listed
//! here is a communication object the project links to a group address; it
//! says nothing about whether that device received, accepted or acted on a
//! telegram. The snapshot is built once per interpretation context (see
//! `bus::GroupAddressContext`), never mutated afterwards, and compared in
//! full so that a link-, flag-, activation- or device-only edit is
//! detectable even when DPTs and names stay equal.
//!
//! Absent facts stay absent: a flag no layer states is `None`, not `false`
//! (`knx_core::ResolvedFlags`), a device without an individual address has
//! `None`, and project inconsistencies (duplicate addresses, dangling links,
//! objects of unknown devices, ambiguous placements) are listed as
//! diagnostics instead of being resolved by guessing.

use std::collections::{BTreeMap, HashMap};

use knx_core::{Direction, GroupAddressDpt, Override, Project};
use serde::Serialize;

/// The participant facts of one interpretation context. Lists are sorted
/// (devices by id, groups by raw address then id, members by object id then
/// direction), so equal projects give equal snapshots.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FlowParticipants {
    pub(crate) devices: Vec<FlowDevice>,
    pub(crate) groups: Vec<FlowGroup>,
    pub(crate) diagnostics: FlowDiagnostics,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowDevice {
    pub(crate) device_id: u32,
    /// The installation whose topology places the device; `None` when no
    /// installation or more than one does (the latter is also a diagnostic).
    pub(crate) installation_id: Option<u8>,
    pub(crate) name: String,
    pub(crate) individual_address_raw: Option<u16>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowGroup {
    pub(crate) ga_raw: u16,
    pub(crate) ga_id: u32,
    pub(crate) installation_id: u8,
    pub(crate) name: String,
    /// The single DPT the project resolves for this raw address; `None`
    /// when none or conflicting DPTs resolve (rows carry the exact decode
    /// outcome, including the conflict).
    pub(crate) dpt: Option<String>,
    pub(crate) members: Vec<FlowMember>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FlowMember {
    pub(crate) device_id: u32,
    pub(crate) com_object_id: u32,
    pub(crate) direction: Direction,
    pub(crate) active: bool,
    pub(crate) flags: FlowFlags,
}

/// The six communication flags as stated; `None` = no layer states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlowFlags {
    pub(crate) communication: Option<bool>,
    pub(crate) read: Option<bool>,
    pub(crate) write: Option<bool>,
    pub(crate) transmit: Option<bool>,
    pub(crate) update: Option<bool>,
    pub(crate) read_on_init: Option<bool>,
}

impl FlowFlags {
    fn is_unknown(&self) -> bool {
        *self == FlowFlags::default()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct FlowDiagnostics {
    /// Individual address → every device id holding it (two or more).
    pub(crate) duplicate_individual_addresses: Vec<(u16, Vec<u32>)>,
    /// Raw group address → every group-address id with it (two or more,
    /// typically in different installations).
    pub(crate) ambiguous_group_addresses: Vec<(u16, Vec<u32>)>,
    /// Devices placed by more than one installation's topology.
    pub(crate) ambiguous_devices: Vec<u32>,
    /// `(com_object_id, ga_id)` links to a group-address id no installation
    /// defines. Not listed as members of any group.
    pub(crate) dangling_links: Vec<(u32, u32)>,
    /// `(com_object_id, device_id)` for linked objects whose device does not
    /// exist. Still listed as members.
    pub(crate) unknown_devices: Vec<(u32, u32)>,
    /// Linked objects for which no layer states any of the six flags.
    pub(crate) objects_without_flags: Vec<u32>,
}

fn stated(value: &Override<bool>) -> Option<bool> {
    value.value().map(|resolved| resolved.value)
}

impl FlowParticipants {
    /// Builds the snapshot. `dpts` is the context's own DPT map
    /// (`knx_core::resolve_project_group_address_dpts`), passed in so the
    /// group DPT and the row decode can never disagree.
    pub(crate) fn from_project(project: &Project, dpts: &HashMap<u16, GroupAddressDpt>) -> Self {
        let mut diagnostics = FlowDiagnostics::default();

        // Placement: the installation whose topology lists the device.
        let mut placements: BTreeMap<u32, Vec<u8>> = BTreeMap::new();
        for installation in &project.installations {
            let topology = &installation.topology;
            let placed = topology
                .unassigned
                .iter()
                .chain(topology.lines.iter().flat_map(|line| line.devices.iter()));
            for device in placed {
                let owners = placements.entry(device.0).or_default();
                if !owners.contains(&installation.id.0) {
                    owners.push(installation.id.0);
                }
            }
        }

        let mut devices: Vec<FlowDevice> = project
            .devices
            .iter()
            .map(|device| {
                let owners = placements
                    .get(&device.id.0)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]);
                if owners.len() > 1 {
                    diagnostics.ambiguous_devices.push(device.id.0);
                }
                FlowDevice {
                    device_id: device.id.0,
                    installation_id: match owners {
                        [only] => Some(*only),
                        _ => None,
                    },
                    name: device.name.clone(),
                    individual_address_raw: device.address.map(|address| address.raw()),
                }
            })
            .collect();
        devices.sort_by_key(|d| d.device_id);
        diagnostics.ambiguous_devices.sort_unstable();

        let mut by_address: BTreeMap<u16, Vec<u32>> = BTreeMap::new();
        for device in &devices {
            if let Some(raw) = device.individual_address_raw {
                by_address.entry(raw).or_default().push(device.device_id);
            }
        }
        diagnostics.duplicate_individual_addresses = by_address
            .into_iter()
            .filter(|(_, ids)| ids.len() > 1)
            .collect();

        let mut groups: Vec<FlowGroup> = project
            .installations
            .iter()
            .flat_map(|installation| {
                installation.group_addresses.iter().map(|entry| {
                    let raw = entry.address.raw();
                    FlowGroup {
                        ga_raw: raw,
                        ga_id: entry.id.0,
                        installation_id: installation.id.0,
                        name: entry.name.clone(),
                        dpt: match dpts.get(&raw) {
                            Some(GroupAddressDpt::Single(dpt)) => Some(dpt.to_string()),
                            _ => None,
                        },
                        members: Vec::new(),
                    }
                })
            })
            .collect();
        groups.sort_by_key(|g| (g.ga_raw, g.ga_id));

        let mut by_raw: BTreeMap<u16, Vec<u32>> = BTreeMap::new();
        for group in &groups {
            by_raw.entry(group.ga_raw).or_default().push(group.ga_id);
        }
        diagnostics.ambiguous_group_addresses = by_raw
            .into_iter()
            .filter(|(_, ids)| ids.len() > 1)
            .collect();

        let index: HashMap<u32, usize> = groups
            .iter()
            .enumerate()
            .map(|(position, group)| (group.ga_id, position))
            .collect();
        let mut objects: Vec<_> = project.devices.com_objects().collect();
        objects.sort_by_key(|com| com.id.0);
        for com in objects {
            if com.links.is_empty() {
                continue;
            }
            let flags = FlowFlags {
                communication: stated(&com.flags.communication),
                read: stated(&com.flags.read),
                write: stated(&com.flags.write),
                transmit: stated(&com.flags.transmit),
                update: stated(&com.flags.update),
                read_on_init: stated(&com.flags.read_on_init),
            };
            if flags.is_unknown() {
                diagnostics.objects_without_flags.push(com.id.0);
            }
            if project.devices.get(com.device).is_none() {
                diagnostics.unknown_devices.push((com.id.0, com.device.0));
            }
            for link in &com.links {
                match index.get(&link.ga.0) {
                    Some(&position) => groups[position].members.push(FlowMember {
                        device_id: com.device.0,
                        com_object_id: com.id.0,
                        direction: link.direction,
                        active: com.is_active,
                        flags,
                    }),
                    None => diagnostics.dangling_links.push((com.id.0, link.ga.0)),
                }
            }
        }
        for group in &mut groups {
            group
                .members
                .sort_by_key(|m| (m.com_object_id, direction_name(m.direction)));
        }

        Self {
            devices,
            groups,
            diagnostics,
        }
    }
}

// ---------------------------------------------------------------------------
// Wire form: `GET /api/bus/monitor/flow-snapshot`
// ---------------------------------------------------------------------------

/// Response bounds. A snapshot is refused piecewise, never silently: every
/// omitted entry is counted in [`TruncatedDto`].
#[derive(Debug, Clone, Copy)]
pub(crate) struct FlowLimits {
    pub(crate) devices: usize,
    pub(crate) groups: usize,
    /// Members across all returned groups.
    pub(crate) members: usize,
    /// Entries per diagnostics list.
    pub(crate) diagnostics: usize,
}

/// The production bounds. Sized well above the U19 target workload (500
/// devices, 2,500 directed edges) and below anything that would make one
/// response unreasonably large; the 16-bit group address space caps
/// distinct group addresses at 65,536 per installation anyway.
pub(crate) const FLOW_LIMITS: FlowLimits = FlowLimits {
    devices: 10_000,
    groups: 20_000,
    members: 100_000,
    diagnostics: 1_000,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlowDeviceDto {
    device_id: u32,
    installation_id: Option<u8>,
    name: String,
    individual_address_raw: Option<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlowMemberDto {
    device_id: u32,
    com_object_id: u32,
    /// `"Send"` or `"Receive"`, `knx_core::Direction`'s own names.
    direction: &'static str,
    active: bool,
    flags: FlowFlags,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlowGroupDto {
    ga_raw: u16,
    ga_id: u32,
    installation_id: u8,
    name: String,
    dpt: Option<String>,
    members: Vec<FlowMemberDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AddressDevicesDto {
    individual_address_raw: u16,
    device_ids: Vec<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AmbiguousGroupDto {
    ga_raw: u16,
    ga_ids: Vec<u32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DanglingLinkDto {
    com_object_id: u32,
    ga_id: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UnknownDeviceDto {
    com_object_id: u32,
    device_id: u32,
}

#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlowDiagnosticsDto {
    duplicate_individual_addresses: Vec<AddressDevicesDto>,
    ambiguous_group_addresses: Vec<AmbiguousGroupDto>,
    ambiguous_devices: Vec<u32>,
    dangling_links: Vec<DanglingLinkDto>,
    unknown_devices: Vec<UnknownDeviceDto>,
    objects_without_flags: Vec<u32>,
}

/// How many entries each list left out.
#[derive(Debug, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TruncatedDto {
    pub(crate) devices: usize,
    pub(crate) groups: usize,
    pub(crate) members: usize,
    pub(crate) diagnostics: usize,
}

/// The bounded lists of one snapshot response.
#[derive(Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FlowListsDto {
    pub(crate) devices: Vec<FlowDeviceDto>,
    pub(crate) groups: Vec<FlowGroupDto>,
    pub(crate) diagnostics: FlowDiagnosticsDto,
    pub(crate) truncated: TruncatedDto,
}

fn direction_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Send => "Send",
        Direction::Receive => "Receive",
    }
}

/// Keeps the first `limit` items and adds the rest to `omitted`.
fn bounded<T, U>(items: &[T], limit: usize, omitted: &mut usize, map: impl Fn(&T) -> U) -> Vec<U> {
    *omitted += items.len().saturating_sub(limit);
    items.iter().take(limit).map(map).collect()
}

impl FlowListsDto {
    pub(crate) fn bounded(participants: &FlowParticipants, limits: FlowLimits) -> Self {
        let mut truncated = TruncatedDto::default();
        let devices = bounded(
            &participants.devices,
            limits.devices,
            &mut truncated.devices,
            |d| FlowDeviceDto {
                device_id: d.device_id,
                installation_id: d.installation_id,
                name: d.name.clone(),
                individual_address_raw: d.individual_address_raw,
            },
        );
        let mut member_budget = limits.members;
        let mut groups = Vec::new();
        for (index, group) in participants.groups.iter().enumerate() {
            if index >= limits.groups {
                truncated.groups += 1;
                truncated.members += group.members.len();
                continue;
            }
            let take = group.members.len().min(member_budget);
            member_budget -= take;
            truncated.members += group.members.len() - take;
            groups.push(FlowGroupDto {
                ga_raw: group.ga_raw,
                ga_id: group.ga_id,
                installation_id: group.installation_id,
                name: group.name.clone(),
                dpt: group.dpt.clone(),
                members: group.members[..take]
                    .iter()
                    .map(|m| FlowMemberDto {
                        device_id: m.device_id,
                        com_object_id: m.com_object_id,
                        direction: direction_name(m.direction),
                        active: m.active,
                        flags: m.flags,
                    })
                    .collect(),
            });
        }
        let d = &participants.diagnostics;
        let cap = limits.diagnostics;
        let omitted = &mut truncated.diagnostics;
        let diagnostics = FlowDiagnosticsDto {
            duplicate_individual_addresses: bounded(
                &d.duplicate_individual_addresses,
                cap,
                omitted,
                |(raw, ids)| AddressDevicesDto {
                    individual_address_raw: *raw,
                    device_ids: ids.clone(),
                },
            ),
            ambiguous_group_addresses: bounded(
                &d.ambiguous_group_addresses,
                cap,
                omitted,
                |(raw, ids)| AmbiguousGroupDto {
                    ga_raw: *raw,
                    ga_ids: ids.clone(),
                },
            ),
            ambiguous_devices: bounded(&d.ambiguous_devices, cap, omitted, |id| *id),
            dangling_links: bounded(&d.dangling_links, cap, omitted, |(com, ga)| {
                DanglingLinkDto {
                    com_object_id: *com,
                    ga_id: *ga,
                }
            }),
            unknown_devices: bounded(&d.unknown_devices, cap, omitted, |(com, device)| {
                UnknownDeviceDto {
                    com_object_id: *com,
                    device_id: *device,
                }
            }),
            objects_without_flags: bounded(&d.objects_without_flags, cap, omitted, |id| *id),
        };
        Self {
            devices,
            groups,
            diagnostics,
            truncated,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{
        ComObjectInstance, ComObjectInstanceId, DeviceId, DeviceInstance, GroupAddress,
        GroupAddressEntry, GroupAddressId, GroupLink, IndividualAddress, Installation,
        InstallationId, Language, Layer, Resolved, ResolvedFlags, SourceRef, Topology,
    };

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn installation(id: u8, unassigned: Vec<DeviceId>, gas: Vec<(u32, u16, &str)>) -> Installation {
        Installation {
            id: InstallationId(id),
            name: format!("I{id}"),
            default_line: None,
            multicast_address: None,
            completion: knx_core::CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned,
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: gas
                .into_iter()
                .map(|(id, raw, name)| GroupAddressEntry {
                    id: GroupAddressId(id),
                    source: source(),
                    name: name.into(),
                    address: GroupAddress::from_raw(raw),
                    central: false,
                    unfiltered: false,
                    range: None,
                    declared_dpt: Default::default(),
                })
                .collect(),
            parameters: vec![],
        }
    }

    fn device(id: u32, name: &str, address: Option<(u8, u8, u8)>) -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(id),
            source: source(),
            name: name.into(),
            description: None,
            address: address.map(|(a, l, d)| IndividualAddress::new(a, l, d).unwrap()),
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    fn object(
        id: u32,
        device: u32,
        links: Vec<(u32, Direction)>,
        flags: ResolvedFlags,
    ) -> ComObjectInstance {
        ComObjectInstance {
            id: ComObjectInstanceId(id),
            source: source(),
            device: DeviceId(device),
            number: id as u16,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Value(Resolved {
                value: knx_core::DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::Instance,
            }),
            flags,
            size: None,
            is_active: true,
            links: links
                .into_iter()
                .map(|(ga, direction)| GroupLink {
                    ga: GroupAddressId(ga),
                    direction,
                })
                .collect(),
            module_instance: None,
        }
    }

    fn flag(value: bool) -> Override<bool> {
        Override::Value(Resolved {
            value,
            layer: Layer::Instance,
        })
    }

    fn write_only() -> ResolvedFlags {
        ResolvedFlags {
            write: flag(true),
            transmit: flag(false),
            ..ResolvedFlags::none()
        }
    }

    /// Two installations; device 1 (1.1.1) sends on GA 1, device 2 (also
    /// 1.1.1) receives on GA 1 with no flags stated, object 13 of device 3
    /// is inactive and listens on GA 1 too; GA raw 1 also exists in
    /// installation 2 (id 20); object 12 links to a GA id nobody defines;
    /// object 14 belongs to a device that does not exist; device 4 is placed
    /// in both installations.
    fn project() -> Project {
        let mut project = Project::new(Language("en".into()));
        project.installations.push(installation(
            1,
            vec![DeviceId(1), DeviceId(2), DeviceId(3), DeviceId(4)],
            vec![(1, 1, "Light"), (2, 2, "Unlinked")],
        ));
        project.installations.push(installation(
            2,
            vec![DeviceId(4)],
            vec![(20, 1, "Other light")],
        ));
        project
            .devices
            .insert(device(2, "Actuator", Some((1, 1, 1))));
        project.devices.insert(device(1, "Switch", Some((1, 1, 1))));
        project.devices.insert(device(3, "Sleeper", None));
        project
            .devices
            .insert(device(4, "Twice placed", Some((1, 1, 4))));
        project
            .devices
            .insert_com_object(object(10, 1, vec![(1, Direction::Send)], write_only()));
        project.devices.insert_com_object(object(
            11,
            2,
            vec![(1, Direction::Receive)],
            ResolvedFlags::none(),
        ));
        project
            .devices
            .insert_com_object(object(12, 2, vec![(99, Direction::Send)], write_only()));
        let mut sleeper = object(13, 3, vec![(1, Direction::Receive)], write_only());
        sleeper.is_active = false;
        project.devices.insert_com_object(sleeper);
        project.devices.insert_com_object(object(
            14,
            77,
            vec![(2, Direction::Receive)],
            write_only(),
        ));
        project
    }

    fn build(project: &Project) -> FlowParticipants {
        FlowParticipants::from_project(
            project,
            &knx_core::resolve_project_group_address_dpts(project),
        )
    }

    #[test]
    fn devices_are_sorted_with_installation_and_raw_individual_address() {
        let flow = build(&project());
        let ids: Vec<u32> = flow.devices.iter().map(|d| d.device_id).collect();
        assert_eq!(ids, vec![1, 2, 3, 4]);
        assert_eq!(flow.devices[0].installation_id, Some(1));
        assert_eq!(
            flow.devices[0].individual_address_raw,
            Some(IndividualAddress::new(1, 1, 1).unwrap().raw())
        );
        assert_eq!(flow.devices[2].individual_address_raw, None);
        assert_eq!(
            flow.devices[3].installation_id, None,
            "placed twice: not guessed"
        );
    }

    #[test]
    fn groups_list_send_and_receive_members_with_stated_flags_and_activation() {
        let flow = build(&project());
        let light = flow
            .groups
            .iter()
            .find(|g| g.ga_id == 1)
            .expect("GA 1 is listed");
        assert_eq!(light.ga_raw, 1);
        assert_eq!(light.installation_id, 1);
        assert_eq!(light.dpt.as_deref(), Some("DPST-1-1"));
        let members: Vec<(u32, Direction, bool)> = light
            .members
            .iter()
            .map(|m| (m.com_object_id, m.direction, m.active))
            .collect();
        assert_eq!(
            members,
            vec![
                (10, Direction::Send, true),
                (11, Direction::Receive, true),
                (13, Direction::Receive, false)
            ]
        );
        assert_eq!(light.members[0].flags.write, Some(true));
        assert_eq!(light.members[0].flags.transmit, Some(false));
        assert_eq!(light.members[0].flags.read, None, "absent is not false");
        assert!(light.members[1].flags.is_unknown());
        let unlinked = flow.groups.iter().find(|g| g.ga_id == 2).unwrap();
        assert_eq!(
            unlinked.members.len(),
            1,
            "object of an unknown device stays"
        );
        assert_eq!(unlinked.members[0].device_id, 77);
    }

    #[test]
    fn inconsistencies_are_diagnosed_not_resolved() {
        let flow = build(&project());
        let d = &flow.diagnostics;
        assert_eq!(
            d.duplicate_individual_addresses,
            vec![(IndividualAddress::new(1, 1, 1).unwrap().raw(), vec![1, 2])]
        );
        assert_eq!(d.ambiguous_group_addresses, vec![(1, vec![1, 20])]);
        assert_eq!(d.ambiguous_devices, vec![4]);
        assert_eq!(d.dangling_links, vec![(12, 99)]);
        assert_eq!(d.unknown_devices, vec![(14, 77)]);
        assert_eq!(d.objects_without_flags, vec![11]);
        assert!(
            !flow
                .groups
                .iter()
                .any(|g| g.members.iter().any(|m| m.com_object_id == 12)),
            "a dangling link is no member"
        );
    }

    #[test]
    fn equal_projects_give_equal_snapshots_and_link_or_flag_edits_do_not() {
        let base = build(&project());
        assert_eq!(base, build(&project()));

        let mut flags = project();
        flags
            .devices
            .com_object_mut(ComObjectInstanceId(10))
            .unwrap()
            .flags
            .read = flag(true);
        assert_ne!(base, build(&flags));

        let mut direction = project();
        direction
            .devices
            .com_object_mut(ComObjectInstanceId(11))
            .unwrap()
            .links[0]
            .direction = Direction::Send;
        assert_ne!(base, build(&direction));

        let mut renamed = project();
        renamed.devices.get_mut(DeviceId(1)).unwrap().name = "Renamed".into();
        assert_ne!(base, build(&renamed));

        let mut inactive = project();
        inactive
            .devices
            .com_object_mut(ComObjectInstanceId(10))
            .unwrap()
            .is_active = false;
        assert_ne!(base, build(&inactive));
    }

    #[test]
    fn bounded_lists_count_every_omitted_entry() {
        let flow = build(&project());
        let all = FlowListsDto::bounded(&flow, FLOW_LIMITS);
        assert_eq!(all.truncated, TruncatedDto::default());
        assert_eq!(all.devices.len(), 4);

        let tight = FlowListsDto::bounded(
            &flow,
            FlowLimits {
                devices: 1,
                groups: 1,
                members: 2,
                diagnostics: 0,
            },
        );
        assert_eq!(tight.devices.len(), 1);
        assert_eq!(tight.groups.len(), 1);
        assert_eq!(tight.groups[0].members.len(), 2);
        let total_groups = flow.groups.len();
        let total_members: usize = flow.groups.iter().map(|g| g.members.len()).sum();
        let d = &flow.diagnostics;
        let total_diagnostics = d.duplicate_individual_addresses.len()
            + d.ambiguous_group_addresses.len()
            + d.ambiguous_devices.len()
            + d.dangling_links.len()
            + d.unknown_devices.len()
            + d.objects_without_flags.len();
        assert_eq!(
            tight.truncated,
            TruncatedDto {
                devices: 3,
                groups: total_groups - 1,
                members: total_members - 2,
                diagnostics: total_diagnostics,
            }
        );
    }

    #[test]
    fn no_project_facts_give_an_empty_snapshot() {
        let empty = Project::new(Language("en".into()));
        assert_eq!(build(&empty), FlowParticipants::default());
    }

    #[test]
    fn member_wire_form_uses_direction_names_and_nullable_flags() {
        let flow = build(&project());
        let json = serde_json::to_value(FlowListsDto::bounded(&flow, FLOW_LIMITS)).unwrap();
        let light = json["groups"]
            .as_array()
            .unwrap()
            .iter()
            .find(|g| g["gaId"] == 1)
            .unwrap();
        assert_eq!(light["gaRaw"], 1);
        assert_eq!(light["members"][0]["direction"], "Send");
        assert_eq!(light["members"][0]["flags"]["write"], true);
        assert!(light["members"][0]["flags"]["readOnInit"].is_null());
        assert_eq!(json["diagnostics"]["danglingLinks"][0]["gaId"], 99);
        assert_eq!(json["truncated"]["members"], 0);
    }
}
