//! `device_detail_impl` without any Tauri machinery — see
//! `open_reference_project.rs` for the same pattern against a real import.

use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CommissioningState, DeviceId, DeviceInstance, DptRef,
    Language, Layer, Override, Project, Resolved, ResolvedFlags, SourceRef, Text,
};

fn source() -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    }
}

fn project_with_one_device() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source(),
        name: "Switch".into(),
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
    project
}

#[test]
fn device_detail_impl_returns_the_projection_for_a_known_device() {
    let project = project_with_one_device();
    let detail = knx_desktop_lib::device_detail_impl(&project, 1).unwrap();
    assert_eq!(detail.name, "Switch");
    assert_eq!(detail.com_objects.len(), 1);
}

#[test]
fn device_detail_impl_reports_an_unknown_device_by_id() {
    let project = project_with_one_device();
    let err = knx_desktop_lib::device_detail_impl(&project, 42).unwrap_err();
    assert!(err.contains("42"));
}
