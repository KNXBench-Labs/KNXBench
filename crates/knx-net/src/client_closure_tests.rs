//! Loopback tests for why a tunnel closed and for its teardown after that (issue #30).
//!
//! A fake gateway on 127.0.0.1 accepts one tunnel and then scripts the
//! closing case. No real gateway, bus or device is involved.

use super::*;
use crate::cemi::{GroupValue, Tpci};
use knx_core::{GroupAddress, GroupAddressStyle};
use tokio::time::Instant;

/// Longer than any reply in these scripts, shorter than a test hang.
const STEP: Duration = Duration::from_secs(3);
/// How long "no further datagram" is watched for.
const QUIET: Duration = Duration::from_millis(200);
/// An L_Data.ind group write (1.1.1 -> 0/0/1, value 1).
const L_DATA_IND: [u8; 11] = [
    0x29, 0x00, 0xbc, 0xe0, 0x11, 0x01, 0x00, 0x01, 0x01, 0x00, 0x81,
];

struct Gateway {
    socket: UdpSocket,
    client: SocketAddr,
    channel: u8,
}

impl Gateway {
    async fn send(&self, service: u16, body: &[u8]) {
        self.socket
            .send_to(&frame::encode_frame(service, body), self.client)
            .await
            .expect("fake gateway send");
    }

    /// The next datagram from the client, which must carry `service`.
    async fn expect(&self, service: u16) -> Vec<u8> {
        self.expect_within(service, STEP).await
    }

    /// [`Self::expect`] with a longer budget, for the patient waits.
    async fn expect_within(&self, service: u16, limit: Duration) -> Vec<u8> {
        let mut buf = [0u8; 256];
        let (n, _) = tokio::time::timeout(limit, self.socket.recv_from(&mut buf))
            .await
            .unwrap_or_else(|_| panic!("no datagram; expected service {service:#06x}"))
            .expect("fake gateway receive");
        let (header, body) = frame::decode_frame(&buf[..n]).expect("a KNXnet/IP frame");
        assert_eq!(header.service_type, service, "unexpected service");
        body.to_vec()
    }

    async fn expect_quiet(&self) {
        let mut buf = [0u8; 256];
        if let Ok(Ok((n, _))) = tokio::time::timeout(QUIET, self.socket.recv_from(&mut buf)).await {
            let (header, _) = frame::decode_frame(&buf[..n]).expect("a KNXnet/IP frame");
            panic!("unexpected datagram {:#06x}", header.service_type);
        }
    }

    async fn disconnect_request(&self) {
        let hpai = local_hpai(&self.socket).expect("gateway HPAI");
        self.send(
            services::DISCONNECT_REQUEST,
            &services::encode_disconnect_request(self.channel, hpai),
        )
        .await;
    }
}

/// One accepted tunnel: the client and the fake gateway that holds it.
async fn tunnel(channel: u8) -> (TunnelClient, Gateway) {
    tunnel_timed(channel, HEARTBEAT).await
}

/// A heartbeat that gives up after about 300 ms: four 50 ms waits, 100 ms in.
const FAST_HEARTBEAT: HeartbeatTiming = HeartbeatTiming {
    interval: Duration::from_millis(100),
    response_timeout: Duration::from_millis(50),
};

async fn tunnel_timed(channel: u8, heartbeat: HeartbeatTiming) -> (TunnelClient, Gateway) {
    tunnel_with(channel, heartbeat, TunnelAckWait::Standard).await
}

async fn tunnel_with(
    channel: u8,
    heartbeat: HeartbeatTiming,
    ack_wait: TunnelAckWait,
) -> (TunnelClient, Gateway) {
    tunnel_via(channel, move |address| {
        TunnelClient::connect_timed(address, TunnelReturnPath::default(), heartbeat, ack_wait)
    })
    .await
}

