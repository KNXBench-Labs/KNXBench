//! Admission regressions for private recording lifecycle and reviewed dependencies.

use std::fs;
use std::io::Read;
use std::os::unix::fs::PermissionsExt;
use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use knx_app::diagnostic_bundle::SectionKind;
use knx_app::operation_recording::{
    OperationKind, OperationStart, OutcomeStatus, RecordedOutcome, RecordingSummary,
};
use serde_json::{json, Value};

use super::{call, json_of, record, state};

// The fixture is compiled once, in the System-2 fallback tests (clippy::duplicate_mod).
use crate::device_system2::tests::diagnostic_bundle_fallback::fixture as system2_fixture;

fn row<'a>(preview: &'a Value, id: &str) -> &'a Value {
    preview["recordings"]
        .as_array()
        .expect("recordings array")
        .iter()
        .find(|row| row["id"] == id)
        .unwrap_or_else(|| panic!("missing recording {id}"))
}

async fn preview(state: &crate::SharedState, query: Value) -> Value {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    json_of(&body)
}

fn assert_private_recording(summary: &Value, id: &str, size: u64, active: bool, end: Value) {
    assert_eq!(
        summary,
        &json!({
            "id": id,
            "sizeBytes": size,
            "formatVersion": 1,
            "active": active,
            "tornTail": false,
            "end": end,
        }),
        "bundle preview must not expose raw recording header metadata"
    );
}

fn synthetic_summary(size_bytes: u64) -> RecordingSummary {
    RecordingSummary {
        id: "op-20261029T100000000Z-0a1b2c3d".into(),
        size_bytes,
        format_version: Some(1),
        writer_version: Some("test".into()),
        operation_kind: Some("compare".into()),
        started_at: Some("2026-10-29T10:00:00Z".into()),
        target: json!({"individualAddress":"1.1.5"}),
        gateway: Some("203.0.113.250:3671".into()),
        related: json!({}),
        outcome: Some("succeeded".into()),
        active: false,
        torn_tail: false,
    }
}

struct EarlierOperationFixture {
    _dir: tempfile::TempDir,
    state: Arc<crate::AppState>,
    recording_id: String,
    physical_size: u64,
    private_bytes: Vec<u8>,
}

