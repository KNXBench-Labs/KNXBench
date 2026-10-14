use std::collections::BTreeSet;

use super::fixture::{self, CO1, D1, D2, GA1};
use super::*;
use crate::provenance::{Layer, Resolved};
use crate::GroupAddress;

fn content_of(project: &Project, device: DeviceId) -> ProgrammingContent {
    programming_content(project, &BTreeSet::from([device]))
        .remove(&device)
        .expect("device exists")
}

#[test]
fn pending_parts_are_the_four_flags_that_are_false() {
    let mut state = fixture::programmed();
    assert_eq!(state.pending_parts(), ProgrammingParts::NONE);
    state.parameters_loaded = false;
    state.communication_part_loaded = false;
    // Medium configuration has no editor and is not one of the four.
    state.medium_config_loaded = false;
    assert_eq!(
        state.pending_parts(),
        ProgrammingParts {
            parameters: true,
            group_addresses: true,
            ..ProgrammingParts::NONE
        }
    );
    assert_eq!(
        CommissioningState::default().pending_parts(),
        ProgrammingParts::ALL
    );
}

#[test]
fn a_change_returns_its_exact_undo_and_touches_only_named_parts() {
    let mut state = fixture::programmed();
    state.application_program_loaded = false;
    let original = state.clone();
    let change = ProgrammingChange::mark(
        ProgrammingParts {
            address: true,
            application: true,
            ..ProgrammingParts::NONE
        },
        false,
    );
    assert_eq!(
        change,
        ProgrammingChange {
            address: Some(false),
            application: Some(false),
            ..ProgrammingChange::default()
        }
    );
    let undo = change.apply_to(&mut state);
    assert!(!state.individual_address_loaded);
    assert!(!state.application_program_loaded);
    assert!(state.parameters_loaded && state.communication_part_loaded);
    assert_eq!(
        undo,
        ProgrammingChange {
            address: Some(true),
            application: Some(false),
            ..ProgrammingChange::default()
        }
    );
    let redo = undo.apply_to(&mut state);
    assert_eq!(state, original);
    assert_eq!(redo, change);
}

#[test]
fn marking_programmed_sets_every_named_part() {
    let mut state = CommissioningState::default();
    ProgrammingChange::mark(ProgrammingParts::ALL, true).apply_to(&mut state);
    assert_eq!(state.pending_parts(), ProgrammingParts::NONE);
    assert!(
        !state.medium_config_loaded,
        "the fifth flag stays as it was"
    );
}

#[test]
fn dropping_programmed_claims_turns_only_named_true_values_pending() {
    let change = ProgrammingChange {
        address: Some(true),
        application: Some(true),
        parameters: Some(false),
        group_addresses: None,
    };
    let only_application = ProgrammingParts {
        application: true,
        parameters: true,
        group_addresses: true,
        ..ProgrammingParts::NONE
    };
    assert_eq!(
        change.without_programmed_claims(only_application),
        ProgrammingChange {
            address: Some(true),
            application: Some(false),
            parameters: Some(false),
            group_addresses: None,
        }
    );
}

#[test]
fn identical_content_has_no_changed_part() {
    let project = fixture::project();
    let a = content_of(&project, D1);
    assert_eq!(a.changed_parts(&a.clone()), ProgrammingParts::NONE);
}

#[test]
fn each_kind_of_edit_changes_exactly_its_part() {
    let base = fixture::project();
    let before = content_of(&base, D1);
    let only = |part: fn(&mut ProgrammingParts)| {
        let mut parts = ProgrammingParts::NONE;
        part(&mut parts);
        parts
    };

    let mut p = base.clone();
    p.devices.get_mut(D1).unwrap().address = Some(IndividualAddress::new(1, 1, 9).unwrap());
    assert_eq!(
        before.changed_parts(&content_of(&p, D1)),
        only(|s| s.address = true)
    );

    let mut p = base.clone();
    p.devices.get_mut(D1).unwrap().program_ref = "H2".into();
    assert_eq!(
        before.changed_parts(&content_of(&p, D1)),
        only(|s| s.application = true)
    );

    let mut p = base.clone();
    p.devices.get_mut(D1).unwrap().product_ref = "P2".into();
    assert_eq!(
        before.changed_parts(&content_of(&p, D1)),
        only(|s| s.application = true)
    );

    let mut p = base.clone();
    p.installations[0].parameters[0].raw = "2".into();
    assert_eq!(
        before.changed_parts(&content_of(&p, D1)),
        only(|s| s.parameters = true)
    );

    let mut p = base.clone();
    p.devices.com_object_mut(CO1).unwrap().flags.read = Override::Value(Resolved {
        value: true,
        layer: Layer::UserEdit,
    });
    assert_eq!(
        before.changed_parts(&content_of(&p, D1)),
        only(|s| s.group_addresses = true)
    );

    let mut p = base.clone();
    p.devices.com_object_mut(CO1).unwrap().links.clear();
    assert_eq!(
        before.changed_parts(&content_of(&p, D1)),
        only(|s| s.group_addresses = true)
    );

    let mut p = base.clone();
    p.installations[0].group_addresses[0].address = GroupAddress::from_raw(0x0901);
    assert_eq!(
        before.changed_parts(&content_of(&p, D1)),
        only(|s| s.group_addresses = true),
        "renumbering a linked group address changes the group address table"
    );
}

#[test]
fn provenance_and_datapoint_types_are_not_content() {
    let base = fixture::project();
    let before = content_of(&base, D1);
    let mut p = base.clone();
    // Same effective value, different layer: the device holds the same bit.
    p.devices.com_object_mut(CO1).unwrap().flags.write = Override::Value(Resolved {
        value: true,
        layer: Layer::UserEdit,
    });
    p.devices.com_object_mut(CO1).unwrap().dpt = Override::Value(Resolved {
        value: crate::dpt::DptRef {
            main: 1,
            sub: Some(1),
        },
        layer: Layer::UserEdit,
    });
    p.devices.get_mut(D1).unwrap().name = "renamed".into();
    p.installations[0].group_addresses[0].name = "renamed".into();
    assert_eq!(
        before.changed_parts(&content_of(&p, D1)),
        ProgrammingParts::NONE
    );
}

#[test]
fn another_devices_parameters_do_not_belong_to_this_one() {
    let base = fixture::project();
    let before = content_of(&base, D1);
    let mut p = base.clone();
    p.installations[0].parameters[1].raw = "7".into();
    assert_eq!(
        before.changed_parts(&content_of(&p, D1)),
        ProgrammingParts::NONE
    );
    assert_ne!(
        content_of(&base, D2).changed_parts(&content_of(&p, D2)),
        ProgrammingParts::NONE
    );
}

#[test]
fn missing_devices_have_no_content() {
    let project = fixture::project();
    let contents = programming_content(&project, &BTreeSet::from([D1, DeviceId(99)]));
    assert_eq!(contents.keys().copied().collect::<Vec<_>>(), vec![D1]);
}

#[test]
fn linked_devices_are_found_through_their_objects() {
    let project = fixture::project();
    assert_eq!(devices_linked_to(&project, GA1), BTreeSet::from([D1, D2]));
    assert!(devices_linked_to(&project, fixture::GA2).is_empty());
}
