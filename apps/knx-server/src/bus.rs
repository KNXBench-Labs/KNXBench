// apps/knx-server/src/bus.rs
//! The testability seam between `knx-server` and `knx-net` (design spec
//! `docs/superpowers/specs/2026-09-11-group-monitor-design.md` §3 D6).
//!
//! `knx_net::TunnelClient` cannot be constructed directly and
//! `knx_net::BusConnection` is deliberately not `dyn`-safe (single
//! implementer, called directly — the right choice for `knx-net` itself).
//! That means `knx-server` cannot inject a fake by pointing a
//! `Box<dyn BusConnection>` at test code. So this module defines its own,
//! narrower trait pair — [`GatewayConnector`]/[`BusTunnel`] — exposing only
//! the tunnel operations a monitor session actually needs (D7: tunnelling
//! only, no discovery, no routing). [`RealConnector`]/[`RealTunnel`]
//! delegate to `knx-net`; [`fake::FakeConnector`]/[`fake::FakeTunnel`] (see
//! that module's own doc comment) stand in for tests, with no gateway and
//! no socket anywhere.
//!
//! T15 task 1 wired the seam and `AppState`'s fields; no route read them
//! yet. T15 task 2 (this revision) adds what actually runs on top of the
//! seam: [`TelegramBuffer`] (the capped, gap-accounted row store),
//! [`BusSession`] (the session shell — moved here from `domain.rs`'s
//! deliberately minimal placeholder, now that it has real behaviour: a
//! module built *around* a seam is a more natural home for that seam's
//! only consumer than a file that otherwise only shuffles `AppState`
//! plumbing) and the `tokio::spawn`ed drain task that turns a
//! `broadcast::Receiver<TunnelEvent>` into buffered [`TelegramRow`]s. No
//! HTTP route reads any of this yet either — that is task 3. Every test
//! below drives [`BusSession`] directly, no gateway, no socket, no HTTP.

use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::future::Future;
use std::net::SocketAddrV4;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

use knx_core::{
    DptRef, GroupAddress, GroupAddressDpt, GroupAddressStyle, GroupValue, IndividualAddress,
};
use knx_net::{ApplicationService, BusConnection, BusError, Destination, KnxNetIpClient};
use tokio::sync::{broadcast, oneshot};
use tokio::task::JoinHandle;

use crate::session_log;

/// What a monitor session needs from something that can open a tunnel.
/// `knx_net::KnxNetIpClient` is the production implementer via
/// [`RealConnector`]; tests use [`fake::FakeConnector`]. Manually boxed
/// futures, not `async-trait` — this crate already pulls in `axum`/`tokio`
/// directly, so there's no reason to add a proc-macro dependency for a
/// one-method trait, and `dyn` safety is exactly the point here, unlike in
/// `knx-net`'s own `BusConnection`.
pub trait GatewayConnector: Send + Sync {
    #[allow(clippy::type_complexity)]
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>;
}

/// What a monitor session needs from an open tunnel. Mirrors
/// `knx_net::TunnelClient`'s operations a session actually uses
/// (`assigned_address`, `subscribe`, `send` at `crates/knx-net/src/
/// client.rs:286,290,306`; the consuming `disconnect(self)` at :364).
pub trait BusTunnel: Send + Sync {
    fn assigned_address(&self) -> IndividualAddress;
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent>;
    fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>>;
    /// Takes `self: Box<Self>`, not `&mut self` — `&mut self` would
    /// silently weaken the real `TunnelClient::disconnect(self)` contract
    /// that the tunnel is gone afterwards. `Box<dyn BusTunnel>` cannot
    /// offer an unboxed consuming method, so this is the standard way to
    /// keep a genuinely consuming API through a trait object.
    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>>;
}

pub use knx_net::TunnelEvent;

/// Failure surfaced by [`GatewayConnector`]/[`BusTunnel`], or by a
/// server-side rule about how many sessions may exist at once. `Transport`
/// carries whatever `knx_net::BusError` reported (I/O, timeout, gateway
/// refusal, an unimplemented operation, a lower-layer codec error) —
/// everything this seam's production side (`knx-net`) can fail with. An
/// unparsable gateway address is deliberately *not* a variant here: that
/// is a caller mistake the HTTP route rejects before ever reaching a
/// `GatewayConnector`, not something the connector/tunnel themselves can
/// produce.
///
/// `AlreadyActive`/`NoActiveSession` are the server-side cases Task 1's
/// reviewer anticipated ("extend this enum when you need a session-level
/// case — do not introduce a second, parallel error type beside it") —
/// neither a `GatewayConnector` nor a `BusTunnel` ever produces them; the
/// route layer (`bus_routes.rs`) constructs them directly from
/// `AppState.bus_session`'s own state, then maps each to whichever status
/// code its endpoint's contract requires (design spec §4.3: `409` for the
/// three mutating routes, `404` for `GET /telegrams` — the same variant,
/// two different status codes, because the HTTP meaning of "no session"
/// depends on whether the request tried to read or to mutate, not on the
/// underlying fact).
#[derive(Debug)]
pub enum BusSessionError {
    Transport(BusError),
    /// `POST /api/bus/monitor/start` while `AppState.bus_session` already
    /// holds a session whose drain task is still running (design spec
    /// §4.1) — never a silent replacement.
    AlreadyActive,
    /// `AppState.bus_session` is `None` (or, for [`BusSession::send`],
    /// the session's tunnel has already been taken by its own teardown —
    /// see that method's doc comment) when a route needed one.
    NoActiveSession,
}

impl fmt::Display for BusSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BusSessionError::Transport(e) => write!(f, "{e}"),
            BusSessionError::AlreadyActive => {
                write!(f, "a monitor session is already active")
            }
            BusSessionError::NoActiveSession => write!(f, "no monitor session is active"),
        }
    }
}

impl std::error::Error for BusSessionError {}

/// Production [`GatewayConnector`]: wraps a `knx_net::KnxNetIpClient`
/// (stateless, `Sync` — safe to hold once in `AppState` for the whole
/// server lifetime).
pub struct RealConnector {
    client: KnxNetIpClient,
}

impl Default for RealConnector {
    fn default() -> Self {
        Self {
            client: KnxNetIpClient::new(),
        }
    }
}

impl GatewayConnector for RealConnector {
    #[allow(clippy::type_complexity)]
    fn connect_tunnel(
        &self,
        gateway: SocketAddrV4,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
    {
        Box::pin(async move {
            let tunnel = self
                .client
                .connect_tunnel(gateway)
                .await
                .map_err(BusSessionError::Transport)?;
            Ok(Box::new(RealTunnel(tunnel)) as Box<dyn BusTunnel>)
        })
    }
}

/// Production [`BusTunnel`]: delegates every method to a real
/// `knx_net::TunnelClient`. `assigned_address`/`subscribe` forward as-is;
/// `send` maps `knx_net::BusError` to [`BusSessionError`]; `disconnect`
/// unboxes with `*self` and calls the real consuming `disconnect(self)`.
pub struct RealTunnel(knx_net::TunnelClient);

impl BusTunnel for RealTunnel {
    fn assigned_address(&self) -> IndividualAddress {
        self.0.assigned_address()
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.0.subscribe()
    }

    fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
        Box::pin(async move {
            self.0
                .send(destination, service)
                .await
                .map_err(BusSessionError::Transport)
        })
    }

    fn disconnect(
        self: Box<Self>,
    ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
        Box::pin(async move {
            (*self)
                .0
                .disconnect()
                .await
                .map_err(BusSessionError::Transport)
        })
    }
}

/// Test support for [`GatewayConnector`]/[`BusTunnel`] — `FakeConnector`
/// and `FakeTunnel`, reachable from `apps/knx-server/tests/`.
///
/// These are ordinary `pub` items, not `#[cfg(test)]` ones. An integration
/// test under `apps/knx-server/tests/` compiles as its own separate crate
/// linked against this crate's `lib` target, so it never sees anything
/// gated behind `#[cfg(test)]` — that gate only applies within this crate's
/// own compilation. Making the fakes plain public items (with this doc
/// comment as the explanation) is the only way an integration test can
/// reach them; a `cfg(feature = ...)` gate was considered and rejected as
/// more machinery than a four-method trait pair's test double needs — this
/// crate has no feature flags today and `publish = false` means nothing
/// outside this workspace ever sees this module regardless.
pub mod fake {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::{Arc, Mutex};

    use super::{
        ApplicationService, BusError, BusSessionError, BusTunnel, Destination, Future,
        GatewayConnector, IndividualAddress, Pin, SocketAddrV4, TunnelEvent,
    };
    use tokio::sync::broadcast;

