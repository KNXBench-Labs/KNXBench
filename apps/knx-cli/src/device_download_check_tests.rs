//! The CLI's regular download runner, then its read-only check, on one persistent simulator.

use std::future::Future;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use knx_app::access_key::{download_keying, KeySource};
use knx_app::commissioning_activity::{DownloadResult, OneShotLog, Restart};
use knx_app::download_report::{
    DownloadReport, ReportedPlan, ReportedProgram, REPORT_FORMAT, REPORT_KIND,
};
use knx_core::commissioning::load_control_memory::{
    abs_data_segment, abs_task_segment, event_record, AbsoluteSegment, MemoryLoadStateMachine,
    SegmentMemoryType, TaskSegment,
};
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::mutation::TargetKind;
use knx_core::commissioning::partial_memory_download::{derive_partial_plan, PartialDownloadParts};
use knx_core::commissioning::properties::{
    ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::{ApplicationService, BusError, Destination, Tpci, TunnelEvent};
use serde_json::{json, Value};

use super::*;
use MemoryLoadStateMachine::{AddressTable, ApplicationProgram, AssociationTable};

const HARDWARE: [u8; 6] = [0, 0, 0, 0, 0x01, 0x27];
const APPLICATION: [u8; 5] = [0x00, 0x83, 0x00, 0x27, 0x15];

fn fast() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(50),
        response_timeout: Duration::from_millis(50),
        answer_timeout: None,
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(40),
        programming_delay: Duration::from_millis(0),
        restart_basic_t1: Duration::from_millis(1),
        restart_responsive_again: Duration::from_millis(5),
        post_restart_disconnect_wait: Duration::from_millis(5),
        programming_mode_broadcast_timeout: Duration::from_millis(20),
    }
}

/// Every machine `Unloaded` before the transfer, so only the transfer can
/// load them. Fixture assumption, stated: the device already reports
/// `application` as its program version; the simulator does not derive it
/// from the task segment, and nothing presets it after the transfer.
fn device(application: [u8; 5], config: SimulatorConfig) -> SimulatedDevice {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0701,
        ..config
    });
    device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x83]);
    device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
    device.preset_property(3, PID_PROGRAM_VERSION, &application);
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Unloaded);
    }
    device
}

fn record(machine: MemoryLoadStateMachine, event: LoadEvent) -> MemoryDownloadStep {
    MemoryDownloadStep::LoadRecord(event_record(machine, event).expect("an event record"))
}

fn segment(start: u16, length: u16) -> AbsoluteSegment {
    AbsoluteSegment {
        start,
        length,
        access: 0xFF,
        memory_type: SegmentMemoryType::Eeprom,
        checksum_control: true,
    }
}

fn plan() -> MemoryDownloadPlan {
    MemoryDownloadPlan {
        mask: MaskVersion(0x0701),
        manufacturer: 0x0083,
        steps: vec![
            MemoryDownloadStep::Connect,
            MemoryDownloadStep::CompareProperty {
                object_index: 0,
                property_id: PID_HARDWARE_TYPE,
                inline_data: [&HARDWARE[..], &[0, 0, 0, 0]].concat(),
            },
            record(AddressTable, LoadEvent::Unload),
            record(AssociationTable, LoadEvent::Unload),
            record(ApplicationProgram, LoadEvent::Unload),
            record(AddressTable, LoadEvent::StartLoading),
            MemoryDownloadStep::LoadRecord(
                abs_data_segment(AddressTable, segment(0x4000, 4)).expect("a segment"),
            ),
            MemoryDownloadStep::WriteMemory {
                address: 0x4000,
                octets: vec![0x01, 0x11, 0x43, 0x00],
            },
            record(AddressTable, LoadEvent::LoadCompleted),
            record(ApplicationProgram, LoadEvent::StartLoading),
            MemoryDownloadStep::LoadRecord(
                abs_data_segment(ApplicationProgram, segment(0x4400, 8)).expect("a segment"),
            ),
            MemoryDownloadStep::WriteMemory {
                address: 0x4400,
                octets: vec![0xA5; 8],
            },
            MemoryDownloadStep::LoadRecord(abs_task_segment(
                ApplicationProgram,
                TaskSegment {
                    start: 0x4400,
                    pei_type: 1,
                    manufacturer: 0x0083,
                    application: 0x0027,
                    version: 0x15,
                },
            )),
            record(ApplicationProgram, LoadEvent::LoadCompleted),
            MemoryDownloadStep::Restart,
            MemoryDownloadStep::Disconnect,
        ],
    }
}

