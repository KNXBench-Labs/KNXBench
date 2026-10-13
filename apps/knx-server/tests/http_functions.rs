//! Project Functions over HTTP: bound edits, typed refusals and no effect on failure.
//!
//! Project Functions over HTTP (ADR-0124, Goal A4): the read projection and
//! revision-bound edits. Every edit names the snapshot it was prepared
//! against; a stale one is a typed 409, a refused command a typed 422 and a
//! persistence failure a 500, all without model, undo or revision effect.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, CompletionStatus, GroupAddress,
    GroupAddressEntry, GroupAddressId, Installation, InstallationId, Language, Project, SourceRef,
    Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

const READ: &str = "/api/project/functions";
const EDIT: &str = "/api/project/functions/edit";

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: id.into(),
        ets_id: id.into(),
    }
}

fn room(id: u32, name: &str) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: source(name),
        name: name.into(),
        number: None,
        kind: BuildingPartType::Room,
        default_line: None,
        completion: CompletionStatus::Undefined,
        children: vec![],
        devices: vec![],
        parent: None,
    }
}

/// One installation with two rooms (1, 2) and two group addresses (10, 11).
fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Home".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Undefined,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![room(1, "Kitchen"), room(2, "Hall")],
        group_ranges: vec![],
        group_addresses: [(10, 0x0801_u16, "Light"), (11, 0x0802, "Light status")]
            .into_iter()
            .map(|(id, raw, name)| GroupAddressEntry {
                id: GroupAddressId(id),
                source: source(name),
                name: name.into(),
                address: GroupAddress::from_raw(raw),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
            })
            .collect(),
        parameters: vec![],
    });
    project.ids = knx_core::project::IdAllocators::from_counts(0, 0, 0, 0, 0, 11, 2, 0, 0);
    project
}

fn state_in(data_dir: Option<&std::path::Path>) -> Arc<knx_server::AppState> {
    let mut state = knx_server::AppState::default();
    if let Some(dir) = data_dir {
        state.data_dir = dir.to_path_buf();
    }
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
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!(String::from_utf8_lossy(&bytes))),
    )
}

async fn read(state: &Arc<knx_server::AppState>) -> Value {
    let (status, view) = call(state, "GET", READ, None).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    view
}

/// An edit bound to what `view` showed.
fn edit(view: &Value, action: Value) -> Value {
    json!({
        "serverIncarnation": view["serverIncarnation"],
        "projectIncarnation": view["projectIncarnation"],
        "snapshotRevision": view["snapshotRevision"],
        "action": action,
    })
}

async fn apply(state: &Arc<knx_server::AppState>, view: &Value, action: Value) -> Value {
    let (status, next) = call(state, "POST", EDIT, Some(edit(view, action))).await;
    assert_eq!(status, StatusCode::OK, "{next}");
    next
}

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

fn kind(error: &Value) -> String {
    error.to_string()
}

#[tokio::test]
async fn a_bound_client_creates_fills_moves_and_removes_a_function() {
    let state = state_in(None);
    let empty = read(&state).await;
    assert_eq!(empty["functions"], json!([]));

    let created = apply(
        &state,
        &empty,
        json!({"kind": "create", "location": 1, "details": {"name": "Kitchen light", "functionType": "FT-1", "completion": "Editing"}}),
    )
    .await;
    assert_eq!(created["createdId"], 1);
    let first = apply(
        &state,
        &created,
        json!({"kind": "addMember", "functionId": 1, "groupAddressId": 10, "role": "SwitchOnOff"}),
    )
    .await;
    assert_eq!(first["createdId"], 1);
    let both = apply(
        &state,
        &first,
        json!({"kind": "addMember", "functionId": 1, "groupAddressId": 11, "role": "InfoOnOff", "position": 0}),
    )
    .await;
    let function = &read(&state).await["functions"][0];
    assert_eq!(function["location"]["name"], "Kitchen");
    assert_eq!(function["completion"], "Editing");
    let members: Vec<_> = function["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|m| {
            (
                m["groupAddressId"].clone(),
                m["address"].clone(),
                m["role"].clone(),
            )
        })
        .collect();
    assert_eq!(
        members,
        [
            (json!(11), json!("1/0/2"), json!("InfoOnOff")),
            (json!(10), json!("1/0/1"), json!("SwitchOnOff")),
        ]
    );

    let moved = apply(
        &state,
        &both,
        json!({"kind": "move", "id": 1, "location": 2}),
    )
    .await;
    assert_eq!(moved["functions"][0]["location"]["name"], "Hall");
    let renamed = apply(
        &state,
        &moved,
        json!({"kind": "update", "id": 1, "details": {"name": "Hall light"}}),
    )
    .await;
    assert_eq!(renamed["functions"][0]["completion"], "Undefined");
    let fewer = apply(
        &state,
        &renamed,
        json!({"kind": "removeMember", "functionId": 1, "memberId": 2}),
    )
    .await;
    assert_eq!(
        fewer["functions"][0]["members"].as_array().unwrap().len(),
        1
    );
    let gone = apply(&state, &fewer, json!({"kind": "delete", "id": 1})).await;
    assert_eq!(gone["functions"], json!([]));

    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (7, 0)
    );
    let (status, _) = call(&state, "POST", "/api/undo", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read(&state).await["functions"][0]["name"], "Hall light");
}

