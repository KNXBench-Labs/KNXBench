# ADR 0023: A project load is one server-side operation, and the browser polls its phase

Date: 2026-09-19

Status: Accepted

## Context

Both ways a project enters the application are a single HTTP request that
answers only when everything is done:

- `POST /api/project/import` — an ETS `.knxproj`, through
  `knx_etsproj::import_knxproj` → `knx_app::import_ets_project_with` →
  `knx_projection::build_project_tree`.
- `POST /api/project/open` — this application's own `.knxdb`, through
  `knx_store::open_and_migrate` → `load_project` → the same projection.

Until this ADR, `apps/knx-web/src/App.tsx` awaited that one promise and
rendered nothing in the meantime. A long import and a wedged request are
the same picture: a window that does not react. The import of the
maintainer's reference ETS4 export (1.8 MB, 38 container entries, 24
manufacturer files) measures 114 ms end-to-end in a release build and
1 192 ms in a debug build on the development machine, and spends
essentially all of that time inside crates the HTTP layer cannot see into.

Those two numbers also set the honest expectation for this feature. The
largest project in the maintainer's corpus does not make a release build
work long enough for a human to read a phase label; what this buys is
feedback on the loads that *are* slow — debug builds, installations larger
than a single house, slow storage, and the imports that fail partway
through, where "which phase" is the whole diagnosis. The measurements are
in the task-25 report, including the phase sequence an actual run emitted.

That last point decides most of what follows. A server that wants to say
what is happening *truthfully* cannot describe a pipeline it does not
drive. Either the pipeline reports its own stages, or the progress
indicator is fiction — and a fabricated bar is the same defect class as a
test that cannot fail: it is a check whose passing means nothing.

What is verified here: the stage list below is the actual call order in
`knx_etsproj::import_knxproj_bytes_observed` and
`knx_app::import_ets_project_observed`, pinned by tests that record the
stages an import emits. What is assumed: nothing about how long any stage
takes — deliberately, see the decision on percentages.

## Decision

**One operation at a time, owned by the server.** `AppState` holds
`load_operations: Arc<LoadOperations>` — at most one load in flight per
server run, with a monotonic `operationId` that is never reused (the same
shape `bus_session`/`next_bus_session_id` already uses, and for the same
reason: a client that polls across two operations must be able to tell
from the id alone that it is looking at a new one). A second import or
open while one is running is refused with `409 Conflict`, not queued.

**The client half: an opaque token, not a guess.** The POST does not
return the operation id, so something has to establish ownership from the
client side. Three rounds tried to infer it from facts the server already
had — no filter, an id-only filter, an id-and-`source` filter — and each
one let one client's poll adopt another client's operation under some
timing or naming coincidence the previous round had not thought of. Fix
round 3 (finding F9) deletes the inference entirely: before its POST,
`App.tsx`'s `runLoad` generates an opaque token with `crypto.randomUUID()`
and sends it as `clientToken` in the POST body of both
`/api/project/import` and `/api/project/open`. The server stores it
verbatim on the operation (`LoadSnapshot.client_token`) and echoes it in
every snapshot the client polls. `loadProgress.ts`'s `ownsOperation` is
exact equality between that field and the token the load generated for
itself — one rule, no fallback branch, no window of any width for a
same-named stranger to fall into.

A missing token belongs to nobody. An operation started with no
`clientToken` (an older caller, or none sent) carries `null` in every
snapshot, and `ownsOperation` never matches an absent token against
anything — including another load that also sent no token. The guard is
stated positively (fix round 4, F12): the echoed value must *be* a
non-empty string before it is compared at all, because "absent" has three
spellings on this wire — `null`, a field a future server build omits
(`undefined`), and `""` — and a `!== null` test lets two of them through.
Two absences are not a match, they are two absences. The banner for such
a load simply stays on `starting` until its own POST answers; that is
less informative than naming a phase, and exactly as true as the
feature's core rule requires. There is no baseline read before the POST
any more — nothing to compute one from, and nothing it would be used for.

