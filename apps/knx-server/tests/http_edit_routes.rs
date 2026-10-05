use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, CommissioningState,
    CompletionStatus, DeviceId, DeviceInstance, GroupAddress, GroupRange, GroupRangeId,
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

fn state_with_movable_lines(address: Option<IndividualAddress>) -> knx_server::AppState {
    let state = if address.is_some() {
        state_with_one_installation_and_device()
    } else {
        state_with_one_installation()
    };
    let mut guard = state.project.lock().unwrap();
    let project = guard.as_mut().unwrap();
    let line = |id: u32, number: u8, devices: Vec<DeviceId>| Line {
        id: LineId(id),
        source: SourceRef {
            path: format!("line-{id}"),
            ets_id: format!("line-{id}"),
        },
        name: format!("Line {id}"),
        address: number,
        medium_ref: "TP".into(),
        domain_address: None,
        domain_address_is_checked: None,
        ip_routing_multicast_address: None,
        multicast_ttl: None,
        completion: CompletionStatus::FinishedDesign,
        devices,
    };
    project.installations[0].topology = Topology {
        areas: vec![
            Area {
                id: AreaId(10),
                source: SourceRef {
                    path: "area-10".into(),
                    ets_id: "area-10".into(),
                },
                name: "Source".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![LineId(11), LineId(12)],
            },
            Area {
                id: AreaId(20),
                source: SourceRef {
                    path: "area-20".into(),
                    ets_id: "area-20".into(),
                },
                name: "Target".into(),
                address: 2,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![LineId(21)],
            },
        ],
        lines: vec![
            line(11, 1, vec![]),
            line(
                12,
                2,
                if address.is_some() {
                    vec![DeviceId(1)]
                } else {
                    vec![]
                },
            ),
            line(21, 1, vec![]),
        ],
        unassigned: vec![],
    };
    if let Some(address) = address {
        project.devices.get_mut(DeviceId(1)).unwrap().address = Some(address);
    }
    drop(guard);
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
async fn renaming_area_and_line_via_properties_routes_preserves_addresses_and_undoes() {
    let app = knx_server::app(Arc::new(state_with_one_installation()), None);
    let area = app
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
    assert_eq!(area.status(), StatusCode::OK);
    let area_id = body_json(area).await["installations"][0]["topology"][0]["id"]
        .as_u64()
        .unwrap();
    let line = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/lines")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "areaId": area_id, "name": "Line 1", "address": 2, "mediumRef": "MT-0" })
                        .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(line.status(), StatusCode::OK);
    let line_id = body_json(line).await["installations"][0]["topology"][0]["lines"][0]["id"]
        .as_u64()
        .unwrap();

    let area_renamed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/areas/{area_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "North" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(area_renamed.status(), StatusCode::OK);
    let tree = body_json(area_renamed).await;
    assert_eq!(tree["installations"][0]["topology"][0]["name"], "North");
    assert_eq!(tree["installations"][0]["topology"][0]["address"], 1);

    let line_renamed = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/lines/{line_id}"))
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "Main" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(line_renamed.status(), StatusCode::OK);
    let tree = body_json(line_renamed).await;
    assert_eq!(tree["installations"][0]["topology"][0]["name"], "North");
    assert_eq!(
        tree["installations"][0]["topology"][0]["lines"][0]["name"],
        "Main"
    );
    assert_eq!(
        tree["installations"][0]["topology"][0]["lines"][0]["address"],
        2
    );

    for (area_name, line_name) in [("North", "Line 1"), ("Area 1", "Line 1")] {
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
        assert_eq!(tree["installations"][0]["topology"][0]["name"], area_name);
        assert_eq!(
            tree["installations"][0]["topology"][0]["lines"][0]["name"],
            line_name
        );
    }
    for (area_name, line_name) in [("North", "Line 1"), ("North", "Main")] {
        let redo = app
            .clone()
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
        assert_eq!(tree["installations"][0]["topology"][0]["name"], area_name);
        assert_eq!(
            tree["installations"][0]["topology"][0]["lines"][0]["name"],
            line_name
        );
    }
    let unknown = app
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri("/api/lines/999")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "name": "No line" }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unknown.status(), StatusCode::BAD_REQUEST);
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
async fn moving_a_line_between_areas_preserves_order_and_rejects_duplicate_addresses() {
    let state = Arc::new(state_with_movable_lines(None));
    let app = knx_server::app(state.clone(), None);
    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-line-to-area")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "id": 12, "areaId": 20 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let tree = body_json(moved).await;
    let areas = tree["installations"][0]["topology"].as_array().unwrap();
    assert_eq!(
        areas[0]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line["id"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![11]
    );
    assert_eq!(
        areas[1]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line["id"].as_u64().unwrap())
            .collect::<Vec<_>>(),
        vec![21, 12]
    );
    assert_eq!(areas[1]["lines"][1]["address"], 2);

    let undone = app
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
    assert_eq!(undone.status(), StatusCode::OK);
    assert_eq!(
        body_json(undone).await["installations"][0]["topology"][0]["lines"][1]["id"],
        12
    );
    let redone = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/redo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(redone.status(), StatusCode::OK);
    assert_eq!(
        body_json(redone).await["installations"][0]["topology"][1]["lines"][1]["id"],
        12
    );

    let unchanged = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .topology
        .clone();
    for payload in [
        json!({ "id": 11, "areaId": 20 }), // line 21 already owns this address in area 20
        json!({ "id": 12, "areaId": 99 }),
        json!({ "id": 99, "areaId": 20 }),
        json!({ "id": 12 }),
    ] {
        let refused = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/move-line-to-area")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .installations[0]
                .topology,
            unchanged
        );
    }
}

