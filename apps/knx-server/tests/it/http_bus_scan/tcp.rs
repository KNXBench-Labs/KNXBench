//! Verify strict scan identity, cleanup ownership and real HTTP-to-TCP descriptor scanning.

use super::{body_json, call, poll_until_terminal, scan_request};
use axum::{http::StatusCode, Router};
use knx_net::{ApplicationService, BusError, Destination, Tpci, TunnelEvent, TunnelTransport};
use knx_server::fake::{FakeConnector, FakeTunnel};
use knx_server::{AppState, BusSessionError, BusTunnel, GatewayConnector};
use serde_json::{json, Value};
use std::{
    collections::VecDeque,
    future::Future,
    net::SocketAddrV4,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::{Notify, Semaphore},
};

const START: &str = "/api/bus/scan/start-tcp";
const WAIT: Duration = Duration::from_secs(5);
const CHANNEL: u8 = 0x31;
const CONNECT: &[u8] = &[
    6, 16, 2, 5, 0, 26, 8, 2, 0, 0, 0, 0, 0, 0, 8, 2, 0, 0, 0, 0, 0, 0, 4, 4, 2, 0,
];
const CONNECTED: &[u8] = &[
    6, 16, 2, 6, 0, 20, CHANNEL, 0, 8, 2, 0, 0, 0, 0, 0, 0, 4, 4, 0x11, 0xfa,
];

type TunnelFuture<'a> =
    Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + 'a>>;
