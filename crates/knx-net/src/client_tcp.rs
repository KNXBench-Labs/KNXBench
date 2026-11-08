//! Optional KNXnet/IP tunnelling over one client-opened TCP stream (Core v01.06.02 AS §8.4.3).
//!
//! Selected only by an explicit `TunnelTransport::Tcp`; UDP stays the default
//! and nothing here falls back to it. Source rules, by clause and printed page
//! of Core v01.06.02 AS (C) and Tunnelling v01.07.01 AS (T):
//!
//! - C §8.4.3.1 p. 44, §8.4.3.4.3 p. 46: the client opens the TCP connection
//!   to the gateway's control endpoint; control and data share that stream.
//! - C §8.6.2.2 pp. 49–50: every HPAI this client sends, and the data HPAI of
//!   a successful CONNECT_RESPONSE, is the TCP Route Back HPAI
//!   `08 02 00 00 00 00 00 00`; anything else in the response is refused.
//! - C §7.8.1 Figure 35 p. 38: CONNECT_REQUEST is control HPAI, data HPAI,
//!   basic link-layer CRI, as over UDP.
//! - C §7.8.2 pp. 38–39, T §5.4.4 p. 30: CONNECT_RESPONSE within 10 s
//!   (CONNECT_REQUEST_TIMEOUT); a refusal is judged on its status alone; the
//!   server-assigned channel ID is kept as assigned, never zeroed.
//! - C §8.4.3.3 p. 45: stream framing, see [`crate::frame_stream`]. Every
//!   frame read past without being stored is reported as a
//!   `TunnelTapEvent::FrameSkipped` and counted; none is skipped silently.
//! - C §8.4.3.4.1 p. 46, T §2.6.2 p. 10: no TUNNELLING_ACK is sent, a
//!   received one is ignored, a received sequence counter is not evaluated
//!   (no duplicate suppression), and there is no 1 s wait or repetition.
//! - C §5.4 p. 14, §8.4.3.4.2 p. 46: the heartbeat stays: a
//!   CONNECTIONSTATE_REQUEST every 60 s, 10 s for each response, three
//!   repetitions after the original, then DISCONNECT_REQUEST.
//! - C §8.4.3.2–.3 pp. 44–46: with an active inner connection a receive
//!   stall, even inside a partial frame, does not close the stream; the
//!   timeout of the last inner connection closes it promptly; a received
//!   half-close closes the whole connection at once.
//! - C §5.5, §7.8.5–.6 p. 41: DISCONNECT_REQUEST is answered by
//!   DISCONNECT_RESPONSE, both ways, on the same stream.
//!
//! **Local policies, not KNX timings or rules** (the inspected clauses set
//! none): the TCP dial budget, the per-write budget, the cleanup budget for
//! the final control frame, the wait for DISCONNECT_RESPONSE, the queue
//! depth, the receive cap, and closing on a protocol version other than 1.0.
//! The 10 s byte-idle rule that applies before an inner connection exists
//! (C §8.4.3.3) is not represented on its own: the whole wait for
//! CONNECT_RESPONSE is bounded by the 10 s CONNECT_REQUEST_TIMEOUT.
//!
//! **Writing.** One writer task writes every frame whole, in queue order, so
//! no frame is ever interleaved with another. Admission to the queue and the
//! start of teardown happen under one lock (`Gate`): once teardown begins,
//! no TUNNELLING_REQUEST or heartbeat is admitted, every admitted frame not
//! yet taken by the writer is refused (it was never written, its caller gets
//! the closure cause), and the final DISCONNECT_REQUEST or DISCONNECT_RESPONSE
//! is queued behind only the frame being written at that moment. That frame
//! is finished whole, or the stream is closed. Heartbeats go ahead of queued
//! requests, so a full queue cannot delay one. A completed write means the
//! local TCP stack took every octet, not that the gateway or the bus received
//! it. A frame cut off by a closing stream has an uncertain fate and is
//! reported as a closure, never repeated. Nothing is replayed on a new stream.
//!
//! **Bounds** (production values, all local policy): a normal `disconnect()`
//! takes at most the cleanup budget (10 s) to get its DISCONNECT_REQUEST
//! written plus the DISCONNECT_RESPONSE wait (10 s), whatever the queue
//! holds. After a failed heartbeat, or the gateway's own DISCONNECT_REQUEST,
//! the stream is closed at most the cleanup budget later. A send waits for
//! room while the queue (16 requests) is full; every write that outlasts the
//! write budget (10 s) closes the stream. Teardown requests the abort of all
//! tasks synchronously; the runtime drops them, and with them the socket,
//! afterwards.
//!
//! Tested only against loopback peers in this crate. No interoperability
//! with a real gateway, no TCP capability discovery and no KNX IP Secure.

use std::collections::VecDeque;
use std::net::{SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, MutexGuard};
use std::time::Duration;

use knx_core::IndividualAddress;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::TcpStream;
use tokio::sync::{broadcast, oneshot, Mutex, Notify};
use tokio::task::AbortHandle;

use super::{
    wait_for_reply, BusError, HeartbeatTiming, TunnelClosure, TunnelEvent,
    DISCONNECT_RESPONSE_TIMEOUT, HEARTBEAT,
};
use crate::cemi::{self, ApplicationService, Destination, LDataFrame};
use crate::core::hpai::TCP_ROUTE_BACK;
use crate::core::services;
use crate::frame;
use crate::frame_stream::{SkipReason, StreamDecoder, StreamError, StreamItem};
use crate::tunnelling;
use crate::wire_tap::{TunnelTapEvent, WireDirection, WireTap};

