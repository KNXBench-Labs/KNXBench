//! T18 slice 3 task 3 (design D20-D26): `GET`/`POST
//! /api/device/{id}/parameters`. Acceptance criteria numbers below match
//! the design doc's own numbered list (`docs/superpowers/specs/
//! 2026-09-11-parameter-editor-design.md`, "Acceptance criteria").

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, ParameterInstance, ParameterInstanceId, Project, SourceRef, Topology,
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

/// Two top-level parameters (`P-1` Number 0..=255 default 5, `P-2`
/// Restriction Off/On default "0") plus one `Module` (`MD-1`, one
/// instantiation `MOD-1_M-1`) declaring its own Number parameter
/// `MOD-1_P-1_R-1` — enough to cover both an unscoped and a module-scoped
/// section in the same program (AC1, AC5-AC9).
const WRITE_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
  <ParameterType Id="PT-Enum" Name="enum"><TypeRestriction Base="Value" SizeInBit="8">
    <Enumeration Id="PT-Enum_EN-0" Text="Off" Value="0" DisplayOrder="0" />
    <Enumeration Id="PT-Enum_EN-1" Text="On" Value="1" DisplayOrder="1" />
  </TypeRestriction></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
  <Parameter Id="P-2" Name="Mode" Text="Mode" ParameterType="PT-Enum" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="10" Tag="1" />
  <ParameterRef Id="P-2_R-1" RefId="P-2" DisplayOrder="20" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-1_R-1" />
  <ParameterRefRef RefId="P-2_R-1" />
  <Module Id="MOD-1_M-1" RefId="MD-1" />
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="module">
<Static>
<ParameterTypes>
  <ParameterType Id="MOD-1_PT-Num" Name="num"><TypeNumber maxInclusive="10" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="MOD-1_P-1" Name="Channel" Text="Channel" ParameterType="MOD-1_PT-Num" Access="ReadWrite" Value="1" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="MOD-1_P-1_R-1" RefId="MOD-1_P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="MOD-1_P-1_R-1" />
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// Same shape as `WRITE_PROGRAM`'s module, instantiated twice
/// (`MOD-1_M-2`, `MOD-1_M-3`) — AC2 (stale) and AC3 (two sections, same
/// `ets_id` set).
const TWO_INSTANTIATION_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-Top" Name="Top" Text="Top" ParameterType="PT-Num" Access="ReadWrite" Value="1" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-Top_R-1" RefId="P-Top" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="P-Top_R-1" />
  <Module Id="MOD-1_M-2" RefId="MD-1" />
  <Module Id="MOD-1_M-3" RefId="MD-1" />
</Dynamic>
<ModuleDefs><ModuleDef Id="MD-1" Name="module">
<Static>
<ParameterTypes>
  <ParameterType Id="MOD-1_PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="MOD-1_P-1" Name="Channel" Text="Channel" ParameterType="MOD-1_PT-Num" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="MOD-1_P-1_R-1" RefId="MOD-1_P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="MOD-1_P-1_R-1" />
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// The KV v2.5 demo shape verbatim (AC4): declared `ParameterRef`
/// `M-00FA_A-2504-10-C071_MD-2_P-1_R-1`, five `Module` instantiations
/// `M-2`..`M-6` of `ModuleDef` `M-00FA_A-2504-10-C071_MD-2`.
const KV_SHAPE_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static><ParameterRefs/></Static>
<Dynamic>
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-2" RefId="M-00FA_A-2504-10-C071_MD-2" />
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-3" RefId="M-00FA_A-2504-10-C071_MD-2" />
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-4" RefId="M-00FA_A-2504-10-C071_MD-2" />
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-5" RefId="M-00FA_A-2504-10-C071_MD-2" />
  <Module Id="M-00FA_A-2504-10-C071_MD-2_M-6" RefId="M-00FA_A-2504-10-C071_MD-2" />
</Dynamic>
<ModuleDefs><ModuleDef Id="M-00FA_A-2504-10-C071_MD-2" Name="module">
<Static>
<ParameterTypes>
  <ParameterType Id="M-00FA_A-2504-10-C071_MD-2_PT-1" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="M-00FA_A-2504-10-C071_MD-2_P-1" Name="Channel" Text="Channel" ParameterType="M-00FA_A-2504-10-C071_MD-2_PT-1" Access="ReadWrite" Value="0" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="M-00FA_A-2504-10-C071_MD-2_P-1_R-1" RefId="M-00FA_A-2504-10-C071_MD-2_P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="M-00FA_A-2504-10-C071_MD-2_P-1_R-1" />
