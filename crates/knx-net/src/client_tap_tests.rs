//! Loopback proof that a wire tap records every tunnel datagram byte-exactly (DIAG-03).
//!
//! A fake gateway on 127.0.0.1 keeps its own list of every datagram it
//! received from and sent to the client. The tap's records must equal that
//! list per direction, byte for byte, except the zeroed key of a redacted
//! frame. No real gateway, bus or device is involved.

use super::*;
use crate::cemi::Tpci;
use crate::wire_tap::{Redaction, RedactionKind, TunnelTapEvent, WireRecord, WireSink};
use std::net::SocketAddr;

const STEP: Duration = Duration::from_secs(3);
const SENTINEL_KEY: [u8; 4] = [0xC0, 0xFF, 0xEE, 0x42];

#[derive(Default)]
struct Collect(std::sync::Mutex<Vec<WireRecord>>);

impl WireSink for Collect {
    fn record(&self, record: WireRecord) {
        self.0.lock().unwrap().push(record);
    }
}

/// The gateway's own view: datagrams it received (client sent) and sent.
struct LoggingGateway {
    socket: UdpSocket,
    client: SocketAddr,
    from_client: Vec<Vec<u8>>,
    to_client: Vec<Vec<u8>>,
}

impl LoggingGateway {
    async fn recv(&mut self) -> (u16, Vec<u8>) {
        let mut buf = [0u8; 256];
        let (n, from) = tokio::time::timeout(STEP, self.socket.recv_from(&mut buf))
            .await
            .expect("a datagram from the client")
            .expect("gateway receive");
        self.client = from;
        self.from_client.push(buf[..n].to_vec());
        let (header, body) = frame::decode_frame(&buf[..n]).expect("a KNXnet/IP frame");
        (header.service_type, body.to_vec())
    }

    async fn send(&mut self, service: u16, body: &[u8]) {
        let datagram = frame::encode_frame(service, body);
        self.socket
            .send_to(&datagram, self.client)
            .await
            .expect("gateway send");
        self.to_client.push(datagram);
    }
}

fn l_data_ind_group_write() -> Vec<u8> {
    vec![
        0x29, 0x00, 0xbc, 0xe0, 0x11, 0x01, 0x00, 0x01, 0x01, 0x00, 0x81,
    ]
}

