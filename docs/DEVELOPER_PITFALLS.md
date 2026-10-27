# Developer pitfalls

Repository facts that have tripped up contributors and coding agents more than
once. Each entry was checked against the source on `origin/main` `ae9c44f4`
(2026-10-12); if the code moves, fix the entry instead of working around it.
How to run the gates themselves is in [Repository verification targets](VERIFICATION.md);
this page only lists the traps around them. Think of it as the list of rakes
already found lying in the grass.

## Gates

- **Production functional cases run independently within one file (ADR-0155).**
  Only the reviewed production functional lane opts into `--fully-parallel`;
  default and exclusive lanes keep file-local ordering. Do not introduce shared
  test/module state or ordering hooks. `tools/production_lanes.py` and its owning
  tool test catch common hazards, but aliases and hidden effects still need
  review. Config selections must use complete explicit spec literals, not an
  apparently literal array extended by an expression. A direct targeted web
  measurement does not run the package tool tests; invoke the guard separately.
- **Do not move a `CARGO_TARGET_DIR` between gates.** Tauri's build script caches
  absolute paths under `target/debug/build/tauri-*/out`; after a move, clippy and the
  workspace tests stop at `failed to read plugin permissions` for `knx-desktop`. Keep
  the directory where it was built, or link the old path to it.
- **Build the web bundle before any workspace-wide Rust step.** `knx-desktop`'s
  build script needs `apps/knx-web/dist` (the Tauri `frontendDist` and the
  release `frontend` resource). In a fresh worktree run
  `(cd apps/knx-web && npm ci && npm run build)` first; the build script
  otherwise stops with a message naming that command. Never work around it
  with `--exclude knx-desktop`, which hides whether the desktop shell still
  builds (chosen over a placeholder bundle on 17 October 2026: a placeholder
  would have to rewrite two Tauri config keys and could let a build succeed
  without the real UI). [Rust workspace gate](VERIFICATION.md#rust-workspace-gate).
- **`knx-net`'s loopback-join test reads the host's `/proc/net/igmp`.** When that
  table does not fit one `read(2)` (many interfaces or groups at that moment),
  `the_loopback_join_lands_on_lo_and_nowhere_else` refuses the truncated snapshot
  and fails. That is host state, not the change under test: rerun `knx-net` on the
  frozen tree and report both results.
- **A test that spawns a binary inherits the developer's `HOME`.** `knx`,
  `knx-server` and the desktop shell find the user's product database through
  `knx_productdb::default_path()` (`$XDG_DATA_HOME/knx/products.sqlite`, else
  `~/.local/share/knx/`). A spawned `knx doc-export` used to migrate that real
  file to the branch's schema: one gate on a product-DB migration branch left the
  owner's database at v26, and every `main` build and AppImage (v25) then ran
  without a catalogue (19 October 2026). Give every spawned command its own
  `XDG_DATA_HOME` (a temporary directory, or one that never exists), and open
  the user's product database from a reading command only with
  `knx_productdb::open_read_only` (ADR-0090).
- **A fix on `main` does not protect a gate on an older branch.** A gate runs
  the branch's own tests: the same day, a gate on a product-DB branch that
  predated the `doc-export` fix migrated the owner's fresh import (made with
  the Alpha 14 AppImage) to v26 again, inside its `rust-tests` stage. The
  package gate therefore runs its test stages (`tool-tests`, `rust-tests`,
  `bindings`, `corpus`, website and story tests, and the web gate's own
  `tool-tests` step in a complete run) with empty scratch
  `XDG_DATA_HOME`/`XDG_CONFIG_HOME` and fails when a file in the real `knx`
  data or config directory, or behind a symlink in it, is created, changed or
  removed meanwhile (`userDataTouched` in the receipt; `tools/user_data_guard.py`).
  It also fails when it cannot check them (`userDataUnchecked`: a directory it
  may not list or reach, a symlink to an ancestor or to `$HOME`, which it does not walk,
  or more than 5,000 entries). Rebase before gating, so
  the branch carries this runner. The web gate's browser steps keep the real
  values on purpose (fontconfig reads fonts and configuration through them,
  and browser specs pin pixel widths).
  `AppState::default()` itself opens no product database (AR18 M8).
