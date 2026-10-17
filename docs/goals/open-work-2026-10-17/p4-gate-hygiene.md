# P4 — Test-gate hygiene

Planning only; part of the [17 October package set](README.md). Follow its
shared rules.

**Starter:** `/goal Run package P4 from docs/goals/open-work-2026-10-17/p4-gate-hygiene.md on fresh origin/main; follow README.md's rules.`

## Entry conditions

None. Runs alongside P2 or P3; it touches tools, tests and gate configuration,
not product UI. If a web package merges first and changes shared browser
fixtures, recompute item 1 before publishing.

## Items

Each item starts with an observed RED (or a reproduced failure) and ends with
its own focused commit.

1. **Browser lanes run serially.** `apps/knx-web/browser-lanes.json`
   `fixturesSha256` no longer matches `fixture_fingerprint()` in
   `tools/run_web_gate.py` (it hashes the shared e2e fixtures, Playwright/Vite
   configs, `package-lock.json` and `tools/browser_tmp.{py,cjs}`; the temp-path
   tooling changes of 17 October moved it). On a mismatch the gate runs every
   spec file exclusively, which is correct but slow. Re-admit the parallel lane
   with the lane-review procedure of
   [the web test gate contract](../../contracts/web-test-gate.md) (measured
   runs, identity equality per lane, no skips or retries); never just paste the
   new hash.
2. **Wall-clock budget under load.**
   `baggage::tests::hostile_nested_zips_are_refused_in_linear_time`
   (`crates/knx-productdb/src/baggage.rs`) asserts each hostile shape finishes in
   under one second; it failed at 3.2 s on a host with load 11–13. Prove the
   linear bound deterministically (for example a counted number of inspected
   records or bytes against the input size) and keep, at most, a generous
   wall-clock guard. The property tested must not get weaker.
3. **Corpus-output refusal through a symlink (safety bug).**
   `matrix_output_is_refused_inside_the_corpus_root`
   (`crates/knx-productdb/tests/corpus_compatibility_matrix.rs`) fails when
   `TMPDIR` is a symlink: `validate_output_path` then misses that the output
   lies inside the corpus root. Write the RED with an explicit symlinked corpus
   path (independent of the runner's `TMPDIR`), then compare resolved paths,
   including a not-yet-existing output file (resolve its parent). Search for the
   same pattern in other "output must not land in the source" guards and fix
   those too.
4. **`knx-desktop` drops out of fresh-worktree gates.** Its Tauri build script
   needs `apps/knx-web/dist` (`tauri.conf.json` `frontendDist`), so workspace
   Clippy/tests in a fresh worktree exclude it and nobody proves it still
   builds. Make the documented gate include it: either a prepared web build
   step before the Rust stages, or a build-script behaviour that keeps check and
   test working without hiding a missing bundle from release builds. Record the
   choice in the web test gate contract and DEVELOPER_PITFALLS.

## Not in scope

Changing test expectations to make them pass, new gate stages unrelated to the
four items, CI re-enablement (owner decision).

## Done when

All four items are on `main`, the web gate reports the parallel lane admitted,
the full workspace gate includes `knx-desktop` in a fresh worktree, Rust tests
pass with a symlinked `TMPDIR`, and the README rule "run Rust tests with a
resolved `TMPDIR`" is removed from this package set.
