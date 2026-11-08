//! Real loopback TCP tests for the optional TCP tunnel: handshake, framing, lifecycle, teardown.
//!
//! A fake gateway listens on 127.0.0.1 and scripts each case on an actual
//! TCP socket. No real gateway, bus, discovery, multicast or host network is
//! involved. Expected wire bytes are written as literals or built from the
//! codec under test only where the codec itself is not what is being proven.

use super::tcp::{self, TcpPolicy};
use super::*;
use crate::cemi::{GroupValue, LDataMessageKind, Tpci};
use crate::core::hpai::TCP_ROUTE_BACK;
use crate::frame_stream::{SkipReason, StreamDecoder, StreamItem};
use crate::wire_tap::{Redaction, RedactionKind, TunnelTapEvent, WireRecord, WireSink};
use knx_core::{GroupAddress, GroupAddressStyle};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::time::{timeout, Instant};

/// Longer than any reply in these scripts, shorter than a test hang.
const STEP: Duration = Duration::from_secs(3);
/// How long "nothing further arrives" is watched for.
const QUIET: Duration = Duration::from_millis(200);
/// The channel the fake gateway assigns: deliberately not zero.
const CHANNEL: u8 = 0x2A;
/// An L_Data.ind group write (1.1.1 -> 0/0/1, value 1).
const L_DATA_IND: [u8; 11] = [
    0x29, 0x00, 0xbc, 0xe0, 0x11, 0x01, 0x00, 0x01, 0x01, 0x00, 0x81,
];
/// The same group write with value 0, to tell two telegrams apart.
const L_DATA_IND_ZERO: [u8; 11] = [
    0x29, 0x00, 0xbc, 0xe0, 0x11, 0x01, 0x00, 0x01, 0x01, 0x00, 0x80,
];
/// Core v01.06.02 AS §7.8.1 Figure 35 with the TCP Route Back HPAI of
/// §8.6.2.2 twice and the basic link-layer CRI, as computed by hand.
const CONNECT_REQUEST_FRAME: [u8; 26] = [
    0x06, 0x10, 0x02, 0x05, 0x00, 0x1A, // header, total 26
    0x08, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // control HPAI, TCP Route Back
    0x08, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // data HPAI, TCP Route Back
    0x04, 0x04, 0x02, 0x00, // CRI: tunnel connection, link layer
];
const SENTINEL_KEY: [u8; 4] = [0xC0, 0xFF, 0xEE, 0x42];

/// A heartbeat that never fires within a test, unless a test asks for one.
const SILENT_HEARTBEAT: HeartbeatTiming = HeartbeatTiming {
    interval: Duration::from_secs(3600),
    response_timeout: Duration::from_secs(10),
};

/// Gives up after about 300 ms: four 50 ms waits, 100 ms in.
const FAST_HEARTBEAT: HeartbeatTiming = HeartbeatTiming {
    interval: Duration::from_millis(100),
    response_timeout: Duration::from_millis(50),
};

fn policy() -> TcpPolicy {
    TcpPolicy {
        heartbeat: SILENT_HEARTBEAT,
        ..tcp::PRODUCTION
    }
}

/// The fake gateway's end of one accepted connection.
struct TcpGateway {
    stream: TcpStream,
}

impl TcpGateway {
    async fn write(&mut self, bytes: &[u8]) {
        self.stream
            .write_all(bytes)
            .await
            .expect("fake gateway write");
    }

    async fn send(&mut self, service: u16, body: &[u8]) {
        self.write(&frame::encode_frame(service, body)).await;
    }

    /// The next whole frame from the client, cut by its header's length.
    async fn frame(&mut self) -> Vec<u8> {
        timeout(STEP, async {
            let mut header = [0u8; 6];
            self.stream
                .read_exact(&mut header)
                .await
                .expect("frame header");
            let total = usize::from(u16::from_be_bytes([header[4], header[5]]));
            let mut frame = header.to_vec();
            frame.resize(total, 0);
            self.stream
                .read_exact(&mut frame[6..])
                .await
                .expect("frame body");
            frame
        })
        .await
        .expect("a frame from the client")
    }

    /// The next frame, which must carry `service`; returns its body.
    async fn expect(&mut self, service: u16) -> Vec<u8> {
        let frame = self.frame().await;
        let (header, body) = frame::decode_frame(&frame).expect("a KNXnet/IP frame");
        assert_eq!(
            header.service_type, service,
            "expected service {service:#06x}"
        );
        body.to_vec()
    }

    /// Nothing at all arrives within `limit`, and the stream stays open.
    async fn expect_quiet_for(&mut self, limit: Duration) {
        let mut buf = [0u8; 64];
        match timeout(limit, self.stream.read(&mut buf)).await {
            Err(_) => {}
            Ok(Ok(0)) => panic!("the client closed the stream"),
            Ok(Ok(n)) => panic!("unexpected octets {:02x?}", &buf[..n]),
            Ok(Err(error)) => panic!("stream failed: {error}"),
        }
    }

    async fn expect_quiet(&mut self) {
        self.expect_quiet_for(QUIET).await;
    }

    /// The client closed its side with no further octet: a clean FIN.
    async fn expect_eof(&mut self) {
        let mut buf = [0u8; 64];
        let n = timeout(STEP, self.stream.read(&mut buf))
            .await
            .expect("the client closes the stream")
            .expect("a clean close");
        assert_eq!(n, 0, "octets before the close: {:02x?}", &buf[..n]);
    }

    /// The client released the socket with no further octet: FIN, or a
    /// reset when unread octets were still in its receive buffer.
    async fn expect_released(&mut self) {
        let mut buf = [0u8; 64];
        match timeout(STEP, self.stream.read(&mut buf))
            .await
            .expect("the client releases the stream")
        {
            Ok(0) => {}
            Ok(n) => panic!("octets before the close: {:02x?}", &buf[..n]),
            Err(error) => assert_eq!(error.kind(), std::io::ErrorKind::ConnectionReset, "{error}"),
        }
    }
}

async fn listen() -> (TcpListener, SocketAddrV4) {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind loopback gateway");
    let SocketAddr::V4(address) = listener.local_addr().expect("gateway address") else {
        unreachable!("bound an IPv4 socket")
    };
    (listener, address)
}

async fn accept(listener: &TcpListener) -> TcpGateway {
    let (stream, _) = timeout(STEP, listener.accept())
        .await
        .expect("the client connects")
        .expect("accept");
    stream.set_nodelay(true).expect("no delay");
    TcpGateway { stream }
}

/// Channel, status, TCP Route Back data HPAI, CRD assigning 1.1.255.
fn success_response(channel: u8) -> Vec<u8> {
    let mut body = vec![channel, services::E_NO_ERROR];
    body.extend_from_slice(&[0x08, 0x02, 0, 0, 0, 0, 0, 0]);
    body.extend_from_slice(&[0x04, tunnelling::TUNNEL_CONNECTION, 0x11, 0xff]);
    body
}

/// One accepted TCP tunnel on `policy`, after checking its request octets.
async fn tcp_tunnel_with(policy: TcpPolicy, tap: Option<WireTap>) -> (TunnelClient, TcpGateway) {
    let (listener, address) = listen().await;
    let connecting = tokio::spawn(TunnelClient::connect_tcp(address, policy, tap));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    gateway
        .send(services::CONNECT_RESPONSE, &success_response(CHANNEL))
        .await;
    let client = timeout(STEP, connecting)
        .await
        .expect("bounded connect")
        .expect("connect task")
        .expect("loopback TCP tunnel");
    (client, gateway)
}

async fn tcp_tunnel() -> (TunnelClient, TcpGateway) {
    tcp_tunnel_with(policy(), None).await
}

fn connect_error(result: Result<TunnelClient, BusError>) -> BusError {
    match result {
        Ok(_) => panic!("the connect must fail"),
        Err(error) => error,
    }
}

fn group() -> Destination {
    Destination::Group(GroupAddress::parse("0/0/1", GroupAddressStyle::ThreeLevel).unwrap())
}

async fn group_write(client: &TunnelClient) -> Result<(), BusError> {
    client
        .send_frame(
            group(),
            Tpci::UnnumberedData,
            ApplicationService::GroupValueWrite(GroupValue::Short(1)),
        )
        .await
}

/// The cEMI `group_write` puts on the wire, from the codec it relies on.
fn group_write_cemi() -> Vec<u8> {
    cemi::encode_l_data(&LDataFrame {
        kind: LDataMessageKind::Request,
        source: IndividualAddress::from_raw(0),
        destination: group(),
        transport: Tpci::UnnumberedData,
        service: ApplicationService::GroupValueWrite(GroupValue::Short(1)),
        control: None,
    })
    .unwrap()
}

/// An L_Data.ind group write whose value octets are given.
fn indication_with(value: Vec<u8>) -> Vec<u8> {
    cemi::encode_l_data(&LDataFrame {
        kind: LDataMessageKind::Indication,
        source: IndividualAddress::from_raw(0x1101),
        destination: group(),
        transport: Tpci::UnnumberedData,
        service: ApplicationService::GroupValueWrite(GroupValue::Bytes(value)),
        control: None,
    })
    .unwrap()
}

fn tunnelling_request(channel: u8, sequence: u8, cemi: &[u8]) -> Vec<u8> {
    frame::encode_frame(
        tunnelling::TUNNELLING_REQUEST,
        &tunnelling::encode_tunnelling_request(channel, sequence, cemi),
    )
}

fn decoded(cemi: &[u8]) -> LDataFrame {
    cemi::decode_l_data(cemi).expect("a decodable cEMI frame")
}

async fn telegram(events: &mut broadcast::Receiver<TunnelEvent>) -> LDataFrame {
    match timeout(STEP, events.recv()).await {
        Ok(Ok(TunnelEvent::Telegram(telegram))) => telegram,
        other => panic!("expected a telegram, got {other:?}"),
    }
}

async fn closed_event(events: &mut broadcast::Receiver<TunnelEvent>) {
    let event = timeout(STEP, events.recv()).await;
    assert!(
        matches!(event, Ok(Ok(TunnelEvent::Closed))),
        "expected Closed, got {event:?}"
    );
}

