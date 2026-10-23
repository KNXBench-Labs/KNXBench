//! Durable, operation-specific property validation protocol; no networking here.
//!
//! Each checkpoint is immutable, create-new, owner-only, file/ancestor-synced
//! and byte-readback verified before returning permission to the Net callback.
//! A fresh opener replays the complete sequence under the same exclusive lock.
//! Hash chaining and a synced atomic HEAD detect corruption and missing tails,
//! not malicious rewriting; an unfinished operation is never harmless.
use crate::property_validation_bundle::{digest, invalid, PreparedBundle, RecoveryRecord};
use knx_core::{ContactableAddress, IndividualAddress};
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

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
    Failed {
        code: FailureCode,
        local_diagnostic: String,
    },
    Cancelled,
    Finished,
    RestoreStarted,
    RestoreFailed {
        code: FailureCode,
        local_diagnostic: String,
    },
    Restored,
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
    active: Option<usize>,
    completed: usize,
    mutation: bool,
    outcome: LocalOutcome,
    restore_attempted: bool,
}
impl State {
    fn new(binding: Binding) -> Self {
        Self {
            binding,
            recovery: None,
            active: None,
            completed: 0,
            mutation: false,
            outcome: LocalOutcome::Running,
            restore_attempted: false,
        }
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
                recovery.to_snapshot(&recovery.original_bundle)?;
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
            }
            Checkpoint::Failed { .. } | Checkpoint::Cancelled => {
                if self.outcome != LocalOutcome::Running {
                    return Err(invalid("operation already ended"));
                }
                self.outcome = if matches!(event, Checkpoint::Cancelled) {
                    LocalOutcome::Cancelled
                } else {
                    LocalOutcome::Failed
                };
            }
            Checkpoint::RestoreStarted => {
                if self.recovery.is_none() || self.restore_attempted {
                    return Err(invalid(
                        "missing recovery or restore already attempted; no retry",
                    ));
                }
                self.restore_attempted = true;
                self.outcome = LocalOutcome::Restoring;
                self.active = None;
                self.completed = 0;
            }
            Checkpoint::RestoreFailed { .. } => {
                if self.outcome != LocalOutcome::Restoring {
                    return Err(invalid("no restore in progress"));
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
}
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
            if record.format != 1
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
            format: 1,
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
                && !self.state.restore_attempted
                && !matches!(
                    self.outcome(),
                    LocalOutcome::Running | LocalOutcome::Restoring
                ),
            mutation_possible: self.state.mutation,
            recovery_phrases: self.recovery_phrases(),
        }
    }
    /// Pure in-memory projection suitable for status after navigation/restart.
    /// Null while running/restoring or after the once-only attempt is consumed.
    pub fn recovery_phrases(&self) -> Option<OperationRecoveryPhrases> {
        if self.state.restore_attempted
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
            format: 1,
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
    execute_hardware, execute_simulated, restore_hardware, restore_simulated,
};