#[tokio::test]
async fn the_tap_records_every_datagram_byte_exactly_and_redacts_keys_both_ways() {
    let sink = Arc::new(Collect::default());
    let tap = WireTap::new(sink.clone());
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind gateway");
    let SocketAddr::V4(address) = socket.local_addr().expect("gateway address") else {
        unreachable!("bound an IPv4 socket")
    };
    let connecting = tokio::spawn(TunnelClient::connect(
        address,
        TunnelReturnPath::default(),
        TunnelAckWait::Standard,
        Some(tap),
    ));
    let mut gateway = LoggingGateway {
        socket,
        client: SocketAddr::V4(address),
        from_client: Vec::new(),
        to_client: Vec::new(),
    };
    let channel = 0x31;
    let (service, _) = gateway.recv().await;
    assert_eq!(service, services::CONNECT_REQUEST);
    let mut body = vec![channel, services::E_NO_ERROR];
    body.extend_from_slice(
        &Hpai {
            addr: *address.ip(),
            port: address.port(),
        }
        .encode(),
    );
    body.extend_from_slice(&[0x04, tunnelling::TUNNEL_CONNECTION, 0x11, 0xff]);
    gateway.send(services::CONNECT_RESPONSE, &body).await;
    let client = connecting.await.expect("task").expect("loopback tunnel");

    // 1. A request whose first attempt is ignored and whose repeat is acked.
    let sending = {
        let client = &client;
        async move {
            client
                .send_frame(
                    Destination::Individual(IndividualAddress::from_raw(0x1105)),
                    Tpci::UnnumberedData,
                    ApplicationService::IndividualAddressRead,
                )
                .await
        }
    };
    let gateway_side = async {
        let (service, _) = gateway.recv().await;
        assert_eq!(service, tunnelling::TUNNELLING_REQUEST);
        let (service, _) = gateway.recv().await; // the repeat, same sequence
        assert_eq!(service, tunnelling::TUNNELLING_REQUEST);
        gateway
            .send(
                tunnelling::TUNNELLING_ACK,
                &tunnelling::encode_tunnelling_ack(channel, 0, tunnelling::E_NO_ERROR),
            )
            .await;
    };
    let (sent, ()) = tokio::join!(sending, gateway_side);
    sent.expect("acknowledged on the repeat");

    // 2. An incoming telegram, then its duplicate (same sequence).
    for _ in 0..2 {
        gateway
            .send(
                tunnelling::TUNNELLING_REQUEST,
                &tunnelling::encode_tunnelling_request(channel, 0, &l_data_ind_group_write()),
            )
            .await;
        let (service, _) = gateway.recv().await;
        assert_eq!(service, tunnelling::TUNNELLING_ACK);
    }

    // 3. A key-bearing request, and the gateway's confirmation echoing it.
    let authorize = {
        let client = &client;
        async move {
            client
                .send_frame(
                    Destination::Individual(IndividualAddress::from_raw(0x1105)),
                    Tpci::NumberedData { seq: 0 },
                    ApplicationService::AuthorizeRequest { key: SENTINEL_KEY },
                )
                .await
        }
    };
    let gateway_side = async {
        let (service, body) = gateway.recv().await;
        assert_eq!(service, tunnelling::TUNNELLING_REQUEST);
        gateway
            .send(
                tunnelling::TUNNELLING_ACK,
                &tunnelling::encode_tunnelling_ack(channel, 1, tunnelling::E_NO_ERROR),
            )
            .await;
        // Echo the same cEMI back as L_Data.con (message code 0x2E).
        let mut cemi = tunnelling::decode_tunnelling_request(&body)
            .expect("request")
            .cemi
            .to_vec();
        cemi[0] = 0x2E;
        gateway
            .send(
                tunnelling::TUNNELLING_REQUEST,
                &tunnelling::encode_tunnelling_request(channel, 1, &cemi),
            )
            .await;
        let (service, _) = gateway.recv().await;
        assert_eq!(service, tunnelling::TUNNELLING_ACK);
    };
    let (sent, ()) = tokio::join!(authorize, gateway_side);
    sent.expect("authorize acknowledged");

    // 4. A graceful disconnect.
    let disconnecting = client.disconnect();
    let gateway_side = async {
        let (service, _) = gateway.recv().await;
        assert_eq!(service, services::DISCONNECT_REQUEST);
        gateway
            .send(
                services::DISCONNECT_RESPONSE,
                &services::encode_disconnect_response(channel, services::E_NO_ERROR),
            )
            .await;
    };
    let (result, ()) = tokio::join!(disconnecting, gateway_side);
    result.expect("confirmed disconnect");
    // Let the receive loop record the response before it ends.
    tokio::time::sleep(Duration::from_millis(50)).await;

    let records = sink.0.lock().unwrap().clone();
    let mut sent = Vec::new();
    let mut received = Vec::new();
    let mut redactions = Vec::new();
    let mut events = Vec::new();
    for record in &records {
        match record {
            WireRecord::Datagram {
                direction,
                bytes,
                redaction,
                ..
            } => {
                if let Some(redaction) = redaction {
                    redactions.push((*direction, *redaction));
                }
                match direction {
                    WireDirection::Sent => sent.push((bytes.clone(), *redaction)),
                    WireDirection::Received => received.push((bytes.clone(), *redaction)),
                }
            }
            WireRecord::Event { event, .. } => events.push(event.clone()),
        }
    }

    assert_byte_exact("sent", &sent, &gateway.from_client);
    assert_byte_exact("received", &received, &gateway.to_client);

    assert_eq!(redactions.len(), 2, "request and its echo: {redactions:?}");
    assert!(redactions
        .iter()
        .all(|(_, r)| r.kind == RedactionKind::AuthorizeKey && r.octets == 4));

    let rendered = format!("{records:?}");
    assert!(!rendered.contains(&format!("{:?}", SENTINEL_KEY)));
    for record in &records {
        if let WireRecord::Datagram { bytes, .. } = record {
            assert!(!bytes.windows(4).any(|w| w == SENTINEL_KEY));
        }
    }

    assert_eq!(
        events,
        vec![
            TunnelTapEvent::Connecting { gateway: address },
            TunnelTapEvent::Connected {
                channel_id: channel,
                assigned_address: IndividualAddress::from_raw(0x11ff),
            },
            TunnelTapEvent::RequestAttempt {
                sequence: 0,
                attempt: 1
            },
            TunnelTapEvent::AckTimeout {
                sequence: 0,
                attempt: 1
            },
            TunnelTapEvent::RequestAttempt {
                sequence: 0,
                attempt: 2
            },
            TunnelTapEvent::ReceiveDiscarded {
                sequence: 0,
                reason: "repeated sequence, acknowledged again",
            },
            TunnelTapEvent::RequestAttempt {
                sequence: 1,
                attempt: 1
            },
            TunnelTapEvent::Closed(TunnelClosure::ClientDisconnect),
        ]
    );
}

