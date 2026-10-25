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
        program: None,
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
    /// Test-only: holds one frame of the run until the test releases it.
    gate: Option<Arc<Gate>>,
}

/// Where a [`Gate`] holds the run.
#[derive(Clone, Copy, PartialEq, Eq)]
enum GateAt {
    /// The run's first `T_Connect`, before it reaches the device.
    FirstConnect,
    /// The first memory write of the run (the first load record), before it
    /// reaches the device.
    FirstMemoryWrite,
}

/// A one-shot hold on one frame: `entered` fires when the frame arrives,
/// the frame goes on once `release` is notified.
struct Gate {
    at: GateAt,
    armed: std::sync::atomic::AtomicBool,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
}

impl Gate {
    fn at(at: GateAt) -> Arc<Self> {
        Arc::new(Self {
            at,
            armed: std::sync::atomic::AtomicBool::new(true),
            entered: tokio::sync::Notify::new(),
            release: tokio::sync::Notify::new(),
        })
    }

    async fn hold(&self, transport: Tpci, service: &ApplicationService) {
        use std::sync::atomic::Ordering;
        let matches = match self.at {
            GateAt::FirstConnect => transport == Tpci::Connect,
            GateAt::FirstMemoryWrite => matches!(service, ApplicationService::MemoryWrite { .. }),
        };
        if matches && self.armed.swap(false, Ordering::SeqCst) {
            self.entered.notify_one();
            self.release.notified().await;
        }
    }
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
            if let Some(gate) = &self.gate {
                gate.hold(transport, &service).await;
            }
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
    /// Keeps the settler's server alive: it holds only a weak reference.
    _server: crate::SharedState,
}

fn start(device: &Arc<SimulatedDevice>, plan: MemoryDownloadPlan, mode: AfterRestart) -> Started {
    start_gated(device, plan, mode, None)
}

fn start_gated(
    device: &Arc<SimulatedDevice>,
    plan: MemoryDownloadPlan,
    mode: AfterRestart,
    gate: Option<Arc<Gate>>,
) -> Started {
    start_registered(
        device,
        plan,
        mode,
        gate,
        Arc::new(DownloadStop::new()),
        |_| {},
    )
}

/// [`start_gated`] with the token the process stop knows, and `after_keep`
/// run when the backup is durable, before the commit.
fn start_registered(
    device: &Arc<SimulatedDevice>,
    plan: MemoryDownloadPlan,
    mode: AfterRestart,
    gate: Option<Arc<Gate>>,
    stop: Arc<DownloadStop>,
    after_keep: impl FnOnce(&DownloadStop) + Send + 'static,
) -> Started {
    start_with(device, plan, mode, gate, stop, after_keep, None)
}

/// [`start_registered`] as the route starts a run: a preparation receipt on
/// the first row, and the real settler leaving its receipt for the worker.
fn start_receipted(
    device: &Arc<SimulatedDevice>,
    plan: MemoryDownloadPlan,
    mode: AfterRestart,
    gate: Option<Arc<Gate>>,
    stop: Arc<DownloadStop>,
    after_keep: impl FnOnce(&DownloadStop) + Send + 'static,
) -> Started {
    start_with(device, plan, mode, gate, stop, after_keep, Some(None))
}

