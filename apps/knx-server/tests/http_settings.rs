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
async fn geometry_preferences_round_trip_from_a_v1_document_without_the_new_keys() {
    let (state, dir) = state();
    std::fs::write(
        settings_file(&dir),
        json!({ "schemaVersion": 1, "settings": { "theme": "graphite" } }).to_string(),
    )
    .unwrap();
    let (status, body) = send(
        &state,
        "PUT",
        "/api/settings",
        json!({ "settings": {
            "uiScale": 1.2,
            "navigationPaneWidth": 320,
            "inspectorPaneWidth": 410
        } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["settings"]["theme"], "graphite");
    let restarted = Arc::new(knx_server::AppState::new(dir.path().to_path_buf()));
    let (_, reread) = get(&restarted).await;
    assert_eq!(
        reread["settings"],
        json!({
            "theme": "graphite", "uiScale": 1.2,
            "navigationPaneWidth": 320, "inspectorPaneWidth": 410
        })
    );
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

/// ADR-0040: the web UI's "don't ask again" for programming is an object
/// under `programmingConsent`. It must survive the real wire path and a
/// reread from disk, and "Ask again" (a `null` patch) must remove it.
#[tokio::test]
async fn a_programming_consent_round_trips_and_is_forgotten_by_null() {
    let (state, _dir) = state();
    let consent = json!({ "stage": "alpha", "version": "0.1.0-alpha.1+gabc1234" });
    let (status, _) = send(
        &state,
        "PUT",
        "/api/settings",
        json!({ "settings": { "programmingConsent": consent } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);

    let (_, reread) = get(&state).await;
    assert_eq!(reread["settings"]["programmingConsent"], consent);

    send(
        &state,
        "PUT",
        "/api/settings",
        json!({ "settings": { "programmingConsent": null } }),
    )
    .await;
    let (_, reread) = get(&state).await;
    assert_eq!(reread["settings"].get("programmingConsent"), None);
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
    assert_eq!(body["diagnostic"]["kind"], "refusedNewer");
    assert_eq!(
        body["diagnostic"]["fileVersion"],
        knx_server::CURRENT_SCHEMA_VERSION + 4
    );
    assert_eq!(
        body["diagnostic"]["currentVersion"],
        knx_server::CURRENT_SCHEMA_VERSION
    );
    assert!(body["message"].as_str().is_some());
    assert!(body.get("notice").is_none());
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
    assert_eq!(body["diagnostic"]["kind"], "quarantined");
    assert_eq!(body["diagnostic"]["reason"], "invalidJson");
    assert!(body["message"].as_str().is_some());
    assert!(body.get("notice").is_none());
    assert_eq!(body["settings"], json!({}));
    let moved_to = body["movedTo"].as_str().unwrap();
    assert!(moved_to.starts_with("settings.damaged-"), "{moved_to}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join(moved_to)).unwrap(),
        "{ not json, not even close"
    );
    assert!(!settings_file(&dir).exists());
}

async fn assert_quarantine_reason(raw: &str, expected: &str) {
    let (state, dir) = state();
    std::fs::write(settings_file(&dir), raw).unwrap();
    let (status, body) = get(&state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["diagnostic"]["kind"], "quarantined");
    assert_eq!(body["diagnostic"]["reason"], expected);
    assert!(body["diagnostic"]["movedTo"].as_str().is_some());
    assert!(body["message"].as_str().is_some());
    assert!(body.get("notice").is_none());
}

#[tokio::test]
async fn quarantine_reasons_distinguish_wrong_root_missing_version_and_wrong_settings() {
    assert_quarantine_reason("[]", "notObject").await;
    assert_quarantine_reason(r#"{"settings": {}}"#, "missingSchemaVersion").await;
    assert_quarantine_reason(
        &format!(
            r#"{{"schemaVersion": {}, "settings": []}}"#,
            knx_server::CURRENT_SCHEMA_VERSION
        ),
        "settingsNotObject",
    )
    .await;
}

#[cfg(unix)]
#[tokio::test]
async fn quarantine_reason_distinguishes_an_unreadable_settings_path() {
    let (state, dir) = state();
    std::fs::create_dir(settings_file(&dir)).unwrap();
    let (status, body) = get(&state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["diagnostic"]["kind"], "quarantined");
    assert_eq!(body["diagnostic"]["reason"], "unreadable");
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("could not be read"));
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
    assert_eq!(body["diagnostic"]["kind"], "adopted");
    assert_eq!(body["diagnostic"]["fromVersion"], 0);
    assert_eq!(
        body["diagnostic"]["toVersion"],
        knx_server::CURRENT_SCHEMA_VERSION
    );
    assert!(body["message"].as_str().is_some());
    assert!(body.get("notice").is_none());
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
    assert_eq!(body["diagnostic"]["kind"], "migrated");
    assert_eq!(body["diagnostic"]["fromVersion"], 0);
    assert_eq!(
        body["diagnostic"]["toVersion"],
        knx_server::CURRENT_SCHEMA_VERSION
    );
    assert!(body["message"].as_str().is_some());
    assert!(body.get("notice").is_none());

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
    let entry = entries
        .iter()
        .find(|e| e["source"] == "settings" && e["message"].as_str().unwrap().contains("migrated"))
        .unwrap_or_else(|| panic!("missing settings migration entry: {log}"));
    assert_eq!(entry["diagnostic"]["kind"], "migrated");
    assert_eq!(entry["diagnostic"]["fromVersion"], 0);
    assert_eq!(
        entry["diagnostic"]["toVersion"],
        knx_server::CURRENT_SCHEMA_VERSION
    );
}

#[tokio::test]
async fn adopting_over_a_damaged_file_says_it_was_moved_aside_not_that_it_exists() {
    let (state, dir) = state();
    std::fs::write(settings_file(&dir), "{ half a settings file").unwrap();

    // The read inside adoption quarantines the wreck, so "a file already
    // exists" would be a lie by the time it reached the browser.
    let (status, body) = send(
        &state,
        "POST",
        "/api/settings/adopt",
        json!({ "schemaVersion": 0, "settings": { "theme": "dark" } }),
    )
    .await;

    assert_eq!(status, StatusCode::CONFLICT);
    let error = body["error"].as_str().unwrap();
    assert!(error.contains("moved to"), "{error}");
    assert!(!error.contains("nothing to adopt into"), "{error}");

    // And the retry the message implies actually works.
    let (status, body) = send(
        &state,
        "POST",
        "/api/settings/adopt",
        json!({ "schemaVersion": 0, "settings": { "theme": "dark" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["settings"]["theme"], "graphite");
}
