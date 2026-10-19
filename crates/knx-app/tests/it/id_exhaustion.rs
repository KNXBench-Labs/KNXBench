//! ID exhaustion across checked allocation, CSV commands and native storage.
//!
//! All fixtures are synthetic and all operations are offline.

use knx_core::{
    Command, CommandStack, CompletionStatus, GroupAddress, GroupAddressEntry, GroupAddressId,
    GroupAddressStyle, IdAllocationError, IdAllocators, Installation, InstallationId, Language,
    Project, SourceRef, Topology,
};
use knx_csv::{parse_group_addresses, plan_import, Severity};
use knx_store::{load_project, open_and_migrate_in_memory, save_project};

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.group_address_style = GroupAddressStyle::Free;
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Synthetic installation".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Editing,
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

fn all_counters(value: u32) -> IdAllocators {
    IdAllocators::from_counts(
        value, value, value, value, value, value, value, value, value,
    )
}

type Allocate = fn(&mut IdAllocators) -> Result<u32, IdAllocationError>;

const ALLOCATORS: [(&str, Allocate); 9] = [
    ("device", |ids| ids.next_device_id().map(|id| id.0)),
    ("area", |ids| ids.next_area_id().map(|id| id.0)),
    ("line", |ids| ids.next_line_id().map(|id| id.0)),
    ("com_object_instance", |ids| {
        ids.next_com_object_instance_id().map(|id| id.0)
    }),
    ("group_range", |ids| {
        ids.next_group_range_id().map(|id| id.0)
    }),
    ("group_address", |ids| {
        ids.next_group_address_id().map(|id| id.0)
    }),
    ("building_part", |ids| {
        ids.next_building_part_id().map(|id| id.0)
    }),
    ("parameter_instance", |ids| {
        ids.next_parameter_instance_id().map(|id| id.0)
    }),
    ("module_instance", |ids| {
        ids.next_module_instance_id().map(|id| id.0)
    }),
];

fn populate_final_entities(project: &mut Project) {
    use knx_core::{
        Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
        ComObjectInstanceId, DeviceId, DeviceInstance, GroupRange, GroupRangeId, Line, LineId,
        ModuleInstance, ModuleInstanceId, Override, ParameterInstance, ParameterInstanceId,
    };
    let n = u32::MAX;
    let source = |tag: &str| SourceRef {
        path: "synthetic".into(),
        ets_id: tag.into(),
    };
    let installation = &mut project.installations[0];
    installation.topology.areas.push(Area {
        id: AreaId(n),
        source: source("area"),
        name: "Area".into(),
        address: 1,
        completion: CompletionStatus::Editing,
        lines: vec![LineId(n)],
    });
    installation.topology.lines.push(Line {
        id: LineId(n),
        source: source("line"),
        name: "Line".into(),
        address: 1,
        medium_ref: "MT-0".into(),
        domain_address: None,
        domain_address_is_checked: None,
        ip_routing_multicast_address: None,
        multicast_ttl: None,
        completion: CompletionStatus::Editing,
        devices: vec![DeviceId(n)],
    });
    installation.group_ranges.push(GroupRange {
        id: GroupRangeId(n),
        source: source("range"),
        name: "Range".into(),
        start: GroupAddress::from_raw(100),
        end: GroupAddress::from_raw(200),
        parent: None,
        children: vec![],
    });
    installation.group_addresses.push(GroupAddressEntry {
        id: GroupAddressId(n),
        source: source("address"),
        name: "Address".into(),
        address: GroupAddress::from_raw(100),
        central: false,
        unfiltered: false,
        range: Some(GroupRangeId(n)),
        declared_dpt: Default::default(),
    });
    installation.buildings.push(BuildingPart {
        id: BuildingPartId(n),
        source: source("building"),
        name: "Building".into(),
        number: None,
        kind: BuildingPartType::Building,
        default_line: Some(LineId(n)),
        completion: CompletionStatus::Editing,
        children: vec![],
        devices: vec![DeviceId(n)],
        parent: None,
    });
    installation.parameters.push(ParameterInstance {
        id: ParameterInstanceId(n),
        device: DeviceId(n),
        source: source("parameter"),
        raw: "retained".into(),
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(n),
        source: source("device"),
        name: "Device".into(),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![ComObjectInstanceId(n)],
        binary_data: vec![],
    });
    project.devices.insert_module_instance(ModuleInstance {
        id: ModuleInstanceId(n),
        device: DeviceId(n),
        source: source("module"),
        repeat_index: "opaque".into(),
        instance_ets_id: "synthetic-module-instance".into(),
        arguments: vec![],
    });
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(n),
        source: source("object"),
        device: DeviceId(n),
        number: 1,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: Default::default(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: Some(ModuleInstanceId(n)),
    });
}

