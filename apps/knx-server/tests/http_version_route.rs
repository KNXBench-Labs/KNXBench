//! `GET /api/version` answers with the build's version and no crate name, project or not.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use tower::ServiceExt;

async fn version_body(app: axum::Router) -> serde_json::Value {
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/version")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 64 * 1024)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn version_route_serves_the_build_version() {
    let state = Arc::new(knx_server::AppState::default());
    let body = version_body(knx_server::app(state, None)).await;
    let version = body["version"].as_str().expect("version must be a string");
    assert_eq!(version, knx_server::version_string());
    // The About dialog puts the application's name in front of this, so
    // the crate's name must not already be in it.
    assert!(
        !version.contains("knx-server"),
        "the route must serve the version alone, got {version}"
    );
    assert!(
        version.starts_with(env!("CARGO_PKG_VERSION")),
        "expected the manifest version at the front, got {version}"
    );
}

// The route takes no state and reads no project, and the About dialog is
// reachable from the welcome screen — before anything has been opened.
#[tokio::test]
async fn version_route_answers_with_no_project_open() {
    let state = Arc::new(knx_server::AppState::default());
    assert!(state.project.lock().unwrap().is_none());
    let body = version_body(knx_server::app(state, None)).await;
    assert!(!body["version"].as_str().unwrap().is_empty());
}
