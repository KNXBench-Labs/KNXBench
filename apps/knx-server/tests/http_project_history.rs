//! Verifies native version actions, consent, stale guards and edit rollback over HTTP.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn request(
    app: &axum::Router,
    method: &str,
    path: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}

async fn fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    Arc<knx_server::AppState>,
    axum::Router,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.knxdb");
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/new",
            json!({"name": "Version witness"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "POST", "/api/project/save-as", json!({"path": path}))
            .await
            .0,
        StatusCode::OK
    );
    (dir, path, state, app)
}

fn binding(view: &Value, confirmed: bool) -> Value {
    json!({"serverIncarnation": view["serverIncarnation"], "snapshotRevision": view["snapshotRevision"],
        "generation": view["generation"], "confirmed": confirmed})
}

#[tokio::test]
async fn named_version_restore_preserves_unsaved_pre_restore_state_and_clears_edit_stack() {
    let (_dir, _path, state, app) = fixture().await;
    let before = state.project.lock().unwrap().clone().unwrap();
    let (status, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["persistence"], "native");
    let mut payload = binding(&view, false);
    payload["label"] = json!("Before redesign");
    let (status, view) = request(&app, "POST", "/api/project/history/versions", payload).await;
    assert_eq!(status, StatusCode::OK);
    let id = view["versions"][0]["id"].as_i64().unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle": "Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let modified = state.project.lock().unwrap().clone().unwrap();
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let refused = request(
        &app,
        "POST",
        &format!("/api/project/history/versions/{id}/restore"),
        binding(&view, false),
    )
    .await;
    assert_eq!(refused.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&modified));
    let restored = request(
        &app,
        "POST",
        &format!("/api/project/history/versions/{id}/restore"),
        binding(&view, true),
    )
    .await;
    assert_eq!(restored.0, StatusCode::OK);
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&before));
    assert!(!state.command_stack.lock().unwrap().can_undo());
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let versions = view["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 2);
    let safety = versions
        .iter()
        .find(|v| v["reason"] == "pre_restore")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/project/history/versions/{safety}/restore"),
            binding(&view, true)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&modified));
}

