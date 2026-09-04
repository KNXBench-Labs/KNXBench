//! `set_individual_address_impl`/`set_com_object_dpt_impl`/`undo_impl`/
//! `redo_impl` without any Tauri machinery — see `open_reference_project.rs`
//! for the same no-Tauri pattern.

use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CommissioningState, DeviceId, DeviceInstance, DptRef,
    IndividualAddress, Language, Layer, Override, Project, Resolved, ResolvedFlags, SourceRef,
    Text,
};
use knx_desktop_lib::AppState;

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

#[test]
fn setting_individual_address_then_undo_then_redo_round_trips() {
    let state = state_with_two_devices();

    let tree =
        knx_desktop_lib::set_individual_address_impl(&state, 1, Some("1.1.2".into())).unwrap();
    assert!(tree.can_undo);
    assert!(!tree.can_redo);
    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address.as_deref(), Some("1.1.2"));
    drop(project);

    let tree = knx_desktop_lib::undo_impl(&state).unwrap();
    assert!(!tree.can_undo);
    assert!(tree.can_redo);
    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address, None);
    drop(project);

    let tree = knx_desktop_lib::redo_impl(&state).unwrap();
    assert!(tree.can_undo);
    assert!(!tree.can_redo);
    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address.as_deref(), Some("1.1.2"));
}

#[test]
fn a_duplicate_individual_address_is_rejected_and_leaves_the_stack_untouched() {
    let state = state_with_two_devices();
    let err =
        knx_desktop_lib::set_individual_address_impl(&state, 1, Some("1.1.1".into())).unwrap_err();
    assert!(err.contains("already used by device 2"), "{err}");

    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    assert_eq!(detail.address, None); // unchanged
    drop(project);

    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[test]
fn a_malformed_address_string_is_rejected_before_touching_the_project() {
    let state = state_with_two_devices();
    let err =
        knx_desktop_lib::set_individual_address_impl(&state, 1, Some("not-an-address".into()))
            .unwrap_err();
    assert!(err.contains("malformed individual address"), "{err}");
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[test]
fn setting_com_object_dpt_marks_it_user_edit_and_undo_restores_the_program_layer() {
    let state = state_with_two_devices();
    knx_desktop_lib::set_com_object_dpt_impl(&state, 1, Some("DPST-5-1".into())).unwrap();

    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert_eq!(com.dpt.as_deref(), Some("DPST-5-1"));
    assert_eq!(com.dpt_layer.as_deref(), Some("UserEdit"));
    drop(project);

    knx_desktop_lib::undo_impl(&state).unwrap();
    let project = state.project.lock().unwrap();
    let detail = knx_desktop_lib::device_detail_impl(project.as_ref().unwrap(), 1).unwrap();
    let com = &detail.com_objects[0];
    assert_eq!(com.dpt.as_deref(), Some("DPST-1-1"));
    assert_eq!(com.dpt_layer.as_deref(), Some("Program"));
}

#[test]
fn undo_with_nothing_to_undo_is_an_error() {
    let state = state_with_two_devices();
    let err = knx_desktop_lib::undo_impl(&state).unwrap_err();
    assert!(err.contains("nothing to undo"), "{err}");
}
