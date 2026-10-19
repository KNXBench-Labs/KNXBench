//! Uploading a legacy ETS3 product database: password dialog flow and the one remembered password.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::Value;
use tower::ServiceExt;

const BOUNDARY: &str = "knx-legacy-install-boundary";
const PASSWORD: &str = "marvin-synthetic";

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        knx_testsupport::workspace_root()
            .join("crates/knx-productdb/fixtures/legacy")
            .join(name),
    )
    .unwrap()
}

/// A multipart body with the file and optional text fields.
fn upload(uri: &str, filename: &str, bytes: &[u8], fields: &[(&str, &str)]) -> Request<Body> {
    let mut body = Vec::new();
    for (name, value) in fields {
        body.extend_from_slice(
            format!(
                "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n"
            )
            .as_bytes(),
        );
    }
    body.extend_from_slice(
        format!(
            "--{BOUNDARY}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
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

async fn send(app: &axum::Router, request: Request<Body>) -> (StatusCode, Value, String) {
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
        text,
    )
}

fn get(uri: &str, method: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .body(Body::empty())
        .unwrap()
}

/// A server with a product database and a remembered-password file in a
/// temporary directory (never the developer's own).
fn app(remember: bool) -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        product_db: Some(Mutex::new(
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap(),
        )),
        legacy_password: remember.then(|| {
            knx_app::legacy::RememberedPassword::at(
                dir.path().join("config/knx/legacy-vd-password"),
            )
        }),
        ..Default::default()
    };
    (dir, knx_server::app(Arc::new(state), None))
}

fn remembered_file(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join("config/knx/legacy-vd-password")
}