/// [`plan`] without the association table's unload: every machine it
/// touches ends `Loaded`, the one shape whose silent restart the executor
/// accepts as unconfirmed rather than failed (`closes_a_loaded_download`).
fn loaded_plan() -> MemoryDownloadPlan {
    let mut plan = plan();
    plan.steps.remove(3);
    plan
}

fn partial_plan() -> MemoryDownloadPlan {
    derive_partial_plan(
        &plan(),
        PartialDownloadParts {
            parameters: true,
            group_addresses: false,
        },
    )
    .expect("a partial plan")
    .plan
}

/// What the wrapped transport does once the transfer's `A_Restart` went out.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AfterRestart {
    /// Nothing: frames pass.
    Pass,
    /// Every frame is swallowed: a device that does not come back.
    Silent,
    /// The first `T_Data_Connected` fails to leave locally.
    FirstSendFails,
    /// The second load-state read fails to leave locally.
    SecondLoadReadFails,
    /// The check's `T_Connect` never returns: the run is dropped then.
    Hang,
}

/// The simulator behind a transport that counts what the check sends.
struct Wrapped<'a> {
    device: &'a SimulatedDevice,
    mode: AfterRestart,
    restarted: Mutex<bool>,
    numbered: Mutex<u32>,
    load_reads: Mutex<u32>,
    connects_after_restart: Mutex<u32>,
    restart_requests: Mutex<u32>,
    hung: tokio::sync::Notify,
    /// Moved from the first to the second path when the check connects.
    history_away: Option<(PathBuf, PathBuf)>,
}

impl<'a> Wrapped<'a> {
    fn new(device: &'a SimulatedDevice, mode: AfterRestart) -> Self {
        Self {
            device,
            mode,
            restarted: Mutex::new(false),
            numbered: Mutex::new(0),
            load_reads: Mutex::new(0),
            connects_after_restart: Mutex::new(0),
            restart_requests: Mutex::new(0),
            hung: tokio::sync::Notify::new(),
            history_away: None,
        }
    }

    fn connects_after_restart(&self) -> u32 {
        *self.connects_after_restart.lock().unwrap()
    }
}

impl ManagementTransport for Wrapped<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        self.device.assigned_address()
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TunnelEvent> {
        self.device.subscribe()
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let restarted = *self.restarted.lock().unwrap();
        if restarted {
            if transport == Tpci::Connect {
                *self.connects_after_restart.lock().unwrap() += 1;
                if let Some((from, to)) = &self.history_away {
                    // Durable history goes away between the transfer's record and the check's.
                    std::fs::rename(from, to).unwrap();
                }
            }
            match self.mode {
                AfterRestart::Pass => {}
                AfterRestart::Hang if transport == Tpci::Connect => {
                    self.hung.notify_one();
                    std::future::pending::<()>().await;
                }
                AfterRestart::Hang => {}
                AfterRestart::Silent => return Ok(()),
                AfterRestart::FirstSendFails => {
                    if matches!(transport, Tpci::NumberedData { .. }) {
                        let mut numbered = self.numbered.lock().unwrap();
                        *numbered += 1;
                        if *numbered == 1 {
                            return Err(BusError::Timeout);
                        }
                    }
                }
                AfterRestart::SecondLoadReadFails => {
                    if let ApplicationService::MemoryRead { address, .. } = &service {
                        if (0xB6EA..=0xB6ED).contains(address) {
                            let mut reads = self.load_reads.lock().unwrap();
                            *reads += 1;
                            if *reads == 2 {
                                return Err(BusError::Timeout);
                            }
                        }
                    }
                }
            }
        }
        if matches!(
            service,
            ApplicationService::Restart {
                response: false,
                ..
            }
        ) {
            if self.connects_after_restart() > 0 {
                // A restart after the check's own `T_Connect`.
                *self.restart_requests.lock().unwrap() += 1;
            }
            *self.restarted.lock().unwrap() = true;
        }
        self.device
            .send_frame(destination, transport, service)
            .await
    }

    fn target_kind(&self) -> TargetKind {
        self.device.target_kind()
    }
}

struct Run {
    outcome: RegularDownload,
    /// `finish_regular_download`'s exit, as `main.rs` returns it.
    exit: std::process::ExitCode,
    out: String,
    history: PathBuf,
    /// The report file `finish_regular_download` wrote, if any.
    report: Option<Value>,
    _dir: tempfile::TempDir,
}

