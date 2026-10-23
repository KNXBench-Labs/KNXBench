# Alpha 16 process-owned catalogue lifetime

This implementation boundary composes [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
and [P9](protocol-artifacts.md#p9-concrete-ownership-standalone-stop-and-desktopheadless-drain).
It is not a new job framework, a device cancellation policy, or full WP-04.3 acceptance.

## Ownership and callable API

`AppState::catalogue_jobs` is one `Arc<CatalogueUpdateJobs>`, constructed once
with `Incarnation::mint()` through the existing 16-byte CSPRNG identity grammar.
Catalogue routes/runtime must consume that exact owner, not construct another
manager. Existing `CatalogueOwner` SQL authority is unchanged.

`close_process_admission(&AppState)` synchronously closes bus admission and calls
that owner's cooperative `close_admission_and_request_stop()`.
`drain_catalogue_jobs(&AppState, Duration, second_signal)` borrows the same owner's
retained join handles and returns its exact `CatalogueDrain` facts. It does not
abort workers, fabricate terminal status, or replace commit/cleanup evidence.

`EmbeddedServerHandle::spawn(listener, Arc<AppState>, static_dir)` starts the
actual production router inside an entered Tokio runtime. The host supplies an
already-bound Tokio listener and retains the returned handle:

- `request_close()` synchronously closes admission, requests graceful HTTP stop,
  and starts exactly one retained shutdown task on the captured runtime. It can
  be called from a native callback outside an entered Tokio context.
- `drain_close(second_signal).await` requests close and returns the shared
  `Arc<EmbeddedClose>`. Only this explicit headless second-stop future can signal
  force. Cancelling a waiter does not cancel shutdown or transfer/drop join
  ownership. Repeated ordinary requests never become second-stop.
- The HTTP task handle remains in the owner during polling and after HTTP timeout;
  the shutdown task is retained and joined with serialized, cancellation-safe
  polling. Unexpected HTTP termination also closes admission and starts that
  same drain. No Drop-spawn cleanup or blocking-task abort is used.
- `EmbeddedClose` preserves HTTP result, `BusRelease`, `WriteDrain`, and
  `CatalogueDrain` separately. `requires_process_exit()` tells a process host
  when normal runtime teardown would be unsafe; the reusable library does not
  terminate its host. Keep the handle and original state through that decision.
  A returned timed-out result remains a historical outcome, not later completion.
  A host retaining state may inspect/drain the underlying catalogue owner again.

## Standalone ordering

The first signal closes both admission domains and requests cooperative catalogue
stop before the HTTP grace. HTTP error/normal termination also closes admission
instead of panicking past drain. Existing HTTP grace remains five seconds; bus
release remains two seconds. Immediately after bus release, device-write drain
and the fixed thirty-second catalogue drain run concurrently with independent
second-signal receivers. `KNX_STOP_WRITE_WAIT_SECS` parsing/default/maximum and
existing device-write warning text are unchanged. Catalogue timeout never
shortens a device safety wait or cancels a write already underway.

Normal return requires drained HTTP, no timed-out/second-signal device wait, and
no outstanding catalogue work. Otherwise the existing explicit process-exit
escape is retained. Outstanding catalogue work is reported as possibly committed
or outcome unavailable; retained recovery backups may require inspection.
Quarantined cleanup is reported independently. No exit path claims that timeout
proved cancellation, rollback, cleanup, or worker completion.

## Desktop interception and approved Quit

The desktop retains `EmbeddedServerHandle` in managed application state in both
dev and release branches. `Builder::build(...).run(...)` intercepts
`RunEvent::ExitRequested`. Unauthorized requests are prevented and coalesce into
one `begin_close`; main-window `Destroyed` calls that same function directly,
without enqueueing the old early `AppHandle::exit(0)`.

`Destroyed`, implicit exit and explicit queued exit requests cannot authorize
force, regardless of order, count or exit code. The owned shutdown task uses a
never-resolving second-stop future. There is no desktop force command/UI in this
slice; the documented independent bounded waits are its escape. On completion,
exit authorization is set before the sole final `AppHandle::exit(0)`. If work
remains, truthful diagnostics precede explicit process exit rather than a hang
in runtime Drop. Native event-delivery failure and OS/session kill remain
crash-like, not cleanup evidence. Restart is not exposed.

Existing frontend unsaved-changes and dead-frontend native confirmation remain.
`quitApp()` now calls native `close()`, not `destroy()`. A one-shot approval lets
the close-request listener avoid reopening the already-dismissed unsaved dialog;
ordinary later close requests still ask. Pinned Tauri JavaScript performs window
destruction after an accepted callback; process exit still reaches the Rust
lifetime hook. The main capability must explicitly grant
`core:window:allow-close`; Tauri defaults do not grant it. Keep the existing
`allow-destroy` required by Tauri's accepted-close implementation.

## Verification and limits

Focused headless controls cover shared-owner admission closure, repeated close,
waiter cancellation, explicit second-stop with a real blocking worker, and
independent concurrent catalogue/device deadlines. Their embedded fixture drives
an actual isolated loopback HTTP health request. Desktop notification-policy
controls exercise ordinary event-order permutations and final authorization
without a native window or user session. A mock-runtime IPC control uses the
real capability set to admit native close from the existing main-window dev and
release origins while refusing an unrelated origin; malformed arguments prevent
any window operation. Web tests cover approved/ordinary close,
failed-close approval reset, and existing App unsaved-close workflows.
Execution outcomes and exact source identity are recorded in the implementation
handoff, not inferred from this list of test purposes.

This slice does not prove real Tauri event delivery, packaging/deployment,
power-loss durability, SIGKILL recovery, or physical bus/device outcomes.
Production catalogue runtime cancellation/receive/cleanup and retained commit
facts remain owned by the catalogue runtime implementation; this lifetime uses
its exact owner API without manufacturing those facts. Final combined-candidate
acceptance and artifact smoke remain separate.

System-2 asynchronous writes are **not integrated by this lifetime slice**.
There was no existing process-owned System-2 operation service to drain here;
no speculative generic hook was added. The System-2 integration owner must add
its dedicated typed service drain to the shared process decision before claiming
all operation lifetimes are covered.