/// One accepted tunnel opened by `connect`, which gets the gateway address.
async fn tunnel_via<F, Fut>(channel: u8, connect: F) -> (TunnelClient, Gateway)
where
    F: FnOnce(SocketAddrV4) -> Fut,
    Fut: std::future::Future<Output = Result<TunnelClient, BusError>> + Send + 'static,
{
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind gateway");
    let SocketAddr::V4(address) = socket.local_addr().expect("gateway address") else {
        unreachable!("bound an IPv4 socket")
    };
    let connecting = tokio::spawn(connect(address));
    let mut buf = [0u8; 128];
    let (n, client) = socket.recv_from(&mut buf).await.expect("CONNECT_REQUEST");
    let (header, _) = frame::decode_frame(&buf[..n]).expect("a KNXnet/IP frame");
    assert_eq!(header.service_type, services::CONNECT_REQUEST);
    let mut body = vec![channel, services::E_NO_ERROR];
    body.extend_from_slice(
        &Hpai {
            addr: *address.ip(),
            port: address.port(),
        }
        .encode(),
    );
    body.extend_from_slice(&[0x04, tunnelling::TUNNEL_CONNECTION, 0x11, 0xff]);
    socket
        .send_to(
            &frame::encode_frame(services::CONNECT_RESPONSE, &body),
            client,
        )
        .await
        .expect("CONNECT_RESPONSE");
    let tunnel = connecting
        .await
        .expect("connect task")
        .expect("loopback tunnel");
    let gateway = Gateway {
        socket,
        client,
        channel,
    };
    (tunnel, gateway)
}

async fn group_write(client: &TunnelClient) -> Result<(), BusError> {
    let group = GroupAddress::parse("0/0/1", GroupAddressStyle::ThreeLevel).unwrap();
    client
        .send_frame(
            Destination::Group(group),
            Tpci::UnnumberedData,
            ApplicationService::GroupValueWrite(GroupValue::Short(1)),
        )
        .await
}

async fn closed_event(events: &mut broadcast::Receiver<TunnelEvent>) {
    let event = tokio::time::timeout(STEP, events.recv()).await;
    assert!(
        matches!(event, Ok(Ok(TunnelEvent::Closed))),
        "expected Closed, got {event:?}"
    );
}

/// The gateway ended the tunnel before the send: the send refuses at once
/// with that cause, and `disconnect()` asks no dead channel to disconnect.
#[tokio::test]
async fn a_send_after_the_gateway_disconnected_names_that_cause_at_once() {
    let (client, gateway) = tunnel(0x21).await;
    let mut events = client.subscribe();
    gateway.disconnect_request().await;
    let response = gateway.expect(services::DISCONNECT_RESPONSE).await;
    assert_eq!(response, [0x21, services::E_NO_ERROR]);
    closed_event(&mut events).await;
    assert_eq!(client.closure(), Some(TunnelClosure::GatewayDisconnect));

    let started = Instant::now();
    let error = group_write(&client).await.unwrap_err();
    assert!(
        matches!(
            error,
            BusError::TunnelClosed(TunnelClosure::GatewayDisconnect)
        ),
        "{error:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "refused without waiting for an acknowledgement"
    );
    assert_eq!(
        error.to_string(),
        "tunnel closed: the gateway ended the tunnel connection"
    );
    gateway.expect_quiet().await;

    client
        .disconnect()
        .await
        .expect("the gateway already terminated the channel");
    gateway.expect_quiet().await;
}

/// The gateway closes the tunnel while a request waits for its
/// acknowledgement: the send ends before the one-second window.
#[tokio::test]
async fn a_gateway_disconnect_during_the_acknowledgement_wait_ends_the_send() {
    let (client, gateway) = tunnel(0x22).await;
    let started = Instant::now();
    let sending = tokio::spawn(async move {
        let result = group_write(&client).await;
        (client, result)
    });
    gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    gateway.disconnect_request().await;
    gateway.expect(services::DISCONNECT_RESPONSE).await;
    let (client, result) = sending.await.expect("send task");
    assert!(
        matches!(
            result,
            Err(BusError::TunnelClosed(TunnelClosure::GatewayDisconnect))
        ),
        "{result:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(900),
        "ended before the acknowledgement window"
    );
    gateway.expect_quiet().await;
    client.disconnect().await.expect("gateway-ended channel");
}

