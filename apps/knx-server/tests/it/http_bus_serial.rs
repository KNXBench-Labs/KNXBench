//! Verify strict serial HTTP admission, shared ownership and group traffic using fake tunnels.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use knx_net::{
    ApplicationService, BusError, Destination, DiscoveredGateway, LDataFrame, LDataMessageKind,
    Tpci, TunnelEvent,
};
use knx_server::{
    bus_serial::SerialTarget,
    fake::{FakeTunnel, FakeTunnelHandle},
    AppState, BusSessionError, BusTunnel, GatewayConnector,
};
use serde_json::{json, Value};
use std::{
    future::Future,
    net::SocketAddrV4,
    pin::Pin,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tower::ServiceExt;

type LinkFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + 'a>>;
struct Connector {
    calls: Arc<Mutex<Vec<SerialTarget>>>,
    ip_calls: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
    failure_message: Arc<Mutex<Option<String>>>,
    tunnel: Mutex<Option<Box<dyn BusTunnel>>>,
}
impl GatewayConnector for Connector {
    fn connect_tunnel(&self, _: SocketAddrV4) -> LinkFuture<'_> {
        self.ip_calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async { Err(BusSessionError::Transport(BusError::Timeout)) })
    }
    fn connect_serial<'a>(&'a self, target: &'a SerialTarget) -> LinkFuture<'a> {
        self.calls.lock().unwrap().push(target.clone());
        Box::pin(async move {
            if self.fail.load(Ordering::SeqCst) {
                return Err(BusSessionError::Transport(
                    self.failure_message
                        .lock()
                        .unwrap()
                        .clone()
                        .map(BusError::Protocol)
                        .unwrap_or(BusError::Timeout),
                ));
            }
            Ok(self.tunnel.lock().unwrap().take().unwrap())
        })
    }
    fn discover(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<DiscoveredGateway>, BusSessionError>> + Send + '_>>
    {
        panic!("serial workflow must not discover gateways")
    }
}
/// Disconnect really releases the fake, then reports a failed reset acknowledgement.
struct CleanupFailure(FakeTunnel);
impl BusTunnel for CleanupFailure {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        self.0.assigned_address()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TunnelEvent> {
        self.0.subscribe()
    }
    fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        self.0.send(destination, service)
    }
    fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        self.0.send_frame(destination, transport, service)
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async move {
            Box::new(self.0).disconnect().await?;
            Err(BusSessionError::Transport(BusError::Timeout))
        })
    }
}
struct Fixture {
    state: Arc<AppState>,
    app: Router,
    handle: FakeTunnelHandle,
    calls: Arc<Mutex<Vec<SerialTarget>>>,
    ip_calls: Arc<AtomicUsize>,
    fail: Arc<AtomicBool>,
    failure_message: Arc<Mutex<Option<String>>>,
}
fn fixture() -> Fixture {
    fixture_with_cleanup(false)
}
fn fixture_with_cleanup(fail_cleanup: bool) -> Fixture {
    let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 64);
    let calls = Arc::new(Mutex::new(vec![]));
    let ip_calls = Arc::new(AtomicUsize::new(0));
    let fail = Arc::new(AtomicBool::new(false));
    let failure_message = Arc::new(Mutex::new(None));
    let state = Arc::new(AppState {
        connector: Box::new(Connector {
            calls: calls.clone(),
            ip_calls: ip_calls.clone(),
            fail: fail.clone(),
            failure_message: failure_message.clone(),
            tunnel: Mutex::new(Some(if fail_cleanup {
                Box::new(CleanupFailure(tunnel))
            } else {
                Box::new(tunnel)
            })),
        }),
        ..Default::default()
    });
    let app = knx_server::app(state.clone(), None);
    Fixture {
        state,
        app,
        handle,
        calls,
        ip_calls,
        fail,
        failure_message,
    }
}
fn target(mode: &str) -> Value {
    json!({ "path": "/synthetic/tpuart", "source": "1.1.250", "mode": mode })
}
async fn raw(app: &Router, method: &str, uri: &str, body: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(body.to_owned()))
                .unwrap(),
        )
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
async fn call(app: &Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    raw(app, method, uri, &body.to_string()).await
}
async fn start(f: &Fixture, mode: &str) -> Value {
    let (status, body) = call(
        &f.app,
        "POST",
        "/api/bus/monitor/start-serial",
        target(mode),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["serial"], target(mode));
    assert_eq!(body["assignedAddress"], "1.1.250");
    assert_eq!(
        serde_json::to_value(&*f.calls.lock().unwrap()).unwrap(),
        json!([target(mode)])
    );
    assert_eq!(f.ip_calls.load(Ordering::SeqCst), 0);
    body
}
async fn stop(f: &Fixture, started: &Value) -> Value {
    let uri = format!(
        "/api/bus/monitor/stop-bound?sessionId={}&serverIncarnation={}",
        started["sessionId"],
        started["serverIncarnation"].as_str().unwrap()
    );
    let (status, body) = call(&f.app, "POST", &uri, Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(f.handle.disconnected());
    assert!(f.state.bus_session.try_lock().unwrap().is_none());
    body
}
#[tokio::test]
async fn malformed_missing_both_unknown_and_duplicate_fields_never_reach_connector() {
    let f = fixture();
    for body in [
        json!({}),
        json!({"path":"x"}),
        json!({"path":"x", "mode":"passive"}),
        json!({"source":"1.1.250", "mode":"passive"}),
        json!({"path":"x","source":"1.1.250"}),
        json!({"path":"x","source":"1.1.250","mode":"auto"}),
        json!({"path":"x","source":"1.1.250","mode":"passive","gateway":"192.0.2.1:3671"}),
        json!({"path":"x","source":"1.1.250","mode":"passive","unknown":true}),
        json!({"path":"x","source":"1.1.250","mode":null}),
    ] {
        assert_eq!(
            call(&f.app, "POST", "/api/bus/monitor/start-serial", body)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    for body in [
        "{",
        r#"{"path":"x","path":"y","source":"1.1.250","mode":"passive"}"#,
    ] {
        assert_eq!(
            raw(&f.app, "POST", "/api/bus/monitor/start-serial", body)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    for path in ["", " ", " x", "x\n", "x\0"] {
        let mut body = target("passive");
        body["path"] = path.into();
        assert_eq!(
            call(&f.app, "POST", "/api/bus/monitor/start-serial", body)
                .await
                .0,
            StatusCode::BAD_REQUEST
        );
    }
    for source in [
        "",
        "0.0.0",
        "15.15.255",
        "1.1.220",
        "16.1.1",
        "1.16.1",
        "1.1.256",
        "01.1.1",
        "1.1.1\n",
    ] {
        let mut body = target("passive");
        body["source"] = source.into();
        assert_eq!(
            call(&f.app, "POST", "/api/bus/monitor/start-serial", body)
                .await
                .0,
            StatusCode::BAD_REQUEST,
            "{source}"
        );
    }
    assert!(f.calls.lock().unwrap().is_empty());
    assert_eq!(f.ip_calls.load(Ordering::SeqCst), 0);
    assert!(f.state.bus_session.try_lock().unwrap().is_none());
}
#[tokio::test]
async fn serial_start_refuses_held_owner_immediately_without_queueing() {
    let f = fixture();
    let held = f.state.bus_session.lock().await;
    let answer = tokio::time::timeout(
        Duration::from_secs(1),
        call(
            &f.app,
            "POST",
            "/api/bus/monitor/start-serial",
            target("active"),
        ),
    )
    .await;
    drop(held);
    let (status, body) = answer.expect("serial start queued behind a held owner");
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["kind"], "gatewayBusy");
    assert!(f.calls.lock().unwrap().is_empty());
    assert_eq!(f.ip_calls.load(Ordering::SeqCst), 0);
    let started = start(&f, "active").await;
    stop(&f, &started).await;
}
#[tokio::test]
async fn failed_connect_has_no_session_or_ip_fallback() {
    let f = fixture();
    f.fail.store(true, Ordering::SeqCst);
    let (status, body) = call(
        &f.app,
        "POST",
        "/api/bus/monitor/start-serial",
        target("passive"),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY);
    assert_eq!(body["kind"], "serialConnectFailed");
    assert!(f.state.bus_session.try_lock().unwrap().is_none());
    assert_eq!(f.calls.lock().unwrap().len(), 1);
    assert_eq!(f.ip_calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn passive_subscription_projection_poll_and_bound_stop_keep_real_target() {
    let f = fixture();
    let started = start(&f, "passive").await;
    let (_, activity) = call(&f.app, "GET", "/api/bus/activity", Value::Null).await;
    assert_eq!(
        activity["sessions"][0],
        json!({"kind":"busMonitor","id":started["sessionId"],"serial":target("passive"),"state":"active"})
    );
    f.handle
        .sender()
        .send(TunnelEvent::Telegram(LDataFrame {
            kind: LDataMessageKind::Indication,
            source: "1.1.5".parse().unwrap(),
            destination: Destination::Group(knx_core::GroupAddress::from_raw(0x0901)),
            transport: Tpci::UnnumberedData,
            service: ApplicationService::GroupValueRead,
            control: None,
        }))
        .unwrap();
    let poll = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let (status, poll) = call(
                &f.app,
                "GET",
                "/api/bus/monitor/telegrams?since=0",
                Value::Null,
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            if !poll["telegrams"].as_array().unwrap().is_empty() {
                break poll;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(poll["serial"], target("passive"));
    assert_eq!(poll["sessionId"], started["sessionId"]);
    assert_eq!(poll["serverIncarnation"], started["serverIncarnation"]);
    assert_eq!(poll["telegrams"][0]["source"], "1.1.5");
    assert_eq!(poll["telegrams"][0]["destination"], "2305");
    assert_eq!(poll["telegrams"][0]["service"], "GroupValueRead");
    let wrong = format!(
        "/api/bus/monitor/stop-bound?sessionId={}&serverIncarnation=other",
        started["sessionId"]
    );
    assert_eq!(
        call(&f.app, "POST", &wrong, Value::Null).await.1["kind"],
        "busSessionChanged"
    );
    assert!(!f.handle.disconnected());
    let busy = call(
        &f.app,
        "POST",
        "/api/bus/monitor/start",
        json!({"gateway":"192.0.2.1:3671"}),
    )
    .await;
    assert_eq!(busy.1["holder"]["serial"], target("passive"));
    assert!(busy.1["holder"].get("gateway").is_none());
    assert_eq!(stop(&f, &started).await["telegramCount"], 1);
    assert_eq!(f.ip_calls.load(Ordering::SeqCst), 0);
}
#[tokio::test]
async fn passive_group_read_and_write_refuse_before_tunnel_send() {
    let f = fixture();
    let started = start(&f, "passive").await;
    for (uri, body) in [
        (
            "/api/bus/read",
            corrections::binding(&started, json!({"destination":"1/1/1"})),
        ),
        (
            "/api/bus/write-bound",
            corrections::binding(
                &started,
                json!({"destination":"1/1/1","dpt":"DPST-1-1","value":"on"}),
            ),
        ),
    ] {
        let (status, body) = call(&f.app, "POST", uri, body).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["kind"], "passiveSerial");
    }
    assert!(f.handle.sent_calls().is_empty());
    stop(&f, &started).await;
}
#[tokio::test]
async fn explicit_active_group_read_write_and_failure_use_same_tunnel_once() {
    let f = fixture();
    let started = start(&f, "active").await;
    assert_eq!(
        call(
            &f.app,
            "POST",
            "/api/bus/read",
            corrections::binding(&started, json!({"destination":"1/1/1"}))
        )
        .await,
        (StatusCode::OK, json!({"service":"GroupValueRead"}))
    );
    let write = corrections::binding(
        &started,
        json!({"destination":"1/1/1","dpt":"DPST-1-1","value":"on"}),
    );
    let (status, body) = call(&f.app, "POST", "/api/bus/write-bound", write.clone()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["service"], "GroupValueWrite");
    assert_eq!(body["encodedPayload"], "0x01 (6-bit)");
    assert_eq!(f.handle.sent_calls().len(), 2);
    assert_eq!(
        f.handle.sent_calls()[0].1,
        ApplicationService::GroupValueRead
    );
    f.handle.fail_next_send();
    assert_eq!(
        call(&f.app, "POST", "/api/bus/write-bound", write).await.0,
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(f.handle.sent_calls().len(), 2);
    assert_eq!(f.calls.lock().unwrap().len(), 1);
    assert_eq!(f.ip_calls.load(Ordering::SeqCst), 0);
    stop(&f, &started).await;
}

#[tokio::test]
async fn serial_cleanup_failure_is_reported_after_release_without_reconnecting() {
    let f = fixture_with_cleanup(true);
    let started = start(&f, "active").await;
    let stopped = stop(&f, &started).await;
    assert!(stopped["warning"]
        .as_str()
        .unwrap()
        .contains("auto-ACK state unknown"));
    assert!(stopped["warning"]
        .as_str()
        .unwrap()
        .contains("do not retry automatically"));
    assert_eq!(f.calls.lock().unwrap().len(), 1);
    assert_eq!(f.ip_calls.load(Ordering::SeqCst), 0);
}

#[path = "http_bus_serial/corrections.rs"]
mod corrections;

#[tokio::test]
async fn active_initialization_failure_exposes_conservative_auto_ack_uncertainty_without_parsing() {
    for (mode, message, uncertain) in [
        (
            "active",
            Some("initialization failed; reset failed; auto-ACK state unknown"),
            true,
        ),
        ("active", None, true),
        ("passive", None, false),
    ] {
        let f = fixture();
        f.fail.store(true, Ordering::SeqCst);
        *f.failure_message.lock().unwrap() = message.map(str::to_owned);
        let (status, body) = call(
            &f.app,
            "POST",
            "/api/bus/monitor/start-serial",
            target(mode),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_GATEWAY);
        assert_eq!(body["kind"], "serialConnectFailed");
        assert_eq!(body["activeAutoAckStateUncertain"], uncertain);
        if let Some(message) = message {
            assert!(body["error"].as_str().unwrap().contains(message));
        }
        assert!(f.state.bus_session.try_lock().unwrap().is_none());
        assert_eq!(f.calls.lock().unwrap().len(), 1);
        assert_eq!(f.ip_calls.load(Ordering::SeqCst), 0);
    }
    let f = fixture();
    let mut invalid = target("active");
    invalid["path"] = "".into();
    let (status, body) = call(&f.app, "POST", "/api/bus/monitor/start-serial", invalid).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(body.get("activeAutoAckStateUncertain").is_none());
    assert!(f.calls.lock().unwrap().is_empty());
}
