//! Verifies normalized instruction integrity with synthetic data.
use knx_core::instruction::{InstructionDocument, InstructionRun};
use knx_core::*;
fn project() -> Project {
    let mut p = Project::new(Language("en".into()));
    let id = p.ids.next_device_id().unwrap();
    p.devices.insert(DeviceInstance {
        id,
        source: SourceRef {
            path: "synthetic/0.xml".into(),
            ets_id: "D1".into(),
        },
        name: "Synthetic".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "A".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    p.installations.push(Installation {
        id: InstallationId(0),
        name: "Synthetic".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Undefined,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![id],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    p
}
fn doc() -> InstructionDocument {
    InstructionDocument::new(vec![InstructionRun {
        text: "literal Grüße".into(),
        style: Default::default(),
    }])
    .unwrap()
}
#[test]
fn edit_reset_undo_redo_preserve_absent_and_explicitly_empty() {
    let mut p = project();
    let original = p.clone();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut p,
            Command::SetDeviceInstructions {
                device: DeviceId(1),
                document: Some(doc()),
            },
        )
        .unwrap();
    assert_eq!(p.instructions[&DeviceId(1)], doc());
    stack
        .do_command(
            &mut p,
            Command::SetDeviceInstructions {
                device: DeviceId(1),
                document: Some(InstructionDocument::default()),
            },
        )
        .unwrap();
    assert_eq!(p.instructions[&DeviceId(1)], InstructionDocument::default());
    stack
        .do_command(
            &mut p,
            Command::SetDeviceInstructions {
                device: DeviceId(1),
                document: None,
            },
        )
        .unwrap();
    assert_eq!(p, original);
    stack.undo(&mut p).unwrap();
    assert_eq!(p.instructions[&DeviceId(1)], InstructionDocument::default());
    stack.undo(&mut p).unwrap();
    assert_eq!(p.instructions[&DeviceId(1)], doc());
    stack.undo(&mut p).unwrap();
    assert_eq!(p, original);
    for _ in 0..3 {
        stack.redo(&mut p).unwrap();
    }
    assert_eq!(p, original);
}
#[test]
fn unchanged_override_is_a_noop_preserving_redo() {
    let mut p = project();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut p,
            Command::SetDeviceInstructions {
                device: DeviceId(1),
                document: Some(doc()),
            },
        )
        .unwrap();
    stack.undo(&mut p).unwrap();
    let lengths = stack.history_lengths();
    stack
        .do_command(
            &mut p,
            Command::SetDeviceInstructions {
                device: DeviceId(1),
                document: None,
            },
        )
        .unwrap();
    assert_eq!(stack.history_lengths(), lengths);
    assert!(stack.can_redo());
}
#[test]
fn unknown_device_refuses_without_project_or_history_mutation() {
    let mut p = project();
    let before = p.clone();
    let mut stack = CommandStack::new();
    assert!(stack
        .do_command(
            &mut p,
            Command::SetDeviceInstructions {
                device: DeviceId(99),
                document: Some(doc())
            }
        )
        .is_err());
    assert_eq!(p, before);
    assert_eq!(stack.history_lengths(), (0, 0));
}
#[test]
fn delete_removes_and_undo_restores_the_exact_instruction_override() {
    let mut p = project();
    p.instructions.insert(DeviceId(1), doc());
    let before = p.clone();
    let mut stack = CommandStack::new();
    stack
        .do_command(&mut p, Command::DeleteDevice { id: DeviceId(1) })
        .unwrap();
    assert!(
        p.instructions.is_empty(),
        "device deletion must not leave a dangling instruction"
    );
    stack.undo(&mut p).unwrap();
    assert_eq!(p, before);
    stack.redo(&mut p).unwrap();
    assert!(p.instructions.is_empty());
}
