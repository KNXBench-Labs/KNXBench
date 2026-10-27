//! Loopback proof that the real connector hands the patient ACK wait to the tunnel it opens.

use super::*;
use knx_net::core::hpai::Hpai;
use knx_net::core::services;
use knx_net::{frame, tunnelling};
use std::time::Duration;
use tokio::net::UdpSocket;
use tokio::time::Instant;

const CHANNEL: u8 = 0x41;

/// A fake gateway that accepts one tunnel on 127.0.0.1 and answers nothing else.
async fn bind() -> (UdpSocket, SocketAddrV4) {
    let socket = UdpSocket::bind("127.0.0.1:0").await.expect("bind gateway");
    let std::net::SocketAddr::V4(address) = socket.local_addr().unwrap() else {
        unreachable!("bound an IPv4 socket")
    };
    (socket, address)
}

async fn answer_connect(
    socket: &UdpSocket,
    address: SocketAddrV4,
) -> (std::net::SocketAddr, Vec<u8>) {
    let mut buf = [0u8; 128];
    let (n, client) = socket.recv_from(&mut buf).await.expect("CONNECT_REQUEST");
    let (header, request) = frame::decode_frame(&buf[..n]).expect("a KNXnet/IP frame");
    assert_eq!(header.service_type, services::CONNECT_REQUEST);
    let request = request.to_vec();
    let mut body = vec![CHANNEL, services::E_NO_ERROR];
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
    (client, request)
}

async fn group_write(tunnel: &dyn BusTunnel) -> Result<(), BusSessionError> {
    let group = GroupAddress::parse("0/0/1", GroupAddressStyle::ThreeLevel).unwrap();
    tunnel
        .send(
            Destination::Group(group),
            ApplicationService::GroupValueWrite(GroupValue::Short(1)),
        )
        .await
}

/// Opens a tunnel through `connect`, writes once and delivers the ACK after `ack_after`.
/// Returns how many TUNNELLING_REQUEST datagrams the gateway saw before the send ended.
async fn requests_seen(
    connector: RealConnector,
    ack_after: Duration,
    connect: impl FnOnce(
        RealConnector,
        SocketAddrV4,
    ) -> Pin<
        Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send>,
    >,
) -> (usize, Vec<u8>) {
    let (socket, address) = bind().await;
    let connecting = tokio::spawn(connect(connector, address));
    let (client, connect_request) = answer_connect(&socket, address).await;
    let tunnel = connecting.await.unwrap().expect("tunnel");
    let sending = tokio::spawn(async move {
        let result = group_write(tunnel.as_ref()).await;
        (tunnel, result)
    });
    let started = Instant::now();
    let mut requests = 0;
    let mut buf = [0u8; 256];
    let mut acked = false;
    loop {
        let wait = if acked {
            Duration::from_millis(300)
        } else {
            ack_after.saturating_sub(started.elapsed())
        };
        match tokio::time::timeout(wait, socket.recv_from(&mut buf)).await {
            Ok(Ok((n, _))) => {
                let (header, _) = frame::decode_frame(&buf[..n]).unwrap();
                if header.service_type == tunnelling::TUNNELLING_REQUEST {
                    requests += 1;
                }
            }
            Ok(Err(error)) => panic!("{error}"),
            Err(_) if !acked => {
                socket
                    .send_to(
                        &frame::encode_frame(
                            tunnelling::TUNNELLING_ACK,
                            &tunnelling::encode_tunnelling_ack(CHANNEL, 0, 0),
                        ),
                        client,
                    )
                    .await
                    .unwrap();
                acked = true;
            }
            Err(_) => break,
        }
    }
    let (_tunnel, result) = sending.await.unwrap();
    assert!(
        result.is_ok(),
        "the late ACK is accepted either way: {result:?}"
    );
    (requests, connect_request)
}

/// The real connector's patient method gives the tunnel the 10 s wait: an ACK
/// 1.3 s late is accepted and the gateway saw the request once.
#[tokio::test]
async fn the_real_connector_patient_method_waits_past_one_second_for_the_ack() {
    let (seen, _) = requests_seen(
        RealConnector::default(),
        Duration::from_millis(1300),
        |connector, address| {
            Box::pin(async move {
                connector
                    .connect_tunnel_with_ack_wait(address, TunnelAckWait::Patient)
                    .await
            })
        },
    )
    .await;
    assert_eq!(seen, 1, "no repetition before the late ACK");
}

/// The plain `connect_tunnel` is unchanged: the same late ACK is preceded by
/// the one repetition at 1 s.
#[tokio::test]
async fn the_real_connector_plain_method_still_repeats_after_one_second() {
    let (seen, _) = requests_seen(
        RealConnector::default(),
        Duration::from_millis(1300),
        |connector, address| Box::pin(async move { connector.connect_tunnel(address).await }),
    )
    .await;
    assert_eq!(seen, 2, "original plus one repetition");
}

/// The Standard wait through the explicit method is the plain behaviour.
#[tokio::test]
async fn the_real_connector_standard_method_matches_the_plain_one() {
    let (seen, _) = requests_seen(
        RealConnector::default(),
        Duration::from_millis(1300),
        |connector, address| {
            Box::pin(async move {
                connector
                    .connect_tunnel_with_ack_wait(address, TunnelAckWait::Standard)
                    .await
            })
        },
    )
    .await;
    assert_eq!(seen, 2);
}

/// A connector built for Route Back keeps it through the patient method: the CONNECT_REQUEST
/// advertises the all-zero control and data endpoints, and the tunnel still has the 10 s wait.
#[tokio::test]
async fn the_real_connector_patient_method_keeps_its_route_back_return_path() {
    let (seen, connect_request) = requests_seen(
        RealConnector::new(knx_net::TunnelReturnPath::RouteBack),
        Duration::from_millis(1300),
        |connector, address| {
            Box::pin(async move {
                connector
                    .connect_tunnel_with_ack_wait(address, TunnelAckWait::Patient)
                    .await
            })
        },
    )
    .await;
    let route_back = [0x08, 0x01, 0, 0, 0, 0, 0, 0];
    assert_eq!(
        &connect_request[..8],
        route_back,
        "control endpoint 0.0.0.0:0"
    );
    assert_eq!(
        &connect_request[8..16],
        route_back,
        "data endpoint 0.0.0.0:0"
    );
    assert_eq!(seen, 1, "the patient wait applies as well");
}

/// The default connector advertises its own address, not Route Back (positive control).
#[tokio::test]
async fn the_real_connector_patient_method_advertises_its_own_endpoint_by_default() {
    let (_, connect_request) = requests_seen(
        RealConnector::default(),
        Duration::from_millis(1300),
        |connector, address| {
            Box::pin(async move {
                connector
                    .connect_tunnel_with_ack_wait(address, TunnelAckWait::Patient)
                    .await
            })
        },
    )
    .await;
    assert_ne!(&connect_request[..8], [0x08, 0x01, 0, 0, 0, 0, 0, 0]);
    assert_eq!(
        &connect_request[2..6],
        [127, 0, 0, 1],
        "loopback source address"
    );
}
