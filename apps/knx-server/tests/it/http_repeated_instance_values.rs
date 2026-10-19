//! The parameter panel exposes stored instance evidence without write authority.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-TEST">
<Hardware><Hardware Id="H-TEST" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-TEST_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-TEST_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-TEST" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn products() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-TEST/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(
        &conn,
        "M-TEST/A.xml",
        knx_testsupport::REPEATED_INSTANCE_PROGRAM_XML.as_bytes(),
    )
    .unwrap();
    (dir, conn)
}

#[allow(clippy::field_reassign_with_default)]
fn state(products: knx_productdb::Connection) -> knx_server::AppState {
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
        product_ref: "M-TEST_P-1".into(),
        program_ref: "H-TEST_HP-1".into(),
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
        parameters: [2u32, 4]
            .into_iter()
            .map(|k| knx_core::ParameterInstance {
                id: knx_core::ParameterInstanceId(k),
                device: DeviceId(1),
                source: SourceRef {
                    path: "synthetic".into(),
                    ets_id: format!("A-TEST_MD-71_M-93_MI-{k}_P-83_R-61"),
                },
                raw: if k == 2 { "17" } else { "23" }.into(),
            })
            .collect(),
    });
    for k in [2u32, 4] {
        project
            .devices
            .insert_module_instance(knx_core::ModuleInstance {
                id: knx_core::ModuleInstanceId(k),
                device: DeviceId(1),
                source: SourceRef {
                    path: "synthetic".into(),
                    ets_id: "MD-71_M-93".into(),
                },
                instance_ets_id: format!("MD-71_M-93_MI-{k}"),
                repeat_index: format!("71x{k}"),
                arguments: vec![],
            });
    }
    project.ids = knx_core::project::IdAllocators::from_counts(1, 0, 0, 0, 0, 0, 0, 0, 0);
    let mut state = knx_server::AppState::default();
    state.product_db = Some(Mutex::new(products));
    *state.project.lock().unwrap() = Some(project);
    state
}

async fn call(app: axum::Router, request: Request<Body>) -> (StatusCode, Value) {
    let response = app.oneshot(request).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn get_panel(app: axum::Router) -> Value {
    let request = Request::builder()
        .method("GET")
        .uri("/api/device/1/parameters")
        .body(Body::empty())
        .unwrap();
    let (status, dto) = call(app, request).await;
    assert_eq!(status, StatusCode::OK);
    dto
}

async fn post(app: axum::Router, ets_id: &str, raw: &str) -> (StatusCode, Value) {
    let request = Request::builder()
        .method("POST")
        .uri("/api/device/1/parameters")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "etsId": ets_id, "raw": raw }).to_string(),
        ))
        .unwrap();
    call(app, request).await
}

#[tokio::test]
async fn own_instance_values_are_inspectable_but_never_writable() {
    let (_dir, products) = products();
    let state = Arc::new(state(products));
    let app = knx_server::app(Arc::clone(&state), None);
    let before = state.project.lock().unwrap().clone();
    let panel = get_panel(app.clone()).await;
    assert_eq!(
        panel["instanceValues"],
        json!([
            {"etsId":"A-TEST_MD-71_M-93_MI-2_P-83_R-61", "raw":"17"},
            {"etsId":"A-TEST_MD-71_M-93_MI-4_P-83_R-61", "raw":"23"}
        ])
    );
    assert_eq!(panel["stale"], json!([]));
    for k in [2, 4] {
        let (status, _) = post(
            app.clone(),
            &format!("A-TEST_MD-71_M-93_MI-{k}_P-83_R-61"),
            "42",
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
    assert_eq!(*state.project.lock().unwrap(), before);
}
