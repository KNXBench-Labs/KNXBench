//! Display-shaped projections of [`knx_core::Project`] for the desktop UI
//! (ADR-0009). The UI never receives `Project` itself — only these types,
//! generated into TypeScript by `ts-rs` so the two sides cannot disagree
//! without the build failing. Depends on `knx-core` only: no IO, no format,
//! no storage (`xtask check-layering` enforces this, same rule as
//! `knx-core` itself).

use std::collections::HashMap;

use knx_core::{BuildingPart, BuildingPartId, BuildingPartType, Devices, Project, Topology};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
pub struct ProjectTree {
    pub schema_version: u32,
    /// Count of import-report items (errors, unknown constructs, DPT
    /// conflicts, documented capability gaps) that a caller with access to
    /// the `ImportReport` should fill in — always `0` straight out of
    /// [`build_project_tree`], since this crate never sees that type
    /// (CLAUDE.md: never silently discard information; full drill-down is
    /// a later cycle, this is the count that says something is worth
    /// looking at).
    pub warnings: usize,
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
}

/// Builds the full display tree for every installation in `project`. Pure
/// and total: never panics on a project that imported successfully, even
/// one with dangling `BuildingPart` device references (knx-etsproj's
/// `validate.rs` does not check those — see the doc comment there).
pub fn build_project_tree(project: &Project) -> ProjectTree {
    ProjectTree {
        schema_version: project.schema_version,
        warnings: 0,
        installations: project
            .installations
            .iter()
            .map(|inst| build_installation(inst, &project.devices))
            .collect(),
    }
}

fn build_installation(inst: &knx_core::Installation, devices: &Devices) -> InstallationNode {
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
}