/// `receipted`: `None` is the legacy start; `Some(None)` the route's, with
/// the real settler; `Some(Some(receipt))` leaves `receipt` in the slot
/// instead of the settler's, to see a refused recording.
#[allow(clippy::too_many_arguments)]
fn start_with(
    device: &Arc<SimulatedDevice>,
    plan: MemoryDownloadPlan,
    mode: AfterRestart,
    gate: Option<Arc<Gate>>,
    stop: Arc<DownloadStop>,
    after_keep: impl FnOnce(&DownloadStop) + Send + 'static,
    receipted: Option<Option<knx_app::commissioning_activity::DownloadSettlement>>,
) -> Started {
    let (receipted, injected) = (receipted.is_some(), receipted.flatten());
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("history.sqlite");
    let log = Arc::new(OneShotLog::for_run(history.clone()).unwrap());
    let target = device.address();
    let prepared = prepared(target, plan);
    let activity = if receipted {
        let preparation =
            knx_app::commissioning_activity::DownloadPreparation::from_prepared(&prepared)
                .expect("a complete plan has a scope");
        log.start_download_prepared(7, target, preparation).unwrap()
    } else {
        log.start_download(7, target).unwrap()
    };
    // The real settler, on a server with no project: its receipt is
    // `noProject` after a write and `notAttempted` after none.
    let server = Arc::new(crate::AppState::default());
    let (settler, slot) = crate::programming_state::download_settler_receipted(
        &server,
        crate::programming_state::incarnation(&server),
        &prepared,
    );
    let filled = Arc::clone(&slot);
    let settled = Arc::new(Mutex::new(Vec::new()));
    let settled_by_end = Arc::clone(&settled);
    let hung = Arc::new(tokio::sync::Notify::new());
    let restart_requests = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let mask = prepared.plan.mask;
    let token = Arc::clone(&stop);
    let on_end: OnDownloadEnd = Box::new(move |status| {
        settled_by_end.lock().unwrap().push(status.clone());
        if let Some(injected) = &injected {
            *filled.lock().unwrap() = Some(injected.clone());
        } else if receipted {
            settler(status);
        }
    });
    let tunnel = Box::new(SimTunnel {
        device: Arc::clone(device),
        mode,
        restarted: Arc::default(),
        restart_requests: Arc::clone(&restart_requests),
        check_connected: Default::default(),
        hung: Arc::clone(&hung),
        gate,
    });
    // The server's tunnel adapter is hardware-kind, as in the HTTP
    // tests: a simulator authorisation is refused there by design.
    let authorisation = WriteAuthorisation::for_hardware(
        target,
        WriteScope::Download,
        &knx_core::commissioning::mutation::required_confirmation_phrase(
            target,
            WriteScope::Download,
        ),
    )
    .unwrap();
    let keying = download_keying(mask, None, KeySource::None);
    let backups = BackupDestination {
        dir: dir.path().join("backups"),
        taken: "2026-10-25T07:00:00+01:00".into(),
    };
    let report = task_tests::synthetic_report();
    let session = if receipted {
        DeviceDownloadSession::start_with_stop_receipted(
            7,
            tunnel,
            authorisation,
            keying,
            fast(),
            prepared,
            backups,
            activity,
            report,
            on_end,
            slot,
            stop,
        )
    } else {
        DeviceDownloadSession::start_with_stop(
            7,
            tunnel,
            authorisation,
            keying,
            fast(),
            prepared,
            backups,
            activity,
            report,
            on_end,
            stop,
        )
    };
    *session.shared.after_keep.lock().unwrap() = Some(Box::new(move || after_keep(&token)));
    Started {
        session,
        history,
        settled,
        hung,
        restart_requests,
        _dir: dir,
        _server: server,
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
    // Characterization: the legacy start records no run-scoped receipt.
    let evidence = row["downloadEvidence"].as_object().unwrap();
    assert!(!evidence.contains_key("preparation"));
    assert!(!evidence.contains_key("settlement"));
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

/// The old bytes of everything [`plan`] writes, and every machine's state.
fn snapshot(device: &SimulatedDevice) -> (Vec<Vec<Option<u8>>>, Vec<LoadState>) {
    (
        [(0x4000, 4), (0x4400, 8)]
            .iter()
            .map(|&(base, length)| device.memory(base, length))
            .collect(),
        MemoryLoadStateMachine::ALL
            .iter()
            .map(|machine| device.load_state(ObjectIndex::new(machine.type_number())))
            .collect(),
    )
}

fn mutating(seen: &Seen) -> bool {
    matches!(
        seen,
        Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. } | Seen::Restart { .. }
    )
}

/// Installs `started`'s session as the server's download, as the route does.
async fn install(state: &crate::AppState, started: &mut Started) {
    let placeholder =
        DeviceDownloadSession::synthetic_running(started.session.target(), tokio::spawn(async {}));
    let session = std::mem::replace(&mut started.session, placeholder);
    *state.device_download.lock().await = Some(session);
}

/// Shuts the process down while the run is held at `gate`, then lets it go.
/// The release is polled after the drain's synchronous prefix.
async fn shutdown_at(state: &crate::AppState, gate: &Gate) -> crate::graceful_stop::WriteDrain {
    tokio::time::timeout(Duration::from_secs(5), gate.entered.notified())
        .await
        .expect("the run reached the gate");
    crate::graceful_stop::close_process_admission(state);
    let (drain, ()) = tokio::join!(
        crate::graceful_stop::drain_device_writes(
            state,
            Duration::from_secs(30),
            std::future::pending()
        ),
        async { gate.release.notify_one() }
    );
    drain
}

async fn final_session_status(state: &crate::AppState) -> DownloadStatus {
    state
        .device_download
        .lock()
        .await
        .as_ref()
        .expect("installed")
        .status()
}

#[tokio::test]
async fn a_shutdown_while_connecting_prevents_every_write() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let old = snapshot(&device);
    let state = crate::AppState::new(std::env::temp_dir());
    let gate = Gate::at(GateAt::FirstConnect);
    let mut started = start_gated(&device, plan(), AfterRestart::Pass, Some(gate.clone()));
    install(&state, &mut started).await;

    let drain = shutdown_at(&state, &gate).await;
    assert_ne!(drain.end, crate::graceful_stop::WriteWaitEnd::TimedOut);
    assert_eq!(drain.download_stop, Some(StopRequest::WillStop));

    let seen = device.seen();
    let writes: Vec<&Seen> = seen.iter().filter(|seen| mutating(seen)).collect();
    assert_eq!(writes, Vec::<&Seen>::new(), "no write may reach the device");
    assert_eq!(
        snapshot(&device),
        old,
        "the old memory and load states stay"
    );
    assert!(matches!(
        final_session_status(&state).await,
        DownloadStatus::Failed {
            written: Written::No,
            ..
        }
    ));
    let row = reopened(&started.history);
    assert_eq!(row["state"], "failed");
    assert_eq!(row["downloadEvidence"]["written"], "no");
}

