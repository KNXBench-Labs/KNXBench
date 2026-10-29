//! HTTP regression tests binding review, section text and ZIP to the same collected bytes.

use super::tests::*;
use axum::http::StatusCode;
use knx_app::operation_recording::OperationKind;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{io::Read, sync::Arc};

async fn preview_query(state: &Arc<crate::AppState>, query: &Value) -> Value {
    let (status, body, _) = call(
        state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query.clone()),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&body));
    let view = json_of(&body);
    let fingerprint = view["review"]["fingerprint"]
        .as_str()
        .expect("mandatory fingerprint");
    assert_eq!(fingerprint.len(), 64);
    assert!(fingerprint
        .bytes()
        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)));
    assert_eq!(
        view["review"]["context"]["serverIncarnation"],
        state.server_incarnation
    );
    view
}

fn reviewed(query: &Value, preview: &Value) -> Value {
    let mut query = query.clone();
    // The fallback deliberately reaches the original collector, calibrating semantic drift.
    query["review"] = preview.get("review").cloned().unwrap_or(json!({}));
    query["consent"] = json!(true);
    query
}

async fn changed(state: &Arc<crate::AppState>, query: Value) {
    for uri in ["/api/diagnostics/bundle", "/api/diagnostics/bundle/section"] {
        let mut query = query.clone();
        query["key"] = json!("environment");
        let (status, body, disposition) = call(state, "POST", uri, Some(query)).await;
        assert_eq!(status, StatusCode::CONFLICT, "{uri}: {}", json_of(&body));
        assert!(String::from_utf8_lossy(&body).contains("bundlePreviewChanged"));
        assert_eq!(disposition, None);
        assert!(!body.starts_with(b"PK"));
    }
}

#[tokio::test]
async fn newer_matching_operation_cannot_replace_the_reviewed_run() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let first = record(&state, OperationKind::Download, ("downloadId", "1"), true);
    let query = json!({"problemType":"deviceProgramming"});
    let preview = preview_query(&state, &query).await;
    assert_eq!(preview["operationRecordingId"], first);
    record(&state, OperationKind::Download, ("downloadId", "2"), true);
    changed(&state, reviewed(&query, &preview)).await;
}

#[tokio::test]
async fn project_revision_and_replacement_invalidate_review_before_device_lookup() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let query = json!({"problemType":"deviceReading", "deviceId":1});
    let preview = preview_query(&state, &query).await;
    {
        let _project = state.project.lock().unwrap();
        crate::domain::next_project_revision(&state);
    }
    changed(&state, reviewed(&query, &preview)).await;
    let preview = preview_query(&state, &query).await;
    {
        let _project = state.project.lock().unwrap();
        state
            .project_incarnation
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
    changed(&state, reviewed(&query, &preview)).await;
}

#[tokio::test]
async fn mutable_selected_client_bytes_are_bound_but_final_description_is_not() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let query = json!({"problemType":"userInterface", "client":{"recentErrors":["reviewed"]}});
    let preview = preview_query(&state, &query).await;
    let mut altered = reviewed(&query, &preview);
    altered["client"]["recentErrors"] = json!(["unreviewed"]);
    changed(&state, altered).await;
    let mut query = reviewed(&query, &preview);
    query["description"] = json!("entered only after review");
    let (status, bytes, _) = call(&state, "POST", "/api/diagnostics/bundle", Some(query)).await;
    assert_eq!(status, StatusCode::OK, "{}", json_of(&bytes));
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    for section in preview["sections"].as_array().unwrap() {
        for file in section["files"].as_array().unwrap() {
            let mut bytes = Vec::new();
            let name = file.as_str().unwrap();
            let directory = match section["kind"].as_str().unwrap() {
                "environment" => "environment",
                "sessionLog" => "session-log",
                "clientState" => "client",
                _ => panic!("unexpected section"),
            };
            zip.by_name(&format!("{directory}/{name}"))
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            let mut text_query = reviewed(
                &json!({"problemType":"userInterface", "client":{"recentErrors":["reviewed"]}}),
                &preview,
            );
            text_query["key"] = section["key"].clone();
            let (status, text, _) = call(
                &state,
                "POST",
                "/api/diagnostics/bundle/section",
                Some(text_query),
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            let text = json_of(&text);
            let prefix = format!("===== {directory}/{name} =====\n");
            assert!(text["text"].as_str().unwrap().starts_with(&prefix));
            let rendered = text["text"]
                .as_str()
                .unwrap()
                .strip_prefix(&prefix)
                .unwrap()
                .trim_end_matches('\n');
            assert_eq!(
                format!("{:x}", Sha256::digest(&bytes)),
                format!("{:x}", Sha256::digest(rendered.as_bytes()))
            );
        }
    }
}

