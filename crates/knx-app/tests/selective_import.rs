//! Exercises selective merge identity, dependency preservation and honest refusal.

use knx_app::selective_import::{self as import, Selection, Source};
use knx_core::*;
use knx_store::project_history::NativeSnapshot;

fn source() -> Source {
    import::source_from_bytes(knx_testsupport::minimal_knxproj_bytes(), None).unwrap()
}

fn fixture() -> (Source, NativeSnapshot, Selection) {
    let source = source();
    let mut project = source.project.clone();
    project.info.name = "Destination identity".into();
    project.devices = Devices::new();
    for i in &mut project.installations {
        i.parameters.clear();
        i.buildings.clear();
        i.group_addresses.clear();
        i.group_ranges.clear();
        for l in &mut i.topology.lines {
            l.devices.clear();
        }
        i.topology.unassigned.clear();
    }
    let installation = source.project.installations[0].id.0;
    let device = source.project.devices.iter().next().unwrap().id.0;
    let selection = Selection {
        source_installation: installation,
        target_installation: installation,
        devices: vec![device],
        lines: vec![],
    };
    let opaque = vec![knx_store::StoredOpaqueEntry {
        source_path: "seed.txt".into(),
        xpath: String::new(),
        kind: "BinaryData".into(),
        name: String::new(),
        bytes: b"existing evidence".to_vec(),
        sha256: knx_etsproj::opaque::sha256_hex(b"existing evidence"),
    }];
    (
        source,
        NativeSnapshot {
            project,
            opaque,
            manufacturer_refs: vec![],
        },
        selection,
    )
}

#[test]
fn selected_device_carries_links_parameters_modules_and_original_bytes_with_one_undo() {
    let (mut source, mut target, selection) = fixture();
    let device = DeviceId(selection.devices[0]);
    let parameter = ParameterInstance {
        id: ParameterInstanceId(301),
        device,
        source: SourceRef {
            path: "P-0001/0.xml".into(),
            ets_id: "parameter-ref".into(),
        },
        raw: " 007 ".into(),
    };
    source.project.installations[0]
        .parameters
        .push(parameter.clone());
    source
        .project
        .devices
        .insert_module_instance(ModuleInstance {
            id: ModuleInstanceId(302),
            device,
            source: SourceRef {
                path: "P-0001/0.xml".into(),
                ets_id: "module-ref".into(),
            },
            repeat_index: "6x1".into(),
            instance_ets_id: "module-instance".into(),
            arguments: vec![(parameter.source.clone(), "09".into())],
        });
    let before = target.clone();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut target.project,
            Command::RenameInstallation {
                id: InstallationId(selection.target_installation),
                name: "Unsaved edit".into(),
            },
        )
        .unwrap();
    let edited = target.project.clone();
    let plan = import::plan(&target, &source, selection).unwrap();
    assert_eq!(plan.preview.counts.devices, 1);
    assert_eq!(plan.preview.counts.parameters, 1);
    assert_eq!(plan.preview.counts.modules, 1);
    let preview = import::apply(&mut target, &mut stack, plan).unwrap();
    assert_eq!(target.project.info.name, before.project.info.name);
    assert_eq!(target.project.installations[0].name, "Unsaved edit");
    assert_eq!(target.project.installations[0].parameters[0].raw, " 007 ");
    assert_eq!(
        target
            .project
            .devices
            .module_instances()
            .next()
            .unwrap()
            .arguments[0]
            .1,
        "09"
    );
    assert_eq!(target.project.devices.iter().count(), 1);
    let new_device = target.project.devices.iter().next().unwrap();
    assert_ne!(new_device.id, device);
    let new_com = target.project.devices.com_objects().next().unwrap();
    assert_eq!(new_com.device, new_device.id);
    assert_eq!(
        new_com.links.len(),
        source
            .project
            .devices
            .com_objects()
            .next()
            .unwrap()
            .links
            .len()
    );
    assert!(new_com.links.iter().all(|link| {
        target.project.installations[0]
            .group_addresses
            .iter()
            .any(|g| g.id == link.ga)
    }));
    assert_eq!(target.opaque[0], before.opaque[0]);
    let archive = target
        .opaque
        .iter()
        .find(|e| e.kind == "SelectiveImportArchive")
        .unwrap();
    assert_eq!(archive.bytes, knx_testsupport::minimal_knxproj_bytes());
    assert_eq!(archive.sha256, preview.source_hash);
    assert_eq!(stack.history_lengths(), (2, 0));
    let merged = target.project.clone();
    stack.undo(&mut target.project).unwrap();
    assert!(target.project.same_user_content_as(&edited));
    stack.redo(&mut target.project).unwrap();
    assert_eq!(target.project, merged);
    let encoded = target.encode().unwrap();
    let decoded =
        NativeSnapshot::decode(&encoded, &knx_store::project_history::image_hash(&encoded))
            .unwrap();
    assert_eq!(decoded, target);
}

