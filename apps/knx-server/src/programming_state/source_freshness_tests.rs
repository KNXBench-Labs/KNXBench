//! A finished download credits memory only while the product source still plans the same bytes.

use std::path::Path;
use std::sync::Arc;

use knx_app::device_download::{prepare_device_download, PreparedDownload};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::commissioning::pending::{ProgrammingChange, ProgrammingParts};
use knx_core::{DeviceId, DeviceInstance, IndividualAddress, Language, Project, SourceRef};

use super::download_settler;
use crate::device_download::{DownloadStatus, Restart, Written};
use crate::{AppState, CatalogueOwner, SharedState};

const PROGRAM_ID: &str = "M-0001_A-0001-01-0000";
const HARDWARE_TO_PROGRAM: &str = "M-0001_H-1_HP-1";
/// Parameter-segment base image of the synthetic program, as base64.
const BASE_OLD: &str = "AgAAB0BMAAdBbAeqAAu4AA==";
/// The same segment with octets 8 and 9 replaced by the sentinels C3 3C.
const BASE_NEW: &str = "AgAAB0BMAAfDPAeqAAu4AA==";

/// A synthetic, publicly distributable mask-0701 program.
#[derive(Debug)]
struct Source {
    /// `AS-4400` data, base64.
    base: &'static str,
    name: &'static str,
    style: &'static str,
    /// `Access` of the application segment's load control.
    access: u8,
    /// `InlineData` of the mask-version check that opens the load procedure.
    check: &'static str,
    /// `Address` of the application segment's task segment.
    task: u32,
    /// Extra `Static` content, such as a download plug-in declaration.
    extension: &'static str,
}

const OLD: Source = Source {
    base: BASE_OLD,
    name: "P",
    style: "ProductProcedure",
    access: 255,
    check: "000000000127",
    task: 17408,
    extension: "",
};

fn program_xml(
    Source {
        base,
        name,
        style,
        access,
        check,
        task,
        extension,
    }: &Source,
) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="{PROGRAM_ID}" Name="{name}" ApplicationNumber="1"
  ApplicationVersion="1" PeiType="0" MaskVersion="MV-0701" LoadProcedureStyle="{style}"><Static>
<Code>
  <AbsoluteSegment Id="AS-4000" Address="16384" Size="8"><Data>AwAAGQAZAQA=</Data><Mask>/wAA//////8=</Mask></AbsoluteSegment>
  <AbsoluteSegment Id="AS-4100" Address="16640" Size="6"><Data>AgEAAgUA</Data></AbsoluteSegment>
  <AbsoluteSegment Id="AS-4400" Address="17408" Size="16"><Data>{base}</Data></AbsoluteSegment>
</Code>
<ParameterTypes>
  <ParameterType Id="PT-N" Name="N"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="100" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="{PROGRAM_ID}_P-2" Name="Time" ParameterType="PT-N" Value="50"><Memory CodeSegment="AS-4400" Offset="12" BitOffset="0" /></Parameter>
</Parameters>
<ParameterRefs><ParameterRef Id="{PROGRAM_ID}_P-2_R-2" RefId="{PROGRAM_ID}_P-2" /></ParameterRefs>
<ComObjectTable CodeSegment="AS-4400" Offset="0">
  <ComObject Id="{PROGRAM_ID}_O-0" Name="Switch" Number="0" ObjectSize="1 Bit" ReadFlag="Enabled" WriteFlag="Disabled"
    CommunicationFlag="Enabled" TransmitFlag="Enabled" UpdateFlag="Disabled" ReadOnInitFlag="Disabled" />
</ComObjectTable>
<ComObjectRefs><ComObjectRef Id="{PROGRAM_ID}_O-0_R-1" RefId="{PROGRAM_ID}_O-0" /></ComObjectRefs>
<AddressTable CodeSegment="AS-4000" Offset="0" MaxEntries="4" />
<AssociationTable CodeSegment="AS-4100" Offset="0" MaxEntries="2" />
<LoadProcedures><LoadProcedure>
  <LdCtrlConnect />
  <LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="{check}" />
  <LdCtrlUnload LsmIdx="1" /><LdCtrlUnload LsmIdx="2" /><LdCtrlUnload LsmIdx="3" />
  <LdCtrlLoad LsmIdx="1" />
  <LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="16384" Size="8" Access="255" MemType="3" SegFlags="128" />
  <LdCtrlTaskSegment LsmIdx="1" Address="16384" />
  <LdCtrlLoadCompleted LsmIdx="1" />
  <LdCtrlLoad LsmIdx="2" />
  <LdCtrlAbsSegment LsmIdx="2" SegType="0" Address="16640" Size="6" Access="255" MemType="3" SegFlags="128" />
  <LdCtrlTaskSegment LsmIdx="2" Address="16640" />
  <LdCtrlLoadCompleted LsmIdx="2" />
  <LdCtrlLoad LsmIdx="3" />
  <LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="17408" Size="16" Access="{access}" MemType="3" SegFlags="128" />
  <LdCtrlTaskSegment LsmIdx="3" Address="{task}" />
  <LdCtrlLoadCompleted LsmIdx="3" />
  <LdCtrlRestart /><LdCtrlDisconnect />
