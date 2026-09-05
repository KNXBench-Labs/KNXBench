use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn state_with_data_dir() -> (knx_server::AppState, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("a.knxproj"), b"not a real project, just a listing fixture").unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    (knx_server::AppState::new(dir.path().to_path_buf()), dir)
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn listing_the_data_dir_root_shows_its_entries() {
    let (state, _dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(Request::builder().uri("/api/fs/list").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let entries = body_json(response).await;
    let names: Vec<&str> = entries.as_array().unwrap().iter().map(|e| e["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"a.knxproj"));
    assert!(names.contains(&"sub"));
}

#[tokio::test]
async fn listing_a_path_that_escapes_the_data_dir_is_a_400() {
    let (state, _dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/fs/list?path=../../../../etc")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn downloading_with_no_project_open_is_a_400() {
    let (state, _dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(Request::builder().uri("/api/project/download").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
