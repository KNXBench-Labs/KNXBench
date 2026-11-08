//! Session-log, bus-monitor and client-state sections: preview, text, ZIP and withheld values.

use std::io::Read;
use std::sync::Arc;

use axum::http::StatusCode;
use serde_json::{json, Value};

use super::{call, json_of, state};
use crate::session_log::{LogEntry, SettingsDiagnostic, SettingsQuarantineReasonDto, Severity};

/// Values that must never leave in any form: password, BCU key, System-2 readback, names.
const FORBIDDEN: [&str; 12] = [
    "password123",
    "c0ffee42",
    "c0, ff, ee, 42",
    "0xc0",
    "5a5ac3a5",
    "5a 5a c3 a5",
    "5a, 5a, c3, a5",
    "s2readback",
    "alice",
    "bcukeyfield",
    "tunnelsecret",
    "/knx/",
];

/// Everything one preview → section texts → consented ZIP exposes, for scanning.
struct Flow {
    preview: Value,
    texts: Vec<(String, String)>,
    zip: Vec<(String, Vec<u8>)>,
    exposed: String,
}

impl Flow {
    fn text(&self, key: &str) -> &str {
        &self
            .texts
            .iter()
            .find(|(k, _)| k == key)
            .unwrap_or_else(|| panic!("no section text {key}"))
            .1
    }

    fn entry(&self, name: &str) -> Value {
        let bytes = &self
            .zip
            .iter()
            .find(|(n, _)| n == name)
            .unwrap_or_else(|| panic!("no ZIP entry {name}"))
            .1;
        serde_json::from_slice(bytes).unwrap()
    }

    fn has_entry(&self, name: &str) -> bool {
        self.zip.iter().any(|(n, _)| n == name)
    }

