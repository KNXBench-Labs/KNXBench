//! Device detail over HTTP, including module objects whose own instance is unresolved.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{CommissioningState, DeviceId, DeviceInstance, Language, Project, SourceRef};
use serde_json::{json, Value};
use tower::ServiceExt;

fn state_with_one_device() -> knx_server::AppState {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        },
        name: "D1".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let state = knx_server::AppState::default();
    *state.project.lock().unwrap() = Some(project);
    state
}

#[tokio::test]
async fn device_detail_returns_the_device() {
    let state = Arc::new(state_with_one_device());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn device_detail_for_a_missing_device_is_a_400() {
    let state = Arc::new(state_with_one_device());
    let app = knx_server::app(state, None);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/device/999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

// WP07-S1: an imported module object whose own `ModuleInstance` is not in the
// project. Every manufacturer, program, project and object id below is invented.
const ORPHAN_AP: &str = "M-0F7C_A-0001-01-0000";
const ORPHAN_MODULE_OBJECT: &str = "MD-1_M-1_MI-1_O-1-0_R-1";
const ORPHAN_HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0F7C"><Hardware><Hardware Id="M-0F7C_H-1" Name="Fictional zone bench" SerialNumber="ZB" VersionNumber="1"><Products><Product Id="M-0F7C_P-1"/></Products><Hardware2Programs><Hardware2Program Id="M-0F7C_H-1_HP-1"><ApplicationProgramRef RefId="M-0F7C_A-0001-01-0000"/></Hardware2Program></Hardware2Programs></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

/// One static channel (`O-1` active, `O-3` declared but never activated) and
/// one `Module` of a `ModuleDef` whose only object has number 0.
fn orphan_program(with_dynamic: bool) -> String {
    let dynamic = if with_dynamic {
        format!(
            r#"<Dynamic><Channel Id="{ORPHAN_AP}_CH-1" Text="Bench main"><ParameterBlock Id="{ORPHAN_AP}_PB-1" Text="General"><ParameterRefRef RefId="{ORPHAN_AP}_P-1_R-1"/></ParameterBlock><ComObjectRefRef RefId="{ORPHAN_AP}_O-1_R-1"/></Channel><Module Id="{ORPHAN_AP}_MD-1_M-1" RefId="{ORPHAN_AP}_MD-1"/></Dynamic>"#
        )
    } else {
        String::new()
    };
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0F7C"><ApplicationPrograms><ApplicationProgram Id="{ORPHAN_AP}" Name="Fictional zone bench" ApplicationVersion="1" MaskVersion="MV-0701"><Static><ParameterTypes><ParameterType Id="{ORPHAN_AP}_PT-1" Name="level"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="9"/></ParameterType></ParameterTypes><Parameters><Parameter Id="{ORPHAN_AP}_P-1" Name="Level" Text="Level" ParameterType="{ORPHAN_AP}_PT-1" Access="ReadWrite" Value="1"/></Parameters><ParameterRefs><ParameterRef Id="{ORPHAN_AP}_P-1_R-1" RefId="{ORPHAN_AP}_P-1"/></ParameterRefs><ComObjectTable><ComObject Id="{ORPHAN_AP}_O-1" Number="1" Text="Lamp" FunctionText="Switch" ObjectSize="1 Bit"/><ComObject Id="{ORPHAN_AP}_O-3" Number="3" Text="Spare" FunctionText="Spare" ObjectSize="1 Bit"/></ComObjectTable><ComObjectRefs><ComObjectRef Id="{ORPHAN_AP}_O-1_R-1" RefId="{ORPHAN_AP}_O-1"/><ComObjectRef Id="{ORPHAN_AP}_O-3_R-1" RefId="{ORPHAN_AP}_O-3"/></ComObjectRefs></Static><ModuleDefs><ModuleDef Id="{ORPHAN_AP}_MD-1" Name="Fictional zone"><Static><ComObjectTable><ComObject Id="{ORPHAN_AP}_MD-1_O-1-0" Number="0" Text="Zone" FunctionText="Zone state" ObjectSize="1 Bit"/></ComObjectTable><ComObjectRefs><ComObjectRef Id="{ORPHAN_AP}_MD-1_O-1-0_R-1" RefId="{ORPHAN_AP}_MD-1_O-1-0"/></ComObjectRefs></Static><Dynamic><Channel Id="{ORPHAN_AP}_MD-1_CH-1" Text="Zone channel"><ComObjectRefRef RefId="{ORPHAN_AP}_MD-1_O-1-0_R-1"/></Channel></Dynamic></ModuleDef></ModuleDefs>{dynamic}</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

/// How the device states its `ModuleInstances`.
#[derive(Clone, Copy)]
enum Modules {
    /// No `ModuleInstances` element at all.
    Missing,
    /// `<ModuleInstances/>` with no child.
    Empty,
    /// The module object's own instance is present.
    Present,
}

fn orphan_knxproj(modules: Modules) -> Vec<u8> {
    let modules = match modules {
        Modules::Missing => "",
        Modules::Empty => "<ModuleInstances/>",
        Modules::Present => {
            r#"<ModuleInstances><ModuleInstance Id="MD-1_M-1_MI-1" RefId="MD-1_M-1"/></ModuleInstances>"#
        }
    };
    let topology = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><Project Id="P-0F7C"><Installations><Installation InstallationId="0"><Topology><Area Id="P-0F7C-0_A-1" Address="1"><Line Id="P-0F7C-0_L-1" Address="1"><DeviceInstance Id="P-0F7C-0_DI-1" Address="7" ProductRefId="M-0F7C_P-1" Hardware2ProgramRefId="M-0F7C_H-1_HP-1"><ParameterInstanceRefs><ParameterInstanceRef RefId="{ORPHAN_AP}_P-1_R-1" Value="2"/></ParameterInstanceRefs>{modules}<ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-1_R-1" Links="GA-1"/><ComObjectInstanceRef RefId="{ORPHAN_MODULE_OBJECT}" Links="GA-1"/></ComObjectInstanceRefs><GroupObjectTree GroupObjectInstances="O-1_R-1 {ORPHAN_MODULE_OBJECT} O-3_R-1"/></DeviceInstance></Line></Area></Topology><GroupAddresses><GroupRanges><GroupRange Id="P-0F7C-0_GR-1" RangeStart="0" RangeEnd="65535"><GroupAddress Id="P-0F7C-0_GA-1" Address="1"/></GroupRange></GroupRanges></GroupAddresses></Installation></Installations></Project></KNX>"#
    );
    let info = r#"<KNX xmlns="http://knx.org/xml/project/20"><Project Id="P-0F7C"><ProjectInformation Name="Fictional orphan bench"/></Project></KNX>"#;
    knx_testsupport::zip_with_entries(&[
        ("P-0F7C.signature", b""),
        ("P-0F7C/0.xml", topology.as_bytes()),
        ("P-0F7C/project.xml", info.as_bytes()),
    ])
}

async fn send(
    app: &axum::Router,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let request = Request::builder().method(method).uri(path);
    let request = match body {
        Some(body) => request
            .header("content-type", "application/json")
            .body(Body::from(body.to_string())),
        None => request.body(Body::empty()),
    };
    let response = app.clone().oneshot(request.unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024 * 1024)
        .await
        .unwrap();
    let value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, value)
}

async fn ok(app: &axum::Router, method: &str, path: &str, body: Option<Value>) -> Value {
    let (status, value) = send(app, method, path, body).await;
    assert_eq!(status, StatusCode::OK, "{method} {path}: {value}");
    value
}

/// Imports the fictional project through the real importer; `program` is
/// installed through the real catalogue installer when given.
async fn import_orphan(
    dir: &std::path::Path,
    program: Option<&str>,
    modules: Modules,
) -> (Arc<knx_server::AppState>, axum::Router) {
    let product_db = program.map(|program| {
        let db = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
        super::native_source_fixture::install(
            &db,
            20,
            ("M-0F7C/Hardware.xml", ORPHAN_HARDWARE.as_bytes()),
            ("M-0F7C/Application.xml", program.as_bytes()),
        );
        knx_server::CatalogueOwner::adopt_opened(db, dir.join("products.sqlite")).unwrap()
    });
    let state = Arc::new(knx_server::AppState {
        data_dir: dir.to_path_buf(),
        product_db,
        ..Default::default()
    });
    let app = knx_server::app(state.clone(), None);
    let source = dir.join("orphan-bench.knxproj");
    std::fs::write(&source, orphan_knxproj(modules)).unwrap();
    ok(
        &app,
        "POST",
        "/api/project/import",
        Some(json!({ "path": source })),
    )
    .await;
    (state, app)
}

/// The single device's id and each object's projected detail, keyed by `source.ets_id`.
async fn objects_by_source(
    app: &axum::Router,
    state: &knx_server::AppState,
) -> (u32, Value, BTreeMap<String, Value>) {
    let (device, ets_ids) = {
        let project = state.project.lock().unwrap();
        let project = project.as_ref().unwrap();
        let device = project.devices.iter().next().unwrap().id.0;
        let ets_ids: BTreeMap<u32, String> = project
            .devices
            .com_objects()
            .map(|com| (com.id.0, com.source.ets_id.clone()))
            .collect();
        (device, ets_ids)
    };
    let detail = ok(app, "GET", &format!("/api/device/{device}"), None).await;
    let objects = detail["com_objects"]
        .as_array()
        .unwrap()
        .iter()
        .map(|com| {
            (
                ets_ids[&(com["id"].as_u64().unwrap() as u32)].clone(),
                com.clone(),
            )
        })
        .collect();
    (device, detail, objects)
}

fn module_owner(state: &knx_server::AppState) -> Option<knx_core::ModuleInstanceId> {
    let project = state.project.lock().unwrap();
    let owner = project
        .as_ref()
        .unwrap()
        .devices
        .com_objects()
        .find(|com| com.source.ets_id == ORPHAN_MODULE_OBJECT)
        .unwrap()
        .module_instance;
    owner
}

fn assert_orphan_undetermined(objects: &BTreeMap<String, Value>) {
    let orphan = &objects[ORPHAN_MODULE_OBJECT];
    assert_eq!(orphan["activation"], "Undetermined", "{orphan}");
    assert!(orphan["channel"].is_null(), "{orphan}");
    assert_eq!(orphan["number"], 0, "number 0 stays 0: {orphan}");
    assert_eq!(orphan["is_active"], true, "{orphan}");
}

/// A1 + A3: the orphan is kept and `Undetermined`; the static controls keep
/// `Active`/`Inactive`, and the same fixture with the instance present differs.
#[tokio::test]
async fn an_orphan_module_object_is_undetermined_while_controls_keep_their_state() {
    let dir = tempfile::tempdir().unwrap();
    let (state, app) =
        import_orphan(dir.path(), Some(&orphan_program(true)), Modules::Missing).await;
    assert_eq!(
        module_owner(&state),
        None,
        "the importer keeps the object, ownerless"
    );
    let (_, _, objects) = objects_by_source(&app, &state).await;
    assert_eq!(
        objects.keys().map(String::as_str).collect::<Vec<_>>(),
        vec![ORPHAN_MODULE_OBJECT, "O-1_R-1", "O-3_R-1"],
    );
    // The genuine unscoped, never-activated object stays `Inactive`: nothing
    // in this evaluation is uncertain, so that is not what makes the orphan pass.
    assert_eq!(
        objects["O-3_R-1"]["activation"], "Inactive",
        "{}",
        objects["O-3_R-1"]
    );
    assert_eq!(objects["O-1_R-1"]["activation"], "Active");
    assert_eq!(objects["O-1_R-1"]["channel"]["text"], "Bench main");
    assert_orphan_undetermined(&objects);

    // Same fixture, own instance present: a uniquely resolved owner.
    let resolved_dir = tempfile::tempdir().unwrap();
    let (resolved_state, resolved_app) = import_orphan(
        resolved_dir.path(),
        Some(&orphan_program(true)),
        Modules::Present,
    )
    .await;
    assert!(module_owner(&resolved_state).is_some());
    let (_, _, resolved) = objects_by_source(&resolved_app, &resolved_state).await;
    let owned = &resolved[ORPHAN_MODULE_OBJECT];
    assert_eq!(owned["activation"], "Active", "{owned}");
    assert_eq!(owned["channel"]["text"], "Zone channel");
    assert_eq!(owned["number"], 0);
    assert_eq!(resolved["O-3_R-1"]["activation"], "Inactive");
    assert_eq!(resolved["O-1_R-1"], objects["O-1_R-1"]);
}

/// A2: a missing and an explicitly empty `ModuleInstances` container both
/// leave the object without instance authority, never as known-zero.
#[tokio::test]
async fn missing_and_empty_module_instance_containers_both_stay_undetermined() {
    let mut observed = Vec::new();
    for (label, modules) in [("missing", Modules::Missing), ("empty", Modules::Empty)] {
        let dir = tempfile::tempdir().unwrap();
        let (state, app) = import_orphan(dir.path(), Some(&orphan_program(true)), modules).await;
        assert_eq!(module_owner(&state), None);
        assert_eq!(
            state
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .devices
                .module_instances()
                .count(),
            0
        );
        let (_, _, objects) = objects_by_source(&app, &state).await;
        assert_eq!(objects["O-3_R-1"]["activation"], "Inactive");
        let orphan = &objects[ORPHAN_MODULE_OBJECT];
        observed.push((
            label,
            orphan["activation"].clone(),
            orphan["channel"].clone(),
            orphan["number"].clone(),
        ));
    }
    // Both containers are compared at once, so neither variant hides the other.
    assert_eq!(
        observed,
        vec![
            ("missing", json!("Undetermined"), Value::Null, json!(0)),
            ("empty", json!("Undetermined"), Value::Null, json!(0)),
        ]
    );
}

/// A4: without a product database or without a program `Dynamic` tree no
/// evaluation runs, so every object stays `NotEvaluated`.
#[tokio::test]
async fn an_orphan_without_an_evaluable_program_stays_not_evaluated() {
    for program in [None, Some(orphan_program(false))] {
        let dir = tempfile::tempdir().unwrap();
        let (state, app) = import_orphan(dir.path(), program.as_deref(), Modules::Missing).await;
        let (_, _, objects) = objects_by_source(&app, &state).await;
        assert_eq!(objects.len(), 3);
        for (source, object) in &objects {
            assert_eq!(object["activation"], "NotEvaluated", "{source}: {object}");
            assert!(object["channel"].is_null());
        }
        assert_eq!(objects[ORPHAN_MODULE_OBJECT]["number"], 0);
    }
}

/// A5: native save/reopen keeps every persisted fact; the read-only overlay
/// is recomputed as `Undetermined` and repeated reads write no history.
#[tokio::test]
async fn an_orphan_survives_save_reopen_and_repeated_reads_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let (state, app) =
        import_orphan(dir.path(), Some(&orphan_program(true)), Modules::Missing).await;
    let before = state.project.lock().unwrap().clone().unwrap();
    let history = state.command_stack.lock().unwrap().history_lengths();
    let (device, first, _) = objects_by_source(&app, &state).await;
    let (_, second, _) = objects_by_source(&app, &state).await;
    assert_eq!(first, second, "a repeated GET is stable");
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&before));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        history
    );

    let native = dir.path().join("orphan-bench.knxdb");
    ok(
        &app,
        "POST",
        "/api/project/save-as",
        Some(json!({ "path": native })),
    )
    .await;
    ok(
        &app,
        "POST",
        "/api/project/open",
        Some(json!({ "path": native })),
    )
    .await;
    let reopened = state.project.lock().unwrap().clone().unwrap();
    let persisted = |project: &Project| {
        let objects: Vec<_> = project
            .devices
            .com_objects()
            .map(|com| {
                (
                    com.id,
                    com.source.clone(),
                    com.number,
                    com.is_active,
                    com.links.clone(),
                    com.module_instance,
                )
            })
            .collect();
        let order: Vec<_> = project
            .devices
            .iter()
            .map(|d| (d.id, d.com_objects.clone()))
            .collect();
        let parameters: Vec<_> = project
            .installations
            .iter()
            .flat_map(|i| &i.parameters)
            .map(|p| (p.id, p.device, p.source.clone(), p.raw.clone()))
            .collect();
        let group_addresses: Vec<_> = project
            .installations
            .iter()
            .map(|i| format!("{:?}", i.group_addresses))
            .collect();
        (objects, order, parameters, group_addresses)
    };
    assert_eq!(persisted(&reopened), persisted(&before));
    let module_object = reopened
        .devices
        .com_objects()
        .find(|com| com.source.ets_id == ORPHAN_MODULE_OBJECT)
        .unwrap();
    assert_eq!(module_object.links.len(), 1, "the GA link is kept");
    assert_eq!(module_object.module_instance, None, "no owner was invented");
    assert_eq!(reopened.devices.module_instances().count(), 0);

    let history = state.command_stack.lock().unwrap().history_lengths();
    let (reopened_device, after, objects) = objects_by_source(&app, &state).await;
    let (_, again, _) = objects_by_source(&app, &state).await;
    assert_eq!(after, again);
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&reopened));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        history
    );
    assert_eq!(reopened_device, device);
    let ids = |detail: &Value| -> Vec<Value> {
        detail["com_objects"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| json!([c["id"], c["number"], c["is_active"], c["links"]]))
            .collect()
    };
    assert_eq!(ids(&after), ids(&first));
    assert_eq!(objects["O-3_R-1"]["activation"], "Inactive");
    assert_orphan_undetermined(&objects);
    assert_eq!(
        after, first,
        "the reopened projection equals the imported one"
    );
}

/// A6: a parameter edit whose evaluation activates the ownerless module
/// object is still refused as a whole; nothing is changed or recorded.
#[tokio::test]
async fn an_orphan_keeps_the_parameter_write_stop_line() {
    let dir = tempfile::tempdir().unwrap();
    let (state, app) =
        import_orphan(dir.path(), Some(&orphan_program(true)), Modules::Missing).await;
    let before = state.project.lock().unwrap().clone().unwrap();
    let history = state.command_stack.lock().unwrap().history_lengths();
    let device = before.devices.iter().next().unwrap().id.0;
    let (status, result) = send(
        &app,
        "POST",
        &format!("/api/device/{device}/parameters"),
        Some(json!({ "etsId": format!("{ORPHAN_AP}_P-1_R-1"), "raw": "3" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{result}");
    assert!(
        result["error"]
            .as_str()
            .unwrap()
            .contains("whose instance/number mapping is not supported"),
        "{result}"
    );
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&before));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        history
    );
}
