//! P3: an edit marks the parts it changes as pending in its own undo step.

use super::{Command, CommandStack};
use crate::commissioning::pending::fixture::{self, CO1, CO2, D1, D2, GA1, GA2};
use crate::commissioning::pending::{ProgrammingChange, ProgrammingParts};
use crate::flags::{ComFlagKind, Direction};
use crate::ids::{DeviceId, ParameterInstanceId};
use crate::{GroupAddress, IndividualAddress, Project};

fn pending(project: &Project, device: DeviceId) -> ProgrammingParts {
    project
        .devices
        .get(device)
        .unwrap()
        .commissioning
        .pending_parts()
}

fn parts(f: fn(&mut ProgrammingParts)) -> ProgrammingParts {
    let mut parts = ProgrammingParts::NONE;
    f(&mut parts);
    parts
}

fn set_parameter(raw: &str) -> Command {
    Command::SetParameterValue {
        id: ParameterInstanceId(1),
        device: D1,
        ets_id: "P-1".into(),
        raw: raw.into(),
    }
}

/// Runs `cmd`, checks the marks, then that undo restores the whole project
/// and redo brings the edit back with its marks.
fn marks_then_round_trips(cmd: Command, expected: &[(DeviceId, ProgrammingParts)]) {
    let mut project = fixture::project();
    let original = project.clone();
    let mut stack = CommandStack::new();
    stack.do_command(&mut project, cmd).unwrap();
    for device in [D1, D2] {
        let want = expected
            .iter()
            .find(|(d, _)| *d == device)
            .map_or(ProgrammingParts::NONE, |(_, p)| *p);
        assert_eq!(pending(&project, device), want, "device {device:?}");
    }
    let edited = project.clone();
    assert_eq!(stack.history_lengths(), (1, 0), "one undo step");
    stack.undo(&mut project).unwrap();
    assert_eq!(project, original, "undo removes edit and marks together");
    stack.redo(&mut project).unwrap();
    assert_eq!(project, edited, "redo brings both back");
}

#[test]
fn comparison_revisions_survive_edit_undo_redo_without_following_manual_marks() {
    let mut project = fixture::project();
    let mut stack = CommandStack::new();
    let first = stack.comparison_revision(D1);
    stack.do_command(&mut project, set_parameter("2")).unwrap();
    assert!(
        stack.comparison_revision(D1) > first,
        "edit invalidates a comparison even if already pending"
    );
    assert_eq!(
        stack.comparison_revision(D2),
        0,
        "unrelated device stays current"
    );
    let edited = stack.comparison_revision(D1);
    stack.undo(&mut project).unwrap();
    assert!(
        stack.comparison_revision(D1) > edited,
        "undo does not resurrect an old bus reading"
    );
    let undone = stack.comparison_revision(D1);
    stack.redo(&mut project).unwrap();
    assert!(stack.comparison_revision(D1) > undone);
    let redone = stack.comparison_revision(D1);
    stack
        .do_command(
            &mut project,
            Command::SetDeviceProgramming {
                device: D1,
                change: ProgrammingChange::mark(ProgrammingParts::ALL, true),
            },
        )
        .unwrap();
    stack
        .do_command(
            &mut project,
            Command::RenameDevice {
                device: D1,
                name: "Label only".into(),
            },
        )
        .unwrap();
    assert_eq!(
        stack.comparison_revision(D1),
        redone,
        "belief and documentation are not device content"
    );
}

#[test]
fn a_bus_write_invalidates_old_comparison_evidence_even_without_an_edit() {
    let mut stack = CommandStack::new();
    let before = stack.comparison_revision(D1);
    stack.forget_programmed_claims(D1, ProgrammingParts::NONE);
    assert_eq!(stack.comparison_revision(D1), before);
    stack.forget_programmed_claims(D1, parts(|p| p.parameters = true));
    assert!(stack.comparison_revision(D1) > before);
    assert_eq!(stack.comparison_revision(D2), 0);
}

#[test]
fn a_parameter_edit_marks_parameters_of_that_device() {
    marks_then_round_trips(set_parameter("2"), &[(D1, parts(|p| p.parameters = true))]);
}

#[test]
fn setting_the_same_parameter_value_marks_nothing() {
    marks_then_round_trips(set_parameter("1"), &[]);
}

#[test]
fn an_address_change_marks_the_address() {
    marks_then_round_trips(
        Command::SetIndividualAddress {
            device: D2,
            address: Some(IndividualAddress::new(1, 1, 7).unwrap()),
        },
        &[(D2, parts(|p| p.address = true))],
    );
}

#[test]
fn linking_and_unlinking_mark_group_addresses() {
    marks_then_round_trips(
        Command::LinkComObject {
            com_object: CO1,
            ga: GA2,
            direction: Direction::Receive,
        },
        &[(D1, parts(|p| p.group_addresses = true))],
    );
    marks_then_round_trips(
        Command::UnlinkComObject {
            com_object: CO2,
            ga: GA1,
            direction: Direction::Receive,
        },
        &[(D2, parts(|p| p.group_addresses = true))],
    );
}

#[test]
fn an_object_flag_marks_group_addresses_but_a_datapoint_type_does_not() {
    marks_then_round_trips(
        Command::SetComObjectFlag {
            com_object: CO1,
            flag: ComFlagKind::Read,
            value: true,
        },
        &[(D1, parts(|p| p.group_addresses = true))],
    );
    marks_then_round_trips(
        Command::SetComObjectDpt {
            com_object: CO1,
            dpt: Some(crate::dpt::DptRef {
                main: 1,
                sub: Some(1),
            }),
        },
        &[],
    );
}