#[tokio::test]
async fn the_package_installer_names_a_legacy_product_database() {
    let (_dir, app) = app(true);
    for name in ["marvin.vd4", "renamed.knxprod"] {
        let (status, body, text) = send(
            &app,
            upload(
                "/api/catalog/install",
                name,
                &fixture("marvin-program.vd4"),
                &[],
            ),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{name}: {text}");
        assert_eq!(body["kind"], "legacyProductDatabase", "{name}: {text}");
    }
    // A legacy project export is not a product database: still a plain refusal.
    let (status, body, text) = send(
        &app,
        upload(
            "/api/catalog/install",
            "project.pr5",
            &fixture("marvin-project.pr5"),
            &[],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{text}");
    assert!(body["kind"].is_null(), "{text}");
}

#[tokio::test]
async fn an_encrypted_file_asks_for_a_password_then_imports_with_it() {
    let (_dir, app) = app(true);
    let bytes = fixture("marvin-program.vd4");
    let (status, body, text) = send(
        &app,
        upload("/api/catalog/install-legacy", "marvin.vd4", &bytes, &[]),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{text}");
    assert_eq!(body["kind"], "legacyPasswordRequired", "{text}");

    let (status, body, text) = send(
        &app,
        upload(
            "/api/catalog/install-legacy",
            "marvin.vd4",
            &bytes,
            &[("password", "canary-wrong")],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{text}");
    assert_eq!(body["kind"], "legacyWrongPassword", "{text}");
    assert!(!text.contains("canary-wrong"), "{text}");

    let (status, body, text) = send(
        &app,
        upload(
            "/api/catalog/install-legacy",
            "marvin.vd4",
            &bytes,
            &[("password", PASSWORD)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert!(!text.contains(PASSWORD), "{text}");
    assert_eq!(body["skipped"], false);
    assert_eq!(body["password"], "given");
    assert_eq!(body["remembered"], false);
    assert!(body["namespace"].as_str().unwrap().starts_with("LX"));
    assert_eq!(body["programs"].as_array().unwrap().len(), 1);
    assert_eq!(body["parameters"], 12);
    assert!(body["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["kind"] == "secret-withheld"));

    // The catalog now lists it.
    let (status, items, _) = send(&app, get("/api/catalog/items?manufacturer=M-1092", "GET")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(items.as_array().unwrap().len(), 1, "{items}");
}

/// The installer-tree layout of the measured `.vd5` (four members there,
/// two here) is named by the package installer and imported by the legacy
/// route, which reports the member it did not read (ADR-0094, VD5).
#[tokio::test]
async fn an_installer_tree_vd5_is_named_then_imported_with_its_unread_member_reported() {
    let (_dir, app) = app(true);
    let bytes = fixture("marvin-installer.vd5");
    let (status, body, text) = send(
        &app,
        upload("/api/catalog/install", "renamed.knxprod", &bytes, &[]),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{text}");
    assert_eq!(body["kind"], "legacyProductDatabase", "{text}");

    let (status, body, text) = send(
        &app,
        upload(
            "/api/catalog/install-legacy",
            "SIEMENS-like.vd5",
            &bytes,
            &[("password", PASSWORD)],
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{text}");
    assert_eq!(body["skipped"], false);
    assert_eq!(body["programs"].as_array().unwrap().len(), 1);
    let unread: Vec<&str> = body["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["kind"] == "unread-member")
        .map(|d| d["detail"].as_str().unwrap())
        .collect();
    assert_eq!(unread.len(), 1, "{text}");
    assert!(unread[0].contains("MASK/mask4242.bin"), "{text}");
}

#[tokio::test]
async fn remembering_keeps_the_password_privately_and_later_uploads_need_none() {
    let (dir, app) = app(true);
    let (_, status, _) = send(&app, get("/api/legacy-password", "GET")).await;
    assert_eq!(status["remembered"], false, "{status}");
    assert_eq!(status["available"], true, "{status}");

    let bytes = fixture("marvin-program.vd4");
    let (code, body, text) = send(
        &app,
        upload(
            "/api/catalog/install-legacy",
            "marvin.vd4",
            &bytes,
            &[("password", PASSWORD), ("remember", "true")],
        ),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{text}");
    assert_eq!(body["remembered"], true, "{text}");
    use std::os::unix::fs::PermissionsExt;
    let mode = std::fs::metadata(remembered_file(&dir))
        .unwrap()
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    let (_, status, text) = send(&app, get("/api/legacy-password", "GET")).await;
    assert_eq!(status["remembered"], true, "{text}");
    assert!(!text.contains(PASSWORD), "{text}");

    // The same content again needs no password and is already there.
    let (code, body, text) = send(
        &app,
        upload("/api/catalog/install-legacy", "again.vd4", &bytes, &[]),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{text}");
    assert_eq!(body["password"], "remembered");
    assert_eq!(body["skipped"], true);

    let (code, status, text) = send(&app, get("/api/legacy-password", "DELETE")).await;
    assert_eq!(code, StatusCode::OK, "{text}");
    assert_eq!(status["remembered"], false);
    assert_eq!(status["forgot"], true);
    assert!(!remembered_file(&dir).exists());
}

#[tokio::test]
async fn a_remembered_password_that_does_not_fit_asks_again() {
    let (dir, app) = app(true);
    knx_app::legacy::RememberedPassword::at(remembered_file(&dir))
        .store(&knx_app::legacy::LegacyPassword::new("canary-misfit"))
        .unwrap();
    let (code, body, text) = send(
        &app,
        upload(
            "/api/catalog/install-legacy",
            "marvin.vd4",
            &fixture("marvin-program.vd4"),
            &[],
        ),
    )
    .await;
    assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY, "{text}");
    assert_eq!(body["kind"], "legacyRememberedPasswordDoesNotFit", "{text}");
    assert!(!text.contains("canary-misfit"), "{text}");
}

#[tokio::test]
async fn a_failed_import_remembers_nothing() {
    let (dir, app) = app(true);
    let (code, _, text) = send(
        &app,
        upload(
            "/api/catalog/install-legacy",
            "marvin.vd4",
            &fixture("marvin-program.vd4"),
            &[("password", "canary-wrong"), ("remember", "true")],
        ),
    )
    .await;
    assert_eq!(code, StatusCode::UNPROCESSABLE_ENTITY, "{text}");
    assert!(!remembered_file(&dir).exists());
}

#[tokio::test]
async fn without_a_place_to_remember_the_import_still_works_and_says_so() {
    let (_dir, app) = app(false);
    let (_, status, _) = send(&app, get("/api/legacy-password", "GET")).await;
    assert_eq!(status["available"], false, "{status}");
    let (code, body, text) = send(
        &app,
        upload(
            "/api/catalog/install-legacy",
            "marvin.vd4",
            &fixture("marvin-program.vd4"),
            &[("password", PASSWORD), ("remember", "true")],
        ),
    )
    .await;
    assert_eq!(code, StatusCode::OK, "{text}");
    assert_eq!(body["remembered"], false);
    assert!(body["rememberProblem"].as_str().is_some(), "{text}");
}

#[tokio::test]
async fn a_project_export_or_a_modern_package_is_refused_by_the_legacy_route() {
    let (_dir, app) = app(true);
    let (code, _, text) = send(
        &app,
        upload(
            "/api/catalog/install-legacy",
            "project.pr5",
            &fixture("marvin-project.pr5"),
            &[("password", PASSWORD)],
        ),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST, "{text}");
    let (code, _, text) = send(
        &app,
        upload(
            "/api/catalog/install-legacy",
            "x.knxprod",
            b"PK\x05\x06not really",
            &[],
        ),
    )
    .await;
    assert_eq!(code, StatusCode::BAD_REQUEST, "{text}");
}
