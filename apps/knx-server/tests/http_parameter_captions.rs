//! Parameter captions use exact positional references without changing project data.

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

const PROGRAM: &str = r##"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="Fictional" ApplicationVersion="1" MaskVersion="MV-0701">
<Static><ParameterTypes>
<ParameterType Id="PT-T" Name="Text"><TypeText SizeInBit="160" /></ParameterType>
<ParameterType Id="PT-N" Name="Number"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType>
</ParameterTypes><Parameters>
<Parameter Id="P-T" Name="Caption" ParameterType="PT-T" Value="Base default" />
<Parameter Id="P-N" Name="Value" ParameterType="PT-N" Value="1" />
</Parameters><ParameterRefs>
<ParameterRef Id="R-T" RefId="P-T" Value="Ref default" />
<ParameterRef Id="R-N" RefId="P-N" />
</ParameterRefs></Static><Dynamic>
<ParameterBlock Id="PB-1" Text="Functions: {{0}}" TextParameterRefId="R-T">
<ParameterRefRef RefId="R-T" /><ParameterRefRef RefId="R-N" />
<ParameterSeparator Id="PS-1" Text="Note: {{0}}" TextParameterRefId="R-T" UIHint="Headline" />
</ParameterBlock>
<ParameterBlock Id="PB-2" Text="{{0}}" TextParameterRefId="missing"><ParameterRefRef RefId="R-N" /></ParameterBlock>
<ParameterBlock Id="PB-3" Text="#"><ParameterSeparator Id="PS-3" Text="Information" /></ParameterBlock>
</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"##;

fn products() -> (tempfile::TempDir, knx_productdb::Connection) {
    products_from(PROGRAM)
}

fn products_from(source: &str) -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", source.as_bytes()).unwrap();
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

