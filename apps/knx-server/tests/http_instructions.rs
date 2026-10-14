//! Synthetic source → authenticated HTTP preview/apply → native recovery.
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use knx_testsupport::zip_with_entries;
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
    let bytes = axum::body::to_bytes(response.into_body(), 4 * 1024 * 1024)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"nonJsonError":true})),
    )
}
async fn fixture(
    value: &str,
) -> (
    tempfile::TempDir,
    Arc<knx_server::AppState>,
    axum::Router,
    u32,
) {
    let value = value
        .replace('&', "&amp;")
        .replace('"', "&quot;")
        .replace('<', "&lt;");
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><UnassignedDevices><DeviceInstance Id="P-0001-0_DI-1" Name="Synthetic" InstallationHints="{value}"/></UnassignedDevices></Topology></Installation></Installations></Project></KNX>"#
    );
    let info = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name="Synthetic instructions"/></Project></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("synthetic.knxproj");
    std::fs::write(
        &path,
        zip_with_entries(&[
            ("P-0001.signature", b""),
            ("P-0001/0.xml", xml.as_bytes()),
            ("P-0001/Project.xml", info.as_bytes()),
        ]),
    )
    .unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    let opened = request(
        &app,
        "POST",
        "/api/project/import",
        json!({"path":path,"clientToken":"synthetic-instruction-import"}),
    )
    .await;
    assert_eq!(
        opened.0,
        StatusCode::OK,
        "synthetic fixture setup: {}",
        opened.1
    );
    let id = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .iter()
        .next()
        .unwrap()
        .id
        .0;
    (dir, state, app, id)
}
fn document(text: &str) -> Value {
    json!({"runs":[{"text":text,"style":{"bold":false,"italic":false,"underline":false}}]})
}
fn preview_request(view: &Value, doc: Value) -> Value {
    json!({"serverIncarnation":view["serverIncarnation"],"projectIncarnation":view["projectIncarnation"],"snapshotRevision":view["snapshotRevision"],"document":doc})
}
fn apply_request(plan: &Value, confirmed: bool, conversion: bool) -> Value {
    json!({"token":plan["token"],"confirmed":confirmed,"confirmConversion":conversion})
}
#[tokio::test]
async fn read_and_preview_are_nonmutating_then_explicit_apply_is_one_undo_step() {
    let (_dir, state, app, id) = fixture(r"{\rtf1 original}").await;
    let before = state.project.lock().unwrap().clone();
    let opaque = state.opaque.lock().unwrap().clone();
    let (status, view) = request(
        &app,
        "GET",
        &format!("/api/devices/{id}/instructions"),
        Value::Null,
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["effectiveDocument"]["runs"][0]["text"], "original");
    assert_eq!(view["overrideDocument"], Value::Null);
    let (status, plan) = request(
        &app,
        "POST",
        &format!("/api/devices/{id}/instructions/preview"),
        preview_request(&view, document("edited Grüße")),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/apply"),
            apply_request(&plan, false, false)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
    let (status, applied) = request(
        &app,
        "POST",
        &format!("/api/devices/{id}/instructions/apply"),
        apply_request(&plan, true, false),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(applied["changed"], true);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
    assert_eq!(*state.opaque.lock().unwrap(), opaque);
    assert_eq!(
        request(&app, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        request(&app, "POST", "/api/redo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/apply"),
            apply_request(&plan, true, false)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
}
#[tokio::test]
async fn unsupported_conversion_and_cancellation_are_explicit_and_do_not_discard_source() {
    let (_dir, state, app, id) = fixture(r"{\rtf1 a{\*\unknown retained}b}").await;
    let before = state.project.lock().unwrap().clone();
    let (_, view) = request(
        &app,
        "GET",
        &format!("/api/devices/{id}/instructions"),
        Value::Null,
    )
    .await;
    assert!(!view["unsupported"].as_array().unwrap().is_empty());
    let (_, plan) = request(
        &app,
        "POST",
        &format!("/api/devices/{id}/instructions/preview"),
        preview_request(&view, document("subset")),
    )
    .await;
    assert_eq!(plan["conversionRequired"], true);
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/apply"),
            apply_request(&plan, true, false)
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/cancel"),
            json!({"token":plan["token"]})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/apply"),
            apply_request(&plan, true, true)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
}
#[tokio::test]
async fn stale_revision_and_invalid_documents_never_change_the_project() {
    let (_dir, state, app, id) = fixture("original").await;
    let before = state.project.lock().unwrap().clone();
    let (_, view) = request(
        &app,
        "GET",
        &format!("/api/devices/{id}/instructions"),
        Value::Null,
    )
    .await;
    let mut invalid = preview_request(&view, document(""));
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/preview"),
            invalid.clone()
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    invalid = preview_request(&view, document("edited"));
    invalid["snapshotRevision"] = json!(0);
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/preview"),
            invalid
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
}
#[tokio::test]
async fn confirmed_edit_is_durable_and_recovers_original_and_undo_after_native_reopen() {
    let (dir, state, app, id) = fixture("original").await;
    let path = dir.path().join("native.knxdb");
    assert_eq!(
        request(&app, "POST", "/api/project/save-as", json!({"path":path}))
            .await
            .0,
        StatusCode::OK
    );
    let before = state.project.lock().unwrap().clone();
    let opaque = state.opaque.lock().unwrap().clone();
    let (_, view) = request(
        &app,
        "GET",
        &format!("/api/devices/{id}/instructions"),
        Value::Null,
    )
    .await;
    let (_, plan) = request(
        &app,
        "POST",
        &format!("/api/devices/{id}/instructions/preview"),
        preview_request(&view, document("edited")),
    )
    .await;
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/apply"),
            apply_request(&plan, true, false)
        )
        .await
        .0,
        StatusCode::OK
    );
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let recovered = knx_store::project_history::load_editor(&conn)
        .unwrap()
        .unwrap();
    assert_eq!(
        recovered.working.project,
        state.project.lock().unwrap().clone().unwrap()
    );
    assert_eq!(recovered.working.opaque, opaque);
    let mut restored = recovered.working.project;
    let mut stack = knx_core::CommandStack::from_snapshot_states(recovered.undo, recovered.redo);
    stack.undo(&mut restored).unwrap();
    assert_eq!(Some(restored), before);
}

#[tokio::test]
async fn a_preview_cannot_be_substituted_replayed_or_applied_after_an_unrelated_edit() {
    let (_dir, state, app, id) = fixture("original").await;
    let (_, view) = request(
        &app,
        "GET",
        &format!("/api/devices/{id}/instructions"),
        Value::Null,
    )
    .await;
    let (_, plan) = request(
        &app,
        "POST",
        &format!("/api/devices/{id}/instructions/preview"),
        preview_request(&view, document("approved draft")),
    )
    .await;
    let before = state.project.lock().unwrap().clone();
    let mut substituted = apply_request(&plan, true, false);
    substituted["document"] = document("not previewed");
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/apply"),
            substituted
        )
        .await
        .0,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle":"Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    let concurrent = state.project.lock().unwrap().clone();
    let lengths = state.command_stack.lock().unwrap().history_lengths();
    assert_eq!(
        request(
            &app,
            "POST",
            &format!("/api/devices/{id}/instructions/apply"),
            apply_request(&plan, true, false)
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    assert_eq!(state.project.lock().unwrap().clone(), concurrent);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        lengths
    );
}
#[tokio::test]
async fn failed_native_commit_does_not_publish_memory_or_history_changes() {
    let (dir, state, app, id) = fixture("original").await;
    let before = state.project.lock().unwrap().clone();
    let opaque = state.opaque.lock().unwrap().clone();
    let missing = dir.path().join("removed-native.knxdb");
    *state.store_path.lock().unwrap() = Some(missing.clone());
    *state.history_generation.lock().unwrap() = 1;
    let (_, view) = request(
        &app,
        "GET",
        &format!("/api/devices/{id}/instructions"),
        Value::Null,
    )
    .await;
    let (_, plan) = request(
        &app,
        "POST",
        &format!("/api/devices/{id}/instructions/preview"),
        preview_request(&view, document("candidate")),
    )
    .await;
    let (status, response) = request(
        &app,
        "POST",
        &format!("/api/devices/{id}/instructions/apply"),
        apply_request(&plan, true, false),
    )
    .await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response["kind"], "instructionPersistenceFailed");
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(*state.opaque.lock().unwrap(), opaque);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_eq!(*state.history_generation.lock().unwrap(), 1);
    assert!(!missing.exists());
}
#[tokio::test]
async fn supported_noop_preserves_absence_revision_and_redo_stack() {
    let (_dir, state, app, id) = fixture("original").await;
    assert_eq!(
        request(
            &app,
            "POST",
            "/api/project/group-address-style",
            json!({"groupAddressStyle":"Free"})
        )
        .await
        .0,
        StatusCode::OK
    );
    assert_eq!(
        request(&app, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    let before = state.project.lock().unwrap().clone();
    let lengths = state.command_stack.lock().unwrap().history_lengths();
    let (_, view) = request(
        &app,
        "GET",
        &format!("/api/devices/{id}/instructions"),
        Value::Null,
    )
    .await;
    let (_, plan) = request(
        &app,
        "POST",
        &format!("/api/devices/{id}/instructions/preview"),
        preview_request(&view, document("original")),
    )
    .await;
    assert_eq!(plan["noOp"], true);
    let (status, outcome) = request(
        &app,
        "POST",
        &format!("/api/devices/{id}/instructions/apply"),
        apply_request(&plan, true, false),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(outcome["changed"], false);
    assert_eq!(outcome["snapshotRevision"], view["snapshotRevision"]);
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        lengths
    );
    assert!(state.command_stack.lock().unwrap().can_redo());
}

/// A new project with one device as `POST /api/devices` names it (`KB-DEV-<id>`);
/// placing from the catalogue itself is covered in `http_catalog_to_device`.
async fn native_fixture() -> (
    tempfile::TempDir,
    Arc<knx_server::AppState>,
    axum::Router,
    u32,
) {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app = knx_server::app(state.clone(), None);
    let (status, _) = request(&app, "POST", "/api/project/new", json!({})).await;
    assert_eq!(status, StatusCode::OK);
    let mut lock = state.project.lock().unwrap();
    let project = lock.as_mut().unwrap();
    let id = project.ids.next_device_id().unwrap();
    project.devices.insert(knx_core::DeviceInstance {
        id,
        source: knx_core::SourceRef {
            path: format!("KB-DEV-{}", id.0),
            ets_id: format!("KB-DEV-{}", id.0),
        },
        name: "Made here".into(),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    drop(lock);
    (dir, state, app, id.0)
}

#[tokio::test]
async fn a_device_made_here_gets_instructions_without_an_invented_original() {
    let (_dir, state, app, id) = native_fixture().await;
    let opaque = state.opaque.lock().unwrap().clone();
    let url = format!("/api/devices/{id}/instructions");
    let (status, view) = request(&app, "GET", &url, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{view}");
    assert_eq!(view["origin"], "native");
    assert_eq!(view["originalXmlSha256"], Value::Null);
    assert_eq!(view["originalPresent"], false);
    assert_eq!(view["effectiveDocument"], Value::Null);
    assert_eq!(view["overrideDocument"], Value::Null);
    let (status, plan) = request(
        &app,
        "POST",
        &format!("{url}/preview"),
        preview_request(&view, document("Above the door")),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(plan["conversionRequired"], false);
    let (status, applied) = request(
        &app,
        "POST",
        &format!("{url}/apply"),
        apply_request(&plan, true, false),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    let (_, view) = request(&app, "GET", &url, Value::Null).await;
    assert_eq!(
        view["overrideDocument"]["runs"][0]["text"],
        "Above the door"
    );
    assert_eq!(view["origin"], "native");
    assert_eq!(
        *state.opaque.lock().unwrap(),
        opaque,
        "no original is stored for a native device"
    );
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
    // Removing returns to no instructions at all.
    let (_, plan) = request(
        &app,
        "POST",
        &format!("{url}/preview"),
        preview_request(&view, Value::Null),
    )
    .await;
    assert_eq!(plan["noOp"], false);
    let (status, _) = request(
        &app,
        "POST",
        &format!("{url}/apply"),
        apply_request(&plan, true, false),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let (_, view) = request(&app, "GET", &url, Value::Null).await;
    assert_eq!(view["overrideDocument"], Value::Null);
    assert_eq!(view["effectiveDocument"], Value::Null);
}

#[tokio::test]
async fn an_imported_device_still_says_imported_with_its_hash() {
    let (_dir, _state, app, id) = fixture(r"{\rtf1 original}").await;
    let (_, view) = request(
        &app,
        "GET",
        &format!("/api/devices/{id}/instructions"),
        Value::Null,
    )
    .await;
    assert_eq!(view["origin"], "imported");
    assert_eq!(view["originalXmlSha256"].as_str().map(str::len), Some(64));
}
