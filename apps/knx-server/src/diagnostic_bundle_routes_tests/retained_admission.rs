//! Route regressions for exact retained-result admission without another device read.

use super::{call, json_of, state};
use axum::http::StatusCode;
use knx_app::device_diagnostics::{self, Export};
use knx_app::operation_recording::{
    OperationKind, OperationStart, RecordedOutcome, RecordingSummary,
};
use knx_app::support_report::{build_report, ReportInput};
use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use serde_json::{json, Value};
use std::io::Read;
use std::sync::atomic::AtomicBool;

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(7),
        source: SourceRef {
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
    project
}

async fn identity_export() -> Export {
    let project = project();
    let plan = device_diagnostics::prepare(&project, DeviceId(7), None).unwrap();
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0025,
        ..Default::default()
    });
    device.preset_property(0, 12, &[0, 1]);
    device.preset_property(0, 78, &[0, 1, 0, 0, 0, 0x5b]);
    let report = device_diagnostics::acquire(
        &device,
        &plan,
        Default::default(),
        &crate::version_string(),
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    Export {
        report,
        memory: None,
    }
}

fn section_offer<'a>(preview: &'a Value, kind: &str) -> &'a Value {
    preview["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|offer| offer["kind"] == kind)
        .unwrap()
}

async fn historical_recording(
    token: &str,
) -> (
    tempfile::TempDir,
    std::sync::Arc<crate::AppState>,
    RecordingSummary,
) {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let recorder = crate::operation_recordings::start(
        &state,
        OperationStart::new(OperationKind::DeviceDiagnostics)
            .target(json!({"deviceId":7,"individualAddress":"1.1.25"}))
            .related("diagnosticsToken", token),
    );
    let id = recorder.id().unwrap().to_string();
    let export = identity_export().await;
    let summary = json!({
        "report": serde_json::to_value(&export.report).unwrap_or_default(),
        "memory": export
            .memory
            .as_ref()
            .map(|memory| serde_json::to_value(memory).unwrap_or_default()),
    });
    recorder.finish(RecordedOutcome::succeeded(summary));
    assert!(state
        .recordings
        .wait_until_idle(std::time::Duration::from_secs(10)));
    let summary = state
        .recordings
        .list()
        .unwrap()
        .into_iter()
        .find(|summary| summary.id == id)
        .unwrap();
    (dir, state, summary)
}

#[tokio::test]
async fn historical_result_reads_the_exact_completed_recording_identity() {
    let (_dir, state, summary) = historical_recording("historical-token-a").await;
    let mut observed = None;
    let evidence = super::super::retained::diagnostic_recording_observed(
        &state,
        Some(&summary),
        "historical-token-a",
        &mut |bytes| observed = Some(bytes),
    )
    .unwrap_or_else(|_| panic!("valid historical diagnostic result was withheld"));
    assert_eq!(observed, Some(summary.size_bytes as usize));
    assert_eq!(evidence.reports.len(), 3);
    assert_eq!(
        evidence
            .reports
            .iter()
            .map(|file| file.name.as_str())
            .collect::<Vec<_>>(),
        ["report.json", "properties.json", "README.txt"]
    );
    assert_eq!(evidence.device_id, Some(7));
    assert_eq!(evidence.context, super::super::recording_context(&summary));
}

#[tokio::test]
async fn historical_result_refuses_equal_length_header_replacement_after_listing() {
    let token_a = "historical-token-a";
    let token_b = "historical-token-b";
    let (_dir, state, summary) = historical_recording(token_a).await;
    let path = state
        .recordings
        .root()
        .join(format!("{}.jsonl", summary.id));
    let original = std::fs::read_to_string(&path).unwrap();
    assert_eq!(original.matches(token_a).count(), 1);
    let replaced = original.replace(token_a, token_b);
    assert_eq!(replaced.len(), original.len());
    std::fs::write(&path, replaced).unwrap();
    let mut observed = None;
    let result = super::super::retained::diagnostic_recording_observed(
        &state,
        Some(&summary),
        token_a,
        &mut |bytes| observed = Some(bytes),
    );
    let error = match result {
        Err(super::super::SourceFailure::Unavailable(code)) => code,
        _ => panic!("a replaced historical header was admitted"),
    };
    assert!(matches!(
        error,
        super::super::BundleUnavailableCode::RecordingResult
    ));
    assert_eq!(
        error.message(),
        "the diagnostic recording result is malformed"
    );
    assert_eq!(observed, Some(summary.size_bytes as usize));
}

