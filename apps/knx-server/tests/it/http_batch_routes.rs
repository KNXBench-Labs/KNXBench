//! HTTP tests for the four `Command::Batch`-backed batch routes
//! (`docs/superpowers/specs/2026-09-10-bulk-operations-design.md`):
//! batch-delete devices, batch-delete group addresses, batch-move devices
//! to a line, batch-move devices to a building part. Each covers a happy
//! path, the empty-id-list 400, a bad id inside an otherwise-valid batch
//! rolling back cleanly, and (once) that a batch undoes in a single call.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, CommissioningState,
    CompletionStatus, DeviceId, DeviceInstance, Direction, GroupAddress, GroupAddressEntry,
    GroupAddressId, GroupLink, Installation, InstallationId, Language, Line, LineId, Override,
    Project, ResolvedFlags, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

fn device(id: u32) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: source("t"),
        name: format!("D{id}"),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

fn group_address(id: u32, raw: u16) -> GroupAddressEntry {
    GroupAddressEntry {
        id: GroupAddressId(id),
        source: source("t"),
        name: format!("GA{id}"),
        address: GroupAddress::from_raw(raw),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Default::default(),
    }
}

/// Builds a project with:
/// - devices 1, 2, 3 unassigned, no links (freely deletable/movable)
/// - device 4 unassigned, with one com object linking group address 3
///   (`DeviceHasLinks` on delete — unused directly here but keeps the
///   fixture symmetric with `group_address(3, ..)` below being in-use)
/// - group addresses 1, 2 (unlinked), 3 (linked from device 4's com object,
///   so `GroupAddressInUse` on delete)
/// - line 1 (empty), building part 1 (empty)
///
/// `product_db = None`, same deliberate reassignment as every other
/// `http_*.rs` fixture in this crate.
#[allow(clippy::field_reassign_with_default)]
fn state_with_fixture() -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: source("t"),
                name: "Area".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![LineId(1)],
            }],
            lines: vec![Line {
                id: LineId(1),
                source: source("t"),
                name: "Line".into(),
                address: 1,
                medium_ref: "MT-0".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::FinishedDesign,
                devices: vec![],
            }],
            unassigned: vec![DeviceId(1), DeviceId(2), DeviceId(3), DeviceId(4)],
        },
        buildings: vec![BuildingPart {
            id: BuildingPartId(1),
            source: source("t"),
            name: "Part".into(),
            number: None,
            kind: BuildingPartType::Room,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![],
            devices: vec![],
            parent: None,
        }],
        group_ranges: vec![],
        group_addresses: vec![
            group_address(1, 1),
            group_address(2, 2),
            group_address(3, 3),
        ],
        parameters: vec![],
    });
    let mut device4 = device(4);
    device4.com_objects = vec![knx_core::ComObjectInstanceId(1)];
    for d in [device(1), device(2), device(3), device4] {
        project.devices.insert(d);
    }
    project
        .devices
        .insert_com_object(knx_core::ComObjectInstance {
            id: knx_core::ComObjectInstanceId(1),
            source: source("t"),
            device: DeviceId(4),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![GroupLink {
                ga: GroupAddressId(3),
                direction: Direction::Send,
            }],
            module_instance: None,
        });

    let mut state = knx_server::AppState::default();
    state.product_db = None;
    *state.project.lock().unwrap() = Some(project);
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn post(app: axum::Router, uri: &str, body: Value) -> axum::response::Response {
    app.oneshot(
        Request::builder()
            .method("POST")
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap(),
    )
    .await
    .unwrap()
}

fn unassigned_ids(tree: &Value) -> Vec<u64> {
    tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_u64().unwrap())
        .collect()
}

// --- batch-delete devices -------------------------------------------------

#[tokio::test]
async fn batch_delete_devices_happy_path_removes_both() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(app, "/api/devices/batch-delete", json!({ "ids": [1, 2] })).await;
    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    let remaining = unassigned_ids(&tree);
    assert_eq!(remaining, vec![3, 4]);
}

#[tokio::test]
async fn batch_delete_devices_empty_ids_is_a_400() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(app, "/api/devices/batch-delete", json!({ "ids": [] })).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn batch_delete_devices_bad_id_leaves_state_unchanged() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(app, "/api/devices/batch-delete", json!({ "ids": [1, 999] })).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let project = state.project.lock().unwrap();
    let mut unassigned = project.as_ref().unwrap().installations[0]
        .topology
        .unassigned
        .clone();
    unassigned.sort_by_key(|d| d.0);
    assert_eq!(
        unassigned,
        vec![DeviceId(1), DeviceId(2), DeviceId(3), DeviceId(4)],
        "device 1 must still be present — the batch rolled back entirely \
         (order may differ: the inverse re-appends rather than reinserting \
         at the original position)"
    );
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[tokio::test]
async fn batch_delete_devices_undo_restores_all_in_one_call() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(
        app.clone(),
        "/api/devices/batch-delete",
        json!({ "ids": [1, 2] }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);

    let undo = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/undo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(undo.status(), StatusCode::OK);
    let tree = body_json(undo).await;
    let mut remaining = unassigned_ids(&tree);
    remaining.sort_unstable();
    assert_eq!(
        remaining,
        vec![1, 2, 3, 4],
        "one undo restores both devices"
    );
}

