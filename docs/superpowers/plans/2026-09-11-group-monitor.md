# Plan — T15: Group Monitor GUI (gap D5, finishing E4's display side)

Date: 2026-09-11
Spec: `docs/superpowers/specs/2026-09-11-group-monitor-design.md` — **the
binding authority**. Where this plan and the spec disagree, the spec wins;
where the spec and the KNX Standard disagree, the Standard wins and the
discrepancy is reported rather than smoothed over. Section references
below (`§4.2`, `D6`, …) are to that spec.
Branch: `t15-group-monitor`, worktree `.worktrees/t15-group-monitor`,
forked from `b88b286`.

The spec's §9 recommends shipping this as two separate branches. It ships
as one branch with two halves instead: the cut it identifies stays exactly
where it is — Tasks 1-3 are the server, Tasks 4-5 the web UI — but D5 is
only honestly closed when a user can watch the bus without a terminal, and
splitting the branch would mean rewriting the same gap-analysis row twice.
Each task still gets its own review pass, which is what §9's argument
actually asks for.

## Global constraints

These bind every task. A task report that ignores one is not done.

1. **Nothing may require a real gateway or a real network.** Not in a
   test, not in a doctest, not behind an env var. Every test added by this
   plan runs on a laptop with no KNX hardware attached. `crates/knx-net/
   tests/live_gateway.rs` stays exactly what it is today — do not touch it,
   do not follow its pattern.
2. **Never lose a telegram silently.** A telegram that existed and cannot
   be shown must be counted and reported to the client. This is the whole
   point of D3 and it is the one thing a reviewer will check hardest.
3. **Never claim ETS parity or KNX certification**, in code, comments,
   tests, UI strings or documentation. Not "matches ETS", not
   "ETS-compatible", not "like ETS's Group Monitor". The spec's §5 states
   plainly where ETS's filtering is richer; keep that tone.
4. **Never claim hardware verification.** Nothing in this slice has been
   run against a physical KNX installation. No comment, doc line or commit
   message may imply otherwise.
