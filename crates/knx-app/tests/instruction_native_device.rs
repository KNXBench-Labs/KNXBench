//! Instructions for devices made in KNXBench: no imported original, none invented (Goal B4).
use knx_app::instruction_edit::*;
use knx_app::instructions::{instruction_origin, InstructionOrigin, InstructionSourceError};
use knx_core::instruction::{InstructionDocument, InstructionRun};
use knx_core::{
    CommandStack, CommissioningState, DeviceId, DeviceInstance, Language, Project, SourceRef,
};
use knx_store::project_history::{self as history, NativeSnapshot};
use knx_store::StoredOpaqueEntry;

fn device(id: u32, path: &str, ets_id: &str) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: SourceRef {
            path: path.into(),
            ets_id: ets_id.into(),
        },
        name: format!("Device {id}"),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

/// One device created in KNXBench (`KB-DEV-1`, as the add-device command names it).
fn native() -> (NativeSnapshot, DeviceId) {
    let mut project = Project::new(Language("en".into()));
    // Allocated like the add-device command, so the native store accepts it.
    let id = project.ids.next_device_id().unwrap();
    assert_eq!(id, DeviceId(1));
    project.devices.insert(device(1, "KB-DEV-1", "KB-DEV-1"));
    project.installations.push(knx_core::Installation {
        id: knx_core::InstallationId(0),
        name: "Native installation".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::Editing,
        topology: knx_core::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    (
        NativeSnapshot {
            project,
            opaque: vec![],
            manufacturer_refs: vec![],
        },
        DeviceId(1),
    )
}

fn revision() -> InstructionRevision {
    InstructionRevision {
        project_incarnation: "synthetic-native-1".into(),
        generation: 1,
    }
}

fn doc(text: &str) -> InstructionDocument {
    InstructionDocument::new(vec![InstructionRun {
        text: text.into(),
        style: Default::default(),
    }])
    .unwrap()
}

fn container(path: &str) -> StoredOpaqueEntry {
    StoredOpaqueEntry {
        source_path: path.into(),
        xpath: String::new(),
        kind: "ContainerEntry".into(),
        name: String::new(),
        bytes: b"<KNX/>".to_vec(),
        sha256: knx_etsproj::opaque::sha256_hex(b"<KNX/>"),
    }
}

#[test]
fn only_a_device_made_in_knxbench_without_a_retained_source_is_native() {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(device(1, "KB-DEV-1", "KB-DEV-1"));
    project
        .devices
        .insert(device(2, "KB-SCAN-0101", "KB-SCAN-0101"));
    // An imported device whose retained source is missing stays a refusal.
    project
        .devices
        .insert(device(3, "P-0001/0.xml", "P-0001-0_DI-1"));
    // A KB- path with another id is not what KNXBench issues.
    project
        .devices
        .insert(device(4, "KB-DEV-4", "P-0001-0_DI-4"));
    project.devices.insert(device(5, "", ""));
    // A retained source for the path wins over the name.
    project.devices.insert(device(6, "KB-DEV-6", "KB-DEV-6"));
    let opaque = vec![container("KB-DEV-6")];
    assert_eq!(
        instruction_origin(&project, &opaque, DeviceId(1)),
        Ok(InstructionOrigin::Native)
    );
    assert_eq!(
        instruction_origin(&project, &opaque, DeviceId(2)),
        Ok(InstructionOrigin::Native)
    );
    for id in [3, 4, 5] {
        assert_eq!(
            instruction_origin(&project, &opaque, DeviceId(id)),
            Err(InstructionSourceError::MissingSource),
            "device {id}"
        );
    }
    assert_ne!(
        instruction_origin(&project, &opaque, DeviceId(6)),
        Ok(InstructionOrigin::Native)
    );
    assert_eq!(
        instruction_origin(&project, &opaque, DeviceId(9)),
        Err(InstructionSourceError::DeviceNotFound)
    );
}

#[test]
fn a_native_device_gets_instructions_in_one_undo_step_without_an_invented_original() {
    let (mut snapshot, id) = native();
    let before = snapshot.clone();
    let rev = revision();
    let plan =
        preview_instruction_edit(&snapshot, &rev, id, Some(doc("Mount above the door"))).unwrap();
    assert_eq!(plan.origin(), &InstructionOrigin::Native);
    assert!(plan.original().is_none());
    assert!(!plan.conversion_required());
    assert!(!plan.is_no_op());
    let mut stack = CommandStack::new();
    assert!(
        apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None)
            .unwrap()
            .changed
    );
    assert_eq!(
        snapshot.project.instructions[&id].plain_text(),
        "Mount above the door"
    );
    assert_eq!(stack.history_lengths(), (1, 0));
    assert!(
        snapshot.opaque.is_empty(),
        "no original is written for a native device"
    );
    stack.undo(&mut snapshot.project).unwrap();
    assert_eq!(snapshot, before);
}

#[test]
fn removing_leaves_no_instructions_and_an_explicit_empty_document_stays_distinct() {
    let (mut snapshot, id) = native();
    let rev = revision();
    let mut stack = CommandStack::new();
    // Nothing there yet: removing is a no-op, not an invented original.
    let none = preview_instruction_edit(&snapshot, &rev, id, None).unwrap();
    assert!(none.is_no_op());
    // An explicitly empty document is a change and stays present.
    let empty = InstructionDocument::new(vec![]).unwrap();
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(empty.clone())).unwrap();
    assert!(!plan.is_no_op());
    apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None).unwrap();
    assert_eq!(snapshot.project.instructions.get(&id), Some(&empty));
    // Removing returns to absent.
    let plan = preview_instruction_edit(&snapshot, &rev, id, None).unwrap();
    assert!(!plan.is_no_op());
    apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None).unwrap();
    assert!(snapshot.project.instructions.get(&id).is_none());
    assert_eq!(stack.history_lengths(), (2, 0));
}