/// Twice no acknowledgement (or twice an error status): the error names the
/// sequence, subscribers get `Closed`, later telegrams are acknowledged but
/// not delivered, and `disconnect()` confirms the earlier request's response
/// without asking twice.
async fn unacknowledged(channel: u8, ack_status: Option<u8>) {
    let (client, gateway) = tunnel(channel).await;
    let mut events = client.subscribe();
    let sending = tokio::spawn(async move {
        let result = group_write(&client).await;
        (client, result)
    });
    for _attempt in 0..2 {
        let body = gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
        let request = tunnelling::decode_tunnelling_request(&body).expect("request");
        assert_eq!(
            request.sequence_counter, 0,
            "the repetition keeps its sequence"
        );
        if let Some(status) = ack_status {
            gateway
                .send(
                    tunnelling::TUNNELLING_ACK,
                    &tunnelling::encode_tunnelling_ack(channel, 0, status),
                )
                .await;
        }
    }
    let body = gateway.expect(services::DISCONNECT_REQUEST).await;
    assert_eq!(body.first(), Some(&channel));
    let (client, result) = sending.await.expect("send task");
    let expected = TunnelClosure::Unacknowledged {
        sequence: 0,
        last_status: ack_status,
    };
    match result {
        Err(BusError::TunnelClosed(found)) => assert_eq!(found, expected),
        other => panic!("expected the unacknowledged sequence, got {other:?}"),
    }
    assert_eq!(client.closure(), Some(expected));
    closed_event(&mut events).await;

    // The gateway still sends a telegram: acknowledged, not delivered.
    gateway
        .send(
            tunnelling::TUNNELLING_REQUEST,
            &tunnelling::encode_tunnelling_request(channel, 0, &L_DATA_IND),
        )
        .await;
    let ack = gateway.expect(tunnelling::TUNNELLING_ACK).await;
    assert_eq!(ack, tunnelling::encode_tunnelling_ack(channel, 0, 0));
    assert!(
        matches!(
            events.try_recv(),
            Err(broadcast::error::TryRecvError::Empty)
        ),
        "Closed is the last event"
    );

    gateway
        .send(
            services::DISCONNECT_RESPONSE,
            &services::encode_disconnect_response(channel, services::E_NO_ERROR),
        )
        .await;
    let started = Instant::now();
    tokio::time::timeout(STEP, client.disconnect())
        .await
        .expect("disconnect does not wait out its timeout")
        .expect("the gateway confirmed the earlier DISCONNECT_REQUEST");
    assert!(started.elapsed() < Duration::from_secs(1));
    gateway.expect_quiet().await;
    assert!(
        !matches!(events.try_recv(), Ok(TunnelEvent::Closed)),
        "Closed is sent once"
    );
}

#[tokio::test]
async fn an_unacknowledged_request_names_its_sequence_and_teardown_still_confirms() {
    unacknowledged(0x23, None).await;
}

#[tokio::test]
async fn a_refused_request_keeps_the_gateways_last_acknowledgement_status() {
    unacknowledged(0x24, Some(0x29)).await;
    assert_eq!(
        BusError::TunnelClosed(TunnelClosure::Unacknowledged {
            sequence: 5,
            last_status: Some(0x29),
        })
        .to_string(),
        "tunnel closed: the gateway refused tunnelling request 5 and its repetition \
         (TUNNELLING_ACK status 0x29)"
    );
}

/// After the client terminated the tunnel, `disconnect()` waits for the
/// response to that request even when it arrives later, and asks only once.
#[tokio::test]
async fn disconnect_after_an_unacknowledged_request_waits_for_the_late_response() {
    let (client, gateway) = tunnel(0x25).await;
    let sending = tokio::spawn(async move {
        let result = group_write(&client).await;
        (client, result)
    });
    gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    gateway.expect(services::DISCONNECT_REQUEST).await;
    let (client, result) = sending.await.expect("send task");
    assert!(result.is_err());
    let mut disconnecting = tokio::spawn(client.disconnect());
    assert!(
        tokio::time::timeout(Duration::from_millis(100), &mut disconnecting)
            .await
            .is_err(),
        "disconnect returned before DISCONNECT_RESPONSE"
    );
    gateway.expect_quiet().await;
    gateway
        .send(
            services::DISCONNECT_RESPONSE,
            &services::encode_disconnect_response(0x25, services::E_NO_ERROR),
        )
        .await;
    disconnecting
        .await
        .expect("disconnect task")
        .expect("confirmed by the late response");
}

/// A `DISCONNECT_RESPONSE` nobody asked for does not end a live tunnel.
#[tokio::test]
async fn an_unsolicited_disconnect_response_leaves_the_tunnel_open() {
    let (client, gateway) = tunnel(0x26).await;
    let mut events = client.subscribe();
    gateway
        .send(
            services::DISCONNECT_RESPONSE,
            &services::encode_disconnect_response(0x26, services::E_NO_ERROR),
        )
        .await;
    gateway
        .send(
            tunnelling::TUNNELLING_REQUEST,
            &tunnelling::encode_tunnelling_request(0x26, 0, &L_DATA_IND),
        )
        .await;
    let ack = gateway.expect(tunnelling::TUNNELLING_ACK).await;
    assert_eq!(ack, tunnelling::encode_tunnelling_ack(0x26, 0, 0));
    let event = tokio::time::timeout(STEP, events.recv()).await;
    assert!(
        matches!(event, Ok(Ok(TunnelEvent::Telegram(_)))),
        "still forwarded: {event:?}"
    );
    assert_eq!(client.closure(), None);
}