</LoadProcedure></LoadProcedures>
{extension}
</Static>
<Dynamic><Channel Id="CH" Text="C"><ParameterBlock Id="PB" Text="B">
  <ParameterRefRef RefId="{PROGRAM_ID}_P-2_R-2" /><ComObjectRefRef RefId="{PROGRAM_ID}_O-0_R-1" />
</ParameterBlock></Channel></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

/// A product database holding `xml` as program `PROGRAM_ID`, reachable from a project device.
fn catalogue_file(path: &Path, xml: &str) -> knx_productdb::Connection {
    let conn = knx_productdb::open_and_migrate(path).expect("database");
    knx_productdb::ingest_file(&conn, "M-0001/A.xml", xml.as_bytes()).expect("ingests");
    conn.execute_batch(&format!(
        "PRAGMA foreign_keys = OFF;
         INSERT INTO hardware2program (id, manufacturer_id, hardware_id, application_program_ref, source_sha256)
         VALUES ('{HARDWARE_TO_PROGRAM}', 'M-0001', 'M-0001_H-1', '{PROGRAM_ID}', 'fixture');"
    ))
    .expect("hardware2program row");
    conn
}

fn address() -> IndividualAddress {
    "1.1.67".parse().unwrap()
}

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "test".into(),
            ets_id: "DEV-1".into(),
        },
        name: "Synthetic".into(),
        description: None,
        address: Some(address()),
        product_ref: String::new(),
        program_ref: HARDWARE_TO_PROGRAM.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

fn finished() -> DownloadStatus {
    DownloadStatus::Finished {
        written: Written::Yes,
        restart: Restart::Acknowledged,
        restart_note: None,
        post_download_check: knx_app::post_download_check::PostDownloadCheck::not_checked(),
    }
}

fn prepare(state: &SharedState) -> PreparedDownload {
    let project = state.project.lock().unwrap();
    state
        .product_db
        .as_ref()
        .unwrap()
        .with_read(|conn| prepare_device_download(conn, project.as_ref().unwrap(), address()))
        .expect("owner readable")
        .expect("download prepares")
}

fn belief(state: &SharedState) -> ProgrammingParts {
    let project = state.project.lock().unwrap();
    let commissioning = &project
        .as_ref()
        .unwrap()
        .devices
        .get(DeviceId(1))
        .unwrap()
        .commissioning;
    ProgrammingParts {
        address: commissioning.individual_address_loaded,
        application: commissioning.application_program_loaded,
        parameters: commissioning.parameters_loaded,
        group_addresses: commissioning.communication_part_loaded,
    }
}

/// Server running catalogue `old`; the backup `new` is restored over it later.
struct Fixture {
    state: SharedState,
    live: std::path::PathBuf,
    backup: std::path::PathBuf,
    _dir: tempfile::TempDir,
}

fn fixture(old: &Source, new: &Source) -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let live = dir.path().join("products.sqlite");
    let backup = dir.path().join("backup.sqlite");
    drop(catalogue_file(&backup, &program_xml(new)));
    let conn = catalogue_file(&live, &program_xml(old));
    let state = Arc::new(AppState {
        product_db: Some(CatalogueOwner::adopt_opened(conn, live.clone()).unwrap()),
        ..AppState::default()
    });
    *state.project.lock().unwrap() = Some(project());
    Fixture {
        state,
        live,
        backup,
        _dir: dir,
    }
}

/// A catalogue backup restore is the owner's public way to replace the consumed source.
fn restore(fixture: &Fixture) {
    fixture
        .state
        .product_db
        .as_ref()
        .unwrap()
        .restore_legacy(&fixture.live, &fixture.backup)
        .expect("backup restores");
}

