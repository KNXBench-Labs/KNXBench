//! A KNXnet/IP UDP tunnelling front-end that puts one SimulatedDevice behind an unmodified tool.
//!
//! The simulator answers decoded management services in process. This module
//! speaks the KNXnet/IP side of a gateway (search, description, connect,
//! heartbeat, tunnelling, disconnect) so an unmodified engineering tool can
//! talk to that one device over a socket, and so the cEMI codec is exercised
//! on both directions. It is a test double: one tunnel and one device-management
//! connection at a time, link layer only, no routing, no TCP, no secure
//! sessions. The management connection answers `M_PropRead` for the few
//! properties the configuration lists and reports every other one as void.
//! Every datagram and every decoded frame goes to an observer so a session can
//! be recorded; a request the codec cannot decode is reported as such and
//! never answered. A reply that cannot be sent is reported and dropped, so one
//! unreachable address in a tool's HPAI cannot stop the front-end.

use std::collections::{BTreeMap, VecDeque};
use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use knx_core::IndividualAddress;
use tokio::net::UdpSocket;
use tokio::sync::broadcast::error::RecvError;
use tokio::time::Instant;

use super::simulator::SimulatedDevice;
use crate::cemi::{decode_l_data, encode_l_data, LDataFrame, LDataMessageKind, Tpci};
use crate::client::TunnelEvent;
use crate::core::hpai::{Hpai, HPAI_LEN};
use crate::core::services::{
    encode_disconnect_request, CONNECTIONSTATE_REQUEST, CONNECTIONSTATE_RESPONSE, CONNECT_REQUEST,
    CONNECT_RESPONSE, DISCONNECT_REQUEST, DISCONNECT_RESPONSE, E_CONNECTION_TYPE, E_NO_ERROR,
    SEARCH_REQUEST, SEARCH_RESPONSE,
};
use crate::frame::{decode_frame, encode_frame};
use crate::management::ManagementTransport;
use crate::tunnelling::{
    decode_tunnelling_ack, decode_tunnelling_request, encode_tunnelling_ack,
    encode_tunnelling_request, TUNNELLING_ACK, TUNNELLING_REQUEST, TUNNEL_CONNECTION,
    TUNNEL_LINKLAYER,
};

/// Core v01.06.02 AS §7.5.3: `DESCRIPTION_REQUEST` / `DESCRIPTION_RESPONSE`.
const DESCRIPTION_REQUEST: u16 = 0x0203;
const DESCRIPTION_RESPONSE: u16 = 0x0204;
/// Core v01.06.02 AS §7.3 Table 8/9 status codes this front-end produces.
const E_CONNECTION_ID: u8 = 0x21;
const E_NO_MORE_CONNECTIONS: u8 = 0x24;
/// Core v01.06.02 AS §7.8.2: connection type of a device-management connection.
const DEVICE_MGMT_CONNECTION: u8 = 0x03;
/// Device Management v01.05.01 AS §4: `DEVICE_CONFIGURATION_REQUEST` / `_ACK`.
const DEVICE_CONFIGURATION_REQUEST: u16 = 0x0310;
const DEVICE_CONFIGURATION_ACK: u16 = 0x0311;
/// cEMI management message codes (cEMI v01.05.01 AS §4.1.7).
const M_PROP_READ_REQ: u8 = 0xFC;
const M_PROP_READ_CON: u8 = 0xFB;
const M_PROP_WRITE_REQ: u8 = 0xF6;
const M_PROP_WRITE_CON: u8 = 0xF5;
/// cEMI property error codes (cEMI §4.1.7.3.2).
const PROP_ERROR_READ_ONLY: u8 = 0x05;
const PROP_ERROR_VOID: u8 = 0x07;
const E_TUNNELLING_LAYER: u8 = 0x29;
/// DIB codes and the KNX medium code for twisted pair (Core §7.5.4).
const DIB_DEVICE_INFO: u8 = 0x01;
const DIB_SUPP_SVC_FAMILIES: u8 = 0x02;
const MEDIUM_TP1: u8 = 0x02;
const FAMILY_CORE: u8 = 0x02;
const FAMILY_DEVICE_MANAGEMENT: u8 = 0x03;
const FAMILY_TUNNELLING: u8 = 0x04;
/// Tunnelling v01.07.01 AS §2.6.1: the sender waits one second for an ACK, repeats once, then
/// sends a `DISCONNECT_REQUEST`. The management connection reuses this rule (see ARCHITECTURE).
const ACK_TIMEOUT: Duration = Duration::from_secs(1);