/// After the last owner dropped the tunnel, every task that held its state
/// is gone too: the event channel reports its sender closed, no event more.
async fn tasks_reaped(events: &mut broadcast::Receiver<TunnelEvent>) {
    let event = timeout(STEP, events.recv())
        .await
        .expect("the tunnel's tasks end");
    assert!(
        matches!(event, Err(broadcast::error::RecvError::Closed)),
        "expected the channel closed, got {event:?}"
    );
}

fn assert_no_event(events: &mut broadcast::Receiver<TunnelEvent>) {
    let event = events.try_recv();
    assert!(
        matches!(event, Err(broadcast::error::TryRecvError::Empty)),
        "unexpected event {event:?}"
    );
}

/// Real time for the kernel and the client tasks to catch up.
async fn settle() {
    tokio::task::spawn_blocking(|| std::thread::sleep(Duration::from_millis(40)))
        .await
        .expect("settle");
}

// ---- Transport selection ---------------------------------------------------

#[test]
fn udp_is_the_default_and_tcp_is_chosen_only_explicitly() {
    assert_eq!(TunnelTransport::default(), TunnelTransport::Udp);
    assert_eq!(
        KnxNetIpClient::new().tunnel_transport(),
        TunnelTransport::Udp
    );
    assert_eq!(
        KnxNetIpClient::default().tunnel_transport(),
        TunnelTransport::Udp
    );
    assert_eq!(TunnelTransport::Udp.as_str(), "udp");
    assert_eq!(TunnelTransport::Tcp.as_str(), "tcp");
    // The transport builder keeps the other settings, in either order.
    let client = KnxNetIpClient::with_tunnel_return_path(TunnelReturnPath::RouteBack)
        .with_tunnel_transport(TunnelTransport::Tcp)
        .with_tunnel_ack_wait(TunnelAckWait::Patient);
    assert_eq!(client.tunnel_transport(), TunnelTransport::Tcp);
    assert_eq!(client.tunnel_return_path, TunnelReturnPath::RouteBack);
    assert_eq!(client.tunnel_ack_wait, TunnelAckWait::Patient);
}

/// The default `connect_tunnel` still speaks UDP and reports it.
#[tokio::test]
async fn a_default_tunnel_runs_on_udp_and_says_so() {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind gateway");
    let SocketAddr::V4(address) = socket.local_addr().expect("gateway address") else {
        unreachable!("bound an IPv4 socket")
    };
    let connecting =
        tokio::spawn(async move { KnxNetIpClient::new().connect_tunnel(address).await });
    let mut buf = [0u8; 128];
    let (n, client) = timeout(STEP, socket.recv_from(&mut buf))
        .await
        .expect("CONNECT_REQUEST")
        .unwrap();
    let (header, body) = frame::decode_frame(&buf[..n]).unwrap();
    assert_eq!(header.service_type, services::CONNECT_REQUEST);
    assert_eq!(&body[..2], &[0x08, 0x01], "a UDP control HPAI");
    let mut reply = vec![0x07, services::E_NO_ERROR];
    reply.extend_from_slice(
        &Hpai {
            addr: *address.ip(),
            port: address.port(),
        }
        .encode(),
    );
    reply.extend_from_slice(&[0x04, tunnelling::TUNNEL_CONNECTION, 0x11, 0x01]);
    socket
        .send_to(
            &frame::encode_frame(services::CONNECT_RESPONSE, &reply),
            client,
        )
        .await
        .unwrap();
    let tunnel = timeout(STEP, connecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("UDP tunnel");
    assert_eq!(tunnel.transport(), TunnelTransport::Udp);
    assert_eq!(tunnel.skipped_frame_count(), 0);
}

/// A refused TCP connect is an error; nothing is tried over UDP instead,
/// although a UDP socket listens on the very same port.
#[tokio::test]
async fn a_failed_tcp_connect_never_falls_back_to_udp() {
    let (listener, address) = listen().await;
    drop(listener);
    let udp = UdpSocket::bind(SocketAddr::V4(address))
        .await
        .expect("bind the same port number for UDP");
    let result = timeout(
        STEP,
        KnxNetIpClient::new()
            .with_tunnel_transport(TunnelTransport::Tcp)
            .connect_tunnel(address),
    )
    .await
    .expect("bounded connect");
    let error = connect_error(result);
    assert!(
        matches!(&error, BusError::Io(e) if e.kind() == std::io::ErrorKind::ConnectionRefused),
        "{error:?}"
    );
    let mut buf = [0u8; 64];
    assert!(
        timeout(QUIET, udp.recv_from(&mut buf)).await.is_err(),
        "no UDP datagram was sent"
    );
}

// ---- Handshake -------------------------------------------------------------

/// Full lifecycle through the public builder: exact CONNECT_REQUEST, a
/// non-zero channel carried everywhere, sends without acknowledgement,
/// received telegrams, no TUNNELLING_ACK back, graceful disconnect, EOF.
#[tokio::test]
async fn the_public_builder_opens_a_tcp_tunnel_that_sends_receives_and_disconnects() {
    let (listener, address) = listen().await;
    let builder = KnxNetIpClient::new().with_tunnel_transport(TunnelTransport::Tcp);
    let connecting = tokio::spawn(async move { builder.connect_tunnel(address).await });
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    gateway
        .send(services::CONNECT_RESPONSE, &success_response(CHANNEL))
        .await;
    let client = timeout(STEP, connecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("TCP tunnel");
    assert_eq!(client.transport(), TunnelTransport::Tcp);
    assert_eq!(
        client.assigned_address(),
        IndividualAddress::from_raw(0x11ff)
    );
    assert_eq!(client.closure(), None);
    let mut events = client.subscribe();

    for sequence in 0..2u8 {
        timeout(STEP, group_write(&client))
            .await
            .expect("no acknowledgement is awaited")
            .expect("written");
        let mut expected = vec![0x04, CHANNEL, sequence, 0x00];
        expected.extend_from_slice(&group_write_cemi());
        assert_eq!(
            gateway.expect(tunnelling::TUNNELLING_REQUEST).await,
            expected
        );
    }

    // The gateway's L_Data.con for the request and an L_Data.ind still come
    // as TUNNELLING_REQUESTs: no ACK removes them.
    let mut confirmation = group_write_cemi();
    confirmation[0] = 0x2E;
    gateway
        .write(&tunnelling_request(CHANNEL, 0, &confirmation))
        .await;
    gateway
        .write(&tunnelling_request(CHANNEL, 1, &L_DATA_IND))
        .await;
    assert_eq!(telegram(&mut events).await, decoded(&confirmation));
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND));
    gateway.expect_quiet().await;

    let disconnecting = tokio::spawn(client.disconnect());
    let mut expected = vec![CHANNEL, 0x00];
    expected.extend_from_slice(&TCP_ROUTE_BACK);
    assert_eq!(gateway.expect(services::DISCONNECT_REQUEST).await, expected);
    gateway
        .send(
            services::DISCONNECT_RESPONSE,
            &services::encode_disconnect_response(CHANNEL, services::E_NO_ERROR),
        )
        .await;
    timeout(STEP, disconnecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("confirmed disconnect");
    gateway.expect_eof().await;
    closed_event(&mut events).await;
    tasks_reaped(&mut events).await;
}

/// A refusal carries only channel and status (no HPAI, no CRD) and fails
/// the connect with that status; the stream is closed.
#[tokio::test]
async fn a_tcp_refusal_is_judged_on_its_status_alone_and_closes_the_stream() {
    let (listener, address) = listen().await;
    let connecting = tokio::spawn(TunnelClient::connect_tcp(address, policy(), None));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    gateway
        .send(services::CONNECT_RESPONSE, &[0x00, 0x24])
        .await;
    let error = connect_error(timeout(STEP, connecting).await.unwrap().unwrap());
    assert!(
        matches!(error, BusError::ConnectionRefused(0x24)),
        "{error:?}"
    );
    gateway.expect_eof().await;
}

/// No CONNECT_RESPONSE within the response time: `Timeout`, not earlier,
/// and the stream is gone, so a late response can be taken by nobody.
#[tokio::test]
async fn an_absent_connect_response_times_out_and_drops_the_stream() {
    let (listener, address) = listen().await;
    let policy = TcpPolicy {
        connect_response: Duration::from_millis(300),
        ..policy()
    };
    let started = Instant::now();
    let connecting = tokio::spawn(TunnelClient::connect_tcp(address, policy, None));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    let error = connect_error(timeout(STEP, connecting).await.unwrap().unwrap());
    assert!(matches!(error, BusError::Timeout), "{error:?}");
    assert!(started.elapsed() >= Duration::from_millis(300));
    gateway.expect_eof().await;
}

/// A trickle is not a new CONNECT_RESPONSE budget; it must time out while the peer still sends.
#[tokio::test]
async fn a_trickle_of_skipped_frames_cannot_extend_the_connect_deadline() {
    let (listener, address) = listen().await;
    let sink = Arc::new(Collect::default());
    let response_budget = Duration::from_millis(300);
    let policy = TcpPolicy {
        connect_response: response_budget,
        ..policy()
    };
    let started = Instant::now();
    let connecting = tokio::spawn(TunnelClient::connect_tcp(
        address,
        policy,
        Some(WireTap::new(sink.clone())),
    ));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    let trickling = tokio::spawn(async move {
        let unknown = frame::encode_frame(0x0999, &[]);
        let mut ticks = tokio::time::interval(Duration::from_millis(50));
        let mut probe = [0u8; 1];
        loop {
            tokio::select! {
                result = gateway.stream.read(&mut probe) => {
                    match result {
                        Ok(0) => break,
                        Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => break,
                        other => panic!("unexpected client traffic after the connect request: {other:?}"),
                    }
                }
                _ = ticks.tick() => {
                    if gateway.stream.write_all(&unknown).await.is_err() {
                        gateway.expect_released().await;
                        break;
                    }
                }
            }
        }
    });
    let error = connect_error(
        timeout(STEP, connecting)
            .await
            .expect("a trickling peer must not extend the single handshake deadline")
            .unwrap(),
    );
    assert!(matches!(error, BusError::Timeout), "{error:?}");
    assert!(started.elapsed() >= response_budget);
    timeout(STEP, trickling)
        .await
        .expect("the failed handshake releases the stream")
        .unwrap();
    let skips = sink
        .0
        .lock()
        .unwrap()
        .iter()
        .filter(|record| {
            matches!(
                record,
                WireRecord::Event {
                    event: TunnelTapEvent::FrameSkipped { .. },
                    ..
                }
            )
        })
        .count();
    assert!(
        skips >= 4,
        "the peer must actually trickle before the deadline: {skips}"
    );
}

/// Writes `reply` (and optionally closes the gateway's sending side) after
/// the request; returns the connect error and checks the stream is gone.
async fn failed_handshake(reply: &[u8], then_half_close: bool) -> BusError {
    let (listener, address) = listen().await;
    let connecting = tokio::spawn(TunnelClient::connect_tcp(address, policy(), None));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    gateway.write(reply).await;
    if then_half_close {
        gateway.stream.shutdown().await.expect("half-close");
    }
    let error = connect_error(
        timeout(STEP, connecting)
            .await
            .expect("bounded")
            .expect("task"),
    );
    gateway.expect_released().await;
    error
}

fn response_with_data_hpai(hpai: [u8; 8]) -> Vec<u8> {
    let mut body = vec![CHANNEL, services::E_NO_ERROR];
    body.extend_from_slice(&hpai);
    body.extend_from_slice(&[0x04, tunnelling::TUNNEL_CONNECTION, 0x11, 0xff]);
    frame::encode_frame(services::CONNECT_RESPONSE, &body)
}

#[tokio::test]
async fn a_success_without_the_exact_tcp_route_back_data_hpai_is_refused() {
    let cases: [(&str, [u8; 8]); 4] = [
        ("UDP HPAI", [0x08, 0x01, 127, 0, 0, 1, 0x0e, 0x57]),
        ("TCP with a port", [0x08, 0x02, 0, 0, 0, 0, 0x0e, 0x57]),
        ("TCP with an address", [0x08, 0x02, 127, 0, 0, 1, 0, 0]),
        ("wrong structure length", [0x09, 0x02, 0, 0, 0, 0, 0, 0]),
    ];
    for (case, hpai) in cases {
        let error = failed_handshake(&response_with_data_hpai(hpai), false).await;
        assert!(
            matches!(&error, BusError::Protocol(text) if text.contains("TCP Route Back")),
            "{case}: {error:?}"
        );
    }
    // Success status with no data HPAI at all.
    let error = failed_handshake(
        &frame::encode_frame(services::CONNECT_RESPONSE, &[CHANNEL, services::E_NO_ERROR]),
        false,
    )
    .await;
    assert!(matches!(error, BusError::Protocol(_)), "{error:?}");
}

#[tokio::test]
async fn a_wrong_or_broken_handshake_fails_the_connect() {
    // Another known service where the response belongs.
    let error = failed_handshake(
        &frame::encode_frame(
            services::DISCONNECT_REQUEST,
            &services::encode_disconnect_request_with_hpai(CHANNEL, TCP_ROUTE_BACK),
        ),
        false,
    )
    .await;
    assert!(
        matches!(&error, BusError::Protocol(text) if text.contains("expected CONNECT_RESPONSE")),
        "{error:?}"
    );
    // A malformed header.
    let error = failed_handshake(&[0x07, 0x10, 0x02, 0x06, 0x00, 0x14], false).await;
    assert!(matches!(error, BusError::Protocol(_)), "{error:?}");
    // The gateway closes before answering, and inside its answer.
    for reply in [
        Vec::new(),
        frame::encode_frame(services::CONNECT_RESPONSE, &success_response(CHANNEL))[..9].to_vec(),
    ] {
        let error = failed_handshake(&reply, true).await;
        assert!(
            matches!(&error, BusError::Io(e) if e.kind() == std::io::ErrorKind::UnexpectedEof),
            "{error:?}"
        );
    }
}

/// Unknown frames before the response are skipped, header-like body and
/// all; a frame start that arrives with the response is kept for the
/// tunnel and completes later.
#[tokio::test]
async fn frames_around_the_connect_response_are_neither_misread_nor_lost() {
    let (listener, address) = listen().await;
    let connecting = tokio::spawn(TunnelClient::connect_tcp(address, policy(), None));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    let telegram_frame = tunnelling_request(CHANNEL, 0, &L_DATA_IND);
    let mut burst = frame::encode_frame(0x0999, &[0x06, 0x10, 0x02, 0x06, 0x00, 0x14]);
    burst.extend_from_slice(&frame::encode_frame(
        services::CONNECT_RESPONSE,
        &success_response(CHANNEL),
    ));
    burst.extend_from_slice(&telegram_frame[..9]);
    gateway.write(&burst).await;
    let client = timeout(STEP, connecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("TCP tunnel");
    assert_eq!(client.skipped_frame_count(), 1);
    let mut events = client.subscribe();
    gateway.write(&telegram_frame[9..]).await;
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND));
    assert_eq!(client.closure(), None);
}