</Dynamic>
</ModuleDef></ModuleDefs>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

/// One top-level Number parameter controlling a `choose` whose `when
/// test` does not parse (AC10: at least one diagnostic).
const DIAGNOSTIC_PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-1" Name="Delay" Text="Delay" ParameterType="PT-Num" Access="ReadWrite" Value="5" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="P-1_R-1" RefId="P-1" DisplayOrder="1" Tag="1" />
</ParameterRefs>
</Static>
<Dynamic>
  <choose ParamRefId="P-1_R-1">
    <when test="bogus"><ParameterRefRef RefId="P-1_R-1" /></when>
  </choose>
</Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn temp_product_db(program_xml: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    // `ingest_file` (not `parse::program::ingest_program` directly) is what
    // also runs the second, `Dynamic`-reading pass over an
    // `ApplicationProgram` file (`ingest.rs`'s own doc comment) -- skipping
    // it would leave `dynamic_node` empty and every `evaluate()` call inert.
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", program_xml.as_bytes()).unwrap();
    (dir, conn)
}

/// A project with one installation and one device (`DeviceId(1)`) whose
/// `program_ref` is `"H-1_HP-1"` (matching `temp_product_db`'s hardware
/// fixture), pre-seeded with `parameters`.
#[allow(clippy::field_reassign_with_default)]
fn state_with_device(
    products: knx_productdb::Connection,
    parameters: Vec<(&str, &str)>,
) -> knx_server::AppState {
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
        parameters: parameters
            .into_iter()
            .enumerate()
            .map(|(i, (ets_id, raw))| ParameterInstance {
                id: ParameterInstanceId(i as u32 + 1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: ets_id.into(),
                },
                raw: raw.into(),
            })
            .collect(),
    });
    let mut state = knx_server::AppState::default();
    state.product_db = Some(Mutex::new(products));
    *state.project.lock().unwrap() = Some(project);
    state
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get_panel(app: axum::Router, device_id: u32) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/device/{device_id}/parameters"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (status, body_json(response).await)
}

async fn post_panel(
    app: axum::Router,
    device_id: u32,
    ets_id: &str,
    raw: &str,
) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/device/{device_id}/parameters"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({ "etsId": ets_id, "raw": raw }).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    (status, body_json(response).await)
}

fn field<'a>(dto: &'a Value, ets_id: &str) -> Option<&'a Value> {
    dto["sections"].as_array().unwrap().iter().find_map(|s| {
        s["fields"]
            .as_array()
            .unwrap()
            .iter()
            .find(|f| f["etsId"] == ets_id)
    })
}

// AC1: a stored field and a defaulted field, correct `value`/`valueSource`.
#[tokio::test]
async fn get_returns_stored_and_defaulted_top_level_fields() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![("P-1_R-1", "7")]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(dto["programId"], "A-1");

    let p1 = field(&dto, "P-1_R-1").expect("P-1_R-1 present");
    assert_eq!(p1["value"], "7");
    assert_eq!(p1["valueSource"], "Stored");

    let p2 = field(&dto, "P-2_R-1").expect("P-2_R-1 present");
    assert_eq!(p2["value"], "0");
    assert_eq!(p2["valueSource"], "ProgramDefault");
}

