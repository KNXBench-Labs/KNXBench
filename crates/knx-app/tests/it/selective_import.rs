//! Exercises selective merge identity, dependency preservation and honest refusal.

use knx_app::selective_import::{self as import, Selection, Source, UnselectedNotes};
use knx_core::note::{ProjectNote, ProjectNoteTarget};
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
        unselected_notes: None,
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

/// KL-106: the retention note names the sensitive classes it keeps.
#[test]
fn retention_note_names_network_endpoints_and_user_names() {
    let (source, target, selection) = fixture();
    let plan = import::plan(&target, &source, selection).unwrap();
    let note = &plan.preview.notes[0];
    for named in ["network endpoints", "MAC addresses", "user names"] {
        assert!(note.contains(named), "{named} missing from {note:?}");
    }
    assert!(plan.preview.retained_source_may_contain_unselected_data);
}

#[test]
fn selected_instruction_override_is_remapped_without_losing_existing_edits() {
    use knx_core::instruction::{InstructionDocument, InstructionRun};
    let (mut source, mut target, selection) = fixture();
    let id = DeviceId(selection.devices[0]);
    let document = InstructionDocument::new(vec![InstructionRun {
        text: "Source instruction Grüße".into(),
        style: Default::default(),
    }])
    .unwrap();
    source.project.instructions.insert(id, document.clone());
    let existing_id = target.project.ids.next_device_id().unwrap();
    let mut existing = source.project.devices.get(id).unwrap().clone();
    existing.id = existing_id;
    existing.address = None;
    existing.com_objects.clear();
    existing.binary_data.clear();
    existing.source = SourceRef {
        path: "KB-existing".into(),
        ets_id: "KB-existing".into(),
    };
    target.project.devices.insert(existing);
    target.project.installations[0]
        .topology
        .unassigned
        .push(existing_id);
    let existing_document = InstructionDocument::new(vec![InstructionRun {
        text: "Destination instructions must remain".into(),
        style: Default::default(),
    }])
    .unwrap();
    target
        .project
        .instructions
        .insert(existing_id, existing_document.clone());
    let before = target.clone();
    let plan = import::plan(&target, &source, selection).unwrap();
    let mapped = plan
        .preview
        .mappings
        .iter()
        .find(|mapping| mapping.kind == "device" && mapping.source == id.0)
        .unwrap()
        .target;
    assert!(
        plan.preview
            .notes
            .iter()
            .any(|note| note.contains("instruction")),
        "preview must disclose preserved native instruction edits"
    );
    let mut stack = CommandStack::new();
    import::apply(&mut target, &mut stack, plan).unwrap();
    assert_eq!(target.project.instructions[&DeviceId(mapped)], document);
    assert_eq!(target.project.instructions[&existing_id], existing_document);
    let encoded = target.encode().unwrap();
    assert_eq!(
        NativeSnapshot::decode(&encoded, &knx_store::project_history::image_hash(&encoded))
            .unwrap(),
        target
    );
    stack.undo(&mut target.project).unwrap();
    assert!(target.project.same_user_content_as(&before.project));
    assert_eq!(target.project.instructions, before.project.instructions);
    stack.redo(&mut target.project).unwrap();
    assert_eq!(target.project.instructions[&DeviceId(mapped)], document);
}
#[test]
fn dangling_source_instruction_owner_is_explicitly_refused() {
    let (mut source, target, selection) = fixture();
    source.project.instructions.insert(
        DeviceId(u32::MAX),
        knx_core::instruction::InstructionDocument::default(),
    );
    assert!(import::plan(&target, &source, selection)
        .unwrap_err()
        .contains("instruction"));
}

#[test]
fn native14_destination_notes_survive_selection_merge_and_undo() {
    let (source, mut target, selection) = fixture();
    let n = ProjectNote::new(
        ProjectNoteId(9),
        ProjectNoteTarget::Project,
        "keep".into(),
        "literal Grüße🙂".into(),
    )
    .unwrap();
    target.project.notes.push(n.clone());
    target.project.ids = target.project.ids.clone().with_project_note_count(9);
    let before = target.clone();
    let mut stack = CommandStack::new();
    let plan = import::plan(&target, &source, selection).unwrap();
    import::apply(&mut target, &mut stack, plan).unwrap();
    assert_eq!(target.project.notes, [n]);
    assert_eq!(target.project.ids.peek_project_note(), 9);
    stack.undo(&mut target.project).unwrap();
    assert!(target.project.same_user_content_as(&before.project));
    assert_eq!(target.project.notes, before.project.notes);
}
/// Source notes on the selected device, its first object, its line and the
/// source installation (all brought along), plus one on the project and one
/// on a device that is not selected (both outside the selection).
fn noted_source() -> (Source, NativeSnapshot, Selection, Vec<ProjectNote>) {
    let (mut source, target, selection) = fixture();
    let device = DeviceId(selection.devices[0]);
    let object = source.project.devices.get(device).unwrap().com_objects[0];
    let line = source.project.installations[0]
        .topology
        .lines
        .iter()
        .find(|l| l.devices.contains(&device))
        .unwrap()
        .id;
    let mut other = source.project.devices.get(device).unwrap().clone();
    other.id = DeviceId(900);
    other.address = None;
    other.com_objects.clear();
    source.project.devices.insert(other);
    source.project.installations[0]
        .topology
        .unassigned
        .push(DeviceId(900));
    let notes: Vec<_> = [
        (11, ProjectNoteTarget::Device(device), "device"),
        (12, ProjectNoteTarget::Project, "project"),
        (13, ProjectNoteTarget::ComObject(object), "object"),
        (
            14,
            ProjectNoteTarget::Device(DeviceId(900)),
            "unselected device",
        ),
        (15, ProjectNoteTarget::Line(line), "line"),
        (
            16,
            ProjectNoteTarget::Installation(InstallationId(selection.source_installation)),
            "installation",
        ),
    ]
    .into_iter()
    .map(|(id, target, title)| {
        ProjectNote::new(
            ProjectNoteId(id),
            target,
            title.into(),
            "literal Grüße🙂\r\n".into(),
        )
        .unwrap()
        .with_documentation(id % 2 == 0)
    })
    .collect();
    source.project.notes = notes.clone();
    source.project.ids = source.project.ids.clone().with_project_note_count(16);
    (source, target, selection, notes)
}
fn mapped(plan: &import::Plan, kind: &str, source: u32) -> u32 {
    plan.preview
        .mappings
        .iter()
        .find(|m| m.kind == kind && m.source == source)
        .unwrap_or_else(|| panic!("{kind} {source} not mapped"))
        .target
}

