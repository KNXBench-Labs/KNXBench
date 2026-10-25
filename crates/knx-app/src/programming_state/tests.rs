//! Regression tests for programming-belief settlement and attempted parts.

use std::collections::BTreeMap;

use knx_core::commissioning::group_tables::TableLink;
use knx_core::commissioning::pending::ProgrammingParts;
use knx_core::{
    Command, CommandStack, CommissioningState, CompletionStatus, DeviceId, DeviceInstance,
    GroupAddress, IndividualAddress, Installation, InstallationId, Language, ParameterInstanceId,
    Project, SourceRef, Topology,
};
use knx_productdb::image::{FlagOverrides, ImageRequest};

use super::*;

fn ia(octet: u8) -> IndividualAddress {
    IndividualAddress::new(1, 1, octet).unwrap()
}

fn request() -> ImageRequest {
    ImageRequest {
        program_id: "M-0083_A-0001-01-0000".into(),
        individual_address: ia(5),
        values: BTreeMap::from([("P-1".into(), "1".into())]),
        links: vec![TableLink {
            object: 0,
            group_address: GroupAddress::from_raw(0x0801),
            sending: true,
        }],
        flag_overrides: BTreeMap::new(),
    }
}

fn parts(f: fn(&mut ProgrammingParts)) -> ProgrammingParts {
    let mut parts = ProgrammingParts::NONE;
    f(&mut parts);
    parts
}

const MEMORY: ProgrammingParts = ProgrammingParts {
    address: false,
    application: true,
    parameters: true,
    group_addresses: true,
};

#[test]
fn a_complete_download_writes_application_parameters_and_tables() {
    assert_eq!(written_parts(None), MEMORY);
    let both = PartialDownloadParts {
        parameters: true,
        group_addresses: true,
    };
    assert_eq!(
        written_parts(Some(both)),
        parts(|p| {
            p.parameters = true;
            p.group_addresses = true;
        })
    );
    let only_tables = PartialDownloadParts {
        parameters: false,
        group_addresses: true,
    };
    assert_eq!(
        written_parts(Some(only_tables)),
        parts(|p| p.group_addresses = true)
    );
}

#[test]
fn attempted_parts_stop_at_the_failing_step_not_the_end_of_the_plan() {
    use knx_core::commissioning::load_control_memory::{
        event_record, MemoryLoadStateMachine as Machine,
    };
    use knx_core::commissioning::load_state::{LoadEvent, MaskVersion};
    use knx_core::commissioning::memory_download::{
        MemoryDownloadPlan, MemoryDownloadStep as Step,
    };
    let load = |machine| Step::LoadRecord(event_record(machine, LoadEvent::StartLoading).unwrap());
    let plan = MemoryDownloadPlan {
        mask: MaskVersion(0x0701),
        manufacturer: 0x0083,
        steps: vec![
            Step::Connect,
            load(Machine::AddressTable),
            Step::WriteMemory {
                address: 0x4000,
                octets: vec![1],
            },
            load(Machine::AssociationTable),
            load(Machine::ApplicationProgram),
            Step::WriteMemory {
                address: 0x5000,
                octets: vec![2],
            },
            Step::Restart,
        ],
    };
    assert_eq!(
        attempted_parts(&plan, None, Some(1)),
        ProgrammingParts::NONE,
        "read-only prefix"
    );
    assert_eq!(
        attempted_parts(&plan, None, Some(2)),
        parts(|p| p.group_addresses = true),
        "a failed first table write may have arrived; the application has not been touched"
    );
    assert_eq!(
        attempted_parts(&plan, None, Some(3)),
        parts(|p| p.group_addresses = true)
    );
    assert_eq!(
        attempted_parts(&plan, None, Some(5)),
        MEMORY,
        "loading the application touches its parameters too"
    );
    assert_eq!(
        attempted_parts(&plan, None, None),
        MEMORY,
        "unknown progress is conservative"
    );
    let partial = Some(PartialDownloadParts {
        parameters: true,
        group_addresses: false,
    });
    assert_eq!(
        attempted_parts(&plan, partial, Some(6)),
        parts(|p| p.parameters = true)
    );
    let unattributed = MemoryDownloadPlan {
        steps: vec![Step::WriteMemory {
            address: 0x6000,
            octets: vec![3],
        }],
        ..plan
    };
    assert_eq!(
        attempted_parts(&unattributed, None, Some(1)),
        MEMORY,
        "never silently drop unattributed writes"
    );
}

