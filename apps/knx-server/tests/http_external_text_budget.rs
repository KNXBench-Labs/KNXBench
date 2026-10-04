//! Public synthetic outside-walk translated-text resource refusal (AR07).
//!
//! No private package, live bus, host product database or vendor code.
//! The RED runner isolates XDG_DATA_HOME; the response is not printed.
//!

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId,
    DeviceInstance, Installation, InstallationId, Language, Override, Project, ResolvedFlags,
    SourceRef, Topology,
};
use serde_json::Value;
use tower::ServiceExt;

const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<Hardware><Hardware Id="H-1" Name="Public synthetic fixture" SerialNumber="S" VersionNumber="1">
<Products><Product Id="M-1_P-1" /></Products>
<Hardware2Programs><Hardware2Program Id="H-1_HP-1" MediumTypes="MT-0">
<ApplicationProgramRef RefId="A-1" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: "public-synthetic".into(),
        ets_id: id.into(),
    }
}

fn state_with_translation(
    text: &str,
    object_count: u32,
) -> (
    tempfile::TempDir,
    Arc<knx_server::AppState>,
    std::path::PathBuf,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("products.sqlite");
    let products = knx_productdb::open_and_migrate(&path).unwrap();
    assert!((1..=u32::from(u16::MAX)).contains(&object_count));
    let object_table = (1..=object_count)
        .map(|n| {
            format!(r#"<ComObject Id="A-1_O-{n}" Number="{n}" Text="Switch" ObjectSize="1 Bit" />"#)
        })
        .collect::<String>();
    let object_refs = (1..=object_count)
        .map(|n| format!(r#"<ComObjectRef Id="A-1_O-{n}_R-1" RefId="A-1_O-{n}" />"#))
        .collect::<String>();
    let dynamic_refs = (1..=object_count)
        .map(|n| format!(r#"<ComObjectRefRef RefId="A-1_O-{n}_R-1" />"#))
        .collect::<String>();
    let program = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="Public program" ApplicationVersion="1" MaskVersion="MV-0701">
<Static><ComObjectTable>{object_table}</ComObjectTable>
<ComObjectRefs>{object_refs}</ComObjectRefs></Static>
<Dynamic><Channel Id="A-1_CH-1" Text="Small">{dynamic_refs}</Channel></Dynamic>
<Languages><Language Identifier="de-DE"><TranslationUnit RefId="A-1">
<TranslationElement RefId="A-1_CH-1"><Translation AttributeName="Text" Text="{text}" /></TranslationElement>
</TranslationUnit></Language></Languages>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    knx_productdb::ingest_file(&products, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-1/A.xml", program.as_bytes()).unwrap();
    assert_eq!(
        knx_productdb::query::channel_texts(&products, "A-1", "de-DE").unwrap()["A-1_CH-1"],
        text,
    );
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
        com_objects: (1..=object_count).map(ComObjectInstanceId).collect(),
        binary_data: vec![],
    });
    for n in 1..=object_count {
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(n),
            source: source(&format!("A-1_O-{n}_R-1")),
            device: DeviceId(1),
            number: n as u16,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
    }
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
        parameters: vec![],
    });
    let mut state = knx_server::AppState::new(dir.path().to_path_buf());
    state.product_db = Some(Mutex::new(products));
    *state.project.lock().unwrap() = Some(project);
    (dir, Arc::new(state), path)
}

#[tokio::test]
async fn translated_channel_scalar_cost_exhaustion_refuses_whole_device_detail() {
    // Raw input allowance plus complete output is 4,000,002 bytes: larger
    // than the proposed shared 4,000,000-unit device projection budget.
    // This is stronger application policy, not a KNX text-length rule.
    let text = "X".repeat(knx_productdb::dynamic::MAX_EVALUATION_WORK / 2 + 1);
    let (_dir, state, products_path) = state_with_translation(&text, 1);
    let project_before = state.project.lock().unwrap().as_ref().unwrap().clone();
    assert_eq!(project_before.devices.iter().count(), 1);
    assert_eq!(project_before.devices.com_objects().count(), 1);
    let products_before = std::fs::read(&products_path).unwrap();
    let app = knx_server::app(Arc::clone(&state), None);
    let baseline = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(baseline.status(), StatusCode::OK);
    let baseline: Value = serde_json::from_slice(
        &axum::body::to_bytes(baseline.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(baseline["com_objects"][0]["activation"], "Active");
    assert_eq!(baseline["com_objects"][0]["channel"]["text"], "Small");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1?language=de-DE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    assert_eq!(
        state.project.lock().unwrap().as_ref().unwrap(),
        &project_before
    );
    assert_eq!(std::fs::read(&products_path).unwrap(), products_before);
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "outside-walk translation must refuse, not publish an unchecked device detail",
    );
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        body,
        serde_json::json!({"error": "device text projection budget exhausted"})
    );
    assert!(
        body.get("com_objects").is_none(),
        "no partial detail on refusal"
    );
}

#[tokio::test]
async fn cached_channel_text_copies_share_one_request_budget() {
    // Cache preparation costs 2L (input+render); two response clones add 2L,
    // reaching the ceiling exactly. A third clone must refuse the whole GET.
    let text = "X".repeat(knx_productdb::dynamic::MAX_TEXT_PROJECTION_WORK / 4);
    let (_dir, two, products_path) = state_with_translation(&text, 2);
    let project_before = two.project.lock().unwrap().as_ref().unwrap().clone();
    let products_before = std::fs::read(products_path).unwrap();
    let response = knx_server::app(Arc::clone(&two), None)
        .oneshot(
            Request::builder()
                .uri("/api/device/1?language=de-DE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    let objects = body["com_objects"].as_array().unwrap();
    assert_eq!(objects.len(), 2);
    for object in objects {
        assert_eq!(object["activation"], "Active");
        assert_eq!(object["channel"]["text"], text);
    }
    assert_eq!(objects[0]["channel"]["key"], objects[1]["channel"]["key"]);
    assert_eq!(
        two.project.lock().unwrap().as_ref().unwrap(),
        &project_before
    );
    assert_eq!(
        std::fs::read(_dir.path().join("products.sqlite")).unwrap(),
        products_before
    );

    let (_dir, three, products_path) = state_with_translation(&text, 3);
    let project_before = three.project.lock().unwrap().as_ref().unwrap().clone();
    assert_eq!(project_before.devices.com_objects().count(), 3);
    let products_before = std::fs::read(products_path).unwrap();
    let response = knx_server::app(Arc::clone(&three), None)
        .oneshot(
            Request::builder()
                .uri("/api/device/1?language=de-DE")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body: Value = serde_json::from_slice(
        &axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap(),
    )
    .unwrap();
    assert_eq!(
        three.project.lock().unwrap().as_ref().unwrap(),
        &project_before
    );
    assert_eq!(
        std::fs::read(_dir.path().join("products.sqlite")).unwrap(),
        products_before
    );
    assert_eq!(
        status,
        StatusCode::BAD_REQUEST,
        "repeated cache copies must share one budget"
    );
    assert_eq!(
        body,
        serde_json::json!({"error": "device text projection budget exhausted"})
    );
}
