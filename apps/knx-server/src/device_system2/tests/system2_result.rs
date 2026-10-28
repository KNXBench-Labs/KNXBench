//! Typed download result in the status and the bounded shareable System-2 report route.
use crate::device_system2::{initial, Record};
use axum::{body::Body, http::Request, Router};
use knx_app::property_validation::{DownloadResult, LocalOutcome, OperationProgress};
use serde_json::Value;
use std::sync::Arc;
use tower::ServiceExt;

const ID: &str = "0123456789abcdef0123456789abcdef";

fn status_of(progress: OperationProgress) -> Value {
    let meta = super::metadata();
    let record = Record {
        metadata: meta.clone(),
        progress,
        updates: None,
        task: None,
        control_id: meta.operation_id,
        message: None,
        refusal: None,
        settlement: crate::device_system2::settlement::Receipt::NotRecorded,
    };
    serde_json::to_value(record.view()).unwrap()
}
fn progress(outcome: LocalOutcome, mutation: bool, result: DownloadResult) -> OperationProgress {
    let mut p = initial(outcome);
    p.mutation_possible = mutation;
    p.download.result = result;
    p.download.outcome = outcome;
    p
}

#[test]
fn status_projects_the_journal_derived_result_in_camel_case() {
    for (result, text) in [
        (DownloadResult::Verified, "verified"),
        (DownloadResult::TransferIncomplete, "transferIncomplete"),
        (
            DownloadResult::TransferCompleteVerificationUnfinished,
            "transferCompleteVerificationUnfinished",
        ),
        (
            DownloadResult::TransferCompleteVerificationNotStarted,
            "transferCompleteVerificationNotStarted",
        ),
    ] {
        let outcome = if result == DownloadResult::Verified {
            LocalOutcome::Finished
        } else {
            LocalOutcome::Failed
        };
        let s = status_of(progress(outcome, true, result));
        assert_eq!(s["schemaVersion"], 4);
        assert_eq!(s["finalStepFailures"], serde_json::json!([]));
        assert_eq!(s["downloadResult"], text);
    }
}

#[test]
fn status_never_promotes_an_unfinished_or_inconsistent_run_to_verified() {
    // Verified claimed by a progress that did not finish.
    for outcome in [LocalOutcome::Failed, LocalOutcome::Cancelled] {
        let s = status_of(progress(outcome, true, DownloadResult::Verified));
        assert_eq!(s["downloadResult"], "unknown", "{outcome:?}");
    }
    // A finished state without a Verified result is not a success either.
    let s = status_of(progress(
        LocalOutcome::Finished,
        true,
        DownloadResult::TransferIncomplete,
    ));
    assert_eq!(s["downloadResult"], "unknown");
    // Running stays in progress, never a result.
    let s = status_of(progress(
        LocalOutcome::Running,
        true,
        DownloadResult::InProgress,
    ));
    assert_eq!(s["downloadResult"], "inProgress");
}

#[test]
fn an_ended_run_without_a_result_is_nothing_written_or_unknown_by_mutation_only() {
    let s = status_of(progress(
        LocalOutcome::Failed,
        false,
        DownloadResult::InProgress,
    ));
    assert_eq!(s["downloadResult"], "nothingWritten");
    let s = status_of(progress(
        LocalOutcome::Interrupted,
        true,
        DownloadResult::InProgress,
    ));
    assert_eq!(s["downloadResult"], "unknown");
}

