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
    kind: Option<&'static str>,
    started: Option<std::time::Instant>,
    executor_entry: Option<std::time::Duration>,
    first_memory_read_entry: Option<std::time::Duration>,
    current_read: Option<knx_net::management::MemoryReadObservation>,
    read_observed: Option<std::time::Duration>,
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
        let started = std::time::Instant::now();
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
                kind: Some("download"),
                started: Some(started),
                phase: Some("worker-entered"),
                preparation: Some(value),
                ..Default::default()
            }
        });
    }
    pub(super) fn begin_recovery(
        &self,
        bundle: &knx_app::property_validation_bundle::PreparedBundle,
        store: &knx_app::property_validation::OperationStore,
        operation: &str,
        incarnation: &str,
    ) {
        let started = std::time::Instant::now();
        let mut binding = recovery_binding(bundle, operation);
        binding["storeBindingMatchesOriginal"] = (store.binding() == &bundle.binding).into();
        binding["storeRecoveryBundleMatchesOriginal"] =
            json!(store.recovery().map(|r| r.original_bundle == *bundle));
        binding["serverIncarnationSha256"] =
            knx_productdb::sha256_hex(incarnation.as_bytes()).into();
        // Same operation ID as download, but a different actual worker. Reset
        // every cached phase/error/progress/read/terminal clock at its entry.
        self.update(|s| {
            *s = Snapshot {
                kind: Some("recovery"),
                started: Some(started),
                phase: Some("worker-entered"),
                preparation: Some(binding),
                ..Default::default()
            }
        });
    }
    pub(super) fn phase(&self, phase: &'static str) {
        self.update(|s| {
            s.phase = Some(phase);
            if phase == "executor" {
                s.executor_entry = s.started.map(|start| start.elapsed());
            }
        });
    }
    pub(super) fn memory_read(&self, marker: knx_net::management::MemoryReadObservation) {
        let now = std::time::Instant::now();
        self.update(|s| {
            let Some(start) = s.started else { return };
            let entry = marker.entered_at.saturating_duration_since(start);
            s.first_memory_read_entry.get_or_insert(entry);
            s.current_read = Some(marker);
            s.read_observed = Some(now.saturating_duration_since(start));
        });
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
    fn snapshot(&self, operation: &str, include_test_offset: bool) -> Value {
        match self.0.try_lock() {
            Ok(s) => json!({
                "workerKind": s.kind,
                "phase": s.phase,
                "firstFailure": s.failure.map(|(phase, class, kind)| json!({"phase":phase,"class":class,"ioKind":kind.map(|k| format!("{k:?}"))})),
                "workerPreparation": s.preparation.as_ref().filter(|_| s.kind == Some("download")),
                "workerRecovery": s.preparation.as_ref().filter(|_| s.kind == Some("recovery")),
                "preparationMatchesFinalOperation": s.preparation.as_ref().filter(|_| s.kind == Some("download")).map(|p| p["operationSha256"] == knx_productdb::sha256_hex(operation.as_bytes())),
                "lastSendBoundaryProgress": s.progress,
                "readTiming": {
                    "elapsedSinceWorkerEntryMicros": s.started.map(|start| start.elapsed().as_micros()),
                    "executorEntryElapsedMicros": s.executor_entry.map(|d| d.as_micros()),
                    "elapsedSinceExecutorEntryMicros": s.executor_entry.and_then(|entry| s.started.map(|start| start.elapsed().saturating_sub(entry).as_micros())),
                    "firstObservedMemoryReadEntryElapsedMicros": s.first_memory_read_entry.map(|d| d.as_micros()),
                    "currentReadEntryElapsedMicros": s.current_read.and_then(|r| s.started.map(|start| r.entered_at.saturating_duration_since(start).as_micros())),
                    "elapsedSinceCurrentReadEntryMicros": s.current_read.map(|r| r.entered_at.elapsed().as_micros()),
                    "lastBoundaryElapsedMicros": s.read_observed.map(|d| d.as_micros()),
                    "currentRead": s.current_read.map(|r| json!({
                        "testOffset": if include_test_offset { Some(r.offset) } else { None }, "count": r.count,
                        "requestSequence": r.request_seq, "expectedReceiveSequence": r.expected_receive_seq,
                        "lastReceivedSequence": r.received_seq, "attempts": r.attempts,
                        "answerSlices": r.answer_slices, "ackSeen": r.acknowledged,
                        "awaiting": r.awaiting, "disposition": r.disposition,
                        "responseSliceRemainingAtBoundaryMicros": r.response_slice_remaining.map(|d| d.as_micros()),
                    })),
                    "bestEffortNonAtomicObservation": true,
                },
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
fn json_sha256(value: &impl serde::Serialize) -> Option<String> {
    serde_json::to_value(value)
        .and_then(|v| serde_json::to_vec(&v))
        .ok()
        .map(|bytes| knx_productdb::sha256_hex(&bytes))
}
fn recovery_binding(
    bundle: &knx_app::property_validation_bundle::PreparedBundle,
    operation: &str,
) -> Value {
    // Project only already-validated original identities; no plan/preconditions,
    // source reconstruction, or serialization of payloads into the artifact.
    json!({
        "operationSha256":knx_productdb::sha256_hex(operation.as_bytes()),
        "originalBindingSha256":json_sha256(&bundle.binding),
        "planSha256":bundle.binding.plan_sha256,
        "hardwarePreparationSha256":bundle.binding.project_preparation_sha256,
        "configurationSha256":bundle.hardware_preparation.as_ref().map(|p| &p.source_preparation_sha256),
        "sourceOperationMatches":bundle.hardware_preparation.as_ref().map(|p| p.operation == operation),
        "requestSha256":bundle.project_preparation.as_ref().and_then(|p| json_sha256(&p.request)),
        "sourceBindingMatches":bundle.project_preparation.as_ref().map(|p| p.source_sha256 == bundle.binding.source_sha256 && p.plan_sha256 == bundle.binding.plan_sha256),
        "originalFormat":bundle.format,"originalHardwareBinding":!bundle.binding.simulated,
    })
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
        "recoveryControlMatches": record.control_id == format!("{operation}-restore"),
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
            Checkpoint::FinalPointerObservations { .. } => "final-pointer-observations",
            Checkpoint::InProcedurePointerObservation { .. } => "in-procedure-pointer-observation",
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
        json!({"committedSequence":head.sequence,"committedSha256":head.sha256,"grammarAndHashChainVerified":true,
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
            "worker":owner.snapshot(operation, true),"service":service_view(state,operation),"transport":transport,"evidence":evidence_view(data,operation)});
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

pub(super) fn retain_recovery_timeout(
    state: &SharedState,
    owner: &Observation,
    data: &Path,
    operation: &str,
    original: &knx_app::property_validation_bundle::PreparedBundle,
    admission: (&Value, &Value),
    transport: Value,
) {
    let Some(directory) = std::env::var_os("KNXBENCH_MOUNTED_ACCEPTANCE_DIAGNOSTICS") else {
        return;
    };
    let result = (|| -> io::Result<()> {
        let canonical = PathBuf::from(directory).canonicalize()?;
        if canonical.starts_with(data.canonicalize()?)
            || !canonical.is_dir()
            || !dto::valid_id(operation)
        {
            return Err(invalid());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if canonical.metadata()?.permissions().mode() & 0o077 != 0 {
                return Err(invalid());
            }
        }
        // Suppress memory offsets even if a missed best-effort reset leaves a
        // stale DOWNLOAD snapshot. Its kind/incarnation mismatches stay visible.
        let worker = owner.snapshot(operation, false);
        let original_binding = recovery_binding(original, operation);
        let (before, accepted) = admission;
        let root = data.join(storage::ROOT_NAME);
        let selected = bounded_read(&root.join("selected"), 32);
        // The fixture already read/validated this exact ORIGINAL at reopen. Pin
        // its committed journal prefix without reopening a writer or bundle.
        let journal = journal_view(&root.join(operation), &original.binding);
        let value = json!({
            "schemaVersion":1,"trigger":"accepted-original-recovery-worker-timeout","budgetSeconds":10,
            "original":original_binding,
            "admission":{
                "preRecoveryOperationMatches":before["operationId"].as_str() == Some(operation),
                "acceptedOperationMatches":accepted["operationId"].as_str() == Some(operation),
                "preRecoveryBindingSha256":json_sha256(&before["binding"]),
                "acceptedBindingSha256":json_sha256(&accepted["binding"]),
                "acceptedBindingMatchesPreRecovery":accepted["binding"] == before["binding"],
                "acceptedRestoring":accepted["state"] == "restoring",
            },
            "observerMatchesFreshService":state.device_system2.observation.as_deref().is_some_and(|o| std::ptr::eq(o, owner)),
            "workerKindMatchesRecovery":worker["workerKind"] == "recovery",
            "workerOperationMatches":worker["workerRecovery"]["operationSha256"] == knx_productdb::sha256_hex(operation.as_bytes()),
            "workerOriginalBindingMatches":worker["workerRecovery"]["originalBindingSha256"] == original_binding["originalBindingSha256"],
            "workerRequestMatchesOriginal":original_binding["requestSha256"].as_str().is_some_and(|hash| worker["workerRecovery"]["requestSha256"].as_str() == Some(hash)),
            "workerConfigurationMatchesOriginal":original_binding["configurationSha256"].as_str().is_some_and(|hash| worker["workerRecovery"]["configurationSha256"].as_str() == Some(hash)),
            "workerMatchesFreshIncarnation":worker["workerRecovery"]["serverIncarnationSha256"] == knx_productdb::sha256_hex(state.server_incarnation.as_bytes()),
            "worker":worker,"service":service_view(state,operation),"transport":transport,
            "progressSampling":"cached-at-send-boundaries; latest-stream-unavailable-without-blocking-borrow",
            "evidence":{
                "selection":outcome(&selected),"selectedOperationMatches":selected.as_ref().ok().map(|b| b == operation.as_bytes()),
                "journal":match journal { Ok(v) => v, Err(e) => outcome::<()>(&Err(e)) },
                "originalBindingSource":"fixture-pre-recovery-validated-bundle; no-bundle-reopen-or-semantic-replay",
            },
        });
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
        let mut file = options.open(canonical.join("r1-recovery-worker-timeout.json"))?;
        std::io::Write::write_all(&mut file, &bytes)?;
        Ok(())
    })();
    if let Err(error) = result {
        eprintln!(
            "R1 recovery timeout snapshot unavailable: {:?}",
            error.kind()
        );
    }
}
