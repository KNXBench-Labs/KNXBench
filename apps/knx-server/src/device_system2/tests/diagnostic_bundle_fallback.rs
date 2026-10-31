//! Retained System-2 operation projections remain exportable without an operation recording.

use axum::{
    body::{to_bytes, Body},
    http::Request,
};
use knx_app::operation_recording::{OperationKind, OperationStart, RecordedOutcome};
use serde_json::{json, Value};
use std::{io::Read, sync::Arc, time::Duration};
use tower::ServiceExt;

#[path = "../../../../../crates/knx-app/tests/it/system2_validation_preparation/fixture.rs"]
pub(crate) mod fixture;

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
    let expected_private = root
        .bundle_admission(&metadata.operation_id)
        .unwrap()
        .unwrap()
        .projection
        .bytes()
        .to_vec();
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
    zip.by_name("operations/system2-private.json")
        .unwrap()
        .read_to_end(&mut bytes)
        .unwrap();
    assert_eq!(bytes, expected_private);
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

struct AdmittedFixture {
    _dir: tempfile::TempDir,
    state: Arc<crate::AppState>,
    metadata: super::Metadata,
    private_bytes: Vec<u8>,
}

fn persisted_cancelled_fixture(recordings_enabled: bool) -> AdmittedFixture {
    use knx_app::property_validation::private_projection::{
        admit_snapshot, read_snapshot, PrivateSnapshot,
    };
    use knx_app::property_validation::Checkpoint;
    use knx_app::system2_validation_preparation::{
        prepare_retained_project_hardware_source, HardwareObservation, ProjectHardwareRecord,
        RecoveryTrack,
    };

    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(
            dir.path(),
            !recordings_enabled,
        )),
        ..crate::AppState::new(dir.path().into())
    });
    let (_source, products, project, _profile) = fixture::configured_inputs();
    let mut metadata = super::metadata();
    let context = crate::diagnostic_bundle_context::ProjectContext::capture(&state);
    metadata.binding.server_incarnation = context.server_incarnation;
    metadata.binding.project_incarnation = context.project_incarnation.to_string();
    metadata.binding.project_revision = context.revision.to_string();
    let source = prepare_retained_project_hardware_source(
        &products,
        &project,
        knx_core::DeviceId(1),
        &metadata.operation_id,
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
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
    let binding = bundle.binding.clone();
    let root = state.device_system2.root(&state).unwrap();
    root.create(&metadata).unwrap();
    root.record_execution_build(&metadata.operation_id, "9.9.9+gabc1234")
        .unwrap();
    let mut journal = root.observed(&metadata.operation_id, &bundle).unwrap();
    journal.checkpoint(Checkpoint::Cancelled).unwrap();
    drop(journal);
    let journal = dir
        .path()
        .join(super::super::ROOT_NAME)
        .join(&metadata.operation_id)
        .join("journal");
    let snapshot = match read_snapshot(&journal, &binding) {
        PrivateSnapshot::Available(snapshot) => *snapshot,
        PrivateSnapshot::Unavailable(_) => panic!("producer-valid snapshot unavailable"),
    };
    let (_progress, admitted) = admit_snapshot(snapshot).expect("closed App admission");
    AdmittedFixture {
        _dir: dir,
        state,
        metadata,
        private_bytes: admitted.bytes().to_vec(),
    }
}

fn section<'a>(preview: &'a Value, kind: &str, label: Option<&str>) -> &'a Value {
    preview["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| {
            section["kind"] == kind
                && label.map_or(section["label"].is_null(), |value| {
                    section["label"] == value
                })
        })
        .unwrap_or_else(|| panic!("missing section {kind} {label:?}"))
}

async fn preview(state: &crate::SharedState, query: &Value) -> Value {
    let response = post(state, "/api/diagnostics/bundle/preview", query.clone()).await;
    assert_eq!(response.status(), 200);
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

async fn section_text(state: &crate::SharedState, query: &Value, key: &str) -> Value {
    let mut query = query.clone();
    query["key"] = json!(key);
    let response = post(state, "/api/diagnostics/bundle/section", query).await;
    assert_eq!(response.status(), 200);
    serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap()).unwrap()
}

fn recorded_system2(fixture: &AdmittedFixture, edge: &str) -> String {
    let recorder = crate::operation_recordings::start(
        &fixture.state,
        OperationStart::new(OperationKind::System2Download)
            .target(json!({
                "deviceId": fixture.metadata.binding.device_id,
                "individualAddress": fixture.metadata.address,
            }))
            .gateway("203.0.113.250:3671")
            .related("operationId", &fixture.metadata.operation_id)
            .related("consumerEdge", edge),
    );
    recorder.finish(RecordedOutcome::failed(
        vec!["fixed recorded failure".into()],
        Value::Null,
    ));
    assert!(fixture
        .state
        .recordings
        .wait_until_idle(Duration::from_secs(10)));
    std::thread::sleep(Duration::from_millis(3));
    recorder.id().unwrap().to_string()
}