/// The confirming `DISCONNECT_RESPONSE` ends the channel at once, before the
/// caller disconnects: a later request is neither acknowledged nor delivered,
/// and the stored confirmation still completes `disconnect()`.
#[tokio::test]
async fn the_confirming_response_ends_the_channel_before_disconnect_is_called() {
    let (client, gateway) = tunnel(0x27).await;
    let sending = tokio::spawn(async move {
        let result = group_write(&client).await;
        (client, result)
    });
    gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    gateway.expect(services::DISCONNECT_REQUEST).await;
    let (client, result) = sending.await.expect("send task");
    assert!(result.is_err());
    gateway
        .send(
            services::DISCONNECT_RESPONSE,
            &services::encode_disconnect_response(0x27, services::E_NO_ERROR),
        )
        .await;
    tokio::time::sleep(Duration::from_millis(50)).await;
    gateway
        .send(
            tunnelling::TUNNELLING_REQUEST,
            &tunnelling::encode_tunnelling_request(0x27, 0, &L_DATA_IND),
        )
        .await;
    gateway.expect_quiet().await;
    tokio::time::timeout(STEP, client.disconnect())
        .await
        .expect("no wait for a second response")
        .expect("the stored confirmation completes disconnect");
    gateway.expect_quiet().await;
}

/// A refused datagram (ICMP port unreachable on the connected socket) ends
/// the receive loop as `SocketFailed`; a waiting send ends at once, and
/// `disconnect()` still sends its request before reporting the failure.
#[tokio::test]
async fn a_socket_failure_names_its_cause_and_disconnect_still_asks() {
    let (client, gateway) = tunnel(0x28).await;
    let port = gateway.socket.local_addr().unwrap();
    drop(gateway);
    let started = Instant::now();
    let result = group_write(&client).await;
    assert!(
        matches!(
            result,
            Err(BusError::TunnelClosed(TunnelClosure::SocketFailed))
        ),
        "{result:?}"
    );
    assert!(started.elapsed() < Duration::from_millis(900));
    assert_eq!(client.closure(), Some(TunnelClosure::SocketFailed));
    // The gateway is back on the same port: it sees the DISCONNECT_REQUEST.
    let socket = UdpSocket::bind(port).await.expect("rebind gateway port");
    let started = Instant::now();
    let error = client.disconnect().await.unwrap_err();
    assert!(
        matches!(error, BusError::TunnelClosed(TunnelClosure::SocketFailed)),
        "{error:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(500),
        "no response wait"
    );
    let mut buf = [0u8; 64];
    let (n, _) = tokio::time::timeout(STEP, socket.recv_from(&mut buf))
        .await
        .expect("best-effort DISCONNECT_REQUEST")
        .unwrap();
    let (header, body) = frame::decode_frame(&buf[..n]).unwrap();
    assert_eq!(header.service_type, services::DISCONNECT_REQUEST);
    assert_eq!(body.first(), Some(&0x28));
}

/// A failed heartbeat ends a send that waits for its acknowledgement before
/// its one-second window, and asks the gateway to disconnect once.
#[tokio::test]
async fn a_failed_heartbeat_ends_a_waiting_send_early() {
    let (client, gateway) = tunnel_timed(0x29, FAST_HEARTBEAT).await;
    let started = Instant::now();
    let sending = tokio::spawn(async move {
        let result = group_write(&client).await;
        (client, result)
    });
    gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    for _attempt in 0..4 {
        gateway.expect(services::CONNECTIONSTATE_REQUEST).await;
    }
    gateway.expect(services::DISCONNECT_REQUEST).await;
    let (client, result) = sending.await.expect("send task");
    assert!(
        matches!(
            result,
            Err(BusError::TunnelClosed(TunnelClosure::HeartbeatFailed))
        ),
        "{result:?}"
    );
    assert!(
        started.elapsed() < Duration::from_millis(900),
        "ended by the heartbeat, not by the acknowledgement window"
    );
    gateway.expect_quiet().await;
    drop(client);
}