/// A DISCONNECT_REQUEST that arrives together with the response is still
/// answered on the stream, which is then closed.
#[tokio::test]
async fn a_disconnect_request_coalesced_with_the_response_is_answered() {
    let (listener, address) = listen().await;
    let connecting = tokio::spawn(TunnelClient::connect_tcp(address, policy(), None));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    let mut burst = frame::encode_frame(services::CONNECT_RESPONSE, &success_response(CHANNEL));
    burst.extend_from_slice(&frame::encode_frame(
        services::DISCONNECT_REQUEST,
        &services::encode_disconnect_request_with_hpai(CHANNEL, TCP_ROUTE_BACK),
    ));
    gateway.write(&burst).await;
    let client = timeout(STEP, connecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("TCP tunnel");
    assert_eq!(
        gateway.expect(services::DISCONNECT_RESPONSE).await,
        [CHANNEL, services::E_NO_ERROR]
    );
    gateway.expect_eof().await;
    assert_eq!(client.closure(), Some(TunnelClosure::GatewayDisconnect));
}

// ---- Framing ---------------------------------------------------------------

/// The same telegrams arrive whole whether the gateway writes one octet at
/// a time, several frames at once, or cuts inside headers and bodies; a body
/// holding header-like octets is not split.
#[tokio::test]
async fn telegrams_arrive_whole_whatever_the_read_boundaries() {
    let (client, mut gateway) = tcp_tunnel().await;
    let mut events = client.subscribe();
    let magic = indication_with(vec![
        0x06, 0x10, 0x04, 0x20, 0x00, 0x0f, 0x04, CHANNEL, 0x09, 0x00,
    ]);

    let first = tunnelling_request(CHANNEL, 0, &magic);
    for octet in &first {
        gateway.write(&[*octet]).await;
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    assert_eq!(telegram(&mut events).await, decoded(&magic));

    let mut burst = tunnelling_request(CHANNEL, 1, &L_DATA_IND);
    burst.extend_from_slice(&tunnelling_request(CHANNEL, 2, &magic));
    burst.extend_from_slice(&tunnelling_request(CHANNEL, 3, &L_DATA_IND_ZERO));
    gateway.write(&burst).await;
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND));
    assert_eq!(telegram(&mut events).await, decoded(&magic));
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND_ZERO));

    // Cut inside the second header and inside the second body.
    let first_len = tunnelling_request(CHANNEL, 1, &L_DATA_IND).len();
    for piece in [
        &burst[..first_len + 3],
        &burst[first_len + 3..first_len + 12],
        &burst[first_len + 12..],
    ] {
        gateway.write(piece).await;
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND));
    assert_eq!(telegram(&mut events).await, decoded(&magic));
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND_ZERO));

    gateway.expect_quiet().await;
    assert_no_event(&mut events);
    assert_eq!(client.closure(), None);
    assert_eq!(client.skipped_frame_count(), 0);
}

/// Frames of another channel are read and set aside: no telegram, no
/// DISCONNECT_RESPONSE, no closure; this channel's telegram still arrives.
#[tokio::test]
async fn another_channels_frames_are_not_this_tunnels() {
    let (client, mut gateway) = tcp_tunnel().await;
    let mut events = client.subscribe();
    let other = CHANNEL + 1;
    gateway
        .write(&tunnelling_request(other, 0, &L_DATA_IND_ZERO))
        .await;
    gateway
        .send(
            services::DISCONNECT_REQUEST,
            &services::encode_disconnect_request_with_hpai(other, TCP_ROUTE_BACK),
        )
        .await;
    gateway
        .send(
            services::DISCONNECT_RESPONSE,
            &services::encode_disconnect_response(other, services::E_NO_ERROR),
        )
        .await;
    gateway
        .write(&tunnelling_request(CHANNEL, 0, &L_DATA_IND))
        .await;
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND));
    gateway.expect_quiet().await;
    assert_no_event(&mut events);
    assert_eq!(client.closure(), None);
}

/// With a 64-octet cap, an unknown service and a valid but larger request
/// are drained by their stated length, then the next frame is read; with the
/// production cap, the protocol's largest frame (65535) is drained the same.
#[tokio::test]
async fn unknown_and_oversized_frames_are_drained_and_the_stream_continues() {
    let small_cap = TcpPolicy {
        frame_cap: 64,
        ..policy()
    };
    for (cap_name, policy) in [("64 octets", small_cap), ("production", policy())] {
        let (client, mut gateway) = tcp_tunnel_with(policy, None).await;
        let mut events = client.subscribe();
        let unknown = frame::encode_frame(0x0999, &[0x06, 0x10, 0x04, 0x20, 0x00, 0x0a, 0, 0]);
        // The protocol maximum: 65535 octets of a handled service whose body
        // is full of header-like octets.
        let mut largest_body = tunnelling::encode_tunnelling_request(CHANNEL, 1, &[]);
        while largest_body.len() < 65535 - 6 {
            largest_body.extend_from_slice(&[0x06, 0x10]);
        }
        largest_body.truncate(65535 - 6);
        let largest = frame::encode_frame(tunnelling::TUNNELLING_REQUEST, &largest_body);
        assert_eq!(&largest[4..6], &[0xff, 0xff]);
        gateway.write(&unknown).await;
        let mut expected_skips = 2;
        if policy.frame_cap == 64 {
            let oversize = tunnelling_request(CHANNEL, 0, &indication_with(vec![0xAB; 50]));
            assert!(oversize.len() > 64, "{}", oversize.len());
            gateway.write(&oversize).await;
            expected_skips += 1;
        }
        for piece in largest.chunks(4096) {
            gateway.write(piece).await;
        }
        gateway
            .write(&tunnelling_request(CHANNEL, 2, &L_DATA_IND))
            .await;
        assert_eq!(
            telegram(&mut events).await,
            decoded(&L_DATA_IND),
            "{cap_name}"
        );
        assert_eq!(client.skipped_frame_count(), expected_skips, "{cap_name}");
        assert_eq!(client.closure(), None, "{cap_name}");
        gateway.expect_quiet().await;
        assert_no_event(&mut events);
    }
}

