//! The `BusConnection` trait and its tunnelling implementation
//! (ARCHITECTURE.md §8). `discover` (multicast `SEARCH_REQUEST`/
//! `SEARCH_RESPONSE`) is implemented as of Session 6 Cycle 3.
//! `TunnelClient::send` is implemented as of Session 6 Cycle 2
//! (KNOWN_LIMITATIONS.md §26). `connect_routing`/`RoutingClient` are
//! implemented as of Session 6 Cycle 4. Session 6 Cycle 5 hardens
//! connection management: `wait_for_reply` fixes a heartbeat/ack retry
//! race (KNOWN_LIMITATIONS.md #27), `TunnelEvent::Closed` signals
//! subscribers when the tunnel dies (#28), and `RoutingClient::send`
//! honors `ROUTING_BUSY` (#32).

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use knx_core::IndividualAddress;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, Mutex, Notify};

use crate::cemi::{self, ApplicationService, Destination, LDataFrame};
use crate::core::dib;
use crate::core::hpai::Hpai;
use crate::core::services;
use crate::discovery;
use crate::frame;
use crate::tunnelling;

#[derive(Debug)]
pub enum BusError {
    Io(std::io::Error),
    Timeout,
    ConnectionRefused(u8),
    NotImplemented,
    /// Any lower-layer codec error, carried as text — `frame`/`core::hpai`/
    /// `core::services`/`tunnelling`/`cemi` each already have their own
    /// precise error type; this cycle's client only ever needs to report
    /// them upward, not branch on which one it was.
    Protocol(String),
}

impl std::fmt::Display for BusError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BusError::Io(e) => write!(f, "I/O error: {e}"),
            BusError::Timeout => write!(f, "timed out waiting for a response"),
            BusError::ConnectionRefused(status) => {
                write!(f, "gateway refused the connection (status {status:#04x})")
            }
            BusError::NotImplemented => write!(f, "not implemented in this cycle"),
            BusError::Protocol(msg) => write!(f, "protocol error: {msg}"),
        }
    }
}

impl std::error::Error for BusError {}

/// A gateway found via `discover()` (Core v01.06.02 AS §7.4.1's
/// `SEARCH_RESPONSE`). `supports_tunnelling` is derived from the
/// Supported Service Families DIB, when the gateway sends one — some
/// gateways only support routing, and this is how a caller finds out
/// before trying `connect_tunnel` on it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredGateway {
    pub control_endpoint: SocketAddrV4,
    pub individual_address: IndividualAddress,
    pub friendly_name: String,
    pub supports_tunnelling: bool,
}

/// A local KNXnet/IP client, not yet connected to any gateway. `discover`
/// (multicast `SEARCH_REQUEST`, Core v01.06.02 AS §4.2) and `connect_tunnel`
/// are both implemented as of Session 6 Cycle 3; `connect_routing` as of
/// Session 6 Cycle 4.
pub struct KnxNetIpClient;

impl KnxNetIpClient {
    pub fn new() -> Self {
        Self
    }
}

impl Default for KnxNetIpClient {
    fn default() -> Self {
        Self::new()
    }
}

// Native `async fn` in a trait (stable since Rust 1.75, well within this
// workspace's 1.98 floor) — no `async-trait` dependency needed as long as
// nothing needs `dyn BusConnection`, which this cycle never does. Rustc's
// `async_fn_in_trait` lint fires because it can't add a `Send` bound to the
// returned future for us; that bound isn't needed here (no `dyn`, single
// implementer, called directly), so it's allowed rather than worked around.
#[allow(async_fn_in_trait)]
pub trait BusConnection {
    async fn discover(&self) -> Result<Vec<DiscoveredGateway>, BusError>;
    async fn connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient, BusError>;
    async fn connect_routing(
        &self,
        own_address: IndividualAddress,
    ) -> Result<RoutingClient, BusError>;
}

/// Standard KNXnet/IP discovery/routing multicast group and port (Core
/// v01.06.02 AS §4.2). Hardcoded this cycle — not a CLI override (design
/// spec's Cycle 3 Q2).
const DISCOVERY_MULTICAST: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(224, 0, 23, 12), 3671);

/// Standard KNXnet/IP routing multicast group and port (Routing v01.05.02
/// AS §2.3.1) — same address/port discovery already uses (Core v01.06.02
/// AS §4.2), kept as its own named constant since routing and discovery
/// are separate features that happen to share a default today.
const ROUTING_MULTICAST: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(224, 0, 23, 12), 3671);

/// Core v01.06.02 AS §5.2.4 `SEARCH_TIMEOUT`: how long to keep collecting
/// `SEARCH_RESPONSE`s after sending one `SEARCH_REQUEST` (design spec's
/// Cycle 3 Q3 — full spec value, not a shortened one).
const SEARCH_TIMEOUT_SECS: u64 = 10;

