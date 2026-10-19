//! A support report is available before any project, catalogue, plan or bus connection.
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use tower::ServiceExt;

#[tokio::test]
async fn unknown_device_preview_exports_only_after_snapshot_consent() {
    let dir = tempfile::tempdir().unwrap();
    let app = knx_server::app(
        std::sync::Arc::new(knx_server::AppState::new(dir.path().into())),
        None,
    );
    let request = |uri: &str, v: Value| {
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(v.to_string()))
            .unwrap()
    };
    let r = app
        .clone()
        .oneshot(request("/api/support/preview", json!({"input":{}})))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    let p: Value =
        serde_json::from_slice(&to_bytes(r.into_body(), 1024 * 1024).await.unwrap()).unwrap();
    assert_eq!(p["report"]["support"], "unknown");
    let r = app
        .clone()
        .oneshot(request(
            "/api/support/export",
            json!({"token":p["token"],"digest":p["digest"],"consent":false}),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::BAD_REQUEST);
    let r = app
        .oneshot(request(
            "/api/support/export",
            json!({"token":p["token"],"digest":p["digest"],"consent":true}),
        ))
        .await
        .unwrap();
    assert_eq!(r.status(), StatusCode::OK);
    assert_eq!(r.headers()["content-type"], "application/zip");
}