#[test]
fn an_unchanged_request_is_current_in_every_part() {
    assert_eq!(still_current(&request(), &request()), ProgrammingParts::ALL);
}

#[test]
fn each_change_takes_its_part_and_what_depends_on_it() {
    let written = request();
    let mut now = request();
    now.individual_address = ia(6);
    assert_eq!(
        still_current(&written, &now),
        parts(|p| {
            p.application = true;
            p.parameters = true;
        }),
        "the address table carries the address"
    );

    let mut now = request();
    now.program_id = "M-0083_A-0001-02-0000".into();
    assert_eq!(
        still_current(&written, &now),
        parts(|p| p.address = true),
        "parameters and tables belong to the application"
    );

    let mut now = request();
    now.values.insert("P-1".into(), "2".into());
    assert_eq!(
        still_current(&written, &now),
        parts(|p| {
            p.address = true;
            p.application = true;
        }),
        "parameters decide which objects the tables hold"
    );

    let mut now = request();
    now.links[0].sending = false;
    assert_eq!(
        still_current(&written, &now),
        parts(|p| {
            p.address = true;
            p.application = true;
            p.parameters = true;
        })
    );

    let mut now = request();
    now.flag_overrides.insert(
        "O-1".into(),
        FlagOverrides {
            read: Some(true),
            ..FlagOverrides::default()
        },
    );
    assert_eq!(
        still_current(&written, &now),
        parts(|p| {
            p.address = true;
            p.application = true;
            p.parameters = true;
        })
    );
}

#[test]
fn a_completed_complete_download_programs_all_four_and_touches_all_four() {
    let s = settle_download(
        None,
        DownloadEnd::Completed,
        &request(),
        Some(&request()),
        SourcePlan::Unchanged,
    );
    assert_eq!(s.programmed, ProgrammingParts::ALL);
    assert_eq!(s.pending, ProgrammingParts::NONE);
    assert_eq!(s.touched, ProgrammingParts::ALL);
}

#[test]
fn a_completed_partial_download_programs_only_what_it_wrote() {
    let partial = PartialDownloadParts {
        parameters: true,
        group_addresses: false,
    };
    let s = settle_download(
        Some(partial),
        DownloadEnd::Completed,
        &request(),
        Some(&request()),
        SourcePlan::Unchanged,
    );
    let only = parts(|p| p.parameters = true);
    assert_eq!(s.programmed, only);
    assert_eq!(s.pending, ProgrammingParts::NONE);
    assert_eq!(s.touched, only);
}

#[test]
fn a_part_edited_during_the_run_stays_pending() {
    let mut now = request();
    now.values.insert("P-1".into(), "2".into());
    let s = settle_download(
        None,
        DownloadEnd::Completed,
        &request(),
        Some(&now),
        SourcePlan::Unchanged,
    );
    assert_eq!(
        s.programmed,
        parts(|p| {
            p.address = true;
            p.application = true;
        })
    );
    assert_eq!(
        s.pending,
        parts(|p| {
            p.parameters = true;
            p.group_addresses = true;
        })
    );
    assert_eq!(s.touched, ProgrammingParts::ALL);
}

#[test]
fn a_device_the_project_no_longer_describes_is_programmed_in_nothing() {
    let s = settle_download(
        None,
        DownloadEnd::Completed,
        &request(),
        None,
        SourcePlan::Unchanged,
    );
    assert_eq!(s.programmed, ProgrammingParts::NONE);
    assert_eq!(s.pending, ProgrammingParts::ALL);
}

#[test]
fn an_interrupted_download_marks_what_it_touched_even_if_it_was_clean() {
    let s = settle_download(
        None,
        DownloadEnd::Interrupted(MEMORY),
        &request(),
        Some(&request()),
        SourcePlan::Unchanged,
    );
    assert_eq!(s.programmed, ProgrammingParts::NONE);
    assert_eq!(s.pending, MEMORY, "the address was not written");
    assert_eq!(s.touched, MEMORY);
    let partial = PartialDownloadParts {
        parameters: false,
        group_addresses: true,
    };
    let s = settle_download(
        Some(partial),
        DownloadEnd::Interrupted(MEMORY),
        &request(),
        Some(&request()),
        SourcePlan::Unchanged,
    );
    assert_eq!(s.pending, parts(|p| p.group_addresses = true));
}

