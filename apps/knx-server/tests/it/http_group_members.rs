//! Group-address members over HTTP: one undo step each, refusals change nothing.
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// The web UI adds members from the address side through the existing
// `/api/group-links` route. These tests pin what it relies on: one history
// step per action (`Both` included), cross-installation and duplicate
// refusals without any change, and native save/reopen keeping new and
// unrelated imported links exactly.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knx_core::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: "fixture".into(),
        ets_id: id.into(),
    }
}

fn group_address(id: u32, raw: u16) -> GroupAddressEntry {
    GroupAddressEntry {
        id: GroupAddressId(id),
        source: source(&format!("GA-{id}")),
        name: format!("Address {id}"),
        address: GroupAddress::from_raw(raw),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Default::default(),
    }
}

fn installation(
    id: u8,
    addresses: Vec<GroupAddressEntry>,
    unassigned: Vec<DeviceId>,
) -> Installation {
    Installation {
        id: InstallationId(id),
        name: format!("Installation {id}"),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned,
        },
        buildings: vec![],
        group_ranges: vec![],
        parameters: vec![],
        group_addresses: addresses,
    }
}

fn add_device(project: &mut Project, id: u32, links: Vec<GroupLink>) {
    project.devices.insert(DeviceInstance {
        id: DeviceId(id),
        source: source(&format!("D-{id}")),
        name: format!("Device {id}"),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "A".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![ComObjectInstanceId(id)],
        binary_data: vec![],
    });
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(id),
        source: source(&format!("CO-{id}")),
        device: DeviceId(id),
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: Default::default(),
        size: None,
        is_active: true,
        links,
        module_instance: None,
    });
}

/// Installation 1: address 1, devices 2 and 3. Installation 2: addresses 2
/// and 3, device 1 with an imported Send link to address 2. Every device is
/// placed, because the native store refuses to save an unplaced one (it has
/// no installation to be saved under); the core's unplaced-device rule is
/// pinned by its own tests.
fn fixture() -> (Arc<knx_server::AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        diagnostic_results: Default::default(),
        support_results: Default::default(),
        project: Default::default(),
        clean_project: Default::default(),
        last_saved_at: Default::default(),
        store_path: Default::default(),
        view_path_key: Default::default(),
        opaque: Default::default(),
        manufacturer_refs: Default::default(),
        command_stack: Default::default(),
        history_generation: Default::default(),
        import_counts: Default::default(),
        import_report_available: std::sync::atomic::AtomicBool::new(true),
        import_report_historical: std::sync::atomic::AtomicBool::new(false),
        server_incarnation: "members-test".into(),
        project_revision: Default::default(),
        project_incarnation: Default::default(),
        product_db: None,
        catalogue_jobs: Arc::new(knx_server::CatalogueUpdateJobs::new(
            knx_server::CatalogueJobIncarnation::from_bytes([1; 16]),
        )),
        product_db_unavailable: None,
        session_log: Default::default(),
        connector: Box::new(knx_server::fake::FakeConnector::discovering(Vec::new())),
        tunnel_return_path: Default::default(),
        bus_session: Default::default(),
        group_address_style_publication: Default::default(),
        next_bus_session_id: Default::default(),
        line_scan_session: Default::default(),
        next_line_scan_session_id: Default::default(),
        device_download_plan: Default::default(),
        plugin_data_plan: Default::default(),
        device_download: Default::default(),
        device_system2: Arc::new(knx_server::System2Service::new(dir.path())),
        recordings: Arc::new(knx_server::operation_recordings::store(dir.path(), true)),
        next_device_download_id: Default::default(),
        device_download_timing: Default::default(),
        system2_timing: Default::default(),
        diagnostic_lease_dir: None,
        address_programming: Default::default(),
        next_address_programming_id: Default::default(),
        one_shot_activity: Default::default(),
        address_programming_timing: Default::default(),
        address_programming_pause: Default::default(),
        load_operations: Default::default(),
        data_dir: dir.path().to_path_buf(),
        settings_lock: Default::default(),
        achievements_lock: Default::default(),
        catalog_requests: Default::default(),
        legacy_password: None,
        stop_admission: Default::default(),
    };
    let mut project = Project::new(Language("en".into()));
    project.installations.push(installation(
        1,
        vec![group_address(1, 1)],
        vec![DeviceId(2), DeviceId(3)],
    ));
    project.installations.push(installation(
        2,
        vec![group_address(2, 2), group_address(3, 3)],
        vec![DeviceId(1)],
    ));
    add_device(
        &mut project,
        1,
        vec![GroupLink {
            ga: GroupAddressId(2),
            direction: Direction::Send,
        }],
    );
    add_device(&mut project, 2, vec![]);
    add_device(&mut project, 3, vec![]);
    for _ in 0..3 {
        project.ids.next_device_id().unwrap();
        project.ids.next_com_object_instance_id().unwrap();
        project.ids.next_group_address_id().unwrap();
    }
    *state.clean_project.lock().unwrap() = Some(project.clone());
    *state.project.lock().unwrap() = Some(project);
    (Arc::new(state), dir)
}

