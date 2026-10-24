//! Test-owned, bounded timeout evidence; never a second journal writer.
use super::*;
use knx_app::property_validation::{Binding as JournalBinding, Checkpoint};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{io, io::Read, path::PathBuf};

#[derive(Default)]
pub(super) struct Observation(Mutex<Snapshot>);
#[derive(Default)]
struct Snapshot {
    phase: Option<&'static str>,
    failure: Option<(&'static str, &'static str, Option<io::ErrorKind>)>,
    preparation: Option<Value>,
    // Cached at actual worker send boundaries, not a blocking watch borrow at timeout.
    progress: Option<Value>,
    terminal: Option<(LocalOutcome, bool)>,
}
impl Observation {
    fn update(&self, f: impl FnOnce(&mut Snapshot)) {
        // Observation must never wait for a worker/snapshot owner.
        if let Ok(mut snapshot) = self.0.try_lock() {
            f(&mut snapshot);
        }
    }
    pub(super) fn begin(&self, preparation: &Preparation, request_id: &str) {
        let bundle = preparation.source.preview();
        let configuration = bundle.project_preparation.as_ref().map(|p| p.sha256());
        // Canonical JSON Value ordering matches the final HTTP preview projection.
        let binding = serde_json::to_value(&preparation.metadata.binding)
            .and_then(|value| serde_json::to_vec(&value));
        let value = json!({
            "operationSha256": knx_productdb::sha256_hex(preparation.metadata.operation_id.as_bytes()),
            "sourceOperationMatches": preparation.source.operation() == preparation.metadata.operation_id,
            "requestPreparationMatches": request_id == preparation.metadata.operation_id,
            "bindingSha256": binding.as_ref().ok().map(|b| knx_productdb::sha256_hex(b)),
            "configurationSha256": configuration.as_ref().and_then(|r| r.as_ref().ok()),
            "configurationErrorKind": configuration.as_ref().and_then(|r| r.as_ref().err()).map(|e| format!("{:?}", e.kind())),
            "planSha256": bundle.binding.plan_sha256,
        });
        self.update(|s| {
            *s = Snapshot {
                phase: Some("worker-entered"),
                preparation: Some(value),
                ..Default::default()
            }
        });
    }
    pub(super) fn phase(&self, phase: &'static str) {
        self.update(|s| s.phase = Some(phase));
    }
    pub(super) fn failure(
        &self,
        phase: &'static str,
        class: &'static str,
        kind: Option<io::ErrorKind>,
    ) {
        self.update(|s| {
            // Preserve the earliest discarded cause, not its later generic collapse.
            if s.failure.is_none() {
                s.failure = Some((phase, class, kind));
            }
        });
    }
    pub(super) fn progress(&self, progress: &OperationProgress) {
        let value = progress_view(progress);
        self.update(|s| s.progress = Some(value));
    }
    pub(super) fn terminal(&self, outcome: LocalOutcome, clean: bool) {
        self.update(|s| {
            s.phase = Some("terminal-published");
            s.terminal = Some((outcome, clean));
        });
    }
    pub(super) fn session_failure(&self, phase: &'static str, error: &crate::BusSessionError) {
        use knx_net::BusError;
        let (class, kind) = match error {
            crate::BusSessionError::AlreadyActive => ("already-active", None),
            crate::BusSessionError::NoActiveSession => ("no-active-session", None),
            crate::BusSessionError::Transport(error) => match error {
                BusError::Io(error) => ("transport-io", Some(error.kind())),
                BusError::Timeout => ("transport-timeout", None),
                BusError::ConnectionRefused(_) => ("transport-connection-refused", None),
                BusError::NotImplemented => ("transport-not-implemented", None),
                BusError::Protocol(_) => ("transport-protocol", None),
                BusError::NotMulticast(_) => ("transport-not-multicast", None),
            },
        };
        self.failure(phase, class, kind);
    }
    fn snapshot(&self, operation: &str) -> Value {
        match self.0.try_lock() {
            Ok(s) => json!({
                "phase": s.phase,
                "firstFailure": s.failure.map(|(phase, class, kind)| json!({"phase":phase,"class":class,"ioKind":kind.map(|k| format!("{k:?}"))})),
                "workerPreparation": s.preparation,
                "preparationMatchesFinalOperation": s.preparation.as_ref().map(|p| p["operationSha256"] == knx_productdb::sha256_hex(operation.as_bytes())),
                "lastSendBoundaryProgress": s.progress,
                "terminal":s.terminal.map(|(outcome, clean)| json!({"outcome":outcome,"disconnectClean":clean})),
                "missingFieldsMeanUnavailable": true,
            }),
            Err(_) => json!({"unavailable":"observation-lock-busy-or-poisoned"}),
        }
    }
}
fn progress_view(p: &OperationProgress) -> Value {
    json!({"outcome":p.outcome,"completedSteps":p.completed_steps,"totalSteps":p.total_steps,
        "backupAvailable":p.backup_available,"restoreAvailable":p.restore_available,"mutationPossible":p.mutation_possible})
}
fn service_view(state: &SharedState, operation: &str) -> Value {
    let Ok(inner) = state.device_system2.inner.try_lock() else {
        return json!({"unavailable":"service-lock-busy-or-poisoned"});
    };
    let Some(record) = &inner.current else {
        return json!({"unavailable":"no-current-record"});
    };
    // Do not call Record::view/status or borrow the watch channel: those can wait
    // on the worker's progress writer. This is the actual local service record;
    // the attached stream's latest value is explicitly unavailable here.
    json!({
        "operationMatches": record.metadata.operation_id == operation,
        "taskPresent":record.task.is_some(),
        "taskFinished":record.task.as_ref().map(JoinHandle::is_finished),
        "progressAttached":record.updates.is_some(),
        "localProgress":progress_view(&record.progress),
        "runningState":record.task.as_ref().filter(|t| !t.is_finished()).map(|_| if record.control_id.ends_with("-restore") { "restoring" } else { "running" }),
        "streamProgress":"unavailable-without-blocking-borrow; see lastSendBoundaryProgress",
    })
}

