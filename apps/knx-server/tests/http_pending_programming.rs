//! P3 over HTTP: the tree shows pending parts, edits mark them, users can mark by hand.
//!
//! ADR-0129. Every device node carries `pending_programming`; an edit marks
//! the parts it changes in the same undo step; `POST
//! /api/device/{id}/programming` marks all four parts programmed or not,
//! bound to the client's snapshot like the other bound writes, undoable and
//! logged.

use std::sync::{Arc, Mutex};

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};
use serde_json::{json, Value};
use tower::ServiceExt;

fn device(id: u32, octet: u8, programmed: bool) -> DeviceInstance {
    DeviceInstance {
        id: DeviceId(id),
        source: SourceRef {
            path: format!("device-{id}"),
            ets_id: format!("device-{id}"),
        },
        name: format!("Device {id}"),
        description: None,
        address: Some(format!("1.1.{octet}").parse().unwrap()),
        product_ref: "M-1_P-1".into(),
        program_ref: "H-1_HP-1".into(),
        commissioning: CommissioningState {
            individual_address_loaded: programmed,
            application_program_loaded: programmed,
            parameters_loaded: programmed,
            communication_part_loaded: programmed,
            ..CommissioningState::default()
        },
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    }
}

/// Device 1 programmed in every part (as imported from ETS), device 2 never.
fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(device(1, 1, true));
    project.devices.insert(device(2, 2, false));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Home".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1), DeviceId(2)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.ids = knx_core::project::IdAllocators::from_counts(2, 0, 0, 0, 0, 0, 0, 0, 0);
    project
}

