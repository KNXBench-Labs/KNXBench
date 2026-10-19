//! A `TypeNone` parameter is a display-only row: never editable, never written (KL §128).
//!
//! ETS's `TypeNone` (and legacy atomic type 0, which maps to the same kind)
//! declares a parameter without a value: a heading, a label or an empty
//! spacer. The panel lists it, but must not offer it as an input, and a
//! write to it is refused and changes nothing. It is not a manufacturer
//! restriction either, so it adds no access warning.

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

/// `R-HEAD`: a `TypeNone` heading that even claims `ReadWrite`.
/// `R-SPACER`: a `TypeNone` spacer with empty text. `R-NUM`: an ordinary
/// writable number next to them.
const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-None" Name="none"><TypeNone /></ParameterType>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-HEAD" Name="head" Text="Timing" ParameterType="PT-None" Access="ReadWrite" Value="" />
  <Parameter Id="P-SPACER" Name="d_space" Text="" ParameterType="PT-None" Value="" />
  <Parameter Id="P-NUM" Name="delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="4" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="R-HEAD" RefId="P-HEAD" />
  <ParameterRef Id="R-SPACER" RefId="P-SPACER" />
  <ParameterRef Id="R-NUM" RefId="P-NUM" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="R-HEAD" />
  <ParameterRefRef RefId="R-SPACER" />
  <ParameterRefRef RefId="R-NUM" />
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

fn field<'a>(dto: &'a Value, ets_id: &str) -> &'a Value {
    dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["fields"].as_array().unwrap())
        .find(|f| f["etsId"] == ets_id)
        .unwrap_or_else(|| panic!("{ets_id} is listed"))
}

#[tokio::test]
async fn type_none_rows_are_listed_display_only_and_add_no_warning() {
    let (_dir, products) = products();
    let app = knx_server::app(Arc::new(state(products)), None);
    let dto = get_panel(app).await;

    for id in ["R-HEAD", "R-SPACER"] {
        let f = field(&dto, id);
        assert_eq!(f["kind"], "None", "{id}");
        assert_eq!(f["editable"], false, "{id}: {f}");
        assert!(f["writeEtsId"].is_null(), "{id}: {f}");
    }
    assert_eq!(field(&dto, "R-HEAD")["text"], "Timing");
    assert_eq!(
        field(&dto, "R-HEAD")["access"],
        "ReadWrite",
        "access is still shown"
    );
    let num = field(&dto, "R-NUM");
    assert_eq!(num["editable"], true, "{num}");
    assert_eq!(num["writeEtsId"], "R-NUM");

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(
        diagnostics
            .iter()
            .all(|d| !d["detail"].as_str().unwrap_or("").contains("R-HEAD")
                && !d["detail"].as_str().unwrap_or("").contains("R-SPACER")),
        "a display-only row is not a refusal: {diagnostics:?}"
    );
}

#[tokio::test]
async fn a_write_to_a_type_none_row_is_refused_and_changes_nothing() {
    let (_dir, products) = products();
    let state = Arc::new(state(products));
    let app = knx_server::app(Arc::clone(&state), None);
    let before = state.project.lock().unwrap().clone();

    for id in ["R-HEAD", "R-SPACER"] {
        for raw in ["", "x", "1"] {
            let (status, body) = post(app.clone(), id, raw).await;
            assert_eq!(status, StatusCode::BAD_REQUEST, "{id}={raw:?}: {body}");
        }
    }
    assert_eq!(
        *state.project.lock().unwrap(),
        before,
        "no refused write reached the project"
    );

    let (status, _) = post(app.clone(), "R-NUM", "9").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&get_panel(app).await, "R-NUM")["value"], "9");
}