/// C §5.2 / §7.8.2: CONNECT_REQUEST_TIMEOUT, the wait for CONNECT_RESPONSE.
const CONNECT_RESPONSE_TIMEOUT: Duration = Duration::from_secs(10);

/// Local policy, not a KNX timing: how long opening the TCP connection may take.
const DIAL_BUDGET: Duration = Duration::from_secs(10);

/// Local policy, not a KNX timing: how long one write to the local TCP stack
/// may take. A write that outlasts it closes the stream.
const WRITE_BUDGET: Duration = Duration::from_secs(10);

/// Local policy, not a KNX timing: from the start of teardown, how long the
/// final control frame (and the frame being written before it) may take to
/// be written before the stream is closed without it.
const CLEANUP_BUDGET: Duration = Duration::from_secs(10);

/// Local policy: the largest frame stored. Larger well-formed frames are
/// skipped, not refused (C §8.4.3.3); this is no limit on the wire.
pub(super) const FRAME_CAP: usize = 1024;

/// Octets read from the socket at a time; skipping uses no more than this.
const READ_CHUNK: usize = 512;

/// Local policy: TUNNELLING_REQUESTs queued for the writer before a sender waits.
pub(super) const OUTBOX_DEPTH: usize = 16;

/// Every timing and size a TCP tunnel uses; tests shorten them.
#[derive(Debug, Clone, Copy)]
pub(super) struct TcpPolicy {
    pub(super) heartbeat: HeartbeatTiming,
    pub(super) connect_response: Duration,
    pub(super) dial: Duration,
    pub(super) write: Duration,
    pub(super) cleanup: Duration,
    pub(super) disconnect_response: Duration,
    pub(super) frame_cap: usize,
}

/// The values every non-test caller gets.
pub(super) const PRODUCTION: TcpPolicy = TcpPolicy {
    heartbeat: HEARTBEAT,
    connect_response: CONNECT_RESPONSE_TIMEOUT,
    dial: DIAL_BUDGET,
    write: WRITE_BUDGET,
    cleanup: CLEANUP_BUDGET,
    disconnect_response: DISCONNECT_RESPONSE_TIMEOUT,
    frame_cap: FRAME_CAP,
};

/// Test-only control over the real writer task. The writer reports every
/// frame it takes on `taken`. For a frame of `pause_service` (every frame
/// when `None`) it first writes `split` octets to the socket, reports, then
/// waits for one permit before writing the rest; other frames pass. The
/// wait is outside the write budget, so a held frame models a writer stuck
/// for as long as the test chooses.
#[cfg(test)]
pub(super) struct WriterGate {
    pub(super) split: usize,
    pub(super) pause_service: Option<u16>,
    pub(super) taken: tokio::sync::mpsc::UnboundedSender<Vec<u8>>,
    pub(super) permits: tokio::sync::Semaphore,
}

/// Test hooks a tunnel is built with; empty outside tests.
#[derive(Default)]
pub(super) struct Hooks {
    #[cfg(test)]
    pub(super) writer_gate: Option<Arc<WriterGate>>,
}

/// The services a TCP tunnel reads; every other one is skipped unread.
fn handled_service(service: u16) -> bool {
    matches!(
        service,
        services::CONNECT_RESPONSE
            | services::CONNECTIONSTATE_RESPONSE
            | services::DISCONNECT_REQUEST
            | services::DISCONNECT_RESPONSE
            | tunnelling::TUNNELLING_REQUEST
            | tunnelling::TUNNELLING_ACK
    )
}

/// What a queued frame is, which decides how teardown treats it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    /// A TUNNELLING_REQUEST; counts against [`OUTBOX_DEPTH`].
    Request,
    /// A CONNECTIONSTATE_REQUEST; queued ahead of requests.
    Heartbeat,
    /// The last control frame: DISCONNECT_REQUEST or DISCONNECT_RESPONSE.
    Final,
}

/// The writer's answer for one frame: written whole, or why not.
type Written = Result<(), TunnelClosure>;

/// A frame for the writer, and where to report its fate.
struct Outgoing {
    frame: Vec<u8>,
    kind: FrameKind,
    written: oneshot::Sender<Written>,
}

/// Everything that orders admission against teardown, under one lock.
#[derive(Default)]
struct Gate {
    /// The first cause that ended the tunnel; `None` while it is open.
    closure: Option<TunnelClosure>,
    /// The stream is torn down: nothing more is written.
    finished: bool,
    /// Frames not yet taken by the writer, in write order.
    queue: VecDeque<Outgoing>,
    /// How many of `queue` are requests.
    queued_requests: usize,
    /// The next TUNNELLING_REQUEST sequence counter, taken at admission so
    /// counters go out in the order they were taken.
    next_sequence: u8,
    /// This client's DISCONNECT_REQUEST has been queued; it is sent once.
    disconnect_requested: bool,
    /// The gateway sent its own DISCONNECT_REQUEST for this channel.
    gateway_disconnected: bool,
}

/// What starting a teardown did.
struct Teardown {
    /// The kept closure cause.
    kept: TunnelClosure,
    /// Whether the kept cause is the one this teardown asked for.
    first: bool,
    /// The writer's answer for the final frame this teardown queued.
    done: Option<oneshot::Receiver<Written>>,
    /// This client's DISCONNECT_REQUEST is queued by now, by anyone.
    requested: bool,
}

/// The final control frame a teardown queues, when it is not moot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Last {
    /// This client's DISCONNECT_REQUEST, unless already queued or moot.
    DisconnectRequest,
    /// The answer to the gateway's DISCONNECT_REQUEST.
    DisconnectResponse,
}

/// The tasks that own the stream, and whether they have been told to end.
#[derive(Default)]
struct Tasks {
    finished: bool,
    handles: Vec<AbortHandle>,
}

