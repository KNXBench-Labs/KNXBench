//! Durable activity history is metadata only and never contacts the bus.
use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_server::{app, AppState};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn history(state: Arc<AppState>, query: &str) -> (StatusCode, Value) {
    let response = app(state, None)
        .oneshot(
            Request::builder()
                .uri(format!("/api/bus/history{query}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), 65536)
        .await
        .unwrap();
    let value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, value)
}

#[tokio::test]
async fn empty_history_declares_its_version_and_partial_coverage() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        product_db: None,
        ..AppState::new(dir.path().to_path_buf())
    });
    let (status, body) = history(state, "").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["format"], 1);
    assert_eq!(body["coverage"], "partial");
    assert_eq!(body["entries"], json!([]));
    assert_eq!(body["hasMore"], false);
    assert_eq!(body["durability"], "persistent");
    assert!(body.get("path").is_none());
}

#[tokio::test]
async fn invalid_bounds_do_not_create_the_database() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(AppState {
        product_db: None,
        ..AppState::new(dir.path().to_path_buf())
    });
    for query in [
        "?limit=0",
        "?limit=101",
        "?after=18446744073709551615",
        "?limit=abc",
        "?unknown=1",
    ] {
        assert_eq!(
            history(Arc::clone(&state), query).await.0,
            StatusCode::BAD_REQUEST
        );
        assert!(!dir.path().join("activity-history.sqlite").exists());
    }
}

#[tokio::test]
async fn foreign_and_future_history_returns_unavailable_without_overwriting() {
    for future in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity-history.sqlite");
        if future {
            drop(knx_store::activity_history::ActivityHistory::open(&path).unwrap());
            rusqlite::Connection::open(&path)
                .unwrap()
                .pragma_update(None, "user_version", 2)
                .unwrap();
        } else {
            rusqlite::Connection::open(&path).unwrap().execute_batch("CREATE TABLE unrelated(value TEXT); INSERT INTO unrelated VALUES ('retained');").unwrap();
        }
        let before = std::fs::read(&path).unwrap();
        let state = Arc::new(AppState {
            product_db: None,
            ..AppState::new(dir.path().to_path_buf())
        });
        let (status, body) = history(state, "").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(std::fs::read(&path).unwrap(), before);
        assert!(!body.to_string().contains(&path.display().to_string()));
    }
}
