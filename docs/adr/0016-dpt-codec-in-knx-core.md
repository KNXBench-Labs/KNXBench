# ADR 0016: The DPT codec lives in `knx-core`, and `GroupValue` moves down into it

Date: 2026-09-11
Status: Accepted
Session: 7 (gap E4 closure, T29)

## Context

`crates/knx-core/src/dpt.rs` held exactly `DptRef`, `DptRef::parse`,
`DptParseError` — a reference to a datapoint type, never a value of one.
Nothing in the workspace encoded or decoded a DPT's wire form. `apps/knx-cli`'s
`bus monitor` printed `GroupValueWrite 0x01 (6-bit)`; `bus write` accepted only
`0`, `1`, or a hex byte string. CLAUDE.md lists "Datapoint Types (DPT)" among
what the KNX domain core must represent, and `DptRef` already lived there —
the codec that gives a `DptRef` meaning is the same domain concept, not an
application- or UI-layer one.

The codec has to speak the same payload type the bus transport layer speaks.
That type already existed: `knx_net::cemi::GroupValue`, a two-case enum
(`Short(u8)` for a value carried in the six inline APCI bits, `Bytes(Vec<u8>)`
for one or more separate octets) — the payload of a group telegram, which is
domain data, not a transport-protocol detail. `knx-net` already depends on
`knx-core` (`xtask check-layering` enforces the direction), never the other
way round.

## Decision

The codec lives in `knx-core`. `dpt.rs` becomes a module directory:

```
crates/knx-core/src/dpt/
    mod.rs        // DptRef, DptParseError, GroupValue, re-exports
    codec.rs      // decode/encode, DptValue, DptCodecError
    resolve.rs     // group-address -> DPT resolution over a Project
```

`GroupValue` moves down from `knx_net::cemi` into `knx_core::dpt`.
`knx-net` re-exports it (`pub use knx_core::GroupValue;`, from both
`cemi.rs` and the crate root, since `crates/knx-net/src/client.rs`'s test
modules address it via `crate::cemi::GroupValue` specifically) so every
existing call site — `cemi.rs`, `client.rs`, `tests/live_gateway.rs`,
`apps/knx-cli/src/main.rs` — compiles unchanged. Nothing about the direction
of `knx-net`'s dependency on `knx-core` changes; this only moves which crate
owns the type's definition.

## Alternatives considered

**A structurally identical `DptPayload` type in `knx-core`, converted to and
from `knx_net::GroupValue` at the crate boundary.** Rejected. It buys
nothing: a group value's payload is one wire concept, described once,
whichever crate happens to define it. Two names for the same two-variant enum
means a conversion function at every boundary crossing, forever, plus the
inevitable drift risk of one side gaining a case the other has to be taught
about by hand. `knx-net` already depends on `knx-core`, so nothing forces a
duplicate to preserve a dependency direction — the direction that would
justify the duplicate does not exist here.

## Consequences

**A latent bug in `knx-net::cemi::encode_group_value` became a live one, and
this slice is what made it reachable, so this slice fixes it.**
`encode_group_value` computed `apci_lo = ((short_apci & 0x03) << 6) |
inline6`. A `GroupValue::Short(v)` with `v > 0x3F` overwrites the top two bits
of that octet — the two bits belonging to the APCI service selector, not the
value — so `Short(0x40)` on a `GroupValueWrite` was transmitted as a
*different application service* with the value silently gone. Nothing in the
workspace produced such a value before this cycle, because the only producer
of a `GroupValue` was a human typing `0` or `1` at the CLI. A codec is a
producer: `encode` computes whatever six-bit-or-wider value the DPT's wire
form requires, and there was nothing stopping it from handing `encode_group_value`
a `Short` above `0x3F`. `encode_group_value` now promotes an out-of-range
`Short(v)` to the one-octet `Bytes([v])` form instead — the value survives,
the APCI stays intact, and the result is what the Standard requires for a
value that does not fit six inline bits. A regression test
(`encode_promotes_out_of_range_short_instead_of_corrupting_apci`) encodes
`GroupValueWrite(GroupValue::Short(0x40))` and decodes it back, asserting the
service is still `GroupValueWrite` carrying `Bytes(vec![0x40])`.

The codec (`decode`/`encode`/`DptValue`/`DptCodecError`) and the resolution
query (`GroupAddressDpt`/`resolve_group_address_dpt`/
`resolve_project_group_address_dpts`) are reachable from `knx-core`'s crate
root the same way every other module's public surface already is, so
`apps/knx-cli` — and, later, any UI T15 builds — call `knx_core::decode`,
`knx_core::encode`, `knx_core::GroupValue` directly, with no import path
change anywhere that already used `knx_net::GroupValue`. `cargo run -p xtask
-- check-layering` stays green: `knx-core` gains no new dependency, and
`knx-net` gains no new dependent.