impl BusConnection for KnxNetIpClient {
    async fn discover(&self) -> Result<Vec<DiscoveredGateway>, BusError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(BusError::Io)?;
        let discovery_endpoint = local_discovery_hpai(&socket).await?;
        let request_body = discovery::encode_search_request(discovery_endpoint);
        let datagram = frame::encode_frame(services::SEARCH_REQUEST, &request_body);
        socket
            .send_to(&datagram, DISCOVERY_MULTICAST)
            .await
            .map_err(BusError::Io)?;

        let mut gateways: Vec<DiscoveredGateway> = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(SEARCH_TIMEOUT_SECS);
        let mut buf = [0u8; 1024];
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            let Ok(Ok((n, _src))) =
                tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await
            else {
                break; // window elapsed, or the socket errored: stop collecting
            };
            let Ok((header, resp_body)) = frame::decode_frame(&buf[..n]) else {
                continue;
            };
            if header.service_type != services::SEARCH_RESPONSE {
                continue;
            }
            let Ok(response) = discovery::decode_search_response(resp_body) else {
                continue;
            };
            let control_endpoint = SocketAddrV4::new(
                response.control_endpoint.addr,
                response.control_endpoint.port,
            );
            if gateways
                .iter()
                .any(|g| g.control_endpoint == control_endpoint)
            {
                continue;
            }
            let supports_tunnelling = response
                .service_families
                .as_ref()
                .is_some_and(|f| f.supports(dib::SERVICE_FAMILY_TUNNELLING));
            gateways.push(DiscoveredGateway {
                control_endpoint,
                individual_address: response.device_info.individual_address,
                friendly_name: response.device_info.friendly_name,
                supports_tunnelling,
            });
        }
        Ok(gateways)
    }

    async fn connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient, BusError> {
        TunnelClient::connect(gateway).await
    }

    async fn connect_routing(
        &self,
        own_address: IndividualAddress,
    ) -> Result<RoutingClient, BusError> {
        RoutingClient::connect(own_address).await
    }
}

/// What a `TunnelClient` subscriber receives: either a telegram off the bus,
/// or a one-time notice that the tunnel is gone (KNOWN_LIMITATIONS.md #28).
/// Before this, a dead tunnel and a quiet bus were indistinguishable —
/// `subscribe()`'s receiver just stopped yielding anything, forever, either
/// way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TunnelEvent {
    Telegram(LDataFrame),
    /// Sent exactly once, as the last message on this channel, from
    /// whichever cause tore the tunnel down: an explicit `disconnect()`,
    /// `heartbeat_loop` exhausting its retries, or the gateway sending its
    /// own `DISCONNECT_REQUEST`.
    Closed,
}

struct TunnelState {
    socket: UdpSocket,
    channel_id: u8,
    assigned_address: IndividualAddress,
    tx: broadcast::Sender<TunnelEvent>,
    heartbeat_reply: Mutex<Option<u8>>,
    heartbeat_notify: Notify,
    shutdown: Notify,
    /// Guards the whole send-and-wait-for-ack critical section, so at most
    /// one `TUNNELLING_REQUEST` is outstanding at a time (Tunnelling
    /// v01.07.01 AS §2.6 assumes one un-acked request per direction) —
    /// its value is the next sequence counter to use, tracked separately
    /// from `receive_loop`'s own `recv_seq` since the two directions
    /// number independently.
    send_seq: Mutex<u8>,
    ack_reply: Mutex<Option<(u8, u8)>>,
    ack_notify: Notify,
}

/// `disconnect()` is the graceful path — it tells the gateway we're leaving
/// and stops the background tasks. The `Drop` impl below is a safety net for
/// callers that drop a `TunnelClient` without calling it (early return, panic
/// unwind, ...); it is not a substitute for calling `disconnect()`.
pub struct TunnelClient {
    state: Arc<TunnelState>,
}

impl TunnelClient {
    async fn connect(gateway: SocketAddrV4) -> Result<Self, BusError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(BusError::Io)?;
        socket.connect(gateway).await.map_err(BusError::Io)?;
        let control_hpai = local_hpai(&socket)?;

        let cri = tunnelling::TunnelCri {
            layer: tunnelling::TUNNEL_LINKLAYER,
        }
        .encode();
        let body = services::encode_connect_request(control_hpai, control_hpai, &cri);
        let datagram = frame::encode_frame(services::CONNECT_REQUEST, &body);
        socket.send(&datagram).await.map_err(BusError::Io)?;

        let mut buf = [0u8; 64];
        let n = tokio::time::timeout(Duration::from_secs(10), socket.recv(&mut buf))
            .await
            .map_err(|_| BusError::Timeout)?
            .map_err(BusError::Io)?;
        let (header, resp_body) =
            frame::decode_frame(&buf[..n]).map_err(|e| BusError::Protocol(e.to_string()))?;
        if header.service_type != services::CONNECT_RESPONSE {
            return Err(BusError::Protocol(format!(
                "expected CONNECT_RESPONSE, got service type {:#06x}",
                header.service_type
            )));
        }
        let response = services::decode_connect_response(resp_body)
            .map_err(|e| BusError::Protocol(e.to_string()))?;
        if response.status != services::E_NO_ERROR {
            return Err(BusError::ConnectionRefused(response.status));
        }
        let crd = tunnelling::TunnelCrd::decode(&response.crd)
            .map_err(|e| BusError::Protocol(e.to_string()))?;