#[test]
fn source_notes_outside_the_selection_need_an_explicit_choice() {
    let (source, target, selection, _) = noted_source();
    let before = target.clone();
    let error = match import::plan(&target, &source, selection) {
        Ok(_) => panic!("source notes cannot be dropped or attached implicitly"),
        Err(e) => e,
    };
    assert!(
        error.contains("2 source notes are attached to entities outside the selection"),
        "{error}"
    );
    assert!(
        error.contains("skip them or attach them to the project"),
        "{error}"
    );
    assert_eq!(target, before);
}

#[test]
fn native_source_authored_notes_are_explicitly_refused_not_silently_dropped() {
    let (mut source, target, selection) = fixture();
    source.project.notes.push(
        ProjectNote::new(
            ProjectNoteId(1),
            ProjectNoteTarget::Project,
            "source".into(),
            "must not disappear".into(),
        )
        .unwrap(),
    );
    source.project.ids = source.project.ids.clone().with_project_note_count(1);
    let before = target.clone();
    let error = match import::plan(&target, &source, selection) {
        Ok(_) => panic!("source notes cannot be discarded"),
        Err(e) => e,
    };
    assert!(
        error.contains("1 source note is attached to entities outside the selection"),
        "{error}"
    );
    assert_eq!(target, before);
}

#[test]
fn selected_entity_notes_follow_their_remapped_identities_and_skipped_ones_are_reported() {
    let (source, mut target, mut selection, notes) = noted_source();
    let kept = ProjectNote::new(
        ProjectNoteId(3),
        ProjectNoteTarget::Project,
        "destination".into(),
        String::new(),
    )
    .unwrap();
    target.project.notes.push(kept.clone());
    target.project.ids = target.project.ids.clone().with_project_note_count(3);
    selection.unselected_notes = Some(UnselectedNotes::Skip);
    let before = target.clone();
    let device = selection.devices[0];
    let plan = import::plan(&target, &source, selection).unwrap();
    assert_eq!(plan.preview.counts.mapped_notes, 4);
    assert_eq!(plan.preview.counts.attached_notes, 0);
    assert_eq!(plan.preview.counts.skipped_notes, 2);
    let new_device = DeviceId(mapped(&plan, "device", device));
    let line = match notes[4].target() {
        ProjectNoteTarget::Line(id) => id,
        other => panic!("{other:?}"),
    };
    let new_line = LineId(mapped(&plan, "line", line.0));
    let object = match notes[2].target() {
        ProjectNoteTarget::ComObject(id) => id,
        other => panic!("{other:?}"),
    };
    let new_object = ComObjectInstanceId(mapped(&plan, "communication_object", object.0));
    for title in ["project", "unselected device"] {
        assert!(
            plan.preview
                .notes
                .iter()
                .any(|line| line.contains(&format!("{title:?}")) && line.contains("skipped")),
            "{:?}",
            plan.preview.notes
        );
    }
    let mut stack = CommandStack::new();
    import::apply(&mut target, &mut stack, plan).unwrap();
    let imported: Vec<_> = target.project.notes.iter().skip(1).collect();
    assert_eq!(target.project.notes[0], kept);
    assert_eq!(
        imported.iter().map(|n| n.target()).collect::<Vec<_>>(),
        [
            ProjectNoteTarget::Device(new_device),
            ProjectNoteTarget::ComObject(new_object),
            ProjectNoteTarget::Line(new_line),
            ProjectNoteTarget::Installation(InstallationId(before.project.installations[0].id.0)),
        ]
    );
    for (imported, original) in imported
        .iter()
        .zip([&notes[0], &notes[2], &notes[4], &notes[5]])
    {
        assert_eq!(imported.title(), original.title());
        assert_eq!(imported.body(), original.body());
        assert_eq!(
            imported.include_in_documentation(),
            original.include_in_documentation()
        );
    }
    assert_eq!(
        imported.iter().map(|n| n.id().0).collect::<Vec<_>>(),
        [4, 5, 6, 7],
        "fresh destination identities, never source ids"
    );
    assert_eq!(target.project.ids.peek_project_note(), 7);
    knx_core::note::validate_notes(&target.project, &target.project.notes).unwrap();
    stack.undo(&mut target.project).unwrap();
    assert_eq!(target.project.notes, before.project.notes);
}

