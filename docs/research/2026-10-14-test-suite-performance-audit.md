# Test-suite structure and performance audit — 2026-10-14

## Verdict and scope

**The strongest demonstrated problem is serialized browser execution and gate
orchestration, not evidence that the project needs fewer correctness tests.**
Prioritize better scheduling and removal of duplicate harness work. Do not delete
regressions or relax data-integrity checks to make the dashboard greener.

This is an investigation, not an implemented optimization or a fresh whole-project
acceptance. The [machine-readable observations](../evidence/test-suite-performance-audit-2026-10-14.json)
separate source inventory, historical executions and the small checks newly run.

Inspected root: `1817fb2ab3d6cae503433954658f8a54626d9151`.
Tracking ref at initial inspection: `871cc2859be0ae2dc375d0b545feed6d42d6a663`
(not asserted to be the latest remote). Other sessions have active worktrees; the
root also contains unrelated unpublished research. Neither was synchronized,
modified, interrupted or used as an editable benchmark target.

The relevant default CI, package scripts, Vitest config and corpus runner match
that tracking ref. The ref adds a task-status production-browser config and
additional developer-pitfall documentation. Root counts below exclude that added
config. Historical execution counts belong to their own named revisions.

## 1. Current structure

| Layer | Inspected source inventory | Execution model / boundary |
| --- | --- | --- |
| Rust | 294 integration-source files, including helpers | `cargo test --workspace`; binary-level serial execution, parallel cases inside libtest |
| Web unit/component | 250 `*.test.ts(x)` files; 139 explicitly select happy-dom | Vitest 5.0.0; default config does not force one worker |
| Browser | 73 default `*.e2e.ts` candidate files; 29 Playwright configs | 23 explicitly set one worker; the other six inherit one-worker configs |
| TypeScript | 15 configs including the base | All inspected derived configs include application `src` again |
| Python tooling | 11 test files | `unittest`, ordinary sequential discovery |
| Private corpus | 145 selected cases in 32 Cargo targets | A separate explicitly selected, isolated runner; no private tests executed in this audit |
| Hardware | Opt-in ignored cases outside routine selection | Must remain outside any generic acceleration experiment |

These are **source-file/selection counts, not compiled registration counts**.
Helpers, generated registrations and differing revisions explain why historical
runner file/test totals are not identical to this inventory.

Key implementation sources:

- [CI workflow](../../.github/workflows/ci.yml): Rust and frontend stages run
  sequentially in one build job; advisory/license and Docker jobs are separate.
  The workflow has no explicit Playwright execution step. Local browser gates
  therefore exceed this CI job's exercised coverage.
- [Package scripts](../../apps/knx-web/package.json): `build` already runs `tsc`;
  `test:e2e` builds, then executes the default browser configuration.
- [Default browser configuration](../../apps/knx-web/playwright.config.ts):
  `workers: 1`, `fullyParallel: false`, `retries: 0`, dev fixtures on port 4173.
- [Vitest configuration](../../apps/knx-web/vitest.config.ts): node by default,
  individual component files opt into happy-dom; no explicit concurrency cap.
- [Corpus runner](../../tools/run_corpus_tests.py): a blocking Cargo subprocess
  for each selected target, while preserving count checks and scrubbed environment.
- [Verification contract](../VERIFICATION.md) and
  [developer pitfalls](../DEVELOPER_PITFALLS.md): mandatory corpus scopes, selected
  root validation, cache-provenance constraints and isolation requirements.

Dedicated `.study.ts` and `.load.ts` browser configurations already exist and
should not be duplicated. However, ordinary `.e2e.ts` files also contain timing,
large-project, animation and screenshot checks; increasing all their workers at
once would not be a controlled optimization.

## 2. What retained execution actually shows

The archived accepted task-status runtime revision
`53046e6495d271b7d51b01b9aff39ff34d294b85` recorded:

