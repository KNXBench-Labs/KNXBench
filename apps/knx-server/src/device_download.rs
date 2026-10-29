//! Runs one download to a device in the background and keeps its progress readable while it runs.
//!
//! "Download" is KNXBench → device over the bus (docs/GLOSSARY.md). The
//! route (`device_download_routes.rs`) prepares and checks everything,
//! then hands this module a [`PreparedDownload`] and a
//! [`WriteAuthorisation`]. From here on nothing is decided: the executor
//! (`knx_net::commissioning::memory_download`) runs the plan, and every
//! [`Progress`] it reports is appended to a log the UI polls. The log only
//! watches: it cannot change or skip a step (ADR-0045 §5: there is no
//! cancel). The one stop there is lands before the first change only
//! (ADR-0153): a [`DownloadStop`] the worker owns from birth.

use std::sync::{Arc, Mutex};

use knx_app::access_key::DownloadKeying;
use knx_app::device_backup::{write_backup, StoredBackup};
use knx_app::device_download::PreparedDownload;
use knx_app::download_failure::DownloadFailure;
use knx_app::download_report::{DownloadReport, ReportedOutcome};
use knx_app::post_download_check::PostDownloadCheck;
use knx_core::commissioning::authorisation::Authorisation;
use knx_core::commissioning::device_backup::DeviceBackup;
use knx_core::commissioning::memory_download::{MemoryDownloadPlan, MemoryDownloadStep};
use knx_core::commissioning::mutation::WriteAuthorisation;
use knx_core::IndividualAddress;
use knx_net::commissioning::memory_download::{
    locked_device_hint, run_memory_download_with_backup_stoppable, DownloadStop,
    MemoryDownloadError, Progress, RestartOutcome, StopRequest,
};
use knx_net::commissioning::post_download::observe_after_download;
use knx_net::{ApplicationService, BusError, Destination, ManagementSession, ScanTransport, Tpci};
use serde::Serialize;
use tokio::task::JoinHandle;

use crate::bus::{BusTunnel, TunnelEvent};
use crate::programming_state::SettlementSlot;

/// One line of the progress log, in the order it happened.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ProgressEvent {
    /// A step is about to run.
    StepStarted {
        /// 1-based, as shown.
        number: usize,
        of: usize,
        step: String,
    },
    /// One block went to the device and was read back unchanged.
    DataWritten {
        /// 1-based step number.
        number: usize,
        address: u32,
        octets: Vec<u8>,
        written: usize,
        of: usize,
    },
    /// A step finished; `observed` is what the device answered, if the
    /// step reads an answer.
    StepDone {
        number: usize,
        observed: Option<String>,
    },
    /// The access the device granted, before the first write. `level` is
    /// `None` when no key was sent (the free level, value unknown). Never
    /// carries the key.
    Authorised {
        level: Option<u8>,
        /// The level is the minimum a wrong key earns (AL §3.5.7).
        suspicious: bool,
    },
    /// What the plan overwrites was read and kept before the first write.
    BackupTaken { regions: usize, octets: usize },
    /// The transfer finished written and its restart wait is over; the
    /// read-only check on a fresh connection starts now.
    PostDownloadCheckStarted,
}

/// Where the backup before the first write goes.
#[derive(Debug, Clone)]
pub struct BackupDestination {
    /// The directory, created when missing.
    pub dir: std::path::PathBuf,
    /// The time stamp (RFC 3339) the backup is named after.
    pub taken: String,
}

/// Shared payload-free write and restart evidence; the server keeps its progress DTO.
pub use knx_app::commissioning_activity::{Restart, Written};

/// Called once with a download's final status, before that status becomes
/// readable (ADR-0129: the programming belief follows the run).
pub type OnDownloadEnd = Box<dyn FnOnce(&DownloadStatus) + Send>;