        let (tx, _rx) = broadcast::channel(64);
        let state = Arc::new(TunnelState {
            socket,
            channel_id: response.channel_id,
            assigned_address: crd.individual_address,
            tx,
            heartbeat_reply: Mutex::new(None),
            heartbeat_notify: Notify::new(),
            shutdown: Notify::new(),
            send_seq: Mutex::new(0),
            ack_reply: Mutex::new(None),
            ack_notify: Notify::new(),
        });

        tokio::spawn(receive_loop(state.clone()));
        tokio::spawn(heartbeat_loop(state.clone()));

        Ok(TunnelClient { state })
    }

    pub fn assigned_address(&self) -> IndividualAddress {
        self.state.assigned_address
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.state.tx.subscribe()
    }

    /// Sends an `L_Data.req` (Tunnelling v01.07.01 AS §2.6): source/kind
    /// are the client's concern, not the caller's — the gateway assigns
    /// the actual source address and message code, so only the
    /// destination and application service are exposed here.
    ///
    /// Per §2.6.1/§2.6.2: waits up to `TUNNELLING_REQUEST_TIMEOUT` (1s)
    /// for a matching `TUNNELLING_ACK`; on timeout or an error status,
    /// repeats the same `TUNNELLING_REQUEST` once with the same sequence
    /// counter. If that repeat also fails, the connection is terminated
    /// (a `DISCONNECT_REQUEST` is sent, best-effort, and the background
    /// tasks are told to stop) and `BusError::Timeout` is returned — the
    /// same outcome `heartbeat_loop` already reaches on repeated failure.
    pub async fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let frame = LDataFrame {
            kind: cemi::LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0),
            destination,
            service,
        };
        let cemi_bytes = cemi::encode_l_data(&frame);

        let mut seq_guard = self.state.send_seq.lock().await;
        let seq = *seq_guard;
        let datagram = frame::encode_frame(
            tunnelling::TUNNELLING_REQUEST,
            &tunnelling::encode_tunnelling_request(self.state.channel_id, seq, &cemi_bytes),
        );

        for _attempt in 0..2 {
            *self.state.ack_reply.lock().await = None;
            self.state
                .socket
                .send(&datagram)
                .await
                .map_err(BusError::Io)?;
            let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
            let reply = wait_for_reply(
                &self.state.ack_notify,
                &self.state.ack_reply,
                deadline,
                |&(ack_seq, _)| ack_seq == seq,
            )
            .await;
            if let Some((_, status)) = reply {
                if status == tunnelling::E_NO_ERROR {
                    *seq_guard = seq.wrapping_add(1);
                    return Ok(());
                }
            }
        }

        drop(seq_guard);
        self.state.shutdown.notify_waiters();
        let _ = self.try_send_disconnect_request().await;
        Err(BusError::Timeout)
    }

    /// Best-effort graceful disconnect (Core v01.06.02 AS §5.5): sends
    /// `DISCONNECT_REQUEST` and signals the background tasks to stop. Does
    /// not block on the server's `DISCONNECT_RESPONSE` — `receive_loop`
    /// observes it (or the socket simply going quiet) and exits on its own.
    ///
    /// `shutdown.notify_waiters()` always fires, even if sending the
    /// datagram fails — `disconnect` consumes `self`, so a caller who got an
    /// `Err` here has no way to retry; the background tasks must still be
    /// told to stop rather than leaking forever.
    pub async fn disconnect(self) -> Result<(), BusError> {
        let result = self.try_send_disconnect_request().await;
        self.state.shutdown.notify_waiters();
        result
    }

    async fn try_send_disconnect_request(&self) -> Result<(), BusError> {
        let control_hpai = local_hpai(&self.state.socket)?;
        let body = services::encode_disconnect_request(self.state.channel_id, control_hpai);
        let datagram = frame::encode_frame(services::DISCONNECT_REQUEST, &body);
        self.state
            .socket
            .send(&datagram)
            .await
            .map_err(BusError::Io)?;
        Ok(())
    }
}

impl Drop for TunnelClient {
    fn drop(&mut self) {
        // Safety net for a `TunnelClient` dropped without calling
        // `disconnect()` (early return, panic unwind, ...). Idempotent
        // alongside the explicit path: `notify_waiters()` twice is harmless.
        self.state.shutdown.notify_waiters();
    }
}

