//! Verify the strict TCP monitor route, its shared session and a real loopback TCP gateway.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    Router,
};
use knx_net::{TunnelEvent, TunnelTransport};
use knx_server::{
    fake::{FakeConnector, FakeTunnel, FakeTunnelHandle},
    AppState, BusSessionError, BusTunnel, GatewayConnector,
};
use serde_json::{json, Value};
use std::{
    future::Future,
    net::SocketAddrV4,
    pin::Pin,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream, UdpSocket},
    sync::Notify,
};
use tower::ServiceExt;

const GATEWAY: &str = "192.0.2.10:3671";
const START_TCP: &str = "/api/bus/monitor/start-tcp";

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
fn tcp_tunnel() -> (FakeTunnel, FakeTunnelHandle) {
    let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 64);
    (tunnel.with_transport(TunnelTransport::Tcp), handle)
}
fn app_with(connector: impl GatewayConnector + 'static) -> (Arc<AppState>, Router) {
    let state = Arc::new(AppState {
        connector: Box::new(connector),
        ..Default::default()
    });
    let app = knx_server::app(state.clone(), None);
    (state, app)
}
fn binding(started: &Value, rest: Value) -> Value {
    let mut body = rest;
    body["sessionId"] = started["sessionId"].clone();
    body["serverIncarnation"] = started["serverIncarnation"].clone();
    body
}
fn stop_uri(started: &Value) -> String {
    format!(
        "/api/bus/monitor/stop-bound?sessionId={}&serverIncarnation={}",
        started["sessionId"],
        started["serverIncarnation"].as_str().unwrap()
    )
}

#[tokio::test]
async fn strict_shape_and_gateway_errors_never_reach_the_connector() {
    let (tunnel, _handle) = tcp_tunnel();
    let connector = Arc::new(FakeConnector::succeeding(tunnel).accepting_tcp());
    let (state, app) = app_with(Shared(connector.clone()));
    for body in [
        json!({}),
        json!({ "gateway": null }),
        json!({ "gateway": 3671 }),
        json!({ "gateway": GATEWAY, "transport": "udp" }),
        json!({ "gateway": GATEWAY, "unknown": true }),
        json!({ "gateway": "nonsense" }),
        json!({ "gateway": " 192.0.2.10:3671" }),
        json!({ "gateway": "192.0.2.10" }),
        json!({ "gateway": "192.0.2.10:0" }),
        json!({ "gateway": "0.0.0.0:3671" }),
        json!({ "gateway": "224.0.23.12:3671" }),
        json!({ "gateway": "255.255.255.255:3671" }),
        json!({ "gateway": "[::1]:3671" }),
    ] {
        let (status, answer) = call(&app, "POST", START_TCP, body.clone()).await;
        assert_eq!(status, StatusCode::BAD_REQUEST, "{body}: {answer}");
    }
    for body in [
        "{",
        "[]",
        "null",
        r#"{"gateway":"192.0.2.10:3671","gateway":"192.0.2.11:3671"}"#,
    ] {
        assert_eq!(
            raw(&app, "POST", START_TCP, body).await.0,
            StatusCode::BAD_REQUEST,
            "{body}"
        );
    }
    assert_eq!(connector.call_count(), 0);
    assert_eq!(connector.tcp_call_count(), 0);
    assert!(state.bus_session.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn a_connector_without_tcp_refuses_and_opens_no_udp_tunnel() {
    let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 8);
    let connector = Arc::new(FakeConnector::succeeding(tunnel));
    let (state, app) = app_with(Shared(connector.clone()));
    let (status, body) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(
        body["error"]
            .as_str()
            .unwrap()
            .contains("TCP tunnelling is unavailable"),
        "{body}"
    );
    assert_eq!(connector.call_count(), 0, "the refusal fell back to UDP");
    assert_eq!(connector.tcp_call_count(), 0);
    assert!(state.bus_session.try_lock().unwrap().is_none());
    assert!(!handle.disconnected());
    // The scripted outcome is still unspent: the UDP route can use it.
    let (status, body) = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        json!({ "gateway": GATEWAY }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["transport"], "udp");
    assert_eq!(connector.call_count(), 1);
}

#[tokio::test]
async fn a_connector_that_answers_with_a_udp_tunnel_is_closed_and_refused() {
    let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 8);
    let connector = Arc::new(FakeConnector::succeeding(tunnel).accepting_tcp());
    let (state, app) = app_with(Shared(connector.clone()));
    let (status, body) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(
        body["error"].as_str().unwrap().contains("non-TCP"),
        "{body}"
    );
    assert_eq!(connector.tcp_call_count(), 1);
    assert!(handle.disconnected());
    assert!(state.bus_session.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn a_held_reservation_or_a_running_session_refuses_before_the_connector() {
    let (tunnel, _handle) = tcp_tunnel();
    let connector = Arc::new(FakeConnector::succeeding(tunnel).accepting_tcp());
    let (state, app) = app_with(Shared(connector.clone()));
    let held = state.bus_session.lock().await;
    let (status, body) = tokio::time::timeout(
        Duration::from_secs(1),
        call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })),
    )
    .await
    .expect("the TCP start queued behind a held reservation");
    drop(held);
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy");
    assert_eq!(
        body["holder"],
        json!({ "kind": "connecting", "session": "busMonitorOrGroupWrite" })
    );
    assert_eq!(connector.tcp_call_count(), 0);

    let (status, started) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let (status, body) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy");
    assert_eq!(
        body["holder"],
        json!({
            "kind": "busMonitor", "sessionId": started["sessionId"],
            "gateway": GATEWAY, "transport": "tcp", "state": "active"
        })
    );
    assert_eq!(connector.tcp_call_count(), 1);
    assert_eq!(connector.call_count(), 0);
}