// AC2: an undecomposable id and a regex-match-but-undeclared id both land
// in `stale`, not in any section; other valid stored values are
// unaffected.
#[tokio::test]
async fn undecomposable_and_unvalidated_stored_ids_are_reported_stale_not_dropped() {
    let (_dir, products) = temp_product_db(TWO_INSTANTIATION_PROGRAM);
    let state = Arc::new(state_with_device(
        products,
        vec![
            ("P-Top_R-1", "7"),
            ("totally-bogus-id", "x"),
            ("P-Top_R-1_M-99_MI-1_bogus", "y"),
        ],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let stale_ids: Vec<&str> = dto["stale"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["etsId"].as_str().unwrap())
        .collect();
    assert!(stale_ids.contains(&"totally-bogus-id"));
    assert!(stale_ids.contains(&"P-Top_R-1_M-99_MI-1_bogus"));
    assert_eq!(stale_ids.len(), 2);

    // Neither stale id appears in any section's fields.
    for section in dto["sections"].as_array().unwrap() {
        for f in section["fields"].as_array().unwrap() {
            let ets_id = f["etsId"].as_str().unwrap();
            assert_ne!(ets_id, "totally-bogus-id");
            assert_ne!(ets_id, "P-Top_R-1_M-99_MI-1_bogus");
        }
    }

    // The one genuinely valid stored value is untouched.
    let top = field(&dto, "P-Top_R-1").expect("P-Top_R-1 present");
    assert_eq!(top["value"], "7");
    assert_eq!(top["valueSource"], "Stored");
}

// AC3: a Module instantiated twice produces two sections with distinct
// `scope.moduleNode`, both containing the same `etsId` set.
#[tokio::test]
async fn a_module_instantiated_twice_produces_two_sections_with_the_same_ets_id_set() {
    let (_dir, products) = temp_product_db(TWO_INSTANTIATION_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);

    let module_sections: Vec<&Value> = dto["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|s| !s["scope"].is_null())
        .collect();
    assert_eq!(module_sections.len(), 2);

    let module_nodes: std::collections::HashSet<i64> = module_sections
        .iter()
        .map(|s| s["scope"]["moduleNode"].as_i64().unwrap())
        .collect();
    assert_eq!(
        module_nodes.len(),
        2,
        "distinct module_node per instantiation"
    );

    for s in &module_sections {
        let ids: Vec<&str> = s["fields"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["etsId"].as_str().unwrap())
            .collect();
        assert_eq!(ids, vec!["MOD-1_P-1_R-1"]);
        assert_eq!(s["fields"][0]["editable"], false);
    }
}

// AC4: the KV v2.5 demo shape verbatim — five sections, each showing its
// own stored value, `stale` empty.
#[tokio::test]
async fn kv_shape_stored_values_decompose_into_five_sections_each_showing_its_own_value() {
    let (_dir, products) = temp_product_db(KV_SHAPE_PROGRAM);
    let state = Arc::new(state_with_device(
        products,
        vec![
            ("M-00FA_A-2504-10-C071_MD-2_M-2_MI-1_P-1_R-1", "32"),
            ("M-00FA_A-2504-10-C071_MD-2_M-3_MI-1_P-1_R-1", "48"),
            ("M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1", "17"),
            ("M-00FA_A-2504-10-C071_MD-2_M-5_MI-1_P-1_R-1", "33"),
            ("M-00FA_A-2504-10-C071_MD-2_M-6_MI-1_P-1_R-1", "49"),
        ],
    ));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert!(dto["stale"].as_array().unwrap().is_empty());

    let sections = dto["sections"].as_array().unwrap();
    assert_eq!(sections.len(), 5);

    let mut values: Vec<String> = sections
        .iter()
        .map(|s| {
            let f = s["fields"]
                .as_array()
                .unwrap()
                .iter()
                .find(|f| f["etsId"] == "M-00FA_A-2504-10-C071_MD-2_P-1_R-1")
                .expect("declared field present in every section");
            assert_eq!(f["valueSource"], "Stored");
            f["value"].as_str().unwrap().to_string()
        })
        .collect();
    values.sort();
    assert_eq!(values, vec!["17", "32", "33", "48", "49"]);
}

// AC5: POST a valid top-level etsId/raw returns 200 and the same response
// shows the new value as "Stored" — no second request needed.
#[tokio::test]
async fn post_a_valid_top_level_value_is_reflected_in_the_same_response() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = post_panel(app, 1, "P-1_R-1", "42").await;
    assert_eq!(status, StatusCode::OK);
    let p1 = field(&dto, "P-1_R-1").unwrap();
    assert_eq!(p1["value"], "42");
    assert_eq!(p1["valueSource"], "Stored");
}

// AC6: out-of-range Number and non-member Restriction both 400, and
// leave the previously stored value unchanged.
#[tokio::test]
async fn post_out_of_bounds_number_and_non_member_restriction_are_rejected_unchanged() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![("P-1_R-1", "7")]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, _) = post_panel(app.clone(), 1, "P-1_R-1", "999").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = post_panel(app.clone(), 1, "P-2_R-1", "7").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "7");
    assert_eq!(field(&dto, "P-2_R-1").unwrap()["value"], "0");
}

