//! Verifies native version actions, consent, stale guards and edit rollback over HTTP.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn request(
    app: &axum::Router,
    method: &str,
    path: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
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
        if bytes.is_empty() {
            Value::Null
        } else {
            serde_json::from_slice(&bytes).unwrap()
        },
    )
}

async fn fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    Arc<knx_server::AppState>,
    axum::Router,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.knxdb");
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/new",
            json!({"name": "Version witness"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "POST", "/api/project/save-as", json!({"path": path}))
            .await
            .0,
        StatusCode::OK
    );
    (dir, path, state, app)
}

fn binding(view: &Value, confirmed: bool) -> Value {
    json!({"serverIncarnation": view["serverIncarnation"], "snapshotRevision": view["snapshotRevision"],
        "generation": view["generation"], "confirmed": confirmed})
}

#[tokio::test]
async fn retained_context_and_decode_limits_are_exposed_separately_from_model_limit() {
    let (_dir, _path, _state, app) = fixture().await;
    let (status, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        view["limits"]["maxImageBytes"],
        knx_store::project_history::MAX_IMAGE_BYTES
    );
    assert_eq!(
        view["limits"]["maxContextBytes"],
        knx_store::project_history::MAX_CONTEXT_BYTES
    );
    assert_eq!(
        view["limits"]["maxDecodedImageBytes"],
        knx_store::project_history::MAX_DECODED_IMAGE_BYTES
    );
    assert_eq!(
        view["limits"]["maxDecodedHistoryBytes"],
        knx_store::project_history::MAX_DECODED_HISTORY_BYTES
    );
}

#[tokio::test]
async fn named_version_restore_preserves_unsaved_pre_restore_state_and_clears_edit_stack() {
    let (_dir, _path, state, app) = fixture().await;
    let before = state.project.lock().unwrap().clone().unwrap();
    let (status, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["persistence"], "native");
    let mut payload = binding(&view, false);
    payload["label"] = json!("Before redesign");
    let (status, view) = request(&app, "POST", "/api/project/history/versions", payload).await;
    assert_eq!(status, StatusCode::OK);
    let id = view["versions"][0]["id"].as_i64().unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle": "Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let modified = state.project.lock().unwrap().clone().unwrap();
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let refused = request(
        &app,
        "POST",
        &format!("/api/project/history/versions/{id}/restore"),
        binding(&view, false),
    )
    .await;
    assert_eq!(refused.0, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&modified));
    let restored = request(
        &app,
        "POST",
        &format!("/api/project/history/versions/{id}/restore"),
        binding(&view, true),
    )
    .await;
    assert_eq!(restored.0, StatusCode::OK);
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&before));
    assert!(!state.command_stack.lock().unwrap().can_undo());
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let versions = view["versions"].as_array().unwrap();
    assert_eq!(versions.len(), 2);
    let safety = versions
        .iter()
        .find(|v| v["reason"] == "pre_restore")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/project/history/versions/{safety}/restore"),
            binding(&view, true)
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&modified));
}

#[tokio::test]
async fn stale_revision_incarnation_or_generation_cannot_create_restore_or_delete_a_version() {
    let (_dir, path, state, app) = fixture().await;
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    for key in ["serverIncarnation", "snapshotRevision", "generation"] {
        let before = state.project.lock().unwrap().clone();
        let bytes = std::fs::read(&path).unwrap();
        let mut payload = binding(&view, true);
        payload["label"] = json!("Stale");
        payload[key] = if key == "serverIncarnation" {
            json!("different server")
        } else {
            json!(999_999)
        };
        let result = request(&app, "POST", "/api/project/history/versions", payload).await;
        assert_eq!(
            result.0,
            StatusCode::CONFLICT,
            "binding {key} was not enforced"
        );
        assert_eq!(*state.project.lock().unwrap(), before);
        assert_eq!(std::fs::read(&path).unwrap(), bytes);
    }
}