fn private_directory(path: &std::path::Path) {
    fs::create_dir(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

fn private_file(path: &std::path::Path, bytes: &[u8]) {
    fs::write(path, bytes).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
}

fn earlier_operation_fixture() -> EarlierOperationFixture {
    use knx_app::property_validation::private_projection::{
        admit_snapshot, read_snapshot, PrivateSnapshot,
    };
    use knx_app::property_validation::{write_prepared_bundle, Checkpoint, OperationStore};
    use knx_app::system2_validation_preparation::{
        prepare_retained_project_hardware_source, HardwareObservation, ProjectHardwareRecord,
        RecoveryTrack,
    };

    let dir = tempfile::tempdir().unwrap();
    let state = Arc::new(crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(dir.path(), false)),
        ..crate::AppState::new(dir.path().into())
    });
    let operation_id = "0123456789abcdef0123456789abcdef";
    let (_source, products, project, _profile) = system2_fixture::configured_inputs();
    let source = prepare_retained_project_hardware_source(
        &products,
        &project,
        knx_core::DeviceId(1),
        operation_id,
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
        operation: operation_id.into(),
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

    let context = crate::diagnostic_bundle_context::ProjectContext::capture(&state);
    let device_id = 7;
    let metadata = json!({
        "schemaVersion": 1,
        "operationId": operation_id,
        "binding": {
            "serverIncarnation": context.server_incarnation,
            "projectIncarnation": context.project_incarnation.to_string(),
            "projectRevision": context.revision.to_string(),
            "deviceId": device_id,
        },
        "address": bundle.binding.target,
        "deviceName": "Synthetic retained System-2 fixture",
    });
    let root = dir.path().join(crate::device_system2::ROOT_NAME);
    private_directory(&root);
    let operation = root.join(operation_id);
    private_directory(&operation);
    private_file(
        &operation.join("metadata.json"),
        &serde_json::to_vec(&metadata).unwrap(),
    );
    write_prepared_bundle(&operation.join("observed.json"), &bundle).unwrap();
    let journal_path = operation.join("journal");
    let mut journal = OperationStore::create(&journal_path, bundle.binding.clone()).unwrap();
    journal.checkpoint(Checkpoint::Cancelled).unwrap();
    drop(journal);
    let snapshot = match read_snapshot(&journal_path, &bundle.binding) {
        PrivateSnapshot::Available(snapshot) => *snapshot,
        PrivateSnapshot::Unavailable(_) => panic!("producer-valid snapshot unavailable"),
    };
    let (_progress, admitted) = admit_snapshot(snapshot).expect("closed App admission");
    let private_bytes = admitted.bytes().to_vec();

    let recorder = crate::operation_recordings::start(
        &state,
        OperationStart::new(OperationKind::System2Download)
            .target(json!({
                "deviceId": device_id,
                "individualAddress": bundle.binding.target,
            }))
            .gateway("203.0.113.250:3671")
            .related("operationId", operation_id),
    );
    recorder.finish(RecordedOutcome {
        status: OutcomeStatus::Cancelled,
        error_chain: Vec::new(),
        summary: Value::Null,
    });
    assert!(state.recordings.wait_until_idle(Duration::from_secs(10)));
    let recording_id = recorder.id().unwrap().to_string();
    let physical_size = state
        .recordings
        .list()
        .unwrap()
        .into_iter()
        .find(|summary| summary.id == recording_id)
        .expect("real System-2 recording summary")
        .size_bytes;

    EarlierOperationFixture {
        _dir: dir,
        state,
        recording_id,
        physical_size,
        private_bytes,
    }
}

fn earlier_operation_query(fixture: &EarlierOperationFixture, supplied_size: u64) -> Value {
    json!({
        "problemType": "deviceProgramming",
        "operation": {"selection": "none"},
        "earlierRecordingIds": [fixture.recording_id],
        "sections": ["earlierOperation"],
        "testRecordingSummarySupplier": {
            "recordingId": fixture.recording_id,
            "sizeBytes": supplied_size,
        },
    })
}

fn offer<'a>(preview: &'a Value, kind: &str) -> &'a Value {
    preview["offers"]
        .as_array()
        .expect("offers array")
        .iter()
        .find(|offer| offer["kind"] == kind)
        .unwrap_or_else(|| panic!("missing offer {kind}"))
}

