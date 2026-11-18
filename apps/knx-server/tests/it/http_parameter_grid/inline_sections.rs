//! `inline-section-flatten-v1` over the real in-process HTTP parameter routes.

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
const INLINE: &str =
    include_str!("../../../../../crates/knx-productdb/tests/fixtures/passive-inline-section.xml");

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
    let state = knx_server::AppState {
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
async fn an_inline_blocks_field_and_heading_arrive_in_the_outer_group() {
    let dto = panel(INLINE).await;
    let section = device_section(&dto);
    // Both fields carry the outer block's group key.
    for field in section["fields"].as_array().unwrap() {
        assert_eq!(field["group"]["key"], json!("PB-OUTER"), "{field}");
    }
    assert_eq!(section["fields"].as_array().unwrap().len(), 2);
    // The inline block is not an area: exactly one group layout exists.
    let layouts = section["groupLayouts"].as_array().unwrap();
    assert_eq!(layouts.len(), 1, "{section}");
    assert_eq!(layouts[0]["groupKey"], json!("PB-OUTER"));
    // Its heading arrives as a passive label of kind inlineHeading in the
    // outer group, with no cell and no write target.
    let labels = section["passiveLabels"].as_array().expect("passiveLabels");
    let heading = labels
        .iter()
        .find(|label| label["kind"] == "inlineHeading")
        .expect("inline heading present");
    assert_eq!(heading["text"], json!("Fictional inline section"));
    assert!(heading["cell"].is_null());
    assert_eq!(heading["group"]["key"], json!("PB-OUTER"));
    assert!(heading.get("value").is_none() && heading.get("writeEtsId").is_none());
}

#[tokio::test]
async fn a_post_response_carries_the_same_inline_heading_as_the_get() {
    let (_dir, state) = state(INLINE);
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
        device_section(&get)["passiveLabels"],
        device_section(&post)["passiveLabels"]
    );
    assert_eq!(
        device_section(&get)["fields"].as_array().unwrap().len(),
        device_section(&post)["fields"].as_array().unwrap().len()
    );
}
