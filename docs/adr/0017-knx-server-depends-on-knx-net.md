# ADR 0017: `knx-server` depends on `knx-net` directly, no crate interposed

Date: 2026-09-11
Status: Accepted
Session: 7 (T15 task 1, group monitor GUI, gap D5/E4 display side)

## Context

T15's design document
(`docs/superpowers/specs/2026-09-11-group-monitor-design.md`, §3 D1) commits
`apps/knx-server` to opening and reading a live KNXnet/IP tunnel so
`apps/knx-web` can show a group-monitor table. Something in `knx-server`
therefore has to call `knx_net::KnxNetIpClient::connect_tunnel` and read a
`knx_net::TunnelClient`'s `broadcast::Receiver<TunnelEvent>`.

`cargo run -p xtask -- check-layering` (`xtask/src/main.rs`) encodes every
layering rule this workspace actually enforces. It restricts `knx-core`,
`knx-etsproj`, `knx-productdb`, `knx-projection`, `knx-csv`, `knx-report`
and `knx-diff` — it names no rule at all for `knx-server`. Run against this
task's change (`knx-net.workspace = true` added to
`apps/knx-server/Cargo.toml`, nothing else touched yet) before anything
else was written, it printed the same "layering ok: ..." line it prints on
`main`, unchanged — the tool has no opinion on this edge because
`knx-server` is Infrastructure's consumer, not a crate any lower layer is
forbidden from reaching. `knx-server` already depends directly on three
other Infrastructure crates (`knx-store`, `knx-etsproj`, `knx-productdb`)
for the same reason: it is the one process wiring HTTP routes to whatever
back end a route needs, and each of those back ends earns its own crate for
reasons unrelated to `knx-server` (persistence format, import/export
format, a separately versioned product database — ADR-0005, ADR-0011).
`knx-net` is a fourth Infrastructure crate for a fourth reason (a network
protocol), reached the same way, not a new *kind* of edge.

`knx-server` cannot construct a `knx_net::TunnelClient` directly and test
it without a real gateway, because `knx_net::BusConnection` is deliberately
not `dyn`-safe (single implementer, called directly — the right call for
`knx-net` itself, see `crates/knx-net/src/client.rs`'s own comment on it).
Design spec §3 D6 answers that with a narrower trait pair
(`GatewayConnector`/`BusTunnel`) defined inside `knx-server`, not inside
`knx-net` and not inside a third crate — see Decision below.

## Decision

`apps/knx-server` gains a direct dependency on `crates/knx-net`, alongside
its existing `knx-store`/`knx-etsproj`/`knx-productdb` dependencies. No
crate is interposed between them. The `GatewayConnector`/`BusTunnel`
testability seam (design spec §3 D6) is a module (`apps/knx-server/src/
bus.rs`) inside `knx-server` itself, not a new crate.

## Alternatives considered

**A separate `knx-bus-service` crate**, holding the seam and the eventual
session/buffer/drain-task logic, with `knx-server` depending on it instead
of on `knx-net` directly. Rejected. `knx-server` is this seam's only
consumer today — nothing else in the workspace opens a tunnel or would
plausibly need to — and CLAUDE.md's "avoid speculative abstractions"
argues directly against a crate boundary drawn for a hypothetical second
consumer that does not exist. A crate boundary is not free: it is another
`Cargo.toml`, another entry in `check-layering`'s graph, another public API
surface to keep stable, for a module that today has exactly one caller. If
a second consumer (the Tauri desktop shell wiring its own bus session,
say) appears later, extracting this module into its own crate is a
mechanical move with no design left to invent — the seam's shape does not
change based on which crate holds it.

## Consequences

`apps/knx-server/Cargo.toml` gains one line (`knx-net.workspace = true`,
alphabetically placed among the other `knx-*` entries). `check-layering`
needed no new rule and gained none — it already treats `knx-server` as
unrestricted, and stays that way; a future rule restricting `knx-server`
itself (if this project ever wants one) is a separate decision, not implied
by this one. The bus-monitor seam, and everything built on it in later T15
tasks, lives and is tested inside `apps/knx-server` — no second `Cargo.toml`
to keep in sync, no cross-crate version bump for a change that only ever
touches one binary. The trade-off is the one named above: if a second
process ever needs the same seam, today's decision is revisited, not
free — but nothing in this workspace needs that today.