async fn get(router: &Router, uri: &str) -> (u16, Value, String) {
    let response = router
        .clone()
        .oneshot(Request::get(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status().as_u16();
    assert_eq!(response.headers()["cache-control"], "no-store");
    let raw = String::from_utf8(
        axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap()
            .to_vec(),
    )
    .unwrap();
    (
        status,
        serde_json::from_str(&raw).unwrap_or(Value::Null),
        raw,
    )
}
fn state_with_operation() -> (tempfile::TempDir, crate::SharedState, Router) {
    let directory = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState::new(directory.path().to_path_buf()));
    let root = state.device_system2.root(&state).unwrap();
    root.create(&super::metadata()).unwrap();
    let router = crate::app(state.clone(), None);
    (directory, state, router)
}

#[tokio::test]
async fn report_of_an_operation_without_evidence_is_unknown_not_nothing_written_and_leaks_nothing()
{
    let (_dir, _state, router) = state_with_operation();
    let (status, body, raw) = get(
        &router,
        &format!("/api/device-system2/report?operationId={ID}"),
    )
    .await;
    assert_eq!(status, 200, "{raw}");
    let mut keys: Vec<_> = body.as_object().unwrap().keys().cloned().collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "download",
            "evidence",
            "executionBuild",
            "exporterBuild",
            "failureDiagnostics",
            "finalStepFailures",
            "format",
            "kind",
            "mutationPossible",
            "prestateDiffers",
            "recovery",
            "support",
        ]
    );
    assert_eq!(body["kind"], "knxbench-system2-download-summary");
    assert_eq!(body["format"], 3);
    assert_eq!(body["finalStepFailures"], serde_json::json!([]));
    // Missing journal and observed record cannot prove that nothing was written.
    assert_eq!(body["download"]["result"], "unknown");
    assert_eq!(body["mutationPossible"], true);
    assert_eq!(body["recovery"], Value::Null);
    assert_eq!(
        body["executionBuild"],
        serde_json::json!({ "state": "unknown" })
    );
    assert_eq!(body["exporterBuild"], crate::version_string());
    assert!(body["support"].as_str().unwrap().contains("UNVERIFIED"));
    for forbidden in [
        ID,
        "1.1.67",
        "Original selected device",
        "old-server",
        "metadata.json",
        "journal/",
        "checkpoint-",
        "phrase",
    ] {
        assert!(!raw.contains(forbidden), "report leaks {forbidden}");
    }
}

#[tokio::test]
async fn report_names_the_recorded_execution_build_separately_from_the_exporter() {
    let (_dir, state, router) = state_with_operation();
    let root = state.device_system2.root(&state).unwrap();
    assert!(root
        .record_execution_build(ID, "/home/alice/private-project.knxproj")
        .is_err());
    root.record_execution_build(ID, "9.9.9+gabc1234").unwrap();
    let (_, body, _) = get(
        &router,
        &format!("/api/device-system2/report?operationId={ID}"),
    )
    .await;
    assert_eq!(
        body["executionBuild"],
        serde_json::json!({ "state": "recorded", "version": "9.9.9+gabc1234" })
    );
    assert_ne!(body["executionBuild"]["version"], body["exporterBuild"]);
    // A second write never replaces the original; a foreign, oversized or corrupt record is unknown.
    assert!(root.record_execution_build(ID, "other").is_err());
    let path = _dir
        .path()
        .join(crate::device_system2::ROOT_NAME)
        .join(ID)
        .join("execution-build.json");
    for bad in [
        // Private paths and free text are not a version: the record is unknown, never sanitized.
        br#"{"format":1,"operationId":"0123456789abcdef0123456789abcdef","version":"/home/alice/private-project.knxproj"}"#.to_vec(),
        br#"{"format":1,"operationId":"0123456789abcdef0123456789abcdef","version":"build by alice"}"#.to_vec(),
        br#"{"format":1,"operationId":"0123456789abcdef0123456789abcdef","version":"1.0"}"#.to_vec(),
        br#"{"format":1,"operationId":"fedcba9876543210fedcba9876543210","version":"1.0"}"#
            .to_vec(),
        br#"{"format":2,"operationId":"0123456789abcdef0123456789abcdef","version":"1.0"}"#
            .to_vec(),
        br#"{"format":1,"operationId":"0123456789abcdef0123456789abcdef","version":"1.0","x":1}"#
            .to_vec(),
        vec![b'x'; 600],
    ] {
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, bad).unwrap();
        #[cfg(unix)]
        std::fs::set_permissions(&path, std::os::unix::fs::PermissionsExt::from_mode(0o600))
            .unwrap();
        assert_eq!(
            root.execution_build(ID),
            knx_app::property_validation::ExecutionBuild::Unknown
        );
    }
}

#[tokio::test]
async fn report_is_read_only_under_a_quarantined_coordinator_but_refuses_a_live_task() {
    let (_dir, state, router) = state_with_operation();
    // A lease abandoned without settlement quarantines mutation admission for good.
    drop(state.device_system2.control.try_admit(ID).unwrap());
    assert!(
        state.device_system2.idle().is_err(),
        "mutation stays quarantined"
    );
    let (status, body, raw) = get(
        &router,
        &format!("/api/device-system2/report?operationId={ID}"),
    )
    .await;
    assert_eq!(status, 200, "{raw}");
    assert_eq!(body["kind"], "knxbench-system2-download-summary");
}

