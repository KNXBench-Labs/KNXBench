//! Supervised bounded catalogue jobs retain their worker and artifact ownership.

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::body::Body;
use http_body::Body as _;
use tokio::sync::{mpsc, oneshot, Notify, Semaphore};

use super::artifacts::ArtifactRoot;
use super::dto::*;
use super::persistence::{bounded_json, JobWorkspace};
use super::strict::{Capability, U64};
use super::CatalogueUpdateJobs;
use super::{CreateBody, CreateOutcome, DisposeOutcome};

/// Thin adapter over the existing managed owner/control/installer. Construction
/// has no default executor: production must supply the authentic B10 adapter.
pub trait CatalogueJobBridge: Send + Sync + 'static {
    /// Short, no SQL/filesystem/await, no second manager or publication gate.
    fn new_job(
        &self,
        status: Arc<JobStatus>,
    ) -> Result<Arc<dyn CatalogueJobExecution>, FailureKind>;
}

/// One retained session belongs to its manager record through settlement and
/// quarantine. Every operation uses the SAME existing A2/B10 publication gate.
pub trait CatalogueJobExecution: Send + Sync + 'static {
    fn request_stop(&self, cause: StopCause) -> Result<CancelOutcome, FailureKind>;
    /// true = newly accepted, false = exact accepted-command replay.
    fn accept_apply(&self, command: &ApplyCommand) -> Result<bool, FailureKind>;
    fn expire(&self) -> Result<(), FailureKind>;
    /// Calls the real controlled preview/rehearsal, emitting all artifact rows.
    fn preview(&self, source: &[u8], workspace: &mut JobWorkspace) -> Result<(), FailureKind>;
    /// Calls the real controlled apply, durable backup and live COMMIT bridge.
    fn apply(&self, source: &[u8], workspace: &mut JobWorkspace) -> Result<(), FailureKind>;
    /// Exclusively before native preview/apply has run.
    fn receive_failed(&self, failure: FailureKind);
    /// Settles a stopped Receiving/Ready session while no worker/native call is running.
    fn settle_idle(&self);
    /// Inspects actual B10 outcome/cleanup inventory; never assumes rollback.
    fn operation_failed(&self, failure: FailureKind);
    /// Recovers the retained COMMIT witness and failed native handles on unwind.
    fn worker_lost(&self);
    /// An actual inspected native-handle settlement, not a worker-return guess.
    fn handles_closed(&self) -> bool;
}

mod status_publication;
mod supervisor;
use status_publication::initial;
pub use status_publication::JobStatus;

pub(super) struct JobRecord {
    pub(super) status: Arc<JobStatus>,
    execution: OnceLock<Arc<dyn CatalogueJobExecution>>,
    workspace: OnceLock<Arc<Mutex<JobWorkspace>>>,
    upload_claimed: AtomicBool,
    readers_closed: AtomicBool,
}
impl JobRecord {
    fn execution(&self) -> Result<&Arc<dyn CatalogueJobExecution>, FailureKind> {
        self.execution.get().ok_or(FailureKind::JobBusy)
    }
    fn workspace(&self) -> Result<Arc<Mutex<JobWorkspace>>, FailureKind> {
        self.workspace.get().cloned().ok_or(FailureKind::JobBusy)
    }
}

enum Command {
    Setup(Arc<JobRecord>),
    Upload(
        Arc<JobRecord>,
        Body,
        oneshot::Sender<Result<(), FailureKind>>,
    ),
    Apply(Arc<JobRecord>),
    Dispose(
        Arc<JobRecord>,
        Option<oneshot::Sender<Result<(), FailureKind>>>,
    ),
}

pub struct CatalogueRuntime {
    owner: Arc<CatalogueUpdateJobs>,
    root: Arc<ArtifactRoot>,
    incarnation_dir: std::path::PathBuf,
    origin: Instant,
    bridge: Arc<dyn CatalogueJobBridge>,
    commands: mpsc::Sender<Command>,
    closing: AtomicBool,
    wake: Notify,
    pub(super) status_permit: Semaphore,
    pub(super) detail_permit: Arc<Semaphore>,
}