    fn offer(&self, kind: &str) -> Value {
        self.preview["offers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["kind"] == kind)
            .cloned()
            .unwrap_or_else(|| panic!("no offer {kind}"))
    }

    fn section(&self, key: &str) -> Value {
        self.preview["sections"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["key"] == key)
            .cloned()
            .unwrap_or_else(|| panic!("no section {key}"))
    }

    fn assert_clean(&self) {
        let haystack = self.exposed.to_lowercase();
        for forbidden in FORBIDDEN {
            assert!(
                !haystack.contains(forbidden),
                "{forbidden} leaked into an exported surface"
            );
        }
    }
}

fn reviewed(query: &Value, preview: &Value) -> Value {
    let mut query = query.clone();
    query["review"] = preview["review"].clone();
    if query.get("context").is_none() {
        query["context"] = preview["review"]["context"].clone();
    }
    query
}

async fn preview(state: &Arc<crate::AppState>, query: &Value) -> Value {
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

async fn flow(state: &Arc<crate::AppState>, query: Value) -> Flow {
    flow_between(state, query.clone(), query).await
}

/// Previews with `preview_query`, then asks for texts and ZIP with `review_query`, as the UI does.
async fn flow_between(
    state: &Arc<crate::AppState>,
    preview_query: Value,
    review_query: Value,
) -> Flow {
    let query = review_query;
    let preview = preview(state, &preview_query).await;
    let mut exposed = preview.to_string();
    let mut texts = Vec::new();
    for section in preview["sections"].as_array().unwrap() {
        let key = section["key"].as_str().unwrap().to_string();
        let mut request = reviewed(&query, &preview);
        request["key"] = json!(key);
        let (status, body, _) = call(
            state,
            "POST",
            "/api/diagnostics/bundle/section",
            Some(request),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
        let text = json_of(&body)["text"].as_str().unwrap().to_string();
        exposed.push_str(&text);
        texts.push((key, text));
    }
    let mut request = reviewed(&query, &preview);
    request["consent"] = json!(true);
    let (status, body, disposition) =
        call(state, "POST", "/api/diagnostics/bundle", Some(request)).await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    exposed.push_str(&disposition.unwrap_or_default());
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    let mut zip = Vec::new();
    for index in 0..archive.len() {
        let mut file = archive.by_index(index).unwrap();
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes).unwrap();
        exposed.push_str(file.name().unwrap().as_ref());
        exposed.push_str(&String::from_utf8_lossy(&bytes));
        zip.push((file.name().unwrap().into_owned(), bytes));
    }
    Flow {
        preview,
        texts,
        zip,
        exposed,
    }
}

/// The ZIP entry, its manifest digest and the section text carry the same bytes.
fn assert_bound(flow: &Flow, key: &str, path: &str) {
    use sha2::{Digest, Sha256};
    let bytes = &flow.zip.iter().find(|(n, _)| n == path).unwrap().1;
    let manifest = flow.entry("manifest.json");
    let listed = manifest["sections"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["files"].as_array().unwrap().iter())
        .find(|f| f["path"] == path)
        .unwrap_or_else(|| panic!("{path} not in manifest: {manifest}"))
        .clone();
    assert_eq!(listed["sha256"], format!("{:x}", Sha256::digest(bytes)));
    assert!(flow
        .text(key)
        .contains(std::str::from_utf8(bytes).unwrap().trim_end()));
    assert_eq!(flow.section(key)["bytes"], bytes.len() as u64);
}

fn entry(severity: Severity, source: &str, message: &str) -> LogEntry {
    LogEntry {
        timestamp: "2026-11-01T09:00:00.123+00:00".into(),
        severity,
        source: source.into(),
        message: message.into(),
        location: None,
        detail: None,
        diagnostic: None,
    }
}

fn seed_log(state: &crate::AppState) {
    let mut log = state.session_log.lock().unwrap();
    log.push(entry(
        Severity::Warning,
        "new",
        "refused to create a new project: the open one has unsaved edits",
    ));
    log.push(entry(
        Severity::Info,
        "open",
        "opened /home/alice/S2READBACK/Password123.knxdb",
    ));
    log.push(LogEntry {
        location: Some("/KNX/Project[@Password='Password123']".into()),
        detail: Some("BCU key c0ffee42, tunnelsecret".into()),
        ..entry(
            Severity::Error,
            "SetParameterValue",
            "value Password123 rejected: readback 5a 5a c3 a5",
        )
    });
    log.push(entry(Severity::Info, "Password123 c0ffee42", "saved"));
    log.push(LogEntry {
        diagnostic: Some(SettingsDiagnostic::Quarantined {
            reason: SettingsQuarantineReasonDto::InvalidJson,
            moved_to: "/home/alice/Password123.json".into(),
        }),
        ..entry(Severity::Warning, "settings", "settings file quarantined")
    });
}

#[tokio::test]
async fn session_log_leaves_as_typed_entries_with_disclosed_withholding() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    seed_log(&state);
    let flow = flow(&state, json!({"problemType": "deviceProgramming"})).await;
    let offer = flow.offer("sessionLog");
    assert_eq!(offer["unavailable"], Value::Null, "{offer}");
    assert_eq!(offer["selected"], true);
    assert_bound(&flow, "sessionLog", "session-log/session-log.json");
    let body = flow.entry("session-log/session-log.json");
    let entries = body["entries"].as_array().unwrap();
    assert_eq!(body["entriesTotal"], 5);
    assert_eq!(entries.len(), 5);
    assert_eq!(entries[0]["severity"], "warning");
    assert_eq!(entries[0]["source"], "new");
    assert_eq!(
        entries[0]["message"],
        "refused to create a new project: the open one has unsaved edits"
    );
    assert_eq!(entries[0]["timestamp"], "2026-11-01T09:00:00.123Z");
    assert_eq!(entries[1]["message"], "opened [withheld]");
    assert_eq!(entries[2]["severity"], "error");
    assert_eq!(entries[2]["source"], "SetParameterValue");
    assert_eq!(entries[2]["message"], Value::Null);
    assert_eq!(entries[3]["source"], Value::Null);
    assert_eq!(entries[3]["message"], "saved");
    assert_eq!(entries[4]["diagnostic"]["kind"], "quarantined");
    assert_eq!(entries[4]["diagnostic"]["reason"], "invalidJson");
    assert_eq!(body["withheld"]["message"], 2);
    assert_eq!(body["withheld"]["source"], 1);
    assert_eq!(body["withheld"]["location"], 1);
    assert_eq!(body["withheld"]["detail"], 1);
    assert_eq!(body["withheld"]["path"], 2);
    let notes = flow.section("sessionLog")["notes"].clone();
    assert!(notes.to_string().contains("withheld"), "{notes}");
    assert!(!notes
        .to_string()
        .contains(knx_app::diagnostic_bundle::PRIVATE_RAW_NOTE));
    flow.assert_clean();
    // The live log is untouched by preview, text and export.
    let log = state.session_log.lock().unwrap();
    assert_eq!(log.entries().len(), 5);
    assert_eq!(
        log.entries()[2].detail.as_deref(),
        Some("BCU key c0ffee42, tunnelsecret")
    );
}

#[tokio::test]
async fn a_log_change_after_review_refuses_text_and_zip() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    seed_log(&state);
    let query = json!({"problemType": "deviceReading"});
    let reviewed_preview = preview(&state, &query).await;
    state
        .session_log
        .lock()
        .unwrap()
        .push(entry(Severity::Info, "save", "saved"));
    let mut request = reviewed(&query, &reviewed_preview);
    request["key"] = json!("sessionLog");
    let (status, _, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(request),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    let mut request = reviewed(&query, &reviewed_preview);
    request["consent"] = json!(true);
    let (status, body, disposition) =
        call(&state, "POST", "/api/diagnostics/bundle", Some(request)).await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(String::from_utf8_lossy(&body).contains("bundlePreviewChanged"));
    assert_eq!(disposition, None);
}

/// The browser's wire pair: an automatic-selector preview, then reviewed requests with explicit none.
fn automatic_and_none(sections: &[&str]) -> (Value, Value) {
    let automatic = json!({"problemType": "userInterface", "sections": sections,
                           "client": client_facts()});
    let mut none = automatic.clone();
    none["operation"] = json!({"selection": "none"});
    (automatic, none)
}

#[tokio::test]
async fn automatic_selector_review_without_recording_serves_explicit_none_texts_and_zip() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    seed_log(&state);
    let (automatic, none) = automatic_and_none(&["environment", "sessionLog", "clientState"]);
    let flow = flow_between(&state, automatic, none).await;
    assert_eq!(flow.preview["operationRecordingId"], Value::Null);
    assert_bound(&flow, "sessionLog", "session-log/session-log.json");
    assert_bound(&flow, "clientState", "client/client.json");
    flow.assert_clean();
}

