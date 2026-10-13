//! HTTP tests for the naming recipe: preview, confirmation, undo, reopen and conflicts.
//!
//! Synthetic project → preview/confirm/apply → one undo step, native working
//! history, conflicts that need a new preview (Goal B7).
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
        serde_json::from_slice(&bytes).unwrap_or_else(|_| json!({"nonJsonError": true})),
    )
}

/// Three fictional addresses: "Kitchen light", "Kitchen dimmer", "Hall".
async fn fixture() -> (
    tempfile::TempDir,
    Arc<knx_server::AppState>,
    axum::Router,
    Vec<u32>,
) {
    let xml = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><Installations><Installation InstallationId="0"><GroupAddresses><GroupRanges><GroupRange Id="P-0001-0_GR-1" Name="Light" RangeStart="1" RangeEnd="2047"><GroupAddress Id="P-0001-0_GA-1" Address="1" Name="Kitchen light"/><GroupAddress Id="P-0001-0_GA-2" Address="2" Name="Kitchen dimmer"/><GroupAddress Id="P-0001-0_GA-3" Address="3" Name="Hall"/></GroupRange></GroupRanges></GroupAddresses></Installation></Installations></Project></KNX>"#;
    let info = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-0001"><ProjectInformation Name="Synthetic recipe"/></Project></KNX>"#;
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
    let (status, body) = request(
        &app,
        "POST",
        "/api/project/import",
        json!({"path": path, "clientToken": "synthetic-recipe-import"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "fixture setup: {body}");
    let mut ids: Vec<(String, u32)> = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations
        .iter()
        .flat_map(|i| &i.group_addresses)
        .map(|ga| (ga.name.clone(), ga.id.0))
        .collect();
    ids.sort();
    // Hall, Kitchen dimmer, Kitchen light.
    assert_eq!(ids.len(), 3);
    (dir, state, app, ids.into_iter().map(|(_, id)| id).collect())
}

async fn binding(app: &axum::Router) -> Value {
    let (status, tree) = request(app, "GET", "/api/project", Value::Null).await;
    assert_eq!(status, StatusCode::OK);
    json!({
        "serverIncarnation": tree["server_incarnation"],
        "projectIncarnation": tree["project_incarnation"],
        "snapshotRevision": tree["snapshot_revision"],
    })
}

async fn preview(
    app: &axum::Router,
    ids: &[u32],
    prefix: &str,
    suffix: &str,
) -> (StatusCode, Value) {
    let mut body = binding(app).await;
    body["groupAddresses"] = json!(ids);
    body["prefix"] = json!(prefix);
    body["suffix"] = json!(suffix);
    request(app, "POST", "/api/name-recipe/preview", body).await
}

async fn apply(app: &axum::Router, plan: &Value, confirmed: bool) -> (StatusCode, Value) {
    request(
        app,
        "POST",
        "/api/name-recipe/apply",
        json!({"token": plan["token"], "confirmed": confirmed}),
    )
    .await
}

fn names(state: &knx_server::AppState) -> Vec<String> {
    let mut names: Vec<String> = state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .installations
        .iter()
        .flat_map(|i| &i.group_addresses)
        .map(|ga| ga.name.clone())
        .collect();
    names.sort();
    names
}

#[tokio::test]
async fn preview_changes_nothing_and_a_confirmed_recipe_is_one_undo_step() {
    let (_dir, state, app, ids) = fixture().await;
    let before = state.project.lock().unwrap().clone();
    // Selection order is the user's: Kitchen light first, then Hall.
    let (status, plan) = preview(&app, &[ids[2], ids[0]], "EG ", " (alt)").await;
    assert_eq!(status, StatusCode::OK, "{plan}");
    assert_eq!(
        plan["rows"],
        json!([
            {"id": ids[2], "before": "Kitchen light", "after": "EG Kitchen light (alt)", "changed": true},
            {"id": ids[0], "before": "Hall", "after": "EG Hall (alt)", "changed": true},
        ])
    );
    assert_eq!(plan["changed"], 2);
    assert_eq!(plan["noOp"], false);
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );

    let (status, refused) = apply(&app, &plan, false).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(refused["kind"], "nameRecipeConfirmationRequired");
    assert_eq!(state.project.lock().unwrap().clone(), before);

    let (status, applied) = apply(&app, &plan, true).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["changed"], true);
    assert_eq!(applied["persistence"], "session-only");
    assert_eq!(
        names(&state),
        ["EG Hall (alt)", "EG Kitchen light (alt)", "Kitchen dimmer"]
    );
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (1, 0)
    );
    // The tree advertises the new revision, so other bound writes see it.
    let (_, tree) = request(&app, "GET", "/api/project", Value::Null).await;
    assert_eq!(tree["snapshot_revision"], applied["snapshotRevision"]);
    assert_eq!(tree["can_undo"], true);

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
        names(&state),
        ["EG Hall (alt)", "EG Kitchen light (alt)", "Kitchen dimmer"]
    );
    // A confirmed preview is spent: a replay asks for a new one.
    let (status, replay) = apply(&app, &plan, true).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(replay["kind"], "nameRecipeStale");
}

