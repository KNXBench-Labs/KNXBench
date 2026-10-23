//! Atomic adoption uses the normal stack, including P3 pending and full-history binding.
use super::comparison_adoption::{
    AdoptionCommandError, ParameterAdoption, PreparedComparisonAdoption,
};
use super::*;
use crate::{CommissioningState, CompletionStatus, Language};

fn fixture() -> Project {
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
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "fixture".into(),
            ets_id: "D".into(),
        },
        name: "D".into(),
        description: None,
        address: Some("1.1.7".parse().unwrap()),
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}
fn changes() -> Vec<ParameterAdoption> {
    [("R-1", "17"), ("R-2", "29")]
        .into_iter()
        .map(|(reference, raw)| ParameterAdoption {
            reference: reference.into(),
            raw: raw.into(),
        })
        .collect()
}
#[test]
fn adoption_is_one_undo_step_and_preserves_allocator_high_water() {
    let mut project = fixture();
    let before = project.clone();
    let mut stack = CommandStack::new();
    let prepared =
        PreparedComparisonAdoption::prepare(&project, &stack, DeviceId(1), 0, changes()).unwrap();
    assert_eq!(project, before, "preview is read-only");
    prepared.apply(&mut project, &mut stack).unwrap();
    assert_eq!(project.installations[0].parameters.len(), 2);
    assert_eq!(stack.history_lengths(), (1, 0));
    assert_eq!(stack.comparison_revision(DeviceId(1)), 1);
    let adopted = project.clone();
    stack.undo(&mut project).unwrap();
    assert!(project.same_user_content_as(&before));
    assert_eq!(project.ids, adopted.ids, "reservations never rewind");
    stack.redo(&mut project).unwrap();
    assert_eq!(project, adopted);
    assert_eq!(stack.comparison_revision(DeviceId(1)), 3);
}
#[test]
fn adoption_refuses_changed_history_even_when_project_and_p3_are_equal() {
    let mut project = fixture();
    let mut stack = CommandStack::new();
    let prepared =
        PreparedComparisonAdoption::prepare(&project, &stack, DeviceId(1), 0, changes()).unwrap();
    stack
        .do_command(
            &mut project,
            Command::RenameDevice {
                device: DeviceId(1),
                name: "Other".into(),
            },
        )
        .unwrap();
    stack.undo(&mut project).unwrap();
    let before = project.clone();
    let lengths = stack.history_lengths();
    assert_eq!(stack.comparison_revision(DeviceId(1)), 0);
    assert!(matches!(
        prepared.apply(&mut project, &mut stack),
        Err(AdoptionCommandError::Stale)
    ));
    assert_eq!(project, before);
    assert_eq!(stack.history_lengths(), lengths);
}
#[test]
fn adoption_refuses_duplicate_mapping_and_late_command_failure_atomically() {
    let project = fixture();
    let stack = CommandStack::new();
    let mut duplicate = changes();
    duplicate.push(duplicate[0].clone());
    assert!(matches!(
        PreparedComparisonAdoption::prepare(&project, &stack, DeviceId(1), 0, duplicate),
        Err(AdoptionCommandError::DuplicateReference(_))
    ));
    let mut malformed = project.clone();
    // Allocate id 1 in the cloned planner, then collide at the second row.
    malformed.installations[0]
        .parameters
        .push(ParameterInstance {
            id: ParameterInstanceId(2),
            device: DeviceId(1),
            source: SourceRef {
                path: "retained".into(),
                ets_id: "Unknown".into(),
            },
            raw: "opaque".into(),
        });
    let before = malformed.clone();
    assert!(matches!(
        PreparedComparisonAdoption::prepare(&malformed, &stack, DeviceId(1), 0, changes()),
        Err(AdoptionCommandError::Command(_))
    ));
    assert_eq!(malformed, before);
    assert_eq!(stack.history_lengths(), (0, 0));
}
