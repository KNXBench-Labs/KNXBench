# Session 6, Cycle 4 — KNXnet/IP Routing

**Status.** Approved, not yet implemented.

## Goal

Send and receive KNX telegrams over the standard KNXnet/IP routing multicast
group, closing the gap Cycle 1 deliberately left open (`connect_routing`
does not exist yet; there is no routing path at all today —
`BusConnection` only has `discover`/`connect_tunnel`). Unlike tunnelling,
routing needs no gateway to dial: any device on the same IP segment that
joins the multicast group sees every telegram. Two new `knx bus`
subcommands, `route-monitor` and `route-send`, give the CLI a
gateway-free way to watch and inject telegrams.

## Scope

**In.**

* `ROUTING_INDICATION` (0x0530, Routing v01.05.02 AS §5.2) both directions:
  receive (decode into the same `LDataFrame` tunnelling already uses) and
  send (encode via the existing `cemi::encode_l_data`).
* The standard KNXnet/IP System Setup Multicast Address, `224.0.23.12:3671`
  (§2.3.1), hardcoded — not a CLI flag this cycle, same call as Cycle 3's
  discovery multicast address.
* Decoding `ROUTING_LOST_MESSAGE` (0x0531, §6.2) and `ROUTING_BUSY`
  (0x0532, §6.3) far enough to log them (device state, lost-message count /
  wait time) when received. No send-side reaction to either.
* `RoutingClient`, a new sibling to `TunnelClient` implementing send/receive
  over the joined multicast socket.
* `BusConnection::connect_routing(&self, own_address: IndividualAddress) ->
  Result<RoutingClient, BusError>`.
* `knx bus route-monitor --source-address <addr> [--project <path>]` and
  `knx bus route-send --source-address <addr> <group-addr> <value>`.

**Out (deliberately, later cycles or never).**

* A `--multicast` override for a non-default routing multicast address
  (relevant only past 180 KNX subnetworks or multiple installations
  sharing one IP network, §2.3.2) — no environment here needs it; the
  standard group is hardcoded, same as discovery's Cycle 3 call.
* Reacting to `ROUTING_BUSY` by throttling sends (§2.3.5) — this tool sends
  occasional single telegrams via CLI, not a sustained flood, so the
  failure mode the spec guards against barely applies. Decoded and logged,
  not acted on.
* `ROUTING_LOST_MESSAGE`/`ROUTING_BUSY` on the `subscribe()` telegram
  stream — they're logged straight to stderr, never surfacing as an
  `LDataFrame` (they aren't telegrams).
* The KNX Routing Counter's decrement-per-hop behavior (§3.9) — that's a
  KNXnet/IP *router's* job, forwarding between an IP segment and a KNX
  subnetwork. This client is an IP-side endpoint like `TunnelClient`, not a
  router; it always emits the same fixed hop count `cemi::encode_l_data`
  already defaults to (6), never decrements one it forwards.
* KNX IP Secure — separate spec, unresearched (KNOWN_LIMITATIONS.md §26,
  RESEARCH.md).
* Any UI wiring beyond the CLI — same reasoning as Cycles 1-3: prove it
  against real traffic first.

## Ground truth

`/home/knxbench/knx-ai/extracted_clean/03_08_05 Routing v01.05.02 AS.md`:

* §2.3.1 — every installation uses the same multicast address/port for
  routing; port 3671 is IANA-registered for it.
* §3.8 — telegrams from the KNX subnet are transported as cEMI
  `L_Data.ind` (message code 0x29) frames; §5.2/§6.1 confirm
  `ROUTING_INDICATION`'s body is exactly that cEMI frame, nothing more —
  no HPAI, no channel ID, no sequence counter (contrast with
  `TUNNELLING_REQUEST`, which wraps the cEMI frame in a connection header).
* §5.1's service table marks `ROUTING_INDICATION` "unconfirmed" — no ACK,
  no retry, unlike `TunnelClient::send`'s wait/retry-once rule (Tunnelling
  v01.07.01 AS §2.6).