#[tokio::test]
async fn deleting_structure_over_http_undoes_to_the_original_area_and_line_order() {
    let state = Arc::new(state_with_movable_lines(None));
    {
        let mut guard = state.project.lock().unwrap();
        let topology = &mut guard.as_mut().unwrap().installations[0].topology;
        let mut middle = topology.areas[0].clone();
        middle.id = AreaId(15);
        middle.address = 3;
        middle.name = "Middle".into();
        middle.lines.clear();
        topology.areas.insert(1, middle);
        let mut last = topology.lines[1].clone();
        last.id = LineId(13);
        last.address = 3;
        last.name = "Line 13".into();
        topology.lines.insert(2, last);
        topology.areas[0].lines.push(LineId(13));
    }
    let app = knx_server::app(state, None);
    let line_ids = |tree: &Value| {
        tree["installations"][0]["topology"][0]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .map(|line| line["id"].as_u64().unwrap())
            .collect::<Vec<_>>()
    };
    let area_ids = |tree: &Value| {
        tree["installations"][0]["topology"]
            .as_array()
            .unwrap()
            .iter()
            .map(|area| area["id"].as_u64().unwrap())
            .collect::<Vec<_>>()
    };
    let deleted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/lines/12")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::OK);
    assert_eq!(line_ids(&body_json(deleted).await), [11, 13]);
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
    assert_eq!(line_ids(&body_json(undo).await), [11, 12, 13]);
    let redo = app
        .clone()
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
    assert_eq!(line_ids(&body_json(redo).await), [11, 13]);
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
    assert_eq!(line_ids(&body_json(undo).await), [11, 12, 13]);

    let deleted = app
        .clone()
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/areas/15")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(deleted.status(), StatusCode::OK);
    assert_eq!(area_ids(&body_json(deleted).await), [10, 20]);
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
    assert_eq!(area_ids(&tree), [10, 15, 20]);
    assert_eq!(line_ids(&tree), [11, 12, 13]);
}

