//! MODEL-04: opt-in address allocation and unique names for catalog batches.
//!
//! Both are off by default; the existing batch behaviour (indexed names, no
//! addresses) is pinned by `http_catalog_to_device.rs`. With
//! `allocateAddresses` every new device on the target line gets the lowest
//! free device octet (1–255) not used anywhere in the project, in the same
//! undo step. With `uniqueNames` generated names skip names already in use.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

const ITEM: &str = "M-0001_H-1_P-1_CI-1";

fn products(dir: &tempfile::TempDir) -> knx_productdb::Connection {
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    conn.execute_batch(
        "INSERT INTO manufacturer (id, name) VALUES ('M-0001', 'M');
         INSERT INTO hardware (id, manufacturer_id, name, has_application_program, source_sha256)
              VALUES ('M-0001_H-1', 'M-0001', 'Power supply', 0, 'x');
         INSERT INTO product (id, manufacturer_id, hardware_id, text, source_sha256)
              VALUES ('M-0001_H-1_P-1', 'M-0001', 'M-0001_H-1', 'Power supply', 'x');
         INSERT INTO catalog_item (id, manufacturer_id, section_id, name, product_ref_id, source_sha256)
              VALUES ('M-0001_H-1_P-1_CI-1', 'M-0001', 'S', 'Power supply', 'M-0001_H-1_P-1', 'x');",
    )
    .unwrap();
    conn
}

async fn send(app: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, value)
}

/// A new project with area 1 / line 1.1; returns the line id.
async fn app_with_line() -> (tempfile::TempDir, axum::Router, u64) {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        product_db: Some(Mutex::new(products(&dir))),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);
    assert_eq!(
        send(&app, "/api/project/new", json!({})).await.0,
        StatusCode::OK
    );
    let (status, tree) = send(&app, "/api/areas", json!({ "name": "A", "address": 1 })).await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let area = tree["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();
    let (status, tree) = send(
        &app,
        "/api/lines",
        json!({ "areaId": area, "name": "L", "address": 1, "mediumRef": "MT-0" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let line = tree["installations"][0]["topology"][0]["lines"][0]["id"]
        .as_u64()
        .unwrap();
    (dir, app, line)
}

fn line_devices(tree: &Value) -> Vec<(String, Value)> {
    tree["installations"][0]["topology"][0]["lines"][0]["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["name"].as_str().unwrap().to_string(),
                d["address"].clone(),
            )
        })
        .collect()
}

#[tokio::test]
async fn allocation_gives_the_lowest_free_octets_in_one_undo_step() {
    let (_dir, app, line) = app_with_line().await;
    // An existing device already holds 1.1.1 (and 1.1.3 is taken too).
    let (_, created) = send(
        &app,
        "/api/devices",
        json!({ "catalogItemId": ITEM, "name": "Old", "quantity": 2, "lineId": line }),
    )
    .await;
    let old: Vec<u64> = created["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["deviceId"].as_u64().unwrap())
        .collect();
    for (id, address) in [(old[0], "1.1.1"), (old[1], "1.1.3")] {
        let (status, body) = send(
            &app,
            "/api/individual-address",
            json!({ "deviceId": id, "address": address }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
    }

    let (status, body) = send(
        &app,
        "/api/devices",
        json!({ "catalogItemId": ITEM, "name": "New", "quantity": 3, "lineId": line,
                "allocateAddresses": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let addresses: Vec<&str> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["address"].as_str().unwrap())
        .collect();
    assert_eq!(addresses, ["1.1.2", "1.1.4", "1.1.5"]);
    let devices = line_devices(&body["tree"]);
    assert!(
        devices.contains(&("New 1".into(), json!("1.1.2"))),
        "{devices:?}"
    );
    assert!(
        devices.contains(&("New 3".into(), json!("1.1.5"))),
        "{devices:?}"
    );

    // One undo takes back the devices and their addresses together.
    let (status, undone) = send(&app, "/api/undo", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(line_devices(&undone).len(), 2);
    let (_, redone) = send(&app, "/api/redo", json!({})).await;
    assert_eq!(line_devices(&redone).len(), 5);
}

#[tokio::test]
async fn allocation_without_a_target_line_is_refused_before_anything_is_created() {
    let (_dir, app, _line) = app_with_line().await;
    let (status, body) = send(
        &app,
        "/api/devices",
        json!({ "catalogItemId": ITEM, "name": "X", "quantity": 2, "allocateAddresses": true }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.to_string().contains("line"), "{body}");
    let (_, tree) = send(&app, "/api/undo", json!({})).await;
    // Undo takes back the line creation, proving no device batch was pushed.
    assert!(tree["installations"][0]["topology"][0]["lines"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn unique_names_skip_names_already_in_the_project() {
    let (_dir, app, _line) = app_with_line().await;
    let (_, first) = send(
        &app,
        "/api/devices",
        json!({ "catalogItemId": ITEM, "name": "PSU", "quantity": 2 }),
    )
    .await;
    assert_eq!(first["items"][1]["name"], "PSU 2");
    let (status, body) = send(
        &app,
        "/api/devices",
        json!({ "catalogItemId": ITEM, "name": "PSU", "quantity": 2, "uniqueNames": true }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let names: Vec<&str> = body["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["name"].as_str().unwrap())
        .collect();
    assert_eq!(names, ["PSU 3", "PSU 4"]);

    // A single device keeps its exact name when free, else gets a suffix.
    let (_, single) = send(
        &app,
        "/api/devices",
        json!({ "catalogItemId": ITEM, "name": "Gate", "uniqueNames": true }),
    )
    .await;
    assert_eq!(single["items"][0]["name"], "Gate");
    let (_, again) = send(
        &app,
        "/api/devices",
        json!({ "catalogItemId": ITEM, "name": "Gate", "uniqueNames": true }),
    )
    .await;
    assert_eq!(again["items"][0]["name"], "Gate 2");
}

#[tokio::test]
async fn without_the_options_names_repeat_and_no_address_is_invented() {
    let (_dir, app, line) = app_with_line().await;
    for _ in 0..2 {
        let (_, body) = send(
            &app,
            "/api/devices",
            json!({ "catalogItemId": ITEM, "name": "Same", "quantity": 2, "lineId": line }),
        )
        .await;
        assert_eq!(body["items"][0]["name"], "Same 1");
        assert!(body["items"][0]["address"].is_null(), "{body}");
    }
}