#[allow(clippy::field_reassign_with_default)]
fn state() -> Arc<knx_server::AppState> {
    let mut state = knx_server::AppState::default();
    let project = project();
    *state.clean_project.lock().unwrap() = Some(project.clone());
    state.project = Mutex::new(Some(project));
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

fn node(tree: &Value, id: u32) -> Value {
    tree["installations"][0]["unassigned"]
        .as_array()
        .unwrap()
        .iter()
        .find(|d| d["id"] == id)
        .unwrap_or_else(|| panic!("device {id} in {tree}"))
        .clone()
}

fn pending(tree: &Value, id: u32) -> Value {
    node(tree, id)["pending_programming"].clone()
}

fn all(value: bool) -> Value {
    json!({
        "address": value,
        "application": value,
        "parameters": value,
        "group_addresses": value,
    })
}

async fn tree(state: &Arc<knx_server::AppState>) -> Value {
    let (status, tree) = call(state, "GET", "/api/project", None).await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    tree
}

fn mark(tree: &Value, programmed: bool) -> Value {
    json!({
        "programmed": programmed,
        "serverIncarnation": tree["server_incarnation"],
        "projectIncarnation": tree["project_incarnation"],
        "snapshotRevision": tree["snapshot_revision"],
    })
}

#[tokio::test]
async fn programming_marks_and_their_undo_survive_a_native_server_restart() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("programming.knxdb");
    let state = state();
    let (status, saved) = call(
        &state,
        "POST",
        "/api/project/save-as",
        Some(json!({ "path": path })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    let shown = tree(&state).await;
    let (status, marked) = call(
        &state,
        "POST",
        "/api/device/2/programming",
        Some(mark(&shown, true)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{marked}");
    let (status, edited) = call(
        &state,
        "POST",
        "/api/individual-address",
        Some(json!({ "deviceId": 1, "address": "1.1.9" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{edited}");
    let (status, saved) = call(&state, "POST", "/api/project/save", Some(json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{saved}");
    drop(state);

    let restarted = Arc::new(knx_server::AppState::default());
    let (status, opened) = call(
        &restarted,
        "POST",
        "/api/project/open",
        Some(json!({ "path": path })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{opened}");
    assert_eq!(pending(&opened, 1), pending(&edited, 1));
    assert_eq!(pending(&opened, 2), all(false));
    let (status, undone) = call(&restarted, "POST", "/api/undo", None).await;
    assert_eq!(status, StatusCode::OK, "{undone}");
    assert_eq!(pending(&undone, 1), all(false));
    assert_eq!(node(&undone, 1)["address"], "1.1.1");
    let (status, undone) = call(&restarted, "POST", "/api/undo", None).await;
    assert_eq!(status, StatusCode::OK, "{undone}");
    assert_eq!(pending(&undone, 2), all(true));
}

#[tokio::test]
async fn all_four_loaded_flags_keep_their_identity_through_native_reopen_and_projection() {
    let dir = tempfile::tempdir().unwrap();
    for mask in 0u8..16 {
        let state = state();
        {
            let mut project = state.project.lock().unwrap();
            let flags = &mut project
                .as_mut()
                .unwrap()
                .devices
                .get_mut(DeviceId(1))
                .unwrap()
                .commissioning;
            flags.individual_address_loaded = mask & 1 != 0;
            flags.application_program_loaded = mask & 2 != 0;
            flags.parameters_loaded = mask & 4 != 0;
            flags.communication_part_loaded = mask & 8 != 0;
            flags.medium_config_loaded = true;
        }
        let path = dir.path().join(format!("flags-{mask}.knxdb"));
        let (status, saved) = call(
            &state,
            "POST",
            "/api/project/save-as",
            Some(json!({ "path": path })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{saved}");
        drop(state);
        let reopened = Arc::new(knx_server::AppState::default());
        let (status, opened) = call(
            &reopened,
            "POST",
            "/api/project/open",
            Some(json!({ "path": path })),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{opened}");
        assert_eq!(
            pending(&opened, 1),
            json!({
                "address": mask & 1 == 0,
                "application": mask & 2 == 0,
                "parameters": mask & 4 == 0,
                "group_addresses": mask & 8 == 0,
            }),
            "mask {mask}: each bit belongs to its own part"
        );
        assert!(
            reopened
                .project
                .lock()
                .unwrap()
                .as_ref()
                .unwrap()
                .devices
                .get(DeviceId(1))
                .unwrap()
                .commissioning
                .medium_config_loaded,
            "the unrepresented fifth flag is preserved"
        );
    }
}

#[tokio::test]
async fn every_device_node_says_which_parts_are_pending() {
    let state = state();
    let tree = tree(&state).await;
    assert_eq!(pending(&tree, 1), all(false));
    assert_eq!(pending(&tree, 2), all(true), "never programmed");
}

#[tokio::test]
async fn an_address_change_marks_the_address_and_undo_clears_it() {
    let state = state();
    let (status, tree) = call(
        &state,
        "POST",
        "/api/individual-address",
        Some(json!({ "deviceId": 1, "address": "1.1.9" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    assert_eq!(
        pending(&tree, 1),
        json!({ "address": true, "application": false, "parameters": false, "group_addresses": false })
    );
    let (status, tree) = call(&state, "POST", "/api/undo", None).await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    assert_eq!(pending(&tree, 1), all(false));
}

#[tokio::test]
async fn marking_by_hand_is_bound_undoable_and_logged() {
    let state = state();
    let before = tree(&state).await;
    let (status, after) = call(
        &state,
        "POST",
        "/api/device/2/programming",
        Some(mark(&before, true)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{after}");
    assert_eq!(pending(&after, 2), all(false));
    assert_eq!(pending(&after, 1), all(false), "only the named device");
    assert_eq!(after["can_undo"], true, "{after}");

    let (status, log) = call(&state, "GET", "/api/log", None).await;
    assert_eq!(status, StatusCode::OK);
    let text = log.to_string();
    assert!(
        text.contains("“Device 2” (1.1.2) marked as programmed"),
        "{text}"
    );

    let (status, undone) = call(&state, "POST", "/api/undo", None).await;
    assert_eq!(status, StatusCode::OK, "{undone}");
    assert_eq!(pending(&undone, 2), all(true));

    let current = tree(&state).await;
    let (status, marked) = call(
        &state,
        "POST",
        "/api/device/1/programming",
        Some(mark(&current, false)),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{marked}");
    assert_eq!(pending(&marked, 1), all(true));
}

#[tokio::test]
async fn a_stale_or_unknown_mark_changes_nothing() {
    let state = state();
    let stale = tree(&state).await;
    let (status, _) = call(
        &state,
        "POST",
        "/api/device/2/programming",
        Some(mark(&stale, true)),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let snapshot = state.project.lock().unwrap().clone();
    let history = state.command_stack.lock().unwrap().history_lengths();

    let (status, body) = call(
        &state,
        "POST",
        "/api/device/2/programming",
        Some(mark(&stale, false)),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "programmingStale", "{body}");

    let current = tree(&state).await;
    let (status, body) = call(
        &state,
        "POST",
        "/api/device/99/programming",
        Some(mark(&current, false)),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{body}");
    assert_eq!(body["kind"], "programmingRefused", "{body}");

    let (status, _) = call(
        &state,
        "POST",
        "/api/device/2/programming",
        Some(json!({ "programmed": true })),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "the binding is required"
    );

    assert_eq!(*state.project.lock().unwrap(), snapshot);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        history
    );
}