/// Where a download stands.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DownloadStatus {
    Running,
    Finished {
        written: Written,
        restart: Restart,
        /// The restart's own explanation when it is unconfirmed.
        restart_note: Option<String>,
        /// The read-only check after the transfer; never changes the two
        /// fields above.
        post_download_check: PostDownloadCheck,
    },
    Failed {
        written: Written,
        /// 1-based step the run stopped in, when it had started one.
        stopped_in_step: Option<usize>,
        error: String,
        /// When the failure is one a locked device also produces: what to
        /// check. Never a key, never a suggestion to try another.
        hint: Option<String>,
        /// The payload-free terminal diagnostic (DIAG-02), the same value the
        /// log, the history and the report carry.
        failure: DownloadFailure,
    },
}

struct Shared {
    /// This run's one stop token, owned from before the worker spawns.
    stop: Arc<DownloadStop>,
    /// This run's operation recording (DIAG-03); finished after cleanup.
    recorder: knx_app::operation_recording::OperationRecorder,
    /// Test-only: runs once when the backup is durable, before the commit.
    #[cfg(test)]
    after_keep: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    /// Test-only: stalls the journal boundary after the asynchronous check.
    #[cfg(test)]
    before_record_check: Mutex<Option<Box<dyn FnOnce() + Send>>>,
    status: Mutex<DownloadStatus>,
    /// The witnessed transfer while its read-only check runs: an abrupt end
    /// then keeps it, with the check interrupted, instead of a failure.
    transfer: Mutex<Option<DownloadStatus>>,
    events: Mutex<Vec<ProgressEvent>>,
    /// First step from which a mutation may be attempted, not send proof.
    first_mutation_step: Option<usize>,
    /// The last step that started, 0-based.
    last_started: Mutex<Option<usize>>,
    /// Data octets written and read back unchanged so far.
    read_back: Mutex<usize>,
    /// The plan, for a failure diagnostic after an unwitnessed end.
    plan: MemoryDownloadPlan,
    /// What authorisation obtained, once connected.
    granted: Mutex<Option<(Authorisation, bool)>>,
    /// The backup file, once written.
    backup_file: Mutex<Option<std::path::PathBuf>>,
    on_end: Mutex<Option<OnDownloadEnd>>,
}

/// One download, running or finished. Kept after it ends so the final
/// state stays readable until the next download replaces it.
pub struct DeviceDownloadSession {
    id: u64,
    target: IndividualAddress,
    device_name: String,
    steps: usize,
    data_octets: usize,
    shared: Arc<Shared>,
    task: Option<JoinHandle<()>>,
    /// The tester's report of this run, without its outcome (ADR-0121).
    report: DownloadReport,
}

impl DeviceDownloadSession {
    /// Starts the run on `tunnel`. The caller has already checked the
    /// authorisation, the plan and that no other bus session runs. `stop`
    /// is the token the process stop already knows
    /// ([`crate::StopAdmission::register_download`]); the worker owns it
    /// from here, so a stop requested before the task first runs is not lost.
    // Keep authorisation, recovery and lifecycle capabilities explicit at admission.
    #[allow(clippy::too_many_arguments)]
    #[cfg(test)]
    pub(crate) fn start_with_stop(
        id: u64,
        tunnel: Box<dyn BusTunnel>,
        authorisation: WriteAuthorisation,
        keying: DownloadKeying,
        timing: knx_net::SessionTiming,
        prepared: PreparedDownload,
        backups: BackupDestination,
        activity: crate::one_shot_activity::DownloadGuard,
        report: DownloadReport,
        on_end: OnDownloadEnd,
        stop: Arc<DownloadStop>,
    ) -> Self {
        Self::start_inner(
            id,
            tunnel,
            authorisation,
            keying,
            timing,
            prepared,
            backups,
            activity,
            report,
            (on_end, None),
            stop,
            knx_app::operation_recording::OperationRecorder::disabled(),
        )
    }

