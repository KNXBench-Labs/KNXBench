use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crate lives at <root>/apps/knx-server")
        .to_path_buf()
}

fn reference_ets4_path() -> PathBuf {
    workspace_root().join("OriginalData/DemoProjects/Unser Zuhause ets4 - 2025-12-15.knxproj")
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn importing_the_reference_project_returns_the_golden_counts() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    assert_eq!(tree["errors"], 0);
    assert_eq!(tree["warnings"], 2);
    assert_eq!(
        tree["installations"][0]["topology"][0]["lines"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn importing_a_missing_file_is_a_500() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": "/does/not/exist.knxproj" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    let body = body_json(response).await;
    assert!(!body["error"].as_str().unwrap().is_empty());
}

#[tokio::test]
async fn saving_without_an_open_project_is_a_500() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
}

#[tokio::test]
async fn importing_then_saving_as_then_reopening_round_trips() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roundtrip.knxdb");

    let import_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/import")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": reference_ets4_path().to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(import_response.status(), StatusCode::OK);

    let save_response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save-as")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(save_response.status(), StatusCode::OK);

    let reopen_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/open")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": db_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reopen_response.status(), StatusCode::OK);
    let tree = body_json(reopen_response).await;
    assert_eq!(tree["errors"], 0);
    assert_eq!(tree["warnings"], 0);
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[tokio::test]
async fn a_new_project_is_seeded_with_exactly_one_empty_installation() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);

    let response = app
        .oneshot(post(
            "/api/project/new",
            json!({ "name": "Scratch", "installationName": "Ground floor" }),
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    let installations = tree["installations"].as_array().unwrap();
    assert_eq!(installations.len(), 1);
    assert_eq!(installations[0]["name"], "Ground floor");
    // No area and no line: `Command::CreateDevice` takes `line: None` and
    // parks the device in `unassigned`, so none is invented here.
    assert!(installations[0]["topology"].as_array().unwrap().is_empty());
    assert!(installations[0]["buildings"].as_array().unwrap().is_empty());
    assert!(installations[0]["unassigned"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(installations[0]["group_addresses"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(tree["errors"], 0);
    assert_eq!(tree["warnings"], 0);
}

#[tokio::test]
async fn a_new_project_refuses_to_discard_unsaved_edits_unless_told_to() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);

    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    // An unsaved edit. Nothing in `AppState` tracks dirtiness, so the
    // command stack having something to undo is the signal.
    assert_eq!(
        app.clone()
            .oneshot(post("/api/areas", json!({ "name": "A", "address": 1 })))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    let refused = app
        .clone()
        .oneshot(post("/api/project/new", json!({})))
        .await
        .unwrap();
    assert_eq!(refused.status(), StatusCode::CONFLICT);
    assert!(body_json(refused).await["error"]
        .as_str()
        .unwrap()
        .contains("unsaved changes"));

    // The edit survived the refusal.
    let log = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/log")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(body_json(log)
        .await
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e["source"] == "new" && e["severity"] == "warning"));

    let discarded = app
        .oneshot(post("/api/project/new", json!({ "discardChanges": true })))
        .await
        .unwrap();
    assert_eq!(discarded.status(), StatusCode::OK);
    let tree = body_json(discarded).await;
    assert!(tree["installations"][0]["topology"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn a_new_project_clears_the_path_the_previous_one_was_loaded_from() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("previous.knxdb");
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);

    assert_eq!(
        app.clone()
            .oneshot(post(
                "/api/project/import",
                json!({ "path": reference_ets4_path().to_string_lossy() }),
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/api/project/save-as",
                json!({ "path": db_path.to_string_lossy() }),
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    let before = std::fs::metadata(&db_path).unwrap().len();

    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    // Save must now refuse: an empty project silently overwriting the file
    // the previous one came from is exactly the data loss this guards.
    let save = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/save")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_ne!(save.status(), StatusCode::OK);
    assert_eq!(std::fs::metadata(&db_path).unwrap().len(), before);
}