// AC7: POST to a field currently `scope: Some(_)` returns 400 and changes
// nothing.
#[tokio::test]
async fn post_to_a_module_scoped_field_is_rejected() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, _) = post_panel(app.clone(), 1, "MOD-1_P-1_R-1", "3").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    let field = field(&dto, "MOD-1_P-1_R-1").unwrap();
    assert_eq!(field["valueSource"], "ProgramDefault");
}

// AC8: POST naming an etsId not declared by the program at all -> 400.
#[tokio::test]
async fn post_an_undeclared_ets_id_is_rejected() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, _) = post_panel(app, 1, "NOPE", "1").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

// AC9: write + undo + redo round-trips through Command::SetParameterValue
// / RestoreParameterValue — insert case (row absent before) and overwrite
// case (row present before) both covered.
#[tokio::test]
async fn write_undo_redo_round_trips_both_insert_and_overwrite() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![("P-1_R-1", "7")]));
    let app = knx_server::app(Arc::clone(&state), None);

    // Insert case: P-2_R-1 has no stored row yet.
    let (status, dto) = post_panel(app.clone(), 1, "P-2_R-1", "1").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-2_R-1").unwrap()["value"], "1");

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
    let (status, dto) = get_panel(app.clone(), 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        field(&dto, "P-2_R-1").unwrap()["valueSource"],
        "ProgramDefault"
    );
    assert!(!state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .parameters
        .iter()
        .any(|p| p.source.ets_id == "P-2_R-1"));

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
    let (status, dto) = get_panel(app.clone(), 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-2_R-1").unwrap()["value"], "1");

    // Overwrite case: P-1_R-1 already had "7" stored.
    let (status, dto) = post_panel(app.clone(), 1, "P-1_R-1", "9").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "9");

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
    let (status, dto) = get_panel(app.clone(), 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "7");

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
    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(field(&dto, "P-1_R-1").unwrap()["value"], "9");
}

// AC10: `diagnostics.len()` equals `Activation::diagnostics.len()` for a
// fixture producing at least one diagnostic (an unparsable `when/@test`).
#[tokio::test]
async fn diagnostics_are_carried_through_one_to_one() {
    let (_dir, products) = temp_product_db(DIAGNOSTIC_PROGRAM);
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    // The `when/@test` neither parses nor is a `default`, so `evaluate`
    // reports both the unparsable condition and the resulting no-match --
    // exactly what a bare count of `Activation::diagnostics` would produce,
    // which is what this asserts: nothing here is deduplicated or dropped.
    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert_eq!(diagnostics.len(), 2);
    assert_eq!(
        diagnostics[0]["message"],
        "A choice's condition could not be understood."
    );
    assert!(diagnostics[0]["detail"]
        .as_str()
        .unwrap()
        .contains("UnparsableTest"));
    assert_eq!(
        diagnostics[1]["message"],
        "A choice did not match any of its options."
    );
    assert!(diagnostics[1]["detail"]
        .as_str()
        .unwrap()
        .contains("NoBranchMatched"));
}

// Coordinator addition: `parameter_views`' inner joins can silently drop
// a row whose `parameter` does not resolve — that must surface as a
// diagnostic naming the dropped-row count, not vanish.
#[tokio::test]
async fn a_parameter_ref_whose_parameter_row_is_missing_is_reported_not_dropped_silently() {
    let (_dir, products) = temp_product_db(WRITE_PROGRAM);
    products
        .execute("DELETE FROM parameter WHERE id = 'P-2'", [])
        .unwrap();
    let state = Arc::new(state_with_device(products, vec![]));
    let app = knx_server::app(Arc::clone(&state), None);

    let (status, dto) = get_panel(app, 1).await;
    assert_eq!(status, StatusCode::OK);
    assert!(field(&dto, "P-2_R-1").is_none());
    assert!(field(&dto, "P-1_R-1").is_some());

    let diagnostics = dto["diagnostics"].as_array().unwrap();
    assert!(diagnostics.iter().any(|d| d["detail"]
        .as_str()
        .unwrap()
        .contains("1 dropped by an unresolved parameter/parameter_type join")));
}