#[test]
fn interruption_never_marks_parts_the_plan_has_not_reached() {
    let only_tables = parts(|p| p.group_addresses = true);
    let s = settle_download(
        None,
        DownloadEnd::Interrupted(only_tables),
        &request(),
        Some(&request()),
        SourcePlan::Unchanged,
    );
    assert_eq!(s.pending, only_tables);
    assert_eq!(s.touched, only_tables);
    assert_eq!(s.programmed, ProgrammingParts::NONE);
}

#[test]
fn a_changed_source_leaves_a_complete_downloads_memory_pending_but_keeps_its_address() {
    let s = settle_download(
        None,
        DownloadEnd::Completed,
        &request(),
        Some(&request()),
        SourcePlan::ChangedOrUnknown,
    );
    assert_eq!(s.programmed, parts(|p| p.address = true));
    assert_eq!(s.pending, MEMORY);
    assert_eq!(
        s.touched,
        ProgrammingParts::ALL,
        "undo may not resurrect old claims"
    );
}

#[test]
fn a_changed_source_never_programs_a_partial_download() {
    let partial = PartialDownloadParts {
        parameters: true,
        group_addresses: true,
    };
    let s = settle_download(
        Some(partial),
        DownloadEnd::Completed,
        &request(),
        Some(&request()),
        SourcePlan::ChangedOrUnknown,
    );
    assert_eq!(s.programmed, ProgrammingParts::NONE);
    assert_eq!(
        s.pending,
        parts(|p| {
            p.parameters = true;
            p.group_addresses = true;
        })
    );
    assert_eq!(s.touched, s.pending);
}

#[test]
fn a_changed_source_and_an_edited_address_credit_nothing() {
    let mut now = request();
    now.individual_address = ia(6);
    let s = settle_download(
        None,
        DownloadEnd::Completed,
        &request(),
        Some(&now),
        SourcePlan::ChangedOrUnknown,
    );
    assert_eq!(s.programmed, ProgrammingParts::NONE);
    assert_eq!(s.pending, ProgrammingParts::ALL);
}

#[test]
fn a_changed_source_settlement_still_forgets_older_programmed_claims() {
    let mut p = project();
    let mut programmed = CommissioningState::default();
    ProgrammingChange::mark(ProgrammingParts::ALL, true).apply_to(&mut programmed);
    p.devices.get_mut(DeviceId(1)).unwrap().commissioning = programmed;
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut p,
            Command::SetParameterValue {
                id: ParameterInstanceId(1),
                device: DeviceId(1),
                ets_id: "P-1".into(),
                raw: "2".into(),
            },
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let s = settle_download(
        None,
        DownloadEnd::Completed,
        &request(),
        Some(&request()),
        SourcePlan::ChangedOrUnknown,
    );
    apply_settlement(&mut p, &mut stack, DeviceId(1), &s, None);
    stack.undo(&mut p).unwrap();
    let device = p.devices.get(DeviceId(1)).unwrap();
    assert_eq!(
        device.commissioning.pending_parts(),
        MEMORY.union(parts(|p| p.parameters = true))
    );
}

#[test]
fn a_download_that_wrote_nothing_changes_nothing() {
    assert_eq!(
        settle_download(
            None,
            DownloadEnd::NothingWritten,
            &request(),
            Some(&request()),
            SourcePlan::Unchanged
        ),
        Settlement::default()
    );
}

#[test]
fn address_programming_settles_the_address_only() {
    let address = parts(|p| p.address = true);
    let s = settle_address(AddressEnd::Programmed);
    assert_eq!(
        (s.programmed, s.pending, s.touched),
        (address, ProgrammingParts::NONE, address)
    );
    let s = settle_address(AddressEnd::Uncertain);
    assert_eq!(
        (s.programmed, s.pending, s.touched),
        (ProgrammingParts::NONE, address, address)
    );
    assert_eq!(
        settle_address(AddressEnd::NothingWritten),
        Settlement::default()
    );
}