#[test]
fn a_failed_save_changes_neither_project_history_nor_file() {
    let (mut snapshot, id) = native();
    let before = snapshot.clone();
    let rev = revision();
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("edited"))).unwrap();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.knxdb");
    let target = InstructionNativeTarget {
        path: &missing,
        history_generation: 1,
    };
    assert!(
        apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, Some(target))
            .is_err()
    );
    assert_eq!(snapshot, before);
    assert_eq!(stack.history_lengths(), (0, 0));
    assert!(!missing.exists());
}

#[test]
fn a_saved_native_edit_reopens_with_undo() {
    let (mut snapshot, id) = native();
    let before = snapshot.clone();
    let mut stack = CommandStack::new();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("native.knxdb");
    let generation = knx_app::project_history::save_as(&path, &snapshot, &stack).unwrap();
    let rev = InstructionRevision {
        project_incarnation: "synthetic-native".into(),
        generation: generation as u64,
    };
    let plan =
        preview_instruction_edit(&snapshot, &rev, id, Some(doc("Ceiling, kitchen"))).unwrap();
    let target = InstructionNativeTarget {
        path: &path,
        history_generation: generation,
    };
    apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, Some(target)).unwrap();
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(recovered.working, snapshot);
    let mut restored = recovered.working;
    let mut stack = CommandStack::from_snapshot_states(recovered.undo, recovered.redo);
    stack.undo(&mut restored.project).unwrap();
    assert_eq!(restored, before);
}

#[test]
fn a_device_that_gains_a_retained_source_after_preview_is_stale() {
    let (mut snapshot, id) = native();
    let rev = revision();
    let plan = preview_instruction_edit(&snapshot, &rev, id, Some(doc("x"))).unwrap();
    snapshot.opaque.push(container("KB-DEV-1"));
    let mut stack = CommandStack::new();
    assert!(apply_instruction_edit(&mut snapshot, &mut stack, &rev, &plan, false, None).is_err());
    assert!(snapshot.project.instructions.is_empty());
}