/// A header that is not well formed closes the stream after delivering the
/// frames before it; nothing after it is read.
#[tokio::test]
async fn a_malformed_header_closes_the_stream_after_the_frames_before_it() {
    let cases: [(&str, [u8; 6]); 3] = [
        ("header length 07h", [0x07, 0x10, 0x04, 0x20, 0x00, 0x0a]),
        ("total length 5", [0x06, 0x10, 0x04, 0x20, 0x00, 0x05]),
        ("total length 0", [0x06, 0x10, 0x04, 0x20, 0x00, 0x00]),
    ];
    for (case, garbage) in cases {
        let (client, mut gateway) = tcp_tunnel().await;
        let mut events = client.subscribe();
        let mut bytes = tunnelling_request(CHANNEL, 0, &L_DATA_IND);
        bytes.extend_from_slice(&garbage);
        bytes.extend_from_slice(&tunnelling_request(CHANNEL, 1, &L_DATA_IND_ZERO));
        gateway.write(&bytes).await;
        assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND), "{case}");
        closed_event(&mut events).await;
        assert_eq!(
            client.closure(),
            Some(TunnelClosure::MalformedStream),
            "{case}"
        );
        gateway.expect_released().await;
        let error = group_write(&client).await.unwrap_err();
        assert!(
            matches!(
                error,
                BusError::TunnelClosed(TunnelClosure::MalformedStream)
            ),
            "{case}: {error:?}"
        );
    }
}

/// Local policy: a version other than 1.0 closes the stream with nothing
/// sent back; no acknowledgement is fabricated.
#[tokio::test]
async fn an_unsupported_version_closes_the_stream_without_a_reply() {
    let (client, mut gateway) = tcp_tunnel().await;
    let mut events = client.subscribe();
    let mut bytes = tunnelling_request(CHANNEL, 0, &L_DATA_IND);
    bytes[1] = 0x20;
    gateway.write(&bytes).await;
    closed_event(&mut events).await;
    assert_eq!(
        client.closure(),
        Some(TunnelClosure::UnsupportedVersion { version: 0x20 })
    );
    gateway.expect_released().await;
    assert_eq!(
        BusError::TunnelClosed(TunnelClosure::UnsupportedVersion { version: 0x20 }).to_string(),
        "tunnel closed: the gateway sent KNXnet/IP protocol version 0x20 on the TCP connection, \
         which this client does not support"
    );
}

// ---- Sequence and acknowledgement ------------------------------------------

/// Repeated, reset and jumping sequence counters are all delivered, none is
/// acknowledged; a received ACK is ignored; a send is never repeated after
/// the UDP one-second window.
#[tokio::test]
async fn received_sequences_are_not_evaluated_and_nothing_is_acknowledged() {
    let (client, mut gateway) = tcp_tunnel().await;
    let mut events = client.subscribe();
    for sequence in [5u8, 5, 0, 200] {
        gateway
            .write(&tunnelling_request(CHANNEL, sequence, &L_DATA_IND))
            .await;
    }
    for _ in 0..4 {
        assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND));
    }
    gateway.expect_quiet().await;

    gateway
        .send(
            tunnelling::TUNNELLING_ACK,
            &tunnelling::encode_tunnelling_ack(CHANNEL, 9, 0x29),
        )
        .await;
    timeout(STEP, group_write(&client))
        .await
        .expect("bounded")
        .expect("written without an acknowledgement");
    let body = gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    assert_eq!(&body[..4], &[0x04, CHANNEL, 0x00, 0x00]);
    gateway.expect_quiet_for(Duration::from_millis(1500)).await;
    assert_eq!(client.closure(), None);
    assert_no_event(&mut events);
}

// ---- Heartbeat -------------------------------------------------------------

#[tokio::test]
async fn an_answered_heartbeat_over_tcp_keeps_the_tunnel() {
    let heartbeat = HeartbeatTiming {
        interval: Duration::from_millis(200),
        response_timeout: Duration::from_millis(1000),
    };
    let (client, mut gateway) = tcp_tunnel_with(
        TcpPolicy {
            heartbeat,
            ..policy()
        },
        None,
    )
    .await;
    let mut expected = vec![CHANNEL, 0x00];
    expected.extend_from_slice(&TCP_ROUTE_BACK);
    for _ in 0..3 {
        assert_eq!(
            gateway.expect(services::CONNECTIONSTATE_REQUEST).await,
            expected
        );
        gateway
            .send(
                services::CONNECTIONSTATE_RESPONSE,
                &[CHANNEL, services::E_NO_ERROR],
            )
            .await;
    }
    assert_eq!(client.closure(), None);
}

/// A wrong-channel answer and an error status are not a healthy heartbeat:
/// the original and three repetitions, then DISCONNECT_REQUEST, and the
/// stream closes at once without waiting for DISCONNECT_RESPONSE.
#[tokio::test]
async fn an_unhealthy_heartbeat_repeats_three_times_then_disconnects_and_closes() {
    let (client, mut gateway) = tcp_tunnel_with(
        TcpPolicy {
            heartbeat: FAST_HEARTBEAT,
            ..policy()
        },
        None,
    )
    .await;
    let mut events = client.subscribe();
    gateway.expect(services::CONNECTIONSTATE_REQUEST).await;
    gateway
        .send(
            services::CONNECTIONSTATE_RESPONSE,
            &[CHANNEL + 1, services::E_NO_ERROR],
        )
        .await;
    gateway.expect(services::CONNECTIONSTATE_REQUEST).await;
    gateway
        .send(services::CONNECTIONSTATE_RESPONSE, &[CHANNEL, 0x21])
        .await;
    gateway.expect(services::CONNECTIONSTATE_REQUEST).await;
    gateway.expect(services::CONNECTIONSTATE_REQUEST).await;
    let mut expected = vec![CHANNEL, 0x00];
    expected.extend_from_slice(&TCP_ROUTE_BACK);
    assert_eq!(gateway.expect(services::DISCONNECT_REQUEST).await, expected);
    gateway.expect_released().await;
    closed_event(&mut events).await;
    assert_eq!(client.closure(), Some(TunnelClosure::HeartbeatFailed));
    let started = Instant::now();
    let error = client.disconnect().await.unwrap_err();
    assert!(
        matches!(
            error,
            BusError::TunnelClosed(TunnelClosure::HeartbeatFailed)
        ),
        "{error:?}"
    );
    assert!(started.elapsed() < Duration::from_millis(500));
}

/// The gateway starts a frame whose stated length swallows its later
/// heartbeat answers. Framing does not resynchronise on them, and the
/// heartbeat, running on its own, still ends the dead stream.
#[tokio::test]
async fn a_stalled_frame_cannot_hide_a_dead_heartbeat() {
    let (client, mut gateway) = tcp_tunnel_with(
        TcpPolicy {
            heartbeat: FAST_HEARTBEAT,
            ..policy()
        },
        None,
    )
    .await;
    // A header stating 255 octets: 249 body octets still to come.
    gateway.write(&[0x06, 0x10, 0x04, 0x20, 0x00, 0xff]).await;
    for _ in 0..4 {
        gateway.expect(services::CONNECTIONSTATE_REQUEST).await;
        gateway
            .send(
                services::CONNECTIONSTATE_RESPONSE,
                &[CHANNEL, services::E_NO_ERROR],
            )
            .await;
    }
    gateway.expect(services::DISCONNECT_REQUEST).await;
    gateway.expect_released().await;
    assert_eq!(client.closure(), Some(TunnelClosure::HeartbeatFailed));
}

/// C §8.4.3.3: with the inner connection active, a receive stall inside a
/// frame does not close the stream, not after the 10 s byte-idle limit
/// that applies before a connection exists, nor long after it. The parser
/// keeps its place: the rest of the frame completes the telegram.
///
/// Two controls keep this discriminating: a 10 s timer started on the same
/// paused clock does fire, so a 10 s receive timeout in the reader would
/// have fired too; and the clock moved by exactly the 25 s advanced, not
/// on to the next timer, so the absence of a close is not a clock artefact.
#[tokio::test]
async fn an_established_stream_survives_a_long_stall_inside_a_frame() {
    let (client, mut gateway) = tcp_tunnel().await;
    let mut events = client.subscribe();
    let frame = tunnelling_request(CHANNEL, 0, &L_DATA_IND);
    gateway.write(&frame[..8]).await;
    settle().await;
    tokio::time::pause();
    let paused_at = Instant::now();
    let ten_second_timer = tokio::spawn(tokio::time::sleep(Duration::from_secs(10)));
    tokio::time::advance(Duration::from_secs(25)).await;
    settle().await;
    let moved = paused_at.elapsed();
    assert!(
        moved >= Duration::from_secs(25) && moved < Duration::from_secs(26),
        "{moved:?}"
    );
    assert!(
        ten_second_timer.is_finished(),
        "a 10 s timer on this clock fires"
    );
    assert_eq!(client.closure(), None);
    let mut probe = [0u8; 8];
    assert_eq!(
        gateway
            .stream
            .try_read(&mut probe)
            .expect_err("no octet and no close from the client")
            .kind(),
        std::io::ErrorKind::WouldBlock
    );
    tokio::time::resume();
    gateway.write(&frame[8..]).await;
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND));
    assert_no_event(&mut events);
}

// ---- End of stream ---------------------------------------------------------

/// The gateway half-closes after `prefix`: the client closes the whole
/// connection at once, sends refuse, and `disconnect()` treats the closed
/// stream as the end of its connection.
async fn peer_half_close(prefix: &[u8]) {
    let (client, mut gateway) = tcp_tunnel().await;
    let mut events = client.subscribe();
    gateway.write(prefix).await;
    let started = Instant::now();
    gateway.stream.shutdown().await.expect("half-close");
    closed_event(&mut events).await;
    gateway.expect_released().await;
    assert!(started.elapsed() < Duration::from_millis(1000));
    assert_eq!(client.closure(), Some(TunnelClosure::PeerClosed));
    let error = group_write(&client).await.unwrap_err();
    assert!(
        matches!(error, BusError::TunnelClosed(TunnelClosure::PeerClosed)),
        "{error:?}"
    );
    timeout(STEP, client.disconnect())
        .await
        .expect("no wait for a response")
        .expect("the gateway closing the stream ended the connection");
}

