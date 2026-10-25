//! Loopback tests of the KNXnet/IP front-end against real UDP sockets and the simulated device.

use std::collections::BTreeMap;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use knx_core::IndividualAddress;
use tokio::net::UdpSocket;
use tokio::time::timeout;

use super::*;
use crate::cemi::{ApplicationService, Destination};
use crate::commissioning::simulator::Seen;
use crate::core::dib::{decode_device_info, decode_service_families, SERVICE_FAMILY_TUNNELLING};
use crate::core::services::{
    decode_connect_response, decode_connectionstate_response, decode_disconnect_response,
    encode_connect_request, encode_connectionstate_request, encode_disconnect_request,
};

const WAIT: Duration = Duration::from_secs(2);
const ROUTE_BACK: Hpai = Hpai {
    addr: Ipv4Addr::UNSPECIFIED,
    port: 0,
};

struct Harness {
    device: Arc<SimulatedDevice>,
    gateway: SocketAddr,
    from_tool: Arc<AtomicUsize>,
    undecodable: Arc<Mutex<Vec<String>>>,
}

async fn start() -> Harness {
    start_with(BTreeMap::new()).await
}

async fn start_with(properties: BTreeMap<(u16, u8, u8), Vec<u8>>) -> Harness {
    start_configured(|config| config.properties = properties).await
}

async fn start_configured(configure: impl FnOnce(&mut GatewayConfig)) -> Harness {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let gateway = socket.local_addr().unwrap();
    let SocketAddr::V4(advertised) = gateway else {
        unreachable!("bound to an IPv4 address")
    };
    let device = Arc::new(SimulatedDevice::new());
    let from_tool = Arc::new(AtomicUsize::new(0));
    let undecodable = Arc::new(Mutex::new(Vec::new()));
    let (counter, bad) = (from_tool.clone(), undecodable.clone());
    let mut config = GatewayConfig::new(advertised, IndividualAddress::new(1, 1, 0).unwrap());
    configure(&mut config);
    config.observer = Some(Arc::new(move |event| match event {
        GatewayEvent::Frame {
            from_tool: true, ..
        } => {
            counter.fetch_add(1, Ordering::SeqCst);
        }
        GatewayEvent::Undecodable { reason, .. } => bad.lock().unwrap().push(reason),
        _ => {}
    }));
    tokio::spawn(serve(device.clone(), socket, config));
    Harness {
        device,
        gateway,
        from_tool,
        undecodable,
    }
}

struct Tool {
    socket: UdpSocket,
    gateway: SocketAddr,
    channel: u8,
    sequence: u8,
}

impl Tool {
    async fn new(harness: &Harness, bind: &str) -> Self {
        Self {
            socket: UdpSocket::bind(bind).await.unwrap(),
            gateway: harness.gateway,
            channel: 0,
            sequence: 0,
        }
    }

    async fn send(&self, service: u16, body: &[u8]) {
        let datagram = encode_frame(service, body);
        self.socket.send_to(&datagram, self.gateway).await.unwrap();
    }

    async fn receive(&self) -> Option<(u16, Vec<u8>)> {
        let mut buffer = vec![0u8; 2048];
        let (length, _) = timeout(WAIT, self.socket.recv_from(&mut buffer))
            .await
            .ok()?
            .unwrap();
        let (header, body) = decode_frame(&buffer[..length]).unwrap();
        Some((header.service_type, body.to_vec()))
    }

    async fn connect(&mut self) -> u8 {
        let cri = [0x04, TUNNEL_CONNECTION, TUNNEL_LINKLAYER, 0x00];
        self.send(
            CONNECT_REQUEST,
            &encode_connect_request(ROUTE_BACK, ROUTE_BACK, &cri),
        )
        .await;
        let (service, body) = self.receive().await.expect("a CONNECT_RESPONSE");
        assert_eq!(service, CONNECT_RESPONSE);
        let response = decode_connect_response(&body).unwrap();
        self.channel = response.channel_id;
        response.status
    }

    async fn connect_management(&mut self) -> (u8, Vec<u8>) {
        let cri = [0x02, DEVICE_MGMT_CONNECTION];
        self.send(
            CONNECT_REQUEST,
            &encode_connect_request(ROUTE_BACK, ROUTE_BACK, &cri),
        )
        .await;
        let (service, body) = self.receive().await.expect("a CONNECT_RESPONSE");
        assert_eq!(service, CONNECT_RESPONSE);
        let response = decode_connect_response(&body).unwrap();
        self.channel = response.channel_id;
        (response.status, response.crd)
    }

