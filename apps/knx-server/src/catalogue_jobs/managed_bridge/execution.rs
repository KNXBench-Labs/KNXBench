//! Native workers and the same-gate publication/report callbacks.
use super::*;

impl CommitArbitration for Session {
    fn gate(&self) -> &PublicationGate {
        self.control.gate()
    }
    fn claim_commit(&self) -> Option<CommitClaim<'_>> {
        let _serial = self.transition.lock().ok()?;
        let claim = self.control.claim_commit();
        // Publish while the same short transition is serialized, before native
        // COMMIT. A projection refusal consumes no invented successful receipt.
        if self.sync_control(self.control.snapshot().ok()?).is_err() {
            return None;
        }
        claim
    }
}
struct ResultSink<'a> {
    session: &'a Session,
    output: &'a mut Option<JobResult>,
    failure: &'a mut Option<FailureKind>,
}
impl JobResultAdmission for ResultSink<'_> {
    fn recovery_published(
        &mut self,
        workspace: &JobWorkspace,
    ) -> Result<(), db::RecoveryPublicationError> {
        let result = (|| {
            let _serial = self
                .session
                .transition
                .lock()
                .map_err(|_| FailureKind::CatalogueUnavailable)?;
            let v = self.session.control.snapshot().map_err(control_error)?;
            if v.gate != PublicationState::Open {
                return Err(FailureKind::Cancelled);
            }
            self.session.status.update(|s| {
                s.phase = Some(Phase::Installing);
                s.backup = backup(workspace, false);
            })
        })();
        result.map_err(|error| {
            *self.failure = Some(error);
            db::RecoveryPublicationError::Refused
        })
    }
    fn admit_result(
        &mut self,
        install: &knx_productdb::ProductFileInstall,
        bytes: u64,
        workspace: &mut JobWorkspace,
    ) -> Result<(), db::RecoveryPublicationError> {
        let result = (|| {
            let shown = self
                .session
                .control
                .binding()
                .get()
                .ok_or(FailureKind::ReportMismatch)?;
            let artifact = report_adapter::write(
                workspace,
                ArtifactKind::Result,
                install,
                shown.report(),
                bytes,
            )?;
            self.session.status.admit_artifact(workspace, &artifact)?;
            *self.output = Some(JobResult {
                artifact,
                catalogue_page_bytes: U64(bytes),
                next_job_input_limit_exceeded: bytes > db::CATALOGUE_INPUT_BYTE_LIMIT,
            });
            Ok(())
        })();
        result.map_err(|e| {
            *self.failure = Some(e);
            db::RecoveryPublicationError::Refused
        })
    }
}
impl CatalogueJobExecution for Session {
    fn request_stop(&self, c: StopCause) -> Result<CancelOutcome, FailureKind> {
        let _serial = self
            .transition
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        if self.finished.load(Ordering::SeqCst) {
            return match self.status.snapshot()?.gate {
                Gate::CommitClaimed => Ok(CancelOutcome::TooLate),
                Gate::CancelAccepted => Ok(CancelOutcome::AlreadyRequested),
                Gate::Open => Err(FailureKind::InvalidPhase),
            };
        }
        let out = self
            .control
            .request_stop(self.identity, cause(c))
            .map_err(control_error)?;
        self.sync_control(self.control.snapshot().map_err(control_error)?)?;
        Ok(match out {
            knx_productdb::operation::CancelOutcome::Accepted => CancelOutcome::Requested,
            knx_productdb::operation::CancelOutcome::AlreadyAccepted => {
                CancelOutcome::AlreadyRequested
            }
            knx_productdb::operation::CancelOutcome::TooLate => CancelOutcome::TooLate,
        })
    }
    fn accept_apply(&self, c: &ApplyCommand) -> Result<bool, FailureKind> {
        let _serial = self
            .transition
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        let command = ctl::Command {
            command: *c.command_id.as_bytes(),
            consent: *c.consent_id.as_bytes(),
            artifact: *c.preview_artifact_id.as_bytes(),
            digest: *c.preview_public_sha256.as_bytes(),
            confirmed: c.confirmed,
        };
        let result = self.control.apply(self.identity, command);
        match result {
            Ok(ctl::Apply::Replay(_)) => Ok(false),
            Ok(ctl::Apply::AcceptedOnce(v)) => {
                self.sync_control(v)?;
                Ok(true)
            }
            Err(e) => {
                let _ = self.sync_control(self.control.snapshot().map_err(control_error)?);
                Err(control_error(e))
            }
        }
    }
    fn expire(&self) -> Result<(), FailureKind> {
        let _serial = self
            .transition
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        if self.finished.load(Ordering::SeqCst) {
            return Ok(());
        }
        if let ctl::Expiry::Stop(_, v) = self
            .control
            .expire(self.control.ticket().map_err(control_error)?)
            .map_err(control_error)?
        {
            self.sync_control(v)?;
        }
        Ok(())
    }
    fn preview(&self, source: &[u8], workspace: &mut JobWorkspace) -> Result<(), FailureKind> {
        {
            let _serial = self
                .transition
                .lock()
                .map_err(|_| FailureKind::CatalogueUnavailable)?;
            let s = self.status.snapshot()?;
            self.control
                .begin_preview(s.lease.deadline_tick_ms.get())
                .map_err(control_error)?;
        }
        let mut native = self
            .native
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        native.started = true;
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), FailureKind> {
            let state = self
                .state
                .upgrade()
                .ok_or(FailureKind::CatalogueUnavailable)?;
            let owner = state
                .product_db
                .as_ref()
                .ok_or(FailureKind::CatalogueUnavailable)?;
            let s = self.status.snapshot()?;
            native.workspace = Some(
                ApplyWorkspace::in_owned_directory(workspace.directory().to_path_buf())
                    .map_err(|_| FailureKind::IoFailure)?,
            );
            let start = Instant::now();
            loop {
                let Native {
                    workspace: native_workspace,
                    result,
                    result_failure,
                    managed_settled,
                    ..
                } = &mut *native;
                let mut sink = ResultSink {
                    session: self,
                    output: result,
                    failure: result_failure,
                };
                let mut publisher = JobRecoveryPublisher::new(
                    workspace,
                    &self.catalogue,
                    s.source.sha256.ok_or(FailureKind::ReportMismatch)?,
                    &mut sink,
                );
                *managed_settled = false;
                let outcome = controlled_apply::prepare_owned(
                    owner,
                    self.source(&s.source.name, source, s.profile.name)?,
                    &self.control,
                    native_workspace.as_mut().ok_or(FailureKind::WorkerFailed)?,
                    &mut publisher,
                );
                *managed_settled = match &outcome {
                    Ok(())
                    | Err(
                        controlled_apply::BridgeError::Control(_)
                        | controlled_apply::BridgeError::PreviewPending
                        | controlled_apply::BridgeError::PreviewAlreadyBound,
                    ) => true,
                    Err(controlled_apply::BridgeError::Owner(
                        ControlledOwnerError::Busy | ControlledOwnerError::Stopped,
                    )) => true,
                    Err(controlled_apply::BridgeError::Owner(ControlledOwnerError::Preview(
                        error,
                    ))) => error.native_settled(),
                    _ => false,
                };
                match outcome {
                    Err(controlled_apply::BridgeError::Owner(ControlledOwnerError::Busy)) => {
                        self.check_wait(start)?
                    }
                    other => {
                        other.map_err(bridge_error)?;
                        break;
                    }
                }
            }
            self.settle_workspace(&mut native, workspace, false)?;
            workspace
                .cleanup_rehearsal()
                .map_err(|_| FailureKind::CleanupFailed)?;
            let shown = self
                .control
                .binding()
                .get()
                .ok_or(FailureKind::ReportMismatch)?;
            let artifact = report_adapter::write(
                workspace,
                ArtifactKind::Preview,
                shown.install(),
                shown.report(),
                0,
            )?;
            self.status.admit_artifact(workspace, &artifact)?;
            let consent = JobId::try_mint().map_err(|_| FailureKind::IoFailure)?;
            let preview =
                report_adapter::summary(shown.install(), shown.report(), artifact, consent)?;
            let _serial = self
                .transition
                .lock()
                .map_err(|_| FailureKind::CatalogueUnavailable)?;
            let v = self
                .control
                .enter_ready(
                    self.control.ticket().map_err(control_error)?,
                    ctl::ReadyEvidence {
                        consent: *consent.as_bytes(),
                        artifact: *preview.artifact.artifact_id.as_bytes(),
                        digest: *preview.artifact.public_sha256.as_bytes(),
                    },
                )
                .map_err(control_error)?;
            self.status.update(|s| {
                s.preview = Some(preview);
                s.phase = Some(Phase::Ready);
                s.apply.state = ApplyState::Available;
                s.progress = None;
                s.lease = Lease {
                    kind: LeaseKind::Ready,
                    deadline_tick_ms: U64(v.lease.deadline_tick_ms),
                };
            })?;
            Ok(())
        }));
        match result {
            Ok(Ok(())) => Ok(()),
            other => {
                let e = match other {
                    Ok(Err(e)) => e,
                    _ => FailureKind::WorkerFailed,
                };
                let cleanup = self.settle_workspace(&mut native, workspace, false);
                self.finish(
                    Some(if cleanup.is_err() {
                        FailureKind::CleanupFailed
                    } else {
                        e
                    }),
                    &native,
                )?;
                Ok(())
            }
        }
    }
    fn apply(&self, source: &[u8], workspace: &mut JobWorkspace) -> Result<(), FailureKind> {
        let mut native = self
            .native
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        native.closed = false;
        let result = catch_unwind(AssertUnwindSafe(|| -> Result<(), FailureKind> {
            let state = self
                .state
                .upgrade()
                .ok_or(FailureKind::CatalogueUnavailable)?;
            let owner = state
                .product_db
                .as_ref()
                .ok_or(FailureKind::CatalogueUnavailable)?;
            let s = self.status.snapshot()?;
            native.workspace = Some(
                ApplyWorkspace::in_owned_directory(workspace.directory().to_path_buf())
                    .map_err(|_| FailureKind::IoFailure)?,
            );
            let start = Instant::now();
            loop {
                self.expire()?;
                let Native {
                    workspace: native_workspace,
                    receipt,
                    result,
                    result_failure,
                    managed_settled,
                    ..
                } = &mut *native;
                let mut sink = ResultSink {
                    session: self,
                    output: result,
                    failure: result_failure,
                };
                let mut publisher = JobRecoveryPublisher::new(
                    workspace,
                    &self.catalogue,
                    s.source.sha256.ok_or(FailureKind::ReportMismatch)?,
                    &mut sink,
                );
                *managed_settled = false;
                let execution = owner.try_apply_controlled(
                    self.source(&s.source.name, source, s.profile.name)?,
                    self.control
                        .binding()
                        .get()
                        .ok_or(FailureKind::ReportMismatch)?,
                    self,
                    native_workspace.as_mut().ok_or(FailureKind::WorkerFailed)?,
                    receipt,
                    &mut publisher,
                );
                *managed_settled = match &execution {
                    Ok(execution) => execution.native_settled(),
                    Err(ControlledOwnerError::Busy | ControlledOwnerError::Stopped) => true,
                    _ => false,
                };
                match execution {
                    Err(ControlledOwnerError::Busy) => self.check_wait(start)?,
                    Ok(execution) => {
                        native.execution = Some(execution);
                        break;
                    }
                    Err(ControlledOwnerError::Stopped) => return Err(FailureKind::Cancelled),
                    Err(_) => return Err(FailureKind::CatalogueUnavailable),
                }
            }
            Ok(())
        }));
        let error = match result {
            Ok(Ok(())) => native.result_failure.or_else(|| {
                native
                    .execution
                    .as_ref()
                    .and_then(|e| e.failure_kind())
                    .map(failure_kind)
            }),
            Ok(Err(e)) => Some(e),
            Err(_) => Some(FailureKind::WorkerFailed),
        };
        let retain = matches!(
            native.receipt.witness().map(|w| w.trace()),
            Some(db::LiveCommitTrace::LiveCommitReturnedOk { autocommit: true })
        ) || native.execution.as_ref().is_none_or(|e| {
            // NotCommitted alone is not disposal authority: restoration can
            // fail after a successful rollback and require native Retain.
            e.artifacts() != db::ArtifactDisposition::Dispose
                || e.commit() != db::CommitEvidence::NotCommitted
                || e.rollback() == db::RollbackDisposition::Unresolved
                || e.cleanup_failed()
                || !e.native_settled()
        });
        let cleanup = self.settle_workspace(&mut native, workspace, retain);
        if cleanup.is_err() {
            workspace.quarantine();
            native.backup = Some(backup(workspace, true));
        }
        self.finish(error.or(cleanup.err()), &native)
    }
    fn receive_failed(&self, f: FailureKind) {
        if matches!(
            f,
            FailureKind::ReceivingTimeout | FailureKind::ReceivingIdleTimeout
        ) {
            let _ = self.request_stop(if f == FailureKind::ReceivingTimeout {
                StopCause::ReceivingTimeout
            } else {
                StopCause::ReceivingIdleTimeout
            });
        }
        if let Ok(mut n) = self.native.try_lock() {
            if !n.started {
                n.closed = true;
                let _ = self.finish(Some(f), &n);
            }
        }
    }
    fn settle_idle(&self) {
        if let Ok(n) = self.native.try_lock() {
            if !self.finished.load(Ordering::SeqCst) {
                let _ = self.finish(Some(FailureKind::Cancelled), &n);
            }
        }
    }
    fn operation_failed(&self, f: FailureKind) {
        if let Ok(n) = self.native.try_lock() {
            let _ = self.finish(Some(f), &n);
        }
    }
    fn worker_lost(&self) {
        self.operation_failed(FailureKind::WorkerFailed);
    }
    fn handles_closed(&self) -> bool {
        self.native.try_lock().is_ok_and(|n| !n.started || n.closed)
    }
}
