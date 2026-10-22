//! Revision counter, monotonic lease deadlines and status snapshot assembly.

use super::dto::{
    to_string, Apply, Backup, Cleanup, Commit, Failure, Gate, Handle, JobResult, JobState, Lease,
    Phase, Preview, Profile, Progress, Source, Status, StopCause, POLICY_VERSION, SCHEMA_VERSION,
};
use super::strict::U64;

/// Serialization bound for a `Status` (policy `caps.statusBytes`).
pub const STATUS_BYTES: usize = 16_384;
/// The last 64 revisions are reserved for settlement; ordinary bumps stop.
pub const SETTLEMENT_RESERVE: u64 = 64;

/// A status-snapshot failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusError {
    CounterExhausted,
    DeadlineOverflow,
    InvalidState,
    Serialize,
    TooLarge,
}

/// A checked, never-wrapping revision counter (starts at 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Revision(U64);

impl Revision {
    pub fn initial() -> Self {
        Revision(U64::ONE)
    }

    pub fn get(self) -> U64 {
        self.0
    }

    /// Checked increment; refuses once the settlement reserve is reached.
    pub fn bump(&mut self) -> Result<U64, StatusError> {
        let next = self
            .0
            .checked_add(U64::ONE)
            .ok_or(StatusError::CounterExhausted)?;
        if next.get() > u64::MAX - SETTLEMENT_RESERVE {
            return Err(StatusError::CounterExhausted);
        }
        self.0 = next;
        Ok(next)
    }
}

/// A monotonic lease deadline: `now_tick_ms + duration_ms`, checked, never wraps.
pub fn deadline_after(now_tick_ms: u64, duration_ms: u64) -> Result<U64, StatusError> {
    now_tick_ms
        .checked_add(duration_ms)
        .map(U64)
        .ok_or(StatusError::DeadlineOverflow)
}

/// The complete job-state snapshot a status is assembled from.
#[derive(Debug, Clone)]
pub struct StatusSnapshot {
    pub handle: Handle,
    pub source: Source,
    pub profile: Profile,
    pub state: JobState,
    pub phase: Option<Phase>,
    pub gate: Gate,
    pub stop_cause: Option<StopCause>,
    pub progress: Option<Progress>,
    pub lease: Lease,
    pub apply: Apply,
    pub preview: Option<Preview>,
    pub result: Option<JobResult>,
    pub failure: Option<Failure>,
    pub commit: Commit,
    pub cleanup: Cleanup,
    pub backup: Backup,
}

/// Assembles a `Status`, validates its state contract and enforces the 16 KiB
/// serialization bound.
pub fn assemble_status(snapshot: StatusSnapshot, revision: U64) -> Result<Status, StatusError> {
    let status = Status {
        schema_version: SCHEMA_VERSION,
        policy_version: POLICY_VERSION.to_string(),
        handle: snapshot.handle,
        revision,
        source: snapshot.source,
        profile: snapshot.profile,
        state: snapshot.state,
        phase: snapshot.phase,
        gate: snapshot.gate,
        stop_cause: snapshot.stop_cause,
        progress: snapshot.progress,
        lease: snapshot.lease,
        apply: snapshot.apply,
        preview: snapshot.preview,
        result: snapshot.result,
        failure: snapshot.failure,
        commit: snapshot.commit,
        cleanup: snapshot.cleanup,
        backup: snapshot.backup,
    };
    status.validate().map_err(|_| StatusError::InvalidState)?;
    let wire = to_string(&status).map_err(|_| StatusError::Serialize)?;
    if wire.len() > STATUS_BYTES {
        return Err(StatusError::TooLarge);
    }
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalogue_jobs::dto::{
        Apply, ApplyState, Backup, BackupPublication, BackupState, Cleanup, CleanupDisposition,
        Commit, CommitEvidence, CommitOutcome, Gate, Handle, JobState, Lease, LeaseKind, Phase,
        Profile, ProfileName, Progress, ProgressStage, ProgressUnit, Source, StopCause,
    };
    use crate::catalogue_jobs::strict::{Incarnation, JobId, U64};

    fn u64(v: u64) -> U64 {
        U64(v)
    }

    fn id(b: u8) -> JobId {
        JobId::from_bytes([b; 16])
    }

    fn inc(b: u8) -> Incarnation {
        Incarnation::from_bytes([b; 16])
    }

    fn receiving_snapshot() -> StatusSnapshot {
        StatusSnapshot {
            handle: Handle {
                server_incarnation: inc(0x01),
                admission_serial: u64(1),
                job_id: id(0x02),
            },
            source: Source {
                name: "sample.knxprod".to_string(),
                declared_bytes: u64(128),
                received_bytes: u64(64),
                sha256: None,
            },
            profile: Profile {
                name: ProfileName::Standard,
                max_member_bytes: u64(1_000_000_000),
                max_expanded_bytes: u64(4_294_967_296),
            },
            state: JobState::Running,
            phase: Some(Phase::Receiving),
            gate: Gate::Open,
            stop_cause: None,
            progress: Some(Progress {
                stage: ProgressStage::Receiving,
                pass: u64(1),
                unit: ProgressUnit::Bytes,
                completed: u64(64),
                total: Some(u64(128)),
            }),
            lease: Lease {
                kind: LeaseKind::Receiving,
                deadline_tick_ms: u64(1000),
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
        }
    }

    #[test]
    fn revision_starts_at_one_and_bumps_checked() {
        let mut rev = Revision::initial();
        assert_eq!(rev.get(), u64(1));
        assert_eq!(rev.bump().unwrap(), u64(2));
        assert_eq!(rev.bump().unwrap(), u64(3));
    }

    #[test]
    fn revision_reserves_the_last_64() {
        let mut rev = Revision(U64(u64::MAX - 65));
        assert!(rev.bump().is_ok()); // enters MAX-64
        assert_eq!(rev.bump(), Err(StatusError::CounterExhausted));
    }

    #[test]
    fn deadline_after_checks_overflow() {
        assert_eq!(deadline_after(10, 20).unwrap(), u64(30));
        assert_eq!(
            deadline_after(u64::MAX, 1),
            Err(StatusError::DeadlineOverflow)
        );
    }

    #[test]
    fn assemble_status_produces_valid_bounded_status() {
        let snapshot = receiving_snapshot();
        let status = assemble_status(snapshot, u64(1)).unwrap();
        assert_eq!(status.revision, u64(1));
        assert!(status.validate().is_ok());
    }

    #[test]
    fn assemble_status_rejects_invalid_state() {
        let mut snapshot = receiving_snapshot();
        snapshot.stop_cause = Some(StopCause::UserCancel); // gate is Open
        assert_eq!(
            assemble_status(snapshot, u64(1)),
            Err(StatusError::InvalidState)
        );
    }
}