// Same HEAD/envelope/hash grammar as property_validation::{Head,Envelope,open}.
// This reads a pinned committed prefix, not OperationStore::open (exclusive writer).
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Head {
    format: u16,
    sequence: usize,
    sha256: String,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Envelope {
    format: u16,
    kind: String,
    sequence: usize,
    previous_sha256: String,
    checkpoint: Checkpoint,
}
fn invalid() -> io::Error {
    io::Error::from(io::ErrorKind::InvalidData)
}
fn bounded_read(path: &Path, limit: u64) -> io::Result<Vec<u8>> {
    let metadata = std::fs::symlink_metadata(path)?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err(invalid());
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
    }
    let mut bytes = Vec::new();
    options
        .open(path)?
        .take(limit + 1)
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > limit {
        return Err(invalid());
    }
    Ok(bytes)
}
fn outcome<T>(result: &io::Result<T>) -> Value {
    match result {
        Ok(_) => json!({"readable":true}),
        Err(e) => json!({"unavailable":true,"ioKind":format!("{:?}",e.kind())}),
    }
}
fn journal_view(operation: &Path, expected: &JournalBinding) -> io::Result<Value> {
    let directory = operation.join("journal");
    let bytes = bounded_read(&directory.join("HEAD"), 4096)?;
    let head: Head = serde_json::from_slice(&bytes).map_err(|_| invalid())?;
    // Bounded file count and aggregate input bytes, independent of output cap.
    if head.format != 1 || head.sequence >= 256 {
        return Err(invalid());
    }
    let mut remaining = 32 * 1024 * 1024u64;
    let mut previous = String::new();
    let mut recovery_count = 0usize;
    let mut last_kind = "unavailable";
    let mut last_index = None;
    let mut failure = None;
    for index in 0..=head.sequence {
        let checkpoint_bytes = bounded_read(
            &directory.join(format!("checkpoint-{index:08}.json")),
            remaining.min(16 * 1024 * 1024),
        )?;
        remaining = remaining
            .checked_sub(checkpoint_bytes.len() as u64)
            .ok_or_else(invalid)?;
        let record: Envelope = serde_json::from_slice(&checkpoint_bytes).map_err(|_| invalid())?;
        if record.format != 1
            || record.kind != "knxbench-property-validation-protocol"
            || record.sequence != index
            || record.previous_sha256 != previous
        {
            return Err(invalid());
        }
        if index == 0
            && !matches!(&record.checkpoint, Checkpoint::Created { binding } if binding == expected)
        {
            return Err(invalid());
        }
        last_index = None;
        last_kind = match record.checkpoint {
            Checkpoint::Created { .. } if index == 0 => "created",
            Checkpoint::Created { .. } => return Err(invalid()),
            Checkpoint::RecoveryAcquired { recovery } => {
                if recovery.original_bundle.binding != *expected {
                    return Err(invalid());
                }
                recovery_count += 1;
                "recovery-acquired"
            }
            Checkpoint::StepStarted { index, .. } => {
                last_index = Some(index);
                "step-started"
            }
            Checkpoint::StepFinished { index } => {
                last_index = Some(index);
                "step-finished"
            }
            Checkpoint::Failed { code, .. } => {
                failure = Some(code);
                "failed"
            }
            Checkpoint::RestoreFailed { code, .. } => {
                failure = Some(code);
                "restore-failed"
            }
            Checkpoint::Cancelled => "cancelled",
            Checkpoint::Finished => "finished",
            Checkpoint::RestoreStarted => "restore-started",
            Checkpoint::Restored => "restored",
        };
        previous = knx_productdb::sha256_hex(&checkpoint_bytes);
    }
    if previous != head.sha256 || recovery_count > 1 {
        return Err(invalid());
    }
    let current_head = bounded_read(&directory.join("HEAD"), 4096)?;
    Ok(
        json!({"committedSequence":head.sequence,"grammarAndHashChainVerified":true,
        "fullProtocolReplayPerformed":false,"headChangedDuringRead":current_head != bytes,
        "lastCheckpoint":last_kind,"lastStepIndex":last_index,"terminalFailureCode":failure,
        "recoveryAcquiredCount":recovery_count,"committedRecoveryAcquired":recovery_count == 1}),
    )
}
fn evidence_view(data: &Path, operation: &str) -> Value {
    if !dto::valid_id(operation) {
        return json!({"unavailable":"invalid-operation-id"});
    }
    let root = data.join(storage::ROOT_NAME);
    let selected = bounded_read(&root.join("selected"), 32);
    let path = root.join(operation); // Never inspect a different selected operation.
    let observed = knx_app::property_validation::read_bundle(&path.join("observed.json"));
    let head = bounded_read(&path.join("journal/HEAD"), 4096);
    let mut value = json!({"selection":outcome(&selected),"selectedOperationMatches":selected.as_ref().ok().map(|b| b == operation.as_bytes()),
        "observedBundle":outcome(&observed),"journalHead":outcome(&head),"absenceDoesNotEstablishBackupState":true});
    if let Ok(bundle) = observed {
        let matches = bundle
            .hardware_preparation
            .as_ref()
            .is_some_and(|p| p.operation == operation);
        value["observedOperationMatches"] = matches.into();
        value["observedPlanSha256"] = json!(bundle.binding.plan_sha256);
        value["observedFormat"] = json!(bundle.format);
        value["observedHardwareBinding"] = json!(!bundle.binding.simulated);
        value["observedConfigurationSha256"] =
            match bundle.project_preparation.as_ref().map(|p| p.sha256()) {
                Some(Ok(hash)) => json!(hash),
                Some(Err(error)) => outcome::<()>(&Err(error)),
                None => json!({"unavailable":"no-project-preparation"}),
            };
        if matches && selected.as_ref().is_ok_and(|b| b == operation.as_bytes()) {
            let journal = journal_view(&path, &bundle.binding);
            value["journal"] = match journal {
                Ok(v) => v,
                Err(e) => outcome::<()>(&Err(e)),
            };
        } else {
            value["journal"] = json!({"unavailable":"operation-selection-mismatch"});
        }
    } else {
        value["journal"] = json!({"unavailable":"observed-binding-unavailable"});
    }
    value
}

