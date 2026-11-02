//! A monitor start meets a held tunnel reservation with an instant `gatewayBusy`, never a queue.
//!
//! WP-26 backend prerequisite. A bus-connection control must hear "busy" at
//! once instead of connecting minutes later behind a long management or
//! System-2 run. Real mounted HTTP, a counting fake connector, no socket, no
//! gateway. Every held-lock request runs in its own task under a bound; a
//! start that queues is released, awaited and only then reported, so a failing
//! test never leaves a task hanging on a lock.

use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_app::property_validation::{LocalOutcome, System2Settlement};
use knx_net::DiscoveredGateway;
use knx_server::fake::{FakeConnector, FakeTunnel};
use knx_server::{BusSessionError, BusTunnel, GatewayConnector};
use serde_json::{json, Value};
use tower::ServiceExt;

const GATEWAY: &str = "192.0.2.10:3671";
const OPERATION: &str = "0123456789abcdef0123456789abcdef";
/// Far above an instant refusal; only a start queued behind a lock reaches it.
const BOUND: Duration = Duration::from_secs(5);

/// Lets the test read the call count after the state owns the connector.
struct CountingConnector(Arc<FakeConnector>);

impl GatewayConnector for CountingConnector {
    #[allow(clippy::type_complexity)]
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>>
                + Send
                + '_,
        >,
    > {
        self.0.connect_tunnel(gateway)
    }

    #[allow(clippy::type_complexity)]
    fn discover(
        &self,
    ) -> Pin<
        Box<
            dyn std::future::Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>>
                + Send
                + '_,
        >,
    > {
        self.0.discover()
    }
}

struct Fixture {
    state: Arc<knx_server::AppState>,
    app: axum::Router,
    connector: Arc<FakeConnector>,
}

fn fixture() -> Fixture {
    let tunnel = FakeTunnel::new("1.1.250".parse().unwrap(), 64).0;
    let connector = Arc::new(FakeConnector::succeeding(tunnel));
    let state = Arc::new(knx_server::AppState {
        connector: Box::new(CountingConnector(Arc::clone(&connector))),
        ..Default::default()
    });
    let app = knx_server::app(Arc::clone(&state), None);
    Fixture {
        state,
        app,
        connector,
    }
}

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

/// Sends a monitor start in its own task and waits at most [`BOUND`]. On a
/// miss the reservation in `held` is released and the task awaited before the
/// test fails, so nothing outlives it.
async fn start_bounded<H>(
    app: &axum::Router,
    gateway: &str,
    held: &mut Option<H>,
) -> (StatusCode, Value) {
    let request_app = app.clone();
    let gateway = gateway.to_owned();
    let mut task = tokio::spawn(async move {
        call(
            &request_app,
            "POST",
            "/api/bus/monitor/start",
            Some(json!({ "gateway": gateway })),
        )
        .await
    });
    match tokio::time::timeout(BOUND, &mut task).await {
        Ok(done) => done.expect("the start task panicked"),
        Err(_) => {
            held.take();
            let late = tokio::time::timeout(BOUND, &mut task).await;
            if late.is_err() {
                task.abort();
                let _ = task.await;
            }
            panic!("the start queued behind a held lock instead of answering gatewayBusy (late outcome after release: {late:?})");
        }
    }
}

fn connecting(session: &str) -> Value {
    json!({ "kind": "connecting", "session": session })
}

/// The four reservation locks a System-2 operation holds while it runs.
macro_rules! hold_all_locks {
    ($state:expr) => {
        Some((
            $state.device_download.lock().await,
            $state.address_programming.lock().await,
            $state.bus_session.lock().await,
            $state.line_scan_session.lock().await,
        ))
    };
}

