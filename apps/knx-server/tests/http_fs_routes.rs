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

/// Generates a real multipart stream without allocating a whole large request.
struct LargeUploadBody {
    prefix: Option<axum::body::Bytes>,
    remaining: usize,
    chunk: axum::body::Bytes,
    suffix: Option<axum::body::Bytes>,
}

impl http_body::Body for LargeUploadBody {
    type Data = axum::body::Bytes;
    type Error = std::convert::Infallible;

    fn poll_frame(
        mut self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> {
        let data = if let Some(prefix) = self.prefix.take() {
            Some(prefix)
        } else if self.remaining > 0 {
            let len = self.remaining.min(self.chunk.len());
            self.remaining -= len;
            Some(self.chunk.slice(..len))
        } else {
            self.suffix.take()
        };
        std::task::Poll::Ready(data.map(|bytes| Ok(http_body::Frame::data(bytes))))
    }
}

fn large_upload_request(size: usize) -> Request<Body> {
    let empty = multipart_file_body("file", "large.knxproj", b"");
    let split = empty.windows(4).position(|w| w == b"\r\n\r\n").unwrap() + 4;
    Request::builder()
        .method("POST")
        .uri("/api/fs/upload")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::new(LargeUploadBody {
            prefix: Some(empty[..split].to_vec().into()),
            remaining: size,
            chunk: vec![0x5a; 1024 * 1024].into(),
            suffix: Some(empty[split..].to_vec().into()),
        }))
        .unwrap()
}

async fn assert_large_upload_preserved(size: usize) {
    use std::io::Read;

    let (state, dir) = state_with_data_dir();
    let response = knx_server::app(Arc::new(state), None)
        .oneshot(large_upload_request(size))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["path"], "uploads/large.knxproj");
    let mut file = std::fs::File::open(dir.path().join("uploads/large.knxproj")).unwrap();
    assert_eq!(file.metadata().unwrap().len(), size as u64);
    let mut buffer = [0; 64 * 1024];
    loop {
        let count = file.read(&mut buffer).unwrap();
        if count == 0 {
            break;
        }
        assert!(buffer[..count].iter().all(|&byte| byte == 0x5a));
    }
}

#[tokio::test]
async fn uploading_a_115_mib_file_preserves_every_byte() {
    // GitHub #1: the HTTP upload must not reject a 115 MB project before import.
    // This is an upload fixture, not a claim that these bytes are a valid project.
    assert_large_upload_preserved(115 * 1024 * 1024).await;
}

#[tokio::test]
async fn uploading_exactly_256_mib_allows_multipart_overhead() {
    assert_large_upload_preserved(256 * 1024 * 1024).await;
}

#[tokio::test]
async fn oversized_upload_is_413_and_leaves_no_partial_file() {
    let (state, dir) = state_with_data_dir();
    let state = Arc::new(state);
    let response = knx_server::app(state.clone(), None)
        .oneshot(large_upload_request(256 * 1024 * 1024 + 1))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    assert!(body_json(response).await["error"]
        .as_str()
        .unwrap()
        .contains("256 MiB"));
    assert_eq!(
        std::fs::read_dir(dir.path().join("uploads"))
            .unwrap()
            .count(),
        0
    );
    assert!(state.project.lock().unwrap().is_none());
}

#[tokio::test]
async fn malformed_upload_leaves_no_partial_file() {
    let (state, dir) = state_with_data_dir();
    let mut body = multipart_file_body("file", "broken.knxproj", b"incomplete");
    body.truncate(body.len() - format!("\r\n--{BOUNDARY}--\r\n").len());
    let response = knx_server::app(Arc::new(state), None)
        .oneshot(multipart_request("/api/fs/upload", body))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        std::fs::read_dir(dir.path().join("uploads"))
            .unwrap()
            .count(),
        0
    );
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
async fn duplicate_uploads_preserve_the_first_file_and_report_conflict() {
    let (state, dir) = state_with_data_dir();
    let app = knx_server::app(Arc::new(state), None);
    for (content, status) in [
        (b"first".as_slice(), StatusCode::OK),
        (b"second".as_slice(), StatusCode::CONFLICT),
    ] {
        let response = app
            .clone()
            .oneshot(multipart_request(
                "/api/fs/upload",
                multipart_file_body("file", "same.knxproj", content),
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), status);
    }
    assert_eq!(
        std::fs::read(dir.path().join("uploads/same.knxproj")).unwrap(),
        b"first"
    );
    assert_eq!(
        std::fs::read_dir(dir.path().join("uploads"))
            .unwrap()
            .count(),
        1
    );
}

#[tokio::test]
async fn upload_preserves_a_preexisting_destination() {
    let (state, dir) = state_with_data_dir();
    std::fs::create_dir(dir.path().join("uploads")).unwrap();
    std::fs::write(dir.path().join("uploads/same.knxproj"), b"existing").unwrap();
    let app = knx_server::app(Arc::new(state), None);
    let response = app
        .oneshot(multipart_request(
            "/api/fs/upload",
            multipart_file_body("file", "same.knxproj", b"replacement"),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    assert!(body_json(response).await["error"]
        .as_str()
        .unwrap()
        .contains("already exists"));
    assert_eq!(
        std::fs::read(dir.path().join("uploads/same.knxproj")).unwrap(),
        b"existing"
    );
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

#[tokio::test]
async fn downloading_an_unsaved_project_returns_a_readable_current_store() {
    let (state, dir) = state_with_data_dir();
    let state = Arc::new(state);
    let app = knx_server::app(state.clone(), None);
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/new")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"Unsaved download","installationName":"Current installation"}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(state.store_path.lock().unwrap().is_none());
    state.project.lock().unwrap().as_mut().unwrap().info.name = "Latest unsaved edit".into();
    let opaque = knx_store::StoredOpaqueEntry {
        source_path: "P-Test/0.xml".into(),
        xpath: "/Unknown".into(),
        kind: "element".into(),
        name: "Unknown".into(),
        bytes: b"<Unknown keep=\"yes\"/>".to_vec(),
        sha256: "fixture opaque hash".into(),
    };
    let manufacturer_ref = knx_store::ManufacturerRef {
        source_path: "M-Test/Hardware.xml".into(),
        sha256: "fixture manufacturer hash".into(),
        len: 42,
        kind: "hardware".into(),
    };
    state.opaque.lock().unwrap().push(opaque.clone());
    state
        .manufacturer_refs
        .lock()
        .unwrap()
        .push(manufacturer_ref.clone());

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/project/download")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "application/octet-stream"
    );
    assert_eq!(
        response.headers()["content-disposition"],
        "attachment; filename=\"project.knxdb\""
    );
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let path = dir.path().join("download.knxdb");
    std::fs::write(&path, bytes).unwrap();
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let project = knx_store::load_project(&conn).unwrap();
    assert_eq!(project.info.name, "Latest unsaved edit");
    assert_eq!(project.installations.len(), 1);
    assert_eq!(project.installations[0].name, "Current installation");
    assert_eq!(knx_store::load_opaque(&conn).unwrap(), vec![opaque]);
    assert_eq!(
        knx_store::load_manufacturer_refs(&conn).unwrap(),
        vec![manufacturer_ref]
    );
}