- **Cargo started inside the checkout gets scratch homes.** `.cargo/config.toml` sets
  `XDG_DATA_HOME`/`XDG_CONFIG_HOME` to `target/dev-home/{data,config}` for
  `cargo test`, `cargo run` and build scripts when Cargo's invocation directory
  is the checkout or one of its descendants (24 October 2026). Cargo searches
  for config from that directory and its ancestors: `--manifest-path` does not
  change the search, so a command started elsewhere bypasses this protection.
  Start Cargo inside the checkout even when using `--manifest-path`, or supply
  synthetic XDG homes explicitly. The config uses
  `force = true` because desktop sessions (via the systemd user environment)
  and GitHub's Ubuntu images export the *real* homes, which `force = false`
  would let win. Consequences: a `cargo run` of the server or desktop shell
  starts with an empty catalogue, without your user fonts and fontconfig
  settings, and build scripts' `git` calls skip `~/.config/git`; the
  directory survives between runs (delete `target/dev-home` for a clean
  slate). To use real data, run the built binary directly
  (`target/debug/knx-server`), or let the exported value win with
  `cargo run --config env.XDG_DATA_HOME.force=false --config env.XDG_CONFIG_HOME.force=false …`.
  With those `force=false` overrides, the exported paths are accepted as-is;
  choosing safe scratch locations is the caller's responsibility. A custom
  exported path is not rejected merely because it could contain real data.
  The package gate and `tools/run_corpus_tests.py` pass exactly these
  arguments to keep their own scratch homes (`CARGO_KEEPS_EXPORTED_HOMES`).
  Nothing is baked in: a binary started outside Cargo (release artifacts,
  the AppImage) sees only its own environment. `knx-cli`'s `cargo_dev_homes`
  tests retain the default-home guard and cover both the external-cwd bypass
  and an explicit caller-supplied scratch override.
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
- **Pick the gate by purpose.** A routine package runs
  `python3 tools/run_package_gate.py --base origin/main` (change-scoped); only
  Alpha, Beta and full releases run it with `--full` (ADR-0144). Look at
  `--dry-run` first: if the plan says `COMPLETE GATE`, a path without a rule
  was changed; give `tools/gate_scope.py` a rule instead of living with it.
- **A new Rust integration test is a module, not a file of its own.** Put it in
  `<crate>/tests/it/<name>.rs` and add `mod <name>;` to `tests/it/main.rs`; a
  file the bundle does not declare is never compiled, and a new loose
  `tests/*.rs` costs a link of its own. `tools/tests/test_rust_test_layout.py`
  refuses both. Run one file's tests with `cargo test -p <crate> --test it <name>::`.
  All tests of a crate share one process, so a test must not rely on
  library-level global state being fresh. A helper module used by one test
  file only and reaching back with `super::` lives below that file
  (`tests/it/dynamic_tree/dynamic_scalar_copy/mod.rs`); shared helpers sit in
  `tests/it/<helper>/` and are named with `crate::`. Trait imports are shared
  too: a `.as_ref()` that was unambiguous in a file of its own can become
  ambiguous in the bundle, so compare with explicit types (`&*bytes`).
- **Every commit relinks the binaries' dependents.** `knx-build-stamp` watches
  `HEAD` and its reflog in development builds, so a commit, checkout or reset
  rebuilds `knx-cli`, `knx-mcp` and `knx-server` and everything that links
  them (147 s for the old 356-binary layout on 19 October 2026). Commit, then
  test, rather than alternating, when timing matters.
