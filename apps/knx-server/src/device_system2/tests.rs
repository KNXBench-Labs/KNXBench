//! Regression assertions reserved for the single final integrated candidate.
mod mounted_hardware;

use super::dto::*;

#[test]
fn closed_mutating_grammar_rejects_duplicates_unknowns_and_noncanonical_ids() {
    assert!(
        decode::<Prepare>(br#"{"deviceId":1,"deviceId":2,"sourceMode":"retainedSources"}"#)
            .is_err()
    );
    assert!(
        decode::<Prepare>(br#"{"deviceId":1,"sourceMode":"retainedSources","bundle":{}}"#).is_err()
    );
    assert!(decode::<Prepare>(br#"{"deviceId":1,"sourceMode":"defaults"}"#).is_err());
    assert!(decode::<Operation>(br#"{"operationId":"a","operationId":"b"}"#).is_err());
    assert!(decode::<Prepare>(br#"{"deviceId":1,"sourceMode":{"retainedSources":null}}"#).is_err());
    assert!(valid_id("0123456789abcdef0123456789abcdef"));
    assert!(!valid_id("0123456789abcdef0123456789abcdeF"));
    assert!(!valid_id("../0123456789abcdef0123456789abcd"));
}

#[tokio::test]
async fn mounted_routes_are_authenticated_and_idle_is_null() {
    use axum::{body::Body, http::Request};
    use std::sync::Arc;
    use tower::ServiceExt;
    let directory = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState::new(directory.path().to_path_buf()));
    let response = crate::app(state.clone(), None)
        .oneshot(
            Request::get("/api/device-system2/status")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        &axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap()[..],
        b"null"
    );
    let auth =
        crate::AuthConfig::from_plaintext_password("test-only-not-a-user-credential").unwrap();
    let router = crate::app_with_auth(state.clone(), None, auth);
    for (method, action) in [
        ("GET", "status"),
        ("GET", "operations"),
        ("POST", "prepare"),
        ("POST", "start"),
        ("POST", "cancel"),
        ("POST", "recover"),
        ("POST", "select"),
    ] {
        let request = Request::builder()
            .method(method)
            .uri(format!("/api/device-system2/{action}"))
            .header("Content-Type", "application/json")
            .body(Body::from("{}"))
            .unwrap();
        assert_eq!(router.clone().oneshot(request).await.unwrap().status(), 401);
    }
    crate::close_process_admission(&state);
    assert!(!state.device_system2.control.snapshot().accepting);
}

fn metadata() -> Metadata {
    Metadata {
        schema_version: 1,
        operation_id: "0123456789abcdef0123456789abcdef".into(),
        binding: Binding {
            server_incarnation: "old-server".into(),
            project_incarnation: "1".into(),
            project_revision: "2".into(),
            device_id: 7,
        },
        address: "1.1.67".into(),
        device_name: "Original selected device".into(),
    }
}
#[test]
fn restart_reattaches_selected_original_without_inventing_a_backup() {
    let directory = tempfile::tempdir().unwrap();
    let root = super::storage::Storage::open(directory.path(), true)
        .unwrap()
        .unwrap();
    let meta = metadata();
    root.create(&meta).unwrap();
    drop(root);
    let service = super::System2Service::new(directory.path());
    let status = service.status().unwrap().unwrap();
    assert_eq!(status.metadata.binding.server_incarnation, "old-server");
    assert_eq!(status.metadata.operation_id, meta.operation_id);
    assert_eq!(status.state, "interrupted");
    assert!(!status.backup_available);
    assert!(status.recovery_phrase.is_none());
    assert!(status.download_phrase.is_none());
    assert!(status.release_phrase.is_none());
    assert!(matches!(
        service.control.snapshot().settlement,
        knx_app::property_validation::System2Settlement::Idle
    ));
}

#[tokio::test]
async fn timeout_keeps_actual_task_and_busy_legacy_locks_cannot_escape_their_deadline() {
    use knx_app::property_validation::LocalOutcome;
    use std::{sync::Arc, time::Duration};
    let directory = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState::new(directory.path().to_path_buf()));
    let meta = metadata();
    let lease = state
        .device_system2
        .control
        .try_admit(&meta.operation_id)
        .unwrap();
    let cancel = lease.cancel_token();
    let (release, wait) = tokio::sync::oneshot::channel::<()>();
    let copy = state.clone();
    let id = meta.operation_id.clone();
    let task = tokio::spawn(async move {
        let _download = copy.device_download.lock().await;
        let _programming = copy.address_programming.lock().await;
        let _ = wait.await;
        copy.device_system2
            .publish_end(&id, LocalOutcome::Cancelled, None);
        drop(_programming);
        drop(_download);
        lease.settle(LocalOutcome::Cancelled).unwrap();
    });
    state.device_system2.inner.lock().unwrap().current = Some(super::Record {
        metadata: meta.clone(),
        progress: super::initial(LocalOutcome::Running),
        updates: None,
        task: Some(task),
        control_id: meta.operation_id,
        message: None,
    });
    // Directly hold an admission lock: the ordinary drain's initial inspection
    // must itself be inside its bounded behavior, not wait before starting it.
    let programming = state.address_programming.lock().await;
    let writes = tokio::time::timeout(
        Duration::from_secs(1),
        crate::drain_device_writes(&state, Duration::ZERO, std::future::pending()),
    )
    .await
    .unwrap();
    assert_eq!(writes.end, crate::WriteWaitEnd::TimedOut);
    drop(programming);
    let drain = super::drain_device_system2(&state, Duration::ZERO, std::future::pending()).await;
    assert!(drain.work_outstanding);
    assert!(cancel.load(std::sync::atomic::Ordering::SeqCst));
    assert!(state
        .device_system2
        .inner
        .lock()
        .unwrap()
        .current
        .as_ref()
        .unwrap()
        .task
        .is_some());
    release.send(()).unwrap();
    let settled =
        super::drain_device_system2(&state, Duration::from_secs(1), std::future::pending()).await;
    assert!(!settled.work_outstanding);
    assert!(state
        .device_system2
        .inner
        .lock()
        .unwrap()
        .current
        .as_ref()
        .unwrap()
        .task
        .is_none());
}

#[cfg(unix)]
#[tokio::test]
async fn recovery_evidence_is_fenced_from_file_routes_and_static_symlink_aliases() {
    use axum::{body::Body, http::Request};
    use std::sync::Arc;
    use tower::ServiceExt;
    let directory = tempfile::tempdir().unwrap();
    let root = super::storage::Storage::open(directory.path(), true)
        .unwrap()
        .unwrap();
    let meta = metadata();
    root.create(&meta).unwrap();
    drop(root);
    let public = tempfile::tempdir().unwrap();
    std::os::unix::fs::symlink(
        directory.path().join(super::ROOT_NAME),
        public.path().join("alias"),
    )
    .unwrap();
    std::os::unix::fs::symlink(
        directory
            .path()
            .join(super::ROOT_NAME)
            .join(&meta.operation_id)
            .join("metadata.json"),
        public.path().join("index.html"),
    )
    .unwrap();
    let state = Arc::new(crate::AppState::new(directory.path().to_path_buf()));
    let router = crate::app(state, Some(public.path().to_path_buf()));
    for uri in [
        "/".to_owned(),
        format!("/alias/{}/metadata.json", meta.operation_id),
        format!("/{}/{}/metadata.json", super::ROOT_NAME, meta.operation_id),
    ] {
        assert_eq!(
            router
                .clone()
                .oneshot(Request::get(uri).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            404
        );
    }
    assert!(crate::paths::resolve_in_data_dir(directory.path(), super::ROOT_NAME).is_err());
}

fn settled_status(progress: knx_app::property_validation::OperationProgress) -> serde_json::Value {
    let meta = metadata();
    let record = super::Record {
        metadata: meta.clone(),
        progress,
        updates: None,
        task: None,
        control_id: meta.operation_id,
        message: None,
    };
    serde_json::to_value(record.view()).unwrap()
}
#[test]
fn status_reports_whether_mutation_may_have_started_for_failed_operations() {
    use knx_app::property_validation::LocalOutcome;
    // Issue #30: a refusal before the durable backup barrier wrote nothing.
    let refused = settled_status(super::initial(LocalOutcome::Failed));
    assert_eq!(refused["state"], "failed");
    assert_eq!(refused["mutationMayHaveStarted"], false);
    // A mid-write failure keeps the conservative flag.
    let mut progress = super::initial(LocalOutcome::Failed);
    progress.mutation_possible = true;
    progress.backup_available = true;
    let mid_write = settled_status(progress);
    assert_eq!(mid_write["mutationMayHaveStarted"], true);
    let mut progress = super::initial(LocalOutcome::Cancelled);
    progress.mutation_possible = true;
    assert_eq!(settled_status(progress)["mutationMayHaveStarted"], true);
}
#[test]
fn status_carries_prestate_observations_and_offers_no_recovery_after_a_difference() {
    use knx_app::property_validation::{
        LocalOutcome, PointerObservationPhase, PointerObservationProgress, PointerObservationRun,
    };
    use knx_app::property_validation_bundle::PointerObservationRecord;
    let mut progress = super::initial(LocalOutcome::Finished);
    progress.backup_available = true;
    progress.mutation_possible = true;
    progress.prestate_differs = true;
    progress.pointer_observations = vec![
        PointerObservationProgress {
            run: PointerObservationRun::Download,
            phase: PointerObservationPhase::Prestate,
            operation_index: None,
            record: PointerObservationRecord {
                object: 3,
                expected: [0, 0, 0x3c, 0],
                observed: Some([0; 4]),
                state: 1,
            },
        },
        PointerObservationProgress {
            run: PointerObservationRun::Restore,
            phase: PointerObservationPhase::InProcedure,
            operation_index: Some(4),
            record: PointerObservationRecord {
                object: 3,
                expected: [0, 0, 0x3c, 0],
                observed: None,
                state: 2,
            },
        },
    ];
    progress.program_version_observation = Some(([0, 1, 0x92, 5, 4], Vec::new()));
    let status = settled_status(progress);
    assert_eq!(status["prestateDiffers"], true);
    assert_eq!(
        status["pointerObservations"],
        serde_json::json!([
            {"run": "download", "phase": "prestate", "object": 3, "operationIndex": null, "expected": "00003c00",
                "observed": "00000000", "loadState": "Loaded"},
            {"run": "restore", "phase": "inProcedure", "object": 3, "operationIndex": 4, "expected": "00003c00",
                "observed": "refused", "loadState": "Loading"}
        ])
    );
    assert_eq!(
        status["programVersionObservation"],
        serde_json::json!({"expected": "0001920504", "observed": ""})
    );
    assert!(status["recoveryPhrase"].is_null());
}