#[tokio::test]
async fn ambiguous_imported_structure_ids_are_refused_by_http_without_mutation() {
    let state = Arc::new(state_with_movable_lines(None));
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let mut later = project.installations[0].clone();
        later.id = InstallationId(2);
        project.installations.push(later);
    }
    let before = format!("{:#?}", state.project.lock().unwrap().as_ref().unwrap());
    let app = knx_server::app(state.clone(), None);
    for (method, uri, payload) in [
        ("PATCH", "/api/areas/10", json!({ "name": "Wrong area" })),
        ("PATCH", "/api/lines/11", json!({ "name": "Wrong line" })),
        ("DELETE", "/api/lines/11", Value::Null),
        (
            "POST",
            "/api/move-line-to-area",
            json!({ "id": 11, "areaId": 20 }),
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(if payload.is_null() {
                        Body::empty()
                    } else {
                        Body::from(payload.to_string())
                    })
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{method} {uri}");
        assert_eq!(
            format!("{:#?}", state.project.lock().unwrap().as_ref().unwrap()),
            before
        );
    }
}

#[tokio::test]
async fn moving_an_addressed_line_refuses_a_new_mismatch_but_undoes_an_imported_one() {
    let original = IndividualAddress::new(1, 2, 9).unwrap();
    let repaired = IndividualAddress::new(2, 2, 9).unwrap();
    let state = Arc::new(state_with_movable_lines(Some(original)));
    let app = knx_server::app(state.clone(), None);
    let request = || {
        Request::builder()
            .method("POST")
            .uri("/api/move-line-to-area")
            .header("content-type", "application/json")
            .body(Body::from(json!({ "id": 12, "areaId": 20 }).to_string()))
            .unwrap()
    };
    let rejected = app.clone().oneshot(request()).await.unwrap();
    assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .installations[0]
            .topology
            .areas[0]
            .lines,
        vec![LineId(11), LineId(12)]
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
    // Reproduce an imported mismatch whose raw address already matches the
    // desired destination. Reparenting repairs its placement, not its bytes.
    state
        .project
        .lock()
        .unwrap()
        .as_mut()
        .unwrap()
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .address = Some(repaired);
    let moved = app.clone().oneshot(request()).await.unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    assert_eq!(
        body_json(moved).await["installations"][0]["topology"][1]["lines"][1]["devices"][0]
            ["address"],
        "2.2.9"
    );
    let undone = app
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
    assert_eq!(undone.status(), StatusCode::OK);
    assert_eq!(
        body_json(undone).await["installations"][0]["topology"][0]["lines"][1]["devices"][0]
            ["address"],
        "2.2.9"
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
        Some(repaired)
    );
}

