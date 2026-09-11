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
//! This task (T15 task 1) wires the seam and `AppState`'s fields only — no
//! route reads them yet. The session lifecycle (start/stop/drain-task/
//! lagged accounting) is built on top of this in later tasks.

use std::fmt;
use std::future::Future;
use std::net::SocketAddrV4;
use std::pin::Pin;

use knx_core::IndividualAddress;
use knx_net::{ApplicationService, BusConnection, BusError, Destination, KnxNetIpClient};
use tokio::sync::broadcast;

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

/// Failure surfaced by [`GatewayConnector`]/[`BusTunnel`]. `Transport`
/// carries whatever `knx_net::BusError` reported (I/O, timeout, gateway
/// refusal, an unimplemented operation, a lower-layer codec error) —
/// everything this seam's production side (`knx-net`) can fail with. An
/// unparsable gateway address is deliberately *not* a variant here: that
/// is a caller mistake the eventual HTTP route rejects before ever
/// reaching a `GatewayConnector`, not something the connector/tunnel
/// themselves can produce. Later tasks may add session-level variants (an
/// already-active session, for instance) once a route exists to produce
/// them; this task adds no route, so none exist yet.
#[derive(Debug)]
pub enum BusSessionError {
    Transport(BusError),
}

impl fmt::Display for BusSessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BusSessionError::Transport(e) => write!(f, "{e}"),
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
        ApplicationService, BusSessionError, BusTunnel, Destination, Future, GatewayConnector,
        IndividualAddress, Pin, SocketAddrV4, TunnelEvent,
    };
    use tokio::sync::broadcast;

    struct Shared {
        address: IndividualAddress,
        tx: broadcast::Sender<TunnelEvent>,
        sent: Mutex<Vec<(Destination, ApplicationService)>>,
        disconnected: Mutex<bool>,
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
