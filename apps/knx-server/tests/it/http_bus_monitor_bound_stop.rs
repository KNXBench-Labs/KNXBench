//! The bound monitor stop releases only the named session, at once, and refuses everything else.
//!
//! WP-26. `POST /api/bus/monitor/stop-bound?sessionId=…&serverIncarnation=…`
//! is a separate route: the legacy `/stop` ignores a query string, so an older
//! server would silently stop whatever session it holds. Real mounted HTTP, a
//! counting fake connector and fake tunnels; no socket, no gateway. Every
//! request that could queue behind a held lock runs in its own task under a
//! bound, and the lock is released before a failing test reports.

use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::Arc;
use std::time::Duration;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use knx_app::property_validation::{LocalOutcome, System2Settlement};
use knx_net::DiscoveredGateway;
use knx_server::fake::{FakeConnector, FakeTunnel, FakeTunnelHandle};
use knx_server::{BusSessionError, BusTunnel, GatewayConnector};
use serde_json::{json, Value};
use tower::ServiceExt;

const GATEWAY: &str = "192.0.2.10:3671";
const OPERATION: &str = "0123456789abcdef0123456789abcdef";
/// Far above an instant answer; only a request queued behind a lock reaches it.
const BOUND: Duration = Duration::from_secs(5);

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
    handle: FakeTunnelHandle,
}

fn fixture() -> Fixture {
    let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 64);
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
        handle,
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

/// Starts the one monitor session; returns its id and server incarnation.
async fn start(f: &Fixture) -> (u64, String) {
    let (status, body) = call(
        &f.app,
        "POST",
        "/api/bus/monitor/start",
        Some(json!({ "gateway": GATEWAY })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    (
        body["sessionId"].as_u64().unwrap(),
        body["serverIncarnation"].as_str().unwrap().to_owned(),
    )
}

fn bound_uri(session_id: &str, incarnation: &str) -> String {
    format!("/api/bus/monitor/stop-bound?sessionId={session_id}&serverIncarnation={incarnation}")
}

/// Sends a POST in its own task and waits at most [`BOUND`]; on a miss the
/// reservation in `held` is released and the task awaited before failing.
async fn post_bounded<H>(
    app: &axum::Router,
    uri: &str,
    held: &mut Option<H>,
) -> (StatusCode, Value) {
    let request_app = app.clone();
    let uri = uri.to_owned();
    let mut task = tokio::spawn(async move { call(&request_app, "POST", &uri, None).await });
    match tokio::time::timeout(BOUND, &mut task).await {
        Ok(done) => done.expect("the request task panicked"),
        Err(_) => {
            held.take();
            let late = tokio::time::timeout(BOUND, &mut task).await;
            if late.is_err() {
                task.abort();
                let _ = task.await;
            }
            panic!(
                "the bound stop queued behind a held lock (late outcome after release: {late:?})"
            );
        }
    }
}

/// The session is still in the slot, still active, and its tunnel is open.
fn assert_untouched(f: &Fixture, id: u64) {
    let guard = f.state.bus_session.try_lock().expect("lock free");
    let session = guard.as_ref().expect("the session is still present");
    assert_eq!(session.id(), id);
    assert!(!f.handle.disconnected(), "the tunnel was disconnected");
}

fn connecting(session: &str) -> Value {
    json!({ "kind": "connecting", "session": session })
}

fn assert_session_changed(status: StatusCode, body: &Value) {
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "busSessionChanged", "{body}");
    let keys: Vec<&String> = body.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["error", "kind"], "{body}");
}

#[tokio::test]
async fn a_matching_bound_stop_releases_the_active_session_once_with_the_legacy_summary() {
    let f = fixture();
    let (id, incarnation) = start(&f).await;
    let (status, body) = call(
        &f.app,
        "POST",
        &bound_uri(&id.to_string(), &incarnation),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({ "sessionId": id, "serverIncarnation": incarnation, "telegramCount": 0, "droppedCount": 0 })
    );
    assert!(f.state.bus_session.try_lock().unwrap().is_none());
    assert!(f.handle.disconnected());

    // A repeat names a session that no longer exists: refused, nothing else.
    let (status, body) = call(
        &f.app,
        "POST",
        &bound_uri(&id.to_string(), &incarnation),
        None,
    )
    .await;
    assert_session_changed(status, &body);
    assert_eq!(f.connector.call_count(), 1);
}