struct TcpState {
    channel_id: u8,
    assigned_address: IndividualAddress,
    tx: broadcast::Sender<TunnelEvent>,
    policy: TcpPolicy,
    /// Observes complete frames and connection events; never alters them.
    tap: Option<WireTap>,
    gate: std::sync::Mutex<Gate>,
    /// Wakes the writer: a frame was queued.
    ready: Notify,
    /// Wakes senders waiting for room: a request left the queue, or teardown began.
    room: Notify,
    /// Why the stream itself was torn down; the first teardown is kept.
    stream_end: std::sync::Mutex<Option<TunnelClosure>>,
    closed_announced: AtomicBool,
    heartbeat_reply: Mutex<Option<u8>>,
    heartbeat_notify: Notify,
    disconnect_reply: Mutex<Option<u8>>,
    disconnect_notify: Notify,
    tasks: std::sync::Mutex<Tasks>,
    decode_failures: AtomicU64,
    skipped_frames: AtomicU64,
    #[cfg(test)]
    writer_gate: Option<Arc<WriterGate>>,
}

/// Reports one skipped frame to `tap`.
fn report_skip(tap: &Option<WireTap>, service_type: u16, total_length: u16, reason: SkipReason) {
    if let Some(tap) = tap {
        tap.event(TunnelTapEvent::FrameSkipped {
            service_type,
            total_length,
            reason,
        });
    }
}

impl TcpState {
    fn tap_event(&self, event: TunnelTapEvent) {
        if let Some(tap) = &self.tap {
            tap.event(event);
        }
    }

    fn gate(&self) -> MutexGuard<'_, Gate> {
        self.gate
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    fn closure(&self) -> Option<TunnelClosure> {
        self.gate().closure
    }

    fn is_finished(&self) -> bool {
        self.gate().finished
    }

    fn disconnect_requested(&self) -> bool {
        self.gate().disconnect_requested
    }

    fn gateway_disconnected(&self) -> bool {
        self.gate().gateway_disconnected
    }

    fn announce_closed(&self) {
        if !self.closed_announced.swap(true, Ordering::SeqCst) {
            let _ = self.tx.send(TunnelEvent::Closed);
        }
    }

    /// A heartbeat or `disconnect()` still waiting stops waiting.
    fn wake_waiters(&self) {
        self.heartbeat_notify.notify_one();
        self.disconnect_notify.notify_one();
    }

