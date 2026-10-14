# Developer pitfalls

Repository facts that have tripped up contributors and coding agents more than
once. Each entry was checked against the source on `origin/main` `ae9c44f4`
(2026-10-12); if the code moves, fix the entry instead of working around it.
How to run the gates themselves is in [Repository verification targets](VERIFICATION.md);
this page only lists the traps around them. Think of it as the list of rakes
already found lying in the grass.

## Gates

- **Do not move a `CARGO_TARGET_DIR` between gates.** Tauri's build script caches
  absolute paths under `target/debug/build/tauri-*/out`; after a move, clippy and the
  workspace tests stop at `failed to read plugin permissions` for `knx-desktop`. Keep
  the directory where it was built, or link the old path to it.
- **`knx-net`'s loopback-join test reads the host's `/proc/net/igmp`.** When that
  table does not fit one `read(2)` (many interfaces or groups at that moment),
  `the_loopback_join_lands_on_lo_and_nowhere_else` refuses the truncated snapshot
  and fails. That is host state, not the change under test: rerun `knx-net` on the
  frozen tree and report both results.
- **CI runs exactly these checks** (`.github/workflows/`): `cargo fmt --all --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
  `npm ci`, `npm test` and `npm run build` in `apps/knx-web`, the ts-rs bindings
  diff, and the `xtask` checks `check-layering`, `check-headers`, `check-anchors`,
  `check-ledger` and `check-corpus-gates`. A local gate that skips one of them is
  a partial gate; say which part ran.
- **`xtask` checks the workspace you point it at.** Since `dea70329`
  (2026-10-01) the scan tasks resolve their root at runtime (`--root PATH`, or
  the current workspace) instead of the checkout they were compiled in. Older
  binaries could report success for a deleted worktree; judge an unexpected
  green by the size of its output, not only the exit code.
- **Header rule (ADR-0018, `xtask/src/headers.rs`):** line 1 is one sentence, a
  `//!` doc in Rust or a single-line `/** … */` in TypeScript, at most
  `MAX_WIDTH` = 100 characters. Wrapping it into a multi-line block turns it
  into a missing header. Files without a header are counted against
  `ABSENT_CEILING`; fix the header, never raise the ceiling.
- **A full disk looks like a build failure.** A shared `target/` directory can
  grow past 150 GB. `No space left on device` or a linker `Bus error` before any
  test result is an environment problem; check `df -h` before a full gate.
- **Read gate verdicts from the gate, not from a pipeline.** In
  `cmd | tail -1; echo $?` the status belongs to `tail`. Redirect to a log and
  read the gate's own exit line.

- **Do not run `cargo test` as namespace root.** Under
  `unshare --user --map-root-user`, `knx-mcp`'s
  `a_persistence_failure_after_approval_leaves_the_file_unchanged` refuses its
  precondition (root ignores file modes) and fails. Run the Rust suite with
  `--offline` outside that wrapper; keep the wrapper for browser suites.

- **Never share one `CARGO_TARGET_DIR` between worktrees for a gate.** Cargo
  can reuse another worktree's artifacts when sources look unchanged,
  including that worktree's baked `CARGO_MANIFEST_DIR` and build stamp.
  Seen 2026-10-13: corpus tests reported "OriginalData not present" in a
  worktree that had it, because the baked root was another worktree. Give
  every gate its own fresh target directory and delete it afterwards.
- **A new Rust file's header is one sentence on line 1** (`//! … .`, then
  an empty `//!`). A two-line opening paragraph counts as no header, and the
  ADR-0018 ratchet fails the gate (seen 2026-10-13 with twelve new files).

## Handover and Git

- **`.ai/` is ignored, but `.ai/CURRENT_STATE.md` is tracked.** A plain
  `git add .ai/CURRENT_STATE.md` prints the ignored-path hint and exits 1, so a
  `git add … && git commit` chain stops there. Stage the handover with
  `git add -u -- .ai/CURRENT_STATE.md`; a new log file needs `git add -f`.
- **Numbered documents collide in parallel work.** `KNOWN_LIMITATIONS §N`, ADR
  numbers and research chapters are assigned per branch. Pick the next number
  after `git fetch`, from `git ls-tree --name-only origin/main docs/adr/` (or
  the fetched file), not from a local checkout. In a merge collision the
  integration branch keeps its numbers.
- **Status of record lives in one place.** Per-ID status is
  [`docs/status/LEDGER.md`](status/LEDGER.md), checked by
  `cargo run -p xtask -- check-ledger`. Other documents describe; they do not
  open or close work.

## Private corpus

- **`OriginalData/` is ignored in both spellings** (`.gitignore`), because a
  worktree borrows the corpus through a symlink, which `OriginalData/` alone
  would not match. A fresh worktree has no corpus, so corpus-backed tests take
  their skip path there; link the corpus before trusting a green run.
- **The xknxproject oracle is a second corpus input.** `project_dump.json`
  (gitignored, at the workspace root) feeds `oracle_xknxproject` and
  `golden_reference_products`; without it they panic instead of skipping. Link
  it into a worktree together with `OriginalData/` before
  `tools/run_corpus_tests.py`.
- **`walk_corpus_files` skips symlinks** (`crates/knx-testsupport/src/lib.rs`)
  and panics on unreadable directories. A symlink farm therefore measures as an
  empty corpus; copy files into a scratch directory and point
  `KNXBENCH_PRODUCT_CORPUS` at it instead.
- **Absent private data must never produce a green test.** `#[ignore]` with a
  reason is honest; a test that silently returns early is not. `check-corpus-gates`
  looks for such silent gates.

## Schema and storage

- **Use the version constants, not literals.** `CURRENT_SCHEMA_VERSION` exists
  in `knx-core` (project model) and in `knx-store` (store migrations). A test
  asserting a literal version breaks on the next migration.