/// After the gateway closed the tunnel the heartbeat sends nothing more,
/// also when the closure falls into its interval wait.
#[tokio::test]
async fn no_heartbeat_follows_a_gateway_disconnect() {
    // A wider interval leaves the fake gateway ample time to disconnect even
    // when the OS schedules this test late under gate load.
    let heartbeat = HeartbeatTiming {
        interval: Duration::from_secs(1),
        response_timeout: FAST_HEARTBEAT.response_timeout,
    };
    let (client, gateway) = tunnel_timed(0x2a, heartbeat).await;
    // One answered heartbeat first, so the loop is waiting for its next tick.
    gateway.expect(services::CONNECTIONSTATE_REQUEST).await;
    gateway
        .send(
            services::CONNECTIONSTATE_RESPONSE,
            &[0x2a, services::E_NO_ERROR],
        )
        .await;
    // Let the heartbeat loop pass its closure check and wait for the next tick.
    tokio::time::sleep(Duration::from_millis(30)).await;
    gateway.disconnect_request().await;
    gateway.expect(services::DISCONNECT_RESPONSE).await;
    // Several intervals: neither a CONNECTIONSTATE_REQUEST nor a DISCONNECT_REQUEST.
    let mut buf = [0u8; 64];
    if let Ok(Ok((n, _))) = tokio::time::timeout(
        Duration::from_millis(2500),
        gateway.socket.recv_from(&mut buf),
    )
    .await
    {
        let (header, _) = frame::decode_frame(&buf[..n]).unwrap();
        panic!("unexpected datagram {:#06x}", header.service_type);
    }
    assert_eq!(client.closure(), Some(TunnelClosure::GatewayDisconnect));
    client.disconnect().await.expect("gateway-ended channel");
}

// ---- Patient acknowledgement wait (issue #30) ------------------------------
//
// The expected figures below are written as literals on purpose: they must
// not follow `TunnelAckWait::timeout`, or a wrong constant would agree with
// its own test. Long budgets run on a paused Tokio clock that the test moves
// by hand; the `real_udp` tests are the wall-clock witnesses on the same
// loopback socket.
//
// A paused clock also jumps to the next timer whenever the runtime is idle,
// even while a loopback datagram is still on its way, which reorders client
// timers and fake-gateway reads. So these tests never wait on the gateway
// socket under a paused clock: they `step` the clock, give the kernel and
// the client's tasks real time to act (`settle`, a blocking task, during
// which the clock cannot jump), and then drain what has arrived.

const WIRE_ACK: fn(u8, u8) -> [u8; 4] = |channel, sequence| {
    tunnelling::encode_tunnelling_ack(channel, sequence, tunnelling::E_NO_ERROR)
};

/// Real time for the kernel and the client tasks to catch up.
async fn settle() {
    tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_millis(40)))
        .await
        .expect("settle");
}

/// One datagram the client sent, and the paused-clock time of the step that
/// drained it.
struct Sent {
    at: Duration,
    service: u16,
    body: Vec<u8>,
}

/// A tunnel on a paused clock, with the clock starting at zero.
struct Paused {
    client: Arc<TunnelClient>,
    gateway: Gateway,
    start: Instant,
}

async fn paused(channel: u8, heartbeat: HeartbeatTiming, ack_wait: TunnelAckWait) -> Paused {
    let (client, gateway) = tunnel_with(channel, heartbeat, ack_wait).await;
    tokio::time::pause();
    Paused {
        client: Arc::new(client),
        gateway,
        start: Instant::now(),
    }
}

impl Paused {
    /// A group write that runs while the test moves the clock.
    fn write(&self) -> tokio::task::JoinHandle<Result<(), BusError>> {
        let client = self.client.clone();
        tokio::spawn(async move { group_write(&client).await })
    }

    /// Datagrams from the client that are waiting right now.
    fn drain(&self) -> Vec<Sent> {
        let mut found = Vec::new();
        let mut buf = [0u8; 256];
        while let Ok((n, _)) = self.gateway.socket.try_recv_from(&mut buf) {
            let (header, body) = frame::decode_frame(&buf[..n]).expect("a KNXnet/IP frame");
            found.push(Sent {
                at: self.start.elapsed(),
                service: header.service_type,
                body: body.to_vec(),
            });
        }
        found
    }

    /// Lets the client act on what the test just did, without moving the clock.
    async fn now(&self) -> Vec<Sent> {
        settle().await;
        self.drain()
    }

    /// Moves the clock by `by`, lets the client act, drains what it sent.
    async fn step(&self, by: Duration) -> Vec<Sent> {
        tokio::time::advance(by).await;
        self.now().await
    }