    /// Sends one management cEMI and returns the ACK status plus the `.con` that follows.
    async fn manage(&mut self, cemi: &[u8]) -> (u8, Vec<u8>) {
        let body = encode_tunnelling_request(self.channel, self.sequence, cemi);
        self.send(DEVICE_CONFIGURATION_REQUEST, &body).await;
        let (service, body) = self.receive().await.expect("a DEVICE_CONFIGURATION_ACK");
        assert_eq!(service, DEVICE_CONFIGURATION_ACK);
        let ack = decode_tunnelling_ack(&body).unwrap();
        assert_eq!(ack.sequence_counter, self.sequence);
        self.sequence = self.sequence.wrapping_add(1);
        let (service, body) = self
            .receive()
            .await
            .expect("a DEVICE_CONFIGURATION_REQUEST");
        assert_eq!(service, DEVICE_CONFIGURATION_REQUEST);
        let request = decode_tunnelling_request(&body).unwrap();
        let ack = encode_tunnelling_ack(request.channel_id, request.sequence_counter, 0);
        self.send(DEVICE_CONFIGURATION_ACK, &ack).await;
        (ack_status(&ack), request.cemi.to_vec())
    }

    async fn send_cemi(&mut self, cemi: &[u8]) {
        let body = encode_tunnelling_request(self.channel, self.sequence, cemi);
        self.send(TUNNELLING_REQUEST, &body).await;
    }

    /// Sends one request and returns its ACK status.
    async fn request(&mut self, frame: &LDataFrame) -> u8 {
        self.send_cemi(&encode_l_data(frame).unwrap()).await;
        let (service, body) = self.receive().await.expect("a TUNNELLING_ACK");
        assert_eq!(service, TUNNELLING_ACK);
        let ack = decode_tunnelling_ack(&body).unwrap();
        assert_eq!(ack.sequence_counter, self.sequence);
        self.sequence = self.sequence.wrapping_add(1);
        ack.status
    }

    /// The next frame the gateway tunnels to the tool, acknowledged like a real client would.
    async fn next_frame(&self) -> Option<LDataFrame> {
        Some(decode_l_data(&self.next_cemi().await?).unwrap())
    }

    /// The same as [`Self::next_frame`], as the cEMI octets that crossed the socket.
    async fn next_cemi(&self) -> Option<Vec<u8>> {
        loop {
            let (service, body) = self.receive().await?;
            if service != TUNNELLING_REQUEST {
                continue;
            }
            let request = decode_tunnelling_request(&body).unwrap();
            let ack = encode_tunnelling_ack(request.channel_id, request.sequence_counter, 0);
            self.send(TUNNELLING_ACK, &ack).await;
            return Some(request.cemi.to_vec());
        }
    }
}

fn request(harness: &Harness, transport: Tpci, service: ApplicationService) -> LDataFrame {
    LDataFrame {
        kind: LDataMessageKind::Request,
        source: IndividualAddress::from_raw(0),
        destination: Destination::Individual(harness.device.address()),
        transport,
        service,
        control: None,
    }
}

#[tokio::test]
async fn search_and_description_describe_a_tunnelling_gateway() {
    let harness = start().await;
    let tool = Tool::new(&harness, "127.0.0.1:0").await;
    for (request, response) in [
        (SEARCH_REQUEST, SEARCH_RESPONSE),
        (DESCRIPTION_REQUEST, DESCRIPTION_RESPONSE),
    ] {
        tool.send(request, &ROUTE_BACK.encode()).await;
        let (service, body) = tool.receive().await.expect("an answer");
        assert_eq!(service, response);
        // A search answer opens with the control endpoint; a description does not.
        let dibs = if service == SEARCH_RESPONSE {
            let (endpoint, rest) = Hpai::decode(&body).unwrap();
            assert_eq!(
                SocketAddr::V4(SocketAddrV4::new(endpoint.addr, endpoint.port)),
                harness.gateway
            );
            rest
        } else {
            &body[..]
        };
        let (info, rest) = decode_device_info(dibs).unwrap();
        assert_eq!(info.friendly_name, "KNXBench simulator");
        assert_eq!(
            info.individual_address,
            IndividualAddress::new(1, 1, 0).unwrap()
        );
        let (families, _) = decode_service_families(rest).unwrap();
        assert!(families.supports(FAMILY_CORE));
        assert!(families.supports(FAMILY_DEVICE_MANAGEMENT));
        assert!(families.supports(SERVICE_FAMILY_TUNNELLING));
    }
}