- **Full debug information is opt-in.** The workspace builds with line tables
  and optimised third-party dependencies (ADR-0144). Backtraces keep file and
  line; for variable inspection in a debugger build with
  `CARGO_PROFILE_DEV_DEBUG=2`.

- **Do not run `cargo test` as namespace root.** Under
  `unshare --user --map-root-user`, `knx-mcp`'s
  `a_persistence_failure_after_approval_leaves_the_file_unchanged` refuses its
  precondition (root ignores file modes) and fails. Run the Rust suite with
  `--offline` outside that wrapper; keep the wrapper for browser suites.

- **Record a child's PID only inside the block that cleans it up.** A
  supervisor may act on the receipt the moment the PID appears; a signal in the
  gap between writing it and entering `try` orphans the child's whole session
  (`web_gate_support.MeasuredRun.step`, fixed 17 October 2026).
- **File existence does not mean a child-written marker is complete.**
  `Path.write_text` creates the file before writing its content. Publish with a
  same-directory temporary plus `os.replace`, or make the bounded reader retry
  both the read and parse before it acts on the marker.
- **A process group is not a stage's process tree.** Playwright's `webServer`
  may start a new session, outside the stage leader's group, and an environment
  marker disappears when a child clears its environment or becomes a zombie.
  Gate stages therefore run below a dedicated Linux child subreaper whose
  direct children are exclusively stage-owned. Signal only PID/start-time
  identities retained by pidfds, then reap them after bounded TERM/KILL
  escalation; never replace that ownership proof with `pkill -f` or command
  substring matching.
- **Cleanup must not itself be cancellable.** Defer handled signals across
  TERM/KILL/reap on both runner and supervisor, including normal-completion
  leftovers; restore the caller's mask afterwards so interruption still fails
  the attempt. Keep normal cleanup inside the exception guard so interruption
  at entry/exit can safely retry it. An exclusive subreaper killed mid-cleanup
  loses ownership of detached, environment-scrubbed children; a non-atomic outer
  `/proc` scan cannot reliably recover that ownership. SIGKILL cannot be
  deferred: forcible supervisor death remains a cleanup limitation, not merely
  an incomplete-receipt case. Do not compensate by claiming unrelated children.
- **A Python test child that waits for a signal must block it and take it
  with `sigtimedwait`.** On CPython 3.14.7 a pending handler does not run when
  a keyword call returns: in `os.system(command=f'kill -TERM {os.getpid()}')`
  followed by `time.sleep(3)` the handler runs after three seconds, while the
  positional call takes 0.06 s. A child with a SIGTERM handler that announced
  itself with `print(..., flush=True)` and then slept lost the signal whenever
  it landed while `print` returned, which the reader's own wake-up makes
  likely under load; `test_cli_sigterm_waits_for_child_shutdown_before_cleanup`
  timed out in 14 of 54 loaded gate-shaped suite runs for this reason
  (24 October 2026).
- **Prove a work bound by counting, not by the clock.** A loaded host can stall
  a thread for seconds inside a window that costs microseconds:
  `hostile_nested_zips_are_refused_in_linear_time` failed at 3.2 s against a
  1 s budget for 71 µs of work, and a debugger pause of 2 s inside the window
  reproduces it. The ZIP validator now counts the bytes it inspects
  (`package::zip_work`, test builds only) and the test bounds that count by
  the input size; a 30 s clock at 2 MiB remains only as a backstop for work
  outside the meter (17 October 2026).
- **`tempfile::tempdir()` inherits the runner's `TMPDIR`, which may be a
  symlink.** A fixture that compares paths with resolved ones (`canonicalize`,
  an `openat2` no-symlink open, a directory sync chain) must anchor on
  `temp.path().canonicalize()`, or name the symlink explicitly when the
  symlink is what it tests. Five fixtures assumed a symlink-free temp path
  until P4 (17 October 2026); Rust gates now run with the configured `TMPDIR`
  unresolved.
