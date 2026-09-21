//! Incremental line-scan HTTP lifecycle tests over the in-process fake tunnel only.

use std::sync::Arc;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use knx_net::Destination;
use knx_server::fake::{FakeConnector, FakeTunnel, FakeTunnelHandle};

fn addr(device: u8) -> knx_core::IndividualAddress {
    knx_core::IndividualAddress::new(1, 1, device).expect("synthetic test address is valid")
}

fn fake_tunnel() -> (FakeTunnel, FakeTunnelHandle) {
    FakeTunnel::new(addr(9), 64)
}

fn state_with(tunnel: FakeTunnel) -> knx_server::AppState {
    knx_server::AppState {
        connector: Box::new(FakeConnector::succeeding(tunnel)),
        ..Default::default()
    }
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> axum::response::Response {
    let mut request = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(value) => {
            request = request.header("content-type", "application/json");
            Body::from(value.to_string())
        }
        None => Body::empty(),
    };
    app.clone()
        .oneshot(request.body(body).unwrap())
        .await
        .unwrap()
}

async fn body_json(response: axum::response::Response) -> Value {
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

fn scan_request(first: u8, last: u8, excluded: &[&str], timeout_ms: u64) -> Value {
    json!({
        "gateway": "192.0.2.10:3671",
        "area": 1,
        "line": 1,
        "firstDevice": first,
        "lastDevice": last,
        "excluded": excluded,
        "responseTimeoutMs": timeout_ms,
        "interProbePauseMs": 0
    })
}

async fn poll_until_terminal(app: &axum::Router) -> Value {
    for _ in 0..200 {
        let response = call(app, "GET", "/api/bus/scan/results?since=0", None).await;
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        if body["status"] != "running" {
            return body;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    panic!("scan did not become terminal");
}

#[tokio::test]
async fn estimate_discloses_candidate_count_omissions_and_worst_case_basis() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let response = call(
        &app,
        "POST",
        "/api/bus/scan/estimate",
        Some(scan_request(2, 3, &["1.1.2"], 10)),
    )
    .await;

    assert_eq!(response.status(), StatusCode::OK);
    let body = body_json(response).await;
    assert_eq!(body["candidateCount"], 1);
    assert_eq!(body["omittedAddresses"], json!(["1.1.2"]));
    assert_eq!(body["responseTimeoutMs"], 10);
    assert_eq!(body["vacantConfirmations"], 1);
    assert_eq!(body["interProbePauseMs"], 0);
    assert_eq!(body["worstCaseMs"], 10);
}

#[tokio::test]
async fn estimate_rejects_device_zero_as_the_line_coupler_address() {
    let app = knx_server::app(Arc::new(knx_server::AppState::default()), None);
    let response = call(
        &app,
        "POST",
        "/api/bus/scan/estimate",
        Some(scan_request(0, 3, &[], 10)),
    )
    .await;

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn a_scan_completes_and_streams_the_self_address_as_its_own_outcome() {
    let (tunnel, handle) = fake_tunnel();
    let app = knx_server::app(Arc::new(state_with(tunnel)), None);

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(9, 9, &[], 10)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);

    let body = poll_until_terminal(&app).await;
    assert_eq!(body["status"], "completed");
    assert_eq!(body["completedCount"], 1);
    assert_eq!(body["totalCount"], 1);
    assert_eq!(body["results"][0]["address"], "1.1.9");
    assert_eq!(body["results"][0]["outcome"]["kind"], "selfAddress");
    assert!(handle.sent_frames().is_empty());
    assert!(handle.disconnected());
}

#[tokio::test]
async fn an_excluded_address_never_reaches_the_transport() {
    let (tunnel, handle) = fake_tunnel();
    let app = knx_server::app(Arc::new(state_with(tunnel)), None);

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 3, &["1.1.2"], 1)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);
    let body = poll_until_terminal(&app).await;
    assert_eq!(body["status"], "completed");

    let destinations: Vec<_> = handle
        .sent_frames()
        .into_iter()
        .map(|(destination, _, _)| destination)
        .collect();
    assert!(!destinations.contains(&Destination::Individual(addr(2))));
    assert!(destinations.contains(&Destination::Individual(addr(3))));
}

#[tokio::test]
async fn cancellation_stops_the_probe_and_disconnects_the_tunnel() {
    let (tunnel, handle) = fake_tunnel();
    let app = knx_server::app(Arc::new(state_with(tunnel)), None);

    let start = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(2, 3, &[], 60_000)),
    )
    .await;
    assert_eq!(start.status(), StatusCode::OK);

    for _ in 0..200 {
        if !handle.sent_frames().is_empty() {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    assert!(!handle.sent_frames().is_empty(), "probe never started");

    let cancel = call(&app, "POST", "/api/bus/scan/cancel", None).await;
    assert_eq!(cancel.status(), StatusCode::OK);
    assert_eq!(body_json(cancel).await["status"], "cancelled");
    let sent_after_cancel = handle.sent_frames().len();
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    assert_eq!(handle.sent_frames().len(), sent_after_cancel);
    assert!(handle.disconnected());
}