#[tokio::test]
async fn moving_a_group_range_repairs_a_misplaced_import_with_lossless_undo() {
    let state = Arc::new(state_with_one_installation());
    {
        let mut guard = state.project.lock().unwrap();
        let installation = &mut guard.as_mut().unwrap().installations[0];
        let range =
            |id: u32, start: u16, end: u16, parent: Option<u32>, children: Vec<u32>| GroupRange {
                id: GroupRangeId(id),
                source: SourceRef {
                    path: format!("range-{id}"),
                    ets_id: format!("range-{id}"),
                },
                name: format!("Range {id}"),
                start: GroupAddress::from_raw(start),
                end: GroupAddress::from_raw(end),
                parent: parent.map(GroupRangeId),
                children: children.into_iter().map(GroupRangeId).collect(),
            };
        installation.group_ranges = vec![
            range(1, 0, 2047, None, vec![2, 3, 4]),
            range(2, 0, 255, Some(1), vec![]),
            range(3, 2560, 2815, Some(1), vec![]),
            range(4, 512, 767, Some(1), vec![]),
            range(5, 2048, 4095, None, vec![]),
        ];
    }
    let app = knx_server::app(state.clone(), None);
    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-group-range")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "id": 3, "parentId": 5 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let tree = body_json(moved).await;
    assert_eq!(tree["installations"][0]["group_ranges"][2]["parent"], 5);
    {
        let guard = state.project.lock().unwrap();
        let ranges = &guard.as_ref().unwrap().installations[0].group_ranges;
        assert_eq!(ranges[0].children, vec![GroupRangeId(2), GroupRangeId(4)]);
        assert_eq!(ranges[4].children, vec![GroupRangeId(3)]);
    }
    let undone = app
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
    assert_eq!(undone.status(), StatusCode::OK);
    assert_eq!(
        body_json(undone).await["installations"][0]["group_ranges"][2]["parent"],
        1
    );
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .installations[0]
            .group_ranges[0]
            .children,
        vec![GroupRangeId(2), GroupRangeId(3), GroupRangeId(4)]
    );
    let redone = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/redo")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(redone.status(), StatusCode::OK);
    assert_eq!(
        body_json(redone).await["installations"][0]["group_ranges"][2]["parent"],
        5
    );

    let unchanged = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .group_ranges
        .clone();
    for payload in [
        json!({ "id": 1, "parentId": 2 }),
        json!({ "id": 3, "parentId": 2 }),
        json!({ "id": 3, "parentId": null }),
        json!({ "id": 3, "parentId": 99 }),
        json!({ "id": 3 }),
    ] {
        let refused = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/move-group-range")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(refused.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .installations[0]
                .group_ranges,
            unchanged
        );
    }
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
async fn moving_a_building_part_preserves_child_order_and_requires_an_explicit_parent() {
    let state = Arc::new(state_with_one_installation());
    {
        let mut guard = state.project.lock().unwrap();
        let installation = &mut guard.as_mut().unwrap().installations[0];
        let part = |id: u32, parent: Option<u32>, children: Vec<u32>| BuildingPart {
            id: BuildingPartId(id),
            source: SourceRef {
                path: format!("fixture-{id}"),
                ets_id: format!("fixture-{id}"),
            },
            name: format!("Part {id}"),
            number: None,
            kind: BuildingPartType::Building,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: children.into_iter().map(BuildingPartId).collect(),
            devices: vec![],
            parent: parent.map(BuildingPartId),
        };
        installation.buildings = vec![
            part(1, None, vec![2, 3, 4]),
            part(2, Some(1), vec![]),
            part(3, Some(1), vec![]),
            part(4, Some(1), vec![]),
            part(5, None, vec![]),
        ];
    }
    let app = knx_server::app(state.clone(), None);
    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-building-part")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "id": 3, "parentId": 5 }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(moved.status(), StatusCode::OK);
    let tree = body_json(moved).await;
    let children: Vec<_> = tree["installations"][0]["buildings"][0]["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["id"].as_u64().unwrap())
        .collect();
    assert_eq!(children, vec![2, 4]);
    assert_eq!(
        tree["installations"][0]["buildings"][1]["children"][0]["id"],
        3
    );

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
    let children: Vec<_> = tree["installations"][0]["buildings"][0]["children"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["id"].as_u64().unwrap())
        .collect();
    assert_eq!(children, vec![2, 3, 4]);

    let redo = app
        .clone()
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
    assert_eq!(
        body_json(redo).await["installations"][0]["buildings"][1]["children"][0]["id"],
        3
    );

    let to_root = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/move-building-part")
                .header("content-type", "application/json")
                .body(Body::from(json!({ "id": 3, "parentId": null }).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(to_root.status(), StatusCode::OK);
    let tree = body_json(to_root).await;
    let roots: Vec<_> = tree["installations"][0]["buildings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|part| part["id"].as_u64().unwrap())
        .collect();
    assert_eq!(roots, vec![1, 3, 5]);
    let undo_root = app
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
    assert_eq!(undo_root.status(), StatusCode::OK);
    assert_eq!(
        body_json(undo_root).await["installations"][0]["buildings"][1]["children"][0]["id"],
        3
    );

    let unchanged = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .buildings
        .clone();
    for payload in [
        json!({ "id": 3, "parentId": 99 }),
        json!({ "id": 1, "parentId": 4 }),
        json!({ "id": 3 }),
    ] {
        let rejected = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/move-building-part")
                    .header("content-type", "application/json")
                    .body(Body::from(payload.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(rejected.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .installations[0]
                .buildings,
            unchanged
        );
    }
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
            declared_dpt: Default::default(),
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
            declared_dpt: Default::default(),
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
