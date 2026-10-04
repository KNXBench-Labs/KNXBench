//! MODEL-01: every installation is editable, not only the first one.
//!
//! A project may hold several installations (separate infrastructures,
//! ADR-0038). Commands addressed by an entity id act in the installation that
//! owns that entity; nothing silently lands in or is looked up only in the
//! first installation, and nothing connects two installations.

use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
    ComObjectInstanceId, Command, CommandError, CommandStack, CommissioningState, CompletionStatus,
    DeviceId, DeviceInstance, Direction, GroupAddress, GroupAddressEntry, GroupAddressId,
    GroupRange, GroupRangeId, IndividualAddress, Installation, InstallationId, Language, Line,
    LineId, Override, ParameterInstanceId, Project, ResolvedFlags, SourceRef, Topology,
};

fn source() -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    }
}

fn line(id: u32, address: u8, devices: Vec<DeviceId>) -> Line {
    Line {
        id: LineId(id),
        source: source(),
        name: format!("L{id}"),
        address,
        medium_ref: "TP".into(),
        domain_address: None,
        domain_address_is_checked: None,
        ip_routing_multicast_address: None,
        multicast_ttl: None,
        completion: CompletionStatus::FinishedDesign,
        devices,
    }
}

fn area(id: u32, address: u8, lines: Vec<LineId>) -> Area {
    Area {
        id: AreaId(id),
        source: source(),
        name: format!("A{id}"),
        address,
        completion: CompletionStatus::FinishedDesign,
        lines,
    }
}

fn range(id: u32, start: u16, end: u16) -> GroupRange {
    GroupRange {
        id: GroupRangeId(id),
        source: source(),
        name: format!("R{id}"),
        start: GroupAddress::from_raw(start),
        end: GroupAddress::from_raw(end),
        parent: None,
        children: vec![],
    }
}

fn ga(id: u32, raw: u16, range: Option<u32>) -> GroupAddressEntry {
    GroupAddressEntry {
        id: GroupAddressId(id),
        source: source(),
        name: format!("G{id}"),
        address: GroupAddress::from_raw(raw),
        central: false,
        unfiltered: false,
        range: range.map(GroupRangeId),
    }
}

fn part(id: u32, parent: Option<u32>) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: source(),
        name: format!("B{id}"),
        number: None,
        kind: BuildingPartType::Room,
        default_line: None,
        completion: CompletionStatus::FinishedDesign,
        children: vec![],
        devices: vec![],
        parent: parent.map(BuildingPartId),
    }
}

fn installation(id: u8, name: &str) -> Installation {
    Installation {
        id: InstallationId(id),
        name: name.into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
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

fn device(id: u32) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: source(),
        name: format!("D{id}"),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![ComObjectInstanceId(id)],
        binary_data: vec![],
    }
}