/// What the observer is told about; borrowed so recording costs nothing when ignored.
#[derive(Debug)]
pub enum GatewayEvent<'a> {
    /// One KNXnet/IP datagram, as received from or sent to the tool.
    Datagram { from_tool: bool, bytes: &'a [u8] },
    /// A cEMI frame that decoded, with its raw octets.
    Frame {
        from_tool: bool,
        cemi: &'a [u8],
        frame: &'a LDataFrame,
    },
    /// A cEMI management message on the device-management connection.
    Management {
        from_tool: bool,
        cemi: &'a [u8],
        summary: String,
    },
    /// A cEMI frame the codec refused; it is not forwarded and not confirmed.
    Undecodable { cemi: &'a [u8], reason: String },
    /// Connection life cycle and other remarks.
    Note(String),
}

/// Receives every [`GatewayEvent`]; must not block.
pub type Observer = Arc<dyn Fn(GatewayEvent<'_>) + Send + Sync>;

/// How the front-end presents itself.
#[derive(Clone)]
pub struct GatewayConfig {
    /// Address and port a tool should use; goes into every HPAI the front-end sends.
    pub advertised: SocketAddrV4,
    /// ISO 8859-1 name in the device-information DIB, cut to 29 octets.
    pub friendly_name: String,
    /// The individual address the gateway reports about itself in the DIB.
    pub gateway_address: IndividualAddress,
    /// A tunnel with no `CONNECTIONSTATE_REQUEST` for this long is dropped (spec: 120 s).
    pub heartbeat_timeout: Duration,
    /// Properties the management connection serves, keyed by (object type, instance, property id).
    pub properties: BTreeMap<(u16, u8, u8), Vec<u8>>,
    pub observer: Option<Observer>,
}

impl GatewayConfig {
    pub fn new(advertised: SocketAddrV4, gateway_address: IndividualAddress) -> Self {
        Self {
            advertised,
            friendly_name: "KNXBench simulator".to_string(),
            gateway_address,
            heartbeat_timeout: Duration::from_secs(120),
            properties: BTreeMap::new(),
            observer: None,
        }
    }
}

struct InFlight {
    sequence: u8,
    datagram: Vec<u8>,
    deadline: Instant,
    repeated: bool,
}

/// The two connection types this front-end serves, one of each at a time.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Tunnel,
    Management,
}

impl Kind {
    const ALL: [Kind; 2] = [Kind::Tunnel, Kind::Management];

    fn other(self) -> Kind {
        match self {
            Kind::Tunnel => Kind::Management,
            Kind::Management => Kind::Tunnel,
        }
    }

    fn request_service(self) -> u16 {
        match self {
            Kind::Tunnel => TUNNELLING_REQUEST,
            Kind::Management => DEVICE_CONFIGURATION_REQUEST,
        }
    }

    fn ack_service(self) -> u16 {
        match self {
            Kind::Tunnel => TUNNELLING_ACK,
            Kind::Management => DEVICE_CONFIGURATION_ACK,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Kind::Tunnel => "tunnel",
            Kind::Management => "management connection",
        }
    }
}

struct Connection {
    channel: u8,
    control: SocketAddrV4,
    data: SocketAddrV4,
    /// Next sequence counter expected from the tool.
    receive_sequence: u8,
    /// Sequence counter for the next frame sent to the tool.
    send_sequence: u8,
    last_heard: Instant,
    in_flight: Option<InFlight>,
    queue: VecDeque<Vec<u8>>,
}

struct Gateway<'a> {
    socket: &'a UdpSocket,
    device: &'a SimulatedDevice,
    config: &'a GatewayConfig,
    tunnel: Option<Connection>,
    management: Option<Connection>,
    last_channel: u8,
}

