//! Pins the service-owned note target catalogue against the domain's own target check.

use knx_app::note_targets::{note_target_catalogue, NoteTargetOption};
use knx_core::note::ProjectNoteTarget as Target;
use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
    ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId, DeviceInstance,
    GroupAddress, GroupAddressEntry, GroupAddressId, GroupRange, GroupRangeId, Installation,
    InstallationId, Language, Layer, Line, LineId, Override, Project, Resolved, ResolvedFlags,
    SourceRef, Text, Topology,
};

fn source() -> SourceRef {
    SourceRef {
        path: "synthetic".into(),
        ets_id: "synthetic".into(),
    }
}

fn device(id: u32, name: &str, objects: &[u32]) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: source(),
        name: name.into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "A".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: objects.iter().copied().map(ComObjectInstanceId).collect(),
        binary_data: vec![],
    }
}

fn object(id: u32, device: u32, number: u16, name: Option<&str>) -> ComObjectInstance {
    ComObjectInstance {
        id: ComObjectInstanceId(id),
        source: source(),
        device: DeviceId(device),
        number,
        text: name.map_or(Override::Absent, |name| {
            Override::Value(Resolved {
                value: Text::Literal(name.into()),
                layer: Layer::Instance,
            })
        }),
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
    }
}

fn installation(id: u8, area: u32, devices: &[u32]) -> Installation {
    Installation {
        id: InstallationId(id),
        name: format!("install {id}"),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![Area {
                id: AreaId(area),
                source: source(),
                name: "area".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![LineId(area)],
            }],
            lines: vec![Line {
                id: LineId(area),
                source: source(),
                name: "line".into(),
                address: 2,
                medium_ref: "MT-0".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::FinishedDesign,
                devices: devices.iter().copied().map(DeviceId).collect(),
            }],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    }
}

/// One of everything, with the device placed both on a line and in a room.
fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project
        .devices
        .insert(device(1, "Thermostat", &[11, 10, 99]));
    project
        .devices
        .insert_com_object(object(10, 1, 0, Some("Temperature")));
    project.devices.insert_com_object(object(11, 1, 3, None));
    let mut install = installation(0, 1, &[1]);
    install.buildings.push(BuildingPart {
        id: BuildingPartId(1),
        source: source(),
        name: "Kitchen".into(),
        number: None,
        kind: BuildingPartType::Room,
        default_line: None,
        completion: CompletionStatus::FinishedDesign,
        children: vec![],
        devices: vec![DeviceId(1)],
        parent: None,
    });
    install.group_ranges.push(GroupRange {
        id: GroupRangeId(1),
        source: source(),
        name: "Lights".into(),
        start: GroupAddress::from_raw(0),
        end: GroupAddress::from_raw(255),
        parent: None,
        children: vec![],
    });
    install.group_addresses.push(GroupAddressEntry {
        id: GroupAddressId(1),
        source: source(),
        name: "Hall".into(),
        address: GroupAddress::from_raw(1),
        central: false,
        unfiltered: false,
        range: Some(GroupRangeId(1)),
        declared_dpt: Override::Absent,
    });
    project.installations.push(install);
    project
}

fn option(target: Target, owner: &[&str], parts: &[&str]) -> NoteTargetOption {
    NoteTargetOption {
        target,
        owner: owner.iter().map(|part| part.to_string()).collect(),
        parts: parts.iter().map(|part| part.to_string()).collect(),
    }
}

#[test]
fn offers_every_projected_kind_once_including_communication_objects() {
    let project = project();
    let before = project.clone();
    let catalogue = note_target_catalogue(&project);
    assert_eq!(
        project, before,
        "reading the catalogue must not touch the project"
    );
    assert_eq!(
        catalogue,
        vec![
            option(Target::Installation(InstallationId(0)), &[], &["install 0"]),
            option(Target::Area(AreaId(1)), &[], &["1", "area"]),
            option(Target::Line(LineId(1)), &[], &["1.2", "line"]),
            option(Target::BuildingPart(BuildingPartId(1)), &[], &["Kitchen"]),
            // On a line and in a room, offered once.
            option(Target::Device(DeviceId(1)), &[], &["Thermostat"]),
            // In the device's own object order; the dangling id 99 is not offered.
            option(
                Target::ComObject(ComObjectInstanceId(11)),
                &["Thermostat"],
                &["3"]
            ),
            option(
                Target::ComObject(ComObjectInstanceId(10)),
                &["Thermostat"],
                &["0", "Temperature"],
            ),
            option(
                Target::GroupRange(GroupRangeId(1)),
                &[],
                &["0/0/0–0/0/255", "Lights"]
            ),
            option(
                Target::GroupAddress(GroupAddressId(1)),
                &[],
                &["0/0/1", "Hall"]
            ),
        ]
    );
}

