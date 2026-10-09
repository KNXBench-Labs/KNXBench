//! A device from a legacy ETS3 product database is placed, parameterised and linked like any other.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn products(dir: &tempfile::TempDir) -> (knx_productdb::Connection, String, String) {
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    let bytes = std::fs::read(
        knx_testsupport::workspace_root()
            .join("crates/knx-productdb/fixtures/legacy/marvin-program.vd4"),
    )
    .unwrap();
    let password = knx_app::legacy::LegacyPassword::new("marvin-synthetic");
    let report =
        knx_app::legacy::import_legacy_file(&conn, "marvin.vd4", &bytes, Some(&password)).unwrap();
    let program = report.programs[0].clone();
    let item = format!("M-1092_CI-{}-500", report.namespace);
    (conn, program, item)
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

fn field<'a>(panel: &'a Value, ets_id: &str) -> Option<&'a Value> {
    panel["sections"]
        .as_array()?
        .iter()
        .flat_map(|section| section["fields"].as_array().into_iter().flatten())
        .find(|f| f["etsId"] == ets_id)
}

#[tokio::test]
async fn a_legacy_device_is_placed_parameterised_and_linked() {
    let dir = tempfile::tempdir().unwrap();
    let (conn, program, item) = products(&dir);
    let state = knx_server::AppState {
        product_db: Some(Mutex::new(conn)),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);
    let (status, tree) = call(&app, "POST", "/api/project/new", Some(json!({ "seed": {
        "areas": [{ "name": "A", "address": 1, "lines": [{ "name": "L", "address": 1, "mediumRef": "MT-0" }] }]
    } }))).await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let line = tree["installations"][0]["topology"][0]["lines"][0]["id"]
        .as_u64()
        .unwrap();

    // The catalog lists it under its manufacturer.
    let (status, items) = call(&app, "GET", "/api/catalog/items?manufacturer=M-1092", None).await;
    assert_eq!(status, StatusCode::OK, "{items}");
    assert!(items.to_string().contains(&item), "{items}");

    // Placed from the legacy catalog, with its three objects.
    let (status, created) = call(
        &app,
        "POST",
        "/api/devices",
        Some(json!({ "catalogItemId": item, "name": "Sensor", "lineId": line })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{created}");
    let device = &created["tree"]["installations"][0]["topology"][0]["lines"][0]["devices"][0];
    assert_eq!(device["com_object_count"], 3, "{device}");
    let id = device["id"].as_u64().unwrap();

    // Parameters are editable, and the visible branch follows the value.
    let mode = format!("{program}_P-1001_R-1001");
    let (status, panel) = call(&app, "GET", &format!("/api/device/{id}/parameters"), None).await;
    assert_eq!(status, StatusCode::OK, "{panel}");
    let shown = field(&panel, &mode).unwrap_or_else(|| panic!("{panel}"));
    assert_eq!(shown["editable"], true, "{shown}");
    assert_eq!(shown["value"], "1");
    assert!(field(&panel, &format!("{program}_P-1002_R-1002")).is_some());
    assert!(field(&panel, &format!("{program}_P-1002_R-1003")).is_none());
    // A legacy atomic-type-0 heading is shown, but only as a label (KL §128).
    let heading = format!("{program}_P-1006_R-1006");
    let shown = field(&panel, &heading).unwrap_or_else(|| panic!("{panel}"));
    assert_eq!(shown["kind"], "None", "{shown}");
    assert_eq!(shown["editable"], false, "{shown}");
    assert!(shown["writeEtsId"].is_null(), "{shown}");
    let (status, refused) = call(
        &app,
        "POST",
        &format!("/api/device/{id}/parameters"),
        Some(json!({ "etsId": heading, "raw": "" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    let (status, panel) = call(
        &app,
        "POST",
        &format!("/api/device/{id}/parameters"),
        Some(json!({ "etsId": mode, "raw": "0" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{panel}");
    assert_eq!(field(&panel, &mode).unwrap()["value"], "0");
    assert!(
        field(&panel, &format!("{program}_P-1002_R-1003")).is_some(),
        "{panel}"
    );
    assert!(
        field(&panel, &format!("{program}_P-1002_R-1002")).is_none(),
        "{panel}"
    );

    // An object links to a group address.
    let (status, detail) = call(&app, "GET", &format!("/api/device/{id}"), None).await;
    assert_eq!(status, StatusCode::OK, "{detail}");
    let objects = detail["com_objects"]
        .as_array()
        .unwrap_or_else(|| panic!("{detail}"));
    // The objects follow the parameter too: mode 0 activates 10002's object.
    let active: Vec<_> = objects
        .iter()
        .map(|o| (o["function_text"].clone(), o["activation"].clone()))
        .collect();
    assert_eq!(
        active,
        [
            (json!("Send"), json!("Active")),
            (json!("Receive"), json!("Inactive")),
            (json!("Value"), json!("Active")),
        ]
    );
    let object = objects[0]["id"].as_u64().unwrap();
    let (status, tree) = call(
        &app,
        "POST",
        "/api/group-addresses",
        Some(json!({ "name": "Alarm", "address": "1/1/1" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let ga = tree["installations"][0]["group_addresses"][0]["id"]
        .as_u64()
        .unwrap_or_else(|| panic!("{tree}"));
    let (status, tree) = call(
        &app,
        "POST",
        "/api/group-links",
        Some(json!({ "comObjectId": object, "gaId": ga, "direction": "Send" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let (_, detail) = call(&app, "GET", &format!("/api/device/{id}"), None).await;
    assert!(detail.to_string().contains("1/1/1"), "{detail}");
}
