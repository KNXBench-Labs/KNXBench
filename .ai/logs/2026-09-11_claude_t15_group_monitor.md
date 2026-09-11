# 2026-09-11 — T15: a Group Monitor GUI, and a seam to test it without hardware

Architecture log for the T15 cycle, branch `t15-group-monitor`, sixteen
commits from `b88b286` (`e3b7208`..`bf435e3`), merged to `main` as `ee4b27d`
— 28 files, 6947 insertions, 49 deletions.

Design: `docs/superpowers/specs/2026-09-11-group-monitor-design.md`.
Plan: `docs/superpowers/plans/2026-09-11-group-monitor.md`.
Decision record: `docs/adr/0017-knx-server-depends-on-knx-net.md`.

## What changed architecturally

Three things. The first moves a dependency edge, the second introduces a
seam, and the third is a lifetime that did not exist in this application
before.

### 1. `knx-server` now depends on `knx-net` directly

Before T15, `apps/knx-server` reached the KNX bus not at all: bus
communication existed only in `crates/knx-net` and was driven exclusively by
`apps/knx-cli`. The web and desktop front ends had no path to it, which is
precisely what `GAP_ANALYSIS_ETS.md`'s **D5** recorded.

```text
apps/knx-web ──HTTP──→ apps/knx-server ──→ knx-net ──→ KNXnet/IP tunnel
                                       └──→ knx-core::dpt (decode/encode)
```

No new crate was interposed. ADR-0017 records why: a crate whose only
purpose is to re-export one trait pair buys an indirection and costs a
compile unit, and `xtask check-layering`'s existing rules already forbid the
edge that would actually be dangerous (`knx-core` reaching a transport or a
runtime). `knx-server` was already the application layer for
`knx-store`/`knx-etsproj`/`knx-productdb`; `knx-net` joins that list rather
than justifying a new one.

### 2. `GatewayConnector` / `BusTunnel` — a seam owned by the consumer

`apps/knx-server/src/bus.rs:52` and `:64` define two traits:

```rust
pub trait GatewayConnector: Send + Sync { /* connect_tunnel */ }
pub trait BusTunnel: Send + Sync { /* subscribe, send, disconnect */ }
```

They live in `knx-server`, not in `knx-net`. `knx_net::BusConnection` is
deliberately not `dyn`-safe — one implementer, called directly, with a
comment in `crates/knx-net/src/client.rs` saying so — and widening it to
suit a consumer's test strategy would have been the transport crate paying
for the application crate's convenience.

The consequence is the one that mattered for this whole slice: **every test
in this branch runs on a laptop with no KNX hardware attached.**
`FakeConnector`/`FakeTunnel` are scripted with an exact list of outcomes, so
an unexpected second `connect_tunnel` panics the fake rather than quietly
succeeding — a structural proof that needs no call-count assertion to hold.
`crates/knx-net/tests/live_gateway.rs` is byte-for-byte unchanged and still
the only place that wants real hardware.

### 3. A bus session with a lifetime of its own

`AppState` gains `bus_session`, and with it the first long-lived background
task in this application:

- a bounded buffer, `MAX_TELEGRAMS = 5000` (`bus.rs:494`);
- a drain task that pulls `TunnelEvent`s off `knx-net`'s broadcast channel
  and pushes rows into the buffer;
- `dropped_before` (`bus.rs:908`), a monotonically increasing count of every
  telegram that existed and cannot be shown;
- four endpoints in `bus_routes.rs:45-48` — `POST /api/bus/monitor/start`,
  `POST /api/bus/monitor/stop`, `GET /api/bus/monitor/telegrams`,
  `POST /api/bus/write`.

**The branch's single load-bearing promise is that no telegram is lost
silently.** There are exactly two ways to lose one, and both increment the
same counter: buffer eviction when the 5000-row ring wraps, and
`broadcast::error::RecvError::Lagged(n)` when the drain task falls behind
the tunnel. The client reads `droppedBefore` on every poll and renders a gap
notice from it. The final whole-branch review was asked to find a third path
and traced the code end to end: there is none.

`bus_session` is a `tokio::sync::Mutex`, not a `std` one, because
`std::sync::MutexGuard` is not `Send` and the guard is held across
`connect_tunnel().await`. The cost is real and bounded: `/telegrams` and
`/stop` can block behind a `/start` that is still connecting, for at most
the 10-second timeout in `knx_net::TunnelClient::connect`
(`crates/knx-net/src/client.rs`). Recorded rather than hidden.

