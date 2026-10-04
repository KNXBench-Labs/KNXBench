//! MODEL-02: explicit, undoable repair of ambiguous imported topology.
//!
//! An imported project may place a device twice (two lines, twice on one
//! line, line and unassigned, twice unassigned) or list a line under two
//! areas. Editing then stays refused because KNXBench will not guess. These
//! repair commands let the user say which existing placement is the real
//! one; every other occurrence is removed, nothing else changes, and undo
//! restores the exact imported state.

use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
    ComObjectInstanceId, Command, CommandError, CommandStack, CommissioningState, CompletionStatus,
    DeviceId, DeviceInstance, DevicePlacementSlot, GroupAddress, GroupAddressEntry, GroupAddressId,
    GroupRange, GroupRangeId, Installation, InstallationId, Language, Line, LineId, Override,
    Project, ResolvedFlags, SourceRef, Topology,
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
/// Leaves `project` in the redone (repaired) state.
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

fn refused(project: &mut Project, command: Command) -> CommandError {
    let before = project.clone();
    let error = command.apply(project).unwrap_err();
    assert_eq!(
        *project, before,
        "a refused repair leaves the project untouched"
    );
    error
}

#[test]
fn a_device_on_two_lines_keeps_only_the_chosen_line() {
    let mut project = project();
    // Imported oddity: device 1 also sits on Home's second line 4.
    project.installations[0]
        .topology
        .lines
        .push(line(4, 2, vec![DeviceId(1)]));
    project.installations[0].topology.areas[0]
        .lines
        .push(LineId(4));
    let move_before = Command::MoveDeviceToLine {
        device: DeviceId(1),
        line: None,
    }
    .apply(&mut project.clone());
    assert!(
        move_before.is_err(),
        "the ambiguity blocks ordinary editing"
    );

    let repair = Command::RepairDevicePlacement {
        device: DeviceId(1),
        keep: DevicePlacementSlot::Line(LineId(4)),
    };
    round_trip(&mut project, repair.clone());
    assert!(project.installations[0].topology.lines[0]
        .devices
        .is_empty());
    assert_eq!(
        project.installations[0].topology.lines[1].devices,
        [DeviceId(1)]
    );
    // Ordinary editing works again.
    Command::MoveDeviceToLine {
        device: DeviceId(1),
        line: None,
    }
    .apply(&mut project)
    .unwrap();
}

#[test]
fn duplicates_on_one_line_and_in_unassigned_collapse_to_one() {
    let mut project = project();
    project.devices.insert(device(3));
    let home = &mut project.installations[0];
    home.topology.lines[0].devices.push(DeviceId(1));
    // Device 3 sits between the duplicates: undo must restore the exact order.
    home.topology.unassigned = vec![DeviceId(1), DeviceId(3), DeviceId(1)];
    let repair = Command::RepairDevicePlacement {
        device: DeviceId(1),
        keep: DevicePlacementSlot::Line(LineId(1)),
    };
    round_trip(&mut project, repair.clone());
    assert_eq!(
        project.installations[0].topology.lines[0].devices,
        [DeviceId(1)]
    );
    assert_eq!(project.installations[0].topology.unassigned, [DeviceId(3)]);
}

#[test]
fn twice_unassigned_can_keep_the_unassigned_slot() {
    let mut project = project();
    project.installations[1].topology.lines[0].devices.clear();
    project.installations[1].topology.unassigned = vec![DeviceId(2), DeviceId(1), DeviceId(2)];
    let repair = Command::RepairDevicePlacement {
        device: DeviceId(2),
        keep: DevicePlacementSlot::Unassigned(InstallationId(1)),
    };
    round_trip(&mut project, repair.clone());
    assert_eq!(
        project.installations[1].topology.unassigned,
        [DeviceId(2), DeviceId(1)],
        "the first occurrence stays, other devices keep their order"
    );
}

#[test]
fn a_placement_spread_over_two_installations_is_repairable() {
    let mut project = project();
    project.installations[1]
        .topology
        .unassigned
        .push(DeviceId(1));
    let repair = Command::RepairDevicePlacement {
        device: DeviceId(1),
        keep: DevicePlacementSlot::Unassigned(InstallationId(1)),
    };
    round_trip(&mut project, repair.clone());
    assert!(project.installations[0].topology.lines[0]
        .devices
        .is_empty());
    assert_eq!(project.installations[1].topology.unassigned, [DeviceId(1)]);
}