#[test]
fn unselected_source_notes_can_be_attached_to_the_project_instead() {
    let (source, mut target, mut selection, _) = noted_source();
    selection.unselected_notes = Some(UnselectedNotes::AttachToProject);
    let plan = import::plan(&target, &source, selection).unwrap();
    assert_eq!(plan.preview.counts.mapped_notes, 4);
    assert_eq!(plan.preview.counts.attached_notes, 2);
    assert_eq!(plan.preview.counts.skipped_notes, 0);
    assert!(plan
        .preview
        .notes
        .iter()
        .any(|line| line.contains("\"unselected device\"")
            && line.contains("attached to the project")));
    assert_eq!(
        plan.preview
            .mappings
            .iter()
            .filter(|m| m.kind == "note")
            .count(),
        6
    );
    let mut stack = CommandStack::new();
    import::apply(&mut target, &mut stack, plan).unwrap();
    let titles: Vec<_> = target
        .project
        .notes
        .iter()
        .filter(|n| n.target() == ProjectNoteTarget::Project)
        .map(|n| n.title().to_owned())
        .collect();
    assert_eq!(titles, ["project", "unselected device"]);
    assert_eq!(target.project.notes.len(), 6);
}

#[test]
fn imported_notes_beyond_the_destination_budget_are_refused_not_truncated() {
    let (source, mut target, mut selection, _) = noted_source();
    selection.unselected_notes = Some(UnselectedNotes::Skip);
    target.project.notes = (1..=u32::try_from(knx_core::note::MAX_PROJECT_NOTES).unwrap())
        .map(|id| {
            ProjectNote::new(
                ProjectNoteId(id),
                ProjectNoteTarget::Project,
                "x".into(),
                String::new(),
            )
            .unwrap()
        })
        .collect();
    target.project.ids = target
        .project
        .ids
        .clone()
        .with_project_note_count(u32::try_from(knx_core::note::MAX_PROJECT_NOTES).unwrap());
    let before = target.clone();
    let error = match import::plan(&target, &source, selection) {
        Ok(_) => panic!("a full note collection cannot take more"),
        Err(e) => e,
    };
    assert!(error.contains("notes"), "{error}");
    assert_eq!(target, before);
}

#[test]
fn the_unselected_note_choice_is_part_of_the_strict_selection_wire() {
    let parsed: Selection = serde_json::from_value(serde_json::json!({
        "sourceInstallation": 0, "targetInstallation": 0, "devices": [1],
        "unselectedNotes": "attachToProject"
    }))
    .unwrap();
    assert_eq!(
        parsed.unselected_notes,
        Some(UnselectedNotes::AttachToProject)
    );
    let skip: Selection = serde_json::from_value(serde_json::json!({
        "sourceInstallation": 0, "targetInstallation": 0, "devices": [1],
        "unselectedNotes": "skip"
    }))
    .unwrap();
    assert_eq!(skip.unselected_notes, Some(UnselectedNotes::Skip));
    let absent: Selection = serde_json::from_value(serde_json::json!({
        "sourceInstallation": 0, "targetInstallation": 0, "devices": [1]
    }))
    .unwrap();
    assert_eq!(absent.unselected_notes, None);
    assert!(serde_json::from_value::<Selection>(serde_json::json!({
        "sourceInstallation": 0, "targetInstallation": 0, "devices": [1],
        "unselectedNotes": "drop"
    }))
    .is_err());
}

/// ADR-0124: a room and a group address of the destination with one
/// Function over them, unrelated to anything the selection brings in.
fn destination_function(target: &mut NativeSnapshot) -> knx_core::function::Function {
    use knx_core::function::{Function, FunctionDetails, FunctionMember};
    let native = SourceRef {
        path: String::new(),
        ets_id: String::new(),
    };
    let installation = &mut target.project.installations[0];
    installation.buildings.push(BuildingPart {
        id: BuildingPartId(900),
        source: native.clone(),
        name: "Destination room".into(),
        number: None,
        kind: BuildingPartType::Room,
        default_line: None,
        completion: CompletionStatus::Undefined,
        children: vec![],
        devices: vec![],
        parent: None,
    });
    installation.group_addresses.push(GroupAddressEntry {
        id: GroupAddressId(901),
        source: native.clone(),
        name: "Destination light".into(),
        address: GroupAddress::from_raw(0x7FFF),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Default::default(),
    });
    target
        .project
        .ids
        .raise_to(&IdAllocators::from_counts(0, 0, 0, 0, 0, 901, 900, 0, 0));
    let function = Function {
        id: FunctionId(1),
        source: native.clone(),
        location: BuildingPartId(900),
        details: FunctionDetails {
            name: "Keep me".into(),
            ..Default::default()
        },
        members: vec![FunctionMember {
            id: FunctionMemberId(1),
            source: native,
            group_address: GroupAddressId(901),
            name: String::new(),
            role: Some("SwitchOnOff".into()),
        }],
    };
    target.project.functions.push(function.clone());
    target.project.ids = target.project.ids.clone().with_function_counts(1, 1);
    function
}

