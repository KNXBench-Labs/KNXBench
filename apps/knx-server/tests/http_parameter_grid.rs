//! Declared parameter grids reach the HTTP parameter panel as versioned geometry.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="X" SerialNumber="S" VersionNumber="1"><Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0"><ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn program(dynamic: &str) -> String {
    let ids = ["P-1", "P-2", "P-3", "P-4", "P-5"];
    let parameters: String = ids
        .iter()
        .map(|id| {
            format!(
                r#"<Parameter Id="{id}" Text="Fictional {id}" ParameterType="PT-1" Value="0"/>"#
            )
        })
        .collect();
    let refs: String = ids
        .iter()
        .map(|id| format!(r#"<ParameterRef Id="{id}_R-1" RefId="{id}"/>"#))
        .collect();
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1"><ApplicationPrograms><ApplicationProgram Id="A-1" Name="Grid" ApplicationVersion="1" MaskVersion="MV-0701"><Static>
<ParameterTypes><ParameterType Id="PT-1"><TypeRestriction SizeInBit="8" Base="Value"><Enumeration Id="E-0" Value="0" Text="Off"/><Enumeration Id="E-1" Value="1" Text="On"/></TypeRestriction></ParameterType></ParameterTypes>
<Parameters>{parameters}</Parameters><ParameterRefs>{refs}</ParameterRefs></Static><Dynamic>{dynamic}</Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

const GRID: &str = r#"<ParameterBlock Id="PB-G" Text="Fictional pairs" Layout="Table">
<Rows><Row Id="PB-G_R-1" Text="First pair"/><Row Id="PB-G_R-2" CollapseIfEmpty="true"/></Rows>
<Columns><Column Id="PB-G_C-1" Width="30%"/><Column Id="PB-G_C-2" Text="Left" Width="70%" TextParameterRefId="P-4_R-1"/></Columns>
<ParameterSeparator Id="S-1" Text="Pair caption" Cell="1,1"/>
<ParameterRefRef RefId="P-1_R-1" Cell="1,2"/>
<choose ParamRefId="P-1_R-1"><when test="0"><ParameterRefRef RefId="P-2_R-1" Cell="2,2"/></when>
<when test="1"><ParameterSeparator Id="S-2" Text="Hidden caption" Cell="2,1"/></when></choose>
<ParameterRefRef RefId="P-3_R-1"/>
</ParameterBlock><ParameterBlock Id="PB-L" Text="Plain list"><ParameterRefRef RefId="P-4_R-1"/></ParameterBlock>
<ParameterBlock Id="PB-H" Text="Second grid" Layout="Grid"><Rows><Row Id="PB-H_R-1"/></Rows><Columns><Column Id="PB-H_C-1" Width="100%"/></Columns>
<ParameterSeparator Id="S-3" Text="Second caption" Cell="1,1"/><ParameterRefRef RefId="P-5_R-1" Cell="1,1"/></ParameterBlock>"#;

#[allow(clippy::field_reassign_with_default)]
async fn panel(dynamic: &str) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-1/A.xml", program(dynamic).as_bytes()).unwrap();
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
    let response = knx_server::app(Arc::new(state), None)
        .oneshot(
            Request::builder()
                .uri("/api/device/1/parameters")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn cell(dto: &Value, ets_id: &str) -> Value {
    dto["sections"][0]["fields"]
        .as_array()
        .unwrap()
        .iter()
        .find(|field| field["etsId"] == ets_id)
        .unwrap_or_else(|| panic!("{ets_id} present"))["cell"]
        .clone()
}

#[tokio::test]
async fn a_declared_grid_is_versioned_geometry_with_cells_and_active_captions() {
    let dto = panel(GRID).await;
    assert_eq!(
        dto["sections"][0]["groupLayouts"],
        json!([{
            "groupKey": "PB-G",
            "version": 1,
            "declaredLayout": "Table",
            "status": "grid",
            "rows": [
                { "id": "PB-G_R-1", "text": "First pair", "textParameterRefId": null, "collapseIfEmpty": null },
                { "id": "PB-G_R-2", "text": null, "textParameterRefId": null, "collapseIfEmpty": "true" }
            ],
            "columns": [
                { "id": "PB-G_C-1", "text": null, "textParameterRefId": null, "width": "30%" },
                { "id": "PB-G_C-2", "text": "Left", "textParameterRefId": "P-4_R-1", "width": "70%" }
            ],
            "labels": [{ "row": 1, "column": 1, "text": "Pair caption" }],
            "issues": [{ "kind": "missingCell", "element": "ParameterRefRef", "reference": "P-3_R-1", "cell": null }]
        }, {
            "groupKey": "PB-H",
            "version": 1,
            "declaredLayout": "Grid",
            "status": "grid",
            "rows": [{ "id": "PB-H_R-1", "text": null, "textParameterRefId": null, "collapseIfEmpty": null }],
            "columns": [{ "id": "PB-H_C-1", "text": null, "textParameterRefId": null, "width": "100%" }],
            "labels": [{ "row": 1, "column": 1, "text": "Second caption" }],
            "issues": []
        }])
    );
    assert_eq!(cell(&dto, "P-5_R-1"), json!({ "row": 1, "column": 1 }));
    assert_eq!(cell(&dto, "P-1_R-1"), json!({ "row": 1, "column": 2 }));
    assert_eq!(cell(&dto, "P-2_R-1"), json!({ "row": 2, "column": 2 }));
    assert_eq!(cell(&dto, "P-3_R-1"), Value::Null);
    assert_eq!(cell(&dto, "P-4_R-1"), Value::Null, "list block field");
}

#[tokio::test]
async fn a_program_without_a_grid_reports_an_empty_layout_list_and_no_cells() {
    let dto = panel(r#"<ParameterBlock Id="PB-L" Text="Plain list"><ParameterRefRef RefId="P-1_R-1"/><ParameterRefRef RefId="P-2_R-1"/></ParameterBlock>"#).await;
    assert_eq!(dto["sections"][0]["groupLayouts"], json!([]));
    assert_eq!(cell(&dto, "P-1_R-1"), Value::Null);
    assert_eq!(cell(&dto, "P-2_R-1"), Value::Null);
}

#[tokio::test]
async fn an_unknown_layout_variant_is_preserved_and_reported_without_cells() {
    let dto = panel(&GRID.replace(r#"Layout="Table""#, r#"Layout="Carousel""#)).await;
    let layout = &dto["sections"][0]["groupLayouts"][0];
    assert_eq!(layout["status"], "unknownLayout");
    assert_eq!(layout["declaredLayout"], "Carousel");
    assert_eq!(layout["version"], 1);
    assert_eq!(
        layout["rows"].as_array().unwrap().len(),
        2,
        "rows preserved"
    );
    assert_eq!(layout["labels"], json!([]));
    assert_eq!(cell(&dto, "P-1_R-1"), Value::Null);
}
