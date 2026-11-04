//! Wrong observed serial refuses mounted recovery without changing device or original evidence.

use super::*;
use knx_app::property_validation::FailureCode;
use knx_app::property_validation_bundle::OperationRecord;
use knx_core::commissioning::properties::PID_SERIAL_NUMBER;

pub(super) struct Original<'a> {
    pub case: Case,
    pub data: &'a Path,
    pub operation: &'a Path,
    pub original: &'a PreparedBundle,
    pub original_bytes: &'a [u8],
    pub backup: &'a RecoveryRecord,
    pub facts: &'a DownloadFacts,
    pub before: &'a Editor,
    pub finished: &'a Value,
    pub recovery: Value,
}

fn checkpoints(operation: &Path) -> Vec<(PathBuf, Vec<u8>)> {
    let mut files: Vec<_> = std::fs::read_dir(operation.join("journal"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("checkpoint-")
        })
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|path| {
            let bytes = std::fs::read(&path).unwrap();
            (path, bytes)
        })
        .collect()
}

pub(super) async fn finish(
    state: &crate::SharedState,
    router: &Router,
    hardware: &Arc<Hardware>,
    counters: &Arc<Counters>,
    fixture: Original<'_>,
) {
    let Original {
        case,
        data,
        operation,
        original,
        original_bytes,
        backup,
        facts,
        before,
        finished,
        recovery,
    } = fixture;
    let id = finished["operationId"].as_str().unwrap();
    no_worker(state);
    assert!(!operation.join("recovery-command").exists());
    let frozen = checkpoints(operation);
    assert!(!frozen.is_empty());
    let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert_eq!(store.progress().download, *facts);
    assert_eq!(store.recovery(), Some(backup));
    assert!(store.progress().restore_available);
    assert!(!store.progress().restore_attempted);
    drop(store);

    // Only the actual simulated observation changes. Never rewrite binding,
    // observed.json, backup or producer deviation metadata to fake another device.
    let completed_device = hardware.device.system2_snapshot();
    assert_eq!(
        completed_device.properties.get(&(0, PID_SERIAL_NUMBER)),
        Some(&SERIAL.to_vec())
    );
    let wrong = [0, 1, 2, 3, 4, 6];
    assert_ne!(wrong, original.binding.serial);
    hardware
        .device
        .preset_property(0, PID_SERIAL_NUMBER, &wrong);
    let before_device = hardware.device.system2_snapshot();
    let mut expected_device = completed_device;
    expected_device
        .properties
        .insert((0, PID_SERIAL_NUMBER), wrong.to_vec());
    assert_eq!(
        before_device, expected_device,
        "only PID_SERIAL_NUMBER changed"
    );
    assert_eq!(Editor::read(state), *before);
    let traffic = counters.read();
    let service_count = counters.services.lock().unwrap().len();
    assert!(
        counters.services.lock().unwrap().iter().any(|s| matches!(
            s,
            ApplicationService::MemoryWrite { .. }
                | ApplicationService::UserMemoryWrite { .. }
                | ApplicationService::PropertyValueWrite { .. }
        )),
        "trace saw the original download writes"
    );

    let accepted = http(
        router,
        "/api/device-system2/recover",
        Some(recovery.clone()),
        200,
    )
    .await;
    // Recover's constructor returns the original facts with a restoring view;
    // unlike synchronous conflicts it does not return an error/kind body.
    let mut expected_accepted = finished.clone();
    expected_accepted["state"] = "restoring".into();
    // The live view suppresses Verified until RestoreStarted is attached;
    // the original durable download facts are still Verified throughout.
    expected_accepted["downloadResult"] = "inProgress".into();
    expected_accepted["projectSettlement"] = "notRecorded".into();
    for field in ["recoveryPhrase", "downloadPhrase", "releasePhrase"] {
        expected_accepted[field] = Value::Null;
    }
    assert_eq!(accepted, expected_accepted);
    let failed = super::super::super::settled(state, router, "restoreFailed").await;
    no_worker(state);

    // A real asynchronous transport read, not HTTP-only refusal. Allow exactly
    // the executor's read/control grammar; this excludes ALL mutation services,
    // including VerifyMode and load-control property writes, even idempotent ones.
    assert_eq!(counters.read()[0], traffic[0] + 1);
    assert_eq!(counters.read()[1], traffic[1] + 1);
    assert!(counters.read()[2] > traffic[2]);
    let services = counters.services.lock().unwrap()[service_count..].to_vec();
    assert!(
        services.iter().any(|s| matches!(
            s,
            ApplicationService::PropertyValueRead {
                object_index: 0,
                property_id: PID_SERIAL_NUMBER,
                nr_of_elem: 1,
                start_index: 1
            }
        )),
        "actual worker read the changed six-byte serial"
    );
    for service in &services {
        assert!(
            matches!(
                service,
                ApplicationService::NoApplicationPdu
                    | ApplicationService::DeviceDescriptorRead { .. }
                    | ApplicationService::MemoryRead { .. }
                    | ApplicationService::UserMemoryRead { .. }
                    | ApplicationService::PropertyValueRead { .. }
            ),
            "{case:?}: forbidden recovery mutation frame: {service:?}"
        );
    }
    assert_eq!(
        hardware.device.system2_snapshot(),
        before_device,
        "failed no-write recovery preserves the complete pre-recovery model, not OLD restoration"
    );

    // The serial guard is at HardwareCheck before VerifyMode and every data write.
    let hardware_index = original
        .normalized_plan
        .steps
        .iter()
        .position(|step| matches!(step.operation, OperationRecord::HardwareCheck { .. }))
        .unwrap();
    assert!(hardware_index > 0);
    let store = OperationStore::open(&operation.join("journal"), &original.binding).unwrap();
    let progress = store.progress();
    assert_eq!(store.outcome(), LocalOutcome::RestoreFailed);
    assert_eq!(progress.outcome, LocalOutcome::RestoreFailed);
    assert_eq!(progress.completed_steps, hardware_index);
    assert_eq!(
        progress.total_steps,
        Some(original.normalized_plan.steps.len())
    );
    assert!(progress.restore_attempted);
    assert!(!progress.restore_available);
    assert!(progress.backup_available);
    assert!(
        progress.mutation_possible,
        "original download mutation is still a fact"
    );
    assert_eq!(progress.download, *facts);
    assert_eq!(store.recovery(), Some(backup));
    assert_eq!(
        backup.observed_serial, None,
        "matching original acquisition is not a deviation"
    );
    assert!(store.recovery_phrases().is_none());
    assert!(
        progress.failure_diagnostics.is_empty(),
        "no invented pointer-width diagnostic"
    );
    assert!(progress.final_step_failures.is_empty());
    drop(store);
    let mut expected_failed = expected_accepted;
    expected_failed["state"] = "restoreFailed".into();
    expected_failed["downloadResult"] = "verified".into();
    expected_failed["completedSteps"] = json!(hardware_index);
    expected_failed["totalSteps"] = json!(original.normalized_plan.steps.len());
    expected_failed["restoreAttempted"] = true.into();
    expected_failed["projectSettlement"] = "sessionOnly".into();
    assert_eq!(
        failed, expected_failed,
        "exact terminal wire grammar, no invented serial/refusal fields"
    );
    assert_eq!(
        std::fs::read(operation.join("observed.json")).unwrap(),
        original_bytes
    );
    let all = checkpoints(operation);
    assert_eq!(
        &all[..frozen.len()],
        frozen.as_slice(),
        "original checkpoints stay byte-exact"
    );
    let appended: Vec<Checkpoint> = all[frozen.len()..]
        .iter()
        .map(|(_, bytes)| {
            let envelope: Value = serde_json::from_slice(bytes).unwrap();
            serde_json::from_value(envelope["checkpoint"].clone()).unwrap()
        })
        .collect();
    assert_eq!(appended.len(), 2 * hardware_index + 3);
    assert_eq!(appended[0], Checkpoint::RestoreStarted);
    for index in 0..hardware_index {
        assert_eq!(
            appended[1 + 2 * index],
            Checkpoint::StepStarted {
                index,
                mutation: false
            }
        );
        assert_eq!(appended[2 + 2 * index], Checkpoint::StepFinished { index });
    }
    assert_eq!(
        appended[1 + 2 * hardware_index],
        Checkpoint::StepStarted {
            index: hardware_index,
            mutation: false,
        }
    );
    match appended.last().unwrap() {
        Checkpoint::RestoreFailed {
            code,
            local_diagnostic,
            pointer_failure,
            final_step,
            allocation_evidence,
        } => {
            assert_eq!(*code, FailureCode::IdentityOrRecovery);
            assert!(local_diagnostic.contains("recovery refused: device serial number differs from the device whose prestate was captured"));
            assert!(pointer_failure.is_none());
            assert!(final_step.is_none());
            assert!(allocation_evidence.is_none());
        }
        other => panic!("expected typed original-serial refusal, got {other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(operation.join("recovery-command")).unwrap(),
        id
    );
    assert_eq!(
        state.device_system2.control.snapshot().settlement,
        System2Settlement::Settled {
            operation: format!("{id}-restore"),
            outcome: LocalOutcome::RestoreFailed,
        }
    );
    let after = Editor::read(state);
    // Existing conservative policy uses original mutation + RestoreStarted,
    // despite this recovery itself emitting no mutation. Preserve that policy.
    assert_recovery_settlement(before, &after, case);

    // Admission is consumed even by this no-write failure. No serial reset/retry
    // bypass: independent OriginalSerial leaves remain the valid restore controls.
    refused(
        state,
        router,
        data,
        hardware,
        counters,
        (
            "/api/device-system2/recover",
            recovery.clone(),
            409,
            "system2RecoveryConsumed",
        ),
    )
    .await;
    let evidence = super::super::super::evidence_tree(data);
    let traffic = counters.read();
    let report = http(
        router,
        &format!("/api/device-system2/report?operationId={id}"),
        None,
        200,
    )
    .await;
    assert_eq!(
        report["download"],
        json!({"outcome":"finished", "result":"verified",
        "completedSteps": facts.completed_steps, "totalSteps": facts.total_steps})
    );
    assert_eq!(
        report["recovery"],
        json!({"outcome":"restoreFailed",
        "completedSteps":hardware_index, "totalSteps":original.normalized_plan.steps.len()})
    );
    assert_eq!(report["failureDiagnostics"], json!([]));
    assert_eq!(report["finalStepFailures"], json!([]));
    let selected = http(
        router,
        "/api/device-system2/select",
        Some(json!({"operationId":id})),
        200,
    )
    .await;
    let mut expected_selected = failed;
    expected_selected["projectSettlement"] = "notRecorded".into();
    expected_selected["message"] = "Original retained operation; no automatic resume or retry. Recovery remains experimental best-effort.".into();
    assert_eq!(selected, expected_selected);
    assert_eq!(
        http(router, "/api/device-system2/status", None, 200).await,
        expected_selected
    );
    assert_eq!(Editor::read(state), after, "reads/reopen cannot re-settle");
    assert_eq!(super::super::super::evidence_tree(data), evidence);
    assert_eq!(counters.read(), traffic);
    assert_eq!(hardware.device.system2_snapshot(), before_device);
    no_worker(state);
}

#[tokio::test]
async fn caption_mounted_recovery_wrong_serial_standalone20() {
    run_with_mode(
        Case {
            namespace: 20,
            retained: false,
        },
        RecoveryMode::WrongObservedSerial,
    )
    .await;
}
#[tokio::test]
async fn caption_mounted_recovery_wrong_serial_standalone23() {
    run_with_mode(
        Case {
            namespace: 23,
            retained: false,
        },
        RecoveryMode::WrongObservedSerial,
    )
    .await;
}
#[tokio::test]
async fn caption_mounted_recovery_wrong_serial_retained20() {
    run_with_mode(
        Case {
            namespace: 20,
            retained: true,
        },
        RecoveryMode::WrongObservedSerial,
    )
    .await;
}
#[tokio::test]
async fn caption_mounted_recovery_wrong_serial_retained23() {
    run_with_mode(
        Case {
            namespace: 23,
            retained: true,
        },
        RecoveryMode::WrongObservedSerial,
    )
    .await;
}