#[test]
fn native15_destination_functions_survive_selection_merge_and_undo() {
    let (source, mut target, selection) = fixture();
    let kept = destination_function(&mut target);
    let before = target.clone();
    let mut stack = CommandStack::new();
    let plan = import::plan(&target, &source, selection).unwrap();
    import::apply(&mut target, &mut stack, plan).unwrap();
    assert_eq!(target.project.functions, [kept]);
    assert_eq!(
        knx_core::function::validate_functions(&target.project),
        Ok(())
    );
    stack.undo(&mut target.project).unwrap();
    assert!(target.project.same_user_content_as(&before.project));
}

#[test]
fn source_functions_are_explicitly_refused_not_silently_dropped() {
    let (mut source, target, selection) = fixture();
    let mut donor = target.clone();
    let function = destination_function(&mut donor);
    source.project.functions.push(function);
    let before = target.clone();
    let error = match import::plan(&target, &source, selection) {
        Ok(_) => panic!("source Functions cannot be discarded"),
        Err(e) => e,
    };
    assert!(
        error
            .contains("source project Functions require an explicit selection/disposition mapping"),
        "{error}"
    );
    assert_eq!(target, before);
}

// This historical NativeSnapshot was harvested from a successful ORIGINAL
// source_from_bytes + plan/apply + capture_primary, before stream-policy repair.
// It contains the actually retained raw archive (including trailer), decoded
// opaque rows and exact original primary/scoped report evidence. Normal native
// save/load below does not run the importer again or certify that old source.
fn wp16_historical_target() -> NativeSnapshot {
    use base64::Engine;
    use std::io::Read;
    let compressed = base64::engine::general_purpose::STANDARD
        .decode(WP16_HISTORICAL_IMAGE)
        .unwrap();
    // Only fixture storage is compressed; the decoded image/hash are the exact
    // bytes harvested from the original successful importer, not reconstructed SQL.
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(compressed)).unwrap();
    let mut bytes = Vec::new();
    zip.by_name("snapshot.knxdb")
        .unwrap()
        .take(327681)
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes.len(), 327680);
    let target = NativeSnapshot::decode(&bytes, WP16_HISTORICAL_IMAGE_HASH).unwrap();
    let archive = target
        .opaque
        .iter()
        .find(|e| e.kind == "SelectiveImportArchive")
        .unwrap();
    assert_eq!(
        archive.bytes,
        super::project_stream_fixture::project(
            super::project_stream_fixture::Layer::Outer,
            true,
            false
        )
    );
    assert_eq!(archive.sha256, WP16_HISTORICAL_SOURCE_HASH);
    assert_eq!(
        knx_etsproj::opaque::sha256_hex(&archive.bytes),
        WP16_HISTORICAL_SOURCE_HASH
    );
    let report =
        knx_app::import_evidence::source_reports::read(&target.opaque, WP16_HISTORICAL_SOURCE_HASH)
            .unwrap();
    assert_eq!(
        knx_etsproj::opaque::sha256_hex(report.report_json.as_bytes()),
        WP16_HISTORICAL_REPORT_HASH
    );
    let primary = knx_app::import_evidence::read(&target.opaque)
        .unwrap()
        .unwrap();
    assert_eq!(primary.report_json, report.report_json);
    let witness = target
        .opaque
        .iter()
        .find(|e| e.source_path == format!("imports/{WP16_HISTORICAL_SOURCE_HASH}/z-stream.bin"))
        .unwrap();
    assert_eq!(witness.bytes, super::project_stream_fixture::WITNESS);
    assert_eq!(
        witness.sha256,
        knx_etsproj::opaque::sha256_hex(super::project_stream_fixture::WITNESS)
    );
    assert!(target
        .opaque
        .iter()
        .any(|e| e.source_path == "seed.txt" && e.bytes == b"existing evidence"));
    target
}

