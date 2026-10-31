//! HTTP/ZIP fallback readback for the exact retained download session without a recording.

use super::*;
use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use serde_json::{json, Value};
use tower::ServiceExt;

async fn post(state: &crate::SharedState, body: Value, uri: &str) -> axum::response::Response {
    crate::app(state.clone(), None)
        .oneshot(
            Request::post(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_string()))
                .unwrap(),
        )
        .await
        .unwrap()
}

#[tokio::test]
async fn exact_retained_download_result_without_recording_keeps_its_original_status_and_report() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(dir.path(), true)),
        ..crate::AppState::new(dir.path().into())
    });
    // An owned synthetic ended session tests projection, not wire/acquisition behavior.
    let mut ended = session(tokio::spawn(async {}), finished(), Some(2));
    ended.join().await;
    let expected_status = serde_json::to_value(ended.status()).unwrap();
    let expected_report = serde_json::to_value(ended.report().unwrap()).unwrap();
    *state.device_download.lock().await = Some(ended);
    let mut query = json!({"problemType":"deviceProgramming","deviceId":7,"operation":{"relatedKey":"downloadId","relatedValue":"1"}});
    let response = post(&state, query.clone(), "/api/diagnostics/bundle/preview").await;
    assert_eq!(response.status(), 200);
    let preview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(preview["operationRecordingId"], Value::Null);
    let device = preview["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "device")
        .unwrap();
    assert_eq!(
        device["selected"], false,
        "no proven project context means no borrowed numeric id"
    );
    query["review"] = preview["review"].clone();
    query["consent"] = json!(true);
    let response = post(&state, query, "/api/diagnostics/bundle").await;
    assert_eq!(response.status(), 200);
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert!(zip.by_name("reports/download-report.json").is_err());
    assert!(zip.by_name("operations/download-status.json").is_err());
    assert!(!zip
        .file_names()
        .any(|name| name.starts_with("device/") || name.ends_with("recording.jsonl")));
    {
        let retained = state.device_download.lock().await;
        let retained = retained.as_ref().unwrap();
        assert_eq!(
            serde_json::to_value(retained.status()).unwrap(),
            expected_status
        );
        assert_eq!(
            serde_json::to_value(retained.report().unwrap()).unwrap(),
            expected_report
        );
    }
    let response = post(&state,json!({"problemType":"deviceProgramming","operation":{"relatedKey":"downloadId","relatedValue":"2"}}),"/api/diagnostics/bundle/preview").await;
    let preview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    let reports = preview["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "legacyReports")
        .unwrap();
    assert_eq!(
        reports["unavailable"],
        "the selected diagnostic source is unavailable"
    );
}
