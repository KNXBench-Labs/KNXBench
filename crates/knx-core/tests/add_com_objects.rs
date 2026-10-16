//! Tests atomic reversible addition of newly configured communication objects.
use knx_core::*;
fn project() -> Project {
    let mut p = Project::new(Language("en".into()));
    p.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "device".into(),
            ets_id: "device".into(),
        },
        name: "Device".into(),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: false,
        com_objects: vec![],
        binary_data: vec![],
    });
    p
}
fn object(id: u32) -> ComObjectInstance {
    ComObjectInstance {
        id: ComObjectInstanceId(id),
        source: SourceRef {
            path: format!("ref-{id}"),
            ets_id: format!("ref-{id}"),
        },
        device: DeviceId(1),
        number: id as u16,
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
#[test]
fn additions_keep_order_and_roundtrip_through_the_exact_inverse() {
    let mut p = project();
    let before = p.clone();
    let cmd = Command::AddComObjects {
        device: DeviceId(1),
        objects: vec![object(2), object(3)],
        program_defaults: vec![],
    };
    let inverse = cmd.apply(&mut p).unwrap();
    let after = p.clone();
    assert_eq!(
        p.devices.get(DeviceId(1)).unwrap().com_objects,
        vec![ComObjectInstanceId(2), ComObjectInstanceId(3)]
    );
    let redo = inverse.apply(&mut p).unwrap();
    assert_eq!(p, before);
    redo.apply(&mut p).unwrap();
    assert_eq!(p, after);
}
#[test]
fn additions_and_removal_preserve_defaults_and_refuse_live_links() {
    let mut p = project();
    let defaults = ProgramDefaults {
        description: Some(Resolved {
            value: Text::Literal("catalogue".into()),
            layer: Layer::Program,
        }),
        ..Default::default()
    };
    let inverse = Command::AddComObjects {
        device: DeviceId(1),
        objects: vec![object(2)],
        program_defaults: vec![(ComObjectInstanceId(2), defaults.clone())],
    }
    .apply(&mut p)
    .unwrap();
    let unlinked = p.clone();
    p.devices
        .com_object_mut(ComObjectInstanceId(2))
        .unwrap()
        .links
        .push(GroupLink {
            ga: GroupAddressId(7),
            direction: Direction::Send,
        });
    let linked = p.clone();
    assert!(inverse.apply(&mut p).is_err());
    assert_eq!(p, linked);
    p = unlinked;
    let redo = inverse.apply(&mut p).unwrap();
    assert!(p.devices.program_defaults(ComObjectInstanceId(2)).is_none());
    redo.apply(&mut p).unwrap();
    assert_eq!(
        p.devices.program_defaults(ComObjectInstanceId(2)),
        Some(&defaults)
    );
}

#[test]
fn additions_refuse_initial_links_and_missing_or_foreign_modules_atomically() {
    for case in 0..3 {
        let mut p = project();
        let mut added = object(2);
        match case {
            0 => added.links.push(GroupLink {
                ga: GroupAddressId(7),
                direction: Direction::Send,
            }),
            1 => added.module_instance = Some(ModuleInstanceId(7)),
            _ => {
                let mut other = p.devices.get(DeviceId(1)).unwrap().clone();
                other.id = DeviceId(99);
                p.devices.insert(other);
                p.devices.insert_module_instance(ModuleInstance {
                    id: ModuleInstanceId(7),
                    device: DeviceId(99),
                    source: SourceRef {
                        path: "module".into(),
                        ets_id: "module".into(),
                    },
                    repeat_index: "1".into(),
                    instance_ets_id: "module_MI-1".into(),
                    arguments: vec![],
                });
                added.module_instance = Some(ModuleInstanceId(7));
            }
        }
        let before = p.clone();
        assert!(
            Command::AddComObjects {
                device: DeviceId(1),
                objects: vec![object(1), added],
                program_defaults: vec![]
            }
            .apply(&mut p)
            .is_err(),
            "case {case}"
        );
        assert_eq!(p, before, "case {case}");
    }
}

#[test]
fn duplicate_ids_and_wrong_owners_refuse_without_any_mutation() {
    for mut objects in [vec![object(2), object(2)], vec![object(2), object(3)]] {
        if objects[0].id != objects[1].id {
            objects[1].device = DeviceId(999);
        }
        let mut p = project();
        let before = p.clone();
        assert!(Command::AddComObjects {
            device: DeviceId(1),
            objects,
            program_defaults: vec![]
        }
        .apply(&mut p)
        .is_err());
        assert_eq!(p, before);
    }
}