/// Serves `device` on `socket` until the socket fails to receive. Each datagram is handled to
/// completion, so no ordering between a request and the answers it provokes can be lost. A
/// failed send is reported to the observer and never ends the loop.
pub async fn serve(
    device: Arc<SimulatedDevice>,
    socket: UdpSocket,
    config: GatewayConfig,
) -> io::Result<()> {
    let mut events = ManagementTransport::subscribe(&*device);
    let mut gateway = Gateway {
        socket: &socket,
        device: &device,
        config: &config,
        tunnel: None,
        management: None,
        last_channel: 0,
    };
    let mut buffer = vec![0u8; 2048];
    loop {
        let deadline = gateway.next_deadline();
        tokio::select! {
            received = socket.recv_from(&mut buffer) => {
                let (length, from) = received?;
                gateway.on_datagram(&buffer[..length], from).await;
            }
            event = events.recv() => match event {
                Ok(TunnelEvent::Telegram(frame)) => gateway.on_device_frame(&frame).await,
                Ok(TunnelEvent::Closed) => {}
                Err(RecvError::Lagged(missed)) => {
                    gateway.note(format!("device events lost: {missed}"));
                }
                Err(RecvError::Closed) => return Ok(()),
            },
            () = sleep_until(deadline), if deadline.is_some() => gateway.on_timer().await,
        }
    }
}

async fn sleep_until(deadline: Option<Instant>) {
    match deadline {
        Some(at) => tokio::time::sleep_until(at).await,
        None => std::future::pending().await,
    }
}

