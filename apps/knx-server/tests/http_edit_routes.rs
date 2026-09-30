use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    Area, AreaId, CommissioningState, CompletionStatus, DeviceId, DeviceInstance,
    IndividualAddress, Installation, InstallationId, Language, Line, LineId, Project, SourceRef,
    Topology,
};
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
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

fn state_with_one_installation_and_device() -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        },
        name: "D".into(),
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

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
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
                .body(Body::from(
                    json!({ "name": "Living room", "address": "1/1/1" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
    let tree = body_json(create).await;
    let ga_id = tree["installations"][0]["group_addresses"][0]["id"]
        .as_u64()
        .unwrap();

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
    assert!(tree["installations"][0]["group_addresses"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn creating_a_malformed_group_address_is_a_422() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "GA", "address": "not-an-address" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let body = body_json(response).await;
    assert_eq!(body["kind"], "group_address");
    assert_eq!(body["syntax"], "main/middle/sub");
}

#[tokio::test]
async fn undo_after_create_removes_it_and_redo_brings_it_back() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "GA", "address": "1/1/1" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let undo = app
        .clone()
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
    assert!(tree["installations"][0]["group_addresses"]
        .as_array()
        .unwrap()
        .is_empty());

    let redo = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/redo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(redo.status(), StatusCode::OK);
    let tree = body_json(redo).await;
    assert_eq!(
        tree["installations"][0]["group_addresses"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn creating_then_deleting_an_area_round_trips() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/areas")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Area 1", "address": 1 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
    let tree = body_json(create).await;
    let area_id = tree["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/areas/{area_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);
    let tree = body_json(delete).await;
    assert!(tree["installations"][0]["topology"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn deleting_a_nonempty_area_is_a_400() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let area_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/areas")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Area 1", "address": 1 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(area_create).await;
    let area_id = tree["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lines")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "areaId": area_id,
                        "name": "Line 1",
                        "address": 1,
                        "mediumRef": "MT-0"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/areas/{area_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn creating_a_line_nests_it_under_its_area_then_deletes() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let area_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/areas")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Area 1", "address": 1 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(area_create).await;
    let area_id = tree["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();

    let line_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lines")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "areaId": area_id,
                        "name": "Line 1",
                        "address": 1,
                        "mediumRef": "MT-0"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(line_create.status(), StatusCode::OK);
    let tree = body_json(line_create).await;
    let lines = tree["installations"][0]["topology"][0]["lines"]
        .as_array()
        .unwrap();
    assert_eq!(lines.len(), 1);
    let line_id = lines[0]["id"].as_u64().unwrap();

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/lines/{line_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);
    let tree = body_json(delete).await;
    assert!(tree["installations"][0]["topology"][0]["lines"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn moving_a_device_between_unassigned_and_a_line() {
    let state = Arc::new(state_with_one_installation_and_device());
    let app = knx_server::app(state, None);

    let area_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/areas")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Area 1", "address": 1 }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(area_create).await;
    let area_id = tree["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();

    let line_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lines")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "areaId": area_id,
                        "name": "Line 1",
                        "address": 1,
                        "mediumRef": "MT-0"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(line_create).await;
    let line_id = tree["installations"][0]["topology"][0]["lines"][0]["id"]
        .as_u64()
        .unwrap();

    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-device")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "deviceId": 1, "lineId": line_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let tree = body_json(moved).await;
    assert!(tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        tree["installations"][0]["topology"][0]["lines"][0]["devices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let back = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-device")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "deviceId": 1, "lineId": null }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(back.status(), StatusCode::OK);
    let tree = body_json(back).await;
    assert_eq!(
        tree["installations"][0]["unassigned"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn creating_a_nested_group_range_then_renaming_and_deleting_it() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let main = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-ranges")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Main", "start": "0/0/0", "end": "0/7/255" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(main.status(), StatusCode::OK);
    let tree = body_json(main).await;
    let main_id = tree["installations"][0]["group_ranges"][0]["id"]
        .as_u64()
        .unwrap();

    let middle = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-ranges")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "name": "Middle",
                        "start": "0/0/0",
                        "end": "0/0/255",
                        "parentId": main_id
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(middle.status(), StatusCode::OK);
    let tree = body_json(middle).await;
    let ranges = tree["installations"][0]["group_ranges"].as_array().unwrap();
    assert_eq!(ranges.len(), 2);
    let middle_id = ranges
        .iter()
        .find(|r| r["parent"].as_u64() == Some(main_id))
        .unwrap()["id"]
        .as_u64()
        .unwrap();

    let renamed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/group-ranges/{middle_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Renamed" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(renamed.status(), StatusCode::OK);
    let tree = body_json(renamed).await;
    let renamed_range = tree["installations"][0]["group_ranges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"].as_u64() == Some(middle_id))
        .unwrap();
    assert_eq!(renamed_range["name"], "Renamed");

    let delete_middle = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/group-ranges/{middle_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_middle.status(), StatusCode::OK);

    let delete_main = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/group-ranges/{main_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_main.status(), StatusCode::OK);
    let tree = body_json(delete_main).await;
    assert!(tree["installations"][0]["group_ranges"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn building_part_creation_preserves_all_documented_space_types() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);
    for token in ["Stairway", "RoomPart", "Area", "Ground", "Segment"] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/building-parts")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({"name": token, "kind": token}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK, "{token}");
        let tree = body_json(response).await;
        let parts = tree["installations"][0]["buildings"].as_array().unwrap();
        assert!(parts
            .iter()
            .any(|part| part["name"] == token && part["kind"] == token));
    }
}

#[tokio::test]
async fn creating_a_nested_building_part_then_renaming_and_deleting_it() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let root = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Main building", "kind": "Building" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(root.status(), StatusCode::OK);
    let tree = body_json(root).await;
    let root_id = tree["installations"][0]["buildings"][0]["id"]
        .as_u64()
        .unwrap();

    let child = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Floor 1", "kind": "Floor", "parentId": root_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(child.status(), StatusCode::OK);
    let tree = body_json(child).await;
    let child_id = tree["installations"][0]["buildings"][0]["children"][0]["id"]
        .as_u64()
        .unwrap();

    let renamed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/building-parts/{child_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Ground floor" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(renamed.status(), StatusCode::OK);
    let tree = body_json(renamed).await;
    assert_eq!(
        tree["installations"][0]["buildings"][0]["children"][0]["name"],
        "Ground floor"
    );

    let delete_child = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/building-parts/{child_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_child.status(), StatusCode::OK);

    let delete_root = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/building-parts/{root_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_root.status(), StatusCode::OK);
    let tree = body_json(delete_root).await;
    assert!(tree["installations"][0]["buildings"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn deleting_a_nonempty_building_part_is_a_400() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let root = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Main building", "kind": "Building" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(root).await;
    let root_id = tree["installations"][0]["buildings"][0]["id"]
        .as_u64()
        .unwrap();

    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Floor 1", "kind": "Floor", "parentId": root_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    let delete_root = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/building-parts/{root_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete_root.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn moving_a_device_into_a_building_part_and_back_out() {
    let state = Arc::new(state_with_one_installation_and_device());
    let app = knx_server::app(state, None);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Living room", "kind": "Room" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(create).await;
    let part_id = tree["installations"][0]["buildings"][0]["id"]
        .as_u64()
        .unwrap();

    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-device-to-building-part")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "deviceId": 1, "partId": part_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let tree = body_json(moved).await;
    assert_eq!(
        tree["installations"][0]["buildings"][0]["devices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let back = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-device-to-building-part")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "deviceId": 1, "partId": null }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(back.status(), StatusCode::OK);
    let tree = body_json(back).await;
    assert!(tree["installations"][0]["buildings"][0]["devices"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn creating_a_building_part_with_an_unknown_kind_is_a_400() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/building-parts")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "X", "kind": "Basement" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn creating_a_group_address_without_a_range_still_works_unchanged() {
    // Regression guard: the shipped frontend (Session 5 cycle 9) never
    // sends `rangeId` — this must keep working exactly as before.
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let create = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Living room", "address": "1/1/1" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
}

#[tokio::test]
async fn creating_a_group_address_with_a_range_id_validates_it_falls_inside() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let range_create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-ranges")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "Main", "start": "0/0/0", "end": "0/7/255" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let tree = body_json(range_create).await;
    let range_id = tree["installations"][0]["group_ranges"][0]["id"]
        .as_u64()
        .unwrap();

    let inside = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "GA", "address": "0/0/1", "rangeId": range_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(inside.status(), StatusCode::OK);

    let outside = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-addresses")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "name": "GA2", "address": "5/0/1", "rangeId": range_id }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(outside.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_line_bound_address_route_refuses_invalid_values_and_undo_restores_imported_data() {
    let state = Arc::new(state_with_one_installation_and_device());
    let original = IndividualAddress::new(2, 1, 9).unwrap();
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let installation = &mut project.installations[0];
        installation.topology = Topology {
            areas: vec![Area {
                id: AreaId(10),
                source: SourceRef {
                    path: "fixture".into(),
                    ets_id: "area".into(),
                },
                name: "A".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![LineId(11)],
            }],
            lines: vec![Line {
                id: LineId(11),
                source: SourceRef {
                    path: "fixture".into(),
                    ets_id: "line".into(),
                },
                name: "L".into(),
                address: 1,
                medium_ref: String::new(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::FinishedDesign,
                devices: vec![DeviceId(1), DeviceId(2)],
            }],
            unassigned: vec![],
        };
        project.devices.get_mut(DeviceId(1)).unwrap().address = Some(original);
        let mut other = project.devices.get(DeviceId(1)).unwrap().clone();
        other.id = DeviceId(2);
        other.address = Some(IndividualAddress::new(1, 1, 17).unwrap());
        project.devices.insert(other);
    }
    let app = knx_server::app(Arc::clone(&state), None);
    for (address, reason) in [
        ("1.2.18", "assigned line 1.1"),
        ("1.1.0", "reserved for couplers"),
        ("1.1.17", "already used"),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/individual-address")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "deviceId": 1, "address": address }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{address}");
        let error = body_json(response).await;
        assert!(
            error["error"].as_str().unwrap().contains(reason),
            "{address}: {error}"
        );
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .devices
                .get(DeviceId(1))
                .unwrap()
                .address,
            Some(original)
        );
    }

    let accepted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/individual-address")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "deviceId": 1, "address": "1.1.18" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(accepted.status(), StatusCode::OK);
    let accepted_tree = body_json(accepted).await;
    assert_eq!(
        accepted_tree["installations"][0]["topology"][0]["lines"][0]["devices"][0]["address"],
        "1.1.18"
    );
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .devices
            .get(DeviceId(1))
            .unwrap()
            .address,
        Some(IndividualAddress::new(1, 1, 18).unwrap())
    );
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
    let undone_tree = body_json(undo).await;
    assert_eq!(
        undone_tree["installations"][0]["topology"][0]["lines"][0]["devices"][0]["address"],
        "2.1.9"
    );
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .devices
            .get(DeviceId(1))
            .unwrap()
            .address,
        Some(original)
    );
}

#[tokio::test]
async fn linking_then_unlinking_a_com_object_to_a_group_address() {
    // No device-creation route exists yet (Sub-Project 2), so this test
    // seeds a com object directly the same way command.rs's own fixtures
    // do, via a raw sqlite-free in-memory Project built by hand.
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![knx_core::GroupAddressEntry {
            id: knx_core::GroupAddressId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "GA".into(),
            address: knx_core::GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: None,
        }],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        },
        name: "D".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![knx_core::ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    project
        .devices
        .insert_com_object(knx_core::ComObjectInstance {
            id: knx_core::ComObjectInstanceId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            device: DeviceId(1),
            number: 0,
            text: knx_core::Override::Absent,
            description: knx_core::Override::Absent,
            dpt: knx_core::Override::Absent,
            flags: knx_core::ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    let app = knx_server::app(Arc::new(state), None);

    let link = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": 1, "gaId": 1, "direction": "Send" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(link.status(), StatusCode::OK);

    let unlink = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": 1, "gaId": 1, "direction": "Send" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unlink.status(), StatusCode::OK);

    // If Receive already exists, the second item of Both fails. The Send
    // item must be rolled back, not left as a half-applied UI action.
    let receive = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": 1, "gaId": 1, "direction": "Receive" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(receive.status(), StatusCode::OK);
    let conflict = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": 1, "gaId": 1, "direction": "Both" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(conflict.status(), StatusCode::BAD_REQUEST);
    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let links = body_json(detail).await["com_objects"][0]["links"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(links.len(), 1);
    assert_eq!(links[0]["direction"], "Receive");
    let unlink_receive = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": 1, "gaId": 1, "direction": "Receive" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unlink_receive.status(), StatusCode::OK);

    // The additive "Both" action creates two directional links in one
    // command, without changing the original Send/Receive request shapes.
    let both = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": 1, "gaId": 1, "direction": "Both" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(both.status(), StatusCode::OK);
    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        body_json(detail).await["com_objects"][0]["links"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let remove_both = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/group-links")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "comObjectId": 1, "gaId": 1, "direction": "Both" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(remove_both.status(), StatusCode::OK);
    let detail = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert!(body_json(detail).await["com_objects"][0]["links"]
        .as_array()
        .unwrap()
        .is_empty());
}

/// `Project::new` defaults to `ThreeLevel` (`ProjectInfo::default`), so
/// restyling to `Free` and `TwoLevel` here exercises the other two
/// (KNOWN_LIMITATIONS.md §84), with a group address already present —
/// `Command::SetGroupAddressStyle` checks it and, per the exhaustive proof
/// in `knx-core/src/address.rs`, always finds it fits, so this is also the
/// route's ordinary, expected-to-succeed path, not a corner case.
#[tokio::test]
async fn group_address_style_restyles_a_project_and_round_trips_through_undo() {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![knx_core::GroupAddressEntry {
            id: knx_core::GroupAddressId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "GA".into(),
            address: knx_core::GroupAddress::from_raw(4242),
            central: false,
            unfiltered: false,
            range: None,
        }],
        parameters: vec![],
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    let app = knx_server::app(Arc::new(state), None);

    let restyled = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/group-address-style")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "groupAddressStyle": "Free" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(restyled.status(), StatusCode::OK);
    let restyled_tree = body_json(restyled).await;
    // A handler that ignored `groupAddressStyle` and restyled to
    // `ThreeLevel` regardless would still answer `200` here — the actual
    // forward effect only shows up in the response body.
    assert_eq!(restyled_tree["group_address_style"], "Free");
    assert_eq!(
        restyled_tree["installations"][0]["group_addresses"][0]["address"],
        "4242"
    );

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
    // Undone back to `ThreeLevel` (`Project::new`'s default), so the same
    // raw address formats with a middle group again.
    assert_eq!(
        tree["installations"][0]["group_addresses"][0]["address"],
        "2/0/146"
    );
}

/// The counterpart to `POST /api/project/new`'s own `400` on an unknown
/// `groupAddressStyle` — same wire vocabulary, same refusal, now at the
/// other end of a project's life.
#[tokio::test]
async fn restyling_to_an_unknown_style_is_a_400() {
    let state = Arc::new(state_with_one_installation());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/project/group-address-style")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "groupAddressStyle": "Sideways" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}