5. **Do not downgrade a limitation.** `docs/KNOWN_LIMITATIONS.md` §61 (the
   codec's coverage accounting) is inherited unchanged and must not be
   edited, weakened or "superseded" by this work. Task 6 adds a new entry;
   it does not rewrite an old one.
6. **Do not touch the codec.** `crates/knx-core/src/dpt/` is a dependency
   of this slice, not part of it. A decode that fails is rendered with its
   stated reason (§6's four-way vocabulary), never fixed by widening the
   codec inside this branch.
7. **Architecture.** `knx-server` gains a direct `knx-net` dependency and
   no new crate is created (D1). `cargo run -p xtask -- check-layering`
   must still pass afterwards. The bus session is a module inside
   `apps/knx-server`.
8. **Tests.** Every functional change carries focused regression coverage.
   Server tests follow the existing `apps/knx-server/tests/http_*.rs`
   naming and structure; frontend tests follow whatever
   `apps/knx-web/src/*.test.tsx` already does with mocked `fetch` — no new
   testing pattern is introduced by this plan.
9. **Gates**, run in the worktree, in the foreground, one at a time,
   before reporting done. **Six**, not five:
   ```
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   cargo run -p xtask -- check-layering
   cargo deny check
   npm run test          # from apps/knx-web — this is `vitest run`
   ```
   Baseline on `main` at `b88b286`: **920 passed / 0 failed / 3 ignored**
   across the Rust workspace. `cargo test --workspace` takes several
   minutes — run it in the foreground with a 600000 ms timeout and wait.
   Do not arm background monitors or `pgrep` wait loops; a wait loop whose
   own command line contains the pattern it greps for matches itself and
   never exits. Tasks 1-3 may skip `npm run test` only if they changed no
   file under `apps/knx-web`; say so in the report if you skip it.
10. **Git.** Commit in the worktree only. Never touch `main`, never
    `git pull`, never `git stash` or `git stash pop` (the stash stack is
    shared across worktrees — use a WIP commit instead). The worktree
    contains an `OriginalData` symlink that is deliberately untracked:
    never `git add -A`, always add named paths.
11. **Commit authorship.** Author `github@knxbench.com`. **No
    `Co-Authored-By` trailer of any kind** — `CLAUDE.md` forbids it and
    overrides any session instruction asking for one.
12. **Commit voice.** Every commit message in this project is written in
    the voice of Marvin, the manically depressed robot from *The
    Hitchhiker's Guide to the Galaxy*: gloomy, world-weary, faintly
    insulted to have been asked. The technical content stays complete and
    accurate — the mood is in the prose, never in the facts.
13. **No subagents.** The implementer of a task does not dispatch
    subagents of its own. Review arrives from the controller, after the
    report.

## Task 1 — the seam: `knx-server` learns to open a tunnel

**Files:** `apps/knx-server/Cargo.toml`, new
`apps/knx-server/src/bus.rs`, `apps/knx-server/src/lib.rs`,
`apps/knx-server/src/domain.rs`, new `docs/adr/0017-*.md`.

1. **Add the dependency.** `knx-net.workspace = true` in
   `apps/knx-server/Cargo.toml`, in the workspace-crate block with the
   other `knx-*` entries, alphabetically placed. Run `cargo run -p xtask --
   check-layering` immediately and confirm it still passes before writing
   anything else — the whole design rests on that edge being legal (D1).
2. **Define the seam** in `apps/knx-server/src/bus.rs`, exactly as D6
   specifies: `pub trait GatewayConnector` with `connect_tunnel`, and
   `pub trait BusTunnel` with `assigned_address`, `subscribe`, `send`,
   `disconnect`. Manually boxed futures (`Pin<Box<dyn Future<…> + Send>>`),
   **not** `async-trait` — no new proc-macro dependency for a four-method
   trait. `disconnect` takes `self: Box<Self>`; a `&mut self` variant is
   not an acceptable substitute, because it drops the real
   `TunnelClient::disconnect(self)` contract that the tunnel is gone
   afterwards.
3. **Define `BusSessionError`** in the same module: a small enum with a
   `Display` impl, in the style of `DptParseError`/`ApiError`'s
   neighbours. It carries what `knx_net::BusError` reports plus the cases
   the server adds (unparsable gateway address is *not* one of those — the
   route rejects that before reaching the connector). Implement
   `std::error::Error`.
4. **`RealConnector` / `RealTunnel`.** `RealConnector` holds a
   `knx_net::KnxNetIpClient` and implements `connect_tunnel` by calling
   the real `connect_tunnel` and boxing the result in `RealTunnel`.
   `RealTunnel`'s four methods delegate: `assigned_address` and
   `subscribe` forward as-is, `send` maps `knx_net::BusError` into
   `BusSessionError`, `disconnect` unboxes with `*self` and calls the real
   consuming `disconnect`.
5. **`FakeConnector` / `FakeTunnel`.** These must be reachable from
   `apps/knx-server/tests/` — integration tests link against the `lib`
   target, so they cannot see `#[cfg(test)]` items. Put them behind a
   plain `pub mod fake` (or `pub struct` with a doc comment saying
   test-support), exported from `lib.rs`, and say in the doc comment why
   test-support code is public: an integration test in `tests/` is a
   separate crate and there is no other way. `FakeConnector` decides in
   advance what `connect_tunnel` returns — a controllable `FakeTunnel`, or
   a `BusSessionError` for the "gateway refused" path — and counts its
   calls. `FakeTunnel` exposes the `broadcast::Sender<TunnelEvent>` side
   to the test, records every `send` as `(Destination,
   ApplicationService)` in a call log, and records that `disconnect` was
   called. The broadcast channel's capacity must be settable by the test —
   Task 2's `Lagged(n)` test depends on forcing a real lag.
6. **Wire `AppState`** (`domain.rs`): add `pub connector: Box<dyn
   GatewayConnector>` and `pub bus_session: Mutex<Option<BusSession>>`
   (a placeholder `BusSession` with just an id and the boxed tunnel is
   enough for this task — Task 2 fills it in). `AppState::new` constructs
   a `RealConnector`. Every existing test that builds `AppState` must
   still compile and pass **unchanged**: verify by running
   `cargo test -p knx-server` and confirming the count is what it was.
   `apps/knx-server/tests/http_product_install.rs:40-67` shows the
   `..Default::default()` pattern the fake will be injected with — do not
   add a test-only constructor.
7. **Write ADR-0017** (`docs/adr/0017-knx-server-depends-on-knx-net.md`,
   or a name in the same shape). Follow `docs/adr/0016-*.md`'s structure
   exactly: Date/Status/Session header, Context, Decision, Consequences.
   It decides one thing — `knx-server` depends directly on `knx-net`, with
   no crate interposed — and records the rejected alternative (a separate
   `knx-bus-service` crate) and why: one consumer today, and CLAUDE.md's
   standing guidance against speculative abstraction. Cite the design
   document and the `xtask check-layering` evidence.

**Done when:** the six gates pass (`npm run test` skippable per constraint
9); `cargo test -p knx-server`'s count matches the pre-task count; the ADR
exists; and no route or handler has been added yet — this task is
structure only.

## Task 2 — the session: drain task, capped buffer, honest gaps

**Files:** `apps/knx-server/src/bus.rs` (extend), `apps/knx-server/src/
domain.rs` if `AppState` needs adjusting.

1. **`TelegramBuffer`.** A capped `VecDeque` of rows behind
   `Arc<Mutex<…>>`, with a monotonically increasing `u64` sequence counter
   assigned on push (never derived from the wire — KNXnet/IP telegrams
   carry no sequence number, say so in a comment), a `dropped_before:
   u64`, and a named `MAX_TELEGRAMS` constant. Pick the cap deliberately
   and justify it in the doc comment the way `session_log.rs`'s
   `MAX_ENTRIES` does; `apps/knx-server/src/session_log.rs:57-125` is the
   idiom to read first.
2. **Three ways a telegram can be lost, one counter.** Eviction at the cap
   and `RecvError::Lagged(n)` both add to the *same* `dropped_before`
   counter (§4.2). A test must prove the `Lagged` path specifically:
   construct the fake tunnel with a deliberately small broadcast capacity,
   push more events than it holds while the drain task is not consuming,
   and assert `dropped_before` reaches the exact expected count. "It
   compiles and the eviction test passes" does not satisfy this.
3. **The row model** (§4.4): `seq`, `timestamp` (server wall-clock at the
   moment the drain task received the event — match whatever
   `session_log.rs`'s `LogEntry.timestamp` uses; comment that this is
   explicitly *when the server saw it*, not a bus-side timestamp, because
   KNXnet/IP tunnelling carries none), `source`, `destination`,
   `destination_name`, `service`, `raw_payload`, `decoded`. Reuse the
   CLI's rendering decisions rather than re-deriving them:
   `apps/knx-cli/src/main.rs`'s `format_telegram`, `format_service`,
   `format_group_value_payload` and `format_decoded_value` are the
   reference for every string this produces.
4. **Decoded value** is a tagged enum with four cases —
   `value` / `unresolved` / `conflict` / `error` — each carrying its
   human-readable text, plus the DPT for `value` and the codec error text
   for `error` (§4.3). A single pre-formatted string is explicitly wrong
   here: the frontend must be able to style a conflict differently from a
   clean decode without re-parsing prose.
5. **The DPT and name maps** are computed **once** at session start from
   the project open in `AppState`, via
   `knx_core::resolve_project_group_address_dpts` and the same traversal
   `apps/knx-cli`'s `load_group_address_names` uses. The group-address
   formatting style comes from the project's
   `info.group_address_style` (`crates/knx-core/src/project.rs:152`); with
   no project open, the raw `u16` is shown and names are `null`. Comment
   the staleness explicitly (§4.4): editing the project mid-session does
   not re-resolve anything until the session restarts.
6. **`BusSession` and the drain task.** `BusSession` owns the session id,
   the gateway address, the shared buffer, a status (`Active` /
   `Closed`), the `oneshot::Sender<()>` stop signal and the task's
   `JoinHandle`. The task is `tokio::spawn`ed and loops over
   `tokio::select!` between the stop receiver and `receiver.recv()`,
   exactly as §4.2's sketch shows — which is itself a mirror of
   `run_bus_monitor_async`'s `select!` between `ctrl_c()` and
   `telegrams.recv()`. On `TunnelEvent::Closed` or `RecvError::Closed`:
   push a "session closed by gateway" marker into the buffer so a polling
   client learns *why* telegrams stopped, set the status to `Closed`, and
   break. After the loop, `disconnect()` the boxed tunnel the task owns
   outright.
7. **Exactly one code path clears `bus_session`**, and it is the stop
   handler in Task 3 — the drain task never clears it, even after a
   gateway-side close. §4.1 makes this a ruling with a reason (no race
   between the task and a client-initiated stop); preserve the reason in a
   comment.
8. **Tests** in `bus.rs`'s own `#[cfg(test)] mod tests`, driving
   `BusSession` directly without any HTTP: start-drain-stop; eviction
   accounting at the cap; the `Lagged(n)` accounting from step 2;
   `TunnelEvent::Closed` flipping the status and leaving the buffer
   readable; `disconnect` actually being called on stop (assert the
   fake's flag).

**Done when:** the six gates pass (`npm run test` skippable per constraint
9); the `Lagged(n)` test exists and asserts an exact count; no HTTP route
exists yet.

## Task 3 — the four endpoints

**Files:** new `apps/knx-server/src/bus_routes.rs` (or a `bus_routes()`
function beside the existing `project_routes()`/`fs_routes()` — match
whichever fits `routes.rs`'s existing shape better and say which you chose
and why), `apps/knx-server/src/lib.rs`, `apps/knx-server/src/errors.rs`,
new `apps/knx-server/tests/http_bus_monitor.rs`, new
`apps/knx-server/tests/http_bus_write.rs`.

1. **Four routes**, merged into `app()` in `lib.rs` next to
   `project_routes()` and `fs_routes()`, with the existing handler idiom:
   `#[serde(rename_all = "camelCase")]` DTOs, `Result<Json<T>, ApiError>`
   returns.
   - `POST /api/bus/monitor/start`
   - `POST /api/bus/monitor/stop`
   - `GET  /api/bus/monitor/telegrams?since=<seq>`
   - `POST /api/bus/write`
2. **The wire shape is §4.3, exactly** — field names, status codes and
   error bodies. Do not improvise a field name; the frontend's
   hand-written bindings in Task 4 are checked against that section
   verbatim. The parts most easily got wrong, called out:
   - `start` returns `sessionId` and `assignedAddress`; `409` when a
     session is already active, and the body **names** the existing
     session (id and gateway); `400` for an unparsable gateway address;
     `502` when the connector reports a `BusSessionError`.
   - `stop` returns `sessionId`, `telegramCount`, `droppedCount`, and only
     returns **after** the drain task has actually exited — await the join
     handle, do not fire and forget. `409` when there is no session.
   - `telegrams` returns `sessionId`, `status` (`"active" | "closed"`),
     `nextSince`, `droppedBefore` and `telegrams[]`. A `since` below the
     buffer's floor is **not** an error — that is what `droppedBefore`
     communicates. No active session is `404`, not `409`: a `GET` against
     a session that never existed is a not-found, not a conflict over
     state you tried to mutate.
   - `write` takes `destination`, optional `dpt`, `value`; returns
     `encodedPayload` and `service`. `409` with no session (D5 — never
     silently open a second connection), `400` for bad address, encode
     failure, or an unresolved/conflicting DPT with none supplied, `502`
     when the tunnel's `send` fails.
3. **`502` needs a note.** `errors.rs`'s doc comment draws an explicit
   400/500 split by *which operation failed*. A far-end failure is neither
   — extend that doc comment to say so, using `ApiError::with_status`
   (which already exists as the escape hatch for exactly this kind of
   case). Do not weaken or restate the existing split; add the third
   category beside it.
4. **DPT resolution for a write** mirrors `resolve_write_value`
   (`apps/knx-cli/src/main.rs:~1609`): an explicit `dpt` in the request
   wins; otherwise the session's cached resolution decides;
   `None`/`Conflict` is a `400` naming the reason, **never** a guess.
5. **Tests** — `http_bus_monitor.rs` and `http_bus_write.rs`, following
   the structure of the existing `http_*.rs` files, injecting
   `FakeConnector` through `AppState { connector: …,
   ..Default::default() }`. At minimum the six cases enumerated at the end
   of the spec's D6, plus: the `502` path, the `404`-vs-`409` distinction
   for `telegrams`, and a poll whose `since` is below the floor returning
   rows with a non-zero `droppedBefore` rather than an error.
6. **A decode test with a real project fixture.** At least one test opens
   a project (the way the existing tests do), feeds a telegram for a group
   address whose DPT resolves, and asserts the row's `decoded` is the
   `value` case with the expected text — and a second address that does
   not resolve, asserting the `unresolved` case with its stated reason.

**Done when:** the six gates pass (`npm run test` skippable per constraint
9); all four endpoints answer exactly the shapes in §4.3; every new test
runs with no gateway and no network.

## Task 4 — the web table and its filter

**Files:** `apps/knx-web/src/api.ts`, new
`apps/knx-web/src/BusMonitorPanel.tsx`, new
`apps/knx-web/src/BusMonitorPanel.test.tsx`, `apps/knx-web/src/App.tsx`,
`apps/knx-web/src/styles.css`.

1. **Hand-written bindings** in `api.ts`, not `ts-rs` (§4.3).
   `apps/knx-web/src/bindings/` holds only generated `knx-projection`
   types; every server-local DTO is hand-written — `LogEntry` at
   `api.ts:433-445` is the precedent, including its comment naming the
   Rust source and stating it is deliberately not `ts-rs`-bound. Follow
   that, field names matching §4.3 exactly, plus the four fetch functions.
2. **`BusMonitorPanel`** renders: a connect form (gateway address), the
   session's state (id, assigned address, or the error), the telegram
   table, and the gap notice. Polling is `setInterval` while a session is
   active, cursor advanced by `nextSince`, cleared on unmount — the
   interval value comes from §4.2/§4.3's reasoning, named as a constant
   with a comment, not a bare magic number.
3. **The gap is visible in the UI, not just in the JSON.** When
   `droppedBefore` grows, the table shows an explicit row or banner saying
   how many telegrams are missing. This is the user-facing half of
   constraint 2 and the reviewer will look for it specifically. `LogPanel`
   already shows the tone to take.
4. **Filter** (§5): a text filter over `destination` and
   `destinationName`, plus service-type checkboxes. **Client-side only,
   over already-fetched rows** — it never changes what is requested and
   never changes the cursor. `LogPanel.tsx`'s severity filter is the exact
   precedent, including its comment explaining that filters do not
   re-fetch.
5. **Panel wiring** in `App.tsx`: a toolbar toggle in the same slot
   `LogPanel` uses (`App.tsx:306-354`). The panel works **with no project
   open** — same as the Log panel — because monitoring does not need one;
   it just shows raw addresses and no names. Say that in the comment, in
   the style of the existing `KNOWN_LIMITATIONS.md #36` comment beside the
   Log button.
6. **Tests** (`BusMonitorPanel.test.tsx`) with mocked `fetch`, matching
   what `LogPanel.test.tsx` and `ProjectDiffPanel.test.tsx` already do:
   rows render from a mocked poll; the filter hides and re-shows rows
   without re-fetching; a `droppedBefore` jump renders the gap notice with
   the right count; a failed start renders the server's error text.

**Done when:** all six gates pass, `npm run test` included; the panel is
reachable from `App.tsx`; no generated binding was added.

## Task 5 — send from the table

**Files:** `apps/knx-web/src/BusMonitorPanel.tsx` (extend) or a new
`BusComposeForm.tsx` if the panel is getting long — your call, say which
and why, `apps/knx-web/src/BusMonitorPanel.test.tsx` (extend) or a sibling
test file, `apps/knx-web/src/api.ts` if a binding is still missing.

1. **The compose form** sits above the table (§6). Clicking a row
   prefills it: `destination` from the row, `dpt` from the row's resolved
   DPT when the row decoded via a `value` outcome, blank otherwise.
2. **Resolution mirrors `resolve_write_value`** (§6): an explicit DPT
   wins; otherwise `Single` encodes, and `None`/`Conflict` is rejected
   *client-side before the request is sent*, with these messages —
   - `No DPT resolved for this group address — enter one explicitly.`
   - `Conflicting DPTs for this group address: {names} — enter one
     explicitly.`
   deliberately echoing `format_decoded_value`'s vocabulary rather than
   inventing a second tone for the same fact. With no project open, the
   form states that a DPT must be entered explicitly rather than silently
   disabling itself.
3. **Feedback.** On success, show the `encodedPayload` the server actually
   sent — the real send's own echo, closing the loop the way the CLI's
   `--dry-run` preview does, except this one is not a preview. On
   `400`/`409`/`502`, show the server's error text **inline on the form**,
   not only as a toast; a `502` is exactly the kind of thing a KNX
   engineer needs to read in full.
4. **Tests**: a successful send posts the expected body and renders the
   echoed payload; a `Conflict`-resolved address with no DPT never issues
   a request at all (assert `fetch` was not called); a `502` renders the
   error inline.

**Done when:** all six gates pass, `npm run test` included.

## Task 6 — documentation reconciliation

**Files:** `docs/KNOWN_LIMITATIONS.md`, `docs/GAP_ANALYSIS_ETS.md`,
`docs/IMPLEMENTATION_STATUS.md`, `docs/ROADMAP.md`,
`docs/ARCHITECTURE.md`, `docs/COMPATIBILITY.md`.

1. **A new, dated `KNOWN_LIMITATIONS.md` entry** listing items 1-7 and
   9-10 of the spec's §7. Item 8 is a **pointer** to the existing §61
   entry — §61 itself is not edited, not reworded, not marked superseded.
   A reviewer will diff §61 specifically.
2. **`GAP_ANALYSIS_ETS.md`**: update the **D5** row and the **E4** row to
   say precisely what closed — a tunnelling monitor and write GUI — and
   precisely what did not: routing, filtering depth, multi-session,
   individual-address frames, no hardware verification. Mark T15 closed in
   the Tier 4 list. Do not delete a row; the history of what a row used to
   say is the point of the file.
3. **`IMPLEMENTATION_STATUS.md`**: the bus/KNXnet-IP section gains the
   server-side session and the web panel.
4. **`ROADMAP.md`**: T15 moves out of the open list. Leave every other
   memo in that file alone — including the animation-styles memo.
5. **`ARCHITECTURE.md`**: record the new `knx-server → knx-net` edge in
   the dependency description, pointing at ADR-0017 rather than repeating
   its argument.
6. **`COMPATIBILITY.md`**: state what a user can now do against a real
   installation and — in the same breath — that none of it has been
   verified against physical hardware in this branch.
7. **Numbers must be real.** Any test count, endpoint count or limitation
   number written into these files comes from a command you actually ran
   in this worktree, quoted in your report.

**Done when:** all six gates pass; every file above reflects the merged
state; and `git diff` on `docs/KNOWN_LIMITATIONS.md` shows §61 untouched.

## Task 7 — record the user's scope rulings of 2026-09-11

**Files:** `docs/GAP_ANALYSIS_ETS.md`, `docs/KNOWN_LIMITATIONS.md`,
`docs/ROADMAP.md`. **Its own commit**, separate from Task 6's — this is
project scope, not group-monitor work. It rides on this branch only
because it edits the same files Task 6 does and a parallel edit on `main`
would conflict.

On 2026-09-11 the user ruled on the five items that were neither
implemented nor accepted out of scope. Record each **exactly as ruled** —
the difference between "rejected" and "deferred with a named unblocking
condition" is the whole content of these answers:

1. **`.vd2` support — accepted out of scope, permanently.** Its
   gap-analysis row is closed as "out of scope, user decision
   2026-09-11", not deleted, and its limitation entry stays but says so.
2. **Encrypted `.knxprod` — accepted out of scope.** The limitation entry
   stays as written (it is honest about being untested for want of a
   sample) and gains the same dated acceptance note. Do **not** reword it
   into a claim about what encrypted files do or do not contain.
3. **T19 KNX Secure — deferred, and explicitly to be documented as a
   limitation.** The user asked for this in as many words. If there is no
   standalone `KNOWN_LIMITATIONS.md` entry for KNX Secure today, add one;
   if there is, date it and state the deferral. It is not rejected.
4. **T20 Functions — deferred until the new KNX specification
   documentation is available.** Record the unblocking condition, not
   just "later". Stays on the roadmap.
5. **E1 Commissioning — NOT excluded.** The user said plainly it must
   work too, but the work waits until the KNX specification database is
   finished. Its gap-analysis row stays **open**, with the blocker named.
   Every existing place that calls commissioning permanently out of
   scope, a non-goal, or excluded by design is now wrong and must be
   corrected here — leaving it would be the documentation equivalent of a
   silent downgrade. The six known sites, to be re-checked rather than
   trusted, since line numbers move:
   - `docs/KNOWN_LIMITATIONS.md` §7 — title "…are out of scope", and its
     "Lifted when: a deliberate decision to take it on" line;
   - `docs/GAP_ANALYSIS_ETS.md:102` (row E1: "Explicit, permanent scope
     decision … a durable non-goal, not a backlog item");
   - `docs/GAP_ANALYSIS_ETS.md:760-761` ("permanent scope exclusion");
   - `docs/ARCHITECTURE.md:25` and `:274` ("stay out of scope");
   - `docs/COMPATIBILITY.md:83`.

   **Keep the technical cause intact.** The bricking risk, the
   undocumented `Legacy*` matrix and the vendor-DLL involvement in
   download procedures are all still true, and CLAUDE.md's "only
   implement protocol behaviour that is technically verified" still
   binds. What changes is the *disposition*: from "we will never do this"
   to "this is required, and it is blocked until the KNX specification
   database makes the procedures verifiable". Do not delete the reasons;
   re-file them as the blocker.

   For items 3 and 4, note that `KNOWN_LIMITATIONS.md` §8 (KNX Secure)
   and §26 (IP Secure in `BusConnection`) already exist and are honest —
   they need the dated deferral and the T19 pointer, not a rewrite.

**Done when:** the six gates pass; each of the five items is findable in
the docs with its 2026-09-11 date and its exact disposition; no row was
deleted; and item 5's wording nowhere reads as a permanent exclusion.

## Review

After Task 7, the whole branch gets one broad review against the spec's
§8 acceptance criteria — all ten, each independently checkable by a
reviewer who has only this repository. The reviewer re-runs all six gates
itself rather than trusting a report.