    struct Shared {
        address: IndividualAddress,
        tx: broadcast::Sender<TunnelEvent>,
        sent: Mutex<Vec<(Destination, ApplicationService)>>,
        disconnected: Mutex<bool>,
        /// Set by [`FakeTunnelHandle::panic_on_disconnect`] — the Task 3
        /// regression test for the carried `bus.rs:941` finding needs a
        /// drain task to panic without any real gateway, and `disconnect()`
        /// is the one call this task always makes on its way out
        /// regardless of which branch broke its loop, so it is the
        /// narrowest place to script a panic from.
        panic_on_disconnect: Mutex<bool>,
        /// Set by [`FakeTunnelHandle::fail_next_send`] — the `POST
        /// /api/bus/write` `502` test (design spec §4.3) needs a tunnel
        /// whose `send` fails without any real gateway. Always
        /// `BusError::Timeout`: `BusError` does not derive `Clone` (its
        /// `Io` variant holds a `std::io::Error`), so this cannot carry an
        /// arbitrary scripted error the way [`FakeConnector`] can for
        /// `connect_tunnel` — one fixed, easily recognised variant is
        /// enough to prove the `502` mapping without adding a second,
        /// heavier scripting mechanism for a single test. Consumed by the
        /// next `send` call, then cleared, so a test can still assert a
        /// later `send` succeeds normally.
        fail_next_send: Mutex<bool>,
    }

    /// The boxed side of a fake tunnel — what `FakeConnector::connect_tunnel`
    /// hands to code under test as a `Box<dyn BusTunnel>`. Created together
    /// with a [`FakeTunnelHandle`] via [`FakeTunnel::new`]; the two share one
    /// `Arc`, so everything the code under test does through the trait
    /// object (`send`, `disconnect`) is visible through the handle the test
    /// kept for itself.
    pub struct FakeTunnel {
        shared: Arc<Shared>,
    }

    /// The test-facing side of a fake tunnel: feed `TunnelEvent`s in via
    /// [`sender`](FakeTunnelHandle::sender), read back what [`FakeTunnel`]
    /// recorded.
    pub struct FakeTunnelHandle {
        shared: Arc<Shared>,
    }

    impl FakeTunnel {
        /// Builds a linked `(FakeTunnel, FakeTunnelHandle)` pair sharing one
        /// `broadcast` channel of `capacity` slots. `capacity` is a test
        /// parameter, not a fixed constant — forcing a real
        /// `RecvError::Lagged(n)` (a later task's regression test, design
        /// spec §4.2/§3 D3) needs a channel small enough to overflow on
        /// purpose.
        pub fn new(address: IndividualAddress, capacity: usize) -> (Self, FakeTunnelHandle) {
            let (tx, _rx) = broadcast::channel(capacity);
            let shared = Arc::new(Shared {
                address,
                tx,
                sent: Mutex::new(Vec::new()),
                disconnected: Mutex::new(false),
                panic_on_disconnect: Mutex::new(false),
                fail_next_send: Mutex::new(false),
            });
            (
                FakeTunnel {
                    shared: shared.clone(),
                },
                FakeTunnelHandle { shared },
            )
        }
    }

    impl FakeTunnelHandle {
        /// The sender side of this fake tunnel's event channel — send
        /// `TunnelEvent::Telegram(..)` to simulate bus traffic, or enough of
        /// them (with a small `capacity`) to force a `RecvError::Lagged` in
        /// whatever holds a receiver; send `TunnelEvent::Closed` to simulate
        /// the gateway dropping the connection.
        pub fn sender(&self) -> broadcast::Sender<TunnelEvent> {
            self.shared.tx.clone()
        }

        /// Every `(Destination, ApplicationService)` pair a `send` call on
        /// the linked [`FakeTunnel`] recorded, in call order.
        pub fn sent_calls(&self) -> Vec<(Destination, ApplicationService)> {
            self.shared
                .sent
                .lock()
                .expect("fake mutex poisoned")
                .clone()
        }

        /// Whether `disconnect` has been called on the linked [`FakeTunnel`].
        pub fn disconnected(&self) -> bool {
            *self
                .shared
                .disconnected
                .lock()
                .expect("fake mutex poisoned")
        }

        /// Scripts the linked [`FakeTunnel`]'s next `disconnect()` call to
        /// panic instead of completing normally — the regression test for
        /// the carried `bus.rs:941` finding ("a panicked drain task must
        /// not be reported as a clean stop") drives a real panic through
        /// this, with no gateway and no socket anywhere.
        pub fn panic_on_disconnect(&self) {
            *self
                .shared
                .panic_on_disconnect
                .lock()
                .expect("fake mutex poisoned") = true;
        }

        /// Scripts the linked [`FakeTunnel`]'s next `send()` call to fail
        /// with `BusSessionError::Transport(BusError::Timeout)` instead of
        /// recording and succeeding — the `POST /api/bus/write` `502` test
        /// drives this, with no gateway and no socket anywhere. Consumed by
        /// that one call; a later `send` on the same tunnel succeeds
        /// normally unless this is called again.
        pub fn fail_next_send(&self) {
            *self
                .shared
                .fail_next_send
                .lock()
                .expect("fake mutex poisoned") = true;
        }
    }

    impl BusTunnel for FakeTunnel {
        fn assigned_address(&self) -> IndividualAddress {
            self.shared.address
        }

        fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
            self.shared.tx.subscribe()
        }

        fn send(
            &self,
            destination: Destination,
            service: ApplicationService,
        ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send + '_>> {
            Box::pin(async move {
                let should_fail = {
                    let mut flag = self
                        .shared
                        .fail_next_send
                        .lock()
                        .expect("fake mutex poisoned");
                    std::mem::replace(&mut *flag, false)
                };
                if should_fail {
                    return Err(BusSessionError::Transport(BusError::Timeout));
                }
                self.shared
                    .sent
                    .lock()
                    .expect("fake mutex poisoned")
                    .push((destination, service));
                Ok(())
            })
        }

        fn disconnect(
            self: Box<Self>,
        ) -> Pin<Box<dyn Future<Output = Result<(), BusSessionError>> + Send>> {
            Box::pin(async move {
                if *self
                    .shared
                    .panic_on_disconnect
                    .lock()
                    .expect("fake mutex poisoned")
                {
                    panic!("FakeTunnel: scripted disconnect panic (test-only)");
                }
                *self
                    .shared
                    .disconnected
                    .lock()
                    .expect("fake mutex poisoned") = true;
                Ok(())
            })
        }
    }

    /// A [`GatewayConnector`] that never touches a socket. Holds one
    /// scripted outcome — a [`FakeTunnel`] to hand back on success, or a
    /// [`BusSessionError`] to exercise the "gateway refused" path — decided
    /// by the test before `connect_tunnel` is ever called, plus a call
    /// counter so a test can assert a second `start` never reaches the
    /// connector at all (design spec §3 D6, test 2). A single scripted
    /// outcome (rather than a queue) is the common one-session-per-test
    /// case the design spec itself calls out as an acceptable
    /// simplification; nothing in this task's scope needs more than one
    /// scripted `connect_tunnel` call per test.
    pub struct FakeConnector {
        outcome: Mutex<Option<Result<FakeTunnel, BusSessionError>>>,
        calls: AtomicUsize,
    }

    impl FakeConnector {
        pub fn succeeding(tunnel: FakeTunnel) -> Self {
            Self {
                outcome: Mutex::new(Some(Ok(tunnel))),
                calls: AtomicUsize::new(0),
            }
        }

        pub fn failing(error: BusSessionError) -> Self {
            Self {
                outcome: Mutex::new(Some(Err(error))),
                calls: AtomicUsize::new(0),
            }
        }

        /// How many times `connect_tunnel` has actually been called —
        /// asserted by tests that expect the route layer to short-circuit
        /// a second `start` before it ever reaches the connector.
        pub fn call_count(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
    }

    impl GatewayConnector for FakeConnector {
        #[allow(clippy::type_complexity)]
        fn connect_tunnel(
            &self,
            _gateway: SocketAddrV4,
        ) -> Pin<Box<dyn Future<Output = Result<Box<dyn BusTunnel>, BusSessionError>> + Send + '_>>
        {
            self.calls.fetch_add(1, Ordering::SeqCst);
            let outcome = self
                .outcome
                .lock()
                .expect("fake mutex poisoned")
                .take()
                .expect(
                    "FakeConnector::connect_tunnel called with no scripted outcome left — \
                     script exactly as many outcomes as the test calls connect_tunnel",
                );
            Box::pin(async move { outcome.map(|tunnel| Box::new(tunnel) as Box<dyn BusTunnel>) })
        }
    }
}

// ---------------------------------------------------------------------------
// T15 task 2 — the session: capped buffer, drain task, honest gaps.
// design spec `docs/superpowers/specs/2026-09-11-group-monitor-design.md`
// §3 D3/D4, §4.1, §4.2, §4.4.
// ---------------------------------------------------------------------------

