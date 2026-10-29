//! Retained System-2 operation projections remain exportable without an operation recording.

use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use serde_json::{json, Value};
use std::{io::Read, sync::Arc};
use tower::ServiceExt;

#[path = "../../../../../crates/knx-app/tests/it/system2_validation_preparation/fixture.rs"]
mod fixture;

#[tokio::test]
async fn a_terminal_retained_system2_journal_is_reopened_and_exported_without_a_recording() {
    use knx_app::property_validation::Checkpoint;
    use knx_app::system2_validation_preparation::{
        prepare_retained_project_hardware_source, HardwareObservation, ProjectHardwareRecord,
        RecoveryTrack,
    };
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(dir.path(), true)),
        ..crate::AppState::new(dir.path().into())
    });
    let (_source, products, project, _profile) = fixture::configured_inputs();
    let mut metadata = super::metadata();
    let source = prepare_retained_project_hardware_source(
        &products,
        &project,
        knx_core::DeviceId(1),
        &metadata.operation_id,
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    // Reuse App's offline synthetic observed-record fixture, never handed to a
    // transport or used as write authority. This tests durable replay/projection only.
    let mut bundle = source.preview().clone();
    bundle.format = 3;
    bundle.binding.simulated = false;
    bundle.binding.serial = [0, 1, 2, 3, 4, 5];
    let hardware = ProjectHardwareRecord {
        pointer_width_policy: None,
        revision: 1,
        operation: metadata.operation_id.clone(),
        recovery_track: RecoveryTrack::KnxBenchBestEffort,
        recovery_basis:
            "experimental-same-application-source-shape-not-observed-allocator-history-v1".into(),
        observation: HardwareObservation {
            target: bundle.binding.target.clone(),
            serial: bundle.binding.serial,
            mask: bundle.binding.mask,
            manufacturer: bundle.binding.manufacturer,
            hardware: bundle.binding.hardware,
            application: bundle.binding.application,
            observed_application: None,
        },
        source_preparation_sha256: bundle
            .project_preparation
            .as_ref()
            .unwrap()
            .sha256()
            .unwrap(),
    };
    bundle.binding.project_preparation_sha256 = Some(hardware.sha256().unwrap());
    bundle.hardware_preparation = Some(Box::new(hardware));
    bundle.plan().unwrap();
    metadata.address = bundle.binding.target.clone();
    let root = state.device_system2.root(&state).unwrap();
    root.create(&metadata).unwrap();
    let mut journal = root.observed(&metadata.operation_id, &bundle).unwrap();
    journal.checkpoint(Checkpoint::Cancelled).unwrap();
    let expected = serde_json::to_value(journal.progress()).unwrap();
    assert_eq!(expected["outcome"], "cancelled");
    drop(journal);
    assert_eq!(
        serde_json::to_value(root.progress(&metadata.operation_id).unwrap().unwrap()).unwrap(),
        expected
    );
    let mut query = json!({"problemType":"deviceProgramming","operation":{"relatedKey":"operationId","relatedValue":metadata.operation_id}});
    let response = post(&state, "/api/diagnostics/bundle/preview", query.clone()).await;
    assert_eq!(response.status(), 200);
    let preview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(preview["operationRecordingId"], Value::Null);
    query["review"] = preview["review"].clone();
    query["consent"] = json!(true);
    let response = post(&state, "/api/diagnostics/bundle", query).await;
    assert_eq!(response.status(), 200);
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut bytes = Vec::new();
    zip.by_name("operations/system2-progress.json")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&bytes).unwrap(), expected);
    let mut bytes = Vec::new();
    zip.by_name("reports/system2-report.json")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report["download"]["result"], "nothingWritten");
}

async fn post(state: &crate::SharedState, uri: &str, body: Value) -> axum::response::Response {
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
async fn retained_operation_without_a_journal_or_recording_keeps_unknown_outcome_in_zip() {
    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(dir.path(), true)),
        ..crate::AppState::new(dir.path().into())
    });
    let metadata = super::metadata();
    let root = state.device_system2.root(&state).unwrap();
    root.create(&metadata).unwrap();
    root.record_execution_build(&metadata.operation_id, "9.9.9+gabc1234")
        .unwrap();
    let mut query = json!({"problemType":"deviceProgramming","operation":{"relatedKey":"operationId","relatedValue":metadata.operation_id}});
    let response = post(&state, "/api/diagnostics/bundle/preview", query.clone()).await;
    assert_eq!(response.status(), 200);
    let preview: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(preview["operationRecordingId"], Value::Null);
    assert_eq!(
        preview["deviceId"],
        Value::Null,
        "metadata's reusable numeric id is not current-project authority"
    );
    query["review"] = preview["review"].clone();
    query["consent"] = json!(true);
    let response = post(&state, "/api/diagnostics/bundle", query).await;
    assert_eq!(response.status(), 200);
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut bytes = Vec::new();
    zip.by_name("reports/system2-report.json")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    let report: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(report["download"]["result"], "unknown");
    assert_eq!(
        report["executionBuild"],
        json!({"state":"recorded","version":"9.9.9+gabc1234"})
    );
    let mut bytes = Vec::new();
    zip.by_name("operations/system2-progress.json")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    let progress: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(progress["download"]["result"], "unknown");
    assert!(!zip
        .file_names()
        .any(|name| name.starts_with("device/") || name.ends_with("recording.jsonl")));
}
