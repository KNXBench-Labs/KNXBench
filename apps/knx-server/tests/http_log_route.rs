//! `GET /api/log` (T11 session log) — reflects entries pushed by an
//! import, a failed edit and a successful edit, in append order. Uses the
//! reference project the same way `http_project_routes.rs` does, since the
//! entries this route reports only exist once something has actually
//! happened to `AppState`.

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

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(builder.body(body).unwrap())
        .await
        .unwrap()
}

/// Walks `tree` (a `ProjectTree` JSON body) looking for the first
/// non-empty `"devices"` array, wherever it is nested (unassigned, a
/// line, or a building part) — the reference project's own device
/// placement is not this test's concern.
fn first_device_id(tree: &Value) -> Option<u64> {
    if let Some(devices) = tree.get("devices").and_then(Value::as_array) {
        if let Some(first) = devices.first() {
            if let Some(id) = first.get("id").and_then(Value::as_u64) {
                return Some(id);
            }
        }
    }
    match tree {
        Value::Object(map) => map.values().find_map(first_device_id),
        Value::Array(items) => items.iter().find_map(first_device_id),
        _ => None,
    }
}

#[tokio::test]
async fn get_log_reflects_import_a_failed_edit_and_a_successful_edit_in_order() {
    if !reference_ets4_path().exists() {
        eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
        return;
    }
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);

    // No project, no history yet — the route still answers with `[]`, not
    // a 404.
    let empty = call(&app, "GET", "/api/log", None).await;
    assert_eq!(empty.status(), StatusCode::OK);
    assert_eq!(body_json(empty).await, json!([]));

    // 1. Import.
    let import = call(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({ "path": reference_ets4_path().to_string_lossy() })),
    )
    .await;
    assert_eq!(import.status(), StatusCode::OK);
    let tree = body_json(import).await;
    let device_id = first_device_id(&tree).expect("reference project has at least one device");

    // 2. A failed edit — no device with this id exists.
    let failed = call(
        &app,
        "POST",
        "/api/individual-address",
        Some(json!({ "deviceId": 999_999_999u32, "address": "1.1.2" })),
    )
    .await;
    assert_eq!(failed.status(), StatusCode::BAD_REQUEST);

    // 3. A successful edit — clearing an address always succeeds,
    // regardless of what the device already had.
    let succeeded = call(
        &app,
        "POST",
        "/api/individual-address",
        Some(json!({ "deviceId": device_id, "address": Value::Null })),
    )
    .await;
    assert_eq!(succeeded.status(), StatusCode::OK);

    let log = call(&app, "GET", "/api/log", None).await;
    assert_eq!(log.status(), StatusCode::OK);
    let entries = body_json(log).await;
    let entries = entries.as_array().unwrap();

    // The import notice is the last entry `from_import_report` +
    // `open_project`'s own push write, i.e. the first thing this test's
    // own actions add after the reset `open_project` performs.
    let import_entry = entries
        .iter()
        .find(|e| e["source"] == "import" && e["severity"] == "info")
        .expect("open_project's own success notice");
    let failed_entry = entries
        .iter()
        .position(|e| e["severity"] == "error")
        .map(|i| &entries[i])
        .expect("the failed edit's error entry");
    let succeeded_entry = entries.last().expect("at least one entry");

    let import_index = entries.iter().position(|e| e == import_entry).unwrap();
    let failed_index = entries.iter().position(|e| e == failed_entry).unwrap();
    let succeeded_index = entries.len() - 1;

    assert!(
        import_index < failed_index && failed_index < succeeded_index,
        "expected import, then the failed edit, then the successful edit, in that order: {entries:#?}"
    );
    assert_eq!(succeeded_entry["severity"], "info");
}