#[tokio::test]
async fn historical_growth_is_refused_without_an_unbounded_owned_read() {
    const MAX_HISTORICAL_BYTES: u64 = 8 * 1024 * 1024;
    let token = "historical-token-c";
    let (_dir, state, summary) = historical_recording(token).await;
    let path = state
        .recordings
        .root()
        .join(format!("{}.jsonl", summary.id));
    std::fs::OpenOptions::new()
        .write(true)
        .open(path)
        .unwrap()
        .set_len(MAX_HISTORICAL_BYTES + 2)
        .unwrap();
    let mut observed = None;
    let result = super::super::retained::diagnostic_recording_observed(
        &state,
        Some(&summary),
        token,
        &mut |bytes| observed = Some(bytes),
    );
    let error = match result {
        Err(super::super::SourceFailure::Unavailable(code)) => code,
        _ => panic!("a grown historical source was admitted"),
    };
    assert!(matches!(
        error,
        super::super::BundleUnavailableCode::RecordingResult
    ));
    assert_eq!(
        error.message(),
        "the diagnostic recording result is malformed"
    );
    assert!(
        observed.unwrap_or_default() <= MAX_HISTORICAL_BYTES as usize + 1,
        "historical source exceeded the bounded owned-read witness"
    );
}

#[tokio::test]
async fn unsupported_historical_version_stays_a_distinct_payload_free_refusal() {
    let token = "historical-token-d";
    let (_dir, state, mut summary) = historical_recording(token).await;
    summary.format_version = Some(2);
    let mut observed = None;
    let result = super::super::retained::diagnostic_recording_observed(
        &state,
        Some(&summary),
        token,
        &mut |bytes| observed = Some(bytes),
    );
    let error = match result {
        Err(super::super::SourceFailure::Unavailable(code)) => code,
        _ => panic!("an unsupported historical version was admitted"),
    };
    assert!(matches!(
        error,
        super::super::BundleUnavailableCode::RecordingVersion
    ));
    assert_eq!(
        error.message(),
        "the diagnostic recording version is unsupported"
    );
    assert_eq!(observed, None);
}

async fn retained_unavailable_message(
    state: &std::sync::Arc<crate::AppState>,
    token: &str,
) -> Value {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({
            "problemType":"deviceReading",
            "operation":{"relatedKey":"diagnosticsToken","relatedValue":token},
            "sections":["operation"]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let preview = json_of(&body);
    section_offer(&preview, "operation")["unavailable"].clone()
}

#[tokio::test]
async fn missing_retained_result_keeps_a_specific_payload_free_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), false);
    assert_eq!(
        retained_unavailable_message(&state, "missing-diagnostic-token").await,
        "the retained diagnostic result is unavailable"
    );
}

async fn seeded_retained_unavailable(
    retention: crate::device_diagnostics_routes::BundleTestRetention,
) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), false);
    let context = crate::diagnostic_bundle_context::ProjectContext::capture(&state);
    state.diagnostic_results.retain_for_bundle_test_as(
        "retained-state-token",
        identity_export().await,
        context,
        7,
        retention,
    );
    retained_unavailable_message(&state, "retained-state-token").await
}

#[tokio::test]
async fn expired_retained_result_keeps_a_specific_payload_free_refusal() {
    assert_eq!(
        seeded_retained_unavailable(crate::device_diagnostics_routes::BundleTestRetention::Expired)
            .await,
        "the retained diagnostic result has expired"
    );
}

#[tokio::test]
async fn unfinished_retained_result_keeps_a_specific_payload_free_refusal() {
    assert_eq!(
        seeded_retained_unavailable(
            crate::device_diagnostics_routes::BundleTestRetention::Unfinished
        )
        .await,
        "the diagnostic result is not complete"
    );
}

#[tokio::test]
async fn non_exportable_retained_result_keeps_a_specific_payload_free_refusal() {
    assert_eq!(
        seeded_retained_unavailable(
            crate::device_diagnostics_routes::BundleTestRetention::NonExportable
        )
        .await,
        "the diagnostic result is not exportable"
    );
}