#[tokio::test]
async fn run_target_cannot_be_replaced_with_an_explicit_foreign_device() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let recorder = crate::operation_recordings::start(
        &state,
        knx_app::operation_recording::OperationStart::new(OperationKind::Download)
            .target(json!({"deviceId":1, "individualAddress":"1.1.1"})),
    );
    recorder.finish(knx_app::operation_recording::RecordedOutcome::succeeded(
        Value::Null,
    ));
    assert!(state
        .recordings
        .wait_until_idle(std::time::Duration::from_secs(10)));
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(json!({
            "problemType":"deviceProgramming", "deviceId":2,
            "operation":{"recordingId":recorder.id().unwrap()}
        })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{}", json_of(&body));
    assert!(String::from_utf8_lossy(&body).contains("bundleTargetMismatch"));
}

#[tokio::test]
async fn session_log_and_recording_availability_changes_require_a_fresh_review() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let query = json!({"problemType":"deviceReading"});
    let preview = preview_query(&state, &query).await;
    state
        .session_log
        .lock()
        .unwrap()
        .push(crate::session_log::LogEntry {
            timestamp: crate::session_log::now(),
            severity: crate::session_log::Severity::Error,
            source: "test".into(),
            message: "UNREVIEWED_LOG_SENTINEL".into(),
            location: None,
            detail: None,
            diagnostic: None,
        });
    changed(&state, reviewed(&query, &preview)).await;
    let id = record(&state, OperationKind::Compare, ("x", "1"), true);
    let query = json!({"problemType":"deviceReading","operation":{"recordingId":id}});
    let preview = preview_query(&state, &query).await;
    assert_eq!(state.recordings.delete_all().unwrap().removed, 1);
    changed(&state, reviewed(&query, &preview)).await;
}

#[tokio::test]
async fn selected_recording_deletion_is_bound_independently_of_the_session_log() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(&state, OperationKind::Compare, ("x", "1"), true);
    let query = json!({"problemType":"deviceReading","operation":{"recordingId":id}});
    let preview = preview_query(&state, &query).await;
    assert_eq!(state.recordings.delete_all().unwrap().removed, 1);
    changed(&state, reviewed(&query, &preview)).await;
}