    fn stream_end(&self) -> Option<TunnelClosure> {
        *self
            .stream_end
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// Records `cause` under the gate unless an earlier one is kept, and
    /// refuses every queued frame that is not a final control frame: none of
    /// them has been taken by the writer, so none was written. Returns the
    /// kept cause and whether it is `cause`.
    fn close_gate(
        gate: &mut Gate,
        state: &TcpState,
        cause: TunnelClosure,
    ) -> (TunnelClosure, bool) {
        let first = gate.closure.is_none();
        if first {
            gate.closure = Some(cause);
            state.tap_event(TunnelTapEvent::Closed(cause));
        }
        let kept = gate.closure.unwrap_or(cause);
        let mut finals = VecDeque::new();
        for item in gate.queue.drain(..) {
            if item.kind == FrameKind::Final {
                finals.push_back(item);
            } else {
                let _ = item.written.send(Err(kept));
            }
        }
        gate.queue = finals;
        gate.queued_requests = 0;
        (kept, first)
    }

    /// Begins teardown for `cause` in one step under the gate (see the module
    /// docs): no request or heartbeat is admitted after this, queued ones are
    /// refused, and `last` is queued as the final control frame unless it is
    /// moot (stream torn down; DISCONNECT_REQUEST already queued, or the
    /// gateway already disconnected). Never waits for queue room.
    fn begin_teardown(&self, cause: TunnelClosure, last: Last) -> Teardown {
        let result = {
            let mut gate = self.gate();
            let (kept, first) = Self::close_gate(&mut gate, self, cause);
            let frame = match last {
                Last::DisconnectRequest
                    if !gate.finished
                        && !gate.disconnect_requested
                        && !gate.gateway_disconnected =>
                {
                    gate.disconnect_requested = true;
                    Some(self.disconnect_request())
                }
                Last::DisconnectResponse if !gate.finished => {
                    gate.gateway_disconnected = true;
                    Some(frame::encode_frame(
                        services::DISCONNECT_RESPONSE,
                        &services::encode_disconnect_response(
                            self.channel_id,
                            services::E_NO_ERROR,
                        ),
                    ))
                }
                Last::DisconnectResponse => {
                    gate.gateway_disconnected = true;
                    None
                }
                _ => None,
            };
            let done = frame.map(|frame| {
                let (written, done) = oneshot::channel();
                gate.queue.push_back(Outgoing {
                    frame,
                    kind: FrameKind::Final,
                    written,
                });
                done
            });
            Teardown {
                kept,
                first,
                done,
                requested: gate.disconnect_requested,
            }
        };
        self.ready.notify_one();
        self.room.notify_waiters();
        result
    }

    /// Tears the stream down for `cause`: keeps the first closure cause,
    /// refuses every frame still queued, tells subscribers once, wakes every
    /// waiter and requests the abort of every task. The abort is requested
    /// synchronously; the runtime drops the tasks, and with them the socket,
    /// afterwards. Idempotent.
    fn finish(&self, cause: TunnelClosure) -> TunnelClosure {
        let kept = {
            let mut gate = self.gate();
            let (kept, _) = Self::close_gate(&mut gate, self, cause);
            gate.finished = true;
            for item in gate.queue.drain(..) {
                let _ = item.written.send(Err(kept));
            }
            kept
        };
        self.room.notify_waiters();
        self.stream_end
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_insert(cause);
        self.announce_closed();
        self.wake_waiters();
        let handles = {
            let mut tasks = self
                .tasks
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            tasks.finished = true;
            std::mem::take(&mut tasks.handles)
        };
        for handle in handles {
            handle.abort();
        }
        kept
    }

    /// Keeps `handle` for teardown, or requests its abort at once if that has
    /// begun: a task may end the stream before its siblings are registered.
    fn register(&self, handle: AbortHandle) {
        let mut tasks = self
            .tasks
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if tasks.finished {
            handle.abort();
        } else {
            tasks.handles.push(handle);
        }
    }

    /// The cause to report for a frame whose writer went away.
    fn write_refusal(&self) -> TunnelClosure {
        self.closure().unwrap_or(TunnelClosure::StreamWriteFailed)
    }

    /// Admits one TUNNELLING_REQUEST around `cemi`, taking the next sequence
    /// counter. Waits for room while [`OUTBOX_DEPTH`] requests are queued;
    /// refuses at once, also while waiting, once teardown has begun.
    /// Cancelling it before it returns admits nothing.
    async fn admit_request(&self, cemi: &[u8]) -> Result<oneshot::Receiver<Written>, BusError> {
        loop {
            let room = self.room.notified();
            tokio::pin!(room);
            room.as_mut().enable();
            {
                let mut gate = self.gate();
                if let Some(cause) = gate.closure {
                    return Err(BusError::TunnelClosed(cause));
                }
                if gate.queued_requests < OUTBOX_DEPTH {
                    let sequence = gate.next_sequence;
                    let frame = encode_checked(
                        tunnelling::TUNNELLING_REQUEST,
                        &tunnelling::encode_tunnelling_request(self.channel_id, sequence, cemi),
                    )?;
                    gate.next_sequence = sequence.wrapping_add(1);
                    let (written, done) = oneshot::channel();
                    gate.queue.push_back(Outgoing {
                        frame,
                        kind: FrameKind::Request,
                        written,
                    });
                    gate.queued_requests += 1;
                    drop(gate);
                    self.ready.notify_one();
                    return Ok(done);
                }
            }
            room.await;
        }
    }

    /// Admits heartbeat `attempt` ahead of every queued request, so a full
    /// queue cannot delay it; refused once teardown has begun. The attempt is
    /// reported before the writer can take the frame.
    fn admit_heartbeat(
        &self,
        frame: Vec<u8>,
        attempt: u8,
    ) -> Result<oneshot::Receiver<Written>, TunnelClosure> {
        let done = {
            let mut gate = self.gate();
            if let Some(cause) = gate.closure {
                return Err(cause);
            }
            self.tap_event(TunnelTapEvent::HeartbeatAttempt { attempt });
            let (written, done) = oneshot::channel();
            gate.queue.push_front(Outgoing {
                frame,
                kind: FrameKind::Heartbeat,
                written,
            });
            done
        };
        self.ready.notify_one();
        Ok(done)
    }

    /// The writer's answer for an admitted frame.
    async fn written(&self, done: oneshot::Receiver<Written>) -> Written {
        done.await.unwrap_or_else(|_| Err(self.write_refusal()))
    }

    /// Waits for a final control frame within the cleanup budget, which runs
    /// from the start of teardown and covers the frame being written before
    /// it. On expiry the stream is closed without it.
    async fn final_written(&self, done: Option<oneshot::Receiver<Written>>) -> bool {
        let Some(done) = done else {
            return false;
        };
        match tokio::time::timeout(self.policy.cleanup, done).await {
            Ok(Ok(Ok(()))) => true,
            Ok(_) => false,
            Err(_) => {
                self.tap_event(TunnelTapEvent::SocketError {
                    operation: "send",
                    reason: "the final control frame was not written within the local cleanup \
                             budget"
                        .to_string(),
                });
                self.finish(TunnelClosure::StreamWriteFailed);
                false
            }
        }
    }

    fn disconnect_request(&self) -> Vec<u8> {
        frame::encode_frame(
            services::DISCONNECT_REQUEST,
            &services::encode_disconnect_request_with_hpai(self.channel_id, TCP_ROUTE_BACK),
        )
    }

    /// Ends the inner connection for a cause this client detected (a failed
    /// heartbeat): queues DISCONNECT_REQUEST as the final frame, then closes
    /// the stream, since that connection was its last (C §8.4.3.3), at most
    /// the cleanup budget later. A tunnel already closing for another cause
    /// is left to that cause's owner.
    async fn terminate(&self, cause: TunnelClosure) {
        let teardown = self.begin_teardown(cause, Last::DisconnectRequest);
        if !teardown.first {
            return;
        }
        self.announce_closed();
        self.wake_waiters();
        self.final_written(teardown.done).await;
        self.finish(teardown.kept);
    }

    /// Counts and reports one skipped frame.
    fn record_skip(&self, service_type: u16, total_length: u16, reason: SkipReason) {
        self.skipped_frames.fetch_add(1, Ordering::Relaxed);
        report_skip(&self.tap, service_type, total_length, reason);
    }

    /// Acts on one decoded unit. `false` once the stream has been torn down.
    async fn handle(&self, item: StreamItem) -> bool {
        let bytes = match item {
            StreamItem::Skipped {
                service_type,
                total_length,
                reason,
            } => {
                self.record_skip(service_type, total_length, reason);
                return true;
            }
            StreamItem::Frame(bytes) => bytes,
        };
        if let Some(tap) = &self.tap {
            tap.datagram(WireDirection::Received, &bytes);
        }
        // Cannot fail: the stream decoder checked the header and the length.
        let Ok((header, body)) = frame::decode_frame(&bytes) else {
            return true;
        };
        match header.service_type {
            tunnelling::TUNNELLING_REQUEST => self.deliver(body),
            // C §8.4.3.4.1, T §2.6.2: never sent over TCP; one received is ignored.
            tunnelling::TUNNELLING_ACK => {}
            services::CONNECTIONSTATE_RESPONSE => {
                if let Ok(response) = services::decode_connectionstate_response(body) {
                    if response.channel_id == self.channel_id {
                        *self.heartbeat_reply.lock().await = Some(response.status);
                        self.heartbeat_notify.notify_one();
                    }
                }
            }
            services::DISCONNECT_RESPONSE => {
                if let Ok(response) = services::decode_disconnect_response(body) {
                    if response.channel_id == self.channel_id && self.disconnect_requested() {
                        *self.disconnect_reply.lock().await = Some(response.status);
                        self.disconnect_notify.notify_one();
                        // C §5.5: the channel's final termination, and it was
                        // this stream's only one.
                        self.finish(TunnelClosure::ClientDisconnect);
                        return false;
                    }
                }
            }
            services::DISCONNECT_REQUEST if body.first() == Some(&self.channel_id) => {
                self.gateway_disconnect().await;
                return false;
            }
            // Another channel's disconnect or a CONNECT_RESPONSE after the handshake: ignored.
            _ => {}
        }
        true
    }

    /// Forwards one TUNNELLING_REQUEST body of this channel to subscribers.
    fn deliver(&self, body: &[u8]) {
        let Ok(request) = tunnelling::decode_tunnelling_request(body) else {
            return;
        };
        if request.channel_id != self.channel_id {
            self.tap_event(TunnelTapEvent::ReceiveDiscarded {
                sequence: request.sequence_counter,
                reason: "foreign channel",
            });
            return;
        }
        // C §8.4.3.4.1: the sequence counter is not evaluated over TCP, so
        // every request is delivered and none is acknowledged. Forwarded
        // under the gate lock: no telegram follows `Closed`.
        let gate = self.gate();
        if gate.closure.is_some() {
            drop(gate);
            self.tap_event(TunnelTapEvent::ReceiveDiscarded {
                sequence: request.sequence_counter,
                reason: "tunnel already closed",
            });
            return;
        }
        match cemi::decode_l_data(request.cemi) {
            Ok(telegram) => {
                let _ = self.tx.send(TunnelEvent::Telegram(telegram));
            }
            Err(error) => {
                self.tap_event(TunnelTapEvent::CemiDecodeFailed {
                    reason: error.to_string(),
                });
                if self.decode_failures.fetch_add(1, Ordering::Relaxed) == 0 {
                    eprintln!(
                        "TUNNELLING_REQUEST: cEMI frame failed to decode: {error} \
                         (further failures on this connection are counted, not printed; \
                         see decode_failure_count)"
                    );
                }
            }
        }
    }

    /// The gateway's own DISCONNECT_REQUEST (C §5.5): answered on the same
    /// stream as the final frame while it still takes one, then the stream
    /// is closed, at most the cleanup budget later.
    async fn gateway_disconnect(&self) {
        let teardown =
            self.begin_teardown(TunnelClosure::GatewayDisconnect, Last::DisconnectResponse);
        self.announce_closed();
        self.wake_waiters();
        self.final_written(teardown.done).await;
        self.finish(TunnelClosure::GatewayDisconnect);
    }

    /// What `disconnect()` reports when no DISCONNECT_RESPONSE came.
    fn ended_without_reply(&self) -> Result<(), BusError> {
        if self.gateway_disconnected() {
            return Ok(());
        }
        match self.stream_end() {
            // C §8.4.3.5: closing the stream ends every connection on it,
            // so the gateway closing it confirms the termination.
            Some(TunnelClosure::PeerClosed) => Ok(()),
            Some(TunnelClosure::ClientDisconnect) | None => match self.closure() {
                Some(cause) if self.is_finished() => Err(BusError::TunnelClosed(cause)),
                _ => Err(BusError::Timeout),
            },
            Some(cause) => Err(BusError::TunnelClosed(cause)),
        }
    }
}

/// A frame of `service` around `body`, refused when the 16-bit total
/// length (C §2.3) cannot state its size instead of being cut short.
fn encode_checked(service: u16, body: &[u8]) -> Result<Vec<u8>, BusError> {
    if body.len() > usize::from(u16::MAX) - usize::from(frame::HEADER_SIZE_10) {
        return Err(BusError::Protocol(format!(
            "a {}-octet body does not fit a KNXnet/IP frame",
            body.len()
        )));
    }
    Ok(frame::encode_frame(service, body))
}

/// The sole reader of the stream. Applies no receive timeout: with the
/// inner connection active a stall must not close the stream (C §8.4.3.3);
/// the heartbeat is what ends a stream that has stopped answering.
async fn reader_loop(
    state: Arc<TcpState>,
    mut read: OwnedReadHalf,
    mut decoder: StreamDecoder,
    pending: Vec<StreamItem>,
) {
    for item in pending {
        if !state.handle(item).await {
            return;
        }
    }
    let mut buf = [0u8; READ_CHUNK];
    let mut items = Vec::new();
    loop {
        let n = match read.read(&mut buf).await {
            Ok(0) => {
                // C §8.4.3.3: a received half-close ends the whole connection
                // at once, also inside a partial header or body.
                state.finish(TunnelClosure::PeerClosed);
                return;
            }
            Ok(n) => n,
            Err(error) => {
                state.tap_event(TunnelTapEvent::SocketError {
                    operation: "receive",
                    reason: error.to_string(),
                });
                state.finish(TunnelClosure::SocketFailed);
                return;
            }
        };
        let outcome = decoder.push(&buf[..n], &mut items);
        for item in items.drain(..) {
            if !state.handle(item).await {
                return;
            }
        }
        if let Err(error) = outcome {
            state.tap_event(TunnelTapEvent::SocketError {
                operation: "receive",
                reason: error.to_string(),
            });
            state.finish(stream_error_closure(error));
            return;
        }
    }
}

fn stream_error_closure(error: StreamError) -> TunnelClosure {
    match error {
        StreamError::UnsupportedVersion(version) => TunnelClosure::UnsupportedVersion { version },
        StreamError::BadHeaderLength(_) | StreamError::TotalLengthBelowHeader(_) => {
            TunnelClosure::MalformedStream
        }
    }
}

/// The next queued frame, waiting until there is one.
async fn next_frame(state: &TcpState) -> Outgoing {
    loop {
        let ready = state.ready.notified();
        tokio::pin!(ready);
        ready.as_mut().enable();
        let taken = {
            let mut gate = state.gate();
            let taken = gate.queue.pop_front();
            if taken.as_ref().map(|item| item.kind) == Some(FrameKind::Request) {
                gate.queued_requests -= 1;
            }
            taken
        };
        if let Some(item) = taken {
            state.room.notify_waiters();
            return item;
        }
        ready.await;
    }
}

/// Writes `part` within the write budget.
async fn write_part(
    state: &TcpState,
    write: &mut OwnedWriteHalf,
    part: &[u8],
) -> Result<(), String> {
    match tokio::time::timeout(state.policy.write, write.write_all(part)).await {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(error.to_string()),
        Err(_) => Err("the frame was not written within the local write budget".to_string()),
    }
}

/// Writes one frame whole; with a test gate, in two parts around the gate.
async fn write_whole(
    state: &TcpState,
    write: &mut OwnedWriteHalf,
    frame: &[u8],
) -> Result<(), String> {
    #[cfg(test)]
    let frame = match &state.writer_gate {
        Some(gate)
            if gate
                .pause_service
                .is_none_or(|service| frame.get(2..4) == Some(&service.to_be_bytes()[..])) =>
        {
            let split = gate.split.min(frame.len());
            write_part(state, write, &frame[..split]).await?;
            let _ = gate.taken.send(frame.to_vec());
            gate.permits
                .acquire()
                .await
                .map_err(|error| error.to_string())?
                .forget();
            &frame[split..]
        }
        Some(gate) => {
            let _ = gate.taken.send(frame.to_vec());
            frame
        }
        None => frame,
    };
    write_part(state, write, frame).await
}

/// The sole writer of the stream: takes each queued frame in order and
/// writes it whole. A frame whose write fails or outlasts the write budget
/// closes the stream; it is never retried.
async fn writer_loop(state: Arc<TcpState>, mut write: OwnedWriteHalf) {
    loop {
        let Outgoing { frame, written, .. } = next_frame(&state).await;
        // Recorded first, so the tape never shows an answer before its request.
        if let Some(tap) = &state.tap {
            tap.datagram(WireDirection::Sent, &frame);
        }
        if let Err(reason) = write_whole(&state, &mut write, &frame).await {
            state.tap_event(TunnelTapEvent::SocketError {
                operation: "send",
                reason,
            });
            let kept = state.finish(TunnelClosure::StreamWriteFailed);
            let _ = written.send(Err(kept));
            return;
        }
        let _ = written.send(Ok(()));
    }
}

/// C §5.4, §8.4.3.4.2: a CONNECTIONSTATE_REQUEST every interval; on no
/// reply, or an error status, within the response time, three repetitions,
/// then disconnect.
async fn heartbeat_loop(state: Arc<TcpState>) {
    let mut interval = tokio::time::interval(state.policy.heartbeat.interval);
    interval.tick().await; // the first tick is immediate; the first heartbeat is one interval in
    loop {
        interval.tick().await;
        if state.closure().is_some() {
            return;
        }
        if !heartbeat_with_retries(&state).await {
            state.terminate(TunnelClosure::HeartbeatFailed).await;
            return;
        }
    }
}

async fn heartbeat_with_retries(state: &TcpState) -> bool {
    for attempt in 1..=4u8 {
        if state.closure().is_some() {
            return false;
        }
        *state.heartbeat_reply.lock().await = None;
        let request = frame::encode_frame(
            services::CONNECTIONSTATE_REQUEST,
            &services::encode_connectionstate_request_with_hpai(state.channel_id, TCP_ROUTE_BACK),
        );
        let Ok(done) = state.admit_heartbeat(request, attempt) else {
            return false;
        };
        if state.written(done).await.is_err() {
            return false;
        }
        let deadline = tokio::time::Instant::now() + state.policy.heartbeat.response_timeout;
        let status = wait_for_reply(
            &state.heartbeat_notify,
            &state.heartbeat_reply,
            deadline,
            |_| true,
            || state.closure().is_some(),
        )
        .await;
        if status == Some(services::E_NO_ERROR) {
            return true;
        }
    }
    false
}

/// Decode one handshake read without losing completed skip notices on a malformed suffix.
pub(super) fn decode_handshake_batch(
    decoder: &mut StreamDecoder,
    bytes: &[u8],
    items: &mut Vec<StreamItem>,
    tap: &Option<WireTap>,
) -> Result<(), BusError> {
    if let Err(error) = decoder.push(bytes, items) {
        for item in items.drain(..) {
            if let StreamItem::Skipped {
                service_type,
                total_length,
                reason,
            } = item
            {
                report_skip(tap, service_type, total_length, reason);
            }
        }
        return Err(BusError::Protocol(error.to_string()));
    }
    Ok(())
}

/// One TCP tunnel: the stream's reader, writer and heartbeat tasks and
/// their shared state. Dropping it requests their abort at once; the
/// runtime then drops them and releases the socket.
pub(super) struct TcpTunnel {
    state: Arc<TcpState>,
}

impl TcpTunnel {
    /// Opens the stream and runs the connect handshake; a failure before a
    /// channel exists is reported to `tap` too. Cancelling this future
    /// drops the stream, whether or not the gateway has assigned a channel.
    pub(super) async fn connect(
        gateway: SocketAddrV4,
        policy: TcpPolicy,
        tap: Option<WireTap>,
    ) -> Result<Self, BusError> {
        Self::connect_with(gateway, policy, tap, Hooks::default()).await
    }

