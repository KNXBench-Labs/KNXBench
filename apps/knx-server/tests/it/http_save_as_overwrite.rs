//! Save As refuses an existing file unless the request explicitly asks to replace it.
//!
//! Owner request 2026-10-14: saving must ask before an existing file is
//! overwritten. The question lives in the client; this guarantee lives in
//! the server, so no client, old or new, can replace a file by accident.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

fn get(uri: &str) -> Request<Body> {
    Request::builder().uri(uri).body(Body::empty()).unwrap()
}

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, body)
}

/// A server with one new project, saved once as `first.knxdb`, then edited.
async fn saved_then_edited() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state, None);
    let (status, _) = send(
        &app,
        post(
            "/api/project/new",
            json!({ "name": "First", "force": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (status, _) = send(
        &app,
        post("/api/project/save-as", json!({ "path": "first.knxdb" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "a new file needs no overwrite flag");
    let (status, _) = send(
        &app,
        post("/api/areas", json!({ "name": "Area A", "address": 1 })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    (dir, app)
}

#[tokio::test]
async fn save_as_onto_an_existing_file_is_refused_and_writes_nothing() {
    let (dir, app) = saved_then_edited().await;
    let target = dir.path().join("other.knxdb");
    std::fs::copy(dir.path().join("first.knxdb"), &target).unwrap();
    let before = std::fs::read(&target).unwrap();
    let (_, project_before) = send(&app, get("/api/project")).await;

    let (status, body) = send(
        &app,
        post("/api/project/save-as", json!({ "path": "other.knxdb" })),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["kind"], "fileExists");
    assert_eq!(
        std::fs::read(&target).unwrap(),
        before,
        "not one byte written"
    );
    let (_, project_after) = send(&app, get("/api/project")).await;
    assert_eq!(project_after["is_modified"], true, "still unsaved");
    assert_eq!(
        project_after["view_key"], project_before["view_key"],
        "the project still belongs to its old file"
    );
    assert_eq!(
        project_after["last_saved_at"],
        project_before["last_saved_at"]
    );
}

#[tokio::test]
async fn an_explicit_overwrite_replaces_and_keeps_the_previous_state_in_history() {
    let (dir, app) = saved_then_edited().await;

    // Save As onto the project's own file is a replacement too.
    let (status, body) = send(
        &app,
        post("/api/project/save-as", json!({ "path": "first.knxdb" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["kind"], "fileExists");

    let (status, _) = send(
        &app,
        post(
            "/api/project/save-as",
            json!({ "path": "first.knxdb", "overwrite": true }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, project) = send(&app, get("/api/project")).await;
    assert_eq!(project["is_modified"], false);

    let conn =
        knx_store::project_history::open_editor_store(&dir.path().join("first.knxdb"), false)
            .unwrap();
    let versions = knx_store::project_history::list_versions(&conn).unwrap();
    assert!(
        versions.iter().any(|version| version.reason == "save"),
        "the replaced state stays restorable: {versions:?}"
    );
}

#[tokio::test]
async fn a_file_that_is_not_a_knxbench_project_is_never_overwritten() {
    let (dir, app) = saved_then_edited().await;
    let target = dir.path().join("notes.knxdb");
    std::fs::write(&target, b"not a project, just somebody's notes").unwrap();

    let (status, _) = send(
        &app,
        post(
            "/api/project/save-as",
            json!({ "path": "notes.knxdb", "overwrite": true }),
        ),
    )
    .await;

    assert_ne!(status, StatusCode::OK);
    assert_eq!(
        std::fs::read(&target).unwrap(),
        b"not a project, just somebody's notes"
    );
}
