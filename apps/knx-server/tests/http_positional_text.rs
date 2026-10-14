//! Positional `{{0}}` texts in the device detail (ADR-0126, Goal B14, issue #17).
//!
//! Public synthetic fixture only: no private package, no bus, no vendor code.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId,
    DeviceInstance, Installation, InstallationId, Language, Layer, Override, ParameterInstance,
    ParameterInstanceId, Project, Resolved, ResolvedFlags, SourceRef, Text, Topology,
};
use serde_json::Value;
use tower::ServiceExt;

const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="Public synthetic fixture" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="Public program" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
<ParameterType Id="A-1_PT-T" Name="T"><TypeText SizeInBit="240" /></ParameterType>
<ParameterType Id="A-1_PT-N" Name="N"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="9" /></ParameterType>
</ParameterTypes>
<Parameters>
<Parameter Id="A-1_P-1" Name="Page" ParameterType="A-1_PT-T" Text="Page name" Value="" />
<Parameter Id="A-1_P-2" Name="Icon" ParameterType="A-1_PT-T" Text="Icon name" Value="Lamp" />
<Parameter Id="A-1_P-3" Name="Count" ParameterType="A-1_PT-N" Text="Count" Value="1" />
</Parameters>
<ParameterRefs>
<ParameterRef Id="A-1_P-1_R-1" RefId="A-1_P-1" />
<ParameterRef Id="A-1_P-2_R-1" RefId="A-1_P-2" />
<ParameterRef Id="A-1_P-3_R-1" RefId="A-1_P-3" />
</ParameterRefs>
<ComObjectTable>
<ComObject Id="A-1_O-1" Number="1" Text="Page 1 - {{0:Icon 1}}" ObjectSize="1 Bit" />
<ComObject Id="A-1_O-2" Number="2" Text="Switch {{0:Output}}" ObjectSize="1 Bit" />
<ComObject Id="A-1_O-3" Number="3" Text="Dim {{0}}" ObjectSize="1 Bit" />
<ComObject Id="A-1_O-4" Number="4" Text="Count {{0:n}}" ObjectSize="1 Bit" />
<ComObject Id="A-1_O-5" Number="5" Text="Mine {{0}}" ObjectSize="1 Bit" />
</ComObjectTable>
<ComObjectRefs>
<ComObjectRef Id="A-1_O-1_R-1" RefId="A-1_O-1" TextParameterRefId="A-1_P-2_R-1" />
<ComObjectRef Id="A-1_O-2_R-1" RefId="A-1_O-2" TextParameterRefId="A-1_P-1_R-1" />
<ComObjectRef Id="A-1_O-3_R-1" RefId="A-1_O-3" />
<ComObjectRef Id="A-1_O-4_R-1" RefId="A-1_O-4" TextParameterRefId="A-1_P-3_R-1" />
<ComObjectRef Id="A-1_O-5_R-1" RefId="A-1_O-5" TextParameterRefId="A-1_P-1_R-1" />
</ComObjectRefs>
</Static>
<Dynamic><Channel Id="A-1_CH-1" Name="A" Number="1" Text="{{0:Page 1}}" TextParameterRefId="A-1_P-1_R-1">
<ComObjectRefRef RefId="A-1_O-1_R-1" /><ComObjectRefRef RefId="A-1_O-2_R-1" /><ComObjectRefRef RefId="A-1_O-3_R-1" />
<ComObjectRefRef RefId="A-1_O-4_R-1" /><ComObjectRefRef RefId="A-1_O-5_R-1" />
</Channel></Dynamic>
<Languages><Language Identifier="de-DE"><TranslationUnit RefId="A-1">
<TranslationElement RefId="A-1_CH-1"><Translation AttributeName="Text" Text="{{0:Seite 1}}" /></TranslationElement>
<TranslationElement RefId="A-1_O-2"><Translation AttributeName="Text" Text="Schalten {{0:Ausgang}}" /></TranslationElement>
</TranslationUnit></Language></Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: "public-synthetic".into(),
        ets_id: id.into(),
    }
}

fn object(n: u32, text: &str, layer: Layer) -> ComObjectInstance {
    ComObjectInstance {
        id: ComObjectInstanceId(n),
        source: source(&format!("A-1_O-{n}_R-1")),
        device: DeviceId(1),
        number: n as u16,
        text: Override::Value(Resolved {
            value: Text::Literal(text.into()),
            layer,
        }),
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
    }
}

/// `page`: the device's stored value of the page-name parameter, if any.
fn state(page: Option<&str>) -> (tempfile::TempDir, Arc<knx_server::AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-1/A.xml", PROGRAM.as_bytes()).unwrap();
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source("device-1"),
        name: "Public device".into(),
        description: None,
        address: None,
        product_ref: "M-1_P-1".into(),
        program_ref: "H-1_HP-1".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: (1..=5).map(ComObjectInstanceId).collect(),
        binary_data: vec![],
    });
    // As an import stores them: the product's own texts, at the program layer.
    project
        .devices
        .insert_com_object(object(1, "Page 1 - {{0:Icon 1}}", Layer::Program));
    // As ETS 6 stores it: the device-local `O-<n>_R-<m>` id, no program prefix.
    let mut switch = object(2, "Switch {{0:Output}}", Layer::Program);
    switch.source = source("O-2_R-1");
    project.devices.insert_com_object(switch);
    project
        .devices
        .insert_com_object(object(3, "Dim {{0}}", Layer::Program));
    project
        .devices
        .insert_com_object(object(4, "Count {{0:n}}", Layer::Program));
    // A name the user typed, braces included, is the user's.
    project
        .devices
        .insert_com_object(object(5, "My {{0}} name", Layer::UserEdit));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Public installation".into(),
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
        parameters: page
            .map(|raw| ParameterInstance {
                id: ParameterInstanceId(1),
                device: DeviceId(1),
                source: source("A-1_P-1_R-1"),
                raw: raw.into(),
            })
            .into_iter()
            .collect(),
    });
    let mut state = knx_server::AppState::new(dir.path().to_path_buf());
    state.product_db = Some(Mutex::new(products));
    *state.project.lock().unwrap() = Some(project);
    (dir, Arc::new(state))
}