- **Product-DB rewinds share one helper.** Older-version fixtures go through
  `crates/knx-productdb/tests/v20_rewind/mod.rs`; a new migration adds its drops
  there as well as to any rewind that bypasses it.
- **Positional `INSERT INTO <table> VALUES (…)` in tests encodes the old column
  count.** Convert such statements to named columns when a migration adds
  columns.

## Desktop app

- **An unpackaged release `knx-desktop` finds its UI only below a folder named
  `target`.** Tauri resolves the bundled `frontend` resource next to the
  binary only when the binary sits in `target/<profile>/` (or
  `target/<triple>/<profile>/`) beside a `.cargo-lock`. Anywhere else, including
  a `CARGO_TARGET_DIR` like `.target-xyz/release`, it looks in
  `/usr/lib/<app>/frontend`, the embedded server answers every page with 404,
  and the window stays blank while `/api/version` works. That looks like a
  WebKit or compositor problem but is not. For a manual check, copy the binary,
  its `frontend` folder and an empty `.cargo-lock` into `<dir>/target/release/`.
  On Hyprland a window started with `GDK_BACKEND=x11` is visible to the
  computer-use driver; its GTK dialogs need foreground keyboard input.

## Web app

- **Keep gate stage exits separate from freeze metadata.** `inputs_frozen=1`
  is successful proof, not a failed test. The task-status private runner's final
  grep once treated it as a nonzero stage: every actual stage and raw report
  passed. Preserve the original wrapper exit, validate nonempty reports and
  start/end hashes independently, and correct the verdict logic test-first.
- **The existing Toast-hover fixture can fail intermittently on unchanged source.**
  `ux-feedback.e2e.ts:129` (en, 1440) reported zero held error cards instead of one
  on `1817fb2a` (2/20 repetitions) and the task-status candidate (1/20); the final
  integrated suite passed 531/0. The root cause is unestablished; do not attribute
  it to an unrelated feature, weaken the assertion or silently retry it. Retain
  failures and compare against an immutable baseline before changing anything.
  [Receipt](evidence/task-status-shimmer-2026-10-14.json).

- **A browser fixture must keep what `PUT /api/settings` stored.** The app reads
  `/api/settings` again every few seconds (`settingsStore.ts`), so a fixture that
  answers every GET with fixed defaults resets a preference (e.g. the address
  view) in mid-test once a slow host keeps a spec on one view for that long.
  `group-address-views.e2e.ts` showed it RED with a 6.5 s pause and GREEN after
  the fixture remembered the PUTs.
- **A module from outside `apps/knx-web` fails only in Vitest's DOM tests.**
  Vitest's `happy-dom` environment refuses it (`Denied ID …`) while the `node`
  environment does not check, so a unit test can pass while `App.test.tsx` fails
  to load. Add exactly that directory to `server.fs.allow` in `vitest.config.ts`.
  The Vite dev server (and so the browser fixtures) and a production build serve
  it as imported by the app without help; do not widen their allow-lists.
- **Two tests pin the diagnostics companion's dependencies.**
  `apps/knx-web/src/DiagnosticsCompanion.test.tsx` pins its transitive imports,
  and `apps/knx-web/src/diagnosticShell.test.ts` scans literal hyphenated ids as
  class names. A new module on a shared import path can turn them red, so run
  the whole Vitest suite once, not only the package's own tests.
- **Bus-workspace browser fixtures must answer
  `GET /api/bus/monitor/telegrams`.** The bus monitor panel polls it
  (`pollBusTelegrams` in `apps/knx-web/src/api.ts`; server route in
  `apps/knx-server/src/bus_routes.rs`).
- **Browser specs see the wall clock.** After 23:00 local time the app may show a
  late-night humour toast next to the usual ones. Dismiss notifications with
  `dismissToasts` from `e2e/communication-fixture.ts` (front-most first, until none
  are left), not with a snapshot of `.all()` locators: those are nth-based and lose
  track once two are visible. Reproduce a time-of-day failure by running the spec
  with a shifted `TZ` (e.g. `TZ=Asia/Tokyo`).
- **Motion settings live in `data-motion-level`** (set by
  `apps/knx-web/src/motion.ts`), not `data-motion`; assert the attribute the
  app actually sets.
- **Release checks are on by default and read `/api/version` too.**
  `useReleaseUpdates` asks the server for its version and then
  `https://api.github.com/...` on every load, which a `**/api/**` route does not
  catch. A spec that counts version reads or must stay offline sets
  `releaseChecksEnabled: false` in its fixture settings and routes
  `https://api.github.com/**` to a recorded abort (`onboarding-guide.e2e.ts` was red
  about one run in four with 4 reads for 2 until it did).

## Effects, bursts and real-time browser specs (ux-feedback port, 2026-10-13)

- **StrictMode runs every effect cleanup once at mount.** A hook whose cleanup
  discards state (a timer map, a queue) loses whatever an effect of the first pass
  already created, e.g. App's startup toast. Stop timers in the cleanup, keep the
  data, re-arm on mount; test with a StrictMode probe that pushes from an effect.
- **A flag refreshed by an effect lags a burst.** Several synchronous updates run
  before the effect that recomputes `visible`; decide by queue position or by state
  computed in the same update.
- **A browser spec that needs an env var cannot live in the default suite**, which
  runs every `*.e2e.ts`; default to `test.info().outputPath()`.
- **Real-time capture specs need a stepped clock.** A capture that refuses rather
  than skips late frames will refuse on a busy host. Pause the page clock and step
  one interval per frame after the frame's progress appears; the work itself stays
  real. `pauseAt` must be ahead of the page's flowing time (use `Date.now() + 1000`).