struct RoutingState {
    socket: UdpSocket,
    tx: broadcast::Sender<LDataFrame>,
    shutdown: Notify,
    /// Set while a received `ROUTING_BUSY` is still in effect (Routing
    /// v01.05.02 AS §2.3.5's mandatory "device receiving ROUTING_BUSY"
    /// rule) — `None` when clear. `send()` waits this out before
    /// transmitting. The spec's optional additional random back-off
    /// (`trandom`, a `MAY`) is not implemented — see KNOWN_LIMITATIONS.md.
    busy_until: Mutex<Option<tokio::time::Instant>>,
}

/// A KNXnet/IP routing endpoint — joined to the standard routing
/// multicast group, sending and receiving `ROUTING_INDICATION` frames
/// unconfirmed (Routing v01.05.02 AS §5.1). Unlike `TunnelClient`, there
/// is no connection to a specific peer: `own_address` is this client's
/// own claimed source address for outgoing frames, not something a
/// gateway assigns, since routing has no `CONNECT_REQUEST`/`CRD`
/// handshake to assign one through.
pub struct RoutingClient {
    state: Arc<RoutingState>,
    own_address: IndividualAddress,
}

impl RoutingClient {
    async fn connect(own_address: IndividualAddress) -> Result<Self, BusError> {
        use socket2::{Domain, Socket, Type};

        let socket2_socket = Socket::new(Domain::IPV4, Type::DGRAM, None).map_err(BusError::Io)?;
        socket2_socket
            .set_reuse_address(true)
            .map_err(BusError::Io)?;
        socket2_socket
            .bind(
                &std::net::SocketAddr::from((Ipv4Addr::UNSPECIFIED, ROUTING_MULTICAST.port()))
                    .into(),
            )
            .map_err(BusError::Io)?;
        socket2_socket.set_nonblocking(true).map_err(BusError::Io)?;
        let std_socket: std::net::UdpSocket = socket2_socket.into();
        let socket = UdpSocket::from_std(std_socket).map_err(BusError::Io)?;

        socket
            .join_multicast_v4(*ROUTING_MULTICAST.ip(), Ipv4Addr::UNSPECIFIED)
            .map_err(BusError::Io)?;
        // Without this, our own sends would loop back through this same
        // socket and appear in `subscribe()` as if another device sent
        // them (design spec's Architecture section).
        socket.set_multicast_loop_v4(false).map_err(BusError::Io)?;

        let (tx, _rx) = broadcast::channel(64);
        let state = Arc::new(RoutingState {
            socket,
            tx,
            shutdown: Notify::new(),
            busy_until: Mutex::new(None),
        });
        tokio::spawn(routing_receive_loop(state.clone()));

        Ok(RoutingClient { state, own_address })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LDataFrame> {
        self.state.tx.subscribe()
    }

    /// Sends one `ROUTING_INDICATION` (Routing v01.05.02 AS §5.1: an
    /// unconfirmed service) — no ACK to wait for, no retry, unlike
    /// `TunnelClient::send`. Every routing frame on the wire is an
    /// `L_Data.ind` regardless of direction (§3.8), so `kind` is always
    /// `Indication`, never `Request`.
    pub async fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        self.wait_out_routing_busy().await;
        let frame = LDataFrame {
            kind: cemi::LDataMessageKind::Indication,
            source: self.own_address,
            destination,
            service,
        };
        let datagram =
            frame::encode_frame(services::ROUTING_INDICATION, &cemi::encode_l_data(&frame));
        self.state
            .socket
            .send_to(&datagram, ROUTING_MULTICAST)
            .await
            .map_err(BusError::Io)?;
        Ok(())
    }

    /// Blocks until any `ROUTING_BUSY` in effect has elapsed (Routing
    /// v01.05.02 AS §2.3.5, KNOWN_LIMITATIONS.md #32). Re-checks after
    /// waking in case a later `ROUTING_BUSY` extended the deadline while
    /// this was asleep.
    async fn wait_out_routing_busy(&self) {
        loop {
            let deadline = *self.state.busy_until.lock().await;
            match deadline {
                Some(d) if d > tokio::time::Instant::now() => tokio::time::sleep_until(d).await,
                _ => return,
            }
        }
    }
}

impl Drop for RoutingClient {
    fn drop(&mut self) {
        // Safety net mirroring `TunnelClient`'s Drop — stops the
        // background receive loop. Leaving the multicast group itself is
        // the OS's job when the socket closes, no explicit call needed.
        self.state.shutdown.notify_waiters();
    }
}

/// Routing v01.05.02 AS §2.3.5: "If another ROUTING_BUSY Frame is received
/// before the time tw has elapsed[,] the resulting time tw shall be
/// determined by the higher value of the remaining time of a previous
/// ROUTING_BUSY and the value tw received with this last ROUTING_BUSY."
fn merge_busy_deadline(
    existing: Option<tokio::time::Instant>,
    new_deadline: tokio::time::Instant,
) -> tokio::time::Instant {
    match existing {
        Some(e) if e > new_deadline => e,
        _ => new_deadline,
    }
}

