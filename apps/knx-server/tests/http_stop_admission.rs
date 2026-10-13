//! Once a stop has begun, no route opens a new tunnel (Goal A3, KL-163).
//!
//! The connector here panics if `connect_tunnel` is ever reached, so a route
//! that forgot the admission check fails loudly rather than reaching a
//! (fake) gateway. Request bodies are minimal but well-formed: the JSON and
//! query extractors run before the handler, and the refusal must come from
//! the handler's own first statement, not from a malformed body.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

/// A server whose only connector panics if reached. The service-control
/// debug setting is on, so those two routes get as far as their tunnel
/// reservation; with it off they answer 403 first, which opens no tunnel
/// either.
async fn stopping_state(dir: &std::path::Path) -> Arc<knx_server::AppState> {
    let state = Arc::new(knx_server::AppState {
        connector: Box::new(knx_server::fake::FakeConnector::discovering(Vec::new())),
        ..knx_server::AppState::new(dir.to_path_buf())
    });
    let (status, body) = call(
        &state,
        "PUT",
        "/api/settings",
        Some(json!({ "settings": { "debugIndividualAddressWriteEnable": true } })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    state.stop_admission.close();
    state
}

const GATEWAY: &str = "127.0.0.1:3671";

/// Every route that would open a tunnel, as (method, uri, body).
fn tunnel_routes() -> Vec<(&'static str, String, Option<Value>)> {
    vec![
        (
            "POST",
            "/api/device-download/start".into(),
            Some(json!({"planId": 1, "gateway": GATEWAY, "confirmation": "x"})),
        ),
        (
            "POST",
            "/api/device-address/start".into(),
            Some(json!({"address": "1.1.9", "gateway": GATEWAY, "confirmation": "x"})),
        ),
        (
            "POST",
            "/api/device-address/by-serial".into(),
            Some(
                json!({"address": "1.1.9", "gateway": GATEWAY, "confirmation": "x",
                        "serialNumber": "0083:00000001"}),
            ),
        ),
        (
            "GET",
            format!("/api/device-address/find-serial?gateway={GATEWAY}&serialNumber=0083:00000001"),
            None,
        ),
        (
            "POST",
            "/api/device-compare".into(),
            Some(json!({"address": "1.1.9", "gateway": GATEWAY})),
        ),
        (
            "GET",
            format!("/api/device/service-control?address=1.1.9&gateway={GATEWAY}"),
            None,
        ),
        (
            "POST",
            "/api/device/service-control".into(),
            Some(json!({
                "address": "1.1.9",
                "gateway": GATEWAY,
                "confirmation": "I confirm individual-address write enable to 1.1.9",
                "enable": true,
            })),
        ),
        (
            "POST",
            "/api/bus/monitor/start".into(),
            Some(json!({"gateway": GATEWAY})),
        ),
        (
            "POST",
            "/api/bus/scan/start".into(),
            Some(
                json!({"gateway": GATEWAY, "area": 1, "line": 1, "firstDevice": 1,
                        "lastDevice": 2, "responseTimeoutMs": 100, "interProbePauseMs": 0}),
            ),
        ),
    ]
}

async fn call(
    state: &Arc<knx_server::AppState>,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut request = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(body) => {
            request = request.header("content-type", "application/json");
            Body::from(body.to_string())
        }
        None => Body::empty(),
    };
    let response = knx_server::app(state.clone(), None)
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[tokio::test]
async fn every_tunnel_route_refuses_once_the_stop_has_begun() {
    let dir = tempfile::tempdir().unwrap();
    let state = stopping_state(dir.path()).await;
    for (method, uri, body) in tunnel_routes() {
        let (status, error) = call(&state, method, &uri, body).await;
        assert_eq!(
            status,
            StatusCode::SERVICE_UNAVAILABLE,
            "{method} {uri}: {error}"
        );
        assert_eq!(error["kind"], "serverStopping", "{method} {uri}: {error}");
    }
    assert!(state.device_download.lock().await.is_none());
    assert!(state.address_programming.lock().await.is_none());
    assert!(state.bus_session.lock().await.is_none());
    assert!(state.line_scan_session.lock().await.is_none());
}