    /// [`Self::connect`] with test hooks.
    pub(super) async fn connect_with(
        gateway: SocketAddrV4,
        policy: TcpPolicy,
        tap: Option<WireTap>,
        hooks: Hooks,
    ) -> Result<Self, BusError> {
        if let Some(tap) = &tap {
            tap.event(TunnelTapEvent::Connecting { gateway });
        }
        let result = Self::handshake(gateway, policy, tap.clone(), hooks).await;
        if let (Some(tap), Err(error)) = (&tap, &result) {
            tap.event(TunnelTapEvent::ConnectFailed {
                reason: error.to_string(),
            });
        }
        result
    }

    async fn handshake(
        gateway: SocketAddrV4,
        policy: TcpPolicy,
        tap: Option<WireTap>,
        hooks: Hooks,
    ) -> Result<Self, BusError> {
        let stream = tokio::time::timeout(policy.dial, TcpStream::connect(SocketAddr::V4(gateway)))
            .await
            .map_err(|_| BusError::Timeout)?
            .map_err(BusError::Io)?;
        // Local choice: small control frames go out without Nagle's delay.
        stream.set_nodelay(true).map_err(BusError::Io)?;
        let (mut read, mut write) = stream.into_split();

        let cri = tunnelling::TunnelCri {
            layer: tunnelling::TUNNEL_LINKLAYER,
        }
        .encode();
        let request = frame::encode_frame(
            services::CONNECT_REQUEST,
            &services::encode_connect_request_with_hpai(TCP_ROUTE_BACK, TCP_ROUTE_BACK, &cri),
        );
        if let Some(tap) = &tap {
            tap.datagram(WireDirection::Sent, &request);
        }
        tokio::time::timeout(policy.write, write.write_all(&request))
            .await
            .map_err(|_| BusError::Timeout)?
            .map_err(BusError::Io)?;

        // The first stored frame must be the CONNECT_RESPONSE; skipped ones
        // (unknown services, above the cap) are read past, each reported in
        // its place. Whatever follows the response in the same reads stays
        // queued for the reader task.
        let deadline = tokio::time::Instant::now() + policy.connect_response;
        let mut decoder = StreamDecoder::new(policy.frame_cap, handled_service);
        let mut items: Vec<StreamItem> = Vec::new();
        let mut skipped = 0u64;
        let mut buf = [0u8; READ_CHUNK];
        let (response, pending) = loop {
            let position = items
                .iter()
                .position(|item| matches!(item, StreamItem::Frame(_)));
            let pending = position.map(|position| items.split_off(position + 1));
            let response = match pending {
                Some(_) => match items.pop() {
                    Some(StreamItem::Frame(response)) => Some(response),
                    _ => unreachable!("the position found holds a frame"),
                },
                None => None,
            };
            for item in items.drain(..) {
                if let StreamItem::Skipped {
                    service_type,
                    total_length,
                    reason,
                } = item
                {
                    skipped += 1;
                    report_skip(&tap, service_type, total_length, reason);
                }
            }
            if let (Some(response), Some(pending)) = (response, pending) {
                break (response, pending);
            }
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                return Err(BusError::Timeout);
            }
            let n = tokio::time::timeout(remaining, read.read(&mut buf))
                .await
                .map_err(|_| BusError::Timeout)?
                .map_err(BusError::Io)?;
            if n == 0 {
                return Err(BusError::Io(std::io::Error::new(
                    std::io::ErrorKind::UnexpectedEof,
                    "the gateway closed the TCP connection before its CONNECT_RESPONSE",
                )));
            }
            decode_handshake_batch(&mut decoder, &buf[..n], &mut items, &tap)?;
        };
        if let Some(tap) = &tap {
            tap.datagram(WireDirection::Received, &response);
        }
        let (channel_id, assigned_address) = decode_connect_response(&response)?;
        if let Some(tap) = &tap {
            tap.event(TunnelTapEvent::Connected {
                channel_id,
                assigned_address,
            });
        }