#[tokio::test]
async fn a_half_close_between_frames_closes_the_connection() {
    peer_half_close(&[]).await;
}

#[tokio::test]
async fn a_half_close_inside_a_header_closes_the_connection() {
    peer_half_close(&tunnelling_request(CHANNEL, 0, &L_DATA_IND)[..3]).await;
}

#[tokio::test]
async fn a_half_close_inside_a_body_closes_the_connection() {
    peer_half_close(&tunnelling_request(CHANNEL, 0, &L_DATA_IND)[..9]).await;
}

#[tokio::test]
async fn a_half_close_inside_a_skipped_frame_closes_the_connection() {
    let mut prefix = vec![0x06, 0x10, 0x04, 0x20, 0xff, 0xff];
    prefix.extend_from_slice(&[0x06; 1000]);
    peer_half_close(&prefix).await;
}

/// A heartbeat waiting for its 10 s answer does not hold up the close.
#[tokio::test]
async fn a_half_close_during_a_heartbeat_wait_closes_at_once() {
    let heartbeat = HeartbeatTiming {
        interval: Duration::from_millis(100),
        response_timeout: Duration::from_secs(10),
    };
    let (client, mut gateway) = tcp_tunnel_with(
        TcpPolicy {
            heartbeat,
            ..policy()
        },
        None,
    )
    .await;
    gateway.expect(services::CONNECTIONSTATE_REQUEST).await;
    let started = Instant::now();
    gateway.stream.shutdown().await.expect("half-close");
    gateway.expect_released().await;
    assert!(started.elapsed() < Duration::from_millis(1000));
    assert_eq!(client.closure(), Some(TunnelClosure::PeerClosed));
}

/// A `disconnect()` waiting for its response ends when the gateway closes
/// the stream instead, well inside the response budget.
#[tokio::test]
async fn a_half_close_answers_a_waiting_disconnect() {
    let (client, mut gateway) = tcp_tunnel().await;
    let disconnecting = tokio::spawn(client.disconnect());
    gateway.expect(services::DISCONNECT_REQUEST).await;
    let started = Instant::now();
    gateway.stream.shutdown().await.expect("half-close");
    timeout(STEP, disconnecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("the closed stream ended the connection");
    assert!(started.elapsed() < Duration::from_millis(1000));
    gateway.expect_released().await;
}

#[tokio::test]
async fn a_reset_ends_the_tunnel_as_a_socket_failure() {
    let (client, gateway) = tcp_tunnel().await;
    let mut events = client.subscribe();
    socket2::SockRef::from(&gateway.stream)
        .set_linger(Some(Duration::ZERO))
        .expect("linger zero");
    drop(gateway); // closes with RST
    closed_event(&mut events).await;
    assert_eq!(client.closure(), Some(TunnelClosure::SocketFailed));
    let error = group_write(&client).await.unwrap_err();
    assert!(
        matches!(error, BusError::TunnelClosed(TunnelClosure::SocketFailed)),
        "{error:?}"
    );
    let error = timeout(STEP, client.disconnect())
        .await
        .expect("bounded")
        .unwrap_err();
    assert!(
        matches!(error, BusError::TunnelClosed(TunnelClosure::SocketFailed)),
        "{error:?}"
    );
    tasks_reaped(&mut events).await;
}

/// The gateway's DISCONNECT_REQUEST is answered on the same stream, then
/// the stream closes; later sends refuse and `disconnect()` is satisfied.
#[tokio::test]
async fn a_gateway_disconnect_is_answered_on_the_stream_which_then_closes() {
    let (client, mut gateway) = tcp_tunnel().await;
    let mut events = client.subscribe();
    gateway
        .send(
            services::DISCONNECT_REQUEST,
            &services::encode_disconnect_request_with_hpai(CHANNEL, TCP_ROUTE_BACK),
        )
        .await;
    assert_eq!(
        gateway.expect(services::DISCONNECT_RESPONSE).await,
        [CHANNEL, services::E_NO_ERROR]
    );
    gateway.expect_eof().await;
    closed_event(&mut events).await;
    assert_eq!(client.closure(), Some(TunnelClosure::GatewayDisconnect));
    let error = group_write(&client).await.unwrap_err();
    assert!(
        matches!(
            error,
            BusError::TunnelClosed(TunnelClosure::GatewayDisconnect)
        ),
        "{error:?}"
    );
    client.disconnect().await.expect("the gateway ended it");
    tasks_reaped(&mut events).await;
}

// ---- Cancellation, drop and cleanup ----------------------------------------

#[tokio::test]
async fn cancelling_the_connect_before_its_response_drops_the_stream() {
    let (listener, address) = listen().await;
    let connecting = tokio::spawn(TunnelClient::connect_tcp(address, policy(), None));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    connecting.abort();
    assert!(matches!(connecting.await, Err(error) if error.is_cancelled()));
    gateway.expect_released().await;
}

/// The gateway has assigned a channel when the connect is cancelled. If the
/// cancel beat the handshake the future's stream is dropped; if the
/// handshake beat the cancel the finished tunnel is dropped. Either way no
/// task keeps the stream.
#[tokio::test]
async fn cancelling_the_connect_after_the_channel_was_assigned_leaves_no_stream() {
    let (listener, address) = listen().await;
    let connecting = tokio::spawn(TunnelClient::connect_tcp(address, policy(), None));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    gateway
        .send(services::CONNECT_RESPONSE, &success_response(CHANNEL))
        .await;
    connecting.abort();
    drop(connecting.await);
    gateway.expect_released().await;
}

/// Dropped inside a partial frame, and inside a frame being skipped: the
/// stream is released, `Closed` is sent once, and every task is gone.
#[tokio::test]
async fn dropping_a_tunnel_mid_frame_releases_the_stream_and_its_tasks() {
    let mut skipped = vec![0x06, 0x10, 0x04, 0x20, 0xff, 0xff];
    skipped.extend_from_slice(&[0x06; 1000]);
    let partial = tunnelling_request(CHANNEL, 0, &L_DATA_IND)[..9].to_vec();
    for prefix in [partial, skipped] {
        let (client, mut gateway) = tcp_tunnel().await;
        let mut events = client.subscribe();
        gateway.write(&prefix).await;
        settle().await;
        drop(client);
        closed_event(&mut events).await;
        tasks_reaped(&mut events).await;
        gateway.expect_released().await;
    }
}

fn big_cemi() -> Vec<u8> {
    cemi::encode_l_data(&LDataFrame {
        kind: LDataMessageKind::Request,
        source: IndividualAddress::from_raw(0),
        destination: group(),
        transport: Tpci::UnnumberedData,
        service: ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0x5A; 200])),
        control: None,
    })
    .unwrap()
}

async fn big_write(client: &TunnelClient) -> Result<(), BusError> {
    client
        .send_frame(
            group(),
            Tpci::UnnumberedData,
            ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0x5A; 200])),
        )
        .await
}

/// Every frame is this channel's whole `big_cemi` request, numbered on.
fn assert_consecutive_requests(frames: &[Vec<u8>]) {
    let cemi = big_cemi();
    for (index, frame) in frames.iter().enumerate() {
        let (header, body) = frame::decode_frame(frame).expect("a whole frame");
        assert_eq!(header.service_type, tunnelling::TUNNELLING_REQUEST);
        let request = tunnelling::decode_tunnelling_request(body).expect("request");
        assert_eq!(request.channel_id, CHANNEL);
        assert_eq!(request.sequence_counter, index as u8, "frame {index}");
        assert_eq!(request.cemi, cemi.as_slice(), "frame {index}");
    }
}

/// Real socket, no gate: a gateway that stops reading fills the socket
/// buffers until one write outlasts the write budget, which closes the
/// stream. The send's failure is the evidence that the write blocked;
/// nothing is inferred from how long a send took.
#[tokio::test]
async fn a_peer_that_stops_reading_is_closed_by_the_write_budget() {
    let (client, mut gateway) = tcp_tunnel_with(
        TcpPolicy {
            write: Duration::from_millis(300),
            ..policy()
        },
        None,
    )
    .await;
    let mut events = client.subscribe();
    let (completed, error) = timeout(STEP, async {
        let mut completed = 0usize;
        loop {
            match big_write(&client).await {
                Ok(()) => completed += 1,
                Err(error) => return (completed, error),
            }
            assert!(completed < 200_000, "the socket buffers never filled");
        }
    })
    .await
    .expect("the write budget ends the blocked write");
    assert!(completed > 0);
    assert!(
        matches!(
            error,
            BusError::TunnelClosed(TunnelClosure::StreamWriteFailed)
        ),
        "{error:?}"
    );
    assert_eq!(client.closure(), Some(TunnelClosure::StreamWriteFailed));
    closed_event(&mut events).await;

    // What reached the gateway: whole consecutive requests, the last one
    // possibly cut by the close, then the end of the stream.
    let mut decoder = StreamDecoder::new(usize::from(u16::MAX), |_| true);
    let mut items = Vec::new();
    let mut buf = vec![0u8; 65536];
    loop {
        match timeout(STEP, gateway.stream.read(&mut buf))
            .await
            .expect("the stream ends")
        {
            Ok(0) => break,
            Ok(n) => decoder
                .push(&buf[..n], &mut items)
                .expect("only well-formed frames"),
            Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => break,
            Err(error) => panic!("{error}"),
        }
    }
    let frames: Vec<Vec<u8>> = items
        .into_iter()
        .map(|item| match item {
            StreamItem::Frame(frame) => frame,
            other => panic!("unexpected {other:?}"),
        })
        .collect();
    assert!(
        (completed..=completed + 1).contains(&frames.len()),
        "{} frames for {completed} completed sends",
        frames.len()
    );
    assert_consecutive_requests(&frames);
}