#[tokio::test]
async fn earlier_operation_route_requires_summary_admission_before_private_zip() {
    const ABOVE_SAFE_JSON_INTEGER: u64 = 9_007_199_254_740_992;
    let fixture = earlier_operation_fixture();
    let (status, body, _) = call(&fixture.state, "GET", "/api/diagnostics/recordings", None).await;
    assert_eq!(status, StatusCode::OK);
    let listing = json_of(&body);
    let public = row(&listing, &fixture.recording_id);
    assert_eq!(public["sizeBytes"], fixture.physical_size);
    assert_eq!(public["formatVersion"], 1);
    assert_eq!(public["operationKind"], "system2-download");
    assert_eq!(
        public["related"]["operationId"],
        "0123456789abcdef0123456789abcdef"
    );

    let safe_query = earlier_operation_query(&fixture, fixture.physical_size);
    let safe_preview = preview(&fixture.state, safe_query.clone()).await;
    assert_private_recording(
        row(&safe_preview, &fixture.recording_id),
        &fixture.recording_id,
        fixture.physical_size,
        false,
        json!({"state":"known", "outcome":"cancelled"}),
    );
    let safe_section = safe_preview["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["kind"] == "earlierOperation")
        .expect("admitted earlier operation section");
    assert_eq!(safe_section["label"], fixture.recording_id);
    assert_eq!(safe_section["files"], json!(["system2-private.json"]));
    assert_eq!(safe_section["bytes"], fixture.private_bytes.len() as u64);

    let mut text_query = safe_query.clone();
    text_query["review"] = safe_preview["review"].clone();
    text_query["key"] = safe_section["key"].clone();
    let (status, body, _) = call(
        &fixture.state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(text_query),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    assert!(json_of(&body)["text"]
        .as_str()
        .unwrap()
        .contains(&String::from_utf8(fixture.private_bytes.clone()).unwrap()));

    let mut safe_export = safe_query;
    safe_export["review"] = safe_preview["review"].clone();
    safe_export["consent"] = json!(true);
    let (status, body, _) = call(
        &fixture.state,
        "POST",
        "/api/diagnostics/bundle",
        Some(safe_export),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    let private_path = format!("operations/{}/system2-private.json", fixture.recording_id);
    let mut private = Vec::new();
    archive
        .by_name(&private_path)
        .unwrap()
        .read_to_end(&mut private)
        .unwrap();
    assert_eq!(private, fixture.private_bytes);
    assert!(archive
        .by_name(&format!(
            "operations/{}/recording.jsonl",
            fixture.recording_id
        ))
        .is_err());

    let unsafe_query = earlier_operation_query(&fixture, ABOVE_SAFE_JSON_INTEGER);
    let unsafe_preview = preview(&fixture.state, unsafe_query.clone()).await;
    assert!(unsafe_preview["recordings"]
        .as_array()
        .unwrap()
        .iter()
        .all(|summary| summary["id"] != fixture.recording_id));
    assert_eq!(
        offer(&unsafe_preview, "earlierOperation")["selected"],
        false
    );
    assert_eq!(
        offer(&unsafe_preview, "earlierOperation")["unavailable"],
        "the selected diagnostic source is unavailable",
        "unsafe earlier summary escaped admission"
    );
    assert!(unsafe_preview["sections"]
        .as_array()
        .unwrap()
        .iter()
        .all(|section| section["kind"] != "earlierOperation"));

    let mut unsafe_export = unsafe_query;
    unsafe_export["review"] = unsafe_preview["review"].clone();
    unsafe_export["consent"] = json!(true);
    let (status, body, _) = call(
        &fixture.state,
        "POST",
        "/api/diagnostics/bundle",
        Some(unsafe_export),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    assert!(!archive.file_names().any(|name| name == private_path));
    assert_eq!(
        fs::metadata(
            fixture
                .state
                .recordings
                .root()
                .join(format!("{}.jsonl", fixture.recording_id))
        )
        .unwrap()
        .len(),
        fixture.physical_size
    );
    let (status, body, _) = call(&fixture.state, "GET", "/api/diagnostics/recordings", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(
        row(&json_of(&body), &fixture.recording_id)["sizeBytes"],
        fixture.physical_size
    );
}

#[tokio::test]
async fn explicit_unsafe_recording_is_fixed_unavailable_before_any_section_file() {
    const ABOVE_SAFE_JSON_INTEGER: u64 = 9_007_199_254_740_992;
    let summary = synthetic_summary(ABOVE_SAFE_JSON_INTEGER);
    let query: super::super::BundleQuery = serde_json::from_value(json!({
        "problemType":"deviceReading",
        "operation":{"recordingId":summary.id},
        "sections":["operation"],
    }))
    .unwrap();

    let resolved = super::super::resolve_operation(
        &query,
        std::slice::from_ref(&summary),
        "server-incarnation",
    )
    .unwrap();
    assert!(
        resolved.is_none(),
        "unsafe summary reached evidence selection"
    );

    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let mut cache = super::super::System2Cache::default();
    let built = super::super::build_section(
        &state,
        &query,
        SectionKind::Operation,
        resolved.as_ref(),
        Err(super::super::SourceFailure::unavailable()),
        super::super::SectionScope {
            recordings: std::slice::from_ref(&summary),
            projectable: Err(super::super::SourceFailure::unavailable()),
        },
        &mut cache,
    )
    .await;
    let error = match built {
        Ok(sections) => panic!(
            "unsafe recording produced files: {} section(s)",
            sections.len()
        ),
        Err(error) => error,
    };
    assert_eq!(
        error.message(),
        "the selected diagnostic source is unavailable"
    );
}

#[test]
fn valid_recording_resolution_and_private_summary_keep_one_identity() {
    let summary = synthetic_summary(42);
    let query: super::super::BundleQuery = serde_json::from_value(json!({
        "problemType":"deviceReading",
        "operation":{"recordingId":summary.id},
    }))
    .unwrap();

    let resolved = super::super::resolve_operation(
        &query,
        std::slice::from_ref(&summary),
        "server-incarnation",
    )
    .unwrap()
    .expect("safe explicit summary");
    let admitted = super::super::admit_bundle_recording(&resolved).expect("private summary");

    assert_eq!(resolved.id, admitted.id);
    assert_eq!(resolved.size_bytes, admitted.size_bytes);
}

async fn wait_for_active_compare_header(state: &crate::SharedState, id: &str) -> u64 {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            if let Some(summary) = state
                .recordings
                .list()
                .unwrap()
                .into_iter()
                .find(|summary| {
                    summary.id == id
                        && summary.active
                        && summary.format_version == Some(1)
                        && summary.writer_version.is_some()
                        && summary.operation_kind.as_deref() == Some("compare")
                        && summary.started_at.is_some()
                        && summary.target == json!({"individualAddress":"1.1.5"})
                        && summary.gateway.as_deref() == Some("203.0.113.250:3671")
                        && !summary.torn_tail
                        && summary.size_bytes > 0
                })
            {
                break summary.size_bytes;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("complete active recording header became listable")
}

#[tokio::test]
async fn bundle_preview_uses_the_closed_private_recording_lifecycle() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(
        &state,
        OperationKind::Compare,
        ("privateLifecycle", "known"),
        true,
    );
    let size = state.recordings.read(&id).unwrap().len() as u64;

    let preview = preview(
        &state,
        json!({
            "problemType": "deviceReading",
            "operation": {"recordingId": id},
            "sections": ["environment"],
        }),
    )
    .await;

    assert_private_recording(
        row(&preview, &id),
        &id,
        size,
        false,
        json!({"state":"known", "outcome":"succeeded"}),
    );
    let encoded = serde_json::to_string(row(&preview, &id)).unwrap();
    for forbidden in [
        "writerVersion",
        "operationKind",
        "startedAt",
        "target",
        "gateway",
        "related",
    ] {
        assert!(
            !encoded.contains(forbidden),
            "raw field escaped: {forbidden}"
        );
    }
}

#[tokio::test]
async fn public_reports_and_existing_guards_remain_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(
        &state,
        OperationKind::Compare,
        ("publicShape", "retained"),
        true,
    );
    let (status, body, _) = call(&state, "GET", "/api/diagnostics/recordings", None).await;
    assert_eq!(status, StatusCode::OK);
    let listing = json_of(&body);
    let public = row(&listing, &id).clone();
    assert_eq!(public["formatVersion"], 1);
    assert_eq!(public["operationKind"], "compare");
    assert!(public["writerVersion"].as_str().is_some());
    assert!(public["startedAt"].as_str().is_some());
    assert_eq!(public["related"]["publicShape"], "retained");

    let bundle = preview(
        &state,
        json!({
            "problemType": "deviceReading",
            "operation": {"recordingId": id},
            "sections": ["environment"],
        }),
    )
    .await;
    assert!(row(&bundle, &id).get("writerVersion").is_none());
    assert!(row(&bundle, &id).get("operationKind").is_none());
    assert!(row(&bundle, &id).get("related").is_none());
}

#[tokio::test]
async fn active_recording_has_explicit_missing_end_without_completion_inference() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let recorder = crate::operation_recordings::start(
        &state,
        OperationStart::new(OperationKind::Compare)
            .target(json!({"individualAddress":"1.1.5"}))
            .gateway("203.0.113.250:3671")
            .related("privateLifecycle", "missing"),
    );
    let id = recorder.id().unwrap().to_string();
    let size = wait_for_active_compare_header(&state, &id).await;

    let preview = preview(
        &state,
        json!({
            "problemType": "deviceReading",
            "operation": {"recordingId": id},
            "sections": ["environment"],
        }),
    )
    .await;
    assert_private_recording(
        row(&preview, &id),
        &id,
        size,
        true,
        json!({"state":"missing"}),
    );
    recorder.finish(RecordedOutcome {
        status: OutcomeStatus::Cancelled,
        error_chain: Vec::new(),
        summary: Value::Null,
    });
}

#[tokio::test]
async fn unknown_recorded_outcome_is_withheld_without_copying_its_value() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(
        &state,
        OperationKind::Compare,
        ("privateLifecycle", "withheld"),
        true,
    );
    let path = state.recordings.root().join(format!("{id}.jsonl"));
    let mut bytes = fs::read(&path).unwrap();
    let from = b"\"outcome\":\"succeeded\"";
    let to = b"\"outcome\":\"futurexyz\"";
    assert_eq!(from.len(), to.len());
    let offset = bytes
        .windows(from.len())
        .position(|window| window == from)
        .expect("closed outcome in producer output");
    bytes[offset..offset + from.len()].copy_from_slice(to);
    fs::write(&path, &bytes).unwrap();

    let preview = preview(
        &state,
        json!({
            "problemType": "deviceReading",
            "operation": {"recordingId": id},
            "sections": ["environment"],
        }),
    )
    .await;
    assert_private_recording(
        row(&preview, &id),
        &id,
        bytes.len() as u64,
        false,
        json!({"state":"withheld"}),
    );
    assert!(!serde_json::to_string(row(&preview, &id))
        .unwrap()
        .contains("futurexyz"));
}

fn replace_header_value(bytes: &mut [u8], key: &str, replacement: u8) {
    let header_end = bytes
        .iter()
        .position(|byte| *byte == b'\n')
        .expect("recording header line");
    let needle = format!("\"{key}\":\"");
    let start = bytes[..header_end]
        .windows(needle.len())
        .position(|window| window == needle.as_bytes())
        .unwrap_or_else(|| panic!("header key {key}"))
        + needle.len();
    let end = bytes[start..header_end]
        .iter()
        .position(|byte| *byte == b'\"')
        .map(|offset| start + offset)
        .expect("plain JSON string value");
    assert!(end > start);
    bytes[start..end].fill(replacement);
}

#[tokio::test]
async fn withheld_equal_length_recording_metadata_is_review_noninterfering() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let recorder = crate::operation_recordings::start(
        &state,
        OperationStart::new(OperationKind::Compare)
            .target(json!({"individualAddress":"1.1.5", "targetOpaque":"aaaa"}))
            .gateway("203.0.113.250:3671")
            .related("relatedOpaque", "bbbb"),
    );
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(state.recordings.wait_until_idle(Duration::from_secs(10)));
    let id = recorder.id().unwrap().to_string();
    let query = json!({
        "problemType": "deviceReading",
        "operation": {"recordingId": id},
        "sections": ["environment"],
    });
    let baseline = preview(&state, query.clone()).await;
    let baseline_summary = row(&baseline, &id).clone();
    let baseline_fingerprint = baseline["review"]["fingerprint"].clone();
    let path = state.recordings.root().join(format!("{id}.jsonl"));
    let mut bytes = fs::read(&path).unwrap();
    let original_len = bytes.len();

    for (key, replacement) in [
        ("version", b'v'),
        ("started_at", b'2'),
        ("gateway", b'g'),
        ("targetOpaque", b't'),
        ("relatedOpaque", b'r'),
    ] {
        replace_header_value(&mut bytes, key, replacement);
        fs::write(&path, &bytes).unwrap();
        assert_eq!(fs::metadata(&path).unwrap().len(), original_len as u64);
        let changed = preview(&state, query.clone()).await;
        assert_eq!(row(&changed, &id), &baseline_summary, "withheld {key}");
        assert_eq!(changed["offers"], baseline["offers"], "withheld {key}");
        assert_eq!(changed["sections"], baseline["sections"], "withheld {key}");
        assert_eq!(
            changed["operationRecordingId"], baseline["operationRecordingId"],
            "withheld {key}"
        );
        assert_eq!(changed["deviceId"], baseline["deviceId"], "withheld {key}");
        assert_eq!(
            changed["totalBytes"], baseline["totalBytes"],
            "withheld {key}"
        );
        assert_eq!(
            changed["review"]["fingerprint"], baseline_fingerprint,
            "withheld {key} changed reviewed contents"
        );
    }
}

#[tokio::test]
async fn selected_recording_lifecycle_is_an_effective_review_dependency() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let recorder = crate::operation_recordings::start(
        &state,
        OperationStart::new(OperationKind::Compare)
            .target(json!({"individualAddress":"1.1.5"}))
            .gateway("203.0.113.250:3671"),
    );
    let id = recorder.id().unwrap().to_string();
    let _active_size = wait_for_active_compare_header(&state, &id).await;
    let query = json!({
        "problemType": "deviceReading",
        "operation": {"recordingId": id},
        "sections": ["environment"],
    });
    let active = preview(&state, query.clone()).await;
    recorder.finish(RecordedOutcome::failed(
        vec!["fixed failure".into()],
        Value::Null,
    ));
    assert!(state.recordings.wait_until_idle(Duration::from_secs(10)));
    let finished = preview(&state, query).await;

    assert_ne!(
        active["review"]["fingerprint"], finished["review"]["fingerprint"],
        "selected missing-to-known lifecycle transition must invalidate review"
    );
}