#[tokio::test]
async fn report_refuses_running_unknown_malformed_and_corrupt_operations() {
    let (dir, state, router) = state_with_operation();
    let (status, _, _) = get(&router, "/api/device-system2/report?operationId=NOT-AN-ID").await;
    assert_eq!(status, 400);
    let (status, body, _) = get(&router, "/api/device-system2/report").await;
    assert_eq!(status, 400, "{body}");
    let other = "fedcba9876543210fedcba9876543210";
    let (status, body, _) = get(
        &router,
        &format!("/api/device-system2/report?operationId={other}"),
    )
    .await;
    assert_eq!(
        (status, body["kind"].as_str()),
        (409, Some("system2OperationUnknown"))
    );
    // Corrupt retained journal: explicit unavailable, never a guessed result.
    let journal = dir.path().join(storage_root()).join(ID).join("journal");
    std::fs::create_dir(&journal).unwrap();
    std::fs::write(journal.join("checkpoint-00000001.json"), b"not json").unwrap();
    let (status, body, raw) = get(
        &router,
        &format!("/api/device-system2/report?operationId={ID}"),
    )
    .await;
    assert_eq!(
        (status, body["kind"].as_str()),
        (409, Some("system2ReportUnavailable")),
        "{raw}"
    );
    assert!(!raw.contains("not json"));
    // A live task refuses before any journal is touched.
    let task = tokio::spawn(std::future::pending::<()>());
    state.device_system2.inner.lock().unwrap().current = Some(Record {
        metadata: super::metadata(),
        progress: initial(LocalOutcome::Running),
        updates: None,
        task: Some(task),
        control_id: ID.into(),
        message: None,
        refusal: None,
        settlement: crate::device_system2::settlement::Receipt::NotRecorded,
    });
    let (status, body, _) = get(
        &router,
        &format!("/api/device-system2/report?operationId={ID}"),
    )
    .await;
    assert_eq!(
        (status, body["kind"].as_str()),
        (409, Some("system2Busy")),
        "a live task refuses"
    );
    state
        .device_system2
        .inner
        .lock()
        .unwrap()
        .current
        .as_ref()
        .unwrap()
        .task
        .as_ref()
        .unwrap()
        .abort();
}
fn storage_root() -> &'static str {
    crate::device_system2::ROOT_NAME
}

#[test]
fn the_build_vocabulary_accepts_real_versions_only() {
    use knx_app::property_validation::ExecutionBuild;
    for good in [
        "0.1.0-alpha.17",
        "0.1.0-alpha.17+g0123abc",
        "1.2.3",
        "1.2.3+g0123456789abcdef0123456789abcdef01234567",
    ] {
        assert!(
            matches!(
                ExecutionBuild::recorded(good),
                ExecutionBuild::Recorded { .. }
            ),
            "{good}"
        );
    }
    for bad in [
        "",
        "/home/alice/p.knxproj",
        "C:\\Users\\a",
        "1.0",
        "1.2.3 (alice)",
        "1.2.3+gXYZ",
        "v1.2.3",
        "1.2.3/..",
        "1.2.3+g",
        &"1.".repeat(40),
    ] {
        assert_eq!(
            ExecutionBuild::recorded(bad),
            ExecutionBuild::Unknown,
            "{bad}"
        );
    }
    assert!(
        crate::version_string().len() > 4
            && matches!(
                ExecutionBuild::recorded(&crate::version_string()),
                ExecutionBuild::Recorded { .. }
            )
    );
}

#[test]
fn status_projects_the_typed_final_step_failures_per_run_without_text() {
    use knx_app::property_validation::{
        FailureDiagnosticRun, FinalStepCategory, FinalStepDiagnostic, FinalStepStage,
    };
    let mut p = progress(
        LocalOutcome::RestoreFailed,
        true,
        DownloadResult::TransferCompleteVerificationUnfinished,
    );
    p.download.outcome = LocalOutcome::Failed;
    p.restore_attempted = true;
    p.final_step_failures = vec![
        FinalStepDiagnostic {
            run: FailureDiagnosticRun::Download,
            category: FinalStepCategory::VerificationMismatch,
            stage: FinalStepStage::LoadedState,
        },
        FinalStepDiagnostic {
            run: FailureDiagnosticRun::Restore,
            category: FinalStepCategory::WriteFailed,
            stage: FinalStepStage::VerifyModeWrite,
        },
    ];
    let s = status_of(p);
    assert_eq!(
        s["finalStepFailures"],
        serde_json::json!([
            {"run": "download", "category": "verificationMismatch", "stage": "loadedState"},
            {"run": "restore", "category": "writeFailed", "stage": "verifyModeWrite"},
        ])
    );
    assert_eq!(
        s["downloadResult"],
        "transferCompleteVerificationUnfinished"
    );
}