#[tokio::test]
async fn a_shutdown_after_the_first_load_write_goes_on_to_the_full_schedule() {
    let reference = device(APPLICATION, SimulatorConfig::default());
    let mut ungated = start(&reference, plan(), AfterRestart::Pass);
    ungated.session.join().await;

    let device = device(APPLICATION, SimulatorConfig::default());
    let state = crate::AppState::new(std::env::temp_dir());
    let gate = Gate::at(GateAt::FirstMemoryWrite);
    let mut started = start_gated(&device, plan(), AfterRestart::Pass, Some(gate.clone()));
    install(&state, &mut started).await;
    let drain = shutdown_at(&state, &gate).await;
    assert_eq!(drain.download_stop, Some(StopRequest::TooLate));

    assert_eq!(device.seen(), reference.seen(), "same schedule as ungated");
    let status = final_session_status(&state).await;
    assert!(
        matches!(
            status,
            DownloadStatus::Finished {
                written: Written::Yes,
                ..
            }
        ),
        "{status:?}"
    );
    assert_eq!(snapshot(&device), snapshot(&reference));
}

#[tokio::test]
async fn a_shutdown_does_not_change_a_manufacturer_mismatch() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let state = crate::AppState::new(std::env::temp_dir());
    let gate = Gate::at(GateAt::FirstConnect);
    let mut wrong = plan();
    wrong.manufacturer = 0x0084;
    let mut started = start_gated(&device, wrong, AfterRestart::Pass, Some(gate.clone()));
    install(&state, &mut started).await;
    shutdown_at(&state, &gate).await;

    match final_session_status(&state).await {
        DownloadStatus::Failed {
            written: Written::No,
            error,
            ..
        } => assert!(error.contains("manufacturer"), "{error}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_new_run_on_the_same_device_after_a_shutdown_stop_succeeds() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let old = snapshot(&device);
    let state = crate::AppState::new(std::env::temp_dir());
    let gate = Gate::at(GateAt::FirstConnect);
    let mut stopped = start_gated(&device, plan(), AfterRestart::Pass, Some(gate.clone()));
    install(&state, &mut stopped).await;
    shutdown_at(&state, &gate).await;
    assert_eq!(snapshot(&device), old);

    let mut next = start(&device, plan(), AfterRestart::Pass);
    next.session.join().await;
    assert!(matches!(
        next.session.status(),
        DownloadStatus::Finished {
            written: Written::Yes,
            ..
        }
    ));
}

fn assert_stopped_before_any_change(status: &DownloadStatus) {
    match status {
        DownloadStatus::Failed {
            written: Written::No,
            error,
            hint: None,
            ..
        } => assert!(error.contains("stopped on request"), "{error}"),
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn a_close_before_the_session_is_installed_stops_the_registered_worker() {
    // The route registers while it holds the reservation and while the
    // tunnel is still being acquired; the close lands before `*download =
    // Some(..)`. The worker owns the very token the close requested.
    let device = device(APPLICATION, SimulatorConfig::default());
    let old = snapshot(&device);
    let state = crate::AppState::new(std::env::temp_dir());
    let reservation = state.device_download.lock().await;
    let stop = state.stop_admission.register_download().expect("open");
    assert_eq!(
        state.stop_admission.close_and_request_download_stop(),
        Some(StopRequest::WillStop),
        "the close needs no async lock"
    );
    assert!(stop.stop_pending());
    drop(reservation);

    let mut started = start_registered(&device, plan(), AfterRestart::Pass, None, stop, |_| {});
    started.session.join().await;
    assert_eq!(device.seen(), vec![], "S0: not a single frame");
    assert_eq!(snapshot(&device), old);
    assert_stopped_before_any_change(&started.session.status());
    let row = reopened(&started.history);
    assert_eq!(row["state"], "failed");
    assert_eq!(row["downloadEvidence"]["written"], "no");
    assert_eq!(row["writeEvidence"]["sendPossible"], false);
}

#[tokio::test]
async fn a_close_while_the_worker_connects_stops_it_without_the_drain() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let old = snapshot(&device);
    let state = crate::AppState::new(std::env::temp_dir());
    let stop = state.stop_admission.register_download().expect("open");
    let gate = Gate::at(GateAt::FirstConnect);
    let mut started = start_registered(
        &device,
        plan(),
        AfterRestart::Pass,
        Some(gate.clone()),
        stop,
        |_| {},
    );
    tokio::time::timeout(Duration::from_secs(5), gate.entered.notified())
        .await
        .expect("connecting");
    crate::graceful_stop::close_process_admission(&state);
    gate.release.notify_one();
    started.session.join().await;

    assert_eq!(mutating_frames(&device), 0);
    assert_eq!(snapshot(&device), old);
    assert_stopped_before_any_change(&started.session.status());
    assert_eq!(started.session.request_stop(), StopRequest::Ended);
}

fn mutating_frames(device: &SimulatedDevice) -> usize {
    device.seen().iter().filter(|seen| mutating(seen)).count()
}

#[tokio::test]
async fn a_close_after_the_first_change_committed_is_too_late_and_the_run_is_unchanged() {
    let reference = device(APPLICATION, SimulatorConfig::default());
    let mut ungated = start(&reference, plan(), AfterRestart::Pass);
    ungated.session.join().await;

    let device = device(APPLICATION, SimulatorConfig::default());
    let state = crate::AppState::new(std::env::temp_dir());
    let stop = state.stop_admission.register_download().expect("open");
    let gate = Gate::at(GateAt::FirstMemoryWrite);
    let mut started = start_registered(
        &device,
        plan(),
        AfterRestart::Pass,
        Some(gate.clone()),
        stop,
        |_| {},
    );
    tokio::time::timeout(Duration::from_secs(5), gate.entered.notified())
        .await
        .expect("writing");
    assert_eq!(
        state.stop_admission.close_and_request_download_stop(),
        Some(StopRequest::TooLate)
    );
    gate.release.notify_one();
    started.session.join().await;

    assert_eq!(device.seen(), reference.seen());
    assert!(matches!(
        started.session.status(),
        DownloadStatus::Finished {
            written: Written::Yes,
            ..
        }
    ));
}

#[tokio::test]
async fn a_stop_after_the_backup_is_durable_is_stored_as_failed_no_with_send_possible() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let old = snapshot(&device);
    let mut started = start_registered(
        &device,
        plan(),
        AfterRestart::Pass,
        None,
        Arc::new(DownloadStop::new()),
        |stop| {
            assert_eq!(stop.request(), StopRequest::WillStop);
        },
    );
    started.session.join().await;

    assert_eq!(mutating_frames(&device), 0);
    assert_eq!(snapshot(&device), old);
    assert_stopped_before_any_change(&started.session.status());
    assert!(started.session.backup_file().is_some(), "this run's backup");
    let row = reopened(&started.history);
    assert_eq!(row["state"], "failed", "{row}");
    assert_eq!(row["downloadEvidence"]["written"], "no");
    assert_eq!(row["writeEvidence"]["sendPossible"], true);
    assert_eq!(row["writeEvidence"]["backupRecorded"], true);
    assert_eq!(row["interrupted"], false);
}

#[tokio::test]
async fn a_run_dropped_without_a_result_still_reads_unknown_after_a_late_stop() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let gate = Gate::at(GateAt::FirstMemoryWrite);
    let mut started = start_registered(
        &device,
        plan(),
        AfterRestart::Pass,
        Some(gate.clone()),
        Arc::new(DownloadStop::new()),
        |_| {},
    );
    tokio::time::timeout(Duration::from_secs(5), gate.entered.notified())
        .await
        .expect("writing");
    assert_eq!(started.session.request_stop(), StopRequest::TooLate);
    started.session.task.as_ref().unwrap().abort();
    started.session.join().await;
    assert!(matches!(
        started.session.status(),
        DownloadStatus::Failed {
            written: Written::Partially,
            ..
        }
    ));
    let row = reopened(&started.history);
    assert!(
        row["downloadEvidence"]["written"].is_null(),
        "an unwitnessed outcome stays unwitnessed: {row}"
    );
    assert_eq!(row["downloadEvidence"]["cleanup"], "unknown");
}