impl Gateway<'_> {
    fn observe(&self, event: GatewayEvent<'_>) {
        if let Some(observer) = &self.config.observer {
            observer(event);
        }
    }

    fn note(&self, text: String) {
        self.observe(GatewayEvent::Note(text));
    }

    fn slot(&mut self, kind: Kind) -> &mut Option<Connection> {
        match kind {
            Kind::Tunnel => &mut self.tunnel,
            Kind::Management => &mut self.management,
        }
    }

    fn connection(&self, kind: Kind) -> Option<&Connection> {
        match kind {
            Kind::Tunnel => self.tunnel.as_ref(),
            Kind::Management => self.management.as_ref(),
        }
    }

    fn kind_of_channel(&self, channel: u8) -> Option<Kind> {
        Kind::ALL
            .into_iter()
            .find(|kind| self.connection(*kind).is_some_and(|c| c.channel == channel))
    }

    fn next_deadline(&self) -> Option<Instant> {
        Kind::ALL
            .into_iter()
            .filter_map(|kind| self.connection(kind))
            .filter_map(|connection| {
                // A heartbeat timeout too large to add to an instant never expires.
                let heartbeat = connection
                    .last_heard
                    .checked_add(self.config.heartbeat_timeout);
                let flight = connection.in_flight.as_ref().map(|flight| flight.deadline);
                heartbeat.into_iter().chain(flight).min()
            })
            .min()
    }

    /// Sends one datagram. A failure (for example an HPAI naming a broadcast or unreachable
    /// address) is reported and the datagram dropped; it never stops the front-end.
    async fn transmit(&self, datagram: &[u8], to: SocketAddrV4) {
        self.observe(GatewayEvent::Datagram {
            from_tool: false,
            bytes: datagram,
        });
        if let Err(error) = self.socket.send_to(datagram, to).await {
            self.note(format!("sending to {to} failed, datagram dropped: {error}"));
        }
    }

    async fn send(&self, to: SocketAddrV4, service: u16, body: &[u8]) {
        self.transmit(&encode_frame(service, body), to).await;
    }

    async fn on_datagram(&mut self, bytes: &[u8], from: SocketAddr) {
        self.observe(GatewayEvent::Datagram {
            from_tool: true,
            bytes,
        });
        let SocketAddr::V4(from) = from else {
            return;
        };
        let Ok((header, body)) = decode_frame(bytes) else {
            self.note(format!("malformed datagram from {from} ignored"));
            return;
        };
        match header.service_type {
            SEARCH_REQUEST => self.on_search(body, from).await,
            DESCRIPTION_REQUEST => self.on_description(body, from).await,
            CONNECT_REQUEST => self.on_connect(body, from).await,
            CONNECTIONSTATE_REQUEST => self.on_connectionstate(body, from).await,
            DISCONNECT_REQUEST => self.on_disconnect(body, from).await,
            // The front-end forgets a connection as soon as it sends its own DISCONNECT_REQUEST,
            // so a response never closes anything; a stray one cannot clear a live channel.
            DISCONNECT_RESPONSE => self.note(format!(
                "DISCONNECT_RESPONSE from {from} for channel {:?} ignored",
                body.first()
            )),
            TUNNELLING_REQUEST => self.on_request(Kind::Tunnel, body).await,
            DEVICE_CONFIGURATION_REQUEST => self.on_request(Kind::Management, body).await,
            TUNNELLING_ACK => self.on_ack(Kind::Tunnel, body).await,
            DEVICE_CONFIGURATION_ACK => self.on_ack(Kind::Management, body).await,
            other => self.note(format!("service {other:#06x} is not implemented; ignored")),
        }
    }

    async fn on_search(&mut self, body: &[u8], from: SocketAddrV4) {
        let Ok((endpoint, _)) = Hpai::decode(body) else {
            return;
        };
        let mut answer = hpai_of(self.config.advertised).to_vec();
        answer.extend(self.device_info_dib());
        answer.extend(supported_families_dib());
        self.send(resolve(endpoint, from), SEARCH_RESPONSE, &answer)
            .await;
    }

    async fn on_description(&mut self, body: &[u8], from: SocketAddrV4) {
        let Ok((endpoint, _)) = Hpai::decode(body) else {
            return;
        };
        let mut answer = self.device_info_dib();
        answer.extend(supported_families_dib());
        self.send(resolve(endpoint, from), DESCRIPTION_RESPONSE, &answer)
            .await;
    }

    async fn on_connect(&mut self, body: &[u8], from: SocketAddrV4) {
        let parsed = Hpai::decode(body)
            .and_then(|(control, rest)| Hpai::decode(rest).map(|(data, cri)| (control, data, cri)));
        // A TCP HPAI on UDP is dropped without an answer: Core v01.06.02 AS §8.6.2.1 discards a
        // non-Route-Back TCP HPAI, Device Management v01.07.03 AS §2.3.2 ignores any TCP HPAI.
        let Ok((control, data, cri)) = parsed else {
            self.note("CONNECT_REQUEST with unreadable HPAI ignored".into());
            return;
        };
        let reply_to = resolve(control, from);
        let kind = match cri.get(1).copied() {
            Some(TUNNEL_CONNECTION) => Kind::Tunnel,
            Some(DEVICE_MGMT_CONNECTION) => Kind::Management,
            _ => {
                self.note("connect refused: connection type not served".into());
                return self
                    .send(reply_to, CONNECT_RESPONSE, &[0, E_CONNECTION_TYPE])
                    .await;
            }
        };
        if kind == Kind::Tunnel && cri.get(2).copied() != Some(TUNNEL_LINKLAYER) {
            self.note(format!(
                "connect refused: tunnelling layer {:?}",
                cri.get(2)
            ));
            return self
                .send(reply_to, CONNECT_RESPONSE, &[0, E_TUNNELLING_LAYER])
                .await;
        }
        if let Some(existing) = self.connection(kind) {
            if existing.control.ip() != from.ip() {
                self.note(format!("connect refused: {} busy", kind.name()));
                return self
                    .send(reply_to, CONNECT_RESPONSE, &[0, E_NO_MORE_CONNECTIONS])
                    .await;
            }
            // A tool that crashed or was restarted reconnects before the old
            // connection times out; the same host takes it over.
            self.note(format!(
                "{} {} taken over by its host",
                kind.name(),
                existing.channel
            ));
        }
        let channel = self.next_channel(kind);
        *self.slot(kind) = Some(Connection {
            channel,
            control: reply_to,
            data: resolve(data, from),
            receive_sequence: 0,
            send_sequence: 0,
            last_heard: Instant::now(),
            in_flight: None,
            queue: VecDeque::new(),
        });
        let mut answer = vec![channel, E_NO_ERROR];
        answer.extend(hpai_of(self.config.advertised));
        match kind {
            Kind::Tunnel => {
                let assigned = ManagementTransport::assigned_address(self.device);
                self.note(format!(
                    "tunnel {channel} opened for {from}, assigned address {assigned}"
                ));
                answer.extend([0x04, TUNNEL_CONNECTION]);
                answer.extend(assigned.raw().to_be_bytes());
            }
            Kind::Management => {
                self.note(format!("management connection {channel} opened for {from}"));
                answer.extend([0x02, DEVICE_MGMT_CONNECTION]);
            }
        }
        self.send(reply_to, CONNECT_RESPONSE, &answer).await;
    }

    /// The next channel ID after the last one handed out, skipping 0 and the channel the
    /// other connection still holds (the slot of `kind` itself is being replaced).
    fn next_channel(&mut self, kind: Kind) -> u8 {
        let taken = self.connection(kind.other()).map(|c| c.channel);
        loop {
            self.last_channel = self.last_channel.wrapping_add(1).max(1);
            if Some(self.last_channel) != taken {
                return self.last_channel;
            }
        }
    }

    async fn on_connectionstate(&mut self, body: &[u8], from: SocketAddrV4) {
        let Some(&channel) = body.first() else {
            return;
        };
        let kind = self.kind_of_channel(channel);
        let status = if kind.is_some() {
            E_NO_ERROR
        } else {
            E_CONNECTION_ID
        };
        if let Some(connection) = kind.and_then(|kind| self.slot(kind).as_mut()) {
            connection.last_heard = Instant::now();
        }
        let reply_to = body
            .get(2..)
            .and_then(|rest| Hpai::decode(rest).ok())
            .map_or(from, |(hpai, _)| resolve(hpai, from));
        self.send(reply_to, CONNECTIONSTATE_RESPONSE, &[channel, status])
            .await;
    }

    async fn on_disconnect(&mut self, body: &[u8], from: SocketAddrV4) {
        let Some(&channel) = body.first() else {
            return;
        };
        let kind = self.kind_of_channel(channel);
        let status = if kind.is_some() {
            E_NO_ERROR
        } else {
            E_CONNECTION_ID
        };
        let reply_to = body
            .get(2..)
            .and_then(|rest| Hpai::decode(rest).ok())
            .map_or(from, |(hpai, _)| resolve(hpai, from));
        if let Some(kind) = kind {
            *self.slot(kind) = None;
            self.note(format!("{} {channel} closed by the tool", kind.name()));
        }
        self.send(reply_to, DISCONNECT_RESPONSE, &[channel, status])
            .await;
    }

    /// Ends a connection from the server side: Core v01.06.02 AS §5.4 (no heartbeat) and
    /// Tunnelling v01.07.01 AS §2.6.1 (no good ACK after one repeat) both require a
    /// `DISCONNECT_REQUEST` to the client's control endpoint. The slot is freed at once; the
    /// client's `DISCONNECT_RESPONSE` is not awaited.
    async fn terminate(&mut self, kind: Kind, reason: &str) {
        let Some(connection) = self.slot(kind).take() else {
            return;
        };
        self.note(format!(
            "{} {} dropped: {reason}",
            kind.name(),
            connection.channel
        ));
        let body = encode_disconnect_request(
            connection.channel,
            Hpai {
                addr: *self.config.advertised.ip(),
                port: self.config.advertised.port(),
            },
        );
        self.send(connection.control, DISCONNECT_REQUEST, &body)
            .await;
    }

    /// A `TUNNELLING_REQUEST` or `DEVICE_CONFIGURATION_REQUEST`: same connection header and
    /// sequence rules (Tunnelling §2.6, Device Management §2.3), different payload.
    async fn on_request(&mut self, kind: Kind, body: &[u8]) {
        let Ok(request) = decode_tunnelling_request(body) else {
            return;
        };
        let Some(connection) = self.slot(kind).as_mut() else {
            return;
        };
        if request.channel_id != connection.channel {
            return;
        }
        let expected = connection.receive_sequence;
        let duplicate = request.sequence_counter == expected.wrapping_sub(1);
        if request.sequence_counter != expected && !duplicate {
            self.note(format!(
                "{} request with sequence {} (expected {expected}) ignored",
                kind.name(),
                request.sequence_counter
            ));
            return;
        }
        // Core v01.06.02 AS §5.4, p14: rejected sequence numbers do not retrigger heartbeat.
        connection.last_heard = Instant::now();
        let data = connection.data;
        let ack = encode_tunnelling_ack(request.channel_id, request.sequence_counter, E_NO_ERROR);
        self.send(data, kind.ack_service(), &ack).await;
        if duplicate {
            return;
        }
        if let Some(connection) = self.slot(kind).as_mut() {
            connection.receive_sequence = expected.wrapping_add(1);
        }
        match kind {
            Kind::Tunnel => self.on_tool_frame(request.cemi).await,
            Kind::Management => self.on_management(request.cemi).await,
        }
    }

    async fn on_tool_frame(&mut self, cemi: &[u8]) {
        let frame = match decode_l_data(cemi) {
            Ok(frame) => frame,
            Err(error) => {
                self.observe(GatewayEvent::Undecodable {
                    cemi,
                    reason: format!("{error:?}"),
                });
                return;
            }
        };
        self.observe(GatewayEvent::Frame {
            from_tool: true,
            cemi,
            frame: &frame,
        });
        if frame.kind != LDataMessageKind::Request {
            return;
        }
        // The simulator confirms a T_Connect itself; every other request is
        // confirmed here, before the device's answer can be queued behind it.
        if frame.transport != Tpci::Connect {
            let confirmation = LDataFrame {
                kind: LDataMessageKind::Confirmation { error: false },
                source: ManagementTransport::assigned_address(self.device),
                destination: frame.destination,
                transport: frame.transport,
                service: frame.service.clone(),
                control: frame.control,
            };
            self.enqueue(Kind::Tunnel, &confirmation).await;
        }
        if let Err(error) = self
            .device
            .send_frame(frame.destination, frame.transport, frame.service)
            .await
        {
            self.note(format!("device refused the frame: {error:?}"));
        }
    }

    /// `M_PropRead.req` is answered from the configured properties, anything else listed as
    /// unanswered; every message is recorded so a session shows what the tool asked for.
    /// A negative `.con` carries NoE 0 and the request's full 12-bit start index
    /// (EMI_IMI v01.04.02 AS §4.1.7.3.3 and §4.1.7.3.5).
    async fn on_management(&mut self, cemi: &[u8]) {
        let summary = describe_management(cemi);
        self.observe(GatewayEvent::Management {
            from_tool: true,
            cemi,
            summary,
        });
        let answer = match cemi {
            [M_PROP_READ_REQ, ot_hi, ot_lo, instance, pid, count_start, start_lo] => {
                let key = (u16::from_be_bytes([*ot_hi, *ot_lo]), *instance, *pid);
                let wanted = count_start >> 4;
                let start_hi = count_start & 0x0F;
                let mut out = vec![M_PROP_READ_CON, *ot_hi, *ot_lo, *instance, *pid];
                match self.config.properties.get(&key) {
                    Some(value) => {
                        out.extend([(wanted.max(1) << 4) | start_hi, *start_lo]);
                        out.extend(value);
                    }
                    None => {
                        out.extend([start_hi, *start_lo, PROP_ERROR_VOID]);
                    }
                }
                Some(out)
            }
            [M_PROP_WRITE_REQ, ot_hi, ot_lo, instance, pid, count_start, start_lo, ..] => {
                Some(vec![
                    M_PROP_WRITE_CON,
                    *ot_hi,
                    *ot_lo,
                    *instance,
                    *pid,
                    count_start & 0x0F,
                    *start_lo,
                    PROP_ERROR_READ_ONLY,
                ])
            }
            _ => None,
        };
        if let Some(answer) = answer {
            self.observe(GatewayEvent::Management {
                from_tool: false,
                cemi: &answer,
                summary: describe_management(&answer),
            });
            self.enqueue_raw(Kind::Management, answer).await;
        }
    }

    async fn on_device_frame(&mut self, frame: &LDataFrame) {
        if self.tunnel.is_none() {
            return;
        }
        self.enqueue(Kind::Tunnel, frame).await;
    }

    async fn enqueue(&mut self, kind: Kind, frame: &LDataFrame) {
        let cemi = match encode_l_data(frame) {
            Ok(cemi) => cemi,
            Err(error) => {
                self.note(format!("frame cannot be encoded: {error:?}"));
                return;
            }
        };
        self.observe(GatewayEvent::Frame {
            from_tool: false,
            cemi: &cemi,
            frame,
        });
        self.enqueue_raw(kind, cemi).await;
    }

    async fn enqueue_raw(&mut self, kind: Kind, cemi: Vec<u8>) {
        if let Some(connection) = self.slot(kind).as_mut() {
            connection.queue.push_back(cemi);
        }
        self.pump(kind).await;
    }

    /// Sends the next queued frame when none is waiting for its ACK.
    async fn pump(&mut self, kind: Kind) {
        let Some(connection) = self.slot(kind).as_mut() else {
            return;
        };
        if connection.in_flight.is_some() {
            return;
        }
        let Some(cemi) = connection.queue.pop_front() else {
            return;
        };
        let sequence = connection.send_sequence;
        connection.send_sequence = sequence.wrapping_add(1);
        let body = encode_tunnelling_request(connection.channel, sequence, &cemi);
        let datagram = encode_frame(kind.request_service(), &body);
        let data = connection.data;
        connection.in_flight = Some(InFlight {
            sequence,
            datagram: datagram.clone(),
            deadline: Instant::now() + ACK_TIMEOUT,
            repeated: false,
        });
        self.transmit(&datagram, data).await;
    }

    /// An ACK counts only on a connection of its own kind (a `TUNNELLING_ACK` never settles a
    /// management frame) and only with status `E_NO_ERROR`; an error status is treated like a
    /// missing ACK (Tunnelling v01.07.01 AS §2.6.1).
    async fn on_ack(&mut self, kind: Kind, body: &[u8]) {
        let Ok(ack) = decode_tunnelling_ack(body) else {
            return;
        };
        let Some(connection) = self.slot(kind).as_mut() else {
            return;
        };
        if connection.channel != ack.channel_id {
            return;
        }
        let matches = connection
            .in_flight
            .as_ref()
            .is_some_and(|flight| flight.sequence == ack.sequence_counter);
        if !matches {
            return;
        }
        if ack.status == E_NO_ERROR {
            connection.last_heard = Instant::now();
            connection.in_flight = None;
            self.pump(kind).await;
        } else {
            self.note(format!(
                "{} {} ACK status {:#04x}",
                kind.name(),
                ack.channel_id,
                ack.status
            ));
            self.repeat_or_terminate(kind).await;
        }
    }

    /// The frame in flight was not acknowledged: repeat it once, and end the connection
    /// if it already was repeated (Tunnelling v01.07.01 AS §2.6.1).
    async fn repeat_or_terminate(&mut self, kind: Kind) {
        let Some(connection) = self.slot(kind).as_mut() else {
            return;
        };
        let data = connection.data;
        let Some(flight) = connection.in_flight.as_mut() else {
            return;
        };
        if flight.repeated {
            self.terminate(kind, "no ACK after one repeat").await;
            return;
        }
        flight.repeated = true;
        flight.deadline = Instant::now() + ACK_TIMEOUT;
        let datagram = flight.datagram.clone();
        self.transmit(&datagram, data).await;
    }

    async fn on_timer(&mut self) {
        let now = Instant::now();
        for kind in Kind::ALL {
            let heartbeat_timeout = self.config.heartbeat_timeout;
            let Some(connection) = self.connection(kind) else {
                continue;
            };
            let silent = connection
                .last_heard
                .checked_add(heartbeat_timeout)
                .is_some_and(|deadline| now >= deadline);
            if silent {
                self.terminate(kind, "no heartbeat").await;
                continue;
            }
            if connection
                .in_flight
                .as_ref()
                .is_some_and(|flight| now >= flight.deadline)
            {
                self.repeat_or_terminate(kind).await;
            }
        }
    }

    /// Core v01.06.02 AS §7.5.4.2: the 54-octet device-information DIB.
    fn device_info_dib(&self) -> Vec<u8> {
        let mut dib = vec![54, DIB_DEVICE_INFO, MEDIUM_TP1, 0x00];
        dib.extend(self.config.gateway_address.raw().to_be_bytes());
        dib.extend([0x00, 0x00]); // project-installation identifier
        dib.extend([0x00, 0x00, 0x00, 0x00, 0x00, 0x01]); // made-up serial number
        dib.extend([224, 0, 23, 12]); // routing multicast address (routing is not served)
        dib.extend([0x02, 0x00, 0x00, 0x00, 0x00, 0x01]); // made-up locally administered MAC
        let mut name = [0u8; 30];
        for (slot, ch) in name
            .iter_mut()
            .zip(self.config.friendly_name.chars().take(29))
        {
            *slot = u8::try_from(u32::from(ch)).unwrap_or(b'?');
        }
        dib.extend(name);
        dib
    }
}

