//! The server download worker's read-only check after a transfer, on one persistent simulator.

use super::*;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use knx_app::access_key::{download_keying, KeySource};
use knx_app::commissioning_activity::OneShotLog;
use knx_core::commissioning::load_control_memory::{
    abs_data_segment, abs_task_segment, event_record, AbsoluteSegment, MemoryLoadStateMachine,
    SegmentMemoryType, TaskSegment,
};
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::memory_download::MemoryDownloadPlan;
use knx_core::commissioning::mutation::WriteScope;
use knx_core::commissioning::properties::{
    ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::{BusError, Destination, ManagementTransport, Tpci};
use knx_productdb::code::ProgramCode;
use knx_productdb::image::{DownloadImage, ImageRequest};
use serde_json::{json, Value};

use crate::bus::BusSessionError;
use MemoryLoadStateMachine::{AddressTable, ApplicationProgram, AssociationTable};

const HARDWARE: [u8; 6] = [0, 0, 0, 0, 0x01, 0x27];
const APPLICATION: [u8; 5] = [0x00, 0x83, 0x00, 0x27, 0x15];

fn fast() -> knx_net::SessionTiming {
    knx_net::SessionTiming {
        connection_timeout: Duration::from_millis(50),
        response_timeout: Duration::from_millis(50),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(40),
        programming_delay: Duration::from_millis(0),
        restart_basic_t1: Duration::from_millis(1),
        restart_responsive_again: Duration::from_millis(5),
        post_restart_disconnect_wait: Duration::from_millis(5),
        programming_mode_broadcast_timeout: Duration::from_millis(20),
    }
}

/// Every machine `Unloaded` before the transfer. Fixture assumption, stated:
/// the device already reports `application` as its program version; the
/// simulator does not derive it from the task segment, and nothing presets
/// it after the transfer.
fn device(application: [u8; 5], config: SimulatorConfig) -> Arc<SimulatedDevice> {
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
    Arc::new(device)
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

fn prepared(target: IndividualAddress, plan: MemoryDownloadPlan) -> PreparedDownload {
    PreparedDownload {
        device: knx_core::DeviceId(1),
        device_name: "synthetic device".into(),
        target,
        request: ImageRequest {
            program_id: "M-0083_A-0027-15-0BAC".into(),
            individual_address: target,
            values: Default::default(),
            links: vec![],
            flag_overrides: Default::default(),
        },
        image: DownloadImage {
            code: ProgramCode::default(),
            segments: vec![],
            parameters: Default::default(),
            texts: Default::default(),
            objects: vec![],
            inferences: vec![],
            parameter_fields: vec![],
            table_writes: vec![],
        },
        plan,
        inferences: vec![],
        warnings: vec![],
        partial: None,
    }
}

/// What the tunnel does once the transfer's `A_Restart` went out.
#[derive(Clone, Copy, PartialEq, Eq)]
enum AfterRestart {
    Pass,
    /// Every frame swallowed: a device that does not come back.
    Silent,
    /// The check's first frame never returns: the worker is stopped then.
    Hang,
}

/// The server's tunnel over one persistent simulated device.
struct SimTunnel {
    device: Arc<SimulatedDevice>,
    mode: AfterRestart,
    restarted: Arc<std::sync::atomic::AtomicBool>,
    /// `A_Restart` frames (any repetition) sent after the check's `T_Connect`.
    restart_requests: Arc<std::sync::atomic::AtomicUsize>,
    check_connected: std::sync::atomic::AtomicBool,
    hung: Arc<tokio::sync::Notify>,
}

impl BusTunnel for SimTunnel {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device.as_ref())
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device.as_ref())
    }

    fn send(
        &self,
        _destination: Destination,
        _service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        panic!("a download sends no group telegram")
    }

    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        use std::sync::atomic::Ordering;
        Box::pin(async move {
            if self.restarted.load(Ordering::SeqCst) && transport == Tpci::Connect {
                self.check_connected.store(true, Ordering::SeqCst);
                match self.mode {
                    AfterRestart::Pass => {}
                    AfterRestart::Silent => return Ok(()),
                    AfterRestart::Hang => {
                        self.hung.notify_one();
                        std::future::pending::<()>().await;
                    }
                }
            }
            if self.restarted.load(Ordering::SeqCst) && self.mode == AfterRestart::Silent {
                return Ok(());
            }
            if matches!(
                service,
                ApplicationService::Restart {
                    response: false,
                    ..
                }
            ) {
                if self.check_connected.load(Ordering::SeqCst) {
                    self.restart_requests.fetch_add(1, Ordering::SeqCst);
                }
                self.restarted.store(true, Ordering::SeqCst);
            }
            ManagementTransport::send_frame(self.device.as_ref(), destination, transport, service)
                .await
        })
    }

    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async { Ok(()) })
    }
}

struct Started {
    session: DeviceDownloadSession,
    history: std::path::PathBuf,
    settled: Arc<Mutex<Vec<DownloadStatus>>>,
    hung: Arc<tokio::sync::Notify>,
    restart_requests: Arc<std::sync::atomic::AtomicUsize>,
    _dir: tempfile::TempDir,
}

