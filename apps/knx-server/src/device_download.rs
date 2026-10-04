//! Runs one download to a device in the background and keeps its progress readable while it runs.
//!
//! "Download" is KNXBench → device over the bus (docs/GLOSSARY.md). The
//! route (`device_download_routes.rs`) prepares and checks everything,
//! then hands this module a [`PreparedDownload`] and a
//! [`WriteAuthorisation`]. From here on nothing is decided: the executor
//! (`knx_net::commissioning::memory_download`) runs the plan, and every
//! [`Progress`] it reports is appended to a log the UI polls. The log only
//! watches: it cannot change, skip or stop a step (ADR-0045 §5: there is
//! no cancel).

use std::sync::{Arc, Mutex};

use knx_app::access_key::DownloadKeying;
use knx_app::device_backup::{write_backup, StoredBackup};
use knx_app::device_download::PreparedDownload;
use knx_core::commissioning::authorisation::Authorisation;
use knx_core::commissioning::device_backup::DeviceBackup;
use knx_core::commissioning::memory_download::MemoryDownloadStep;
use knx_core::commissioning::mutation::WriteAuthorisation;
use knx_core::IndividualAddress;
use knx_net::commissioning::memory_download::{
    locked_device_hint, run_memory_download_with_backup, Progress, RestartOutcome,
};
use knx_net::{ApplicationService, BusError, Destination, ManagementSession, ScanTransport, Tpci};
use serde::{Deserialize, Serialize};
use tokio::task::JoinHandle;

use crate::bus::{BusTunnel, TunnelEvent};

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
}

/// Where the backup before the first write goes.
#[derive(Debug, Clone)]
pub struct BackupDestination {
    /// The directory, created when missing.
    pub dir: std::path::PathBuf,
    /// The time stamp (RFC 3339) the backup is named after.
    pub taken: String,
}

/// Whether anything was written to the device, as the CLI says it
/// (`written to the device: yes | no | partially`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Written {
    Yes,
    No,
    Partially,
}

/// What became of the closing restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Restart {
    Acknowledged,
    NotInPlan,
    /// The device did not acknowledge it; the data is written and read
    /// back. See KNOWN_LIMITATIONS §136.
    Unconfirmed,
}

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
    },
    Failed {
        written: Written,
        /// 1-based step the run stopped in, when it had started one.
        stopped_in_step: Option<usize>,
        error: String,
        /// When the failure is one a locked device also produces: what to
        /// check. Never a key, never a suggestion to try another.
        hint: Option<String>,
    },
}

#[derive(Debug)]
struct Shared {
    status: Mutex<DownloadStatus>,
    events: Mutex<Vec<ProgressEvent>>,
    /// First step from which a mutation may be attempted, not send proof.
    first_mutation_step: Option<usize>,
    /// The last step that started, 0-based.
    last_started: Mutex<Option<usize>>,
    /// What authorisation obtained, once connected.
    granted: Mutex<Option<(Authorisation, bool)>>,
    /// The backup file, once written.
    backup_file: Mutex<Option<std::path::PathBuf>>,
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
}

impl DeviceDownloadSession {
    /// Starts the run on `tunnel`. The caller has already checked the
    /// authorisation, the plan and that no other bus session runs.
    // Keep authorisation, recovery and lifecycle capabilities explicit at admission.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn start(
        id: u64,
        tunnel: Box<dyn BusTunnel>,
        authorisation: WriteAuthorisation,
        keying: DownloadKeying,
        timing: knx_net::SessionTiming,
        prepared: PreparedDownload,
        backups: BackupDestination,
        activity: crate::one_shot_activity::DownloadGuard,
    ) -> Self {
        let shared = Arc::new(Shared {
            status: Mutex::new(DownloadStatus::Running),
            events: Mutex::new(Vec::new()),
            first_mutation_step: prepared
                .plan
                .steps
                .iter()
                .position(MemoryDownloadStep::changes_device),
            last_started: Mutex::new(None),
            granted: Mutex::new(None),
            backup_file: Mutex::new(None),
        });
        let session = Self {
            id,
            target: prepared.target,
            device_name: prepared.device_name.clone(),
            steps: prepared.plan.steps.len(),
            data_octets: prepared.plan.data_octets(),
            shared: Arc::clone(&shared),
            task: None,
        };
        let task = tokio::spawn(run(
            tunnel,
            authorisation,
            keying,
            timing,
            prepared,
            backups,
            shared,
            activity,
        ));
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
            let written = written_so_far(&self.shared);
            let stopped = *self
                .shared
                .last_started
                .lock()
                .expect("download progress poisoned");
            *status = DownloadStatus::Failed {
                written,
                stopped_in_step: stopped.map(|index| index + 1),
                error: "the download task ended without a terminal result; any attempted write has an unknown outcome; connection cleanup is unconfirmed".into(),
                hint: None,
            };
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
) {
    let status = {
        let transport = TunnelTransport(tunnel.as_ref());
        match ManagementSession::authorised(&transport, keying.plan, timing, authorisation) {
            Err(e) => DownloadStatus::Failed {
                written: Written::No,
                stopped_in_step: None,
                error: e.to_string(),
                hint: None,
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
                    Ok(())
                };
                let result = run_memory_download_with_backup(
                    &mut session,
                    &prepared.plan,
                    move |progress| record(&observer, progress),
                    &mut keep,
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
                        }
                    }
                    Err(e) => {
                        let stopped = *shared
                            .last_started
                            .lock()
                            .expect("download progress poisoned");
                        let granted = *shared.granted.lock().expect("download progress poisoned");
                        DownloadStatus::Failed {
                            written: written_so_far(&shared),
                            stopped_in_step: stopped.map(|index| index + 1),
                            error: e.to_string(),
                            hint: locked_device_hint(
                                &e,
                                granted.map(|(authorisation, _)| authorisation),
                                granted.is_some_and(|(_, suspicious)| suspicious),
                            ),
                        }
                    }
                }
            }
        }
    };
    // Recording failure must not alter the worker outcome or skip cleanup.
    let _ = activity.record_result(&status);
    *shared.status.lock().expect("download status poisoned") = status;
    let cleanup = tunnel.disconnect().await;
    let _ = activity.record_cleanup(cleanup.is_ok());
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
        } => ProgressEvent::DataWritten {
            number: index + 1,
            address,
            octets,
            written,
            of,
        },
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

#[cfg(test)]
#[path = "device_download_task_tests.rs"]
mod task_tests;