#[tokio::test]
async fn exact_retained_identity_reaches_preview_text_and_zip_without_a_recording() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), false);
    *state.project.lock().unwrap() = Some(project());
    let context = crate::diagnostic_bundle_context::ProjectContext::capture(&state);
    let export = identity_export().await;
    let expected = serde_json::to_value(&export.report).unwrap();
    state
        .diagnostic_results
        .retain_for_bundle_test("diagnostic-token-a", export, context, 7);
    let mut query = json!({
        "problemType": "deviceReading",
        "operation": {"relatedKey":"diagnosticsToken","relatedValue":"diagnostic-token-a"},
        "sections": ["operation", "legacyReports"]
    });
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let preview = json_of(&body);
    assert_eq!(preview["operationRecordingId"], Value::Null);
    for kind in ["operation", "legacyReports"] {
        assert_eq!(section_offer(&preview, kind)["unavailable"], Value::Null);
    }
    let report_section = preview["sections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|section| section["kind"] == "legacyReports")
        .unwrap();
    let mut text_query = query.clone();
    text_query["key"] = report_section["key"].clone();
    text_query["review"] = preview["review"].clone();
    let (status, text, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(text_query),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&text));
    assert!(json_of(&text)["text"]
        .as_str()
        .unwrap()
        .contains("reports/report.json"));
    query["review"] = preview["review"].clone();
    query["consent"] = json!(true);
    let (status, bytes, _) = call(&state, "POST", "/api/diagnostics/bundle", Some(query)).await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&bytes));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    let mut report = Vec::new();
    archive
        .by_name("reports/report.json")
        .unwrap()
        .read_to_end(&mut report)
        .unwrap();
    assert_eq!(serde_json::from_slice::<Value>(&report).unwrap(), expected);
    assert!(archive
        .file_names()
        .all(|name| !name.ends_with("recording.jsonl")));
}

#[tokio::test]
async fn projectless_retained_support_keeps_its_exact_result_namespace() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), false);
    let report = build_report(
        None,
        None,
        None,
        &ReportInput {
            program_id: Some("M-0001_A-0001-01-0001".into()),
            firmware: Some("1.2.3".into()),
            mask: Some(0x0025),
            ..Default::default()
        },
        &crate::version_string(),
    );
    state
        .support_results
        .retain_for_bundle_test("support-token-a", report.clone(), None);
    let query = json!({
        "problemType":"deviceReading",
        "operation":{"relatedKey":"supportToken","relatedValue":"support-token-a"},
        "sections":["operation","legacyReports"]
    });
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let preview = json_of(&body);
    assert_eq!(preview["operationRecordingId"], Value::Null);
    assert_eq!(preview["deviceId"], Value::Null);
    assert_eq!(
        section_offer(&preview, "operation")["unavailable"],
        Value::Null
    );
    assert_eq!(
        section_offer(&preview, "legacyReports")["unavailable"],
        Value::Null
    );
}

async fn support_unavailable_message(
    state: &std::sync::Arc<crate::AppState>,
    token: &str,
) -> Value {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({
            "problemType":"deviceReading",
            "operation":{"relatedKey":"supportToken","relatedValue":token},
            "sections":["legacyReports"]
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let preview = json_of(&body);
    assert_eq!(preview["operationRecordingId"], Value::Null);
    let reports = section_offer(&preview, "legacyReports");
    assert_eq!(reports["selected"], false);
    reports["unavailable"].clone()
}

async fn seeded_support_unavailable(
    retention: crate::support_routes::BundleTestRetention,
) -> Value {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), false);
    let report = build_report(
        None,
        None,
        None,
        &ReportInput {
            program_id: Some("M-0001_A-0001-01-0001".into()),
            firmware: Some("1.2.3".into()),
            mask: Some(0x0025),
            ..Default::default()
        },
        &crate::version_string(),
    );
    state
        .support_results
        .retain_for_bundle_test_as("support-state-token", report, None, retention);
    support_unavailable_message(&state, "support-state-token").await
}

#[tokio::test]
async fn missing_retained_support_keeps_a_support_family_payload_free_refusal() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), false);
    assert_eq!(
        support_unavailable_message(&state, "missing-support-token").await,
        "the retained support result is unavailable"
    );
}

#[tokio::test]
async fn expired_retained_support_keeps_a_support_family_payload_free_refusal() {
    assert_eq!(
        seeded_support_unavailable(crate::support_routes::BundleTestRetention::Expired).await,
        "the retained support result has expired"
    );
}

#[tokio::test]
async fn unacquired_retained_support_keeps_a_support_family_payload_free_refusal() {
    assert_eq!(
        seeded_support_unavailable(crate::support_routes::BundleTestRetention::Unacquired).await,
        "the support result is not complete"
    );
}