#[tokio::test]
async fn a_matching_bound_stop_releases_a_closed_retained_session() {
    let f = fixture();
    let (id, incarnation) = start(&f).await;
    f.handle
        .sender()
        .send(knx_net::TunnelEvent::Closed)
        .unwrap();
    let mut closed = false;
    for _ in 0..500 {
        let (_, body) = call(&f.app, "GET", "/api/bus/monitor/telegrams?since=0", None).await;
        if body["status"] == "closed" {
            closed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert!(closed, "the fake close never reached the session");
    let (status, body) = call(
        &f.app,
        "POST",
        &bound_uri(&id.to_string(), &incarnation),
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["sessionId"], id);
    assert!(f.state.bus_session.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn another_session_id_or_an_older_process_incarnation_is_refused_without_a_stop() {
    let f = fixture();
    let (id, incarnation) = start(&f).await;
    let (status, body) = call(
        &f.app,
        "POST",
        &bound_uri(&(id + 1).to_string(), &incarnation),
        None,
    )
    .await;
    assert_session_changed(status, &body);
    assert_untouched(&f, id);

    // Ids restart at 1 per process: the same id from an older process differs
    // only in its incarnation and must not stop this one.
    let older = "AAAAAAAAAAAAAAAAAAAAAA";
    assert_ne!(older, incarnation);
    let (status, body) = call(&f.app, "POST", &bound_uri(&id.to_string(), older), None).await;
    assert_session_changed(status, &body);
    assert_untouched(&f, id);
}

#[tokio::test]
async fn a_bound_stop_without_a_session_is_a_structured_change_not_a_plain_conflict() {
    let f = fixture();
    let incarnation = f.state.server_incarnation.clone();
    let (status, body) = call(&f.app, "POST", &bound_uri("1", &incarnation), None).await;
    assert_session_changed(status, &body);
    assert!(f.state.bus_session.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn missing_partial_duplicate_unknown_or_malformed_identity_is_refused_before_any_effect() {
    let f = fixture();
    let (id, inc) = start(&f).await;
    let ok = id.to_string();
    let cases: Vec<String> = vec![
        String::new(),
        "?".into(),
        format!("?sessionId={ok}"),
        format!("?serverIncarnation={inc}"),
        format!("?sessionId={ok}&sessionId={ok}&serverIncarnation={inc}"),
        format!("?sessionId={ok}&serverIncarnation={inc}&serverIncarnation={inc}"),
        format!("?sessionId={ok}&serverIncarnation={inc}&force=1"),
        format!("?sessionid={ok}&serverIncarnation={inc}"),
        format!("?sessionId=&serverIncarnation={inc}"),
        format!("?sessionId&serverIncarnation={inc}"),
        format!("?sessionId={ok}&serverIncarnation="),
        format!("?sessionId=+{ok}&serverIncarnation={inc}"),
        format!("?sessionId=0{ok}&serverIncarnation={inc}"),
        format!("?sessionId=-{ok}&serverIncarnation={inc}"),
        format!("?sessionId={ok}.0&serverIncarnation={inc}"),
        format!("?sessionId=1e0&serverIncarnation={inc}"),
        format!("?sessionId=%20{ok}&serverIncarnation={inc}"),
        format!("?sessionId=18446744073709551616&serverIncarnation={inc}"),
        format!("?sessionId=abc&serverIncarnation={inc}"),
    ];
    // Held lock: a malformed request must answer before the lock is looked at.
    let mut held = Some(f.state.bus_session.lock().await);
    for query in &cases {
        let uri = format!("/api/bus/monitor/stop-bound{query}");
        let (status, body) = post_bounded(&f.app, &uri, &mut held).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
        assert!(body.get("kind").is_none(), "{uri}: {body}");
    }
    drop(held);
    for query in &cases {
        let uri = format!("/api/bus/monitor/stop-bound{query}");
        let (status, body) = call(&f.app, "POST", &uri, None).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{uri}: {body}");
    }
    assert_untouched(&f, id);
    assert_eq!(f.connector.call_count(), 1);
}

#[tokio::test]
async fn a_held_bus_session_lock_answers_busy_at_once_and_never_stops_late() {
    let f = fixture();
    let (id, inc) = start(&f).await;
    let uri = bound_uri(&id.to_string(), &inc);
    let mut held = Some(f.state.bus_session.lock().await);
    let (status, body) = post_bounded(&f.app, &uri, &mut held).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(
        body["holder"],
        connecting("busMonitorOrGroupWrite"),
        "{body}"
    );
    drop(held);
    // Nothing was queued: after the release the session is still there.
    tokio::task::yield_now().await;
    assert_untouched(&f, id);
}

#[tokio::test]
async fn a_held_lock_keeps_the_known_download_and_management_names() {
    let f = fixture();
    let (id, inc) = start(&f).await;
    let uri = bound_uri(&id.to_string(), &inc);
    let mut held = Some((
        f.state.device_download.lock().await,
        f.state.bus_session.lock().await,
    ));
    let (status, body) = post_bounded(&f.app, &uri, &mut held).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["holder"], connecting("deviceDownload"), "{body}");
    drop(held);
    let mut held = Some((
        f.state.address_programming.lock().await,
        f.state.bus_session.lock().await,
    ));
    let (status, body) = post_bounded(&f.app, &uri, &mut held).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["holder"], connecting("managementOperation"), "{body}");
    drop(held);
    assert_untouched(&f, id);
}

#[tokio::test]
async fn a_running_system2_operation_is_named_first_and_never_preempted() {
    let f = fixture();
    let (id, inc) = start(&f).await;
    let uri = bound_uri(&id.to_string(), &inc);
    let lease = f.state.device_system2.control.try_admit(OPERATION).unwrap();
    // Admitted, locks not yet held: still refused, the session stays.
    let mut none: Option<()> = None;
    let (status, body) = post_bounded(&f.app, &uri, &mut none).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy", "{body}");
    assert_eq!(body["holder"], connecting("system2Operation"), "{body}");
    assert_untouched(&f, id);
    // Admitted and holding the session lock: System-2 wins the name.
    let mut held = Some(f.state.bus_session.lock().await);
    let (status, body) = post_bounded(&f.app, &uri, &mut held).await;
    assert_eq!(
        body["holder"],
        connecting("system2Operation"),
        "{status} {body}"
    );
    drop(held);
    assert!(matches!(
        f.state.device_system2.control.snapshot().settlement,
        System2Settlement::Running { .. }
    ));
    lease.settle(LocalOutcome::Failed).unwrap();
    assert_untouched(&f, id);
    let (status, body) = call(&f.app, "POST", &uri, None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// Shutdown policy: admission refuses new bus jobs only. Releasing a tunnel
/// is what the orderly stop does itself, so a matching bound stop still
/// releases, exactly like the legacy stop; a foreign identity still refuses.
#[tokio::test]
async fn a_closed_stop_admission_still_lets_the_named_session_be_released() {
    let f = fixture();
    let (id, inc) = start(&f).await;
    f.state.stop_admission.close();
    let (status, body) = call(
        &f.app,
        "POST",
        &bound_uri(&(id + 7).to_string(), &inc),
        None,
    )
    .await;
    assert_session_changed(status, &body);
    assert_untouched(&f, id);
    let (status, body) = call(&f.app, "POST", &bound_uri(&id.to_string(), &inc), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(f.handle.disconnected());
}

/// Why the route is separate: the legacy stop ignores any query and stops
/// whatever it holds, and it still waits for a held lock. Both unchanged.
#[tokio::test]
async fn the_legacy_stop_is_unchanged_it_ignores_identity_and_still_queues() {
    let f = fixture();
    let (id, _) = start(&f).await;
    let (status, body) = call(
        &f.app,
        "POST",
        "/api/bus/monitor/stop?sessionId=999&serverIncarnation=foreign",
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["sessionId"], id);
    assert!(f.state.bus_session.try_lock().unwrap().is_none());

    // The fake connector scripts one tunnel per fixture.
    let g = fixture();
    let (id, _) = start(&g).await;
    let held = g.state.bus_session.lock().await;
    let app = g.app.clone();
    let mut task =
        tokio::spawn(async move { call(&app, "POST", "/api/bus/monitor/stop", None).await });
    assert!(
        tokio::time::timeout(Duration::from_millis(200), &mut task)
            .await
            .is_err(),
        "the legacy stop no longer waits for the lock"
    );
    drop(held);
    let (status, body) = tokio::time::timeout(BOUND, task).await.unwrap().unwrap();
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["sessionId"], id);
}

#[tokio::test]
async fn a_get_on_the_bound_route_is_not_a_stop() {
    let f = fixture();
    let (id, inc) = start(&f).await;
    let (status, _) = call(&f.app, "GET", &bound_uri(&id.to_string(), &inc), None).await;
    assert_eq!(status, StatusCode::METHOD_NOT_ALLOWED);
    assert_untouched(&f, id);
}