fn replace_recorded_device(fixture: &AdmittedFixture, id: &str, device_id: u32) -> Vec<u8> {
    let path = fixture.state.recordings.root().join(format!("{id}.jsonl"));
    let original = std::fs::read(&path).unwrap();
    let newline = original.iter().position(|byte| *byte == b'\n').unwrap();
    let mut header: Value = serde_json::from_slice(&original[..newline]).unwrap();
    header["target"]["deviceId"] = json!(device_id);
    let mut changed = serde_json::to_vec(&header).unwrap();
    changed.push(b'\n');
    changed.extend_from_slice(&original[newline + 1..]);
    std::fs::write(path, changed).unwrap();
    original
}

#[tokio::test]
async fn direct_system2_hook_uses_the_one_admitted_snapshot_for_text_review_and_zip() {
    let fixture = persisted_cancelled_fixture(false);
    let mut query = json!({
        "problemType": "deviceProgramming",
        "operation": {
            "relatedKey": "operationId",
            "relatedValue": fixture.metadata.operation_id,
        },
        "sections": ["operation", "legacyReports"],
    });
    let preview = preview(&fixture.state, &query).await;
    let operation = section(&preview, "operation", None);
    assert_eq!(operation["files"], json!(["system2-private.json"]));
    assert_eq!(operation["bytes"], fixture.private_bytes.len() as u64);
    query["review"] = preview["review"].clone();
    let text = section_text(&fixture.state, &query, operation["key"].as_str().unwrap()).await;
    assert_eq!(
        text["text"].as_str().unwrap().strip_suffix('\n').unwrap(),
        format!(
            "===== operations/system2-private.json =====\n{}",
            String::from_utf8(fixture.private_bytes.clone()).unwrap()
        )
    );

    query["consent"] = json!(true);
    let response = post(&fixture.state, "/api/diagnostics/bundle", query).await;
    assert_eq!(response.status(), 200);
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut private = Vec::new();
    zip.by_name("operations/system2-private.json")
        .unwrap()
        .read_to_end(&mut private)
        .unwrap();
    assert_eq!(private, fixture.private_bytes);
    let mut report = Vec::new();
    zip.by_name("reports/system2-report.json")
        .unwrap()
        .read_to_end(&mut report)
        .unwrap();
    let report: Value = serde_json::from_slice(&report).unwrap();
    assert_eq!(report["executionBuild"], json!({"state":"unknown"}));
    assert_eq!(report["exporterBuild"], crate::version_string());
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("9.9.9+gabc1234"));
}

#[tokio::test]
async fn recording_and_history_hooks_share_admitted_bytes_without_raw_recording_files() {
    let fixture = persisted_cancelled_fixture(true);
    let admission_reads = Arc::new(super::super::storage::AdmissionReadCounts::default());
    fixture
        .state
        .device_system2
        .observe_bundle_admission(admission_reads.clone());
    let earlier = recorded_system2(&fixture, "history");
    let current = recorded_system2(&fixture, "current");
    let mut query = json!({
        "problemType": "deviceProgramming",
        "operation": {"recordingId": current},
        "earlierRecordingIds": [earlier],
        "sections": ["operation", "earlierOperation", "legacyReports"],
    });
    let preview = preview(&fixture.state, &query).await;
    assert_eq!(admission_reads.values(), (1, 1, 1));
    let current_section = section(&preview, "operation", Some(&current));
    let history_section = section(&preview, "earlierOperation", Some(&earlier));
    for selected in [current_section, history_section] {
        assert_eq!(selected["files"], json!(["system2-private.json"]));
        assert_eq!(selected["bytes"], fixture.private_bytes.len() as u64);
    }
    query["review"] = preview["review"].clone();
    for selected in [current_section, history_section] {
        let text = section_text(&fixture.state, &query, selected["key"].as_str().unwrap()).await;
        assert!(text["text"]
            .as_str()
            .unwrap()
            .strip_suffix('\n')
            .unwrap()
            .ends_with(&String::from_utf8(fixture.private_bytes.clone()).unwrap()));
        assert!(!text["text"].as_str().unwrap().contains("recording.jsonl"));
    }
    assert_eq!(admission_reads.values(), (3, 3, 3));

    query["consent"] = json!(true);
    let response = post(&fixture.state, "/api/diagnostics/bundle", query).await;
    assert_eq!(response.status(), 200);
    assert_eq!(admission_reads.values(), (4, 4, 4));
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    for id in [&current, &earlier] {
        let mut actual = Vec::new();
        zip.by_name(&format!("operations/{id}/system2-private.json"))
            .unwrap()
            .read_to_end(&mut actual)
            .unwrap();
        assert_eq!(actual, fixture.private_bytes);
        assert!(zip
            .by_name(&format!("operations/{id}/recording.jsonl"))
            .is_err());
    }
    let mut report = Vec::new();
    zip.by_name("reports/system2-report.json")
        .unwrap()
        .read_to_end(&mut report)
        .unwrap();
    let report: Value = serde_json::from_slice(&report).unwrap();
    assert_eq!(report["executionBuild"], json!({"state":"unknown"}));
    assert_eq!(report["exporterBuild"], crate::version_string());
    assert!(!serde_json::to_string(&report)
        .unwrap()
        .contains("9.9.9+gabc1234"));
}

