use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{CompletionStatus, Installation, InstallationId, Language, Project, Topology};
use serde_json::{json, Value};
use tower::ServiceExt;

fn state_with_one_installation() -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology { areas: vec![], lines: vec![], unassigned: vec![] },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn creating_then_deleting_a_group_address_round_trips() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Living room", "address": "1/1/1" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
    let tree = body_json(create).await;
    let ga_id = tree["installations"][0]["group_addresses"][0]["id"].as_u64().unwrap();

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/group-addresses/{ga_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);
    let tree = body_json(delete).await;
    assert!(tree["installations"][0]["group_addresses"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn creating_a_malformed_group_address_is_a_400() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "GA", "address": "not-an-address" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
