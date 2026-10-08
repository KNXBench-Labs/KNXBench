//! HTTP tests for the add-device wizard: preview, placement and stale-preview refusal.

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

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(body) => {
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(request.body(body).unwrap())
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

async fn post(app: &axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    call(app, "POST", uri, Some(body)).await
}

async fn project(app: &axum::Router) -> Value {
    let (status, tree) = call(app, "GET", "/api/project", None).await;
    assert_eq!(status, StatusCode::OK);
    tree
}

/// A seeded project with area 1 / line 1.1 and a house with one room.
/// Returns `(line id, room id)`.
async fn app_with_room() -> (tempfile::TempDir, axum::Router, u64, u64) {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        product_db: Some(Mutex::new(products(&dir))),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);
    let (status, tree) = post(
        &app,
        "/api/project/new",
        json!({ "seed": {
            "areas": [{ "name": "A", "address": 1, "lines": [{ "name": "L", "address": 1, "mediumRef": "MT-0" }] }],
            "buildings": [{ "name": "House", "kind": "Building", "children": [{ "name": "Kitchen", "kind": "Room" }] }]
        } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let line = tree["installations"][0]["topology"][0]["lines"][0]["id"]
        .as_u64()
        .unwrap();
    let room = tree["installations"][0]["buildings"][0]["children"][0]["id"]
        .as_u64()
        .unwrap();
    (dir, app, line, room)
}

fn room_devices(tree: &Value) -> Vec<String> {
    tree["installations"][0]["buildings"][0]["children"][0]["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["name"].as_str().unwrap().to_string())
        .collect()
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
async fn preview_changes_nothing_and_create_with_its_values_places_devices_in_one_undo_step() {
    let (_dir, app, line, room) = app_with_room().await;
    let before = project(&app).await;
    let request = json!({ "catalogItemId": ITEM, "name": "PSU", "quantity": 2, "lineId": line,
        "buildingPartId": room, "allocateAddresses": true });

    let (status, preview) = post(&app, "/api/devices/preview", request.clone()).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(
        preview["items"],
        json!([
            { "index": 1, "name": "PSU 1", "address": "1.1.1" },
            { "index": 2, "name": "PSU 2", "address": "1.1.2" }
        ])
    );
    assert_eq!(preview["installationId"], 0);
    assert!(
        preview["diagnostics"]
            .to_string()
            .contains("programlessProduct"),
        "{preview}"
    );
    // Nothing moved: same tree, no undo entry, no revision bump.
    assert_eq!(project(&app).await, before);
    // Previewing twice gives the same answer: no IDs were reserved.
    assert_eq!(
        post(&app, "/api/devices/preview", request.clone()).await.1,
        preview
    );

    let mut create = request.clone();
    create["expected"] = json!([
        { "name": "PSU 1", "address": "1.1.1" },
        { "name": "PSU 2", "address": "1.1.2" }
    ]);
    let (status, created) = post(&app, "/api/devices", create).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let tree = &created["tree"];
    assert_eq!(room_devices(tree), ["PSU 1", "PSU 2"]);
    assert_eq!(
        line_devices(tree),
        [
            ("PSU 1".into(), json!("1.1.1")),
            ("PSU 2".into(), json!("1.1.2"))
        ]
    );

    let (status, undone) = post(&app, "/api/undo", json!({})).await;
    assert_eq!(status, StatusCode::OK, "{undone}");
    assert!(room_devices(&undone).is_empty());
    assert!(line_devices(&undone).is_empty());
    assert_eq!(
        undone["can_undo"], false,
        "creation and placement were one step"
    );
}

#[tokio::test]
async fn a_stale_preview_is_refused_with_409_and_creates_nothing() {
    let (_dir, app, line, _room) = app_with_room().await;
    let request =
        json!({ "catalogItemId": ITEM, "name": "PSU", "lineId": line, "allocateAddresses": true });
    let (_, preview) = post(&app, "/api/devices/preview", request.clone()).await;
    assert_eq!(preview["items"][0]["address"], "1.1.1");

    // Someone else takes 1.1.1 in the meantime.
    let (status, _) = post(&app, "/api/devices", request.clone()).await;
    assert_eq!(status, StatusCode::OK);
    let before = project(&app).await;

    let mut create = request.clone();
    create["expected"] = json!([{ "name": "PSU", "address": "1.1.1" }]);
    let (status, body) = post(&app, "/api/devices", create).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "catalogPreviewStale");
    assert!(
        body["error"].as_str().unwrap().contains("PSU @ 1.1.2"),
        "{body}"
    );
    assert_eq!(project(&app).await, before);
}

#[tokio::test]
async fn a_line_less_device_can_go_into_a_room_of_the_named_installation() {
    let (_dir, app, _line, room) = app_with_room().await;
    let request = json!({ "catalogItemId": ITEM, "name": "Sensor", "installationId": 0, "buildingPartId": room });
    let (status, preview) = post(&app, "/api/devices/preview", request.clone()).await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(
        preview["items"],
        json!([{ "index": 1, "name": "Sensor", "address": null }])
    );
    let mut create = request;
    create["expected"] = json!([{ "name": "Sensor" }]);
    let (status, created) = post(&app, "/api/devices", create).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(room_devices(&created["tree"]), ["Sensor"]);
    assert_eq!(
        created["tree"]["installations"][0]["unassigned"][0]["name"],
        "Sensor"
    );
}

#[tokio::test]
async fn unknown_placements_are_refused_by_preview_and_create_alike() {
    let (_dir, app, line, _room) = app_with_room().await;
    let before = project(&app).await;
    for (extra, reason) in [
        (
            json!({ "buildingPartId": 999 }),
            "building part 999 not found",
        ),
        (json!({ "installationId": 7 }), "installation 7 not found"),
        (json!({ "lineId": 999 }), "target line 999 not found"),
        (
            json!({ "lineId": line, "installationId": 0, "buildingPartId": 1_000 }),
            "building part 1000 not found",
        ),
    ] {
        let mut request = json!({ "catalogItemId": ITEM, "name": "X" });
        for (key, value) in extra.as_object().unwrap() {
            request[key] = value.clone();
        }
        for route in ["/api/devices/preview", "/api/devices"] {
            let (status, body) = post(&app, route, request.clone()).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{route} {request}: {body}");
            assert!(
                body["error"].as_str().unwrap().contains(reason),
                "{route}: {body}"
            );
        }
    }
    assert_eq!(project(&app).await, before);
}

#[tokio::test]
async fn placement_and_expectation_are_part_of_the_replay_fingerprint() {
    let (_dir, app, line, room) = app_with_room().await;
    let first = json!({ "catalogItemId": ITEM, "name": "PSU", "lineId": line, "requestId": "r-1",
        "buildingPartId": room });
    let (status, created) = post(&app, "/api/devices", first.clone()).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let (status, replayed) = post(&app, "/api/devices", first.clone()).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(replayed["replayed"], true);

    for (key, value) in [
        ("buildingPartId", Value::Null),
        ("expected", json!([{ "name": "PSU" }])),
    ] {
        let mut other = first.clone();
        other[key] = value;
        let (status, body) = post(&app, "/api/devices", other).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{key}: {body}");
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("different request"),
            "{body}"
        );
    }
}

#[tokio::test]
async fn a_malformed_expected_address_is_refused_before_anything_happens() {
    let (_dir, app, line, _room) = app_with_room().await;
    let before = project(&app).await;
    let (status, body) = post(
        &app,
        "/api/devices",
        json!({ "catalogItemId": ITEM, "name": "PSU", "lineId": line,
            "expected": [{ "name": "PSU", "address": "1.1" }] }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("expected address"),
        "{body}"
    );
    assert_eq!(project(&app).await, before);
}
