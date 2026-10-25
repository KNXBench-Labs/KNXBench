# Regular download prewrite stop acceptance — 25 October 2026

## Published source and contract

`6c9ca2d7d` is published on `main` with matching live and tracking-ref readback.
The main-first merge preserves upstream `a8304e9a3` outside fourteen reviewed paths.
[ADR-0153](../adr/0153-regular-download-stop-ends-at-first-change.md) narrows the
existing server shutdown policy only before the first regular download mutation
([contract](../contracts/regular-download-prewrite-stop.md)).

A run-bound handle is registered before history, tunnel acquisition and worker spawn.
A short synchronous registry serializes registration with process closure; neither
an async reservation held by an in-flight start nor a not-yet-installed session can
hide the handle. Stop acceptance and the first mutation commitment are atomically
ordered, not inferred from an earlier global closed flag alone. An accepted stop
prevents this run's memory, user-memory, load-control, property and restart writes.
After commitment, the existing execution, verification and cleanup run unchanged.
No future is dropped to stop a write. There is no automatic restore, retry or reset.

The additive backed-up stoppable executor validates plan shape, mask, download
scope, no held connection and single-operation ownership before sending anything.
Legacy entry points remain unchanged. Stop checks occur between completed exchanges,
including backup reads and keeper completion. A real read/backup/keeper failure
still wins over a stop request. A stop can have sent connection, authorization and
read traffic; only a stop before Connect sends no frames. No claim of physical
inertness for session authorization is made.

A claimed handle cannot be reused, even after a stopped, failed or nonmutating run.
It settles to `Ended`; rejection before ownership leaves a previously unused handle
unclaimed and does not prove any live run exists. Synthetic committed handles are
test stand-ins, not evidence of a sent frame or device effect.

## Original and corrected evidence

- An original-runtime executable was built with test-only changes using existing
  server start, process-close and drain entry points. Six selected methods ran:
  four controls passed and two intended semantic protection assertions failed.
  Thirteen mutating frames still reached the synthetic device during the original
  shutdown path. The independent OLD-memory preservation assertion failed too.
  This is semantic RED, not failure to compile a new API.
- Parent authenticated original sources and executable digests and replayed that
  original binary: the same four controls passed and two intended failures remained.
  Final tests and helper wiring are retained separately, not relabelled as original.
- Corrected parent replay passed the complete sixteen-method worker check module,
  twenty-five graceful-stop methods and twenty executor stop methods. The original
  same-name prefix expands to twelve methods on the final source; it is not claimed
  to be the same six-method inventory.
- The worker's full owning executions passed 558 network and 604 server library
  tests, zero failed, with four server tests ignored. These retained logs and the
  executable hashes were authenticated; overlapping focused totals are not added.
- Independent Opus implementation review approved without blockers. Nonblocking
  lifecycle/keeper/index/HTTP-evidence wording and one misleading test name were
  corrected without runtime control-flow changes. Narrow re-review approved; all
  three focused final modules were freshly compiled and passed again.

Executions used isolated, non-root, capability-free loopback-only namespaces with
scratch application data/configuration. No physical installation was contacted.

## Integrated acceptance on `6c9ca2d7d`

All fourteen stages selected by the maintained change-scoped runner passed in order,
with identical start/end input commitments and empty touched/unchecked user-data lists.

- Rust: 3,244 passed, zero failed, 131 ignored across seventeen result blocks.
  All 61 named stop, shutdown and worker checks passed on this merge.
- Private corpus: 91/91 across five selected targets. This is selected regression
  evidence, not an exhaustive project/device census or a new recovery capability.
- Formatting, Clippy, tooling, documentation/local-information, browser-lane
  classification, layering, headers, anchors, ledger, corpus registration and diff
  checks passed. Browser-lane classification is not browser execution.
- The frontend was freshly built as the desktop Rust prerequisite. Browser execution
  was not selected; interactive desktop, release and deployment acceptance are not claimed.

## Remaining boundaries

The actual HTTP start route's registration order was source-reviewed and compiled,
not driven through HTTP by the focused fixtures. Real-worker fixtures use the same
preregistered ownership, including close before installation and a held async lock.
A stop after commitment, forced exit, second-signal escape or expired wait can still
leave a partially written device; a request outcome is not necessarily a `Stopped`
terminal result. Existing crash history remains interrupted/unknown, not a fabricated
stopped checkpoint. No schema migration, per-step effect journal, resume or whole-device
recovery is introduced. UI/stop route, CLI interrupt and System-2 cancellation are
outside this increment. [PRG-01](../open-work/PRG-01-programming-gaps.md),
[KL-007](../limitations/KL-007-bounded-commissioning-support.md) and the shutdown
[forced-exit boundary](../limitations/KL-163-server-shutdown-and-active-writes.md)
remain open within their recorded scopes.
