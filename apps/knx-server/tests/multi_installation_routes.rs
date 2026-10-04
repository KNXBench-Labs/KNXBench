//! MODEL-01 over HTTP: a later installation can be renamed and edited.
//!
//! The core decides ownership (`knx-core/tests/multi_installation.rs`); this
//! pins the route wiring: `PATCH /api/installations/{id}`, the optional
//! `installationId` on root creates, edits addressed by id in a later
//! installation, and the refusal of a cross-installation move.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    Area, AreaId, CompletionStatus, Installation, InstallationId, Language, Line, LineId, Project,
    SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn source() -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    }
}

fn installation(id: u8, name: &str, area: u32, line: u32) -> Installation {
    Installation {
        id: InstallationId(id),
        name: name.into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![Area {
                id: AreaId(area),
                source: source(),
                name: format!("Area {area}"),
                address: id + 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![LineId(line)],
            }],
            lines: vec![Line {
                id: LineId(line),
                source: source(),
                name: format!("Line {line}"),
                address: 1,
                medium_ref: "TP".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::FinishedDesign,
                devices: vec![],
            }],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    }
}

fn app() -> axum::Router {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(installation(0, "Home", 1, 1));
    project.installations.push(installation(1, "Garage", 2, 2));
    project.ids.raise_to(&knx_core::IdAllocators::from_counts(
        10, 10, 10, 10, 10, 10, 10, 10, 10,
    ));
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    knx_server::app(Arc::new(state), None)
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
    let value = serde_json::from_slice(&bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).into_owned()));
    (status, value)
}

#[tokio::test]
async fn a_later_installation_is_renamed_and_edited_over_http() {
    let app = app();
    let (status, tree) = send(
        &app,
        "PATCH",
        "/api/installations/1",
        json!({ "name": " Carport " }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    assert_eq!(tree["installations"][1]["name"], "Carport");
    assert_eq!(tree["installations"][0]["name"], "Home");

    let (status, tree) = send(
        &app,
        "PATCH",
        "/api/areas/2",
        json!({ "name": "Garage area" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    assert_eq!(
        tree["installations"][1]["topology"][0]["name"],
        "Garage area"
    );

    let (status, tree) = send(
        &app,
        "POST",
        "/api/areas",
        json!({ "name": "Shed", "address": 5, "installationId": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    assert_eq!(
        tree["installations"][1]["topology"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        tree["installations"][0]["topology"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let (status, tree) = send(
        &app,
        "POST",
        "/api/building-parts",
        json!({ "name": "Workshop", "kind": "Building", "installationId": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    assert_eq!(
        tree["installations"][1]["buildings"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    // One undo per edit, in reverse order.
    for _ in 0..4 {
        let (status, _) = send(&app, "POST", "/api/undo", json!({})).await;
        assert_eq!(status, StatusCode::OK);
    }
    let (_, tree) = send(&app, "POST", "/api/redo", json!({})).await;
    assert_eq!(tree["installations"][1]["name"], "Carport");
}

#[tokio::test]
async fn blank_names_unknown_installations_and_cross_moves_are_refused() {
    let app = app();
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/installations/1",
        json!({ "name": "  " }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = send(
        &app,
        "PATCH",
        "/api/installations/7",
        json!({ "name": "X" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, body) = send(
        &app,
        "POST",
        "/api/areas",
        json!({ "name": "X", "address": 9, "installationId": 7 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, body) = send(
        &app,
        "POST",
        "/api/move-line-to-area",
        json!({ "id": 1, "areaId": 2 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(
        body.to_string().contains("separate infrastructures"),
        "{body}"
    );
}
