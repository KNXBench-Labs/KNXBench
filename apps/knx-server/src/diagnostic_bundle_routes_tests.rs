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
    assert_eq!(
        keys,
        ["environment", "sessionLog", &format!("operation:{compare}")]
    );
    // Device and legacy reports are preselected but unavailable here, and say why.
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
async fn the_exported_zip_carries_the_recording_without_any_key() {
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
    let mut recording = String::new();
    archive
        .by_name(&format!("operations/{id}/recording.jsonl"))
        .unwrap()
        .read_to_string(&mut recording)
        .unwrap();
    assert!(recording.contains("\"redacted\":{\"kind\":\"authorize-key\""));
    assert!(!recording.to_ascii_lowercase().contains("c0ffee42"));
    let mut manifest = String::new();
    archive
        .by_name("manifest.json")
        .unwrap()
        .read_to_string(&mut manifest)
        .unwrap();
    let manifest: Value = serde_json::from_str(&manifest).unwrap();
    assert_eq!(manifest["description"], "it stopped at the end");
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
        .any(|s| s["key"] == "clientState"));
}

#[tokio::test]
async fn section_text_shows_one_section_and_unknown_ids_are_refused() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(&state, OperationKind::Compare, ("x", "1"), true);
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
        Some(json!({ "problemType": "deviceReading", "key": format!("operation:{id}"), "review": json_of(&preview)["review"] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert!(json_of(&body)["text"]
        .as_str()
        .unwrap()
        .contains("\"type\":\"header\""));
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
