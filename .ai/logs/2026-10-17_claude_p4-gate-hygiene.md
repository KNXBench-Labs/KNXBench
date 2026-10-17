# P4 — test-gate hygiene (Claude, 17 October 2026)

Package: `docs/goals/open-work-2026-10-17/p4-gate-hygiene.md`. Claim `f579f8d4`.
Worktree `KNXBench-gate-hygiene-20261017`, branch `fix/gate-hygiene-20261017`.

## Item 3 — corpus-output refusal through a symlink (`67830ee3`)

- RED: new `matrix_output_is_refused_inside_a_corpus_root_named_through_a_symlink`
  failed under both a symlinked and a resolved `TMPDIR` ("symlinked corpus
  root, real output: None"); the old test failed only under the symlinked one.
- Fix: `validate_output_path` canonicalizes the corpus root as well as the
  output parent. Production callers already passed `canonical_root`, so the
  live matrix/AR05 runs were safe; the guard itself no longer trusts callers.
- Sweep: `tools/run_web_gate.py`, `website/build.py`, `knx-server` `paths.rs`,
  `tools/documentation_history.py` already resolve both sides. Two report
  writers (`procedure_resolution_corpus`, `support_guidance_offline_audit`)
  have no inside-the-source guard at all (one uses `create_new`); not the same
  flaw, left alone.

## Item 2 — linear time by counting (`aaaf291b`)

- Measured window: 71 µs debug, 20 µs release at 256 KiB. CPU starvation
  (SCHED_IDLE plus hogs, SIGSTOP slices) never landed a stall inside it;
  a `gdb` breakpoint at `preflight_zip` with `shell sleep 2` did:
  "262156 bytes took 2.00 s", FAILED. That is the reproduced failure.
- Fix: `package::zip_work` meter (thread-local, `cfg(test)` only; a no-op
  `#[inline(always)]` otherwise) at every read in `preflight_zip`,
  `validate_central_directory`, the descriptor read and
  `validated_zip_metadata`. Test: both hostile shapes at 16 KiB, 64 KiB,
  256 KiB, 2 MiB; `0 < inspected <= 2 * len`; 30 s backstop at 2 MiB (stricter
  than the old 1 s at 256 KiB for any quadratic path). Control test: a valid
  200-entry archive is read and metered within the same bound.
- Proof: stall replay (eight 2 s breakpoints, 16 s) green; mutant M1
  (forward scan re-reading the span per candidate) killed with 6,132,869 bytes
  inspected for 16 KiB; mutant M2 (unmetered `zip::ZipArchive` refusal) killed
  by "validator not used". Sources restored byte-identically.

## Item 4 — knx-desktop in the workspace gate (`54cdf0e3`)

- RED: fresh-worktree `cargo clippy -p knx-desktop` failed at the build script
  ("resource path `../../knx-web/dist` doesn't exist"); new
  `tools/tests/test_rust_gate_order.py` failed 3/3 (CI ran Clippy and tests
  before the web build; VERIFICATION.md had no Rust gate section).
- Choice: a prepared web build step, not a build-script placeholder. A
  placeholder would need JSON-merge overrides of `build.frontendDist` and the
  `bundle.resources` map (with a `null` delete) plus process environment, and
  could let a build pass without the real UI. The build script now refuses
  early with the remedy command and never substitutes anything.
- Changes: CI order (install + build before Clippy/Test, new test listed),
  VERIFICATION.md "Rust workspace gate", DEVELOPER_PITFALLS, web-test-gate
  contract. Green: test 3/3, `knx-desktop` Clippy and tests 24/0 after
  `npm ci && npm run build`.

## Item 1 — parallel browser lane (`bb7762b3`)

- 23 of 45 admitted specs edited since `f256e881`, fixture fingerprint moved
  (temp tooling `7eda04ff`/`beab0112`, disclosure helpers, lockfile version,
  diff fixture, other configs' entry files). Each classified in
  `docs/evidence/browser-lane-review-2026-10-17.md`; all page-scoped.
- Re-admitted exactly the 45 files (no additions). Benchmark profile:
  success, inputs frozen, six runs 276/276 identical case IDs, 0 retries,
  skips or flaky; independent recount of the JSON reports agrees.

## Item 5 — four knx-app fixtures under a symlinked TMPDIR (`9a922cb8`)

- Found by gate run 1 (the first gate without the "resolved TMPDIR"
  workaround): `backup_directory` alias test, `device_backup` and
  `service_control_backup` nested-sync tests, `authorized_restore_exports`
  symlink confinement test. Production behaved as documented (sync_chain also
  persists the resolved chain; the harness's manifest opener refuses any
  symlink); the fixtures assumed a symlink-free runner temp path.
- Fix: anchor each fixture on `temp.path().canonicalize()`. Green under both
  TMPDIR shapes. Mutants: no resolved chain (1 fail), no ancestors (6 fail),
  symlink-following export opener (1 fail); sources restored identically.

## Package gate

- Run 1 on `9edf5253` (TMPDIR symlinked, full template): web profile success,
  inputs frozen, `fixturesMatch=True`, functional 276 (4 workers, 150 s),
  exclusive 320 (748 s), whole web profile about 25 min; Clippy with a freshly
  checked knx-desktop; Rust 4,838 passed / 4 failed (item 5); bindings, xtask,
  docs, tools 220 OK, corpus 153/153, inputs frozen. Verdict red → item 5.
- Run 2 on merge `cca59b22` (over docs-only `981ad37f`; web half carried, web
  inputs byte-identical to run 1): all 16 stages green, Rust 4,842/0/212 in
  361 binaries, corpus 153/153.
- `main` then moved by `869899a5` (block Access + separators: web, server,
  product DB) and `8e612c29` (docs). Fingerprint on main equals the
  re-admitted value (no fixture/config change), so item 1 stands. Re-merged
  feature tip `9a922cb8` onto `8e612c29` (main first parent) as `f854d356`;
  only conflict: both IMPLEMENTATION_STATUS entries kept. Run 3 = full gate on
  that merge (TMPDIR symlinked): web profile 64/64 stages, success, inputs
  frozen, parallel lane admitted (functional 276 in 135 s at 4 workers,
  exclusive 320); fmt; Clippy -D warnings with a freshly checked knx-desktop;
  Rust 4,855 passed / 0 failed / 212 ignored in 364 binaries (knx-desktop ran);
  ts-rs bindings unchanged; xtask layering/headers/anchors/ledger/corpus-gates;
  check_documentation; tools 220 OK; private corpus 153/153 in 33 targets;
  diff-check; inputs frozen. gate_exit=0.
- `main` moved by handover-only `291eaf8f`/`4aff5f73` (Alpha 14 plan, no
  freeze; it lists P4 item 3 as a release-gate blocker, now fixed). Rebuilt
  the merge as `223c1f16` (rerere replayed the resolution; tree differs from
  the gated `f854d356` only in `.ai/CURRENT_STATE.md`), then this handover
  commit with docs stages.