#[tokio::test]
async fn retained_result_and_recorded_target_conflict_is_not_borrowed() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    *state.project.lock().unwrap() = Some(project());
    let context = crate::diagnostic_bundle_context::ProjectContext::capture(&state);
    state.diagnostic_results.retain_for_bundle_test(
        "diagnostic-token-b",
        identity_export().await,
        context.clone(),
        7,
    );
    let recorded_context = json!({
        "serverIncarnation": context.server_incarnation,
        "projectIncarnation": context.project_incarnation,
        "revision": context.revision
    });
    let recorder = crate::operation_recordings::start(
        &state,
        OperationStart::new(OperationKind::DeviceDiagnostics)
            .target(json!({"deviceId":8,"individualAddress":"1.1.26"}))
            .related("diagnosticsToken", "diagnostic-token-b")
            .related("projectContext", recorded_context.to_string()),
    );
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(state
        .recordings
        .wait_until_idle(std::time::Duration::from_secs(10)));
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({
            "problemType":"deviceReading",
            "operation":{"relatedKey":"diagnosticsToken","relatedValue":"diagnostic-token-b"}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", json_of(&body));
    assert_eq!(json_of(&body)["error"], "bundleSourceConflict");
}

#[tokio::test]
async fn a_cleared_retained_context_never_lends_its_device_to_the_recording() {
    // The retained slot's context differs from its recording's, so provenance
    // clears it: the slot's device id belongs to a ruled-out namespace.
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    *state.project.lock().unwrap() = Some(project());
    let current = crate::diagnostic_bundle_context::ProjectContext::capture(&state);
    let stale = crate::diagnostic_bundle_context::ProjectContext {
        revision: current.revision + 1,
        ..current.clone()
    };
    state.diagnostic_results.retain_for_bundle_test(
        "diagnostic-token-c",
        identity_export().await,
        stale,
        7,
    );
    let recorded_context = json!({
        "serverIncarnation": current.server_incarnation,
        "projectIncarnation": current.project_incarnation,
        "revision": current.revision
    });
    let recorder = crate::operation_recordings::start(
        &state,
        OperationStart::new(OperationKind::DeviceDiagnostics)
            .target(json!({"individualAddress":"1.1.99"}))
            .related("diagnosticsToken", "diagnostic-token-c")
            .related("projectContext", recorded_context.to_string()),
    );
    recorder.finish(RecordedOutcome::succeeded(Value::Null));
    assert!(state
        .recordings
        .wait_until_idle(std::time::Duration::from_secs(10)));
    // Ids of the cleared namespace are not comparable: no 409, only unavailable.
    for chosen in [None, Some(7), Some(8)] {
        let mut query = json!({"problemType":"deviceReading",
            "operation":{"relatedKey":"diagnosticsToken","relatedValue":"diagnostic-token-c"},
            "sections":["device","catalogue"]});
        if let Some(chosen) = chosen {
            query["deviceId"] = json!(chosen);
        }
        let preview = preview_of(&state, query).await;
        for kind in ["device", "catalogue"] {
            assert_eq!(
                section_offer(&preview, kind)["selected"],
                false,
                "{chosen:?} {kind}: {preview}"
            );
            assert_eq!(
                section_offer(&preview, kind)["unavailable"],
                "the originating device source is unavailable",
                "{chosen:?} {kind}"
            );
            assert!(sections_of(&preview, kind).is_empty(), "{chosen:?} {kind}");
        }
    }
}

fn sections_of<'a>(preview: &'a Value, kind: &str) -> Vec<&'a Value> {
    preview["sections"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|section| section["kind"] == kind)
        .collect()
}

async fn preview_of(state: &std::sync::Arc<crate::AppState>, query: Value) -> Value {
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

#[tokio::test]
async fn a_system2_recording_without_its_journal_id_is_never_exported_raw() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = super::record(&state, OperationKind::System2Download, ("x", "1"), false);
    let preview = preview_of(
        &state,
        json!({"problemType":"deviceProgramming","operation":{"recordingId":id},"sections":["operation"]}),
    )
    .await;
    assert_eq!(section_offer(&preview, "operation")["selected"], false);
    assert!(section_offer(&preview, "operation")["unavailable"].is_string());
    assert!(sections_of(&preview, "operation").is_empty());
}