#[tokio::test]
async fn a_registered_token_is_held_by_the_session_through_its_whole_life() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let state = crate::AppState::new(std::env::temp_dir());
    let stop = state.stop_admission.register_download().expect("open");
    let mut started = start_registered(&device, plan(), AfterRestart::Pass, None, stop, |_| {});
    started.session.join().await;
    // The run is over and the session is kept readable: a late close
    // reaches an ended token, harmlessly.
    assert_eq!(
        state.stop_admission.close_and_request_download_stop(),
        Some(StopRequest::Ended)
    );
    assert!(matches!(
        started.session.status(),
        DownloadStatus::Finished { .. }
    ));
}

// ---- the run-scoped receipts ---------------------------------------------

/// The route's start, with nothing to hold, stop or hook.
fn receipted(device: &Arc<SimulatedDevice>, mode: AfterRestart) -> Started {
    start_receipted(
        device,
        plan(),
        mode,
        None,
        Arc::new(DownloadStop::new()),
        |_| {},
    )
}

#[tokio::test]
async fn a_receipted_run_records_its_settlement_before_the_check_and_keeps_both_when_stopped() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let mut started = receipted(&device, AfterRestart::Hang);
    tokio::time::timeout(Duration::from_secs(5), started.hung.notified())
        .await
        .expect("the check started after a written transfer");
    // The check is still running, yet both receipts are already durable.
    let row = reopened(&started.history);
    let evidence = &row["downloadEvidence"];
    assert_eq!(row["state"], "finished");
    assert!(evidence.get("postDownloadCheck").is_none(), "{row}");
    assert_eq!(
        evidence["preparation"],
        json!({"state": "available", "format": 1, "scope": "complete",
               "program": {"state": "unavailable"},
               "plannedMask": "0701", "plannedManufacturer": "0083",
               "steps": plan().steps.len(), "dataOctets": 12})
    );
    assert_eq!(
        evidence["settlement"],
        json!({"project": "noProject", "format": 1})
    );
    started.session.task.as_ref().unwrap().abort();
    started.session.join().await;
    let row = reopened(&started.history);
    let evidence = &row["downloadEvidence"];
    assert_eq!(
        evidence["postDownloadCheck"]["observation"]["status"],
        "interrupted"
    );
    assert_eq!(evidence["settlement"]["project"], "noProject", "kept");
    assert!(evidence["preparation"].is_object());
    assert_eq!(evidence["cleanup"], "unknown");
    assert_eq!(started.settled.lock().unwrap().len(), 1, "settled once");
}