#[tokio::test]
async fn connect_assigns_the_address_the_device_answers_to() {
    let harness = start().await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);
    tool.send(
        CONNECTIONSTATE_REQUEST,
        &encode_connectionstate_request(tool.channel, ROUTE_BACK),
    )
    .await;
    let (service, body) = tool.receive().await.unwrap();
    assert_eq!(service, CONNECTIONSTATE_RESPONSE);
    assert_eq!(
        decode_connectionstate_response(&body).unwrap().status,
        E_NO_ERROR
    );
    tool.send(
        CONNECTIONSTATE_REQUEST,
        &encode_connectionstate_request(tool.channel + 1, ROUTE_BACK),
    )
    .await;
    let (_, body) = tool.receive().await.unwrap();
    assert_eq!(
        decode_connectionstate_response(&body).unwrap().status,
        E_CONNECTION_ID
    );
}

#[tokio::test]
async fn a_management_exchange_crosses_the_socket_in_both_directions() {
    let harness = start().await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);

    let connect = request(
        &harness,
        Tpci::Connect,
        ApplicationService::NoApplicationPdu,
    );
    assert_eq!(tool.request(&connect).await, E_NO_ERROR);
    let confirmation = tool.next_frame().await.expect("the T_Connect confirmation");
    assert_eq!(
        confirmation.kind,
        LDataMessageKind::Confirmation { error: false }
    );
    assert_eq!(confirmation.transport, Tpci::Connect);

    let read = request(
        &harness,
        Tpci::NumberedData { seq: 0 },
        ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
    );
    assert_eq!(tool.request(&read).await, E_NO_ERROR);
    let confirmation = tool.next_frame().await.expect("the request confirmation");
    assert_eq!(
        confirmation.kind,
        LDataMessageKind::Confirmation { error: false }
    );
    // A numbered request is first acknowledged by the device's Transport Layer.
    let transport_ack = tool.next_frame().await.expect("the device's T_ACK");
    assert_eq!(transport_ack.transport, Tpci::Ack { seq: 0 });
    let answer = tool.next_frame().await.expect("the device answer");
    assert_eq!(answer.kind, LDataMessageKind::Indication);
    assert_eq!(answer.source, harness.device.address());
    assert_eq!(
        answer.service,
        ApplicationService::DeviceDescriptorResponse {
            descriptor_type: 0,
            data: vec![0x07, 0xB0],
        }
    );
    assert_eq!(harness.device.seen()[0], Seen::Connect);
}

#[tokio::test]
async fn a_repeated_sequence_is_acknowledged_but_reaches_the_device_once() {
    let harness = start().await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);
    let connect = request(
        &harness,
        Tpci::Connect,
        ApplicationService::NoApplicationPdu,
    );
    let cemi = encode_l_data(&connect).unwrap();
    tool.send_cemi(&cemi).await;
    tool.send_cemi(&cemi).await;
    // The gateway may tunnel the confirmation before it acknowledges the repeat; the
    // order of its own requests and its ACKs is not fixed, so both are taken as they come.
    let mut acks = 0;
    while let Some((service, body)) = tool.receive().await {
        match service {
            TUNNELLING_ACK => {
                assert_eq!(decode_tunnelling_ack(&body).unwrap().status, E_NO_ERROR);
                acks += 1;
            }
            TUNNELLING_REQUEST => {
                let request = decode_tunnelling_request(&body).unwrap();
                let ack = encode_tunnelling_ack(request.channel_id, request.sequence_counter, 0);
                tool.send(TUNNELLING_ACK, &ack).await;
            }
            _ => {}
        }
    }
    assert_eq!(acks, 2);
    assert_eq!(harness.from_tool.load(Ordering::SeqCst), 1);
    assert_eq!(harness.device.seen(), vec![Seen::Connect]);
}

