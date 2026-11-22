//! WP-31: catalogue device creation is refused during a Save instead of queueing behind it.
use std::sync::Arc;

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

async fn send(app: &axum::Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
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
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

fn device_count(tree: &Value) -> usize {
    tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .len()
}

#[tokio::test]
async fn creating_devices_during_a_save_is_refused_at_once_and_works_after() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState {
        product_db: Some(
            knx_server::CatalogueOwner::adopt_opened(products(&dir), std::path::PathBuf::new())
                .unwrap(),
        ),
        ..Default::default()
    });
    let app = knx_server::app(Arc::clone(&state), None);
    assert_eq!(
        send(&app, "POST", "/api/project/new", json!({})).await.0,
        StatusCode::OK
    );
    let store = dir.path().join("target.knxdb");
    let (status, saved) = send(&app, "POST", "/api/project/save-as", json!({"path": store})).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let before = state.project.lock().unwrap().clone();
    let file_before = std::fs::read(&store).unwrap();
    let (_, tree_before) = send(&app, "GET", "/api/project", json!({})).await;
    let mut request = json!({ "catalogItemId": ITEM, "name": "PSU", "quantity": 2 });
    let (status, preview) = crate::saving_hold::answered_while_saving(
        &state,
        &app,
        "POST",
        "/api/devices/preview",
        request.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    request["expected"] = json!(preview["items"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| json!({"name": item["name"], "address": item["address"]}))
        .collect::<Vec<_>>());
    request["requestId"] = json!("save-refused-create");

    crate::saving_hold::refused_while_saving(&state, &app, "POST", "/api/devices", request.clone())
        .await;

    // Nothing was reserved, created or pushed; the project answers reads.
    let (status, tree) = send(&app, "GET", "/api/project", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(device_count(&tree), 0, "{tree}");
    assert_eq!(tree, tree_before);
    assert_eq!(*state.project.lock().unwrap(), before); // includes the ID allocator
    assert_eq!(std::fs::read(&store).unwrap(), file_before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );

    let (status, created) = send(&app, "POST", "/api/devices", request.clone()).await;
    assert_eq!(status, StatusCode::OK, "{created}");
    assert_eq!(device_count(&created["tree"]), 2);
    assert_eq!(created["replayed"], false);
    let after = std::fs::read(&store).unwrap();
    let (status, replay) =
        crate::saving_hold::answered_while_saving(&state, &app, "POST", "/api/devices", request)
            .await;
    assert_eq!(status, StatusCode::OK, "{replay}");
    assert_eq!(replay["replayed"], true);
    assert_eq!(replay["items"], created["items"]);
    assert_eq!(device_count(&replay["tree"]), 2);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
    assert_eq!(std::fs::read(&store).unwrap(), after);
}
