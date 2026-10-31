//! Route tests for the diagnostic bundle: consent, defaults, references, deletion and secrets.

use std::io::Read;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_app::operation_recording::{OperationKind, OperationStart, RecordedOutcome};
use knx_net::wire_tap::WireDirection;
use serde_json::{json, Value};
use tower::ServiceExt;

#[path = "diagnostic_bundle_import_tests.rs"]
mod import_tests;

#[path = "diagnostic_bundle_project_tests.rs"]
mod project_tests;

#[path = "diagnostic_bundle_routes_tests/r4_review_binding.rs"]
mod r4_review_binding;

#[path = "diagnostic_bundle_routes_tests/admitted_consumers.rs"]
mod admitted_consumers;

#[path = "diagnostic_bundle_routes_tests/retained_admission.rs"]
mod retained_admission;

const SENTINEL_KEY: [u8; 4] = [0xC0, 0xFF, 0xEE, 0x42];

pub(super) fn state(dir: &std::path::Path, enabled: bool) -> Arc<crate::AppState> {
    Arc::new(crate::AppState {
        recordings: Arc::new(crate::operation_recordings::store(dir, !enabled)),
        data_dir: dir.to_path_buf(),
        ..Default::default()
    })
}

pub(super) async fn call(
    state: &Arc<crate::AppState>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Vec<u8>, Option<String>) {
    let app = crate::app(Arc::clone(state), None);
    let mut request = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(value) => {
            request = request.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    let response = app.oneshot(request.body(body).unwrap()).await.unwrap();
    let status = response.status();
    let disposition = response
        .headers()
        .get("content-disposition")
        .map(|v| v.to_str().unwrap().to_string());
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap()
        .to_vec();
    (status, bytes, disposition)
}

pub(super) fn json_of(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes)
        .unwrap_or_else(|_| Value::String(String::from_utf8_lossy(bytes).into()))
}

/// An authorize request whose key is `SENTINEL_KEY`, as a tunnel datagram.
fn authorize_datagram() -> Vec<u8> {
    use knx_net::cemi::{
        self, ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci,
    };
    let frame = LDataFrame {
        kind: LDataMessageKind::Request,
        source: knx_core::IndividualAddress::from_raw(0),
        destination: Destination::Individual(knx_core::IndividualAddress::from_raw(0x1105)),
        transport: Tpci::NumberedData { seq: 0 },
        service: ApplicationService::AuthorizeRequest { key: SENTINEL_KEY },
        control: None,
    };
    knx_net::frame::encode_frame(
        knx_net::tunnelling::TUNNELLING_REQUEST,
        &knx_net::tunnelling::encode_tunnelling_request(
            1,
            0,
            &cemi::encode_l_data(&frame).unwrap(),
        ),
    )
}

pub(super) fn record(
    state: &crate::AppState,
    kind: OperationKind,
    related: (&str, &str),
    ok: bool,
) -> String {
    let recorder = crate::operation_recordings::start(
        state,
        OperationStart::new(kind)
            .target(json!({ "individualAddress": "1.1.5" }))
            .gateway("203.0.113.250:3671")
            .related(related.0, related.1),
    );
    recorder
        .wire_tap()
        .unwrap()
        .datagram(WireDirection::Sent, &authorize_datagram());
    recorder.finish(if ok {
        RecordedOutcome::succeeded(Value::Null)
    } else {
        RecordedOutcome::failed(vec!["boom".into()], Value::Null)
    });
    assert!(state.recordings.wait_until_idle(Duration::from_secs(10)));
    // Distinct, ordered ids: the id carries milliseconds.
    std::thread::sleep(Duration::from_millis(3));
    recorder.id().unwrap().to_string()
}

#[tokio::test]
async fn export_needs_consent_and_writes_nothing_without_it() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    record(&state, OperationKind::Download, ("downloadId", "1"), false);
    let (status, body, disposition) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle",
        Some(json!({ "problemType": "deviceProgramming" })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("bundleConsentRequired"));
    assert_eq!(disposition, None);
}

