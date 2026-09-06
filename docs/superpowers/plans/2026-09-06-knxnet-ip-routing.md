# KNXnet/IP Routing (Session 6, Cycle 4) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a gateway-free `BusConnection::connect_routing` path — send and receive KNX telegrams over the standard KNXnet/IP routing multicast group (`224.0.23.12:3671`) — plus two CLI subcommands, `knx bus route-monitor` and `knx bus route-send`.

**Architecture:** `RoutingClient` is a new sibling of `TunnelClient` in `crates/knx-net/src/client.rs`. It reuses the existing `cemi`/`frame` codecs unchanged (`ROUTING_INDICATION`'s body is exactly an `L_Data.ind` cEMI frame, no extra wrapping) and adds one new pure-decode module, `routing.rs`, for the two flow-control frame types it only needs to log (`ROUTING_LOST_MESSAGE`, `ROUTING_BUSY`). Unlike tunnelling, routing is connectionless and unconfirmed — no handshake, no ACK, no heartbeat.

**Tech Stack:** Rust, tokio (`net`, `sync`, `time`, `macros`, `rt` features, already enabled), `socket2` (new dependency, already present transitively via tokio at 0.6.5 — pinned to the same version here) for `SO_REUSEADDR` before bind.

**Spec:** `docs/superpowers/specs/2026-09-06-knxnet-ip-routing-design.md`

## Global Constraints

- Routing multicast group/port hardcoded to `224.0.23.12:3671` — no `--multicast` CLI override this cycle.
- `own_address: IndividualAddress` is a required parameter of `connect_routing` — no default/invented address.
- `ROUTING_BUSY`/`ROUTING_LOST_MESSAGE` are decoded and logged (`eprintln!`) on receipt; no send-throttling reaction is implemented.
- Outgoing frames always use `LDataMessageKind::Indication` and the existing fixed hop count (6) from `cemi::encode_l_data` — this client never decrements a routing counter, since it's an IP-side endpoint, not a router.
- No new `BusError` variant — reuse `Io`/`Protocol`.
- Every codec function is pure (byte slice in, typed struct/`Vec<u8>` out) — `client.rs` owns all socket IO, same convention as `discovery.rs`/`tunnelling.rs`.

---

### Task 1: Routing service constants and flow-control frame decoders

**Files:**
- Modify: `crates/knx-net/src/core/services.rs` (add constants near the existing `SEARCH_REQUEST`/`SEARCH_RESPONSE` block)
- Create: `crates/knx-net/src/routing.rs`
- Modify: `crates/knx-net/src/lib.rs` (add `pub mod routing;`)

**Interfaces:**
- Consumes: nothing new — pure byte-slice decoding only.
- Produces (for Task 3): `crate::core::services::{ROUTING_INDICATION: u16 = 0x0530, ROUTING_LOST_MESSAGE: u16 = 0x0531, ROUTING_BUSY: u16 = 0x0532}`; `crate::routing::{RoutingLostMessage { device_state: u8, lost_message_count: u16 }, RoutingBusy { device_state: u8, wait_time_ms: u16, control_field: u16 }, RoutingError, decode_routing_lost_message(buf: &[u8]) -> Result<RoutingLostMessage, RoutingError>, decode_routing_busy(buf: &[u8]) -> Result<RoutingBusy, RoutingError>}`.

- [ ] **Step 1: Add the three service type constants**

In `crates/knx-net/src/core/services.rs`, immediately after the existing `SEARCH_RESPONSE` constant:

```rust
// Routing service type identifiers, Routing v01.05.02 AS §5.1.
pub const ROUTING_INDICATION: u16 = 0x0530;
pub const ROUTING_LOST_MESSAGE: u16 = 0x0531;
pub const ROUTING_BUSY: u16 = 0x0532;
```

- [ ] **Step 2: Write the failing tests for both decoders**

Create `crates/knx-net/src/routing.rs`:

```rust
//! `ROUTING_LOST_MESSAGE`/`ROUTING_BUSY` body decoding (Routing v01.05.02 AS
//! §6.2/§6.3) — decoded far enough to log, never acted on (design spec's
//! Cycle 4 scope cut: no send-throttle reaction to `ROUTING_BUSY`).
//! `ROUTING_INDICATION` needs no decoder here: its body is exactly an
//! `L_Data.ind` cEMI frame, already handled by `cemi::decode_l_data`/
//! `encode_l_data` and `frame::decode_frame`/`encode_frame` — `client.rs`
//! wires those together directly.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingLostMessage {
    pub device_state: u8,
    pub lost_message_count: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoutingBusy {
    pub device_state: u8,
    pub wait_time_ms: u16,
    pub control_field: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RoutingError {
    TooShort { needed: usize, got: usize },
}

impl std::fmt::Display for RoutingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RoutingError::TooShort { needed, got } => write!(
                f,
                "routing frame body too short: needed {needed} octets, got {got}"
            ),
        }
    }
}

impl std::error::Error for RoutingError {}

/// Routing v01.05.02 AS §6.2: 4-octet body — structure length (unused,
/// always 04h), device state, 2-octet lost-message count (big-endian).
pub fn decode_routing_lost_message(buf: &[u8]) -> Result<RoutingLostMessage, RoutingError> {
    if buf.len() < 4 {
        return Err(RoutingError::TooShort {
            needed: 4,
            got: buf.len(),
        });
    }
    Ok(RoutingLostMessage {
        device_state: buf[1],
        lost_message_count: u16::from_be_bytes([buf[2], buf[3]]),
    })
}

/// Routing v01.05.02 AS §5.4/§6.3: 6-octet body — structure length
/// (unused), device state, 2-octet wait time in ms, 2-octet control field
/// (both big-endian).
pub fn decode_routing_busy(buf: &[u8]) -> Result<RoutingBusy, RoutingError> {
    if buf.len() < 6 {
        return Err(RoutingError::TooShort {
            needed: 6,
            got: buf.len(),
        });
    }
    Ok(RoutingBusy {
        device_state: buf[1],
        wait_time_ms: u16::from_be_bytes([buf[2], buf[3]]),
        control_field: u16::from_be_bytes([buf[4], buf[5]]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Routing v01.05.02 AS §6.2's worked example: structure length 04h,
    /// device state 00h, lost-message count 0005h.
    #[test]
    fn decode_routing_lost_message_matches_spec_worked_example() {
        let buf = [0x04, 0x00, 0x00, 0x05];
        let msg = decode_routing_lost_message(&buf).unwrap();
        assert_eq!(msg.device_state, 0x00);
        assert_eq!(msg.lost_message_count, 5);
    }

    #[test]
    fn decode_routing_lost_message_rejects_too_short_buffer() {
        let err = decode_routing_lost_message(&[0x04, 0x00, 0x00]).unwrap_err();
        assert_eq!(
            err,
            RoutingError::TooShort {
                needed: 4,
                got: 3
            }
        );
    }

    /// Hand-built from §5.4's field layout: structure length 04h, device
    /// state 01h, wait time 100ms (0064h), control field 0000h.
    #[test]
    fn decode_routing_busy_matches_hand_built_fixture() {
        let buf = [0x04, 0x01, 0x00, 0x64, 0x00, 0x00];
        let busy = decode_routing_busy(&buf).unwrap();
        assert_eq!(busy.device_state, 0x01);
        assert_eq!(busy.wait_time_ms, 100);
        assert_eq!(busy.control_field, 0x0000);
    }

    #[test]
    fn decode_routing_busy_rejects_too_short_buffer() {
        let err = decode_routing_busy(&[0x04, 0x01, 0x00, 0x64, 0x00]).unwrap_err();
        assert_eq!(
            err,
            RoutingError::TooShort {
                needed: 6,
                got: 5
            }
        );
    }
}
```

In `crates/knx-net/src/lib.rs`, add `pub mod routing;` alongside the existing `pub mod discovery;`/`pub mod tunnelling;` lines (module only — nothing from it is re-exported at the crate root yet, since Task 3 is its only consumer and reaches it via `crate::routing`).

- [ ] **Step 3: Run the tests and confirm they pass**

Run: `cargo test -p knx-net routing::`
Expected: 4 passed (the module compiles standalone — no dependency on Task 3's `client.rs` changes).

- [ ] **Step 4: Commit**

```bash
git add crates/knx-net/src/core/services.rs crates/knx-net/src/routing.rs crates/knx-net/src/lib.rs
git commit -m "feat(knx-net): ROUTING_INDICATION/LOST_MESSAGE/BUSY constants + flow-control decoders

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 2: `RoutingClient` — connect, send, receive

**Files:**
- Modify: `Cargo.toml` (workspace root — add `socket2` to `[workspace.dependencies]`)
- Modify: `crates/knx-net/Cargo.toml` (add `socket2.workspace = true` to `[dependencies]`)
- Modify: `crates/knx-net/src/client.rs` (add `RoutingClient`, `BusConnection::connect_routing`, the loopback round-trip test)
- Modify: `crates/knx-net/src/lib.rs` (re-export `RoutingClient`)

**Interfaces:**
- Consumes: `crate::core::services::{ROUTING_INDICATION, ROUTING_LOST_MESSAGE, ROUTING_BUSY}` (Task 1); `crate::routing::{decode_routing_lost_message, decode_routing_busy}` (Task 1); `crate::cemi::{encode_l_data, decode_l_data, LDataFrame, LDataMessageKind, Destination, ApplicationService}` (existing); `crate::frame::{encode_frame, decode_frame}` (existing); `knx_core::IndividualAddress` (existing).
- Produces (for Task 3): `pub struct RoutingClient` with `pub fn subscribe(&self) -> broadcast::Receiver<LDataFrame>` and `pub async fn send(&self, destination: Destination, service: ApplicationService) -> Result<(), BusError>`; `BusConnection::connect_routing(&self, own_address: IndividualAddress) -> Result<RoutingClient, BusError>`.

- [ ] **Step 1: Add the `socket2` dependency**

In the workspace root `Cargo.toml`, in `[workspace.dependencies]` (alphabetical-ish, next to `serde`/`sha2` is fine — this workspace's list isn't strictly sorted):

```toml
socket2 = "0.6"
```

In `crates/knx-net/Cargo.toml`, add to `[dependencies]`:

```toml
socket2 = { workspace = true, features = ["all"] }
```

(`"all"` pulls in `set_reuse_address`/multicast helpers on every platform this workspace targets — Linux is the only CI/dev target today, no reason to hand-pick a narrower feature set.)

- [ ] **Step 2: Run a build to confirm the dependency resolves**

Run: `cargo build -p knx-net`
Expected: builds clean, `Cargo.lock` gains a `knx-net -> socket2` edge (the crate itself was already in the lockfile transitively via tokio, so no new crate is *downloaded*, just a new direct edge).

- [ ] **Step 3: Write the failing loopback round-trip test**

In `crates/knx-net/src/client.rs`, inside the existing `#[cfg(test)] mod tests` block (after the existing `local_discovery_hpai_resolves_a_real_ip_and_keeps_the_real_port` test), add:

```rust
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

        let sender = match KnxNetIpClient::new()
            .connect_routing(sender_address)
            .await
        {
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

        let received = tokio::time::timeout(Duration::from_secs(5), telegrams.recv())
            .await
            .expect("should receive the sent telegram within 5s")
            .expect("broadcast channel still open");
        assert_eq!(received.source, sender_address);
        assert_eq!(received.destination, Destination::Group(group_address));
        assert_eq!(
            received.service,
            ApplicationService::GroupValueWrite(GroupValue::Short(1))
        );
    }
```

- [ ] **Step 4: Run it to confirm it fails to compile (no `connect_routing` yet)**

Run: `cargo test -p knx-net --lib routing_client_sends_and_receives_a_group_value_write`
Expected: compile error, `no method named 'connect_routing' found`.

- [ ] **Step 5: Implement `RoutingClient` and `connect_routing`**

In `crates/knx-net/src/client.rs`:

Add the multicast constant near the existing `DISCOVERY_MULTICAST` (they're the same address/port — Routing v01.05.02 AS §2.3.1 confirms discovery and routing share the KNXnet/IP System Setup Multicast Address — but keep a separately named constant since the two features are conceptually distinct and Task 1's routing module may one day need its own override independent of discovery's):

```rust
/// Standard KNXnet/IP routing multicast group and port (Routing v01.05.02
/// AS §2.3.1) — same address/port discovery already uses (Core v01.06.02
/// AS §4.2), kept as its own named constant since routing and discovery
/// are separate features that happen to share a default today.
const ROUTING_MULTICAST: SocketAddrV4 = SocketAddrV4::new(Ipv4Addr::new(224, 0, 23, 12), 3671);
```

Add `connect_routing` to the `BusConnection` trait definition:

```rust
#[allow(async_fn_in_trait)]
pub trait BusConnection {
    async fn discover(&self) -> Result<Vec<DiscoveredGateway>, BusError>;
    async fn connect_tunnel(&self, gateway: SocketAddrV4) -> Result<TunnelClient, BusError>;
    async fn connect_routing(&self, own_address: IndividualAddress) -> Result<RoutingClient, BusError>;
}
```

Implement it in `impl BusConnection for KnxNetIpClient`, after the existing `connect_tunnel`:

```rust
    async fn connect_routing(&self, own_address: IndividualAddress) -> Result<RoutingClient, BusError> {
        RoutingClient::connect(own_address).await
    }
```

Add the state and public type, next to `TunnelState`/`TunnelClient`:

```rust
struct RoutingState {
    socket: UdpSocket,
    tx: broadcast::Sender<LDataFrame>,
    shutdown: Notify,
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
        socket2_socket.set_reuse_address(true).map_err(BusError::Io)?;
        socket2_socket
            .bind(&std::net::SocketAddr::from((Ipv4Addr::UNSPECIFIED, ROUTING_MULTICAST.port())).into())
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
        let frame = LDataFrame {
            kind: cemi::LDataMessageKind::Indication,
            source: self.own_address,
            destination,
            service,
        };
        let datagram = frame::encode_frame(services::ROUTING_INDICATION, &cemi::encode_l_data(&frame));
        self.state
            .socket
            .send_to(&datagram, ROUTING_MULTICAST)
            .await
            .map_err(BusError::Io)?;
        Ok(())
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
                }
            }
            _ => {} // unsupported/unknown service type: ignore
        }
    }
}
```

Update `KnxNetIpClient::new`'s doc comment and the trait's doc comment if they enumerate methods by name (check the existing comment above `pub struct KnxNetIpClient` and update it to mention `connect_routing` alongside `discover`/`connect_tunnel`, same style as the Cycle 3 edit that added `discover` to that same sentence).

In `crates/knx-net/src/lib.rs`, add `RoutingClient` to the existing `pub use client::{...}` line:

```rust
pub use client::{BusConnection, BusError, DiscoveredGateway, KnxNetIpClient, RoutingClient, TunnelClient};
```

- [ ] **Step 6: Run the test and confirm it passes**

Run: `cargo test -p knx-net --lib routing_client_sends_and_receives_a_group_value_write`
Expected: PASS (or a printed skip line if the sandbox has no loopback multicast route — check which one actually happened before moving on; if it skipped, that's an environment limitation to note, not a task failure).

- [ ] **Step 7: Run the full crate test suite**

Run: `cargo test -p knx-net`
Expected: all pass (existing tunnelling/discovery/cemi tests unaffected).

- [ ] **Step 8: Commit**

```bash
git add Cargo.toml Cargo.lock crates/knx-net/Cargo.toml crates/knx-net/src/client.rs crates/knx-net/src/lib.rs
git commit -m "feat(knx-net): RoutingClient — connect_routing send/receive over multicast

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 3: `knx bus route-monitor` and `knx bus route-send`

**Files:**
- Modify: `apps/knx-cli/src/main.rs` (add two subcommands, extend `USAGE`)

**Interfaces:**
- Consumes: `knx_net::{BusConnection, KnxNetIpClient, RoutingClient, ApplicationService, Destination}` (Task 2); `knx_core::IndividualAddress` (`FromStr`, existing); the existing `format_telegram`, `load_group_address_names`, `parse_group_value`, `take_value` helpers (all pre-existing, unchanged).
- Produces: nothing further consumes this — it's the CLI-facing leaf.

- [ ] **Step 1: Extend `USAGE` and the `bus` dispatcher**

In `apps/knx-cli/src/main.rs`, extend the `USAGE` constant (after the existing `knx bus write` line):

```rust
     \x20     knx bus route-monitor --source-address <area.line.device> [--project <path.knxdb>]\n\
     \x20     knx bus route-send --source-address <area.line.device> <main/middle/sub> <0|1|hex>\n\
```

Extend `run_bus`'s match:

```rust
fn run_bus(args: &[String]) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("discover") => run_bus_discover(&args[1..]),
        Some("monitor") => run_bus_monitor(&args[1..]),
        Some("write") => run_bus_write(&args[1..]),
        Some("route-monitor") => run_bus_route_monitor(&args[1..]),
        Some("route-send") => run_bus_route_send(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::FAILURE
        }
    }
}
```

- [ ] **Step 2: Implement `route-monitor`**

Add after `run_bus_monitor_async` (mirrors its structure closely — same arg struct shape as `BusMonitorArgs`, `--gateway` replaced by `--source-address`):

```rust
struct BusRouteMonitorArgs {
    source_address: String,
    project: Option<String>,
}

fn parse_bus_route_monitor_args(args: &[String]) -> Result<BusRouteMonitorArgs, String> {
    let mut source_address = None;
    let mut project = None;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--source-address" => {
                source_address = Some(take_value(args, i + 1, "--source-address")?);
                i += 2;
            }
            "--project" => {
                project = Some(take_value(args, i + 1, "--project")?);
                i += 2;
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }
    Ok(BusRouteMonitorArgs {
        source_address: source_address
            .ok_or_else(|| "--source-address is required".to_string())?,
        project,
    })
}

fn run_bus_route_monitor(args: &[String]) -> ExitCode {
    let parsed = match parse_bus_route_monitor_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let own_address: knx_core::IndividualAddress = match parsed.source_address.parse() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("--source-address must be area.line.device, e.g. 1.1.1");
            return ExitCode::FAILURE;
        }
    };
    let ga_names: std::collections::HashMap<u16, String> = match &parsed.project {
        Some(path) => match load_group_address_names(Path::new(path)) {
            Ok(names) => names,
            Err(e) => {
                eprintln!("could not load project {path}: {e}");
                return ExitCode::FAILURE;
            }
        },
        None => std::collections::HashMap::new(),
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("could not start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(run_bus_route_monitor_async(own_address, ga_names))
}

async fn run_bus_route_monitor_async(
    own_address: knx_core::IndividualAddress,
    ga_names: std::collections::HashMap<u16, String>,
) -> ExitCode {
    use knx_net::BusConnection;
    let client = knx_net::KnxNetIpClient::new();
    let routing = match client.connect_routing(own_address).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("could not join the routing multicast group: {e}");
            return ExitCode::FAILURE;
        }
    };
    eprintln!("joined routing multicast as {own_address}. Ctrl-C to stop.");
    let mut telegrams = routing.subscribe();
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            received = telegrams.recv() => match received {
                Ok(telegram) => println!("{}", format_telegram(&telegram, &ga_names)),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    eprintln!("warning: {n} telegram(s) dropped (receiver too slow)");
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
        }
    }
    ExitCode::SUCCESS
}
```

- [ ] **Step 3: Implement `route-send`**

Add after `run_bus_write_async`:

```rust
struct BusRouteSendArgs {
    source_address: String,
    group_address: String,
    value: String,
}