fn start(device: &Arc<SimulatedDevice>, plan: MemoryDownloadPlan, mode: AfterRestart) -> Started {
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("history.sqlite");
    let log = Arc::new(OneShotLog::for_run(history.clone()).unwrap());
    let target = device.address();
    let activity = log.start_download(7, target).unwrap();
    let settled = Arc::new(Mutex::new(Vec::new()));
    let settled_by_end = Arc::clone(&settled);
    let hung = Arc::new(tokio::sync::Notify::new());
    let restart_requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mask = plan.mask;
    let session = DeviceDownloadSession::start(
        7,
        Box::new(SimTunnel {
            device: Arc::clone(device),
            mode,
            restarted: Arc::default(),
            restart_requests: Arc::clone(&restart_requests),
            check_connected: Default::default(),
            hung: Arc::clone(&hung),
        }),
        // The server's tunnel adapter is hardware-kind, as in the HTTP
        // tests: a simulator authorisation is refused there by design.
        WriteAuthorisation::for_hardware(
            target,
            WriteScope::Download,
            &knx_core::commissioning::mutation::required_confirmation_phrase(
                target,
                WriteScope::Download,
            ),
        )
        .unwrap(),
        download_keying(mask, None, KeySource::None),
        fast(),
        prepared(target, plan),
        BackupDestination {
            dir: dir.path().join("backups"),
            taken: "2026-10-25T07:00:00+01:00".into(),
        },
        activity,
        task_tests::synthetic_report(),
        Box::new(move |status| settled_by_end.lock().unwrap().push(status.clone())),
    );
    Started {
        session,
        history,
        settled,
        hung,
        restart_requests,
        _dir: dir,
    }
}

fn reopened(history: &std::path::Path) -> Value {
    let log = OneShotLog::for_run(history.to_path_buf()).unwrap();
    let (rows, _) = log.history_page(0, 10).unwrap();
    assert_eq!(rows.len(), 1);
    serde_json::to_value(&rows[0]).unwrap()
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

fn after_restart(device: &SimulatedDevice) -> Vec<Seen> {
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
        .expect("restarted");
    seen[restart + 1..].to_vec()
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

/// The check's frames from its own `T_Connect` (the run's last, after the
/// transfer's) on, which must exist; only reads. Not anchored on the restart:
/// a device that does not answer it logs none.
fn assert_check_read_only(device: &SimulatedDevice) {
    let seen = device.seen();
    let connects: Vec<usize> = (0..seen.len())
        .filter(|&i| seen[i] == Seen::Connect)
        .collect();
    assert_eq!(
        connects.len(),
        2,
        "the transfer's connection and the check's"
    );
    let check = &seen[connects[1]..];
    let refused: Vec<&Seen> = check.iter().filter(|seen| !read_only_frame(seen)).collect();
    assert_eq!(refused, Vec::<&Seen>::new(), "the check only reads");
    assert!(check.contains(&Seen::Other("DeviceDescriptorRead")));
    assert!(!device.verify_mode());
}

#[tokio::test]
async fn the_worker_checks_after_the_transfer_and_every_reader_keeps_both_facts() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let mut started = start(&device, plan(), AfterRestart::Pass);
    started.session.join().await;

    let (status, _, events) = started.session.snapshot_since(0);
    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["state"], "finished");
    assert_eq!(json["written"], "yes");
    assert_eq!(json["restart"], "acknowledged");
    let check = &json["postDownloadCheck"];
    assert_eq!(check["runtime"], "unknown");
    assert_eq!(check["observation"]["reach"]["identity"]["result"], "match");
    assert_eq!(
        check["observation"]["reach"]["identity"]["planLoadVerdict"],
        "matchesPlan"
    );
    assert_eq!(
        events.last(),
        Some(&ProgressEvent::PostDownloadCheckStarted),
        "the check is announced after every transfer step"
    );
    // One restart, one fresh connection, only reads after it.
    assert_eq!(restarts(&device), 1);
    let after = after_restart(&device);
    assert_eq!(
        after.iter().filter(|seen| **seen == Seen::Connect).count(),
        1
    );
    assert_check_read_only(&device);
    assert_eq!(device.authorize_requests(), 0);
    // Programming settled once, from the transfer, before the check.
    let settled = started.settled.lock().unwrap().clone();
    assert_eq!(settled.len(), 1);
    assert!(matches!(
        &settled[0],
        DownloadStatus::Finished { post_download_check, .. } if !post_download_check.is_checked()
    ));
    // The downloadable report and the durable history agree.
    let report = serde_json::to_value(started.session.report().unwrap()).unwrap();
    assert_eq!(report["outcome"]["written"], "yes");
    assert_eq!(&report["postDownloadCheck"], check);
    let row = reopened(&started.history);
    assert_eq!(row["state"], "finished");
    assert_eq!(row["downloadEvidence"]["written"], "yes");
    assert_eq!(row["downloadEvidence"]["cleanup"], "returnedOk");
    assert_eq!(&row["downloadEvidence"]["postDownloadCheck"], check);
}

