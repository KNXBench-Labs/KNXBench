//! `/api/settings` over HTTP: reading, patching, refusing and adopting a settings file.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

fn state() -> (Arc<knx_server::AppState>, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (
        Arc::new(knx_server::AppState::new(dir.path().to_path_buf())),
        dir,
    )
}

fn settings_file(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join(knx_server::SETTINGS_FILE_NAME)
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

async fn get(state: &Arc<knx_server::AppState>) -> (StatusCode, Value) {
    let app = knx_server::app(state.clone(), None);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/settings")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    (response.status(), body_json(response).await)
}

async fn send(
    state: &Arc<knx_server::AppState>,
    method: &str,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let app = knx_server::app(state.clone(), None);
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    (response.status(), body_json(response).await)
}

#[tokio::test]
async fn a_server_with_no_settings_file_answers_with_defaults_and_writes_nothing() {
    let (state, dir) = state();

    let (status, body) = get(&state).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "absent");
    assert_eq!(body["settings"], json!({}));
    assert_eq!(body["schemaVersion"], knx_server::CURRENT_SCHEMA_VERSION);
    assert!(
        !settings_file(&dir).exists(),
        "reading settings must not create a file"
    );
}

#[tokio::test]
async fn a_patch_is_written_and_read_back() {
    let (state, _dir) = state();

    let (status, body) = send(
        &state,
        "PUT",
        "/api/settings",
        json!({ "settings": { "theme": "graphite", "uiLanguage": "de" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["settings"]["theme"], "graphite");

    let (_, reread) = get(&state).await;
    assert_eq!(reread["status"], "ok");
    assert_eq!(reread["settings"]["theme"], "graphite");
    assert_eq!(reread["settings"]["uiLanguage"], "de");
}

#[tokio::test]
async fn a_patch_leaves_every_key_it_does_not_name_alone() {
    let (state, _dir) = state();
    send(
        &state,
        "PUT",
        "/api/settings",
        json!({ "settings": { "theme": "graphite", "somethingFromTheFuture": [1, 2, 3] } }),
    )
    .await;

    let (_, body) = send(
        &state,
        "PUT",
        "/api/settings",
        json!({ "settings": { "accent": "mint" } }),
    )
    .await;

    assert_eq!(body["settings"]["theme"], "graphite");
    assert_eq!(body["settings"]["accent"], "mint");
    assert_eq!(body["settings"]["somethingFromTheFuture"], json!([1, 2, 3]));
}

#[tokio::test]
async fn a_null_in_a_patch_clears_the_preference() {
    let (state, _dir) = state();
    send(
        &state,
        "PUT",
        "/api/settings",
        json!({ "settings": { "productLanguage": "de-DE" } }),
    )
    .await;

    let (_, body) = send(
        &state,
        "PUT",
        "/api/settings",
        json!({ "settings": { "productLanguage": null } }),
    )
    .await;

    assert_eq!(body["settings"].get("productLanguage"), None);
}

#[tokio::test]
async fn a_file_from_a_newer_build_is_refused_without_being_rewritten() {
    let (state, dir) = state();
    let from_the_future = json!({
        "schemaVersion": knx_server::CURRENT_SCHEMA_VERSION + 4,
        "settings": { "theme": "chartreuse", "somethingNobodyHereUnderstands": true }
    })
    .to_string();
    std::fs::write(settings_file(&dir), &from_the_future).unwrap();

    let (status, body) = get(&state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "refusedNewer");
    assert_eq!(body["settings"], json!({}));
    assert_eq!(
        body["fileSchemaVersion"],
        knx_server::CURRENT_SCHEMA_VERSION + 4
    );

    // And a write attempt in that session is refused rather than
    // flattening the file into something this build can read.
    let (status, _) = send(
        &state,
        "PUT",
        "/api/settings",
        json!({ "settings": { "theme": "graphite" } }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(
        std::fs::read_to_string(settings_file(&dir)).unwrap(),
        from_the_future
    );
}

#[tokio::test]
async fn a_damaged_file_is_moved_aside_and_the_session_starts_from_defaults() {
    let (state, dir) = state();
    std::fs::write(settings_file(&dir), "{ not json, not even close").unwrap();

    let (status, body) = get(&state).await;

    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "quarantined");
    assert_eq!(body["settings"], json!({}));
    let moved_to = body["movedTo"].as_str().unwrap();
    assert!(moved_to.starts_with("settings.damaged-"), "{moved_to}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join(moved_to)).unwrap(),
        "{ not json, not even close"
    );
    assert!(!settings_file(&dir).exists());
}

#[tokio::test]
async fn browser_preferences_are_adopted_once_and_migrated_on_the_way_in() {
    let (state, _dir) = state();

    let (status, body) = send(
        &state,
        "POST",
        "/api/settings/adopt",
        json!({
            "schemaVersion": 0,
            "settings": { "theme": "dark", "accent": "mint", "uiLanguage": "de" }
        }),
    )
    .await;

    assert_eq!(status, StatusCode::OK);
    // The browser era's "dark" is this era's "graphite".
    assert_eq!(body["settings"]["theme"], "graphite");
    assert_eq!(body["settings"]["accent"], "mint");
    assert_eq!(body["schemaVersion"], knx_server::CURRENT_SCHEMA_VERSION);

    // A second browser cannot stamp its own defaults over the record.
    let (status, _) = send(
        &state,
        "POST",
        "/api/settings/adopt",
        json!({ "schemaVersion": 0, "settings": { "theme": "porcelain" } }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    let (_, reread) = get(&state).await;
    assert_eq!(reread["settings"]["theme"], "graphite");
}

#[tokio::test]
async fn adoption_at_a_version_this_build_cannot_write_is_rejected() {
    let (state, dir) = state();

    let (status, _) = send(
        &state,
        "POST",
        "/api/settings/adopt",
        json!({
            "schemaVersion": knx_server::CURRENT_SCHEMA_VERSION + 1,
            "settings": { "theme": "graphite" }
        }),
    )
    .await;

    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(!settings_file(&dir).exists());
}

#[tokio::test]
async fn a_migration_is_reported_in_the_session_log() {
    let (state, dir) = state();
    std::fs::write(
        settings_file(&dir),
        json!({ "schemaVersion": 0, "settings": { "theme": "light" } }).to_string(),
    )
    .unwrap();

    let (_, body) = get(&state).await;
    assert_eq!(body["status"], "migrated");
    assert_eq!(body["settings"]["theme"], "porcelain");
    assert_eq!(body["fileSchemaVersion"], 0);

    let app = knx_server::app(state.clone(), None);
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/log")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let log = body_json(response).await;
    let entries = log.as_array().unwrap();
    assert!(
        entries
            .iter()
            .any(|e| e["source"] == "settings"
                && e["message"].as_str().unwrap().contains("migrated")),
        "{log}"
    );
}