const NONE: ProgrammingParts = ProgrammingParts::NONE;
const MEMORY: ProgrammingParts = ProgrammingParts {
    address: false,
    application: true,
    parameters: true,
    group_addresses: true,
};
const PARAMETERS: ProgrammingParts = ProgrammingParts {
    parameters: true,
    ..NONE
};
const GROUP_ADDRESSES: ProgrammingParts = ProgrammingParts {
    group_addresses: true,
    ..NONE
};
const ADDRESS: ProgrammingParts = ProgrammingParts {
    address: true,
    ..NONE
};

fn mark(state: &SharedState, parts: ProgrammingParts, programmed: bool) {
    let mut project = state.project.lock().unwrap();
    let device = project
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(1))
        .unwrap();
    ProgrammingChange::mark(parts, programmed).apply_to(&mut device.commissioning);
}

fn edit_address(state: &SharedState) {
    let mut project = state.project.lock().unwrap();
    project
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .address = Some("1.1.68".parse().unwrap());
}

/// Runs the download prepared before the restore to the given end, through the real callback.
fn run(f: &Fixture, scope: Option<PartialDownloadParts>, replace: bool, status: DownloadStatus) {
    let mut prepared = prepare(&f.state);
    if let Some(parts) = scope {
        prepared = prepared.into_partial(parts).expect("partial plan");
    }
    let settler = download_settler(&f.state, super::incarnation(&f.state), &prepared);
    if replace {
        restore(f);
    }
    settler(&status);
}

fn full(f: &Fixture, replace: bool) -> ProgrammingParts {
    run(f, None, replace, finished());
    belief(&f.state)
}

fn partial(
    f: &Fixture,
    parameters: bool,
    group_addresses: bool,
    replace: bool,
) -> ProgrammingParts {
    let parts = PartialDownloadParts {
        parameters,
        group_addresses,
    };
    run(f, Some(parts), replace, finished());
    belief(&f.state)
}

fn new_base() -> Source {
    Source {
        base: BASE_NEW,
        ..OLD
    }
}

fn failed(written: Written) -> DownloadStatus {
    DownloadStatus::Failed {
        written,
        stopped_in_step: None,
        error: "lost".into(),
        hint: None,
    }
}

#[test]
fn a_replaced_product_source_with_the_same_request_credits_no_memory_part() {
    let f = fixture(&OLD, &new_base());
    let prepared = prepare(&f.state);
    let settler = download_settler(&f.state, super::incarnation(&f.state), &prepared);

    restore(&f);
    let replaced = prepare(&f.state);
    // Preconditions of the scenario: identical request, genuinely different plan.
    assert_eq!(replaced.request, prepared.request);
    assert_ne!(replaced.plan, prepared.plan);

    settler(&finished());
    let after = belief(&f.state);
    assert!(
        !after.application && !after.parameters && !after.group_addresses,
        "old plan credited against the replaced source: {after:?}"
    );
    assert_eq!(after, ADDRESS, "the reached address is still confirmed");
}

#[test]
fn an_unchanged_source_credits_the_full_download_and_each_partial_scope() {
    let f = fixture(&OLD, &OLD);
    assert_eq!(full(&f, false), ProgrammingParts::ALL);
    for (parameters, group_addresses, expected) in [
        (true, false, PARAMETERS),
        (false, true, GROUP_ADDRESSES),
        (
            true,
            true,
            ProgrammingParts {
                parameters: true,
                group_addresses: true,
                ..NONE
            },
        ),
    ] {
        let f = fixture(&OLD, &OLD);
        assert_eq!(partial(&f, parameters, group_addresses, false), expected);
    }
}

#[test]
fn a_restore_of_the_same_source_keeps_the_credit() {
    let f = fixture(&OLD, &OLD);
    assert_eq!(full(&f, true), ProgrammingParts::ALL);
}

#[test]
fn a_changed_parameter_segment_denies_every_scope_that_wrote_it() {
    let f = fixture(&OLD, &new_base());
    assert_eq!(full(&f, true), ADDRESS);
    let f = fixture(&OLD, &new_base());
    assert_eq!(partial(&f, true, false, true), NONE);
    let f = fixture(&OLD, &new_base());
    assert_eq!(partial(&f, true, true, true), NONE);
    // The group address tables plan the same bytes: their scope keeps its credit.
    let f = fixture(&OLD, &new_base());
    assert_eq!(partial(&f, false, true, true), GROUP_ADDRESSES);
}