/// A tool that negotiated a long APDU sends an extended frame (FT = 0, Ctrl1 `30h`); the
/// confirmation names the same frame type (EMI_IMI v01.04.02 AS §4.1.5.3.4) and the device
/// receives every octet.
#[tokio::test]
async fn an_extended_request_is_confirmed_as_an_extended_frame() {
    let harness = start().await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);
    let connect = request(
        &harness,
        Tpci::Connect,
        ApplicationService::NoApplicationPdu,
    );
    assert_eq!(tool.request(&connect).await, E_NO_ERROR);
    tool.next_frame().await.expect("the T_Connect confirmation");

    let data: Vec<u8> = (0..52).map(|i| i ^ 0x5A).collect();
    let destination = harness.device.address().raw().to_be_bytes();
    let mut cemi = vec![
        0x11,
        0x00,
        0x30,
        0x60,
        0x00,
        0x00,
        destination[0],
        destination[1],
    ];
    cemi.extend([0x37, 0x42, 0xB4, 0x01, 0x16]);
    cemi.extend(&data);
    tool.send_cemi(&cemi).await;
    let (service, _) = tool.receive().await.expect("a TUNNELLING_ACK");
    assert_eq!(service, TUNNELLING_ACK);

    let confirmation = tool.next_cemi().await.expect("the request confirmation");
    assert_eq!(confirmation[0], 0x2E);
    assert_eq!(confirmation[2] & 0x80, 0x00, "FT must say extended frame");
    assert_eq!(confirmation[8], 55);
    assert_eq!(confirmation[9..], cemi[9..]);
    assert!(harness.device.seen().iter().any(|seen| matches!(
        seen,
        Seen::MemoryWrite { address: 0x0116, data: written, .. } if *written == data
    )));
}

#[tokio::test]
async fn a_second_host_is_refused_until_the_first_disconnects() {
    let harness = start().await;
    let mut first = Tool::new(&harness, "127.0.0.1:0").await;
    let mut second = Tool::new(&harness, "127.0.0.2:0").await;
    assert_eq!(first.connect().await, E_NO_ERROR);
    assert_eq!(second.connect().await, E_NO_MORE_CONNECTIONS);

    first
        .send(
            DISCONNECT_REQUEST,
            &encode_disconnect_request(first.channel, ROUTE_BACK),
        )
        .await;
    let (service, body) = first.receive().await.unwrap();
    assert_eq!(service, DISCONNECT_RESPONSE);
    assert_eq!(
        decode_disconnect_response(&body).unwrap().status,
        E_NO_ERROR
    );
    assert_eq!(second.connect().await, E_NO_ERROR);
}

#[tokio::test]
async fn unsupported_tunnel_layers_and_connection_types_are_refused() {
    let harness = start().await;
    let tool = Tool::new(&harness, "127.0.0.1:0").await;
    for (cri, expected) in [
        ([0x04, TUNNEL_CONNECTION, 0x80, 0x00], E_TUNNELLING_LAYER),
        ([0x02, 0x07, 0x00, 0x00], E_CONNECTION_TYPE),
    ] {
        tool.send(
            CONNECT_REQUEST,
            &encode_connect_request(ROUTE_BACK, ROUTE_BACK, &cri),
        )
        .await;
        let (_, body) = tool.receive().await.unwrap();
        assert_eq!(decode_connect_response(&body).unwrap().status, expected);
    }
}

#[tokio::test]
async fn undecodable_cemi_is_reported_and_neither_confirmed_nor_forwarded() {
    let harness = start().await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);
    tool.send_cemi(&[0x99, 0x00, 0xBC, 0xE0, 0x00]).await;
    let (service, _) = tool.receive().await.unwrap();
    assert_eq!(service, TUNNELLING_ACK);
    assert!(
        tool.next_frame().await.is_none(),
        "nothing may be confirmed"
    );
    assert_eq!(harness.undecodable.lock().unwrap().len(), 1);
    assert!(harness.device.seen().is_empty());
}

fn ack_status(ack: &[u8; 4]) -> u8 {
    ack[3]
}

