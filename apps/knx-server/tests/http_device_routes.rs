use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Direction, GroupAddress,
    GroupAddressEntry, GroupAddressId, GroupLink, Installation, InstallationId, Language,
    Override, Project, ResolvedFlags, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

// The `product_db = None` line below is a deliberate, documented
// reassignment (see its own comment) — not an oversight clippy should
// fold into a `..Default::default()` struct literal. Same pattern
// `domain.rs`'s own `state_with_one_installation` test helper already
// carries this exact attribute for.
#[allow(clippy::field_reassign_with_default)]
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
    let mut state = knx_server::AppState::default();
    // `AppState::default()` runs the real `default_path()` lookup (Task
    // 3) — on a machine that already has a product database at e.g.
    // `~/.local/share/knx/products.sqlite`, `default_path()` would pick
    // it up here, making `creating_a_device_without_a_product_database_
    // is_a_400` depend on the environment. Force `None` explicitly.
    state.product_db = None;
    *state.project.lock().unwrap() = Some(project);
    state
}

const HARDWARE: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1"
  MaskVersion="MV-0701"><Static>
<ComObjectTable>
  <ComObject Id="A-1_O-1" Number="1" Text="Schalten" ObjectSize="1 Bit"
             DatapointType="DPST-1-1" WriteFlag="Enabled" />
</ComObjectTable>
<ComObjectRefs>
  <ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" />
</ComObjectRefs>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

const CATALOG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <ManufacturerData>
    <Manufacturer RefId="M-1">
      <Catalog>
        <CatalogSection Id="M-1_CG-1" Name="Actuators" Number="1" DefaultLanguage="de-DE">
          <CatalogItem Id="M-1_CI-1" Name="Schaltaktor" Number="ACT-1"
                       DefaultLanguage="de-DE"
                       ProductRefId="M-1_P-1"
                       Hardware2ProgramRefId="H-1_HP-1" />
        </CatalogSection>
      </Catalog>
    </Manufacturer>
  </ManufacturerData>
</KNX>"#;

fn temp_product_db() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::parse::hardware::ingest_hardware(&conn, "sha-h", "M-1/Hardware.xml", HARDWARE.as_bytes())
        .unwrap();
    knx_productdb::parse::program::ingest_program(&conn, "sha-p", "M-1/A.xml", PROGRAM.as_bytes())
        .unwrap();
    knx_productdb::parse::catalog::ingest_catalog(&conn, "sha-c", "M-1/Catalog.xml", CATALOG.as_bytes())
        .unwrap();
    (dir, conn)
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

#[tokio::test]
async fn creating_a_device_from_a_catalog_item_seeds_its_com_objects_then_deletes() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    state.product_db = Some(Mutex::new(products));
    let app = knx_server::app(Arc::new(state), None);

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "M-1_CI-1", "name": "Actuator 1" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(create.status(), StatusCode::OK);
    let tree = body_json(create).await;
    let devices = tree["installations"][0]["unassigned"].as_array().unwrap();
    assert_eq!(devices.len(), 1);
    let device_id = devices[0]["id"].as_u64().unwrap();

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/device/{device_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_json = body_json(detail).await;
    assert_eq!(detail_json["com_objects"].as_array().unwrap().len(), 1);

    let delete = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri(format!("/api/devices/{device_id}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(delete.status(), StatusCode::OK);
    let tree = body_json(delete).await;
    assert!(tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn creating_a_device_without_a_product_database_is_a_400() {
    let state = state_with_one_installation();
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/devices")
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "catalogItemId": "anything", "name": "D" }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn deleting_a_device_with_a_linked_com_object_is_a_400() {
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
        group_addresses: vec![GroupAddressEntry {
            id: GroupAddressId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "GA".into(),
            address: GroupAddress::from_raw(1),
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
    project.devices.insert_com_object(knx_core::ComObjectInstance {
        id: knx_core::ComObjectInstanceId(1),
        source: SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        },
        device: DeviceId(1),
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        }],
        module_instance: None,
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    let app = knx_server::app(Arc::new(state), None);

    let response = app
        .oneshot(
            Request::builder()
                .method("DELETE")
                .uri("/api/devices/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn catalog_routes_list_the_manufacturer_and_item_just_ingested() {
    let mut state = state_with_one_installation();
    let (_dir, products) = temp_product_db();
    state.product_db = Some(Mutex::new(products));
    let app = knx_server::app(Arc::new(state), None);

    let manufacturers = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/catalog/manufacturers")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(manufacturers.status(), StatusCode::OK);
    let manufacturers = body_json(manufacturers).await;
    assert_eq!(manufacturers.as_array().unwrap().len(), 1);
    assert_eq!(manufacturers[0]["id"], "M-1");

    let items = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/catalog/items?manufacturer=M-1&search=schalt")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(items.status(), StatusCode::OK);
    let items = body_json(items).await;
    assert_eq!(items.as_array().unwrap().len(), 1);
    assert_eq!(items[0]["id"], "M-1_CI-1");
}
