//! Durable, operation-specific property validation protocol; no networking here.
//!
//! Each checkpoint is immutable, create-new, owner-only, file/ancestor-synced
//! and byte-readback verified before returning permission to the Net callback.
//! A fresh opener replays the complete sequence under the same exclusive lock.
//! Hash chaining and a synced atomic HEAD detect corruption and missing tails,
//! not malicious rewriting; an unfinished operation is never harmless.
use crate::property_validation_bundle::{
    digest, invalid, OperationRecord, PointerObservationRecord, PreparedBundle, RecoveryRecord,
};
#[path = "property_validation_diagnostic.rs"]
mod diagnostic;
#[path = "property_validation_lifecycle.rs"]
mod lifecycle;
#[path = "property_validation_result.rs"]
mod result;
pub use diagnostic::{
    FailureDiagnostic, FailureDiagnosticRun, PointerFailureReason, PointerFailureRecord,
};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::property_recovery::PropertyRecoverySnapshot;
pub use lifecycle::{
    System2ControlError, System2ControlSnapshot, System2DrainOutcome, System2OperationControl,
    System2OperationLease, System2Settlement,
};
pub use result::{is_build_version, DownloadFacts, DownloadResult, ExecutionBuild, System2Report};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

/// A journaled pointer observation must be admitted by the bundle's width rule and be a
/// real difference: a miss of the source model or a width change from the original prestate.
fn check_genuine_observation(
    snapshot: &PropertyRecoverySnapshot,
    record: &PointerObservationRecord,
    source: Option<&[u8; 4]>,
) -> io::Result<()> {
    let observation = record.to_observation(&snapshot.preconditions.pointer_widths)?;
    if source.is_some_and(|expected| *expected != record.expected)
        || !snapshot.pointer_needs_observation(
            record.object,
            &record.expected,
            observation.observed,
        )
    {
        return Err(invalid(
            "pointer observation is not a difference from the source or prestate",
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Binding {
    pub target: String,
    pub serial: [u8; 6],
    pub mask: u16,
    pub manufacturer: u16,
    pub hardware: [u8; 6],
    pub application: [u8; 5],
    pub profile_id: String,
    pub profile_version: u32,
    pub profile_sha256: String,
    pub source_sha256: String,
    pub plan_sha256: String,
    pub simulated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_preparation_sha256: Option<String>,
}
impl Binding {
    pub(crate) fn validate(&self) -> io::Result<()> {
        let address = self
            .target
            .parse::<IndividualAddress>()
            .map_err(|_| invalid("invalid recovery target"))?;
        ContactableAddress::new(address).map_err(|_| invalid("excluded recovery target"))?;
        let hash = |s: &str| {
            s.len() == 64
                && s.bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        };
        if self.target != address.to_string()
            || self.mask != 0x0025
            || self.profile_id.is_empty()
            || self.profile_id.len() > 128
            || !self
                .profile_id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-_.".contains(&b))
            || self.profile_version == 0
            || !hash(&self.profile_sha256)
            || !hash(&self.source_sha256)
            || !hash(&self.plan_sha256)
            || self
                .project_preparation_sha256
                .as_ref()
                .is_some_and(|s| !hash(s))
        {
            return Err(invalid(
                "invalid recovery identity/profile/source/plan binding",
            ));
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocalOutcome {
    Running,
    Interrupted,
    Finished,
    Failed,
    Cancelled,
    Restoring,
    Restored,
    RestoreFailed,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureCode {
    Executor,
    Persistence,
    Cancelled,
    IdentityOrRecovery,
}
/// Private protocol events. Raw diagnostics stay local; the concrete simulator
/// adapter uses no access key. Never serialize this enum as a shareable report.
// One short-lived value per durable write; boxing would only churn every caller.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
pub enum Checkpoint {
    Created {
        binding: Binding,
    },
    RecoveryAcquired {
        recovery: RecoveryRecord,
    },
    StepStarted {
        index: usize,
        mutation: bool,
    },
    StepFinished {
        index: usize,
    },
    /// Final pointer reads that differed from the source model after an otherwise
    /// verified outcome. Observation only; Alpha 16 journals never contain it.
    FinalPointerObservations {
        observations: Vec<PointerObservationRecord>,
    },
    /// An allocation/task pointer read-back that differed from its record,
    /// persisted before the next mutation. Observation only (amendment 3).
    InProcedurePointerObservation {
        operation_index: usize,
        observation: PointerObservationRecord,
    },
    Failed {
        code: FailureCode,
        local_diagnostic: String,
        /// Typed, non-raw facts of a PID 7 width refusal. Absent in every older journal and
        /// for every other failure; never backfilled from `local_diagnostic`.
        #[serde(
            default,
            deserialize_with = "diagnostic::present_record",
            skip_serializing_if = "Option::is_none"
        )]
        pointer_failure: Option<PointerFailureRecord>,
    },
    Cancelled,
    Finished,
    RestoreStarted,
    RestoreFailed {
        code: FailureCode,
        local_diagnostic: String,
        #[serde(
            default,
            deserialize_with = "diagnostic::present_record",
            skip_serializing_if = "Option::is_none"
        )]
        pointer_failure: Option<PointerFailureRecord>,
    },
    Restored,
}
impl Checkpoint {
    /// Envelope version 2 marks exactly a populated typed failure record, so an older strict
    /// reader (version 1 only) refuses it instead of dropping the field; all else stays 1.
    fn envelope_format(&self) -> u16 {
        match self {
            Self::Failed {
                pointer_failure: Some(_),
                ..
            }
            | Self::RestoreFailed {
                pointer_failure: Some(_),
                ..
            } => 2,
            _ => 1,
        }
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: u16,
    kind: String,
    sequence: usize,
    previous_sha256: String,
    checkpoint: Checkpoint,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Head {
    format: u16,
    sequence: usize,
    sha256: String,
}
#[derive(Clone)]
struct State {
    binding: Binding,
    recovery: Option<RecoveryRecord>,
    /// The original recovery, validated once when `RecoveryAcquired` is applied (live or on
    /// replay). Later observation checks borrow it; never serialized, never replaced.
    snapshot: Option<Arc<PropertyRecoverySnapshot>>,
    active: Option<usize>,
    completed: usize,
    mutation: bool,
    outcome: LocalOutcome,
    restore_attempted: bool,
    final_observations: Vec<(PointerObservationRun, PointerObservationRecord)>,
    in_procedure_observations: Vec<(PointerObservationRun, usize, PointerObservationRecord)>,
    /// One typed failure per run at most: `Failed` and `RestoreFailed` each end their run.
    failure_diagnostics: Vec<FailureDiagnostic>,
    /// Frozen when the download run ends (terminal checkpoint or restore start).
    download_end: Option<DownloadFacts>,
}
impl State {
    fn new(binding: Binding) -> Self {
        Self {
            binding,
            recovery: None,
            snapshot: None,
            active: None,
            completed: 0,
            mutation: false,
            outcome: LocalOutcome::Running,
            restore_attempted: false,
            final_observations: Vec::new(),
            in_procedure_observations: Vec::new(),
            failure_diagnostics: Vec::new(),
            download_end: None,
        }
    }
    fn facts(&self, result: DownloadResult, outcome: LocalOutcome) -> DownloadFacts {
        DownloadFacts {
            result,
            outcome,
            completed_steps: self.completed,
            total_steps: self
                .recovery
                .as_ref()
                .map(|r| r.original_bundle.normalized_plan.steps.len()),
        }
    }
    /// What an ended, not-`Finished` download provably achieved; see [`DownloadResult`].
    fn ended_result(&self) -> DownloadResult {
        let Some(recovery) = &self.recovery else {
            return if self.mutation {
                DownloadResult::Unknown
            } else {
                DownloadResult::NothingWritten
            };
        };
        let steps = &recovery.original_bundle.normalized_plan.steps;
        let Some(last) = steps.len().checked_sub(1) else {
            return DownloadResult::Unknown;
        };
        if !matches!(steps[last].operation, OperationRecord::Disconnect) {
            return DownloadResult::Unknown;
        }
        match (self.completed, self.active) {
            (done, _) if done < last => DownloadResult::TransferIncomplete,
            (done, None) if done == last => DownloadResult::TransferCompleteVerificationNotStarted,
            (done, Some(active)) if done == last && active == last => {
                DownloadResult::TransferCompleteVerificationUnfinished
            }
            _ => DownloadResult::Unknown,
        }
    }
    /// The run an observation belongs to: an explicit restore or the download.
    fn run(&self) -> PointerObservationRun {
        if self.outcome == LocalOutcome::Restoring {
            PointerObservationRun::Restore
        } else {
            PointerObservationRun::Download
        }
    }
    /// Captured prestate does not describe the source shape or application, or
    /// carries a placeholder serial that cannot bind a replay to one device:
    /// evidence only, original-state recovery unavailable.
    fn prestate_differs(&self) -> bool {
        self.recovery.as_ref().is_some_and(|r| {
            let serial = r
                .observed_serial
                .unwrap_or(r.original_bundle.binding.serial);
            !r.prestate_observations.is_empty()
                || r.observed_program_version.is_some()
                || !r.prestate_unreadable_allocations.is_empty()
                || serial == [0; 6]
                || serial == [0xFF; 6]
        })
    }
    /// The durable original prestate, the width reference of every later observation.
    fn recovery_snapshot(&self) -> io::Result<Arc<PropertyRecoverySnapshot>> {
        self.snapshot
            .clone()
            .ok_or_else(|| invalid("pointer observation without durable recovery"))
    }
    fn apply(&mut self, event: &Checkpoint) -> io::Result<()> {
        match event {
            Checkpoint::Created { .. } => return Err(invalid("duplicate operation header")),
            Checkpoint::RecoveryAcquired { recovery } => {
                if self.outcome != LocalOutcome::Running
                    || self.recovery.is_some()
                    || recovery.original_bundle.binding != self.binding
                {
                    return Err(invalid("unexpected recovery or binding mismatch"));
                }
                let snapshot = recovery.to_snapshot(&recovery.original_bundle)?;
                self.snapshot = Some(Arc::new(snapshot));
                self.recovery = Some(recovery.clone());
                // Persisted before VerifyMode: interruption after this barrier
                // cannot prove whether that first mutation was actually sent.
                self.mutation = true;
            }
            Checkpoint::StepStarted { index, mutation } => {
                if !matches!(
                    self.outcome,
                    LocalOutcome::Running | LocalOutcome::Restoring
                ) || self.active.is_some()
                    || *index != self.completed
                {
                    return Err(invalid(
                        "step start is out of order or operation already ended",
                    ));
                }
                if *mutation && self.recovery.is_none() {
                    return Err(invalid("no durable recovery: mutation refused"));
                }
                self.mutation |= mutation;
                self.active = Some(*index);
            }
            Checkpoint::StepFinished { index } => {
                if self.active != Some(*index)
                    || !matches!(
                        self.outcome,
                        LocalOutcome::Running | LocalOutcome::Restoring
                    )
                {
                    return Err(invalid("step finish has no matching start"));
                }
                self.active = None;
                self.completed += 1;
            }
            Checkpoint::FinalPointerObservations { observations } => {
                if self.active.is_none()
                    || self.recovery.is_none()
                    || observations.is_empty()
                    || !matches!(
                        self.outcome,
                        LocalOutcome::Running | LocalOutcome::Restoring
                    )
                {
                    return Err(invalid("pointer observation outside an active final step"));
                }
                let snapshot = self.recovery_snapshot()?;
                for record in observations {
                    let source = snapshot
                        .preconditions
                        .source_pointers
                        .iter()
                        .find(|p| p.object == record.object)
                        .ok_or_else(|| {
                            invalid("final pointer observation names an untouched object")
                        })?;
                    check_genuine_observation(&snapshot, record, Some(&source.value))?;
                }
                let run = self.run();
                self.final_observations
                    .extend(observations.iter().map(|o| (run, o.clone())));
            }
            Checkpoint::InProcedurePointerObservation {
                operation_index,
                observation,
            } => {
                if self.active != Some(*operation_index)
                    || self.recovery.is_none()
                    || !matches!(
                        self.outcome,
                        LocalOutcome::Running | LocalOutcome::Restoring
                    )
                {
                    return Err(invalid("pointer observation outside its active step"));
                }
                // Only an allocation/task record (03 00/01/02) reads a pointer back,
                // and only for its own object and source address.
                let step = self.recovery.as_ref().and_then(|r| {
                    r.original_bundle
                        .normalized_plan
                        .steps
                        .get(*operation_index)
                });
                match step.map(|s| &s.operation) {
                    Some(OperationRecord::LoadControl { object, octets })
                        if octets[0] == 3
                            && octets[1] <= 2
                            && *object == observation.object
                            && observation.expected == [0, 0, octets[2], octets[3]] => {}
                    _ => {
                        return Err(invalid(
                            "pointer observation does not match the active allocation record",
                        ))
                    }
                }
                let snapshot = self.recovery_snapshot()?;
                check_genuine_observation(&snapshot, observation, None)?;
                let run = self.run();
                self.in_procedure_observations
                    .push((run, *operation_index, observation.clone()));
            }
            Checkpoint::Finished => {
                if self.outcome != LocalOutcome::Running
                    || self.active.is_some()
                    || self.recovery.is_none()
                    || self.completed
                        != self
                            .recovery
                            .as_ref()
                            .map_or(0, |r| r.original_bundle.normalized_plan.steps.len())
                {
                    return Err(invalid("incomplete operation cannot finish"));
                }
                self.outcome = LocalOutcome::Finished;
                self.download_end =
                    Some(self.facts(DownloadResult::Verified, LocalOutcome::Finished));
            }
            Checkpoint::Failed { .. } | Checkpoint::Cancelled => {
                if self.outcome != LocalOutcome::Running {
                    return Err(invalid("operation already ended"));
                }
                if let Checkpoint::Failed {
                    code,
                    pointer_failure: Some(record),
                    ..
                } = event
                {
                    self.record_failure(*code, record, FailureDiagnosticRun::Download)?;
                }
                self.download_end = Some(self.facts(
                    self.ended_result(),
                    if matches!(event, Checkpoint::Cancelled) {
                        LocalOutcome::Cancelled
                    } else {
                        LocalOutcome::Failed
                    },
                ));
                self.outcome = if matches!(event, Checkpoint::Cancelled) {
                    LocalOutcome::Cancelled
                } else {
                    LocalOutcome::Failed
                };
            }
            Checkpoint::RestoreStarted => {
                if self.recovery.is_none() || self.restore_attempted || self.prestate_differs() {
                    return Err(invalid(
                        "missing recovery or restore already attempted; no retry",
                    ));
                }
                self.restore_attempted = true;
                if self.download_end.is_none() {
                    self.download_end =
                        Some(self.facts(self.ended_result(), LocalOutcome::Interrupted));
                }
                self.outcome = LocalOutcome::Restoring;
                self.active = None;
                self.completed = 0;
            }
            Checkpoint::RestoreFailed {
                code,
                pointer_failure,
                ..
            } => {
                if self.outcome != LocalOutcome::Restoring {
                    return Err(invalid("no restore in progress"));
                }
                if let Some(record) = pointer_failure {
                    self.record_failure(*code, record, FailureDiagnosticRun::Restore)?;
                }
                self.outcome = LocalOutcome::RestoreFailed;
            }
            Checkpoint::Restored => {
                if self.outcome != LocalOutcome::Restoring
                    || self.active.is_some()
                    || self.completed
                        != self
                            .recovery
                            .as_ref()
                            .map_or(0, |r| r.original_bundle.normalized_plan.steps.len())
                {
                    return Err(invalid("restore is not complete"));
                }
                self.outcome = LocalOutcome::Restored;
            }
        }
        Ok(())
    }
    /// A typed record is valid only under the code Net's width refusal maps to.
    fn record_failure(
        &mut self,
        code: FailureCode,
        record: &PointerFailureRecord,
        run: FailureDiagnosticRun,
    ) -> io::Result<()> {
        record.validate()?;
        if code != FailureCode::IdentityOrRecovery
            || self.failure_diagnostics.iter().any(|d| d.run == run)
        {
            return Err(invalid("typed pointer failure contradicts its checkpoint"));
        }
        self.failure_diagnostics.push(record.diagnostic(run));
        Ok(())
    }
}
/// Lock is held for the complete operation; never resumes implicitly on reopen.
pub struct OperationStore {
    directory: PathBuf,
    _lock: File,
    state: State,
    next: usize,
    previous: String,
    reopened: bool,
    poisoned: bool,
    progress: Option<tokio::sync::watch::Sender<OperationProgress>>,
    /// Caller journal hook, run once when the first possible send is next.
    send_boundary: Option<SendBoundary>,
}
/// Records a caller's durable send intent; an error stops the operation unsent.
pub type SendBoundary = Box<dyn FnMut() -> io::Result<()> + Send>;
/// Local status derived only from durable checkpoints, never synthetic progress.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationProgress {
    pub outcome: LocalOutcome,
    pub completed_steps: usize,
    pub total_steps: Option<usize>,
    pub backup_available: bool,
    pub restore_available: bool,
    pub mutation_possible: bool,
    pub recovery_phrases: Option<OperationRecoveryPhrases>,
    /// The captured prestate differed from the source shape: evidence only,
    /// original-state recovery is unavailable for this operation.
    pub prestate_differs: bool,
    pub pointer_observations: Vec<PointerObservationProgress>,
    /// Plan application versus the device's prior PID_PROGRAM_VERSION (empty:
    /// refused), when they differed.
    pub program_version_observation: Option<([u8; 5], Vec<u8>)>,
    /// Allocations whose prestate read was refused or short. Their captured
    /// bytes are zero placeholders, never a backup.
    pub prestate_unreadable_allocations: Vec<UnreadableAllocation>,
    /// Typed, non-raw failure facts: at most one per run, download before restore.
    pub failure_diagnostics: Vec<FailureDiagnostic>,
    /// Provable achievement of the download run, derived from the journal alone.
    pub download: DownloadFacts,
    /// A recovery was started for this operation: the current outcome and counters are its.
    pub restore_attempted: bool,
}
/// One declared allocation whose prior memory could not be read.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnreadableAllocation {
    pub object: u8,
    pub address: u16,
    pub length: usize,
}
/// Where a pointer observation was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerObservationPhase {
    Prestate,
    InProcedure,
    Final,
}
/// Which run of the durable operation made an observation; both are kept.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PointerObservationRun {
    Download,
    Restore,
}
/// One recorded source-versus-device pointer difference, for status display.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PointerObservationProgress {
    pub run: PointerObservationRun,
    pub phase: PointerObservationPhase,
    /// Plan operation of an in-procedure read-back; `None` otherwise.
    pub operation_index: Option<usize>,
    pub record: PointerObservationRecord,
}
/// Exact acknowledgements from the ORIGINAL retained operation, never current preparation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationRecoveryPhrases {
    pub recovery_phrase: String,
    pub download_phrase: String,
    pub release_phrase: String,
}
impl OperationStore {
    pub fn create(directory: &Path, binding: Binding) -> io::Result<Self> {
        binding.validate()?;
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        {
            use std::os::unix::fs::DirBuilderExt;
            builder.mode(0o700);
        }
        builder.create(directory)?;
        let lock = open_new(&directory.join("operation.lock"))?;
        lock.try_lock().map_err(io::Error::other)?;
        let event = Checkpoint::Created {
            binding: binding.clone(),
        };
        let bytes = encode(&Envelope {
            format: 1,
            kind: "knxbench-property-validation-protocol".into(),
            sequence: 0,
            previous_sha256: String::new(),
            checkpoint: event,
        })?;
        persist(directory, 0, &bytes)?;
        Ok(Self {
            directory: directory.into(),
            _lock: lock,
            state: State::new(binding),
            next: 1,
            previous: digest(&bytes),
            reopened: false,
            poisoned: false,
            progress: None,
            send_boundary: None,
        })
    }
    pub fn open(directory: &Path, expected: &Binding) -> io::Result<Self> {
        expected.validate()?;
        if !fs::symlink_metadata(directory)?.file_type().is_dir() {
            return Err(invalid("recovery directory is missing or aliased"));
        }
        let lock_path = directory.join("operation.lock");
        regular_private(&lock_path)?;
        let lock = OpenOptions::new().read(true).write(true).open(lock_path)?;
        lock.try_lock().map_err(io::Error::other)?;
        regular_private(&directory.join("HEAD"))?;
        let head: Head = serde_json::from_slice(&bounded_read(&directory.join("HEAD"))?)
            .map_err(|_| invalid("corrupt protocol head"))?;
        let mut names = fs::read_dir(directory)?
            .map(|e| e.map(|v| v.file_name()))
            .collect::<io::Result<Vec<_>>>()?;
        names.retain(|n| n != "operation.lock" && n != "HEAD");
        names.sort();
        if names.is_empty() || head.format != 1 || head.sequence.checked_add(1) != Some(names.len())
        {
            return Err(invalid("missing or uncommitted operation protocol"));
        }
        let mut previous = String::new();
        let mut state = State::new(expected.clone());
        for (sequence, name) in names.iter().enumerate() {
            if name != format!("checkpoint-{sequence:08}.json").as_str() {
                return Err(invalid("missing or unexpected protocol checkpoint"));
            }
            let path = directory.join(name);
            regular_private(&path)?;
            let bytes = bounded_read(&path)?;
            let record: Envelope = serde_json::from_slice(&bytes)
                .map_err(|_| invalid("corrupt protocol checkpoint"))?;
            if record.format != record.checkpoint.envelope_format()
                || record.kind != "knxbench-property-validation-protocol"
                || record.sequence != sequence
                || record.previous_sha256 != previous
            {
                return Err(invalid("unsupported or broken protocol chain"));
            }
            if sequence == 0 {
                match record.checkpoint {
                    Checkpoint::Created { binding } if binding == *expected => (),
                    _ => {
                        return Err(invalid(
                            "recovery identity/application/profile/source/plan mismatch",
                        ))
                    }
                }
            } else {
                state.apply(&record.checkpoint)?;
            }
            previous = digest(&bytes);
        }
        if previous != head.sha256 {
            return Err(invalid(
                "protocol head digest differs from latest checkpoint",
            ));
        }
        Ok(Self {
            directory: directory.into(),
            _lock: lock,
            state,
            next: names.len(),
            previous,
            reopened: true,
            poisoned: false,
            progress: None,
            send_boundary: None,
        })
    }
    pub fn checkpoint(&mut self, checkpoint: Checkpoint) -> io::Result<()> {
        if self.poisoned {
            return Err(invalid("protocol persistence failed; operation is blocked"));
        }
        if self.reopened
            && matches!(checkpoint, Checkpoint::StepStarted { .. })
            && self.state.outcome != LocalOutcome::Restoring
        {
            return Err(invalid(
                "reopened operation cannot resume; explicit restore required",
            ));
        }
        if matches!(checkpoint, Checkpoint::RestoreStarted) && !self.reopened {
            return Err(invalid("restore must reopen the durable operation first"));
        }
        let mut next_state = self.state.clone();
        next_state.apply(&checkpoint)?;
        let bytes = encode(&Envelope {
            format: checkpoint.envelope_format(),
            kind: "knxbench-property-validation-protocol".into(),
            sequence: self.next,
            previous_sha256: self.previous.clone(),
            checkpoint,
        })?;
        if let Err(error) = persist(&self.directory, self.next, &bytes) {
            self.poisoned = true;
            if let Some(progress) = &self.progress {
                progress.send_replace(self.progress());
            }
            return Err(error);
        }
        self.state = next_state;
        self.next += 1;
        self.previous = digest(&bytes);
        if let Some(progress) = &self.progress {
            progress.send_replace(self.progress());
        }
        Ok(())
    }
    pub fn progress(&self) -> OperationProgress {
        OperationProgress {
            outcome: self.outcome(),
            completed_steps: self.state.completed,
            total_steps: self
                .state
                .recovery
                .as_ref()
                .map(|r| r.original_bundle.normalized_plan.steps.len()),
            backup_available: self.state.recovery.is_some(),
            restore_available: self.state.recovery.is_some()
                && !self.state.prestate_differs()
                && !self.state.restore_attempted
                && !matches!(
                    self.outcome(),
                    LocalOutcome::Running | LocalOutcome::Restoring
                ),
            mutation_possible: self.state.mutation,
            recovery_phrases: self.recovery_phrases(),
            prestate_differs: self.state.prestate_differs(),
            prestate_unreadable_allocations: self
                .state
                .recovery
                .iter()
                .flat_map(|r| {
                    r.prestate_unreadable_allocations
                        .iter()
                        .filter_map(|&index| r.allocations.get(index))
                })
                .map(|a| UnreadableAllocation {
                    object: a.object,
                    address: a.address,
                    length: a.data.len(),
                })
                .collect(),
            program_version_observation: self.state.recovery.as_ref().and_then(|r| {
                r.observed_program_version
                    .clone()
                    .map(|observed| (r.original_bundle.binding.application, observed))
            }),
            pointer_observations: self
                .state
                .recovery
                .iter()
                .flat_map(|r| &r.prestate_observations)
                .map(|record| {
                    (
                        PointerObservationRun::Download,
                        PointerObservationPhase::Prestate,
                        None,
                        record,
                    )
                })
                .chain(
                    self.state
                        .in_procedure_observations
                        .iter()
                        .map(|(run, index, record)| {
                            (
                                *run,
                                PointerObservationPhase::InProcedure,
                                Some(*index),
                                record,
                            )
                        }),
                )
                .chain(
                    self.state
                        .final_observations
                        .iter()
                        .map(|(run, record)| (*run, PointerObservationPhase::Final, None, record)),
                )
                .map(
                    |(run, phase, operation_index, record)| PointerObservationProgress {
                        run,
                        phase,
                        operation_index,
                        record: record.clone(),
                    },
                )
                .collect(),
            failure_diagnostics: self.state.failure_diagnostics.clone(),
            download: match (self.state.download_end, self.outcome()) {
                (Some(frozen), _) => frozen,
                (None, LocalOutcome::Interrupted) => self
                    .state
                    .facts(self.state.ended_result(), LocalOutcome::Interrupted),
                _ => self
                    .state
                    .facts(DownloadResult::InProgress, LocalOutcome::Running),
            },
            restore_attempted: self.state.restore_attempted,
        }
    }
    /// Pure in-memory projection suitable for status after navigation/restart.
    /// Null while running/restoring or after the once-only attempt is consumed.
    pub fn recovery_phrases(&self) -> Option<OperationRecoveryPhrases> {
        if self.state.restore_attempted
            || self.state.prestate_differs()
            || matches!(
                self.outcome(),
                LocalOutcome::Running | LocalOutcome::Restoring
            )
        {
            return None;
        }
        let original = &self.state.recovery.as_ref()?.original_bundle;
        if original.format != 3 {
            return None;
        }
        let target = original.binding.target.parse::<IndividualAddress>().ok()?;
        Some(OperationRecoveryPhrases {
            recovery_phrase: original.restore_phrase(),
            download_phrase: knx_core::commissioning::mutation::required_confirmation_phrase(
                target,
                knx_core::commissioning::mutation::WriteScope::Download,
            ),
            release_phrase: knx_net::commissioning::hardware_release::required_release_phrase(
                target,
            ),
        })
    }
    /// Install the hook an outer activity journal needs at the send boundary:
    /// after the durable recovery barrier, immediately before the first
    /// possible mutation. It runs at most once; an error refuses that send.
    pub fn on_send_boundary(&mut self, hook: SendBoundary) {
        self.send_boundary = Some(hook);
    }
    pub(crate) fn reach_send_boundary(&mut self) -> io::Result<()> {
        match self.send_boundary.as_mut() {
            Some(hook) => {
                hook()?;
                self.send_boundary = None;
                Ok(())
            }
            None => Ok(()),
        }
    }
    /// Subscribe before moving the store into the executor. Channel polling does
    /// no filesystem, SQL or bus work and needs no long-running operation lock.
    pub fn subscribe_progress(&mut self) -> tokio::sync::watch::Receiver<OperationProgress> {
        if let Some(progress) = &self.progress {
            return progress.subscribe();
        }
        let (sender, receiver) = tokio::sync::watch::channel(self.progress());
        self.progress = Some(sender);
        receiver
    }
    pub fn binding(&self) -> &Binding {
        &self.state.binding
    }
    pub(crate) fn require_fresh_execution(&self) -> io::Result<()> {
        if self.reopened || self.poisoned || self.next != 1 {
            return Err(invalid(
                "hardware execution requires a new unused operation; no resume or retry",
            ));
        }
        Ok(())
    }
    pub(crate) fn require_restore_available(&self) -> io::Result<()> {
        if self.state.prestate_differs() {
            return Err(invalid(
                "original-state recovery unavailable: device prestate differed from source allocation shape or application",
            ));
        }
        if !self.reopened
            || self.poisoned
            || self.state.recovery.is_none()
            || self.state.restore_attempted
        {
            return Err(invalid("recovery requires a freshly reopened intact operation and no previous restore attempt"));
        }
        Ok(())
    }
    pub fn recovery(&self) -> Option<&RecoveryRecord> {
        self.state.recovery.as_ref()
    }
    pub fn mutation_possible(&self) -> bool {
        self.state.mutation
    }
    pub fn outcome(&self) -> LocalOutcome {
        if (self.reopened || self.poisoned)
            && matches!(
                self.state.outcome,
                LocalOutcome::Running | LocalOutcome::Restoring
            )
        {
            LocalOutcome::Interrupted
        } else {
            self.state.outcome
        }
    }
    pub fn shareable_report(&self) -> ShareableReport {
        ShareableReport {
            format: 2,
            kind: "knxbench-property-validation-summary",
            support: if self.state.binding.simulated {
                "STRONG HYPOTHESIS SIMULATOR TESTED"
            } else {
                "EXPERIMENTAL BEST-EFFORT; HARDWARE COMPATIBILITY AND RECOVERY UNVERIFIED"
            },
            simulated: self.state.binding.simulated,
            outcome: self.outcome(),
            mutation_possible: self.state.mutation,
            completed_operations: self.state.completed,
            failure_diagnostics: self.state.failure_diagnostics.clone(),
        }
    }
    pub fn write_shareable_report(&self, path: &Path) -> io::Result<()> {
        write_new_synced(path, &encode(&self.shareable_report())?)
    }
}
#[derive(Debug, Serialize)]
pub struct ShareableReport {
    format: u16,
    kind: &'static str,
    support: &'static str,
    simulated: bool,
    outcome: LocalOutcome,
    mutation_possible: bool,
    completed_operations: usize,
    /// Format 2: typed failure facts only; empty when none was recorded or it is unknown.
    failure_diagnostics: Vec<FailureDiagnostic>,
}
fn encode(value: &impl Serialize) -> io::Result<Vec<u8>> {
    serde_json::to_vec_pretty(value).map_err(io::Error::other)
}
fn open_new(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true).read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}
fn regular_private(path: &Path) -> io::Result<()> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(invalid("protocol entry is not a regular file"));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(invalid("protocol entry is not owner-only"));
        }
    }
    Ok(())
}
/// Owner-only create-new prepared evidence with file/ancestor sync and byte readback.
pub fn write_prepared_bundle(path: &Path, bundle: &PreparedBundle) -> io::Result<()> {
    bundle.plan()?;
    bundle.preconditions()?;
    write_new_synced(path, &encode(bundle)?)
}
pub fn read_bundle(path: &Path) -> io::Result<PreparedBundle> {
    let bundle: PreparedBundle = serde_json::from_slice(&bounded_read(path)?)
        .map_err(|_| invalid("invalid immutable prepared bundle"))?;
    bundle.plan()?;
    bundle.preconditions()?;
    Ok(bundle)
}
fn bounded_read(path: &Path) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take(16 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(invalid("property protocol exceeds bounded size"));
    }
    Ok(bytes)
}
fn persist(directory: &Path, sequence: usize, bytes: &[u8]) -> io::Result<()> {
    write_new_synced(
        &directory.join(format!("checkpoint-{sequence:08}.json")),
        bytes,
    )?;
    let head = encode(&Head {
        format: 1,
        sequence,
        sha256: digest(bytes),
    })?;
    let next = directory.join("head-next");
    write_new_synced(&next, &head)?;
    fs::rename(&next, directory.join("HEAD"))?;
    crate::backup_directory::sync_chain(directory, |dir| File::open(dir)?.sync_all())?;
    if bounded_read(&directory.join("HEAD"))? != head {
        return Err(invalid("protocol head differs on readback"));
    }
    Ok(())
}
fn write_new_synced(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let mut file = open_new(path)?;
    file.write_all(bytes)?;
    file.sync_all()?;
    let directory = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    crate::backup_directory::sync_chain(directory, |dir| File::open(dir)?.sync_all())?;
    if bounded_read(path)? != bytes {
        return Err(invalid("property checkpoint differs on readback"));
    }
    Ok(())
}

pub use crate::property_validation_execution::{
    execute_hardware, execute_simulated, refuse_superseded_defaults_profile, restore_hardware,
    restore_simulated,
};