// ---- Gated writer: ordering, teardown and bounds ---------------------------
//
// These tests hold the real writer task at a test-only gate inside a frame,
// on a real loopback stream. The gate reports each frame the writer takes,
// so where the writer is is known, not inferred from timing.

/// A TCP tunnel whose writer pauses inside frames until the test grants a
/// permit; see `tcp::WriterGate`.
struct Gated {
    client: Arc<TunnelClient>,
    gateway: TcpGateway,
    gate: Arc<tcp::WriterGate>,
    taken: tokio::sync::mpsc::UnboundedReceiver<Vec<u8>>,
}

async fn gated_tunnel(
    policy: TcpPolicy,
    split: usize,
    pause_service: Option<u16>,
    tap: Option<WireTap>,
) -> Gated {
    let (taken_tx, taken) = tokio::sync::mpsc::unbounded_channel();
    let gate = Arc::new(tcp::WriterGate {
        split,
        pause_service,
        taken: taken_tx,
        permits: tokio::sync::Semaphore::new(0),
    });
    let (listener, address) = listen().await;
    let hooks = tcp::Hooks {
        writer_gate: Some(gate.clone()),
    };
    let connecting = tokio::spawn(TunnelClient::connect_tcp_hooked(
        address, policy, tap, hooks,
    ));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    gateway
        .send(services::CONNECT_RESPONSE, &success_response(CHANNEL))
        .await;
    let client = timeout(STEP, connecting)
        .await
        .expect("bounded connect")
        .expect("connect task")
        .expect("gated TCP tunnel");
    Gated {
        client: Arc::new(client),
        gateway,
        gate,
        taken,
    }
}

impl Gated {
    /// The next frame the writer took.
    async fn taken(&mut self) -> Vec<u8> {
        timeout(STEP, self.taken.recv())
            .await
            .expect("the writer takes a frame")
            .expect("the gate is alive")
    }

    /// The writer has taken no further frame.
    fn nothing_taken(&mut self) {
        let next = self.taken.try_recv();
        assert!(
            matches!(next, Err(tokio::sync::mpsc::error::TryRecvError::Empty)),
            "the writer took {next:?}"
        );
    }

    /// Lets the writer finish `frames` paused frames.
    fn release(&self, frames: usize) {
        self.gate.permits.add_permits(frames);
    }

    /// A group write on a task of its own.
    fn send(&self) -> tokio::task::JoinHandle<Result<(), BusError>> {
        let client = self.client.clone();
        tokio::spawn(async move { group_write(&client).await })
    }
}

fn service_of(frame: &[u8]) -> u16 {
    u16::from_be_bytes([frame[2], frame[3]])
}

fn route_back_body() -> Vec<u8> {
    let mut body = vec![CHANNEL, 0x00];
    body.extend_from_slice(&TCP_ROUTE_BACK);
    body
}

async fn read_exactly(gateway: &mut TcpGateway, count: usize) -> Vec<u8> {
    let mut octets = vec![0u8; count];
    timeout(STEP, gateway.stream.read_exact(&mut octets))
        .await
        .expect("bounded read")
        .expect("octets from the client");
    octets
}