#[tokio::test]
async fn a_stale_binding_is_a_typed_conflict_and_changes_nothing() {
    let state = state_in(None);
    let before_view = read(&state).await;
    apply(
        &state,
        &before_view,
        json!({"kind": "create", "location": 1, "details": {"name": "A"}}),
    )
    .await;
    let before = observe(&state);
    let (status, error) = call(
        &state,
        "POST",
        EDIT,
        Some(edit(
            &before_view,
            json!({"kind": "create", "location": 2, "details": {"name": "B"}}),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert!(kind(&error).contains("functionStale"), "{error}");
    assert_eq!(observe(&state), before);
}

#[tokio::test]
async fn refused_edits_are_typed_and_do_not_use_up_ids() {
    let state = state_in(None);
    let view = read(&state).await;
    let before = observe(&state);
    for action in [
        json!({"kind": "create", "location": 1, "details": {"name": "  "}}),
        json!({"kind": "create", "location": 99, "details": {"name": "Nowhere"}}),
        json!({"kind": "addMember", "functionId": 1, "groupAddressId": 10}),
        json!({"kind": "delete", "id": 1}),
    ] {
        let (status, error) = call(&state, "POST", EDIT, Some(edit(&view, action.clone()))).await;
        assert_eq!(
            status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{action}: {error}"
        );
        assert!(kind(&error).contains("functionRefused"), "{error}");
        assert_eq!(observe(&state), before, "{action}");
    }
    let created = apply(
        &state,
        &view,
        json!({"kind": "create", "location": 1, "details": {"name": "Light"}}),
    )
    .await;
    assert_eq!(created["createdId"], 1, "refusals allocated nothing");
    let with_member = apply(
        &state,
        &created,
        json!({"kind": "addMember", "functionId": 1, "groupAddressId": 10}),
    )
    .await;
    let before = observe(&state);
    let (status, error) = call(
        &state,
        "POST",
        EDIT,
        Some(edit(
            &with_member,
            json!({"kind": "addMember", "functionId": 1, "groupAddressId": 99}),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
    assert_eq!(observe(&state), before);

    // A member's group address cannot be deleted out from under the Function.
    let (status, error) = call(&state, "DELETE", "/api/group-addresses/10", None).await;
    assert!(!status.is_success(), "{error}");
    assert!(kind(&error).contains("part of a function"), "{error}");
    assert_eq!(observe(&state), before);
}

#[tokio::test]
async fn misspelt_fields_and_unknown_statuses_are_rejected_before_any_effect() {
    let state = state_in(None);
    let view = read(&state).await;
    let before = observe(&state);
    let mut misspelt_binding = edit(
        &view,
        json!({"kind": "create", "location": 1, "details": {"name": "A"}}),
    );
    misspelt_binding["snapshotRevison"] = misspelt_binding["snapshotRevision"].take();
    misspelt_binding
        .as_object_mut()
        .unwrap()
        .remove("snapshotRevision");
    for body in [
        misspelt_binding,
        edit(
            &view,
            json!({"kind": "create", "location": 1, "details": {"name": "A", "completion": "Done"}}),
        ),
        edit(
            &view,
            json!({"kind": "create", "location": 1, "details": {"name": "A"}, "position": 0}),
        ),
        edit(
            &view,
            json!({"kind": "addMember", "functionId": 1, "group_address_id": 10}),
        ),
    ] {
        let (status, error) = call(&state, "POST", EDIT, Some(body.clone())).await;
        assert!(status.is_client_error(), "{body}: {status} {error}");
        assert_eq!(observe(&state), before, "{body}");
    }
}

#[tokio::test]
async fn a_failed_persistence_is_500_and_publishes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let state = state_in(Some(dir.path()));
    let path = dir.path().join("project.knxdb");
    let (status, saved) = call(
        &state,
        "POST",
        "/api/project/save-as",
        Some(json!({ "path": path.display().to_string() })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let view = read(&state).await;
    // A second native writer moves the working-history generation, so this
    // server's next persistence is refused.
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
        EDIT,
        Some(edit(
            &view,
            json!({"kind": "create", "location": 1, "details": {"name": "A"}}),
        )),
    )
    .await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{error}");
    assert_eq!(observe(&state), before);
    assert_eq!(std::fs::read(&path).unwrap(), file);
}
