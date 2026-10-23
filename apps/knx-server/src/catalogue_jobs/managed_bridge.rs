//! Production session joining the sole managed catalogue, B10 gate and job sink.
use super::{
    dto::*,
    persistence::JobWorkspace,
    publication::{JobRecoveryPublisher, JobResultAdmission},
    report_adapter,
    runtime::{CatalogueJobBridge, CatalogueJobExecution, CatalogueRuntime, JobStatus},
    strict::{JobId, U64},
};
use crate::catalogue_job_control::{self as ctl, controlled_apply};
use crate::catalogue_owner::ControlledOwnerError;
use crate::{AppState, SharedState};
use knx_productdb::managed_apply::{
    self as db, AdmittedSource, ApplyReceipt, ApplyWorkspace, ControlledPolicy,
};
use knx_productdb::operation::{CommitArbitration, CommitClaim, PublicationGate, PublicationState};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::time::{Duration, Instant};

pub(super) struct ManagedBridge {
    state: Weak<AppState>,
    catalogue: PathBuf,
}
impl CatalogueJobBridge for ManagedBridge {
    fn new_job(
        &self,
        status: Arc<JobStatus>,
    ) -> Result<Arc<dyn CatalogueJobExecution>, FailureKind> {
        let s = status.snapshot()?;
        let identity = ctl::Identity {
            incarnation: *s.handle.server_incarnation.as_bytes(),
            serial: s.handle.admission_serial.get(),
            job: *s.handle.job_id.as_bytes(),
        };
        let control = ctl::Control::new_receiving(
            identity,
            OnceLock::new(),
            ctl::MonotonicClock {
                origin: status.monotonic_origin(),
            },
            s.lease.deadline_tick_ms.get(),
        )
        .map_err(control_error)?;
        Ok(Arc::new(Session {
            state: self.state.clone(),
            catalogue: self.catalogue.clone(),
            identity,
            control,
            status,
            transition: Mutex::new(()),
            native: Mutex::new(Native {
                closed: true,
                managed_settled: true,
                ..Native::default()
            }),
            finished: AtomicBool::new(false),
        }))
    }
}

