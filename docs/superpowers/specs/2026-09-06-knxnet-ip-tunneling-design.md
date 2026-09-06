# Session 6, Cycle 1 — KNXnet/IP Tunnelling (read-only)

**Status.** Approved, not yet implemented.

## Goal

First slice of Session 6 (KNXnet/IP). Connect to a real KNXnet/IP gateway over
UDP tunnelling, receive KNX telegrams, decode them, and print them resolved
against a loaded project's group address names — an own-implementation
equivalent of `monitor_bus.py`, grounded in the official KNX specification
rather than a third-party library.

## Scope

**In.**

* KNXnet/IP frame header encode/decode (`06h`/`10h`/service/length).
* Core connection lifecycle against a *known* gateway IP: `CONNECT_REQUEST`/
  `CONNECT_RESPONSE`, `CONNECTIONSTATE_REQUEST`/`RESPONSE` (heartbeat, every
  60s per spec), `DISCONNECT_REQUEST`/`RESPONSE`.
* `TUNNELLING_REQUEST`/`TUNNELLING_ACK` on the data connection, receive
  direction only.
* cEMI `L_Data.ind` decode: source/destination address, APCI
  (`GroupValueWrite`/`Read`/`Response`), payload (`DPTBinary`/`DPTArray`
  forms, matching what was already observed live — RESEARCH.md §8.1).
* A `knx-cli` subcommand that connects, decodes, resolves against a loaded
  project's group addresses, prints to stdout, and disconnects cleanly on
  Ctrl-C.

**Out (deliberately, later cycles).**

* Discovery (`SEARCH_REQUEST`/`RESPONSE`) — needs its own cycle; also
  intersects the already-documented Docker `--network host` constraint
  (ROADMAP.md, Session 6 entry).
* Sending telegrams (`GroupValueWrite` out) — read-only this cycle.
* Routing (multicast), Remote Diagnosis, KNX IP Secure — separate specs.
* Any UI wiring beyond the CLI (desktop/web bus monitor) — needs the
  `BusConnection` trait proven against real hardware first.
* Commissioning/device download — out of scope for the whole roadmap
  (RESEARCH.md §8.3), unaffected by this cycle.

## Ground truth

The official KNX Association specification (System Specifications, not
freely redistributable) is available locally at
`/home/knxbench/knx-ai/extracted_clean/`, notably:

* `03_08_02 Core v01.06.02 AS.md` — frame header, HPAI/CRI/CRD/DIB,
  connection lifecycle, heartbeat, service identifiers, error codes.
* `03_08_04 Tunnelling v01.07.01 AS.md` — tunnelling-specific CRI/CRD,
  `TUNNELLING_REQUEST`/`ACK`.
* `03_06_03 EMI_IMI v01.04.02 AS.md` — cEMI frame format (`L_Data.ind` etc.).
* `03_02_06 Communication Medium KNX IP v01.01.02 AS.md` — IPv4 HPAI
  specifics, port 3671, multicast address.

