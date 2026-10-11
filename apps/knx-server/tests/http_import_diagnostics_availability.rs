//! Native open must not turn missing historical import evidence into a measured zero.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn request(app: &axum::Router, method: &str, path: &str, value: Value) -> Value {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(value.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), 1024 * 1024)
        .await
        .unwrap();
    if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

#[tokio::test]
async fn native_reopen_marks_import_counts_unavailable_through_edit_undo_save_and_get() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("diagnostics.knxdb");
    let state = Arc::new(knx_server::AppState::new(dir.path().into()));
    let app = knx_server::app(state.clone(), None);
    let new = request(
        &app,
        "POST",
        "/api/project/new",
        json!({"name":"Synthetic diagnostics witness"}),
    )
    .await;
    assert_eq!(new["import_report_available"], true);
    // Mimics an already produced import count, not a second importer or a persisted report.
    *state.import_counts.lock().unwrap() = (2, 7);
    let measured = request(&app, "GET", "/api/project", Value::Null).await;
    assert_eq!(measured["import_report_available"], true);
    assert_eq!(measured["warnings"], 7);
    request(&app, "POST", "/api/project/save-as", json!({"path":path})).await;
    drop(app);
    drop(state);
    let state = Arc::new(knx_server::AppState::new(dir.path().into()));
    let app = knx_server::app(state.clone(), None);
    let opened = request(&app, "POST", "/api/project/open", json!({"path":path})).await;
    assert_eq!(
        opened["import_report_available"], false,
        "missing historical diagnostics are not a measured zero"
    );
    let detached =
        serde_json::to_value(knx_server::open_native_project_impl(&path).unwrap()).unwrap();
    assert_eq!(detached["import_report_available"], false);
    // A current-session diagnostic does not magically restore the missing history.
    *state.import_counts.lock().unwrap() = (0, 3);
    let current = request(&app, "GET", "/api/project", Value::Null).await;
    assert_eq!(current["import_report_available"], false);
    assert_eq!(current["warnings"], 3);
    for (method, route, body) in [
        ("GET", "/api/project", Value::Null),
        (
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle":"Free"}),
        ),
        ("POST", "/api/undo", Value::Null),
    ] {
        let tree = request(&app, method, route, body).await;
        assert_eq!(tree["import_report_available"], false);
    }
    request(&app, "POST", "/api/project/save", Value::Null).await;
    let tree = request(&app, "GET", "/api/project", Value::Null).await;
    assert_eq!(tree["import_report_available"], false);
    let reset = request(
        &app,
        "POST",
        "/api/project/new",
        json!({"name":"Fresh project"}),
    )
    .await;
    assert_eq!(reset["import_report_available"], true);
}
