//! Bounded status publication and manifest-bound visibility.
use super::*;

pub(super) struct StatusCell {
    pub(super) visible: Status,
    pub(super) terminal: Option<Status>,
}

/// Bounded immutable status publication, retained inside the sole manager's job
/// record. This is not a parallel job registry or COMMIT/cancellation gate.
pub struct JobStatus {
    pub(super) current: Mutex<StatusCell>,
    admitted: Mutex<[Option<Artifact>; 2]>,
    pub changed: Notify,
    origin: Instant,
}
impl JobStatus {
    pub fn snapshot(&self) -> Result<Status, FailureKind> {
        self.current
            .lock()
            .map(|s| s.visible.clone())
            .map_err(|_| FailureKind::CatalogueUnavailable)
    }
    pub fn monotonic_origin(&self) -> Instant {
        self.origin
    }

    pub fn server_tick_ms(&self) -> Result<U64, FailureKind> {
        u64::try_from(self.origin.elapsed().as_millis())
            .map(U64)
            .map_err(|_| FailureKind::CounterExhausted)
    }
    /// New snapshots must be complete and bounded. Terminal snapshots are held
    /// pending until the persistent supervisor JOINED the actual worker and
    /// checked native ownership, never exposed merely because a worker returned.
    pub fn publish(&self, mut next: Status) -> Result<(), FailureKind> {
        if next.state == JobState::Running
            && next.phase != Some(Phase::Settling)
            && next.revision.get() >= u64::MAX - crate::catalogue_jobs::status::SETTLEMENT_RESERVE
        {
            return Err(FailureKind::CounterExhausted);
        }
        next.validate().map_err(|_| FailureKind::ReportMismatch)?;
        bounded_json(&next, crate::catalogue_jobs::status::STATUS_BYTES)?;
        let mut cell = self
            .current
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        let old = &cell.visible;
        if next.handle != old.handle
            || next.profile != old.profile
            || next.source.name != old.source.name
            || next.source.declared_bytes != old.source.declared_bytes
            || next.source.received_bytes < old.source.received_bytes
            || old
                .source
                .sha256
                .is_some_and(|sha| next.source.sha256 != Some(sha))
            || old.preview.is_some() && next.preview != old.preview
            || old.result.is_some() && next.result != old.result
            || old.gate != Gate::Open && next.gate != old.gate
            || old.stop_cause.is_some() && next.stop_cause != old.stop_cause
            || old.apply.accepted_command_id.is_some()
                && next.apply.accepted_command_id != old.apply.accepted_command_id
            || next.revision <= old.revision
            || old.state != JobState::Running
            || old.commit.outcome == CommitOutcome::Committed && next.commit != old.commit
        {
            return Err(FailureKind::ReportMismatch);
        }
        let admitted = self
            .admitted
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        if next
            .preview
            .as_ref()
            .is_some_and(|p| admitted[0].as_ref() != Some(&p.artifact))
            || next
                .result
                .as_ref()
                .is_some_and(|r| admitted[1].as_ref() != Some(&r.artifact))
        {
            return Err(FailureKind::ReportMismatch);
        }
        if next.state != JobState::Running {
            let mut settling = next.clone();
            settling.state = JobState::Running;
            settling.phase = Some(Phase::Settling);
            settling.progress = None;
            settling.cleanup = Cleanup {
                disposition: CleanupDisposition::Pending,
                failure_kind: None,
            };
            settling.lease = old.lease.clone();
            if settling.lease.kind != LeaseKind::Working {
                settling.lease.kind = LeaseKind::Working;
                settling.lease.deadline_tick_ms = U64(self
                    .server_tick_ms()?
                    .get()
                    .checked_add(7_200_000)
                    .ok_or(FailureKind::CounterExhausted)?);
            }
            settling
                .validate()
                .map_err(|_| FailureKind::ReportMismatch)?;
            next.revision = next
                .revision
                .checked_add(U64(1))
                .ok_or(FailureKind::CounterExhausted)?;
            cell.visible = settling;
            cell.terminal = Some(next);
        } else {
            cell.visible = next;
        }
        drop(cell);
        self.changed.notify_waiters();
        Ok(())
    }
    pub fn admit_artifact(
        &self,
        workspace: &JobWorkspace,
        artifact: &Artifact,
    ) -> Result<(), FailureKind> {
        let index = match artifact.kind {
            ArtifactKind::Preview => 0,
            ArtifactKind::Result => 1,
        };
        let current = self.snapshot()?;
        if current.handle.job_id != workspace.handle().job
            || workspace.published[index].as_ref() != Some(artifact)
        {
            return Err(FailureKind::ReportMismatch);
        }
        let mut admitted = self
            .admitted
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        if admitted[index].as_ref().is_some_and(|old| old != artifact) {
            return Err(FailureKind::ReportMismatch);
        }
        admitted[index] = Some(artifact.clone());
        Ok(())
    }
    pub(super) fn update(&self, change: impl FnOnce(&mut Status)) -> Result<(), FailureKind> {
        let mut next = self.snapshot()?;
        next.revision = next
            .revision
            .checked_add(U64(1))
            .ok_or(FailureKind::CounterExhausted)?;
        change(&mut next);
        self.publish(next)
    }
    pub(super) fn finish_joined(&self) -> Result<bool, FailureKind> {
        let mut cell = self
            .current
            .lock()
            .map_err(|_| FailureKind::CatalogueUnavailable)?;
        let Some(mut terminal) = cell.terminal.take() else {
            return Ok(false);
        };
        terminal.lease = Lease {
            kind: LeaseKind::Retention,
            deadline_tick_ms: U64(self
                .server_tick_ms()?
                .get()
                .checked_add(3_600_000)
                .ok_or(FailureKind::CounterExhausted)?),
        };
        terminal
            .validate()
            .map_err(|_| FailureKind::ReportMismatch)?;
        bounded_json(&terminal, crate::catalogue_jobs::status::STATUS_BYTES)?;
        cell.visible = terminal;
        drop(cell);
        self.changed.notify_waiters();
        Ok(true)
    }
}