/// A synthetic report as `prepare_report` would give it, before the run.
fn report_template() -> DownloadReport {
    DownloadReport {
        kind: REPORT_KIND,
        format: REPORT_FORMAT,
        knxbench_version: "test".into(),
        program: ReportedProgram {
            id: "M-0083_A-0027-15-0BAC".into(),
            name: None,
            application_version: None,
            source_sha256: None,
            manufacturer: "0083".into(),
            mask: "0701".into(),
        },
        scope: "complete",
        support: "untested",
        inferences: vec![],
        warnings: vec![],
        plan: ReportedPlan {
            steps: 16,
            data_octets: 12,
            group_links: 0,
        },
        outcome: None,
        post_download_check: None,
        failure: None,
        parameter_values: Default::default(),
    }
}

fn start(dir: &tempfile::TempDir, target: IndividualAddress) -> DownloadGuard {
    let log = Arc::new(OneShotLog::for_run(dir.path().join("history.sqlite")).unwrap());
    log.start_download(1, target).unwrap()
}

fn backup(dir: &std::path::Path) -> BackupTarget<'_> {
    BackupTarget {
        dir,
        application: "synthetic-application",
        partial: None,
        taken: "2026-10-25T07:00:00Z".into(),
    }
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
}

/// `main.rs`'s regular path from the tunnel on: the runner, then the
/// composition `main.rs` calls after the disconnect, with `--report`.
fn run_regular<T: ManagementTransport>(
    transport: &T,
    target: IndividualAddress,
    plan: &MemoryDownloadPlan,
) -> Run {
    run_regular_in(tempfile::tempdir().unwrap(), transport, target, plan)
}

fn run_regular_in<T: ManagementTransport>(
    dir: tempfile::TempDir,
    transport: &T,
    target: IndividualAddress,
    plan: &MemoryDownloadPlan,
) -> Run {
    let history = dir.path().join("history.sqlite");
    let mut activity = start(&dir, target);
    let report_path = dir.path().join("report.json");
    let backups = dir.path().join("backups");
    let mut out = Vec::new();
    let outcome = runtime().block_on(execute_regular_download(
        transport,
        WriteAuthorisation::for_simulator(target, WriteScope::Download).unwrap(),
        download_keying(plan.mask, None, KeySource::None),
        fast(),
        DownloadRun {
            plan,
            backup: &backup(&backups),
        },
        &mut activity,
        &mut out,
    ));
    let exit = finish_regular_download(
        &outcome,
        true,
        activity,
        Some((report_path.as_path(), &report_template())),
        &mut out,
    );
    let report = std::fs::read_to_string(&report_path)
        .ok()
        .map(|text| serde_json::from_str(&text).unwrap());
    Run {
        outcome,
        exit,
        out: String::from_utf8(out).unwrap(),
        history,
        report,
        _dir: dir,
    }
}

/// The history as a later process reads it: a fresh log on the same file.
fn reopened(run: &Run) -> Value {
    reopened_at(&run.history)
}

fn reopened_at(history: &std::path::Path) -> Value {
    let log = OneShotLog::for_run(history.to_path_buf()).unwrap();
    let (rows, _) = log.history_page(0, 10).unwrap();
    assert_eq!(rows.len(), 1);
    serde_json::to_value(&rows[0]).unwrap()
}

/// What a read-only check may send: its own connection, property and memory
/// reads and the mask read. Anything else (a write, restart, key, address or
/// function-property service, any unnamed service) is refused.
fn read_only_frame(seen: &Seen) -> bool {
    matches!(
        seen,
        Seen::Connect
            | Seen::Disconnect
            | Seen::PropertyRead { .. }
            | Seen::MemoryRead { .. }
            | Seen::Other("DeviceDescriptorRead")
    )
}

/// The check's frames: from its fresh `T_Connect` on, which must exist.
fn check_frames(device: &SimulatedDevice) -> Vec<Seen> {
    let frames = device.seen()[transfer_end(device)..].to_vec();
    assert_eq!(
        frames.first(),
        Some(&Seen::Connect),
        "the check connects afresh"
    );
    frames
}

