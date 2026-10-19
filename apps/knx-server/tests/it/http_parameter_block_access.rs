//! Block `Access` decides write authority and separators reach the panel (ADR-0139).
//!
//! A field below a `ParameterBlock` declaring `Read` or `None` is read-only
//! even when its own `Access` is `ReadWrite`; a write to it is refused with
//! 400 and changes nothing. A field also shown in an open block stays
//! writable. Activated separators are listed with their position among the
//! fields, group, hint and both access levels; a block holding only text
//! still forms its group. Fictional program, no vendor data.

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
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-OPEN" Name="o" Text="Open" ParameterType="PT-Num" Value="1" />
  <Parameter Id="P-BOTH" Name="b" Text="Both" ParameterType="PT-Num" Value="2" />
  <Parameter Id="P-READ-BLOCK" Name="r" Text="In a read block" ParameterType="PT-Num" Value="3" />
  <Parameter Id="P-HIDDEN-BLOCK" Name="h" Text="In a hidden block" ParameterType="PT-Num" Value="4" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="R-OPEN" RefId="P-OPEN" />
  <ParameterRef Id="R-BOTH" RefId="P-BOTH" />
  <ParameterRef Id="R-READ-BLOCK" RefId="P-READ-BLOCK" Access="ReadWrite" />
  <ParameterRef Id="R-HIDDEN-BLOCK" RefId="P-HIDDEN-BLOCK" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterBlock Id="PB-OPEN" Text="Open page">
    <ParameterSeparator Id="PS-HEAD" Text="Heading" UIHint="Headline" />
    <ParameterRefRef RefId="R-OPEN" />
    <ParameterSeparator Id="PS-RULE" UIHint="HorizontalRuler" />
    <ParameterRefRef RefId="R-BOTH" />
  </ParameterBlock>
  <ParameterBlock Id="PB-READ" Text="Read page" Access="Read">
    <ParameterRefRef RefId="R-READ-BLOCK" />
    <ParameterSeparator Id="PS-ERR" Text="Invalid combination" UIHint="Error" Access="None" />
  </ParameterBlock>
  <ParameterBlock Id="PB-HIDDEN" Text="Hidden page" Access="None">
    <ParameterRefRef RefId="R-HIDDEN-BLOCK" />
    <ParameterRefRef RefId="R-BOTH" />
  </ParameterBlock>
  <ParameterBlock Id="PB-INFO" Text="About">
    <ParameterSeparator Id="PS-INFO" Text="Firmware notes" UIHint="Information" />
  </ParameterBlock>
</Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn products() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", PROGRAM.as_bytes()).unwrap();
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
        product_ref: "M-1_P-1".into(),
        program_ref: "H-1_HP-1".into(),
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

async fn post(app: axum::Router, ets_id: &str, raw: &str) -> StatusCode {
    let request = Request::builder()
        .method("POST")
        .uri("/api/device/1/parameters")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({ "etsId": ets_id, "raw": raw }).to_string(),
        ))
        .unwrap();
    call(app, request).await.0
}

fn field<'a>(dto: &'a Value, ets_id: &str) -> &'a Value {
    dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["fields"].as_array().unwrap())
        .find(|f| f["etsId"] == ets_id)
        .unwrap_or_else(|| panic!("{ets_id} is listed"))
}

fn diagnostic<'a>(dto: &'a Value, kind: &str) -> Option<&'a Value> {
    dto["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["kind"] == kind)
}

#[tokio::test]
async fn an_enclosing_block_decides_write_authority_and_is_reported() {
    let (_dir, products) = products();
    let app = knx_server::app(Arc::new(state(products)), None);
    let dto = get_panel(app).await;

    let fields: Vec<&str> = dto["sections"][0]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| f["etsId"].as_str().unwrap())
        .collect();
    assert_eq!(
        fields,
        ["R-OPEN", "R-BOTH", "R-READ-BLOCK", "R-HIDDEN-BLOCK"]
    );
    for (id, editable, access, block_access) in [
        ("R-OPEN", true, Value::Null, Value::Null),
        // Shown in the open page too: writable there, so writable.
        ("R-BOTH", true, Value::Null, Value::Null),
        // Its own `ReadWrite` does not re-open a `Read` block.
        ("R-READ-BLOCK", false, json!("ReadWrite"), json!("Read")),
        ("R-HIDDEN-BLOCK", false, Value::Null, json!("None")),
    ] {
        let f = field(&dto, id);
        assert_eq!(f["editable"], editable, "{id}");
        assert_eq!(f["writeEtsId"].is_null(), !editable, "{id}");
        assert_eq!(f["access"], access, "{id}: its own access stays its own");
        assert_eq!(f["blockAccess"], block_access, "{id}");
    }
    let restricted =
        diagnostic(&dto, "parameterBlockAccessRestricted").expect("block access warning");
    assert_eq!(restricted["severity"], "warning");
    assert_eq!(
        restricted["detail"],
        "2 field(s), an enclosing ParameterBlock's Access is not ReadWrite: R-HIDDEN-BLOCK, R-READ-BLOCK"
    );
    assert!(diagnostic(&dto, "parameterAccessReadOnly").is_none());
}

#[tokio::test]
async fn writes_below_a_restricting_block_are_400_and_change_nothing() {
    let (_dir, products) = products();
    let state = Arc::new(state(products));
    let app = knx_server::app(Arc::clone(&state), None);
    for id in ["R-READ-BLOCK", "R-HIDDEN-BLOCK"] {
        assert_eq!(
            post(app.clone(), id, "9").await,
            StatusCode::BAD_REQUEST,
            "{id}"
        );
    }
    let stored = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .parameters
        .len();
    assert_eq!(stored, 0, "no refused write reached the project");
    assert_eq!(post(app.clone(), "R-OPEN", "9").await, StatusCode::OK);
    assert_eq!(post(app.clone(), "R-BOTH", "9").await, StatusCode::OK);
    let dto = get_panel(app).await;
    assert_eq!(field(&dto, "R-BOTH")["value"], "9");
}

#[tokio::test]
async fn separators_are_listed_between_the_fields_with_their_declarations() {
    let (_dir, products) = products();
    let app = knx_server::app(Arc::new(state(products)), None);
    let dto = get_panel(app).await;
    assert_eq!(dto["sections"].as_array().unwrap().len(), 1);
    assert_eq!(
        dto["sections"][0]["separators"],
        json!([
            {
                "position": 0, "group": { "key": "PB-OPEN", "label": "Open page" },
                "elementId": "PS-HEAD", "text": "Heading", "uiHint": "Headline",
                "access": null, "blockAccess": null
            },
            {
                "position": 1, "group": { "key": "PB-OPEN", "label": "Open page" },
                "elementId": "PS-RULE", "text": null, "uiHint": "HorizontalRuler",
                "access": null, "blockAccess": null
            },
            {
                "position": 3, "group": { "key": "PB-READ", "label": "Read page" },
                "elementId": "PS-ERR", "text": "Invalid combination", "uiHint": "Error",
                "access": "None", "blockAccess": "Read"
            },
            {
                "position": 4, "group": { "key": "PB-INFO", "label": "About" },
                "elementId": "PS-INFO", "text": "Firmware notes", "uiHint": "Information",
                "access": null, "blockAccess": null
            }
        ])
    );
    // The fields keep their groups; the text-only page has none of them.
    assert_eq!(field(&dto, "R-OPEN")["group"]["key"], "PB-OPEN");
    assert_eq!(field(&dto, "R-READ-BLOCK")["group"]["key"], "PB-READ");
}