/// Maximum number of [`TelegramRow`]s [`TelegramBuffer`] ever holds at once.
///
/// Not derived from a measured bus rate — TP1 arbitration keeps sustained
/// KNX bus traffic well under 50 telegrams/second even under load, and this
/// design's own polling interval (§4.3's client, `[R]` 500 ms) means an
/// ordinarily-behaved client drains the buffer twice a second. 5000 rows is
/// therefore generous: at a genuinely pathological sustained rate it holds
/// well over a minute of traffic before the oldest row is evicted, which is
/// far longer than a client would ever go without polling unless it had
/// stopped polling altogether — at which point losing old rows (and saying
/// so via `dropped_before`, never silently) is the correct behaviour, not a
/// bug to work around with a still-larger cap. Memory cost stays trivial at
/// this size (a few hundred bytes per row, so a full buffer is a few MB at
/// most), so there was no measured pressure pushing the number down either
/// (CLAUDE.md: "performance optimizations must be based on measured
/// bottlenecks" — this is a deliberate ceiling, not a performance fix).
pub const MAX_TELEGRAMS: usize = 5000;

/// One monitor-table row (design spec §4.4). Built once per telegram by
/// [`TelegramBuffer::push_telegram`] (or, for the synthetic "gateway closed"
/// marker, by [`TelegramBuffer::push_closed_marker`]) and never mutated
/// afterwards — a row is a record of an event, not a live view of anything.
///
/// `seq` is assigned here, by this buffer, when the row is pushed — never
/// derived from the wire. KNXnet/IP telegrams carry no sequence number of
/// their own; `next_seq` (see [`TelegramBuffer`]) is this application's own
/// invention, purely so a polling client has something monotonic to page
/// against.
#[derive(Debug, Clone, PartialEq)]
pub struct TelegramRow {
    pub seq: u64,
    /// RFC3339, via [`session_log::now`] — the same clock and format
    /// `session_log::LogEntry.timestamp` uses, deliberately: this is server
    /// wall-clock time at the moment the drain task received the event, NOT
    /// a bus-side timestamp. KNXnet/IP tunnelling carries no timestamp of
    /// its own, so "when the server saw it" is the only honest claim this
    /// field can make, and it is never presented as anything else.
    pub timestamp: String,
    /// `LDataFrame.source`, `Display`-formatted (e.g. `"1.1.5"`) — mirrors
    /// `apps/knx-cli/src/main.rs`'s `format_telegram`.
    pub source: String,
    /// The destination group address, formatted per the session's cached
    /// `GroupAddressStyle` (raw `u16` as a string if no project was open at
    /// session start — see [`GroupAddressContext`]). A frame addressed to
    /// an `IndividualAddress` never reaches this struct at all: see
    /// [`TelegramBuffer::push_telegram`]'s doc comment for why.
    pub destination: String,
    /// The destination's name from the project open at session start, or
    /// `None` if no project was open or the address has no matching entry.
    /// A session-start snapshot (see [`GroupAddressContext`]) — editing the
    /// project's group-address names mid-session does not change this for
    /// rows already pushed, nor for rows pushed later in the same session.
    pub destination_name: Option<String>,
    /// One of `"GroupValueRead"`, `"GroupValueResponse"`, `"GroupValueWrite"`,
    /// `"Other"` — mirrors `format_service`'s variant naming — or the
    /// synthetic marker's own `"SessionClosed"`, which is not one of
    /// `ApplicationService`'s variants and must not be treated as one by
    /// whatever renders this row.
    pub service: String,
    /// `GroupValueWrite`/`GroupValueResponse`: the payload, rendered exactly
    /// as `format_group_value_payload` renders it. `GroupValueRead`: `None`
    /// (it carries no payload). `Other{apci, data}`: the APCI/data hex form
    /// `format_service` already uses for that variant. The closed-session
    /// marker: a short human sentence explaining why telegrams stopped.
    pub raw_payload: Option<String>,
    /// The four-way decode outcome (design spec §4.3/§4.4, D4) for
    /// `GroupValueWrite`/`GroupValueResponse` rows only — `None` for
    /// `GroupValueRead`, `Other`, and the closed-session marker, since none
    /// of those carry a `GroupValue` to decode against anything.
    pub decoded: Option<DecodedValue>,
}

/// D4's four-way decode outcome, kept as a tagged enum rather than one
/// pre-formatted string so a future frontend can style a conflict
/// differently from a clean decode without re-parsing prose back out of it
/// (design spec §4.3's own reasoning, restated here because this is where
/// the type is actually defined). Each variant's `text` is a short,
/// human-readable sentence; `Value::dpt` and `Error::error` carry the
/// specific machine-relevant string each case adds beyond that prose —
/// `Error::text` deliberately mirrors `Error::error` verbatim (both are the
/// `DptCodecError`'s own `Display` text) so a consumer that only reads
/// `text` still gets the real reason, not a vaguer paraphrase of it.
#[derive(Debug, Clone, PartialEq)]
pub enum DecodedValue {
    /// Exactly one DPT resolved for this group address and `knx_core::decode`
    /// succeeded against it. `dpt` is `DptRef`'s own `Display` text (e.g.
    /// `"DPST-1-1"`) — the same `DPST-<main>-<sub>`/`DPT-<main>` form
    /// `apps/knx-cli/src/main.rs`'s `format_decoded_value` embeds in its
    /// `"{dpt_ref} {value}"` line, chosen for consistency with that
    /// existing convention over the dotted `"1.001"` shorthand the design
    /// spec's own §4.3 JSON example uses — Task 3, which actually builds
    /// the wire format, is free to reformat this string for the HTTP
    /// response if it wants the dotted form there; this module only
    /// promises an unambiguous, round-trippable DPT identifier.
    Value { dpt: String, text: String },
    /// No project was open at session start, or the group address resolved
    /// to no DPT at all (`GroupAddressDpt::None`, or simply absent from the
    /// map). Both collapse to the same outcome kind because a client only
    /// has four kinds to render (§4.3); `text` still names which reason it
    /// was, so nothing is lost, only folded into one tag.
    Unresolved { text: String },
    /// Linked communication objects disagree on this group address's DPT
    /// (`GroupAddressDpt::Conflict`) — `text` names every candidate, mirroring
    /// `format_decoded_value`'s `"conflicting DPTs: ..."` wording.
    Conflict { text: String },
    /// Exactly one DPT resolved, but `knx_core::decode` itself failed
    /// against it (wrong payload length, or a main type this codec slice
    /// does not implement — `docs/KNOWN_LIMITATIONS.md` §61, unchanged and
    /// inherited, never fixed inside this branch: `crates/knx-core/src/dpt/`
    /// is not touched by this task).
    Error { text: String, error: String },
}

/// The DPT-resolution and group-address-name maps a session computes once,
/// at start, from the project open in `AppState` at that moment (design
/// spec §4.4, D4) — never re-resolved mid-session. `None` project (nothing
/// open when the session started) collapses `style` to `None` too, which
/// [`GroupAddressContext::format_destination`] reads as "show the raw
/// `u16`", per the design's explicit "no project open: raw `u16`, names
/// `null`" rule. Deliberately private — nothing outside a session's own
/// start-up needs this, and it holds no reference to `AppState` or its
/// mutex, only an owned snapshot, which is the whole point: the project
/// can change (or vanish) after this is built and the session's rows keep
/// using what was true when it started.
///
/// `Clone`: [`BusSession`] keeps its own copy alongside the one moved into
/// `drain_task` — both need to resolve against the same session-start
/// snapshot (the task for incoming rows, the session itself for
/// [`BusSession::resolve_write_dpt`]), and an owned snapshot is cheap
/// enough (two small maps) that sharing it behind another `Arc` would be
/// more machinery than the duplication it avoids.
///
/// `pub(crate)` (Task 3 addition, was module-private through Task 2): the
/// `/start` route handler in `bus_routes.rs` must build one of these from
/// `AppState.project` *before* calling [`BusSession::start`], so the
/// project mutex is never held across that call's `.await` — see
/// `BusSession::start`'s doc comment for why. Still opaque outside this
/// module: only [`GroupAddressContext::from_project`] is constructible
/// from `bus_routes.rs`, and the value it returns is only ever handed
/// straight to `BusSession::start`, never inspected field-by-field there.
#[derive(Clone)]
pub(crate) struct GroupAddressContext {
    style: Option<GroupAddressStyle>,
    dpts: HashMap<u16, GroupAddressDpt>,
    names: HashMap<u16, String>,
}

impl GroupAddressContext {
    /// Builds the snapshot. Mirrors `apps/knx-cli/src/main.rs`'s
    /// `load_group_address_names`/`load_group_address_dpts`, minus the
    /// store round-trip those need (the CLI reads a `.knxdb` path; a
    /// running server already holds the live `knx_core::Project` in
    /// `AppState.project`, so there is nothing to open here, only to
    /// borrow once).
    pub(crate) fn from_project(project: Option<&knx_core::Project>) -> Self {
        match project {
            None => Self {
                style: None,
                dpts: HashMap::new(),
                names: HashMap::new(),
            },
            Some(project) => {
                let mut names = HashMap::new();
                for installation in &project.installations {
                    for entry in &installation.group_addresses {
                        names.insert(entry.address.raw(), entry.name.clone());
                    }
                }
                Self {
                    style: Some(project.info.group_address_style),
                    dpts: knx_core::resolve_project_group_address_dpts(project),
                    names,
                }
            }
        }
    }