/// The check only reads, sends no key and leaves Verify Mode off.
fn assert_check_read_only(device: &SimulatedDevice) {
    let frames = check_frames(device);
    let refused: Vec<&Seen> = frames
        .iter()
        .filter(|seen| !read_only_frame(seen))
        .collect();
    assert_eq!(refused, Vec::<&Seen>::new(), "the check only reads");
    assert!(!device.verify_mode(), "no Verify Mode");
}

fn restarts(device: &SimulatedDevice) -> usize {
    device
        .seen()
        .iter()
        .filter(|seen| {
            matches!(
                seen,
                Seen::Restart {
                    response: false,
                    ..
                }
            )
        })
        .count()
}

/// Where the transfer's frames end: its closing `T_Disconnect`.
fn transfer_end(device: &SimulatedDevice) -> usize {
    let seen = device.seen();
    let restart = seen
        .iter()
        .rposition(|seen| {
            matches!(
                seen,
                Seen::Restart {
                    response: false,
                    ..
                }
            )
        })
        .expect("the transfer restarted");
    restart
        + 1
        + seen[restart + 1..]
            .iter()
            .position(|seen| *seen == Seen::Connect)
            .expect("a connection after the transfer's restart")
}

fn check_json(run: &Run) -> Value {
    check_json_of(&run.outcome.check)
}

fn check_json_of(check: &PostDownloadCheck) -> Value {
    serde_json::to_value(check).unwrap()
}

#[test]
fn a_complete_transfer_is_followed_by_one_read_only_check_on_the_same_device() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let run = run_regular(&device, device.address(), &plan());

    assert_eq!(
        run.outcome.result,
        DownloadResult::Finished {
            written: Written::Yes,
            restart: Restart::Acknowledged
        }
    );
    assert!(run.outcome.recorded && run.outcome.check_recorded);
    assert_eq!(run.exit, ExitCode::SUCCESS);
    assert_eq!(restarts(&device), 1, "no second restart");
    assert_eq!(device.seen().last(), Some(&Seen::Disconnect));
    assert_check_read_only(&device);
    assert!(check_frames(&device).contains(&Seen::Other("DeviceDescriptorRead")));
    assert_eq!(device.authorize_requests(), 0);
    let identity = &check_json(&run)["observation"]["reach"]["identity"];
    assert_eq!(identity["result"], "match", "{}", run.out);
    assert_eq!(identity["planLoadVerdict"], "matchesPlan");
    // Transfer and restart first, then the check's own section.
    let restart_line = run.out.find("restart: acknowledged by the device").unwrap();
    let check = run.out.find("== read-only check of device").unwrap();
    assert!(restart_line < check, "{}", run.out);
    for line in [
        "reached / identity: reached; identity matches the plan; load states match the plan",
        "  address table: expected loaded, observed loaded",
        "  association table: expected unloaded, observed unloaded",
        "  application program: expected loaded, observed loaded",
        "not proven the same physical device",
        // This fixture presets no object 3, so a run-state record exists (the
        // type read is refused): no state defined, not "no read recorded".
        "application running: unknown (no defined run state observed)",
        "  run-state read: unavailable (object type read: refused)",
    ] {
        assert!(run.out.contains(line), "{line}\n{}", run.out);
    }
    assert!(
        !run.out.contains("unknown (not observed)"),
        "the legacy wording is for records without a run state: {}",
        run.out
    );
    assert_eq!(
        identity["runState"],
        json!({ "status": "unavailable", "step": "objectType", "cause": "refused" })
    );

    let row = reopened(&run);
    assert_eq!(row["state"], "finished");
    assert_eq!(row["downloadEvidence"]["written"], "yes");
    assert_eq!(row["downloadEvidence"]["restart"], "acknowledged");
    assert_eq!(row["downloadEvidence"]["cleanup"], "returnedOk");
    assert_eq!(
        row["downloadEvidence"]["postDownloadCheck"],
        check_json(&run)
    );
}

