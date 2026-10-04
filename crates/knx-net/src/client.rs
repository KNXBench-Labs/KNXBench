//! The `BusConnection` trait and its tunnelling implementation
//! (ARCHITECTURE.md §8). `discover` (multicast `SEARCH_REQUEST`/
//! `SEARCH_RESPONSE`) is implemented as of Session 6 Cycle 3.
//! `TunnelClient::send` is implemented as of Session 6 Cycle 2
//! (KNOWN_LIMITATIONS.md §26). `connect_routing`/`RoutingClient` are
//! implemented as of Session 6 Cycle 4. Session 6 Cycle 5 hardens
//! connection management: `wait_for_reply` fixes a heartbeat/ack retry
//! race (KNOWN_LIMITATIONS.md #27), `TunnelEvent::Closed` signals
//! subscribers when the tunnel dies (#28), and `RoutingClient::send`
//! honors `ROUTING_BUSY` (#32). `connect_routing_to_group` (E6,
//! KNOWN_LIMITATIONS.md #31) lets a caller join a non-default routing
//! multicast group instead of `ROUTING_MULTICAST`.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicU64, Ordering};
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

/// Local bound for the final control-channel response. Core §5.5 requires
/// `DISCONNECT_RESPONSE` but defines no separate timeout; ten seconds matches
/// the existing CONNECT/CONNECTIONSTATE control-response budget.
const DISCONNECT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

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
    /// `connect_routing_to_group` was asked to join an address outside
    /// 224.0.0.0/4 (KNOWN_LIMITATIONS.md §31) — most likely a unicast or
    /// broadcast address passed where a multicast group belongs. Caught
    /// before any socket call, so this never carries an OS error code.
    NotMulticast(Ipv4Addr),
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
            BusError::NotMulticast(addr) => write!(
                f,
                "{addr} is not an IPv4 multicast address (224.0.0.0/4) — \
                 a routing multicast group must be, per Core v01.06.02 AS §8.5.2.2"
            ),
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
    /// Complete decoded Device Info DIB. `None` is explicitly unavailable
    /// metadata from a non-wire adapter; a decoded SEARCH_RESPONSE has Some.
    pub device_info: Option<dib::DeviceInfo>,
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
    /// Same as `connect_routing`, but joins `group` instead of the standard
    /// default (KNOWN_LIMITATIONS.md §31) — for an installation assigned a
    /// custom Routing Multicast Address (Core v01.06.02 AS §8.5.2.2). The
    /// port is not an independent choice: Routing v01.05.02 AS §2.3.1 fixes
    /// it at 3671 for every installation, so only the address varies here.
    /// Never verified against a real installation using a non-default
    /// group.
    async fn connect_routing_to_group(
        &self,
        own_address: IndividualAddress,
        group: Ipv4Addr,
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

/// Shared discovery exchange. Production supplies its existing multicast
/// destination, route-resolved HPAI and timeout; offline tests supply only
/// loopback sockets. No public endpoint override or protocol change.
async fn discover_on_socket(
    socket: &UdpSocket,
    discovery_endpoint: Hpai,
    destination: SocketAddrV4,
    timeout: Duration,
) -> Result<Vec<DiscoveredGateway>, BusError> {
    let request_body = discovery::encode_search_request(discovery_endpoint);
    let datagram = frame::encode_frame(services::SEARCH_REQUEST, &request_body);
    socket
        .send_to(&datagram, destination)
        .await
        .map_err(BusError::Io)?;

    let mut gateways: Vec<DiscoveredGateway> = Vec::new();
    let deadline = tokio::time::Instant::now() + timeout;
    let mut buf = [0u8; 1024];
    loop {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            break;
        }
        let Ok(Ok((n, _src))) = tokio::time::timeout(remaining, socket.recv_from(&mut buf)).await
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
            friendly_name: response.device_info.friendly_name.clone(),
            supports_tunnelling,
            device_info: Some(response.device_info),
        });
    }
    Ok(gateways)
}

