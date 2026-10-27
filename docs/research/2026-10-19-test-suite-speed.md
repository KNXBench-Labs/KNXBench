# Test-suite speed: measured causes and changes (19 October 2026)

Owner request: "far too many tests run, which slows development down; analyse
the construct and make it faster." Decisions and their implementation:
[ADR-0144](../adr/0144-change-scoped-gates-tagged-browser-lanes-bundled-rust-tests.md).
The earlier [structure audit](2026-10-14-test-suite-performance-audit.md) is
background; this dossier records new measurements only.

## Conditions

- Host: the dev machine (8 cores / 16 threads, 62 GiB). A desktop VM and other
  sessions' work ran at times; every measurement below waited for the shared
  `knx-workspace-gates.lock`, so no two gates or measurements overlapped, but
  host load is not zero. Load averages are kept in the raw JSON.
- Rust inputs: commit `9a653986` (identical Rust/web inputs to `652dba4d`,
  which differs by two documentation files). Each experiment used its own
  worktree and a fresh, private `CARGO_TARGET_DIR`.
- Harness: a scratch script ran, in order, a cold `cargo test --workspace
  --no-run --offline --locked`, a no-op rebuild, a rebuild after appending one
  constant to `crates/knx-core/src/lib.rs` (and after restoring it), the same
  for `apps/knx-server/src/lib.rs`, then `cargo test --workspace --no-fail-fast`
  on the built target and `cargo test --workspace --doc`. Wall times are
  monotonic; per-binary times are libtest's own `finished in` values.

## Where a package gate spent its time