#[tokio::test]
async fn an_unconfirmed_restart_is_checked_once_without_another_restart() {
    let device = device(
        APPLICATION,
        SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        },
    );
    let mut started = start(&device, loaded_plan(), AfterRestart::Pass);
    started.session.join().await;
    let json = serde_json::to_value(started.session.status()).unwrap();
    assert_eq!(json["restart"], "unconfirmed", "{json}");
    assert!(json["restartNote"].is_string());
    assert_check_read_only(&device);
    assert_eq!(
        json["postDownloadCheck"]["observation"]["reach"]["result"],
        "reached"
    );
    assert_eq!(
        started
            .restart_requests
            .load(std::sync::atomic::Ordering::SeqCst),
        0,
        "no restart after the check connected"
    );
    assert_eq!(
        reopened(&started.history)["downloadEvidence"]["restart"],
        "unconfirmed"
    );
}

#[tokio::test]
async fn a_mismatch_or_an_unreached_device_never_turns_the_transfer_into_a_failure() {
    let device_b = device([0x00, 0x83, 0x00, 0x27, 0x16], SimulatorConfig::default());
    let mut started = start(&device_b, plan(), AfterRestart::Pass);
    started.session.join().await;
    let json = serde_json::to_value(started.session.status()).unwrap();
    assert_eq!(json["state"], "finished");
    assert_eq!(json["written"], "yes");
    assert_eq!(
        json["postDownloadCheck"]["observation"]["reach"]["identity"],
        json!({ "result": "mismatch", "part": { "kind": "application" }, "kind": "differs" })
    );
    let row = reopened(&started.history);
    assert_eq!(row["downloadEvidence"]["written"], "yes");

    let device_c = device(APPLICATION, SimulatorConfig::default());
    let mut started = start(&device_c, plan(), AfterRestart::Silent);
    started.session.join().await;
    let json = serde_json::to_value(started.session.status()).unwrap();
    assert_eq!(json["state"], "finished");
    assert_eq!(json["written"], "yes");
    assert_eq!(
        json["postDownloadCheck"]["observation"]["reach"],
        json!({ "result": "notReached" })
    );
    assert_eq!(restarts(&device_c), 1);
}

#[tokio::test]
async fn a_failed_transfer_gets_no_check_and_reports_not_checked() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let mut wrong = plan();
    wrong.manufacturer = 0x0084;
    let mut started = start(&device, wrong, AfterRestart::Pass);
    started.session.join().await;
    let (status, _, events) = started.session.snapshot_since(0);
    assert!(matches!(
        status,
        DownloadStatus::Failed {
            written: Written::No,
            ..
        }
    ));
    assert!(!events.contains(&ProgressEvent::PostDownloadCheckStarted));
    assert_eq!(
        device
            .seen()
            .iter()
            .filter(|seen| **seen == Seen::Connect)
            .count(),
        1,
        "no follow-up connection"
    );
    let report = serde_json::to_value(started.session.report().unwrap()).unwrap();
    assert_eq!(
        report["postDownloadCheck"]["observation"]["status"],
        "notChecked"
    );
    let row = reopened(&started.history);
    assert_eq!(row["state"], "failed");
    assert!(row["downloadEvidence"].get("postDownloadCheck").is_none());
}

#[tokio::test]
async fn stopping_the_worker_during_the_check_keeps_the_written_transfer() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let mut started = start(&device, plan(), AfterRestart::Hang);
    tokio::time::timeout(Duration::from_secs(5), started.hung.notified())
        .await
        .expect("the check started after a written transfer");
    assert!(
        started.session.is_running(),
        "the check still owns the tunnel"
    );
    assert!(matches!(started.session.status(), DownloadStatus::Running));
    started.session.task.as_ref().unwrap().abort();
    started.session.join().await;

    let json = serde_json::to_value(started.session.status()).unwrap();
    assert_eq!(json["state"], "finished", "{json}");
    assert_eq!(json["written"], "yes");
    assert_eq!(
        json["postDownloadCheck"]["observation"]["status"],
        "interrupted"
    );
    assert!(!started.session.is_running());
    let row = reopened(&started.history);
    assert_eq!(row["state"], "finished");
    assert_eq!(row["downloadEvidence"]["written"], "yes");
    assert_eq!(row["downloadEvidence"]["cleanup"], "unknown");
    assert_eq!(
        row["downloadEvidence"]["postDownloadCheck"]["observation"]["status"],
        "interrupted"
    );
    assert_eq!(started.settled.lock().unwrap().len(), 1, "settled once");
}

#[test]
fn the_read_only_allowlist_refuses_every_other_service() {
    assert!(read_only_frame(&Seen::Other("DeviceDescriptorRead")));
    for seen in [
        Seen::Other("IndividualAddressWrite"),
        Seen::Other("FunctionPropertyCommand"),
        Seen::Other("Other"),
        Seen::Authorize(0),
        Seen::Restart {
            response: false,
            restart_type: 0,
            data: vec![],
        },
    ] {
        assert!(!read_only_frame(&seen), "{seen:?}");
    }
}
