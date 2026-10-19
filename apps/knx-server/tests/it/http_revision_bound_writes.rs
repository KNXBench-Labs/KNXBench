//! Parameter writes and installation renames bound to the caller's project revision.
//!
//! Goal A2: a bound request names the server incarnation, project incarnation
//! and snapshot revision it was prepared against, plus the value (or name) it
//! expects to replace. The server checks all of it under the same lock that
//! applies, persists and publishes the change. A stale or mismatched request is
//! a typed 409 with no model, file, undo or revision effect. A request without
//! any binding keeps the documented legacy behaviour; a partial or misspelled
//! binding is refused instead of silently becoming unbound.

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

const PROGRAM: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-1">
<ApplicationPrograms><ApplicationProgram Id="A-1" Name="P" ApplicationVersion="1" MaskVersion="MV-0701">
<Static>
<ParameterTypes>
  <ParameterType Id="PT-Num" Name="num"><TypeNumber maxInclusive="255" minInclusive="0" SizeInBit="8" Type="unsignedInt" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="P-STORED" Name="s" Text="Stored" ParameterType="PT-Num" Value="1" />
  <Parameter Id="P-DEFAULT" Name="d" Text="Default" ParameterType="PT-Num" Value="4" />
</Parameters>
<ParameterRefs>
  <ParameterRef Id="R-STORED" RefId="P-STORED" />
  <ParameterRef Id="R-DEFAULT" RefId="P-DEFAULT" />
</ParameterRefs>
</Static>
<Dynamic>
  <ParameterRefRef RefId="R-STORED" />
  <ParameterRefRef RefId="R-DEFAULT" />
</Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn products() -> (tempfile::TempDir, knx_productdb::Connection) {
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/Hardware.xml", HARDWARE.as_bytes()).unwrap();
    knx_productdb::ingest_file(&conn, "M-1/A.xml", PROGRAM.as_bytes()).unwrap();
    (dir, conn)
}

fn installation(id: u8, name: &str) -> Installation {
    Installation {
        id: InstallationId(id),
        name: name.into(),
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
    }
}

/// Device 1 lives in the *second* installation, which also holds its one
/// stored parameter value (`R-STORED` = "9", row id 5). The first
/// installation is empty, so a first-installation assumption is visible.
fn project() -> Project {
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
    project.installations.push(installation(0, "Home"));
    let mut garage = installation(1, "Garage");
    garage.topology.unassigned.push(DeviceId(1));
    garage.parameters.push(ParameterInstance {
        id: ParameterInstanceId(5),
        device: DeviceId(1),
        source: SourceRef {
            path: "device-1".into(),
            ets_id: "R-STORED".into(),
        },
        raw: "9".into(),
    });
    project.installations.push(garage);
    project.ids = knx_core::project::IdAllocators::from_counts(1, 0, 0, 0, 0, 0, 0, 5, 0);
    project
}

#[allow(clippy::field_reassign_with_default)]
fn state(products: knx_productdb::Connection) -> Arc<knx_server::AppState> {
    state_in(products, None)
}

#[allow(clippy::field_reassign_with_default)]
fn state_in(
    products: knx_productdb::Connection,
    data_dir: Option<&std::path::Path>,
) -> Arc<knx_server::AppState> {
    let mut state = knx_server::AppState::default();
    if let Some(dir) = data_dir {
        state.data_dir = dir.to_path_buf();
    }
    state.product_db = Some(Mutex::new(products));
    let project = project();
    *state.clean_project.lock().unwrap() = Some(project.clone());
    *state.project.lock().unwrap() = Some(project);
    Arc::new(state)
}