        #[cfg(not(test))]
        let Hooks {} = hooks;
        let (tx, _rx) = broadcast::channel(64);
        let state = Arc::new(TcpState {
            channel_id,
            assigned_address,
            tx,
            policy,
            tap,
            gate: std::sync::Mutex::new(Gate::default()),
            ready: Notify::new(),
            room: Notify::new(),
            stream_end: std::sync::Mutex::new(None),
            closed_announced: AtomicBool::new(false),
            heartbeat_reply: Mutex::new(None),
            heartbeat_notify: Notify::new(),
            disconnect_reply: Mutex::new(None),
            disconnect_notify: Notify::new(),
            tasks: std::sync::Mutex::new(Tasks::default()),
            decode_failures: AtomicU64::new(0),
            skipped_frames: AtomicU64::new(skipped),
            #[cfg(test)]
            writer_gate: hooks.writer_gate,
        });
        let writer = tokio::spawn(writer_loop(state.clone(), write));
        state.register(writer.abort_handle());
        let reader = tokio::spawn(reader_loop(state.clone(), read, decoder, pending));
        state.register(reader.abort_handle());
        let heartbeat = tokio::spawn(heartbeat_loop(state.clone()));
        state.register(heartbeat.abort_handle());
        Ok(Self { state })
    }