/// The sole reader of `state.socket` — routing has no second concurrent
/// task touching it (no heartbeat, unlike `TunnelClient`).
async fn routing_receive_loop(state: Arc<RoutingState>) {
    let mut buf = [0u8; 1024];
    loop {
        let n = tokio::select! {
            _ = state.shutdown.notified() => break,
            result = state.socket.recv(&mut buf) => match result {
                Ok(n) => n,
                Err(_) => break,
            },
        };
        let Ok((header, body)) = frame::decode_frame(&buf[..n]) else {
            continue; // malformed datagram: ignore it, don't crash (§6.2/§6.3-style tolerance)
        };
        match header.service_type {
            services::ROUTING_INDICATION => {
                if let Ok(telegram) = cemi::decode_l_data(body) {
                    let _ = state.tx.send(telegram);
                }
            }
            services::ROUTING_LOST_MESSAGE => {
                if let Ok(msg) = crate::routing::decode_routing_lost_message(body) {
                    eprintln!(
                        "ROUTING_LOST_MESSAGE: device_state={:#04x}, lost {} message(s)",
                        msg.device_state, msg.lost_message_count
                    );
                }
            }
            services::ROUTING_BUSY => {
                if let Ok(busy) = crate::routing::decode_routing_busy(body) {
                    eprintln!(
                        "ROUTING_BUSY: device_state={:#04x}, wait {}ms, control={:#06x}",
                        busy.device_state, busy.wait_time_ms, busy.control_field
                    );
                    let new_deadline = tokio::time::Instant::now()
                        + Duration::from_millis(u64::from(busy.wait_time_ms));
                    let mut guard = state.busy_until.lock().await;
                    *guard = Some(merge_busy_deadline(*guard, new_deadline));
                }
            }
            _ => {} // unsupported/unknown service type: ignore
        }
    }
}

fn local_hpai(socket: &UdpSocket) -> Result<Hpai, BusError> {
    match socket.local_addr().map_err(BusError::Io)? {
        SocketAddr::V4(addr) => Ok(Hpai {
            addr: *addr.ip(),
            port: addr.port(),
        }),
        SocketAddr::V6(_) => Err(BusError::Protocol(
            "local socket bound to an IPv6 address, expected IPv4".to_string(),
        )),
    }
}

/// `local_hpai(&socket)` alone is not enough for the discovery socket:
/// `socket` is bound to the wildcard address and stays unconnected (it must,
/// to receive unicast `SEARCH_RESPONSE`s from any gateway), so
/// `local_addr()`/`getsockname()` reports IP `0.0.0.0` — a real interface
/// address is only resolved once a socket is `connect()`ed and the kernel
/// picks an outgoing route. Core v01.06.02 AS §8.6.2.2: an HPAI with one of
/// {address, port} zero and the other non-zero is invalid and must be
/// ignored by the receiving device — exactly what an unconnected
/// wildcard-bound socket would otherwise produce here. Work around it with
/// a throwaway socket connected to the multicast group purely to force
/// route resolution (UDP `connect()` sends no packet), then pair its
/// resolved IP with the real discovery socket's actual port.
async fn local_discovery_hpai(socket: &UdpSocket) -> Result<Hpai, BusError> {
    let probe = UdpSocket::bind("0.0.0.0:0").await.map_err(BusError::Io)?;
    probe
        .connect(DISCOVERY_MULTICAST)
        .await
        .map_err(BusError::Io)?;
    let probe_hpai = local_hpai(&probe)?;
    let real_port = local_hpai(socket)?.port;
    Ok(Hpai {
        addr: probe_hpai.addr,
        port: real_port,
    })
}

