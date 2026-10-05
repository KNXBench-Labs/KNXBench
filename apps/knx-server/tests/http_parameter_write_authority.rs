//! ADR-0080 at the HTTP surface: a field whose effective `Access` is not
//! `ReadWrite`, or that a `ParameterCalculation` names, is shown read-only,
//! a write to it is refused with 400 and changes nothing, and a program
//! without recorded write authority fails closed.

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

/// `R-OPEN`: Parameter `None`, ParameterRef `ReadWrite` -> writable.
/// `R-HIDDEN`: Parameter `None` -> read-only. `R-READ`: ParameterRef `Read`
/// -> read-only. `R-PLAIN`: no Access anywhere -> writable (no default
/// invented). `R-ODD`: an unknown token -> read-only. `R-L`/`R-R`: the two
/// sides of one `ParameterCalculation` -> both read-only.
const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-OPEN" Name="o" Text="Open" ParameterType="PT-Num" Access="None" Value="1" />
  <Parameter Id="P-HIDDEN" Name="h" Text="Hidden" ParameterType="PT-Num" Access="None" Value="2" />
  <Parameter Id="P-READ" Name="r" Text="Read" ParameterType="PT-Num" Value="3" />
  <Parameter Id="P-PLAIN" Name="p" Text="Plain" ParameterType="PT-Num" Value="4" />
  <Parameter Id="P-ODD" Name="d" Text="Odd" ParameterType="PT-Num" Access="Sometimes" Value="5" />
  <Parameter Id="P-L" Name="l" Text="Minutes" ParameterType="PT-Num" Value="6" />
  <Parameter Id="P-R" Name="rr" Text="Seconds" ParameterType="PT-Num" Value="7" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="R-OPEN" RefId="P-OPEN" Access="ReadWrite" />
  <ParameterRef Id="R-HIDDEN" RefId="P-HIDDEN" />
  <ParameterRef Id="R-READ" RefId="P-READ" Access="Read" />
  <ParameterRef Id="R-PLAIN" RefId="P-PLAIN" />
  <ParameterRef Id="R-ODD" RefId="P-ODD" />
  <ParameterRef Id="R-L" RefId="P-L" />
  <ParameterRef Id="R-R" RefId="P-R" />
</ParameterRefs>
<ParameterCalculations>
  <ParameterCalculation Id="PC-1" Name="c" Language="JavaScript" LRTransformationFunc="lr" RLTransformationFunc="rl">
    <LParameters><ParameterRefRef RefId="R-L" /></LParameters>
    <RParameters><ParameterRefRef RefId="R-R" /></RParameters>
  </ParameterCalculation>
</ParameterCalculations>
</Static>
<Dynamic>
  <ParameterRefRef RefId="R-OPEN" />
  <ParameterRefRef RefId="R-HIDDEN" />
  <ParameterRefRef RefId="R-READ" />
  <ParameterRefRef RefId="R-PLAIN" />
  <ParameterRefRef RefId="R-ODD" />
  <ParameterRefRef RefId="R-L" />
  <ParameterRefRef RefId="R-R" />
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
async fn access_and_calculations_decide_write_authority_and_every_field_stays_listed() {
    let (_dir, products) = products();
    let app = knx_server::app(Arc::new(state(products)), None);
    let dto = get_panel(app).await;

    for (id, editable, access) in [
        ("R-OPEN", true, json!("ReadWrite")),
        ("R-HIDDEN", false, json!("None")),
        ("R-READ", false, json!("Read")),
        ("R-PLAIN", true, Value::Null),
        ("R-ODD", false, json!("Sometimes")),
        ("R-L", false, Value::Null),
        ("R-R", false, Value::Null),
    ] {
        let f = field(&dto, id);
        assert_eq!(f["editable"], editable, "{id}");
        assert_eq!(f["writeEtsId"].is_null(), !editable, "{id}");
        assert_eq!(f["access"], access, "{id}: effective access is shown");
    }

    let access = diagnostic(&dto, "parameterAccessReadOnly").expect("access warning");
    assert_eq!(access["severity"], "warning");
    let detail = access["detail"].as_str().unwrap();
    for id in ["R-HIDDEN", "R-ODD", "R-READ"] {
        assert!(detail.contains(id), "{detail}");
    }
    assert!(detail.starts_with("3 field(s)"), "{detail}");
    let calculated = diagnostic(&dto, "manufacturerCalculation").expect("calculation warning");
    let detail = calculated["detail"].as_str().unwrap();
    assert!(detail.contains("R-L") && detail.contains("R-R"), "{detail}");
    assert!(diagnostic(&dto, "writeAuthorityUnavailable").is_none());
}

#[tokio::test]
async fn writes_to_refused_fields_are_400_and_change_nothing() {
    let (_dir, products) = products();
    let state = Arc::new(state(products));
    let app = knx_server::app(Arc::clone(&state), None);
    for id in ["R-HIDDEN", "R-READ", "R-ODD", "R-L", "R-R"] {
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

    // The fields the program does open stay writable.
    assert_eq!(post(app.clone(), "R-OPEN", "9").await, StatusCode::OK);
    assert_eq!(post(app.clone(), "R-PLAIN", "9").await, StatusCode::OK);
    let dto = get_panel(app).await;
    assert_eq!(field(&dto, "R-OPEN")["value"], "9");
    assert_eq!(field(&dto, "R-PLAIN")["value"], "9");
}

#[tokio::test]
async fn a_program_without_recorded_write_authority_is_read_only() {
    let (_dir, products) = products();
    products
        .execute(
            "UPDATE application_program SET write_authority_recorded = 0",
            [],
        )
        .unwrap();
    let app = knx_server::app(Arc::new(state(products)), None);
    let dto = get_panel(app.clone()).await;
    for id in ["R-OPEN", "R-PLAIN"] {
        assert_eq!(field(&dto, id)["editable"], false, "{id}");
    }
    let unavailable = diagnostic(&dto, "writeAuthorityUnavailable").expect("fail-closed warning");
    assert!(unavailable["detail"]
        .as_str()
        .unwrap()
        .starts_with("7 field(s)"));
    assert_eq!(post(app, "R-PLAIN", "9").await, StatusCode::BAD_REQUEST);
}
