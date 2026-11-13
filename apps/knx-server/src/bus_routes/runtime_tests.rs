//! Prove bus project waits leave the runtime free without surrendering session custody.

use std::{
    future::Future,
    net::SocketAddrV4,
    pin::Pin,
    sync::{mpsc, Arc, Mutex},
    task::{Context, Wake, Waker},
    time::Duration,
};

use axum::{body::Body, http::Request, http::StatusCode, Router};
use knx_core::{DeviceId, DeviceInstance, SourceRef};
use knx_net::TunnelTransport;
use serde_json::{json, Value};
use tokio::sync::{Notify, Semaphore};
use tower::ServiceExt;

use crate::bus::fake::{FakeConnector, FakeTunnel, FakeTunnelHandle};
use crate::bus::{BusSessionError, BusTunnel, GatewayConnector};
use crate::bus_serial::SerialTarget;
use crate::{domain, AppState};

const WAIT: Duration = Duration::from_secs(10);
const GATEWAY: &str = "192.0.2.10:3671";
type Link<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Target {
    Udp,
    Tcp,
    Serial,
}
impl Target {
    fn path(self) -> &'static str {
        match self {
            Self::Udp => "/api/bus/monitor/start",
            Self::Tcp => "/api/bus/monitor/start-tcp",
            Self::Serial => "/api/bus/monitor/start-serial",
        }
    }
    fn body(self) -> Value {
        match self {
            Self::Udp | Self::Tcp => json!({"gateway": GATEWAY}),
            Self::Serial => {
                json!({"path": "/synthetic/tpuart", "source": "1.1.250", "mode": "passive"})
            }
        }
    }
}

/// Gate the existing in-memory fake, recording the exact connector selected.
struct Connector {
    ip: FakeConnector,
    serial: Mutex<Option<FakeTunnel>>,
    calls: Arc<Mutex<Vec<Target>>>,
    entered: Arc<Notify>,
    permit: Arc<Semaphore>,
}
impl Connector {
    async fn enter(&self, target: Target) {
        self.calls.lock().unwrap().push(target);
        self.entered.notify_one();
        self.permit.acquire().await.unwrap().forget();
    }
}
impl GatewayConnector for Connector {
    fn connect_tunnel(&self, gateway: SocketAddrV4) -> Link<'_> {
        Box::pin(async move {
            self.enter(Target::Udp).await;
            self.ip.connect_tunnel(gateway).await
        })
    }
    fn connect_tunnel_with_transport(
        &self,
        gateway: SocketAddrV4,
        transport: TunnelTransport,
    ) -> Link<'_> {
        Box::pin(async move {
            assert_eq!(transport, TunnelTransport::Tcp);
            self.enter(Target::Tcp).await;
            self.ip
                .connect_tunnel_with_transport(gateway, transport)
                .await
        })
    }
    fn connect_serial<'a>(&'a self, target: &'a SerialTarget) -> Link<'a> {
        Box::pin(async move {
            assert_eq!(serde_json::to_value(target).unwrap(), Target::Serial.body());
            self.enter(Target::Serial).await;
            Ok(Box::new(self.serial.lock().unwrap().take().unwrap()) as Box<dyn BusTunnel>)
        })
    }
    fn discover(
        &self,
    ) -> Pin<
        Box<
            dyn Future<Output = Result<Vec<knx_net::DiscoveredGateway>, BusSessionError>>
                + Send
                + '_,
        >,
    > {
        panic!("project-wait tests never discover or open a real endpoint")
    }
}