* §6.2/§6.3 — the exact byte layouts for `ROUTING_LOST_MESSAGE` (4-byte
  body: 1-byte structure length, 1-byte device state, 2-byte lost-message
  count) and `ROUTING_BUSY` (6-byte body: 1-byte structure length, 1-byte
  device state, 2-byte wait time in ms, 2-byte control field).
* §2.3.5 — `ROUTING_BUSY` flow control: a device SHALL stop sending
  `ROUTING_INDICATION` for `tw` after receiving one with control field
  `0000h`. Read for the "out of scope" call above, not implemented.

Section/table numbers cited, spec prose never reproduced beyond short
structural fragments — same handling rule as Cycles 1-3.

## Architecture

```
crates/knx-net/
  src/
    core/
      services.rs   // +ROUTING_INDICATION/LOST_MESSAGE/BUSY constants
    routing.rs        // new: decode_routing_lost_message, decode_routing_busy
    client.rs          // +RoutingClient, +BusConnection::connect_routing
```

`routing.rs` holds only the two log-only decoders — `ROUTING_INDICATION`
needs no wrapper of its own since its body *is* a cEMI frame, already
handled by `cemi::encode_l_data`/`decode_l_data` and `frame::encode_frame`/
`decode_frame`. Same pure-function convention as every other codec module
in this crate: byte slice in, typed struct out, hand-rolled `Display` +
`std::error::Error` per error type.

```rust
pub struct RoutingLostMessage {
    pub device_state: u8,
    pub lost_message_count: u16,
}

pub struct RoutingBusy {
    pub device_state: u8,
    pub wait_time_ms: u16,
    pub control_field: u16,
}

pub enum RoutingError { TooShort, WrongStructureLength(u8) }

pub fn decode_routing_lost_message(buf: &[u8]) -> Result<RoutingLostMessage, RoutingError>;
pub fn decode_routing_busy(buf: &[u8]) -> Result<RoutingBusy, RoutingError>;
```

**`BusConnection`** gains one method:

```rust
async fn connect_routing(&self, own_address: IndividualAddress) -> Result<RoutingClient, BusError>;
```

`own_address` is required, not defaulted — routing has no `CONNECT_REQUEST`/
`CRD` handshake to assign one the way tunnelling does (design Q2), so the
caller must supply the individual address these outgoing frames should
claim as their source.

**`RoutingClient::connect_routing`** (in `client.rs`, next to
`TunnelClient::connect`):

1. Bind a UDP socket to `0.0.0.0:3671` with `SO_REUSEADDR` (so more than
   one process, e.g. this client and a real router, can share the port on
   the same host — standard practice for multicast receivers).
2. `join_multicast_v4(224.0.23.12, INADDR_ANY)`.
3. `set_multicast_loop_v4(false)` — without this, our own sends would loop
   back through the same socket and appear in `subscribe()` as if some
   other device sent them.