#[test]
fn offers_only_what_the_domain_would_accept_as_a_note_target() {
    let mut project = project();
    // A second installation reusing area/line id 1 makes that id ambiguous.
    project.devices.insert(device(2, "Second", &[]));
    project.installations.push(installation(1, 1, &[2]));
    // So does a group address id reused in the second installation.
    let mut duplicate = project.installations[0].group_addresses[0].clone();
    duplicate.name = "Elsewhere".into();
    project.installations[1].group_addresses.push(duplicate);
    let catalogue = note_target_catalogue(&project);
    for entry in &catalogue {
        assert_eq!(
            entry.target.validate(&project),
            Ok(()),
            "{:?}",
            entry.target
        );
    }
    let offered = |target: Target| catalogue.iter().any(|entry| entry.target == target);
    assert!(!offered(Target::Area(AreaId(1))));
    assert!(!offered(Target::Line(LineId(1))));
    assert!(!offered(Target::GroupAddress(GroupAddressId(1))));
    assert!(offered(Target::GroupRange(GroupRangeId(1))));
    assert!(offered(Target::Installation(InstallationId(1))));
    assert!(offered(Target::Device(DeviceId(2))));
    assert!(
        !offered(Target::Project),
        "the editor owns the project choice"
    );
}

#[test]
fn an_empty_project_offers_nothing() {
    assert!(note_target_catalogue(&Project::new(Language("en".into()))).is_empty());
}

#[test]
fn devices_read_by_address_and_foreign_or_orphaned_entries_are_handled_honestly() {
    use knx_core::IndividualAddress;
    let mut project = project();
    let mut addressed = device(5, "Actuator", &[50, 10]);
    addressed.address = Some(IndividualAddress::new(1, 1, 2).unwrap());
    project.devices.insert(addressed);
    project.devices.insert(device(3, "Unaddressed", &[]));
    project
        .devices
        .insert_com_object(object(50, 5, 1, Some("Switch")));
    // A building part that no root reaches stays offered.
    project.installations[0].buildings.push(BuildingPart {
        id: BuildingPartId(7),
        source: source(),
        name: "Lost room".into(),
        number: None,
        kind: BuildingPartType::Room,
        default_line: None,
        completion: CompletionStatus::FinishedDesign,
        children: vec![],
        devices: vec![],
        parent: Some(BuildingPartId(99)),
    });
    let catalogue = note_target_catalogue(&project);
    let of_kind = |pick: fn(&Target) -> bool| {
        catalogue
            .iter()
            .filter(|entry| pick(&entry.target))
            .cloned()
            .collect::<Vec<_>>()
    };
    let devices = of_kind(|target| matches!(target, Target::Device(_)));
    assert_eq!(
        devices.iter().map(|entry| entry.target).collect::<Vec<_>>(),
        [5, 1, 3].map(|id| Target::Device(DeviceId(id))),
        "addressed devices first, then unaddressed ones by ID"
    );
    assert_eq!(devices[0].parts, ["1.1.2", "Actuator"]);
    let objects = of_kind(|target| matches!(target, Target::ComObject(_)));
    assert_eq!(
        objects.iter().map(|entry| entry.target).collect::<Vec<_>>(),
        [50, 11, 10].map(|id| Target::ComObject(ComObjectInstanceId(id))),
        "object 10 belongs to device 1; listing it under device 5 must not move it"
    );
    assert_eq!(objects[0].owner, ["1.1.2", "Actuator"]);
    assert_eq!(objects[0].parts, ["1", "Switch"]);
    let parts = of_kind(|target| matches!(target, Target::BuildingPart(_)));
    assert_eq!(parts.last().unwrap().parts, ["Lost room"]);
}

#[test]
fn deeply_nested_building_parts_are_walked_without_recursion_limits() {
    let mut project = project();
    let parts = &mut project.installations[0].buildings;
    parts.clear();
    for id in 1000..=5000u32 {
        parts.push(BuildingPart {
            id: BuildingPartId(id),
            source: source(),
            name: format!("Level {id}"),
            number: None,
            kind: BuildingPartType::Floor,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: if id < 5000 {
                vec![BuildingPartId(id + 1)]
            } else {
                vec![]
            },
            devices: vec![],
            parent: (id > 1000).then(|| BuildingPartId(id - 1)),
        });
    }
    let offered: Vec<_> = note_target_catalogue(&project)
        .into_iter()
        .filter_map(|entry| match entry.target {
            Target::BuildingPart(id) => Some(id.0),
            _ => None,
        })
        .collect();
    assert_eq!(offered, (1000..=5000).collect::<Vec<_>>());
}