impl BusConnection for KnxNetIpClient {
    async fn discover(&self) -> Result<Vec<DiscoveredGateway>, BusError> {
        let socket = UdpSocket::bind("0.0.0.0:0").await.map_err(BusError::Io)?;
        let discovery_endpoint = local_discovery_hpai(&socket).await?;
        discover_on_socket(
            &socket,
            discovery_endpoint,
            DISCOVERY_MULTICAST,
            Duration::from_secs(SEARCH_TIMEOUT_SECS),
        )
        .await
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

    async fn connect_routing_to_group(
        &self,
        own_address: IndividualAddress,
        group: Ipv4Addr,
    ) -> Result<RoutingClient, BusError> {
        RoutingClient::connect_to_group(own_address, group).await
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
    disconnect_reply: Mutex<Option<(u8, u8)>>,
    disconnect_notify: Notify,
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
    /// Counts every `TUNNELLING_REQUEST` whose cEMI payload failed to
    /// decode. Review finding (T17 fix round 1, #8): printing one
    /// `eprintln!` per bad frame is unbounded — a single chronically
    /// malformed device would flood stderr at bus rate for a whole scan's
    /// duration. Only the first failure is printed; the rest are counted
    /// here instead, the way `bus.rs`'s `record_lagged` accounts for a
    /// loss rather than narrating each one.
    decode_failures: AtomicU64,
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
            disconnect_reply: Mutex::new(None),
            disconnect_notify: Notify::new(),
            shutdown: Notify::new(),
            send_seq: Mutex::new(0),
            ack_reply: Mutex::new(None),
            ack_notify: Notify::new(),
            decode_failures: AtomicU64::new(0),
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

    /// How many `TUNNELLING_REQUEST`s on this connection carried a cEMI
    /// payload that failed to decode. Only the first one is ever printed
    /// to stderr (see `receive_loop`); this is where the rest are seen.
    pub fn decode_failure_count(&self) -> u64 {
        self.state.decode_failures.load(Ordering::Relaxed)
    }

    /// Sends an `L_Data.req` carrying `Tpci::UnnumberedData` (Tunnelling
    /// v01.07.01 AS §2.6) — what every group service, and every
    /// unconnected point-to-point service, uses. Delegates to
    /// [`Self::send_frame`]; kept as its own method because it is the
    /// overwhelming majority of callers and they should not have to name
    /// a TPCI they never vary.
    pub async fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        self.send_frame(destination, cemi::Tpci::UnnumberedData, service)
            .await
    }

    /// Sends an `L_Data.req` (Tunnelling v01.07.01 AS §2.6): source/kind
    /// are the client's concern, not the caller's — the gateway assigns
    /// the actual source address and message code, so only the
    /// destination, transport (TPCI) and application service are exposed
    /// here. The general path behind [`Self::send`]; a line scan
    /// (T17) needs `T_Connect`/numbered `T_Data_Connected`/`T_Disconnect`,
    /// which a fixed `Tpci::UnnumberedData` cannot express.
    ///
    /// Per §2.6.1/§2.6.2: waits up to `TUNNELLING_REQUEST_TIMEOUT` (1s)
    /// for a matching `TUNNELLING_ACK`; on timeout or an error status,
    /// repeats the same `TUNNELLING_REQUEST` once with the same sequence
    /// counter. If that repeat also fails, the connection is terminated
    /// (a `DISCONNECT_REQUEST` is sent, best-effort, and the background
    /// tasks are told to stop) and `BusError::Timeout` is returned — the
    /// same outcome `heartbeat_loop` already reaches on repeated failure.
    pub async fn send_frame(
        &self,
        destination: Destination,
        transport: cemi::Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let frame = LDataFrame {
            kind: cemi::LDataMessageKind::Request,
            source: IndividualAddress::from_raw(0),
            destination,
            transport,
            service,
            control: None,
        };
        let cemi_bytes =
            cemi::encode_l_data(&frame).map_err(|e| BusError::Protocol(e.to_string()))?;

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

    /// Graceful disconnect (Core v01.06.02 AS §5.5): sends
    /// `DISCONNECT_REQUEST`, waits for the matching `DISCONNECT_RESPONSE`
    /// which marks final channel termination, then stops background tasks.
    ///
    /// `shutdown.notify_waiters()` always fires, even if sending the
    /// datagram fails — `disconnect` consumes `self`, so a caller who got an
    /// `Err` here has no way to retry; the background tasks must still be
    /// told to stop rather than leaking forever.
    pub async fn disconnect(self) -> Result<(), BusError> {
        *self.state.disconnect_reply.lock().await = None;
        if let Err(error) = self.try_send_disconnect_request().await {
            self.state.shutdown.notify_waiters();
            return Err(error);
        }

        let deadline = tokio::time::Instant::now() + DISCONNECT_RESPONSE_TIMEOUT;
        let response = wait_for_reply(
            &self.state.disconnect_notify,
            &self.state.disconnect_reply,
            deadline,
            |&(channel_id, _)| channel_id == self.state.channel_id,
        )
        .await;
        self.state.shutdown.notify_waiters();
        match response {
            Some((_, services::E_NO_ERROR)) => Ok(()),
            Some((_, status)) => Err(BusError::Protocol(format!(
                "DISCONNECT_RESPONSE returned status {status:#04x}"
            ))),
            None => Err(BusError::Timeout),
        }
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
    /// See `TunnelState::decode_failures` — same bound-the-eprintln fix,
    /// same reasoning, applied to the routing receive loop.
    decode_failures: AtomicU64,
}

/// A KNXnet/IP routing endpoint — joined to the routing multicast group
/// (the standard one by default, or `group` from `connect_to_group`),
/// sending and receiving `ROUTING_INDICATION` frames unconfirmed (Routing
/// v01.05.02 AS §5.1). Unlike `TunnelClient`, there is no connection to a
/// specific peer: `own_address` is this client's own claimed source
/// address for outgoing frames, not something a gateway assigns, since
/// routing has no `CONNECT_REQUEST`/`CRD` handshake to assign one through.
pub struct RoutingClient {
    state: Arc<RoutingState>,
    own_address: IndividualAddress,
    /// The joined group, port included — `ROUTING_MULTICAST` by default,
    /// a caller's choice after `connect_to_group`. `send()` targets this,
    /// never the bare constant, so the two connect paths cannot drift.
    group: SocketAddrV4,
}

/// How a `RoutingClient`'s socket is attached to the network: which
/// interface its datagrams leave by, whether the host gets a copy, and how
/// many hops they may travel. Every non-test caller gets `PRODUCTION`,
/// which is the behaviour this type was extracted from, unchanged; the
/// in-process tests below get `LOOPBACK_ONLY` so that a plain
/// `cargo test` cannot put a `ROUTING_INDICATION` on a real installation's
/// LAN (KNOWN_LIMITATIONS.md §33).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RoutingSocketOptions {
    /// `IP_MULTICAST_IF`, and the interface `IP_ADD_MEMBERSHIP` joins on.
    /// `Ipv4Addr::UNSPECIFIED` leaves both to the kernel's routing table —
    /// the only correct default for a real installation, whose gateway is
    /// reached over whichever interface the route points at.
    interface: Ipv4Addr,
    /// `IP_MULTICAST_LOOP`: whether this host also receives what this
    /// socket sends.
    loop_back: bool,
    /// `IP_MULTICAST_TTL`. `None` leaves the kernel default of 1. `Some(0)`
    /// stops an IP *router* from forwarding the datagram off this subnet
    /// and nothing more: measured on Linux 7.2.5 in an isolated namespace,
    /// a TTL-0 multicast datagram is still put on the link as a real
    /// Ethernet frame (`dst_mac=01:00:5e:00:17:0c ip_ttl=0`), and a switch
    /// forwards L2 by MAC without ever reading the IP header. This is not
    /// a containment mechanism on a switched LAN.
    ttl: Option<u32>,
}

impl RoutingSocketOptions {
    /// The real thing: kernel-chosen interface, no loopback copy (our own
    /// sends must not surface in our own `subscribe()` — design spec's
    /// Architecture section), default TTL.
    const PRODUCTION: Self = Self {
        interface: Ipv4Addr::UNSPECIFIED,
        loop_back: false,
        ttl: None,
    };

    /// Tests only. **There is exactly one lock, and it is
    /// `IP_MULTICAST_IF`**: it names `lo`, so the kernel never consults the
    /// routing table (which on a developer's machine resolves
    /// `224.0.23.12` to the physical LAN interface). Two tests assert that
    /// lock from both ends — `loopback_only_options_actually_reach_the_socket`
    /// reads the option back off the live socket, and
    /// `the_loopback_join_lands_on_lo_and_nowhere_else` reads the
    /// membership the kernel actually recorded out of `/proc/net/igmp`.
    ///
    /// TTL 0 is *not* a second lock, whatever this comment and
    /// KNOWN_LIMITATIONS.md §33 said until 2026-09-20. It stops IP-level
    /// forwarding only; the frame still goes on the link, and a switch
    /// forwards it by MAC to every other port in the group, a KNXnet/IP
    /// router included (measured — see the `ttl` field above). Keep it as
    /// defence against an IP-level mistake, and expect nothing else of it.
    ///
    /// `loop_back` is `true` — the opposite of production, and the one
    /// deliberate deviation — because it is the portable way to ask for
    /// local delivery. On `lo` this kernel delivers either way (measured:
    /// a probe pair with `IP_MULTICAST_LOOP` read back as 0 still reached
    /// each other), so nothing here actually leans on it.
    ///
    /// The cost of asking for local delivery is that every socket joined
    /// to the group sees every other one's telegrams, whichever test sent
    /// them — hence `recv_from_source`, which each round-trip test uses
    /// instead of trusting the first frame to arrive to be its own.
    #[cfg(test)]
    const LOOPBACK_ONLY: Self = Self {
        interface: Ipv4Addr::LOCALHOST,
        loop_back: true,
        ttl: Some(0),
    };
}

impl RoutingClient {
    async fn connect(own_address: IndividualAddress) -> Result<Self, BusError> {
        Self::connect_with(own_address, RoutingSocketOptions::PRODUCTION).await
    }

    /// `connect()` with the socket options spelled out, so a test can join
    /// the *standard* group — the thing `connect()` is there to get right —
    /// without the datagrams leaving the machine.
    async fn connect_with(
        own_address: IndividualAddress,
        options: RoutingSocketOptions,
    ) -> Result<Self, BusError> {
        Self::connect_to_group_with(own_address, *ROUTING_MULTICAST.ip(), options).await
    }

    /// `connect()`'s actual implementation, generalized over the group —
    /// `connect()` is just this with `ROUTING_MULTICAST`'s own address, so
    /// the default and the override can never join or send to different
    /// places by accident. `group`'s port is always `ROUTING_MULTICAST`'s
    /// (Routing v01.05.02 AS §2.3.1 fixes it at 3671 for every
    /// installation); only the address is a caller's choice.
    async fn connect_to_group(
        own_address: IndividualAddress,
        group: Ipv4Addr,
    ) -> Result<Self, BusError> {
        Self::connect_to_group_with(own_address, group, RoutingSocketOptions::PRODUCTION).await
    }

    /// `connect_to_group()` with the socket options spelled out — see
    /// `RoutingSocketOptions`. Private, and the only caller that passes
    /// anything but `PRODUCTION` is this module's own test section.
    async fn connect_to_group_with(
        own_address: IndividualAddress,
        group: Ipv4Addr,
        options: RoutingSocketOptions,
    ) -> Result<Self, BusError> {
        if !group.is_multicast() {
            return Err(BusError::NotMulticast(group));
        }
        let group = SocketAddrV4::new(group, ROUTING_MULTICAST.port());

        use socket2::{Domain, Socket, Type};

        let socket2_socket = Socket::new(Domain::IPV4, Type::DGRAM, None).map_err(BusError::Io)?;
        socket2_socket
            .set_reuse_address(true)
            .map_err(BusError::Io)?;
        socket2_socket
            .bind(&std::net::SocketAddr::from((Ipv4Addr::UNSPECIFIED, group.port())).into())
            .map_err(BusError::Io)?;
        // A socket bound to the wildcard address on Linux otherwise receives
        // every group *any* socket on the host has joined (ip(7),
        // `IP_MULTICAST_ALL`, default on). Two routing clients for separate
        // installations on one host — default group next to a custom one —
        // would then hear each other's telegrams, defeating the separation a
        // custom group exists for (Routing v01.05.02 AS §2.3.2). Measured by
        // `a_custom_group_telegram_reaches_its_group_and_not_the_default_one`
        // (AR14). Other platforms' delivery policy is unverified.
        #[cfg(target_os = "linux")]
        socket2_socket
            .set_multicast_all_v4(false)
            .map_err(BusError::Io)?;
        socket2_socket.set_nonblocking(true).map_err(BusError::Io)?;
        // Both of these are skipped for `PRODUCTION`, which asks for the
        // kernel's own defaults (`IP_MULTICAST_IF` = `INADDR_ANY`, TTL 1):
        // not making the syscall at all is the clearest possible proof
        // that production behaviour is untouched.
        if !options.interface.is_unspecified() {
            socket2_socket
                .set_multicast_if_v4(&options.interface)
                .map_err(BusError::Io)?;
        }
        if let Some(ttl) = options.ttl {
            socket2_socket
                .set_multicast_ttl_v4(ttl)
                .map_err(BusError::Io)?;
        }
        let std_socket: std::net::UdpSocket = socket2_socket.into();
        let socket = UdpSocket::from_std(std_socket).map_err(BusError::Io)?;

        // `options.interface` here, not `UNSPECIFIED`: the membership — and
        // the IGMP membership report that announces it — must land on the
        // same interface `IP_MULTICAST_IF` pins. `socket2` exposes no getter
        // for what a join used, so `the_loopback_join_lands_on_lo_and_nowhere_else`
        // asks `/proc/net/igmp` instead; without it this argument could be
        // reverted with the whole suite staying green.
        socket
            .join_multicast_v4(*group.ip(), options.interface)
            .map_err(BusError::Io)?;
        // `false` in production: without it, our own sends would loop back
        // through this same socket and appear in `subscribe()` as if
        // another device sent them (design spec's Architecture section).
        // `options.loop_back` rather than a literal, and
        // `a_loop_back_of_false_reaches_the_socket_as_false` reads the result
        // back off a real socket — hardcoding `true` here would otherwise
        // ship exactly that bug with every test still passing.
        socket
            .set_multicast_loop_v4(options.loop_back)
            .map_err(BusError::Io)?;

        let (tx, _rx) = broadcast::channel(64);
        let state = Arc::new(RoutingState {
            socket,
            tx,
            shutdown: Notify::new(),
            busy_until: Mutex::new(None),
            decode_failures: AtomicU64::new(0),
        });
        tokio::spawn(routing_receive_loop(state.clone()));

        Ok(RoutingClient {
            state,
            own_address,
            group,
        })
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LDataFrame> {
        self.state.tx.subscribe()
    }

    /// See `TunnelClient::decode_failure_count`.
    pub fn decode_failure_count(&self) -> u64 {
        self.state.decode_failures.load(Ordering::Relaxed)
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
            transport: cemi::Tpci::UnnumberedData,
            service,
            control: None,
        };
        let cemi_bytes =
            cemi::encode_l_data(&frame).map_err(|e| BusError::Protocol(e.to_string()))?;
        let datagram = frame::encode_frame(services::ROUTING_INDICATION, &cemi_bytes);
        self.state
            .socket
            .send_to(&datagram, self.group)
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
            services::ROUTING_INDICATION => match cemi::decode_l_data(body) {
                Ok(telegram) => {
                    let _ = state.tx.send(telegram);
                }
                Err(e) => {
                    if state.decode_failures.fetch_add(1, Ordering::Relaxed) == 0 {
                        eprintln!(
                            "ROUTING_INDICATION: cEMI frame failed to decode: {e} \
                             (further failures on this connection are counted, not printed; \
                             see decode_failure_count)"
                        );
                    }
                }
            },
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
                    match cemi::decode_l_data(req.cemi) {
                        Ok(telegram) => {
                            let _ = state.tx.send(TunnelEvent::Telegram(telegram));
                        }
                        Err(e) => {
                            if state.decode_failures.fetch_add(1, Ordering::Relaxed) == 0 {
                                eprintln!(
                                    "TUNNELLING_REQUEST: cEMI frame failed to decode: {e} \
                                     (further failures on this connection are counted, not \
                                     printed; see decode_failure_count)"
                                );
                            }
                        }
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
            services::DISCONNECT_RESPONSE => {
                if let Ok(resp) = services::decode_disconnect_response(body) {
                    if resp.channel_id == state.channel_id {
                        *state.disconnect_reply.lock().await = Some((resp.channel_id, resp.status));
                        state.disconnect_notify.notify_one();
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

    // The same synthetic DIB layout as discovery.rs's codec fixture; not a
    // private gateway capture. All transport endpoints stay on 127.0.0.1.
    fn loopback_search_response(endpoint: SocketAddrV4, tunnelling: bool) -> Vec<u8> {
        let mut body = Hpai {
            addr: *endpoint.ip(),
            port: endpoint.port(),
        }
        .encode()
        .to_vec();
        body.extend_from_slice(&[0x36, dib::DEVICE_INFO, 0x81, 0x80, 0x11, 0x01]);
        body.extend_from_slice(&[0x12, 0x34]); // project-installation id
        body.extend_from_slice(&[1, 2, 3, 4, 5, 6]); // serial
        body.extend_from_slice(&[224, 0, 23, 12]);
        body.extend_from_slice(&[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff]); // MAC
        let mut name = b"Offline gateway".to_vec();
        name.resize(30, 0);
        body.extend_from_slice(&name);
        if tunnelling {
            body.extend_from_slice(&[0x04, dib::SUPP_SVC_FAMILIES, 0x04, 0x01]);
        }
        frame::encode_frame(services::SEARCH_RESPONSE, &body)
    }

    #[tokio::test]
    async fn discovery_loopback_roundtrip_uses_advertised_hpai_and_filters_datagrams() {
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let peer = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let other_peer = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let endpoint = local_hpai(&client).unwrap();
        let peer_addr = SocketAddrV4::new(Ipv4Addr::LOCALHOST, peer.local_addr().unwrap().port());
        let other_addr =
            SocketAddrV4::new(Ipv4Addr::LOCALHOST, other_peer.local_addr().unwrap().port());
        let valid = loopback_search_response(peer_addr, true);
        let other = loopback_search_response(other_addr, false);

        let roundtrip = async {
            tokio::join!(
                discover_on_socket(&client, endpoint, peer_addr, Duration::from_millis(200),),
                async {
                    let mut buf = [0; 1024];
                    let (n, source) = peer.recv_from(&mut buf).await.unwrap();
                    assert_eq!(source, client.local_addr().unwrap());
                    let (header, body) = frame::decode_frame(&buf[..n]).unwrap();
                    assert_eq!(header.service_type, services::SEARCH_REQUEST);
                    let [hi, lo] = endpoint.port.to_be_bytes();
                    assert_eq!(body, &[8, 1, 127, 0, 0, 1, hi, lo]);
                    let reply_to = SocketAddrV4::new(endpoint.addr, endpoint.port);
                    // Wrong header, wrong service and invalid SEARCH_RESPONSE body
                    // must not prevent a later valid response being received.
                    peer.send_to(&[0], reply_to).await.unwrap();
                    peer.send_to(
                        &frame::encode_frame(
                            services::CONNECT_RESPONSE,
                            &loopback_search_response(reply_to, true)[6..],
                        ),
                        reply_to,
                    )
                    .await
                    .unwrap();
                    peer.send_to(
                        &frame::encode_frame(services::SEARCH_RESPONSE, &[]),
                        reply_to,
                    )
                    .await
                    .unwrap();
                    peer.send_to(&valid, reply_to).await.unwrap();
                    // A different UDP source can advertise the same endpoint;
                    // discovery must stay unconnected and de-duplicate by HPAI.
                    other_peer.send_to(&valid, reply_to).await.unwrap();
                    other_peer.send_to(&other, reply_to).await.unwrap();
                }
            )
        };
        let (gateways, ()) = tokio::time::timeout(Duration::from_secs(2), roundtrip)
            .await
            .expect("bounded offline discovery roundtrip");
        let mut gateways = gateways.unwrap();
        gateways.sort_by_key(|gateway| gateway.control_endpoint.port());
        let info = dib::DeviceInfo {
            medium: 0x81,
            status: 0x80,
            individual_address: IndividualAddress::new(1, 1, 1).unwrap(),
            project_installation_id: 0x1234,
            serial_number: [1, 2, 3, 4, 5, 6],
            routing_multicast: Ipv4Addr::new(224, 0, 23, 12),
            mac_address: [0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff],
            friendly_name: "Offline gateway".into(),
        };
        let mut expected = vec![
            DiscoveredGateway {
                control_endpoint: peer_addr,
                individual_address: IndividualAddress::new(1, 1, 1).unwrap(),
                friendly_name: "Offline gateway".into(),
                supports_tunnelling: true,
                device_info: Some(info.clone()),
            },
            DiscoveredGateway {
                control_endpoint: other_addr,
                individual_address: IndividualAddress::new(1, 1, 1).unwrap(),
                friendly_name: "Offline gateway".into(),
                supports_tunnelling: false,
                device_info: Some(info),
            },
        ];
        expected.sort_by_key(|gateway| gateway.control_endpoint.port());
        assert_eq!(gateways, expected);
        let mut buf = [0; 1024];
        assert_eq!(
            peer.try_recv_from(&mut buf).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
        assert_eq!(
            other_peer.try_recv_from(&mut buf).unwrap_err().kind(),
            std::io::ErrorKind::WouldBlock
        );
    }

    #[tokio::test]
    async fn discovery_loopback_no_reply_returns_empty_at_the_deadline() {
        let client = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let peer = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let gateways = tokio::time::timeout(
            Duration::from_secs(2),
            discover_on_socket(
                &client,
                local_hpai(&client).unwrap(),
                SocketAddrV4::new(Ipv4Addr::LOCALHOST, peer.local_addr().unwrap().port()),
                Duration::from_millis(20),
            ),
        )
        .await
        .expect("no-reply discovery must be bounded")
        .unwrap();
        assert!(gateways.is_empty());
        let mut buf = [0; 1024];
        let (n, _) = peer
            .try_recv_from(&mut buf)
            .expect("a real search was sent");
        assert_eq!(
            frame::decode_frame(&buf[..n]).unwrap().0.service_type,
            services::SEARCH_REQUEST
        );
    }

    use std::collections::HashMap;

    /// Core v01.06.02 AS §5.5: `DISCONNECT_RESPONSE` is the final
    /// termination of the communication channel. A graceful client must not
    /// report completion while the server is still holding that response.
    #[tokio::test]
    async fn graceful_disconnect_waits_for_the_servers_final_response() {
        let server = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind loopback KNXnet/IP server");
        let server_addr = match server.local_addr().expect("server address") {
            SocketAddr::V4(addr) => addr,
            SocketAddr::V6(_) => unreachable!("bound an IPv4 socket"),
        };
        let (request_seen_tx, request_seen_rx) = tokio::sync::oneshot::channel();
        let (release_response_tx, release_response_rx) = tokio::sync::oneshot::channel();

        let peer = tokio::spawn(async move {
            let mut buf = [0u8; 128];
            let (n, client_addr) = server
                .recv_from(&mut buf)
                .await
                .expect("receive CONNECT_REQUEST");
            let (header, _) = frame::decode_frame(&buf[..n]).expect("decode CONNECT_REQUEST");
            assert_eq!(header.service_type, services::CONNECT_REQUEST);

            let channel_id = 0x15;
            let mut body = vec![channel_id, services::E_NO_ERROR];
            body.extend_from_slice(
                &Hpai {
                    addr: *server_addr.ip(),
                    port: server_addr.port(),
                }
                .encode(),
            );
            body.extend_from_slice(&[0x04, tunnelling::TUNNEL_CONNECTION, 0x11, 0x01]);
            server
                .send_to(
                    &frame::encode_frame(services::CONNECT_RESPONSE, &body),
                    client_addr,
                )
                .await
                .expect("send CONNECT_RESPONSE");

            let (n, client_addr) = server
                .recv_from(&mut buf)
                .await
                .expect("receive DISCONNECT_REQUEST");
            let (header, body) = frame::decode_frame(&buf[..n]).expect("decode DISCONNECT_REQUEST");
            assert_eq!(header.service_type, services::DISCONNECT_REQUEST);
            assert_eq!(body.first(), Some(&channel_id));
            request_seen_tx.send(()).expect("test still waiting");

            release_response_rx.await.expect("release final response");
            server
                .send_to(
                    &frame::encode_frame(
                        services::DISCONNECT_RESPONSE,
                        &services::encode_disconnect_response(channel_id, services::E_NO_ERROR),
                    ),
                    client_addr,
                )
                .await
                .expect("send DISCONNECT_RESPONSE");
        });

        let client = KnxNetIpClient::new()
            .connect_tunnel(server_addr)
            .await
            .expect("connect to loopback server");
        let mut disconnect = tokio::spawn(async move { client.disconnect().await });
        request_seen_rx
            .await
            .expect("server saw disconnect request");

        assert!(
            tokio::time::timeout(Duration::from_millis(50), &mut disconnect)
                .await
                .is_err(),
            "disconnect returned before DISCONNECT_RESPONSE"
        );

        release_response_tx.send(()).expect("peer still waiting");
        disconnect
            .await
            .expect("disconnect task did not panic")
            .expect("matching successful response completes disconnect");
        peer.await.expect("loopback peer did not panic");
    }

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
    ///
    /// The skip guard covers only `probe.connect()` *failing*. In a
    /// namespace that has a multicast route on `lo` (`unshare -rn` plus
    /// `ip route add 224.0.0.0/4 dev lo`) the `connect()` succeeds, the
    /// resolved HPAI is `0.0.0.0`, and this hard-fails below instead of
    /// skipping — measured, and recorded in KNOWN_LIMITATIONS.md §33. A
    /// bare `unshare -rn` with only `lo up` is unaffected: no such route,
    /// so the `connect()` gets `ENETUNREACH` and the skip fires.
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
    ///
    /// Goes through `RoutingClient::connect_with` rather than
    /// `KnxNetIpClient::connect_routing` for one reason only:
    /// `RoutingSocketOptions::LOOPBACK_ONLY`. With production's options
    /// this test sent a real `GroupValueWrite` to `1/2/3` out of whatever
    /// interface the routing table pointed at — on a KNX engineer's
    /// machine, the one the installation is on (KNOWN_LIMITATIONS.md §33).
    #[tokio::test]
    async fn routing_client_sends_and_receives_a_group_value_write() {
        use crate::cemi::{ApplicationService, Destination, GroupValue};
        use knx_core::{GroupAddress, GroupAddressStyle, IndividualAddress};

        let sender_address = IndividualAddress::new(1, 1, 1).unwrap();
        let receiver_address = IndividualAddress::new(1, 1, 2).unwrap();

        let sender =
            match RoutingClient::connect_with(sender_address, RoutingSocketOptions::LOOPBACK_ONLY)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "skipping routing_client_sends_and_receives_a_group_value_write: \
                     could not join the routing multicast group on loopback \
                     in this sandbox: {e}"
                    );
                    return;
                }
            };
        let receiver =
            RoutingClient::connect_with(receiver_address, RoutingSocketOptions::LOOPBACK_ONLY)
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
        // yet never actually deliver the datagram locally — a stricter,
        // no-route-at-all failure than the ones `connect_with` above can
        // detect. Before skipping on that theory, check it: ask the kernel
        // the same question with sockets `RoutingClient` had no hand in
        // (`loopback_multicast_is_deliverable`). If those two do reach each
        // other, "no datagram arrived" is a regression in `RoutingClient`,
        // not a sandbox property, and must fail rather than skip — without
        // this, deleting the `send_to` call outright leaves the test
        // passing (measured).
        //
        // `recv_from_source` rather than a bare `telegrams.recv()`: this
        // receiver also hears the other round-trip test's `1.1.5` and the
        // ROUTING_BUSY test's `1.1.3`, and used to assert on whichever
        // arrived first. See `recv_from_source`'s own comment.
        let received = match recv_from_source(&mut telegrams, sender_address, TELEGRAM_WAIT).await {
            Some(telegram) => telegram,
            None => {
                assert!(
                    !loopback_multicast_is_deliverable().await,
                    "no ROUTING_INDICATION from the sender arrived within 5s, yet two plain UDP sockets \
                     configured the same way do deliver to each other on this host — \
                     that is a regression in RoutingClient's send/receive path, not a \
                     sandbox limitation"
                );
                eprintln!(
                    "skipping routing_client_sends_and_receives_a_group_value_write: \
                     joined the multicast group but no datagram arrived within 5s, and \
                     a plain-socket probe does not deliver either — this sandbox accepts \
                     the join/send but does not deliver multicast locally"
                );
                return;
            }
        };
        // Guaranteed by `recv_from_source`'s filter rather than tested by
        // it, and kept because it is the property being claimed: the frame
        // that came back carries the sender's own individual address.
        assert_eq!(received.source, sender_address);
        assert_eq!(received.destination, Destination::Group(group_address));
        assert_eq!(
            received.service,
            ApplicationService::GroupValueWrite(GroupValue::Short(1))
        );
    }

    /// How long a round-trip test waits for its own telegram before it
    /// concludes nothing is coming. Measured delivery on this host is
    /// 0.15 s; five seconds is slack, not an expectation.
    const TELEGRAM_WAIT: Duration = Duration::from_secs(5);

    /// Waits for a telegram whose source is `source`, discarding everyone
    /// else's, and gives up after `within`.
    ///
    /// Both round-trip tests join `224.0.23.12:3671`, so does
    /// `routing_client_send_waits_out_a_routing_busy_deadline`, and a UDP
    /// socket bound to the wildcard address is handed every multicast
    /// datagram the host accepts on that port — the membership decides what
    /// the *host* accepts, not which socket gets a copy. The harness runs
    /// those tests concurrently, so a bare `telegrams.recv()` returns
    /// whichever telegram arrived first, from whichever test. That is new
    /// as of B1: before `LOOPBACK_ONLY` pinned `IP_MULTICAST_IF` to `lo`,
    /// nothing was ever delivered locally and the collision could not
    /// happen — `IP_MULTICAST_LOOP` turning on at the same time is not the
    /// reason; measured inert on this kernel, a socket pair with it read
    /// back as 0 still delivers. Measured with the bare `recv()`: 2 failures
    /// in 100 runs of `client::tests::`, every one of them `1.1.5` — the
    /// probe-agreement test's sender — surfacing in `1.1.1`'s receiver.
    ///
    /// Separate multicast groups would *not* have fixed it, for the same
    /// reason: a wildcard-bound socket receives datagrams for groups it
    /// never joined, as long as some socket on the host joined them.
    ///
    /// Filters on source address alone, not on any per-run identifier, so a
    /// second concurrent copy of this test binary could satisfy a wait with
    /// a foreign process's identical `1.1.1`/`1.1.5` frame. Harmless as
    /// written, but any future mutation proof that deletes a send must run
    /// with exactly one copy of the suite live, or it will pass for the
    /// wrong reason.
    async fn recv_from_source(
        telegrams: &mut broadcast::Receiver<LDataFrame>,
        source: IndividualAddress,
        within: Duration,
    ) -> Option<LDataFrame> {
        let deadline = tokio::time::Instant::now() + within;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            match tokio::time::timeout(remaining, telegrams.recv()).await {
                Ok(Ok(telegram)) if telegram.source == source => return Some(telegram),
                // Another test's telegram: keep waiting.
                Ok(Ok(_)) => {}
                // Capacity 64 measured sufficient for three concurrent senders,
                // 0/300 runs — a lag here is not this test's business as usual,
                // and folding it into "keep waiting" would let it masquerade as
                // the timeout arm's "no datagram arrived" if it dropped the one
                // telegram this call is waiting for. Name it instead.
                Ok(Err(broadcast::error::RecvError::Lagged(skipped))) => {
                    panic!("broadcast receiver lagged, dropping {skipped} telegram(s) while waiting for {source}")
                }
                Ok(Err(broadcast::error::RecvError::Closed)) => {
                    panic!("broadcast channel closed unexpectedly")
                }
                Err(_) => return None,
            }
        }
    }

    /// Does this machine deliver a loopback-pinned multicast datagram at
    /// all? Deliberately built from plain `socket2`/`tokio` sockets rather
    /// than from `RoutingClient`, so that the answer is about the kernel
    /// and not about the code under test — that is the whole point: it
    /// tells a sandbox limitation apart from a `RoutingClient` regression,
    /// which KNOWN_LIMITATIONS.md §33 originally listed as the thing this
    /// test could not do.
    ///
    /// Same options as `RoutingSocketOptions::LOOPBACK_ONLY`
    /// (`IP_MULTICAST_IF` = `127.0.0.1`, TTL 0 — one lock and one
    /// courtesy, see there), on a different group: `239.0.2.1`, RFC 2365
    /// administratively scoped, never seen on a real installation.
    ///
    /// The different group does *not* keep the payload out of a
    /// `RoutingClient`'s receive loop, whatever this comment claimed
    /// until 2026-09-20 —
    /// `connect_routing_to_group_accepts_an_administratively_scoped_address`
    /// and `connect_routing_to_group_joins_the_given_group_not_the_default`
    /// join exactly `239.0.2.1:3671`, and their loops do get it. It never
    /// counts as a decode failure for a different reason:
    /// `frame::decode_frame` rejects `b"knxbench-loopback-probe"` at the
    /// header (`'k'` is not `0x06`) and `routing_receive_loop` `continue`s,
    /// so the payload never reaches the cEMI decode that owns the counter.
    ///
    /// Any error at all means "cannot tell" and is reported as not
    /// deliverable: this only ever decides whether to skip, so the
    /// conservative answer is the one that skips.
    async fn loopback_multicast_is_deliverable() -> bool {
        use socket2::{Domain, Socket, Type};

        const PROBE_GROUP: Ipv4Addr = Ipv4Addr::new(239, 0, 2, 1);
        const PROBE_PORT: u16 = 3671;

        fn probe_socket() -> std::io::Result<UdpSocket> {
            let socket2_socket = Socket::new(Domain::IPV4, Type::DGRAM, None)?;
            socket2_socket.set_reuse_address(true)?;
            socket2_socket
                .bind(&std::net::SocketAddr::from((Ipv4Addr::UNSPECIFIED, PROBE_PORT)).into())?;
            socket2_socket.set_nonblocking(true)?;
            socket2_socket.set_multicast_if_v4(&Ipv4Addr::LOCALHOST)?;
            socket2_socket.set_multicast_ttl_v4(0)?;
            let std_socket: std::net::UdpSocket = socket2_socket.into();
            let socket = UdpSocket::from_std(std_socket)?;
            socket.join_multicast_v4(PROBE_GROUP, Ipv4Addr::LOCALHOST)?;
            // Inert here, and kept only so the probe mirrors
            // `LOOPBACK_ONLY` option for option: measured on this kernel,
            // a probe pair with `IP_MULTICAST_LOOP` read back as 0 still
            // reaches itself, because delivery on `lo` goes through the
            // device path regardless. The probe's verdict does not depend
            // on this line.
            socket.set_multicast_loop_v4(true)?;
            Ok(socket)
        }

        let (Ok(sender), Ok(receiver)) = (probe_socket(), probe_socket()) else {
            return false;
        };
        if sender
            .send_to(
                b"knxbench-loopback-probe",
                SocketAddrV4::new(PROBE_GROUP, PROBE_PORT),
            )
            .await
            .is_err()
        {
            return false;
        }
        let mut buf = [0u8; 32];
        matches!(
            tokio::time::timeout(Duration::from_secs(2), receiver.recv_from(&mut buf)).await,
            Ok(Ok(_))
        )
    }

    /// The probe above has to be right about *this* machine before the
    /// round-trip test may lean on it, and it is the kind of helper that
    /// could quietly start answering `false` for everyone (a typo'd
    /// option, an error arm swallowing the wrong thing) and turn the
    /// round-trip test back into an unconditional skip. On a host that
    /// does deliver loopback multicast — measured here and in a bare
    /// `unshare -rn` namespace — it must say so. A sandbox that genuinely
    /// cannot is exactly the case the probe exists to detect, so this one
    /// cannot assert unconditionally either; it asserts agreement with the
    /// round trip instead, which is the property that matters.
    #[tokio::test]
    async fn the_loopback_probe_agrees_with_an_actual_loopback_round_trip() {
        use crate::cemi::{ApplicationService, Destination, GroupValue};
        use knx_core::{GroupAddress, GroupAddressStyle, IndividualAddress};

        let deliverable = loopback_multicast_is_deliverable().await;

        let sender_address = IndividualAddress::new(1, 1, 5).unwrap();
        let sender =
            match RoutingClient::connect_with(sender_address, RoutingSocketOptions::LOOPBACK_ONLY)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "skipping the_loopback_probe_agrees_with_an_actual_loopback_round_trip: \
                     could not join the routing multicast group on loopback \
                     in this sandbox: {e}"
                    );
                    return;
                }
            };
        let receiver = RoutingClient::connect_with(
            IndividualAddress::new(1, 1, 6).unwrap(),
            RoutingSocketOptions::LOOPBACK_ONLY,
        )
        .await
        .expect("second RoutingClient should join the same group fine (SO_REUSEADDR)");
        let mut telegrams = receiver.subscribe();
        sender
            .send(
                Destination::Group(
                    GroupAddress::parse("1/2/3", GroupAddressStyle::ThreeLevel).unwrap(),
                ),
                ApplicationService::GroupValueWrite(GroupValue::Short(1)),
            )
            .await
            .expect("send over loopback multicast should succeed");
        // `recv_from_source`, not a bare `recv()`: this receiver also hears
        // `routing_client_sends_and_receives_a_group_value_write`'s `1.1.1`
        // and the ROUTING_BUSY test's `1.1.3`, and "some telegram arrived"
        // would call the round trip a success even if this sender's own
        // frame never left.
        let round_tripped = recv_from_source(&mut telegrams, sender_address, TELEGRAM_WAIT)
            .await
            .is_some();

        assert_eq!(
            deliverable, round_tripped,
            "the probe and the real round trip must agree about whether this host \
             delivers loopback multicast; if they disagree the probe is lying and the \
             skip decision it guards is worthless"
        );
    }

    /// KNOWN_LIMITATIONS.md #32: `send()` must wait out a `ROUTING_BUSY`
    /// deadline already in effect before transmitting. Sets `busy_until`
    /// directly (this test lives inside `client` itself, so `RoutingState`'s
    /// private field is reachable) rather than round-tripping a real
    /// `ROUTING_BUSY` datagram through the receive loop — that path is
    /// exercised by `merge_busy_deadline_keeps_the_later_of_the_two` above,
    /// so this test's job is only to confirm `send()` actually honours the
    /// deadline once set. Only needs the connect to succeed (to reach a
    /// real `send()`); delivery is irrelevant, so unlike the round-trip
    /// test above this one only skips if joining the multicast group itself
    /// fails. `LOOPBACK_ONLY` for the same reason as the test above: this
    /// one reaches a real `send()`, and a real `send()` with production's
    /// options is a real telegram on a real LAN.
    #[tokio::test]
    async fn routing_client_send_waits_out_a_routing_busy_deadline() {
        use crate::cemi::{ApplicationService, Destination, GroupValue};
        use knx_core::{GroupAddress, GroupAddressStyle, IndividualAddress};

        let own_address = IndividualAddress::new(1, 1, 3).unwrap();
        let client =
            match RoutingClient::connect_with(own_address, RoutingSocketOptions::LOOPBACK_ONLY)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "skipping routing_client_send_waits_out_a_routing_busy_deadline: \
                     could not join the routing multicast group on loopback \
                     in this sandbox: {e}"
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

    /// E6, R1: the Standard never narrows a routing multicast group below
    /// "any IPv4 multicast address" (Core v01.06.02 AS §8.5.2.2 only speaks
    /// of an offset from the default, with no stated bound), so the
    /// permitted range is exactly `Ipv4Addr::is_multicast()`'s 224.0.0.0/4.
    /// A unicast address must be rejected before any socket call — no
    /// sandbox dependency, so this never skips. `192.0.2.1` is an RFC 5737
    /// example address, not a real gateway.
    #[tokio::test]
    async fn connect_routing_to_group_rejects_a_unicast_address() {
        let own_address = IndividualAddress::new(1, 1, 4).unwrap();
        let unicast = Ipv4Addr::new(192, 0, 2, 1);
        let result = RoutingClient::connect_to_group(own_address, unicast).await;
        match result {
            Err(BusError::NotMulticast(addr)) => assert_eq!(addr, unicast),
            Err(other) => panic!("expected BusError::NotMulticast({unicast}), got {other:?}"),
            Ok(_) => panic!("expected BusError::NotMulticast({unicast}), got Ok"),
        }
    }

    /// Boundary just below the permitted range — one bit short of Class D
    /// — must still be rejected, not rounded into it.
    #[tokio::test]
    async fn connect_routing_to_group_rejects_the_address_just_below_the_multicast_range() {
        let own_address = IndividualAddress::new(1, 1, 4).unwrap();
        let below_range = Ipv4Addr::new(223, 255, 255, 255);
        let result = RoutingClient::connect_to_group(own_address, below_range).await;
        assert!(matches!(result, Err(BusError::NotMulticast(a)) if a == below_range));
    }

    /// Boundary just above the permitted range — the first Class E address
    /// — must also be rejected.
    #[tokio::test]
    async fn connect_routing_to_group_rejects_the_address_just_above_the_multicast_range() {
        let own_address = IndividualAddress::new(1, 1, 4).unwrap();
        let above_range = Ipv4Addr::new(240, 0, 0, 0);
        let result = RoutingClient::connect_to_group(own_address, above_range).await;
        assert!(matches!(result, Err(BusError::NotMulticast(a)) if a == above_range));
    }

    /// A genuine multicast address must clear validation regardless of
    /// whether this sandbox can actually join it — `239.0.2.1` sits in the
    /// administratively-scoped range (RFC 2365), chosen only as an example
    /// never observed on a real installation. Unlike the round-trip tests
    /// above, this does not skip: a validation bug would show up as
    /// `Err(NotMulticast(_))` regardless of sandbox networking, so there is
    /// nothing here for a sandbox limitation to hide. `LOOPBACK_ONLY`
    /// because validation passing means the socket really is created and
    /// really does join — on loopback, where no installation is listening.
    #[tokio::test]
    async fn connect_routing_to_group_accepts_an_administratively_scoped_address() {
        let own_address = IndividualAddress::new(1, 1, 4).unwrap();
        let group = Ipv4Addr::new(239, 0, 2, 1);
        let result = RoutingClient::connect_to_group_with(
            own_address,
            group,
            RoutingSocketOptions::LOOPBACK_ONLY,
        )
        .await;
        assert!(
            !matches!(result, Err(BusError::NotMulticast(_))),
            "a genuine multicast address must not fail validation"
        );
    }

    /// `connect_routing()` with no override must still join exactly
    /// `ROUTING_MULTICAST` — asserted against the named constant, not a
    /// repeated `224.0.23.12:3671` literal, so this fails the moment the
    /// default silently drifts from the constant the rest of the module
    /// uses. Skips, rather than fails, if this sandbox has no multicast
    /// route at all, same policy as the round-trip test above.
    ///
    /// `connect_with` is `connect()`'s body with the socket options as a
    /// parameter, so the group this asserts on is still the one
    /// `connect()` picks; what it no longer covers is the one-line
    /// `KnxNetIpClient::connect_routing` -> `RoutingClient::connect`
    /// delegation, which cannot be exercised without joining the real
    /// group on the real interface.
    #[tokio::test]
    async fn connect_routing_joins_the_standard_group_by_default() {
        let own_address = IndividualAddress::new(1, 1, 4).unwrap();
        let client =
            match RoutingClient::connect_with(own_address, RoutingSocketOptions::LOOPBACK_ONLY)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "skipping connect_routing_joins_the_standard_group_by_default: \
                     could not join the routing multicast group on loopback \
                     in this sandbox: {e}"
                    );
                    return;
                }
            };
        assert_eq!(
            client.group, ROUTING_MULTICAST,
            "connect() without an override must still join the standard group"
        );
    }

    /// KNOWN_LIMITATIONS §31 / AR14: a custom routing group exists to keep
    /// installations apart (Routing v01.05.02 AS §2.3.2), so a telegram sent
    /// on one must reach that group's members and not a default-group client
    /// on the same host. Loopback only: evidence for this host's socket
    /// delivery, not for a KNXnet/IP router on a real network.
    #[tokio::test]
    async fn a_custom_group_telegram_reaches_its_group_and_not_the_default_one() {
        use crate::cemi::{ApplicationService, Destination, GroupValue};
        use knx_core::GroupAddress;

        let group = Ipv4Addr::new(239, 0, 2, 14);
        let sender_address = IndividualAddress::new(1, 1, 14).unwrap();
        let sender = match RoutingClient::connect_to_group_with(
            sender_address,
            group,
            RoutingSocketOptions::LOOPBACK_ONLY,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "skipping a_custom_group_telegram_reaches_its_group_and_not_the_default_one: \
                     could not join {group} on loopback in this sandbox: {e}"
                );
                return;
            }
        };
        let member = RoutingClient::connect_to_group_with(
            IndividualAddress::new(1, 1, 15).unwrap(),
            group,
            RoutingSocketOptions::LOOPBACK_ONLY,
        )
        .await
        .expect("a second client joins the same custom group");
        let bystander = RoutingClient::connect_with(
            IndividualAddress::new(1, 1, 16).unwrap(),
            RoutingSocketOptions::LOOPBACK_ONLY,
        )
        .await
        .expect("a default-group client joins next to it");
        let mut member_telegrams = member.subscribe();
        let mut bystander_telegrams = bystander.subscribe();

        let destination = Destination::Group(GroupAddress::from_raw(0x0A0E));
        sender
            .send(
                destination,
                ApplicationService::GroupValueWrite(GroupValue::Short(1)),
            )
            .await
            .expect("send over loopback multicast should succeed");

        let Some(received) =
            recv_from_source(&mut member_telegrams, sender_address, TELEGRAM_WAIT).await
        else {
            assert!(
                !loopback_multicast_is_deliverable().await,
                "the custom group's own member never heard the sender, yet plain sockets \
                 deliver multicast on this host"
            );
            eprintln!(
                "skipping a_custom_group_telegram_reaches_its_group_and_not_the_default_one: \
                 this sandbox does not deliver multicast locally"
            );
            return;
        };
        assert_eq!(received.destination, destination);

        let leaked = recv_from_source(
            &mut bystander_telegrams,
            sender_address,
            Duration::from_millis(500),
        )
        .await;
        assert!(
            leaked.is_none(),
            "a default-group client heard a telegram sent on custom group {group}: {leaked:?}"
        );
    }

    /// The override path must join the *given* group, not silently fall
    /// back to the default — the one way the two connect paths could
    /// drift despite sharing `connect_to_group`'s body.
    #[tokio::test]
    async fn connect_routing_to_group_joins_the_given_group_not_the_default() {
        let own_address = IndividualAddress::new(1, 1, 4).unwrap();
        let group = Ipv4Addr::new(239, 0, 2, 1);
        let client = match RoutingClient::connect_to_group_with(
            own_address,
            group,
            RoutingSocketOptions::LOOPBACK_ONLY,
        )
        .await
        {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "skipping connect_routing_to_group_joins_the_given_group_not_the_default: \
                     could not join {group} on loopback in this sandbox: {e}"
                );
                return;
            }
        };
        assert_eq!(
            client.group,
            SocketAddrV4::new(group, ROUTING_MULTICAST.port())
        );
        assert_ne!(client.group, ROUTING_MULTICAST);
    }

    /// The options every real caller gets must stay the kernel's own
    /// defaults: a routing client that pinned itself to `lo`, or refused
    /// to leave the host, would be useless on a real installation. No
    /// socket involved — this is a guard on the constant itself, so it
    /// cannot skip.
    #[test]
    fn production_routing_socket_options_leave_the_network_to_the_kernel() {
        let production = RoutingSocketOptions::PRODUCTION;
        assert!(
            production.interface.is_unspecified(),
            "production must let the routing table pick the outgoing interface"
        );
        assert!(
            !production.loop_back,
            "production must not deliver our own sends back into our own subscribe()"
        );
        assert_eq!(
            production.ttl, None,
            "production must leave IP_MULTICAST_TTL at the kernel default, not force one"
        );
    }

    /// The regression guard for KNOWN_LIMITATIONS.md §33: asks the kernel
    /// what the test socket is actually set to, rather than trusting that
    /// asking for `LOOPBACK_ONLY` had any effect. If someone routes these
    /// tests back through `PRODUCTION`, or `set_multicast_if_v4` stops
    /// being called, this fails instead of quietly resuming transmission
    /// on the LAN interface.
    #[tokio::test]
    async fn loopback_only_options_actually_reach_the_socket() {
        let own_address = IndividualAddress::new(1, 1, 4).unwrap();
        let client =
            match RoutingClient::connect_with(own_address, RoutingSocketOptions::LOOPBACK_ONLY)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "skipping loopback_only_options_actually_reach_the_socket: \
                     could not join the routing multicast group on loopback \
                     in this sandbox: {e}"
                    );
                    return;
                }
            };
        let socket = socket2::SockRef::from(&client.state.socket);
        assert_eq!(
            socket
                .multicast_if_v4()
                .expect("IP_MULTICAST_IF should be readable"),
            Ipv4Addr::LOCALHOST,
            "test sockets must send via lo, never via whatever the routing table picks"
        );
        assert_eq!(
            socket
                .multicast_ttl_v4()
                .expect("IP_MULTICAST_TTL should be readable"),
            0,
            "TTL 0 stops an IP router forwarding the datagram off this subnet — \
             it is not a second lock, and the frame still goes on the link"
        );
        assert!(
            socket
                .multicast_loop_v4()
                .expect("IP_MULTICAST_LOOP should be readable"),
            "local delivery is the whole point of a loopback-only round-trip test"
        );
    }

    /// `/proc/net/igmp` in exactly one `read(2)`, not via
    /// `std::fs::read_to_string`. That helper cannot learn a procfs file's
    /// length, so it reads in several growing chunks — and `/proc/net/igmp`
    /// is a `seq_file` whose iterator re-seeks by *index* between reads. With
    /// the rest of this suite joining and dropping the same group
    /// concurrently, that re-seek lands past records that were there a
    /// moment ago. Measured: 5 failures in 120 runs of `client::tests::`,
    /// every one a snapshot with no membership at all for a group a re-read
    /// microseconds later showed held by five sockets. One read is one pass
    /// of the iterator, so one consistent answer.
    ///
    /// A `seq_file` hands back at most one kernel page per `read(2)`, no
    /// matter how large the caller's buffer is — a 64 KiB buffer proves
    /// nothing about a file that grows past 4 KiB. Measured with 800
    /// memberships: the file was 26521 bytes and the first read returned
    /// 4081, comfortably clear of the buffer's own size. The only real test
    /// is a second `read(2)` on the same fd: EOF (`0`) means the first read
    /// really was the whole file, anything else means it was cut at a page
    /// boundary. Measured: `0` at genuine EOF, `4092` on a truncated file.
    /// Truncation is treated as "cannot be sure", the same skip path an
    /// unreadable file already takes — a wrong answer here is worse than a
    /// skipped test.
    fn read_proc_net_igmp() -> std::io::Result<String> {
        use std::io::Read;

        let mut file = std::fs::File::open("/proc/net/igmp")?;
        let mut buf = vec![0u8; 64 * 1024];
        let filled = file.read(&mut buf)?;
        buf.truncate(filled);

        let mut probe = [0u8; 1];
        if file.read(&mut probe)? != 0 {
            return Err(std::io::Error::other(
                "/proc/net/igmp did not fit in one seq_file read(2); \
                 the snapshot is truncated and cannot be trusted",
            ));
        }

        String::from_utf8(buf).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Every device in `/proc/net/igmp` currently holding a membership for
    /// `group`, mapped to the number of sockets holding it. The file lists
    /// an interface line (`Idx<TAB>Device : Count Querier`) followed by
    /// tab-indented group lines, and prints the group as the in-memory
    /// 32-bit address in hex — which on a little-endian host reads
    /// back-to-front, `224.0.23.12` as `0C1700E0`. Both orderings are
    /// accepted so this does not quietly stop matching on a big-endian
    /// machine and pass by finding nothing.
    fn igmp_memberships(group: Ipv4Addr) -> std::io::Result<HashMap<String, u32>> {
        let o = group.octets();
        let little = format!("{:02X}{:02X}{:02X}{:02X}", o[3], o[2], o[1], o[0]);
        let big = format!("{:02X}{:02X}{:02X}{:02X}", o[0], o[1], o[2], o[3]);

        let text = read_proc_net_igmp()?;
        let mut memberships = HashMap::new();
        let mut device = String::new();
        for line in text.lines().skip(1) {
            if line.starts_with('\t') {
                let mut fields = line.split_whitespace();
                if let (Some(listed), Some(users)) = (fields.next(), fields.next()) {
                    if listed.eq_ignore_ascii_case(&little) || listed.eq_ignore_ascii_case(&big) {
                        *memberships.entry(device.clone()).or_insert(0) +=
                            users.parse::<u32>().unwrap_or(0);
                    }
                }
            } else if let Some((_index, rest)) = line.split_once('\t') {
                device = rest
                    .split(':')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string();
            }
        }
        Ok(memberships)
    }

    /// The mutation guard for `join_multicast_v4`'s interface argument.
    /// Changing it back to `Ipv4Addr::UNSPECIFIED` leaves `IP_MULTICAST_IF`
    /// pinned, so `loopback_only_options_actually_reach_the_socket` still
    /// passes and the whole suite stays green — while the membership, and
    /// the IGMP membership report announcing it, move to whatever the
    /// routing table picks. On the machine KNOWN_LIMITATIONS.md §33 was
    /// written on that is the interface the installation is on. `socket2`
    /// exposes no getter for a join's interface, so this asks the kernel.
    ///
    /// Asserts only that no device other than `lo` *gained* a membership:
    /// the rest of the suite joins and drops this same group on `lo`
    /// concurrently, and another process on the machine may hold it on a
    /// physical interface for reasons of its own. A before/after delta is
    /// the part that is actually about this code.
    #[tokio::test]
    async fn the_loopback_join_lands_on_lo_and_nowhere_else() {
        let group = *ROUTING_MULTICAST.ip();
        let Ok(before) = igmp_memberships(group) else {
            eprintln!(
                "skipping the_loopback_join_lands_on_lo_and_nowhere_else: \
                 /proc/net/igmp is not readable in this sandbox"
            );
            return;
        };

        let own_address = IndividualAddress::new(1, 1, 4).unwrap();
        let client =
            match RoutingClient::connect_with(own_address, RoutingSocketOptions::LOOPBACK_ONLY)
                .await
            {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "skipping the_loopback_join_lands_on_lo_and_nowhere_else: \
                     could not join the routing multicast group on loopback \
                     in this sandbox: {e}"
                    );
                    return;
                }
            };

        let after = igmp_memberships(group).expect("/proc/net/igmp was readable a moment ago");
        assert!(
            after.contains_key("lo"),
            "a LOOPBACK_ONLY client must hold {group} on lo; /proc/net/igmp lists it on \
             {:?}. The table as it reads now:\n{}",
            after.keys().collect::<Vec<_>>(),
            read_proc_net_igmp().unwrap_or_default()
        );
        for (device, users) in &after {
            if device == "lo" {
                continue;
            }
            let was = before.get(device).copied().unwrap_or(0);
            assert!(
                *users <= was,
                "joining {group} with LOOPBACK_ONLY added a membership on {device} \
                 ({was} before, {users} after) — the interface argument to \
                 join_multicast_v4 is not reaching the kernel, and IGMP membership \
                 reports are going out on a real installation's LAN again \
                 (KNOWN_LIMITATIONS.md §33)"
            );
        }
        drop(client);
    }

    /// The mutation guard for `set_multicast_loop_v4(options.loop_back)`.
    /// `production_routing_socket_options_leave_the_network_to_the_kernel`
    /// guards the *constant*, and nothing read the value back off a socket:
    /// hardcoding `true` at that call site ships a routing client that
    /// hears its own sends in `subscribe()` — the bug the comment there
    /// exists to prevent — with the suite still green (measured).
    ///
    /// A `PRODUCTION` socket cannot be built in a test: it would join the
    /// real group on the real interface, which is the whole of §33. So this
    /// takes production's `loop_back` and keeps `LOOPBACK_ONLY`'s pin, which
    /// is the part that makes it safe to build at all.
    #[tokio::test]
    async fn a_loop_back_of_false_reaches_the_socket_as_false() {
        let options = RoutingSocketOptions {
            loop_back: false,
            ..RoutingSocketOptions::LOOPBACK_ONLY
        };
        assert_eq!(
            options.loop_back,
            RoutingSocketOptions::PRODUCTION.loop_back,
            "this test is only worth anything while it carries production's own value"
        );

        let own_address = IndividualAddress::new(1, 1, 4).unwrap();
        let client = match RoutingClient::connect_with(own_address, options).await {
            Ok(c) => c,
            Err(e) => {
                eprintln!(
                    "skipping a_loop_back_of_false_reaches_the_socket_as_false: \
                     could not join the routing multicast group on loopback \
                     in this sandbox: {e}"
                );
                return;
            }
        };
        assert!(
            !socket2::SockRef::from(&client.state.socket)
                .multicast_loop_v4()
                .expect("IP_MULTICAST_LOOP should be readable"),
            "loop_back: false must reach the socket as IP_MULTICAST_LOOP = 0; \
             a literal `true` at that call site would put every send back \
             into this client's own subscribe()"
        );
    }

    /// Finding 6 (T17 fix round 2): `decode_failure_count()` is public,
    /// has callers nowhere but the two `eprintln!` sites in `receive_loop`
    /// above, and had zero tests. `TunnelState.socket` is a concrete
    /// `tokio::net::UdpSocket` with no trait behind it and the
    /// increment-and-print gate lives inline inside the socket-driven
    /// read loop, so there is no way to drive it honestly without a real
    /// socket — this one talks to itself on loopback only (`127.0.0.1`,
    /// OS-assigned port): no gateway, no bus, no KNX individual/group
    /// address is ever sent.
    ///
    /// Builds a `TunnelState` directly (same file, private fields
    /// reachable) rather than going through `TunnelClient::connect`'s
    /// real `CONNECT_REQUEST`/`CONNECT_RESPONSE` handshake, which needs
    /// an actual gateway to answer it and is not this test's concern.
    #[tokio::test]
    async fn decode_failure_count_counts_undecodable_frames_once_each() {
        let state_socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind loopback state socket");
        let peer_socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("bind loopback peer socket");
        let peer_addr = peer_socket.local_addr().expect("peer socket local addr");
        state_socket
            .connect(peer_addr)
            .await
            .expect("connect state socket to its loopback peer");
        let state_addr = state_socket.local_addr().expect("state socket local addr");

        let (tx, _rx) = broadcast::channel(64);
        let channel_id = 7u8;
        let state = Arc::new(TunnelState {
            socket: state_socket,
            channel_id,
            assigned_address: IndividualAddress::new(1, 1, 1).unwrap(),
            tx,
            heartbeat_reply: Mutex::new(None),
            heartbeat_notify: Notify::new(),
            disconnect_reply: Mutex::new(None),
            disconnect_notify: Notify::new(),
            shutdown: Notify::new(),
            send_seq: Mutex::new(0),
            ack_reply: Mutex::new(None),
            ack_notify: Notify::new(),
            decode_failures: AtomicU64::new(0),
        });
        let client = TunnelClient {
            state: state.clone(),
        };
        tokio::spawn(receive_loop(state));

        assert_eq!(
            client.decode_failure_count(),
            0,
            "must start at zero before any frame has arrived"
        );

        // An empty cEMI payload: `cemi::decode_l_data` rejects anything
        // under 2 octets as `TooShort`, so this is undecodable by
        // construction, not by accident.
        let bad_cemi: &[u8] = &[];
        for sequence_counter in 0..2u8 {
            let body =
                tunnelling::encode_tunnelling_request(channel_id, sequence_counter, bad_cemi);
            let datagram = frame::encode_frame(tunnelling::TUNNELLING_REQUEST, &body);
            peer_socket
                .send_to(&datagram, state_addr)
                .await
                .expect("send undecodable frame to the loopback state socket");
        }

        // `fetch_add(...) == 0` (the print gate) can only be true once:
        // the first undecodable frame takes the counter 0 -> 1 and
        // prints; the second takes it 1 -> 2 and does not. Landing on
        // exactly 2, not 1 (dropped) and not more (double-counted),
        // proves both halves of that gate fired the way `receive_loop`
        // intends.
        let expected = 2u64;
        let mut observed = 0u64;
        for _ in 0..200 {
            observed = client.decode_failure_count();
            if observed >= expected {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
        assert_eq!(
            observed, expected,
            "two undecodable frames must each increment the counter exactly once"
        );
    }
}
