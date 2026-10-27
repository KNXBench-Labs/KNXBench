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
        let mut buf = [0u8; 256];
        let (n, _) = tokio::time::timeout(STEP, self.socket.recv_from(&mut buf))
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
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind gateway");
    let SocketAddr::V4(address) = socket.local_addr().expect("gateway address") else {
        unreachable!("bound an IPv4 socket")
    };
    let connecting = tokio::spawn(async move {
        TunnelClient::connect_timed(address, TunnelReturnPath::default(), heartbeat).await
    });
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
    let (client, gateway) = tunnel_timed(0x2a, FAST_HEARTBEAT).await;
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
        Duration::from_millis(400),
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