fn wp16_new_selective_source_refusal(layer: super::project_stream_fixture::Layer) {
    use super::project_stream_fixture as stream;
    let target = wp16_historical_target();
    let before = target.clone();
    let dir = tempfile::tempdir().unwrap();
    let native_path = dir.path().join("historical.knxdb");
    let stack = CommandStack::new();
    knx_app::project_history::save_as(&native_path, &target, &stack).unwrap();
    let source_path = dir.path().join("fixed-new-source.knxproj");
    let bytes = stream::project(layer, true, false);
    std::fs::write(&source_path, &bytes).unwrap();
    let disk_before = stream::disk_snapshot(dir.path());
    let password = stream::password(layer);
    let error =
        match import::source_from_bytes(std::fs::read(&source_path).unwrap(), password.as_ref()) {
            Err(error) => error,
            Ok(_) => {
                panic!("damaged new source must not return Source, archive or report evidence")
            }
        };
    if matches!(layer, stream::Layer::ZipCrypto) {
        let expected =
            knx_etsproj::ImportFailure::Container(knx_etsproj::ContainerError::WrongPassword {
                nested_entry: "P-0001.zip".into(),
            })
            .to_string();
        assert_eq!(
            error, expected,
            "correct-password damage must retain existing WrongPassword mapping"
        );
    } else {
        let path = match layer {
            stream::Layer::Outer => stream::LATE_OPAQUE,
            stream::Layer::PlainNested => stream::NESTED_LEAF,
            stream::Layer::CompressedCarrier => "P-0001.zip",
            stream::Layer::ZipCrypto => unreachable!(),
        };
        assert!(
            error.contains(&format!("failed to read {path}:")),
            "{error}"
        );
    }
    assert!(target == before);
    assert!(
        stream::disk_snapshot(dir.path()) == disk_before,
        "refused new-source preparation changed historical destination/source bytes"
    );
    let reopened = knx_store::open_existing_read_only(&native_path).unwrap();
    assert!(NativeSnapshot::read(&reopened.conn).unwrap() == before);
    assert_eq!(std::fs::read(source_path).unwrap(), bytes);
}

#[test]
fn wp16_selective_refuses_new_outer_stream_before_source_evidence() {
    wp16_new_selective_source_refusal(super::project_stream_fixture::Layer::Outer);
}
#[test]
fn wp16_selective_refuses_new_plain_nested_stream_before_source_evidence() {
    wp16_new_selective_source_refusal(super::project_stream_fixture::Layer::PlainNested);
}
#[test]
fn wp16_selective_correct_password_zipcrypto_damage_retains_error_identity() {
    wp16_new_selective_source_refusal(super::project_stream_fixture::Layer::ZipCrypto);
}
#[test]
fn wp16_selective_refuses_new_compressed_carrier_before_source_evidence() {
    wp16_new_selective_source_refusal(super::project_stream_fixture::Layer::CompressedCarrier);
}

#[test]
fn wp16_historical_selective_raw_archive_and_exact_reports_survive_native_save_reopen() {
    let target = wp16_historical_target();
    let expected = target.clone();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("historical.knxdb");
    let stack = CommandStack::new();
    let generation = knx_app::project_history::save_as(&path, &target, &stack).unwrap();
    let reopened = knx_store::open_existing_read_only(&path).unwrap();
    let actual = NativeSnapshot::read(&reopened.conn).unwrap();
    assert!(actual == expected);
    let editor = knx_store::project_history::load_editor(&reopened.conn)
        .unwrap()
        .unwrap();
    assert!(editor.working == expected && editor.baseline == expected);
    drop(reopened);
    knx_app::project_history::save(&path, &actual, &stack, generation).unwrap();
    let reopened = knx_store::open_existing_read_only(&path).unwrap();
    let saved = NativeSnapshot::read(&reopened.conn).unwrap();
    assert!(
        saved == expected,
        "normal save relabelled or discarded historical source evidence"
    );
    for entry in &saved.opaque {
        assert_eq!(knx_etsproj::opaque::sha256_hex(&entry.bytes), entry.sha256);
    }
    assert_eq!(
        knx_app::import_evidence::source_reports::read(&saved.opaque, WP16_HISTORICAL_SOURCE_HASH)
            .unwrap()
            .report_json,
        knx_app::import_evidence::read(&saved.opaque)
            .unwrap()
            .unwrap()
            .report_json
    );
}

// Immutable original successful-import evidence; not synthesized report JSON.
const WP16_HISTORICAL_IMAGE_HASH: &str =
    "56d1fc1f317e5d7744760312d3f191707fb878fad5901967efe023c75a046215";
const WP16_HISTORICAL_SOURCE_HASH: &str =
    "08dfa2fa3942753a45b3885eabd94c5ef18b29d62ae3f587acf99ee354619ab2";
const WP16_HISTORICAL_REPORT_HASH: &str =
    "bdea485a039322a97f5be0a393fe62358e3937ce4361626054576b1bad142e38";