/// One line for a management message: its name and, for property services, object/property.
fn describe_management(cemi: &[u8]) -> String {
    let name = match cemi.first() {
        Some(&M_PROP_READ_REQ) => "M_PropRead.req",
        Some(&M_PROP_READ_CON) => "M_PropRead.con",
        Some(&M_PROP_WRITE_REQ) => "M_PropWrite.req",
        Some(&M_PROP_WRITE_CON) => "M_PropWrite.con",
        Some(0xF1) => "M_Reset.req",
        Some(0xF0) => "M_Reset.ind",
        Some(0xF7) => "M_PropInfo.ind",
        Some(0xF8) => "M_FuncPropCommand.req",
        Some(0xF9) => "M_FuncPropStateRead.req",
        Some(0xFA) => "M_FuncPropCon",
        Some(_) => "unknown management code",
        None => "empty",
    };
    match cemi {
        [_, ot_hi, ot_lo, instance, pid, count_start, start_lo, rest @ ..] => format!(
            "{name} object {:#06x} instance {instance} property {pid} count {} start {} data {:02x?}",
            u16::from_be_bytes([*ot_hi, *ot_lo]),
            count_start >> 4,
            (u16::from(count_start & 0x0F) << 8) | u16::from(*start_lo),
            rest
        ),
        _ => name.to_string(),
    }
}

/// Core v01.06.02 AS §7.5.4.3: the services this front-end really serves. Device Management
/// (03h) is allowed in a SEARCH_RESPONSE and DESCRIPTION_RESPONSE (Table 3) and is served.
fn supported_families_dib() -> Vec<u8> {
    vec![
        8,
        DIB_SUPP_SVC_FAMILIES,
        FAMILY_CORE,
        1,
        FAMILY_DEVICE_MANAGEMENT,
        1,
        FAMILY_TUNNELLING,
        1,
    ]
}

fn hpai_of(endpoint: SocketAddrV4) -> [u8; HPAI_LEN as usize] {
    Hpai {
        addr: *endpoint.ip(),
        port: endpoint.port(),
    }
    .encode()
}

/// An HPAI of `0.0.0.0:0` is the Route Back / NAT form (Core §8.4.3.4.3): reply to the sender.
fn resolve(hpai: Hpai, from: SocketAddrV4) -> SocketAddrV4 {
    if hpai.addr == Ipv4Addr::UNSPECIFIED || hpai.port == 0 {
        from
    } else {
        SocketAddrV4::new(hpai.addr, hpai.port)
    }
}

#[cfg(test)]
mod tests;