#[test]
fn all_nine_final_ids_and_exhausted_counters_survive_native_reopen() {
    let mut project = project();
    project.ids = all_counters(u32::MAX - 1);
    for (kind, allocate) in ALLOCATORS {
        assert_eq!(allocate(&mut project.ids), Ok(u32::MAX), "{kind}");
    }
    populate_final_entities(&mut project);
    let conn = open_and_migrate_in_memory().unwrap();
    save_project(&conn, &project).unwrap();
    let mut reopened = load_project(&conn).unwrap();
    assert_eq!(reopened, project);
    for (kind, allocate) in ALLOCATORS {
        assert_eq!(allocate(&mut reopened.ids), Err(IdAllocationError { kind }));
        assert_eq!(reopened, project, "refusal after reopen changed {kind}");
    }
}

#[test]
fn a_csv_batch_that_needs_one_id_too_many_is_refused_without_consuming_any() {
    let mut project = project();
    project.ids = IdAllocators::from_counts(0, 0, 0, 0, 0, u32::MAX - 1, 0, 0, 0);
    let before = project.clone();
    let parsed = parse_group_addresses(
        "Address,Name\n100,First\n101,Second\n",
        GroupAddressStyle::Free,
    );
    let plan = plan_import(&project, &parsed);
    assert!(
        plan.command.is_none(),
        "a partial batch must never be applicable"
    );
    assert!(plan.report.problems.iter().any(|problem| {
        problem.row == Some(3)
            && problem.severity == Severity::Error
            && problem.detail == "project group_address ID range exhausted"
    }));
    assert_eq!(project, before);
}

#[test]
fn exhaustion_does_not_prevent_an_update_that_needs_no_new_id() {
    let mut project = project();
    project.ids = all_counters(u32::MAX);
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id: GroupAddressId(1),
            source: SourceRef {
                path: "synthetic".into(),
                ets_id: "synthetic".into(),
            },
            name: "Old".into(),
            address: GroupAddress::from_raw(100),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        });
    let parsed = parse_group_addresses("Address,Name\n100,Updated\n", GroupAddressStyle::Free);
    let plan = plan_import(&project, &parsed);
    let mut stack = CommandStack::new();
    stack
        .do_command(&mut project, plan.command.unwrap())
        .unwrap();
    assert_eq!(project.installations[0].group_addresses[0].name, "Updated");
    assert_eq!(project.ids, all_counters(u32::MAX));
    stack.undo(&mut project).unwrap();
    assert_eq!(project.installations[0].group_addresses[0].name, "Old");
    assert_eq!(project.ids, all_counters(u32::MAX));
}

#[test]
fn the_final_csv_id_is_undoable_redoable_and_never_reissued_after_reopen() {
    let mut project = project();
    project.ids = IdAllocators::from_counts(0, 0, 0, 0, 0, u32::MAX - 1, 0, 0, 0);
    let parsed = parse_group_addresses("Address,Name\n100,Final\n", GroupAddressStyle::Free);
    let command = plan_import(&project, &parsed).command.unwrap();
    let mut stack = CommandStack::new();
    stack.do_command(&mut project, command).unwrap();
    assert_eq!(
        project.installations[0].group_addresses[0].id,
        GroupAddressId(u32::MAX)
    );
    let after = project.clone();
    stack.undo(&mut project).unwrap();
    assert!(project.installations[0].group_addresses.is_empty());
    assert_eq!(project.ids.peek_group_address(), u32::MAX);
    assert!(project.ids.next_group_address_id().is_err());
    let conn = open_and_migrate_in_memory().unwrap();
    save_project(&conn, &project).unwrap();
    let mut reopened = load_project(&conn).unwrap();
    assert_eq!(reopened, project);
    assert!(reopened.ids.next_group_address_id().is_err());
    stack.redo(&mut project).unwrap();
    assert_eq!(project, after);
    save_project(&conn, &project).unwrap();
    assert_eq!(load_project(&conn).unwrap(), after);
}

#[test]
fn a_rejected_command_batch_restores_reservation_at_the_maximum() {
    let mut project = project();
    let before = project.clone();
    let mut stack = CommandStack::new();
    let result = stack.do_command(
        &mut project,
        Command::Batch(vec![
            Command::ReserveIds {
                through: all_counters(u32::MAX),
            },
            Command::DeleteGroupAddress {
                id: GroupAddressId(99),
            },
        ]),
    );
    assert!(result.is_err());
    assert_eq!(project, before);
    assert!(!stack.can_undo());
    assert!(!stack.can_redo());
}