fn parse_bus_route_send_args(args: &[String]) -> Result<BusRouteSendArgs, String> {
    let mut source_address = None;
    let mut positional = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--source-address" => {
                source_address = Some(take_value(args, i + 1, "--source-address")?);
                i += 2;
            }
            other => {
                positional.push(other.to_string());
                i += 1;
            }
        }
    }
    let [group_address, value] = &positional[..] else {
        return Err("expected exactly one group address and one value".to_string());
    };
    Ok(BusRouteSendArgs {
        source_address: source_address
            .ok_or_else(|| "--source-address is required".to_string())?,
        group_address: group_address.clone(),
        value: value.clone(),
    })
}

fn run_bus_route_send(args: &[String]) -> ExitCode {
    let parsed = match parse_bus_route_send_args(args) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("{e}\n{USAGE}");
            return ExitCode::FAILURE;
        }
    };
    let own_address: knx_core::IndividualAddress = match parsed.source_address.parse() {
        Ok(a) => a,
        Err(_) => {
            eprintln!("--source-address must be area.line.device, e.g. 1.1.1");
            return ExitCode::FAILURE;
        }
    };
    let group_address = match knx_core::GroupAddress::parse(
        &parsed.group_address,
        knx_core::GroupAddressStyle::ThreeLevel,
    ) {
        Ok(ga) => ga,
        Err(e) => {
            eprintln!("invalid group address {}: {e}", parsed.group_address);
            return ExitCode::FAILURE;
        }
    };
    let value = match parse_group_value(&parsed.value) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            return ExitCode::FAILURE;
        }
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(rt) => rt,
        Err(e) => {
            eprintln!("could not start async runtime: {e}");
            return ExitCode::FAILURE;
        }
    };
    runtime.block_on(run_bus_route_send_async(own_address, group_address, value))
}