**Handling rule.** These files never enter this repository and are never
quoted verbatim beyond short structural fragments needed to justify a byte
layout. Code comments and this doc cite section/table numbers (e.g. "Core
v01.06.02 AS §7.5.1") rather than reproducing spec prose — matching the
existing rule for `knx_master.xml` (RESEARCH.md §10). A live `tcpdump`
capture against the real gateway (`192.0.2.1:3671`, already verified
reachable — RESEARCH.md §8.1) cross-checks the byte-level interpretation and
seeds regression fixtures.

## Architecture

```
crates/knx-net/
  src/
    frame.rs        // generic KNXnet/IP header: encode/decode, no IO
    core/
      hpai.rs        // Host Protocol Address Information (IPv4/UDP form)
      cri.rs         // Connection Request Information
      crd.rs         // Connection Response Data Block
      services.rs    // CONNECT_*, CONNECTIONSTATE_*, DISCONNECT_* bodies
    tunnelling.rs    // tunnelling CRI/CRD, TUNNELLING_REQUEST/ACK
    cemi.rs          // L_Data frame, APCI/TPCI extraction
    client.rs        // BusConnection trait + TunnelClient (IO, state machine)
    lib.rs
```

Every file except `client.rs` is pure: byte slice in, typed struct out (or
the reverse), synchronous, unit-testable without a socket. `client.rs` is
the only place that touches a socket, a timer, or a background task.

**`BusConnection` trait** (interface fixed in ARCHITECTURE.md §8):

```rust
#[async_trait::async_trait]
pub trait BusConnection {
    async fn discover(&self) -> Result<Vec<GatewayInfo>, BusError>;
    async fn connect_tunnel(&self, gateway: SocketAddr) -> Result<TunnelHandle, BusError>;
    async fn send(&self, telegram: Telegram) -> Result<(), BusError>;
    fn subscribe(&self) -> broadcast::Receiver<Telegram>;
}
```

This cycle implements `connect_tunnel` and `subscribe` fully. `discover` and
`send` return `BusError::NotImplemented` — an explicit, typed stub, not a
silent no-op or a `todo!()` panic, so a caller gets a catchable error rather
than a crash (CLAUDE.md: never silently discard, meaningful error handling).

**Runtime.** `tokio`, for the UDP socket, the heartbeat timer, and the
`CONNECT_REQUEST_TIMEOUT`/`CONNECTIONSTATE_REQUEST_TIMEOUT` waits (Core
v01.06.02 AS §5.4, both spec'd in seconds). `knx-net` is infrastructure, so
`xtask check-layering`'s `tokio`-ban (which applies only to `knx-core`/
`knx-projection`) does not apply here.

**Connection lifecycle (`TunnelClient`).** A small internal state machine:
`Disconnected -> Connecting -> Connected -> Disconnecting -> Disconnected`,
plus an `Error` terminal state on protocol violation (unsupported version,
sequence error) per Core v01.06.02 AS §6.2/§6.3. A background task owns the
UDP socket, decodes incoming frames, publishes `L_Data.ind` telegrams onto a
`tokio::sync::broadcast` channel (`subscribe()`'s return value), answers
`TUNNELLING_REQUEST` with `TUNNELLING_ACK` per §2.6, and runs the heartbeat
loop independently.

## Error handling

`BusError` enum: `Timeout`, `VersionNotSupported`, `SequenceError`,
`ConnectionRefused(reason)`, `NotImplemented`, `Io(std::io::Error)`,
`Protocol(String)` for malformed-frame cases (logged with the offending
bytes at `debug` level, never panicking — per Core v01.06.02 AS §6.2/§6.3,
"if an invalid data packet is received, ignore it without further action";
our client logs and continues rather than tearing down the connection for a
single bad frame).

## Testing

* Unit tests per pure module (`frame.rs`, `core/*.rs`, `tunnelling.rs`,
  `cemi.rs`) against byte fixtures built by hand from the spec's structure
  tables, cross-checked against a `tcpdump` capture of a real session.
* One `#[ignore]`-gated integration test in `knx-net` that connects to
  `192.0.2.1:3671`, waits for at least one telegram, and disconnects —
  runs only with `cargo test -- --ignored`, never in CI (no gateway there).
  First test of this kind in the repo; noted in `IMPLEMENTATION_STATUS.md`.
* `knx-cli` subcommand gets a smoke test description in
  `IMPLEMENTATION_STATUS.md`'s manual-check list (same pattern as the
  desktop UI's un-runnable-in-this-environment checks), since it needs the
  same real gateway.

## Documentation impact

* `docs/RESEARCH.md` §8: add a `[V]` entry once the live round-trip
  (connect, receive, decode, disconnect) is confirmed against the real
  gateway, citing spec section numbers, not quoting them.
* `docs/ARCHITECTURE.md` §8: update from "the work is Session 6" to what
  actually shipped, same pattern as prior sessions' ARCHITECTURE.md updates.
* `docs/IMPLEMENTATION_STATUS.md`: new cycle entry, plus the `#[ignore]`
  test and CLI manual-check additions to the "known gaps" / manual-check
  list.
* `docs/KNOWN_LIMITATIONS.md`: note that discovery, sending, routing, and
  secure tunnelling are not yet implemented, so a reader doesn't assume
  `BusConnection` is feature-complete because it compiles.

## Open questions carried forward (not blocking this cycle)

* Discovery's interaction with the Docker deployment's `--network host`
  requirement — Session 6's own carried-forward item (ROADMAP.md).
* Whether `knx-server` should eventually expose live telegrams over a
  websocket/SSE endpoint — deferred until this cycle's `BusConnection`
  proves itself against hardware; no API surface decided yet.