    fn format_destination(&self, ga: GroupAddress) -> String {
        match self.style {
            Some(style) => ga.format(style),
            None => ga.raw().to_string(),
        }
    }

    fn name(&self, ga: GroupAddress) -> Option<String> {
        self.names.get(&ga.raw()).cloned()
    }

    /// D4's four-way outcome for one `GroupValue`, computed against this
    /// snapshot. Deliberately re-derives `format_decoded_value`'s decisions
    /// (`apps/knx-cli/src/main.rs:2076`) rather than calling it: that
    /// function lives in a `[[bin]]`-only crate with no `lib` target, so
    /// nothing outside it can call in, and the brief for this task is
    /// explicit that refactoring the CLI to share code is out of scope here
    /// — a scope widening, not a task-2 concern. The strings below match
    /// its wording on purpose, not by coincidence.
    fn decode(&self, ga: GroupAddress, value: &GroupValue) -> DecodedValue {
        if self.style.is_none() {
            return DecodedValue::Unresolved {
                text: "no project open".to_string(),
            };
        }
        match self.dpts.get(&ga.raw()) {
            None | Some(GroupAddressDpt::None) => DecodedValue::Unresolved {
                text: "no DPT resolved for this group address".to_string(),
            },
            Some(GroupAddressDpt::Single(dpt)) => decode_single(*dpt, value),
            Some(GroupAddressDpt::Conflict(dpts)) => DecodedValue::Conflict {
                text: format!("conflicting DPTs: {}", format_dpt_list(dpts)),
            },
        }
    }
}

fn decode_single(dpt: DptRef, value: &GroupValue) -> DecodedValue {
    match knx_core::decode(dpt, value) {
        Ok(v) => DecodedValue::Value {
            dpt: dpt.to_string(),
            text: v.format(dpt),
        },
        Err(e) => DecodedValue::Error {
            text: e.to_string(),
            error: e.to_string(),
        },
    }
}

fn format_dpt_list(dpts: &[DptRef]) -> String {
    dpts.iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Renders one `GroupValueWrite`/`GroupValueResponse` payload exactly as
/// `apps/knx-cli/src/main.rs`'s `format_group_value_payload` does — see
/// [`GroupAddressContext::decode`]'s doc comment for why this is a
/// deliberate re-derivation, not a shared call. `pub(crate)`: `bus_routes.rs`
/// reuses this to render `POST /api/bus/write`'s `encodedPayload`
/// (design spec §4.3) exactly as a monitored row would show the same
/// payload, rather than a second, independently-drifting formatter.
pub(crate) fn format_group_value_payload(v: &GroupValue) -> String {
    match v {
        GroupValue::Short(bits) => format!("{bits:#04x} (6-bit)"),
        GroupValue::Bytes(bytes) => format!("{bytes:02x?}"),
    }
}

/// Whether a session is still draining its tunnel or has stopped receiving
/// telegrams. Read alongside the buffer under the same lock (see
/// [`TelegramBuffer`]) precisely so a poll response's `status` and its
/// `telegrams`/`droppedBefore` are never observed from two different
/// instants (design spec §4.3's response shape bundles them for exactly
/// this reason).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionStatus {
    Active,
    /// The drain task exited on `TunnelEvent::Closed` or
    /// `RecvError::Closed` (design spec §4.1's "gateway drops mid-session"
    /// case). The buffer stays fully readable — nothing about `Closed`
    /// clears it — and `bus_session` itself is untouched: exactly one code
    /// path clears that (Task 3's `/stop` handler), never this task's drain
    /// loop, so there is never a race between the loop's own exit and a
    /// client-initiated stop both trying to tear the session down (design
    /// spec §4.1's `[R]` ruling, restated here since this is the status
    /// that ruling protects).
    Closed,
}

/// Fields for one new row, minus its `seq` — [`TelegramBuffer::push`]
/// assigns that. A small struct instead of a six-argument `push` purely for
/// readability at the two call sites ([`TelegramBuffer::push_telegram`],
/// [`TelegramBuffer::push_closed_marker`]).
struct NewRow {
    source: String,
    destination: String,
    destination_name: Option<String>,
    service: String,
    raw_payload: Option<String>,
    decoded: Option<DecodedValue>,
}

/// The capped, gap-accounted telegram store (design spec §3 D3). Mirrors
/// `session_log.rs`'s `SessionLog` in honesty — a loss is always counted,
/// never silent — but not its mechanism: `SessionLog` pins a synthetic
/// "N entries dropped" notice at index 0 and refreshes it in place, which
/// fits a log a human scrolls top-to-bottom. A telegram buffer is paged by
/// a numeric cursor (`since=<seq>`) instead, so a single running
/// `dropped_before` watermark — "this many telegrams existed before your
/// cursor and cannot be shown" — is the better fit: the client compares it
/// against what it already knew, the same way it would read a
/// `Content-Range` gap, rather than parsing a notice string out of the row
/// list itself (design spec §4.2's own reasoning).
///
/// Exactly one counter accounts for both ways a telegram is lost (design
/// spec §3 D3, restated as a ruling in the brief): ring-buffer eviction at
/// [`MAX_TELEGRAMS`] (`push`, below) and a lagged `broadcast::Receiver`
/// (`record_lagged`, driven by `RecvError::Lagged(n)` in the drain task).
/// Both add to `dropped_before`. A client cannot tell which cause produced
/// a given gap, and does not need to: from its point of view "N telegrams
/// existed between my cursor and what you can show me" is the same fact
/// either way.
#[derive(Debug)]
pub struct TelegramBuffer {
    entries: VecDeque<TelegramRow>,
    /// The next `seq` a pushed row receives. Monotonic for the life of the
    /// buffer; never reused, never derived from the wire (see
    /// `TelegramRow::seq`'s doc comment).
    next_seq: u64,
    /// Running count of telegrams that existed but cannot be shown — ring-
    /// buffer eviction and `RecvError::Lagged(n)` both add to this, never
    /// to a separate counter each (see this struct's own doc comment).
    dropped_before: u64,
    status: SessionStatus,
}

impl TelegramBuffer {
    fn new() -> Self {
        Self {
            entries: VecDeque::new(),
            next_seq: 0,
            dropped_before: 0,
            status: SessionStatus::Active,
        }
    }

    /// Assigns `seq`, appends, then enforces [`MAX_TELEGRAMS`] by evicting
    /// the oldest row and advancing `dropped_before` by exactly one per
    /// eviction — a cap breach never removes more than one row per push,
    /// so `dropped_before` never jumps by more than one here (contrast
    /// `RecvError::Lagged(n)`, which can jump it by many at once).
    fn push(&mut self, row: NewRow) {
        let seq = self.next_seq;
        self.next_seq += 1;
        self.entries.push_back(TelegramRow {
            seq,
            timestamp: session_log::now(),
            source: row.source,
            destination: row.destination,
            destination_name: row.destination_name,
            service: row.service,
            raw_payload: row.raw_payload,
            decoded: row.decoded,
        });
        if self.entries.len() > MAX_TELEGRAMS {
            self.entries.pop_front();
            self.dropped_before += 1;
        }
    }

    /// Turns one received `LDataFrame` into a row and pushes it — unless
    /// its destination is an `IndividualAddress`, in which case nothing is
    /// pushed at all: no row, no `seq` consumed, no addition to
    /// `dropped_before` either, because this is not a loss, it is this
    /// table's declared scope (design spec §7, item 7 — a group-monitor
    /// table's columns, `destinationName`/DPT resolution both included,
    /// assume a group address; an individually-addressed frame is a
    /// different diagnostic this slice does not attempt).
    fn push_telegram(&mut self, frame: knx_net::LDataFrame, ctx: &GroupAddressContext) {
        let knx_net::LDataFrame {
            source,
            destination,
            service,
            ..
        } = frame;
        let Destination::Group(ga) = destination else {
            return;
        };
        let source = source.to_string();
        let destination_name = ctx.name(ga);
        let destination = ctx.format_destination(ga);
        let (service_name, raw_payload, decoded) = match service {
            ApplicationService::GroupValueRead => ("GroupValueRead".to_string(), None, None),
            ApplicationService::GroupValueResponse(value) => (
                "GroupValueResponse".to_string(),
                Some(format_group_value_payload(&value)),
                Some(ctx.decode(ga, &value)),
            ),
            ApplicationService::GroupValueWrite(value) => (
                "GroupValueWrite".to_string(),
                Some(format_group_value_payload(&value)),
                Some(ctx.decode(ga, &value)),
            ),
            ApplicationService::Other { apci, data } => (
                "Other".to_string(),
                Some(format!("APCI {apci:#06x} data {data:02x?}")),
                None,
            ),
        };
        self.push(NewRow {
            source,
            destination,
            destination_name,
            service: service_name,
            raw_payload,
            decoded,
        });
    }

