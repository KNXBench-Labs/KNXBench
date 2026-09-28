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

use knx_app::device_download::PreparedDownload;
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::memory_download::MemoryDownloadStep;
use knx_core::commissioning::mutation::WriteAuthorisation;
use knx_core::IndividualAddress;
use knx_net::commissioning::memory_download::{
    run_memory_download_observed, Progress, RestartOutcome,
};
use knx_net::{ApplicationService, BusError, Destination, ManagementSession, ScanTransport, Tpci};
use serde::Serialize;
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
}

/// Whether anything was written to the device, as the CLI says it
/// (`written to the device: yes | no | partially`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Written {
    Yes,
    No,
    Partially,
}

/// What became of the closing restart.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
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
    },
}

#[derive(Debug)]
struct Shared {
    status: Mutex<DownloadStatus>,
    events: Mutex<Vec<ProgressEvent>>,
    /// The last step that started, 0-based.
    last_started: Mutex<Option<usize>>,
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
    pub fn start(
        id: u64,
        tunnel: Box<dyn BusTunnel>,
        authorisation: WriteAuthorisation,
        timing: knx_net::SessionTiming,
        prepared: PreparedDownload,
    ) -> Self {
        let shared = Arc::new(Shared {
            status: Mutex::new(DownloadStatus::Running),
            events: Mutex::new(Vec::new()),
            last_started: Mutex::new(None),
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
        let task = tokio::spawn(run(tunnel, authorisation, timing, prepared, shared));
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

    pub fn is_running(&self) -> bool {
        matches!(self.status(), DownloadStatus::Running)
    }

    pub fn status(&self) -> DownloadStatus {
        self.shared
            .status
            .lock()
            .expect("download status poisoned")
            .clone()
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
        if let Some(task) = self.task.take() {
            if task.await.is_err() {
                let written = written_so_far(&self.shared, None);
                *self.shared.status.lock().expect("download status poisoned") =
                    DownloadStatus::Failed {
                        written,
                        stopped_in_step: None,
                        error: "the download task stopped unexpectedly".to_string(),
                    };
            }
        }
    }
}

/// The server's tunnel seen as what the management session needs.
struct TunnelTransport<'a>(&'a dyn BusTunnel);

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

fn written_so_far(shared: &Shared, plan: Option<&[MemoryDownloadStep]>) -> Written {
    let last = *shared
        .last_started
        .lock()
        .expect("download progress poisoned");
    match (last, plan) {
        (Some(last), Some(steps))
            if steps[..=last]
                .iter()
                .any(MemoryDownloadStep::changes_device) =>
        {
            Written::Partially
        }
        (Some(_), None) => Written::Partially,
        _ => Written::No,
    }
}

async fn run(
    tunnel: Box<dyn BusTunnel>,
    authorisation: WriteAuthorisation,
    timing: knx_net::SessionTiming,
    prepared: PreparedDownload,
    shared: Arc<Shared>,
) {
    let status = {
        let transport = TunnelTransport(tunnel.as_ref());
        match ManagementSession::authorised(
            &transport,
            AuthorisationPlan::Skip,
            timing,
            authorisation,
        ) {
            Err(e) => DownloadStatus::Failed {
                written: Written::No,
                stopped_in_step: None,
                error: e.to_string(),
            },
            Ok(mut session) => {
                let observer = Arc::clone(&shared);
                let result =
                    run_memory_download_observed(&mut session, &prepared.plan, move |progress| {
                        record(&observer, progress);
                    })
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
                        DownloadStatus::Failed {
                            written: written_so_far(&shared, Some(&prepared.plan.steps)),
                            stopped_in_step: stopped.map(|index| index + 1),
                            error: e.to_string(),
                        }
                    }
                }
            }
        }
    };
    *shared.status.lock().expect("download status poisoned") = status;
    let _ = tunnel.disconnect().await;
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
    };
    shared
        .events
        .lock()
        .expect("download events poisoned")
        .push(event);
}