When a load ends in an error, the client accepts the final snapshot only
if it is its own **and** says `failed`. Every other case — no snapshot, a
foreign one, or its own reporting `succeeded` because only the response
was lost — becomes a locally constructed failed snapshot carrying the
error the POST threw, on the last phase this load actually observed. A
load that is over must never leave an indicator moving; that is the same
honesty rule as the percentage one, applied to the failure path.

**The transport is polling.** `GET /api/project/load-progress` returns the
current operation's snapshot, or `null` when this server run has never
loaded anything:

```json
{ "operationId": 1, "kind": "import", "source": "Unser Zuhause.knxproj",
  "phase": "parseTopology", "completed": null, "total": null,
  "status": "running", "error": null,
  "clientToken": "3fa8b1d2-...-c9e4" }
```

`status` is `running`, `succeeded` or `failed`; `error` is set only when
`failed`. The snapshot deliberately carries **no timestamps and no
duration** — see the percentage rule.

**The POST keeps its shape.** `/api/project/import` and `/api/project/open`
still answer with the `ProjectTree` (or an error), unchanged for every
existing caller including `knx-cli` and the Tauri shell. Their request body
(`PathBody`) gains one optional field, `clientToken`, read only by these
two handlers; every other route sharing `PathBody` ignores it, and a
caller that never sends it (`knx-cli`, the Tauri shell) simply starts an
operation nobody can claim by token — harmless, since nothing but a
browser polls. The work moves to `tokio::task::spawn_blocking`, which it
should always have been on: an import is seconds of CPU on a runtime
worker thread, and a blocked worker is what would have made a progress
poll arrive late.

**The phases come from the pipeline, not from the HTTP layer.**
`knx-etsproj` gains `ImportStage`/`ImportObserver` (its own vocabulary, no
new dependency), `knx-app` gains `LoadStage`/`LoadObserver` which forwards
the parser's stages and adds its own, and `knx-server` maps both — plus the
store and projection steps it drives itself — onto one wire vocabulary in
`load_progress.rs`. Each crate names only its own work.

| wire phase | emitted by |
| --- | --- |
| `starting` | `knx-server`, at `begin` — the operation exists, nothing has reported yet |
| `openContainer`, `detectSchema`, `parseTopology`, `parseProjectInfo`, `validate`, `map`, `inferDatapointTypes`, `collectContainerEntries` | `knx-etsproj` |
| `ingestManufacturerData`, `ingestMasterData`, `enrichFromProductDatabase`, `persistOpaque` | `knx-app` |
| `openStore`, `loadStoredProject`, `loadOpaque`, `loadManufacturerRefs`, `buildProjectTree` | `knx-server` |

**A percentage exists only where a real completed/total exists.** Two
places in the whole pipeline count something whose total is known before
the loop starts: collecting container entries (total = archive entries not
regenerated) and ingesting manufacturer files (total =
`ImportOutcome::manufacturer.len()`). Those two phases carry `completed`
and `total`; every other phase carries `null` for both and the UI shows an
indeterminate indicator with the phase label. Elapsed time is never a
progress source, and the snapshot has no timestamp for anyone downstream
to be tempted by. Neither is the *phase ordinal* published as a fraction:
"stage 6 of 17" looks like a measurement but the stages have wildly unequal
durations, so a bar driven by it would move at a lie's pace.

**A client that disconnects does not cancel anything.** The operation
belongs to the server, not to the request that started it: the blocking
task runs to completion, the project is replaced or not on its own merits,
and the snapshot survives for whoever polls next. The lost response is the
disconnecting client's problem, and a documented one — there is no
`GET /api/project` to re-fetch the tree from, so a browser that loses the
response sees the outcome in the snapshot and must reload the page
(`docs/KNOWN_LIMITATIONS.md`).

**The previous project survives until the replacement is complete.** Both
domain functions build everything they need before a single assignment
to `AppState`, so a failure at any stage leaves the open project, its
`store_path` and its undo history exactly as they were. The failure is
reported twice: in the POST's error body, and in the retained snapshot's
`status: "failed"` + `error`.