    pub(super) fn assigned_address(&self) -> IndividualAddress {
        self.state.assigned_address
    }

    pub(super) fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.state.tx.subscribe()
    }

    pub(super) fn decode_failure_count(&self) -> u64 {
        self.state.decode_failures.load(Ordering::Relaxed)
    }

    pub(super) fn skipped_frame_count(&self) -> u64 {
        self.state.skipped_frames.load(Ordering::Relaxed)
    }

    pub(super) fn closure(&self) -> Option<TunnelClosure> {
        self.state.closure()
    }

    /// Queues one `L_Data.req` TUNNELLING_REQUEST and returns once the local
    /// TCP stack took it whole; there is no TUNNELLING_ACK to wait for and
    /// no repetition (C §8.4.3.4.1). Cancelled while waiting for queue room,
    /// nothing was queued. Cancelled after it was queued, the frame is still
    /// written whole unless teardown begins first, in which case it is never
    /// written. It is never sent twice.
    pub(super) async fn send_frame(
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
        let done = self.state.admit_request(&cemi_bytes).await?;
        self.state
            .written(done)
            .await
            .map_err(BusError::TunnelClosed)
    }

    /// Graceful disconnect (C §5.5, §8.4.3.2): refuses every queued request
    /// and heartbeat, sends DISCONNECT_REQUEST behind only the frame being
    /// written, waits for the matching DISCONNECT_RESPONSE, then closes the
    /// stream. Bounded by the cleanup budget plus the response budget, whatever
    /// the queue held. The stream is closed on every outcome, also when this
    /// future is cancelled, since that drops the tunnel.
    pub(super) async fn disconnect(self) -> Result<(), BusError> {
        self.close_gracefully().await
    }

    /// [`Self::disconnect`]'s whole path without taking ownership, so a test
    /// can race it against senders that still borrow the tunnel.
    #[cfg(test)]
    pub(super) async fn disconnect_shared(&self) -> Result<(), BusError> {
        self.close_gracefully().await
    }

    async fn close_gracefully(&self) -> Result<(), BusError> {
        let result = self.disconnect_inner().await;
        self.state.finish(TunnelClosure::ClientDisconnect);
        result
    }

    async fn disconnect_inner(&self) -> Result<(), BusError> {
        let state = &self.state;
        let teardown =
            state.begin_teardown(TunnelClosure::ClientDisconnect, Last::DisconnectRequest);
        state.announce_closed();
        state.wake_waiters();
        if state.gateway_disconnected() {
            return Ok(());
        }
        if !teardown.requested {
            // The stream was torn down before any request could be queued.
            return state.ended_without_reply();
        }
        // `None` while requested: a failed heartbeat queued the request and
        // closes the stream itself; only its outcome is awaited.
        if teardown.done.is_some() && !state.final_written(teardown.done).await {
            return state.ended_without_reply();
        }
        let deadline = tokio::time::Instant::now() + state.policy.disconnect_response;
        let reply = wait_for_reply(
            &state.disconnect_notify,
            &state.disconnect_reply,
            deadline,
            |_| true,
            || state.is_finished(),
        )
        .await;
        match reply {
            Some(services::E_NO_ERROR) => Ok(()),
            Some(status) => Err(BusError::Protocol(format!(
                "DISCONNECT_RESPONSE returned status {status:#04x}"
            ))),
            None => state.ended_without_reply(),
        }
    }
}