pub(crate) fn mount(state: &SharedState) -> axum::Router {
    let runtime = (|| {
        if let Some(runtime) = state.catalogue_jobs.attached_runtime() {
            return Ok(runtime);
        }
        let owner = state
            .product_db
            .as_ref()
            .ok_or(FailureKind::CatalogueUnavailable)?;
        let path = owner
            .with_read(|conn| {
                conn.path()
                    .filter(|s| !s.is_empty())
                    .map(PathBuf::from)
                    .ok_or(FailureKind::UnsupportedCatalogue)
            })
            .map_err(|_| FailureKind::CatalogueUnavailable)??;
        let canonical = path
            .canonicalize()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        // Existing backup listing is bound to the configured filename. Do not
        // silently publish under a different symlink target's basename.
        if canonical.file_name() != path.file_name() {
            return Err(FailureKind::UnsupportedCatalogue);
        }
        let path = canonical;
        CatalogueRuntime::start(
            Arc::clone(&state.catalogue_jobs),
            &path,
            Arc::new(ManagedBridge {
                state: Arc::downgrade(state),
                catalogue: path.clone(),
            }),
        )
    })();
    match runtime {
        Ok(r) => super::http::router(r),
        Err(f) => super::http::unavailable_router(f),
    }
}
#[derive(Default)]
struct Native {
    workspace: Option<ApplyWorkspace>,
    receipt: ApplyReceipt,
    result: Option<JobResult>,
    result_failure: Option<FailureKind>,
    execution: Option<db::ApplyExecution>,
    started: bool,
    backup: Option<Backup>,
    closed: bool,
    managed_settled: bool,
    quarantined: bool,
}
struct Session {
    state: Weak<AppState>,
    catalogue: PathBuf,
    identity: ctl::Identity,
    control: controlled_apply::ManagedApplyControl<ctl::MonotonicClock>,
    status: Arc<JobStatus>,
    // All control/status transitions (including native claim) serialize here.
    // Native work never holds it. Native receipt/workspace survive caught unwind.
    transition: Mutex<()>,
    native: Mutex<Native>,
    finished: AtomicBool,
}
fn control_error(e: ctl::Error) -> FailureKind {
    match e {
        ctl::Error::Unavailable => FailureKind::CatalogueUnavailable,
        ctl::Error::InvalidHandle => FailureKind::InvalidHandle,
        ctl::Error::InvalidPhase => FailureKind::InvalidPhase,
        ctl::Error::CommandConflict => FailureKind::CommandConflict,
        ctl::Error::ServerStopping => FailureKind::ServerStopping,
        ctl::Error::ConsentExpired => FailureKind::ConsentExpired,
        ctl::Error::CounterExhausted => FailureKind::CounterExhausted,
        ctl::Error::InvalidRequest => FailureKind::InvalidRequest,
        ctl::Error::CatalogueUpdateStale => FailureKind::CatalogueUpdateStale,
    }
}
fn failure_kind(k: db::ControlledFailureKind) -> FailureKind {
    use db::ControlledFailureKind as K;
    match k {
        K::Stale => FailureKind::CatalogueUpdateStale,
        K::Growth => FailureKind::CatalogueGrowthLimit,
        K::AdmissionLimit => FailureKind::CatalogueAdmissionLimit,
        K::Configuration => FailureKind::UnsupportedCatalogue,
        K::Unavailable => FailureKind::CatalogueUnavailable,
        K::Stopped | K::ClaimRefused => FailureKind::Cancelled,
        K::Install => FailureKind::PackageRefused,
        K::LegacyProductDatabase => FailureKind::LegacyProductDatabase,
        K::PreviewIdentity | K::SourceMismatch | K::PolicyMismatch => FailureKind::ReportMismatch,
        K::Cleanup => FailureKind::CleanupFailed,
        K::CounterExhausted => FailureKind::CounterExhausted,
        K::ResultAdmission => FailureKind::ResultLimit,
        K::RecoveryPublication => FailureKind::IoFailure,
        K::Panicked => FailureKind::WorkerFailed,
        _ => FailureKind::DatabaseFailure,
    }
}
fn bridge_error(e: controlled_apply::BridgeError) -> FailureKind {
    match e {
        controlled_apply::BridgeError::Control(e) => control_error(e),
        controlled_apply::BridgeError::Owner(ControlledOwnerError::Preview(e)) => {
            failure_kind(e.kind())
        }
        controlled_apply::BridgeError::Owner(ControlledOwnerError::Stopped) => {
            FailureKind::Cancelled
        }
        controlled_apply::BridgeError::Owner(ControlledOwnerError::Busy) => FailureKind::JobBusy,
        _ => FailureKind::CatalogueUnavailable,
    }
}
fn cause(c: StopCause) -> ctl::Cause {
    match c {
        StopCause::UserCancel => ctl::Cause::UserCancel,
        StopCause::ServerStop => ctl::Cause::ServerStop,
        StopCause::ReadyExpired => ctl::Cause::ReadyExpired,
        StopCause::ReceivingIdleTimeout => ctl::Cause::ReceivingIdleTimeout,
        StopCause::ReceivingTimeout => ctl::Cause::ReceivingTimeout,
        StopCause::WorkingTimeout => ctl::Cause::WorkingTimeout,
        StopCause::CounterExhausted => ctl::Cause::CounterExhausted,
    }
}
fn stop(c: ctl::Cause) -> StopCause {
    match c {
        ctl::Cause::UserCancel => StopCause::UserCancel,
        ctl::Cause::ServerStop => StopCause::ServerStop,
        ctl::Cause::ReadyExpired => StopCause::ReadyExpired,
        ctl::Cause::ReceivingIdleTimeout => StopCause::ReceivingIdleTimeout,
        ctl::Cause::ReceivingTimeout => StopCause::ReceivingTimeout,
        ctl::Cause::WorkingTimeout => StopCause::WorkingTimeout,
        ctl::Cause::CounterExhausted => StopCause::CounterExhausted,
    }
}
fn stop_failure(c: StopCause) -> FailureKind {
    match c {
        StopCause::UserCancel => FailureKind::Cancelled,
        StopCause::ServerStop => FailureKind::ServerStop,
        StopCause::ReadyExpired => FailureKind::ReadyExpired,
        StopCause::ReceivingIdleTimeout => FailureKind::ReceivingIdleTimeout,
        StopCause::ReceivingTimeout => FailureKind::ReceivingTimeout,
        StopCause::WorkingTimeout => FailureKind::WorkingTimeout,
        StopCause::CounterExhausted => FailureKind::CounterExhausted,
    }
}
fn backup(workspace: &JobWorkspace, retain: bool) -> Backup {
    workspace.recovery_publication().map_or(
        Backup {
            state: BackupState::None,
            name: None,
            identity: None,
            publication: BackupPublication::None,
        },
        |b| Backup {
            state: if !b.sync_acknowledged || workspace.quarantined() {
                BackupState::Quarantined
            } else if retain {
                BackupState::Retained
            } else {
                BackupState::Published
            },
            name: Some(b.name.clone()),
            identity: Some(b.identity.clone()),
            publication: if b.sync_acknowledged {
                BackupPublication::FileAndDirectorySyncAcknowledged
            } else {
                BackupPublication::None
            },
        },
    )
}
impl Session {
    fn sync_control(&self, v: ctl::Snapshot) -> Result<(), FailureKind> {
        let settlement_deadline = self
            .status
            .server_tick_ms()?
            .get()
            .checked_add(7_200_000)
            .ok_or(FailureKind::CounterExhausted)?;
        self.status.update(|s| {
            let old_lease = s.lease.clone();
            s.gate = match v.gate {
                PublicationState::Open => Gate::Open,
                PublicationState::CancelAccepted => Gate::CancelAccepted,
                PublicationState::CommitClaimed => Gate::CommitClaimed,
            };
            s.stop_cause = v.stop_cause.map(stop);
            s.phase = Some(match v.phase {
                ctl::Phase::Receiving => Phase::Receiving,
                ctl::Phase::Previewing => Phase::Previewing,
                ctl::Phase::Ready => Phase::Ready,
                ctl::Phase::WaitingForWriter => Phase::WaitingForWriter,
                ctl::Phase::Committing => Phase::Committing,
                ctl::Phase::Settling => Phase::Settling,
            });
            s.lease = Lease {
                kind: match v.lease.kind {
                    ctl::LeaseKind::Receiving => LeaseKind::Receiving,
                    ctl::LeaseKind::Working => LeaseKind::Working,
                    ctl::LeaseKind::Ready => LeaseKind::Ready,
                },
                deadline_tick_ms: U64(v.lease.deadline_tick_ms),
            };
            s.apply = Apply {
                state: match v.consent {
                    ctl::Consent::Unavailable => ApplyState::Unavailable,
                    ctl::Consent::Available => ApplyState::Available,
                    ctl::Consent::Spent => {
                        if v.phase == ctl::Phase::Settling {
                            ApplyState::Spent
                        } else {
                            ApplyState::Accepted
                        }
                    }
                },
                accepted_command_id: v.accepted.map(|c| JobId::from_bytes(c.command)),
            };
            s.progress = None;
            if v.phase == ctl::Phase::Settling {
                s.lease = if old_lease.kind == LeaseKind::Working {
                    old_lease
                } else {
                    Lease {
                        kind: LeaseKind::Working,
                        deadline_tick_ms: U64(settlement_deadline),
                    }
                };
            }
        })
    }
    fn check_wait(&self, start: Instant) -> Result<(), FailureKind> {
        self.expire()?;
        self.control
            .checkpoint()
            .map_err(control_error)?
            .map_err(|_| FailureKind::Cancelled)?;
        if start.elapsed() >= Duration::from_secs(30) {
            return Err(FailureKind::WriterTimeout);
        }
        std::thread::sleep(Duration::from_millis(20));
        Ok(())
    }
    fn source<'a>(
        &self,
        name: &'a str,
        source: &'a [u8],
        profile: ProfileName,
    ) -> Result<AdmittedSource<'a>, FailureKind> {
        AdmittedSource::admit(
            name,
            source,
            if profile == ProfileName::Large {
                ControlledPolicy::Large
            } else {
                ControlledPolicy::Standard
            },
        )
        .map_err(|_| FailureKind::PackageRefused)
    }
    fn finish(&self, failure: Option<FailureKind>, native: &Native) -> Result<(), FailureKind> {
        let _transition = self
            .transition
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        if self.finished.load(Ordering::SeqCst) {
            return Ok(());
        }
        let v = self.control.snapshot().map_err(control_error)?;
        let mut s = self.status.snapshot()?;
        let witness = native.receipt.witness().map(|w| w.trace());
        let committed = matches!(
            witness,
            Some(db::LiveCommitTrace::LiveCommitReturnedOk { autocommit: true })
        );
        let execution = native.execution.as_ref();
        s.commit = if committed {
            Commit {
                outcome: CommitOutcome::Committed,
                evidence: CommitEvidence::CommitReturnedOk,
            }
        } else if execution.is_some_and(|e| e.commit() == db::CommitEvidence::NotCommitted) {
            Commit {
                outcome: CommitOutcome::NotCommitted,
                evidence: match execution.map(|e| e.rollback()) {
                    Some(db::RollbackDisposition::Explicit) => CommitEvidence::ExplicitRollback,
                    Some(db::RollbackDisposition::Automatic) => {
                        CommitEvidence::AutomaticRollbackBeforeCommit
                    }
                    _ => CommitEvidence::NoLiveMutation,
                },
            }
        } else if !native.started || (native.receipt.witness().is_none() && native.closed) {
            Commit {
                outcome: CommitOutcome::NotCommitted,
                evidence: CommitEvidence::NoLiveMutation,
            }
        } else {
            Commit {
                outcome: CommitOutcome::Unknown,
                evidence: if failure == Some(FailureKind::WorkerFailed) {
                    CommitEvidence::WorkerLost
                } else {
                    CommitEvidence::UnresolvedRollback
                },
            }
        };
        s.gate = match v.gate {
            PublicationState::Open => Gate::Open,
            PublicationState::CancelAccepted => Gate::CancelAccepted,
            PublicationState::CommitClaimed => Gate::CommitClaimed,
        };
        s.stop_cause = v.stop_cause.map(stop);
        s.result = if committed {
            native.result.clone()
        } else {
            None
        };
        s.backup = native.backup.clone().unwrap_or(Backup {
            state: BackupState::None,
            name: None,
            identity: None,
            publication: BackupPublication::None,
        });
        let quarantined = native.quarantined
            || !native.closed
            || execution.is_some_and(|e| e.cleanup_failed())
            || s.commit.outcome == CommitOutcome::Unknown
            || s.backup.state == BackupState::Quarantined;
        s.cleanup = Cleanup {
            disposition: if quarantined {
                CleanupDisposition::Quarantined
            } else {
                CleanupDisposition::Complete
            },
            failure_kind: quarantined.then_some(CleanupFailureKind::CleanupFailed),
        };
        let failure = if !committed {
            s.stop_cause.map(stop_failure).or(failure)
        } else {
            failure
        };
        s.failure = failure.map(|kind| Failure {
            kind,
            context: None,
        });
        s.state = if committed && failure.is_none() {
            JobState::Succeeded
        } else if quarantined {
            JobState::Failed
        } else {
            match failure {
                Some(FailureKind::Cancelled | FailureKind::ServerStop)
                    if s.gate == Gate::CancelAccepted =>
                {
                    JobState::Cancelled
                }
                Some(
                    FailureKind::ReadyExpired
                    | FailureKind::ReceivingTimeout
                    | FailureKind::ReceivingIdleTimeout
                    | FailureKind::WorkingTimeout,
                ) if s.gate == Gate::CancelAccepted => JobState::Expired,
                Some(FailureKind::CatalogueUpdateStale)
                    if s.gate == Gate::Open && s.preview.is_some() =>
                {
                    JobState::Stale
                }
                _ => JobState::Failed,
            }
        };
        if s.state == JobState::Failed && s.failure.is_none() {
            s.failure = Some(Failure {
                kind: FailureKind::CleanupFailed,
                context: None,
            });
        }
        s.phase = None;
        s.progress = None;
        s.apply.state = ApplyState::Spent;
        s.lease = Lease {
            kind: LeaseKind::Retention,
            deadline_tick_ms: U64(self
                .status
                .server_tick_ms()?
                .get()
                .checked_add(3_600_000)
                .ok_or(FailureKind::CounterExhausted)?),
        };
        s.revision = s
            .revision
            .checked_add(U64(1))
            .ok_or(FailureKind::CounterExhausted)?;
        self.status.publish(s)?;
        self.finished.store(true, Ordering::SeqCst);
        Ok(())
    }
    fn settle_workspace(
        &self,
        native: &mut Native,
        workspace: &mut JobWorkspace,
        retain: bool,
    ) -> Result<(), FailureKind> {
        let result = self.settle_workspace_inner(native, workspace, retain);
        if result.is_err() {
            native.quarantined = true;
            workspace.quarantine();
            native.backup = Some(backup(workspace, true));
        }
        result
    }
    fn settle_workspace_inner(
        &self,
        native: &mut Native,
        workspace: &mut JobWorkspace,
        retain: bool,
    ) -> Result<(), FailureKind> {
        native.closed = native
            .workspace
            .as_ref()
            .is_none_or(|w| w.native_handles_closed())
            && native.managed_settled;
        if !native.closed {
            workspace.quarantine();
            native.backup = Some(backup(workspace, true));
            return Err(FailureKind::CleanupFailed);
        }
        for name in ["rehearsal.sqlite", "recovery.part.sqlite"] {
            if workspace.owns_database(name) {
                workspace
                    .reconcile_closed_database(name, false)
                    .map_err(|_| FailureKind::CleanupFailed)?;
            }
        }
        if let Some(w) = native.workspace.as_mut() {
            w.release_closed_ownership()
                .map_err(|_| FailureKind::CleanupFailed)?;
        }
        if retain && workspace.recovery_publication().is_some() {
            workspace
                .retain_recovery()
                .map_err(|_| FailureKind::CleanupFailed)?;
        }
        if !retain {
            workspace
                .remove_uncommitted_backup(
                    workspace
                        .catalogue_parent()
                        .map(|p| p.to_path_buf())
                        .as_deref()
                        .ok_or(FailureKind::CleanupFailed)?,
                )
                .map_err(|_| FailureKind::CleanupFailed)?;
        }
        native.backup = Some(backup(workspace, retain));
        Ok(())
    }
}
mod execution;