Two complete package gates ran on the morning of 19 October 2026 (other
sessions' packages, receipts read, nothing re-run):

| Gate | Interval | Note |
| --- | --- | --- |
| UI feedback delivery | 02:54:34 – 04:55:44, **2 h 01 min** | web gate until the workspace lease at 04:28:17 (lease wait included), Rust/corpus 27 min |
| issues 25/26 | web gate 04:56:59 – 05:26:39 (**29.7 min**), Rust tests until 05:37:31, corpus after 05:38 | stage receipt below |

Web stages of the second gate (`receipt.json`, 66 stages):

| Stage | Wall (s) |
| --- | ---: |
| `browser-exclusive` (713 cases, **one worker**, no functional stage) | 1,172.9 |
| 15 production suites, serial, inventories included | 505.6 |
| Vitest | 37.0 |
| tool tests | 26.9 |
| 20 derived typechecks | ≈ 15 |
| all web stages | 1,780.2 |

The parallel lane admitted files by content hash and closed when a shared
fixture changed. On 19 October 21 of its 45 admitted specs had changed and the
fixture fingerprint no longer matched (`2dff73b7…` recorded, `1ffb15f7…`
current), so every file ran serially. Summed per-file durations of that stage:
311.0 s in the 45 formerly admitted files, 853.5 s in the rest; the largest were
`devices-navigation` 106.1 s, `group-address-views` 76.8 s,
`communication-objects` 73.6 s and `monitor-dpt-context` 49.2 s. Most of those
were serial only because nobody had reviewed them, not because they measure
time.

## Rust: baseline and experiments

| Measurement | Baseline | E1: line tables, optimised dependencies | E2: E1 + one test binary per crate |
| --- | ---: | ---: | ---: |
| test binaries | 356 | 356 | 37 |
| cold `--no-run` build | 206.5 s | 156.4 s | 118.0 s |
| target directory after the cold build | 71.6 GB | 26.8 GB | 6.7 GB |
| no-op rebuild | 147.0 s ¹ | 4.7 s | 2.6 s |
| rebuild after a `knx-core` change | 194.6 s | 77.5 s | 10.0 s |
| rebuild after a `knx-server` change | 123.4 s | 43.9 s | 5.3 s |
| test run (built target) | 467.2 s | 185.7 s | 145.0 s |
| sum of per-binary `finished in` | 460.7 s | 181.8 s | 142.9 s |
| passed / failed / ignored | 4,978 / 0 / 215 | 4,978 / 0 / 215 | 4,977 / 1 / 215 ² |

¹ Not a pure no-op: the claim commit of this package landed while the
baseline ran. `knx-build-stamp` watches `HEAD` and its reflog in development
builds, so the commit relinked `knx-cli`, `knx-mcp`, `knx-server` and every
test binary depending on them. That is what every commit costs a developer
who tests right after committing; E1/E2 ran without a commit in between.

² The one failure was a one-second hang guard in
`http_service_control::cancellation_after_the_backup_retains_unknown_possible_write_evidence`:
the test waits until the request reaches a blocked bus write, and in the
bundle the crate's other 685 tests compete for the same cores. The wait
returns as soon as the point is reached, so it is now ten seconds (three
guards in that file, one in `http_serial_address`); an earlier run of the
same bundle passed all 4,978. See the stability check below.

E2 is the candidate itself (the bundled layout plus the profile, after fixing
two bundling errors: a helper module that reached its includer through
`super::`, and an `assert_eq!` whose `.as_ref()` became ambiguous once all
test modules' trait imports met in one crate).

Observations:

- The baseline run phase is strictly serial: the per-binary times add up to
  the wall time, with the load average around 3 on 16 threads. 187 of the
  356 binaries hold five tests or fewer.
- The slowest binaries were CPU-bound work in unoptimised dependencies:
  `http_large_project_history` (one test) took 113.2 s in the baseline and
  11.9 s in E1; `cli_package_raw_admission` (four tests) 41.1 s.
- Full debug information made each test binary about 164 MB; line tables
  bring the 356 binaries from 58.4 GB to 19.8 GB.
- Bundling is what removes the relink cost: after a `knx-core` change E1
  still relinked 356 binaries (77.5 s); E2 relinks 37 (10.0 s). The test
  binaries shrink to 1.3 GB, the target directory after the whole
  measurement from 75.4 GB to 8.3 GB.
- The run phase is still mostly serial between binaries, but each bundle
  runs its tests on all cores; the longest bundle (`knx-server`, 686 tests)
  takes about 40 s of the 145 s.
- The doc-test phase fell from 20.5 s to 2.0 s; the baseline value includes
  the relink after the claim commit and is not a fair comparison.

## Stability of the bundled `knx-server` binary

After the hang-guard change the `knx-server` bundle ran five times in a row,
each time with `cargo test -p knx-app -p knx-store -p knx-productdb` started
alongside (load average 18–20 on 16 threads): 5 × 686 passed, 0 failed,
37.0–39.8 s each.

## What the change-scoped plan would have selected

`tools/gate_scope.py` applied to the last 40 first-parent commits on `main`
(17–19 October, `git diff <commit>^1 <commit>`):

- none needed the complete gate;
- 22 changed documentation or handover files only: static checks only;
- 16 changed Rust code (the UP258 stage-0 series, diagnostic-evidence merges):
  1 to 16 Rust packages and the corpus tests of those packages, **no browser
  case**;
- the UI-wave integration (123 files): all 88 default spec files (shared
  styles and fixtures), **no Rust package**;
- the UI-set delivery (76 files, including the web lockfile): every spec,
  every production suite, one Rust package.

## Browser lanes after the tag change

The default inventory on the candidate holds 842 cases: 740 untagged
(functional lane, four workers) and 102 tagged `@exclusive` in 17 files, most
of them the File-menu hover specs (52).

The first complete gate on the candidate ran the functional lane (744 cases
after the rebase, four workers) in 384.5 s, against 1,172.9 s for the whole
default inventory on one worker before. It failed one case, which had passed
serially every time: `explorer-structure` "a pulse in a long tree touches only
the rows on screen". Two races in the spec, not in the app: Playwright's
installed clock keeps flowing while the project opens, so a fixed
`runFor(12_000)` window contains the start of the twelve-second pulse only
when opening was quick; and the case measured the marked rows before
expand-all had rendered them. Eight competing workers reproduced the phase
race in 40 of 40 runs. The specs now step the clock until a pulse starts and
wait for the tree first: 90 of 90 passed under the same load. Parallel
execution is doing its job here: it shows the load-dependence that a quiet
one-worker run hides.

## The complete gate on the candidate

`python3 tools/run_package_gate.py --full` on the rebased candidate passed in
2,386.5 s (39.8 min, no lease wait): web gate 1,727.3 s, clippy 81.0 s, Rust
tests 174.5 s including the build (4,993 passed, 0 failed, 215 ignored, 54
test binaries with unit tests), bindings 5.8 s, corpus 392.6 s (155 of 155),
website and story tests 3.2 s. For comparison, the issues 25/26 gate ran its
web part for 29.7 min and its Rust tests for another 10.9 min before the
corpus stage.

The web gate itself shrank only from 1,780.2 s to 1,727.3 s, and the reasons
are worth keeping:

- Five workers of another session built and tested without the workspace
  lease while the functional lane ran (load average up to 25): its 744 cases
  needed 1,823 s of summed case time (the whole default inventory of 713
  cases had needed 1,172.9 s on one worker before) and still finished in
  495.9 s. An unloaded run will be faster; it was not measured here.
- The exclusive lane took 513.5 s for 102 cases. `file-menu-submenus` alone
  took 206 s; most of it were application-zoom hit tests that measure no time
  at all. They waited 300 ms for a date-dependent startup toast and were
  tagged through that helper. They now pin a weekday noon with
  `page.clock.setFixedTime` (no holiday, not late at night, so no startup
  toast; timers keep running) and lost the tag: 37 of the file's 52
  exclusive cases now run in the functional lane, 15 hover-intent cases stay
  exclusive. Stress check with eight workers, three repetitions: all moved
  cases passed in both runs (138 of 138 in the focused second run). The first
  run, over the whole file's untagged cases, also lost three cases of
  "File menu holds entries only", which this change does not touch: each
  timed out in its first 30 s waiting for the File button, all within the first
  75 of 327 starts, i.e. while eight workers loaded the app from a cold Vite
  development server at once. The gate uses four workers; the cases passed
  there and in the focused rerun. Recorded here, not fixed.
- The production suites took 596.8 s against 505.6 s, because `main` gained a
  16th suite (`table-columns`, 123.7 s with its exclusive part) in the
  meantime. They still run one after another.

### After two review rounds (`3fc73c77`)

Two independent read-only reviews of the branch found gaps in the planner
(deleted files, web unit tests reading Rust and docs, unknown web paths, the
missing `table-columns` suite, an `xtask` corpus selection that always failed)
and in the lane guard (fixtures, methods, several import shapes, helper
findings dropped silently). All were fixed test-first; production suites went
back to one worker each, as their configs pin, because four workers had
overridden that without a lane review.

The complete gate on the fixed candidate passed: 2,244.1 s of stages (37.4
min) plus 143.5 s waiting for the browser leases of another session's gate.
Web gate 1,770.8 s: functional lane 781 cases on four workers in 453.9 s,
exclusive lane 65 cases in 277.5 s (513.5 s and 102 cases before the
file-menu clock change), production suites 771.7 s on one worker each
(`table-columns` alone 231.3 s against 123.7 s when split by tag). Rust tests
124.0 s with a warm target (4,993 passed, 0 failed, 215 ignored), corpus
335.8 s (155 of 155).

A routine package does not pay for any of this unless it touches the web
app's shared inputs; that is where most of the time goes back to development.
A change to `tools/browser_lanes.py` alone, for example, ran the
change-scoped gate with every default spec but no Rust, corpus or production
stage: 11 stages, all 846 default cases (781 functional, 65 exclusive), 790.4 s
of work plus 1,324.3 s waiting for another session's browser leases.

## Cold-start concurrency qualification — 27 October 2026

The earlier eight-worker cold-start explanation above is an inference, not
measured concurrency or an established root cause. The raw three failure
identities/stacks and exact failed worktree were not recovered. On current
source with the same locked Playwright 1.63.0, the whole-file untagged
repeat-three selection compiles to 327 entries in only three ordered groups.
The proposed focused four-cap/eight-cap comparison likewise has three groups
in either arm, so it is not a four-way/eight-way load experiment. This is a
compiled-inventory/source-function result, not a rerun of the old failures;
see the [bounded assessment](2026-10-27-cold-start-worker-shape.md). The old
reported passes, failures and durations remain historical observations.

## Limits of these numbers

One host, one run per configuration, background load from other work. The
ratios are large enough to survive that noise; the seconds are not a promise
for other machines. The change-scoped plan is checked against its tests and
against history, not against a missed defect; releases still run everything.