async fn call(
    state: &Arc<knx_server::AppState>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(body) => {
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = knx_server::app(state.clone(), None)
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

/// What a client read: the project context of `GET /api/project`.
#[derive(Clone)]
struct Context {
    server: Value,
    project: Value,
    revision: Value,
}

async fn context(state: &Arc<knx_server::AppState>) -> Context {
    let (status, tree) = call(state, "GET", "/api/project", None).await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let context = Context {
        server: tree["server_incarnation"].clone(),
        project: tree["project_incarnation"].clone(),
        revision: tree["snapshot_revision"].clone(),
    };
    assert!(
        context.server.is_string() && context.project.is_u64() && context.revision.is_u64(),
        "the tree carries its context: {tree}"
    );
    context
}

fn bound_parameter(context: &Context, ets_id: &str, raw: &str, expected: Value) -> Value {
    json!({
        "etsId": ets_id,
        "raw": raw,
        "serverIncarnation": context.server,
        "projectIncarnation": context.project,
        "snapshotRevision": context.revision,
        "expectedValue": expected,
    })
}

fn bound_rename(context: &Context, name: &str, expected: &str) -> Value {
    json!({
        "name": name,
        "expectedName": expected,
        "serverIncarnation": context.server,
        "projectIncarnation": context.project,
        "snapshotRevision": context.revision,
    })
}

/// Everything a refused request must leave exactly as it was.
#[derive(Debug, PartialEq)]
struct Observable {
    project: Option<Project>,
    undo_redo: (usize, usize),
    revision: u64,
}

fn observe(state: &knx_server::AppState) -> Observable {
    Observable {
        project: state.project.lock().unwrap().clone(),
        undo_redo: state.command_stack.lock().unwrap().history_lengths(),
        revision: state
            .project_revision
            .load(std::sync::atomic::Ordering::SeqCst),
    }
}

fn stored(state: &knx_server::AppState) -> Vec<(u8, u32, String, String)> {
    let project = state.project.lock().unwrap();
    project
        .as_ref()
        .unwrap()
        .installations
        .iter()
        .flat_map(|i| {
            i.parameters
                .iter()
                .map(move |p| (i.id.0, p.id.0, p.source.ets_id.clone(), p.raw.clone()))
        })
        .collect()
}

const PARAMETERS: &str = "/api/device/1/parameters";

fn panel_field<'a>(panel: &'a Value, ets_id: &str) -> &'a Value {
    panel["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|section| section["fields"].as_array().unwrap())
        .find(|field| field["etsId"] == ets_id)
        .unwrap_or_else(|| panic!("{ets_id} is listed: {panel}"))
}

/// The panel is the authority a bound write compares against, so it must
/// show the value stored in the device's own (here: second) installation.
#[tokio::test]
async fn the_panel_shows_a_value_stored_in_the_devices_own_installation() {
    let (_dir, products) = products();
    let state = state(products);

    let (status, panel) = call(&state, "GET", PARAMETERS, None).await;

    assert_eq!(status, StatusCode::OK, "{panel}");
    assert_eq!(panel_field(&panel, "R-STORED")["value"], "9");
    assert_eq!(panel_field(&panel, "R-DEFAULT")["value"], "4");
}

#[tokio::test]
async fn a_bound_write_overwrites_the_row_in_its_own_installation_without_spending_an_id() {
    let (_dir, products) = products();
    let state = state(products);
    let ids_before = state.project.lock().unwrap().as_ref().unwrap().ids.clone();
    let ctx = context(&state).await;

    let (status, panel) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(bound_parameter(&ctx, "R-STORED", "42", json!("9"))),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{panel}");
    assert_eq!(stored(&state), vec![(1, 5, "R-STORED".into(), "42".into())]);
    assert_eq!(
        state.project.lock().unwrap().as_ref().unwrap().ids,
        ids_before,
        "overwriting an existing row must not allocate a parameter id"
    );
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
}

#[tokio::test]
async fn a_bound_write_of_a_default_value_creates_its_row_in_the_devices_installation() {
    let (_dir, products) = products();
    let state = state(products);
    let ctx = context(&state).await;

    let (status, panel) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(bound_parameter(&ctx, "R-DEFAULT", "7", json!("4"))),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{panel}");
    assert_eq!(
        stored(&state),
        vec![
            (1, 5, "R-STORED".into(), "9".into()),
            (1, 6, "R-DEFAULT".into(), "7".into()),
        ]
    );
}

#[tokio::test]
async fn the_second_of_two_clients_on_one_revision_gets_a_typed_conflict_and_changes_nothing() {
    let (_dir, products) = products();
    let state = state(products);
    let first = context(&state).await;
    let second = first.clone();

    let (status, _) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(bound_parameter(&first, "R-STORED", "42", json!("9"))),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let after_first = observe(&state);

    let (status, error) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(bound_parameter(&second, "R-STORED", "13", json!("9"))),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert_eq!(error["kind"], "parameterWriteConflict");
    assert_eq!(observe(&state), after_first);
    assert_eq!(stored(&state), vec![(1, 5, "R-STORED".into(), "42".into())]);
}

#[tokio::test]
async fn a_value_that_is_no_longer_the_expected_one_is_a_conflict() {
    let (_dir, products) = products();
    let state = state(products);
    let ctx = context(&state).await;
    let before = observe(&state);

    let (status, error) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(bound_parameter(&ctx, "R-STORED", "42", json!("8"))),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert_eq!(error["kind"], "parameterWriteConflict");
    assert_eq!(observe(&state), before);
}

#[tokio::test]
async fn a_write_prepared_for_another_project_is_a_conflict_even_at_the_same_revision() {
    let (_dir, products) = products();
    let state = state(products);
    let ctx = context(&state).await;
    // Another client opened a different project: new project incarnation,
    // same server, and (here) the very same snapshot revision number.
    state
        .project_incarnation
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let before = observe(&state);

    let (status, error) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(bound_parameter(&ctx, "R-STORED", "42", json!("9"))),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert_eq!(observe(&state), before);

    let mut other_server = ctx.clone();
    other_server.server = json!("another-server-run");
    other_server.project = json!(state
        .project_incarnation
        .load(std::sync::atomic::Ordering::SeqCst));
    let (status, _) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(bound_parameter(&other_server, "R-STORED", "42", json!("9"))),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(observe(&state), before);
}

#[tokio::test]
async fn a_failed_persistence_leaves_model_history_revision_and_file_unchanged() {
    let (_dir, products) = products();
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(products, Some(dir.path()));
    let path = dir.path().join("project.knxdb");
    let (status, saved) = call(
        &state,
        "POST",
        "/api/project/save-as",
        Some(json!({ "path": path.display().to_string() })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let ctx = context(&state).await;
    // A second native writer moves the file's working-history generation, so
    // this server's next persistence is refused (stale generation).
    let snapshot = {
        let read = knx_store::open_existing_read_only(&path).unwrap();
        knx_store::project_history::NativeSnapshot::read(&read.conn).unwrap()
    };
    let generation = *state.history_generation.lock().unwrap();
    knx_app::project_history::persist_working(
        &path,
        &snapshot,
        &knx_core::CommandStack::new(),
        generation,
    )
    .unwrap();
    let file = std::fs::read(&path).unwrap();
    let before = observe(&state);

    let (status, error) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(bound_parameter(&ctx, "R-STORED", "42", json!("9"))),
    )
    .await;

    assert!(!status.is_success(), "{status} {error}");
    assert_ne!(
        status,
        StatusCode::CONFLICT,
        "a storage failure is not a revision conflict"
    );
    assert_eq!(observe(&state), before);
    assert_eq!(std::fs::read(&path).unwrap(), file);

    let (status, error) = call(
        &state,
        "PATCH",
        GARAGE,
        Some(bound_rename(&ctx, "Carport", "Garage")),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{error}");
    assert_eq!(observe(&state), before);
    assert_eq!(std::fs::read(&path).unwrap(), file);
}

#[tokio::test]
async fn partial_or_misspelled_bindings_are_refused_not_treated_as_unbound() {
    let (_dir, products) = products();
    let state = state(products);
    let ctx = context(&state).await;
    let before = observe(&state);
    let mut partial = bound_parameter(&ctx, "R-STORED", "42", json!("9"));
    partial.as_object_mut().unwrap().remove("snapshotRevision");
    let mut misspelled = json!({ "etsId": "R-STORED", "raw": "42" });
    misspelled["snapshotRevison"] = ctx.revision.clone();
    let value_only = json!({ "etsId": "R-STORED", "raw": "42", "expectedValue": "9" });

    for body in [partial, misspelled, value_only] {
        let (status, error) = call(&state, "POST", PARAMETERS, Some(body.clone())).await;
        assert!(
            status == StatusCode::BAD_REQUEST || status == StatusCode::UNPROCESSABLE_ENTITY,
            "{body}: {status} {error}"
        );
        assert_eq!(observe(&state), before, "{body}");
    }
}

/// Documented legacy boundary (contract: revision-bound writes): an
/// unbound request keeps working until its last caller is migrated.
#[tokio::test]
async fn an_unbound_legacy_write_keeps_its_behaviour() {
    let (_dir, products) = products();
    let state = state(products);

    let (status, panel) = call(
        &state,
        "POST",
        PARAMETERS,
        Some(json!({ "etsId": "R-STORED", "raw": "42" })),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{panel}");
    assert_eq!(stored(&state), vec![(1, 5, "R-STORED".into(), "42".into())]);
}

const GARAGE: &str = "/api/installations/1";

#[tokio::test]
async fn a_bound_installation_rename_is_trimmed_and_applied_once() {
    let (_dir, products) = products();
    let state = state(products);
    let ctx = context(&state).await;

    let (status, tree) = call(
        &state,
        "PATCH",
        GARAGE,
        Some(bound_rename(&ctx, " Carport ", "Garage")),
    )
    .await;

    assert_eq!(status, StatusCode::OK, "{tree}");
    assert_eq!(tree["installations"][1]["name"], "Carport");
    assert_eq!(tree["installations"][0]["name"], "Home");
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
}

#[tokio::test]
async fn stale_or_mismatched_installation_renames_are_conflicts_without_effect() {
    let (_dir, products) = products();
    let state = state(products);
    let first = context(&state).await;

    let (status, _) = call(
        &state,
        "PATCH",
        GARAGE,
        Some(bound_rename(&first, "Carport", "Garage")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let after_first = observe(&state);

    let (status, error) = call(
        &state,
        "PATCH",
        GARAGE,
        Some(bound_rename(&first, "Workshop", "Garage")),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert_eq!(error["kind"], "renameConflict");
    assert_eq!(observe(&state), after_first);

    let fresh = context(&state).await;
    let (status, error) = call(
        &state,
        "PATCH",
        GARAGE,
        Some(bound_rename(&fresh, "Workshop", "Garage")),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert_eq!(observe(&state), after_first);
}

#[tokio::test]
async fn bound_installation_renames_refuse_partial_bindings_blank_names_and_unknown_ids() {
    let (_dir, products) = products();
    let state = state(products);
    let ctx = context(&state).await;
    let before = observe(&state);
    let mut partial = bound_rename(&ctx, "Carport", "Garage");
    partial.as_object_mut().unwrap().remove("expectedName");
    let misspelled = json!({ "name": "Carport", "expectedNmae": "Garage" });

    for (uri, body) in [
        (GARAGE, partial),
        (GARAGE, misspelled),
        (GARAGE, bound_rename(&ctx, "   ", "Garage")),
        (
            "/api/installations/7",
            bound_rename(&ctx, "Carport", "Garage"),
        ),
    ] {
        let (status, error) = call(&state, "PATCH", uri, Some(body.clone())).await;
        assert!(
            status.is_client_error() && status != StatusCode::NOT_FOUND,
            "{uri} {body}: {status} {error}"
        );
        assert_eq!(observe(&state), before, "{uri} {body}");
    }
}