#[tokio::test]
async fn a_receipted_run_has_the_status_report_and_settlement_calls_of_a_legacy_one() {
    let device_a = device(APPLICATION, SimulatorConfig::default());
    let mut legacy = start(&device_a, plan(), AfterRestart::Pass);
    legacy.session.join().await;
    let device_b = device(APPLICATION, SimulatorConfig::default());
    let mut receipt = receipted(&device_b, AfterRestart::Pass);
    receipt.session.join().await;

    assert_eq!(
        serde_json::to_value(legacy.session.status()).unwrap(),
        serde_json::to_value(receipt.session.status()).unwrap()
    );
    assert_eq!(
        serde_json::to_value(legacy.session.report().unwrap()).unwrap(),
        serde_json::to_value(receipt.session.report().unwrap()).unwrap()
    );
    assert_eq!(
        *legacy.settled.lock().unwrap(),
        *receipt.settled.lock().unwrap()
    );
    assert_eq!(device_a.seen(), device_b.seen(), "the same frames");
    let (old, new) = (reopened(&legacy.history), reopened(&receipt.history));
    let mut without = new["downloadEvidence"].clone();
    let object = without.as_object_mut().unwrap();
    assert!(object.remove("preparation").is_some());
    assert!(object.remove("settlement").is_some());
    assert_eq!(without, old["downloadEvidence"], "nothing else differs");
    assert_eq!(new["downloadEvidence"]["cleanup"], "returnedOk");
}

