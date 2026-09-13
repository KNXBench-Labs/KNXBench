# Session 6, Cycle 3 — KNXnet/IP Discovery

**Status.** Approved, not yet implemented.

## Goal

Find KNXnet/IP gateways on the local network without a known IP, closing the
gap Cycle 1 deliberately left open (its `discover` stub, `BusError::NotImplemented`).
A `knx bus discover` subcommand that multicasts a `SEARCH_REQUEST` and prints
whatever gateways answer, so `knx bus monitor`/`write --gateway` no longer
require the user to already know the address.

## Scope

**In.**

* `SEARCH_REQUEST`/`SEARCH_RESPONSE` (0x0201/0x0202, Core v01.06.02 AS §7.4,
  the original form — not `SEARCH_REQUEST_EXTENDED`/`RESPONSE_EXTENDED`,
  Core v2's superset).
* The standard discovery multicast group and port, `224.0.23.12:3671` (Core
  v01.06.02 AS §4.2), hardcoded — not a CLI flag this cycle.
* Full spec `SEARCH_TIMEOUT` (10s) as the response-collection window, not a
  shortened one.
* Parsing both DIBs a `SEARCH_RESPONSE` carries: Device Info (friendly name,
  individual address, MAC, serial, routing multicast address — §7.5.4.2) and
  Supported Service Families (§7.5.4.3), the latter used to flag whether a
  found gateway actually supports tunnelling.
* A `knx bus discover` subcommand printing one line per gateway found.

**Out (deliberately, later cycles or never).**

* `SEARCH_REQUEST_EXTENDED`/`RESPONSE_EXTENDED` (Core v2) — superset, own
  cycle if a gateway is ever found that needs it.
* A `--timeout`/multicast-address CLI override — hardcoded constants this
  cycle, per the "smallest clean solution" call above; configurable later if
  a real gateway setup needs it.
* Routing, KNX IP Secure — separate specs (ROADMAP.md, Session 6 entry).
* The Docker `--network host` requirement for multicast — a known constraint
  (ROADMAP.md), not solved here; `knx-server` doesn't call `discover` yet.
* Any UI wiring beyond the CLI — same reasoning as Cycle 1: prove it against
  hardware first.

## Ground truth

Same source as Cycle 1, `/home/knxbench/knx-ai/extracted_clean/`:

* `03_08_02 Core v01.06.02 AS.md` §4.2 (discovery multicast group/port),
  §7.4 (`SEARCH_REQUEST`/`RESPONSE` service identifiers and body layout),
  §7.5.4.2 (Device Info DIB), §7.5.4.3 (Supported Service Families DIB).

Same handling rule as Cycle 1: section/table numbers cited, spec prose never
reproduced beyond short structural fragments. A live capture against the
real gateway (`192.0.2.1:3671`, RESEARCH.md §8.1) cross-checks the DIB
byte layout.

## Architecture

```
crates/knx-net/
  src/
    core/
      dib.rs         // new: Device Info DIB, Supported Service Families DIB
      services.rs    // +SEARCH_REQUEST/SEARCH_RESPONSE constants
    discovery.rs      // new: SEARCH_REQUEST/RESPONSE body encode/decode
    client.rs          // KnxNetIpClient::discover implemented (was NotImplemented)
```

`core/dib.rs` and `discovery.rs` are pure — byte slice in, typed struct out —
same convention as `core/hpai.rs`/`tunnelling.rs`: a hand-rolled `Display` +
`std::error::Error` impl per error type, no new dependency.

`client.rs`'s `discover()`:

1. Bind a UDP socket to `0.0.0.0:0`, read back its own HPAI (`local_hpai`,
   already exists).
2. Encode and send one `SEARCH_REQUEST` (own HPAI only — no CRI/CRD, unlike
   `CONNECT_REQUEST`) to `224.0.23.12:3671`.
3. Loop receiving datagrams until `SEARCH_TIMEOUT` (10s) elapses in total;
   decode each as a `SEARCH_RESPONSE`, skip (don't abort) anything that
   fails to parse — one malformed reply on the segment shouldn't blank out
   every gateway found, same rule `receive_loop` already applies to stray
   tunnelling datagrams.
4. Dedupe by control endpoint (a gateway can answer more than once), return
   `Ok(Vec<DiscoveredGateway>)` — empty on zero responses, not `BusError::Timeout`;
   "no gateway found" is an answer, not a failure.

**`DiscoveredGateway`** (new, in `client.rs` next to `BusConnection`):

```rust
pub struct DiscoveredGateway {
    pub control_endpoint: SocketAddrV4,
    pub individual_address: IndividualAddress,
    pub friendly_name: String,
    pub supports_tunnelling: bool,
}
```

`supports_tunnelling` is derived from the Supported Service Families DIB
(KNXnet/IP Tunnelling service family ID `0x04`). Device Info's MAC address,
serial number, and routing multicast address are parsed into `dib.rs`'s
`DeviceInfo` struct (skipping them would be sloppy for a fixed-length
structure already being walked) but not re-exposed on `DiscoveredGateway`
yet — nothing today needs them; adding a field later is not a breaking
change most callers notice.

`BusConnection`'s signature is unchanged (`discover(&self) -> Result<Vec<_>, BusError>`);
only the element type of the `Vec` becomes `DiscoveredGateway` instead of
`SocketAddrV4`.

## Error handling

No new `BusError` variants — `Io`, `Timeout` (only for the initial socket
bind failing, not for an empty result set), and `Protocol(String)` (reused
for a `SEARCH_RESPONSE` that fails to decode entirely — logged and skipped,
per the same "ignore malformed, don't crash" rule as Cycle 1, Core v01.06.02
AS §6.2/§6.3) already cover every failure mode this cycle introduces.

## CLI integration

New `knx bus discover` subcommand in `apps/knx-cli/src/main.rs`, alongside
`monitor`/`write`. No `--gateway` flag — that's the feature. Output, one
line per gateway:

```
1.1.0  Example IP Gateway   192.0.2.1:3671  [tunnelling]
```

(individual address, friendly name, control endpoint, a `[tunnelling]` tag
only when `supports_tunnelling` is true). Zero gateways found prints a plain
"no gateways responded" line and exits `0` — not finding anything on a
10-second multicast probe isn't a CLI error.

## Testing

* `core/dib.rs` unit tests against byte fixtures built by hand from
  §7.5.4.2/§7.5.4.3's structure tables (Device Info fixed at 54 octets,
  Supported Service Families variable-length pairs).
* `discovery.rs` unit tests for `SEARCH_REQUEST`/`RESPONSE` body encode/decode,
  including a response carrying both DIBs and one missing the optional
  Service Families DIB (Device Info is mandatory, Service Families is not
  guaranteed present on every implementation).
* One more `#[ignore]`-gated case in `tests/live_gateway.rs`: call `discover()`
  against the real network, assert the known gateway (`192.0.2.1`) is
  among the results with `supports_tunnelling == true`. Runs only with
  `cargo test -- --ignored`, never in CI.
* `knx-cli`'s `discover` subcommand gets a manual-check entry in
  `IMPLEMENTATION_STATUS.md` (same pattern as `monitor`/`write` — needs the
  real gateway, no display or network segment available in this
  environment).

## Documentation impact

* `docs/RESEARCH.md` §8: add a `[V]` entry once discovery is confirmed
  against the real gateway.
* `docs/ARCHITECTURE.md` §8: update `BusConnection.discover`'s status from
  stub to implemented.
* `docs/IMPLEMENTATION_STATUS.md`: new cycle entry; move discovery off the
  "known gaps" list, add the CLI manual-check item above.
* `docs/KNOWN_LIMITATIONS.md`: narrow the existing "discovery not
  implemented" note to "extended discovery (Core v2) not implemented" /
  "no `--network host` handling for the Docker deployment path yet".

## Open questions carried forward (not blocking this cycle)

* Whether `SEARCH_REQUEST_EXTENDED` is ever actually needed — no gateway
  encountered so far requires it (RESEARCH.md §8.1's reference gateway
  answers the original form).
* Docker `--network host` for multicast — still Session 6's own carried-forward
  item (ROADMAP.md), untouched by this cycle since `knx-server` doesn't call
  `discover`.