#[tokio::test]
async fn durable_command_failure_does_not_publish_model_stack_or_revision() {
    let (_dir, path, state, app) = fixture().await;
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle": "Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    // Refuse only the second durable undo row: admission accepts this
    // payload-free index, so the failure is still inside journal persistence.
    conn.execute_batch("CREATE UNIQUE INDEX refuse_journal ON project_history_stack(side)")
        .unwrap();
    let (_, before) = request(&app, "GET", "/api/project", Value::Null).await;
    let model = state.project.lock().unwrap().clone();
    let result = request(
        &app,
        "POST",
        "/api/project/group-address-style",
        json!({"groupAddressStyle": "TwoLevel"}),
    )
    .await;
    assert!(!result.0.is_success());
    let (_, after) = request(&app, "GET", "/api/project", Value::Null).await;
    assert_eq!(after, before);
    assert_eq!(*state.project.lock().unwrap(), model);
    assert_eq!(
        knx_store::project_history::load_editor(&conn)
            .unwrap()
            .unwrap()
            .working
            .project,
        model.unwrap()
    );
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
}

#[tokio::test]
async fn two_native_editors_cannot_overwrite_each_others_recovery_journal() {
    let (dir, path, state, first) = fixture().await;
    let other = Arc::new(knx_server::AppState::new(dir.path().join("other")));
    let second = knx_server::app(other.clone(), None);
    assert_eq!(
        request(&second, "POST", "/api/project/open", json!({"path": path}))
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &first,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle": "Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let before = other.project.lock().unwrap().clone();
    let result = request(
        &second,
        "POST",
        "/api/project/group-address-style",
        json!({"groupAddressStyle": "TwoLevel"}),
    )
    .await;
    assert!(!result.0.is_success());
    assert_eq!(*other.project.lock().unwrap(), before);
    assert_eq!(
        state
            .project
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .info
            .group_address_style,
        knx_core::GroupAddressStyle::Free
    );
}

#[tokio::test]
async fn session_only_history_is_explicit_and_version_writes_require_a_native_path() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state, None);
    assert_eq!(
        request(&app, "POST", "/api/project/new", json!({})).await.0,
        StatusCode::OK
    );
    let (status, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["persistence"], "session");
    let mut payload = binding(&view, false);
    payload["label"] = json!("Cannot persist yet");
    assert_eq!(
        request(&app, "POST", "/api/project/history/versions", payload)
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
}

#[tokio::test]
async fn opening_a_legacy_native_store_validates_history_without_rewriting_the_file() {
    let (dir, path, _state, _app) = fixture().await;
    let conn = knx_store::Connection::open(&path).unwrap();
    conn.execute_batch("DROP TABLE device_parameter_baseline_value; DROP TABLE device_parameter_baseline; DROP TABLE group_address_description; DROP TABLE com_object_priority_edit; DROP TABLE com_object_saved_priority; DROP TABLE instruction_run; DROP TABLE instruction_document; DROP TABLE manufacturer_file; DROP TABLE project_function_member; DROP TABLE project_function; DROP TABLE project_function_state; DROP TABLE project_note; DROP TABLE project_note_state; DROP TABLE project_history_stack; DROP TABLE project_history_state; DROP TABLE project_history_version; DROP TABLE project_history_context; ALTER TABLE line DROP COLUMN model_position; PRAGMA user_version = 10;").unwrap();
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        10
    );
    assert_eq!(
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_schema WHERE name = 'com_object_saved_priority'",
            [],
            |r| r.get::<_, i64>(0),
        )
        .unwrap(),
        0
    );
    drop(conn);
    let before = std::fs::read(&path).unwrap();
    let app = knx_server::app(
        Arc::new(knx_server::AppState::new(dir.path().join("legacy-reader"))),
        None,
    );
    assert_eq!(
        request(&app, "POST", "/api/project/open", json!({"path": path}))
            .await
            .0,
        StatusCode::OK
    );
    assert!(
        std::fs::read(&path).unwrap() == before,
        "opening must not migrate the user's legacy file"
    );
    let (status, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["generation"], 0);
    let mut payload = binding(&view, false);
    payload["label"] = json!("First native history version");
    assert_eq!(
        request(&app, "POST", "/api/project/history/versions", payload)
            .await
            .0,
        StatusCode::OK
    );
}