4. Spawn `receive_loop`, store `own_address`. No heartbeat task — routing
   has no keep-alive concept (§2.3.2: routers "do not establish
   communication channels between each other").

```rust
pub struct RoutingClient {
    socket: Arc<UdpSocket>,
    own_address: IndividualAddress,
    tx: broadcast::Sender<LDataFrame>,
    shutdown: Arc<Notify>,
}
```

`RoutingClient::send(destination, service)`:

```rust
pub async fn send(&self, destination: Destination, service: ApplicationService) -> Result<(), BusError> {
    let frame = LDataFrame { kind: LDataMessageKind::Indication, source: self.own_address, destination, service };
    let datagram = frame::encode_frame(services::ROUTING_INDICATION, &cemi::encode_l_data(&frame));
    self.socket.send_to(&datagram, ROUTING_MULTICAST).await.map_err(BusError::Io)
}
```

One `send_to`, no ACK wait, no retry — `ROUTING_INDICATION` is
unconfirmed (ground truth above). `kind: Indication` because every routing
frame on the wire is an `L_Data.ind` regardless of direction (§3.8) —
there's no separate `.req` form the way tunnelling's client→server leg
uses one.

`RoutingClient::subscribe()` — identical shape to `TunnelClient::subscribe`.
Its `receive_loop` decodes the KNXnet/IP header first; on
`ROUTING_INDICATION` it decodes the cEMI body via the existing
`cemi::decode_l_data` and pushes to `tx`; on `ROUTING_LOST_MESSAGE`/
`ROUTING_BUSY` it decodes via the new `routing.rs` functions and
`eprintln!`s a line, then continues the loop; anything else (malformed
frame, unknown service type) is skipped, same "don't crash on one bad
datagram" rule Cycle 3's discovery already applies.

No `disconnect()` method — routing is connectionless (§2.3.2, no
communication channel to tear down). Dropping `RoutingClient` closes the
socket; the OS sends the IGMP leave as part of that, same as any other
multicast consumer shutting down. `Drop` still signals `shutdown` to stop
`receive_loop`, mirroring `TunnelClient`'s safety-net pattern.

## Error handling

No new `BusError` variant. `Io` covers bind/join/send failures,
`Protocol(String)` covers a `ROUTING_INDICATION` whose cEMI body fails to
decode (logged and skipped in `receive_loop`, never fatal — same rule
Cycle 1's tunnelling receive and Cycle 3's discovery already apply to a
malformed inbound datagram).

## CLI integration

Two new subcommands in `apps/knx-cli/src/main.rs`, mirroring
`monitor`/`write`'s existing arg-parsing and telegram-formatting code
(`format_telegram`, `load_group_address_names`, `parse_group_value`) with
`--gateway` replaced by `--source-address` and no dial-in step:

```
knx bus route-monitor --source-address <indiv-addr> [--project <path.knxdb>]
knx bus route-send --source-address <indiv-addr> <main/middle/sub> <0|1|hex>
```

`route-monitor` joins the multicast group, prints `format_telegram`'s
existing output for every received telegram until Ctrl-C, same loop shape
as `run_bus_monitor_async` minus the `tunnel.disconnect()` call on exit
(nothing to disconnect). `route-send` joins, sends one
`GroupValueWrite`, and exits — no round trip to wait for, since the
service is unconfirmed; the CLI reports "sent" rather than "wrote" to
avoid implying a confirmation that doesn't exist for routing.

## Testing

* `routing.rs` unit tests for `decode_routing_lost_message`/
  `decode_routing_busy` against hand-built fixtures matching §6.2's worked
  example (`04 00 00 05` → device_state 0, lost_message_count 5) and a
  constructed `ROUTING_BUSY` fixture from §5.4's field layout, plus a
  too-short-buffer case for each.
* A `client.rs` integration test: two `RoutingClient`s on `127.0.0.1`
  (loopback supports multicast; `224.0.23.12` joined on the loopback
  interface works without any real network segment) — one sends, the
  other's `subscribe()` receives it. Unlike tunnelling/discovery, this
  cycle's core round trip is testable without hardware, since it's plain
  UDP multicast rather than a protocol exchange with a specific gateway.
* One more `#[ignore]`-gated case in `tests/live_gateway.rs`, only if the
  reference gateway (`192.0.2.1`, RESEARCH.md §8.1) turns out to
  support routing as well as tunnelling — confirmed during implementation,
  not assumed here.
* `knx-cli`'s two new subcommands get a manual-check entry in
  `IMPLEMENTATION_STATUS.md`, same pattern as `monitor`/`write`/`discover`.

## Documentation impact

* `docs/ARCHITECTURE.md` §8: update `BusConnection`'s method list and
  status.
* `docs/IMPLEMENTATION_STATUS.md`: new Cycle 4 entry; move routing off the
  "known gaps" list.
* `docs/KNOWN_LIMITATIONS.md` §26: narrow from "routing or KNX IP Secure"
  to "KNX IP Secure" only; add two new narrowly-scoped entries — no
  custom multicast address override, and `ROUTING_BUSY` is logged but not
  honored as a send-throttle signal.
* `docs/ROADMAP.md`: Session 6 status line gains Cycle 4.

## Open questions carried forward (not blocking this cycle)

* Whether the reference gateway (RESEARCH.md §8.1) supports routing at
  all — tunnelling and discovery are confirmed against it, routing isn't
  yet.
* A `--multicast` override and `ROUTING_BUSY` throttling — both deferred
  above, no fixed cycle to pick them back up until a real setup needs
  them.
* KNX IP Secure remains its own unresearched spec (ROADMAP.md, Session 6
  entry).
