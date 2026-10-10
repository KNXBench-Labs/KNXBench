//! Verifies selective import previews without replacing the current project.

use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use serde_json::{json, Value};
use std::sync::Arc;
use tower::ServiceExt;

async fn post(app: &axum::Router, path: &str, payload: Value) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
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

#[tokio::test]
async fn source_inventory_is_read_only_and_exposes_selectable_device_ids() {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        post(&app, "/api/project/new", json!({"name":"Keep me"}))
            .await
            .0,
        StatusCode::OK
    );
    let before = state.project.lock().unwrap().clone().unwrap();
    let lengths = state.command_stack.lock().unwrap().history_lengths();
    let (status, inventory) = post(
        &app,
        "/api/project/import-selection/source",
        json!({"path":path}),
    )
    .await;
    assert_eq!(
        status,
        StatusCode::OK,
        "selective source inspection must exist"
    );
    assert_eq!(inventory["installations"].as_array().unwrap().len(), 1);
    assert_eq!(
        inventory["installations"][0]["devices"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(inventory["installations"][0]["devices"][0]["id"].is_u64());
    assert!(inventory["sourceHash"].is_string());
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&before));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        lengths
    );
    assert!(state.opaque.lock().unwrap().is_empty());
}

async fn merge_fixture() -> (
    tempfile::TempDir,
    std::path::PathBuf,
    std::sync::Arc<knx_server::AppState>,
    axum::Router,
    Value,
) {
    let dir = tempfile::tempdir().unwrap();
    let path = knx_testsupport::write_minimal_knxproj(dir.path());
    let source = knx_app::selective_import::read_source(&path, None).unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    assert_eq!(
        post(&app, "/api/project/new", json!({"name":"Existing target"}))
            .await
            .0,
        StatusCode::OK
    );
    {
        let mut guard = state.project.lock().unwrap();
        let project = guard.as_mut().unwrap();
        let mut seed = source.project.devices.iter().next().unwrap().clone();
        seed.id = project.ids.next_device_id().unwrap();
        seed.name = "Existing seed".into();
        seed.address = Some(knx_core::IndividualAddress::new(1, 1, 220).unwrap());
        seed.com_objects.clear();
        project.installations[0].topology.unassigned.push(seed.id);
        project.devices.insert(seed);
    }
    let store = dir.path().join("target.knxdb");
    assert_eq!(
        post(&app, "/api/project/save-as", json!({"path":store}))
            .await
            .0,
        StatusCode::OK
    );
    let persisted = knx_store::open_and_migrate(&store).unwrap();
    assert_eq!(
        knx_store::load_project(&persisted).unwrap(),
        state.project.lock().unwrap().clone().unwrap()
    );
    let source_installation = source.project.installations[0].id.0;
    let target_installation = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations[0]
        .id
        .0;
    let payload = json!({"path":path,"sourceHash":source.source_hash,"selection":{
        "sourceInstallation":source_installation,"targetInstallation":target_installation,
        "devices":[source.project.devices.iter().next().unwrap().id.0],"lines":[]}});
    (dir, store, state, app, payload)
}

#[tokio::test]
async fn preview_confirm_undo_redo_and_native_reopen_preserve_one_atomic_import() {
    let (_dir, store, state, app, mut payload) = merge_fixture().await;
    let before = state.project.lock().unwrap().clone().unwrap();
    let (status, preview) = post(
        &app,
        "/api/project/import-selection/preview",
        payload.clone(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{preview}");
    assert_eq!(preview["preview"]["counts"]["devices"], 1);
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&before));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    payload["confirmationToken"] = preview["confirmationToken"].clone();
    let (status, applied) =
        post(&app, "/api/project/import-selection/apply", payload.clone()).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    let merged = state.project.lock().unwrap().clone().unwrap();
    assert_eq!(merged.devices.iter().count(), 2);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
    let context = state.opaque.lock().unwrap().clone();
    assert_eq!(
        context
            .iter()
            .find(|e| e.kind == "SelectiveImportArchive")
            .unwrap()
            .bytes,
        knx_testsupport::minimal_knxproj_bytes()
    );
    assert_eq!(post(&app, "/api/undo", json!({})).await.0, StatusCode::OK);
    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .same_user_content_as(&before));
    assert_eq!(*state.opaque.lock().unwrap(), context);
    assert_eq!(post(&app, "/api/redo", json!({})).await.0, StatusCode::OK);
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&merged));
    let conn = knx_store::project_history::open_editor_store(&store, false).unwrap();
    let editor = knx_store::project_history::load_editor(&conn)
        .unwrap()
        .unwrap();
    assert_eq!(editor.working.project, merged);
    assert_eq!(editor.working.opaque, context);
    let duplicate = post(&app, "/api/project/import-selection/apply", payload).await;
    assert!(duplicate.0 == StatusCode::CONFLICT || duplicate.0 == StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&merged));
}

#[tokio::test]
async fn missing_confirmation_changed_selection_and_changed_target_refuse_without_side_effects() {
    let (_dir, _store, state, app, mut payload) = merge_fixture().await;
    let before = state.project.lock().unwrap().clone().unwrap();
    assert_eq!(
        post(&app, "/api/project/import-selection/apply", payload.clone())
            .await
            .0,
        StatusCode::CONFLICT
    );
    let (_, preview) = post(
        &app,
        "/api/project/import-selection/preview",
        payload.clone(),
    )
    .await;
    payload["confirmationToken"] = preview["confirmationToken"].clone();
    payload["selection"]["devices"] = json!([]);
    assert_eq!(
        post(&app, "/api/project/import-selection/apply", payload.clone())
            .await
            .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&before));
    payload["selection"]["devices"] = json!([1]);
    state.project.lock().unwrap().as_mut().unwrap().info.name = "Unreviewed edit".into();
    let edited = state.project.lock().unwrap().clone().unwrap();
    assert_eq!(
        post(&app, "/api/project/import-selection/apply", payload)
            .await
            .0,
        StatusCode::CONFLICT
    );
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&edited));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert!(state.opaque.lock().unwrap().is_empty());
}

#[tokio::test]
async fn durable_generation_refusal_does_not_publish_model_context_or_history() {
    let (_dir, store, state, app, mut payload) = merge_fixture().await;
    let before = state.project.lock().unwrap().clone().unwrap();
    let (_, preview) = post(
        &app,
        "/api/project/import-selection/preview",
        payload.clone(),
    )
    .await;
    payload["confirmationToken"] = preview["confirmationToken"].clone();
    *state.history_generation.lock().unwrap() += 1;
    let generation = *state.history_generation.lock().unwrap();
    let (status, _) = post(&app, "/api/project/import-selection/apply", payload).await;
    assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(state.project.lock().unwrap().as_ref(), Some(&before));
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_eq!(*state.history_generation.lock().unwrap(), generation);
    assert!(state.opaque.lock().unwrap().is_empty());
    assert!(state.manufacturer_refs.lock().unwrap().is_empty());
    let conn = knx_store::project_history::open_editor_store(&store, false).unwrap();
    assert_eq!(knx_store::load_project(&conn).unwrap(), before);
    assert!(knx_store::load_opaque(&conn).unwrap().is_empty());
}
