use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{DeviceId, DeviceInstance, CommissioningState, Language, Project, SourceRef};
use tower::ServiceExt;

fn state_with_one_device() -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef { path: "t".into(), ets_id: "t".into() },
        name: "D1".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

#[tokio::test]
async fn device_detail_returns_the_device() {
    let state = Arc::new(state_with_one_device());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(Request::builder().uri("/api/device/1").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn device_detail_for_a_missing_device_is_a_400() {
    let state = Arc::new(state_with_one_device());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(Request::builder().uri("/api/device/999").body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