#[tokio::test]
async fn consented_raw_recording_body_is_exported_unchanged_with_the_private_note() {
    // Owner decision 2026-10-31: the recorder-redacted raw body may leave after
    // consent, unchanged and with its visible "keep private" note.
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let ids = [
        record(&state, OperationKind::Compare, ("rawBody", "raw-a"), true),
        record(
            &state,
            OperationKind::DeviceRead,
            ("differentRawBody", "raw-b"),
            false,
        ),
    ];
    for id in ids {
        let mut query = json!({
            "problemType": "deviceReading",
            "operation": {"recordingId": id},
            "sections": ["operation"],
        });
        let preview = preview(&state, query.clone()).await;
        let offer = preview["offers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|offer| offer["kind"] == "operation")
            .expect("operation offer");
        assert_eq!(offer["selected"], true, "{offer}");
        assert_eq!(offer["unavailable"], Value::Null);
        let section = preview["sections"]
            .as_array()
            .unwrap()
            .iter()
            .find(|section| section["kind"] == "operation")
            .expect("operation section");
        assert_eq!(section["files"], json!(["recording.jsonl"]));
        assert!(section["notes"]
            .as_array()
            .unwrap()
            .iter()
            .any(|note| note.as_str().unwrap().contains("possibly passwords")));
        query["review"] = preview["review"].clone();
        query["consent"] = json!(true);
        let (status, body, _) = call(&state, "POST", "/api/diagnostics/bundle", Some(query)).await;
        assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
        let mut zip = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
        let mut exported = Vec::new();
        zip.by_name(&format!("operations/{id}/recording.jsonl"))
            .unwrap()
            .read_to_end(&mut exported)
            .unwrap();
        assert_eq!(
            exported,
            state.recordings.read(&id).unwrap(),
            "never rewritten"
        );
        let text = String::from_utf8_lossy(&exported).to_ascii_lowercase();
        assert!(
            text.contains("\"redacted\":{\"kind\":\"authorize-key\""),
            "{text}"
        );
        assert!(!text.contains("c0ffee42"), "a recognised key escaped");
    }
}

#[tokio::test]
async fn user_description_is_fixed_withheld_and_never_binds_review_to_its_value() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let base = json!({
        "problemType": "userInterface",
        "sections": ["environment"],
        "description": "secret-description-a",
    });
    let first = preview(&state, base.clone()).await;
    let mut second_query = base.clone();
    second_query["description"] = json!("secret-description-b");
    let second = preview(&state, second_query).await;
    assert_eq!(
        first["review"]["fingerprint"],
        second["review"]["fingerprint"]
    );

    let mut export = base;
    export["review"] = first["review"].clone();
    export["consent"] = json!(true);
    let (status, bytes, _) = call(&state, "POST", "/api/diagnostics/bundle", Some(export)).await;
    assert_eq!(status, StatusCode::OK);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest: Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(
        manifest["description"],
        "User description withheld from diagnostic bundle."
    );
    assert!(!manifest.to_string().contains("secret-description"));
}

