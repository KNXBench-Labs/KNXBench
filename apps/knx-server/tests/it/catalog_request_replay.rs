//! DATA-03: a repeated catalog request ID replays its outcome, never applies twice.
//!
//! A client whose batch response was lost cannot know whether the batch was
//! committed. With a `requestId` it may resend the identical request: the
//! server returns the recorded outcome (`replayed: true`) instead of creating
//! the devices again. A reused ID with different content is refused.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

const ITEM: &str = "M-0001_H-1_P-1_CI-1";

fn products(dir: &tempfile::TempDir) -> knx_productdb::Connection {
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    // A programless product: the only catalog chain that needs no program XML.
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

async fn app() -> (tempfile::TempDir, axum::Router) {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        product_db: Some(Mutex::new(products(&dir))),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);
    let (status, _) = send(&app, "/api/project/new", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    (dir, app)
}

fn device_count(tree: &Value) -> usize {
    tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .len()
}

fn batch(request_id: &str, name: &str) -> Value {
    json!({ "catalogItemId": ITEM, "name": name, "quantity": 3, "requestId": request_id })
}

#[tokio::test]
async fn resending_the_same_request_id_replays_instead_of_creating_again() {
    let (_dir, app) = app().await;
    let (status, first) = send(&app, "/api/devices", batch("req-1", "PSU")).await;
    assert_eq!(status, StatusCode::OK, "{first}");
    assert_eq!(first["replayed"], false);
    assert_eq!(device_count(&first["tree"]), 3);

    // The "lost response" retry: identical body, identical ID.
    let (status, again) = send(&app, "/api/devices", batch("req-1", "PSU")).await;
    assert_eq!(status, StatusCode::OK, "{again}");
    assert_eq!(again["replayed"], true);
    assert_eq!(again["items"], first["items"]);
    assert_eq!(device_count(&again["tree"]), 3, "no second batch");

    // One undo removes the one batch; the replay pushed nothing.
    let (status, undone) = send(&app, "/api/undo", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(device_count(&undone), 0);
    assert_eq!(undone["can_undo"], false, "{undone}");
}

#[tokio::test]
async fn a_reused_request_id_with_different_content_is_refused() {
    let (_dir, app) = app().await;
    let (status, _) = send(&app, "/api/devices", batch("req-2", "PSU")).await;
    assert_eq!(status, StatusCode::OK);
    let (status, refused) = send(&app, "/api/devices", batch("req-2", "Other")).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert!(refused.to_string().contains("request"), "{refused}");
    let (_, tree) = send(&app, "/api/undo", json!({})).await;
    assert_eq!(device_count(&tree), 0, "only the first batch existed");
}

#[tokio::test]
async fn a_failed_request_is_not_recorded_and_may_be_retried() {
    let (_dir, app) = app().await;
    let bad = json!({ "catalogItemId": ITEM, "name": "PSU", "quantity": 3, "lineId": 999,
                      "requestId": "req-3" });
    let (status, _) = send(&app, "/api/devices", bad).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, ok) = send(&app, "/api/devices", batch("req-3", "PSU")).await;
    assert_eq!(status, StatusCode::OK, "{ok}");
    assert_eq!(ok["replayed"], false);
    assert_eq!(device_count(&ok["tree"]), 3);
}

#[tokio::test]
async fn requests_without_an_id_keep_their_old_behaviour() {
    let (_dir, app) = app().await;
    let body = json!({ "catalogItemId": ITEM, "name": "PSU", "quantity": 2 });
    let (_, first) = send(&app, "/api/devices", body.clone()).await;
    let (_, second) = send(&app, "/api/devices", body).await;
    assert_eq!(first["replayed"], false);
    assert_eq!(second["replayed"], false);
    assert_eq!(device_count(&second["tree"]), 4);
}

#[tokio::test]
async fn a_malformed_request_id_is_refused_before_anything_is_created() {
    let (_dir, app) = app().await;
    for id in ["", "has space", &"x".repeat(129)] {
        let (status, body) = send(&app, "/api/devices", batch(id, "PSU")).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{id:?}: {body}");
    }
    // Nothing was created: undo has nothing to take back.
    let (status, _) = send(&app, "/api/undo", json!({})).await;
    assert_ne!(status, StatusCode::OK);
}

#[tokio::test]
async fn replacing_the_project_forgets_recorded_requests() {
    let (_dir, app) = app().await;
    let (_, first) = send(&app, "/api/devices", batch("req-4", "PSU")).await;
    assert_eq!(device_count(&first["tree"]), 3);
    let (status, _) = send(&app, "/api/project/new", json!({ "discardChanges": true })).await;
    assert_eq!(status, StatusCode::OK);
    let (status, fresh) = send(&app, "/api/devices", batch("req-4", "PSU")).await;
    assert_eq!(status, StatusCode::OK, "{fresh}");
    assert_eq!(fresh["replayed"], false, "a new project is a new scope");
    assert_eq!(device_count(&fresh["tree"]), 3);
}
