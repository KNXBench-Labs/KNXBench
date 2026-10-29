//! Retained support references disclose missing, expired and uncompleted acquisition states.

use super::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn post(state: &SharedState, uri: &str, body: Value) -> Value {
    let response = crate::app(state.clone(), None)
        .oneshot(
            Request::post(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

#[tokio::test]
async fn exact_support_result_reference_reports_unavailable_states_without_a_live_read() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(dir.path(), true)),
        ..crate::AppState::new(dir.path().into())
    });
    let preview = post(&state, "/api/support/preview", json!({"address":"1.1.25"})).await;
    let token = preview["token"].as_str().unwrap();
    for (expired, requested, expected) in [
        (false, token, "the support result is not complete"),
        (true, token, "the retained support result has expired"),
        (
            false,
            "missing-token",
            "the retained support result is unavailable",
        ),
    ] {
        {
            let mut held = state.support_results.0.lock().unwrap();
            held.as_mut().unwrap().created =
                Instant::now() - Duration::from_secs(if expired { 901 } else { 0 });
        }
        let preview=post(&state,"/api/diagnostics/bundle/preview",json!({"problemType":"deviceReading","operation":{"relatedKey":"supportToken","relatedValue":requested}})).await;
        assert_eq!(preview["operationRecordingId"], Value::Null);
        let reports = preview["offers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["kind"] == "legacyReports")
            .unwrap();
        assert_eq!(reports["unavailable"], expected);
        assert_eq!(reports["selected"], false);
    }
}