impl Drop for TcpTunnel {
    /// Safety net for a tunnel dropped without `disconnect()`: refuses every
    /// queued frame and requests the abort of every task before returning.
    /// The runtime drops the aborted tasks, and with them the socket, later;
    /// this does not wait for that.
    fn drop(&mut self) {
        self.state.finish(TunnelClosure::ClientDisconnect);
    }
}

/// Checks a TCP CONNECT_RESPONSE frame (C §7.8.2) and returns the channel
/// and the assigned individual address. A refusal is judged on its status
/// alone; success must carry the TCP Route Back data HPAI (C §8.6.2.2).
fn decode_connect_response(response: &[u8]) -> Result<(u8, IndividualAddress), BusError> {
    let (header, body) =
        frame::decode_frame(response).map_err(|e| BusError::Protocol(e.to_string()))?;
    if header.service_type != services::CONNECT_RESPONSE {
        return Err(BusError::Protocol(format!(
            "expected CONNECT_RESPONSE, got service type {:#06x}",
            header.service_type
        )));
    }
    let [channel_id, status, rest @ ..] = body else {
        return Err(BusError::Protocol(format!(
            "CONNECT_RESPONSE body of {} octets is too short",
            body.len()
        )));
    };
    if *status != services::E_NO_ERROR {
        return Err(BusError::ConnectionRefused(*status));
    }
    let Some((data_hpai, crd)) = rest.split_first_chunk::<8>() else {
        return Err(BusError::Protocol(
            "CONNECT_RESPONSE carries no complete data endpoint HPAI".to_string(),
        ));
    };
    if *data_hpai != TCP_ROUTE_BACK {
        return Err(BusError::Protocol(format!(
            "CONNECT_RESPONSE data endpoint {data_hpai:02x?} is not the TCP Route Back HPAI \
             {TCP_ROUTE_BACK:02x?} (Core v01.06.02 AS §8.6.2.2)"
        )));
    }
    let crd = tunnelling::TunnelCrd::decode(crd).map_err(|e| BusError::Protocol(e.to_string()))?;
    Ok((*channel_id, crd.individual_address))
}