#[test]
fn a_reported_run_state_is_printed_scoped_and_survives_the_history_reopen() {
    let device = device(APPLICATION, SimulatorConfig::default());
    device.preset_property(3, 1, &[0x00, 0x03]);
    device.preset_property(3, 6, &[0x01]);
    let run = run_regular(&device, device.address(), &plan());

    assert_eq!(run.exit, ExitCode::SUCCESS);
    assert_eq!(restarts(&device), 1, "no second restart");
    assert_check_read_only(&device);
    let json = check_json(&run);
    assert_eq!(json["runtime"], "running");
    assert_eq!(
        json["observation"]["reach"]["identity"]["runState"],
        json!({ "status": "observed", "objectIndex": 3, "state": "running" })
    );
    for part in [
        "run state: running",
        "validated plan-matching Applicationprogram object 3",
        "not device health",
        "not a successful download",
    ] {
        assert!(run.out.contains(part), "{part}\n{}", run.out);
    }
    assert!(
        !run.out.contains("application running: unknown"),
        "{}",
        run.out
    );
    assert_eq!(
        reopened(&run)["downloadEvidence"]["postDownloadCheck"],
        json
    );
}

#[test]
fn a_partial_transfer_on_the_same_simulator_is_checked_against_what_it_requires() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let first = run_regular(&device, device.address(), &plan());
    assert!(first.outcome.check.is_checked());
    let before = device.seen().len();
    let run = run_regular(&device, device.address(), &partial_plan());
    assert!(matches!(
        run.outcome.result,
        DownloadResult::Finished {
            written: Written::Yes,
            ..
        }
    ));
    let identity = &check_json(&run)["observation"]["reach"]["identity"];
    assert_eq!(identity["planLoadVerdict"], "matchesPlan", "{}", run.out);
    let machines = identity["loadStates"].as_array().unwrap();
    assert!(machines.iter().any(
        |reading| reading["machine"] == "applicationProgram" && reading["expected"] == "loaded"
    ));
    assert_eq!(
        restarts(&device),
        2,
        "one restart per transfer, none for a check"
    );
    assert_check_read_only(&device);
    assert!(device.seen().len() > before);
}

#[test]
fn an_unconfirmed_restart_is_kept_and_checked_without_another_restart() {
    let device = device(
        APPLICATION,
        SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        },
    );
    let wrapped = Wrapped::new(&device, AfterRestart::Pass);
    let run = run_regular(&wrapped, device.address(), &loaded_plan());
    assert_eq!(
        run.outcome.result,
        DownloadResult::Finished {
            written: Written::Yes,
            restart: Restart::Unconfirmed
        },
        "{}",
        run.out
    );
    assert!(run.out.contains("restart: NOT confirmed"), "{}", run.out);
    assert_eq!(wrapped.connects_after_restart(), 1, "one fresh connection");
    assert_eq!(
        *wrapped.restart_requests.lock().unwrap(),
        0,
        "no restart after the check connected"
    );
    assert_eq!(
        check_json(&run)["observation"]["reach"]["result"],
        "reached"
    );
    assert_eq!(reopened(&run)["downloadEvidence"]["restart"], "unconfirmed");
    assert_eq!(device.authorize_requests(), 0);
}

#[test]
fn another_identity_after_the_restart_is_a_mismatch_and_the_transfer_stays_written() {
    let device = device([0x00, 0x83, 0x00, 0x27, 0x16], SimulatorConfig::default());
    let run = run_regular(&device, device.address(), &plan());
    assert!(matches!(
        run.outcome.result,
        DownloadResult::Finished {
            written: Written::Yes,
            ..
        }
    ));
    assert_eq!(
        check_json(&run)["observation"]["reach"]["identity"],
        json!({ "result": "mismatch", "part": { "kind": "application" }, "kind": "differs" })
    );
    assert!(run
        .out
        .contains("identity MISMATCH (application identity differs)"));
    assert!(
        run.out.contains("written to the device: yes"),
        "{}",
        run.out
    );
    assert_eq!(restarts(&device), 1);
    let row = reopened(&run);
    assert_eq!(row["state"], "finished");
    assert_eq!(row["downloadEvidence"]["written"], "yes");
}

#[test]
fn a_device_that_does_not_come_back_is_not_reached_and_nothing_is_retried() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let wrapped = Wrapped::new(&device, AfterRestart::Silent);
    let run = run_regular(&wrapped, device.address(), &plan());
    assert!(matches!(
        run.outcome.result,
        DownloadResult::Finished {
            written: Written::Yes,
            restart: Restart::Acknowledged
        }
    ));
    assert_eq!(
        check_json(&run)["observation"]["reach"],
        json!({ "result": "notReached" })
    );
    assert_eq!(wrapped.connects_after_restart(), 1, "one attempt, no retry");
    assert_eq!(restarts(&device), 1);
    assert!(run.out.contains("not reached in one read-only attempt"));
    assert_eq!(reopened(&run)["downloadEvidence"]["written"], "yes");
}

