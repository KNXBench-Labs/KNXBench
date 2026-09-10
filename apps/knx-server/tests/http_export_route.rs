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
async fn exporting_without_a_store_path_is_a_400() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let export_path = dir.path().join("out.knxproj");

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

    // No save-as in between: `store_path` is still unset.
    let export_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/export")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": export_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(export_response.status(), StatusCode::BAD_REQUEST);
    let body = body_json(export_response).await;
    let error = body["error"].as_str().unwrap();
    assert!(!error.is_empty());
    assert!(
        error.contains(".knxdb"),
        "error should point at saving as .knxdb first, got: {error}"
    );
}

#[tokio::test]
async fn importing_saving_then_exporting_round_trips_and_reports_the_unsigned_warning() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("roundtrip.knxdb");
    let export_path = dir.path().join("roundtrip.knxproj");

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

    let export_response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/export")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "path": export_path.to_string_lossy() }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(export_response.status(), StatusCode::OK);
    let report = body_json(export_response).await;
    let warnings = report["warnings"].as_array().unwrap();
    // Every export is unsigned (`export_knxproj`'s own doc comment: "Always
    // present"); the JSON tag is the externally-tagged, camelCase-rendered
    // unit-like-variant key `serde` actually emits for `ExportWarningDto`
    // (confirmed against a throwaway `serde_json::to_string` check, not
    // guessed) — `{"unsigned":{"detail": "..."}}`.
    assert!(
        warnings.iter().any(|w| w.get("unsigned").is_some()),
        "expected an 'unsigned' warning in {warnings:?}"
    );

    assert!(
        std::fs::metadata(&export_path).unwrap().len() > 0,
        "export should have written a non-empty .knxproj file"
    );
}