| Observation | Retained result |
| --- | --- |
| Gate interval after acquiring leases | 12:14:49–12:31:44 = **1,015 s / 16 min 55 s** |
| Default Chromium | **531 passed, one worker, reported 14.6 min** |
| Chromium share of that interval | **approximately 86%**, using the rounded browser summary |
| Vitest | 3,530 passed in 251 files |
| Additional production browser proof | 27 passed, 27.3 s |
| Focused repeat proof | 12 passed, 27.9 s |
| Actual command stages | 32, all exit zero; inputs frozen |

Provenance caveat: the original wrapper exited one because it misclassified
`inputs_frozen=1`. A retained corrected aggregation verifies the original actual
stage results and exits zero. It is **not another runtime execution**, and the
original wrapper failure has not been erased or recounted.

Separate recent P2 Vitest-stage logs record 3,589 cases / 256 files in **33.61 s**
and 3,547 cases / 252 files in **40.00 s**. These successful stages do not establish
that the corresponding whole gates passed. Four issue19 JSON reports place
`BusMonitorPanel.test.tsx` at 31.334–36.840 s and `App.test.tsx` at 10.900–13.024 s.
File durations are not additive whole-suite wall time.

The host snapshot shows a Ryzen 7 5800X with eight physical cores / 16 logical
CPUs, about 62 GiB reported memory and an RTX 4070 Ti with 12,282 MiB GPU memory.
A foreign browser gate was executing and another gate was queued on a lease.
No competing browser benchmark was launched. One Playwright worker does **not**
mean one CPU core: Chromium uses multiple processes and threads.

Most inspected acceptance receipts record exits and counts but lack per-stage
wall time, CPU time, peak RSS and time spent waiting for leases. Consequently,
there is no measured total-Rust bottleneck ranking or worker-scaling speedup yet.

## 3. Optimization findings, in priority order

### P0 — Make timing and coverage an explicit runner contract

Use one maintained repository runner rather than independently drifting scratch
wrappers. Record queue wait separately from execution, then per-stage monotonic
wall duration, CPU/RSS where supported, command/exit, tool versions, selected
compiled cases and immutable input identity. Keep failed attempts immutable.
Fail on missing/empty reports and unexpected omissions, not only a wrapper exit.
Playwright 1.63 also supplies a built-in `perfetto` reporter for per-worker timelines
[2]; prefer existing instrumentation before adding a profiling dependency.

Provide named **development**, **integration** and **release** profiles. Focused
development checks are not full acceptance. Integration/release profiles must
retain their documented coverage; a docs-only closure can reuse unchanged runtime
proof only when its full relevant input/provenance commitment still matches.
Never reuse branch proof to certify a changed integrated runtime tree.

Move inexpensive documentation/selection checks before long browser runs. The
inspected private web runner already preflights header width; extend that principle
without silently changing the existing acceptance contract.

### P1 — Parallelize functional browser files, not performance measurements

Start with a **1/2/4-worker A/B matrix** on an immutable representative functional
scope, then the complete admitted functional lane. Keep in-file ordering initially;
`fullyParallel: true` is not needed to parallelize independent files [1].

Audit external shared state first: fixture server state, fixed ports, database
paths, screenshot/report paths, environment-provided capture roots and storage.
Use test-scoped outputs and isolated mutable data. Preserve network isolation,
zero automatic retries, original assertions and failure evidence.

Playwright 1.63.0 is locked locally and its installed types expose named test
locks. The 1.63 release documents test/describe locks [2]; these may serialize
specific shared resources within a runner. Do not assume they replace cross-process
shell leases. The `test.describe.configure({ lock: ... })` convenience API is listed
under 1.64, so do not accidentally require an unapproved upgrade.

Keep timing/paint/FPS/animation-performance checks in an exclusive measurement
lane, with no competing functional browser or compiler workload. A one-worker
performance project running alongside other projects is **not** exclusive.
Preserve the existing study/load lanes rather than building a second framework.

No honest speedup claim is possible until equal-case A/B results exist. Dividing
14.6 minutes by four ignores serial subsets, browser thread use, shared-server
load and setup. The evidence establishes priority, not a promised 4× improvement.

### P2 — Remove repeated TypeScript checks and proven equivalent browser work

The inspected root web runner performs **18 TypeScript compiler invocations**:
`build`'s `tsc`, another app `tsc`, the explicit flow/theme scripts and the 14
`tsconfig.*.json` checks. The archived task-status gate had one added fixture
config, hence **19** invocations. All derived configs include `src` again.