/// The sole reader of `state.socket` for the lifetime of the connection —
/// `heartbeat_loop` only ever sends, then waits on `heartbeat_notify`, so
/// there is never a second concurrent reader racing this one.
async fn receive_loop(state: Arc<TunnelState>) {
    let mut recv_seq: u8 = 0;
    let mut buf = [0u8; 1024];
    loop {
        let n = tokio::select! {
            _ = state.shutdown.notified() => break,
            result = state.socket.recv(&mut buf) => match result {
                Ok(n) => n,
                Err(_) => break, // socket gone; nothing more to receive
            },
        };
        let Ok((header, body)) = frame::decode_frame(&buf[..n]) else {
            continue; // malformed datagram: ignore it (Core v01.06.02 AS §6.2/§6.3), don't crash
        };
        match header.service_type {
            tunnelling::TUNNELLING_REQUEST => {
                let Ok(req) = tunnelling::decode_tunnelling_request(body) else {
                    continue;
                };
                if req.channel_id != state.channel_id {
                    continue;
                }
                if req.sequence_counter == recv_seq {
                    send_ack(&state, req.sequence_counter, tunnelling::E_NO_ERROR).await;
                    recv_seq = recv_seq.wrapping_add(1);
                    if let Ok(telegram) = cemi::decode_l_data(req.cemi) {
                        let _ = state.tx.send(TunnelEvent::Telegram(telegram));
                    }
                } else if req.sequence_counter == recv_seq.wrapping_sub(1) {
                    // Duplicate of the frame just processed (our own ACK
                    // was presumably lost) — ack again, discard (Tunnelling
                    // v01.07.01 AS §2.6.1).
                    send_ack(&state, req.sequence_counter, tunnelling::E_NO_ERROR).await;
                }
                // Any other sequence number: no ack, discard (§2.6.1).
            }
            tunnelling::TUNNELLING_ACK => {
                if let Ok(ack) = tunnelling::decode_tunnelling_ack(body) {
                    if ack.channel_id == state.channel_id {
                        *state.ack_reply.lock().await = Some((ack.sequence_counter, ack.status));
                        state.ack_notify.notify_one();
                    }
                }
            }
            services::CONNECTIONSTATE_RESPONSE => {
                if let Ok(resp) = services::decode_connectionstate_response(body) {
                    if resp.channel_id == state.channel_id {
                        *state.heartbeat_reply.lock().await = Some(resp.status);
                        state.heartbeat_notify.notify_one();
                    }
                }
            }
            services::DISCONNECT_REQUEST => {
                // Server-initiated disconnect (Core v01.06.02 AS §5.5):
                // acknowledge with a DISCONNECT_RESPONSE, then tear down.
                let body =
                    services::encode_disconnect_response(state.channel_id, services::E_NO_ERROR);
                let datagram = frame::encode_frame(services::DISCONNECT_RESPONSE, &body);
                let _ = state.socket.send(&datagram).await; // best-effort, we're tearing down anyway
                break;
            }
            _ => {} // unsupported/unknown service type: ignore (§6.2/§6.3)
        }
    }
    // Signal any subscriber that the tunnel is gone (KNOWN_LIMITATIONS.md
    // #28). Every exit path above — explicit shutdown, a dead socket, or a
    // server-initiated DISCONNECT_REQUEST — falls through to here, so one
    // send covers all of them. Best-effort: no subscribers is not an error.
    let _ = state.tx.send(TunnelEvent::Closed);
}

async fn send_ack(state: &TunnelState, sequence_counter: u8, status: u8) {
    let body = tunnelling::encode_tunnelling_ack(state.channel_id, sequence_counter, status);
    let datagram = frame::encode_frame(tunnelling::TUNNELLING_ACK, &body);
    // Best-effort: a lost ACK makes the peer retry the TUNNELLING_REQUEST
    // (Tunnelling v01.07.01 AS §2.6.1), which this loop will then see again.
    let _ = state.socket.send(&datagram).await;
}

/// Core v01.06.02 AS §5.4: a `CONNECTIONSTATE_REQUEST` every 60 seconds;
/// on no reply (or an error status) within 10 seconds, repeat up to three
/// times, then disconnect.
async fn heartbeat_loop(state: Arc<TunnelState>) {
    let mut interval = tokio::time::interval(Duration::from_secs(60));
    interval.tick().await; // the first tick fires immediately; consume it
                           // so the first heartbeat happens 60s in, not at connect time
    loop {
        tokio::select! {
            _ = state.shutdown.notified() => return,
            _ = interval.tick() => {}
        }
        if !send_heartbeat_with_retries(&state).await {
            state.shutdown.notify_waiters();
            let _ = state
                .socket
                .send(&frame::encode_frame(
                    services::DISCONNECT_REQUEST,
                    &services::encode_disconnect_request(
                        state.channel_id,
                        match local_hpai(&state.socket) {
                            Ok(hpai) => hpai,
                            Err(_) => return,
                        },
                    ),
                ))
                .await;
            return;
        }
    }
}

async fn send_heartbeat_with_retries(state: &Arc<TunnelState>) -> bool {
    for _attempt in 0..4 {
        let Ok(control_hpai) = local_hpai(&state.socket) else {
            return false;
        };
        let body = services::encode_connectionstate_request(state.channel_id, control_hpai);
        let datagram = frame::encode_frame(services::CONNECTIONSTATE_REQUEST, &body);
        *state.heartbeat_reply.lock().await = None;
        if state.socket.send(&datagram).await.is_err() {
            return false;
        }
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        let status = wait_for_reply(
            &state.heartbeat_notify,
            &state.heartbeat_reply,
            deadline,
            |_| true,
        )
        .await;
        if status == Some(services::E_NO_ERROR) {
            return true;
        }
    }
    false
}