    /// `step`s through `total` in increments of `by`.
    async fn watch(&self, total: Duration, by: Duration) -> Vec<Sent> {
        let mut found = Vec::new();
        let mut moved = Duration::ZERO;
        while moved < total {
            found.extend(self.step(by).await);
            moved += by;
        }
        found
    }

    async fn send(&self, service: u16, body: &[u8]) -> Vec<Sent> {
        self.gateway.send(service, body).await;
        self.now().await
    }
}

fn secs(whole: u64) -> Duration {
    Duration::from_secs(whole)
}

#[tokio::test(start_paused = true)]
async fn settle_keeps_the_paused_clock_from_jumping() {
    let started = Instant::now();
    let timer = tokio::spawn(tokio::time::sleep(secs(5)));
    settle().await;
    assert!(started.elapsed() < ms(500), "{:?}", started.elapsed());
    timer.await.unwrap();
}

/// `later` came at least `wait` after `earlier` and less than `wait + slack`
/// plus one step of clock quantisation after it.
fn assert_gap(earlier: &Sent, later: &Sent, wait: Duration, step: Duration) {
    let gap = later.at - earlier.at;
    assert!(
        gap >= wait && gap < wait + step * 2,
        "waited {gap:?}, expected at least {wait:?} and under {:?}",
        wait + step * 2
    );
}

fn ms(whole: u64) -> Duration {
    Duration::from_millis(whole)
}

fn services_of(found: &[Sent]) -> Vec<u16> {
    found.iter().map(|sent| sent.service).collect()
}

#[test]
fn the_ack_wait_figures_and_defaults_are_the_documented_ones() {
    assert_eq!(TunnelAckWait::default(), TunnelAckWait::Standard);
    assert_eq!(
        TunnelAckWait::Standard.timeout(),
        Duration::from_millis(1000)
    );
    assert_eq!(
        TunnelAckWait::Patient.timeout(),
        Duration::from_millis(10_000)
    );
    let client = KnxNetIpClient::new();
    assert_eq!(client.tunnel_ack_wait, TunnelAckWait::Standard);
    let patient = KnxNetIpClient::with_tunnel_return_path(TunnelReturnPath::RouteBack)
        .with_tunnel_ack_wait(TunnelAckWait::Patient);
    assert_eq!(patient.tunnel_ack_wait, TunnelAckWait::Patient);
    assert_eq!(
        patient.tunnel_return_path,
        TunnelReturnPath::RouteBack,
        "the builder keeps the return path"
    );
}

/// Wall-clock witness: an ACK 1.3 s after the request is accepted by a
/// patient client with exactly one datagram on the wire. The old fixed 1 s
/// window repeated the request at 1.0 s.
#[tokio::test]
async fn real_udp_patient_send_accepts_a_1300_ms_late_ack_with_one_datagram() {
    let (client, gateway) = tunnel_with(0x30, HEARTBEAT, TunnelAckWait::Patient).await;
    let sending = tokio::spawn(async move { group_write(&client).await });
    let request = gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    let mut buf = [0u8; 256];
    let early = tokio::time::timeout(
        Duration::from_millis(1300),
        gateway.socket.recv_from(&mut buf),
    )
    .await;
    assert!(early.is_err(), "no repetition within 1.3 s: {early:?}");
    gateway
        .send(tunnelling::TUNNELLING_ACK, &WIRE_ACK(0x30, 0))
        .await;
    sending
        .await
        .expect("send task")
        .expect("late ACK accepted");
    assert_eq!(
        tunnelling::decode_tunnelling_request(&request)
            .unwrap()
            .sequence_counter,
        0
    );
    gateway.expect_quiet().await;
}

/// The same fake gateway through the public `KnxNetIpClient` builder, so the
/// client really hands the patient wait to the tunnel it connects.
#[tokio::test]
async fn real_udp_client_builder_hands_the_patient_wait_to_its_tunnel() {
    let (client, gateway) = tunnel_via(0x31, |address| async move {
        KnxNetIpClient::with_tunnel_return_path(TunnelReturnPath::default())
            .with_tunnel_ack_wait(TunnelAckWait::Patient)
            .connect_tunnel(address)
            .await
    })
    .await;
    let sending = tokio::spawn(async move { group_write(&client).await });
    gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    let mut buf = [0u8; 256];
    let early = tokio::time::timeout(
        Duration::from_millis(1300),
        gateway.socket.recv_from(&mut buf),
    )
    .await;
    assert!(early.is_err(), "no repetition within 1.3 s: {early:?}");
    gateway
        .send(tunnelling::TUNNELLING_ACK, &WIRE_ACK(0x31, 0))
        .await;
    sending
        .await
        .expect("send task")
        .expect("late ACK accepted");
}

