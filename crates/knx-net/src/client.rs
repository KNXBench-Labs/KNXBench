//! The `BusConnection` trait and its tunnelling implementation
//! (ARCHITECTURE.md §8). `discover`/`TunnelClient::send` are explicit
//! `BusError::NotImplemented` stubs this cycle — never `todo!()`, never a
//! silent no-op (CLAUDE.md).

use std::net::{SocketAddr, SocketAddrV4};
use std::sync::Arc;
use std::time::Duration;

use knx_core::IndividualAddress;
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, Mutex, Notify};

use crate::cemi::{self, LDataFrame};
use crate::core::hpai::Hpai;
use crate::core::services;
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

/// A local KNXnet/IP client, not yet connected to any gateway. Cycle 1
/// implements `connect_tunnel` only; `discover` (multicast `SEARCH_REQUEST`,
/// Core v01.06.02 AS §4.2) is a later cycle's work.
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
    async fn discover(&self) -> Result<Vec<SocketAddrV4>, BusError>;
    async fn connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient, BusError>;
}

impl BusConnection for KnxNetIpClient {
    async fn discover(&self) -> Result<Vec<SocketAddrV4>, BusError> {
        Err(BusError::NotImplemented)
    }

    async fn connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient, BusError> {
        TunnelClient::connect(gateway).await
    }
}

struct TunnelState {
    socket: UdpSocket,
    channel_id: u8,
    assigned_address: IndividualAddress,
    tx: broadcast::Sender<LDataFrame>,
    heartbeat_reply: Mutex<Option<u8>>,
    heartbeat_notify: Notify,
    shutdown: Notify,
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
        let body = services::encode_connect_request(control_hpai, &cri, control_hpai);
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
        });

        tokio::spawn(receive_loop(state.clone()));
        tokio::spawn(heartbeat_loop(state.clone()));

        Ok(TunnelClient { state })
    }

    pub fn assigned_address(&self) -> IndividualAddress {
        self.state.assigned_address
    }

    pub fn subscribe(&self) -> broadcast::Receiver<LDataFrame> {
        self.state.tx.subscribe()
    }

    /// Sending is out of scope this cycle (design spec §Scope) — this is a
    /// typed, catchable stub, not a silent no-op.
    pub async fn send(&self, _frame: &LDataFrame) -> Result<(), BusError> {
        Err(BusError::NotImplemented)
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
                        let _ = state.tx.send(telegram);
                    }
                } else if req.sequence_counter == recv_seq.wrapping_sub(1) {
                    // Duplicate of the frame just processed (our own ACK
                    // was presumably lost) — ack again, discard (Tunnelling
                    // v01.07.01 AS §2.6.1).
                    send_ack(&state, req.sequence_counter, tunnelling::E_NO_ERROR).await;
                }
                // Any other sequence number: no ack, discard (§2.6.1).
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
        let waited =
            tokio::time::timeout(Duration::from_secs(10), state.heartbeat_notify.notified()).await;
        let status = state.heartbeat_reply.lock().await.take();
        if waited.is_ok() && status == Some(services::E_NO_ERROR) {
            return true;
        }
    }
    false
}