#[tokio::test]
async fn automatic_selector_review_refuses_changed_bytes_and_a_tampered_review_under_none() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    seed_log(&state);
    let (automatic, none) = automatic_and_none(&["environment", "sessionLog", "clientState"]);
    let reviewed_preview = preview(&state, &automatic).await;
    let refused = |request: Value, path: &'static str| {
        let state = state.clone();
        async move {
            let (status, body, _) = call(&state, "POST", path, Some(request)).await;
            assert_eq!(status, StatusCode::CONFLICT, "{}", json_of(&body));
            assert!(String::from_utf8_lossy(&body).contains("bundlePreviewChanged"));
        }
    };
    let section = "/api/diagnostics/bundle/section";
    let mut baseline = reviewed(&none, &reviewed_preview);
    baseline["key"] = json!("sessionLog");
    let (status, body, _) = call(&state, "POST", section, Some(baseline)).await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let mut tampered = reviewed(&none, &reviewed_preview);
    tampered["key"] = json!("sessionLog");
    tampered["review"]["fingerprint"] = json!("0".repeat(64));
    refused(tampered, section).await;

    let mut changed_client = reviewed(&none, &reviewed_preview);
    changed_client["key"] = json!("clientState");
    changed_client["client"]["theme"] = json!("porcelain");
    refused(changed_client, section).await;

    state
        .session_log
        .lock()
        .unwrap()
        .push(entry(Severity::Info, "save", "saved"));
    let mut changed_log = reviewed(&none, &reviewed_preview);
    changed_log["key"] = json!("sessionLog");
    refused(changed_log, section).await;
    let mut export = reviewed(&none, &reviewed_preview);
    export["consent"] = json!(true);
    refused(export, "/api/diagnostics/bundle").await;
}

#[tokio::test]
async fn explicit_none_never_adopts_a_recording_the_automatic_review_did_not_bind() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let (automatic, none) =
        automatic_and_none(&["environment", "sessionLog", "clientState", "operation"]);
    // Review taken without any recording; a recording appears afterwards.
    let early = preview(&state, &automatic).await;
    super::record(
        &state,
        knx_app::operation_recording::OperationKind::Compare,
        ("role", "late"),
        true,
    );
    let mut request = reviewed(&none, &early);
    request["key"] = json!("sessionLog");
    let (status, _, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(request),
    )
    .await;
    // `none` still binds to the none-state the review described, not to the new recording.
    assert_eq!(status, StatusCode::OK);
    let mut export = reviewed(&none, &early);
    export["consent"] = json!(true);
    let (status, body, _) = call(&state, "POST", "/api/diagnostics/bundle", Some(export)).await;
    assert_eq!(status, StatusCode::OK);
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
    let names: Vec<_> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().unwrap().into_owned())
        .collect();
    assert!(
        !names.iter().any(|n| n.starts_with("operations/")),
        "recording attached: {names:?}"
    );
    // An automatic review taken WITH the recording must not serve explicit none.
    let with_recording = preview(&state, &automatic).await;
    assert_ne!(with_recording["operationRecordingId"], Value::Null);
    let mut request = reviewed(&none, &with_recording);
    request["key"] = json!("sessionLog");
    let (status, _, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(request),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
}