Remove exact duplicate invocations first. Then test whether an app check plus a
comprehensive fixture/config check preserves all admitted files and diagnostics.
Some separate configs may be necessary; prove equivalence before consolidating.
A malformed type introduced into each formerly covered fixture category must
still fail the new gate. Do not merely exclude fixtures for speed.

Browser config selection overlaps, for example `ga-members-download.e2e.ts` in
the standalone members, support and newer task-status production configs. Compare
compiled test IDs, bundle mode, fixtures and environment before deduplicating.
Dev-versus-production coverage is intentional and is not automatically redundant.
Avoid repeated build/preview startups where the same immutable artifact suffices,
without reusing a foreign listener or sharing mutable server state.

### P3 — Narrow lease lifetimes and use a resource budget

Both inspected private web wrappers acquire workspace, alpha and browser leases
before building and keep them through typechecks, Vitest, browser suites, Python
and xtask checks. This blocks unrelated work for the entire gate, not just the
stage with a conflicting resource. Do **not** bypass today's leases.

After introducing resource-owning stages and proving isolation, independent
static checks can run concurrently, browser leases can cover the browser stages,
and performance measurements can acquire a genuinely exclusive resource fence.
Keep fixed acquisition order and deadlock/cleanup tests. Fixed ports such as 4173,
4831, 4832 and 4833 recur across configs and must be handled explicitly.

On this host, test a bounded budget rather than launching 16 workers for each
runner. A useful experiment is four functional browser workers, **or** a Rust
build budget of four/eight jobs, rather than concurrently maximizing all tools.
Compare wall time, peak memory, failures and interference before choosing defaults.

### P4 — Improve Rust/corpus scheduling without losing provenance

The inspected private runner sets `CARGO_BUILD_JOBS=4`. This limits compilation,
not Rust test-harness threads; Cargo runs test targets serially [3]. Benchmark
build concurrency independently of execution concurrency.

`cargo-nextest` is a reasonable **optional experiment**, not an installed feature:
the local version probe fails with “no such command”. Its process-per-case scheduler
can execute cases across binaries [4]. Adoption must retain doctests through a
separate `cargo test --doc` stage [5], zero automatic retries, complete selection,
custom-harness coverage and hardware exclusions. Reject empty selection.

The corpus runner's 145 cases are grouped into 32 sequential Cargo invocations.
Separate build/discovery from execution, then parallelize only proven independent
targets/cases with isolated XDG directories and mutable state. Do not launch many
Cargo builds against one target directory or multiply outer workers by unrestricted
inner test threads. Shared-database coexistence tests must stay ordered. Preserve
the scrubbed `KNX_*` environment, network confinement, count checks and explicit
missing-corpus refusal. Never use indiscriminate `--run-ignored=all` as a shortcut.

Compiler cache experiments can be evaluated later; they are not justification
for sharing or moving Cargo targets between worktrees. The current fresh-target
acceptance policy remains in force. The private runner comment attributing this
solely to a baked xtask root is outdated: xtask selects its root at runtime, while
Tauri paths, build stamps and other baked paths remain real constraints.

### P5 — Profile the long Vitest files, not the number of cores blindly

Vitest's recorded 34–40 s stages are much shorter than the serial browser stage.
Investigate the monitor/app files and import/environment work after the browser
critical path. `BusMonitorPanel` already uses fake timers; “replace all real
sleeps” is not an established diagnosis. Consider contract-focused file splits,
mount/cleanup profiling and worker/pool comparisons only with measured evidence.

The scan found 13 direct `waitForTimeout` call sites in eight default browser
source files. This is a static call-site count, not accumulated runtime; loops,
parameters and implicit assertion/action waits are not captured by that count.
Do not claim those few calls explain fourteen minutes. Replace waits with explicit
settled-state conditions where equivalent; real motion/paint measurements must
retain real-time behavior rather than being made vacuously green with fake clocks.