fn device(id: u32, octet: Option<u8>, commissioning: CommissioningState) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: SourceRef {
            path: "t".into(),
            ets_id: "d".into(),
        },
        name: format!("D{id}"),
        description: None,
        address: octet.map(ia),
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning,
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

fn project() -> Project {
    let mut p = Project::new(Language("en".into()));
    p.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: (1..=4).map(DeviceId).collect(),
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    p.devices
        .insert(device(1, Some(5), CommissioningState::default()));
    p.devices
        .insert(device(2, Some(6), CommissioningState::default()));
    p.devices
        .insert(device(3, Some(6), CommissioningState::default()));
    p.devices
        .insert(device(4, None, CommissioningState::default()));
    p
}

#[test]
fn only_a_sole_device_at_an_address_is_named() {
    let p = project();
    assert_eq!(sole_device_at(&p, ia(5)), Some(DeviceId(1)));
    assert_eq!(sole_device_at(&p, ia(6)), None, "two devices share it");
    assert_eq!(sole_device_at(&p, ia(7)), None);
}

#[test]
fn applying_a_settlement_sets_belief_and_download_time() {
    let mut p = project();
    let mut stack = CommandStack::new();
    let at = DateTime::parse_from_rfc3339("2026-10-14T20:00:00Z")
        .unwrap()
        .with_timezone(&Utc);
    let s = Settlement {
        programmed: parts(|p| {
            p.address = true;
            p.application = true;
        }),
        pending: parts(|p| p.parameters = true),
        touched: ProgrammingParts::ALL,
    };
    assert!(apply_settlement(
        &mut p,
        &mut stack,
        DeviceId(1),
        &s,
        Some(at)
    ));
    let c = &p.devices.get(DeviceId(1)).unwrap().commissioning;
    assert_eq!(
        c.pending_parts(),
        parts(|p| {
            p.parameters = true;
            p.group_addresses = true;
        })
    );
    assert_eq!(c.last_download, Some(at));
    assert!(!apply_settlement(
        &mut p,
        &mut stack,
        DeviceId(9),
        &s,
        Some(at)
    ));
}

#[test]
fn applying_a_settlement_without_programmed_parts_keeps_the_download_time() {
    let mut p = project();
    let mut stack = CommandStack::new();
    let s = settle_download(
        None,
        DownloadEnd::Interrupted(MEMORY),
        &request(),
        None,
        SourcePlan::Unchanged,
    );
    let at = Utc::now();
    assert!(
        !apply_settlement(&mut p, &mut stack, DeviceId(1), &s, Some(at)),
        "already pending: nothing changes"
    );
    let state = &mut p.devices.get_mut(DeviceId(1)).unwrap().commissioning;
    ProgrammingChange::mark(ProgrammingParts::ALL, true).apply_to(state);
    assert!(apply_settlement(
        &mut p,
        &mut stack,
        DeviceId(1),
        &s,
        Some(at)
    ));
    let state = &p.devices.get(DeviceId(1)).unwrap().commissioning;
    assert_eq!(state.pending_parts(), MEMORY);
    assert_eq!(state.last_download, None);
}

#[test]
fn applying_a_settlement_forgets_older_programmed_claims() {
    let mut p = project();
    let mut programmed = CommissioningState::default();
    ProgrammingChange::mark(ProgrammingParts::ALL, true).apply_to(&mut programmed);
    p.devices.get_mut(DeviceId(1)).unwrap().commissioning = programmed;
    let mut stack = CommandStack::new();
    stack
        .do_command(
            &mut p,
            Command::SetParameterValue {
                id: ParameterInstanceId(1),
                device: DeviceId(1),
                ets_id: "P-1".into(),
                raw: "2".into(),
            },
        )
        .unwrap_or_else(|e| panic!("{e}"));
    let s = settle_download(
        None,
        DownloadEnd::Completed,
        &request(),
        Some(&request()),
        SourcePlan::Unchanged,
    );
    apply_settlement(&mut p, &mut stack, DeviceId(1), &s, None);
    stack.undo(&mut p).unwrap();
    assert_eq!(
        p.devices
            .get(DeviceId(1))
            .unwrap()
            .commissioning
            .pending_parts(),
        parts(|p| p.parameters = true)
    );
}