#[test]
fn a_local_send_failure_on_the_first_read_leaves_reach_unknown() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let wrapped = Wrapped::new(&device, AfterRestart::FirstSendFails);
    let run = run_regular(&wrapped, device.address(), &plan());
    assert_eq!(
        check_json(&run)["observation"]["reach"],
        json!({ "result": "unknown" })
    );
    assert_eq!(restarts(&device), 1);
    assert_check_read_only(&device);
    assert_eq!(
        reopened(&run)["downloadEvidence"]["postDownloadCheck"]["observation"]["reach"]["result"],
        "unknown"
    );
}

#[test]
fn a_later_failed_load_state_read_keeps_the_earlier_reading() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let wrapped = Wrapped::new(&device, AfterRestart::SecondLoadReadFails);
    let run = run_regular(&wrapped, device.address(), &plan());
    let identity = &check_json(&run)["observation"]["reach"]["identity"];
    assert_eq!(
        identity["loadStates"],
        json!([
            { "machine": "addressTable", "expected": "loaded",
              "observed": { "result": "observed", "loadState": "loaded" } },
            { "machine": "associationTable", "expected": "unloaded",
              "observed": { "result": "unavailable" } },
            { "machine": "applicationProgram", "expected": "loaded",
              "observed": { "result": "notRead" } },
        ])
    );
    assert_eq!(identity["planLoadVerdict"], "unknown");
    assert!(run
        .out
        .contains("  association table: expected unloaded, observed unavailable"));
    assert_eq!(
        reopened(&run)["downloadEvidence"]["postDownloadCheck"],
        check_json(&run)
    );
}

#[test]
fn a_failed_transfer_is_not_followed_by_any_connection() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let mut wrong = plan();
    wrong.manufacturer = 0x0084;
    let run = run_regular(&device, device.address(), &wrong);
    assert_eq!(
        run.outcome.result,
        DownloadResult::Failed {
            written: Written::No
        }
    );
    let connects = device
        .seen()
        .iter()
        .filter(|seen| **seen == Seen::Connect)
        .count();
    assert_eq!(connects, 1, "the transfer's own connection only");
    assert!(!run.outcome.check.is_checked());
    assert!(!run.out.contains("read-only check"), "{}", run.out);
    let row = reopened(&run);
    assert_eq!(row["state"], "failed");
    assert!(row["downloadEvidence"].get("postDownloadCheck").is_none());
}

#[test]
fn the_report_main_writes_carries_the_check_and_an_older_report_shape_has_none() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let run = run_regular(&device, device.address(), &plan());
    let json = run
        .report
        .clone()
        .expect("finish_regular_download wrote the report");
    assert_eq!(json["format"], 1, "an added optional field keeps format 1");
    assert_eq!(json["outcome"]["written"], "yes");
    assert_eq!(json["postDownloadCheck"], check_json(&run));
    assert!(run.out.contains("report: "), "{}", run.out);
    let text = json.to_string();
    assert!(!text.contains(&device.address().to_string()), "no address");
    let outcome = reported_outcome(&run.outcome.result, run.outcome.stopped_in_step).unwrap();
    let summary = report_template()
        .with_outcome(outcome)
        .with_check(run.outcome.check.clone())
        .summary();
    assert!(
        summary.contains(
            "Read-only check after the download: reached; identity matches the plan; \
             load states match the plan; not proven the same physical device"
        ),
        "{summary}"
    );
    let older: Value =
        serde_json::from_str(&report_template().with_outcome(outcome).to_json()).unwrap();
    assert!(older.get("postDownloadCheck").is_none());
    assert_eq!(older["outcome"], json["outcome"]);
}

#[test]
fn a_failed_run_reports_not_checked_and_exits_nonzero() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let mut wrong = plan();
    wrong.manufacturer = 0x0084;
    let run = run_regular(&device, device.address(), &wrong);
    assert_eq!(run.exit, ExitCode::FAILURE);
    let json = run.report.expect("a stopped run is reported");
    assert_eq!(json["outcome"]["written"], "no");
    assert_eq!(
        json["postDownloadCheck"],
        json!({ "observation": { "status": "notChecked" }, "runtime": "unknown" })
    );
}