#[tokio::test]
async fn an_edit_after_the_preview_or_a_newer_preview_needs_a_new_preview() {
    let (_dir, state, app, ids) = fixture().await;
    let (_, plan) = preview(&app, &[ids[0]], "A ", "").await;
    // Another window renames something unrelated: the shown preview is stale.
    let mut rename = binding(&app).await;
    rename["name"] = json!("Dimmer");
    rename["expectedName"] = json!("Kitchen dimmer");
    let (status, body) = request(
        &app,
        "PATCH",
        &format!("/api/group-addresses/{}/name", ids[1]),
        rename,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let concurrent = state.project.lock().unwrap().clone();
    let lengths = state.command_stack.lock().unwrap().history_lengths();
    let (status, stale) = apply(&app, &plan, true).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(stale["kind"], "nameRecipeStale");
    assert_eq!(state.project.lock().unwrap().clone(), concurrent);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        lengths
    );

    // A preview bound to an old revision is refused before it is computed.
    let mut old = binding(&app).await;
    old["snapshotRevision"] = json!(old["snapshotRevision"].as_u64().unwrap() - 1);
    old["groupAddresses"] = json!([ids[0]]);
    old["prefix"] = json!("A ");
    old["suffix"] = json!("");
    let (status, _) = request(&app, "POST", "/api/name-recipe/preview", old).await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Two previews: the newer one wins, the older dialog must preview again.
    let (_, first) = preview(&app, &[ids[0]], "First ", "").await;
    let (_, second) = preview(&app, &[ids[0]], "Second ", "").await;
    assert_eq!(apply(&app, &first, true).await.0, StatusCode::CONFLICT);
    assert_eq!(apply(&app, &second, true).await.0, StatusCode::OK);
    assert!(names(&state).contains(&"Second Hall".to_string()));
}

#[tokio::test]
async fn selections_are_explicit_bounded_and_refused_whole() {
    let (_dir, state, app, ids) = fixture().await;
    let before = state.project.lock().unwrap().clone();
    let cases: Vec<(Vec<u32>, String, &str)> = vec![
        (vec![], "X".into(), "nameRecipeSelection"),
        (vec![ids[0], ids[0]], "X".into(), "nameRecipeDuplicate"),
        (vec![ids[0], 999_999], "X".into(), "nameRecipeTarget"),
        ((0..257).collect(), "X".into(), "nameRecipeSelection"),
        (vec![ids[0]], "x".repeat(4097), "nameRecipeBudget"),
        (vec![ids[0]], "\u{0}".into(), "nameRecipeInvalidName"),
    ];
    for (selection, prefix, kind) in cases {
        let (status, body) = preview(&app, &selection, &prefix, "").await;
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY, "{kind}: {body}");
        assert_eq!(body["kind"], kind);
    }
    // Unknown members (a pattern, a query) are not part of the contract.
    let mut generated = binding(&app).await;
    generated["groupAddresses"] = json!([ids[0]]);
    generated["prefix"] = json!("");
    generated["suffix"] = json!("");
    generated["match"] = json!("Kitchen*");
    let (status, _) = request(&app, "POST", "/api/name-recipe/preview", generated).await;
    assert!(status.is_client_error());
    assert_eq!(state.project.lock().unwrap().clone(), before);
}