/// Nothing leaked: the slot is empty and a genuine start and stop still work.
async fn assert_start_and_stop_still_work(f: &Fixture) {
    assert!(f.state.bus_session.try_lock().unwrap().is_none());
    assert_eq!(f.connector.call_count(), 0);
    let mut none: Option<()> = None;
    let (status, body) = start_bounded(&f.app, GATEWAY, &mut none).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(f.connector.call_count(), 1);
    let (status, body) = call(&f.app, "POST", "/api/bus/monitor/stop", None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(f.state.bus_session.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn a_start_behind_a_held_bus_session_lock_is_refused_at_once_without_connecting() {
    let f = fixture();
    let mut held = Some(f.state.bus_session.lock().await);
    let (status, body) = start_bounded(&f.app, GATEWAY, &mut held).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    // The holder of the lock is unknown: only the session kind is named.
    assert_eq!(
        body["holder"],
        connecting("busMonitorOrGroupWrite"),
        "{body}"
    );
    assert_eq!(f.connector.call_count(), 0);
    drop(held);
    assert_start_and_stop_still_work(&f).await;
}

#[tokio::test]
async fn a_start_names_a_download_while_its_bus_session_lock_is_held() {
    let f = fixture();
    let mut held = Some((
        f.state.device_download.lock().await,
        f.state.bus_session.lock().await,
    ));
    let (status, body) = start_bounded(&f.app, GATEWAY, &mut held).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(body["holder"], connecting("deviceDownload"), "{body}");
    assert_eq!(f.connector.call_count(), 0);
    drop(held);
    assert_start_and_stop_still_work(&f).await;
}

#[tokio::test]
async fn a_start_names_management_while_its_bus_session_lock_is_held() {
    let f = fixture();
    let mut held = Some((
        f.state.address_programming.lock().await,
        f.state.bus_session.lock().await,
    ));
    let (status, body) = start_bounded(&f.app, GATEWAY, &mut held).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(body["holder"], connecting("managementOperation"), "{body}");
    assert_eq!(f.connector.call_count(), 0);
    drop(held);
    assert_start_and_stop_still_work(&f).await;
}

#[tokio::test]
async fn a_start_behind_the_locks_of_a_running_system2_operation_names_that_operation() {
    let f = fixture();
    let lease = f.state.device_system2.control.try_admit(OPERATION).unwrap();
    let mut held = hold_all_locks!(f.state);
    let (status, body) = start_bounded(&f.app, GATEWAY, &mut held).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(body["holder"], connecting("system2Operation"), "{body}");
    assert_eq!(f.connector.call_count(), 0);
    drop(held);
    // The refusal neither stopped nor cleared the operation.
    assert!(matches!(
        f.state.device_system2.control.snapshot().settlement,
        System2Settlement::Running { .. }
    ));
    lease.settle(LocalOutcome::Failed).unwrap();
    assert_start_and_stop_still_work(&f).await;
}

/// System-2 admits its ticket only after it holds the locks, but a start must
/// not race the window either side of it: an admitted ticket alone refuses.
#[tokio::test]
async fn a_start_after_system2_admission_but_before_its_locks_are_held_is_refused_without_connecting(
) {
    let f = fixture();
    let lease = f.state.device_system2.control.try_admit(OPERATION).unwrap();
    let mut none: Option<()> = None;
    let (status, body) = start_bounded(&f.app, GATEWAY, &mut none).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(body["holder"], connecting("system2Operation"), "{body}");
    assert_eq!(f.connector.call_count(), 0);
    assert!(f.state.bus_session.try_lock().unwrap().is_none());
    lease.settle(LocalOutcome::Failed).unwrap();
    assert_start_and_stop_still_work(&f).await;
}

/// An abandoned System-2 ticket is Uncertain, not Running: a held lock then
/// has an unknown holder and must not be renamed after System-2.
#[tokio::test]
async fn an_uncertain_system2_control_does_not_rename_a_held_bus_session_lock() {
    let f = fixture();
    drop(f.state.device_system2.control.try_admit(OPERATION).unwrap());
    assert!(matches!(
        f.state.device_system2.control.snapshot().settlement,
        System2Settlement::Uncertain { .. }
    ));
    let mut held = Some(f.state.bus_session.lock().await);
    let (status, body) = start_bounded(&f.app, GATEWAY, &mut held).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(
        body["holder"],
        connecting("busMonitorOrGroupWrite"),
        "{body}"
    );
    assert_eq!(f.connector.call_count(), 0);
}

/// Malformed input and a stopping server still answer first, even when the
/// bus session lock is held; neither is rewritten into `gatewayBusy`.
#[tokio::test]
async fn a_malformed_gateway_and_a_stopping_server_answer_before_any_lock_is_looked_at() {
    let f = fixture();
    let mut held = Some(f.state.bus_session.lock().await);
    let (status, body) = start_bounded(&f.app, "not-a-gateway", &mut held).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_ne!(body["kind"], "gatewayBusy", "{body}");

    f.state.stop_admission.close();
    let (status, body) = start_bounded(&f.app, GATEWAY, &mut held).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["kind"], "serverStopping", "{body}");
    assert_eq!(f.connector.call_count(), 0);
}