#[tokio::test]
async fn effective_identity_matches_when_client_pins_the_automatically_resolved_recording() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(&state, OperationKind::Compare, ("x", "1"), true);
    let query = json!({"problemType":"deviceReading"});
    let preview = preview_query(&state, &query).await;
    let mut pinned = reviewed(&query, &preview);
    pinned["operation"] = json!({"recordingId":id});
    let (status, _, _) = call(&state, "POST", "/api/diagnostics/bundle", Some(pinned)).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn context_from_another_server_is_refused_on_initial_preview_and_export() {
    let dir = tempfile::tempdir().unwrap();
    let one = state(dir.path(), true);
    let other = state(dir.path(), true);
    let query = json!({"problemType":"userInterface", "sections":["environment"]});
    let preview = preview_query(&one, &query).await;
    changed(&other, reviewed(&query, &preview)).await;
    let mut query = query;
    query["context"] = preview["review"]["context"].clone();
    let (status, body, _) = call(
        &other,
        "POST",
        "/api/diagnostics/bundle/preview",
        Some(query),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert!(String::from_utf8_lossy(&body).contains("bundlePreviewChanged"));
}

#[tokio::test]
async fn explicit_none_never_selects_a_previous_run_and_client_can_be_selected_again() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let id = record(&state, OperationKind::Download, ("downloadId", "1"), true);
    let query = json!({"problemType":"userInterface", "operation":{"selection":"none"},"sections":["environment"]});
    let view = preview_query(&state, &query).await;
    assert_eq!(view["operationRecordingId"], Value::Null);
    let client = view["offers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["kind"] == "clientState")
        .unwrap();
    assert_eq!(client["unavailable"], Value::Null);
    assert_eq!(client["selected"], false);
    for reference in [
        json!({"selection":"none","recordingId":id}),
        json!({"selection":"latest"}),
    ] {
        let (status, body, _) = call(
            &state,
            "POST",
            "/api/diagnostics/bundle/preview",
            Some(json!({"problemType":"deviceProgramming","operation":reference})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(String::from_utf8_lossy(&body).contains("bundleOperationReferenceRefused"));
    }
}

#[tokio::test]
async fn bus_monitor_availability_changes_require_repreview() {
    monitor_changes(false).await;
}

#[tokio::test]
async fn bus_monitor_content_changes_require_repreview() {
    monitor_changes(true).await;
}

async fn monitor_changes(change_contents: bool) {
    use crate::bus::{
        fake::{FakeConnector, FakeTunnel},
        BusSession, GroupAddressContext,
    };
    use knx_net::{
        ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci, TunnelEvent,
    };
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let query = json!({"problemType":"deviceReading","sections":["environment","busMonitor"]});
    let absent = preview_query(&state, &query).await;
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
    if !change_contents {
        changed(&state, reviewed(&query, &absent)).await;
        let session = state.bus_session.lock().await.take().unwrap();
        session.stop().await;
        assert!(handle.disconnected());
        return;
    }
    let empty = preview_query(&state, &query).await;
    handle
        .sender()
        .send(TunnelEvent::Telegram(LDataFrame {
            kind: LDataMessageKind::Indication,
            source: "1.1.5".parse().unwrap(),
            destination: Destination::Group(knx_core::GroupAddress::from_raw(0x0a03)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueRead,
            control: None,
        }))
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if !buffer.lock().unwrap().telegrams_since(0).is_empty() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    changed(&state, reviewed(&query, &empty)).await;
    state.bus_session.lock().await.take().unwrap().stop().await;
    assert!(handle.disconnected());
}

#[tokio::test]
async fn a_context_change_during_collection_is_refused_before_any_owned_bytes_are_returned() {
    use axum::{extract::State, response::IntoResponse, Json};
    use std::{future::Future, task::Poll};
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let monitor = state.bus_session.lock().await;
    let query = serde_json::from_value(json!({"problemType":"deviceReading"})).unwrap();
    let mut pending = Box::pin(super::preview(State(state.clone()), Json(query)));
    // The real handler captures/validates context synchronously before its first await.
    // Holding monitor guarantees collection cannot finish in this first poll.
    let first = std::future::poll_fn(|cx| Poll::Ready(pending.as_mut().poll(cx))).await;
    assert!(first.is_pending());
    {
        let _project = state.project.lock().unwrap();
        crate::domain::next_project_revision(&state);
    }
    drop(monitor);
    let error = pending
        .await
        .err()
        .expect("context drift during collection must refuse");
    let response = error.into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let bytes = axum::body::to_bytes(response.into_body(), 4096)
        .await
        .unwrap();
    assert!(String::from_utf8_lossy(&bytes).contains("bundlePreviewChanged"));
}

#[tokio::test]
async fn section_requires_review_and_malformed_binding_fails_closed() {
    let dir = tempfile::tempdir().unwrap();
    let state = state(dir.path(), true);
    let (status, body, _) = call(
        &state,
        "POST",
        "/api/diagnostics/bundle/section",
        Some(json!({"problemType":"deviceReading", "key":"environment"})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(String::from_utf8_lossy(&body).contains("bundlePreviewRequired"));
    for review in [json!({}), json!({"fingerprint":"NOT-A-HASH", "context":{}})] {
        let (status, body, disposition) = call(
            &state,
            "POST",
            "/api/diagnostics/bundle",
            Some(json!({"problemType":"deviceReading", "consent":true,"review":review})),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(String::from_utf8_lossy(&body).contains("bundlePreviewRequired"));
        assert_eq!(disposition, None);
    }
}