Keep isolation by default. Vitest documents that pool/isolation changes have
compatibility and state-sharing consequences, and that its phase percentages sum
tracked parallel work rather than representing wall-time shares [6]. No isolation
or pool setting was changed in this audit.

## 4. GPU assessment

**Do not port ordinary KNX correctness tests to CUDA/WebGPU.** The inspected
suites exercise Rust/TypeScript logic, files, SQLite, parser/crypto behavior,
HTTP/session behavior and UI assertions. There is no identified GPU-compute test
backend whose configuration would make these execute on the RTX automatically.
Introducing one would change the test/implementation under examination.

The plausible experiment is a separate hardware-accelerated Chromium rendering
lane for expensive visual/flow workloads. First capture the exact browser's GPU
feature status and renderer, not merely `nvidia-smi` utilization. Chrome documents
using `chrome://gpu` and shows that a visible adapter or launch flags alone do not
establish actual hardware use [7]. Do not copy unsafe flags or install/change
system drivers as part of this audit.

Compare software/hardware rendering on the same production build, keeping viewport,
DPR, fonts, assertions and capture geometry identical. Retain the portable baseline,
separate performance thresholds, and verify visual results before admission.
Hardware rendering can change output and resource interference; it cannot be
promised to improve this suite without the experiment. Multi-GPU sharding is not
a current project requirement or a demonstrated bottleneck solution.

## 5. Small verification actually executed

- `python3 -m unittest tools.tests.test_run_corpus_tests -v`: **4 passed**, exit zero;
  includes missing-corpus refusal and environment scrubbing with synthetic data.
- `python3 tools/run_corpus_tests.py --list`: **145 cases / 32 targets**, exit zero;
  one observed process wall time of 0.1359 s, not a scaling benchmark.
- Programmatic checks reconcile the retained 32-stage gate, all zero actual exits,
  the timing calculation and the source/selection inventory.
- The new report's links parse with the production documentation parser and all
  eight local targets exist; evidence arithmetic/selection totals reconcile.
  Whole-root documentation verification exits one only for the pre-existing,
  unrelated signing research note's undefined reference `3`; that file remains
  byte-identical to the protection witness. This is not a green whole-root gate.
- No full Rust, browser or private-corpus suite was rerun; no CPU/GPU A/B experiment
  was performed while foreign gates held resources. No product/test configuration,
  dependencies, locks, hardware or installed system software were changed.

## Recommended delivery order

1. Canonical measured runner and unchanged coverage contract.
2. Functional browser lane isolation and 1/2/4-worker equivalence/scaling proof.
3. Exact TypeScript duplicates, then coverage-proven consolidation.
4. Stage-scoped leases and controlled same-host resource budgeting.
5. Optional nextest/corpus build-and-run separation; preserve doctests and isolation.
6. Only then investigate Vitest tails and a separate GPU rendering experiment.

This audit leaves existing feature/release completion records untouched: no
optimization is marked shipped. The report and evidence are local, uncommitted
artifacts until reviewed and integrated separately from the owner's research.

## Primary references

Retrieved 2026-10-14; documentation guidance is not a project-specific benchmark.

[1]: https://playwright.dev/docs/test-parallel
[2]: https://playwright.dev/docs/release-notes
[3]: https://doc.rust-lang.org/cargo/commands/cargo-test.html
[4]: https://nexte.st/docs/design/how-it-works/
[5]: https://nexte.st/docs/running/
[6]: https://vitest.dev/guide/improving-performance.html
[7]: https://developer.chrome.com/blog/supercharge-web-ai-testing

- [1] Playwright parallel workers, file ordering and external-state isolation.
- [2] Playwright release notes: 1.63 test locks versus 1.64 configure convenience.
- [3] Cargo execution model and distinct build jobs / test threads.
- [4] Nextest scheduler model.
- [5] Nextest doctest limitation and explicit selection controls.
- [6] Vitest performance phases and isolation/pool trade-offs.
- [7] Chrome GPU feature/renderer verification; no unsafe launch recipe adopted.

## Subsequent implementation

The audit above records its original inspection boundary. The owner-approved
first implementation package is described in the [measured web test gate](../contracts/web-test-gate.md);
its own evidence remains separate from these historical observations.
