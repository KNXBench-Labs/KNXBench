//! Name-only edits preserve the full model and have one reversible effect.
// SPDX-License-Identifier: AGPL-3.0-or-later
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knx_core::*;
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

fn fixture() -> (Arc<knx_server::AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState {
        project: Default::default(),
        clean_project: Default::default(),
        last_saved_at: Default::default(),
        store_path: Default::default(),
        opaque: Default::default(),
        manufacturer_refs: Default::default(),
        command_stack: Default::default(),
        history_generation: Default::default(),
        import_counts: Default::default(),
        server_incarnation: "rename-test".into(),
        project_revision: Default::default(),
        project_incarnation: Default::default(),
        product_db: None,
        session_log: Default::default(),
        connector: Box::new(knx_server::fake::FakeConnector::discovering(Vec::new())),
        bus_session: Default::default(),
        group_address_style_publication: Default::default(),
        next_bus_session_id: Default::default(),
        line_scan_session: Default::default(),
        next_line_scan_session_id: Default::default(),
        device_download_plan: Default::default(),
        device_download: Default::default(),
        next_device_download_id: Default::default(),
        device_download_timing: Default::default(),
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
    };
    let mut project = Project::new(Language("en".into()));
    for id in 1..=2 {
        project.installations.push(Installation {
            id: InstallationId(id),
            name: format!("Installation {id}"),
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
            parameters: vec![],
            group_addresses: vec![GroupAddressEntry {
                id: GroupAddressId(id as u32),
                source: SourceRef {
                    path: "fixture".into(),
                    ets_id: format!("GA-{id}"),
                },
                name: "Original".into(),
                address: GroupAddress::from_raw(1),
                central: true,
                unfiltered: true,
                range: None,
                declared_dpt: Default::default(),
            }],
        });
    }
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "fixture".into(),
            ets_id: "D-1".into(),
        },
        name: "Original".into(),
        description: Some("Preserve me".into()),
        address: None,
        product_ref: "P".into(),
        program_ref: "A".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
        .devices
        .get_mut(DeviceId(1))
        .unwrap()
        .com_objects
        .push(ComObjectInstanceId(1));
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: SourceRef {
            path: "fixture".into(),
            ets_id: "CO-1".into(),
        },
        device: DeviceId(1),
        number: 0,
        text: Override::Absent,
        description: Override::Empty,
        dpt: Override::Value(Resolved {
            value: DptRef::parse("DPST-1-1").unwrap(),
            layer: Layer::Instance,
        }),
        flags: Default::default(),
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga: GroupAddressId(2),
            direction: Direction::Send,
        }],
        module_instance: None,
    });
    project.ids.next_com_object_instance_id().unwrap();
    project.installations[1].group_addresses[0].declared_dpt = Override::Value(Resolved {
        value: DptRef::parse("DPST-1-1").unwrap(),
        layer: Layer::Instance,
    });
    project.installations[1]
        .topology
        .unassigned
        .push(DeviceId(1));
    assert_eq!(project.ids.next_device_id().unwrap(), DeviceId(1));
    assert_eq!(
        project.ids.next_group_address_id().unwrap(),
        GroupAddressId(1)
    );
    assert_eq!(
        project.ids.next_group_address_id().unwrap(),
        GroupAddressId(2)
    );
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
                .body(if method == "GET" {
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

fn proposal(revision: u64, name: &str) -> Value {
    json!({"name": name, "expectedName": "Original", "serverIncarnation": "rename-test", "snapshotRevision": revision, "projectIncarnation": 0})
}

#[tokio::test]
async fn invalid_names_ambiguous_ids_stale_and_repeated_requests_preserve_model_and_history() {
    for path in ["/api/devices/1/name", "/api/group-addresses/2/name"] {
        for invalid in [
            "".into(),
            "   ".into(),
            "bad\nname".into(),
            "🛠".repeat(1025),
        ] {
            let (state, _dir) = fixture();
            let before = state.project.lock().unwrap().clone();
            let (status, error) = send(&state, "PATCH", path, proposal(0, &invalid)).await;
            assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{error}");
            assert_eq!(state.project.lock().unwrap().clone(), before);
            assert!(!state.command_stack.lock().unwrap().can_undo());
            assert_eq!(
                state
                    .project_revision
                    .load(std::sync::atomic::Ordering::Relaxed),
                0
            );
        }
        let (state, _dir) = fixture();
        let unchanged = send(&state, "PATCH", path, proposal(0, "Original")).await;
        assert_eq!(unchanged.0, StatusCode::OK);
        assert_eq!(unchanged.1["snapshot_revision"], 0);
        assert!(!state.command_stack.lock().unwrap().can_undo());
        assert_eq!(
            send(&state, "PATCH", path, proposal(0, "Original")).await.0,
            StatusCode::OK
        );
        assert_eq!(
            send(&state, "PATCH", path, proposal(99, "Stale")).await.0,
            StatusCode::CONFLICT
        );
        let mut wrong = proposal(0, "Wrong");
        wrong["serverIncarnation"] = json!("other-process");
        assert_eq!(
            send(&state, "PATCH", path, wrong).await.0,
            StatusCode::CONFLICT
        );
        assert_eq!(
            send(&state, "PATCH", path, proposal(0, "New")).await.0,
            StatusCode::OK
        );
        let after = state.project.lock().unwrap().clone();
        assert_eq!(
            send(&state, "PATCH", path, proposal(0, "New")).await.0,
            StatusCode::CONFLICT
        );
        assert_eq!(state.project.lock().unwrap().clone(), after);
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            (1, 0)
        );
        assert_eq!(
            send(&state, "POST", "/api/undo", Value::Null).await.0,
            StatusCode::OK
        );
        assert_eq!(
            send(&state, "PATCH", path, proposal(2, "Original")).await.0,
            StatusCode::OK
        );
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            (0, 1)
        );
    }
    for same_installation in [false, true] {
        let (state, _dir) = fixture();
        {
            let mut p = state.project.lock().unwrap();
            let p = p.as_mut().unwrap();
            let ga = p.installations[1].group_addresses[0].clone();
            p.installations[usize::from(same_installation)]
                .group_addresses
                .push(ga);
        }
        let before = state.project.lock().unwrap().clone();
        assert_eq!(
            send(
                &state,
                "PATCH",
                "/api/group-addresses/2/name",
                proposal(0, "No guess")
            )
            .await
            .0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(state.project.lock().unwrap().clone(), before);
        assert!(!state.command_stack.lock().unwrap().can_undo());
    }
}

#[tokio::test]
async fn native_history_recovers_unsaved_rename_and_exact_original_after_reopen() {
    for path in ["/api/devices/1/name", "/api/group-addresses/2/name"] {
        let (state, dir) = fixture();
        let before = state.project.lock().unwrap().clone().unwrap();
        let file = dir.path().join("rename.knxdb");
        knx_server::save_project_as(&state, &file).unwrap();
        let (_, tree) = send(&state, "GET", "/api/project", Value::Null).await;
        let body = proposal(tree["snapshot_revision"].as_u64().unwrap(), "Unsaved name");
        assert_eq!(send(&state, "PATCH", path, body).await.0, StatusCode::OK);
        let working = state.project.lock().unwrap().clone().unwrap();
        let (fresh, _fresh_dir) = fixture();
        let (status, reopened) = send(
            &fresh,
            "POST",
            "/api/project/open",
            json!({"path":file,"clientToken":"rename-reopen","discardChanges":true}),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{reopened}");
        assert_eq!(fresh.project.lock().unwrap().as_ref().unwrap(), &working);
        assert_eq!(
            fresh.command_stack.lock().unwrap().history_lengths(),
            (1, 0)
        );
        assert_eq!(reopened["is_modified"], true);
        assert_eq!(
            send(&fresh, "POST", "/api/undo", Value::Null).await.0,
            StatusCode::OK
        );
        assert_eq!(fresh.project.lock().unwrap().as_ref().unwrap(), &before);
        assert_eq!(
            send(&fresh, "POST", "/api/redo", Value::Null).await.0,
            StatusCode::OK
        );
        assert_eq!(fresh.project.lock().unwrap().as_ref().unwrap(), &working);
        knx_server::save_project(&fresh).unwrap();
        let (again, _dir2) = fixture();
        assert_eq!(
            send(
                &again,
                "POST",
                "/api/project/open",
                json!({"path":file,"clientToken":"rename-reopen-saved","discardChanges":true})
            )
            .await
            .0,
            StatusCode::OK
        );
        assert_eq!(again.project.lock().unwrap().as_ref().unwrap(), &working);
        assert!(
            !knx_server::current_project_tree(&again)
                .unwrap()
                .tree
                .is_modified
        );
    }
}

#[tokio::test]
async fn native_noop_and_failed_journal_commit_preserve_exact_durable_and_visible_state() {
    let (state, dir) = fixture();
    let file = dir.path().join("failure.knxdb");
    knx_server::save_project_as(&state, &file).unwrap();
    let (_, tree) = send(&state, "GET", "/api/project", Value::Null).await;
    let rev = tree["snapshot_revision"].as_u64().unwrap();
    let bytes = std::fs::read(&file).unwrap();
    assert_eq!(
        send(
            &state,
            "PATCH",
            "/api/devices/1/name",
            proposal(rev, "Original")
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
    assert_eq!(
        send(
            &state,
            "PATCH",
            "/api/devices/1/name",
            proposal(rev, "First rename")
        )
        .await
        .0,
        StatusCode::OK
    );
    let conn = knx_store::Connection::open(&file).unwrap();
    conn.execute_batch("CREATE UNIQUE INDEX refuse_rename_journal ON project_history_stack(side)")
        .unwrap();
    drop(conn);
    let bytes = std::fs::read(&file).unwrap();
    let before = state.project.lock().unwrap().clone();
    let generation = *state.history_generation.lock().unwrap();
    let mut body = proposal(rev + 1, "Must roll back");
    body["expectedName"] = json!("First rename");
    let (status, error) = send(&state, "PATCH", "/api/devices/1/name", body).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR, "{error}");
    assert!(
        error["error"].as_str().unwrap().contains("UNIQUE"),
        "{error}"
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
    assert_eq!(
        state
            .project_revision
            .load(std::sync::atomic::Ordering::Relaxed),
        rev + 1
    );
    assert_eq!(*state.history_generation.lock().unwrap(), generation);
    assert_eq!(std::fs::read(&file).unwrap(), bytes);
}

#[tokio::test]
async fn successful_replacement_changes_context_but_refused_replacement_does_not() {
    let (state, _dir) = fixture();
    assert_eq!(
        send(
            &state,
            "POST",
            "/api/project/new",
            json!({"name":"Replacement"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let (_, tree) = send(&state, "GET", "/api/project", Value::Null).await;
    assert_eq!(tree["project_incarnation"], 1);
    let mut body = proposal(tree["snapshot_revision"].as_u64().unwrap(), "Wrong project");
    assert_eq!(
        send(&state, "PATCH", "/api/devices/1/name", body.clone())
            .await
            .0,
        StatusCode::CONFLICT
    );
    body["projectIncarnation"] = json!(1);
    assert_eq!(
        send(&state, "PATCH", "/api/devices/1/name", body).await.0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let (state, _dir) = fixture();
    assert_eq!(
        send(
            &state,
            "PATCH",
            "/api/devices/1/name",
            proposal(0, "Modified")
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        send(
            &state,
            "POST",
            "/api/project/new",
            json!({"name":"Refused"})
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(
        state
            .project_incarnation
            .load(std::sync::atomic::Ordering::Relaxed),
        0
    );
}

#[tokio::test]
async fn malformed_missing_and_unknown_fields_cannot_mutate_or_clear_history() {
    let (state, _dir) = fixture();
    let before = state.project.lock().unwrap().clone();
    for missing in [
        "name",
        "expectedName",
        "serverIncarnation",
        "snapshotRevision",
        "projectIncarnation",
    ] {
        let mut body = proposal(0, "Bad");
        body.as_object_mut().unwrap().remove(missing);
        assert_eq!(
            send(&state, "PATCH", "/api/devices/1/name", body).await.0,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    let mut body = proposal(0, "Bad");
    body["central"] = json!(false);
    assert_eq!(
        send(&state, "PATCH", "/api/group-addresses/2/name", body)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        send(&state, "POST", "/api/devices/1/name", proposal(0, "Bad"))
            .await
            .0,
        StatusCode::METHOD_NOT_ALLOWED
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[tokio::test]
async fn foreign_project_incarnation_is_refused_without_history_or_model_changes() {
    let (state, _dir) = fixture();
    let before = state.project.lock().unwrap().clone();
    let mut body = proposal(0, "Wrong project");
    body["projectIncarnation"] = json!(999);
    let (status, error) = send(&state, "PATCH", "/api/devices/1/name", body).await;
    assert_eq!(status, StatusCode::CONFLICT, "{error}");
    assert_eq!(error["kind"], "renameConflict");
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert!(!state.command_stack.lock().unwrap().can_undo());
}

#[tokio::test]
async fn rename_second_installation_ga_changes_only_name_and_undo_restores_exact_model() {
    let (state, _dir) = fixture();
    let before = state.project.lock().unwrap().clone().unwrap();
    let (status, tree) = send(
        &state,
        "PATCH",
        "/api/group-addresses/2/name",
        proposal(0, "  Küche 🛠  "),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{tree}");
    let mut expected = before.clone();
    expected.installations[1].group_addresses[0].name = "  Küche 🛠  ".into();
    assert_eq!(state.project.lock().unwrap().as_ref().unwrap(), &expected);
    assert_eq!(tree["snapshot_revision"], 1);
    assert_eq!(
        send(&state, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(state.project.lock().unwrap().as_ref().unwrap(), &before);
    assert_eq!(
        send(&state, "POST", "/api/redo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(state.project.lock().unwrap().as_ref().unwrap(), &expected);
    assert!(state.bus_session.lock().await.is_none());
}
