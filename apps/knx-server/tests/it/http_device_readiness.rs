//! `/api/device-readiness`: the open project's devices, graded offline.
//!
//! No corpus: the project is put into the state directly and the product
//! database is empty, so every addressed device is refused for its
//! configuration, the excluded one is never prepared, and one without an
//! address has no download. What is pinned here is the route's contract;
//! the grades themselves are `knx_app::project_readiness`'s, tested there
//! and against the house corpus.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use serde_json::Value;
use tower::ServiceExt;

fn device(id: u32, address: Option<&str>) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: SourceRef {
            path: "test".into(),
            ets_id: format!("DEV-{id}"),
        },
        name: format!("Device {id}"),
        description: None,
        address: address.map(|a| a.parse().unwrap()),
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

// Same documented reassignment as `http_device_routes.rs`: the product
// database is set explicitly, never picked up from the machine.
#[allow(clippy::field_reassign_with_default)]
fn state(project: Option<Project>, products: bool) -> (tempfile::TempDir, knx_server::AppState) {
    let dir = tempfile::tempdir().unwrap();
    let mut state = knx_server::AppState::default();
    state.product_db = products.then(|| {
        knx_server::CatalogueOwner::adopt_opened(
            knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap(),
            std::path::PathBuf::new(),
        )
        .unwrap()
    });
    *state.project.lock().unwrap() = project;
    (dir, state)
}

async fn get(state: knx_server::AppState) -> (StatusCode, Value) {
    get_query(state, "").await
}

async fn get_query(state: knx_server::AppState, query: &str) -> (StatusCode, Value) {
    let app = knx_server::app(Arc::new(state), None);
    get_from(app, query).await
}

async fn get_from(app: axum::Router, query: &str) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/api/device-readiness{query}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let body = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, body)
}

#[tokio::test]
async fn every_device_is_listed_with_its_grade_and_the_counts() {
    let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(device(1, Some("1.1.10")));
    project.devices.insert(device(2, Some(&excluded)));
    project.devices.insert(device(3, None));
    let (_dir, state) = state(Some(project), true);
    let (status, body) = get(state).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let devices = body["devices"].as_array().unwrap();
    assert_eq!(devices.len(), 3, "{body}");

    assert_eq!(devices[0]["address"], "1.1.10");
    assert_eq!(devices[0]["readiness"], "unsupported");
    assert_eq!(devices[0]["category"], "configuration");
    assert!(
        devices[0]["detail"]
            .as_str()
            .unwrap()
            .contains("names no application program"),
        "{body}"
    );
    assert_eq!(devices[0]["steps"], Value::Null);

    assert_eq!(devices[1]["address"], excluded.as_str());
    assert_eq!(devices[1]["readiness"], "excluded");
    assert_eq!(devices[1]["detail"], Value::Null);

    assert_eq!(devices[2]["address"], Value::Null);
    assert_eq!(devices[2]["readiness"], "no-address");

    assert_eq!(body["counts"]["unsupported"], 1);
    assert_eq!(body["counts"]["excluded"], 1);
    assert_eq!(body["counts"]["no-address"], 1);
}

#[tokio::test]
async fn typed_device_selector_returns_only_that_row_and_subset_counts() {
    let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
    let mut project = Project::new(Language("en".into()));
    for device in [
        device(1, Some("1.1.10")),
        device(2, Some(&excluded)),
        device(3, None),
        device(4, Some("1.1.10")),
    ] {
        project.devices.insert(device);
    }
    let (_dir, state) = state(Some(project), true);
    let app = knx_server::app(Arc::new(state), None);
    let (status, full) = get_from(app.clone(), "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(full["devices"].as_array().unwrap().len(), 4);
    for id in 1..=4 {
        let (status, body) = get_from(app.clone(), &format!("?deviceId={id}")).await;
        assert_eq!(status, StatusCode::OK, "{body}");
        let expected = full["devices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["name"] == format!("Device {id}"))
            .unwrap();
        assert_eq!(body["devices"], serde_json::json!([expected]));
        let code = expected["readiness"].as_str().unwrap();
        assert_eq!(body["counts"], serde_json::json!({ code: 1 }));
        if id == 1 || id == 4 {
            assert!(body["devices"][0]["detail"]
                .as_str()
                .unwrap()
                .contains("2 devices"));
        }
    }
    let (status, body) = get_from(app, "?deviceId=99").await;
    assert_eq!(status, StatusCode::NOT_FOUND, "{body}");
    assert_eq!(body["error"], "device not found");
    assert!(body.get("devices").is_none());
}

#[tokio::test]
async fn malformed_or_duplicate_device_selectors_are_not_silently_full_reads() {
    let (_dir, state) = state(Some(Project::new(Language("en".into()))), true);
    let app = knx_server::app(Arc::new(state), None);
    for query in [
        "?deviceId=",
        "?deviceId=no",
        "?deviceId=-1",
        "?deviceId=4294967296",
        "?deviceId=1&deviceId=2",
        "?deviceId=1&deviceId=1",
    ] {
        let (status, body) = get_from(app.clone(), query).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{query}: {body}");
        assert!(body.get("devices").is_none(), "{query}: {body}");
    }
}

#[tokio::test]
async fn without_a_project_or_product_database_it_says_why() {
    let (_dir, no_project) = state(None, true);
    let (status, body) = get(no_project).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body.to_string().contains("no project is open"), "{body}");

    let (_dir, no_products) = state(Some(Project::new(Language("en".into()))), false);
    let (status, body) = get(no_products).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(body.to_string().contains("no product database"), "{body}");
}

#[tokio::test]
async fn it_is_read_only_http_too() {
    let (_dir, state) = state(Some(Project::new(Language("en".into()))), true);
    let app = knx_server::app(Arc::new(state), None);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/device-readiness")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
}
