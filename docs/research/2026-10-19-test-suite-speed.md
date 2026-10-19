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
of them the File-menu hover specs (52). BROWSER_RESULT

## Limits of these numbers

One host, one run per configuration, background load from other work. The
ratios are large enough to survive that noise; the seconds are not a promise
for other machines. The change-scoped plan is checked against its tests and
against history, not against a missed defect; releases still run everything.