/// Without the builder the default client keeps the one-second window: the
/// request is repeated at about 1 s (real time, same socket).
#[tokio::test]
async fn real_udp_default_client_still_repeats_after_one_second() {
    let (client, gateway) = tunnel_via(0x32, |address| async move {
        KnxNetIpClient::new().connect_tunnel(address).await
    })
    .await;
    let sending = tokio::spawn(async move { group_write(&client).await });
    let first = gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    let started = Instant::now();
    let second = gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    assert_eq!(first, second, "same sequence and payload");
    let waited = started.elapsed();
    assert!(
        waited >= Duration::from_millis(900) && waited < Duration::from_millis(2500),
        "{waited:?}"
    );
    gateway
        .send(tunnelling::TUNNELLING_ACK, &WIRE_ACK(0x32, 0))
        .await;
    sending.await.expect("send task").expect("acknowledged");
}

/// Silence on a standard tunnel: one repetition at 1 s, teardown at 2 s.
#[tokio::test]
async fn a_standard_send_repeats_once_after_one_second_and_gives_up_after_two() {
    let tunnel = paused(0x33, HEARTBEAT, TunnelAckWait::Standard).await;
    let sending = tunnel.write();
    let mut found = tunnel.now().await;
    found.extend(tunnel.watch(ms(3_000), ms(250)).await);
    assert_eq!(
        services_of(&found),
        [
            tunnelling::TUNNELLING_REQUEST,
            tunnelling::TUNNELLING_REQUEST,
            services::DISCONNECT_REQUEST
        ]
    );
    assert_eq!(found[0].body, found[1].body, "same sequence and payload");
    // The clock moves in 250 ms steps, so each gap is exact to one step.
    assert_gap(&found[0], &found[1], ms(1_000), ms(250));
    assert_gap(&found[1], &found[2], ms(1_000), ms(250));
    assert!(matches!(
        sending.await.unwrap(),
        Err(BusError::TunnelClosed(TunnelClosure::Unacknowledged {
            sequence: 0,
            ..
        }))
    ));
}

/// Silence: the original, one repetition at 10 s with the same sequence and
/// payload, teardown at 20 s, and nothing else.
#[tokio::test]
async fn a_patient_send_repeats_once_at_ten_seconds_then_tears_down_at_twenty() {
    let tunnel = paused(0x34, HEARTBEAT, TunnelAckWait::Patient).await;
    let sending = tunnel.write();
    let mut found = tunnel.now().await;
    found.extend(tunnel.watch(secs(25), ms(500)).await);
    assert_eq!(
        services_of(&found),
        [
            tunnelling::TUNNELLING_REQUEST,
            tunnelling::TUNNELLING_REQUEST,
            services::DISCONNECT_REQUEST
        ]
    );
    assert_eq!(found[0].body, found[1].body, "same sequence and payload");
    // The clock moves in 500 ms steps, so each gap is exact to one step.
    assert_gap(&found[0], &found[1], secs(10), ms(500));
    assert_gap(&found[1], &found[2], secs(10), ms(500));
    match sending.await.unwrap() {
        Err(BusError::TunnelClosed(TunnelClosure::Unacknowledged {
            sequence: 0,
            last_status: None,
        })) => {}
        other => panic!("unexpected {other:?}"),
    }
}

/// An ACK for another sequence or another channel does not end the wait; the
/// matching one still does, long after the old window and without a repeat.
#[tokio::test]
async fn a_patient_send_ignores_acks_for_another_sequence_or_channel() {
    let tunnel = paused(0x35, HEARTBEAT, TunnelAckWait::Patient).await;
    let mut sending = tunnel.write();
    let mut found = tunnel.now().await;
    found.extend(
        tunnel
            .send(tunnelling::TUNNELLING_ACK, &WIRE_ACK(0x35, 9))
            .await,
    );
    found.extend(
        tunnel
            .send(tunnelling::TUNNELLING_ACK, &WIRE_ACK(0x36, 0))
            .await,
    );
    found.extend(tunnel.watch(secs(6), ms(500)).await);
    assert_eq!(services_of(&found), [tunnelling::TUNNELLING_REQUEST]);
    assert!(!sending.is_finished(), "still waiting for the matching ACK");
    found.extend(
        tunnel
            .send(tunnelling::TUNNELLING_ACK, &WIRE_ACK(0x35, 0))
            .await,
    );
    assert_eq!(
        (&mut sending).await.unwrap().ok(),
        Some(()),
        "matching ACK accepted"
    );
    found.extend(tunnel.watch(secs(15), ms(500)).await);
    assert_eq!(services_of(&found), [tunnelling::TUNNELLING_REQUEST]);
}