    /// [`Self::start_with_stop`] for a receipted run: once `on_end` (the
    /// settler that fills `settlement`) has run, the worker records what it
    /// left there, before the check and the cleanup.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start_with_stop_receipted(
        id: u64,
        tunnel: Box<dyn BusTunnel>,
        authorisation: WriteAuthorisation,
        keying: DownloadKeying,
        timing: knx_net::SessionTiming,
        prepared: PreparedDownload,
        backups: BackupDestination,
        activity: crate::one_shot_activity::DownloadGuard,
        report: DownloadReport,
        on_end: OnDownloadEnd,
        settlement: SettlementSlot,
        stop: Arc<DownloadStop>,
        recorder: knx_app::operation_recording::OperationRecorder,
    ) -> Self {
        Self::start_inner(
            id,
            tunnel,
            authorisation,
            keying,
            timing,
            prepared,
            backups,
            activity,
            report,
            (on_end, Some(settlement)),
            stop,
            recorder,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn start_inner(
        id: u64,
        tunnel: Box<dyn BusTunnel>,
        authorisation: WriteAuthorisation,
        keying: DownloadKeying,
        timing: knx_net::SessionTiming,
        prepared: PreparedDownload,
        backups: BackupDestination,
        activity: crate::one_shot_activity::DownloadGuard,
        report: DownloadReport,
        (on_end, settlement): (OnDownloadEnd, Option<SettlementSlot>),
        stop: Arc<DownloadStop>,
        recorder: knx_app::operation_recording::OperationRecorder,
    ) -> Self {
        recorder.note(
            "plan",
            serde_json::json!({
                "target": prepared.target.to_string(),
                "steps": prepared.plan.steps.iter().map(ToString::to_string).collect::<Vec<_>>(),
                "dataOctets": prepared.plan.data_octets(),
                "timing": format!("{timing:?}"),
            }),
        );
        let shared = Arc::new(Shared {
            stop,
            recorder,
            #[cfg(test)]
            after_keep: Mutex::new(None),
            #[cfg(test)]
            before_record_check: Mutex::new(None),
            status: Mutex::new(DownloadStatus::Running),
            transfer: Mutex::new(None),
            events: Mutex::new(Vec::new()),
            first_mutation_step: prepared
                .plan
                .steps
                .iter()
                .position(MemoryDownloadStep::changes_device),
            last_started: Mutex::new(None),
            read_back: Mutex::new(0),
            plan: prepared.plan.clone(),
            granted: Mutex::new(None),
            backup_file: Mutex::new(None),
            on_end: Mutex::new(Some(on_end)),
        });
        let session = Self {
            id,
            target: prepared.target,
            device_name: prepared.device_name.clone(),
            steps: prepared.plan.steps.len(),
            data_octets: prepared.plan.data_octets(),
            shared: Arc::clone(&shared),
            task: None,
            report,
        };
        // The backup keeper, the journal rows and the settler persist
        // synchronously while the tunnel is open (S2-04, issue #30).
        let task = tokio::spawn(crate::tunnel_workers::off_tunnel_workers(run(
            tunnel,
            authorisation,
            keying,
            timing,
            prepared,
            backups,
            shared,
            activity,
            settlement,
        )));
        Self {
            task: Some(task),
            ..session
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn target(&self) -> IndividualAddress {
        self.target
    }

    pub fn device_name(&self) -> &str {
        &self.device_name
    }

    pub fn steps(&self) -> usize {
        self.steps
    }

    pub fn data_octets(&self) -> usize {
        self.data_octets
    }

    /// The backup taken before the first write, once it is on disk.
    pub fn backup_file(&self) -> Option<std::path::PathBuf> {
        self.shared
            .backup_file
            .lock()
            .expect("download backup poisoned")
            .clone()
    }

    /// The tester's report, once the run has ended; `None` while it runs.
    pub fn report(&self) -> Option<DownloadReport> {
        if self.is_running() {
            return None;
        }
        let (outcome, check, failure) = match self.status() {
            DownloadStatus::Running => return None,
            DownloadStatus::Finished {
                written,
                restart,
                post_download_check,
                ..
            } => (
                ReportedOutcome::completed(written, restart),
                post_download_check,
                None,
            ),
            DownloadStatus::Failed {
                written,
                stopped_in_step,
                failure,
                ..
            } => (
                ReportedOutcome::stopped(written, stopped_in_step),
                PostDownloadCheck::not_checked(),
                Some(failure),
            ),
        };
        Some(
            self.report
                .clone()
                .with_outcome(outcome)
                .with_check(check)
                .with_failure(failure),
        )
    }

    /// Asks the run to stop before its first change; see [`StopRequest`].
    /// Never waits and never cancels the task.
    pub(crate) fn request_stop(&self) -> StopRequest {
        self.shared.stop.request()
    }

    pub fn is_running(&self) -> bool {
        // A terminal device result is recorded before IP tunnel cleanup.
        // Keep the reservation while that cleanup future is still alive.
        if self.task.as_ref().is_some_and(|task| !task.is_finished()) {
            return true;
        }
        matches!(self.status(), DownloadStatus::Running)
    }

    pub fn status(&self) -> DownloadStatus {
        if self.task.as_ref().is_some_and(JoinHandle::is_finished) {
            self.record_unexpected_end();
        }
        self.shared
            .status
            .lock()
            .expect("download status poisoned")
            .clone()
    }

    fn record_unexpected_end(&self) {
        let mut status = self.shared.status.lock().expect("download status poisoned");
        // Do not erase an already witnessed result if cleanup later panics.
        if matches!(*status, DownloadStatus::Running) {
            let transfer = self
                .shared
                .transfer
                .lock()
                .expect("download transfer poisoned")
                .clone();
            if let Some(transfer) = transfer {
                // The transfer was witnessed and settled; only its check
                // did not finish.
                *status = with_check(transfer, PostDownloadCheck::interrupted());
                return;
            }
            let written = written_so_far(&self.shared);
            let stopped = *self
                .shared
                .last_started
                .lock()
                .expect("download progress poisoned");
            let stopped_in_step = stopped.map(|index| index + 1);
            let read_back = *self
                .shared
                .read_back
                .lock()
                .expect("download progress poisoned");
            let failed = DownloadStatus::Failed {
                written,
                stopped_in_step,
                error: "the download task ended without a terminal result; any attempted write has an unknown outcome; connection cleanup is unconfirmed".into(),
                hint: None,
                failure: DownloadFailure::task_ended(&self.shared.plan, stopped_in_step, read_back),
            };
            if let Some(on_end) = self
                .shared
                .on_end
                .lock()
                .expect("download settlement poisoned")
                .take()
            {
                on_end(&failed);
            }
            *status = failed;
        }
    }

    /// Status first, then events: a terminal status is only ever paired
    /// with the events published before it (the line scan's rule).
    pub fn snapshot_since(&self, since: usize) -> (DownloadStatus, usize, Vec<ProgressEvent>) {
        let status = self.status();
        let events = self.shared.events.lock().expect("download events poisoned");
        let start = since.min(events.len());
        (status, events.len(), events[start..].to_vec())
    }

    /// Waits for the run to end. For tests and shutdown; the routes poll.
    pub async fn join(&mut self) {
        if let Some(task) = self.task.as_mut() {
            let _ = task.await;
            self.task = None;
            self.record_unexpected_end();
        }
    }
}

/// The server's tunnel seen as what the management session needs.
pub(crate) struct TunnelTransport<'a>(pub(crate) &'a dyn BusTunnel);

impl ScanTransport for TunnelTransport<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        self.0.assigned_address()
    }

    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TunnelEvent> {
        self.0.subscribe()
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        self.0.send_frame(destination, transport, service).await
    }
}