// --- batch-delete group addresses -----------------------------------------

#[tokio::test]
async fn batch_delete_group_addresses_happy_path_removes_both() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(
        app,
        "/api/group-addresses/batch-delete",
        json!({ "ids": [1, 2] }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    let remaining: Vec<u64> = tree["installations"][0]["group_addresses"]
        .as_array()
        .unwrap()
        .iter()
        .map(|ga| ga["id"].as_u64().unwrap())
        .collect();
    assert_eq!(remaining, vec![3]);
}

#[tokio::test]
async fn batch_delete_group_addresses_empty_ids_is_a_400() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(
        app,
        "/api/group-addresses/batch-delete",
        json!({ "ids": [] }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn batch_delete_group_addresses_in_use_id_leaves_state_unchanged() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    // GA 3 is still linked from device 4's com object -> GroupAddressInUse.
    let response = post(
        app,
        "/api/group-addresses/batch-delete",
        json!({ "ids": [1, 3] }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let project = state.project.lock().unwrap();
    let gas = &project.as_ref().unwrap().installations[0].group_addresses;
    assert_eq!(gas.len(), 3, "GA 1 must still be present — full rollback");
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

// --- batch-move devices to a line -----------------------------------------

#[tokio::test]
async fn batch_move_devices_to_line_happy_path_moves_both() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(
        app,
        "/api/devices/batch-move-line",
        json!({ "deviceIds": [1, 2], "lineId": 1 }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    let mut remaining = unassigned_ids(&tree);
    remaining.sort_unstable();
    assert_eq!(remaining, vec![3, 4]);
    let line_devices: Vec<u64> = tree["installations"][0]["topology"][0]["lines"][0]["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_u64().unwrap())
        .collect();
    assert_eq!(line_devices, vec![1, 2]);
}

#[tokio::test]
async fn batch_move_devices_to_line_empty_ids_is_a_400() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(
        app,
        "/api/devices/batch-move-line",
        json!({ "deviceIds": [], "lineId": 1 }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn batch_move_devices_to_line_bad_id_leaves_state_unchanged() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(
        app,
        "/api/devices/batch-move-line",
        json!({ "deviceIds": [1, 999], "lineId": 1 }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let project = state.project.lock().unwrap();
    let topology = &project.as_ref().unwrap().installations[0].topology;
    assert!(
        topology.lines[0].devices.is_empty(),
        "line must still be empty — full rollback"
    );
    let mut unassigned = topology.unassigned.clone();
    unassigned.sort_by_key(|d| d.0);
    assert_eq!(
        unassigned,
        vec![DeviceId(1), DeviceId(2), DeviceId(3), DeviceId(4)]
    );
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

// --- batch-move devices to a building part --------------------------------

#[tokio::test]
async fn batch_move_devices_to_building_part_happy_path_moves_both() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(
        app,
        "/api/devices/batch-move-building-part",
        json!({ "deviceIds": [1, 2], "buildingPartId": 1 }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let tree = body_json(response).await;
    let part_devices: Vec<u64> = tree["installations"][0]["buildings"][0]["devices"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["id"].as_u64().unwrap())
        .collect();
    assert_eq!(part_devices, vec![1, 2]);
}

#[tokio::test]
async fn batch_move_devices_to_building_part_empty_ids_is_a_400() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(
        app,
        "/api/devices/batch-move-building-part",
        json!({ "deviceIds": [], "buildingPartId": 1 }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn batch_move_devices_to_building_part_bad_id_leaves_state_unchanged() {
    let state = Arc::new(state_with_fixture());
    let app = knx_server::app(Arc::clone(&state), None);

    let response = post(
        app,
        "/api/devices/batch-move-building-part",
        json!({ "deviceIds": [1, 999], "buildingPartId": 1 }),
    )
    .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let project = state.project.lock().unwrap();
    let part = &project.as_ref().unwrap().installations[0].buildings[0];
    assert!(
        part.devices.is_empty(),
        "building part must still be empty — full rollback"
    );
    assert!(!state.command_stack.lock().unwrap().can_undo());
}