/// Per direction, the tap's datagrams equal the gateway's, byte for byte,
/// except a redacted key, which must be zero where the gateway saw the key.
fn assert_byte_exact(direction: &str, tapped: &[(Vec<u8>, Option<Redaction>)], wire: &[Vec<u8>]) {
    assert_eq!(
        tapped.len(),
        wire.len(),
        "{direction}: same datagram count as on the wire"
    );
    for (index, ((bytes, redaction), original)) in tapped.iter().zip(wire).enumerate() {
        match redaction {
            None => assert_eq!(bytes, original, "{direction} datagram {index}"),
            Some(redaction) => {
                let key = redaction.offset..redaction.offset + redaction.octets;
                assert_eq!(&original[key.clone()], &SENTINEL_KEY, "key on the wire");
                assert_eq!(&bytes[key.clone()], &[0u8; 4], "zero in the record");
                let mut restored = bytes.clone();
                restored[key].copy_from_slice(&SENTINEL_KEY);
                assert_eq!(&restored, original, "{direction} datagram {index}");
            }
        }
    }
}

/// A refused connection still leaves a recording that says so.
#[tokio::test]
async fn a_refused_connection_is_recorded_with_its_reason() {
    let sink = Arc::new(Collect::default());
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind gateway");
    let SocketAddr::V4(address) = socket.local_addr().expect("gateway address") else {
        unreachable!("bound an IPv4 socket")
    };
    let connecting = tokio::spawn(TunnelClient::connect(
        address,
        TunnelReturnPath::default(),
        TunnelAckWait::Standard,
        Some(WireTap::new(sink.clone())),
    ));
    let mut buf = [0u8; 128];
    let (_, client) = socket.recv_from(&mut buf).await.expect("CONNECT_REQUEST");
    // E_NO_MORE_CONNECTIONS, no channel.
    let mut body = vec![0x00, 0x24];
    body.extend_from_slice(
        &Hpai {
            addr: *address.ip(),
            port: address.port(),
        }
        .encode(),
    );
    socket
        .send_to(
            &frame::encode_frame(services::CONNECT_RESPONSE, &body),
            client,
        )
        .await
        .expect("send refusal");
    let Err(error) = connecting.await.expect("task") else {
        panic!("the refusal must fail the connect");
    };
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
        panic!("last record is the failure: {records:?}");
    };
    assert!(reason.contains("E_NO_MORE_CONNECTIONS"), "{reason}");
}