- **On ext4 a deleted file's inode comes straight back.** Deleting a file and
  creating another in the same directory reused the inode 200 of 200 times on
  the project drive (17 October 2026). A file identity that must notice a
  replacement needs the change time as well as device, inode, size and
  modification time; tests that pass on tmpfs can fail on ext4 for this reason.
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
- **The maintainer's checkout moved on 17 October 2026** from
  `<old-sources>` to `<sources>`; the old path is a
  symlink to the new one. Dated records keep the old path as history. New
  code and tests derive sibling locations from the checkout (`../ProjectStats`,
  `../KNXBench.story-private`, `../knx-spec-kb`) or take an environment
  override, never a disk path. After moving a checkout, run
  `git worktree repair <worktree paths>` from the main checkout so every
  linked worktree points at the new location. Path-keyed tool state outside
  the repository moves too: Codex trusts projects by path
  (`~/.codex/config.toml`), Claude Code files memory under a path-derived
  project key (`~/.claude/projects/-mnt-…-KNXBench/memory`), and a running
  container keeps its bind-mount source from creation time, so recreate it
  rather than relying on the symlink.

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

## KNXnet/IP tunnel

- **Blocking durable IO on a current-thread runtime starves the tunnel.**
  `connect_tunnel` spawns the receive and heartbeat tasks, and the receive
  task answers every TUNNELLING_REQUEST with a TUNNELLING_ACK. On a
  `new_current_thread()` runtime those tasks run only while the operation
  future awaits, so a synchronous checkpoint inside it (fsync, readback,
  validation) leaves the gateway's request unacknowledged. The gateway repeats
  it once after one second and then drops the connection (Tunnelling
  v01.07.01 AS §2.6.1); the next request then times out on a dead channel.
  Seen 2026-10-26 in a System-2 validation run that stopped at its recovery
  barrier without sending a write. Build the runtime for such a command with
  `tunnel_runtime()` (`apps/knx-cli/src/tunnel_runtime.rs`); its loopback test
  fails when the helper falls back to a current-thread runtime. `device
  system2-validation` (hardware execute and recovery) and `device
  system2-project`, `device download`, `device restore` and `device
  service-control` use it; the last three persist a pre-write backup and a
  send-possible marker between frames. A source-contract test in `main.rs`
  pins those three call sites (not a CLI end-to-end proof); other tunnel
  commands still build current-thread runtimes. The server's System-2 worker
  had the same starvation on a runtime worker (issue #30; a container limited
  to one CPU can end up with a single worker): it runs its executors through `off_tunnel_workers`
  (`block_in_place`) in `apps/knx-server/src/device_system2/worker.rs`. The
  ordinary `/api/device-download` backup (`keep` closure) and the service-control
  backup still persist on a runtime worker while they hold a tunnel; they are
  open in [S2-04](open-work/S2-04-tunnel-loss-after-checkpoint.md). Any new server
  path that persists synchronously while it holds a tunnel needs the same.
- **A failed tunnel send names its cause.** `BusError::TunnelClosed` carries a
  `TunnelClosure` (gateway disconnect, unacknowledged sequence and last ACK
  status, heartbeat, socket, client). Do not map it back to a bare timeout. After
  an unacknowledged request the receive loop keeps running until `disconnect()`
  so the gateway's `DISCONNECT_RESPONSE` can still confirm the teardown.

## Docker image

- **The image's frontend stage sees only the files it copies.** `npm run build`
  typechecks `apps/knx-web`, and `src/fixtureServer.test.ts` imports the
  Playwright config, which reaches `tools/browser_tmp.cjs`. A web build that
  works in the checkout can therefore fail only in `docker build`. The stage
  mirrors the repository layout (`/src/apps/knx-web`, helpers in `/src/tools`);
  `tools/tests/test_docker_frontend_context.py` fails when an app file imports
  a repository file the stage does not copy beside it.

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

