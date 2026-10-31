//! Helpers for real-producer R4 diagnostic bundle route witnesses.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use knx_app::operation_recording::{
    OperationKind, OperationRecorder, OperationStart, RecordingSummary,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

use super::super::{call, json_of, record, state};

pub(super) struct PinnedFixture {
    pub(super) state: Arc<crate::AppState>,
    pub(super) a_id: String,
    pub(super) c_id: String,
    pub(super) a_bytes: Vec<u8>,
    pub(super) c_bytes: Vec<u8>,
    pub(super) query: Value,
    _dir: tempfile::TempDir,
}

impl PinnedFixture {
    pub(super) fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        let c_id = record(&state, OperationKind::Compare, ("r4Role", "C"), true);
        let a_id = record(&state, OperationKind::Compare, ("r4Role", "A"), true);
        assert!(c_id < a_id, "C must be the genuinely earlier completed run");
        let c_bytes = state.recordings.read(&c_id).unwrap();
        let a_bytes = state.recordings.read(&a_id).unwrap();
        assert!(!a_bytes.is_empty() && !c_bytes.is_empty());
        let query = pinned_query(&a_id, &c_id);
        Self {
            state,
            a_id,
            c_id,
            a_bytes,
            c_bytes,
            query,
            _dir: dir,
        }
    }

    pub(super) fn start_ambient(&self, label: &str) -> OperationRecorder {
        let recorder = crate::operation_recordings::start(
            &self.state,
            OperationStart::new(OperationKind::Compare)
                .target(json!({"individualAddress":"1.1.5"}))
                .gateway("203.0.113.250:3671")
                .related("r4Role", label),
        );
        assert!(recorder.id().is_some(), "ambient producer must record");
        recorder
    }

    pub(super) fn complete_ambient(&self, label: &str) -> String {
        record(&self.state, OperationKind::Compare, ("r4Role", label), true)
    }
}

pub(super) fn pinned_query(a_id: &str, c_id: &str) -> Value {
    json!({
        "problemType": "deviceReading",
        "operation": {"recordingId": a_id},
        "earlierRecordingIds": [c_id],
        "sections": ["environment", "operation", "earlierOperation"],
    })
}