async fn wait_for_heartbeat_attempt(sink: &Collect, attempt: u8) {
    timeout(STEP, async {
        loop {
            let seen = sink.0.lock().unwrap().iter().any(|record| {
                matches!(
                    record,
                    WireRecord::Event {
                        event: TunnelTapEvent::HeartbeatAttempt { attempt: seen },
                        ..
                    } if *seen == attempt
                )
            });
            if seen {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("the heartbeat attempt is admitted");
}

/// A send cancelled while its frame is half on the wire: the writer keeps
/// its place, finishes that frame whole once, and only then writes the next
/// one. Nothing interleaves and nothing is repeated.
#[tokio::test]
async fn a_send_cancelled_mid_frame_finishes_whole_once_before_the_next_frame() {
    let mut g = gated_tunnel(policy(), 9, None, None).await;
    let sending = g.send();
    let first = g.taken().await;
    assert_eq!(first, tunnelling_request(CHANNEL, 0, &group_write_cemi()));
    assert_eq!(read_exactly(&mut g.gateway, 9).await, &first[..9]);
    sending.abort();
    assert!(sending.await.unwrap_err().is_cancelled());
    let queued = g.send();
    settle().await;
    g.nothing_taken();
    assert!(
        !queued.is_finished(),
        "queued behind the half-written frame"
    );
    g.gateway.expect_quiet().await;

    g.release(2);
    assert_eq!(
        read_exactly(&mut g.gateway, first.len() - 9).await,
        &first[9..]
    );
    let second = tunnelling_request(CHANNEL, 1, &group_write_cemi());
    assert_eq!(g.gateway.frame().await, second);
    assert_eq!(g.taken().await, second);
    timeout(STEP, queued)
        .await
        .expect("bounded")
        .expect("task")
        .expect("the queued send is written");
    g.gateway.expect_quiet().await;
    g.nothing_taken();
    assert_eq!(g.client.closure(), None);
}

/// Dropped while its writer is inside a frame and another frame waits: the
/// drop does not wait for either, `Closed` comes once, the tasks go, and
/// the gateway gets the half frame and then the end, nothing else.
#[tokio::test]
async fn dropping_a_tunnel_while_its_writer_is_mid_frame_releases_the_stream() {
    let mut g = gated_tunnel(policy(), 9, None, None).await;
    let mut events = g.client.subscribe();
    let sending = g.send();
    let first = g.taken().await;
    let queued = g.send();
    settle().await;
    assert!(!sending.is_finished() && !queued.is_finished());
    sending.abort();
    queued.abort();
    let _ = sending.await;
    let _ = queued.await;
    let Gated {
        client,
        mut gateway,
        ..
    } = g;
    assert_eq!(Arc::strong_count(&client), 1, "the last owner");
    drop(client);
    closed_event(&mut events).await;
    tasks_reaped(&mut events).await;
    assert_eq!(read_exactly(&mut gateway, 9).await, &first[..9]);
    gateway.expect_released().await;
}

/// With the writer inside a frame and the request queue full, a disconnect
/// neither waits for queue room nor lets a queued request or a waiting
/// sender reach the wire: each is refused at once. The started frame
/// finishes whole, then DISCONNECT_REQUEST, then nothing.
#[tokio::test]
async fn a_disconnect_refuses_the_full_queue_and_nothing_follows_its_request() {
    let mut g = gated_tunnel(policy(), 0, None, None).await;
    let first = g.send();
    let in_flight = g.taken().await;
    assert_eq!(
        in_flight,
        tunnelling_request(CHANNEL, 0, &group_write_cemi())
    );
    let queued: Vec<_> = (0..tcp::OUTBOX_DEPTH).map(|_| g.send()).collect();
    settle().await;
    let waiting = g.send();
    settle().await;
    assert!(queued.iter().all(|sender| !sender.is_finished()));
    assert!(
        !waiting.is_finished(),
        "the full queue holds the next sender"
    );

    let disconnecting = tokio::spawn({
        let client = g.client.clone();
        async move { client.disconnect_shared().await }
    });
    for sender in queued.into_iter().chain([waiting]) {
        let result = timeout(STEP, sender)
            .await
            .expect("refused without waiting for the writer")
            .expect("task");
        assert!(
            matches!(
                result,
                Err(BusError::TunnelClosed(TunnelClosure::ClientDisconnect))
            ),
            "{result:?}"
        );
    }
    g.gateway.expect_quiet().await;
    assert!(!first.is_finished(), "the started frame is still held");

    g.release(2);
    assert_eq!(
        g.gateway.frame().await,
        in_flight,
        "the started frame, whole"
    );
    assert_eq!(
        g.gateway.expect(services::DISCONNECT_REQUEST).await,
        route_back_body()
    );
    assert_eq!(service_of(&g.taken().await), services::DISCONNECT_REQUEST);
    g.gateway
        .send(
            services::DISCONNECT_RESPONSE,
            &services::encode_disconnect_response(CHANNEL, services::E_NO_ERROR),
        )
        .await;
    timeout(STEP, disconnecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("confirmed disconnect");
    timeout(STEP, first)
        .await
        .expect("bounded")
        .expect("task")
        .expect("the started frame was written");
    g.gateway.expect_eof().await;
    g.nothing_taken();
    let late = timeout(STEP, group_write(&g.client))
        .await
        .expect("bounded");
    assert!(
        matches!(
            late,
            Err(BusError::TunnelClosed(TunnelClosure::ClientDisconnect))
        ),
        "{late:?}"
    );
}

/// A heartbeat already admitted when the disconnect begins is refused with
/// the queue: no CONNECTIONSTATE_REQUEST follows the DISCONNECT_REQUEST.
#[tokio::test]
async fn a_heartbeat_admitted_before_a_disconnect_never_follows_its_request() {
    let sink = Arc::new(Collect::default());
    let heartbeat = HeartbeatTiming {
        interval: Duration::from_millis(100),
        response_timeout: Duration::from_secs(10),
    };
    let mut g = gated_tunnel(
        TcpPolicy {
            heartbeat,
            ..policy()
        },
        0,
        None,
        Some(WireTap::new(sink.clone())),
    )
    .await;
    let first = g.send();
    let in_flight = g.taken().await;
    // Reported under the queue lock, before the writer could take it.
    wait_for_heartbeat_attempt(&sink, 1).await;
    let disconnecting = tokio::spawn({
        let client = g.client.clone();
        async move { client.disconnect_shared().await }
    });
    settle().await;
    g.release(2);
    assert_eq!(g.gateway.frame().await, in_flight);
    assert_eq!(
        g.gateway.expect(services::DISCONNECT_REQUEST).await,
        route_back_body()
    );
    assert_eq!(service_of(&g.taken().await), services::DISCONNECT_REQUEST);
    g.gateway
        .send(
            services::DISCONNECT_RESPONSE,
            &services::encode_disconnect_response(CHANNEL, services::E_NO_ERROR),
        )
        .await;
    timeout(STEP, disconnecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("confirmed disconnect");
    timeout(STEP, first)
        .await
        .expect("bounded")
        .expect("task")
        .expect("written");
    g.gateway.expect_eof().await;
    g.nothing_taken();
    assert_eq!(g.client.closure(), Some(TunnelClosure::ClientDisconnect));
}

/// A slow writer and a full queue cannot hold back the heartbeat: each
/// attempt goes out straight after the request in flight, ahead of the 15
/// queued behind it. When the last attempt goes unanswered, the queued
/// requests are refused and only DISCONNECT_REQUEST follows the request in
/// flight; then the stream closes.
#[tokio::test]
async fn a_failed_heartbeat_goes_ahead_of_a_full_queue_and_writes_none_of_it() {
    let sink = Arc::new(Collect::default());
    let mut g = gated_tunnel(
        TcpPolicy {
            heartbeat: FAST_HEARTBEAT,
            ..policy()
        },
        0,
        Some(tunnelling::TUNNELLING_REQUEST),
        Some(WireTap::new(sink.clone())),
    )
    .await;
    let mut events = g.client.subscribe();
    let first = g.send();
    assert_eq!(
        g.taken().await,
        tunnelling_request(CHANNEL, 0, &group_write_cemi())
    );
    let queued: Vec<_> = (0..tcp::OUTBOX_DEPTH).map(|_| g.send()).collect();
    settle().await;
    assert!(queued.iter().all(|sender| !sender.is_finished()));

    for attempt in 1..=4u8 {
        // Admitted at the queue front while request `attempt - 1` is held.
        wait_for_heartbeat_attempt(&sink, attempt).await;
        g.release(1);
        let body = g.gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
        assert_eq!(body[2], attempt - 1, "request {}", attempt - 1);
        assert_eq!(
            g.gateway.expect(services::CONNECTIONSTATE_REQUEST).await,
            route_back_body(),
            "attempt {attempt} straight after the request in flight"
        );
        assert_eq!(
            service_of(&g.taken().await),
            services::CONNECTIONSTATE_REQUEST
        );
        assert_eq!(
            g.taken().await,
            tunnelling_request(CHANNEL, attempt, &group_write_cemi()),
            "the next request is taken only after the heartbeat"
        );
    }

    // Attempt 4 goes unanswered: requests 5..=16 are refused, never written.
    let mut queued = queued.into_iter();
    let written: Vec<_> = queued.by_ref().take(4).collect();
    for sender in queued {
        let result = timeout(STEP, sender)
            .await
            .expect("refused at teardown")
            .expect("task");
        assert!(
            matches!(
                result,
                Err(BusError::TunnelClosed(TunnelClosure::HeartbeatFailed))
            ),
            "{result:?}"
        );
    }
    g.release(1); // request 4, in flight when teardown began, finishes whole
    let body = g.gateway.expect(tunnelling::TUNNELLING_REQUEST).await;
    assert_eq!(body[2], 4);
    assert_eq!(
        g.gateway.expect(services::DISCONNECT_REQUEST).await,
        route_back_body()
    );
    assert_eq!(service_of(&g.taken().await), services::DISCONNECT_REQUEST);
    g.gateway.expect_released().await;
    closed_event(&mut events).await;
    assert_eq!(g.client.closure(), Some(TunnelClosure::HeartbeatFailed));
    for sender in std::iter::once(first).chain(written) {
        timeout(STEP, sender)
            .await
            .expect("bounded")
            .expect("task")
            .expect("written before the teardown");
    }
    g.nothing_taken();
}

/// A disconnect behind a frame that never finishes and a full queue ends
/// within the local cleanup budget: the queue is refused, the stuck frame's
/// sender learns the closure, and the gateway sees only the half frame.
#[tokio::test]
async fn a_disconnect_behind_a_stuck_frame_and_a_full_queue_ends_within_the_cleanup_budget() {
    let policy = TcpPolicy {
        cleanup: Duration::from_millis(300),
        ..policy()
    };
    let mut g = gated_tunnel(policy, 9, None, None).await;
    let mut events = g.client.subscribe();
    let first = g.send();
    let in_flight = g.taken().await;
    let queued: Vec<_> = (0..tcp::OUTBOX_DEPTH).map(|_| g.send()).collect();
    settle().await;
    let waiting = g.send();
    settle().await;
    assert!(
        !waiting.is_finished(),
        "the full queue holds the next sender"
    );

    let started = Instant::now();
    let result = timeout(STEP, g.client.disconnect_shared())
        .await
        .expect("bounded");
    let elapsed = started.elapsed();
    assert!(
        matches!(
            result,
            Err(BusError::TunnelClosed(TunnelClosure::StreamWriteFailed))
        ),
        "{result:?}"
    );
    assert!(
        elapsed >= Duration::from_millis(300) && elapsed < Duration::from_millis(1500),
        "{elapsed:?}"
    );
    for sender in std::iter::once(first).chain(queued).chain([waiting]) {
        let result = timeout(STEP, sender).await.expect("bounded").expect("task");
        assert!(
            matches!(
                result,
                Err(BusError::TunnelClosed(TunnelClosure::ClientDisconnect))
            ),
            "{result:?}"
        );
    }
    closed_event(&mut events).await;
    assert_eq!(read_exactly(&mut g.gateway, 9).await, &in_flight[..9]);
    g.gateway.expect_released().await;
    g.nothing_taken();
}

/// Cancelling `disconnect()` while it waits for the response still closes
/// the stream; no task keeps it.
#[tokio::test]
async fn cancelling_a_waiting_disconnect_still_closes_the_stream() {
    let (client, mut gateway) = tcp_tunnel().await;
    let mut events = client.subscribe();
    let disconnecting = tokio::spawn(client.disconnect());
    gateway.expect(services::DISCONNECT_REQUEST).await;
    disconnecting.abort();
    assert!(disconnecting.await.unwrap_err().is_cancelled());
    gateway.expect_eof().await;
    closed_event(&mut events).await;
    tasks_reaped(&mut events).await;
}

/// No DISCONNECT_RESPONSE within the local budget: `Timeout`, and the
/// stream is closed anyway.
#[tokio::test]
async fn an_unanswered_disconnect_times_out_and_closes_the_stream() {
    let (client, mut gateway) = tcp_tunnel_with(
        TcpPolicy {
            disconnect_response: Duration::from_millis(300),
            ..policy()
        },
        None,
    )
    .await;
    let started = Instant::now();
    let disconnecting = tokio::spawn(client.disconnect());
    gateway.expect(services::DISCONNECT_REQUEST).await;
    let error = timeout(STEP, disconnecting)
        .await
        .expect("bounded")
        .expect("task")
        .unwrap_err();
    assert!(matches!(error, BusError::Timeout), "{error:?}");
    assert!(started.elapsed() >= Duration::from_millis(300));
    gateway.expect_eof().await;
}

// ---- Wire tap --------------------------------------------------------------

#[derive(Default)]
struct Collect(std::sync::Mutex<Vec<WireRecord>>);

impl WireSink for Collect {
    fn record(&self, record: WireRecord) {
        self.0.lock().unwrap().push(record);
    }
}

/// Per direction, the tap's frames equal the wire's, byte for byte, except
/// a redacted key, which is zero where the wire carried it.
fn assert_tapped(direction: &str, tapped: &[(Vec<u8>, Option<Redaction>)], wire: &[Vec<u8>]) {
    assert_eq!(tapped.len(), wire.len(), "{direction}: frame count");
    for (index, ((bytes, redaction), original)) in tapped.iter().zip(wire).enumerate() {
        match redaction {
            None => assert_eq!(bytes, original, "{direction} frame {index}"),
            Some(redaction) => {
                let key = redaction.offset..redaction.offset + redaction.octets;
                assert_eq!(&original[key.clone()], &SENTINEL_KEY, "key on the wire");
                assert_eq!(&bytes[key.clone()], &[0u8; 4], "zero in the record");
                let mut restored = bytes.clone();
                restored[key].copy_from_slice(&SENTINEL_KEY);
                assert_eq!(&restored, original, "{direction} frame {index}");
            }
        }
    }
}

/// Every whole TCP frame is recorded in both directions, keys redacted
/// before the sink; the events are the TCP ones only, with no UDP request
/// attempt or acknowledgement timeout.
#[tokio::test]
async fn the_tap_records_whole_tcp_frames_redacted_and_no_udp_retry_events() {
    let sink = Arc::new(Collect::default());
    let (client, mut gateway) = tcp_tunnel_with(policy(), Some(WireTap::new(sink.clone()))).await;
    let SocketAddr::V4(address) = gateway.stream.local_addr().unwrap() else {
        unreachable!("bound an IPv4 socket")
    };
    let mut events = client.subscribe();
    let mut from_client = vec![CONNECT_REQUEST_FRAME.to_vec()];
    let mut to_client = vec![frame::encode_frame(
        services::CONNECT_RESPONSE,
        &success_response(CHANNEL),
    )];

    timeout(
        STEP,
        client.send_frame(
            Destination::Individual(IndividualAddress::from_raw(0x1105)),
            Tpci::NumberedData { seq: 0 },
            ApplicationService::AuthorizeRequest { key: SENTINEL_KEY },
        ),
    )
    .await
    .expect("bounded")
    .expect("written");
    let request = gateway.frame().await;
    assert!(
        request.windows(4).any(|w| w == SENTINEL_KEY),
        "key on the wire"
    );
    from_client.push(request.clone());

    // An ACK (ignored, still recorded) and the confirmation echoing the key.
    let ack = frame::encode_frame(
        tunnelling::TUNNELLING_ACK,
        &tunnelling::encode_tunnelling_ack(CHANNEL, 0, tunnelling::E_NO_ERROR),
    );
    gateway.write(&ack).await;
    to_client.push(ack);
    let mut cemi = tunnelling::decode_tunnelling_request(&request[6..])
        .expect("request")
        .cemi
        .to_vec();
    cemi[0] = 0x2E;
    let echo = tunnelling_request(CHANNEL, 0, &cemi);
    gateway.write(&echo).await;
    to_client.push(echo);
    telegram(&mut events).await;

    let disconnecting = tokio::spawn(client.disconnect());
    from_client.push(gateway.frame().await);
    let response = frame::encode_frame(
        services::DISCONNECT_RESPONSE,
        &services::encode_disconnect_response(CHANNEL, services::E_NO_ERROR),
    );
    gateway.write(&response).await;
    to_client.push(response);
    timeout(STEP, disconnecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("confirmed");

    let records = sink.0.lock().unwrap().clone();
    let mut sent = Vec::new();
    let mut received = Vec::new();
    let mut tap_events = Vec::new();
    for record in &records {
        match record {
            WireRecord::Datagram {
                direction,
                bytes,
                redaction,
                ..
            } => match direction {
                WireDirection::Sent => sent.push((bytes.clone(), *redaction)),
                WireDirection::Received => received.push((bytes.clone(), *redaction)),
            },
            WireRecord::Event { event, .. } => tap_events.push(event.clone()),
        }
    }
    assert_tapped("sent", &sent, &from_client);
    assert_tapped("received", &received, &to_client);
    let redactions: Vec<Redaction> = sent
        .iter()
        .chain(&received)
        .filter_map(|(_, redaction)| *redaction)
        .collect();
    assert_eq!(redactions.len(), 2, "{redactions:?}");
    assert!(redactions
        .iter()
        .all(|r| r.kind == RedactionKind::AuthorizeKey && r.octets == 4));
    for record in &records {
        if let WireRecord::Datagram { bytes, .. } = record {
            assert!(!bytes.windows(4).any(|w| w == SENTINEL_KEY));
        }
    }
    assert_eq!(
        tap_events,
        vec![
            TunnelTapEvent::Connecting { gateway: address },
            TunnelTapEvent::Connected {
                channel_id: CHANNEL,
                assigned_address: IndividualAddress::from_raw(0x11ff),
            },
            TunnelTapEvent::Closed(TunnelClosure::ClientDisconnect),
        ]
    );
}

/// One tap record, reduced to what an ordering check needs.
#[derive(Debug, PartialEq)]
enum Trace {
    Sent(u16),
    Received(u16),
    Event(TunnelTapEvent),
}

fn trace(records: &[WireRecord]) -> Vec<Trace> {
    records
        .iter()
        .map(|record| match record {
            WireRecord::Datagram {
                direction, bytes, ..
            } => match direction {
                WireDirection::Sent => Trace::Sent(service_of(bytes)),
                WireDirection::Received => Trace::Received(service_of(bytes)),
            },
            WireRecord::Event { event, .. } => Trace::Event(event.clone()),
        })
        .collect()
}

#[test]
fn completed_skip_notices_survive_a_malformed_suffix_in_the_same_handshake_batch() {
    let sink = Arc::new(Collect::default());
    let tap = Some(WireTap::new(sink.clone()));
    let mut decoder = StreamDecoder::new(64, |service| service == services::CONNECT_RESPONSE);
    let mut items = Vec::new();
    let mut batch = frame::encode_frame(0x0999, b"not-retained");
    let expected_length = u16::try_from(batch.len()).unwrap();
    batch.extend_from_slice(&[0x06, 0x20, 0x02, 0x06, 0x00, 0x06]);
    let error = tcp::decode_handshake_batch(&mut decoder, &batch, &mut items, &tap).unwrap_err();
    assert!(matches!(error, BusError::Protocol(_)), "{error:?}");
    assert!(items.is_empty());
    let observed = trace(&sink.0.lock().unwrap());
    assert_eq!(
        observed.len(),
        1,
        "only structural skip metadata, no packet bytes"
    );
    assert!(
        matches!(&observed[0], Trace::Event(TunnelTapEvent::FrameSkipped {
        service_type: 0x0999, total_length, ..
    }) if *total_length == expected_length),
        "{observed:?}"
    );
}

#[tokio::test]
async fn a_failed_tcp_handshake_records_completed_skips_before_its_failure() {
    let sink = Arc::new(Collect::default());
    let (listener, address) = listen().await;
    let connecting = tokio::spawn(TunnelClient::connect_tcp(
        address,
        policy(),
        Some(WireTap::new(sink.clone())),
    ));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    let mut batch = frame::encode_frame(0x0999, b"not-retained");
    let expected_length = u16::try_from(batch.len()).unwrap();
    batch.extend_from_slice(&[0x06, 0x20, 0x02, 0x06, 0x00, 0x06]);
    gateway.write(&batch).await;
    let error = connect_error(
        timeout(STEP, connecting)
            .await
            .expect("bounded handshake failure")
            .unwrap(),
    );
    assert!(matches!(error, BusError::Protocol(_)), "{error:?}");
    gateway.expect_released().await;
    let observed = trace(&sink.0.lock().unwrap());
    assert_eq!(observed.len(), 4, "{observed:?}");
    assert!(
        matches!(&observed[0], Trace::Event(TunnelTapEvent::Connecting { gateway }) if *gateway == address)
    );
    assert!(matches!(
        &observed[1],
        Trace::Sent(services::CONNECT_REQUEST)
    ));
    assert!(
        matches!(&observed[2], Trace::Event(TunnelTapEvent::FrameSkipped {
        service_type: 0x0999, total_length, ..
    }) if *total_length == expected_length),
        "{observed:?}"
    );
    assert!(
        matches!(&observed[3], Trace::Event(TunnelTapEvent::ConnectFailed { reason }) if reason == &error.to_string()),
        "{observed:?}"
    );
}

/// Every skipped frame is reported in its place, before the CONNECT_RESPONSE
/// and after it, with its header facts only; the cumulative count covers
/// both. (The byte-exact tap test above, which has no skip, is the negative
/// control: its event list holds no `FrameSkipped`.)
#[tokio::test]
async fn skipped_frames_are_reported_in_place_during_and_after_the_handshake() {
    let sink = Arc::new(Collect::default());
    let (listener, address) = listen().await;
    let policy = TcpPolicy {
        frame_cap: 64,
        ..policy()
    };
    let connecting = tokio::spawn(TunnelClient::connect_tcp(
        address,
        policy,
        Some(WireTap::new(sink.clone())),
    ));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    let unknown = frame::encode_frame(0x0999, &[0x06, 0x10, 0x02, 0x06, 0x00, 0x14, 0xAA, 0xBB]);
    assert_eq!(unknown.len(), 14);
    let oversize = tunnelling_request(CHANNEL, 0, &indication_with(vec![0xAB; 50]));
    assert!(oversize.len() > 64, "{}", oversize.len());
    let oversize_length = u16::try_from(oversize.len()).unwrap();
    let mut burst = unknown.clone();
    burst.extend_from_slice(&oversize);
    burst.extend_from_slice(&frame::encode_frame(
        services::CONNECT_RESPONSE,
        &success_response(CHANNEL),
    ));
    gateway.write(&burst).await;
    let client = timeout(STEP, connecting)
        .await
        .expect("bounded")
        .expect("task")
        .expect("TCP tunnel");
    assert_eq!(client.skipped_frame_count(), 2);
    let mut events = client.subscribe();

    let mut largest_body = tunnelling::encode_tunnelling_request(CHANNEL, 2, &[]);
    largest_body.resize(65535 - 6, 0xAB);
    let largest = frame::encode_frame(tunnelling::TUNNELLING_REQUEST, &largest_body);
    gateway
        .write(&tunnelling_request(CHANNEL, 1, &L_DATA_IND))
        .await;
    gateway.write(&unknown).await;
    for piece in largest.chunks(4096) {
        gateway.write(piece).await;
    }
    gateway
        .write(&tunnelling_request(CHANNEL, 3, &L_DATA_IND_ZERO))
        .await;
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND));
    assert_eq!(telegram(&mut events).await, decoded(&L_DATA_IND_ZERO));
    assert_eq!(client.skipped_frame_count(), 4);

    let skipped_unknown = TunnelTapEvent::FrameSkipped {
        service_type: 0x0999,
        total_length: 14,
        reason: SkipReason::UnhandledService,
    };
    let records = sink.0.lock().unwrap().clone();
    assert_eq!(
        trace(&records),
        vec![
            Trace::Event(TunnelTapEvent::Connecting { gateway: address }),
            Trace::Sent(services::CONNECT_REQUEST),
            Trace::Event(skipped_unknown.clone()),
            Trace::Event(TunnelTapEvent::FrameSkipped {
                service_type: tunnelling::TUNNELLING_REQUEST,
                total_length: oversize_length,
                reason: SkipReason::AboveLocalCap,
            }),
            Trace::Received(services::CONNECT_RESPONSE),
            Trace::Event(TunnelTapEvent::Connected {
                channel_id: CHANNEL,
                assigned_address: IndividualAddress::from_raw(0x11ff),
            }),
            Trace::Received(tunnelling::TUNNELLING_REQUEST),
            Trace::Event(skipped_unknown),
            Trace::Event(TunnelTapEvent::FrameSkipped {
                service_type: tunnelling::TUNNELLING_REQUEST,
                total_length: 65535,
                reason: SkipReason::AboveLocalCap,
            }),
            Trace::Received(tunnelling::TUNNELLING_REQUEST),
        ]
    );
    // No octet of a skipped frame reaches the tap.
    for record in &records {
        if let WireRecord::Datagram { bytes, .. } = record {
            assert!(bytes.len() <= 64, "only stored frames: {}", bytes.len());
            assert!(!bytes.windows(4).any(|w| w == [0xAB; 4]));
            assert!(!bytes.ends_with(&[0xAA, 0xBB]));
        }
    }
}

/// A refused TCP connect leaves a recording of request, refusal and reason.
#[tokio::test]
async fn a_refused_tcp_connect_is_recorded_with_its_reason() {
    let sink = Arc::new(Collect::default());
    let (listener, address) = listen().await;
    let connecting = tokio::spawn(TunnelClient::connect_tcp(
        address,
        policy(),
        Some(WireTap::new(sink.clone())),
    ));
    let mut gateway = accept(&listener).await;
    assert_eq!(gateway.frame().await, CONNECT_REQUEST_FRAME);
    gateway
        .send(services::CONNECT_RESPONSE, &[0x00, 0x24])
        .await;
    let error = connect_error(timeout(STEP, connecting).await.unwrap().unwrap());
    assert!(
        matches!(error, BusError::ConnectionRefused(0x24)),
        "{error:?}"
    );
    let records = sink.0.lock().unwrap();
    let datagrams = records
        .iter()
        .filter(|r| matches!(r, WireRecord::Datagram { .. }))
        .count();
    assert_eq!(datagrams, 2, "request and refusal recorded");
    let WireRecord::Event {
        event: TunnelTapEvent::ConnectFailed { reason },
        ..
    } = records.last().expect("records")
    else {
        panic!("the last record is the failure: {records:?}");
    };
    assert!(reason.contains("E_NO_MORE_CONNECTIONS"), "{reason}");
}
