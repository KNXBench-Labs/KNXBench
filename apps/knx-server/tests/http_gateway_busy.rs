//! Issue #10: a gateway start refused over HTTP names its holder or the reason.
//!
//! A start refused because another KNXBench session holds the gateway's one
//! tunnel names that holder in a structured `409` (`kind: "gatewayBusy"`,
//! `holder: { kind, … }`), next to the unchanged `error` text; a gateway that
//! refuses or does not answer is named in a structured `502`. No corpus, no
//! hardware: a fake connector and an injected address-programming session.

use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tower::ServiceExt;

use knx_core::IndividualAddress;
use knx_net::commissioning::programming_button_wait::{
    AddressProgrammingAuthorisation, ButtonWait,
};
use knx_net::SessionTiming;
use knx_server::fake::{FakeConnector, FakeTunnel};

const GATEWAY: &str = "192.0.2.10:3671";

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    body: Option<Value>,
) -> (StatusCode, Value) {
    let mut builder = Request::builder().method(method).uri(uri);
    let body = match body {
        Some(v) => {
            builder = builder.header("content-type", "application/json");
            Body::from(v.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(builder.body(body).unwrap())
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

fn fake_tunnel() -> FakeTunnel {
    FakeTunnel::new("1.1.250".parse().unwrap(), 64).0
}

fn timing() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(50),
        response_timeout: Duration::from_millis(50),
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(40),
        programming_delay: Duration::from_millis(0),
        restart_basic_t1: Duration::from_millis(1),
        restart_responsive_again: Duration::from_millis(5),
        ..SessionTiming::default()
    }
}

#[tokio::test]
async fn a_second_monitor_start_names_the_running_monitor_and_its_gateway() {
    let state = knx_server::AppState {
        connector: Box::new(FakeConnector::succeeding(fake_tunnel())),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);
    let (status, body) = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let id = body["sessionId"].clone();

    let (status, body) = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": "203.0.113.5:3671" })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(
        body["holder"],
        json!({ "kind": "busMonitor", "sessionId": id, "gateway": GATEWAY, "state": "active" }),
        "{body}"
    );
    // The old text stays for older clients and the CLI.
    assert!(
        body["error"].as_str().unwrap().contains("already active"),
        "{body}"
    );

    // The activity snapshot names the same gateway, so a client can offer it.
    let (status, activity) = call(&app, "GET", "/api/bus/activity", None).await;
    assert_eq!(status, StatusCode::OK, "{activity}");
    let monitor = activity["sessions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["kind"] == "busMonitor")
        .expect("monitor listed")
        .clone();
    assert_eq!(monitor["gateway"], GATEWAY, "{activity}");
}

#[tokio::test]
async fn a_waiting_address_programming_is_named_to_a_scan_and_a_monitor_start() {
    let state = Arc::new(knx_server::AppState {
        connector: Box::new(FakeConnector::succeeding(fake_tunnel())),
        ..Default::default()
    });
    let address: IndividualAddress = "1.1.30".parse().unwrap();
    let authorisation = AddressProgrammingAuthorisation::for_hardware(
        address,
        "I confirm individual-address programming to 1.1.30",
    )
    .unwrap();
    // Injected, like the corpus suite does: the public start route is
    // fail-closed without a verified backup.
    *state.address_programming.lock().await = Some(knx_server::AddressProgrammingSession::start(
        7,
        Box::new(fake_tunnel()),
        address,
        authorisation,
        timing(),
        ButtonWait {
            give_up_after: Duration::from_secs(30),
            pause_between_rounds: Duration::from_millis(10),
        },
        Box::new(|_| {}),
    ));
    let app = knx_server::app(Arc::clone(&state), None);
    let expected = json!({ "kind": "addressProgramming", "address": "1.1.30", "state": "waiting" });

    let (status, body) = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(body["holder"], expected, "{body}");

    let scan = json!({ "gateway": GATEWAY, "area": 1, "line": 1, "firstDevice": 1, "lastDevice": 2,
        "responseTimeoutMs": 100, "interProbePauseMs": 0 });
    let (status, body) = call(&app, "POST", "/api/bus/scan/start", Some(scan)).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(body["holder"], expected, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("one tunnel"),
        "{body}"
    );
}

async fn start_with_failing_connector(
    error: knx_net::BusError,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let state = knx_server::AppState {
        connector: Box::new(FakeConnector::failing(
            knx_server::BusSessionError::Transport(error),
        )),
        ..Default::default()
    };
    let app = knx_server::app(Arc::new(state), None);
    call(&app, "POST", uri, Some(body)).await
}

#[tokio::test]
async fn a_gateway_with_no_free_connection_is_named_with_its_status_on_monitor_and_scan_starts() {
    let scan = json!({ "gateway": GATEWAY, "area": 1, "line": 1, "firstDevice": 1, "lastDevice": 2,
        "responseTimeoutMs": 100, "interProbePauseMs": 0 });
    for (uri, body) in [
        ("/api/bus/monitor/start", json!({ "gateway": GATEWAY })),
        ("/api/bus/scan/start", scan),
    ] {
        let (status, body) =
            start_with_failing_connector(knx_net::BusError::ConnectionRefused(0x24), uri, body)
                .await;
        assert_eq!(status, StatusCode::BAD_GATEWAY, "{uri}: {body}");
        assert_eq!(body["kind"], "gatewayRefused", "{uri}: {body}");
        assert_eq!(body["gateway"], GATEWAY, "{uri}: {body}");
        assert_eq!(body["gatewayStatus"], 0x24, "{uri}: {body}");
        assert_eq!(
            body["gatewayError"], "E_NO_MORE_CONNECTIONS",
            "{uri}: {body}"
        );
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("all of its connections are in use"),
            "{uri}: {body}"
        );
    }
}

#[tokio::test]
async fn an_undefined_refusal_code_keeps_its_number_and_no_guessed_name() {
    let (status, body) = start_with_failing_connector(
        knx_net::BusError::ConnectionRefused(0x77),
        "/api/bus/monitor/start",
        json!({ "gateway": GATEWAY }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(body["kind"], "gatewayRefused", "{body}");
    assert_eq!(body["gatewayStatus"], 0x77, "{body}");
    assert_eq!(body["gatewayError"], Value::Null, "{body}");
}

#[tokio::test]
async fn a_silent_gateway_is_a_named_timeout_with_its_address() {
    let (status, body) = start_with_failing_connector(
        knx_net::BusError::Timeout,
        "/api/bus/monitor/start",
        json!({ "gateway": GATEWAY }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert_eq!(body["kind"], "gatewayTimeout", "{body}");
    assert_eq!(body["gateway"], GATEWAY, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("timed out"),
        "{body}"
    );
}
