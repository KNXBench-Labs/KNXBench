//! HTTP availability guards distinguish missing, expired, unfinished and non-exportable jobs.

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
async fn exact_diagnostic_result_reference_reports_each_unavailable_state_without_fallback() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(dir.path(), true)),
        ..crate::AppState::new(dir.path().into())
    });
    let mut project = knx_core::Project::new(knx_core::Language("en".into()));
    project.devices.insert(knx_core::DeviceInstance {
        id: DeviceId(7),
        source: knx_core::SourceRef {
            path: "synthetic.xml".into(),
            ets_id: "D-7".into(),
        },
        name: "synthetic".into(),
        description: None,
        address: Some("1.1.25".parse().unwrap()),
        product_ref: "unknown-product".into(),
        program_ref: "unknown-program".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    *state.project.lock().unwrap() = Some(project);
    let preview=post(&state,"/api/device-diagnostics/preview",json!({"deviceId":7,"gateway":"203.0.113.250:3671","profile":service::PROFILE,"context":crate::diagnostic_bundle_context::ProjectContext::capture(&state)})).await;
    let token = preview["token"].as_str().unwrap();
    for (phase, expired, exportable, requested, expected) in [
        (
            "preview",
            false,
            false,
            token,
            "the diagnostic result is not complete",
        ),
        (
            "running",
            false,
            false,
            token,
            "the diagnostic result is not complete",
        ),
        (
            "complete",
            false,
            false,
            token,
            "the diagnostic result is not exportable",
        ),
        (
            "complete",
            true,
            false,
            token,
            "the retained diagnostic result has expired",
        ),
        (
            "complete",
            false,
            false,
            "missing-token",
            "the retained diagnostic result is unavailable",
        ),
    ] {
        {
            let mut held = state.diagnostic_results.0.lock().unwrap();
            let job = held.as_mut().unwrap();
            job.phase = phase;
            job.exportable = exportable;
            job.created = Instant::now() - Duration::from_secs(if expired { 901 } else { 0 });
        }
        let preview=post(&state,"/api/diagnostics/bundle/preview",json!({"problemType":"deviceReading","operation":{"relatedKey":"diagnosticsToken","relatedValue":requested}})).await;
        assert_eq!(preview["operationRecordingId"], Value::Null);
        let reports = preview["offers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["kind"] == "legacyReports")
            .unwrap();
        assert_eq!(
            reports["unavailable"], expected,
            "{phase}, expired={expired}"
        );
        assert_eq!(reports["selected"], false);
    }
}