impl CatalogueRuntime {
    /// Adopts the existing AppState owner AFTER managed catalogue startup and
    /// path/static fencing. Registers exactly one joinable persistent supervisor.
    pub fn start(
        owner: Arc<CatalogueUpdateJobs>,
        catalogue: &std::path::Path,
        bridge: Arc<dyn CatalogueJobBridge>,
    ) -> Result<Arc<Self>, FailureKind> {
        tokio::runtime::Handle::try_current().map_err(|_| FailureKind::WorkerFailed)?;
        let mut attachment = owner
            .runtime
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        if attachment
            .as_ref()
            .and_then(std::sync::Weak::upgrade)
            .is_some()
        {
            return Err(FailureKind::JobBusy);
        }
        let parent = catalogue
            .parent()
            .ok_or(FailureKind::UnsupportedCatalogue)?;
        let root = Arc::new(
            ArtifactRoot::acquire(parent).map_err(|_| FailureKind::ArtifactStoreUnavailable)?,
        );
        // Unknown prior files are never swept by prefix. Exact inventory recovery
        // is performed before a new incarnation directory is allocated.
        JobWorkspace::reclaim_previous(&root).map_err(|_| FailureKind::ArtifactStoreUnavailable)?;
        let incarnation = owner
            .manager
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?
            .admission()
            .incarnation;
        let incarnation_dir = root
            .create_incarnation_dir(&incarnation)
            .map_err(|_| FailureKind::ArtifactStoreUnavailable)?;
        let (commands, receiver) = mpsc::channel(2);
        let runtime = Arc::new(Self {
            owner: Arc::clone(&owner),
            root,
            incarnation_dir,
            origin: Instant::now(),
            bridge,
            commands,
            closing: AtomicBool::new(false),
            wake: Notify::new(),
            status_permit: Semaphore::new(1),
            detail_permit: Arc::new(Semaphore::new(1)),
        });
        // A parked start barrier prevents unregistered work from starting.
        let (start, started) = oneshot::channel();
        let worker_runtime = Arc::clone(&runtime);
        let worker = tokio::spawn(async move {
            if started.await.is_ok() {
                worker_runtime.supervise(receiver).await;
            }
        });
        if let Err(handle) = owner.register_worker(worker) {
            // The barrier was never released: there can be no native/file work.
            drop(start);
            handle.abort();
            return Err(FailureKind::JobBusy);
        }
        *attachment = Some(Arc::downgrade(&runtime));
        start.send(()).map_err(|_| FailureKind::WorkerFailed)?;
        Ok(runtime)
    }

    pub fn owner(&self) -> &Arc<CatalogueUpdateJobs> {
        &self.owner
    }
    pub fn admission(&self) -> Result<Admission, FailureKind> {
        let (offer, busy) = {
            let manager = self
                .owner
                .manager
                .lock()
                .map_err(|_| FailureKind::CatalogueUnavailable)?;
            (manager.admission(), manager.state.active.is_some())
        };
        let admission = match offer.state {
            super::AdmissionState::Open if busy => AdmissionState::Busy,
            super::AdmissionState::Open => AdmissionState::Open,
            super::AdmissionState::Busy => AdmissionState::Busy,
            super::AdmissionState::Stopping => AdmissionState::Stopping,
            super::AdmissionState::Exhausted => AdmissionState::Exhausted,
            super::AdmissionState::ArtifactStoreUnavailable => {
                AdmissionState::ArtifactStoreUnavailable
            }
            super::AdmissionState::CatalogueUnavailable => AdmissionState::CatalogueUnavailable,
        };
        Ok(Admission {
            schema_version: 1,
            policy_version: POLICY_VERSION.into(),
            server_incarnation: offer.incarnation,
            next_admission_serial: offer.next_serial,
            server_tick_ms: U64(u64::try_from(self.origin.elapsed().as_millis())
                .map_err(|_| FailureKind::CounterExhausted)?),
            admission,
        })
    }