#[test]
fn unknown_duplicate_and_empty_selections_refuse_without_changing_seeded_target() {
    let (source, target, mut selection) = fixture();
    let before = target.clone();
    selection.devices.push(selection.devices[0]);
    assert!(import::plan(&target, &source, selection.clone())
        .unwrap_err()
        .contains("duplicate"));
    selection.devices = vec![u32::MAX];
    assert!(import::plan(&target, &source, selection.clone())
        .unwrap_err()
        .contains("device"));
    selection.devices.clear();
    assert!(import::plan(&target, &source, selection)
        .unwrap_err()
        .contains("empty"));
    assert_eq!(target, before);
}

#[test]
fn a_line_selects_its_devices_but_not_an_unselected_unassigned_device() {
    let (mut source, mut target, mut selection) = fixture();
    let existing = source.project.devices.iter().next().unwrap().clone();
    let mut unselected = existing.clone();
    unselected.id = DeviceId(999);
    unselected.com_objects.clear();
    unselected.address = None;
    source.project.devices.insert(unselected.clone());
    source.project.installations[0]
        .topology
        .unassigned
        .push(unselected.id);
    selection.devices.clear();
    selection.lines = vec![source.project.installations[0].topology.lines[0].id.0];
    let plan = import::plan(&target, &source, selection).unwrap();
    assert_eq!(plan.preview.counts.devices, 1);
    import::apply(&mut target, &mut CommandStack::new(), plan).unwrap();
    assert_eq!(target.project.devices.iter().count(), 1);
    assert!(target.project.devices.get(unselected.id).is_none());
}

#[test]
fn occupied_individual_address_and_dangling_link_refuse() {
    let (mut source, mut target, selection) = fixture();
    let mut existing = source.project.devices.iter().next().unwrap().clone();
    existing.id = target.project.ids.next_device_id().unwrap();
    existing.com_objects.clear();
    target.project.installations[0].topology.lines[0]
        .devices
        .push(existing.id);
    target.project.devices.insert(existing.clone());
    let before = target.clone();
    assert!(import::plan(&target, &source, selection.clone())
        .unwrap_err()
        .contains("address"));
    assert_eq!(target, before);
    target.project.devices = Devices::new();
    target.project.installations[0].topology.lines[0]
        .devices
        .clear();
    let com = source.project.devices.com_objects().next().unwrap().id;
    source
        .project
        .devices
        .com_object_mut(com)
        .unwrap()
        .links
        .push(GroupLink {
            ga: GroupAddressId(u32::MAX),
            direction: Direction::Receive,
        });
    assert!(import::plan(&target, &source, selection)
        .unwrap_err()
        .contains("group address"));
}

#[test]
fn group_address_conflict_and_stale_target_are_not_silent_overwrites() {
    let (source, mut target, selection) = fixture();
    let plan = import::plan(&target, &source, selection.clone()).unwrap();
    let mut stack = CommandStack::new();
    target.project.info.name = "Changed after preview".into();
    let before = target.clone();
    assert!(import::apply(&mut target, &mut stack, plan)
        .unwrap_err()
        .contains("stale"));
    assert_eq!(target, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    let mut ga = source.project.installations[0].group_addresses[0].clone();
    ga.range = None;
    ga.name = "Conflicting metadata".into();
    target.project.installations[0].group_addresses.push(ga);
    assert!(import::plan(&target, &source, selection)
        .unwrap_err()
        .contains("group address"));
}

fn source_with_installation_key_and_serial() -> Source {
    use std::io::Read;
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(
        knx_testsupport::minimal_knxproj_bytes(),
    ))
    .unwrap();
    let mut entries = Vec::new();
    for index in 0..zip.len() {
        let mut file = zip.by_index(index).unwrap();
        let name = file.name().to_owned();
        let mut bytes = vec![];
        file.read_to_end(&mut bytes).unwrap();
        if name.ends_with(".xml") {
            let mut text = String::from_utf8(bytes).unwrap().replace(
                "http://knx.org/xml/project/11",
                "http://knx.org/xml/project/23",
            );
            if name.ends_with("/0.xml") {
                assert!(text.contains("<Installation ") && text.contains("<DeviceInstance "));
                text = text
                    .replacen("<Installation ", "<Installation BCUKey=\"123\" ", 1)
                    .replacen(
                        "<DeviceInstance ",
                        "<Segment Id=\"S-1\"><DeviceInstance SerialNumber=\"BgcICQoL\" ",
                        1,
                    );
            }
            text = text.replacen("</DeviceInstance>", "</DeviceInstance></Segment>", 1);
            bytes = text.into_bytes();
        }
        entries.push((name, bytes));
    }
    let refs: Vec<_> = entries
        .iter()
        .map(|(name, bytes)| (name.as_str(), bytes.as_slice()))
        .collect();
    import::source_from_bytes(knx_testsupport::zip_with_entries(&refs), None).unwrap()
}