/// Waits for `notify` to fire and `reply` to hold a value matching
/// `is_match`, within `deadline` — retrying stale wakeups instead of
/// treating them as a timeout (KNOWN_LIMITATIONS.md #27).
///
/// `tokio::sync::Notify` keeps at most one permit: if a wakeup meant for a
/// previous attempt (e.g. a reply that arrived just after that attempt gave
/// up) fires here before this attempt's real reply does, a naive single
/// `timeout(..., notified())` would wake immediately, find nothing useful,
/// and burn this attempt without ever really waiting out its budget. This
/// loops on the same deadline instead: a stale or non-matching wakeup is
/// discarded and waited past, so only a genuine timeout or a matching reply
/// ends it.
async fn wait_for_reply<T: Clone>(
    notify: &Notify,
    reply: &Mutex<Option<T>>,
    deadline: tokio::time::Instant,
    is_match: impl Fn(&T) -> bool,
) -> Option<T> {
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return None;
        }
        if tokio::time::timeout(remaining, notify.notified())
            .await
            .is_err()
        {
            return None; // genuine timeout
        }
        let mut guard = reply.lock().await;
        match guard.take() {
            Some(value) if is_match(&value) => return Some(value),
            _ => continue, // stale wakeup or a reply for someone else: keep waiting
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// KNOWN_LIMITATIONS.md #27: a stale `Notify` wakeup — one meant for an
    /// earlier attempt, firing after this attempt already reset the shared
    /// reply slot — must not be mistaken for a timeout. This fires the
    /// notify twice: once immediately with a non-matching reply (the stale
    /// wakeup), once after a short delay with the matching one. The old
    /// single-`timeout(notified())` code would return `None` on the first
    /// (non-matching) wakeup; `wait_for_reply` must instead keep waiting and
    /// return the real reply.
    #[tokio::test]
    async fn wait_for_reply_survives_a_stale_non_matching_wakeup() {
        let notify = Notify::new();
        let reply: Mutex<Option<(u8, u8)>> = Mutex::new(None);

        tokio::join!(
            async {
                // The "stale" wakeup: a reply for some other request,
                // delivered right away.
                *reply.lock().await = Some((0xFF, 0));
                notify.notify_one();
                tokio::time::sleep(Duration::from_millis(20)).await;
                // The real reply, for sequence 7, arrives after a delay —
                // well inside the deadline, but after the stale wakeup.
                *reply.lock().await = Some((7, tunnelling::E_NO_ERROR));
                notify.notify_one();
            },
            async {
                let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
                let result = wait_for_reply(&notify, &reply, deadline, |&(seq, _)| seq == 7).await;
                assert_eq!(
                    result,
                    Some((7, tunnelling::E_NO_ERROR)),
                    "must keep waiting past a stale/non-matching wakeup instead of timing out"
                );
            }
        );
    }

    /// A genuine timeout — no reply ever arrives — must still return `None`
    /// rather than loop forever.
    #[tokio::test]
    async fn wait_for_reply_times_out_when_nothing_ever_matches() {
        let notify = Notify::new();
        let reply: Mutex<Option<(u8, u8)>> = Mutex::new(None);
        let deadline = tokio::time::Instant::now() + Duration::from_millis(50);
        let result = wait_for_reply(&notify, &reply, deadline, |&(seq, _)| seq == 7).await;
        assert_eq!(result, None);
    }

    /// Routing v01.05.02 AS §2.3.5: a second `ROUTING_BUSY` arriving before
    /// the first one's `tw` has elapsed must extend the deadline to the
    /// *later* of the two, never shorten it.
    #[test]
    fn merge_busy_deadline_keeps_the_later_of_the_two() {
        let now = tokio::time::Instant::now();
        let far = now + Duration::from_millis(100);
        let near = now + Duration::from_millis(20);

        // A shorter new deadline than the one already in effect: keep the
        // existing (later) one.
        assert_eq!(merge_busy_deadline(Some(far), near), far);
        // A longer new deadline: adopt it.
        assert_eq!(merge_busy_deadline(Some(near), far), far);
        // No previous ROUTING_BUSY in effect: the new deadline wins outright.
        assert_eq!(merge_busy_deadline(None, far), far);
    }

    /// Confirms the fix for the discovery HPAI bug: a discovery socket
    /// bound to the wildcard address and left unconnected must still
    /// yield a real (non-`0.0.0.0`) local IP once paired via
    /// `local_discovery_hpai`, with the discovery socket's own port
    /// (not the throwaway probe's). Skipped rather than failed if this
    /// sandbox has no route to the discovery multicast group at all —
    /// that's an environment limitation, not a regression.
    #[tokio::test]
    async fn local_discovery_hpai_resolves_a_real_ip_and_keeps_the_real_port() {
        let socket = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("bind discovery socket");
        let discovery_port = local_hpai(&socket)
            .expect("read discovery socket's own port")
            .port;

        let probe = UdpSocket::bind("0.0.0.0:0")
            .await
            .expect("bind probe socket");
        if probe.connect(DISCOVERY_MULTICAST).await.is_err() {
            eprintln!(
                "skipping local_discovery_hpai_resolves_a_real_ip_and_keeps_the_real_port: \
                 no route to {DISCOVERY_MULTICAST} in this sandbox"
            );
            return;
        }
        drop(probe);

        let hpai = local_discovery_hpai(&socket)
            .await
            .expect("local_discovery_hpai should succeed when the route exists");
        assert_ne!(
            hpai.addr,
            Ipv4Addr::new(0, 0, 0, 0),
            "resolved HPAI must not be the invalid wildcard address (Core v01.06.02 AS §8.6.2.2)"
        );
        assert_eq!(
            hpai.port, discovery_port,
            "resolved HPAI must carry the real discovery socket's port, not the probe's"
        );
    }

    /// Round-trip proof that `RoutingClient` actually multicasts and
    /// receives, entirely on loopback — unlike tunnelling/discovery,
    /// routing needs no real gateway to test, since it's plain UDP
    /// multicast rather than a protocol exchange with a specific peer.
    /// Skipped (not failed) if this sandbox has no multicast route on
    /// loopback at all, same policy as the discovery test above it.
    #[tokio::test]
    async fn routing_client_sends_and_receives_a_group_value_write() {
        use crate::cemi::{ApplicationService, Destination, GroupValue};
        use knx_core::{GroupAddress, GroupAddressStyle, IndividualAddress};

        let sender_address = IndividualAddress::new(1, 1, 1).unwrap();
        let receiver_address = IndividualAddress::new(1, 1, 2).unwrap();

        let sender = match KnxNetIpClient::new().connect_routing(sender_address).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "skipping routing_client_sends_and_receives_a_group_value_write: \
                     could not join the routing multicast group in this sandbox: {e}"
                );
                return;
            }
        };
        let receiver = KnxNetIpClient::new()
            .connect_routing(receiver_address)
            .await
            .expect("second RoutingClient should join the same group fine (SO_REUSEADDR)");
        let mut telegrams = receiver.subscribe();

        let group_address = GroupAddress::parse("1/2/3", GroupAddressStyle::ThreeLevel).unwrap();
        sender
            .send(
                Destination::Group(group_address),
                ApplicationService::GroupValueWrite(GroupValue::Short(1)),
            )
            .await
            .expect("send over loopback multicast should succeed");

        // Some sandboxes accept `IP_ADD_MEMBERSHIP`/`sendto()` without error
        // yet never actually deliver the datagram locally (verified here by
        // hand with plain Python sockets, forcing `IP_MULTICAST_IF` to
        // 127.0.0.1: join and send both succeed, nothing arrives) — a
        // stricter, no-route-at-all failure than the ones `connect_routing`
        // above can detect. Treat that the same way: skip, don't fail.
        let received = match tokio::time::timeout(Duration::from_secs(5), telegrams.recv()).await {
            Ok(Ok(telegram)) => telegram,
            Ok(Err(_)) => panic!("broadcast channel closed unexpectedly"),
            Err(_) => {
                eprintln!(
                    "skipping routing_client_sends_and_receives_a_group_value_write: \
                     joined the multicast group but no datagram arrived within 5s — \
                     this sandbox appears to accept the join/send but not deliver \
                     multicast locally"
                );
                return;
            }
        };
        assert_eq!(received.source, sender_address);
        assert_eq!(received.destination, Destination::Group(group_address));
        assert_eq!(
            received.service,
            ApplicationService::GroupValueWrite(GroupValue::Short(1))
        );
    }

    /// KNOWN_LIMITATIONS.md #32: `send()` must wait out a `ROUTING_BUSY`
    /// deadline already in effect before transmitting. Sets `busy_until`
    /// directly (this test lives inside `client` itself, so `RoutingState`'s
    /// private field is reachable) rather than round-tripping a real
    /// `ROUTING_BUSY` datagram through the receive loop — that path is
    /// exercised by `merge_busy_deadline_keeps_the_later_of_the_two` above,
    /// so this test's job is only to confirm `send()` actually honours the
    /// deadline once set. Only needs `connect_routing` to succeed (to reach
    /// a real `send()`); delivery is irrelevant, so unlike the round-trip
    /// test above this one only skips if joining the multicast group itself
    /// fails.
    #[tokio::test]
    async fn routing_client_send_waits_out_a_routing_busy_deadline() {
        use crate::cemi::{ApplicationService, Destination, GroupValue};
        use knx_core::{GroupAddress, GroupAddressStyle, IndividualAddress};

        let own_address = IndividualAddress::new(1, 1, 3).unwrap();
        let client = match KnxNetIpClient::new().connect_routing(own_address).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "skipping routing_client_send_waits_out_a_routing_busy_deadline: \
                     could not join the routing multicast group in this sandbox: {e}"
                );
                return;
            }
        };
        let wait = Duration::from_millis(150);
        *client.state.busy_until.lock().await = Some(tokio::time::Instant::now() + wait);

        let started = tokio::time::Instant::now();
        let group_address = GroupAddress::parse("1/2/3", GroupAddressStyle::ThreeLevel).unwrap();
        let _ = client
            .send(
                Destination::Group(group_address),
                ApplicationService::GroupValueWrite(GroupValue::Short(0)),
            )
            .await;
        assert!(
            started.elapsed() >= wait,
            "send() must wait out the ROUTING_BUSY deadline before transmitting"
        );
    }
}
