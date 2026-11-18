//! `nested-block-cell-passive-v1` over the real in-process HTTP parameter routes.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const HARDWARE: &str =
    include_str!("../../../../../crates/knx-productdb/tests/fixtures/passive-button-hardware.xml");
const NESTED: &str =
    include_str!("../../../../../crates/knx-productdb/tests/fixtures/passive-nested-cell.xml");

const PARAMETERS: &str = "/api/device/1/parameters";

fn state(program: &str) -> (tempfile::TempDir, Arc<knx_server::AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-FIX/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-FIX/A-FIX.xml", program.as_bytes()).unwrap();
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "device-1".into(),
            ets_id: "device-1".into(),
        },
        name: "Device 1".into(),
        description: None,
        address: None,
        product_ref: "M-FIX_P-1".into(),
        program_ref: "H-FIX_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
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
    project.ids = knx_core::project::IdAllocators::from_counts(1, 0, 0, 0, 0, 0, 0, 0, 0);
    let mut state = knx_server::AppState {
        product_db: Some(
            knx_server::CatalogueOwner::adopt_opened(products, std::path::PathBuf::new()).unwrap(),
        ),
        ..Default::default()
    };
    *state.clean_project.lock().unwrap() = Some(project.clone());
    *state.project.lock().unwrap() = Some(project);
    (dir, Arc::new(state))
}

async fn panel(program: &str) -> Value {
    let (_dir, state) = state(program);
    let request = Request::builder()
        .method("GET")
        .uri(PARAMETERS)
        .body(Body::empty())
        .unwrap();
    let response = knx_server::app(state, None).oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn device_section(dto: &Value) -> &Value {
    let sections = dto["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 1, "{dto}");
    assert!(sections[0]["scope"].is_null());
    &sections[0]
}

#[tokio::test]
async fn a_nested_block_cell_arrives_with_caption_cell_group_and_no_write_target() {
    let dto = panel(NESTED).await;
    let section = device_section(&dto);
    let cells = section["nestedCells"]
        .as_array()
        .expect("nestedCells present");
    let by_id = |id: &str| {
        cells
            .iter()
            .find(|cell| cell["elementId"] == id)
            .unwrap()
            .clone()
    };
    let nested = by_id("PB-NESTED");
    assert_eq!(nested["text"], json!("Fictional nested caption"));
    assert_eq!(nested["cell"], json!({ "row": 1, "column": 2 }));
    assert_eq!(nested["group"]["key"], json!("PB-OUTER"));
    // Display only: no field identity, no writeEtsId, no value is attached.
    for key in ["etsId", "writeEtsId", "value", "kind"] {
        assert!(
            nested.get(key).is_none(),
            "nested cell must not carry {key}"
        );
    }
    // A nested block without displayable text still reports its placement, with no caption.
    let silent = by_id("PB-SILENT");
    assert!(silent.get("text").is_none() || silent["text"].is_null());
    assert_eq!(silent["cell"], json!({ "row": 1, "column": 1 }));
    // The outer grid's layout still lists the nested block as unshown source
    // content; the web layer marks it shown via the nested cell.
    let layout = section["groupLayouts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|layout| layout["groupKey"] == "PB-OUTER")
        .expect("outer grid layout");
    let unshown: Vec<&Value> = layout["issues"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|issue| issue["kind"] == "unshownContent" && issue["element"] == "ParameterBlock")
        .collect();
    assert_eq!(unshown.len(), 2, "{layout}");
}

#[tokio::test]
async fn a_post_response_carries_the_same_nested_cells_as_the_get() {
    let (_dir, state) = state(NESTED);
    let get = knx_server::app(state.clone(), None)
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(PARAMETERS)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let get: Value = serde_json::from_slice(
        &axum::body::to_bytes(get.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let post = knx_server::app(state, None)
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(PARAMETERS)
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "etsId": "PR-EDIT", "raw": "42" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(post.status(), StatusCode::OK);
    let post: Value = serde_json::from_slice(
        &axum::body::to_bytes(post.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        device_section(&get)["nestedCells"],
        device_section(&post)["nestedCells"]
    );
}