#[tokio::test]
async fn all_outward_surfaces_use_only_admitted_values() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let query = json!({
        "problemType": "deviceReading",
        "operation": {"selection": "none"},
        "sections": ["sessionLog", "device", "catalogue", "busMonitor"],
    });
    let view = preview(&state, query.clone()).await;
    // Without a source and a chosen device only a device-free catalogue is offered.
    let catalogue = view["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|offer| offer["kind"] == "catalogue")
        .unwrap();
    assert_eq!(catalogue["selected"], true, "{catalogue}");
    assert_eq!(catalogue["unavailable"], Value::Null);
    let withheld = "this section is withheld until its content is admitted as free of credentials";
    for (kind, reason) in [
        ("sessionLog", withheld),
        ("device", "no device is chosen"),
        ("busMonitor", withheld),
        ("legacyReports", "no operation is chosen"),
    ] {
        let offer = view["offers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|offer| offer["kind"] == kind)
            .unwrap();
        assert_eq!(offer["selected"], false, "{offer}");
        assert_eq!(offer["unavailable"], reason, "{kind}");
        assert!(view["sections"]
            .as_array()
            .unwrap()
            .iter()
            .all(|section| section["kind"] != kind));
    }
    let mut reviewed = query;
    reviewed["review"] = view["review"].clone();
    reviewed["consent"] = json!(true);
    let (status, bytes, _) = call(&state, "POST", "/api/diagnostics/bundle", Some(reviewed)).await;
    assert_eq!(status, StatusCode::OK);
    let archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let names: Vec<_> = archive.file_names().map(str::to_string).collect();
    assert_eq!(
        names,
        [
            "manifest.json",
            "environment/environment.json",
            "catalogue/catalogue.json"
        ]
    );

    let client = preview(
        &state,
        json!({
            "problemType": "userInterface",
            "operation": {"selection": "none"},
            "sections": ["clientState"],
            "client": {"recentErrors": ["PRIVATE_CLIENT_SENTINEL"]},
        }),
    )
    .await;
    let offer = client["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|offer| offer["kind"] == "clientState")
        .unwrap();
    assert_eq!(offer["selected"], false);
    assert_eq!(
        offer["unavailable"],
        "this section is withheld until its content is admitted as free of credentials"
    );
    assert!(!client.to_string().contains("PRIVATE_CLIENT_SENTINEL"));
}

#[tokio::test]
async fn recording_listing_is_bounded_private_io_not_zero_body_io() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(
        &state,
        OperationKind::Compare,
        ("boundedListing", "control"),
        true,
    );
    let path = state.recordings.root().join(format!("{id}.jsonl"));
    let original = fs::read(&path).unwrap();
    let first_end = original.iter().position(|byte| *byte == b'\n').unwrap() + 1;
    let last_start = original[..original.len() - 1]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .unwrap()
        + 1;
    let mut enlarged = original[..first_end].to_vec();
    enlarged.extend(std::iter::repeat_n(b'X', 1024 * 1024 + 64 * 1024 + 1));
    enlarged.push(b'\n');
    enlarged.extend_from_slice(&original[last_start..]);
    fs::write(&path, &enlarged).unwrap();

    let listed = state.recordings.list().unwrap();
    let summary = listed.iter().find(|summary| summary.id == id).unwrap();
    assert_eq!(summary.size_bytes, enlarged.len() as u64);
    let view = preview(
        &state,
        json!({
            "problemType": "deviceReading",
            "operation": {"recordingId": id},
            "sections": ["operation"],
        }),
    )
    .await;
    assert_eq!(row(&view, &id)["sizeBytes"], enlarged.len() as u64);
    let reviewed_fingerprint = view["review"]["fingerprint"].clone();
    let body_offset = first_end + 32;
    enlarged[body_offset] = b'Y';
    fs::write(&path, &enlarged).unwrap();
    assert_eq!(fs::metadata(&path).unwrap().len(), enlarged.len() as u64);
    let body_changed = preview(
        &state,
        json!({
            "problemType": "deviceReading",
            "operation": {"recordingId": id},
            "sections": ["operation"],
        }),
    )
    .await;
    // Owner decision 2026-10-31: the raw body is exported, so every body byte is
    // part of what the user reviewed; the listing itself stays bounded metadata.
    assert_ne!(
        body_changed["review"]["fingerprint"], reviewed_fingerprint,
        "an exported raw body byte must invalidate the review"
    );
    assert_eq!(row(&body_changed, &id), row(&view, &id));
    assert!(!view.to_string().contains(&"X".repeat(128)));
    let offer = view["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|offer| offer["kind"] == "operation")
        .unwrap();
    assert_eq!(offer["selected"], true, "{offer}");
    assert_eq!(offer["unavailable"], Value::Null);
}