/// Each fact `main.rs` needs for success, removed one at a time from a real run.
#[test]
fn every_unrecorded_fact_makes_the_exit_nonzero_and_the_check_verdict_does_not() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let run = run_regular(&device, device.address(), &plan());
    assert_eq!(run.exit, ExitCode::SUCCESS);
    // A guard with the transfer's result recorded, as `main.rs` hands it over.
    let finish = |outcome: &RegularDownload,
                  cleanup_ok: bool,
                  report: Option<(&std::path::Path, &DownloadReport)>| {
        let dir = tempfile::tempdir().unwrap();
        let mut activity = start(&dir, device.address());
        activity.mark_send_possible().unwrap();
        activity.record_result(&outcome.result).unwrap();
        finish_regular_download(outcome, cleanup_ok, activity, report, &mut Vec::<u8>::new())
    };
    let with = |edit: fn(&mut RegularDownload)| {
        let mut outcome = RegularDownload {
            result: run.outcome.result,
            stopped_in_step: run.outcome.stopped_in_step,
            failure: run.outcome.failure.clone(),
            recorded: run.outcome.recorded,
            check: run.outcome.check.clone(),
            check_recorded: run.outcome.check_recorded,
        };
        edit(&mut outcome);
        outcome
    };
    assert_eq!(finish(&with(|_| {}), true, None), ExitCode::SUCCESS);
    assert_eq!(
        finish(&with(|_| {}), false, None),
        ExitCode::FAILURE,
        "cleanup"
    );
    let unrecorded = with(|outcome| outcome.recorded = false);
    assert_eq!(
        finish(&unrecorded, true, None),
        ExitCode::FAILURE,
        "transfer record"
    );
    let unchecked = with(|outcome| outcome.check_recorded = false);
    assert_eq!(
        finish(&unchecked, true, None),
        ExitCode::FAILURE,
        "check record"
    );
    // A not reached device is a finding, not a failed run.
    let unreached = with(|outcome| {
        outcome.check = serde_json::from_value(
            json!({ "observation": { "status": "attempted", "reach": { "result": "notReached" } },
                "runtime": "unknown" }),
        )
        .unwrap();
    });
    assert_eq!(finish(&unreached, true, None), ExitCode::SUCCESS);
    // An unwritable report path fails the run, never the device's record.
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("absent").join("report.json");
    let exit = finish(
        &with(|_| {}),
        true,
        Some((missing.as_path(), &report_template())),
    );
    assert_eq!(exit, ExitCode::FAILURE, "report");
    // The report is written but stdout refuses its location: the run fails,
    // and the file still carries the written transfer and the check.
    struct Refusing;
    impl Write for Refusing {
        fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
            Err(std::io::Error::other("stdout closed"))
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }
    let dir = tempfile::tempdir().unwrap();
    let written = dir.path().join("report.json");
    let mut activity = start(&dir, device.address());
    activity.mark_send_possible().unwrap();
    activity.record_result(&run.outcome.result).unwrap();
    let exit = finish_regular_download(
        &with(|_| {}),
        true,
        activity,
        Some((written.as_path(), &report_template())),
        &mut Refusing,
    );
    assert_eq!(exit, ExitCode::FAILURE, "report location");
    let json: Value = serde_json::from_str(&std::fs::read_to_string(&written).unwrap()).unwrap();
    assert_eq!(json["outcome"]["written"], "yes");
    assert_eq!(json["postDownloadCheck"], check_json(&run));
}

/// The history goes away after the transfer's record: the check runs and
/// is shown, its record fails, the transfer's row stays and the exit is nonzero.
#[test]
fn a_history_that_refuses_the_check_keeps_the_transfer_and_fails_the_run() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("history.sqlite");
    let aside = dir.path().join("history-aside.sqlite");
    let mut wrapped = Wrapped::new(&device, AfterRestart::Pass);
    wrapped.history_away = Some((history.clone(), aside.clone()));
    let run = run_regular_in(dir, &wrapped, device.address(), &plan());
    assert!(run.outcome.recorded, "the transfer was recorded first");
    assert!(!run.outcome.check_recorded);
    assert!(run.outcome.check.is_checked(), "the check itself ran");
    assert!(
        run.out.contains("reached / identity: reached"),
        "{}",
        run.out
    );
    assert_eq!(run.exit, ExitCode::FAILURE);
    assert_eq!(
        run.report.unwrap()["postDownloadCheck"],
        check_json_of(&run.outcome.check),
        "the report still says what was seen"
    );
    std::fs::rename(&aside, &history).unwrap();
    let row = reopened_at(&history);
    assert_eq!(row["state"], "finished");
    assert_eq!(row["downloadEvidence"]["written"], "yes");
    assert!(row["downloadEvidence"].get("postDownloadCheck").is_none());
    assert_eq!(row["downloadEvidence"]["cleanup"], "unknown");
    assert_eq!(row["interrupted"], true);
}