#[tokio::test]
async fn restoring_a_version_republishes_live_context_without_sending_bus_frames() {
    use knx_server::fake::{FakeConnector, FakeTunnel};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("context-history.knxdb");
    let (tunnel, handle) = FakeTunnel::new(knx_core::IndividualAddress::new(1, 1, 5).unwrap(), 64);
    let state = Arc::new(knx_server::AppState {
        connector: Box::new(FakeConnector::succeeding(tunnel)),
        ..knx_server::AppState::new(dir.path().to_path_buf())
    });
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/new",
            json!({"name": "Synthetic context history"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "POST", "/api/project/save-as", json!({"path": path}))
            .await
            .0,
        StatusCode::OK
    );
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let mut payload = binding(&view, false);
    payload["label"] = json!("Original rendering");
    let (status, view) = request(&app, "POST", "/api/project/history/versions", payload).await;
    assert_eq!(status, StatusCode::OK);
    let id = view["versions"][0]["id"].as_i64().unwrap();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/bus/monitor/start",
            json!({"gateway": "192.0.2.10:3671"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let session_id = state.bus_session.lock().await.as_ref().unwrap().id();
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle": "Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        state
            .bus_session
            .lock()
            .await
            .as_ref()
            .unwrap()
            .group_address_style(),
        Some(knx_core::GroupAddressStyle::Free)
    );
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let (status, restored) = request(
        &app,
        "POST",
        &format!("/api/project/history/versions/{id}/restore"),
        binding(&view, true),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        state
            .bus_session
            .lock()
            .await
            .as_ref()
            .unwrap()
            .group_address_style(),
        Some(knx_core::GroupAddressStyle::ThreeLevel)
    );
    assert_eq!(
        restored["project"]["group_address_context_session_id"],
        session_id
    );
    assert!(
        handle.sent_calls().is_empty(),
        "restoring native engineering data must not send bus traffic"
    );
    assert_eq!(
        request(&app, "POST", "/api/bus/monitor/stop", Value::Null)
            .await
            .0,
        StatusCode::OK
    );
}

/// Puts one device with one parameter at `raw`, every part programmed, into
/// the open project (a stand-in for a settled download).
fn put_programmed_device(state: &knx_server::AppState, raw: &str) {
    let mut guard = state.project.lock().unwrap();
    let project = guard.as_mut().unwrap();
    let device = knx_core::DeviceId(1);
    let installation = &mut project.installations[0];
    installation.parameters = vec![knx_core::ParameterInstance {
        id: knx_core::ParameterInstanceId(1),
        device,
        source: knx_core::SourceRef {
            path: "synthetic".into(),
            ets_id: "M-0001_A-0001-01-0001_P-1_R-1".into(),
        },
        raw: raw.into(),
    }];
    if !installation.topology.unassigned.contains(&device) {
        installation.topology.unassigned.push(device);
    }
    project.devices.remove(device);
    project.devices.insert(knx_core::DeviceInstance {
        id: device,
        source: knx_core::SourceRef {
            path: "synthetic".into(),
            ets_id: "synthetic-device".into(),
        },
        name: "Witness".into(),
        description: None,
        address: Some(knx_core::IndividualAddress::new(1, 1, 1).unwrap()),
        product_ref: "P".into(),
        program_ref: "M-0001_A-0001-01-0001".into(),
        commissioning: knx_core::CommissioningState {
            individual_address_loaded: true,
            application_program_loaded: true,
            parameters_loaded: true,
            communication_part_loaded: true,
            ..knx_core::CommissioningState::default()
        },
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project.ids.raise_to(&knx_core::IdAllocators::from_counts(
        1, 0, 0, 0, 0, 0, 0, 1, 0,
    ));
}

#[tokio::test]
async fn restoring_an_older_version_marks_what_the_device_no_longer_holds_and_says_so() {
    let (_dir, _path, state, app) = fixture().await;
    put_programmed_device(&state, "1");
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let mut payload = binding(&view, false);
    payload["label"] = json!("Value 1 programmed");
    let (status, view) = request(&app, "POST", "/api/project/history/versions", payload).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    let id = view["versions"][0]["id"].as_i64().unwrap();
    // The device was then programmed with value 2.
    put_programmed_device(&state, "2");

    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let (status, restored) = request(
        &app,
        "POST",
        &format!("/api/project/history/versions/{id}/restore"),
        binding(&view, true),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{restored}");

    let project = state.project.lock().unwrap().clone().unwrap();
    let pending = project
        .devices
        .get(knx_core::DeviceId(1))
        .unwrap()
        .commissioning
        .pending_parts();
    assert!(pending.parameters && pending.group_addresses, "{pending:?}");
    assert!(!pending.address && !pending.application, "{pending:?}");
    assert_eq!(project.installations[0].parameters[0].raw, "1");
    {
        let log = state.session_log.lock().unwrap();
        let entry = log
            .entries()
            .iter()
            .find(|entry| entry.source == "ProjectHistory")
            .expect("the correction is logged");
        assert!(
            entry
                .message
                .contains("“Witness” (1.1.1) needs programming for parameters and group addresses"),
            "{}",
            entry.message
        );
    }

    // Going back to the preserved state restores its value 2, but not its
    // "programmed" flags: the conservative direction (ADR-0173 §6).
    let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
    let safety = view["versions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["reason"] == "pre_restore")
        .unwrap()["id"]
        .as_i64()
        .unwrap();
    let (status, back) = request(
        &app,
        "POST",
        &format!("/api/project/history/versions/{safety}/restore"),
        binding(&view, true),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{back}");
    let project = state.project.lock().unwrap().clone().unwrap();
    assert_eq!(project.installations[0].parameters[0].raw, "2");
    let pending = project
        .devices
        .get(knx_core::DeviceId(1))
        .unwrap()
        .commissioning
        .pending_parts();
    assert!(pending.parameters && pending.group_addresses, "{pending:?}");
}

/// What the native file holds: the reference the held set must equal.
fn file_baselines(
    path: &std::path::Path,
) -> knx_core::commissioning::parameter_baseline::ParameterBaselines {
    let store = knx_store::open_existing_read_only(path).unwrap();
    knx_store::project_history::load_parameter_baselines(&store.conn).unwrap()
}

fn seed_baselines(state: &knx_server::AppState, path: &std::path::Path) {
    use knx_core::commissioning::parameter_baseline::{BaselineEdit, ParameterBaseline};
    let edit = BaselineEdit {
        device: knx_core::DeviceId(1),
        baseline: Some(ParameterBaseline {
            program_id: "M-0001_A-0001-01-0001".into(),
            values: [("M-0001_A-0001-01-0001_P-1_R-1".to_string(), "1".to_string())].into(),
            at: chrono::DateTime::UNIX_EPOCH,
        }),
    };
    let conn = knx_store::project_history::open_editor_store(path, false).unwrap();
    knx_store::project_history::write_parameter_baselines(&conn, std::slice::from_ref(&edit))
        .unwrap();
    state.parameter_baselines.lock().unwrap().apply(&edit);
}

#[tokio::test]
async fn a_restore_leaves_the_held_baselines_equal_to_what_the_file_kept() {
    // Another project id is covered where it can be built, in the store test;
    // the server takes the store's surviving set for every case alike.
    for (label, reimported) in [
        ("same device", false),
        ("another device under the id", true),
    ] {
        let (_dir, path, state, app) = fixture().await;
        put_programmed_device(&state, "1");
        let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
        let mut payload = binding(&view, false);
        payload["label"] = json!("Version");
        let (status, view) = request(&app, "POST", "/api/project/history/versions", payload).await;
        assert_eq!(status, StatusCode::OK, "{label}: {view}");
        let id = view["versions"][0]["id"].as_i64().unwrap();
        if reimported {
            let mut guard = state.project.lock().unwrap();
            let device = guard
                .as_mut()
                .unwrap()
                .devices
                .get_mut(knx_core::DeviceId(1))
                .unwrap();
            device.source.ets_id = "re-imported".into();
        }
        seed_baselines(&state, &path);

        let (_, view) = request(&app, "GET", "/api/project/history", Value::Null).await;
        let (status, restored) = request(
            &app,
            "POST",
            &format!("/api/project/history/versions/{id}/restore"),
            binding(&view, true),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{label}: {restored}");
        let held = state.parameter_baselines.lock().unwrap().clone();
        assert_eq!(held, file_baselines(&path), "{label}");
        assert_eq!(held.is_empty(), reimported, "{label}");
    }
}
