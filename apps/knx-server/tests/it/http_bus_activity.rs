//! The aggregate bus snapshot is read-only and explicitly incomplete.
//!
//! No real connector or KNX device is contacted by these tests.
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_server::{app, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn snapshot(state: Arc<AppState>) -> (StatusCode, Value) {
    let response = app(state, None)
        .oneshot(
            Request::builder()
                .uri("/api/bus/activity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&body).unwrap())
}

fn state() -> (tempfile::TempDir, Arc<AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        product_db: None,
        ..AppState::new(dir.path().to_path_buf())
    });
    (dir, state)
}

#[tokio::test]
async fn empty_snapshot_does_not_pretend_to_cover_one_shot_operations() {
    let (_dir, state) = state();
    let (status, body) = snapshot(Arc::clone(&state)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["serverIncarnation"], state.server_incarnation);
    assert_eq!(body["coverage"], "partial");
    assert_eq!(body["sessions"], json!([]));
    assert_eq!(body["busyLocks"], json!([]));
    assert_eq!(body["oneShot"], json!([]));
    assert_eq!(body["oneShotDropped"], 0);
    assert_eq!(body["untracked"], json!(["groupWrite", "serialAddress"]));
}

#[tokio::test]
async fn a_locked_route_is_reported_without_waiting_or_claiming_idle() {
    let (_dir, state) = state();
    let downloading = state.device_download.lock().await;
    let (status, body) = snapshot(Arc::clone(&state)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["busyLocks"], json!(["deviceDownload"]));
    assert_eq!(body["sessions"], json!([]));
    drop(downloading);

    let programming = state.address_programming.lock().await;
    let (status, body) = snapshot(Arc::clone(&state)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["busyLocks"], json!(["managementOperation"]));
    drop(programming);

    let monitoring = state.bus_session.lock().await;
    let (status, body) = snapshot(Arc::clone(&state)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["busyLocks"], json!(["busMonitorOrGroupWrite"]));
    drop(monitoring);

    let _scanning = state.line_scan_session.lock().await;
    let (status, body) = snapshot(Arc::clone(&state)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["busyLocks"], json!(["lineScan"]));
}

/// Issue #30: an admitted System-2 operation holds the shared tunnel locks
/// itself. It is named first, so a client never reports its own run as a
/// foreign address or management operation.
#[tokio::test]
async fn a_running_system2_operation_is_named_before_the_locks_it_holds() {
    let (_dir, state) = state();
    let lease = state
        .device_system2
        .control
        .try_admit("0123456789abcdef0123456789abcdef")
        .unwrap();
    let (status, body) = snapshot(Arc::clone(&state)).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["busyLocks"], json!(["system2Operation"]));
    let programming = state.address_programming.lock().await;
    let (_, body) = snapshot(Arc::clone(&state)).await;
    assert_eq!(
        body["busyLocks"],
        json!(["system2Operation", "managementOperation"])
    );
    assert_eq!(body["sessions"], json!([]));
    drop(programming);
    lease
        .settle(knx_app::property_validation::LocalOutcome::Failed)
        .unwrap();
    let (_, body) = snapshot(Arc::clone(&state)).await;
    assert_eq!(body["busyLocks"], json!([]));
    // Dropping an unsettled owner quarantines the coordinator; do not call
    // that uncertain state a running operation or invent a live holder.
    let uncertain = state
        .device_system2
        .control
        .try_admit("fedcba9876543210fedcba9876543210")
        .unwrap();
    drop(uncertain);
    assert!(matches!(
        state.device_system2.control.snapshot().settlement,
        knx_app::property_validation::System2Settlement::Uncertain { .. }
    ));
    let (_, body) = snapshot(Arc::clone(&state)).await;
    assert_eq!(body["busyLocks"], json!([]));
}

#[tokio::test]
async fn activity_does_not_accept_a_write_method() {
    let (_dir, state) = state();
    let response = app(state, None)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/bus/activity")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