    /// The design spec §4.1/§4.2 "session closed by gateway" marker: a row
    /// a polling client can actually see, naming *why* telegrams stopped,
    /// pushed by the drain task in the same instant it flips
    /// [`SessionStatus::Closed`]. `service` is deliberately not one of
    /// `ApplicationService`'s variant names — `"SessionClosed"` cannot be
    /// mistaken for real bus traffic by anything that later renders this
    /// row.
    fn push_closed_marker(&mut self) {
        self.push(NewRow {
            source: "-".to_string(),
            destination: "-".to_string(),
            destination_name: None,
            service: "SessionClosed".to_string(),
            raw_payload: Some("session closed by gateway".to_string()),
            decoded: None,
        });
    }

    /// `RecvError::Lagged(n)`'s accounting (design spec §4.2): the *same*
    /// counter ring-buffer eviction advances, jumped by the exact count the
    /// broadcast channel itself reported as overwritten-and-unread. No row
    /// is pushed for a lag — there is nothing to show, only a count of what
    /// cannot be shown.
    fn record_lagged(&mut self, n: u64) {
        self.dropped_before += n;
    }

    pub fn status(&self) -> SessionStatus {
        self.status
    }

    pub fn dropped_before(&self) -> u64 {
        self.dropped_before
    }

    pub fn next_seq(&self) -> u64 {
        self.next_seq
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Every row with `seq >= since`, oldest first — what a poll handler
    /// (Task 3) hands back for `GET /telegrams?since=<seq>`. `since` below
    /// the buffer's current floor is not an error (design spec §4.3): every
    /// row still held comes back, possibly none, and `dropped_before`
    /// already explains what is missing — never a silent hole.
    pub fn telegrams_since(&self, since: u64) -> Vec<TelegramRow> {
        self.entries
            .iter()
            .filter(|row| row.seq >= since)
            .cloned()
            .collect()
    }
}

/// Final tally handed back by [`BusSession::stop`] — design spec §4.3's
/// `/stop` response fields (`telegramCount`/`droppedCount`), snake_cased
/// here since JSON naming is Task 3's concern, not this module's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusSessionSummary {
    pub telegram_count: usize,
    pub dropped_count: u64,
    /// `Some(message)` if the drain task's own `JoinHandle` reports a
    /// panic (carried Task 2 review finding, `bus.rs:941` before this
    /// fix) — `None` for the ordinary case, an orderly `break` out of the
    /// select loop followed by a clean `disconnect()`. Never discarded:
    /// see [`BusSession::stop`]'s doc comment for why a panic must not be
    /// reported as an indistinguishable clean stop. Not `Copy` any more
    /// (a `String` inside), hence dropping that derive here.
    pub drain_panic: Option<String>,
}

/// One open KNXnet/IP monitor session (design spec §4.1): an id, the
/// gateway it connected to, the shared buffer the drain task and any poll
/// handler both read, and the means to stop the task cleanly. Replaces
/// T15 task 1's deliberately minimal placeholder (`id` + boxed tunnel only,
/// formerly in `domain.rs`) now that a session actually has behaviour —
/// `bus.rs` is where that behaviour (the seam, the buffer, the drain task)
/// already lives, so this is its natural home too, not a second file that
/// would need to import half of this one to do anything.
///
/// `status` is deliberately not a field here — it lives inside
/// [`TelegramBuffer`] instead (see that struct's doc comment on why status
/// and the row/gap data are read under one lock), and [`BusSession::status`]
/// reads it there. This struct "owns" status in the sense the design spec
/// means (nothing outside a session can observe or change it except through
/// this session), just not as a literal separate field.
pub struct BusSession {
    id: u64,
    gateway: SocketAddrV4,
    /// `TunnelClient::assigned_address()` (design spec §4.3's `/start`
    /// response `assignedAddress`), captured once, synchronously, right
    /// after `connect_tunnel` returns — a plain field rather than an async
    /// method that would need to lock `tunnel` (below) for something that
    /// never changes for the life of a session.
    assigned_address: IndividualAddress,
    buffer: Arc<Mutex<TelegramBuffer>>,
    /// The open tunnel, shared with `drain_task` — `tokio::sync::Mutex`,
    /// not `std::sync::Mutex`: [`BusSession::send`] must hold the guard
    /// across an `.await` (the tunnel's own `send` is async), which a
    /// `std::sync::MutexGuard` cannot do (it is not `Send`). `Some` until
    /// `drain_task`'s own teardown takes it to call `disconnect()` — after
    /// that, [`BusSession::send`] sees `None` and reports
    /// [`BusSessionError::NoActiveSession`] rather than reaching for a
    /// tunnel that is already gone (the race window between a gateway-side
    /// `Closed`/`Lagged`-driven exit and a write request that arrives just
    /// after it; see that method's doc comment).
    tunnel: Arc<tokio::sync::Mutex<Option<Box<dyn BusTunnel>>>>,
    /// This session's own copy of the DPT/name snapshot — see
    /// [`GroupAddressContext`]'s doc comment on why it is `Clone` rather
    /// than shared behind another `Arc`. Used by
    /// [`BusSession::resolve_write_dpt`]; `drain_task` keeps its own clone
    /// for decoding incoming rows.
    ctx: GroupAddressContext,
    /// `Some` until [`BusSession::stop`] consumes it (or the drain task's
    /// own exit makes it moot) — sending on this is how `stop` asks the
    /// task to leave its `tokio::select!` loop. Kept as an `Option` even
    /// though `stop` takes `self` by value (so it can only ever be called
    /// once) purely so this struct does not need an `unsafe`/`ManuallyDrop`
    /// trick to move a non-`Copy` field out of `&mut self` in a `Drop`-less
    /// world; `stop` always finds it `Some`.
    stop_tx: oneshot::Sender<()>,
    join_handle: JoinHandle<()>,
}

