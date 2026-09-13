//! Proves a device can be installed from a manufacturer package with no ETS project anywhere.

use std::path::PathBuf;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

const BOUNDARY: &str = "knx-catalog-to-device-boundary";

/// The corpus package this test installs. Chosen because it carries a
/// program with communication objects; the exact count is asserted below,
/// measured from this file and nothing else.
const PACKAGE: &str = "MDT_KP_AMI_AMS_03_Switch_Actuator_V31a.knxprod";

/// The catalog item picked out of `PACKAGE` — the lexicographically first
/// id, so the choice survives any reordering of the `catalog_items` query,
/// and named here so a change in the package is a test failure rather than
/// a silently different device.
const CATALOG_ITEM: &str = "M-0083_H-352-3_HP-0317-31-7DC6_CI-AMI.2D1216.2E03-1";

/// Communication objects the device above is seeded with. Measured by
/// running this test against the corpus, not carried over from anywhere.
const COM_OBJECTS: usize = 104;

/// Same lookup order as `crates/knx-productdb/tests/standalone_packages.rs`:
/// an explicit `KNXBENCH_PRODUCT_CORPUS`, else the gitignored local corpus.
fn corpus_root() -> PathBuf {
    std::env::var_os("KNXBENCH_PRODUCT_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../OriginalData/ProductDatabases")
        })
}

fn multipart(filename: &str, bytes: &[u8]) -> Request<Body> {
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{BOUNDARY}\r\n").as_bytes());
    body.extend_from_slice(
        format!("Content-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\n\r\n")
            .as_bytes(),
    );
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{BOUNDARY}--\r\n").as_bytes());
    Request::builder()
        .method("POST")
        .uri("/api/catalog/install")
        .header(
            "content-type",
            format!("multipart/form-data; boundary={BOUNDARY}"),
        )
        .body(Body::from(body))
        .unwrap()
}

fn post(uri: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap()
}

#[tokio::test]
async fn a_new_project_takes_a_device_from_a_manufacturer_package_with_no_ets_import() {
    let package = corpus_root().join(PACKAGE);
    if !package.exists() {
        eprintln!(
            "skip: {PACKAGE} not present (OriginalData/ is gitignored, local-only); \
             set KNXBENCH_PRODUCT_CORPUS to a directory holding it to run this test"
        );
        return;
    }
    let bytes = std::fs::read(&package).unwrap();

    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        product_db: Some(std::sync::Mutex::new(
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap(),
        )),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);

    // 1. A project exists because the user asked for one, not because a
    //    `.knxproj` was imported. Nothing below ever names one.
    let response = app
        .clone()
        .oneshot(post("/api/project/new", json!({})))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    assert_eq!(tree["installations"].as_array().unwrap().len(), 1);
    assert!(tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .is_empty());

    // 2. The manufacturer's own product package, uploaded as a file.
    let response = app
        .clone()
        .oneshot(multipart(PACKAGE, &bytes))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let report = body_json(response).await;
    assert_eq!(report["skipped"], false);

    // 3. The catalog can be browsed without a project ever having been
    //    imported.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/catalog/items")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let items = body_json(response).await;
    let mut ids: Vec<String> = items
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["id"].as_str().unwrap().to_string())
        .collect();
    ids.sort();
    assert_eq!(ids.first().map(String::as_str), Some(CATALOG_ITEM));

    // 4. The device itself.
    let response = app
        .clone()
        .oneshot(post(
            "/api/devices",
            json!({ "catalogItemId": CATALOG_ITEM, "name": "Device under test" }),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let created = body_json(response).await;
    let unassigned = created["tree"]["installations"][0]["unassigned"]
        .as_array()
        .unwrap();
    assert_eq!(unassigned.len(), 1);
    assert_eq!(unassigned[0]["name"], "Device under test");
    assert_eq!(unassigned[0]["com_object_count"], COM_OBJECTS);

    // 5. And it is genuinely in the project, not just in one response body.
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/device/{}", unassigned[0]["id"]))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let detail = body_json(response).await;
    assert_eq!(detail["com_objects"].as_array().unwrap().len(), COM_OBJECTS);
}
