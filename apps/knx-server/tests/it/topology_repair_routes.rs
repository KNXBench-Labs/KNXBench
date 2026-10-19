//! MODEL-02 over HTTP: ambiguous imported topology can be repaired.
//!
//! The core semantics live in `knx-core/tests/topology_repair.rs`; this pins
//! the route wiring of `POST /api/repair/device-placement` and
//! `POST /api/repair/line-owner`, their refusals, and undo.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    Area, AreaId, CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation,
    InstallationId, Language, Line, LineId, Project, SourceRef, Topology,
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
    let mut home = installation(0, "Home", 1, 1);
    // Imported oddities: line 1 also listed by a second area 3, and device 1
    // both on line 1 and in the unassigned list.
    home.topology.areas.push(Area {
        id: AreaId(3),
        source: source(),
        name: "Area 3".into(),
        address: 3,
        completion: CompletionStatus::FinishedDesign,
        lines: vec![LineId(1)],
    });
    home.topology.lines[0].devices.push(DeviceId(1));
    home.topology.unassigned.push(DeviceId(1));
    project.installations.push(home);
    project.installations.push(installation(1, "Garage", 2, 2));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source(),
        name: "D1".into(),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
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
async fn a_device_placement_is_repaired_and_undone_over_http() {
    let app = app();
    let (status, body) = send(
        &app,
        "POST",
        "/api/repair/device-placement",
        json!({ "deviceId": 1, "keepLineId": 1, "keepUnassignedInstallationId": 0 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, body) = send(
        &app,
        "POST",
        "/api/repair/device-placement",
        json!({ "deviceId": 1, "keepLineId": 2 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.to_string().contains("current placements"), "{body}");

    let (status, tree) = send(
        &app,
        "POST",
        "/api/repair/device-placement",
        json!({ "deviceId": 1, "keepUnassignedInstallationId": 0 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let repaired = tree["installations"].clone();
    let (status, _) = send(&app, "POST", "/api/undo", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let (status, tree) = send(&app, "POST", "/api/redo", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(tree["installations"], repaired);
    // Now unambiguous: a second repair is refused.
    let (status, body) = send(
        &app,
        "POST",
        "/api/repair/device-placement",
        json!({ "deviceId": 1, "keepUnassignedInstallationId": 0 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(body.to_string().contains("nothing to repair"), "{body}");
}

#[tokio::test]
async fn a_line_owner_is_repaired_over_http() {
    let app = app();
    let (status, body) = send(
        &app,
        "POST",
        "/api/repair/line-owner",
        json!({ "lineId": 1, "keepAreaId": 2 }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    let (status, tree) = send(
        &app,
        "POST",
        "/api/repair/line-owner",
        json!({ "lineId": 1, "keepAreaId": 3 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    // The line still carries the doubly placed device; repair that too,
    // then the ordinary line move is allowed again.
    let (status, tree) = send(
        &app,
        "POST",
        "/api/repair/device-placement",
        json!({ "deviceId": 1, "keepLineId": 1 }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let (status, tree) = send(
        &app,
        "POST",
        "/api/move-line-to-area",
        json!({ "id": 1, "areaId": 1 }),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "ordinary editing works again: {tree}"
    );
}