#[tokio::test]
async fn direct_current_history_edges_validate_before_cache_reuse() {
    let fixture = persisted_cancelled_fixture(true);
    let history = recorded_system2(&fixture, "history-conflict");
    let current = recorded_system2(&fixture, "current-conflict");
    let query = json!({
        "problemType": "deviceProgramming",
        "operation": {"recordingId": current},
        "earlierRecordingIds": [history],
        "sections": ["operation", "earlierOperation", "legacyReports"],
    });

    let history_original =
        replace_recorded_device(&fixture, &history, fixture.metadata.binding.device_id + 1);
    let response = post(
        &fixture.state,
        "/api/diagnostics/bundle/preview",
        query.clone(),
    )
    .await;
    assert_eq!(response.status(), 409);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(body["error"], "bundleSourceConflict");

    std::fs::write(
        fixture
            .state
            .recordings
            .root()
            .join(format!("{history}.jsonl")),
        history_original,
    )
    .unwrap();
    replace_recorded_device(&fixture, &current, fixture.metadata.binding.device_id + 1);
    let response = post(&fixture.state, "/api/diagnostics/bundle/preview", query).await;
    assert_eq!(response.status(), 409);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(body["error"], "bundleSourceConflict");
}

#[tokio::test]
async fn proven_source_disagreement_returns_bundle_source_conflict() {
    let fixture = persisted_cancelled_fixture(true);
    let current = recorded_system2(&fixture, "proven-conflict");
    replace_recorded_device(&fixture, &current, fixture.metadata.binding.device_id + 1);
    let response = post(
        &fixture.state,
        "/api/diagnostics/bundle/preview",
        json!({
            "problemType": "deviceProgramming",
            "operation": {"recordingId": current},
            "sections": ["operation", "legacyReports"],
        }),
    )
    .await;
    assert_eq!(response.status(), 409);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 1024 * 1024).await.unwrap())
            .unwrap();
    assert_eq!(body["error"], "bundleSourceConflict");
}

#[tokio::test]
async fn bundle_admission_owns_metadata_and_snapshot_after_return() {
    let fixture = persisted_cancelled_fixture(false);
    let root = fixture.state.device_system2.root(&fixture.state).unwrap();
    let admitted = root
        .bundle_admission(&fixture.metadata.operation_id)
        .unwrap()
        .unwrap();
    let expected_progress = serde_json::to_value(&admitted.progress).unwrap();
    let expected_projection = admitted.projection.bytes().to_vec();
    let operation = fixture
        ._dir
        .path()
        .join(super::super::ROOT_NAME)
        .join(&fixture.metadata.operation_id);
    std::fs::write(operation.join("metadata.json"), b"{}").unwrap();
    std::fs::remove_dir_all(operation.join("journal")).unwrap();

    assert_eq!(
        admitted.metadata.operation_id,
        fixture.metadata.operation_id
    );
    assert_eq!(
        serde_json::to_value(&admitted.progress).unwrap(),
        expected_progress
    );
    assert_eq!(admitted.projection.bytes(), expected_projection);
}

#[tokio::test]
async fn retained_operation_without_a_journal_or_recording_is_fixed_unavailable() {
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
    assert_eq!(preview["deviceId"], Value::Null);
    for kind in ["operation", "legacyReports"] {
        let offer = preview["offers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|offer| offer["kind"] == kind)
            .unwrap();
        assert_eq!(
            offer["unavailable"],
            "the selected diagnostic source is unavailable"
        );
    }
    query["review"] = preview["review"].clone();
    query["consent"] = json!(true);
    let response = post(&state, "/api/diagnostics/bundle", query).await;
    assert_eq!(response.status(), 200);
    let bytes = to_bytes(response.into_body(), 8 * 1024 * 1024)
        .await
        .unwrap();
    let zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert!(!zip.file_names().any(|name| {
        name == "reports/system2-report.json" || name == "operations/system2-private.json"
    }));
}