async fn run_bus_route_send_async(
    own_address: knx_core::IndividualAddress,
    group_address: knx_core::GroupAddress,
    value: knx_net::GroupValue,
) -> ExitCode {
    use knx_net::{ApplicationService, BusConnection, Destination};
    let client = knx_net::KnxNetIpClient::new();
    let routing = match client.connect_routing(own_address).await {
        Ok(r) => r,
        Err(e) => {
            eprintln!("could not join the routing multicast group: {e}");
            return ExitCode::FAILURE;
        }
    };
    // Routing is unconfirmed (Routing v01.05.02 AS §5.1) — "sent", not
    // "wrote", since there's no ACK to confirm delivery, unlike tunnelling.
    match routing
        .send(
            Destination::Group(group_address),
            ApplicationService::GroupValueWrite(value),
        )
        .await
    {
        Ok(()) => {
            println!(
                "sent to {}",
                group_address.format(knx_core::GroupAddressStyle::ThreeLevel)
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("send failed: {e}");
            ExitCode::FAILURE
        }
    }
}
```

- [ ] **Step 4: Build and smoke-check argument parsing**

Run: `cargo build -p knx-cli`
Expected: builds clean.

Run: `cargo run -p knx-cli -- bus route-send`
Expected: prints `--source-address is required` followed by `USAGE`, exits non-zero — confirms the arg-parsing error path without needing a real network.

Run: `cargo run -p knx-cli -- bus route-send --source-address 1.1.1`
Expected: prints `expected exactly one group address and one value`, exits non-zero.

- [ ] **Step 5: Commit**

```bash
git add apps/knx-cli/src/main.rs
git commit -m "feat(knx-cli): 'bus route-monitor'/'bus route-send' subcommands

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```

---

### Task 4: Workspace-wide verification and documentation

**Files:**
- Modify: `docs/ARCHITECTURE.md` (§8 KNXnet/IP)
- Modify: `docs/KNOWN_LIMITATIONS.md` (§26)
- Modify: `docs/IMPLEMENTATION_STATUS.md` (new Cycle 4 entry)
- Modify: `docs/ROADMAP.md` (Session 6 status line)

**Interfaces:**
- Consumes: nothing — documentation only, no code interfaces.
- Produces: nothing further consumes this.

- [ ] **Step 1: Run the full workspace verification suite**

Run:
```bash
cargo build --workspace
cargo test --workspace
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- check-layering
cargo deny check
```
Expected: all pass. If `cargo deny check` flags `socket2`'s license, resolve it before continuing (check its license against this workspace's `deny.toml` allow-list; `socket2` is MIT/Apache-2.0 dual-licensed, which should already be allowed since `tokio` itself depends on it transitively).

- [ ] **Step 2: Update `docs/ARCHITECTURE.md` §8**

Replace the sentence:
> `TunnelClient::send` (Cycle 2) and `discover` (Cycle 3, multicast `SEARCH_REQUEST`/`SEARCH_RESPONSE`) are both implemented now; routing and KNX IP Secure remain later cycles.

with:
> `TunnelClient::send` (Cycle 2), `discover` (Cycle 3, multicast `SEARCH_REQUEST`/`SEARCH_RESPONSE`), and `connect_routing` (Cycle 4, unconfirmed `ROUTING_INDICATION` over the standard routing multicast group) are all implemented now; KNX IP Secure remains out of scope, handled separately by `knx-secure`.

- [ ] **Step 3: Update `docs/KNOWN_LIMITATIONS.md` §26**

Rename the heading from `## 26. \`BusConnection\` does not yet support routing or KNX IP Secure` to `## 26. \`BusConnection\` does not yet support KNX IP Secure`.

Replace the **Limitation** paragraph's last two sentences (`Routing and all secure-protocol paths remain \`BusError::NotImplemented\` stubs. A reader should not assume the trait is feature-complete because it compiles.`) with:

> `connect_routing` (Cycle 4) sends/receives unconfirmed `ROUTING_INDICATION` frames over the standard multicast group — no custom multicast address override, and `ROUTING_BUSY` is decoded and logged but never used to throttle sends (see the two new limitation entries below). Secure-protocol paths remain unimplemented. A reader should not assume the trait is feature-complete because it compiles.

Replace the **Cause** paragraph's last sentence (`Routing is still a later cycle. Secure protocols are out of v1 scope, handled by the isolated \`knx-secure\` crate.`) with:

> Cycle 4 added routing. Secure protocols are out of v1 scope, handled by the isolated `knx-secure` crate.

In the **Impact** paragraph, replace `Routing (multicast) and KNX IP Secure remain unreachable regardless.` with `KNX IP Secure remains unreachable regardless; routing (unencrypted multicast) is reachable as of Cycle 4.`

Replace the **Lifted when** paragraph with:

> KNX IP Secure lands in a later cycle of Session 6, or in Session 7.

Then add two new limitation entries after this one (renumber nothing else — these are new entries appended at the end of the file's numbered sequence, following this document's existing "each limitation gets the next number" convention — check the last entry number in the file and use the next two):

```markdown
## <N>. KNXnet/IP routing has no custom multicast address override

**Limitation.** `RoutingClient::connect_routing` always joins the standard
KNXnet/IP System Setup Multicast Address, `224.0.23.12:3671` (Routing
v01.05.02 AS §2.3.1). No CLI flag or API parameter selects a different
group.

**Cause.** Session 6 Cycle 4's design spec deliberately hardcoded it,
same call as Cycle 3's discovery multicast address — no environment here
needs a non-default group.

**Impact.** A KNX installation using a custom routing multicast address
(needed only past 180 KNX subnetworks, or when multiple installations
share one IP network, per §2.3.2) cannot be reached by `route-monitor`/
`route-send` yet.

**Lifted when.** A real setup needs a non-default group — no fixed cycle.

## <N+1>. `ROUTING_BUSY` is logged, not honored, by `RoutingClient`

**Limitation.** Routing v01.05.02 AS §2.3.5 requires any KNX IP device to
stop sending `ROUTING_INDICATION` for a received `tw` after a
`ROUTING_BUSY` frame. `RoutingClient` decodes and logs `ROUTING_BUSY` (and
`ROUTING_LOST_MESSAGE`) but never reacts to either.

**Cause.** Session 6 Cycle 4's design spec deliberately cut this: the CLI
sends occasional single telegrams, not a sustained flood, so the failure
mode the spec guards against barely applies to this tool's actual usage.

**Impact.** In a busy installation already under flow-control pressure
from other devices, `route-send` could still add to that pressure instead
of backing off. Low risk given the CLI's own send pattern; would matter
more if `RoutingClient` were ever driven by something that sends in a
tight loop.

**Lifted when.** A caller that sends fast enough for this to matter
exists — no fixed cycle.
```

- [ ] **Step 4: Update `docs/IMPLEMENTATION_STATUS.md`**

After the existing "Session 6, Cycle 3" section's "Known gaps added this cycle" bullet list (immediately before `## Next session`), add:

```markdown
**Session 6, Cycle 4 (2026-09-06) — KNXnet/IP routing.** Own design spec
(`docs/superpowers/specs/2026-09-06-knxnet-ip-routing-design.md`), per the
brainstorming skill's classification: a new subsystem, not an extension
of the existing tunnel connection. `routing.rs` decodes
`ROUTING_LOST_MESSAGE`/`ROUTING_BUSY` far enough to log them;
`ROUTING_INDICATION` needed no new codec at all, since its body is
exactly an `L_Data.ind` cEMI frame — the same `cemi::encode_l_data`/
`decode_l_data` Cycles 1-2 already built. `RoutingClient` joins the
standard routing multicast group (`224.0.23.12:3671`, shared with
discovery's default) with multicast loopback disabled, and implements
`BusConnection::connect_routing(own_address)` — `own_address` is a
required parameter, not negotiated, since routing has no
`CONNECT_REQUEST`/`CRD` handshake to assign one through the way
tunnelling does. `RoutingClient::send` is a single unconfirmed multicast
send with no ACK wait and no retry (Routing v01.05.02 AS §5.1 marks the
service unconfirmed outright, unlike Tunnelling's `TUNNELLING_REQUEST`/
`ACK` pair). `knx bus route-monitor --source-address <addr>` and
`knx bus route-send --source-address <addr> <ga> <value>` are the
CLI-facing pieces. Unlike Cycles 1-3, this cycle's core round trip
(`RoutingClient` send/receive) is tested on loopback multicast directly —
no real gateway needed, since routing is plain UDP multicast rather than
a protocol exchange with one specific peer.

Known gaps added this cycle (not bugs, scope decisions):

- No `--multicast` override for a non-default routing multicast address —
  hardcoded to the standard group (KNOWN_LIMITATIONS.md).
- `ROUTING_BUSY` is decoded and logged, never used to throttle sends
  (KNOWN_LIMITATIONS.md).
- Whether the reference gateway (`192.0.2.1`) supports routing at all
  is unconfirmed — tunnelling and discovery are verified against it,
  routing isn't yet. Manual verification (same policy as Cycles 2-3: a
  human chooses when to probe the LAN) is left for the user:
  `cargo run -p knx-cli -- bus route-monitor --source-address <spare-address>`
  against a running installation.
```

Then, in the `## Next session` section immediately following, update the "Session 6 is in progress (Cycles 1-3 done)" sentence to "Cycles 1-4 done".

- [ ] **Step 5: Update `docs/ROADMAP.md`**

In the "Session 6 — KNXnet/IP" section's **Status** paragraph, after the existing sentence about Cycle 2, add:

> Cycle 3 (2026-09-06) delivered discovery: `core::dib` DIB decoding,
> `KnxNetIpClient::discover`, and a `knx bus discover` CLI subcommand.
> Cycle 4 (2026-09-06) delivered routing: `RoutingClient` over the
> standard multicast group, and `knx bus route-monitor`/`route-send` CLI
> subcommands. KNX IP Secure remains for a later cycle.

(If Cycle 3 is already described in this paragraph from a prior update, only append the Cycle 4 sentence and adjust "Discovery, routing, and KNX IP Secure remain for later cycles." at the end of the Status paragraph to "KNX IP Secure remains for a later cycle.")

- [ ] **Step 6: Commit**

```bash
git add docs/ARCHITECTURE.md docs/KNOWN_LIMITATIONS.md docs/IMPLEMENTATION_STATUS.md docs/ROADMAP.md
git commit -m "docs: Session 6 Cycle 4 KNXnet/IP routing shipped

Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
Claude-Session: https://claude.ai/code/session_019Armveec8wNNuGLVN7WhQv"
```
