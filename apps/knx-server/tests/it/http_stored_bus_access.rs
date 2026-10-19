//! Stored bus access over HTTP: hidden by default, one record on request, no writes (Goal B3).
//!
//! Synthetic RFC 5737 witness only (`knx-testsupport`); no network.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_store::StoredOpaqueEntry;
use knx_testsupport::{retained_privacy_knxproj_bytes, RETAINED_PRIVACY_PLANTED};
use serde_json::Value;
use tower::ServiceExt;

fn state(open: bool) -> (tempfile::TempDir, Arc<knx_server::AppState>) {
    let dir = tempfile::tempdir().unwrap();
    let state = knx_server::AppState::new(dir.path().to_path_buf());
    if open {
        let outcome = knx_etsproj::import_knxproj_bytes(
            retained_privacy_knxproj_bytes(),
            "synthetic.knxproj",
        )
        .unwrap();
        *state.opaque.lock().unwrap() = outcome
            .opaque
            .iter()
            .map(|e| StoredOpaqueEntry {
                source_path: e.source_path.clone(),
                xpath: e.xpath.clone(),
                kind: format!("{:?}", e.kind),
                name: e.name.clone(),
                bytes: e.bytes.clone(),
                sha256: e.sha256.clone(),
            })
            .collect();
        *state.project.lock().unwrap() = Some(outcome.project);
    }
    (dir, Arc::new(state))
}

async fn call(
    state: &Arc<knx_server::AppState>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, String) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(body.map_or_else(Body::empty, |b| Body::from(b.to_string())))
        .unwrap();
    let response = knx_server::app(Arc::clone(state), None)
        .oneshot(request)
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, String::from_utf8(bytes.to_vec()).unwrap())
}

fn record<'a>(view: &'a Value, element: &str) -> &'a Value {
    view["records"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["element"] == element)
        .unwrap_or_else(|| panic!("no {element} record"))
}

fn leaks(text: &str) -> Vec<&'static str> {
    RETAINED_PRIVACY_PLANTED
        .iter()
        .copied()
        .filter(|p| text.contains(p))
        .collect()
}

#[tokio::test]
async fn the_list_hides_every_endpoint_and_changes_nothing() {
    let (_dir, state) = state(true);
    let project_before = state.project.lock().unwrap().clone();
    let opaque_before = state.opaque.lock().unwrap().clone();
    let (_, settings_before) = call(&state, "GET", "/api/settings", None).await;
    let (status, body) = call(&state, "GET", "/api/project/stored-bus-access", None).await;
    assert_eq!(status, StatusCode::OK);
    let view: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(view["records"].as_array().unwrap().len(), 2);
    assert_eq!(record(&view, "BusAccess")["owner"]["kind"], "line");
    assert_eq!(record(&view, "IPConfig")["owner"]["reason"], "notKeyed");
    assert_eq!(leaks(&body), Vec::<&str>::new());
    assert_eq!(*state.project.lock().unwrap(), project_before);
    assert_eq!(*state.opaque.lock().unwrap(), opaque_before);
    // A stored endpoint never becomes a gateway setting.
    assert_eq!(
        call(&state, "GET", "/api/settings", None).await.1,
        settings_before
    );
}

#[tokio::test]
async fn revealing_answers_that_one_record_and_logs_no_value() {
    let (_dir, state) = state(true);
    let (_, body) = call(&state, "GET", "/api/project/stored-bus-access", None).await;
    let view: Value = serde_json::from_str(&body).unwrap();
    let access_id = record(&view, "BusAccess")["recordId"]
        .as_str()
        .unwrap()
        .to_string();
    let (status, body) = call(
        &state,
        "POST",
        "/api/project/stored-bus-access/reveal",
        Some(serde_json::json!({ "recordId": access_id })),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let record: Value = serde_json::from_str(&body).unwrap();
    assert_eq!(record["recordId"], access_id.as_str());
    assert_eq!(record["revealed"], true);
    // Only this record's values: the BusAccess host and peer, not the
    // IPConfig address and MAC.
    assert_eq!(
        leaks(&body),
        vec![RETAINED_PRIVACY_PLANTED[0], RETAINED_PRIVACY_PLANTED[1]]
    );
    let (_, log) = call(&state, "GET", "/api/log", None).await;
    assert_eq!(leaks(&log), Vec::<&str>::new());
}

#[tokio::test]
async fn an_unknown_record_a_stray_field_and_no_project_are_refused() {
    let (_dir, state) = state(true);
    let (status, body) = call(
        &state,
        "POST",
        "/api/project/stored-bus-access/reveal",
        Some(serde_json::json!({ "recordId": "7:0000000000000000" })),
    )
    .await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(
        serde_json::from_str::<Value>(&body).unwrap()["kind"],
        "storedBusAccessChanged"
    );
    let (status, _) = call(
        &state,
        "POST",
        "/api/project/stored-bus-access/reveal",
        Some(serde_json::json!({ "recordId": "0:x", "connect": true })),
    )
    .await;
    assert!(status.is_client_error(), "{status}");
    let (_none, empty) = self::state(false);
    let (status, _) = call(&empty, "GET", "/api/project/stored-bus-access", None).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}