- **A browser case that measures real time gets `{ tag: "@exclusive" }`.**
  Everything else runs with four workers. `waitForTimeout`, `performance.now`,
  `Date.now`, `requestAnimationFrame`, `setTimeout` and `setInterval` in a test,
  hook or helper it calls must sit in a test or `describe` carrying the tag;
  `python3 tools/browser_lanes.py` names the line otherwise, and the gate stops
  before the build. A Playwright fixture (`test.extend`) or module code that
  waits cannot be tagged and is refused where it is declared: move the wait
  into a named helper the tagged tests call. Keep test-side helpers in `.ts`
  below `e2e/`; a `.tsx` import from a spec or helper is refused, and helpers
  in the app tree are not followed. Call clocks directly
  (`performance.now()`), not through destructuring, a re-bound alias, a
  getter or a bracket call, which the guard does not see. Prefer an explicit
  settled-state wait over the tag where the case does not measure time
  (ADR-0144).

- **An installed Playwright clock keeps running.** After `page.clock.install()`
  time flows naturally until `pauseAt`, so the phase of an app interval
  relative to a later `runFor` depends on how long the steps before it took.
  Step the clock until the awaited state appears (see `runUntilPulse` in
  `explorer-structure.e2e.ts`) instead of assuming where a tick falls, and wait
  for rendered content before advancing past the timer that measures it.

- **The built-server native specs are outside the web gate.** `rename`,
  `functions` and `name-recipe` (`*.native.ts`) run only through
  `tools/verify_name_editing.py --spec`, so a change to disclosure defaults or
  the open flow can leave them red until a release gate. Run all three after
  such a change. Open helpers there wait for `POST /api/project/open` itself:
  after a reload the welcome card is already gone, and a late open resets a
  selection made in the meantime. A narrow layout remounts the navigation pane
  with its sections folded again.