#[test]
fn renumbering_a_linked_group_address_marks_every_linked_device() {
    marks_then_round_trips(
        Command::ReaddressGroupAddress {
            id: GA1,
            address: GroupAddress::from_raw(0x0901),
            range: None,
        },
        &[
            (D1, parts(|p| p.group_addresses = true)),
            (D2, parts(|p| p.group_addresses = true)),
        ],
    );
}

#[test]
fn deleting_a_linked_group_address_marks_every_linked_device() {
    marks_then_round_trips(
        // The delete service unlinks first; Core correctly refuses deleting
        // a group address that is still in use. The whole batch is one edit.
        Command::Batch(vec![
            Command::UnlinkComObject {
                com_object: CO1,
                ga: GA1,
                direction: Direction::Send,
            },
            Command::UnlinkComObject {
                com_object: CO2,
                ga: GA1,
                direction: Direction::Receive,
            },
            Command::DeleteGroupAddress { id: GA1 },
        ]),
        &[
            (D1, parts(|p| p.group_addresses = true)),
            (D2, parts(|p| p.group_addresses = true)),
        ],
    );
}

#[test]
fn renaming_marks_nothing() {
    marks_then_round_trips(
        Command::RenameDevice {
            device: D1,
            name: "Kitchen".into(),
        },
        &[],
    );
}

#[test]
fn a_batch_marks_once_for_all_its_changes() {
    marks_then_round_trips(
        Command::Batch(vec![
            set_parameter("3"),
            Command::LinkComObject {
                com_object: CO1,
                ga: GA2,
                direction: Direction::Send,
            },
        ]),
        &[(
            D1,
            parts(|p| {
                p.parameters = true;
                p.group_addresses = true;
            }),
        )],
    );
}

#[test]
fn a_refused_edit_leaves_belief_and_history_untouched() {
    let mut project = fixture::project();
    let original = project.clone();
    let mut stack = CommandStack::new();
    let refused = Command::Batch(vec![
        set_parameter("2"),
        Command::LinkComObject {
            com_object: CO1,
            ga: GA1,
            direction: Direction::Send,
        },
    ]);
    assert!(stack.do_command(&mut project, refused).is_err());
    assert_eq!(project, original);
    assert_eq!(stack.history_lengths(), (0, 0));
}

#[test]
fn marking_by_hand_is_one_undo_step_and_marks_nothing_else() {
    let mut project = fixture::project();
    let original = project.clone();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut project,
            Command::SetDeviceProgramming {
                device: D1,
                change: ProgrammingChange::mark(ProgrammingParts::ALL, false),
            },
        )
        .unwrap();
    assert_eq!(pending(&project, D1), ProgrammingParts::ALL);
    assert_eq!(pending(&project, D2), ProgrammingParts::NONE);
    stack.undo(&mut project).unwrap();
    assert_eq!(project, original);
}

#[test]
fn undo_after_a_download_does_not_claim_the_old_content_is_on_the_device() {
    let mut project = fixture::project();
    let mut stack = CommandStack::new();
    stack.do_command(&mut project, set_parameter("2")).unwrap();
    // The download writes the edited value and clears the part.
    let written = ProgrammingParts::ALL;
    ProgrammingChange::mark(written, true)
        .apply_to(&mut project.devices.get_mut(D1).unwrap().commissioning);
    stack.forget_programmed_claims(D1, written);
    stack.undo(&mut project).unwrap();
    assert_eq!(
        pending(&project, D1),
        parts(|p| p.parameters = true),
        "the device holds \"2\", the project is back at \"1\""
    );
}

#[test]
fn undo_after_a_download_re_marks_a_part_that_was_pending_before_the_edit() {
    let mut project = fixture::project();
    project
        .devices
        .get_mut(D1)
        .unwrap()
        .commissioning
        .parameters_loaded = false;
    let mut stack = CommandStack::new();
    stack.do_command(&mut project, set_parameter("2")).unwrap();
    ProgrammingChange::mark(ProgrammingParts::ALL, true)
        .apply_to(&mut project.devices.get_mut(D1).unwrap().commissioning);
    stack.forget_programmed_claims(D1, ProgrammingParts::ALL);
    stack.undo(&mut project).unwrap();
    assert_eq!(pending(&project, D1), parts(|p| p.parameters = true));
}

#[test]
fn forgetting_claims_spares_other_devices_and_unwritten_parts() {
    let mut project = fixture::project();
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut project,
            Command::ReaddressGroupAddress {
                id: GA1,
                address: GroupAddress::from_raw(0x0901),
                range: None,
            },
        )
        .unwrap();
    // A partial download of D1's parameters only: D1's group address claim
    // and D2's claims stay restorable.
    stack.forget_programmed_claims(D1, parts(|p| p.parameters = true));
    stack.undo(&mut project).unwrap();
    assert_eq!(pending(&project, D1), ProgrammingParts::NONE);
    assert_eq!(pending(&project, D2), ProgrammingParts::NONE);
}

#[test]
fn forgetting_claims_reaches_recovered_snapshot_history() {
    let older = fixture::project();
    let mut project = older.clone();
    project.installations[0].parameters[0].raw = "2".into();
    project
        .devices
        .get_mut(D1)
        .unwrap()
        .commissioning
        .parameters_loaded = false;
    let mut stack = CommandStack::from_snapshot_states(vec![older], vec![]);
    stack.forget_programmed_claims(D1, parts(|p| p.parameters = true));
    stack.undo(&mut project).unwrap();
    assert_eq!(project.installations[0].parameters[0].raw, "1");
    assert_eq!(pending(&project, D1), parts(|p| p.parameters = true));
    assert_eq!(pending(&project, D2), ProgrammingParts::NONE);
}
