# 8.1 Verified live [V]

[Research index](../knxnet-ip-and-bus.md) · [Evidence tags and sources](../../RESEARCH.md).

This is a preserved research dossier, not fresh protocol or hardware acceptance.
Original dates, assumptions, corrections and numbered sections stay in force.

<a id="81-verified-live-v"></a>
<!-- BEGIN preserved record-1-003 -->
### 8.1 Verified live [V]

Tunnelling to a real gateway at `192.0.2.1:3671` works from Linux via `xknx`, and 280 telegrams were captured in a 300 s window. Observed APCI: `GroupValueWrite` (majority), `GroupValueRead` (30), `GroupValueResponse` (30). Payload forms: `DPTBinary` (small values) and `DPTArray` (2/3/4-byte). All 280 telegrams resolved to a named group address from the project — the project model and the live bus agree.

This confirms end to end: project data → group address semantics → live telegram decoding, on Linux, without ETS.

**Session 6, Cycle 1 (2026-09):** An own Rust implementation of the
tunnelling connect/heartbeat/receive/disconnect lifecycle and cEMI `L_Data`
decode, built directly from the KNX Association specification (Core v01.06.02
AS, Tunnelling v01.07.01 AS, EMI_IMI v01.04.02 AS — not from reading
`xknx`'s implementation). This cycle delivered `crates/knx-net`'s five codec
modules with unit tests, the `TunnelClient` state machine, an `#[ignore]`d
live-gateway integration test (the first of its kind in this repository —
a pattern for future hardware-dependent tests), and the `knx bus monitor`
CLI subcommand. Live-gateway verification against a real KNXnet/IP gateway
is the next step, to be run by the user via `cargo test -p knx-net --
--ignored` and `knx bus monitor` against their own gateway in an environment
with LAN access.

**Session 6, Cycle 3 (2026-09):** `discover()` implemented — a
`SEARCH_REQUEST` multicast to `224.0.23.12:3671` (Core v01.06.02 AS §4.2),
collecting `SEARCH_RESPONSE`s for the full 10s `SEARCH_TIMEOUT` and
parsing both the Device Info and Supported Service Families DIBs
(§7.5.4.2/§7.5.4.3). `crates/knx-net` gained `core::dib` and `discovery`,
plus a `knx bus discover` CLI subcommand. Live verification against the
reference gateway — confirming it answers and advertises tunnelling
support — is the next step, to be run by the user via
`cargo test -p knx-net -- --ignored` and `knx bus discover` on their own
LAN.


<!-- END preserved record-1-003 -->