#[test]
fn device_repairs_refuse_guessing() {
    let mut project = project();
    // Not ambiguous: a repair is not a move.
    assert_eq!(
        refused(
            &mut project,
            Command::RepairDevicePlacement {
                device: DeviceId(1),
                keep: DevicePlacementSlot::Line(LineId(1)),
            },
        ),
        CommandError::RepairNotNeeded
    );
    project.installations[0]
        .topology
        .unassigned
        .push(DeviceId(1));
    // The kept slot must be one the device occupies now.
    assert_eq!(
        refused(
            &mut project,
            Command::RepairDevicePlacement {
                device: DeviceId(1),
                keep: DevicePlacementSlot::Line(LineId(2)),
            },
        ),
        CommandError::RepairKeepNotPresent
    );
    assert_eq!(
        refused(
            &mut project,
            Command::RepairDevicePlacement {
                device: DeviceId(1),
                keep: DevicePlacementSlot::Unassigned(InstallationId(1)),
            },
        ),
        CommandError::RepairKeepNotPresent
    );
    assert_eq!(
        refused(
            &mut project,
            Command::RepairDevicePlacement {
                device: DeviceId(9),
                keep: DevicePlacementSlot::Line(LineId(1)),
            },
        ),
        CommandError::DeviceNotFound(DeviceId(9))
    );
    assert_eq!(
        refused(
            &mut project,
            Command::RepairDevicePlacement {
                device: DeviceId(1),
                keep: DevicePlacementSlot::Unassigned(InstallationId(7)),
            },
        ),
        CommandError::InstallationNotFound
    );
}

#[test]
fn a_line_under_two_areas_keeps_only_the_chosen_area() {
    let mut project = project();
    project.installations[0]
        .topology
        .areas
        .push(area(3, 3, vec![LineId(1)]));
    assert_eq!(
        Command::MoveLineToArea {
            id: LineId(1),
            area: AreaId(1),
        }
        .apply(&mut project.clone()),
        Err(CommandError::LinePlacementAmbiguous(LineId(1))),
        "the ambiguity blocks ordinary editing"
    );
    let repair = Command::RepairLineOwner {
        line: LineId(1),
        keep: AreaId(3),
    };
    round_trip(&mut project, repair.clone());
    assert!(project.installations[0].topology.areas[0].lines.is_empty());
    assert_eq!(
        project.installations[0].topology.areas[1].lines,
        [LineId(1)]
    );
    Command::MoveLineToArea {
        id: LineId(1),
        area: AreaId(1),
    }
    .apply(&mut project)
    .unwrap();
}

#[test]
fn a_line_listed_twice_and_from_another_installation_collapses() {
    let mut project = project();
    project.installations[0].topology.areas[0]
        .lines
        .push(LineId(1));
    project.installations[1].topology.areas[0]
        .lines
        .insert(1, LineId(1));
    let repair = Command::RepairLineOwner {
        line: LineId(1),
        keep: AreaId(1),
    };
    round_trip(&mut project, repair.clone());
    assert_eq!(
        project.installations[0].topology.areas[0].lines,
        [LineId(1)]
    );
    assert_eq!(
        project.installations[1].topology.areas[0].lines,
        [LineId(2), LineId(3)]
    );
}

#[test]
fn line_repairs_refuse_guessing_and_crossing() {
    let mut project = project();
    assert_eq!(
        refused(
            &mut project,
            Command::RepairLineOwner {
                line: LineId(1),
                keep: AreaId(1),
            },
        ),
        CommandError::RepairNotNeeded
    );
    project.installations[1].topology.areas[0]
        .lines
        .push(LineId(1));
    // Area 2 lists the line but lives in the other installation.
    assert_eq!(
        refused(
            &mut project,
            Command::RepairLineOwner {
                line: LineId(1),
                keep: AreaId(2),
            },
        ),
        CommandError::CrossInstallation {
            from: InstallationId(0),
            to: InstallationId(1),
        }
    );
    project.installations[0]
        .topology
        .areas
        .push(area(3, 3, vec![]));
    assert_eq!(
        refused(
            &mut project,
            Command::RepairLineOwner {
                line: LineId(1),
                keep: AreaId(3),
            },
        ),
        CommandError::RepairKeepNotPresent
    );
    assert_eq!(
        refused(
            &mut project,
            Command::RepairLineOwner {
                line: LineId(9),
                keep: AreaId(1),
            },
        ),
        CommandError::LineNotFound(LineId(9))
    );
}