/// Runs the CLI's runner until the check's connection hangs, then drops it.
fn dropped_during_the_check(wrapped: &Wrapped<'_>, dir: &tempfile::TempDir) -> DownloadGuard {
    let mut activity = start(dir, wrapped.device.address());
    let backups = dir.path().join("backups");
    let checked_plan = plan();
    let target = backup(&backups);
    let mut sink = Vec::<u8>::new();
    runtime().block_on(async {
        let mut transfer = std::pin::pin!(execute_regular_download(
            wrapped,
            WriteAuthorisation::for_simulator(wrapped.device.address(), WriteScope::Download)
                .unwrap(),
            download_keying(checked_plan.mask, None, KeySource::None),
            fast(),
            DownloadRun {
                plan: &checked_plan,
                backup: &target,
            },
            &mut activity,
            &mut sink,
        ));
        let mut hung = std::pin::pin!(wrapped.hung.notified());
        std::future::poll_fn(|cx| {
            if hung.as_mut().poll(cx).is_ready() {
                return std::task::Poll::Ready(());
            }
            assert!(transfer.as_mut().poll(cx).is_pending(), "the check hangs");
            std::task::Poll::Pending
        })
        .await;
    });
    activity
}

/// A panic unwinding `main.rs` drops the guard: the check reads interrupted.
/// A kill runs no destructor: no check result, read later as not checked.
#[test]
fn a_run_ended_during_the_check_keeps_the_written_transfer() {
    let unwound = device(APPLICATION, SimulatorConfig::default());
    let killed = device(APPLICATION, SimulatorConfig::default());
    let wrapped = Wrapped::new(&unwound, AfterRestart::Hang);
    let dir = tempfile::tempdir().unwrap();
    drop(dropped_during_the_check(&wrapped, &dir));
    assert_eq!(restarts(&unwound), 1);
    let row = reopened_at(&dir.path().join("history.sqlite"));
    assert_eq!(row["state"], "finished");
    assert_eq!(row["downloadEvidence"]["written"], "yes");
    assert_eq!(row["downloadEvidence"]["cleanup"], "unknown");
    assert_eq!(
        row["downloadEvidence"]["postDownloadCheck"],
        json!({ "observation": { "status": "interrupted" }, "runtime": "unknown" })
    );

    let wrapped = Wrapped::new(&killed, AfterRestart::Hang);
    let dir = tempfile::tempdir().unwrap();
    std::mem::forget(dropped_during_the_check(&wrapped, &dir));
    let row = reopened_at(&dir.path().join("history.sqlite"));
    assert_eq!(row["state"], "finished");
    assert_eq!(row["downloadEvidence"]["written"], "yes");
    assert!(row["downloadEvidence"].get("postDownloadCheck").is_none());
    assert_eq!(row["downloadEvidence"]["cleanup"], "unknown");
    assert_eq!(row["interrupted"], true);
}

#[test]
fn the_read_only_allowlist_refuses_every_other_service() {
    for seen in [
        Seen::Connect,
        Seen::Disconnect,
        Seen::PropertyRead {
            object_index: 0,
            property_id: 78,
        },
        Seen::Other("DeviceDescriptorRead"),
    ] {
        assert!(read_only_frame(&seen), "{seen:?}");
    }
    for seen in [
        Seen::Other("IndividualAddressWrite"),
        Seen::Other("FunctionPropertyCommand"),
        Seen::Other("DomainAddressWrite"),
        Seen::Other("Other"),
        Seen::Authorize(0),
        Seen::IndividualAddressReadBroadcast,
        Seen::MemoryWrite {
            address: 0x4000,
            data: vec![0],
            service: knx_core::commissioning::memory::MemoryService::Memory,
        },
        Seen::Restart {
            response: false,
            restart_type: 0,
            data: vec![],
        },
        Seen::PropertyWrite {
            object_index: 0,
            property_id: 78,
            data: vec![0],
        },
    ] {
        assert!(!read_only_frame(&seen), "{seen:?}");
    }
}

#[path = "device_download_failure_tests.rs"]
mod failure_tests;