#[tokio::test]
async fn a_stop_before_any_change_settles_as_not_attempted_and_a_refused_receipt_changes_nothing() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let mut started = start_receipted(
        &device,
        plan(),
        AfterRestart::Pass,
        None,
        Arc::new(DownloadStop::new()),
        |stop| assert_eq!(stop.request(), StopRequest::WillStop),
    );
    started.session.join().await;
    assert_eq!(mutating_frames(&device), 0);
    assert_stopped_before_any_change(&started.session.status());
    let row = reopened(&started.history);
    assert_eq!(row["state"], "failed");
    assert_eq!(row["downloadEvidence"]["written"], "no");
    assert_eq!(
        row["downloadEvidence"]["settlement"],
        json!({"project": "notAttempted", "format": 1})
    );

    // A receipt the row contradicts is refused; transfer, check and cleanup stay.
    let device = device_with_default();
    let contradiction = knx_app::commissioning_activity::DownloadSettlement::NotAttempted {
        format: knx_app::commissioning_activity::Format1,
    };
    let mut refused = start_with(
        &device,
        plan(),
        AfterRestart::Pass,
        None,
        Arc::new(DownloadStop::new()),
        |_| {},
        Some(Some(contradiction)),
    );
    refused.session.join().await;
    let row = reopened(&refused.history);
    let evidence = &row["downloadEvidence"];
    assert!(evidence.get("settlement").is_none(), "{row}");
    assert!(evidence["preparation"].is_object());
    assert_eq!(evidence["written"], "yes");
    assert_eq!(evidence["cleanup"], "returnedOk");
    assert_eq!(evidence["postDownloadCheck"]["runtime"], "unknown");
    assert!(matches!(
        refused.session.status(),
        DownloadStatus::Finished {
            written: Written::Yes,
            ..
        }
    ));
}

fn device_with_default() -> Arc<SimulatedDevice> {
    device(APPLICATION, SimulatorConfig::default())
}

#[tokio::test]
async fn a_receipted_run_dropped_without_a_result_keeps_its_preparation_and_gains_no_settlement() {
    let device = device(APPLICATION, SimulatorConfig::default());
    let gate = Gate::at(GateAt::FirstMemoryWrite);
    let mut started = start_receipted(
        &device,
        plan(),
        AfterRestart::Pass,
        Some(gate.clone()),
        Arc::new(DownloadStop::new()),
        |_| {},
    );
    tokio::time::timeout(Duration::from_secs(5), gate.entered.notified())
        .await
        .expect("writing");
    started.session.task.as_ref().unwrap().abort();
    started.session.join().await;
    // The unexpected-end path settles the belief but the guard is gone.
    let row = reopened(&started.history);
    let evidence = &row["downloadEvidence"];
    assert!(evidence["written"].is_null());
    assert!(evidence["preparation"].is_object());
    assert!(evidence.get("settlement").is_none(), "{row}");
}
