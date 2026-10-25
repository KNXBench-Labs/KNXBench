//! Real parameter and link edits during a run leave source freshness and per-part settlement alone.

use knx_core::{
    ComObjectInstance, ComObjectInstanceId, Command, CompletionStatus, Direction, GroupAddress,
    GroupAddressEntry, GroupAddressId, GroupLink, Installation, InstallationId, Override,
    ParameterInstance, ParameterInstanceId, ResolvedFlags,
};

use super::*;
use crate::domain::{self, SnapshotBinding};

/// The `Time` parameter's shipped value and the user's edit of it.
pub(super) const TIME_ORIGINAL: &str = "50";
pub(super) const TIME_EDITED: &str = "77";
const TIME_ID: &str = "M-0001_A-0001-01-0000_P-2_R-2";

fn source_ref(ets_id: &str) -> SourceRef {
    SourceRef {
        path: "test".into(),
        ets_id: ets_id.into(),
    }
}

fn group_address(id: u32, raw: u16) -> GroupAddressEntry {
    GroupAddressEntry {
        id: GroupAddressId(id),
        source: source_ref(&format!("GA-{id}")),
        name: format!("Group {id}"),
        address: GroupAddress::from_raw(raw),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Default::default(),
    }
}

/// The synthetic device with one parameter, one object sending to a first
/// group address and a second, still unlinked group address.
pub(super) fn rich_project() -> Project {
    let mut project = project();
    project.info.project_id = "P-0001".into();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: knx_core::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![group_address(1, 0x1035), group_address(2, 0x1236)],
        parameters: vec![ParameterInstance {
            id: ParameterInstanceId(1),
            device: DeviceId(1),
            source: source_ref(TIME_ID),
            raw: TIME_ORIGINAL.into(),
        }],
    });
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: source_ref(&format!("{PROGRAM_ID}_O-0_R-1")),
        device: DeviceId(1),
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        }],
        module_instance: None,
    });
    project
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .com_objects
        .push(ComObjectInstanceId(1));
    // The id counters name what is in use, as a saved native file requires.
    let ids = &mut project.ids;
    ids.next_device_id().unwrap();
    ids.next_com_object_instance_id().unwrap();
    ids.next_group_address_id().unwrap();
    ids.next_group_address_id().unwrap();
    ids.next_parameter_instance_id().unwrap();
    project
}

pub(super) fn install_rich_project(state: &SharedState) {
    *state.project.lock().unwrap() = Some(rich_project());
}

/// Runs `command` as the undoable, persisted project command a client's write is.
pub(super) fn command(state: &SharedState, command: Command) {
    let binding = SnapshotBinding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: super::super::incarnation(state),
        snapshot_revision: domain::current_project_revision(state),
    };
    domain::apply_bound_prepared(state, &binding, "Test", command).expect("command applies");
}

pub(super) fn edit_time(state: &SharedState) {
    command(
        state,
        Command::SetParameterValue {
            id: ParameterInstanceId(1),
            device: DeviceId(1),
            ets_id: TIME_ID.into(),
            raw: TIME_EDITED.into(),
        },
    );
}

pub(super) fn edit_link(state: &SharedState) {
    command(
        state,
        Command::LinkComObject {
            com_object: ComObjectInstanceId(1),
            ga: GroupAddressId(2),
            direction: Direction::Receive,
        },
    );
}

/// A run started on the unedited project, edited, then settled as finished.
fn edited_run(f: &Fixture, edit: fn(&SharedState), replace: bool) -> ProgrammingParts {
    install_rich_project(&f.state);
    let prepared = prepare(&f.state);
    let settler = download_settler(&f.state, super::super::incarnation(&f.state), &prepared);
    edit(&f.state);
    // Preconditions: the edit really changes what the project would write now.
    assert_ne!(prepare(&f.state).request, prepared.request);
    if replace {
        restore(f);
    }
    settler(&finished());
    belief(&f.state)
}

#[test]
fn a_parameter_edit_over_an_unchanged_source_keeps_application_and_address() {
    let f = fixture(&OLD, &OLD);
    // The application is current, the edited parameter and the tables built
    // from it are not: the source check rebuilt the original request, not
    // the edited one, or the application would be denied too.
    assert_eq!(
        edited_run(&f, edit_time, false),
        ProgrammingParts {
            application: true,
            ..ADDRESS
        }
    );
}

#[test]
fn a_link_edit_over_an_unchanged_source_keeps_the_parameters_too() {
    let f = fixture(&OLD, &OLD);
    assert_eq!(
        edited_run(&f, edit_link, false),
        ProgrammingParts {
            application: true,
            parameters: true,
            ..ADDRESS
        }
    );
}

#[test]
fn a_source_change_together_with_a_parameter_edit_credits_only_the_address() {
    let f = fixture(&OLD, &new_base());
    assert_eq!(edited_run(&f, edit_time, true), ADDRESS);
}

#[test]
fn a_source_change_together_with_a_link_edit_credits_only_the_address() {
    let f = fixture(&OLD, &new_base());
    assert_eq!(edited_run(&f, edit_link, true), ADDRESS);
}