#[tokio::test]
async fn stale_revision_incarnation_or_generation_cannot_create_restore_or_delete_a_version() {
    let (_dir, path, state, app) = fixture().await;
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    for key in ["serverIncarnation", "snapshotRevision", "generation"] {
        let before = state.project.lock().unwrap().clone();
        let bytes = std::fs::read(&path).unwrap();
        let mut payload = binding(&view, true);
        payload["label"] = json!("Stale");
        payload[key] = if key == "serverIncarnation" {
            json!("different server")
        } else {
            json!(999_999)
        };
        let result = request(&app, "POST", "/api/project/history/versions", payload).await;
        assert_eq!(
            result.0,
            StatusCode::CONFLICT,
            "binding {key} was not enforced"
        );
        assert_eq!(*state.project.lock().unwrap(), before);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[tokio::test]
async fn durable_command_failure_does_not_publish_model_stack_or_revision() {
    let (_dir, path, state, app) = fixture().await;
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle": "Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    // Refuse only the second durable undo row: admission accepts this
    // payload-free index, so the failure is still inside journal persistence.
    conn.execute_batch("CREATE UNIQUE INDEX refuse_journal ON project_history_stack(side)")
        .unwrap();
    let (_, before) = request(&app, "GET", "/api/project", Value::Null).await;
    let model = state.project.lock().unwrap().clone();
    let result = request(
        &app,
        "POST",
        "/api/project/group-address-style",
        json!({"groupAddressStyle": "TwoLevel"}),
    )
    .await;
    assert!(!result.0.is_success());
    let (_, after) = request(&app, "GET", "/api/project", Value::Null).await;
    assert_eq!(after, before);
    assert_eq!(*state.project.lock().unwrap(), model);
    assert_eq!(
        knx_store::project_history::load_editor(&conn)
            .unwrap()
            .unwrap()
            .working
            .project,
        model.unwrap()
    );
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
}

#[tokio::test]
async fn two_native_editors_cannot_overwrite_each_others_recovery_journal() {
    let (dir, path, state, first) = fixture().await;
    let other = Arc::new(knx_server::AppState::new(dir.path().join("other")));
    let second = knx_server::app(other.clone(), None);
    assert_eq!(
        request(&second, "POST", "/api/project/open", json!({"path": path}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &first,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle": "Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let before = other.project.lock().unwrap().clone();
    let result = request(
        &second,
        "POST",
        "/api/project/group-address-style",
        json!({"groupAddressStyle": "TwoLevel"}),
    )
    .await;
    assert!(!result.0.is_success());
    assert_eq!(*other.project.lock().unwrap(), before);
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .info
            .group_address_style,
        knx_core::GroupAddressStyle::Free
    );
}

#[tokio::test]
async fn session_only_history_is_explicit_and_version_writes_require_a_native_path() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state, None);
    assert_eq!(
        request(&app, "POST", "/api/project/new", json!({})).await.0,
        StatusCode::OK
    );
    let (status, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["persistence"], "session");
    let mut payload = binding(&view, false);
    payload["label"] = json!("Cannot persist yet");
    assert_eq!(
        request(&app, "POST", "/api/project/history/versions", payload)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn opening_a_legacy_native_store_validates_history_without_rewriting_the_file() {
    let (dir, path, _state, _app) = fixture().await;
    let conn = knx_store::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE project_history_stack; DROP TABLE project_history_state; DROP TABLE project_history_version; DROP TABLE project_history_context; ALTER TABLE line DROP COLUMN model_position; PRAGMA user_version = 10;").unwrap();
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    let app = knx_server::app(
        Arc::new(knx_server::AppState::new(dir.path().join("legacy-reader"))),
        None,
    );
    assert_eq!(
        request(&app, "POST", "/api/project/open", json!({"path": path}))
            .await
            .0,
        StatusCode::OK
    );
    assert!(
        std::fs::read(&path).unwrap() == before,
        "opening must not migrate the user's legacy file"
    );
    let (status, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["generation"], 0);
    let mut payload = binding(&view, false);
    payload["label"] = json!("First native history version");
    assert_eq!(
        request(&app, "POST", "/api/project/history/versions", payload)
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn restoring_a_version_republishes_live_context_without_sending_bus_frames() {
    use knx_server::fake::{FakeConnector, FakeTunnel};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("context-history.knxdb");
    let (tunnel, handle) = FakeTunnel::new(knx_core::IndividualAddress::new(1, 1, 5).unwrap(), 64);
    let state = Arc::new(knx_server::AppState {
        connector: Box::new(FakeConnector::succeeding(tunnel)),
        ..knx_server::AppState::new(dir.path().to_path_buf())
    });
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/new",
            json!({"name": "Synthetic context history"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "POST", "/api/project/save-as", json!({"path": path}))
            .await
            .0,
        StatusCode::OK
    );
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let mut payload = binding(&view, false);
    payload["label"] = json!("Original rendering");
    let (status, view) = request(&app, "POST", "/api/project/history/versions", payload).await;
    assert_eq!(status, StatusCode::OK);
    let id = view["versions"][0]["id"].as_i64().unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/bus/monitor/start",
            json!({"gateway": "192.0.2.10:3671"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let session_id = state.bus_session.lock().await.as_ref().unwrap().id();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle": "Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        state
            .bus_session
            .lock()
            .await
            .as_ref()
            .unwrap()
            .group_address_style(),
        Some(knx_core::GroupAddressStyle::Free)
    );
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let (status, restored) = request(
        &app,
        "POST",
        &format!("/api/project/history/versions/{id}/restore"),
        binding(&view, true),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        state
            .bus_session
            .lock()
            .await
            .as_ref()
            .unwrap()
            .group_address_style(),
        Some(knx_core::GroupAddressStyle::ThreeLevel)
    );
    assert_eq!(
        restored["project"]["group_address_context_session_id"],
        session_id
    );
    assert!(
        handle.sent_calls().is_empty(),
        "restoring native engineering data must not send bus traffic"
    );
    assert_eq!(
        request(&app, "POST", "/api/bus/monitor/stop", Value::Null)
            .await
            .0,
        StatusCode::OK
    );
}
