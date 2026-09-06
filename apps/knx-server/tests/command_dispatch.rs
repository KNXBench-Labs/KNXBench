//! `set_individual_address_impl`/`set_com_object_dpt_impl`/`undo_impl`/
//! `redo_impl` without any Tauri machinery — see `open_reference_project.rs`
//! for the same no-Tauri pattern.

use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId,
    DeviceInstance, DptRef, GroupAddressId, IndividualAddress, Installation, InstallationId,
    Language, Layer, Override, Project, Resolved, ResolvedFlags, SourceRef, Text, Topology,
};
use knx_server::AppState;

fn source() -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    }
}

fn state_with_two_devices() -> AppState {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source(),
        name: "D1".into(),
        description: None,
        address: None,
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
        text: Override::Value(Resolved {
            value: Text::Literal("Obj".into()),
            layer: Layer::Program,
        }),
        description: Override::Absent,
        dpt: Override::Value(Resolved {
            value: DptRef {
                main: 1,
                sub: Some(1),
            },
            layer: Layer::Program,
        }),
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(2),
        source: source(),
        name: "D2".into(),
        description: None,
        address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });

    let state = AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

fn state_with_one_installation() -> AppState {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
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
    });
    let state = AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

#[test]
fn creating_a_group_address_then_deleting_it_round_trips_through_undo() {
    let state = state_with_one_installation();

    let tree =
        knx_server::create_group_address_impl(&state, "Living room light".into(), "1/1/1".into())
            .unwrap();
    assert!(tree.can_undo);
    assert_eq!(tree.installations[0].group_addresses.len(), 1);
    let ga = &tree.installations[0].group_addresses[0];
    assert_eq!(ga.name, "Living room light");
    assert_eq!(ga.address, "1/1/1");
    let id = ga.id;

    let tree = knx_server::delete_group_address_impl(&state, id).unwrap();
    assert!(tree.installations[0].group_addresses.is_empty());

    let tree = knx_server::undo_impl(&state).unwrap(); // undoes the delete
    assert_eq!(tree.installations[0].group_addresses.len(), 1);
    let tree = knx_server::undo_impl(&state).unwrap(); // undoes the create
    assert!(tree.installations[0].group_addresses.is_empty());
}

#[test]
fn creating_a_group_address_with_a_malformed_address_is_rejected() {
    let state = state_with_one_installation();
    let err = knx_server::create_group_address_impl(&state, "GA".into(), "not-an-address".into())
        .unwrap_err();
    assert!(err.contains("malformed group address"), "{err}");
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[test]
fn creating_a_duplicate_group_address_is_rejected() {
    let state = state_with_one_installation();
    knx_server::create_group_address_impl(&state, "First".into(), "1/1/1".into()).unwrap();
    let err =
        knx_server::create_group_address_impl(&state, "Second".into(), "1/1/1".into()).unwrap_err();
    assert!(err.contains("already used"), "{err}");
    let project = state.project.lock().unwrap();
    assert_eq!(
        project.as_ref().unwrap().installations[0]
            .group_addresses
            .len(),
        1
    );
}

#[test]
fn deleting_a_group_address_still_linked_from_a_com_object_is_rejected() {
    let state = state_with_one_installation();
    let tree = knx_server::create_group_address_impl(&state, "GA".into(), "1/1/1".into()).unwrap();
    let ga_id = tree.installations[0].group_addresses[0].id;

    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        project.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "D".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![ComObjectInstanceId(1)],
            binary_data: vec![],
        });
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![knx_core::GroupLink {
                ga: GroupAddressId(ga_id),
                direction: knx_core::Direction::Send,
            }],
        });
    }

    let err = knx_server::delete_group_address_impl(&state, ga_id).unwrap_err();
    assert!(err.contains("still linked"), "{err}");
}

#[test]
fn setting_individual_address_then_undo_then_redo_round_trips() {
    let state = state_with_two_devices();

    let tree = knx_server::set_individual_address_impl(&state, 1, Some("1.1.2".into())).unwrap();
    assert!(tree.can_undo);
    assert!(!tree.can_redo);
    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address.as_deref(), Some("1.1.2"));
    drop(project);

    let tree = knx_server::undo_impl(&state).unwrap();
    assert!(!tree.can_undo);
    assert!(tree.can_redo);
    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address, None);
    drop(project);

    let tree = knx_server::redo_impl(&state).unwrap();
    assert!(tree.can_undo);
    assert!(!tree.can_redo);
    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address.as_deref(), Some("1.1.2"));
}

#[test]
fn a_duplicate_individual_address_is_rejected_and_leaves_the_stack_untouched() {
    let state = state_with_two_devices();
    let err = knx_server::set_individual_address_impl(&state, 1, Some("1.1.1".into())).unwrap_err();
    assert!(err.contains("already used by device 2"), "{err}");

    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address, None); // unchanged
    drop(project);

    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[test]
fn a_malformed_address_string_is_rejected_before_touching_the_project() {
    let state = state_with_two_devices();
    let err = knx_server::set_individual_address_impl(&state, 1, Some("not-an-address".into()))
        .unwrap_err();
    assert!(err.contains("malformed individual address"), "{err}");
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[test]
fn setting_device_description_then_undo_then_redo_round_trips() {
    let state = state_with_two_devices();

    let tree =
        knx_server::set_device_description_impl(&state, 1, Some("Flur, links".into())).unwrap();
    assert!(tree.can_undo);
    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.description.as_deref(), Some("Flur, links"));
    drop(project);

    knx_server::undo_impl(&state).unwrap();
    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.description, None);
    drop(project);

    knx_server::redo_impl(&state).unwrap();
    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.description.as_deref(), Some("Flur, links"));
}

#[test]
fn setting_com_object_description_marks_it_user_edit_and_undo_restores_the_program_layer() {
    let state = state_with_two_devices();
    knx_server::set_com_object_description_impl(&state, 1, Some("Aktoreingang 1".into())).unwrap();

    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert_eq!(com.description.as_deref(), Some("Aktoreingang 1"));
    assert_eq!(com.description_layer.as_deref(), Some("UserEdit"));
    drop(project);

    knx_server::undo_impl(&state).unwrap();
    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert_eq!(com.description, None); // fixture's com object had no description at all
}

#[test]
fn setting_com_object_dpt_marks_it_user_edit_and_undo_restores_the_program_layer() {
    let state = state_with_two_devices();
    knx_server::set_com_object_dpt_impl(&state, 1, Some("DPST-5-1".into())).unwrap();

    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert_eq!(com.dpt.as_deref(), Some("DPST-5-1"));
    assert_eq!(com.dpt_layer.as_deref(), Some("UserEdit"));
    drop(project);

    knx_server::undo_impl(&state).unwrap();
    let project = state.project.lock().unwrap();
    let detail = knx_server::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert_eq!(com.dpt.as_deref(), Some("DPST-1-1"));
    assert_eq!(com.dpt_layer.as_deref(), Some("Program"));
}

#[test]
fn undo_with_nothing_to_undo_is_an_error() {
    let state = state_with_two_devices();
    let err = knx_server::undo_impl(&state).unwrap_err();
    assert!(err.contains("nothing to undo"), "{err}");
}