async fn detail(state: &Arc<knx_server::AppState>, uri: &str) -> Value {
    let response = knx_server::app(Arc::clone(state), None)
        .oneshot(Request::builder().uri(uri).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap()
}

fn fill(template: &str, source: &str) -> Value {
    serde_json::json!({ "template": template, "source": source })
}

#[tokio::test]
async fn names_and_the_heading_show_the_text_parameter_and_say_where_it_came_from() {
    let (_dir, state) = state(Some("Kitchen"));
    let before = state.project.lock().unwrap().as_ref().unwrap().clone();
    let body = detail(&state, "/api/device/1").await;
    let objects = body["com_objects"].as_array().unwrap();
    let channel = &objects[0]["channel"];
    assert_eq!(channel["text"], "Kitchen");
    assert_eq!(channel["text_fill"], fill("{{0:Page 1}}", "StoredValue"));

    let name = |n: usize| (&objects[n]["name"], &objects[n]["name_fill"]);
    assert_eq!(
        name(0),
        (
            &Value::from("Page 1 - Lamp"),
            &fill("Page 1 - {{0:Icon 1}}", "ProductDefault")
        )
    );
    assert_eq!(
        name(1),
        (
            &Value::from("Switch Kitchen"),
            &fill("Switch {{0:Output}}", "StoredValue")
        )
    );
    // Not filled, and visibly so: no reference, or not a text parameter.
    assert_eq!(
        name(2),
        (&Value::from("Dim {{0}}"), &fill("Dim {{0}}", "NoReference"))
    );
    assert_eq!(
        name(3),
        (
            &Value::from("Count {{0:n}}"),
            &fill("Count {{0:n}}", "NotTextParameter")
        )
    );
    // The user's own name is never filled, and has no fill record.
    assert_eq!(objects[4]["name"], "My {{0}} name");
    assert!(objects[4].get("name_fill").is_none());
    // Reading changes nothing.
    assert_eq!(state.project.lock().unwrap().as_ref().unwrap(), &before);
}

#[tokio::test]
async fn an_empty_value_shows_the_template_default_also_in_another_language() {
    let (_dir, state) = state(Some(""));
    let body = detail(&state, "/api/device/1").await;
    let objects = body["com_objects"].as_array().unwrap();
    assert_eq!(objects[0]["channel"]["text"], "Page 1");
    assert_eq!(
        objects[0]["channel"]["text_fill"],
        fill("{{0:Page 1}}", "TemplateDefault")
    );
    assert_eq!(objects[1]["name"], "Switch Output");
    assert_eq!(
        objects[1]["name_fill"],
        fill("Switch {{0:Output}}", "TemplateDefault")
    );

    let german = detail(&state, "/api/device/1?language=de-DE").await;
    let channel = &german["com_objects"][0]["channel"];
    assert_eq!(channel["text"], "Seite 1");
    assert_eq!(
        channel["text_fill"],
        fill("{{0:Seite 1}}", "TemplateDefault")
    );
    // The name is translated first, then filled: the German default shows.
    let switch = &german["com_objects"][1];
    assert_eq!(switch["name"], "Schalten Ausgang");
    assert_eq!(
        switch["name_fill"],
        fill("Schalten {{0:Ausgang}}", "TemplateDefault")
    );
}

#[tokio::test]
async fn without_a_stored_value_the_empty_product_default_counts_as_empty() {
    let (_dir, state) = state(None);
    let body = detail(&state, "/api/device/1").await;
    let objects = body["com_objects"].as_array().unwrap();
    assert_eq!(objects[0]["channel"]["text"], "Page 1");
    assert_eq!(objects[1]["name"], "Switch Output");
    // An empty product default is a value that is empty.
    assert_eq!(
        objects[1]["name_fill"],
        fill("Switch {{0:Output}}", "TemplateDefault")
    );
}

/// A fill that would not fit is refused like any over-budget product text:
/// nothing is shortened, the whole detail is refused, and nothing changes.
#[tokio::test]
async fn a_fill_that_would_not_fit_refuses_the_detail_and_shortens_nothing() {
    let quarter = knx_productdb::dynamic::MAX_TEXT_PROJECTION_WORK / 4;
    let huge = "X".repeat(quarter);
    let program = PROGRAM
        .replace(
            r#"<TypeText SizeInBit="240" />"#,
            &format!(r#"<TypeText SizeInBit="{}" />"#, quarter * 8),
        )
        .replace(
            r#"Text="Icon name" Value="Lamp""#,
            &format!(r#"Text="Icon name" Value="{huge}""#),
        );
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-1/A.xml", program.as_bytes()).unwrap();
    let (_old, state) = state(None);
    *state.product_db.as_ref().unwrap().lock().unwrap() = products;
    // Five names read the quarter-budget icon default: 5/4 of the budget.
    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        for n in 1..=5u32 {
            let mut com = object(n, "{{0}}", Layer::Program);
            com.source = source("A-1_O-1_R-1");
            project.devices.insert_com_object(com);
        }
    }
    let before = state.project.lock().unwrap().as_ref().unwrap().clone();
    let response = knx_server::app(Arc::clone(&state), None)
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        body,
        serde_json::json!({"error": "device text projection budget exhausted"})
    );
    assert_eq!(state.project.lock().unwrap().as_ref().unwrap(), &before);
}
