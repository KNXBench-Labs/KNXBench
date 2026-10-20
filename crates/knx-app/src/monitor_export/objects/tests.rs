//! Object dictionary insertion refuses collisions even without graph admission.
use super::*;
use std::collections::BTreeMap;

#[test]
fn object_dictionary_collision_is_refused_without_admission() {
    let language = Language("en-US".into());
    let mut p = Project::new(language.clone());
    let source = SourceRef {
        path: "synthetic".into(),
        ets_id: "local".into(),
    };
    p.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source.clone(),
        name: "Device".into(),
        description: None,
        address: Some(IndividualAddress::new(1, 1, 9).unwrap()),
        product_ref: "product".into(),
        program_ref: "program".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: false,
        com_objects: vec![ComObjectInstanceId(1), ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    p.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source,
        device: DeviceId(1),
        number: 1,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
    });
    let installation = Installation {
        id: InstallationId(0),
        name: "Installation".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::default(),
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    };
    // Bypass select deliberately to exercise the independent insertion defense.
    let selection = Selection {
        installation: &installation,
        devices: BTreeMap::from([(DeviceId(1), p.devices.get(DeviceId(1)).unwrap())]),
        addresses: BTreeMap::new(),
        spaces: BTreeMap::new(),
        ranges: BTreeMap::new(),
        functions: vec![],
    };
    let before = p.clone();
    let result = project(
        &p,
        &selection,
        None,
        &language,
        &mut Context::default(),
        &mut ExportCounts::default(),
    );
    assert_eq!(p, before);
    assert!(
        result.is_err(),
        "object dictionary insertion must refuse a duplicate key"
    );
    assert!(result
        .unwrap_err()
        .contains("duplicate object dictionary key"));
}
