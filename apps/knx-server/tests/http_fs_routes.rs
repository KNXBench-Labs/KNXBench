use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

fn state_with_data_dir() -> (knx_server::AppState, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("a.knxproj"),
        b"not a real project, just a listing fixture",
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    (knx_server::AppState::new(dir.path().to_path_buf()), dir)
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

const BOUNDARY: &str = "knx-test-boundary";

/// Builds a raw `multipart/form-data` body by hand — axum's `Multipart`
/// extractor (a thin wrapper over `multer`) expects a real encoded body,
/// there's no in-process shortcut around constructing one.
fn multipart_file_body(field_name: &str, filename: &str, content: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{filename}\"\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/octet-stream\r\n\r\n");
    body.extend_from_slice(content);
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

/// A multipart body with a plain form field (no `filename`) — what a
/// client sends when there's no file to upload at all.
fn multipart_no_file_body(field_name: &str, value: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"{field_name}\"\r\n\r\n").as_bytes(),
    );
    body.extend_from_slice(value.as_bytes());
    body.extend_from_slice(b"\r\n");
    body.extend_from_slice(format!("--{BOUNDARY}--\r\n").as_bytes());
    body
}

fn multipart_request(uri: &str, body: Vec<u8>) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

#[tokio::test]
async fn listing_the_data_dir_root_shows_its_entries() {
    let (state, _dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/fs/list")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let entries = body_json(response).await;
    let names: Vec<&str> = entries
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["name"].as_str().unwrap())
        .collect();
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
async fn uploading_a_file_round_trips_into_the_uploads_dir() {
    let (state, dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let content = b"a fake .knxproj for upload testing";
    let body = multipart_file_body("file", "project.knxproj", content);

    let response = app
        .oneshot(multipart_request("/api/fs/upload", body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let parsed = body_json(response).await;
    assert_eq!(parsed["path"].as_str().unwrap(), "uploads/project.knxproj");

    let written = std::fs::read(dir.path().join("uploads").join("project.knxproj")).unwrap();
    assert_eq!(written, content);
}

#[tokio::test]
async fn uploading_with_a_path_traversal_filename_lands_safely_in_uploads() {
    let (state, dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let body = multipart_file_body("file", "../../etc/passwd", b"not actually /etc/passwd");

    let response = app
        .oneshot(multipart_request("/api/fs/upload", body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let parsed = body_json(response).await;
    assert_eq!(parsed["path"].as_str().unwrap(), "uploads/passwd");

    // Lands under data_dir/uploads/passwd, not anywhere near a real /etc/passwd,
    // and nothing was written directly into data_dir's root either.
    assert!(dir.path().join("uploads").join("passwd").is_file());
    assert!(!dir.path().join("passwd").exists());
    assert!(!dir.path().join("etc").exists());
}

#[tokio::test]
async fn uploading_with_no_file_field_is_a_400() {
    let (state, _dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let body = multipart_no_file_body("not-a-file", "just some text");

    let response = app
        .oneshot(multipart_request("/api/fs/upload", body))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn downloading_with_no_project_open_is_a_400() {
    let (state, _dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/project/download")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