pub(super) async fn wait_for_recording(
    state: &Arc<crate::AppState>,
    id: &str,
    ready: impl Fn(&RecordingSummary, &[u8]) -> bool,
) -> (RecordingSummary, Vec<u8>) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let summary = state
                .recordings
                .list()
                .unwrap()
                .into_iter()
                .find(|summary| summary.id == id);
            if let Some(summary) = summary {
                if let Ok(bytes) = state.recordings.read(id) {
                    if ready(&summary, &bytes) {
                        return (summary, bytes);
                    }
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap_or_else(|_| panic!("recording {id} never reached the required persisted/list state"))
}

pub(super) async fn preview_ok(state: &Arc<crate::AppState>, query: &Value) -> Value {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    json_of(&body)
}

pub(super) fn fingerprint(preview: &Value) -> &str {
    preview["review"]["fingerprint"]
        .as_str()
        .expect("preview has a review fingerprint")
}

pub(super) fn chooser_ids(preview: &Value) -> Vec<String> {
    preview["recordings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|summary| summary["id"].as_str().unwrap().to_string())
        .collect()
}

pub(super) fn assert_chooser_contains(preview: &Value, id: &str) {
    assert!(chooser_ids(preview).iter().any(|candidate| candidate == id));
}

fn row<'a>(rows: &'a Value, field: &str, value: &str) -> &'a Value {
    rows.as_array()
        .unwrap()
        .iter()
        .find(|row| row[field] == value)
        .unwrap_or_else(|| panic!("missing {field}={value}"))
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct PinnedProjection {
    problem_type: Value,
    device_id: Value,
    context: Value,
    offers: Vec<Value>,
    sections: Vec<Value>,
    summaries: Vec<Value>,
}

pub(super) fn pinned_projection(preview: &Value, fixture: &PinnedFixture) -> PinnedProjection {
    assert_eq!(preview["operationRecordingId"], fixture.a_id);
    assert_eq!(preview["problemType"], "deviceReading");
    assert_eq!(preview["deviceId"], Value::Null);
    assert!(preview["review"]["context"]["serverIncarnation"].is_string());
    assert!(preview["review"]["context"]["projectIncarnation"].is_u64());
    assert!(preview["review"]["context"]["revision"].is_u64());
    let a_key = format!("operation:{}", fixture.a_id);
    let c_key = format!("earlierOperation:{}", fixture.c_id);
    let sections = ["environment", a_key.as_str(), c_key.as_str()]
        .into_iter()
        .map(|key| row(&preview["sections"], "key", key).clone())
        .collect::<Vec<_>>();
    assert_eq!(
        preview["sections"].as_array().unwrap().len(),
        sections.len()
    );
    // Owner decision 2026-10-31: the redacted raw recordings leave after consent.
    for (section, id, bytes) in [
        (&sections[1], fixture.a_id.as_str(), &fixture.a_bytes),
        (&sections[2], fixture.c_id.as_str(), &fixture.c_bytes),
    ] {
        assert_eq!(section["label"], id);
        assert_eq!(section["bytes"], bytes.len() as u64);
        assert_eq!(section["files"], json!(["recording.jsonl"]));
        assert!(section["entries"].as_u64().unwrap() > 0);
    }
    let summaries = [&fixture.a_id, &fixture.c_id]
        .into_iter()
        .map(|id| row(&preview["recordings"], "id", id).clone())
        .collect::<Vec<_>>();
    for (summary, bytes) in summaries.iter().zip([&fixture.a_bytes, &fixture.c_bytes]) {
        assert_eq!(summary["active"], false);
        assert_eq!(
            summary["end"],
            json!({"state":"known","outcome":"succeeded"})
        );
        assert_eq!(summary["sizeBytes"], bytes.len() as u64);
        assert_eq!(summary["formatVersion"], 1);
        assert_eq!(summary.as_object().unwrap().len(), 6);
    }
    let selected_offers = ["environment", "operation", "earlierOperation"]
        .into_iter()
        .map(|kind| row(&preview["offers"], "kind", kind).clone())
        .collect::<Vec<_>>();
    assert!(selected_offers
        .iter()
        .all(|offer| offer["selected"] == true));
    assert!(selected_offers
        .iter()
        .all(|offer| offer["unavailable"] == Value::Null));
    assert_eq!(selected_offers[0]["offer"], "fixed");
    assert_eq!(selected_offers[1]["offer"], "preselected");
    assert_eq!(selected_offers[2]["offer"], "offered");
    PinnedProjection {
        problem_type: preview["problemType"].clone(),
        device_id: preview["deviceId"].clone(),
        context: preview["review"]["context"].clone(),
        offers: preview["offers"].as_array().unwrap().clone(),
        sections,
        summaries,
    }
}

fn reviewed(query: &Value, preview: &Value, consent: bool) -> Value {
    let mut reviewed = query.clone();
    reviewed["review"] = preview["review"].clone();
    if consent {
        reviewed["consent"] = json!(true);
    }
    reviewed
}

pub(super) async fn assert_reviewed_section_texts(
    fixture: &PinnedFixture,
    original_preview: &Value,
) {
    for (key, id, bytes) in [
        (
            format!("operation:{}", fixture.a_id),
            fixture.a_id.as_str(),
            fixture.a_bytes.as_slice(),
        ),
        (
            format!("earlierOperation:{}", fixture.c_id),
            fixture.c_id.as_str(),
            fixture.c_bytes.as_slice(),
        ),
    ] {
        let mut query = reviewed(&fixture.query, original_preview, false);
        query["key"] = json!(key);
        let (status, body, disposition) = call(
            &fixture.state,
            "POST",
            "/api/diagnostics/bundle/section",
            Some(query),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
        assert_eq!(disposition, None);
        let response = json_of(&body);
        let expected = format!(
            "===== operations/{id}/recording.jsonl =====\n{}",
            String::from_utf8_lossy(bytes)
        );
        assert_eq!(response["text"], expected);
    }
}

pub(super) async fn export_response(
    fixture: &PinnedFixture,
    original_preview: &Value,
) -> (StatusCode, Vec<u8>) {
    export_response_for(&fixture.state, &fixture.query, original_preview).await
}

pub(super) async fn export_response_for(
    state: &Arc<crate::AppState>,
    query: &Value,
    preview: &Value,
) -> (StatusCode, Vec<u8>) {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle",
        Some(reviewed(query, preview, true)),
    )
    .await;
    (status, body)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ObservedTimestamps {
    created_at: String,
    consent_at: String,
}

#[derive(Clone, Debug)]
pub(super) struct ExportSnapshot {
    sections: Vec<Value>,
    entries: BTreeMap<String, Vec<u8>>,
    pub(super) timestamps: ObservedTimestamps,
}

pub(super) fn exported_snapshot(
    status: StatusCode,
    body: Vec<u8>,
    preview: &Value,
    fixture: &PinnedFixture,
    excluded_ids: &[&str],
) -> ExportSnapshot {
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    let mut entries = BTreeMap::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let name = file.name().to_string();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        assert!(entries.insert(name, bytes).is_none());
    }
    let manifest_bytes = entries.remove("manifest.json").unwrap();
    let manifest: Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let expected_paths = BTreeSet::from([
        "environment/environment.json".to_string(),
        format!("operations/{}/recording.jsonl", fixture.a_id),
        format!("operations/{}/recording.jsonl", fixture.c_id),
    ]);
    assert_eq!(
        entries.keys().cloned().collect::<BTreeSet<_>>(),
        expected_paths
    );
    assert_eq!(
        entries[&format!("operations/{}/recording.jsonl", fixture.a_id)],
        fixture.a_bytes
    );
    assert_eq!(
        entries[&format!("operations/{}/recording.jsonl", fixture.c_id)],
        fixture.c_bytes
    );
    for id in excluded_ids {
        assert!(entries.keys().all(|path| !path.contains(id)));
    }

    let sections = manifest["sections"].as_array().unwrap().clone();
    assert_eq!(
        sections.len(),
        preview["sections"].as_array().unwrap().len()
    );
    for shown in preview["sections"].as_array().unwrap() {
        let key = shown["key"].as_str().unwrap();
        let included = sections
            .iter()
            .find(|section| section["key"] == key)
            .unwrap();
        for field in ["key", "kind", "label", "entries", "markers", "notes"] {
            assert_eq!(included[field], shown[field], "{key}: {field}");
        }
        let files = included["files"].as_array().unwrap();
        assert_eq!(files.len(), shown["files"].as_array().unwrap().len());
        let mut section_bytes = 0_u64;
        for file in files {
            let path = file["path"].as_str().unwrap();
            let bytes = &entries[path];
            section_bytes += bytes.len() as u64;
            assert_eq!(file["bytes"], bytes.len() as u64);
            assert_eq!(file["sha256"], format!("{:x}", Sha256::digest(bytes)));
            assert!(shown["files"]
                .as_array()
                .unwrap()
                .iter()
                .any(|name| path.ends_with(name.as_str().unwrap())));
        }
        assert_eq!(shown["bytes"], section_bytes);
    }
    assert_eq!(
        manifest["totalBytes"],
        entries
            .values()
            .map(|bytes| bytes.len() as u64)
            .sum::<u64>()
    );
    let timestamps = ObservedTimestamps {
        created_at: manifest["createdAt"].as_str().unwrap().to_string(),
        consent_at: manifest["consentAt"].as_str().unwrap().to_string(),
    };
    assert!(!timestamps.created_at.is_empty() && !timestamps.consent_at.is_empty());
    ExportSnapshot {
        sections,
        entries,
        timestamps,
    }
}

pub(super) fn assert_same_selected_export(before: &ExportSnapshot, after: &ExportSnapshot) {
    assert_eq!(after.sections, before.sections);
    assert_eq!(after.entries, before.entries);
}