    pub fn create(
        &self,
        capability: &Capability,
        body: Create,
    ) -> Result<(bool, Status), FailureKind> {
        body.validate().map_err(|_| FailureKind::InvalidRequest)?;
        let receiving_deadline = u64::try_from(self.origin.elapsed().as_millis())
            .ok()
            .and_then(|n| n.checked_add(3_600_000))
            .map(U64)
            .ok_or(FailureKind::CounterExhausted)?;
        let mut manager = self
            .owner
            .manager
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        let outcome = manager.create(
            capability,
            CreateBody {
                incarnation: body.server_incarnation,
                serial: body.admission_serial,
                source_name: body.source_name.clone(),
                declared_bytes: body.declared_bytes,
                profile: if body.profile == ProfileName::Large {
                    super::Profile::Large
                } else {
                    super::Profile::Standard
                },
                allow_large_package: body.allow_large_package,
            },
        );
        let id = match outcome {
            CreateOutcome::Fresh(id) => id,
            CreateOutcome::Replay(id) => {
                let handle = super::Handle {
                    incarnation: body.server_incarnation,
                    serial: body.admission_serial,
                    job: id,
                };
                return Self::lookup_locked(&manager, handle, capability)
                    .and_then(|r| r.status.snapshot())
                    .map(|s| (false, s));
            }
            CreateOutcome::JobBusy => {
                match manager.state.admission {
                    super::AdmissionState::ArtifactStoreUnavailable => {
                        return Err(FailureKind::ArtifactStoreUnavailable)
                    }
                    super::AdmissionState::CatalogueUnavailable => {
                        return Err(FailureKind::CatalogueUnavailable)
                    }
                    _ => (),
                }
                if let Some(oldest) = manager
                    .state
                    .terminals
                    .front()
                    .filter(|_| manager.state.terminals.len() >= super::MAX_TERMINALS)
                    .and_then(|r| r.runtime.clone())
                {
                    if !oldest.readers_closed.swap(true, Ordering::SeqCst)
                        && self
                            .commands
                            .try_send(Command::Dispose(Arc::clone(&oldest), None))
                            .is_err()
                    {
                        oldest.readers_closed.store(false, Ordering::SeqCst);
                    }
                }
                return Err(FailureKind::JobBusy);
            }
            CreateOutcome::CommandConflict => return Err(FailureKind::CommandConflict),
            CreateOutcome::OutcomeUnavailable => return Err(FailureKind::OutcomeUnavailable),
            CreateOutcome::StaleOffer => return Err(FailureKind::StaleOffer),
            CreateOutcome::IncarnationChanged => return Err(FailureKind::IncarnationChanged),
            CreateOutcome::Exhausted => return Err(FailureKind::CounterExhausted),
            CreateOutcome::Stopping => return Err(FailureKind::ServerStopping),
            CreateOutcome::EntropyUnavailable => return Err(FailureKind::IoFailure),
        };
        let handle = super::Handle {
            incarnation: body.server_incarnation,
            serial: body.admission_serial,
            job: id,
        };
        let record = Arc::new(JobRecord {
            status: Arc::new(initial(handle, &body, self.origin, receiving_deadline)),
            execution: OnceLock::new(),
            workspace: OnceLock::new(),
            upload_claimed: AtomicBool::new(false),
            readers_closed: AtomicBool::new(false),
        });
        manager
            .state
            .active
            .as_mut()
            .ok_or(FailureKind::CatalogueUnavailable)?
            .runtime = Some(Arc::clone(&record));
        drop(manager);
        let execution = catch_unwind(AssertUnwindSafe(|| {
            self.bridge.new_job(Arc::clone(&record.status))
        }));
        match execution {
            Ok(Ok(execution)) => {
                let _ = record.execution.set(execution);
            }
            Ok(Err(kind)) => {
                self.fail_before_worker(&record, kind)?;
                return record.status.snapshot().map(|s| (true, s));
            }
            Err(_) => {
                self.fail_before_worker(&record, FailureKind::WorkerFailed)?;
                if let Ok(mut manager) = self.owner.manager.lock() {
                    manager.state.admission = super::AdmissionState::CatalogueUnavailable;
                }
                return record.status.snapshot().map(|s| (true, s));
            }
        }
        if self
            .commands
            .try_send(Command::Setup(Arc::clone(&record)))
            .is_err()
        {
            self.fail_before_worker(&record, FailureKind::WorkerFailed)?;
            self.quarantine(&record);
        }
        record.status.snapshot().map(|s| (true, s))
    }