#[tokio::test]
async fn preview_preselects_the_latest_fitting_operation_and_the_o1_defaults() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let compare = record(&state, OperationKind::Compare, ("x", "1"), true);
    let download = record(&state, OperationKind::Download, ("downloadId", "4"), false);
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({ "problemType": "deviceReading" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let view = json_of(&body);
    // Reading picks the compare, not the newer download.
    assert_eq!(view["operationRecordingId"], compare.as_str());
    let keys: Vec<&str> = view["sections"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["key"].as_str().unwrap())
        .collect();
    // Owner decision 2026-10-31: the redacted raw recording leaves after consent.
    assert_eq!(keys, ["environment", &format!("operation:{compare}")]);
    // Unadmitted sources remain visible as fixed unavailable offers.
    let offer = |kind: &str| {
        view["offers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["kind"] == kind)
            .cloned()
            .unwrap()
    };
    assert_eq!(offer("device")["offer"], "preselected");
    assert!(offer("device")["unavailable"].is_string());
    assert_eq!(offer("sessionLog")["selected"], false);
    assert_eq!(
        offer("sessionLog")["unavailable"],
        "this section is withheld until its content is admitted as free of credentials"
    );
    assert_eq!(offer("project")["offer"], "offered");
    assert_eq!(offer("project")["selected"], false);
    assert!(view["offers"]
        .as_array()
        .unwrap()
        .iter()
        .all(|o| o["kind"] != "importFile" && o["kind"] != "clientState"));
    assert_eq!(view["recordingEnabled"], true);
    assert!(view["mailDraft"]
        .as_str()
        .unwrap()
        .starts_with("mailto:issue@knxbench.com?subject="));
    assert_eq!(view["recordings"].as_array().unwrap().len(), 2);

    // A shortcut names the download by its id and this server run.
    let (_, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({
            "problemType": "deviceProgramming",
            "operation": { "relatedKey": "downloadId", "relatedValue": "4" },
        })),
    )
    .await;
    assert_eq!(json_of(&body)["operationRecordingId"], download.as_str());
}

#[tokio::test]
async fn a_section_the_problem_type_does_not_offer_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({ "problemType": "projectImport", "sections": ["operation"] })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("bundleSectionNotOffered"));
}

#[tokio::test]
async fn the_exported_zip_withholds_the_unadmitted_recording_and_description() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(
        &state,
        OperationKind::System2Download,
        ("operationId", "00"),
        false,
    );
    let (_, preview, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({"problemType":"deviceProgramming", "operation":{"recordingId":id}})),
    )
    .await;
    let (status, body, disposition) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle",
        Some(json!({
            "review": json_of(&preview)["review"],
            "problemType": "deviceProgramming",
            "operation": { "recordingId": id },
            "consent": true,
            "description": "it stopped at the end",
        })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", String::from_utf8_lossy(&body));
    assert!(disposition.unwrap().contains(&id));
    assert!(!body.windows(4).any(|w| w == SENTINEL_KEY));
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    assert!(archive
        .file_names()
        .all(|name| !name.ends_with("recording.jsonl")));
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
    assert_eq!(manifest["writer"]["version"], crate::version_string());
    for index in 0..archive.len() {
        let mut bytes = Vec::new();
        archive
            .by_index(index)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        assert!(!bytes.windows(4).any(|w| w == SENTINEL_KEY));
        assert!(!String::from_utf8_lossy(&bytes)
            .to_ascii_lowercase()
            .contains("c0ffee42"));
    }
}