async fn send(
    state: &Arc<knx_server::AppState>,
    method: &str,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = knx_server::app(state.clone(), None)
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(if body.is_null() {
                    Body::empty()
                } else {
                    Body::from(body.to_string())
                })
                .unwrap(),
        )
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

fn member(object: u32, ga: u32, direction: &str) -> Value {
    json!({ "comObjectId": object, "gaId": ga, "direction": direction })
}

fn links(state: &knx_server::AppState, object: u32) -> Vec<GroupLink> {
    let guard = state.project.lock().unwrap();
    let project = guard.as_ref().unwrap();
    project
        .devices
        .com_object(ComObjectInstanceId(object))
        .unwrap()
        .links
        .clone()
}

fn history(state: &knx_server::AppState) -> (usize, usize) {
    state.command_stack.lock().unwrap().history_lengths()
}

fn link(ga: u32, direction: Direction) -> GroupLink {
    GroupLink {
        ga: GroupAddressId(ga),
        direction,
    }
}

#[tokio::test]
async fn each_member_action_is_one_undo_step_and_survives_native_reopen() {
    let (state, dir) = fixture();
    let file = dir.path().join("members.knxdb");
    knx_server::save_project_as(&state, &file).unwrap();
    let imported = links(&state, 1);

    let (status, body) = send(&state, "POST", "/api/group-links", member(1, 3, "Receive")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(history(&state), (1, 0));
    assert_eq!(
        links(&state, 1),
        vec![link(2, Direction::Send), link(3, Direction::Receive)]
    );

    // `Both` is one batch: one step adds both links, one undo removes both.
    let (status, body) = send(&state, "POST", "/api/group-links", member(3, 1, "Both")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(history(&state), (2, 0));
    assert_eq!(
        links(&state, 3),
        vec![link(1, Direction::Send), link(1, Direction::Receive)]
    );
    assert_eq!(
        send(&state, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(history(&state), (1, 1));
    assert!(links(&state, 3).is_empty());
    assert_eq!(
        send(&state, "POST", "/api/redo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(history(&state), (2, 0));
    assert_eq!(links(&state, 3).len(), 2);

    // Removing a member is one step too, and undo restores it in place.
    let (status, _) = send(&state, "DELETE", "/api/group-links", member(1, 2, "Send")).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(links(&state, 1), vec![link(3, Direction::Receive)]);
    assert_eq!(
        send(&state, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        links(&state, 1),
        vec![link(2, Direction::Send), link(3, Direction::Receive)]
    );
    assert_eq!(
        links(&state, 1)[0],
        imported[0],
        "the imported link is untouched"
    );
    assert!(
        links(&state, 2).is_empty(),
        "an unrelated object is untouched"
    );

    let working = state.project.lock().unwrap().clone().unwrap();
    knx_server::save_project(&state).unwrap();
    let (fresh, _fresh_dir) = fixture();
    let (status, reopened) = send(
        &fresh,
        "POST",
        "/api/project/open",
        json!({"path": file, "clientToken": "members-reopen", "discardChanges": true}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{reopened}");
    assert_eq!(fresh.project.lock().unwrap().as_ref().unwrap(), &working);
    assert_eq!(reopened["is_modified"], false);
}

#[tokio::test]
async fn refused_member_links_change_neither_project_nor_history() {
    let (state, _dir) = fixture();
    let before = state.project.lock().unwrap().clone().unwrap();
    let (_, tree_before) = send(&state, "GET", "/api/project", Value::Null).await;
    for (object, ga, direction, why) in [
        (
            2,
            2,
            "Send",
            "device of installation 1, address of installation 2",
        ),
        (1, 2, "Send", "duplicate of the imported link"),
        (1, 2, "Both", "Both while its Send half exists"),
        (1, 99, "Send", "unknown address"),
        (99, 2, "Send", "unknown object"),
    ] {
        let (status, body) = send(
            &state,
            "POST",
            "/api/group-links",
            member(object, ga, direction),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{why}: {body}");
        assert_eq!(
            state.project.lock().unwrap().as_ref().unwrap(),
            &before,
            "{why}"
        );
        assert_eq!(history(&state), (0, 0), "{why}");
    }
    let (_, tree_after) = send(&state, "GET", "/api/project", Value::Null).await;
    assert_eq!(tree_after["installations"], tree_before["installations"]);
    assert_eq!(tree_after["is_modified"], false);
}