    fn fail_before_worker(
        &self,
        record: &Arc<JobRecord>,
        kind: FailureKind,
    ) -> Result<(), FailureKind> {
        // The bridge factory is explicitly forbidden to do native/file work,
        // and no supervisor command has yet been handed off.
        record.status.update(|s| {
            s.state = JobState::Failed;
            s.phase = None;
            s.progress = None;
            s.apply.state = ApplyState::Spent;
            s.failure = Some(Failure {
                kind,
                context: None,
            });
            s.commit = Commit {
                outcome: CommitOutcome::NotCommitted,
                evidence: CommitEvidence::NoLiveMutation,
            };
            s.cleanup = Cleanup {
                disposition: CleanupDisposition::Complete,
                failure_kind: None,
            };
            s.lease.kind = LeaseKind::Retention;
        })?;
        record.status.finish_joined()?;
        let mut manager = self
            .owner
            .manager
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        if manager
            .state
            .active
            .as_ref()
            .and_then(|a| a.runtime.as_ref())
            .is_some_and(|r| Arc::ptr_eq(r, record))
        {
            manager.finish_active(true);
        }
        Ok(())
    }

    fn lookup_locked(
        manager: &super::CatalogueJobManager,
        handle: super::Handle,
        capability: &Capability,
    ) -> Result<Arc<JobRecord>, FailureKind> {
        if manager.state.incarnation != handle.incarnation {
            return Err(FailureKind::IncarnationChanged);
        }
        if let Some(active) = manager
            .state
            .active
            .as_ref()
            .filter(|a| a.meta.handle == handle && a.meta.capability.verify(capability))
        {
            return active.runtime.clone().ok_or(FailureKind::JobBusy);
        }
        manager
            .state
            .terminals
            .iter()
            .find(|t| t.meta.handle == handle && t.meta.capability.verify(capability))
            .and_then(|t| t.runtime.clone())
            .ok_or(FailureKind::OutcomeUnavailable)
    }
    fn lookup(
        &self,
        handle: super::Handle,
        capability: &Capability,
    ) -> Result<Arc<JobRecord>, FailureKind> {
        let manager = self
            .owner
            .manager
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        Self::lookup_locked(&manager, handle, capability)
    }
    pub fn status(
        &self,
        handle: super::Handle,
        capability: &Capability,
    ) -> Result<Status, FailureKind> {
        self.lookup(handle, capability)?.status.snapshot()
    }