## The front end

`apps/knx-web/src/BusMonitorPanel.tsx` (411 lines) is the table, the filter
and the connect/stop controls. `BusComposeForm.tsx` (223 lines) is a
sibling, not a child — it survives polls and resets only on a new row click,
via a `key`-remount. Clicking a row prefills the form; sending it puts the
telegram back on the bus it came from.

The form rejects an unresolvable DPT client-side, mirroring the server's own
`resolve_write_value`, with the same two messages word for word. That
duplication is deliberate: the server is still the authority, and the client
copy exists only so the user is not made to round-trip to learn something
the browser already knows.

Rows accumulate without limit in the browser. Capping them client-side was
considered and rejected — a second drop-accounting mechanism could disagree
with the server's, and exact drop accounting is the promise this slice
exists to keep. It is written down as a limitation instead.

## What this slice deliberately is not

`docs/KNOWN_LIMITATIONS.md` **§62** is the full accounting, thirteen items.
The headline four:

- **Tunnelling only.** Routing is not wired to the GUI.
- **One session at a time**, process-wide. A second `/start` gets `409`.
- **A client-side filter**, over the rows the browser happens to hold.
  It is not ETS's filter and does not behave like it.
- **Group telegrams only**, and **none of it has ever been near a physical
  KNX installation.** Every test drives a fake.

`docs/KNOWN_LIMITATIONS.md` §61 — the DPT codec's own coverage accounting,
inherited from T29 — is byte-for-byte untouched. A decode that fails is
rendered with its stated reason; the codec was not widened inside this
branch to make the GUI look better.

`apps/knx-cli` still parses `destination` as `ThreeLevel` unconditionally,
the same bug that was fixed server-side in `b540264`. Fixing it here would
have mixed unrelated changes. It is recorded in the docs instead.

## The scope rulings of 2026-09-11, riding along

The branch carries one unrelated documentation change, with its own commits,
because it edits the same files. Asked to settle five long-open scope
questions, the user ruled:

1. `.vd2` support — **out**, permanently.
2. Encrypted `.knxprod` — **out**, untestable without a sample.
3. T19 KNX Secure — **deferred** until hardware exists, documented as a
   limitation meanwhile.
4. T20 Functions — **deferred** until the new KNX specification
   documentation is available.
5. E1 commissioning — **not out at all.** It must work; it waits on the KNX
   specification database.

The fifth is the one that cost work. Six documents called commissioning
permanently out of scope, in prose that had accumulated over several cycles.
All six now say *blocked*, with the ruling quoted and dated, and a new
backlog task **T30** (Tier 5). `GAP_ANALYSIS_ETS.md`'s E1 row stays open
rather than being closed as a non-goal.

## Two reviews, two implications, one lesson

Both blocking findings on this branch were the same failure mode: a claim
nobody made explicitly that a reader would nonetheless take as true.

1. `GAP_ANALYSIS_ETS.md`'s E4 row said the GUI decodes "the way ETS's Group
   Monitor does". §61 records that the codec deliberately differs from a
   published AN188 reference figure by one step in a known case. The phrase
   predated this branch, but this branch extended it to a new subject, and
   extending a risky claim is authorship, not inheritance. Fixed in
   `a4b90d0`.
2. Three sentences in `docs/RESEARCH.md` still called commissioning
   permanently out of scope after the other six sites were fixed. The cause
   was my own brief: it said "grep everywhere" and also listed five files,
   and the narrower instruction won. Fixed in `696c6b3`.

The lesson is worth keeping: a brief that contains both a general
instruction and a specific file list has silently given the specific one.

## Verification

Gates on merged `main` (`ee4b27d`), all six:

- `cargo fmt --all --check` — clean
- `cargo clippy --workspace --all-targets -- -D warnings` — clean
- `cargo test --workspace` — **951 passed / 0 failed / 3 ignored**
  (up from 920 at `b88b286`)
- `cargo test -p knx-server` — **147 passed / 0 failed / 0 ignored**
- `cargo run -p xtask -- check-layering` — ok
- `cargo deny check` — advisories ok, bans ok, licenses ok, sources ok
- `npm run test` (`vitest run`, from `apps/knx-web`) — **179 passed across
  17 files**

No claim of ETS parity, KNX certification, or hardware verification is made
anywhere in this slice — not in code, comments, tests, UI strings,
documentation or commit messages.
