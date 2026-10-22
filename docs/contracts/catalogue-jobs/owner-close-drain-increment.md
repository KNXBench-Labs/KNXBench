# WP-04.3 stage D — catalogue jobs owner and close/drain lifecycle (P9)

Claimed 22 October 2026. The fifth increment of WP-04.3 stage D
([design-readiness.md](design-readiness.md) stage D: owned job/lifetime API)
under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md). This is the last
stage-D increment. It delivers the server-side catalogue jobs owner and its
close/drain lifecycle as a private unwired addition. No production route,
worker execution, `EmbeddedServerHandle` or desktop exit interception becomes
reachable.

## Scope of this increment

A new private module `catalogue_jobs::owner`:

- **`CatalogueUpdateJobs`** — the server-side owner wrapping the delivered
  `CatalogueJobManager` (admission/replay) plus the worker/cleanup handles it
  will hold. Construction takes the server incarnation. All admission state
  lives under one short `Mutex<CatalogueJobManager>`; the worker handle lives
  under a separate short lock, so no lock is held across an await.

- **`close_admission_and_request_stop`** — sync, idempotent: closes fresh
  admission through the manager's existing `close_admission` (retained replay
  and dispose stay available). No SQL, filesystem, await or parser work.

- **`drain(limit, second_signal)`** — async: takes the worker/cleanup handles
  outside the short lock and joins them with a bounded wait racing the limit
  and `second_signal`, returning the exact `dto::CatalogueDrain`
  (`nothingRunning` with no registered work, `finished` when all joined,
  `timedOut`/`secondSignal` with `workOutstanding = true`). A timeout or second
  signal is never a cancellation witness.

## Explicitly out of scope (later increments)

- Route wiring and worker execution (the Ready/apply installer orchestration);
  `register_worker` exists only so that increment can attach a real handle.
- The `EmbeddedServerHandle` and the standalone `main.rs` / desktop exit
  interception: they are a separate, review-heavy slice of P9 that touches the
  Tauri process lifecycle and the existing device-write drain preservation.
- `DetailQuery`/`Page`/`PublicRecord` artifact records (P6).

## Done when

- Close is idempotent and leaves retained replay/dispose reachable.
- Drain reports `nothingRunning` with no registered work, `finished` when a
  registered worker ends, and `timedOut`/`secondSignal` truthfully when work
  remains; the worker/cleanup handle is always joined or dropped after the
  drain, never left for a later owner to double-join.
- Focused tests and the owning `knx-server` suite pass; warning-denied Clippy,
  `cargo fmt` and header checks are clean; no production route becomes
  reachable and no quiet API, schema or response shape changes.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [design-readiness.md](design-readiness.md) (stage D ordering)
- [protocol-artifacts.md](protocol-artifacts.md) (P9)
- [wp043-protocol-policy.json](wp043-protocol-policy.json) (durationsSeconds)
