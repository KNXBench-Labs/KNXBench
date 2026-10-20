//! Synthetic normalized projects and exact monitoring bundle checks.
use super::*;

pub(super) fn source() -> SourceRef {
    SourceRef {
        path: "synthetic".into(),
        ets_id: "local".into(),
    }
}
pub(super) fn value<T>(value: T) -> Override<T> {
    Override::Value(Resolved {
        value,
        layer: Layer::UserEdit,
    })
}
pub(super) fn fixture() -> Project {
    let mut p = Project::new(Language("en-US".into()));
    p.info.name = "Current project".into();
    p.info.project_id = "local-project".into();
    let mut i = Installation {
        id: InstallationId(0),
        name: "Installation".into(),
        default_line: Some(LineId(1)),
        multicast_address: None,
        completion: CompletionStatus::default(),
        topology: Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: source(),
                name: "Area".into(),
                address: 1,
                completion: CompletionStatus::default(),
                lines: vec![LineId(1)],
            }],
            lines: vec![Line {
                id: LineId(1),
                source: source(),
                name: "Line".into(),
                address: 1,
                medium_ref: "MT-5".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::default(),
                devices: vec![DeviceId(1)],
            }],
            unassigned: vec![],
        },
        buildings: vec![BuildingPart {
            id: BuildingPartId(1),
            source: source(),
            name: "Room".into(),
            number: Some("7".into()),
            kind: BuildingPartType::Room,
            default_line: Some(LineId(1)),
            completion: CompletionStatus::default(),
            children: vec![],
            devices: vec![DeviceId(1)],
            parent: None,
        }],
        group_ranges: vec![GroupRange {
            id: GroupRangeId(1),
            source: source(),
            name: "Range".into(),
            start: GroupAddress::from_raw(2048),
            end: GroupAddress::from_raw(2303),
            parent: None,
            children: vec![],
        }],
        group_addresses: vec![],
        parameters: vec![],
    };
    i.group_addresses.push(GroupAddressEntry {
        id: GroupAddressId(1),
        source: source(),
        name: "Current GA".into(),
        address: GroupAddress::from_raw(2051),
        central: false,
        unfiltered: false,
        range: Some(GroupRangeId(1)),
        declared_dpt: value(DptRef {
            main: 1,
            sub: Some(2),
        }),
    });
    p.installations.push(i);
    p.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source(),
        name: "Current device".into(),
        description: Some("Authored".into()),
        address: Some(IndividualAddress::new(1, 1, 9).unwrap()),
        product_ref: "product".into(),
        program_ref: "program".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: false,
        com_objects: vec![ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    p.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: source(),
        device: DeviceId(1),
        number: 42,
        text: value(Text::Literal("Current {name}".into())),
        description: Override::Empty,
        dpt: value(DptRef {
            main: 1,
            sub: Some(1),
        }),
        flags: ResolvedFlags {
            read: value(false),
            write: value(true),
            ..ResolvedFlags::none()
        },
        size: Some(Resolved {
            value: ObjectSize::Byte(2),
            layer: Layer::Program,
        }),
        is_active: false,
        links: vec![GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        }],
        module_instance: None,
    });
    p.functions.push(Function {
        id: FunctionId(1),
        source: source(),
        location: BuildingPartId(1),
        details: FunctionDetails {
            name: "Light".into(),
            ..Default::default()
        },
        members: vec![FunctionMember {
            id: FunctionMemberId(1),
            source: source(),
            group_address: GroupAddressId(1),
            name: "Switch".into(),
            role: None,
        }],
    });
    p.ids = IdAllocators::from_counts(1, 1, 1, 1, 1, 1, 1, 0, 0).with_function_counts(1, 1);
    p
}
pub(super) fn unpack(bytes: &[u8]) -> (Value, ExportReport, String) {
    let mut z = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert_eq!(z.len(), 3);
    let mut contents = Vec::new();
    for (index, name) in ["project.json", "export-report.json", "README.txt"]
        .iter()
        .enumerate()
    {
        let mut f = z.by_index(index).unwrap();
        assert_eq!(f.name(), *name);
        let mut s = String::new();
        f.read_to_string(&mut s).unwrap();
        contents.push(s);
    }
    assert!(contents[0].ends_with('\n'));
    let mut canonical =
        serde_json::to_string_pretty(&serde_json::from_str::<Value>(&contents[0]).unwrap())
            .unwrap();
    canonical.push('\n');
    assert_eq!(contents[0], canonical);
    use sha2::{Digest, Sha256};
    let report: ExportReport = serde_json::from_str(&contents[1]).unwrap();
    assert_eq!(
        report.project_sha256,
        format!("{:x}", Sha256::digest(contents[0].as_bytes()))
    );
    (
        serde_json::from_str(&contents[0]).unwrap(),
        report,
        contents.remove(2),
    )
}
pub(super) fn document(p: &Project) -> (Value, ExportReport) {
    let b = build(p, None, &ExportOptions::default()).expect("valid current snapshot must export");
    let (v, r, _) = unpack(&b.bytes);
    assert_eq!(r, b.report);
    (v, r)
}
pub(super) fn refuses(p: &Project, needle: &str) {
    let e = match build(p, None, &ExportOptions::default()) {
        Err(e) => e,
        Ok(_) => panic!("expected whole-export refusal"),
    };
    assert!(e.contains(needle), "expected {needle}, got {e}");
}
pub(super) fn has_issue(r: &ExportReport, entity: &str, field: &str) -> bool {
    r.issues
        .iter()
        .any(|i| i.entity == entity && i.field == field)
}