**Cancellation stays out of scope**, on the condition the brief named: an
operation that cannot publish partial state has nothing to roll back, and
this one publishes exactly once, at the end. If a future stage ever writes
into the live project as it goes, this decision has to be revisited with
it.

## Alternatives considered

**Server-sent events (`text/event-stream`).** Push instead of poll, no
interval to tune, and the natural fit for a progress stream. Rejected for
this slice: it makes the response the operation's lifetime, so a
disconnect becomes a decision ("does the import continue?") rather than a
non-event, and it needs a second code path in the Tauri shell, which
speaks the same routes. Polling also matches what `bus_routes.rs` already
does for telegrams, so the frontend gains no new mechanism. Worth
revisiting if a future operation needs sub-100 ms feedback.

**A WebSocket.** Everything SSE costs, plus a protocol upgrade and a
dependency the workspace does not have.

**Progress inside the existing response** (chunked JSON lines, or HTTP
trailers). No new route, but every existing client would have to learn a
new body shape for a response they already parse, and `fetch()` in the
browser cannot read trailers at all.

**Coarse phases invented by the HTTP layer** ("reading", "processing",
"done"). No plumbing, no crate changes — and no truth: "processing" would
cover 90 % of the duration and say nothing. Rejected on the ADR's own
premise.

**A percentage derived from file size or elapsed time.** The cheapest
convincing bar, and a lie by construction: the archive's compressed size
predicts neither parse time nor entry count, and a time-based bar reports
the clock, not the work. Explicitly forbidden, not merely unused.

**Letting the frontend start a second load and racing them.** Two imports
would fight over `AppState.project` and the winner would be whichever
finished last. `409` instead, and a disabled button so it rarely comes up.

## Consequences

Easier: any future long operation (export, a product-database install, a
diff) can reuse `LoadOperations` by adding a `LoadKind` and its phases;
the frontend's banner already renders anything with a phase and an
optional count. `spawn_blocking` also removes a real, if unreported,
runtime hazard — before this, a multi-second import blocked a tokio worker
and every concurrent request that landed on it.

Harder: the pipeline now has an observer parameter threaded through it.
`import_knxproj`/`import_knxproj_bytes`/`collect_container_entries` and
`knx_app::import_ets_project_with` keep their signatures and delegate to
`*_observed` variants with a no-op observer (`impl ImportObserver for ()`),
so no existing call site changed — but a new stage added to the pipeline
without an `observer.stage(..)` call is a stage the user will never see,
and nothing but review catches that.

Enforced by tests: `crates/knx-etsproj/src/progress.rs` records an import's
stages and asserts the exact order; `crates/knx-app/tests/load_progress.rs`
does the same across both crates and asserts the determinate counts are a
real count, not a guess; `apps/knx-server/src/load_progress.rs` covers the
lifecycle (duplicate refusal, failure retention, id monotonicity, the
phase change that clears a stale item count, a token stored and echoed
verbatim, and no token stored as `None`, never a guess);
`apps/knx-server/tests/http_load_progress.rs` covers the route, the
`409`, that a failed load leaves the previously open project in place,
and that `clientToken` round-trips through the HTTP layer in both
directions — on the import route, on the native open route, and on a
load that fails (fix round 4, F11 and F13: the open route had no such
test, so dropping the token from that one POST body passed every gate);
`apps/knx-web/src/api.test.ts` asserts both POST bodies actually carry
the token; `apps/knx-web/src/loadProgress.test.ts` covers `ownsOperation`
as exact token equality (including the same-basename case fix round 3
closed, and the absent/empty tokens on either side that fix round 4's
positive guard refuses) and `LoadProgressBanner.test.tsx` covers
determinate vs
indeterminate rendering, the `aria-live` announcement and the failure
state; `App.test.tsx` covers duplicate prevention in the UI and that a
same-basename stranger's failure is never rendered as the local client's
own.

Not covered: nothing proves a phase label is *accurate* — that the string
`parseTopology` is emitted where topology is parsed is a claim review
makes, not a test. And no test measures how long a phase takes, on purpose:
a duration assertion would be the first step back towards a time-derived
bar.