pub(super) fn retain_timeout(
    state: &SharedState,
    owner: &Observation,
    data: &Path,
    operation: &str,
    final_preview: &Value,
    transport: Value,
) {
    let Some(directory) = std::env::var_os("KNXBENCH_MOUNTED_ACCEPTANCE_DIAGNOSTICS") else {
        return;
    };
    let result = (|| -> io::Result<()> {
        let directory = PathBuf::from(directory);
        // Must survive TempDir teardown and use an existing private directory.
        let canonical = directory.canonicalize()?;
        if canonical.starts_with(data.canonicalize()?) || !canonical.is_dir() {
            return Err(invalid());
        }
        let binding = serde_json::to_vec(&final_preview["binding"]).map_err(io::Error::other)?;
        let value = json!({"schemaVersion":1,"trigger":"actual-data-write-timeout","budgetSeconds":10,
            "finalPreparation":{"operationMatches":final_preview["preparationId"].as_str() == Some(operation),
                "bindingSha256":knx_productdb::sha256_hex(&binding),
                "configurationSha256":final_preview["configurationSha256"],"planSha256":final_preview["planSha256"]},
            "worker":owner.snapshot(operation),"service":service_view(state,operation),"transport":transport,"evidence":evidence_view(data,operation)});
        let bytes = serde_json::to_vec_pretty(&value).map_err(io::Error::other)?;
        if bytes.len() > 16 * 1024 {
            return Err(invalid());
        }
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(canonical.join("r1-worker-timeout.json"))?;
        std::io::Write::write_all(&mut file, &bytes)?;
        Ok(())
    })();
    // Never replace the original timeout panic, even if evidence is unavailable.
    if let Err(error) = result {
        eprintln!("R1 timeout snapshot unavailable: {:?}", error.kind());
    }
}