fn written_so_far(shared: &Shared) -> Written {
    let last = *shared
        .last_started
        .lock()
        .expect("download progress poisoned");
    match (last, shared.first_mutation_step) {
        (Some(last), Some(first)) if last >= first => Written::Partially,
        _ => Written::No,
    }
}

// Mirror the admitted capabilities without hiding independent effects in an options bag.
#[allow(clippy::too_many_arguments)]
async fn run(
    tunnel: Box<dyn BusTunnel>,
    authorisation: WriteAuthorisation,
    keying: DownloadKeying,
    timing: knx_net::SessionTiming,
    prepared: PreparedDownload,
    backups: BackupDestination,
    shared: Arc<Shared>,
    mut activity: crate::one_shot_activity::DownloadGuard,
    settlement: Option<SettlementSlot>,
) {
    let target = authorisation.target();
    let status = {
        let transport = TunnelTransport(tunnel.as_ref());
        match ManagementSession::authorised(&transport, keying.plan, timing, authorisation) {
            Err(e) => DownloadStatus::Failed {
                written: Written::No,
                stopped_in_step: None,
                error: e.to_string(),
                hint: None,
                failure: DownloadFailure::from_session_refusal(&e, &prepared.plan),
            },
            Ok(session) => {
                let session = session.with_level_count(keying.level_count);
                let mut session = if keying.two_key {
                    session.with_two_key_extension()
                } else {
                    session
                };
                let observer = Arc::clone(&shared);
                let keeper = Arc::clone(&shared);
                let mut keep = |backup: &DeviceBackup| {
                    let stored = StoredBackup {
                        backup: backup.clone(),
                        application: prepared.request.program_id.clone(),
                        partial: prepared.partial.as_ref().map(|(parts, _)| *parts),
                        taken: backups.taken.clone(),
                        plan_steps: prepared
                            .plan
                            .steps
                            .iter()
                            .map(ToString::to_string)
                            .collect(),
                    };
                    let path = write_backup(&backups.dir, &stored).map_err(|e| e.to_string())?;
                    *keeper.backup_file.lock().expect("download backup poisoned") = Some(path);
                    activity.mark_send_possible().map_err(|e| e.to_string())?;
                    #[cfg(test)]
                    if let Some(hook) = keeper.after_keep.lock().expect("hook poisoned").take() {
                        hook();
                    }
                    Ok(())
                };
                let result = run_memory_download_with_backup_stoppable(
                    &mut session,
                    &prepared.plan,
                    move |progress| record(&observer, progress),
                    &mut keep,
                    &shared.stop,
                )
                .await;
                match result {
                    Ok(report) => {
                        let (restart, restart_note) = match &report.restart {
                            RestartOutcome::Acknowledged { .. } => (Restart::Acknowledged, None),
                            RestartOutcome::NotInPlan => (Restart::NotInPlan, None),
                            RestartOutcome::Unconfirmed { .. } => {
                                (Restart::Unconfirmed, Some(report.restart.to_string()))
                            }
                        };
                        DownloadStatus::Finished {
                            written: Written::Yes,
                            restart,
                            restart_note,
                            post_download_check: PostDownloadCheck::not_checked(),
                        }
                    }
                    Err(e) => {
                        // The full error, readback bytes included, for the recording only.
                        shared.recorder.note(
                            "downloadError",
                            serde_json::json!({ "display": e.to_string(), "debug": format!("{e:?}") }),
                        );
                        let stopped = *shared
                            .last_started
                            .lock()
                            .expect("download progress poisoned");
                        let granted = *shared.granted.lock().expect("download progress poisoned");
                        // A stop lands before the first change, whatever
                        // step the read-only part had reached.
                        let written = if matches!(e, MemoryDownloadError::Stopped { .. }) {
                            Written::No
                        } else {
                            written_so_far(&shared)
                        };
                        let stopped_in_step = stopped.map(|index| index + 1);
                        let read_back =
                            *shared.read_back.lock().expect("download progress poisoned");
                        DownloadStatus::Failed {
                            written,
                            stopped_in_step,
                            error: e.to_string(),
                            hint: locked_device_hint(
                                &e,
                                granted.map(|(authorisation, _)| authorisation),
                                granted.is_some_and(|(_, suspicious)| suspicious),
                            ),
                            failure: DownloadFailure::from_error(
                                &e,
                                &prepared.plan,
                                stopped_in_step,
                                read_back,
                            ),
                        }
                    }
                }
            }
        }
    };
    // Recording failure must not alter the worker outcome or skip cleanup.
    let witnessed = match &status {
        DownloadStatus::Running => knx_app::commissioning_activity::DownloadResult::Running,
        DownloadStatus::Finished {
            written, restart, ..
        } => knx_app::commissioning_activity::DownloadResult::Finished {
            written: *written,
            restart: *restart,
        },
        DownloadStatus::Failed { written, .. } => {
            knx_app::commissioning_activity::DownloadResult::Failed { written: *written }
        }
    };
    let _ = match &status {
        DownloadStatus::Failed {
            written, failure, ..
        } => activity
            .record_failure(*written, failure)
            // A refused diagnostic still leaves the row failed, cause not recorded.
            .or_else(|_| activity.record_result(&witnessed)),
        _ => activity.record_result(&witnessed),
    };
    if let Some(on_end) = shared
        .on_end
        .lock()
        .expect("download settlement poisoned")
        .take()
    {
        on_end(&status);
    }
    // The settler's receipt is recorded once, next to the transfer result and
    // before the check; a recording failure changes nothing else.
    let receipt = settlement
        .as_ref()
        .and_then(|slot| slot.lock().ok()?.take());
    if let Some(receipt) = receipt {
        let _ = activity.record_settlement(&receipt);
    }
    // The transfer is recorded and its session is gone (its plan ends with
    // `Disconnect`, after the restart wait). One read-only check follows on
    // the same tunnel, still reserved by this task: no restart, key, write or
    // second observation attempt (Transport Layer repetitions are
    // independent), and its outcome never changes the transfer's.
    let status = if matches!(
        status,
        DownloadStatus::Finished {
            written: Written::Yes,
            ..
        }
    ) {
        *shared.transfer.lock().expect("download transfer poisoned") = Some(status.clone());
        shared
            .events
            .lock()
            .expect("download events poisoned")
            .push(ProgressEvent::PostDownloadCheckStarted);
        let began = activity.begin_check();
        let observation = observe_after_download(
            &TunnelTransport(tunnel.as_ref()),
            target,
            timing,
            &prepared.plan,
        )
        .await;
        let check = PostDownloadCheck::from_observation(&observation);
        #[cfg(test)]
        if shared
            .before_record_check
            .lock()
            .expect("hook poisoned")
            .is_some()
        {
            // Force a pending boundary: a transfer-only block_in_place can
            // leave this poll on its handed-off thread until the next yield.
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        #[cfg(test)]
        if let Some(hook) = shared
            .before_record_check
            .lock()
            .expect("hook poisoned")
            .take()
        {
            hook();
        }
        let _ = began.and_then(|()| activity.record_check(&check));
        with_check(status, check)
    } else {
        status
    };
    let recorded_status = serde_json::to_value(&status).unwrap_or_default();
    let failed = matches!(status, DownloadStatus::Failed { .. });
    let error = match &status {
        DownloadStatus::Failed { error, .. } => Some(error.clone()),
        _ => None,
    };
    *shared.status.lock().expect("download status poisoned") = status;
    let cleanup = tunnel.disconnect().await;
    let _ = activity.record_cleanup(cleanup.is_ok());
    let summary = serde_json::json!({
        "status": recorded_status,
        "cleanup": cleanup.as_ref().map(|()| "ok".to_string()).unwrap_or_else(|e| e.to_string()),
    });
    if failed {
        crate::operation_recordings::finish_failed(
            &shared.recorder,
            error.unwrap_or_default(),
            summary,
        );
    } else {
        shared
            .recorder
            .finish(knx_app::operation_recording::RecordedOutcome::succeeded(
                summary,
            ));
    }
}

/// `status` with its check; any other state is returned as it is.
fn with_check(status: DownloadStatus, check: PostDownloadCheck) -> DownloadStatus {
    match status {
        DownloadStatus::Finished {
            written,
            restart,
            restart_note,
            ..
        } => DownloadStatus::Finished {
            written,
            restart,
            restart_note,
            post_download_check: check,
        },
        other => other,
    }
}

fn record(shared: &Shared, progress: Progress) {
    let event = match progress {
        // The session already knows target, steps and octets.
        Progress::Started { .. } => return,
        Progress::StepStarted { index, of, step } => {
            *shared
                .last_started
                .lock()
                .expect("download progress poisoned") = Some(index);
            ProgressEvent::StepStarted {
                number: index + 1,
                of,
                step,
            }
        }
        Progress::DataWritten {
            index,
            address,
            octets,
            written,
            of,
        } => {
            // `written` counts the plan's data octets written and read back.
            *shared.read_back.lock().expect("download progress poisoned") = written;
            ProgressEvent::DataWritten {
                number: index + 1,
                address,
                octets,
                written,
                of,
            }
        }
        Progress::StepDone(done) => ProgressEvent::StepDone {
            number: done.index + 1,
            observed: done.observed,
        },
        Progress::Authorised {
            authorisation,
            suspicious,
        } => {
            *shared.granted.lock().expect("download progress poisoned") =
                Some((authorisation, suspicious));
            ProgressEvent::Authorised {
                level: authorisation.level().map(|level| level.octet()),
                suspicious,
            }
        }
        Progress::BackupTaken { regions, octets } => ProgressEvent::BackupTaken { regions, octets },
    };
    shared
        .events
        .lock()
        .expect("download events poisoned")
        .push(event);
}

/// A three-step plan whose third step writes one octet, for synthetic runs.
#[cfg(test)]
pub(crate) fn synthetic_plan() -> MemoryDownloadPlan {
    MemoryDownloadPlan {
        mask: knx_core::commissioning::load_state::MaskVersion(0x0701),
        manufacturer: 0x0083,
        steps: vec![
            MemoryDownloadStep::Connect,
            MemoryDownloadStep::CompareProperty {
                object_index: 0,
                property_id: 78,
                inline_data: vec![0],
            },
            MemoryDownloadStep::WriteMemory {
                address: 0x4000,
                octets: vec![1],
            },
        ],
    }
}

/// A synthetic failure diagnostic for tests that only need a failed status.
#[cfg(test)]
pub(crate) fn synthetic_failure(stopped_in_step: Option<usize>) -> DownloadFailure {
    DownloadFailure::task_ended(&synthetic_plan(), stopped_in_step, 0)
}

#[cfg(test)]
impl DeviceDownloadSession {
    /// A download that runs exactly as long as `task`, for stop tests.
    pub(crate) fn synthetic_running(target: IndividualAddress, task: JoinHandle<()>) -> Self {
        Self {
            id: 1,
            target,
            device_name: "synthetic device".into(),
            steps: 3,
            data_octets: 1,
            shared: Arc::new(Shared {
                // The stand-in's write is under way: committed, so a stop is
                // too late, as for a real run past its first change.
                stop: Arc::new(DownloadStop::already_committed()),
                recorder: knx_app::operation_recording::OperationRecorder::disabled(),
                after_keep: Mutex::new(None),
                #[cfg(test)]
                before_record_check: Mutex::new(None),
                status: Mutex::new(DownloadStatus::Running),
                transfer: Mutex::new(None),
                events: Mutex::new(Vec::new()),
                first_mutation_step: Some(2),
                last_started: Mutex::new(None),
                read_back: Mutex::new(0),
                plan: synthetic_plan(),
                granted: Mutex::new(None),
                backup_file: Mutex::new(None),
                on_end: Mutex::new(None),
            }),
            task: Some(task),
            report: task_tests::synthetic_report(),
        }
    }
}

#[cfg(test)]
#[path = "device_download_task_tests.rs"]
mod task_tests;

#[cfg(test)]
#[path = "device_download_check_tests.rs"]
mod check_tests;