#[test]
fn a_changed_load_control_with_identical_octets_is_not_the_same_plan() {
    let changed = Source { access: 254, ..OLD };
    let f = fixture(&OLD, &changed);
    let before = prepare(&f.state);
    restore(&f);
    let after = prepare(&f.state);
    assert_eq!(after.request, before.request);
    assert_eq!(after.image.segments, before.image.segments, "same octets");
    assert_ne!(after.plan, before.plan, "only a load control differs");
    let f = fixture(&OLD, &changed);
    assert_eq!(full(&f, true), ADDRESS);
}

#[test]
fn a_source_the_download_now_refuses_leaves_memory_pending_and_the_transfer_result_alone() {
    let refused = Source {
        style: "MergedProcedure",
        ..OLD
    };
    let f = fixture(&OLD, &refused);
    assert_eq!(full(&f, true), ADDRESS);
}

#[test]
fn a_program_the_catalogue_lost_credits_nothing() {
    let f = fixture(&OLD, &OLD);
    let prepared = prepare(&f.state);
    let settler = download_settler(&f.state, super::incarnation(&f.state), &prepared);
    // The restored backup is a catalogue that holds no such program at all.
    std::fs::remove_file(&f.backup).unwrap();
    drop(knx_productdb::open_and_migrate(&f.backup).unwrap());
    restore(&f);
    settler(&finished());
    assert_eq!(belief(&f.state), NONE);
}

#[test]
fn unrelated_program_metadata_with_the_same_effective_plan_keeps_the_credit() {
    let renamed = Source {
        name: "Renamed",
        ..OLD
    };
    let f = fixture(&OLD, &renamed);
    assert_eq!(full(&f, true), ProgrammingParts::ALL);
}

#[test]
fn an_ordinary_address_edit_during_an_unchanged_source_run_keeps_its_selective_outcome() {
    let f = fixture(&OLD, &OLD);
    let prepared = prepare(&f.state);
    let settler = download_settler(&f.state, super::incarnation(&f.state), &prepared);
    edit_address(&f.state);
    settler(&finished());
    assert_eq!(
        belief(&f.state),
        ProgrammingParts {
            application: true,
            parameters: true,
            ..NONE
        }
    );
}

#[test]
fn a_source_change_and_a_project_edit_together_cannot_mark_memory_current() {
    let f = fixture(&OLD, &new_base());
    let prepared = prepare(&f.state);
    let settler = download_settler(&f.state, super::incarnation(&f.state), &prepared);
    edit_address(&f.state);
    restore(&f);
    settler(&finished());
    assert_eq!(belief(&f.state), NONE);
}

#[test]
fn a_run_that_wrote_nothing_leaves_every_flag_even_after_a_source_change() {
    let f = fixture(&OLD, &new_base());
    mark(&f.state, ProgrammingParts::ALL, true);
    run(&f, None, true, failed(Written::No));
    assert_eq!(belief(&f.state), ProgrammingParts::ALL);
}

#[test]
fn an_interrupted_run_still_marks_what_it_may_have_written_after_a_source_change() {
    let f = fixture(&OLD, &new_base());
    mark(&f.state, ProgrammingParts::ALL, true);
    run(&f, None, true, failed(Written::Partially));
    assert_eq!(belief(&f.state), ADDRESS);
}

#[test]
fn another_project_open_after_the_start_gets_no_claim_even_from_an_unchanged_source() {
    let f = fixture(&OLD, &OLD);
    let prepared = prepare(&f.state);
    let settler = download_settler(&f.state, super::incarnation(&f.state), &prepared);
    f.state
        .project_incarnation
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    settler(&finished());
    assert_eq!(belief(&f.state), NONE);
}

#[test]
fn a_source_change_does_not_touch_unrelated_programmed_flags() {
    // A partial run of the parameters under a changed source: the address and
    // tables the run never wrote keep what they held.
    let f = fixture(&OLD, &new_base());
    mark(&f.state, ADDRESS.union(GROUP_ADDRESSES), true);
    assert_eq!(
        partial(&f, true, false, true),
        ADDRESS.union(GROUP_ADDRESSES)
    );
}

mod admission;
mod owner_health;
mod persistence;
mod project_edits;

#[test]
fn the_memory_constant_matches_the_parts_a_full_run_writes() {
    assert_eq!(knx_app::programming_state::written_parts(None), MEMORY);
}