- **Keep the central Chromium temp guard in every Playwright config.**
  `browserTmp.ts` and standalone Node launchers load `tools/browser_tmp.cjs`,
  which delegates allocation to `tools/browser_tmp.py`. The allocator budgets
  encoded bytes, reuses a private explicit leaf when another component would
  overflow, and deletes only directories it created. Nested Node processes
  inherit the validated leaf instead of adding another directory. A deep
  `TMPDIR` may use an owned, non-writable-by-others `scratch` ancestor (also a
  user-owned `scratch` symlink to such a directory); otherwise
  provide `KNXBENCH_BROWSER_TMP_ROOT` or the gate's `--temp-root`. For one-off
  scripts use `python3 tools/browser_tmp.py --run COMMAND ...`, or preload the
  Node guard with `node --require ./tools/browser_tmp.cjs SCRIPT`. Do not disable
  the guard, add unsafe Chromium switches, reuse
  another process's profile, or move temporary data into system `/tmp` to conceal
  a setup failure. The [contract](contracts/web-test-gate.md#recurring-chromium-socket-path-protection)
  and CI regressions cover direct/CJS config loads as well as the measured runner.

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
- **P1's toast entry/exit spec is timing-sensitive under host load.**
  `toast.e2e.ts` waits 800 ms for `animationend` after a dismissal. While parallel
  gates pushed the load average past ~10, it failed 2/140 and 3/170 times on the
  issue #19 candidate; at normal load interleaved runs passed 340/340 on both
  `main` `07df8a4d` and the candidate, and the accepted #19 gate passed it. The
  same load also timed out `communication-objects.e2e.ts` on its first click
  (70/70 when rerun alone). Record `uptime` with a red browser run and compare
  against `main` before blaming a diff; do not widen the window or retry silently.
  [Receipt](evidence/issue19-docker-hall-link-2026-10-14.json).

- **A browser fixture must keep what `PUT /api/settings` stored.** The app reads
  `/api/settings` again every few seconds (`settingsStore.ts`), so a fixture that
  answers every GET with fixed defaults resets a preference (e.g. the address
  view) in mid-test once a slow host keeps a spec on one view for that long.
  `group-address-views.e2e.ts` showed it RED with a 6.5 s pause and GREEN after
  the fixture remembered the PUTs.
- **A host build is not a Docker frontend build.** The frontend stage copies
  only `apps/knx-web` to `/src`; imports from repository documentation can resolve
  on the host yet fail inside Docker. Issue #19 reproduced this with the bundled
  Hall of Fame JSON. About now links the GitHub document and needs no snapshot or
  Vitest external-directory exception. Verify `docker build --target frontend
  -f apps/knx-server/Dockerfile .` when changing production asset imports; do not
  enlarge the build context or allow-list merely to conceal an accidental
  cross-directory dependency.
- **Two tests pin the diagnostics companion's dependencies.**
  `apps/knx-web/src/DiagnosticsCompanion.test.tsx` pins its transitive imports,
  and `apps/knx-web/src/diagnosticShell.test.ts` scans literal hyphenated ids as
  class names. A new module on a shared import path can turn them red, so run
  the whole Vitest suite once, not only the package's own tests.
- **Install a signal forwarder before the child it protects.** `tools/browser_tmp.py`
  used to spawn its child group and install the SIGTERM/SIGINT/SIGHUP forwarders
  afterwards. A signal landing in that window met the caller's disposition: a
  default or ignoring wrapper dies or drops it without forwarding, and a caller
  handler never sees the child. The forwarders now go in first; a signal that
  arrives before the group exists is kept and replayed once it does
  (`tools/tests/test_browser_tmp_spawn_signals.py`). No field timeout is
  attributed to this window. SIGKILL cannot be forwarded and stays a boundary.
- **Large-data unit tests carry their own timeout.** A Vitest case that renders
  thousands of rows and asserts no elapsed time takes 1–3 s alone, but 5–6.5 s
  in the full suite while other worktrees build (load average about 20 on 16
  cores): Vitest's 5 s default then measures the host, not the code. Such cases
  pass `LARGE_DATA_TIMEOUT_MS` (30 s) as the third argument of `it`. Never use
  it for a case whose assertion depends on time, and never raise a budget that
  a test asserts.
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
- **File menu entries are `menuitem`s, and three submenus start folded.** Since
  UI-03 `getByRole("button", { name: "About KNXBench…" })` finds nothing: use
  `getByRole("menuitem", …)` and `openFileSubmenu(page, "help" | "history" | "exchange")`
  from `e2e/file-menu.ts` before an entry of a submenu
  ([contract](contracts/file-menu.md)). Welcome-screen cards with the same label stay
  buttons. A new File entry must be declared (`FileMenuItem`, or
  `useFileMenuItemProps()` inside a reused component); an undeclared button gets no
  role and no arrow-key or roving-tabindex handling. The menu holds declared entries
  only: anything interactive that an entry produces (the project comparison's result)
  opens in a portalled `Overlay` dialog, never inside the menu, and a dialog's keydowns
  reach `FileMenuContent` through React's portal bubbling, so key handling there must
  test DOM containment, not the React tree. `Overlay`'s focus trap only wraps at the
  ends of its own tab-stop list (`FOCUSABLE_SELECTOR`); a natively focusable element
  missing from it (a `summary` was) is skipped at the ends and unreachable by keyboard.
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
- **A hovered element that moves keeps its hover only until the next frame.**
  Chromium recomputes hover (and fires `mouseleave`) on a rendered frame. Under a
  paused page clock with nothing animating, a card that slid away from the pointer
  still counts as hovered, so the spec passes by luck; any new animation on the page
  (the explorer's structure-line pulse) makes it fail. Hover what stays put.
- **Never run ad-hoc Playwright in a worktree while its gate runs there.** The
  default config and several production configs share `target/playwright/knx-web`,
  and each run clears it at start: the gate's traces vanish (`ENOENT … .trace`).
