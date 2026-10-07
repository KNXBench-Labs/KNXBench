//! `/api/achievements` over HTTP: reading, merging, refusing and resetting the record.

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

fn record_file(dir: &tempfile::TempDir) -> std::path::PathBuf {
    dir.path().join(knx_server::ACHIEVEMENTS_FILE_NAME)
}

async fn send(
    state: &Arc<knx_server::AppState>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let app = knx_server::app(state.clone(), None);
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(value) => {
            builder = builder.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = app.oneshot(builder.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

async fn get(state: &Arc<knx_server::AppState>) -> (StatusCode, Value) {
    send(state, "GET", "/api/achievements", None).await
}

async fn record(state: &Arc<knx_server::AppState>, delta: Value) -> (StatusCode, Value) {
    send(state, "POST", "/api/achievements/record", Some(delta)).await
}

#[tokio::test]
async fn with_no_file_nothing_is_unlocked_and_nothing_is_written() {
    let (state, dir) = state();
    let (status, body) = get(&state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "absent");
    assert_eq!(body["unlocked"], json!({}));
    assert_eq!(body["progress"], json!({}));
    assert_eq!(
        body["schemaVersion"],
        knx_server::ACHIEVEMENTS_SCHEMA_VERSION
    );
    assert!(!record_file(&dir).exists());
}

#[tokio::test]
async fn a_report_is_merged_kept_on_disk_and_read_back() {
    let (state, dir) = state();
    let (status, body) = record(
        &state,
        json!({ "unlocked": { "foundation": "2026-10-07T21:30:00Z" },
                "progress": { "time-traveller": 7 } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "ok");
    assert_eq!(body["unlocked"]["foundation"], "2026-10-07T21:30:00Z");
    assert!(record_file(&dir).exists());

    // A later report with a later unlock and a lower counter changes nothing:
    // the record only grows, and an unlock keeps its first moment.
    let (_, body) = record(
        &state,
        json!({ "unlocked": { "foundation": "2026-10-09T00:00:00Z" },
                "progress": { "time-traveller": 3, "palette-pro": 2 } }),
    )
    .await;
    assert_eq!(body["unlocked"]["foundation"], "2026-10-07T21:30:00Z");
    assert_eq!(
        body["progress"],
        json!({ "time-traveller": 7, "palette-pro": 2 })
    );

    let (_, fresh) = get(&state).await;
    assert_eq!(fresh["unlocked"], body["unlocked"]);
    assert_eq!(fresh["progress"], body["progress"]);
}

#[tokio::test]
async fn a_malformed_report_is_refused_and_the_file_is_untouched() {
    let (state, dir) = state();
    record(&state, json!({ "progress": { "time-traveller": 1 } })).await;
    let before = std::fs::read(record_file(&dir)).unwrap();

    for (delta, expected) in [
        (
            json!({ "unlocked": { "../etc": "2026-10-07T21:30:00Z" } }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "unlocked": { "foundation": "last tuesday" } }),
            StatusCode::BAD_REQUEST,
        ),
        (
            json!({ "progress": { "x": 9_007_199_254_740_992_u64 } }),
            StatusCode::BAD_REQUEST,
        ),
    ] {
        let (status, body) = record(&state, delta.clone()).await;
        assert_eq!(status, expected, "{delta}");
        assert!(body["error"].is_string(), "{delta}: {body}");
    }
    // Shapes the server does not know are refused by the extractor.
    let (status, _) = record(&state, json!({ "unlocked": {}, "rarity": 3 })).await;
    assert!(status.is_client_error(), "{status}");
    let (status, _) = record(&state, json!({ "progress": { "x": -1 } })).await;
    assert!(status.is_client_error(), "{status}");

    assert_eq!(std::fs::read(record_file(&dir)).unwrap(), before);
}

#[tokio::test]
async fn a_file_from_a_newer_build_is_shown_as_empty_refuses_writes_and_is_left_alone() {
    let (state, dir) = state();
    let body = r#"{"schemaVersion": 99, "unlocked": {"from-the-future": "2030-01-01T00:00:00Z"}}"#;
    std::fs::write(record_file(&dir), body).unwrap();

    let (status, read) = get(&state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read["status"], "refusedNewer");
    assert_eq!(read["fileSchemaVersion"], 99);
    assert_eq!(read["unlocked"], json!({}));
    assert!(read["message"].is_string());

    let (status, _) = record(&state, json!({ "progress": { "time-traveller": 1 } })).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(std::fs::read_to_string(record_file(&dir)).unwrap(), body);
}

#[tokio::test]
async fn a_damaged_file_is_moved_aside_and_named_without_a_host_path() {
    let (state, dir) = state();
    std::fs::write(record_file(&dir), "this is not a record").unwrap();

    let (status, read) = get(&state).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(read["status"], "quarantined");
    let moved_to = read["movedTo"].as_str().unwrap();
    assert!(moved_to.starts_with("achievements.damaged-"), "{moved_to}");
    assert!(!moved_to.contains('/'), "{moved_to}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join(moved_to)).unwrap(),
        "this is not a record"
    );
    // Tracking starts over on a fresh record.
    let (status, _) = record(&state, json!({ "progress": { "time-traveller": 1 } })).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn reset_starts_over_and_keeps_the_old_record_beside_it() {
    let (state, dir) = state();
    record(
        &state,
        json!({ "unlocked": { "foundation": "2026-10-07T21:30:00Z" } }),
    )
    .await;
    let before = std::fs::read_to_string(record_file(&dir)).unwrap();

    let (status, body) = send(&state, "POST", "/api/achievements/reset", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["status"], "absent");
    assert_eq!(body["unlocked"], json!({}));
    let moved_to = body["movedTo"].as_str().unwrap();
    assert!(moved_to.starts_with("achievements.reset-"), "{moved_to}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join(moved_to)).unwrap(),
        before
    );
    assert!(!record_file(&dir).exists());

    // A second reset with nothing left to move is a quiet success.
    let (status, body) = send(&state, "POST", "/api/achievements/reset", None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.get("movedTo").is_none());
}

#[tokio::test]
async fn a_reset_also_clears_a_file_from_a_newer_build_without_destroying_it() {
    let (state, dir) = state();
    let body = r#"{"schemaVersion": 99}"#;
    std::fs::write(record_file(&dir), body).unwrap();

    let (status, reply) = send(&state, "POST", "/api/achievements/reset", None).await;
    assert_eq!(status, StatusCode::OK);
    let moved_to = reply["movedTo"].as_str().unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.path().join(moved_to)).unwrap(),
        body
    );
}
