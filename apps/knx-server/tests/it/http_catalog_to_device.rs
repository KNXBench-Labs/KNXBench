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
#[ignore = "requires the gitignored OriginalData/ product corpus (or KNXBENCH_PRODUCT_CORPUS); run with --ignored"]
async fn a_new_project_takes_a_device_from_a_manufacturer_package_with_no_ets_import() {
    // Looked up by name anywhere under the root: the local corpus files
    // packages by manufacturer (`MDT/…`), and a flat `join(PACKAGE)` missed
    // it on every run since this test was written, where the early return
    // then passed it (docs/KNOWN_LIMITATIONS.md §131).
    let root = corpus_root();
    assert!(
        root.exists(),
        "product corpus {} not present (OriginalData/ is gitignored, local-only); \
         set KNXBENCH_PRODUCT_CORPUS to a directory holding {PACKAGE}",
        root.display()
    );
    let package = knx_testsupport::find_corpus_file(&root, PACKAGE)
        .unwrap_or_else(|| panic!("{PACKAGE} not found under {}", root.display()));
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

#[tokio::test]
#[ignore = "requires the gitignored OriginalData/ product corpus; run with --ignored"]
async fn catalog_quantity_is_one_atomic_creation_with_one_undo_and_no_leaked_ids() {
    let root = corpus_root();
    assert!(
        root.exists(),
        "product corpus must be present for this regression"
    );
    let package = knx_testsupport::find_corpus_file(&root, PACKAGE).unwrap();
    let bytes = std::fs::read(package).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        product_db: Some(std::sync::Mutex::new(
            knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap(),
        )),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);
    assert_eq!(
        app.clone()
            .oneshot(post("/api/project/new", json!({})))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );
    assert_eq!(
        app.clone()
            .oneshot(multipart(PACKAGE, &bytes))
            .await
            .unwrap()
            .status(),
        StatusCode::OK
    );

    // Invalid quantity and line are refused before the first device/ID is committed.
    let too_many = app
        .clone()
        .oneshot(post(
            "/api/devices",
            json!({
                "catalogItemId": CATALOG_ITEM, "name": "Device", "quantity": 33
            }),
        ))
        .await
        .unwrap();
    assert_eq!(too_many.status(), StatusCode::BAD_REQUEST);
    let wrong_line = app
        .clone()
        .oneshot(post(
            "/api/devices",
            json!({
                "catalogItemId": CATALOG_ITEM, "name": "Device", "quantity": 3, "lineId": 999
            }),
        ))
        .await
        .unwrap();
    assert_eq!(wrong_line.status(), StatusCode::BAD_REQUEST);

    let created = app
        .clone()
        .oneshot(post(
            "/api/devices",
            json!({
                "catalogItemId": CATALOG_ITEM, "name": "Device", "quantity": 3
            }),
        ))
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::OK);
    let created = body_json(created).await;
    let items = created["items"]
        .as_array()
        .expect("per-device creation results");
    assert_eq!(items.len(), 3);
    let devices = created["tree"]["installations"][0]["unassigned"]
        .as_array()
        .unwrap();
    assert_eq!(devices.len(), 3);
    for (index, device) in devices.iter().enumerate() {
        assert_eq!(device["id"], (index + 1) as u32);
        assert_eq!(device["name"], format!("Device {}", index + 1));
        assert!(
            device["address"].is_null(),
            "do not invent physical addresses"
        );
        assert_eq!(device["com_object_count"], COM_OBJECTS);
        assert_eq!(items[index]["index"], (index + 1) as u32);
        assert_eq!(items[index]["deviceId"], device["id"]);
        assert!(items[index]["diagnostics"].is_array());
    }
    let undone = app
        .clone()
        .oneshot(post("/api/undo", json!({})))
        .await
        .unwrap();
    assert_eq!(undone.status(), StatusCode::OK);
    assert!(body_json(undone).await["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .is_empty());
    let redone = app
        .clone()
        .oneshot(post("/api/redo", json!({})))
        .await
        .unwrap();
    assert_eq!(redone.status(), StatusCode::OK);
    assert_eq!(
        body_json(redone).await["installations"][0]["unassigned"]
            .as_array()
            .unwrap()
            .len(),
        3
    );

    let area = app
        .clone()
        .oneshot(post("/api/areas", json!({ "name": "Area", "address": 1 })))
        .await
        .unwrap();
    assert_eq!(area.status(), StatusCode::OK);
    let area_id = body_json(area).await["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();
    let line = app
        .clone()
        .oneshot(post(
            "/api/lines",
            json!({
                "areaId": area_id, "name": "Line", "address": 1, "mediumRef": "MT-0"
            }),
        ))
        .await
        .unwrap();
    assert_eq!(line.status(), StatusCode::OK);
    let line_id = body_json(line).await["installations"][0]["topology"][0]["lines"][0]["id"]
        .as_u64()
        .unwrap();
    let placed = app
        .clone()
        .oneshot(post("/api/devices", json!({
            "catalogItemId": CATALOG_ITEM, "name": "Line device", "quantity": 2, "lineId": line_id
        })))
        .await
        .unwrap();
    assert_eq!(placed.status(), StatusCode::OK);
    let tree = body_json(placed).await["tree"].clone();
    let line_devices = tree["installations"][0]["topology"][0]["lines"][0]["devices"]
        .as_array()
        .unwrap();
    assert_eq!(line_devices.len(), 2);
    assert!(line_devices
        .iter()
        .all(|device| device["address"].is_null()));
    assert_eq!(
        tree["installations"][0]["unassigned"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let undo_placed = app.oneshot(post("/api/undo", json!({}))).await.unwrap();
    assert_eq!(undo_placed.status(), StatusCode::OK);
    let tree = body_json(undo_placed).await;
    assert!(
        tree["installations"][0]["topology"][0]["lines"][0]["devices"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        tree["installations"][0]["unassigned"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
}
