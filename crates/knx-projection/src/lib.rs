//! Display-shaped projections of [`knx_core::Project`] for the desktop UI
//! (ADR-0009). The UI never receives `Project` itself — only these types,
//! generated into TypeScript by `ts-rs` so the two sides cannot disagree
//! without the build failing. Depends on `knx-core` only: no IO, no format,
//! no storage (`xtask check-layering` enforces this, same rule as
//! `knx-core` itself).

use std::collections::HashMap;

use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, Devices, GroupAddressEntry, GroupAddressStyle,
    GroupRange, Project, Topology,
};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ProjectTree {
    pub schema_version: u32,
    /// Count of genuine `Severity::Error` items from the `ImportReport` —
    /// data actually lost or misread, as `knx_etsproj::report::Severity`
    /// distinguishes it (see `ImportReport::has_losses()`) — that a caller
    /// with access to the `ImportReport` should fill in — always `0`
    /// straight out of [`build_project_tree`], since this crate never sees
    /// that type (CLAUDE.md: never silently discard information; full
    /// drill-down is a later cycle, this is the count that says something
    /// was genuinely lost, never to be shown to the user as a mere
    /// "warning").
    pub errors: usize,
    /// Count of everything else worth a look but not a real loss:
    /// `Severity::Warning` items, unknown constructs, DPT conflicts, and
    /// documented capability gaps — that a caller with access to the
    /// `ImportReport` should fill in — always `0` straight out of
    /// [`build_project_tree`], since this crate never sees that type
    /// (CLAUDE.md: never silently discard information; full drill-down is
    /// a later cycle, this is the count that says something is worth
    /// looking at).
    pub warnings: usize,
    /// Always `false` straight out of [`build_project_tree`] — this crate
    /// never sees a `CommandStack`. The desktop shell overlays the real
    /// value from its own `CommandStack` after every command/undo/redo.
    pub can_undo: bool,
    /// See `can_undo`.
    pub can_redo: bool,
    pub installations: Vec<InstallationNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct InstallationNode {
    pub id: u8,
    pub name: String,
    pub topology: Vec<AreaNode>,
    pub buildings: Vec<BuildingNode>,
    pub unassigned: Vec<DeviceNode>,
    pub group_addresses: Vec<GroupAddressNode>,
    pub group_ranges: Vec<GroupRangeNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct AreaNode {
    pub id: u32,
    pub name: String,
    pub address: u8,
    pub lines: Vec<LineNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct LineNode {
    pub id: u32,
    pub name: String,
    pub address: u8,
    pub devices: Vec<DeviceNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct BuildingNode {
    pub id: u32,
    pub name: String,
    /// `BuildingPartType` as a plain string (e.g. `"Room"`, `"Floor"`) — the
    /// enum itself stays in `knx-core`; a typed TS union is not worth the
    /// extra `ts-rs` surface for a single label this cycle.
    pub kind: String,
    pub children: Vec<BuildingNode>,
    pub devices: Vec<DeviceNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DeviceNode {
    pub id: u32,
    pub name: String,
    /// Formatted individual address (e.g. `"1.1.1"`) — `None` if the device
    /// has no address assigned, which is valid project state.
    pub address: Option<String>,
    pub description: Option<String>,
    /// Count of this device's communication objects, regardless of
    /// `is_active` — the dashboard's project-wide total sums this field
    /// across every `DeviceNode` it visits (Session 5, cycle 8).
    pub com_object_count: usize,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroupAddressNode {
    pub id: u32,
    pub name: String,
    /// Formatted per the project's own `GroupAddressStyle`
    /// (`GroupAddress::format`), e.g. `"4/2/100"`.
    pub address: String,
}

/// A flat (not nested) view of one `GroupRange` — `parent` names the
/// containing main range's id for a middle range, `None` for a main
/// range. Deliberately does not nest `GroupAddressNode`s inside their
/// range: `InstallationNode.group_addresses` stays a flat list, matching
/// its existing shape, until a future cycle redesigns the group-address
/// tree branch around the real main/middle/address hierarchy.
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroupRangeNode {
    pub id: u32,
    pub name: String,
    /// Formatted per the project's own `GroupAddressStyle`, same as
    /// `GroupAddressNode::address`.
    pub start: String,
    pub end: String,
    pub parent: Option<u32>,
}

/// Builds the full display tree for every installation in `project`. Pure
/// and total: never panics on a project that imported successfully, even
/// one with dangling `BuildingPart` device references (knx-etsproj's
/// `validate.rs` does not check those — see the doc comment there).
pub fn build_project_tree(project: &Project) -> ProjectTree {
    ProjectTree {
        schema_version: project.schema_version,
        errors: 0,
        warnings: 0,
        can_undo: false,
        can_redo: false,
        installations: project
            .installations
            .iter()
            .map(|inst| {
                build_installation(inst, &project.devices, project.info.group_address_style)
            })
            .collect(),
    }
}

fn build_installation(
    inst: &knx_core::Installation,
    devices: &Devices,
    ga_style: GroupAddressStyle,
) -> InstallationNode {
    InstallationNode {
        id: inst.id.0,
        name: inst.name.clone(),
        topology: build_topology(&inst.topology, devices),
        buildings: build_building_forest(&inst.buildings, devices),
        unassigned: inst
            .topology
            .unassigned
            .iter()
            .filter_map(|id| devices.get(*id))
            .map(build_device_node)
            .collect(),
        group_addresses: inst
            .group_addresses
            .iter()
            .map(|entry| build_group_address_node(entry, ga_style))
            .collect(),
        group_ranges: inst
            .group_ranges
            .iter()
            .map(|range| build_group_range_node(range, ga_style))
            .collect(),
    }
}

fn build_group_address_node(
    entry: &GroupAddressEntry,
    style: GroupAddressStyle,
) -> GroupAddressNode {
    GroupAddressNode {
        id: entry.id.0,
        name: entry.name.clone(),
        address: entry.address.format(style),
    }
}

fn build_group_range_node(range: &GroupRange, style: GroupAddressStyle) -> GroupRangeNode {
    GroupRangeNode {
        id: range.id.0,
        name: range.name.clone(),
        start: range.start.format(style),
        end: range.end.format(style),
        parent: range.parent.map(|p| p.0),
    }
}

fn build_topology(topology: &Topology, devices: &Devices) -> Vec<AreaNode> {
    topology
        .areas
        .iter()
        .map(|area| AreaNode {
            id: area.id.0,
            name: area.name.clone(),
            address: area.address,
            lines: area
                .lines
                .iter()
                .filter_map(|line_id| topology.line(*line_id))
                .map(|line| LineNode {
                    id: line.id.0,
                    name: line.name.clone(),
                    address: line.address,
                    devices: line
                        .devices
                        .iter()
                        .filter_map(|id| devices.get(*id))
                        .map(build_device_node)
                        .collect(),
                })
                .collect(),
        })
        .collect()
}

fn build_device_node(device: &knx_core::DeviceInstance) -> DeviceNode {
    DeviceNode {
        id: device.id.0,
        name: device.name.clone(),
        address: device.address.map(|a| a.to_string()),
        description: device.description.clone(),
        com_object_count: device.com_objects.len(),
    }
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct DeviceDetail {
    pub id: u32,
    pub name: String,
    pub description: Option<String>,
    /// Formatted individual address (e.g. `"1.1.1"`), `None` if unassigned.
    pub address: Option<String>,
    pub com_objects: Vec<ComObjectNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ComObjectNode {
    pub id: u32,
    /// From `_O-<n>` in the source `RefId`.
    pub number: u16,
    pub name: Option<String>,
    /// Formatted datapoint type reference (e.g. `"DPST-1-1"`, `"DPT-1"`),
    /// `None` if never stated at any layer.
    pub dpt: Option<String>,
    /// The layer `dpt` resolved from (`"Program"`, `"ProgramRef"`,
    /// `"Instance"`, `"Inferred"`, `"UserEdit"`), `None` alongside `dpt:
    /// None`.
    pub dpt_layer: Option<String>,
    /// Resolved through the project's string table, same as `name` — `None`
    /// if never stated at any layer.
    pub description: Option<String>,
    /// The layer `description` resolved from, `None` alongside
    /// `description: None`.
    pub description_layer: Option<String>,
    pub is_active: bool,
    /// Editable via `Command::SetComObjectFlag` (one flag at a time) —
    /// see `ComFlagKind` in `knx-core`.
    pub read: bool,
    pub write: bool,
    pub transmit: bool,
    pub update: bool,
    pub communication: bool,
    /// The `GroupLink`s already on this communication object —
    /// `knx_core::Command::LinkComObject`/`UnlinkComObject` (2026-09-06)
    /// had no projection field to read or drive from until this cycle.
    pub links: Vec<GroupLinkNode>,
}

/// One directional link from a communication object to a group address,
/// as seen from the communication object's side. `address`/`name` are
/// `None` only if `ga_id` names no address anywhere in the project — a
/// dangling link, which `Command::DeleteGroupAddress` already refuses to
/// create (`CommandError::GroupAddressInUse`) but this stays defensive
/// rather than panicking on data that reached the model some other way
/// (e.g. a future import path that doesn't route through that check).
#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct GroupLinkNode {
    pub ga_id: u32,
    /// Formatted per the project's own `GroupAddressStyle`, same as
    /// `GroupAddressNode::address`.
    pub address: Option<String>,
    pub name: Option<String>,
    /// `"Send"` or `"Receive"` (`Direction`'s `Debug` form, same
    /// convention as `dpt_layer`/`description_layer`).
    pub direction: String,
}

/// Builds the detail panel for one device, resolving each communication
/// object's text through the project's string table. `None` if `id` does
/// not name a device in `project` (a stale selection after an edit, for
/// instance).
pub fn build_device_detail(project: &Project, id: knx_core::DeviceId) -> Option<DeviceDetail> {
    let device = project.devices.get(id)?;
    Some(DeviceDetail {
        id: device.id.0,
        name: device.name.clone(),
        description: device.description.clone(),
        address: device.address.map(|a| a.to_string()),
        com_objects: device
            .com_objects
            .iter()
            .filter_map(|com_id| project.devices.com_object(*com_id))
            .map(|com| build_com_object_node(com, project))
            .collect(),
    })
}

fn build_com_object_node(com: &knx_core::ComObjectInstance, project: &Project) -> ComObjectNode {
    let name = com.text.value().and_then(|resolved| {
        project
            .strings
            .text(&resolved.value, project.strings.default_language())
            .map(|s| s.to_string())
    });
    let dpt = com.dpt.value().map(|resolved| resolved.value.to_string());
    let dpt_layer = com.dpt.layer().map(|layer| format!("{layer:?}"));
    let description = com.description.value().and_then(|resolved| {
        project
            .strings
            .text(&resolved.value, project.strings.default_language())
            .map(|s| s.to_string())
    });
    let description_layer = com.description.layer().map(|layer| format!("{layer:?}"));
    let flag = |o: &knx_core::Override<bool>| o.value().map(|r| r.value).unwrap_or(false);
    ComObjectNode {
        id: com.id.0,
        number: com.number,
        name,
        dpt,
        dpt_layer,
        description,
        description_layer,
        is_active: com.is_active,
        read: flag(&com.flags.read),
        write: flag(&com.flags.write),
        transmit: flag(&com.flags.transmit),
        update: flag(&com.flags.update),
        communication: flag(&com.flags.communication),
        links: com
            .links
            .iter()
            .map(|link| build_group_link_node(link, project))
            .collect(),
    }
}

/// Finds a `GroupAddressEntry` by id across every installation — a
/// `GroupLink` names its target by id alone, with no installation
/// context of its own to narrow the search.
fn find_group_address_entry(
    project: &Project,
    id: knx_core::GroupAddressId,
) -> Option<&GroupAddressEntry> {
    project
        .installations
        .iter()
        .flat_map(|inst| inst.group_addresses.iter())
        .find(|entry| entry.id == id)
}

fn build_group_link_node(link: &knx_core::GroupLink, project: &Project) -> GroupLinkNode {
    let entry = find_group_address_entry(project, link.ga);
    GroupLinkNode {
        ga_id: link.ga.0,
        address: entry.map(|e| e.address.format(project.info.group_address_style)),
        name: entry.map(|e| e.name.clone()),
        direction: format!("{:?}", link.direction),
    }
}

/// `BuildingPart`s are stored flat, linked by `parent`/`children` ids
/// (DATA_MODEL §5) — this resolves that into the actual nested shape the
/// tree needs, once, in Rust, per ADR-0009.
fn build_building_forest(parts: &[BuildingPart], devices: &Devices) -> Vec<BuildingNode> {
    let by_id: HashMap<BuildingPartId, &BuildingPart> = parts.iter().map(|p| (p.id, p)).collect();

    parts
        .iter()
        .filter(|p| p.parent.is_none())
        .map(|root| build_building_node(root, &by_id, devices))
        .collect()
}

fn build_building_node(
    part: &BuildingPart,
    by_id: &HashMap<BuildingPartId, &BuildingPart>,
    devices: &Devices,
) -> BuildingNode {
    BuildingNode {
        id: part.id.0,
        name: part.name.clone(),
        kind: building_kind_str(part.kind).to_string(),
        children: part
            .children
            .iter()
            .filter_map(|id| by_id.get(id))
            .map(|child| build_building_node(child, by_id, devices))
            .collect(),
        devices: part
            .devices
            .iter()
            .filter_map(|id| devices.get(*id))
            .map(build_device_node)
            .collect(),
    }
}

fn building_kind_str(kind: BuildingPartType) -> &'static str {
    match kind {
        BuildingPartType::Building => "Building",
        BuildingPartType::Floor => "Floor",
        BuildingPartType::Room => "Room",
        BuildingPartType::Corridor => "Corridor",
        BuildingPartType::DistributionBoard => "DistributionBoard",
        BuildingPartType::BuildingPart => "BuildingPart",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::{
        Area, BuildingPart, BuildingPartType, CommissioningState, CompletionStatus, DeviceId,
        DeviceInstance, IndividualAddress, Installation, InstallationId, Language, Line, Project,
        SourceRef, Topology,
    };

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn device(id: u32, name: &str, address: Option<(u8, u8, u8)>) -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(id),
            source: source(),
            name: name.into(),
            description: None,
            address: address.map(|(a, l, d)| IndividualAddress::new(a, l, d).unwrap()),
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    fn building(
        id: u32,
        name: &str,
        kind: BuildingPartType,
        parent: Option<u32>,
        children: Vec<u32>,
        devices: Vec<u32>,
    ) -> BuildingPart {
        BuildingPart {
            id: knx_core::BuildingPartId(id),
            source: source(),
            name: name.into(),
            number: None,
            kind,
            default_line: None,
            completion: CompletionStatus::Undefined,
            children: children.into_iter().map(knx_core::BuildingPartId).collect(),
            devices: devices.into_iter().map(DeviceId).collect(),
            parent: parent.map(knx_core::BuildingPartId),
        }
    }

    fn empty_installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::Undefined,
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

    #[test]
    fn empty_project_produces_an_empty_tree() {
        let project = Project::new(Language("en".into()));
        let tree = build_project_tree(&project);
        assert_eq!(tree.schema_version, project.schema_version);
        assert_eq!(tree.errors, 0);
        assert_eq!(tree.warnings, 0);
        assert!(tree.installations.is_empty());
    }

    #[test]
    fn installation_with_no_buildings_or_topology_has_empty_children() {
        let mut project = Project::new(Language("en".into()));
        project.installations.push(empty_installation());
        let tree = build_project_tree(&project);
        let inst = &tree.installations[0];
        assert!(inst.topology.is_empty());
        assert!(inst.buildings.is_empty());
        assert!(inst.unassigned.is_empty());
    }

    #[test]
    fn topology_resolves_area_line_device_in_order() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(1, "Dimmer", Some((1, 1, 1))));
        project.devices.insert(device(2, "Switch", None));

        let mut inst = empty_installation();
        inst.topology.areas.push(Area {
            id: knx_core::AreaId(1),
            source: source(),
            name: "Area 1".into(),
            address: 1,
            completion: CompletionStatus::Undefined,
            lines: vec![knx_core::LineId(1)],
        });
        inst.topology.lines.push(Line {
            id: knx_core::LineId(1),
            source: source(),
            name: "Line 1".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::Undefined,
            devices: vec![DeviceId(1), DeviceId(2)],
        });
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        let area = &tree.installations[0].topology[0];
        assert_eq!(area.name, "Area 1");
        assert_eq!(area.lines[0].devices.len(), 2);
        assert_eq!(area.lines[0].devices[0].name, "Dimmer");
        assert_eq!(area.lines[0].devices[0].address.as_deref(), Some("1.1.1"));
        assert_eq!(area.lines[0].devices[1].address, None);
    }

    #[test]
    fn unassigned_devices_form_their_own_bucket() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(9, "Orphan", None));
        let mut inst = empty_installation();
        inst.topology.unassigned.push(DeviceId(9));
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        assert_eq!(tree.installations[0].unassigned.len(), 1);
        assert_eq!(tree.installations[0].unassigned[0].name, "Orphan");
    }

    #[test]
    fn building_hierarchy_nests_by_parent_child_not_flat() {
        let mut project = Project::new(Language("en".into()));
        project.devices.insert(device(5, "Lamp", None));
        let mut inst = empty_installation();
        inst.buildings = vec![
            building(
                1,
                "Building",
                BuildingPartType::Building,
                None,
                vec![2],
                vec![],
            ),
            building(
                2,
                "Floor 1",
                BuildingPartType::Floor,
                Some(1),
                vec![3],
                vec![],
            ),
            building(
                3,
                "Room 1",
                BuildingPartType::Room,
                Some(2),
                vec![],
                vec![5],
            ),
        ];
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        let roots = &tree.installations[0].buildings;
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].name, "Building");
        assert_eq!(roots[0].kind, "Building");
        assert_eq!(roots[0].children[0].name, "Floor 1");
        assert_eq!(roots[0].children[0].children[0].name, "Room 1");
        assert_eq!(roots[0].children[0].children[0].devices[0].name, "Lamp");
    }

    #[test]
    fn a_dangling_building_device_reference_is_dropped_not_panicked() {
        // knx-etsproj's validate.rs deliberately does not check BuildingPart
        // device references for dangling ids (no measured case has motivated
        // it yet) — the projection must stay resilient to that gap rather
        // than crash the whole desktop app over one malformed project.
        let mut project = Project::new(Language("en".into()));
        let mut inst = empty_installation();
        inst.buildings = vec![building(
            1,
            "Room",
            BuildingPartType::Room,
            None,
            vec![],
            vec![404],
        )];
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        assert!(tree.installations[0].buildings[0].devices.is_empty());
    }

    #[test]
    fn build_device_detail_resolves_name_dpt_and_layer_for_each_com_object() {
        let project = project_with_one_device();
        let detail = build_device_detail(&project, knx_core::DeviceId(1)).unwrap();

        assert_eq!(detail.id, 1);
        assert_eq!(detail.name, "Switch");
        assert_eq!(detail.description.as_deref(), Some("Hallway switch"));
        assert_eq!(detail.address.as_deref(), Some("1.1.1"));
        assert_eq!(detail.com_objects.len(), 1);

        let com = &detail.com_objects[0];
        assert_eq!(com.number, 0);
        assert_eq!(com.name.as_deref(), Some("Switch on/off"));
        assert_eq!(com.dpt.as_deref(), Some("DPST-1-1"));
        assert_eq!(com.dpt_layer.as_deref(), Some("UserEdit"));
        assert_eq!(com.description.as_deref(), Some("Hallway light switch"));
        assert_eq!(com.description_layer.as_deref(), Some("Instance"));
        assert!(com.is_active);
        assert!(!com.read); // ResolvedFlags::none() sets nothing
        assert!(com.links.is_empty());
    }

    #[test]
    fn build_device_detail_projects_a_group_link_with_its_resolved_address_and_name() {
        let mut project = project_with_one_device();
        let mut inst = empty_installation();
        inst.group_addresses.push(knx_core::GroupAddressEntry {
            id: knx_core::GroupAddressId(9),
            source: source(),
            name: "Hallway light on/off".into(),
            address: knx_core::GroupAddress::parse(
                "1/1/1",
                knx_core::GroupAddressStyle::ThreeLevel,
            )
            .unwrap(),
            central: false,
            unfiltered: false,
            range: None,
        });
        project.installations.push(inst);
        project
            .devices
            .com_object_mut(knx_core::ComObjectInstanceId(1))
            .unwrap()
            .links
            .push(knx_core::GroupLink {
                ga: knx_core::GroupAddressId(9),
                direction: knx_core::Direction::Send,
            });

        let detail = build_device_detail(&project, knx_core::DeviceId(1)).unwrap();
        let links = &detail.com_objects[0].links;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].ga_id, 9);
        assert_eq!(links[0].address.as_deref(), Some("1/1/1"));
        assert_eq!(links[0].name.as_deref(), Some("Hallway light on/off"));
        assert_eq!(links[0].direction, "Send");
    }

    #[test]
    fn build_device_detail_stays_defensive_on_a_dangling_group_link() {
        // `Command::DeleteGroupAddress` already refuses to create this state
        // (`CommandError::GroupAddressInUse`), but the projection stays
        // defensive against data that reached the model some other way,
        // same rationale as `a_dangling_building_device_reference_is_dropped_not_panicked`.
        let mut project = project_with_one_device();
        project
            .devices
            .com_object_mut(knx_core::ComObjectInstanceId(1))
            .unwrap()
            .links
            .push(knx_core::GroupLink {
                ga: knx_core::GroupAddressId(404),
                direction: knx_core::Direction::Receive,
            });

        let detail = build_device_detail(&project, knx_core::DeviceId(1)).unwrap();
        let links = &detail.com_objects[0].links;
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].ga_id, 404);
        assert_eq!(links[0].address, None);
        assert_eq!(links[0].name, None);
        assert_eq!(links[0].direction, "Receive");
    }

    #[test]
    fn build_device_detail_returns_none_for_an_unknown_device() {
        let project = project_with_one_device();
        assert!(build_device_detail(&project, knx_core::DeviceId(99)).is_none());
    }

    #[test]
    fn device_node_carries_its_communication_object_count() {
        let mut project = project_with_one_device();
        let mut inst = empty_installation();
        inst.topology.unassigned.push(knx_core::DeviceId(1));
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        assert_eq!(tree.installations[0].unassigned[0].com_object_count, 1);
    }

    fn project_with_one_device() -> Project {
        use knx_core::{
            ComObjectInstance, ComObjectInstanceId, CommissioningState, DeviceId, DptRef,
            IndividualAddress, Layer, ResolvedFlags, Text,
        };

        let mut project = Project::new(Language("en".into()));
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: source(),
            name: "Switch".into(),
            description: Some("Hallway switch".into()),
            address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![ComObjectInstanceId(1)],
            binary_data: vec![],
        });
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: knx_core::Override::Value(knx_core::Resolved {
                value: Text::Literal("Switch on/off".into()),
                layer: Layer::Program,
            }),
            description: knx_core::Override::Value(knx_core::Resolved {
                value: Text::Literal("Hallway light switch".into()),
                layer: Layer::Instance,
            }),
            dpt: knx_core::Override::Value(knx_core::Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::UserEdit,
            }),
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        project
    }

    #[test]
    fn group_addresses_are_projected_and_formatted_per_project_style() {
        let mut project = Project::new(Language("en".into()));
        project.info.group_address_style = knx_core::GroupAddressStyle::TwoLevel;
        let mut inst = empty_installation();
        inst.group_addresses.push(knx_core::GroupAddressEntry {
            id: knx_core::GroupAddressId(1),
            source: source(),
            name: "Living room light".into(),
            address: knx_core::GroupAddress::parse("4/612", knx_core::GroupAddressStyle::TwoLevel)
                .unwrap(),
            central: false,
            unfiltered: false,
            range: None,
        });
        project.installations.push(inst);

        let tree = build_project_tree(&project);
        let ga = &tree.installations[0].group_addresses[0];
        assert_eq!(ga.id, 1);
        assert_eq!(ga.name, "Living room light");
        assert_eq!(ga.address, "4/612");
    }

    #[test]
    fn group_ranges_are_projected_with_their_parent_link() {
        let mut project = knx_core::Project::new(knx_core::Language("en".into()));
        project.installations.push(knx_core::Installation {
            id: knx_core::InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: knx_core::CompletionStatus::FinishedDesign,
            topology: knx_core::Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![
                knx_core::GroupRange {
                    id: knx_core::GroupRangeId(1),
                    source: knx_core::SourceRef {
                        path: "t".into(),
                        ets_id: "t".into(),
                    },
                    name: "Main".into(),
                    start: knx_core::GroupAddress::from_raw(0),
                    end: knx_core::GroupAddress::from_raw(2047),
                    parent: None,
                    children: vec![knx_core::GroupRangeId(2)],
                },
                knx_core::GroupRange {
                    id: knx_core::GroupRangeId(2),
                    source: knx_core::SourceRef {
                        path: "t".into(),
                        ets_id: "t".into(),
                    },
                    name: "Middle".into(),
                    start: knx_core::GroupAddress::from_raw(0),
                    end: knx_core::GroupAddress::from_raw(255),
                    parent: Some(knx_core::GroupRangeId(1)),
                    children: vec![],
                },
            ],
            group_addresses: vec![],
            parameters: vec![],
        });
        let tree = build_project_tree(&project);
        let ranges = &tree.installations[0].group_ranges;
        assert_eq!(ranges.len(), 2);
        assert_eq!(ranges[0].id, 1);
        assert_eq!(ranges[0].parent, None);
        assert_eq!(ranges[1].parent, Some(1));
    }
}