struct Fixture {
    state: Arc<AppState>,
    app: Router,
    handle: FakeTunnelHandle,
    calls: Arc<Mutex<Vec<Target>>>,
    entered: Arc<Notify>,
    permit: Arc<Semaphore>,
}
fn fixture(target: Target, permits: usize) -> Fixture {
    let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 32);
    let tunnel = if target == Target::Tcp {
        tunnel.with_transport(TunnelTransport::Tcp)
    } else {
        tunnel
    };
    let calls = Arc::new(Mutex::new(vec![]));
    let entered = Arc::new(Notify::new());
    let permit = Arc::new(Semaphore::new(permits));
    let (unused, _) = FakeTunnel::new("1.1.250".parse().unwrap(), 1);
    let (ip, serial) = if target == Target::Serial {
        (unused, Some(tunnel))
    } else {
        (tunnel, None)
    };
    let state = Arc::new(AppState {
        connector: Box::new(Connector {
            ip: FakeConnector::succeeding(ip).accepting_tcp(),
            serial: Mutex::new(serial),
            calls: calls.clone(),
            entered: entered.clone(),
            permit: permit.clone(),
        }),
        ..Default::default()
    });
    domain::new_project_impl(&state, None, None, None, None, true).unwrap();
    {
        let mut project = state.project.lock().unwrap();
        let project = project.as_mut().unwrap();
        let id = project.ids.next_device_id().unwrap();
        assert_eq!(id, DeviceId(1));
        project.devices.insert(DeviceInstance {
            id,
            source: SourceRef {
                path: "KB-DEV-1".into(),
                ets_id: "KB-DEV-1".into(),
            },
            name: "Missing".into(),
            description: None,
            address: Some("1.1.2".parse().unwrap()),
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        project.installations[0].topology.unassigned.push(id);
    }
    let app = super::bus_routes().with_state(state.clone());
    Fixture {
        state,
        app,
        handle,
        calls,
        entered,
        permit,
    }
}

async fn call(app: Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap();
    let response = tokio::time::timeout(WAIT, app.oneshot(request))
        .await
        .unwrap()
        .unwrap();
    let status = response.status();
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

/// The holder's wall-clock deadline also bounds an inline-starvation mutant.
/// Drop releases and joins even when an assertion or timed request fails.
struct HeldProject {
    release: Option<mpsc::Sender<()>>,
    thread: Option<std::thread::JoinHandle<bool>>,
}
impl HeldProject {
    fn new(state: &Arc<AppState>) -> Self {
        let state = state.clone();
        let (held, ready) = mpsc::channel();
        let (release, wait) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let _guard = state.project.lock().unwrap();
            held.send(()).unwrap();
            wait.recv_timeout(WAIT).is_ok()
        });
        ready.recv_timeout(WAIT).unwrap();
        Self {
            release: Some(release),
            thread: Some(thread),
        }
    }
    fn release(&mut self) -> bool {
        let _ = self.release.take().unwrap().send(());
        self.thread.take().unwrap().join().unwrap()
    }
}
impl Drop for HeldProject {
    fn drop(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

async fn timer_while_pending<T>(call: &tokio::task::JoinHandle<T>) -> bool {
    tokio::task::yield_now().await;
    tokio::time::sleep(Duration::from_millis(20)).await;
    !call.is_finished()
}

async fn completed_scan(f: &Fixture, bound: bool) -> Value {
    let path = if bound {
        "/api/bus/scan/start-tcp"
    } else {
        "/api/bus/scan/start"
    };
    let (status, start) = call(
        f.app.clone(),
        "POST",
        path,
        json!({
            "gateway": GATEWAY, "area": 1, "line": 1, "firstDevice": 2,
            "lastDevice": 2, "excluded": [], "responseTimeoutMs": 1, "interProbePauseMs": 0
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{start}");
    tokio::time::timeout(WAIT, async {
        loop {
            let (status, snapshot) = call(
                f.app.clone(),
                "GET",
                "/api/bus/scan/results?since=0",
                Value::Null,
            )
            .await;
            assert_eq!(status, StatusCode::OK);
            if snapshot["status"] != "running" {
                assert_eq!(snapshot["status"], "completed");
                assert_eq!(snapshot["results"][0]["outcome"]["kind"], "vacant");
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert!(f.handle.disconnected());
    start
}
fn scan_call(
    start: &Value,
    bound: bool,
    reconcile: bool,
    mutate: bool,
) -> (&'static str, String, Value) {
    let action = if reconcile { "reconcile" } else { "comparison" };
    let uri = if bound {
        format!(
            "/api/bus/scan/{action}-bound?sessionId={}&serverIncarnation={}",
            start["sessionId"],
            start["serverIncarnation"].as_str().unwrap()
        )
    } else if reconcile {
        format!("/api/bus/scan/{action}")
    } else {
        format!("/api/bus/scan/{action}?sessionId={}", start["sessionId"])
    };
    let mut body =
        json!({"unexpected": [], "missing": if mutate { vec!["1.1.2"] } else { vec![] }});
    if !bound {
        body["sessionId"] = start["sessionId"].clone();
    }
    (if reconcile { "POST" } else { "GET" }, uri, body)
}
async fn busy(f: &Fixture, method: &str, uri: &str, body: Value, session: &str) {
    let (status, body) = call(f.app.clone(), method, uri, body).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy");
    assert_eq!(
        body["holder"],
        json!({"kind": "connecting", "session": session})
    );
}

#[tokio::test(flavor = "current_thread")]
async fn scan_project_routes_leave_the_runtime_free() {
    for (bound, reconcile) in [(false, false), (true, false), (false, true), (true, true)] {
        let f = fixture(if bound { Target::Tcp } else { Target::Udp }, 1);
        let start = completed_scan(&f, bound).await;
        let expected = if reconcile {
            serde_json::to_value(domain::current_project_tree(&f.state).unwrap().tree).unwrap()
        } else {
            json!({"unexpected": [], "missing": ["1.1.2"], "excludedInProject": []})
        };
        let (method, uri, body) = scan_call(&start, bound, reconcile, false);
        let mut held = HeldProject::new(&f.state);
        let app = f.app.clone();
        let path = uri.clone();
        let request_body = body.clone();
        let request = tokio::spawn(async move { call(app, method, &path, request_body).await });
        let pending = timer_while_pending(&request).await;
        // A busy request must refuse before this project's lock is released.
        if pending {
            busy(&f, method, &uri, body, "lineScan").await;
        }
        let runtime_free = held.release();
        let result = tokio::time::timeout(WAIT, request).await.unwrap().unwrap();
        assert!(runtime_free, "{uri} blocked the runtime");
        assert!(pending, "{uri} answered before reaching the project lock");
        assert_eq!(result, (StatusCode::OK, expected), "{uri}");
        assert_eq!(
            f.calls.lock().unwrap().len(),
            1,
            "comparison/reconcile never connect"
        );
        f.state
            .line_scan_session
            .lock()
            .await
            .take()
            .unwrap()
            .cancel()
            .await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn scan_routes_reserve_in_the_request_and_refuse_busy_without_queueing() {
    for (bound, reconcile) in [(false, false), (true, false), (false, true), (true, true)] {
        let f = fixture(if bound { Target::Tcp } else { Target::Udp }, 1);
        let start = completed_scan(&f, bound).await;
        let (method, uri, body) = scan_call(&start, bound, reconcile, false);
        let mut held = HeldProject::new(&f.state);
        let mut request = Box::pin(call(f.app.clone(), method, &uri, body.clone()));
        std::future::poll_fn(|cx| {
            assert!(request.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        let reserved = f.state.line_scan_session.try_lock().is_err();
        busy(&f, method, &uri, body, "lineScan").await;
        assert!(held.release(), "{uri} blocked the runtime");
        assert_eq!(request.await.0, StatusCode::OK);
        assert!(reserved, "{uri} deferred admission to a scheduled job");
        f.state
            .line_scan_session
            .lock()
            .await
            .take()
            .unwrap()
            .cancel()
            .await;
    }
}

#[tokio::test(flavor = "current_thread")]
async fn dropped_scan_requests_keep_the_reservation_until_their_project_work_finishes() {
    for (bound, reconcile) in [(false, false), (true, false), (false, true), (true, true)] {
        let f = fixture(if bound { Target::Tcp } else { Target::Udp }, 1);
        let start = completed_scan(&f, bound).await;
        let (method, uri, body) = scan_call(&start, bound, reconcile, reconcile);
        let mut held = HeldProject::new(&f.state);
        let app = f.app.clone();
        let path = uri.clone();
        let request_body = body.clone();
        let request = tokio::spawn(async move { call(app, method, &path, request_body).await });
        let pending = timer_while_pending(&request).await;
        request.abort();
        let _ = request.await;
        let reserved = f.state.line_scan_session.try_lock().is_err();
        // Cancel and replacement also use this same reservation; no request can overtake it.
        busy(
            &f,
            "POST",
            &format!("/api/bus/scan/cancel?sessionId={}", start["sessionId"]),
            Value::Null,
            "lineScan",
        )
        .await;
        let runtime_free = held.release();
        let mut session = tokio::time::timeout(WAIT, f.state.line_scan_session.lock())
            .await
            .unwrap()
            .take()
            .unwrap();
        session.cancel().await;
        assert!(runtime_free, "{uri} blocked the runtime");
        assert!(pending, "{uri} answered before reaching the project lock");
        assert!(reserved, "{uri} lost its reservation on client drop");
        let project = f.state.project.lock().unwrap();
        assert_eq!(
            project.as_ref().unwrap().devices.get(DeviceId(1)).is_none(),
            reconcile,
            "a dropped reconciliation must still finish its selected deletion"
        );
    }
}

#[tokio::test(flavor = "current_thread")]
async fn monitor_starts_leave_the_runtime_free_and_refuse_competing_starts() {
    for target in [Target::Udp, Target::Tcp, Target::Serial] {
        let f = fixture(target, 1);
        let mut held = HeldProject::new(&f.state);
        let app = f.app.clone();
        let request =
            tokio::spawn(async move { call(app, "POST", target.path(), target.body()).await });
        let pending = timer_while_pending(&request).await;
        if pending {
            for competitor in [Target::Udp, Target::Tcp, Target::Serial] {
                busy(
                    &f,
                    "POST",
                    competitor.path(),
                    competitor.body(),
                    "busMonitorOrGroupWrite",
                )
                .await;
            }
        }
        let calls_before_release = f.calls.lock().unwrap().len();
        let runtime_free = held.release();
        let (status, body) = tokio::time::timeout(WAIT, request).await.unwrap().unwrap();
        let mut expected = json!({"sessionId": 1, "serverIncarnation": f.state.server_incarnation,
            "assignedAddress": "1.1.250", "transport": if target == Target::Tcp { json!("tcp") } else if target == Target::Udp { json!("udp") } else { Value::Null }});
        if target == Target::Serial {
            expected["serial"] = target.body();
            expected.as_object_mut().unwrap().remove("transport");
        }
        if target == Target::Tcp {
            expected["skippedFrames"] = json!(0);
        }
        let session = tokio::time::timeout(WAIT, f.state.bus_session.lock())
            .await
            .unwrap()
            .take()
            .unwrap();
        session.stop().await;
        assert!(runtime_free, "{target:?} blocked the runtime");
        assert!(
            pending,
            "{target:?} answered before reaching the project lock"
        );
        assert_eq!(calls_before_release, 0);
        assert_eq!((status, body), (StatusCode::OK, expected));
        assert_eq!(*f.calls.lock().unwrap(), vec![target]);
        assert!(f.handle.disconnected());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn monitor_reserves_before_scheduling_its_context_snapshot() {
    for target in [Target::Udp, Target::Tcp, Target::Serial] {
        let f = fixture(target, 1);
        let mut request = Box::pin(call(f.app.clone(), "POST", target.path(), target.body()));
        std::future::poll_fn(|cx| {
            assert!(request.as_mut().poll(cx).is_pending());
            std::task::Poll::Ready(())
        })
        .await;
        let reserved = f.state.bus_session.try_lock().is_err();
        busy(
            &f,
            "POST",
            target.path(),
            target.body(),
            "busMonitorOrGroupWrite",
        )
        .await;
        assert_eq!(request.await.0, StatusCode::OK);
        let released_on_success = f.state.bus_session.try_lock().is_ok();
        f.state
            .bus_session
            .lock()
            .await
            .take()
            .unwrap()
            .stop()
            .await;
        assert!(
            released_on_success,
            "{target:?} returned success before releasing its reservation"
        );
        assert!(
            reserved,
            "{target:?} scheduled its reservation instead of taking it immediately"
        );
        assert_eq!(*f.calls.lock().unwrap(), vec![target]);
    }
}

#[tokio::test(flavor = "current_thread")]
async fn closing_admission_during_the_monitor_project_wait_opens_nothing() {
    for target in [Target::Udp, Target::Tcp, Target::Serial] {
        let f = fixture(target, 1);
        let mut held = HeldProject::new(&f.state);
        let app = f.app.clone();
        let request =
            tokio::spawn(async move { call(app, "POST", target.path(), target.body()).await });
        let pending = timer_while_pending(&request).await;
        f.state.stop_admission.close();
        let runtime_free = held.release();
        let (status, body) = tokio::time::timeout(WAIT, request).await.unwrap().unwrap();
        assert!(runtime_free, "{target:?} blocked the runtime");
        assert!(
            pending,
            "{target:?} answered before reaching the project lock"
        );
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
        assert!(
            f.calls.lock().unwrap().is_empty(),
            "late connector after stop closed admission"
        );
        assert!(f.state.bus_session.try_lock().unwrap().is_none());
    }
}

#[tokio::test(flavor = "current_thread")]
async fn scan_binding_and_body_refusals_keep_their_order_before_the_project_wait() {
    let f = fixture(Target::Tcp, 1);
    let start = completed_scan(&f, true).await;
    let mut held = HeldProject::new(&f.state);
    for reconcile in [false, true] {
        let (method, uri, body) = scan_call(&start, false, reconcile, false);
        let (status, error) = call(f.app.clone(), method, &uri, body).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(error["kind"], "lineScanSessionBindingRequired");

        let mut wrong = start.clone();
        wrong["serverIncarnation"] = json!("another-server");
        let (method, uri, body) = scan_call(&wrong, true, reconcile, false);
        let (status, error) = call(f.app.clone(), method, &uri, body.clone()).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(error["kind"], "lineScanSessionChanged");
        let guard = f.state.line_scan_session.lock().await;
        // With a reservation held, busy precedes an otherwise valid but stale binding.
        busy(&f, method, &uri, body, "lineScan").await;
        // Malformed bindings precede even that busy refusal.
        let bad_query = format!("{uri}&extra=1");
        let (status, _) = call(
            f.app.clone(),
            method,
            &bad_query,
            json!({"unexpected": [], "missing": []}),
        )
        .await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        drop(guard);
    }
    let (_, uri, _) = scan_call(&start, true, true, false);
    let guard = f.state.line_scan_session.lock().await;
    let (status, error) = call(
        f.app.clone(),
        "POST",
        &uri,
        json!({"unexpected": [], "missing": [], "foreign": true}),
    )
    .await;
    assert_eq!(
        (status, error),
        (
            StatusCode::BAD_REQUEST,
            json!({"error": "bound reconciliation requires exactly unexpected and missing selections"})
        )
    );
    drop(guard);
    let (status, error) = call(
        f.app.clone(),
        "POST",
        &uri,
        json!({"unexpected": ["invalid"], "missing": []}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert!(error["error"]
        .as_str()
        .unwrap()
        .starts_with("invalid unexpected address"));
    assert!(
        held.release(),
        "a pre-project refusal waited for the project"
    );
    f.state
        .line_scan_session
        .lock()
        .await
        .take()
        .unwrap()
        .cancel()
        .await;
}

struct DeliveryWake(Notify);
impl Wake for DeliveryWake {
    fn wake(self: Arc<Self>) {
        self.0.notify_one();
    }
}

async fn dropped_delivered_monitor_start(target: Target) {
    let f = fixture(target, 1);
    let request = Request::builder()
        .method("POST")
        .uri(target.path())
        .header("content-type", "application/json")
        .body(Body::from(target.body().to_string()))
        .unwrap();
    // Poll the mounted HTTP future once, then observe its wake without repolling it.
    // Unlike `call`, this future has no timer that could produce an unrelated wake.
    let mut request = Box::pin(f.app.clone().oneshot(request));
    let wake = Arc::new(DeliveryWake(Notify::new()));
    let waker = Waker::from(wake.clone());
    let pending = request
        .as_mut()
        .poll(&mut Context::from_waker(&waker))
        .is_pending();
    // The project snapshot and connector are polled by the independent owner, not
    // this HTTP waker. Its wake witnesses answer delivery, not reservation release.
    let delivered = tokio::time::timeout(WAIT, wake.0.notified()).await.is_ok();
    let reserved = f.state.bus_session.try_lock().is_err();
    let disconnected_before_drop = f.handle.disconnected();
    drop(request);
    // Taking the reservation joins independent cleanup after discarding the answer.
    let session = tokio::time::timeout(WAIT, f.state.bus_session.lock())
        .await
        .unwrap()
        .take();
    let leaked = session.is_some();
    let disconnected_after_drop = f.handle.disconnected();
    // Also stop and join a leaked fixture on pre-fix production, before asserting RED.
    if let Some(session) = session {
        session.stop().await;
    }
    assert!(
        pending,
        "{target:?}: the HTTP future did not remain pending"
    );
    assert!(
        delivered,
        "{target:?}: no answer-delivery wake was observed"
    );
    assert_eq!(*f.calls.lock().unwrap(), vec![target]);
    assert!(
        !leaked,
        "{target:?}: pending HTTP future dropped after successful delivery leaked a session"
    );
    assert!(
        disconnected_after_drop,
        "{target:?}: the unclaimed fake tunnel was not disconnected"
    );
    assert!(
        !disconnected_before_drop,
        "{target:?}: no live delivered session"
    );
    assert!(
        reserved,
        "{target:?}: custody ended before success was consumed"
    );
}

#[tokio::test(flavor = "current_thread")]
async fn dropped_delivered_udp_monitor_start_cleans_up() {
    dropped_delivered_monitor_start(Target::Udp).await;
}

#[tokio::test(flavor = "current_thread")]
async fn dropped_delivered_tcp_monitor_start_cleans_up() {
    dropped_delivered_monitor_start(Target::Tcp).await;
}

#[tokio::test(flavor = "current_thread")]
async fn dropped_delivered_serial_monitor_start_cleans_up() {
    dropped_delivered_monitor_start(Target::Serial).await;
}

#[tokio::test(flavor = "current_thread")]
async fn dropped_monitor_starts_keep_custody_through_context_connect_and_cleanup() {
    for target in [Target::Udp, Target::Tcp, Target::Serial] {
        for drop_during_context in [true, false] {
            let f = fixture(target, 0);
            let mut held = HeldProject::new(&f.state);
            let app = f.app.clone();
            let request =
                tokio::spawn(async move { call(app, "POST", target.path(), target.body()).await });
            let pending = timer_while_pending(&request).await;
            if drop_during_context {
                request.abort();
            }
            let runtime_free = held.release();
            tokio::time::timeout(WAIT, f.entered.notified())
                .await
                .unwrap();
            if !drop_during_context {
                request.abort();
            }
            let _ = request.await;
            let reserved = f.state.bus_session.try_lock().is_err();
            busy(
                &f,
                "POST",
                target.path(),
                target.body(),
                "busMonitorOrGroupWrite",
            )
            .await;
            f.permit.add_permits(1);
            // Obtaining the reservation joins the owner through its disconnect, not just connect.
            let guard = tokio::time::timeout(WAIT, f.state.bus_session.lock())
                .await
                .unwrap();
            assert!(guard.is_none(), "an aborted start leaked a session");
            assert!(
                f.handle.disconnected(),
                "the unclaimed fake tunnel was not disconnected"
            );
            assert!(runtime_free, "{target:?} blocked the runtime");
            assert!(
                pending,
                "{target:?} answered before reaching the project lock"
            );
            assert!(reserved, "{target:?} released custody on client drop");
            assert_eq!(*f.calls.lock().unwrap(), vec![target]);
        }
    }
}