fn client_facts() -> Value {
    json!({
        "appVersion": "0.1.0-alpha.1",
        "shell": "browser",
        "uiLanguage": "de",
        "theme": "graphite",
        "viewport": {"width": 400, "height": 900, "devicePassword123": 1},
        "uiScale": 1.25,
        "truncated": false,
        "metadataTruncated": false,
        "omittedErrorCount": 0,
        "truncatedErrorCount": 0,
        "recentErrors": [
            {"timestamp": "2026-11-01T10:00:00Z", "source": "save", "message": "saved",
             "detail": "Password123"},
            {"timestamp": "2026-11-01T10:00:01Z", "source": "Password123",
             "message": "readback 5a5ac3a5 Password123", "detail": null},
        ],
        "bcuKeyField": "c0ffee42",
        "S2READBACK": 1,
    })
}

#[tokio::test]
async fn client_state_leaves_only_the_closed_schema() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let flow = flow(
        &state,
        json!({"problemType": "userInterface", "client": client_facts()}),
    )
    .await;
    let offer = flow.offer("clientState");
    assert_eq!(offer["unavailable"], Value::Null, "{offer}");
    assert_eq!(offer["selected"], true);
    assert_bound(&flow, "clientState", "client/client.json");
    let body = flow.entry("client/client.json");
    assert_eq!(body["appVersion"], "0.1.0-alpha.1");
    assert_eq!(body["shell"], "browser");
    assert_eq!(body["uiLanguage"], "de");
    assert_eq!(body["theme"], "graphite");
    assert_eq!(body["viewport"], json!({"width": 400, "height": 900}));
    assert_eq!(body["uiScale"], 1.25);
    let errors = body["recentErrors"].as_array().unwrap();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[0]["source"], "save");
    assert_eq!(errors[0]["message"], "saved");
    assert_eq!(errors[1]["source"], Value::Null);
    assert_eq!(errors[1]["message"], Value::Null);
    assert_eq!(body["withheld"]["unknownFields"], 3);
    assert_eq!(body["withheld"]["detail"], 1);
    assert_eq!(body["withheld"]["message"], 1);
    assert_eq!(body["withheld"]["source"], 1);
    flow.assert_clean();
}