#[tokio::test]
async fn a_disabled_recorder_is_said_plainly_and_old_recordings_stay_deletable() {
    let dir = tempfile::tempdir().unwrap();
    let recording = state(dir.path(), true);
    record(&recording, OperationKind::Compare, ("x", "1"), true);
    let disabled = state(dir.path(), false);
    let (_, body, _) = call(&disabled, "GET", "/api/diagnostics/recordings", None).await;
    let view = json_of(&body);
    assert_eq!(view["enabled"], false);
    assert_eq!(view["recordings"].as_array().unwrap().len(), 1);
    let (status, body, _) = call(&disabled, "DELETE", "/api/diagnostics/recordings", None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(json_of(&body)["removed"], 1);
    let (_, body, _) = call(&disabled, "GET", "/api/diagnostics/recordings", None).await;
    assert!(json_of(&body)["recordings"].as_array().unwrap().is_empty());
    let (_, body, _) = call(
        &disabled,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({ "problemType": "userInterface", "client": { "uiLanguage": "de" } })),
    )
    .await;
    let view = json_of(&body);
    assert_eq!(view["recordingEnabled"], false);
    assert!(view["sections"]
        .as_array()
        .unwrap()
        .iter()
        .all(|s| s["key"] != "clientState"));
    let client = view["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|offer| offer["kind"] == "clientState")
        .unwrap();
    assert_eq!(client["selected"], false);
    assert_eq!(
        client["unavailable"],
        "this section is withheld until its content is admitted as free of credentials"
    );
}

#[tokio::test]
async fn section_text_shows_one_section_and_unknown_ids_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    record(&state, OperationKind::Compare, ("x", "1"), true);
    let (_, preview, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({"problemType":"deviceReading"})),
    )
    .await;
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(json!({ "problemType": "deviceReading", "key": "environment", "review": json_of(&preview)["review"] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(json_of(&body)["text"]
        .as_str()
        .unwrap()
        .contains("environment/environment.json"));
    let (status, _, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({
            "problemType": "deviceReading",
            "operation": { "recordingId": "../../etc/passwd" },
        })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn consented_export_requires_a_review_before_returning_any_zip() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let (status, body, disposition) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle",
        Some(json!({"problemType": "deviceProgramming", "consent": true})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{}", json_of(&body));
    assert!(String::from_utf8_lossy(&body).contains("bundlePreviewRequired"));
    assert_eq!(disposition, None);
}

#[test]
fn the_diagnostics_directory_is_reserved() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("diagnostics")).unwrap();
    assert!(crate::paths::resolve_in_data_dir(dir.path(), "diagnostics").is_err());
}

/// The web translates the server's fixed reasons and notes by exact text
/// (`reportProblem.ts`); a reworded or new server text must not silently
/// fall back to untranslated English in another UI language.
#[test]
fn every_fixed_reason_and_note_has_a_web_translation() {
    let web = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../knx-web/src/reportProblem.ts"
    ))
    .unwrap();
    let quoted = |text: &str| format!("[\"{}\", \"reportProblem.", text.replace('"', "\\\""));
    for code in super::BundleUnavailableCode::all() {
        let message = code.message();
        assert!(
            web.contains(&quoted(message)),
            "untranslated reason: {message}"
        );
    }
    for note in super::FIXED_NOTES {
        assert!(web.contains(&quoted(note)), "untranslated note: {note}");
    }
}

/// The offer of `kind` in a fresh preview of `query`; the preview must succeed.
async fn reason_of(state: &Arc<crate::AppState>, query: Value, kind: &str) -> Value {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let view = json_of(&body);
    let offer = view["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == kind)
        .cloned()
        .unwrap();
    assert_eq!(offer["selected"], false, "{offer}");
    offer["unavailable"].clone()
}

#[tokio::test]
async fn metadata_only_reasons_name_exactly_what_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let programming = json!({ "problemType": "deviceProgramming" });
    // No recording at all.
    for kind in ["operation", "legacyReports"] {
        assert_eq!(
            reason_of(&state, programming.clone(), kind).await,
            "no recorded operation fits this problem",
            "{kind}"
        );
    }
    // A fitting recording that fails admission is indistinguishable from none.
    let diagnostics = dir
        .path()
        .join(knx_app::operation_recording::DIAGNOSTICS_DIR_NAME);
    std::fs::create_dir_all(&diagnostics).unwrap();
    std::fs::write(
        diagnostics.join("op-20261031T120000000Z-0000abcd.jsonl"),
        b"{\"format_version\":99,\"operation_kind\":\"download\"}\n",
    )
    .unwrap();
    assert_eq!(
        reason_of(&state, programming.clone(), "operation").await,
        "no recorded operation fits this problem"
    );
    // Explicitly none.
    assert_eq!(
        reason_of(
            &state,
            json!({ "problemType": "deviceProgramming", "operation": {"selection": "none"} }),
            "legacyReports"
        )
        .await,
        "no operation is chosen"
    );
    // An admitted recording without a retained typed result.
    record(&state, OperationKind::Compare, ("x", "1"), true);
    assert_eq!(
        reason_of(
            &state,
            json!({ "problemType": "deviceReading" }),
            "legacyReports"
        )
        .await,
        "no minimised report exists for this operation"
    );
    // Import problems: no project, then a project without stored import evidence.
    let import = json!({ "problemType": "projectImport" });
    assert_eq!(
        reason_of(&state, import.clone(), "importReport").await,
        "no project is open"
    );
    *state.project.lock().unwrap() = Some(knx_core::Project::new(knx_core::Language("en".into())));
    assert_eq!(
        reason_of(&state, import, "importReport").await,
        "this project carries no import report"
    );
}