#[test]
fn retained_imported_installation_attributes_do_not_become_destination_access_keys() {
    let (_, mut target, mut selection) = fixture();
    let source = source_with_installation_key_and_serial();
    target.project.installations[0].topology = Topology {
        areas: vec![],
        lines: vec![],
        unassigned: vec![],
    };
    target.project.installations[0].default_line = None;
    selection.source_installation = source.project.installations[0].id.0;
    selection.devices = vec![source.project.devices.iter().next().unwrap().id.0];
    let before_key = knx_app::access_key::project_access_key(&target.opaque).unwrap();
    let plan = import::plan(&target, &source, selection).unwrap();
    import::apply(&mut target, &mut CommandStack::new(), plan).unwrap();
    assert_eq!(
        knx_app::access_key::project_access_key(&target.opaque).unwrap(),
        before_key
    );
    assert!(target
        .opaque
        .iter()
        .any(|e| e.name == "BCUKey" && e.bytes == b"123"));
}

#[test]
fn duplicate_ets_ids_do_not_select_another_sources_serial_number() {
    let (_, mut target, mut selection) = fixture();
    let source = source_with_installation_key_and_serial();
    target.project.installations[0].topology = Topology {
        areas: vec![],
        lines: vec![],
        unassigned: vec![],
    };
    target.project.installations[0].default_line = None;
    let device = source.project.devices.iter().next().unwrap();
    selection.source_installation = source.project.installations[0].id.0;
    selection.devices = vec![device.id.0];
    target.opaque.push(knx_store::StoredOpaqueEntry {
        source_path: device.source.path.clone(),
        xpath: format!("/DeviceInstance[@Id='{}']", device.source.ets_id),
        kind: "RetainedAttribute".into(),
        name: "SerialNumber".into(),
        bytes: b"AAECAwQF".to_vec(),
        sha256: knx_etsproj::opaque::sha256_hex(b"AAECAwQF"),
    });
    let plan = import::plan(&target, &source, selection).unwrap();
    import::apply(&mut target, &mut CommandStack::new(), plan).unwrap();
    let imported = target.project.devices.iter().next().unwrap();
    let serial =
        knx_app::serial_number::project_serial_number_for_source(&target.opaque, &imported.source)
            .unwrap();
    assert_eq!(
        serial,
        Some(
            knx_core::commissioning::serial_number::SerialNumber::from_octets([6, 7, 8, 9, 10, 11])
        )
    );
}

#[test]
fn selected_line_preserves_source_device_vector_order_not_numeric_id_order() {
    let (mut source, mut target, mut selection) = fixture();
    let old = source.project.devices.iter().next().unwrap().clone();
    let mut other = old.clone();
    other.id = DeviceId(77);
    other.address = None;
    other.com_objects.clear();
    source.project.devices.insert(other.clone());
    source.project.installations[0].topology.lines[0].devices = vec![other.id, old.id];
    selection.devices.clear();
    selection.lines = vec![source.project.installations[0].topology.lines[0].id.0];
    let plan = import::plan(&target, &source, selection).unwrap();
    let expected: Vec<_> = [other.id, old.id]
        .iter()
        .map(|id| {
            DeviceId(
                plan.preview
                    .mappings
                    .iter()
                    .find(|m| m.kind == "device" && m.source == id.0)
                    .unwrap()
                    .target,
            )
        })
        .collect();
    import::apply(&mut target, &mut CommandStack::new(), plan).unwrap();
    assert_eq!(
        target.project.installations[0].topology.lines[0].devices,
        expected
    );
}

#[test]
fn imported_building_flat_order_and_child_order_preserve_the_selected_source() {
    let (mut source, mut target, mut selection) = fixture();
    let old = source.project.devices.iter().next().unwrap().clone();
    let mut other = old.clone();
    other.id = DeviceId(77);
    other.address = None;
    other.com_objects.clear();
    source.project.devices.insert(other.clone());
    source.project.installations[0].topology.lines[0]
        .devices
        .push(other.id);
    let part = |id: u32,
                parent: Option<u32>,
                name: &str,
                children: Vec<BuildingPartId>,
                devices: Vec<DeviceId>| {
        BuildingPart {
            id: BuildingPartId(id),
            source: SourceRef {
                path: "P-0001/0.xml".into(),
                ets_id: format!("B-{id}"),
            },
            parent: parent.map(BuildingPartId),
            name: name.into(),
            number: None,
            kind: BuildingPartType::BuildingPart,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children,
            devices,
        }
    };
    source.project.installations[0].buildings = vec![
        part(
            10,
            None,
            "Root",
            vec![BuildingPartId(20), BuildingPartId(30)],
            vec![],
        ),
        part(20, Some(10), "Floor A", vec![BuildingPartId(40)], vec![]),
        part(40, Some(20), "Room A", vec![], vec![old.id]),
        part(30, Some(10), "Floor B", vec![], vec![other.id]),
    ];
    selection.devices = vec![old.id.0, other.id.0];
    let plan = import::plan(&target, &source, selection).unwrap();
    import::apply(&mut target, &mut CommandStack::new(), plan).unwrap();
    let names: Vec<_> = target.project.installations[0]
        .buildings
        .iter()
        .map(|b| b.name.as_str())
        .collect();
    assert_eq!(names, vec!["Root", "Floor A", "Room A", "Floor B"]);
}