#[tokio::test]
async fn a_management_connection_answers_property_reads_and_reports_void_ones() {
    let mut properties = BTreeMap::new();
    properties.insert((0x000B, 0, 52u8), vec![0x12, 0x34]);
    let harness = start_with(properties).await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    let (status, crd) = tool.connect_management().await;
    assert_eq!(
        (status, crd),
        (E_NO_ERROR, vec![0x02, DEVICE_MGMT_CONNECTION])
    );

    // A configured property comes back with its data and the requested element count.
    let (_, answer) = tool.manage(&[0xFC, 0x00, 0x0B, 0x00, 52, 0x10, 0x01]).await;
    assert_eq!(
        answer,
        vec![0xFB, 0x00, 0x0B, 0x00, 52, 0x10, 0x01, 0x12, 0x34]
    );
    // Any other property is void: zero elements and the error code.
    let (_, answer) = tool.manage(&[0xFC, 0x00, 0x00, 0x00, 99, 0x10, 0x01]).await;
    assert_eq!(answer, vec![0xFB, 0x00, 0x00, 0x00, 99, 0x00, 0x01, 0x07]);
    // Writes are refused as read-only.
    let (_, answer) = tool
        .manage(&[0xF6, 0x00, 0x00, 0x00, 99, 0x10, 0x01, 0xAA])
        .await;
    assert_eq!(answer, vec![0xF5, 0x00, 0x00, 0x00, 99, 0x00, 0x01, 0x05]);
    // A negative answer keeps the request's start index, all twelve bits of it
    // (EMI_IMI v01.04.02 AS §4.1.7.3.3 and §4.1.7.3.5: NoE zero, SIx as in the request).
    let (_, answer) = tool.manage(&[0xFC, 0x00, 0x00, 0x00, 99, 0x11, 0x01]).await;
    assert_eq!(answer, vec![0xFB, 0x00, 0x00, 0x00, 99, 0x01, 0x01, 0x07]);
    let (_, answer) = tool
        .manage(&[0xF6, 0x00, 0x00, 0x00, 99, 0x1A, 0x01, 0xAA])
        .await;
    assert_eq!(answer, vec![0xF5, 0x00, 0x00, 0x00, 99, 0x0A, 0x01, 0x05]);
}

#[tokio::test]
async fn a_tunnel_and_a_management_connection_coexist_on_separate_channels() {
    let harness = start().await;
    let mut tunnel = Tool::new(&harness, "127.0.0.1:0").await;
    let mut management = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tunnel.connect().await, E_NO_ERROR);
    assert_eq!(management.connect_management().await.0, E_NO_ERROR);
    assert_ne!(tunnel.channel, management.channel);
    // Closing one leaves the other open: the heartbeat of the survivor still answers.
    management
        .send(
            DISCONNECT_REQUEST,
            &encode_disconnect_request(management.channel, ROUTE_BACK),
        )
        .await;
    let (service, _) = management.receive().await.unwrap();
    assert_eq!(service, DISCONNECT_RESPONSE);
    tunnel
        .send(
            CONNECTIONSTATE_REQUEST,
            &encode_connectionstate_request(tunnel.channel, ROUTE_BACK),
        )
        .await;
    let (_, body) = tunnel.receive().await.unwrap();
    assert_eq!(
        decode_connectionstate_response(&body).unwrap().status,
        E_NO_ERROR
    );
}