const WP16_HISTORICAL_IMAGE: &str = concat!(
    "UEsDBBQAAAAIAAAAVF3TnTnPgzYAAAAABQAOAAAAc25hcHNob3Qua254ZGLtnX1sHGd+33dIiktKotY+27fnu3M8ok5H7okr7sy+",
    "680mqZXMiKJkcpWzc/atn5l5lhxrd2Y9MyuJ9l0T2rLjS3LIWxuk6DUN0BYp0KRF2iL/9I+g7R9tEgRFUAQBAhRF26Ap0ARtmuba",
    "3rVon2dmX2ZmZ3epk2T7eN+PSHHmeX7P73n7Pb/neeZld/vlDd2hYt20msQRs7EnYoIQe1EUYx43Y30usd8p37nAfp+IHYAj/Dfx",
    "J/zwSuK7iW8n/jzxp4k/if/M9LX4D0+VJ6yJJxL/6yB6wOPnjbl4Mv+MsD+pGxq9RyxKarphO6TRII5uGjVd42En1rYqK9WKuL55",
    "ufKKGCUl3th0w8XFUETqvcvH48lnnhE+uO0QpUG5EP+d66isrqxuVDpJj4p+mNIA65vVytXKlnhza/36ytar4rXKq0uhFKEydVNs",
    "3qiKm7c2NsStypXKVmVzrbIdkF1kxQypapm2zqPCmXdVhcRts22ptNYizq57Xq28Uh0jSx275lZxlKxBmtR/PkqWaJpFbXugvYaI",
    "q2az1aD9OgZU90VT4nZ1a32tej9xLJ48fVr4CeJ2o7/5/MfHA93qjxnXvQfp4HBzjG6QdFrcNH19fk58o6Eb9A3RMu/aokoMUaGi",
    "yizPoRo7ZD6Ris4u1S3RvMsqtTOgzV+dBVvUaJ20G47IlYq6Ld42WDpRa1ssrUjEettpWzRCCcuySQ2mKd1uaSxzkZvNWbG6S0WT",
    "adJoQ1eoxSPahmqyPC3CctDEK9fOBrV1ClDjBfBsqdOEoYZoMiFdJbZT69oIb7Yx9jDWIva/NhtPSpLw3inXIlgxWb1r7nGNVc/a",
    "Gww5GrCOwfiwjdyme+O7uUGMnTbZoSOF7pBGm47T5DM8cZHlvdTTnRqs/trR6eSXJSHmOk77rQabVmuk7ZjueW2wbjVpMOzYfWkm",
    "nlxYED486Y0qrcbsy1SJY1p24GQ2OK78UeMHVuTQEtdeqqxdY/5aEy+KmbD/0+gdXaXRSob5H+7Dh+Q7rOv4yHmwJMxMa6byJlUd",
    "bwoyWCnHJNmxzHarZrGupAfNxUsS9KhjkihtvaHx/m0RyzlYEibJHJpDrX5VBpIMGt5778Q9T1x2baZlmZ3WqJv+45mAxfhjHovB",
    "9DLwKXuQmW2cfFe/0W4y59iXH9VxbBTuNUb7hUGvN87ZMC/aNDW9rlPNJz+ktKxXe8YQIdZz4F0XFs59sPfb0/Hk+bSwn3D9TJMY",
    "7TpR+URj1Sxar9m7RM4XwsHxwPptSCK+hAtHiYteXOrds0fiyXRauJ9wjS4sFz6fDhjfoNbhBjh+EeBfa41cZ3m1Gt+ltN/3Ywbs",
    "bdbo3eMx/eRMxZMXTwv7T7n9ZLbIW+3uROCrgD/8SKCThqXgveSPYz3Uj0y9a066vuF9bz7xC/qPpwLdE1T3UXTNvd5CeXTXDG3v",
    "Ec5klJyy59CuO1/duLH6wKYz2M+GEE+eOiXsdxZB6i4bGzXm1InvcCK47OlHuKsMLwd/o3ZWK4Gcu/nlJ6aT66eGLjs6utnO7nyZ",
    "/ZfOsP+em2f/Pflp9t8Tx+fY/0LiP8fYDwAAAAAAAAAAAL7P+bQwefL5zr2tmrJ327iXth3TokdjfP//32PsBwAAAAAAAAAAAIeG",
    "ucmTs/0LAfzZ/MnEn/OIm96z+R9z8Uax/+Lk0dj+C3PPz+X25/Rmy7QcezlT0upErpNsOScX81mSyyvZUilPiaKVc2qe1qWSIpe1",
    "gkxotp4vFYlaL5cpzeZzBalMFHn5ZjqTyUhnbX3HIPyRpO3ugWE6IhFZUzXEXmSuXJbUXDmn5eWMUsznsjm1KKmkntPUfEbK1hVF",
    "y9GMqpRJRs0VC3Jdqat5lRQyuboiFdT8t56fOBp7ea4s/eb8/txt455CDXU3zXTrd+iyV6W0Rfmfs2/apnHNuLfKJdbdmModXWNn",
    "dFDunXnvDY75c/MhpemOMO2knV+av0MtWzeN+XPS0nzLMrW2Si2W8NrmK25eIr84RFqt5cxZ6WwmTRqtXXI2y9J5+dV4fkz8ndfm",
    "vcd7Xps/x47reoPW+HM37LQbc5Zp4k++vTa/1BGw9be5QDafKbGgztMpnfKwcElioVyJ3SKqp2nXcVrnlpeZprOmtbN8r9lY7jxM",
    "tyxJruK+MbsJKtXtnBvumGbDp9uNEXOsTiU3updNTdNtsmNR9yFsJmi0G42vL3XrUDMVm1p33Ge8ba+qTUr44+CuTlaeNG24SdOm",
    "qrYtizeynWa2k7b5Y3GOrqaJqtKW+zxnp2Ats2Hu7HnaVljxedVZxJrZvOE+yrreefxzi9Y7UZfd52+74Z3Aq/zhxhXv2UZ/0BZ/",
    "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",
    "c9LSfMsytbZKLZbw2uYrbl4i05AmrdZy5qx0NpMmjdYuOZtl6bz8ajw/Jv7Oa/PeFxa9Nn+OHbvfG8e/l4iddmPOMk38ay9em1/q",
    "CNj621wgm8+UWJCt7tImqXXKw8IliYVyJXaLqJ6mXcdpnVteZprOmtbO8r1mY7nzTRrLkuQqVlkPOVSrKXtugkp1O+eGO6bZ8Ol2",
    "Y8Qcq1PJje5lU9N0m+xYlPIvJ2GCRrvR+PpStw41U7Gpdcf90hLbq2qTEkM3dlydrDxp2nCTpk1VbVsWb2Q7zWwnbfOvRXR0NU1U",
    "lbYcYvAquQVrmQ1zZ8/TtsKKz6vOIta6XxKzzr9+holv0Xon6rL7JTPd8E7gVd93zPiD3O+gYQEyC1j3fZNNR2ZDN1wNvJJN6hCN",
    "OLwE73ydB6hm23A6FbXMu/zoK+yQ1U93vAb2CsxE2V+to7LJ7IVqntKgtJfZQaXD1TxouuiWO2hqf6P108j+NHJkml7bHzSnVd9X",
    "B/lTZfypMuFUTJoZq0OtIbUbnfpK5/upDp7CX7fRWb3OTaZt3DbMu9y6vvI6OzVb5K027diN77vPvKqEnK2r+14v2j3l3+Tjnm0H",
    "pHrOxT3reBKZF8j7pk837mH9c7eudsdnEEvdZV60PwGcF1Vi8JlBoWxq2KEGtbj3eW3ebcTo6jJPyvzEyKqumYZD2FCxKoZj7Q2v",
    "r1SWi8EaF0k+W5ZlVhetrhSyckbTyqRQLhEq13M0q8qqKhWlekkulUipqJY0miNlLV8vZohcKNbr4Rqr3ZKIlBdF5HX1VVRU9kRn",
    "V7e9b70bXe2bnqN+ZJWX5VKw7nlSzBGlJKmsNrlyNpsr5VRZprk8axJVLdTlTDlfpppSVDL1Ur2sEkIzMi3nMkxKzmYeS93ZTFVr",
    "EpuN1rH1vu6KXXbd79AOzxSCdZZLaqGkqDJlfSfnS4WCnGWrHVVjnVtQtYxWzFBaLGhUJjkpS5UMYdVl66BSRs3XJSU3YOHOLhUv",
    "36wueysBR/TKzqycTRvmDh/IkdV8O207TEvzrKIbj6Rzs+GRnGejmOQlZtklpa6VpLpC6/m8WsxkZLaAyyo0r9VzJakoZ4vlgqaU",
    "pWIxX6rncrw5CvXs4+jb655de39qK2npAF3c/yLdMR0tycEGUNlglmi9TEtlpZRV6kVSLrJezEmEsJUrqzWRCjlZlvLFQq5AcsVs",
    "tsAGfT1Lc6xwklQKN4D/O32XmQtv6N53W6ZZz++wCUbk64Dzou2YFmsO3RC5ZbDy8LOucXARhdhUXFy5vMVbIZ8a3VSrZGeH7FB7",
    "mX/t2VnnnjOytTrSI4xEChkJUQtqvV4oaRobAjnCbD/H3FuGmUCxTqWsRFkrkQwzn1yZyMxmSEFxvQUpZotZSQu3kTd3Bb6vWmyR",
    "vYZJtPOsEShfClJtSTToHW5I96jaHu78w9YiP0pryYVaIpsvKVJRk9gmR5OVXFEibDukMH9YUtn0Vy4zl1fPlNhuiA0hwhxEViVy",
    "qSApclHSVCWf+eisha8RdKNO2VJZ664Z2OCssxzchacb0DbsdotvNzwZ1rp3d4kz3rKYIpuyLjQ624dhwufEA3c18xO64fBviabc",
    "T5iBfudlZTUxLa/kX5//ulKXpFxBlZWSls0U82yHmSnKbAeqZsqljEYLJZXtR7VclsrMuylsYZLNKQWFFjJyJsfdV0OYjT03d3Lu",
    "7P6cTanGC7uqG8Ta4wZB7+m2w9aPYnc7l2MzOxv0tCzJWS2vlJWirJWK9Ww9WywV2VGmUCrSQpYtCLIlNcdXQWwepJRNIVKdknyJ",
    "7//jsf8bi/+r6fSRp6euTH524go7/cHg3StHj8X21+fSc+9+8RFeUIgc9+GhfeEFFih2tqcX59lOe55NTKrJdwcX59tOPV2af+HS",
    "0QtsVy4yScO+OHorPH/pqCheCOciLjMVJ9Np0d4z2ABlm1GREquxlzboXTGdZpHLLINLD+s89l+ePRrbvzH3ubkffvTtODB2OwEq",
    "sSydDUhnl21XdnbDTvlhp4Z3UzMd49hvPEbjkD5m43D7/2GXGvvX4qz/i3OpuZceYVP5l5jBVSTrUVPzJhpeQ9GTE+/qjsG2rA+7",
    "cnz309Os588zH7z/xiOsTnBj0F/7P67O7urvd/PDbh3e+8wR1jBrrJvfqz7yC7D+3WKwtx9PA3WyE9e1i539qhvej1k3vGuq/NuK",
    "N9ka7OL8dd3Qm6QxL/ovkGw7ew0WV921KN1gHohFr/W+CH3bIU6bleeWodE6q5A2z3uD5dGtbbdrHnYn+7P5qWPeCPzlzz76rsl8",
    "BJ0irnnXcVf3LrrXcOfFqmk2fqSbQe/i7bje81/wtC91vjU5ECr6T7gCVnSvg+fFy7RO2g2HX7XsKk5nahtpeXSvXup9O/OFaufq",
    "7iXfN75f4NdMfWVlKpnn7+a6Mi92bIm140HzcdXyYgbVuiX11G4E1V6nmt5uVvda/OohT3O9ms5E5bbiXq0eyIxlF7w6G8z48nq/",
    "QpfnxZveFqCblTfbvZSWajeZWEhvmJeIpd1lGwn5prfVGFTyEtfir90YjRvMGV5nplln64WL83JGzqYzxbSUq0rSuXz+XDYb1Q5X",
    "2Gi3d6l2mfKLf+OyWDc0na3J26TRKdYG20jw3HhB+9unTpX6kWPU9i79+vWxsjbbRlcjsZwDq/OMgI3kur7jU7huB1R6l9J/RLd1",
    "RW/ozt4aaajtBh+cB8hi1TJvU4MPqrD9MAuKuk5vD8oNkRSDlsCGUO0GM70tbgx8pmuZbKPGLZyP5HV7ReU3wHiRo3Jw8zAMlgPb",
    "vF26sE0NTQxd+A4Y+FV3xC5furDsSxZV8uWooke0RaTcgMoLy8FRF/IAy9wFBFzNMvc1Pn+0POiQLvjrSQNZXujfCwkVxRcTHPlX",
    "t/ojf0NXd5150ZVi48hyXPNyTysGH3r5/KBbGaG458xWjOWVtj1CM1vtRRmcv6ZiRH/6PaSX09WVzhwdbOd+IcM9EB3jD7f93RHd",
    "9heW/ZNSZypbHpjLBtYND3v1n+//BeHnYuwHfBL4VuOYu/koz/2m8wgXcsEb8tu0QV3n6D1XsOLd5Lp5bXLqaKcYJ2Pnt7/59NPs",
    "iP/yF0TCd+6ino4Iapj4uZ9J/kQ8FuO/x3savMUklo9YPmL52AfLxw5YPmL5iOXjR7V8DE7YL0pfKvwxO+K/n+pP2L4Lcz8Il+KC",
    "bXL3z679ypvsiP+eYL/Bq7gf3bXbYKl+46d/+Wdb7Ij/PsN+I+8qfNR3EYJF/C9/+c/+4nl2xH8/0y/iwN2cMbdxgkrf+W+f//1v",
    "sqNvDqm3/BHXe/yttW75Z9zy/8dff/Efn2ZHPxTzlsT+Wxz/+4di//7/jb6zEfu7N68JE0cnh6/Su/yd/VjEmj2cOrxC76deDa3X",
    "wynDrqKf8i/iUY4jnD48rPrp47PhQRZOGzb+ftq/OTtkKIRVhI2zr+LFo8NNNawlbI19LX/t6BDb7KuINoi+ilvHguZx89qR6Zib",
    "bib2HYFJHONnD7sxdPf/k381xn4AAJ8Afkk/7t5mLc79g91HeAlo8K2R0GWgLTfmnc5zbPPn3um/1DF/LvRKx/xS/4WOzvscodc5",
    "3Lc5+m9ZMA2jJ9kl34scTNi9hrMUeInDC+1cxlka8gJH9/2NqNc3eJU6L28wXQ/46oZbGG8rxPXwbZL7Bk3UNsyNCG683CD/YrUf",
    "4C7y+fsGgZc13Hi+PXNfI+i9qOG9p9F5TYMXhL+kwZ+Q6zzAzyrmFm3JfWLfVeI9r++9jdCXclWPlQpVYqx8ZGuMTeVrhq6s3JeV",
    "B2W7bThWs/+Fi550pi+dCUhHvWgxPlX3BYvxkqELBNEJXmfd23mlwn0YsvNQqtvHvkdMeXFDyzum714nih3yp0rZ0bYvtjOU2ZE3",
    "bmWWsfcMKQt92FcnvNp4L4cd9LUJ/uBsZKW8VWdkhYLPIUTUyn1Hol+vh71F4q/XAz1EPrxyvoXx91ZF/iZEv4YP+/DIo6xhcOke",
    "Wbv+9jaq8zIFX80e9oklf83GvegQURn/+vt766hsYIQ95INpj7KfIrdJQ7oruPGN6jRJ9lXzYZ8m9FfzMT2GPrxBBjZ9kW3SkYrq",
    "cMnf4Q/5DKq/Jb63VxMO0PPyQ/V8zl/fh32O+CPpeTajdl8/8ObX3ssH3qnv1QN3zuUvHoy2D99LByMEH8krB27xvRcO3PcNFI2S",
    "XClPMtlyVpbZQKvnFZphm4RsnTJnmS9RdlhUaS5bkApyIZPPsVGnSArRpJxMs6X/D1BLAQIUAxQAAAAIAAAAVF3TnTnPgzYAAAAA",
    "BQAOAAAAAAAAAAAAAACAAQAAAABzbmFwc2hvdC5rbnhkYlBLBQYAAAAAAQABADwAAACvNgAAAAA=",
);
