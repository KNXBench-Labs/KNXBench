//! Pending-part sets and programming-content comparison regressions.

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

fn parts(
    address: bool,
    application: bool,
    parameters: bool,
    group_addresses: bool,
) -> ProgrammingParts {
    ProgrammingParts {
        address,
        application,
        parameters,
        group_addresses,
    }
}

fn set_parameter(project: &mut Project, device: DeviceId, raw: &str) {
    for parameter in project.installations[0].parameters.iter_mut() {
        if parameter.device == device {
            parameter.raw = raw.into();
        }
    }
}

fn at(seconds: i64) -> Option<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::from_timestamp(seconds, 0)
}

#[test]
fn a_restored_version_cannot_claim_values_the_device_no_longer_holds() {
    // The version was stored with parameter value 1 programmed; the device
    // was then programmed with value 2 (live, all programmed).
    let version = fixture::project();
    let mut live = fixture::project();
    set_parameter(&mut live, D1, "2");
    live.devices
        .get_mut(D1)
        .unwrap()
        .commissioning
        .last_download = at(200);
    let mut restored = version.clone();
    restored
        .devices
        .get_mut(D1)
        .unwrap()
        .commissioning
        .last_download = at(100);

    let corrected = settle_restored_version(&live, &mut restored);

    let d1 = &restored.devices.get(D1).unwrap().commissioning;
    // Parameters differ; the group-address tables depend on them.
    assert_eq!(d1.pending_parts(), parts(false, false, true, true));
    assert_eq!(d1.last_download, at(200), "the device's own download time");
    // Device 2 is unchanged and keeps its live, programmed belief.
    assert_eq!(
        restored
            .devices
            .get(D2)
            .unwrap()
            .commissioning
            .pending_parts(),
        ProgrammingParts::NONE
    );
    assert_eq!(
        corrected,
        BTreeMap::from([(D1, parts(false, false, true, true))])
    );
    // Only the flags and the download time changed, not the content.
    assert_eq!(content_of(&restored, D1), content_of(&version, D1));
}

#[test]
fn unchanged_parts_take_the_live_belief_in_both_directions() {
    // Live says pending for parts whose content equals the version's: the
    // version's programmed claim is older, so the live one wins.
    let mut version = fixture::project();
    version
        .devices
        .get_mut(D2)
        .unwrap()
        .commissioning
        .parameters_loaded = false;
    let mut live = fixture::project();
    live.devices
        .get_mut(D1)
        .unwrap()
        .commissioning
        .individual_address_loaded = false;
    let mut restored = version.clone();

    let corrected = settle_restored_version(&live, &mut restored);

    assert_eq!(
        restored
            .devices
            .get(D1)
            .unwrap()
            .commissioning
            .pending_parts(),
        parts(true, false, false, false)
    );
    // The version said pending, live says programmed for the same content.
    assert_eq!(
        restored
            .devices
            .get(D2)
            .unwrap()
            .commissioning
            .pending_parts(),
        ProgrammingParts::NONE
    );
    assert_eq!(
        corrected,
        BTreeMap::from([(D1, parts(true, false, false, false))])
    );
}

#[test]
fn an_application_or_address_change_takes_its_dependants_with_it() {
    let version = fixture::project();
    let mut live = fixture::project();
    live.devices.get_mut(D1).unwrap().program_ref = "H2".into();
    live.devices.get_mut(D2).unwrap().address = Some(IndividualAddress::new(1, 1, 9).unwrap());
    let mut restored = version.clone();

    settle_restored_version(&live, &mut restored);

    assert_eq!(
        restored
            .devices
            .get(D1)
            .unwrap()
            .commissioning
            .pending_parts(),
        parts(false, true, true, true)
    );
    assert_eq!(
        restored
            .devices
            .get(D2)
            .unwrap()
            .commissioning
            .pending_parts(),
        parts(true, false, false, true)
    );
}

#[test]
fn a_device_the_live_project_lacks_is_pending_everywhere() {
    let mut version = fixture::project();
    version
        .devices
        .get_mut(D2)
        .unwrap()
        .commissioning
        .last_download = at(100);
    let mut live = fixture::project();
    live.devices.remove(D2);
    let mut restored = version.clone();

    let corrected = settle_restored_version(&live, &mut restored);

    let d2 = &restored.devices.get(D2).unwrap().commissioning;
    assert_eq!(d2.pending_parts(), ProgrammingParts::ALL);
    assert_eq!(d2.last_download, at(100), "no later download is known");
    assert_eq!(corrected, BTreeMap::from([(D2, ProgrammingParts::ALL)]));
    // The medium flag is not one of the four parts and stays as stored.
    assert_eq!(
        d2.medium_config_loaded,
        version
            .devices
            .get(D2)
            .unwrap()
            .commissioning
            .medium_config_loaded
    );
}

#[test]
fn restoring_the_live_state_itself_changes_nothing() {
    let mut live = fixture::project();
    live.devices
        .get_mut(D1)
        .unwrap()
        .commissioning
        .parameters_loaded = false;
    let mut restored = live.clone();
    assert!(settle_restored_version(&live, &mut restored).is_empty());
    assert!(restored == live);
}