#[tokio::test]
async fn rejected_sequence_numbers_do_not_refresh_the_heartbeat() {
    let socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let endpoint = match socket.local_addr().unwrap() {
        SocketAddr::V4(endpoint) => endpoint,
        _ => unreachable!("IPv4 fixture"),
    };
    let device = SimulatedDevice::new();
    let config = GatewayConfig::new(endpoint, IndividualAddress::new(1, 1, 0).unwrap());
    let old = Instant::now() - Duration::from_secs(1);
    let mut gateway = Gateway {
        socket: &socket,
        device: &device,
        config: &config,
        tunnel: Some(Connection {
            channel: 1,
            control: endpoint,
            data: endpoint,
            receive_sequence: 0,
            send_sequence: 1,
            last_heard: old,
            in_flight: Some(InFlight {
                sequence: 0,
                datagram: vec![],
                deadline: Instant::now() + ACK_TIMEOUT,
                repeated: false,
            }),
            queue: VecDeque::new(),
        }),
        management: None,
        last_channel: 1,
    };
    gateway
        .on_request(Kind::Tunnel, &encode_tunnelling_request(1, 5, &[]))
        .await;
    assert_eq!(gateway.tunnel.as_ref().unwrap().last_heard, old);
    gateway
        .on_ack(Kind::Tunnel, &encode_tunnelling_ack(1, 5, E_NO_ERROR))
        .await;
    assert_eq!(gateway.tunnel.as_ref().unwrap().last_heard, old);
    assert_eq!(
        socket.try_recv(&mut [0; 2048]).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
    gateway
        .on_request(Kind::Tunnel, &encode_tunnelling_request(1, 0, &[]))
        .await;
    assert!(gateway.tunnel.as_ref().unwrap().last_heard > old);
    gateway.tunnel.as_mut().unwrap().last_heard = old;
    gateway
        .on_ack(Kind::Tunnel, &encode_tunnelling_ack(1, 0, E_NO_ERROR))
        .await;
    assert!(gateway.tunnel.as_ref().unwrap().last_heard > old);
    assert!(gateway.tunnel.as_ref().unwrap().in_flight.is_none());
}

#[tokio::test]
async fn an_explicit_hpai_receives_the_reply_instead_of_the_sender() {
    let harness = start().await;
    let tool = Tool::new(&harness, "127.0.0.1:0").await;
    let receiver = UdpSocket::bind("127.0.0.1:0").await.unwrap();
    let SocketAddr::V4(endpoint) = receiver.local_addr().unwrap() else {
        unreachable!("IPv4 fixture");
    };
    tool.send(
        SEARCH_REQUEST,
        &Hpai {
            addr: *endpoint.ip(),
            port: endpoint.port(),
        }
        .encode(),
    )
    .await;
    let mut buffer = [0; 2048];
    let length = timeout(WAIT, receiver.recv(&mut buffer))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        decode_frame(&buffer[..length]).unwrap().0.service_type,
        SEARCH_RESPONSE
    );
    assert_eq!(
        tool.socket.try_recv(&mut buffer).unwrap_err().kind(),
        std::io::ErrorKind::WouldBlock
    );
}

/// Waits for a `DISCONNECT_REQUEST` naming `channel`, skipping anything else the gateway sends.
async fn expect_disconnect(tool: &Tool, channel: u8) {
    loop {
        let (service, body) = tool.receive().await.expect("a DISCONNECT_REQUEST");
        if service == DISCONNECT_REQUEST {
            assert_eq!(body[0], channel);
            let (control, _) = Hpai::decode(&body[2..]).unwrap();
            assert_eq!(
                SocketAddr::V4(SocketAddrV4::new(control.addr, control.port)),
                tool.gateway
            );
            return;
        }
    }
}

/// Sends a heartbeat for `channel` and returns the status the gateway answers with.
async fn heartbeat(tool: &Tool, channel: u8) -> u8 {
    tool.send(
        CONNECTIONSTATE_REQUEST,
        &encode_connectionstate_request(channel, ROUTE_BACK),
    )
    .await;
    loop {
        let (service, body) = tool.receive().await.expect("a CONNECTIONSTATE_RESPONSE");
        if service == CONNECTIONSTATE_RESPONSE {
            return decode_connectionstate_response(&body).unwrap().status;
        }
    }
}

fn unconnected_read(harness: &Harness) -> LDataFrame {
    request(
        harness,
        Tpci::UnnumberedData,
        ApplicationService::DeviceDescriptorRead { descriptor_type: 0 },
    )
}

#[tokio::test]
async fn a_reply_that_cannot_be_sent_does_not_stop_the_gateway() {
    let harness = start().await;
    let tool = Tool::new(&harness, "127.0.0.1:0").await;
    // A loopback socket without SO_BROADCAST cannot send here; the send fails locally.
    let unreachable = Hpai {
        addr: Ipv4Addr::BROADCAST,
        port: 3671,
    };
    tool.send(SEARCH_REQUEST, &unreachable.encode()).await;
    tool.send(SEARCH_REQUEST, &ROUTE_BACK.encode()).await;
    let (service, _) = tool.receive().await.expect("the gateway still answers");
    assert_eq!(service, SEARCH_RESPONSE);
}

#[tokio::test]
async fn a_silent_tunnel_is_disconnected_by_the_gateway() {
    let harness = start_configured(|config| {
        config.heartbeat_timeout = Duration::from_millis(300);
    })
    .await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);
    expect_disconnect(&tool, tool.channel).await;
    assert_eq!(heartbeat(&tool, tool.channel).await, E_CONNECTION_ID);
}