impl BusSession {
    /// Opens a tunnel via `connector` and spawns the drain task around
    /// `ctx`, an already-built session-start snapshot (design spec §4.4 —
    /// never re-resolved once a session is running).
    ///
    /// Takes `ctx: GroupAddressContext`, not `project: Option<&Project>` —
    /// the earlier shape this had through Task 2. The `/start` route
    /// handler (Task 3) must build `ctx` from `AppState.project` *before*
    /// calling this, then drop that lock: `state.project` is a
    /// `std::sync::Mutex`, whose guard is not `Send` and must not be held
    /// across `connector.connect_tunnel`'s `.await` inside this function —
    /// besides the `Send` bound axum's handlers need, holding the
    /// project-wide lock for the duration of a gateway connect (which, for
    /// `RealConnector`, is a real network round trip) would stall every
    /// other route that touches the project for no reason connected to
    /// this session. `GroupAddressContext::from_project` is a synchronous,
    /// no-await borrow, so building `ctx` first and handing over an owned
    /// value here is the fix, not a workaround.
    pub(crate) async fn start(
        id: u64,
        gateway: SocketAddrV4,
        connector: &dyn GatewayConnector,
        ctx: GroupAddressContext,
    ) -> Result<Self, BusSessionError> {
        let tunnel = connector.connect_tunnel(gateway).await?;
        let assigned_address = tunnel.assigned_address();
        let receiver = tunnel.subscribe();
        let buffer = Arc::new(Mutex::new(TelegramBuffer::new()));
        let (stop_tx, stop_rx) = oneshot::channel();
        let task_buffer = Arc::clone(&buffer);
        let tunnel = Arc::new(tokio::sync::Mutex::new(Some(tunnel)));
        let task_tunnel = Arc::clone(&tunnel);
        let task_ctx = ctx.clone();
        let join_handle = tokio::spawn(drain_task(
            task_tunnel,
            receiver,
            task_buffer,
            task_ctx,
            stop_rx,
        ));
        Ok(Self {
            id,
            gateway,
            assigned_address,
            buffer,
            tunnel,
            ctx,
            stop_tx,
            join_handle,
        })
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn gateway(&self) -> SocketAddrV4 {
        self.gateway
    }

    /// `TunnelClient::assigned_address()`, `Display`-formatted — design
    /// spec §4.3's `/start` response `assignedAddress` (e.g. `"1.1.5"`).
    pub fn assigned_address(&self) -> IndividualAddress {
        self.assigned_address
    }

    pub fn status(&self) -> SessionStatus {
        self.buffer
            .lock()
            .expect("bus session buffer poisoned")
            .status()
    }

    /// A cloned handle to the shared buffer — what a poll handler (Task 3)
    /// locks to read `status`/`droppedBefore`/`telegrams_since` together,
    /// under one lock, as one consistent snapshot.
    pub fn buffer(&self) -> Arc<Mutex<TelegramBuffer>> {
        Arc::clone(&self.buffer)
    }

    /// `POST /api/bus/write`'s DPT resolution (design spec §4.4/§6, mirrors
    /// `apps/knx-cli/src/main.rs`'s `resolve_write_value`'s `--project`
    /// path): the session-start snapshot's answer for `ga`, or
    /// `GroupAddressDpt::None` if the map has no entry at all — the same
    /// thing `resolve_project_group_address_dpts` means by an absent key
    /// (it never stores `None` itself, see that function's doc comment),
    /// so collapsing "absent" and "explicitly `None`" here matches its own
    /// convention rather than inventing a third case the caller would have
    /// to handle identically anyway.
    pub fn resolve_write_dpt(&self, ga: GroupAddress) -> GroupAddressDpt {
        self.ctx
            .dpts
            .get(&ga.raw())
            .cloned()
            .unwrap_or(GroupAddressDpt::None)
    }

    /// Sends one group value through this session's open tunnel (design
    /// spec §D5: send-from-the-table reuses the active session's tunnel,
    /// never opens a second connection). Takes `&self`, not `self` —
    /// multiple writes over one session's lifetime are expected, unlike
    /// `stop`. Locks the shared tunnel only for the duration of the send;
    /// `tokio::sync::Mutex` so the lock can be held across the tunnel's own
    /// `.await`. Returns [`BusSessionError::NoActiveSession`] if the tunnel
    /// has already been taken by `drain_task`'s teardown — the narrow race
    /// where a gateway-side close (`Closed`/`RecvError::Closed`) finishes
    /// tearing the tunnel down between this session being looked up in
    /// `AppState.bus_session` and this call actually locking the tunnel;
    /// the route layer's ordinary "no session" `409` covers the same case
    /// when it happens before the lookup, so both timings answer the
    /// caller identically.
    pub async fn send(
        &self,
        destination: Destination,
        service: ApplicationService,
    ) -> Result<(), BusSessionError> {
        let guard = self.tunnel.lock().await;
        match guard.as_ref() {
            Some(tunnel) => tunnel.send(destination, service).await,
            None => Err(BusSessionError::NoActiveSession),
        }
    }

    /// Signals the drain task to stop, awaits its exit (so `disconnect()`
    /// has genuinely already run on the tunnel by the time this returns —
    /// not a fire-and-forget stop, design spec §4.1/acceptance criterion 5),
    /// then reports the final buffer stats. Consumes `self`: the caller
    /// (Task 3's `/stop` handler) is expected to `Option::take()` the
    /// `BusSession` out of `AppState.bus_session` *before* calling this —
    /// that `take()` is the one and only place `bus_session` is ever
    /// cleared (design spec §4.1's `[R]` ruling; see [`SessionStatus::Closed`]'s
    /// doc comment for the race it avoids). This method does not touch
    /// `AppState` at all, by design — it has no reference to one.
    ///
    /// **Panic handling (Task 2 review finding, `bus.rs:941` before this
    /// fix):** a `JoinError` from `self.join_handle` is no longer
    /// discarded. `is_panic()` distinguishes an actual drain-task panic
    /// (its message, if any, becomes `BusSessionSummary::drain_panic`) from
    /// a cancellation (this task never calls `.abort()` on the handle, so
    /// that branch is unreached in production; kept so this match stays
    /// exhaustive rather than a `.unwrap()` that would itself panic on the
    /// one input this method exists to handle honestly). Either way, the
    /// buffer is read *after* the join, under its own lock, same as
    /// before — a panic inside `drain_task` (this module's only such task)
    /// happens either while a buffer-lock guard is held (which would poison
    /// that lock, and the `.expect` below already surfaces that loudly, by
    /// design, same as every other buffer access in this module) or, as
    /// the regression test below exercises, while the tunnel is being
    /// disconnected — outside the buffer's lock scope entirely, so the
    /// buffer stays perfectly readable and its contents are never thrown
    /// away just because something else went wrong on the way out. A `500`
    /// was considered and rejected for this case (see the task report):
    /// the stop operation itself *did* succeed — the signal was sent, the
    /// task exited, the session is genuinely gone — only the teardown that
    /// followed it misbehaved, and a `500` would force discarding the
    /// still-accurate `telegramCount`/`droppedCount` tally or awkwardly
    /// smuggling it into an error body instead of the success shape §4.3
    /// already defines.
    pub async fn stop(self) -> BusSessionSummary {
        // `stop_tx.send` fails only if the drain task already exited (e.g.
        // a gateway-side close beat this call) — the task is gone either
        // way, so a failed send changes nothing about what happens next.
        let _ = self.stop_tx.send(());
        let join_result = self.join_handle.await;
        let buffer = self.buffer.lock().expect("bus session buffer poisoned");
        let drain_panic = match join_result {
            Ok(()) => None,
            Err(e) if e.is_panic() => Some(panic_payload_message(e.into_panic())),
            Err(_) => Some("drain task was cancelled".to_string()),
        };
        BusSessionSummary {
            telegram_count: buffer.len(),
            dropped_count: buffer.dropped_before(),
            drain_panic,
        }
    }
}

/// Extracts a human-readable message from a `JoinError::into_panic()`
/// payload — `std::panic!`/`panic!("{msg}", ...)` payloads are almost
/// always `&str` or `String` (what `std::panic::Location`'s default hook
/// also assumes), covering every panic this module's own code or a test's
/// `FakeTunnel` can produce; anything else still yields an honest, if
/// generic, message rather than silently losing the fact that a panic
/// happened at all.
fn panic_payload_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "drain task panicked with a non-string payload".to_string()
    }
}

