//! `POST /api/project/import` with a password-protected `.knxproj` (AR08, KL-13).
//!
//! Wire contract: the request body gains an optional `password` string.
//! A protected project without one is refused with `422` and
//! `kind: "projectPasswordRequired"`, a wrong one with `422` and
//! `kind: "projectPasswordWrong"`; both leave the open project untouched.
//! The password never comes back in a response, the session log or the
//! load-progress snapshot.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use knx_testsupport::{write_zipcrypto_minimal_knxproj, ZIPCRYPTO_MINIMAL_PASSWORD};

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(match body {
            Some(body) => Body::from(body.to_string()),
            None => Body::empty(),
        })
        .unwrap();
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn app() -> axum::Router {
    knx_server::app(Arc::new(knx_server::AppState::default()), None)
}

#[tokio::test]
async fn the_right_password_imports_the_protected_project() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let app = app();
    let (status, body) = call(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({ "path": source, "password": ZIPCRYPTO_MINIMAL_PASSWORD })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(!body.contains(ZIPCRYPTO_MINIMAL_PASSWORD));
    let tree: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(tree["errors"], 0, "{body}");
    assert!(
        !tree["installations"][0]["topology"]
            .as_array()
            .unwrap()
            .is_empty(),
        "{body}"
    );

    for uri in ["/api/log", "/api/project"] {
        let (_, text) = call(&app, "GET", uri, None).await;
        assert!(!text.contains(ZIPCRYPTO_MINIMAL_PASSWORD), "{uri}");
    }
}

#[tokio::test]
async fn without_a_password_the_refusal_is_machine_readable() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let (status, body) = call(
        &app(),
        "POST",
        "/api/project/import",
        Some(json!({ "path": source })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["kind"], "projectPasswordRequired");
    assert!(body["error"].as_str().unwrap().contains("password"));
}

#[tokio::test]
async fn a_wrong_password_is_refused_and_never_echoed_anywhere() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let app = app();
    let (status, body) = call(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({ "path": source, "password": "canary-wrong-password" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert!(!body.contains("canary-wrong-password"));
    let parsed: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(parsed["kind"], "projectPasswordWrong");

    let (_, log) = call(&app, "GET", "/api/log", None).await;
    assert!(!log.contains("canary-wrong-password"), "{log}");
    let (_, progress) = call(&app, "GET", "/api/project/load-progress", None).await;
    assert!(!progress.contains("canary-wrong-password"), "{progress}");
    let (status, _) = call(&app, "GET", "/api/project", None).await;
    assert_ne!(status, StatusCode::OK, "a refused import opens no project");
}

#[tokio::test]
async fn a_failed_password_attempt_keeps_the_project_that_was_open() {
    let dir = tempfile::tempdir().unwrap();
    let plain = dir.path().join("plain.knxproj");
    std::fs::write(&plain, knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let app = app();
    let (status, _) = call(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({ "path": plain })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, before) = call(&app, "GET", "/api/project", None).await;

    let (status, _) = call(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({ "path": source, "password": "not-the-password" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    let (status, after) = call(&app, "GET", "/api/project", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(before, after);
}

#[tokio::test]
async fn an_empty_password_counts_as_none() {
    let dir = tempfile::tempdir().unwrap();
    let source = write_zipcrypto_minimal_knxproj(dir.path());
    let (status, body) = call(
        &app(),
        "POST",
        "/api/project/import",
        Some(json!({ "path": source, "password": "" })),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    let body: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(body["kind"], "projectPasswordRequired");
}

#[tokio::test]
async fn an_unprotected_import_failure_keeps_the_plain_500_body() {
    let (status, body) = call(
        &app(),
        "POST",
        "/api/project/import",
        Some(json!({ "path": "/does/not/exist.knxproj", "password": "x" })),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    let body: Value = serde_json::from_str(&body).unwrap();
    assert!(body.get("kind").is_none(), "{body}");
}