pub(super) fn initial(
    handle: crate::catalogue_jobs::Handle,
    create: &Create,
    origin: Instant,
    receiving_deadline: U64,
) -> JobStatus {
    JobStatus {
        current: Mutex::new(StatusCell {
            visible: Status {
                schema_version: 1,
                policy_version: POLICY_VERSION.into(),
                handle: wire_handle(handle),
                revision: U64(1),
                source: Source {
                    name: create.source_name.clone(),
                    declared_bytes: create.declared_bytes,
                    received_bytes: U64(0),
                    sha256: None,
                },
                profile: Profile {
                    name: create.profile,
                    max_member_bytes: U64(MAX_MEMBER_BYTES),
                    max_expanded_bytes: U64(if create.profile == ProfileName::Large {
                        LARGE_EXPANDED_BYTES
                    } else {
                        STANDARD_EXPANDED_BYTES
                    }),
                },
                state: JobState::Running,
                phase: Some(Phase::Receiving),
                gate: Gate::Open,
                stop_cause: None,
                progress: Some(Progress {
                    stage: ProgressStage::Receiving,
                    pass: U64(1),
                    unit: ProgressUnit::Bytes,
                    completed: U64(0),
                    total: Some(create.declared_bytes),
                }),
                lease: Lease {
                    kind: LeaseKind::Receiving,
                    deadline_tick_ms: receiving_deadline,
                },
                apply: Apply {
                    state: ApplyState::Unavailable,
                    accepted_command_id: None,
                },
                preview: None,
                result: None,
                failure: None,
                commit: Commit {
                    outcome: CommitOutcome::Pending,
                    evidence: CommitEvidence::NotAttempted,
                },
                cleanup: Cleanup {
                    disposition: CleanupDisposition::Pending,
                    failure_kind: None,
                },
                backup: Backup {
                    state: BackupState::None,
                    name: None,
                    identity: None,
                    publication: BackupPublication::None,
                },
            },
            terminal: None,
        }),
        admitted: Mutex::new([None, None]),
        changed: Notify::new(),
        origin,
    }
}