#[tokio::test]
async fn an_unacknowledged_frame_is_repeated_once_then_the_tunnel_is_disconnected() {
    let harness = start().await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);
    assert_eq!(tool.request(&unconnected_read(&harness)).await, E_NO_ERROR);
    let mut requests = Vec::new();
    loop {
        let (service, body) = tool.receive().await.expect("a request or the disconnect");
        match service {
            TUNNELLING_REQUEST => requests.push(body),
            DISCONNECT_REQUEST => {
                assert_eq!(body[0], tool.channel);
                break;
            }
            _ => {}
        }
    }
    // Only the first frame went out, twice with the same sequence counter.
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0], requests[1]);
}

#[tokio::test]
async fn an_ack_with_an_error_status_counts_as_no_ack() {
    let harness = start().await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);
    assert_eq!(tool.request(&unconnected_read(&harness)).await, E_NO_ERROR);
    let mut seen = Vec::new();
    loop {
        let (service, body) = tool.receive().await.expect("a request or the disconnect");
        match service {
            TUNNELLING_REQUEST => {
                let request = decode_tunnelling_request(&body).unwrap();
                seen.push(request.sequence_counter);
                // E_SEQUENCE_NUMBER (Core §7.3.4): any status but E_NO_ERROR is an error.
                let ack = encode_tunnelling_ack(request.channel_id, request.sequence_counter, 0x04);
                tool.send(TUNNELLING_ACK, &ack).await;
            }
            DISCONNECT_REQUEST => break,
            _ => {}
        }
    }
    assert_eq!(seen, vec![0, 0]);
}

#[tokio::test]
async fn an_ack_of_the_wrong_service_does_not_acknowledge_a_management_frame() {
    let harness = start().await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect_management().await.0, E_NO_ERROR);
    let read = [0xFC, 0x00, 0x00, 0x00, 99, 0x10, 0x01];
    let body = encode_tunnelling_request(tool.channel, 0, &read);
    tool.send(DEVICE_CONFIGURATION_REQUEST, &body).await;
    let mut confirmations = 0;
    while let Some((service, body)) = tool.receive().await {
        match service {
            DEVICE_CONFIGURATION_REQUEST => {
                confirmations += 1;
                let request = decode_tunnelling_request(&body).unwrap();
                let ack = encode_tunnelling_ack(request.channel_id, request.sequence_counter, 0);
                tool.send(TUNNELLING_ACK, &ack).await;
            }
            DISCONNECT_REQUEST => break,
            _ => {}
        }
    }
    assert_eq!(confirmations, 2, "a TUNNELLING_ACK must not end the repeat");
}

#[tokio::test]
async fn a_disconnect_response_nobody_asked_for_closes_nothing() {
    let harness = start().await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);
    tool.send(DISCONNECT_RESPONSE, &[tool.channel, E_NO_ERROR])
        .await;
    assert_eq!(heartbeat(&tool, tool.channel).await, E_NO_ERROR);
}

#[tokio::test]
async fn a_channel_still_in_use_is_never_handed_out_again() {
    let harness = start().await;
    let mut tunnel = Tool::new(&harness, "127.0.0.1:0").await;
    let mut management = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tunnel.connect().await, E_NO_ERROR);
    // The same host takes its management connection over until the channel counter wraps.
    for _ in 0..u8::MAX {
        assert_eq!(management.connect_management().await.0, E_NO_ERROR);
        assert_ne!(management.channel, tunnel.channel);
    }
    management
        .send(
            DISCONNECT_REQUEST,
            &encode_disconnect_request(management.channel, ROUTE_BACK),
        )
        .await;
    management.receive().await.expect("a DISCONNECT_RESPONSE");
    assert_eq!(heartbeat(&tunnel, tunnel.channel).await, E_NO_ERROR);
}

#[tokio::test]
async fn a_huge_heartbeat_timeout_never_expires_and_never_panics() {
    let harness = start_configured(|config| config.heartbeat_timeout = Duration::MAX).await;
    let mut tool = Tool::new(&harness, "127.0.0.1:0").await;
    assert_eq!(tool.connect().await, E_NO_ERROR);
    assert_eq!(heartbeat(&tool, tool.channel).await, E_NO_ERROR);
}