#[tokio::test]
async fn a_no_op_recipe_keeps_revision_history_and_redo() {
    let (_dir, state, app, ids) = fixture().await;
    let (_, plan) = preview(&app, &[ids[0]], "Z ", "").await;
    apply(&app, &plan, true).await;
    assert_eq!(
        request(&app, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    let before = state.project.lock().unwrap().clone();
    let lengths = state.command_stack.lock().unwrap().history_lengths();
    let revision = binding(&app).await["snapshotRevision"].clone();
    let (_, plan) = preview(&app, &[ids[0], ids[1]], "", "").await;
    assert_eq!(plan["noOp"], true);
    assert_eq!(plan["changed"], 0);
    let (status, outcome) = apply(&app, &plan, true).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(outcome["changed"], false);
    assert_eq!(outcome["snapshotRevision"], revision);
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        lengths
    );
    assert!(state.command_stack.lock().unwrap().can_redo());
    // Even a no-op preview is confirmed once.
    let (status, replay) = apply(&app, &plan, true).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(replay["kind"], "nameRecipeStale");
}

#[tokio::test]
async fn a_failed_native_commit_publishes_nothing() {
    let (dir, state, app, ids) = fixture().await;
    let before = state.project.lock().unwrap().clone();
    let missing = dir.path().join("removed-native.knxdb");
    *state.store_path.lock().unwrap() = Some(missing.clone());
    *state.history_generation.lock().unwrap() = 1;
    let (_, plan) = preview(&app, &[ids[0]], "New ", "").await;
    let (status, response) = apply(&app, &plan, true).await;
    assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(response["kind"], "nameRecipePersistenceFailed");
    assert_eq!(state.project.lock().unwrap().clone(), before);
    assert_eq!(
        state.command_stack.lock().unwrap().history_lengths(),
        (0, 0)
    );
    assert_eq!(*state.history_generation.lock().unwrap(), 1);
    assert!(!missing.exists());
}

#[tokio::test]
async fn a_recipe_survives_save_reopen_and_its_undo_and_redo_too() {
    let (dir, state, app, ids) = fixture().await;
    let path = dir.path().join("native.knxdb");
    assert_eq!(
        request(&app, "POST", "/api/project/save-as", json!({"path": path}))
            .await
            .0,
        StatusCode::OK
    );
    let before = state.project.lock().unwrap().clone();
    let (_, plan) = preview(&app, &[ids[1], ids[2]], "", " · EG").await;
    let (status, applied) = apply(&app, &plan, true).await;
    assert_eq!(status, StatusCode::OK, "{applied}");
    assert_eq!(applied["persistence"], "native-working-history");
    assert!(applied["nativeGeneration"].is_i64());
    let after = state.project.lock().unwrap().clone();

    // A fresh server opens the file and finds the recipe and its undo step.
    let reopened = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let app2 = knx_server::app(reopened.clone(), None);
    let (status, body) = request(&app2, "POST", "/api/project/open", json!({"path": path})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(reopened.project.lock().unwrap().clone(), after);
    assert_eq!(
        names(&reopened),
        ["Hall", "Kitchen dimmer · EG", "Kitchen light · EG"]
    );
    assert_eq!(
        request(&app2, "POST", "/api/undo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(reopened.project.lock().unwrap().clone(), before);
    assert_eq!(
        request(&app2, "POST", "/api/redo", Value::Null).await.0,
        StatusCode::OK
    );
    assert_eq!(reopened.project.lock().unwrap().clone(), after);
}

#[tokio::test]
async fn cancel_forgets_only_its_own_preview() {
    let (_dir, _state, app, ids) = fixture().await;
    let (_, plan) = preview(&app, &[ids[0]], "C ", "").await;
    let (_, other) = request(
        &app,
        "POST",
        "/api/name-recipe/cancel",
        json!({"token": "someone-else"}),
    )
    .await;
    assert_eq!(other["cancelled"], false);
    let (_, mine) = request(
        &app,
        "POST",
        "/api/name-recipe/cancel",
        json!({"token": plan["token"]}),
    )
    .await;
    assert_eq!(mine["cancelled"], true);
    assert_eq!(apply(&app, &plan, true).await.0, StatusCode::CONFLICT);
}