#[tokio::test]
async fn a_repeated_earlier_recording_is_read_and_exported_once() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let earlier = super::record(&state, OperationKind::Compare, ("x", "1"), true);
    let current = super::record(&state, OperationKind::DeviceRead, ("y", "2"), true);
    let preview = preview_of(
        &state,
        json!({"problemType":"deviceReading","operation":{"recordingId":current},
            "earlierRecordingIds":[earlier, earlier],"sections":["operation","earlierOperation"]}),
    )
    .await;
    let earlier_sections = sections_of(&preview, "earlierOperation");
    assert_eq!(earlier_sections.len(), 1, "{preview}");
    assert_eq!(earlier_sections[0]["label"], earlier.as_str());
}

#[tokio::test]
async fn the_catalogue_extract_never_carries_the_product_database_startup_reason() {
    const REASON: &str = "PRIVATE_DB_REASON=/home/alice/catalogue.sqlite";
    let dir = tempfile::tempdir().unwrap();
    let state = std::sync::Arc::new(crate::AppState {
        recordings: std::sync::Arc::new(crate::operation_recordings::store(dir.path(), false)),
        product_db_unavailable: Some(REASON.into()),
        ..crate::AppState::new(dir.path().into())
    });
    *state.project.lock().unwrap() = Some(project());
    let query = json!({"problemType":"catalogue","operation":{"selection":"none"},
        "deviceId":7,"sections":["catalogue"]});
    let preview = preview_of(&state, query.clone()).await;
    assert_eq!(
        section_offer(&preview, "catalogue")["unavailable"],
        Value::Null
    );
    let mut text_query = query;
    text_query["review"] = preview["review"].clone();
    text_query["key"] = json!("catalogue");
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(text_query),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let text = json_of(&body)["text"].as_str().unwrap().to_string();
    assert!(text.contains("\"availability\": \"unavailable\""), "{text}");
    assert!(!text.contains("PRIVATE_DB_REASON") && !text.contains("/home/alice"));
    assert!(!preview.to_string().contains("PRIVATE_DB_REASON"));
}

#[tokio::test]
async fn a_failed_named_source_never_projects_the_current_device() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), false);
    *state.project.lock().unwrap() = Some(project());
    let preview = preview_of(
        &state,
        json!({"problemType":"deviceReading",
            "operation":{"relatedKey":"diagnosticsToken","relatedValue":"missing-token"},
            "deviceId":7,"sections":["device","catalogue"]}),
    )
    .await;
    for kind in ["device", "catalogue"] {
        assert_eq!(section_offer(&preview, kind)["selected"], false, "{kind}");
        assert!(
            section_offer(&preview, kind)["unavailable"].is_string(),
            "{kind}"
        );
        assert!(sections_of(&preview, kind).is_empty(), "{kind}");
    }
}

#[tokio::test]
async fn without_any_source_only_the_chosen_device_is_projected() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), false);
    *state.project.lock().unwrap() = Some(project());
    let none = preview_of(
        &state,
        json!({"problemType":"deviceReading","operation":{"selection":"none"},
            "sections":["device","catalogue"]}),
    )
    .await;
    assert!(section_offer(&none, "device")["unavailable"].is_string());
    assert_eq!(
        section_offer(&none, "catalogue")["unavailable"],
        Value::Null
    );
    assert!(sections_of(&none, "device").is_empty());
    let mut text_query = json!({"problemType":"deviceReading","operation":{"selection":"none"},
        "sections":["device","catalogue"]});
    text_query["review"] = none["review"].clone();
    text_query["key"] = json!("catalogue");
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(text_query),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let text = json_of(&body)["text"].as_str().unwrap().to_string();
    assert!(text.contains("\"deviceProductIdentity\": null"), "{text}");
    let chosen = preview_of(
        &state,
        json!({"problemType":"deviceReading","operation":{"selection":"none"},
            "deviceId":7,"sections":["device","catalogue"]}),
    )
    .await;
    let device = sections_of(&chosen, "device");
    assert_eq!(device.len(), 1, "{chosen}");
    assert_eq!(device[0]["label"], "7");
}

#[test]
fn declared_memory_reports_carry_the_private_consent_note() {
    use knx_app::diagnostic_bundle::BundleFile;
    let files = [
        BundleFile::new("report.json", b"{}".to_vec()),
        BundleFile::new("memory.json", b"{}".to_vec()),
    ];
    let note = super::super::report_notes(&files).expect("memory note");
    assert!(note.contains("possibly passwords") && note.contains("keep private"));
    assert_eq!(super::super::report_notes(&files[..1]), None);
}
