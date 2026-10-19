# ADR 0144: Change-scoped gates, tagged browser lanes and bundled Rust tests

Date: 2026-10-19
Status: Accepted
Session: test-suite speed package
Supersedes in part: the per-file hash admission of the
[web test gate contract](../contracts/web-test-gate.md) (14 and 17 October 2026).

## Context

Every package ran the complete 16-stage gate, whatever it changed. The last
one before this decision took 2 h 01 min (02:54 to 04:55 on 19 October 2026,
lease wait included); a second one on the same morning measured, per stage:

- default browser lane: 713 cases on **one** worker, 1,172.9 s. The parallel
  lane admitted files by content hash and closed whenever a shared fixture or
  an admitted spec changed. After its re-admission on 17 October, 21 of the 45
  admitted specs and the shared fixtures changed within a day, so no file ran
  in parallel;
- 15 production suites, serial, 505.6 s;
- the Rust workspace: 356 test binaries (one per integration-test file), each
  linked with full debug information. A cold `cargo test --workspace --no-run`
  took 206.5 s and filled 71.6 GB of target directory; one changed line in
  `knx-core` cost 194.6 s of rebuild, one in `knx-server` 123.4 s. The run
  phase took 467.2 s and was strictly serial: the per-binary `finished in`
  times add up to 460.6 s, and 187 binaries hold five tests or fewer.

The measurements, their conditions and the raw numbers are in the
[test-suite speed dossier](../research/2026-10-19-test-suite-speed.md).

## Decision

The owner decided three things on 19 October 2026; this ADR records how they
are implemented.

1. **The complete gate is for releases.** Alpha, Beta and full releases run
   `python3 tools/run_package_gate.py --full`. A routine package runs the same
   runner without `--full`: `tools/gate_scope.py` maps the changed paths (since
   the merge base with `origin/main`, plus uncommitted and untracked files) to
   the stages they can affect. Cheap static checks always run. Rust tests run
   for the changed members and, when a library input changed (anything but
   `tests/`, `benches/`, `examples/` or Markdown), for every member that
   depends on it. A crate whose sources name a changed repository path re-runs
   too, with its dependents when the name sits in its library; the names are
   collected from string literals whether the path still exists, so deleting
   or renaming a read file re-runs its readers. Web build, typechecks and the
   whole Vitest suite run for every web change and for repository files that
   web sources or unit tests read (a relative path literal or a run of path
   segments such as `"crates", "knx-app", "src", "progress.rs"`). Browser specs
   run when their static import and navigation closure reaches a changed web
   file; runner, build and launch inputs select all of them, and a deleted
   file selects every spec that still names it. A production suite runs for
   production-bundle and shared runner inputs and when one of its own spec
   files or their test helpers changed or was deleted; application sources
   are covered by the default lane. Generated bindings select the bindings check.
   Corpus tests run for the affected packages only. A path no rule recognises
   selects the complete gate: inside `apps/knx-web/` only `src/`, `e2e/`,
   `presets/`, `themes/`, Playwright configs and the named build and runner
   inputs have rules. A change to the planner, the package runner or the
   proof checks they share (`tools/web_gate_support.py`) selects the complete
   gate. Files that screenshot and load scripts write are not web inputs. The runner refuses an empty change set (it would
   test nothing) and uncommitted bindings when it would regenerate them. The
   plan and its reasons are part of the receipt.
2. **Browser cases are parallel unless they measure real time.** Cases whose
   assertions depend on elapsed time or animation frames carry
   `{ tag: "@exclusive" }` on the test or an enclosing `describe` and run
   alone. `tools/browser_lanes.py` refuses an untagged real-time primitive
   before the build. It reads every test-side module below `e2e/` that a
   spec reaches through named, default, namespace, dynamic and side-effect
   imports or re-exports (`export *`, `export * as`, `export { default as }`),
   `.js` specifiers and directory indexes, and solves their taint as one
   fixed point, so import cycles do not depend on order. Functions, arrow
   bindings, object properties and class methods taint their callers, also
   as member calls (`h.wait()`, `this.rest()`). A helper module has no tag of
   its own, so a primitive there that no named function holds (module code, a
   Playwright fixture declared with `extend`, a hook registered on import) is
   refused in the helper; so is a test-side import of a `.tsx`/`.jsx` module,
   which the guard cannot read. Code handed to the page as a string
   (`page.evaluate("…")`) counts like code; Node's timer modules count under
   any local name. Application modules (`../src/…`) are not followed: on the
   test side they hold data and constants, and their own timers run in the
   page. Known blind spots, accepted because no spec uses them and each is
   loud in review: a primitive reached through destructuring
   (`const { now } = performance`) or a re-bound alias
   (`export const pause = nap`). The hash policy file
   `apps/knx-web/browser-lanes.json` is gone. Production suites keep the one
   worker their configs pin: they start their own preview servers, and four
   workers measured no gain worth the unreviewed risk (473 s against 505.6 s
   for the same 15 suites, under foreign load).
3. **Each crate's integration tests are one binary** (`tests/it/main.rs` with
   one module per former file). A test that must own its process stays a
   standalone file and is named with its reason in
   `tools/tests/test_rust_test_layout.py`; that guard also refuses a file in
   `tests/it/` which `main.rs` does not declare (it would never compile).
   Development builds keep line tables only and build third-party
   dependencies optimised without debug information.

## Alternatives considered

- **Keep the hash admission and automate re-admission.** Re-admission still
  needs a run of every changed file in both lanes, and the inputs change with
  nearly every package; the lane would keep closing.
- **Run `vitest related` instead of the whole Vitest suite.** Several unit
  tests read sources through `node:fs` or `import.meta.glob` (for example
  `collapsibleDefaults.test.ts`), which no module graph shows. The whole suite
  takes about 37 s, so it always runs on a web change.
- **Per-test runtime coverage for browser selection.** Would be far more
  selective (234 of the default specs' navigations open the whole app), but
  needs an instrumented recording run and a stored map that can go stale.
  The static closure is exact for fixture pages and errs towards running more.
- **`cargo-nextest`.** Runs binaries in parallel but keeps 356 links, the
  disk use and the per-binary compile of shared helpers; it is not installed
  and would be a new tool dependency.
- **Lower debug information only.** Measured as part of experiment E1; it
  does not remove the serial run phase or the link count.

## Consequences

- A Rust-only package no longer runs browser suites; a web-only package no
  longer runs the Rust workspace or the corpus. CI still runs the whole Rust
  workspace and Vitest on every push to `main`.
- A new real-time browser assertion needs the tag; the guard names the line.
- A new Rust integration test goes into `tests/it/<name>.rs` with a
  `mod <name>;` line in `main.rs`; run one with
  `cargo test -p <crate> --test it <name>::`. Tests in one binary share a
  process: library-level global state that a test changes is visible to the
  others (today only two ID counters exist, both process-wide by design).
- The corpus runner names bundled tests by module path and runs them with
  `--exact`.
- Debugging with variable inspection needs a full-debug build, for example
  `CARGO_PROFILE_DEV_DEBUG=2 cargo test …`.
- The scoped plan is only as good as its rules. Its tests pin the rules, and a
  release still runs everything; a miss shows up there, not in a customer
  build.
