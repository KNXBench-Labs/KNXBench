//! `/api/device-readiness`: the open project's devices, graded offline.
//!
//! No corpus: the project is put into the state directly and the product
//! database is empty, so every addressed device is refused for its
//! configuration, the excluded one is never prepared, and one without an
//! address has no download. What is pinned here is the route's contract;
//! the grades themselves are `knx_app::project_readiness`'s, tested there
//! and against the house corpus.

use std::sync::{Arc, Mutex};

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
        Mutex::new(knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap())
    });
    *state.project.lock().unwrap() = project;
    (dir, state)
}

async fn get(state: knx_server::AppState) -> (StatusCode, Value) {
    let app = knx_server::app(Arc::new(state), None);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device-readiness")
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