#[tokio::test]
async fn one_tcp_session_serves_start_poll_activity_group_traffic_and_stop() {
    let (tunnel, handle) = tcp_tunnel();
    let connector = Arc::new(FakeConnector::succeeding(tunnel).accepting_tcp());
    let (state, app) = app_with(Shared(connector.clone()));
    let (status, started) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::OK, "{started}");
    assert_eq!(started["transport"], "tcp");
    assert_eq!(started["assignedAddress"], "1.1.250");
    assert!(started.get("serial").is_none());
    // A TCP session states its cumulative skipped-frame count from the start; 0 is a reading.
    assert_eq!(started["skippedFrames"], 0);

    let (_, poll) = call(
        &app,
        "GET",
        "/api/bus/monitor/telegrams?since=0",
        Value::Null,
    )
    .await;
    assert_eq!(poll["transport"], "tcp");
    assert_eq!(poll["skippedFrames"], 0);
    assert_eq!(poll["sessionId"], started["sessionId"]);
    assert_eq!(poll["serverIncarnation"], started["serverIncarnation"]);
    // The count is cumulative, separate from dropped rows, and follows the tunnel.
    handle.set_skipped_frames(3);
    let (_, poll) = call(
        &app,
        "GET",
        "/api/bus/monitor/telegrams?since=0",
        Value::Null,
    )
    .await;
    assert_eq!(poll["skippedFrames"], 3);
    assert_eq!(poll["droppedBefore"], 0);
    assert_eq!(poll["telegrams"], json!([]));
    handle.set_skipped_frames(4);
    let (_, activity) = call(&app, "GET", "/api/bus/activity", Value::Null).await;
    assert_eq!(
        activity["sessions"][0],
        json!({
            "kind": "busMonitor", "id": started["sessionId"],
            "gateway": GATEWAY, "transport": "tcp", "state": "active"
        })
    );

    // The unbound write cannot name its session; TCP never accepts it.
    let (status, body) = call(
        &app,
        "POST",
        "/api/bus/write",
        json!({ "destination": "1/1/1", "dpt": "DPST-1-1", "value": "on" }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "busSessionBindingRequired");
    assert!(handle.sent_calls().is_empty());

    // A wrong incarnation, a wrong id: refused, nothing sent, session kept.
    for bad in [
        json!({ "sessionId": started["sessionId"], "serverIncarnation": "other", "destination": "1/1/1" }),
        json!({ "sessionId": 999, "serverIncarnation": started["serverIncarnation"], "destination": "1/1/1" }),
    ] {
        let (status, body) = call(&app, "POST", "/api/bus/read", bad).await;
        assert_eq!(status, StatusCode::CONFLICT, "{body}");
        assert_eq!(body["kind"], "busSessionChanged");
    }
    assert!(handle.sent_calls().is_empty());

    let (status, body) = call(
        &app,
        "POST",
        "/api/bus/read",
        binding(&started, json!({ "destination": "1/1/1" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        body,
        json!({ "service": "GroupValueRead", "transport": "tcp" })
    );
    let (status, body) = call(
        &app,
        "POST",
        "/api/bus/write-bound",
        binding(
            &started,
            json!({ "destination": "1/1/1", "dpt": "DPST-1-1", "value": "on" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["transport"], "tcp");
    assert_eq!(body["encodedPayload"], "0x01 (6-bit)");
    assert_eq!(
        handle.sent_calls().len(),
        2,
        "both actions used the one tunnel"
    );
    assert_eq!(connector.tcp_call_count(), 1);

    // A stop naming another incarnation changes nothing.
    let wrong = format!(
        "/api/bus/monitor/stop-bound?sessionId={}&serverIncarnation=other",
        started["sessionId"]
    );
    assert_eq!(
        call(&app, "POST", &wrong, Value::Null).await.1["kind"],
        "busSessionChanged"
    );
    assert!(!handle.disconnected());

    let (status, stopped) = call(&app, "POST", &stop_uri(&started), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{stopped}");
    assert_eq!(stopped["sessionId"], started["sessionId"]);
    // Sampled just before the disconnect request, so it covers the latest count the tunnel had.
    assert_eq!(stopped["skippedFrames"], 4);
    assert_eq!(stopped["droppedCount"], 0);
    assert!(handle.disconnected());
    assert!(state.bus_session.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn the_legacy_udp_start_reports_udp_and_keeps_the_unbound_write() {
    let (tunnel, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 8);
    let connector = Arc::new(FakeConnector::succeeding(tunnel).accepting_tcp());
    let (_state, app) = app_with(Shared(connector.clone()));
    let (status, started) = call(
        &app,
        "POST",
        "/api/bus/monitor/start",
        json!({ "gateway": GATEWAY }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{started}");
    assert_eq!(started["transport"], "udp");
    assert!(started.get("skippedFrames").is_none());
    assert_eq!(connector.call_count(), 1);
    assert_eq!(connector.tcp_call_count(), 0);
    let (_, poll) = call(
        &app,
        "GET",
        "/api/bus/monitor/telegrams?since=0",
        Value::Null,
    )
    .await;
    assert_eq!(poll["transport"], "udp");
    let (status, body) = call(
        &app,
        "POST",
        "/api/bus/write",
        json!({ "destination": "1/1/1", "dpt": "DPST-1-1", "value": "on" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["transport"], "udp");
    assert_eq!(handle.sent_calls().len(), 1);
}

/// Lets a test keep the connector it injected to read its counters.
struct Shared(Arc<FakeConnector>);
impl GatewayConnector for Shared {
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.0.connect_tunnel(gateway)
    }
    fn connect_tunnel_with_transport(
        &self,
        gateway: SocketAddrV4,
        transport: TunnelTransport,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        self.0.connect_tunnel_with_transport(gateway, transport)
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
        self.0.discover()
    }
}

/// A TCP connect that waits for the test, to hold a reservation mid-connect.
struct Gated {
    entered: Arc<Notify>,
    release: Arc<Notify>,
    tunnel: Mutex<Option<Box<dyn BusTunnel>>>,
}
impl GatewayConnector for Gated {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        panic!("an explicit TCP request must not open UDP")
    }
    fn connect_tunnel_with_transport(
        &self,
        _: SocketAddrV4,
        _: TunnelTransport,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(self.tunnel.lock().unwrap().take().unwrap())
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
        panic!("TCP monitoring must not discover gateways")
    }
}

#[tokio::test]
async fn a_dropped_start_keeps_the_reservation_until_the_late_tunnel_is_released() {
    let (tunnel, handle) = tcp_tunnel();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let (state, app) = app_with(Gated {
        entered: entered.clone(),
        release: release.clone(),
        tunnel: Mutex::new(Some(Box::new(tunnel))),
    });
    // The request gives up while the connect is still pending.
    let pending = tokio::time::timeout(
        Duration::from_millis(100),
        call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })),
    );
    let (waited, ()) = tokio::join!(pending, entered.notified());
    assert!(waited.is_err(), "the connect should still be pending");
    // The dropped request does not free the slot: a new start is refused at once. A broken
    // reservation would queue or connect here, so a bound turns that into a failure.
    let (status, body) = tokio::time::timeout(
        Duration::from_secs(2),
        call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })),
    )
    .await
    .expect("the competing start queued instead of being refused");
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_eq!(body["kind"], "gatewayBusy");
    assert!(!handle.disconnected());

    // The connect now completes with nobody waiting: the tunnel is released
    // before the reservation is, and the slot ends empty.
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !handle.disconnected() || state.bus_session.try_lock().is_err() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the unclaimed tunnel was not released");
    assert!(state.bus_session.try_lock().unwrap().is_none());
}

// ---------------------------------------------------------------------------
// Real TCP loopback: the default connector against a scripted gateway.
// ---------------------------------------------------------------------------

const CHANNEL: u8 = 0x2a;
/// CONNECT_REQUEST for a TCP tunnel: Route Back control and data HPAIs, link layer CRI.
const TCP_CONNECT_REQUEST: [u8; 26] = [
    0x06, 0x10, 0x02, 0x05, 0x00, 0x1a, 0x08, 0x02, 0, 0, 0, 0, 0, 0, 0x08, 0x02, 0, 0, 0, 0, 0, 0,
    0x04, 0x04, 0x02, 0x00,
];
/// CONNECT_RESPONSE: channel 0x2a, no error, Route Back data HPAI, CRD for IA 1.1.250.
const TCP_CONNECT_RESPONSE: [u8; 20] = [
    0x06, 0x10, 0x02, 0x06, 0x00, 0x14, CHANNEL, 0x00, 0x08, 0x02, 0, 0, 0, 0, 0, 0, 0x04, 0x04,
    0x11, 0xfa,
];
/// L_Data.ind: 1.1.1 -> 0/0/1, GroupValueWrite 1, as a TUNNELLING_REQUEST on the channel.
const TCP_TELEGRAM: [u8; 21] = [
    0x06, 0x10, 0x04, 0x20, 0x00, 0x15, 0x04, CHANNEL, 0x07, 0x00, 0x29, 0x00, 0xbc, 0xe0, 0x11,
    0x01, 0x00, 0x01, 0x01, 0x00, 0x81,
];

async fn read_frame(stream: &mut TcpStream) -> Vec<u8> {
    let mut header = [0u8; 6];
    stream.read_exact(&mut header).await.unwrap();
    let total = u16::from_be_bytes([header[4], header[5]]) as usize;
    let mut frame = header.to_vec();
    frame.resize(total, 0);
    stream.read_exact(&mut frame[6..]).await.unwrap();
    frame
}

#[tokio::test]
async fn real_tcp_gateway_sees_the_exact_handshake_group_write_and_orderly_close() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway = listener.local_addr().unwrap();
    let gateway_task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        assert_eq!(read_frame(&mut stream).await, TCP_CONNECT_REQUEST);
        stream.write_all(&TCP_CONNECT_RESPONSE).await.unwrap();
        stream.write_all(&TCP_TELEGRAM).await.unwrap();
        // The bound group write arrives as a TUNNELLING_REQUEST on our channel.
        let request = read_frame(&mut stream).await;
        assert_eq!(&request[..4], [0x06, 0x10, 0x04, 0x20]);
        assert_eq!(request[6], 0x04, "connection header length");
        assert_eq!(request[7], CHANNEL);
        assert_eq!(request[10], 0x11, "L_Data.req");
        // 1/1/1 is 0x0901 on the wire, a group write of 6-bit value 1.
        assert_eq!(
            &request[request.len() - 5..],
            [0x09, 0x01, 0x01, 0x00, 0x81]
        );
        // Stop: DISCONNECT_REQUEST for the channel, then the close.
        let disconnect = read_frame(&mut stream).await;
        assert_eq!(&disconnect[2..4], [0x02, 0x09]);
        assert_eq!(disconnect[6], CHANNEL);
        // The client may close right after its request, so answering can fail.
        let _ = stream
            .write_all(&[0x06, 0x10, 0x02, 0x0a, 0x00, 0x08, CHANNEL, 0x00])
            .await;
        let mut rest = Vec::new();
        if stream.read_to_end(&mut rest).await.is_ok() {
            assert!(rest.is_empty(), "nothing follows the disconnect");
        }
    });

    // The production connector, with no fake anywhere.
    let state = Arc::new(AppState::default());
    let app = knx_server::app(state.clone(), None);
    let (status, started) = call(
        &app,
        "POST",
        START_TCP,
        json!({ "gateway": gateway.to_string() }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{started}");
    assert_eq!(started["transport"], "tcp");
    assert_eq!(started["assignedAddress"], "1.1.250");

    let poll = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let (_, poll) = call(
                &app,
                "GET",
                "/api/bus/monitor/telegrams?since=0",
                Value::Null,
            )
            .await;
            if !poll["telegrams"].as_array().unwrap().is_empty() {
                break poll;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the gateway's telegram never reached the monitor");
    assert_eq!(poll["transport"], "tcp");
    // Negative control: a stream without skips reports a measured zero, not an absence.
    assert_eq!(started["skippedFrames"], 0);
    assert_eq!(poll["skippedFrames"], 0);
    assert_eq!(poll["telegrams"][0]["source"], "1.1.1");
    // Without an open project the existing monitor displays the flat group-address style.
    assert_eq!(poll["telegrams"][0]["destination"], "1");
    assert_eq!(poll["telegrams"][0]["service"], "GroupValueWrite");

    let (status, body) = call(
        &app,
        "POST",
        "/api/bus/write-bound",
        binding(
            &started,
            json!({ "destination": "1/1/1", "dpt": "DPST-1-1", "value": "on" }),
        ),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["transport"], "tcp");

    let (status, stopped) = call(&app, "POST", &stop_uri(&started), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{stopped}");
    assert!(state.bus_session.try_lock().unwrap().is_none());
    tokio::time::timeout(Duration::from_secs(5), gateway_task)
        .await
        .expect("the gateway script did not finish")
        .expect("the gateway script panicked");
}

#[tokio::test]
async fn a_refused_tcp_connect_is_an_error_and_sends_nothing_over_udp() {
    // Take a free TCP port, then close it so nothing listens there.
    let port = {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        listener.local_addr().unwrap().port()
    };
    // A UDP listener on the same port number sees any fallback datagram.
    let udp = UdpSocket::bind(("127.0.0.1", port)).await.unwrap();
    let state = Arc::new(AppState::default());
    let app = knx_server::app(state.clone(), None);
    let (status, body) = call(
        &app,
        "POST",
        START_TCP,
        json!({ "gateway": format!("127.0.0.1:{port}") }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_GATEWAY, "{body}");
    assert!(state.bus_session.try_lock().unwrap().is_none());
    let mut datagram = [0u8; 64];
    assert!(
        tokio::time::timeout(Duration::from_millis(300), udp.recv_from(&mut datagram))
            .await
            .is_err(),
        "a UDP datagram followed the refused TCP connect"
    );
}

#[tokio::test]
async fn a_tcp_gateway_closing_the_stream_marks_the_session_closed_but_keeps_it_readable() {
    let (tunnel, handle) = tcp_tunnel();
    let connector = Arc::new(FakeConnector::succeeding(tunnel).accepting_tcp());
    let (_state, app) = app_with(Shared(connector));
    let (_, started) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    handle.sender().send(TunnelEvent::Closed).unwrap();
    let poll = tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let (_, poll) = call(
                &app,
                "GET",
                "/api/bus/monitor/telegrams?since=0",
                Value::Null,
            )
            .await;
            if poll["status"] == "closed" {
                break poll;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    assert_eq!(poll["transport"], "tcp", "provenance survives the close");
    assert_eq!(poll["sessionId"], started["sessionId"]);
}

// ---------------------------------------------------------------------------
// Skipped TCP frames, the stop reservation and the stop-admission recheck.
// ---------------------------------------------------------------------------

/// A TCP connect that returns `tunnel` at once, for tests that only need a session.
struct Plain {
    tunnel: Mutex<Option<Box<dyn BusTunnel>>>,
}
impl GatewayConnector for Plain {
    fn connect_tunnel(
        &self,
        _: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        panic!("an explicit TCP request must not open UDP")
    }
    fn connect_tunnel_with_transport(
        &self,
        _: SocketAddrV4,
        _: TunnelTransport,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async move { Ok(self.tunnel.lock().unwrap().take().unwrap()) })
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
        panic!("TCP monitoring must not discover gateways")
    }
}

/// A TCP tunnel whose disconnect waits for the test, to hold a stop mid-cleanup.
struct GatedDisconnect {
    inner: FakeTunnel,
    entered: Arc<Notify>,
    release: Arc<Notify>,
}
impl BusTunnel for GatedDisconnect {
    fn transport(&self) -> TunnelTransport {
        TunnelTransport::Tcp
    }
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        self.inner.assigned_address()
    }
    fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TunnelEvent> {
        self.inner.subscribe()
    }
    fn send(
        &self,
        destination: knx_net::Destination,
        service: knx_net::ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        self.inner.send(destination, service)
    }
    fn send_frame(
        &self,
        destination: knx_net::Destination,
        transport: knx_net::Tpci,
        service: knx_net::ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), knx_net::BusError>> + Send + '_>> {
        self.inner.send_frame(destination, transport, service)
    }
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async move {
            self.entered.notify_one();
            self.release.notified().await;
            Box::new(self.inner).disconnect().await
        })
    }
}

#[tokio::test]
async fn a_tcp_stop_never_queues_behind_a_held_reservation_bound_or_legacy() {
    let (tunnel, handle) = tcp_tunnel();
    let connector = Arc::new(FakeConnector::succeeding(tunnel).accepting_tcp());
    let (state, app) = app_with(Shared(connector));
    let (_, started) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    let held = state.bus_session.lock().await;
    for uri in [stop_uri(&started), "/api/bus/monitor/stop".to_owned()] {
        let (status, body) = tokio::time::timeout(
            Duration::from_secs(1),
            call(&app, "POST", &uri, Value::Null),
        )
        .await
        .unwrap_or_else(|_| panic!("{uri} queued behind a held reservation"));
        assert_eq!(status, StatusCode::CONFLICT, "{uri}: {body}");
        assert_eq!(body["kind"], "gatewayBusy", "{uri}");
    }
    drop(held);
    assert!(
        !handle.disconnected(),
        "a refused stop must not touch the tunnel"
    );
    assert!(state.bus_session.try_lock().unwrap().is_some());
    // With the reservation free, the same stop goes through.
    let (status, stopped) = call(&app, "POST", &stop_uri(&started), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{stopped}");
    assert!(handle.disconnected());
}

#[tokio::test]
async fn the_legacy_stop_ends_a_tcp_session_and_reports_no_session_when_there_is_none() {
    let (tunnel, handle) = tcp_tunnel();
    let connector = Arc::new(FakeConnector::succeeding(tunnel).accepting_tcp());
    let (state, app) = app_with(Shared(connector));
    let (status, body) = call(&app, "POST", "/api/bus/monitor/stop", Value::Null).await;
    assert_eq!(status, StatusCode::CONFLICT, "{body}");
    assert_ne!(body["kind"], "busSessionChanged", "{body}");
    let (_, started) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    handle.set_skipped_frames(1);
    let (status, stopped) = call(&app, "POST", "/api/bus/monitor/stop", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{stopped}");
    assert_eq!(stopped["sessionId"], started["sessionId"]);
    assert_eq!(stopped["serverIncarnation"], started["serverIncarnation"]);
    assert_eq!(stopped["skippedFrames"], 1);
    assert!(handle.disconnected());
    assert!(state.bus_session.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn a_dropped_tcp_stop_keeps_the_reservation_until_the_disconnect_has_finished() {
    let (inner, handle) = FakeTunnel::new("1.1.250".parse().unwrap(), 8);
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let (state, app) = app_with(Plain {
        tunnel: Mutex::new(Some(Box::new(GatedDisconnect {
            inner,
            entered: entered.clone(),
            release: release.clone(),
        }))),
    });
    let (_, started) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    // The stop request gives up while the disconnect is still running.
    let stopped_uri = stop_uri(&started);
    let pending = tokio::time::timeout(
        Duration::from_millis(100),
        call(&app, "POST", &stopped_uri, Value::Null),
    );
    let (waited, ()) = tokio::join!(pending, entered.notified());
    assert!(waited.is_err(), "the disconnect should still be running");
    // The reservation is still owned: another start and both stops are refused at once.
    for (uri, body) in [
        (START_TCP.to_owned(), json!({ "gateway": GATEWAY })),
        (stop_uri(&started), Value::Null),
        ("/api/bus/monitor/stop".to_owned(), Value::Null),
    ] {
        let (status, answer) =
            tokio::time::timeout(Duration::from_secs(2), call(&app, "POST", &uri, body))
                .await
                .unwrap_or_else(|_| panic!("{uri} queued behind the stop's reservation"));
        assert_eq!(status, StatusCode::CONFLICT, "{uri}: {answer}");
        assert_eq!(answer["kind"], "gatewayBusy", "{uri}");
    }
    assert!(!handle.disconnected());
    assert!(state.bus_session.try_lock().is_err());
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !handle.disconnected() || state.bus_session.try_lock().is_err() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the stop did not finish after its disconnect was released");
    assert!(state.bus_session.try_lock().unwrap().is_none());
}

#[tokio::test]
async fn admission_closed_while_a_tcp_connect_is_gated_still_releases_the_late_tunnel() {
    let (tunnel, handle) = tcp_tunnel();
    let entered = Arc::new(Notify::new());
    let release = Arc::new(Notify::new());
    let (state, app) = app_with(Gated {
        entered: entered.clone(),
        release: release.clone(),
        tunnel: Mutex::new(Some(Box::new(tunnel))),
    });
    let pending = tokio::time::timeout(
        Duration::from_millis(100),
        call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })),
    );
    let (waited, ()) = tokio::join!(pending, entered.notified());
    assert!(waited.is_err());
    // The orderly stop begins while the connect is inside the connector.
    state.stop_admission.close();
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !handle.disconnected() || state.bus_session.try_lock().is_err() {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the late tunnel was not released");
    assert!(state.bus_session.try_lock().unwrap().is_none());
    // Closed admission refuses a new start before any connector call.
    let (status, body) = call(&app, "POST", START_TCP, json!({ "gateway": GATEWAY })).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE, "{body}");
    assert_eq!(body["kind"], "serverStopping");
}

/// A frame of an unknown service, and a TUNNELLING_REQUEST far above any plausible cEMI size.
fn unknown_service_frame() -> Vec<u8> {
    let mut frame = vec![0x06, 0x10, 0x09, 0x99, 0x00, 0x10];
    frame.extend_from_slice(&[0xee; 10]);
    frame
}
fn oversize_tunnelling_request() -> Vec<u8> {
    let total: u16 = 1500;
    let mut frame = vec![0x06, 0x10, 0x04, 0x20];
    frame.extend_from_slice(&total.to_be_bytes());
    frame.extend_from_slice(&[0x04, CHANNEL, 0x00, 0x00]);
    frame.resize(usize::from(total), 0x00);
    frame
}

#[tokio::test]
async fn real_tcp_skipped_frames_are_counted_apart_from_telegrams_and_the_next_telegram_arrives() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let gateway = listener.local_addr().unwrap();
    let gateway_task = tokio::spawn(async move {
        let (mut stream, _) = listener.accept().await.unwrap();
        assert_eq!(read_frame(&mut stream).await, TCP_CONNECT_REQUEST);
        stream.write_all(&TCP_CONNECT_RESPONSE).await.unwrap();
        // Unknown service, then an over-cap request, then a valid telegram, in one stream.
        stream.write_all(&unknown_service_frame()).await.unwrap();
        stream
            .write_all(&oversize_tunnelling_request())
            .await
            .unwrap();
        stream.write_all(&TCP_TELEGRAM).await.unwrap();
        let disconnect = read_frame(&mut stream).await;
        assert_eq!(&disconnect[2..4], [0x02, 0x09]);
        let _ = stream
            .write_all(&[0x06, 0x10, 0x02, 0x0a, 0x00, 0x08, CHANNEL, 0x00])
            .await;
        let mut rest = Vec::new();
        let _ = stream.read_to_end(&mut rest).await;
    });
    let state = Arc::new(AppState::default());
    let app = knx_server::app(state.clone(), None);
    let (status, started) = call(
        &app,
        "POST",
        START_TCP,
        json!({ "gateway": gateway.to_string() }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{started}");
    let poll = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let (_, poll) = call(
                &app,
                "GET",
                "/api/bus/monitor/telegrams?since=0",
                Value::Null,
            )
            .await;
            if !poll["telegrams"].as_array().unwrap().is_empty() {
                break poll;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the telegram after the skipped frames never arrived");
    // Exactly the valid telegram is a row; the two skipped frames are counted as frames.
    assert_eq!(poll["telegrams"].as_array().unwrap().len(), 1);
    assert_eq!(poll["skippedFrames"], 2);
    assert_eq!(
        poll["droppedBefore"], 0,
        "skipped frames are not dropped rows"
    );
    let (status, stopped) = call(&app, "POST", &stop_uri(&started), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{stopped}");
    assert_eq!(stopped["skippedFrames"], 2);
    assert_eq!(stopped["droppedCount"], 0);
    assert_eq!(stopped["telegramCount"], 1);
    tokio::time::timeout(Duration::from_secs(5), gateway_task)
        .await
        .expect("the gateway script did not finish")
        .expect("the gateway script panicked");
}