    pub async fn upload(
        &self,
        handle: super::Handle,
        capability: &Capability,
        body: Body,
    ) -> Result<Status, FailureKind> {
        let record = self.lookup(handle, capability)?;
        if record
            .upload_claimed
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(FailureKind::UploadNotReplayable);
        }
        if record.status.snapshot()?.phase != Some(Phase::Receiving) {
            return Err(FailureKind::InvalidPhase);
        }
        let (reply, received) = oneshot::channel();
        if self
            .commands
            .try_send(Command::Upload(Arc::clone(&record), body, reply))
            .is_err()
        {
            record.upload_claimed.store(false, Ordering::SeqCst);
            return Err(FailureKind::JobBusy);
        }
        received.await.map_err(|_| FailureKind::WorkerFailed)??;
        record.status.snapshot()
    }

    pub fn apply(
        &self,
        handle: super::Handle,
        capability: &Capability,
        command: &ApplyCommand,
    ) -> Result<(bool, Status), FailureKind> {
        command
            .validate()
            .map_err(|_| FailureKind::InvalidRequest)?;
        let record = self.lookup(handle, capability)?;
        // Retained command replay/conflict precedes queue admission too. A new
        // acceptance reserves its supervisor handoff BEFORE consuming consent;
        // a full/disconnected queue can never strand an accepted operation.
        let fresh = if record
            .status
            .snapshot()?
            .apply
            .accepted_command_id
            .is_some()
        {
            record.execution()?.accept_apply(command)?
        } else {
            let permit = self
                .commands
                .try_reserve()
                .map_err(|_| FailureKind::JobBusy)?;
            let fresh = record.execution()?.accept_apply(command)?;
            if fresh {
                permit.send(Command::Apply(Arc::clone(&record)));
            }
            fresh
        };
        Ok((fresh, record.status.snapshot()?))
    }
    pub fn cancel(
        &self,
        handle: super::Handle,
        capability: &Capability,
        command: &CancelCommand,
    ) -> Result<CancelReply, FailureKind> {
        command
            .validate()
            .map_err(|_| FailureKind::InvalidRequest)?;
        let record = self.lookup(handle, capability)?;
        let outcome = if record.status.snapshot()?.state != JobState::Running {
            CancelOutcome::AlreadyTerminal
        } else {
            record.execution()?.request_stop(StopCause::UserCancel)?
        };
        record.status.changed.notify_waiters();
        self.wake.notify_one();
        Ok(CancelReply {
            schema_version: 1,
            command_id: command.command_id,
            outcome,
            job: record.status.snapshot()?,
        })
    }

    pub async fn details(
        &self,
        handle: super::Handle,
        capability: &Capability,
        artifact: super::JobId,
        digest: super::strict::Sha256,
        offset: U64,
        limit: usize,
    ) -> Result<Vec<u8>, FailureKind> {
        let record = self.lookup(handle, capability)?;
        if record.readers_closed.load(Ordering::SeqCst) {
            return Err(FailureKind::InvalidPhase);
        }
        let permit = Arc::clone(&self.detail_permit)
            .try_acquire_owned()
            .map_err(|_| FailureKind::JobBusy)?;
        let status = record.status.snapshot()?;
        let descriptor = status
            .preview
            .as_ref()
            .map(|p| &p.artifact)
            .into_iter()
            .chain(status.result.as_ref().map(|r| &r.artifact))
            .find(|a| a.artifact_id == artifact && a.public_sha256 == digest)
            .cloned()
            .ok_or(FailureKind::InvalidRequest)?;
        let workspace = record.workspace()?;
        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            if record.readers_closed.load(Ordering::SeqCst) {
                return Err(FailureKind::InvalidPhase);
            }
            let workspace = workspace.try_lock().map_err(|_| FailureKind::JobBusy)?;
            super::records::detail_page(&workspace, &descriptor, offset, limit)
        })
        .await
        .map_err(|_| FailureKind::WorkerFailed)?
    }

    pub async fn dispose(
        &self,
        handle: super::Handle,
        capability: &Capability,
    ) -> Result<DisposeReply, FailureKind> {
        let received = {
            let mut manager = self
                .owner
                .manager
                .lock()
                .map_err(|_| FailureKind::CatalogueUnavailable)?;
            if manager.state.admission == super::AdmissionState::ArtifactStoreUnavailable {
                return Err(FailureKind::ArtifactStoreUnavailable);
            }
            let record = match Self::lookup_locked(&manager, handle, capability) {
                Ok(record) => record,
                Err(FailureKind::OutcomeUnavailable) => {
                    return match manager.dispose(&handle, capability) {
                        DisposeOutcome::Disposed => Ok(DisposeReply {
                            schema_version: 1,
                            handle: wire_handle(handle),
                            disposition: DisposeDisposition::Disposed,
                        }),
                        _ => Err(FailureKind::OutcomeUnavailable),
                    };
                }
                Err(e) => return Err(e),
            };
            if record.status.snapshot()?.state == JobState::Running {
                return Err(FailureKind::InvalidPhase);
            }
            if record.readers_closed.swap(true, Ordering::SeqCst) {
                return Err(FailureKind::JobBusy);
            }
            let (reply, received) = oneshot::channel();
            if self
                .commands
                .try_send(Command::Dispose(Arc::clone(&record), Some(reply)))
                .is_err()
            {
                record.readers_closed.store(false, Ordering::SeqCst);
                return Err(FailureKind::JobBusy);
            }
            received
        };
        received.await.map_err(|_| FailureKind::WorkerFailed)??;
        Ok(DisposeReply {
            schema_version: 1,
            handle: wire_handle(handle),
            disposition: DisposeDisposition::Disposed,
        })
    }

    pub(super) fn request_close(&self) {
        self.closing.store(true, Ordering::SeqCst);
        let active = self
            .owner
            .manager
            .lock()
            .ok()
            .and_then(|m| m.state.active.as_ref().and_then(|a| a.runtime.clone()));
        if let Some(record) = active {
            if let Ok(execution) = record.execution() {
                let _ = catch_unwind(AssertUnwindSafe(|| {
                    execution.request_stop(StopCause::ServerStop)
                }));
            }
            record.status.changed.notify_waiters();
        }
        self.wake.notify_one();
    }
}