/// The gateway ends the tunnel two seconds into a ten-second wait: the send
/// ends at once with that cause, without the clock moving at all.
#[tokio::test]
async fn a_gateway_disconnect_cancels_the_extended_wait_promptly() {
    let tunnel = paused(0x37, HEARTBEAT, TunnelAckWait::Patient).await;
    let sending = tunnel.write();
    let mut found = tunnel.now().await;
    found.extend(tunnel.watch(secs(2), ms(500)).await);
    let before = tunnel.start.elapsed();
    tunnel.gateway.disconnect_request().await;
    found.extend(tunnel.now().await);
    assert_eq!(tunnel.start.elapsed(), before, "no virtual time passed");
    assert!(
        sending.is_finished(),
        "ended without waiting out the window"
    );
    assert!(matches!(
        sending.await.unwrap(),
        Err(BusError::TunnelClosed(TunnelClosure::GatewayDisconnect))
    ));
    found.extend(tunnel.watch(secs(12), ms(500)).await);
    assert_eq!(
        services_of(&found),
        [
            tunnelling::TUNNELLING_REQUEST,
            services::DISCONNECT_RESPONSE
        ],
        "no repetition and no teardown request after the gateway's close"
    );
}

/// A heartbeat that fails ends the extended wait long before ten seconds.
#[tokio::test]
async fn a_failed_heartbeat_cancels_the_extended_wait_promptly() {
    let tunnel = paused(0x38, FAST_HEARTBEAT, TunnelAckWait::Patient).await;
    let sending = tunnel.write();
    let mut found = tunnel.now().await;
    found.extend(tunnel.watch(secs(2), ms(25)).await);
    assert!(sending.is_finished(), "ended by the heartbeat");
    assert!(matches!(
        sending.await.unwrap(),
        Err(BusError::TunnelClosed(TunnelClosure::HeartbeatFailed))
    ));
    let last = found.last().expect("datagrams");
    assert_eq!(last.service, services::DISCONNECT_REQUEST);
    assert!(last.at < secs(2), "{:?}", last.at);
    assert_eq!(
        found
            .iter()
            .filter(|sent| sent.service == tunnelling::TUNNELLING_REQUEST)
            .count(),
        1,
        "the heartbeat ended the wait before any repetition"
    );
}

/// While a send waits for its ACK the receiver still acknowledges an inbound
/// request at once, and the heartbeat keeps its cadence.
#[tokio::test]
async fn inbound_requests_and_heartbeats_progress_during_the_extended_wait() {
    let heartbeat = HeartbeatTiming {
        interval: secs(2),
        response_timeout: secs(1),
    };
    let tunnel = paused(0x39, heartbeat, TunnelAckWait::Patient).await;
    let mut events = tunnel.client.subscribe();
    let sending = tunnel.write();
    let mut found = tunnel.now().await;
    let inbound = tunnelling::encode_tunnelling_request(0x39, 0, &L_DATA_IND);
    found.extend(tunnel.send(tunnelling::TUNNELLING_REQUEST, &inbound).await);
    let ack = found.last().expect("an ACK for the inbound request");
    assert_eq!(ack.service, tunnelling::TUNNELLING_ACK);
    assert_eq!(ack.body, tunnelling::encode_tunnelling_ack(0x39, 0, 0));
    assert_eq!(ack.at, Duration::ZERO, "acknowledged at once");
    assert!(matches!(events.try_recv(), Ok(TunnelEvent::Telegram(_))));
    let mut beats = 0;
    for _step in 0..14 {
        for sent in tunnel.step(ms(500)).await {
            if sent.service == services::CONNECTIONSTATE_REQUEST {
                beats += 1;
                tunnel
                    .send(
                        services::CONNECTIONSTATE_RESPONSE,
                        &[0x39, services::E_NO_ERROR],
                    )
                    .await;
            }
        }
    }
    assert!(beats >= 3, "a beat about every 2 s over 7 s, saw {beats}");
    assert!(
        !sending.is_finished(),
        "still inside the first 10 s attempt"
    );
    tunnel
        .send(tunnelling::TUNNELLING_ACK, &WIRE_ACK(0x39, 0))
        .await;
    assert!(sending.await.unwrap().is_ok());
}