/// The background task a [`BusSession`] spawns (design spec §4.2), mirroring
/// `apps/knx-cli/src/main.rs`'s `run_bus_monitor_async` — the same
/// `tokio::select!` shape between a stop signal (there: `ctrl_c()`; here: a
/// `oneshot::Receiver<()>`) and `receiver.recv()`.
///
/// Holds the boxed tunnel behind the same `Arc<tokio::sync::Mutex<..>>` the
/// owning [`BusSession`] keeps its own clone of (Task 3 addition —
/// [`BusSession::send`] needs concurrent access to the same tunnel while
/// this task is still draining it, which a task-owned `Box<dyn BusTunnel>`
/// with no other reference, as Task 2 originally built it, could not
/// offer). This task still is the one and only place `disconnect()` is
/// called, and still calls it exactly once, on the way out, regardless of
/// which branch broke the loop — `Option::take()` on the shared slot makes
/// that "exactly once" true even though the slot is now reachable from two
/// places, since whichever side calls `take()` first is the only side that
/// ever gets `Some` back.
async fn drain_task(
    tunnel: Arc<tokio::sync::Mutex<Option<Box<dyn BusTunnel>>>>,
    mut receiver: broadcast::Receiver<TunnelEvent>,
    buffer: Arc<Mutex<TelegramBuffer>>,
    ctx: GroupAddressContext,
    mut stop_rx: oneshot::Receiver<()>,
) {
    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                break;
            }
            event = receiver.recv() => match event {
                Ok(TunnelEvent::Telegram(frame)) => {
                    buffer
                        .lock()
                        .expect("bus session buffer poisoned")
                        .push_telegram(frame, &ctx);
                }
                Ok(TunnelEvent::Closed) => {
                    let mut buffer = buffer.lock().expect("bus session buffer poisoned");
                    buffer.push_closed_marker();
                    buffer.status = SessionStatus::Closed;
                    break;
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    buffer
                        .lock()
                        .expect("bus session buffer poisoned")
                        .record_lagged(n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    let mut buffer = buffer.lock().expect("bus session buffer poisoned");
                    buffer.push_closed_marker();
                    buffer.status = SessionStatus::Closed;
                    break;
                }
            }
        }
    }
    // Never clears `bus_session` and never touches `AppState` at all — this
    // task has no reference to either. Exactly one code path clears
    // `bus_session` (design spec §4.1's `[R]` ruling): Task 3's `/stop`
    // handler, after `BusSession::stop` (above) has already awaited this
    // task's exit. A gateway-side close only ever sets `status` (above) and
    // leaves the buffer fully readable — the session itself stays "active"
    // as far as `AppState` is concerned until a client explicitly stops it.
    //
    // `take()` instead of an owned `Box` (Task 2's original shape): if
    // `BusSession::send` is mid-call and already holds the lock, this
    // `.lock().await` simply waits its turn, same as any other tunnel use
    // would; if this task gets there first, `send` finds `None` afterwards
    // and reports `BusSessionError::NoActiveSession` (see that method's
    // doc comment) rather than a panic or a silent no-op.
    if let Some(tunnel) = tunnel.lock().await.take() {
        let _ = tunnel.disconnect().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fake::{FakeConnector, FakeTunnel, FakeTunnelHandle};

    fn addr(device: u8) -> IndividualAddress {
        IndividualAddress::new(1, 1, device).expect("valid test address")
    }

    fn gateway() -> SocketAddrV4 {
        "127.0.0.1:3671".parse().expect("valid test gateway")
    }

    /// A fake tunnel with a generous broadcast capacity — the default for
    /// every test that is not specifically trying to force a lag.
    fn fake_tunnel() -> (FakeTunnel, FakeTunnelHandle) {
        FakeTunnel::new(addr(5), 64)
    }

    fn telegram(destination: knx_net::Destination, service: ApplicationService) -> TunnelEvent {
        TunnelEvent::Telegram(knx_net::LDataFrame {
            kind: knx_net::LDataMessageKind::Indication,
            source: addr(9),
            destination,
            service,
        })
    }

    fn group_value_write(raw: u16, value: GroupValue) -> TunnelEvent {
        telegram(
            Destination::Group(GroupAddress::from_raw(raw)),
            ApplicationService::GroupValueWrite(value),
        )
    }

    // -- start / drain / stop -------------------------------------------

    #[tokio::test]
    async fn start_drain_stop_carries_telegrams_through_and_disconnects() {
        let (tunnel, handle) = fake_tunnel();
        let connector = FakeConnector::succeeding(tunnel);
        let session = BusSession::start(
            1,
            gateway(),
            &connector,
            GroupAddressContext::from_project(None),
        )
        .await
        .expect("fake connector always succeeds");

        handle
            .sender()
            .send(group_value_write(1, GroupValue::Short(1)))
            .expect("receiver still subscribed");
        // Give the spawned task a chance to actually drain the event before
        // asserting on the buffer — `tokio::test`'s single-threaded runtime
        // still needs to yield to the task at least once.
        tokio::task::yield_now().await;
        for _ in 0..50 {
            if session.buffer().lock().unwrap().len() == 1 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }

        {
            let buffer = session.buffer();
            let buffer = buffer.lock().unwrap();
            assert_eq!(buffer.len(), 1);
            let row = &buffer.telegrams_since(0)[0];
            assert_eq!(row.seq, 0);
            assert_eq!(row.source, addr(9).to_string());
            assert_eq!(row.service, "GroupValueWrite");
            assert_eq!(row.destination_name, None);
        }

        assert!(!handle.disconnected());
        let summary = session.stop().await;
        assert_eq!(summary.telegram_count, 1);
        assert_eq!(summary.dropped_count, 0);
        assert!(handle.disconnected());
    }

    #[test]
    fn individual_addressed_frames_are_not_rendered_as_rows() {
        let mut buffer = TelegramBuffer::new();
        let ctx = GroupAddressContext::from_project(None);
        let frame = knx_net::LDataFrame {
            kind: knx_net::LDataMessageKind::Indication,
            source: addr(9),
            destination: Destination::Individual(addr(1)),
            service: ApplicationService::GroupValueRead,
        };
        buffer.push_telegram(frame, &ctx);
        assert_eq!(buffer.len(), 0);
        assert_eq!(buffer.next_seq(), 0);
        assert_eq!(buffer.dropped_before(), 0);
    }

    // -- panicked drain task (carried Task 2 finding, bus.rs:941) ----------

    /// The regression test the brief asks for: a real panic inside
    /// `drain_task`, forced through `FakeTunnel::disconnect`'s scripted
    /// panic (no gateway, no socket) — `stop()` must report it via
    /// `drain_panic`, not discard the `JoinError` and pretend the stop was
    /// clean, and it must not lose the telegram the buffer already holds.
    #[tokio::test]
    async fn stop_reports_a_panicked_drain_task_without_losing_buffered_telegrams() {
        let (tunnel, handle) = fake_tunnel();
        handle.panic_on_disconnect();
        let connector = FakeConnector::succeeding(tunnel);
        let session = BusSession::start(
            1,
            gateway(),
            &connector,
            GroupAddressContext::from_project(None),
        )
        .await
        .expect("fake connector always succeeds");

        handle
            .sender()
            .send(group_value_write(1, GroupValue::Short(1)))
            .expect("receiver still subscribed");
        for _ in 0..200 {
            if session.buffer().lock().unwrap().len() == 1 {
                break;
            }
            tokio::task::yield_now().await;
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        assert_eq!(session.buffer().lock().unwrap().len(), 1);

        let summary = session.stop().await;
        assert_eq!(
            summary.telegram_count, 1,
            "the telegram buffered before the panic must still be reported, not discarded"
        );
        assert_eq!(summary.dropped_count, 0);
        let warning = summary
            .drain_panic
            .expect("a panicked drain task must surface as Some(..), not a silent clean stop");
        assert!(
            warning.contains("scripted disconnect panic"),
            "expected the fake's own panic message, got: {warning}"
        );
    }

    // -- eviction accounting at the cap -----------------------------------

    #[test]
    fn eviction_at_the_cap_advances_dropped_before_by_exactly_one_per_overflow() {
        let mut buffer = TelegramBuffer::new();
        let ctx = GroupAddressContext::from_project(None);
        for raw in 0..(MAX_TELEGRAMS as u16 + 3) {
            buffer.push_telegram(
                knx_net::LDataFrame {
                    kind: knx_net::LDataMessageKind::Indication,
                    source: addr(9),
                    destination: Destination::Group(GroupAddress::from_raw(raw)),
                    service: ApplicationService::GroupValueWrite(GroupValue::Short(0)),
                },
                &ctx,
            );
        }
        assert_eq!(buffer.len(), MAX_TELEGRAMS);
        assert_eq!(buffer.dropped_before(), 3);
        assert_eq!(buffer.next_seq(), MAX_TELEGRAMS as u64 + 3);
        // The oldest surviving row is the 4th one pushed (seq 3) — rows 0-2
        // were evicted to make room, exactly matching `dropped_before`.
        let remaining = buffer.telegrams_since(0);
        assert_eq!(remaining.first().unwrap().seq, 3);
    }

    // -- the Lagged(n) accounting: the test read hardest -------------------

    /// Forces a genuine `RecvError::Lagged(n)` by giving the fake tunnel's
    /// broadcast channel a capacity far smaller than the number of events
    /// sent while nothing is draining it — not simulated, not asserted as
    /// `> 0`: `tokio::sync::broadcast`'s own documented semantics make the
    /// exact lag count deterministic in this setup, and this test proves
    /// it. See the task report for the exact reasoning this test relies on.
    #[tokio::test]
    async fn lagged_receiver_advances_the_same_dropped_before_counter_by_the_exact_count() {
        const CAPACITY: usize = 4;
        const SENT: usize = 10;
        let (tunnel, handle) = FakeTunnel::new(addr(5), CAPACITY);
        let connector = FakeConnector::succeeding(tunnel);

        // Two independent receivers on the same fake channel: one is what
        // the session's drain task will use (created inside `start` via
        // `subscribe()`); this second one is created *now*, before the
        // session starts, purely so it accumulates the exact same backlog
        // without ever being polled — proving the lag is real, not staged.
        // It is dropped immediately after computing the expected count, so
        // it cannot itself affect the session's own receiver.
        let sender = handle.sender();
        {
            let mut probe = sender.subscribe();
            for raw in 0..SENT {
                sender
                    .send(group_value_write(raw as u16, GroupValue::Short(0)))
                    .expect("at least one subscriber (the probe) is live");
            }
            // `broadcast::Receiver::recv` on a receiver that fell behind
            // reports the lag on its *first* read after falling behind —
            // reading it here pins down the exact number this test expects
            // the session's own (never-read-until-now) receiver to report
            // too, since both receivers were subscribed before any send and
            // therefore see an identical backlog.
            match probe.recv().await {
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    assert_eq!(n as usize, SENT - CAPACITY);
                }
                other => panic!("expected a probe Lagged(n), got {other:?}"),
            }
        }

        // Now start the real session. Its receiver, created by `subscribe()`
        // inside `start`, was never live during the sends above, so it has
        // an entirely empty channel to read from — it will NOT see a lag on
        // its own. This confirms the harness: to make the *session's*
        // receiver lag, the sends must happen after `start`, with the
        // drain task deliberately kept from running via a paused/never-
        // yielded window. See the second half of this test below.
        let session = BusSession::start(
            1,
            gateway(),
            &connector,
            GroupAddressContext::from_project(None),
        )
        .await
        .expect("fake connector always succeeds");
        let summary = session.stop().await;
        assert_eq!(summary.telegram_count, 0);
        assert_eq!(summary.dropped_count, 0);
    }

    /// The actual regression test the brief asks for: the *session's own*
    /// drain-task receiver falls behind, and `dropped_before` accounts for
    /// exactly the number of telegrams that existed and could not be shown.
    #[tokio::test(flavor = "current_thread")]
    async fn session_receiver_lag_is_accounted_exactly_in_dropped_before() {
        const CAPACITY: usize = 4;
        const SENT: usize = 10;
        let (tunnel, handle) = FakeTunnel::new(addr(5), CAPACITY);
        let connector = FakeConnector::succeeding(tunnel);

        // Pause the runtime's auto-advance of spawned tasks: on the
        // `current_thread` flavor, a freshly spawned task does not run
        // until the current task yields. By sending every event *before*
        // yielding even once, all `SENT` sends land while the drain task's
        // `subscribe()`-created receiver has not been polled a single time
        // — the same starting condition that produces a real `Lagged(n)`
        // from `tokio::sync::broadcast` on genuine hardware, just made
        // deterministic by controlling scheduling instead of timing.
        let session = BusSession::start(
            1,
            gateway(),
            &connector,
            GroupAddressContext::from_project(None),
        )
        .await
        .expect("fake connector always succeeds");

        for raw in 0..SENT {
            handle
                .sender()
                .send(group_value_write(raw as u16, GroupValue::Short(0)))
                .expect("drain task is still subscribed");
        }

        // Now let the drain task actually run. It will see the channel
        // already overflowed by `SENT - CAPACITY` messages relative to what
        // it could hold, and `broadcast::Receiver::recv` reports that
        // exactly once as `RecvError::Lagged(SENT - CAPACITY)`.
        let mut settled = false;
        for _ in 0..200 {
            tokio::task::yield_now().await;
            if session.buffer().lock().unwrap().dropped_before() > 0 {
                settled = true;
                break;
            }
        }
        assert!(settled, "drain task never observed the lag");

        let summary = session.stop().await;
        assert_eq!(
            summary.dropped_count,
            (SENT - CAPACITY) as u64,
            "dropped_before must equal exactly the number of telegrams the \
             session's receiver could not show — not merely > 0"
        );
        // The `CAPACITY` telegrams that were still in the channel's ring
        // buffer when the drain task caught up are delivered normally.
        assert_eq!(summary.telegram_count, CAPACITY);
        drop(handle);
    }

    // -- TunnelEvent::Closed ------------------------------------------------

    #[tokio::test]
    async fn gateway_close_sets_status_closed_and_leaves_buffer_readable() {
        let (tunnel, handle) = fake_tunnel();
        let connector = FakeConnector::succeeding(tunnel);
        let session = BusSession::start(
            1,
            gateway(),
            &connector,
            GroupAddressContext::from_project(None),
        )
        .await
        .expect("fake connector always succeeds");

        handle
            .sender()
            .send(group_value_write(1, GroupValue::Short(1)))
            .expect("receiver still subscribed");
        handle
            .sender()
            .send(TunnelEvent::Closed)
            .expect("receiver still subscribed");

        let mut closed = false;
        for _ in 0..200 {
            tokio::task::yield_now().await;
            if session.status() == SessionStatus::Closed {
                closed = true;
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        }
        assert!(closed, "drain task never observed TunnelEvent::Closed");

        // The buffer is still fully readable: the real telegram plus the
        // synthetic closed-session marker, both present. Scoped to a block
        // so the `MutexGuard` is unquestionably dropped before the `await`
        // points below — `clippy::await_holding_lock` flags a guard held
        // across an await even when an explicit `drop` follows it in the
        // same scope.
        {
            let buffer = session.buffer();
            let buffer = buffer.lock().unwrap();
            assert_eq!(buffer.len(), 2);
            let rows = buffer.telegrams_since(0);
            assert_eq!(rows[0].service, "GroupValueWrite");
            assert_eq!(rows[1].service, "SessionClosed");
        }

        // `disconnect` still ran, even though nobody called `stop()` — the
        // drain task disconnects on its own way out regardless of why it
        // broke out of the loop.
        for _ in 0..200 {
            if handle.disconnected() {
                break;
            }
            tokio::task::yield_now().await;
        }
        assert!(handle.disconnected());
    }

    // -- decode outcomes (D4's four-way vocabulary) -------------------------

    fn dpt(main: u16, sub: u16) -> DptRef {
        DptRef {
            main,
            sub: Some(sub),
        }
    }

    /// A minimal project with one group address per DPT-resolution outcome
    /// this test exercises: `1` resolves to a single DPT-1.001 (boolean),
    /// `2` has no linked communication object at all (`None`), `3` has two
    /// communication objects disagreeing on the DPT (`Conflict`).
    fn project_with_group_addresses() -> knx_core::Project {
        use knx_core::{
            ComObjectInstance, ComObjectInstanceId, Direction, GroupAddress, GroupAddressEntry,
            GroupAddressId, GroupLink, Installation, InstallationId, Language, Layer, Override,
            Resolved, ResolvedFlags, SourceRef,
        };

        let source = || SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        };
        let stated = |main: u16, sub: u16| {
            Override::Value(Resolved {
                value: dpt(main, sub),
                layer: Layer::Instance,
            })
        };
        let com_object =
            |id: u32, resolved_dpt: Override<DptRef>, ga: GroupAddressId| ComObjectInstance {
                id: ComObjectInstanceId(id),
                source: source(),
                device: knx_core::DeviceId(1),
                number: 0,
                text: Override::Absent,
                description: Override::Absent,
                dpt: resolved_dpt,
                flags: ResolvedFlags::none(),
                size: None,
                is_active: true,
                links: vec![GroupLink {
                    ga,
                    direction: Direction::Send,
                }],
                module_instance: None,
            };

        let mut project = knx_core::Project::new(Language("en".into()));
        project
            .devices
            .insert_com_object(com_object(1, stated(1, 1), GroupAddressId(1)));
        project
            .devices
            .insert_com_object(com_object(3, stated(5, 1), GroupAddressId(3)));
        project
            .devices
            .insert_com_object(com_object(4, stated(1, 1), GroupAddressId(3)));

        project.installations.push(Installation {
            id: InstallationId(1),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: knx_core::CompletionStatus::FinishedDesign,
            topology: knx_core::Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![
                GroupAddressEntry {
                    id: GroupAddressId(1),
                    source: source(),
                    name: "Living room / light / switch".into(),
                    address: GroupAddress::from_raw(1),
                    central: false,
                    unfiltered: false,
                    range: None,
                },
                GroupAddressEntry {
                    id: GroupAddressId(2),
                    source: source(),
                    name: "Unlinked".into(),
                    address: GroupAddress::from_raw(2),
                    central: false,
                    unfiltered: false,
                    range: None,
                },
                GroupAddressEntry {
                    id: GroupAddressId(3),
                    source: source(),
                    name: "Conflicting".into(),
                    address: GroupAddress::from_raw(3),
                    central: false,
                    unfiltered: false,
                    range: None,
                },
            ],
            parameters: vec![],
        });
        project
    }

    #[test]
    fn decode_outcomes_cover_value_unresolved_conflict_and_no_project() {
        let project = project_with_group_addresses();
        let ctx = GroupAddressContext::from_project(Some(&project));

        match ctx.decode(GroupAddress::from_raw(1), &GroupValue::Short(1)) {
            DecodedValue::Value { dpt, text } => {
                assert_eq!(dpt, "DPST-1-1");
                assert_eq!(text, "on");
            }
            other => panic!("expected Value, got {other:?}"),
        }

        match ctx.decode(GroupAddress::from_raw(2), &GroupValue::Short(1)) {
            DecodedValue::Unresolved { text } => {
                assert_eq!(text, "no DPT resolved for this group address");
            }
            other => panic!("expected Unresolved, got {other:?}"),
        }

        match ctx.decode(GroupAddress::from_raw(3), &GroupValue::Short(1)) {
            DecodedValue::Conflict { text } => {
                assert!(text.contains("DPST-1-1"));
                assert!(text.contains("DPST-5-1"));
            }
            other => panic!("expected Conflict, got {other:?}"),
        }

        let no_project = GroupAddressContext::from_project(None);
        match no_project.decode(GroupAddress::from_raw(1), &GroupValue::Short(1)) {
            DecodedValue::Unresolved { text } => assert_eq!(text, "no project open"),
            other => panic!("expected Unresolved, got {other:?}"),
        }
        assert_eq!(
            no_project.format_destination(GroupAddress::from_raw(1)),
            "1"
        );
    }

    #[test]
    fn decode_error_case_surfaces_the_codec_error_text() {
        let mut project = knx_core::Project::new(knx_core::Language("en".into()));
        use knx_core::{
            ComObjectInstance, ComObjectInstanceId, Direction, GroupAddressEntry, GroupAddressId,
            GroupLink, Installation, InstallationId, Layer, Override, Resolved, ResolvedFlags,
            SourceRef,
        };
        let source = SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        };
        // DPT-1.001 with a `Bytes` payload — a length mismatch `decode`
        // rejects (main type 1 wants the 6-bit inline form), not a main
        // type this codec slice fails to implement at all.
        project.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source.clone(),
            device: knx_core::DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Value(Resolved {
                value: dpt(1, 1),
                layer: Layer::Instance,
            }),
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![GroupLink {
                ga: GroupAddressId(1),
                direction: Direction::Send,
            }],
            module_instance: None,
        });
        project.installations.push(Installation {
            id: InstallationId(1),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: knx_core::CompletionStatus::FinishedDesign,
            topology: knx_core::Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![GroupAddressEntry {
                id: GroupAddressId(1),
                source,
                name: "GA".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: None,
            }],
            parameters: vec![],
        });

        let ctx = GroupAddressContext::from_project(Some(&project));
        match ctx.decode(GroupAddress::from_raw(1), &GroupValue::Bytes(vec![1, 2])) {
            DecodedValue::Error { text, error } => {
                assert_eq!(text, error);
                assert!(!text.is_empty());
            }
            other => panic!("expected Error, got {other:?}"),
        }
    }
}
