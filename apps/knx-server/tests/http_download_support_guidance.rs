//! Verifies pre-plan support refusals without a connection or a report.

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use serde_json::{json, Value};
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

fn device(program: &str) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "test".into(),
            ets_id: "D-1".into(),
        },
        name: "PRIVATE device".into(),
        description: None,
        address: Some("1.1.10".parse().unwrap()),
        product_ref: String::new(),
        program_ref: program.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

async fn post(app: axum::Router, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
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
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn configuration_and_missing_application_have_distinct_codes_before_any_session() {
    for (program, category) in [
        ("", "configuration"),
        ("M-0001_A-0001-01-0000", "product-data"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        #[allow(clippy::field_reassign_with_default)]
        let state = {
            let mut state = knx_server::AppState::default();
            state.product_db = Some(Mutex::new(
                knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap(),
            ));
            let mut project = Project::new(Language("en".into()));
            project.devices.insert(device(program));
            *state.project.lock().unwrap() = Some(project);
            Arc::new(state)
        };
        let app = knx_server::app(state.clone(), None);
        let (status, body) = post(
            app.clone(),
            "/api/device-download/plan",
            json!({"address":"1.1.10"}),
        )
        .await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["kind"], "downloadPreparationRefused");
        assert_eq!(body["category"], category);
        assert!(body["error"]
            .as_str()
            .unwrap()
            .contains("no download to device 1.1.10 prepared"));
        let (status, _) = post(app, "/api/device-download/start", json!({"planId":0,"gateway":"192.0.2.10:3671","confirmation":"I confirm download to 1.1.10","acceptUntested":"I accept untested download to 1.1.10"})).await;
        assert!(!status.is_success());
    }
}

#[tokio::test]
async fn missing_database_is_not_an_unsupported_device() {
    let state = Arc::new(knx_server::AppState::default());
    let app = knx_server::app(state, None);
    let (status, body) = post(
        app,
        "/api/device-download/plan",
        json!({"address":"1.1.10"}),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["kind"], "downloadProductDataMissing");
}