#[tokio::test]
async fn client_state_absent_deselected_unknown_theme_and_size_limit() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let absent = preview(&state, &json!({"problemType": "userInterface"})).await;
    let offer = absent["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|o| o["kind"] == "clientState")
        .unwrap()
        .clone();
    assert_eq!(offer["unavailable"], "the browser sent no client state");
    assert_eq!(offer["selected"], false);

    // Deselected: the client facts reach no section, text or ZIP.
    let flow_deselected = flow(
        &state,
        json!({"problemType": "userInterface", "sections": ["environment", "sessionLog"],
               "client": client_facts()}),
    )
    .await;
    assert!(!flow_deselected.has_entry("client/client.json"));
    assert_eq!(flow_deselected.offer("clientState")["selected"], false);
    assert!(!flow_deselected.exposed.contains("graphite"));
    flow_deselected.assert_clean();

    let mut custom = client_facts();
    custom["theme"] = json!("my-Password123-pack");
    custom["uiLanguage"] = json!("xx-Password123");
    let flow_custom = flow(
        &state,
        json!({"problemType": "userInterface", "client": custom}),
    )
    .await;
    let body = flow_custom.entry("client/client.json");
    assert_eq!(body["theme"], "other");
    assert_eq!(body["uiLanguage"], "other");
    flow_custom.assert_clean();

    // Exactly 64 KiB is accepted, one octet more refused, as before.
    let filler = |len: usize| {
        let mut client = json!({"theme": "graphite", "pad": ""});
        let base = client.to_string().len();
        client["pad"] = json!("x".repeat(len - base));
        client
    };
    let exact = filler(64 * 1024);
    assert_eq!(exact.to_string().len(), 64 * 1024);
    let at_limit = flow(
        &state,
        json!({"problemType": "userInterface", "client": exact}),
    )
    .await;
    let body = at_limit.entry("client/client.json");
    assert_eq!(body["withheld"]["unknownFields"], 1);
    assert!(!at_limit.exposed.contains("xxxxxxxx"));
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({"problemType": "userInterface", "client": filler(64 * 1024 + 1)})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("bundleClientStateRefused"));
    for invalid in [json!("text"), json!([1, 2])] {
        let (status, _, _) = call(
            &state,
            "POST",
            "/api/diagnostics/bundle/preview",
            Some(json!({"problemType": "userInterface", "client": invalid})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
    }
}

mod monitor {
    use super::*;
    use crate::bus::{
        fake::{FakeConnector, FakeTunnel, FakeTunnelHandle},
        BusSession, GroupAddressContext, TelegramBuffer,
    };
    use knx_core::GroupValue;
    use knx_net::{
        ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci, TunnelEvent,
    };

    pub(super) async fn start(
        state: &crate::AppState,
    ) -> (FakeTunnelHandle, Arc<std::sync::Mutex<TelegramBuffer>>) {
        let (tunnel, handle) = FakeTunnel::new("1.1.2".parse().unwrap(), 16);
        let connector = FakeConnector::succeeding(tunnel);
        let session = BusSession::start(
            1,
            "203.0.113.250:3671".parse().unwrap(),
            &connector,
            GroupAddressContext::from_project(None),
        )
        .await
        .unwrap();
        let buffer = session.buffer();
        *state.bus_session.lock().await = Some(session);
        (handle, buffer)
    }

    pub(super) fn group(service: ApplicationService) -> TunnelEvent {
        TunnelEvent::Telegram(LDataFrame {
            kind: LDataMessageKind::Indication,
            source: "1.1.5".parse().unwrap(),
            destination: Destination::Group(knx_core::GroupAddress::from_raw(0x0a03)),
            transport: Tpci::UnnumberedData,
            service,
            control: None,
        })
    }

    pub(super) async fn wait_rows(buffer: &Arc<std::sync::Mutex<TelegramBuffer>>, rows: usize) {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while buffer.lock().unwrap().telegrams_since(0).len() < rows {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    const MONITOR_FILE: &str = "bus-monitor/bus-monitor.json";

    /// The reviewed export of `preview`: status and the monitor file, if a ZIP came back.
    async fn export_monitor(
        state: &Arc<crate::AppState>,
        query: &Value,
        preview: &Value,
    ) -> (StatusCode, Option<Vec<u8>>) {
        let mut request = reviewed(query, preview);
        request["consent"] = json!(true);
        export_request(state, request).await
    }

    async fn export_request(
        state: &Arc<crate::AppState>,
        request: Value,
    ) -> (StatusCode, Option<Vec<u8>>) {
        let (status, body, disposition) =
            call(state, "POST", "/api/diagnostics/bundle", Some(request)).await;
        if status != StatusCode::OK {
            assert_eq!(disposition, None);
            assert!(!body.starts_with(b"PK"));
            return (status, None);
        }
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(body)).unwrap();
        let mut bytes = Vec::new();
        archive
            .by_name(MONITOR_FILE)
            .unwrap()
            .read_to_end(&mut bytes)
            .unwrap();
        (status, Some(bytes))
    }

    async fn monitor_text(state: &Arc<crate::AppState>, query: &Value, preview: &Value) -> Value {
        let mut request = reviewed(query, preview);
        request["key"] = json!("busMonitor");
        let (status, body, _) = call(
            state,
            "POST",
            "/api/diagnostics/bundle/section",
            Some(request),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
        json_of(&body)["text"].clone()
    }

    async fn send_rows(
        handle: &FakeTunnelHandle,
        buffer: &Arc<std::sync::Mutex<TelegramBuffer>>,
        count: usize,
    ) {
        let start = buffer.lock().unwrap().next_seq() as usize;
        for index in 0..count {
            handle
                .sender()
                .send(group(ApplicationService::GroupValueRead))
                .unwrap();
            // The broadcast channel holds 16; never let it lag.
            if index % 8 == 7 {
                wait_seq(buffer, start + index + 1).await;
            }
        }
        wait_seq(buffer, start + count).await;
    }

    async fn wait_seq(buffer: &Arc<std::sync::Mutex<TelegramBuffer>>, next: usize) {
        tokio::time::timeout(std::time::Duration::from_secs(20), async {
            while (buffer.lock().unwrap().next_seq() as usize) < next {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn new_traffic_after_review_keeps_text_and_zip_identical() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        let (handle, buffer) = start(&state).await;
        send_rows(&handle, &buffer, 2).await;
        let preview = preview(&state, &QUERY()).await;
        let binding = preview["review"]["monitor"].clone();
        assert_eq!(binding["sessionId"], 1, "{binding}");
        assert_eq!(binding["firstSeq"], 0);
        assert_eq!(binding["endSeq"], 2);
        let text = monitor_text(&state, &QUERY(), &preview).await;
        send_rows(&handle, &buffer, 3).await;
        assert_eq!(monitor_text(&state, &QUERY(), &preview).await, text);
        let (status, bytes) = export_monitor(&state, &QUERY(), &preview).await;
        assert_eq!(status, StatusCode::OK);
        let bytes = bytes.unwrap();
        assert!(text
            .as_str()
            .unwrap()
            .contains(std::str::from_utf8(&bytes).unwrap().trim_end()));
        let body: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body["telegrams"].as_array().unwrap().len(), 2);
        assert_eq!(body["firstSeq"], 0);
        assert_eq!(body["endSeq"], 2);
        // A gateway close keeps the pinned rows readable and the reviewed bytes.
        handle.sender().send(TunnelEvent::Closed).unwrap();
        wait_seq(&buffer, 6).await;
        let (status, closed) = export_monitor(&state, &QUERY(), &preview).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(closed.unwrap(), bytes);
        state.bus_session.lock().await.take().unwrap().stop().await;
    }

    /// Starts the monitor session over a fake of the given transport and reads it back.
    async fn start_over(
        state: &crate::AppState,
        transport: knx_net::TunnelTransport,
    ) -> (FakeTunnelHandle, Arc<std::sync::Mutex<TelegramBuffer>>) {
        let (tunnel, handle) = FakeTunnel::new("1.1.2".parse().unwrap(), 16);
        let tunnel = tunnel.with_transport(transport);
        let connector = FakeConnector::succeeding(tunnel).accepting_tcp();
        let gateway = "203.0.113.250:3671".parse().unwrap();
        let ctx = GroupAddressContext::from_project(None);
        let session = match transport {
            knx_net::TunnelTransport::Tcp => {
                BusSession::start_tcp(1, gateway, &connector, ctx).await
            }
            knx_net::TunnelTransport::Udp => BusSession::start(1, gateway, &connector, ctx).await,
        }
        .unwrap();
        let buffer = session.buffer();
        *state.bus_session.lock().await = Some(session);
        (handle, buffer)
    }

    #[tokio::test]
    async fn the_exported_monitor_names_the_actual_transport_and_skipped_tcp_frames() {
        for (transport, skipped) in [
            (knx_net::TunnelTransport::Tcp, Some(2)),
            (knx_net::TunnelTransport::Udp, None),
        ] {
            let dir = tempfile::tempdir().unwrap();
            let state = state(dir.path(), true);
            let (handle, buffer) = start_over(&state, transport).await;
            handle.set_skipped_frames(2);
            send_rows(&handle, &buffer, 1).await;
            let request_preview = preview;
            let preview = request_preview(&state, &QUERY()).await;
            let text = monitor_text(&state, &QUERY(), &preview).await;
            let (status, bytes) = export_monitor(&state, &QUERY(), &preview).await;
            assert_eq!(status, StatusCode::OK);
            let bytes = bytes.unwrap();
            let body: Value = serde_json::from_slice(&bytes).unwrap();
            assert_eq!(body["transport"], transport.as_str());
            assert_eq!(
                body.get("skippedFramesAtReview"),
                skipped.map(Value::from).as_ref()
            );
            // The reviewed text and the ZIP carry the same metadata, and nothing else new leaks.
            let text = text.as_str().unwrap();
            assert!(text.contains(std::str::from_utf8(&bytes).unwrap().trim_end()));
            for forbidden in FORBIDDEN {
                assert!(!text.to_lowercase().contains(forbidden), "{forbidden}");
            }
            // Later skips must not change the reviewed text or ZIP; only a fresh review sees them.
            if skipped.is_some() {
                handle.set_skipped_frames(5);
                assert_eq!(
                    monitor_text(&state, &QUERY(), &preview)
                        .await
                        .as_str()
                        .unwrap(),
                    text
                );
                let (status, again) = export_monitor(&state, &QUERY(), &preview).await;
                assert_eq!(status, StatusCode::OK);
                assert_eq!(again.unwrap(), bytes);
                let fresh = request_preview(&state, &QUERY()).await;
                let (status, updated) = export_monitor(&state, &QUERY(), &fresh).await;
                assert_eq!(status, StatusCode::OK);
                let updated: Value = serde_json::from_slice(&updated.unwrap()).unwrap();
                assert_eq!(updated["skippedFramesAtReview"], 5);
                assert_eq!(updated["telegrams"], body["telegrams"]);
            }
            state.bus_session.lock().await.take().unwrap().stop().await;
        }
    }

    #[tokio::test]
    async fn an_empty_snapshot_stays_empty_and_a_missing_session_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        let (handle, buffer) = start(&state).await;
        let preview = preview(&state, &QUERY()).await;
        assert_eq!(preview["review"]["monitor"]["firstSeq"], 0);
        assert_eq!(preview["review"]["monitor"]["endSeq"], 0);
        send_rows(&handle, &buffer, 2).await;
        let (status, bytes) = export_monitor(&state, &QUERY(), &preview).await;
        assert_eq!(status, StatusCode::OK);
        let body: Value = serde_json::from_slice(&bytes.unwrap()).unwrap();
        assert_eq!(body["telegrams"], json!([]));
        state.bus_session.lock().await.take().unwrap().stop().await;
        let (status, _) = export_monitor(&state, &QUERY(), &preview).await;
        assert_eq!(status, StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn a_replaced_session_or_an_evicted_pinned_row_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        let (handle, buffer) = start(&state).await;
        send_rows(&handle, &buffer, 2).await;
        let preview = preview(&state, &QUERY()).await;
        // Same seqs, other session: refused.
        state.bus_session.lock().await.take().unwrap().stop().await;
        let (tunnel, other) = FakeTunnel::new("1.1.2".parse().unwrap(), 16);
        let session = BusSession::start(
            2,
            "203.0.113.250:3671".parse().unwrap(),
            &FakeConnector::succeeding(tunnel),
            GroupAddressContext::from_project(None),
        )
        .await
        .unwrap();
        let replaced = session.buffer();
        *state.bus_session.lock().await = Some(session);
        send_rows(&other, &replaced, 2).await;
        let (status, _) = export_monitor(&state, &QUERY(), &preview).await;
        assert_eq!(status, StatusCode::CONFLICT);
        // A pinned row pushed out by the 5000-row cap: refused.
        let preview = super::preview(&state, &QUERY()).await;
        assert_eq!(preview["review"]["monitor"]["sessionId"], 2);
        send_rows(&other, &replaced, crate::bus::MAX_TELEGRAMS).await;
        let (status, _) = export_monitor(&state, &QUERY(), &preview).await;
        assert_eq!(status, StatusCode::CONFLICT);
        drop(handle);
        state.bus_session.lock().await.take().unwrap().stop().await;
    }

    #[tokio::test]
    async fn a_tampered_dropped_or_unasked_monitor_binding_refuses() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        let (handle, buffer) = start(&state).await;
        send_rows(&handle, &buffer, 3).await;
        let preview = preview(&state, &QUERY()).await;
        let base = {
            let mut request = reviewed(&QUERY(), &preview);
            request["consent"] = json!(true);
            request
        };
        for (field, value) in [
            ("firstSeq", json!(1)),
            ("endSeq", json!(2)),
            ("sessionId", json!(9)),
            ("droppedBefore", json!(7)),
            ("statusAtReview", json!("closed")),
            ("capturedAt", json!("2026-11-01T00:00:00.000Z")),
        ] {
            let mut request = base.clone();
            request["review"]["monitor"][field] = value;
            assert_eq!(
                export_request(&state, request).await.0,
                StatusCode::CONFLICT,
                "{field}"
            );
        }
        for (field, value) in [
            ("unknown", json!(1)),
            ("firstSeq", json!(1.5)),
            ("firstSeq", json!(-1)),
            ("firstSeq", json!(4)),
            ("endSeq", json!(9_007_199_254_740_993u64)),
            ("endSeq", json!(6000)),
            ("capturedAt", json!("yesterday")),
            ("statusAtReview", json!("gone")),
        ] {
            let mut request = base.clone();
            request["review"]["monitor"][field] = value;
            assert_eq!(
                export_request(&state, request).await.0,
                StatusCode::BAD_REQUEST,
                "{field}"
            );
        }
        let mut dropped = base.clone();
        dropped["review"].as_object_mut().unwrap().remove("monitor");
        assert_eq!(
            export_request(&state, dropped).await.0,
            StatusCode::CONFLICT
        );
        // A binding for a section not selected is a mismatch, not ignored.
        let unselected = json!({"problemType": "deviceReading", "sections": ["environment"]});
        let plain = super::preview(&state, &unselected).await;
        assert_eq!(plain["review"].get("monitor"), None);
        let mut request = reviewed(&unselected, &plain);
        request["consent"] = json!(true);
        request["review"]["monitor"] = preview["review"]["monitor"].clone();
        let (status, body, _) =
            call(&state, "POST", "/api/diagnostics/bundle", Some(request)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{}", json_of(&body));
        state.bus_session.lock().await.take().unwrap().stop().await;
    }

    const QUERY: fn() -> Value =
        || json!({"problemType": "deviceReading", "sections": ["environment", "busMonitor"]});

    #[tokio::test]
    async fn absent_monitor_is_unavailable_and_an_empty_one_is_delivered() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        let absent = preview(&state, &QUERY()).await;
        let offer = absent["offers"]
            .as_array()
            .unwrap()
            .iter()
            .find(|o| o["kind"] == "busMonitor")
            .unwrap()
            .clone();
        assert_eq!(
            offer["unavailable"],
            "the bus monitor is not running in this server session"
        );
        let (handle, _) = start(&state).await;
        let flow = flow(&state, QUERY()).await;
        assert_eq!(flow.offer("busMonitor")["unavailable"], Value::Null);
        assert_bound(&flow, "busMonitor", "bus-monitor/bus-monitor.json");
        let body = flow.entry("bus-monitor/bus-monitor.json");
        assert_eq!(body["statusAtReview"], "active");
        assert_eq!(body["telegrams"], json!([]));
        assert_eq!(body["windowSeconds"], 600);
        assert_eq!(flow.section("busMonitor")["entries"], 0);
        // Reading the buffer sent nothing on the bus.
        assert!(handle.sent_frames().is_empty());
        state.bus_session.lock().await.take().unwrap().stop().await;
    }

    #[tokio::test]
    async fn monitor_rows_keep_safe_fields_and_withhold_management_payloads() {
        let dir = tempfile::tempdir().unwrap();
        let state = state(dir.path(), true);
        let (handle, buffer) = start(&state).await;
        let sender = handle.sender();
        for service in [
            ApplicationService::GroupValueWrite(GroupValue::Short(1)),
            ApplicationService::GroupValueWrite(GroupValue::Bytes(b"Password123\0\0\0".to_vec())),
            ApplicationService::GroupValueResponse(GroupValue::Bytes(vec![
                0x5a, 0x5a, 0xc3, 0xa5, 0x11,
            ])),
            ApplicationService::GroupValueRead,
            ApplicationService::MemoryResponse {
                address: 0x0116,
                data: vec![0x5a, 0x5a, 0xc3, 0xa5],
            },
            ApplicationService::AuthorizeRequest {
                key: [0xc0, 0xff, 0xee, 0x42],
            },
        ] {
            sender.send(group(service)).unwrap();
        }
        wait_rows(&buffer, 6).await;
        let flow = flow(&state, QUERY()).await;
        let body = flow.entry("bus-monitor/bus-monitor.json");
        let rows = body["telegrams"].as_array().unwrap();
        assert_eq!(rows.len(), 6, "{body}");
        assert_eq!(rows[0]["service"], "GroupValueWrite");
        assert_eq!(rows[0]["source"], "1.1.5");
        // No project: the value is unresolved, so even one octet is not classified.
        assert_eq!(rows[0]["payload"], Value::Null);
        assert_eq!(rows[1]["payload"], Value::Null);
        assert_eq!(rows[2]["payload"], Value::Null);
        assert_eq!(rows[3]["service"], "GroupValueRead");
        assert_eq!(rows[4]["service"], "MemoryResponse");
        assert_eq!(rows[4]["payload"], Value::Null);
        assert_eq!(rows[5]["service"], "AuthorizeRequest");
        assert_eq!(rows[5]["payload"], Value::Null);
        assert_eq!(body["withheld"]["payload"], 5);
        assert_eq!(flow.section("busMonitor")["entries"], 6);
        flow.assert_clean();
        // The live buffer still holds every row unchanged.
        assert_eq!(buffer.lock().unwrap().telegrams_since(0).len(), 6);

        // New traffic after review does not change the reviewed window.
        let reviewed_preview = flow.preview.clone();
        sender
            .send(group(ApplicationService::GroupValueRead))
            .unwrap();
        wait_rows(&buffer, 7).await;
        let (status, after) = export_monitor(&state, &QUERY(), &reviewed_preview).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            after.as_deref(),
            Some(
                flow.zip
                    .iter()
                    .find(|(n, _)| n == MONITOR_FILE)
                    .unwrap()
                    .1
                    .as_slice()
            )
        );

        // An ended session keeps its buffer and is still delivered, marker included.
        sender.send(TunnelEvent::Closed).unwrap();
        wait_rows(&buffer, 8).await;
        let ended = super::flow(&state, QUERY()).await;
        let body = ended.entry("bus-monitor/bus-monitor.json");
        assert_eq!(body["statusAtReview"], "closed");
        let rows = body["telegrams"].as_array().unwrap();
        assert_eq!(rows.last().unwrap()["service"], "SessionClosed");
        assert_eq!(rows.last().unwrap()["payload"], Value::Null);
        ended.assert_clean();
        assert!(handle.sent_frames().is_empty());
        state.bus_session.lock().await.take().unwrap().stop().await;
    }
}