struct Connector {
    tunnels: Mutex<VecDeque<Box<dyn BusTunnel>>>,
    calls: Arc<AtomicUsize>,
    entered: Arc<Notify>,
    permit: Arc<Semaphore>,
}
impl GatewayConnector for Connector {
    fn connect_tunnel(&self, _: SocketAddrV4) -> TunnelFuture<'_> {
        panic!("unexpected UDP fallback")
    }
    fn connect_tunnel_with_transport(
        &self,
        _: SocketAddrV4,
        transport: TunnelTransport,
    ) -> TunnelFuture<'_> {
        assert_eq!(transport, TunnelTransport::Tcp);
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(async move {
            self.entered.notify_one();
            self.permit.acquire().await.unwrap().forget();
            Ok(self
                .tunnels
                .lock()
                .unwrap()
                .pop_front()
                .expect("explicit scripted connect"))
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
        panic!("scan must not discover")
    }
}
fn app_with(
    tunnels: Vec<Box<dyn BusTunnel>>,
    permits: usize,
) -> (
    Arc<AppState>,
    Router,
    Arc<AtomicUsize>,
    Arc<Notify>,
    Arc<Semaphore>,
) {
    let calls = Arc::new(AtomicUsize::new(0));
    let entered = Arc::new(Notify::new());
    let permit = Arc::new(Semaphore::new(permits));
    let state = Arc::new(AppState {
        connector: Box::new(Connector {
            tunnels: Mutex::new(tunnels.into()),
            calls: calls.clone(),
            entered: entered.clone(),
            permit: permit.clone(),
        }),
        ..Default::default()
    });
    (
        state.clone(),
        knx_server::app(state, None),
        calls,
        entered,
        permit,
    )
}
fn tcp_fake() -> (FakeTunnel, knx_server::fake::FakeTunnelHandle) {
    let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 64);
    (tunnel.with_transport(TunnelTransport::Tcp), handle)
}
fn bound_path(action: &str, start: &Value) -> String {
    format!(
        "/api/bus/scan/{action}?sessionId={}&serverIncarnation={}",
        start["sessionId"],
        start["serverIncarnation"].as_str().unwrap()
    )
}
async fn started(app: &Router) -> Value {
    let response = call(
        app,
        "POST",
        START,
        Some(scan_request(5, 7, &["1.1.6"], 60000)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}
async fn stopped(app: &Router, start: &Value) -> Value {
    let response = call(app, "POST", &bound_path("cancel-bound", start), None).await;
    assert_eq!(response.status(), StatusCode::OK);
    body_json(response).await
}

#[tokio::test]
async fn strict_tcp_admission_refuses_unknown_missing_duplicate_and_invalid_fields_before_connect()
{
    let (tunnel, _) = tcp_fake();
    let (_, app, calls, _, _) = app_with(vec![Box::new(tunnel)], 1);
    for field in [
        "gateway",
        "area",
        "line",
        "firstDevice",
        "lastDevice",
        "excluded",
        "responseTimeoutMs",
        "interProbePauseMs",
    ] {
        let mut body = scan_request(5, 7, &[], 10);
        body.as_object_mut().unwrap().remove(field);
        assert_eq!(
            call(&app, "POST", START, Some(body)).await.status(),
            StatusCode::BAD_REQUEST,
            "{field}"
        );
    }
    for gateway in [
        "0.0.0.0:3671",
        "224.0.23.12:3671",
        "255.255.255.255:3671",
        "192.0.2.1:0",
        "[::1]:3671",
        " 192.0.2.1:3671",
    ] {
        let mut body = scan_request(5, 7, &[], 10);
        body["gateway"] = json!(gateway);
        assert_eq!(
            call(&app, "POST", START, Some(body)).await.status(),
            StatusCode::BAD_REQUEST
        );
    }
    let mut unknown = scan_request(5, 7, &[], 10);
    unknown["transport"] = json!("tcp");
    assert_eq!(
        call(&app, "POST", START, Some(unknown)).await.status(),
        StatusCode::BAD_REQUEST
    );
    use tower::ServiceExt;
    let body = scan_request(5, 7, &[], 10)
        .to_string()
        .replacen('{', "{\"area\":1,", 1);
    let response = app
        .oneshot(
            axum::http::Request::builder()
                .method("POST")
                .uri(START)
                .header("content-type", "application/json")
                .body(axum::body::Body::from(body))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn unsupported_tcp_never_calls_udp_and_actual_udp_answer_is_disconnected_without_probes() {
    let (tunnel, handle) = tcp_fake();
    let state = Arc::new(AppState {
        connector: Box::new(FakeConnector::succeeding(tunnel)),
        ..Default::default()
    });
    let app = knx_server::app(state, None);
    assert_eq!(
        call(&app, "POST", START, Some(scan_request(5, 7, &[], 10)))
            .await
            .status(),
        StatusCode::BAD_GATEWAY
    );
    assert!(handle.sent_frames().is_empty());
    assert!(!handle.disconnected());
    let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 8);
    let (_, app, calls, _, _) = app_with(vec![Box::new(tunnel)], 1);
    assert_eq!(
        call(&app, "POST", START, Some(scan_request(5, 7, &[], 10)))
            .await
            .status(),
        StatusCode::BAD_GATEWAY
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(handle.disconnected());
    assert!(handle.sent_frames().is_empty());
}

#[tokio::test]
async fn bound_effects_reject_reused_ids_from_another_process_and_legacy_tcp_mutations() {
    let (a, _) = tcp_fake();
    let (_, app_a, _, _, _) = app_with(vec![Box::new(a)], 1);
    let (b, handle_b) = tcp_fake();
    let (_, app_b, _, _, _) = app_with(vec![Box::new(b)], 1);
    let a = started(&app_a).await;
    let b = started(&app_b).await;
    assert_eq!(a["sessionId"], b["sessionId"]);
    assert_ne!(a["serverIncarnation"], b["serverIncarnation"]);
    for (method, action, body) in [
        ("POST", "cancel-bound", None),
        ("GET", "comparison-bound", None),
        (
            "POST",
            "reconcile-bound",
            Some(json!({"unexpected":[],"missing":[]})),
        ),
    ] {
        let response = call(&app_b, method, &bound_path(action, &a), body).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(body_json(response).await["kind"], "lineScanSessionChanged");
    }
    let poll = format!("{}&since=0", bound_path("results", &a));
    assert_eq!(
        call(&app_b, "GET", &poll, None).await.status(),
        StatusCode::CONFLICT
    );
    for (method, path, body) in [
        ("POST", "/api/bus/scan/cancel?sessionId=1", None),
        (
            "POST",
            "/api/bus/scan/reconcile",
            Some(json!({"sessionId":1})),
        ),
        ("GET", "/api/bus/scan/comparison?sessionId=1", None),
    ] {
        let response = call(&app_b, method, path, body).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(
            body_json(response).await["kind"],
            "lineScanSessionBindingRequired"
        );
    }
    for query in [
        "",
        "sessionId=01&serverIncarnation=x",
        "sessionId=1&sessionId=1&serverIncarnation=x",
        "sessionId=1&serverIncarnation=x&extra=1",
        "sessionId=1&serverIncarnation=x&",
        "sessionId=1",
        "sessionId=1&serverIncarnation=%78",
    ] {
        assert_eq!(
            call(
                &app_b,
                "POST",
                &format!("/api/bus/scan/cancel-bound?{query}"),
                None
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    assert!(!handle_b.disconnected());
    for (app, start) in [(&app_a, &a), (&app_b, &b)] {
        let cancelled = stopped(app, start).await;
        assert_eq!(cancelled["transport"], "tcp");
        assert_eq!(cancelled["skippedFrames"], 0);
        assert_eq!(cancelled["disconnectConfirmed"], true);
        assert!(cancelled["cleanupError"].is_null());
    }
}

struct CleanupTunnel {
    inner: FakeTunnel,
    entered: Arc<Notify>,
    release: Arc<Semaphore>,
    fail: bool,
    panic: bool,
}
impl BusTunnel for CleanupTunnel {
    fn transport(&self) -> TunnelTransport {
        TunnelTransport::Tcp
    }
    fn skipped_frames(&self) -> u64 {
        self.inner.skipped_frames()
    }
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        self.inner.assigned_address()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TunnelEvent> {
        self.inner.subscribe()
    }
    fn send(
        &self,
        d: Destination,
        s: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        self.inner.send(d, s)
    }
    fn send_frame(
        &self,
        d: Destination,
        t: Tpci,
        s: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusError>> + Send + '_>> {
        self.inner.send_frame(d, t, s)
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async move {
            self.entered.notify_one();
            self.release.acquire().await.unwrap().forget();
            Box::new(self.inner).disconnect().await?;
            assert!(
                !self.panic,
                "scripted cleanup panic after synthetic release"
            );
            if self.fail {
                Err(BusSessionError::Transport(BusError::Protocol(
                    "scripted cleanup uncertainty".into(),
                )))
            } else {
                Ok(())
            }
        })
    }
}

#[tokio::test]
async fn dropped_cancel_keeps_join_and_reservation_through_cleanup_and_retains_frozen_skips() {
    for (fail, panic) in [(false, false), (true, false), (false, true)] {
        let (inner, handle) = tcp_fake();
        let entered = Arc::new(Notify::new());
        let release = Arc::new(Semaphore::new(0));
        let (replacement, replacement_handle) = tcp_fake();
        let (state, app, calls, _, _) = app_with(
            vec![
                Box::new(CleanupTunnel {
                    inner,
                    entered: entered.clone(),
                    release: release.clone(),
                    fail,
                    panic,
                }),
                Box::new(replacement),
            ],
            2,
        );
        let start = started(&app).await;
        handle.set_skipped_frames(3);
        let cancel_app = app.clone();
        let uri = bound_path("cancel-bound", &start);
        let request = tokio::spawn(async move { call(&cancel_app, "POST", &uri, None).await });
        tokio::time::timeout(WAIT, entered.notified())
            .await
            .unwrap();
        request.abort();
        let _ = request.await;
        assert!(state.line_scan_session.try_lock().is_err());
        assert!(!handle.disconnected());
        for uri in [
            START.to_owned(),
            bound_path("cancel-bound", &start),
            "/api/bus/scan/start".into(),
            "/api/bus/scan/cancel?sessionId=1".into(),
        ] {
            let response = tokio::time::timeout(
                WAIT,
                call(&app, "POST", &uri, Some(scan_request(5, 7, &[], 10))),
            )
            .await
            .unwrap();
            assert_eq!(response.status(), StatusCode::CONFLICT);
            assert_eq!(body_json(response).await["kind"], "gatewayBusy");
        }
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        handle.set_skipped_frames(9);
        release.add_permits(1);
        tokio::time::timeout(WAIT, async {
            loop {
                if state.line_scan_session.try_lock().is_ok() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        let final_ =
            body_json(call(&app, "GET", "/api/bus/scan/results?since=0", None).await).await;
        assert_eq!(final_["status"], "cancelled");
        assert_eq!(final_["skippedFrames"], 3);
        assert_eq!(final_["disconnectConfirmed"], !fail && !panic);
        assert_eq!(final_["cleanupError"].is_string(), fail || panic);
        assert!(handle.disconnected());
        let next = started(&app).await;
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        let stale = call(&app, "POST", &bound_path("cancel-bound", &start), None).await;
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        assert!(!replacement_handle.disconnected());
        stopped(&app, &next).await;
    }
}

#[tokio::test]
async fn dropped_start_and_shutdown_during_pending_connect_dispose_the_late_tunnel_under_reservation(
) {
    for shutdown in [false, true] {
        let (tunnel, handle) = tcp_fake();
        let (state, app, calls, entered, permit) = app_with(vec![Box::new(tunnel)], 0);
        let pending_app = app.clone();
        let request = tokio::spawn(async move {
            call(
                &pending_app,
                "POST",
                START,
                Some(scan_request(5, 7, &[], 60000)),
            )
            .await
        });
        tokio::time::timeout(WAIT, entered.notified())
            .await
            .unwrap();
        request.abort();
        let _ = request.await;
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(state.line_scan_session.try_lock().is_err());
        let response = call(&app, "POST", START, Some(scan_request(5, 7, &[], 10))).await;
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert_eq!(body_json(response).await["kind"], "gatewayBusy");
        if shutdown {
            state.stop_admission.close();
        }
        permit.add_permits(1);
        tokio::time::timeout(WAIT, async {
            loop {
                if handle.disconnected() && state.line_scan_session.try_lock().is_ok() {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        let final_ =
            body_json(call(&app, "GET", "/api/bus/scan/results?since=0", None).await).await;
        assert_eq!(final_["status"], "cancelled");
        assert_eq!(final_["disconnectConfirmed"], true);
    }
}

async fn frame(stream: &mut TcpStream) -> Vec<u8> {
    let mut header = [0; 6];
    tokio::time::timeout(WAIT, stream.read_exact(&mut header))
        .await
        .unwrap()
        .unwrap();
    let n = u16::from_be_bytes([header[4], header[5]]) as usize;
    assert!((6..=2048).contains(&n));
    let mut bytes = header.to_vec();
    bytes.resize(n, 0);
    tokio::time::timeout(WAIT, stream.read_exact(&mut bytes[6..]))
        .await
        .unwrap()
        .unwrap();
    bytes
}
fn assert_probe(bytes: &[u8], device: u8, tpci: Tpci) {
    assert_eq!(&bytes[..4], &[6, 16, 4, 32]);
    assert_eq!(&bytes[6..8], &[4, CHANNEL]);
    let cemi = knx_net::cemi::decode_l_data(&bytes[10..]).unwrap();
    assert_eq!(cemi.kind, knx_net::LDataMessageKind::Request);
    assert_eq!(cemi.source.to_string(), "0.0.0");
    assert_eq!(
        cemi.destination,
        Destination::Individual(format!("1.1.{device}").parse().unwrap())
    );
    assert_eq!(cemi.transport, tpci);
    assert_eq!(
        cemi.service,
        if matches!(tpci, Tpci::NumberedData { .. }) {
            ApplicationService::DeviceDescriptorRead { descriptor_type: 0 }
        } else {
            ApplicationService::NoApplicationPdu
        }
    );
}
fn descriptor(device: u8) -> Vec<u8> {
    vec![
        6, 16, 4, 32, 0, 23, 4, CHANNEL, 7, 0, 0x29, 0, 0xbc, 0x60, 0x11, device, 0x11, 0xfa, 3,
        0x43, 0x40, 7, 0xb0,
    ]
}
async fn disconnect(stream: &mut TcpStream) {
    let bytes = frame(stream).await;
    assert_eq!(&bytes[2..4], &[2, 9]);
    assert_eq!(bytes[6], CHANNEL);
    stream
        .write_all(&[6, 16, 2, 10, 0, 8, CHANNEL, 0])
        .await
        .unwrap();
    let mut byte = [0];
    assert_eq!(
        tokio::time::timeout(WAIT, stream.read(&mut byte))
            .await
            .unwrap()
            .unwrap(),
        0,
        "observe EOF, not merely disconnect submission"
    );
}
async fn http(
    address: std::net::SocketAddr,
    method: &str,
    path: &str,
    body: Option<Value>,
) -> (u16, Value) {
    let body = body.map(|b| b.to_string()).unwrap_or_default();
    let mut stream = TcpStream::connect(address).await.unwrap();
    stream.write_all(format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).as_bytes()).await.unwrap();
    let mut bytes = Vec::new();
    tokio::time::timeout(WAIT, stream.read_to_end(&mut bytes))
        .await
        .unwrap()
        .unwrap();
    let split = bytes.windows(4).position(|s| s == b"\r\n\r\n").unwrap();
    let status = std::str::from_utf8(&bytes[..split])
        .unwrap()
        .split_whitespace()
        .nth(1)
        .unwrap()
        .parse()
        .unwrap();
    (status, serde_json::from_slice(&bytes[split + 4..]).unwrap())
}

#[tokio::test]
async fn real_http_recorded_tcp_dd0_exclusions_live_skips_cancel_and_confirmed_cleanup() {
    for cancel in [false, true] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway = listener.local_addr().unwrap();
        let trap = UdpSocket::bind(gateway).await.unwrap();
        let release = Arc::new(Notify::new());
        let ready = Arc::new(Notify::new());
        let peer_release = release.clone();
        let peer_ready = ready.clone();
        let peer = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            assert_eq!(frame(&mut stream).await, CONNECT);
            stream.write_all(CONNECTED).await.unwrap();
            for device in [5, 7] {
                assert_probe(&frame(&mut stream).await, device, Tpci::Connect);
                assert_probe(
                    &frame(&mut stream).await,
                    device,
                    Tpci::NumberedData { seq: 0 },
                );
                if device == 7 {
                    stream
                        .write_all(&[6, 16, 9, 0x99, 0, 10, 0xd3, 0xa5, 0x7c, 0x91])
                        .await
                        .unwrap();
                    let mut oversize = vec![6, 16, 4, 32, 5, 0xdc, 4, CHANNEL, 0, 0];
                    oversize.resize(1500, 0xee);
                    stream.write_all(&oversize).await.unwrap();
                    peer_ready.notify_one();
                    peer_release.notified().await;
                    if cancel {
                        disconnect(&mut stream).await;
                        return;
                    }
                }
                stream.write_all(&descriptor(device)).await.unwrap();
                assert_probe(&frame(&mut stream).await, device, Tpci::Disconnect);
            }
            disconnect(&mut stream).await;
        });
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(AppState {
            recordings: Arc::new(knx_server::operation_recordings::store(dir.path(), false)),
            data_dir: dir.path().to_path_buf(),
            connector: Box::new(knx_server::RealConnector::default()),
            ..Default::default()
        });
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let (shutdown, done) = tokio::sync::oneshot::channel::<()>();
        let app = knx_server::app(state.clone(), None);
        let server = tokio::spawn(async move {
            axum::serve(listener, app)
                .with_graceful_shutdown(async {
                    let _ = done.await;
                })
                .await
                .unwrap();
        });
        let mut body = scan_request(5, 7, &["1.1.6"], 2000);
        body["gateway"] = json!(gateway.to_string());
        let (status, start) = http(address, "POST", START, Some(body)).await;
        assert_eq!(status, 200, "{start}");
        assert_eq!(start["transport"], "tcp");
        assert!(start["skippedFrames"].is_u64());
        tokio::time::timeout(WAIT, ready.notified()).await.unwrap();
        let uri = format!("{}&since=0", bound_path("results", &start));
        let live = tokio::time::timeout(WAIT, async {
            loop {
                let (status, poll) = http(address, "GET", &uri, None).await;
                assert_eq!(status, 200);
                if poll["skippedFrames"] == 2 {
                    break poll;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
        assert_eq!(live["status"], "running");
        assert!(live["disconnectConfirmed"].is_null());
        assert!(live["cleanupError"].is_null());
        assert_eq!(live["completedCount"], 1);
        assert_eq!(
            live["results"],
            json!([{"address":"1.1.5","outcome":{"kind":"occupied","maskVersion":0x07b0}}])
        );
        release.notify_one();
        let final_ = if cancel {
            let (status, cancelled) =
                http(address, "POST", &bound_path("cancel-bound", &start), None).await;
            assert_eq!(status, 200);
            cancelled
        } else {
            tokio::time::timeout(WAIT, async {
                loop {
                    let (_, poll) = http(address, "GET", &uri, None).await;
                    if poll["status"] != "running" {
                        break poll;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .unwrap()
        };
        peer.await.unwrap();
        assert_eq!(final_["sessionId"], start["sessionId"]);
        assert_eq!(final_["serverIncarnation"], start["serverIncarnation"]);
        assert_eq!(final_["transport"], "tcp");
        assert_eq!(final_["skippedFrames"], 2);
        assert_eq!(final_["disconnectConfirmed"], true);
        assert!(final_["cleanupError"].is_null());
        assert_eq!(
            final_["status"],
            if cancel { "cancelled" } else { "completed" }
        );
        assert_eq!(final_["totalCount"], 2);
        assert_eq!(final_["excludedAddresses"], json!(["1.1.6", "1.1.220"]));
        assert_eq!(final_["completedCount"], if cancel { 1 } else { 2 });
        if !cancel {
            assert_eq!(
                final_["results"],
                json!([{"address":"1.1.5","outcome":{"kind":"occupied","maskVersion":0x07b0}},{"address":"1.1.7","outcome":{"kind":"occupied","maskVersion":0x07b0}}])
            );
        }
        let mut datagram = [0; 2048];
        assert!(
            tokio::time::timeout(Duration::from_millis(30), trap.recv(&mut datagram))
                .await
                .is_err(),
            "TCP must never fall back to UDP"
        );
        if let Some(root) = std::env::var_os("KNXBENCH_SCAN_CAPTURE_DIR") {
            let root = std::path::PathBuf::from(root);
            std::fs::create_dir_all(&root).unwrap();
            std::fs::write(
                root.join(if cancel {
                    "actual-backend.json"
                } else {
                    "completed-backend.json"
                }),
                serde_json::to_vec_pretty(&json!({"start":start,"poll":live,"cancel":final_}))
                    .unwrap(),
            )
            .unwrap();
        }
        assert!(state.recordings.wait_until_idle(WAIT));
        let recordings = state.recordings.list().unwrap();
        assert_eq!(recordings.len(), 1);
        assert_eq!(recordings[0].operation_kind.as_deref(), Some("line-scan"));
        let recording =
            String::from_utf8(state.recordings.read(&recordings[0].id).unwrap()).unwrap();
        assert!(recording.contains("CONNECT_REQUEST"));
        assert!(recording.contains("TUNNELLING_REQUEST"));
        assert!(
            !recording.contains("d3a57c91"),
            "short unknown skipped body must not be exported"
        );
        assert!(
            !recording.contains(&"ee".repeat(100)),
            "over-cap skipped body must not be exported"
        );
        let lines: Vec<Value> = recording
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect();
        let skipped: Vec<&Value> = lines
            .iter()
            .filter(|line| line["event"] == "frame-skipped")
            .collect();
        assert_eq!(skipped.len(), 2, "both skipped frames must leave metadata");
        for (event, (service_type, total_length, reason)) in skipped.iter().zip([
            ("0x0999", 10, "unhandled-service"),
            ("0x0420", 1500, "above-local-cap"),
        ]) {
            // Only bounded header facts and recorder stamps, never raw/body payload fields.
            let fields = event.as_object().unwrap();
            assert_eq!(fields.len(), 8, "skipped event must contain metadata only");
            assert!(fields.keys().all(|key| matches!(
                key.as_str(),
                "type"
                    | "seq"
                    | "t_us"
                    | "wall"
                    | "event"
                    | "service_type"
                    | "total_length"
                    | "reason"
            )));
            assert_eq!(event["type"], "tunnel");
            assert!(event["seq"].is_u64());
            assert!(event["t_us"].is_u64());
            let wall = event["wall"].as_str().unwrap();
            assert!(!wall.is_empty() && wall.len() <= 32);
            assert_eq!(event["service_type"], service_type);
            assert_eq!(event["total_length"], total_length);
            assert_eq!(event["reason"], reason);
        }
        let end = lines.last().unwrap();
        assert_eq!(end["type"], "end");
        assert_eq!(end["summary"]["skippedFrames"], 2);
        assert_eq!(end["summary"]["disconnectConfirmed"], true);
        shutdown.send(()).unwrap();
        server.await.unwrap();
    }
}

#[tokio::test]
async fn real_tcp_silence_is_vacant_confirmation_is_silent_and_bad_peers_retain_skips() {
    for mode in ["silence", "confirmation", "eof", "malformed"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway = listener.local_addr().unwrap();
        let trap = UdpSocket::bind(gateway).await.unwrap();
        let peer = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            assert_eq!(frame(&mut stream).await, CONNECT);
            stream.write_all(CONNECTED).await.unwrap();
            let mut connect = frame(&mut stream).await;
            assert_probe(&connect, 5, Tpci::Connect);
            assert_probe(&frame(&mut stream).await, 5, Tpci::NumberedData { seq: 0 });
            if mode == "confirmation" {
                connect[10] = 0x2e;
                connect[12] &= !1;
                connect[14..16].copy_from_slice(&[0x11, 0xfa]);
                stream.write_all(&connect).await.unwrap();
            }
            if mode == "eof" || mode == "malformed" {
                stream
                    .write_all(&[6, 16, 9, 0x99, 0, 10, 0xee, 0xee, 0xee, 0xee])
                    .await
                    .unwrap();
                if mode == "malformed" {
                    stream.write_all(&[6, 16, 4, 32, 0, 5]).await.unwrap();
                    let mut byte = [0];
                    assert_eq!(
                        tokio::time::timeout(WAIT, stream.read(&mut byte))
                            .await
                            .unwrap()
                            .unwrap(),
                        0
                    );
                }
                return;
            }
            assert_probe(&frame(&mut stream).await, 5, Tpci::Disconnect);
            disconnect(&mut stream).await;
        });
        let app = knx_server::app(Arc::new(AppState::default()), None);
        let mut body = scan_request(5, 5, &[], 150);
        body["gateway"] = json!(gateway.to_string());
        let response = call(&app, "POST", START, Some(body)).await;
        assert_eq!(response.status(), StatusCode::OK);
        let terminal = tokio::time::timeout(WAIT, poll_until_terminal(&app))
            .await
            .unwrap();
        peer.await.unwrap();
        if mode == "silence" || mode == "confirmation" {
            assert_eq!(terminal["status"], "completed");
            assert_eq!(terminal["skippedFrames"], 0);
            assert_eq!(terminal["disconnectConfirmed"], true);
            assert_eq!(
                terminal["results"][0]["outcome"]["kind"],
                if mode == "silence" {
                    "vacant"
                } else {
                    "occupiedSilent"
                }
            );
        } else {
            assert_eq!(terminal["status"], "failed");
            assert!(terminal["error"].is_string());
            assert_eq!(terminal["skippedFrames"], 1);
            assert_eq!(terminal["results"], json!([]));
            assert!(terminal["disconnectConfirmed"].is_boolean());
        }
        let mut bytes = [0; 256];
        assert!(
            tokio::time::timeout(Duration::from_millis(30), trap.recv(&mut bytes))
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn real_tcp_refusal_handshake_eof_and_connect_refused_never_fall_back_to_udp() {
    for mode in ["refused", "eof", "no-listener"] {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let gateway = listener.local_addr().unwrap();
        let trap = UdpSocket::bind(gateway).await.unwrap();
        let peer = if mode == "no-listener" {
            drop(listener);
            None
        } else {
            Some(tokio::spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                assert_eq!(frame(&mut stream).await, CONNECT);
                if mode == "refused" {
                    stream
                        .write_all(&[6, 16, 2, 6, 0, 8, 0, 0x24])
                        .await
                        .unwrap();
                    let mut byte = [0];
                    assert_eq!(
                        tokio::time::timeout(WAIT, stream.read(&mut byte))
                            .await
                            .unwrap()
                            .unwrap(),
                        0
                    );
                }
            }))
        };
        let state = Arc::new(AppState::default());
        let app = knx_server::app(state.clone(), None);
        let mut body = scan_request(5, 5, &[], 100);
        body["gateway"] = json!(gateway.to_string());
        let response = tokio::time::timeout(WAIT, call(&app, "POST", START, Some(body)))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_GATEWAY);
        if let Some(peer) = peer {
            peer.await.unwrap();
        }
        assert!(state.line_scan_session.try_lock().unwrap().is_none());
        let mut bytes = [0; 256];
        assert!(
            tokio::time::timeout(Duration::from_millis(30), trap.recv(&mut bytes))
                .await
                .is_err()
        );
    }
}

#[tokio::test]
async fn bound_tcp_comparison_reconciliation_and_undo_preserve_existing_project_commands() {
    let (tunnel, handle) = tcp_fake();
    let (unused, _) = tcp_fake();
    let mut state = super::state_with_project(unused);
    state.connector = Box::new(FakeConnector::succeeding(tunnel).accepting_tcp());
    let state = Arc::new(state);
    let before = state.project.lock().unwrap().as_ref().unwrap().clone();
    let app = knx_server::app(state.clone(), None);
    let response = call(&app, "POST", START, Some(scan_request(2, 3, &["1.1.3"], 1))).await;
    assert_eq!(response.status(), StatusCode::OK);
    let start = body_json(response).await;
    assert_eq!(poll_until_terminal(&app).await["status"], "completed");
    let frames = handle.sent_frames();
    let comparison = call(&app, "GET", &bound_path("comparison-bound", &start), None).await;
    assert_eq!(comparison.status(), StatusCode::OK);
    assert_eq!(
        body_json(comparison).await,
        json!({"unexpected":[],"missing":["1.1.2"],"excludedInProject":["1.1.3"]})
    );
    for body in [
        json!({"unexpected":[],"missing":[],"sessionId":1}),
        json!({"unexpected":[]}),
    ] {
        assert_eq!(
            call(
                &app,
                "POST",
                &bound_path("reconcile-bound", &start),
                Some(body)
            )
            .await
            .status(),
            StatusCode::BAD_REQUEST
        );
        assert!(!state.command_stack.lock().unwrap().can_undo());
    }
    let response = call(
        &app,
        "POST",
        &bound_path("reconcile-bound", &start),
        Some(json!({"unexpected":[],"missing":["1.1.2"]})),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(body_json(response).await["can_undo"], true);
    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .devices
        .iter()
        .all(|device| device.address != Some(super::addr(2))));
    assert_eq!(
        handle.sent_frames(),
        frames,
        "project reconciliation must not send bus traffic"
    );
    assert_eq!(
        call(&app, "POST", "/api/undo", None).await.status(),
        StatusCode::OK
    );
    assert!(state
        .project
        .lock()
        .unwrap()
        .as_ref()
        .unwrap()
        .same_user_content_as(&before));
}

#[tokio::test]
async fn legacy_udp_keeps_null_skips_and_discovery_without_query_fields() {
    let (tunnel, handle) = super::fake_tunnel();
    let app = knx_server::app(Arc::new(super::state_with(tunnel)), None);
    let response = call(
        &app,
        "POST",
        "/api/bus/scan/start",
        Some(scan_request(9, 9, &[], 1)),
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let start = body_json(response).await;
    assert_eq!(start["transport"], "udp");
    assert!(start["skippedFrames"].is_null());
    assert!(start["serverIncarnation"].is_string());
    let terminal = poll_until_terminal(&app).await;
    assert_eq!(terminal["status"], "completed");
    assert!(terminal["skippedFrames"].is_null());
    assert_eq!(terminal["disconnectConfirmed"], true);
    assert_eq!(
        call(&app, "GET", "/api/bus/scan/results", None)
            .await
            .status(),
        StatusCode::OK
    );
    let response = call(&app, "POST", "/api/bus/scan/cancel?sessionId=1", None).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(body_json(response).await["skippedFrames"].is_null());
    assert!(handle.disconnected());
}

#[tokio::test]
async fn cleanup_is_running_until_disconnect_settles_even_after_probe_completion() {
    let (inner, handle) = tcp_fake();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Semaphore::new(0));
    let (state, app, _, _, _) = app_with(
        vec![Box::new(CleanupTunnel {
            inner,
            entered: entered.clone(),
            release: release.clone(),
            fail: true,
            panic: false,
        })],
        1,
    );
    let response = call(&app, "POST", START, Some(scan_request(250, 250, &[], 10))).await;
    assert_eq!(response.status(), StatusCode::OK);
    tokio::time::timeout(WAIT, entered.notified())
        .await
        .unwrap();
    let live = body_json(call(&app, "GET", "/api/bus/scan/results?since=0", None).await).await;
    assert_eq!(live["status"], "running");
    assert_eq!(live["results"][0]["outcome"]["kind"], "selfAddress");
    assert!(live["disconnectConfirmed"].is_null());
    assert_eq!(
        format!(
            "{:?}",
            state
                .line_scan_session
                .lock()
                .await
                .as_ref()
                .unwrap()
                .status()
        ),
        "Running"
    );
    assert!(handle.sent_frames().is_empty());
    release.add_permits(1);
    let terminal = poll_until_terminal(&app).await;
    assert_eq!(terminal["status"], "completed");
    assert!(terminal["error"].is_null());
    assert_eq!(terminal["disconnectConfirmed"], false);
    assert!(terminal["cleanupError"].is_string());
    assert!(handle.disconnected());
}