fn wire_handle(handle: super::Handle) -> Handle {
    Handle {
        server_incarnation: handle.incarnation,
        admission_serial: handle.serial,
        job_id: handle.job,
    }
}
fn internal_handle(handle: &Handle) -> super::Handle {
    super::Handle {
        incarnation: handle.server_incarnation,
        serial: handle.admission_serial,
        job: handle.job_id,
    }
}

#[cfg(test)]
mod service_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue_jobs::{Incarnation, JobId};
    #[test]
    fn ready_without_a_published_owned_manifest_is_never_visible() {
        let incarnation = Incarnation::from_bytes([1; 16]);
        let handle = super::super::Handle {
            incarnation,
            serial: U64(1),
            job: JobId::from_bytes([2; 16]),
        };
        let create = Create {
            schema_version: 1,
            policy_version: POLICY_VERSION.into(),
            server_incarnation: incarnation,
            admission_serial: U64(1),
            source_name: "sample.knxprod".into(),
            declared_bytes: U64(0),
            profile: ProfileName::Standard,
            allow_large_package: false,
        };
        let status = initial(handle, &create, Instant::now(), U64(3_600_000));
        let original = status.snapshot().unwrap();
        let mut ready = original.clone();
        ready.revision = U64(2);
        ready.phase = Some(Phase::Ready);
        ready.source.sha256 = Some(super::super::strict::Sha256::from_bytes([0; 32]));
        ready.progress = None;
        ready.lease.kind = LeaseKind::Ready;
        ready.apply.state = ApplyState::Available;
        ready.preview = Some(Preview {
            consent_id: JobId::from_bytes([3; 16]),
            artifact: Artifact {
                artifact_id: JobId::from_bytes([4; 16]),
                kind: ArtifactKind::Preview,
                public_sha256: super::super::strict::Sha256::from_bytes([0; 32]),
                public_bytes: U64(2),
                records: U64(1),
                withheld_fields: U64(0),
                withheld_records: U64(0),
                withheld_occurrences: U64(0),
                projection: Projection::RedactedCompleteV1,
            },
            package_already_installed: false,
            wrapper: false,
            packages: U64(1),
            programs: U64(0),
            products_added: U64(0),
            hardware2_programs_added: U64(0),
            id_conflicts: U64(0),
            growth_cliff_possible: true,
        });
        assert_eq!(
            status.publish(ready),
            Err(FailureKind::ReportMismatch),
            "Ready requires the actual complete owned artifact, not a supplied descriptor"
        );
        assert_eq!(status.snapshot().unwrap(), original);
    }
}