fn com(id: u32) -> ComObjectInstance {
    ComObjectInstance {
        id: ComObjectInstanceId(id),
        source: source(),
        device: DeviceId(id),
        number: 0,
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

/// Installation 0 ("Home"): area 1 / line 1 with device 1, range 1, GA 1,
/// building part 1. Installation 1 ("Garage"): area 2 / lines 2 and 3 with
/// device 2 on line 2, range 2, GA 2, building parts 2 and 3.
fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    let mut home = installation(0, "Home");
    home.topology.areas.push(area(1, 1, vec![LineId(1)]));
    home.topology.lines.push(line(1, 1, vec![DeviceId(1)]));
    home.group_ranges.push(range(1, 0x0800, 0x0FFF));
    home.group_addresses.push(ga(1, 0x0801, Some(1)));
    home.buildings.push(part(1, None));
    let mut garage = installation(1, "Garage");
    garage
        .topology
        .areas
        .push(area(2, 2, vec![LineId(2), LineId(3)]));
    garage.topology.lines.push(line(2, 1, vec![DeviceId(2)]));
    garage.topology.lines.push(line(3, 2, vec![]));
    garage.group_ranges.push(range(2, 0x1000, 0x17FF));
    garage.group_addresses.push(ga(2, 0x1001, Some(2)));
    garage.buildings.push(part(2, None));
    garage.buildings.push(part(3, None));
    project.installations.push(home);
    project.installations.push(garage);
    for id in [1, 2] {
        project.devices.insert(device(id));
        project.devices.insert_com_object(com(id));
    }
    project.ids.raise_to(&knx_core::IdAllocators::from_counts(
        100, 100, 100, 100, 100, 100, 100, 100, 100,
    ));
    project
}

/// Applies, undoes and redoes `command`; the project must return exactly to
/// its state before and after.
fn round_trip(project: &mut Project, command: Command) {
    let before = project.clone();
    let mut stack = CommandStack::new();
    stack.do_command(project, command).unwrap();
    let after = project.clone();
    stack.undo(project).unwrap();
    assert_eq!(*project, before, "undo restores the exact project");
    stack.redo(project).unwrap();
    assert_eq!(*project, after, "redo repeats the exact change");
}

fn garage(project: &Project) -> &Installation {
    &project.installations[1]
}

#[test]
fn renames_reach_entities_of_a_later_installation() {
    let mut project = project();
    round_trip(
        &mut project,
        Command::RenameArea {
            id: AreaId(2),
            name: "Garage area".into(),
        },
    );
    assert_eq!(garage(&project).topology.areas[0].name, "Garage area");
    round_trip(
        &mut project,
        Command::RenameLine {
            id: LineId(2),
            name: "Garage line".into(),
        },
    );
    round_trip(
        &mut project,
        Command::RenameBuildingPart {
            id: BuildingPartId(2),
            name: "Workshop".into(),
        },
    );
    round_trip(
        &mut project,
        Command::RenameGroupRange {
            id: GroupRangeId(2),
            name: "Garage range".into(),
        },
    );
    round_trip(
        &mut project,
        Command::UpdateGroupAddress {
            id: GroupAddressId(2),
            name: "Door".into(),
            central: true,
            unfiltered: false,
        },
    );
    assert_eq!(garage(&project).group_addresses[0].name, "Door");
    assert_eq!(project.installations[0].group_addresses[0].name, "G1");
}

#[test]
fn deletes_and_their_undo_stay_in_the_owning_installation() {
    let mut project = project();
    round_trip(&mut project, Command::DeleteLine { id: LineId(3) });
    assert_eq!(garage(&project).topology.lines.len(), 1);
    assert_eq!(project.installations[0].topology.lines.len(), 1);
    round_trip(
        &mut project,
        Command::DeleteBuildingPart {
            id: BuildingPartId(3),
        },
    );
    assert_eq!(garage(&project).buildings.len(), 1);
}

#[test]
fn creates_under_a_later_parent_land_in_that_installation() {
    let mut project = project();
    round_trip(
        &mut project,
        Command::CreateLine {
            area: AreaId(2),
            line: line(10, 5, vec![]),
        },
    );
    assert!(garage(&project)
        .topology
        .lines
        .iter()
        .any(|l| l.id == LineId(10)));
    round_trip(
        &mut project,
        Command::Batch(vec![
            Command::CreateBuildingPart {
                part: part(11, Some(2)),
                installation: None,
            },
            Command::CreateGroupRange {
                range: GroupRange {
                    parent: Some(GroupRangeId(2)),
                    ..range(12, 0x1100, 0x11FF)
                },
                installation: None,
            },
        ]),
    );
    assert!(garage(&project)
        .buildings
        .iter()
        .any(|p| p.id == BuildingPartId(11)));
    assert!(garage(&project)
        .group_ranges
        .iter()
        .any(|r| r.id == GroupRangeId(12)));
    assert!(project.installations[0].buildings.len() == 1);
}

#[test]
fn devices_move_and_link_within_their_own_installation() {
    let mut project = project();
    round_trip(
        &mut project,
        Command::MoveDeviceToLine {
            device: DeviceId(2),
            line: Some(LineId(3)),
        },
    );
    assert_eq!(garage(&project).topology.lines[1].devices, [DeviceId(2)]);
    round_trip(
        &mut project,
        Command::MoveDeviceToBuildingPart {
            device: DeviceId(2),
            part: Some(BuildingPartId(2)),
        },
    );
    assert_eq!(garage(&project).buildings[0].devices, [DeviceId(2)]);
    round_trip(
        &mut project,
        Command::LinkComObject {
            com_object: ComObjectInstanceId(2),
            ga: GroupAddressId(2),
            direction: Direction::Send,
        },
    );
    assert_eq!(
        project
            .devices
            .com_object(ComObjectInstanceId(2))
            .unwrap()
            .links
            .len(),
        1
    );
}

#[test]
fn a_parameter_edit_is_stored_with_its_devices_installation() {
    let mut project = project();
    round_trip(
        &mut project,
        Command::SetParameterValue {
            id: ParameterInstanceId(50),
            device: DeviceId(2),
            ets_id: "P-1".into(),
            raw: "7".into(),
        },
    );
    assert_eq!(garage(&project).parameters.len(), 1);
    assert!(project.installations[0].parameters.is_empty());
}

#[test]
fn nothing_connects_two_installations() {
    let mut project = project();
    let before = project.clone();
    let mut stack = CommandStack::new();
    let refusals = [
        // Device 1 lives in Home; line 2, part 2 and GA 2 in Garage.
        Command::MoveDeviceToLine {
            device: DeviceId(1),
            line: Some(LineId(2)),
        },
        Command::MoveDeviceToBuildingPart {
            device: DeviceId(1),
            part: Some(BuildingPartId(2)),
        },
        Command::LinkComObject {
            com_object: ComObjectInstanceId(1),
            ga: GroupAddressId(2),
            direction: Direction::Send,
        },
        Command::MoveLineToArea {
            id: LineId(1),
            area: AreaId(2),
        },
        Command::MoveGroupRange {
            id: GroupRangeId(1),
            parent: Some(GroupRangeId(2)),
        },
        Command::MoveBuildingPart {
            id: BuildingPartId(1),
            parent: Some(BuildingPartId(2)),
        },
    ];
    for command in refusals {
        let label = format!("{command:?}");
        let error = stack.do_command(&mut project, command).unwrap_err();
        assert!(
            matches!(error, CommandError::CrossInstallation { .. }),
            "{label}: {error}"
        );
        assert_eq!(project, before, "{label} must not mutate");
    }
}

#[test]
fn an_installation_can_be_renamed_with_undo() {
    let mut project = project();
    round_trip(
        &mut project,
        Command::RenameInstallation {
            id: InstallationId(1),
            name: "Carport".into(),
        },
    );
    assert_eq!(garage(&project).name, "Carport");
    let error = CommandStack::new()
        .do_command(
            &mut project,
            Command::RenameInstallation {
                id: InstallationId(9),
                name: "X".into(),
            },
        )
        .unwrap_err();
    assert_eq!(error, CommandError::InstallationNotFound);
}

#[test]
fn root_creates_can_target_a_later_installation_explicitly() {
    let mut project = project();
    round_trip(
        &mut project,
        Command::Batch(vec![
            Command::CreateArea {
                area: area(20, 3, vec![]),
                installation: Some(InstallationId(1)),
            },
            Command::CreateGroupRange {
                range: range(21, 0x1800, 0x1FFF),
                installation: Some(InstallationId(1)),
            },
            Command::CreateBuildingPart {
                part: part(22, None),
                installation: Some(InstallationId(1)),
            },
            Command::CreateGroupAddress {
                entry: ga(23, 0x1802, Some(21)),
                installation: None,
            },
            Command::CreateGroupAddress {
                entry: ga(24, 0x2000, None),
                installation: Some(InstallationId(1)),
            },
        ]),
    );
    let garage = garage(&project);
    assert!(garage.topology.areas.iter().any(|a| a.id == AreaId(20)));
    assert!(garage.group_ranges.iter().any(|r| r.id == GroupRangeId(21)));
    assert!(garage.buildings.iter().any(|p| p.id == BuildingPartId(22)));
    assert!(garage
        .group_addresses
        .iter()
        .any(|g| g.id == GroupAddressId(23)));
    assert!(garage
        .group_addresses
        .iter()
        .any(|g| g.id == GroupAddressId(24)));
    // The legacy default still means the first installation.
    let mut project = self::project();
    CommandStack::new()
        .do_command(
            &mut project,
            Command::CreateArea {
                area: area(30, 4, vec![]),
                installation: None,
            },
        )
        .unwrap();
    assert_eq!(project.installations[0].topology.areas.len(), 2);
    // A parent in another installation than the explicit target is refused.
    let error = CommandStack::new()
        .do_command(
            &mut project,
            Command::CreateGroupAddress {
                entry: ga(31, 0x0802, Some(1)),
                installation: Some(InstallationId(1)),
            },
        )
        .unwrap_err();
    assert!(
        matches!(error, CommandError::CrossInstallation { .. }),
        "{error}"
    );
}

#[test]
fn addresses_in_a_later_installation_still_follow_its_line() {
    let mut project = project();
    round_trip(
        &mut project,
        Command::SetIndividualAddress {
            device: DeviceId(2),
            address: Some(IndividualAddress::new(2, 1, 5).unwrap()),
        },
    );
}

#[test]
fn an_imported_cross_installation_building_placement_stays_undoable() {
    let mut project = project();
    // Imported oddity: device 1 sits in Home's topology but in Garage's part 2.
    project.installations[1].buildings[0]
        .devices
        .push(DeviceId(1));
    round_trip(
        &mut project,
        Command::MoveDeviceToBuildingPart {
            device: DeviceId(1),
            part: Some(BuildingPartId(1)),
        },
    );
    assert_eq!(project.installations[0].buildings[0].devices, [DeviceId(1)]);
    assert!(garage(&project).buildings[0].devices.is_empty());
    round_trip(
        &mut project,
        Command::MoveDeviceToBuildingPart {
            device: DeviceId(1),
            part: None,
        },
    );
}