#[tokio::test]
async fn block_and_separator_captions_read_exact_text_reference_without_writes() {
    let (_dir, products) = products();
    let state = Arc::new(state(products));
    let app = knx_server::app(state.clone(), None);
    let before = state.project.lock().unwrap().clone();
    let dto = get_panel(app).await;
    let group = &dto["sections"][0]["fields"][0]["group"];
    assert_eq!(group["key"], "PB-1");
    assert_eq!(group["label"], "Functions: Ref default");
    assert_eq!(group["labelFill"]["template"], "Functions: {{0}}");
    assert_eq!(group["labelFill"]["source"], "ProductDefault");
    let stored_caption: (String, String) = state.product_db.as_ref().unwrap().lock().unwrap()
        .query_row("SELECT text, text_parameter_ref_id FROM dynamic_node WHERE program_id = 'A-1' AND element_id = 'PB-1'",
            [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap();
    assert_eq!(stored_caption, ("Functions: {{0}}".into(), "R-T".into()));
    assert_eq!(
        dto["sections"][0]["separators"][0]["text"],
        "Note: Ref default"
    );
    assert_eq!(
        dto["sections"][0]["separators"][0]["textFill"]["source"],
        "ProductDefault"
    );
    assert_eq!(*state.project.lock().unwrap(), before);
}

#[tokio::test]
async fn persisted_override_wins_and_explicit_empty_does_not_revive_product_default() {
    for (raw, expected, source) in [
        ("Kitchen", "Functions: Kitchen", "StoredValue"),
        ("", "Functions: ", "EmptyValue"),
    ] {
        let (_dir, products) = products();
        let state = Arc::new(state(products));
        state
            .project
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .installations[0]
            .parameters
            .push(knx_core::ParameterInstance {
                id: knx_core::ParameterInstanceId(1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: "R-T".into(),
                },
                raw: raw.into(),
            });
        let before = state.project.lock().unwrap().clone();
        let dto = get_panel(knx_server::app(state.clone(), None)).await;
        assert_eq!(dto["sections"][0]["fields"][0]["group"]["label"], expected);
        assert_eq!(
            dto["sections"][0]["fields"][0]["group"]["labelFill"]["source"],
            source
        );
        assert_eq!(*state.project.lock().unwrap(), before);
    }
}

#[tokio::test]
async fn parameter_write_and_undo_refresh_caption_without_overwriting_source_metadata() {
    let (_dir, products) = products();
    let state = Arc::new(state(products));
    let app = knx_server::app(state, None);
    let initial = get_panel(app.clone()).await;
    let request = Request::builder()
        .method("POST")
        .uri("/api/device/1/parameters")
        .header("content-type", "application/json")
        .body(Body::from(
            json!({"etsId":"R-T", "raw":"Kitchen"}).to_string(),
        ))
        .unwrap();
    let (status, dto) = call(app.clone(), request).await;
    assert_eq!(status, StatusCode::OK, "{dto}");
    assert_eq!(
        dto["sections"][0]["fields"][0]["group"]["label"],
        "Functions: Kitchen"
    );
    let request = Request::builder()
        .method("POST")
        .uri("/api/undo")
        .body(Body::empty())
        .unwrap();
    assert_eq!(call(app.clone(), request).await.0, StatusCode::OK);
    assert_eq!(get_panel(app).await["sections"], initial["sections"]);
}

#[tokio::test]
async fn unavailable_references_remain_explicit_not_guessed_from_template_defaults() {
    for (attribute, reason) in [
        ("", "NoReference"),
        (" TextParameterRefId=\"missing\"", "UnknownParameter"),
        (" TextParameterRefId=\"R-N\"", "NotTextParameter"),
    ] {
        let source = PROGRAM.replace(
            "Text=\"Functions: {{0}}\" TextParameterRefId=\"R-T\"",
            &format!("Text=\"{{{{0:Not a verified value}}}}\"{attribute}"),
        );
        let (_dir, products) = products_from(&source);
        let dto = get_panel(knx_server::app(Arc::new(state(products)), None)).await;
        let group = &dto["sections"][0]["fields"][0]["group"];
        assert_eq!(group["label"], "{{0:Not a verified value}}");
        assert_eq!(group["labelFill"]["source"], reason);
    }
}

#[tokio::test]
async fn empty_defaults_use_template_default_and_base_parameter_default_is_supported() {
    for (replacement, expected, source) in [
        ("Value=\"\"", "Functions: Fallback", "TemplateDefault"),
        ("", "Functions: Base default", "ProductDefault"),
    ] {
        let source_xml = PROGRAM
            .replace("Value=\"Ref default\"", replacement)
            .replace("Functions: {{0}}", "Functions: {{0:Fallback}}");
        let (_dir, products) = products_from(&source_xml);
        let dto = get_panel(knx_server::app(Arc::new(state(products)), None)).await;
        let group = &dto["sections"][0]["fields"][0]["group"];
        assert_eq!(group["label"], expected);
        assert_eq!(group["labelFill"]["source"], source);
    }
}

#[tokio::test]
async fn separator_grid_captions_use_the_same_resolution_and_source_record() {
    let source = PROGRAM.replace("Id=\"PB-1\"", "Id=\"PB-1\" Layout=\"Table\"")
        .replace("<ParameterRefRef RefId=\"R-T\" />", "<Rows><Row Id=\"ROW\" /></Rows><Columns><Column Id=\"COL\" /></Columns><ParameterRefRef RefId=\"R-T\" Cell=\"1,1\" />")
        .replace("Id=\"PS-1\"", "Id=\"PS-1\" Cell=\"1,1\"");
    let (_dir, products) = products_from(&source);
    let dto = get_panel(knx_server::app(Arc::new(state(products)), None)).await;
    let label = &dto["sections"][0]["groupLayouts"][0]["labels"][0];
    assert_eq!(label["text"], "Note: Ref default");
    assert_eq!(label["textFill"]["template"], "Note: {{0}}");
}

#[tokio::test]
async fn over_budget_caption_is_refused_without_partial_results_or_project_changes() {
    let huge = "x".repeat(knx_productdb::dynamic::MAX_TEXT_PROJECTION_WORK);
    let source = PROGRAM.replace("Ref default", &huge);
    let (_dir, products) = products_from(&source);
    let state = Arc::new(state(products));
    let before = state.project.lock().unwrap().clone();
    let request = Request::builder()
        .uri("/api/device/1/parameters")
        .body(Body::empty())
        .unwrap();
    let (status, dto) = call(knx_server::app(state.clone(), None), request).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(dto["error"], "device text projection budget exhausted");
    assert!(dto.get("sections").is_none());
    assert_eq!(*state.project.lock().unwrap(), before);
}

#[tokio::test]
async fn activation_over_budget_post_preserves_project_revision_history_and_native_bytes() {
    // A small hidden template expands only after the otherwise valid edit.
    let huge = "x".repeat(100_000);
    let head = PROGRAM
        .split("<Dynamic>")
        .next()
        .unwrap()
        .replace("Ref default", &huge);
    let template = "{{0}}".repeat(64);
    let source = format!(
        r#"{head}<Dynamic><ParameterRefRef RefId="R-N" />
<choose ParamRefId="R-N"><when test="2"><ParameterBlock Id="PB-Large" Text="{template}" TextParameterRefId="R-T"><ParameterRefRef RefId="R-T" /></ParameterBlock></when></choose>
</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let (dir, products) = products_from(&source);
    let mut state = state(products);
    state.data_dir = dir.path().to_path_buf();
    let state = Arc::new(state);
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        get_panel(app.clone()).await["sections"][0]["fields"][0]["value"],
        "1"
    );
    let save = Request::post("/api/project/save-as")
        .header("content-type", "application/json")
        .body(Body::from(json!({"path":"caption.knxdb"}).to_string()))
        .unwrap();
    assert_eq!(call(app.clone(), save).await.0, StatusCode::OK);
    let path = state.store_path.lock().unwrap().clone().unwrap();
    let bytes = std::fs::read(&path).unwrap();
    let before = state.project.lock().unwrap().clone();
    let revision = state
        .project_revision
        .load(std::sync::atomic::Ordering::Relaxed);
    let generation = *state.history_generation.lock().unwrap();
    let history = format!("{:?}", state.command_stack.lock().unwrap());
    let post = Request::post("/api/device/1/parameters")
        .header("content-type", "application/json")
        .body(Body::from(json!({"etsId":"R-N", "raw":"2"}).to_string()))
        .unwrap();
    let (status, dto) = call(app.clone(), post).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(dto["error"], "device text projection budget exhausted");
    assert_eq!(
        *state.project.lock().unwrap(),
        before,
        "refused activation must not publish the edit"
    );
    assert_eq!(
        state
            .project_revision
            .load(std::sync::atomic::Ordering::Relaxed),
        revision
    );
    assert_eq!(*state.history_generation.lock().unwrap(), generation);
    assert_eq!(
        format!("{:?}", state.command_stack.lock().unwrap()),
        history
    );
    assert_eq!(std::fs::read(&path).unwrap(), bytes);
    assert_eq!(
        get_panel(app).await["sections"][0]["fields"][0]["value"],
        "1"
    );
}

#[tokio::test]
async fn module_captions_read_only_their_own_scoped_value_and_refuse_ambiguous_instances() {
    let top = PROGRAM.split("<Dynamic>").next().unwrap();
    let source = format!(
        r#"{top}<Dynamic>
<Module Id="A_MD-1_M-1" RefId="A_MD-1" /><Module Id="A_MD-1_M-2" RefId="A_MD-1" />
</Dynamic><ModuleDefs><ModuleDef Id="A_MD-1" Name="Module"><Static>
<ParameterTypes><ParameterType Id="A_MD-1_PT-T" Name="Text"><TypeText SizeInBit="160" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="A_MD-1_P-T" Name="Caption" ParameterType="A_MD-1_PT-T" Value="Module default" /></Parameters>
<ParameterRefs><ParameterRef Id="A_MD-1_P-T_R-1" RefId="A_MD-1_P-T" /></ParameterRefs></Static><Dynamic>
<ParameterBlock Id="PB-M" Text="{{{{0}}}}" TextParameterRefId="A_MD-1_P-T_R-1"><ParameterRefRef RefId="A_MD-1_P-T_R-1" /></ParameterBlock>
</Dynamic></ModuleDef></ModuleDefs></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let (_dir, products) = products_from(&source);
    let state = Arc::new(state(products));
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        project
            .devices
            .insert_module_instance(knx_core::ModuleInstance {
                id: knx_core::ModuleInstanceId(1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: "MD-1_M-1".into(),
                },
                instance_ets_id: "MD-1_M-1_MI-1".into(),
                repeat_index: String::new(),
                arguments: vec![],
            });
        project.installations[0]
            .parameters
            .push(knx_core::ParameterInstance {
                id: knx_core::ParameterInstanceId(1),
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: "A_MD-1_M-1_MI-1_P-T_R-1".into(),
                },
                raw: "Own module".into(),
            });
    }
    let app = knx_server::app(state.clone(), None);
    let dto = get_panel(app.clone()).await;
    assert_eq!(
        dto["sections"][0]["fields"][0]["group"]["label"],
        "Own module"
    );
    assert_eq!(
        dto["sections"][1]["fields"][0]["group"]["label"],
        "Module default"
    );
    {
        let mut guard = state.project.lock().unwrap();
        guard
            .as_mut()
            .unwrap()
            .devices
            .insert_module_instance(knx_core::ModuleInstance {
                id: knx_core::ModuleInstanceId(2),
                device: DeviceId(1),
                source: SourceRef {
                    path: "device-1".into(),
                    ets_id: "MD-1_M-1".into(),
                },
                instance_ets_id: "MD-1_M-1_MI-2".into(),
                repeat_index: String::new(),
                arguments: vec![],
            });
    }
    let before = state.project.lock().unwrap().clone();
    let dto = get_panel(app).await;
    assert_eq!(
        dto["sections"][0]["fields"][0]["group"]["labelFill"]["source"],
        "Undecided"
    );
    assert_eq!(
        dto["sections"][1]["fields"][0]["group"]["label"],
        "Module default"
    );
    assert_eq!(*state.project.lock().unwrap(), before);
}