#[test]
fn future_normalized_source_versions_refuse_instead_of_ignoring_new_entity_fields() {
    let (mut source, target, selection) = fixture();
    source.project.schema_version += 1;
    assert!(import::plan(&target, &source, selection).is_err());
}

#[test]
fn source_line_flat_and_area_child_order_are_not_sorted_by_id() {
    let (mut source, mut target, mut selection) = fixture();
    target.project.installations[0].topology.areas.clear();
    target.project.installations[0].topology.lines.clear();
    target.project.installations[0].default_line = None;
    let mut first = source.project.installations[0].topology.lines[0].clone();
    first.id = LineId(77);
    first.name = "First source line".into();
    let mut second = first.clone();
    second.id = LineId(22);
    second.address = 2;
    second.name = "Second source line".into();
    second.devices.clear();
    source.project.installations[0].topology.areas[0].lines = vec![first.id, second.id];
    source.project.installations[0].topology.lines = vec![first.clone(), second.clone()];
    selection.devices.clear();
    selection.lines = vec![first.id.0, second.id.0];
    let plan = import::plan(&target, &source, selection).unwrap();
    import::apply(&mut target, &mut CommandStack::new(), plan).unwrap();
    let topology = &target.project.installations[0].topology;
    let names: Vec<_> = topology.lines.iter().map(|l| l.name.as_str()).collect();
    assert_eq!(names, vec!["First source line", "Second source line"]);
    let child_names: Vec<_> = topology.areas[0]
        .lines
        .iter()
        .map(|id| topology.line(*id).unwrap().name.as_str())
        .collect();
    assert_eq!(child_names, names);
}

#[test]
fn selected_group_range_flat_and_child_order_preserves_source_positions() {
    let (mut source, mut target, selection) = fixture();
    let root = source.project.installations[0].group_ranges[0].clone();
    let mut parent = root.clone();
    parent.id = GroupRangeId(900);
    parent.name = "Root range".into();
    parent.parent = None;
    parent.children = vec![GroupRangeId(88), GroupRangeId(77)];
    parent.start = GroupAddress::from_raw(0);
    parent.end = GroupAddress::from_raw(511);
    let mut high = root.clone();
    high.id = GroupRangeId(88);
    high.name = "High range".into();
    high.parent = Some(parent.id);
    high.children.clear();
    high.start = GroupAddress::from_raw(256);
    high.end = GroupAddress::from_raw(511);
    let mut low = root.clone();
    low.id = GroupRangeId(77);
    low.name = "Low range".into();
    low.parent = Some(parent.id);
    low.children.clear();
    low.start = GroupAddress::from_raw(0);
    low.end = GroupAddress::from_raw(255);
    let mut ga = source.project.installations[0].group_addresses[0].clone();
    ga.address = GroupAddress::from_raw(100);
    ga.range = Some(low.id);
    let mut other = ga.clone();
    other.id = GroupAddressId(77);
    other.address = GroupAddress::from_raw(300);
    other.range = Some(high.id);
    other.name = "Other group".into();
    let object_id = source.project.devices.com_objects().next().unwrap().id;
    source
        .project
        .devices
        .com_object_mut(object_id)
        .unwrap()
        .links
        .push(GroupLink {
            ga: other.id,
            direction: Direction::Receive,
        });
    source.project.installations[0].group_ranges = vec![parent, high, low];
    source.project.installations[0].group_addresses = vec![ga, other];
    let plan = import::plan(&target, &source, selection).unwrap();
    import::apply(&mut target, &mut CommandStack::new(), plan).unwrap();
    let installation = &target.project.installations[0];
    let names: Vec<_> = installation
        .group_ranges
        .iter()
        .map(|r| r.name.as_str())
        .collect();
    assert_eq!(names, vec!["Root range", "High range", "Low range"]);
    let children: Vec<_> = installation.group_ranges[0]
        .children
        .iter()
        .map(|id| {
            installation
                .group_ranges
                .iter()
                .find(|r| r.id == *id)
                .unwrap()
                .name
                .as_str()
        })
        .collect();
    assert_eq!(children, vec!["High range", "Low range"]);
}
