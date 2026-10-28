# 5000-device browser preparation and load sensitivity — 28 October 2026

Status: bounded GATE-01 investigation on `1822a9756`; no application, test,
runner, timeout, coverage or scheduling policy change. The expanded-explorer
case remains in both its development and production selections.

## Trigger and scope

The full S2-03 gate on the superseded feature-only merge `24bdcf19d` failed
in the default `browser-exclusive` stage at
`devices-navigation.e2e.ts:425`, a 120 s case. It changed no web sources.
The replacement merge `d39668393` passed its full gate after concurrent own
Cargo work ended. That does **not** establish why the first attempt failed.
Only the original package/web receipts survive: its detailed Playwright failure
output was mistakenly discarded during task cleanup. Do not reconstruct its
exact wait from later observations.

The subsequent probe used the unchanged case on `1822a9756`. It strictly
intercepts fictional API data, creates a 5000-device project, expands navigation
and the explorer, opens Devices through the command palette, requires 5000
table rows, filters to Actuator 5000, requires that one row and its label, and
requires exactly one `GET /api/devices` and no unexpected API request.

The test name says "production table", but the config decides the renderer:

- `playwright.config.ts` runs the **Vite development server** with
  `vite.fixtures.config.ts`.
- `playwright.devices.config.ts` runs **preview of the built bundle**. Building
  `dist` before using the first config does not turn it into a production run.

These modes are deliberately distinct gate coverage, not duplicates to remove.

## Development-server load reproduction

One browser worker, no retries, repeat-three, same original case and 120 s
limit; each run was in a separate user/network namespace with only loopback.
The second series added 16 shell busy loops, which were reaped after the series.
This is synthetic CPU contention, **not** a replay of Cargo or a device load.

| Series | Outcomes | Per-case durations | Attached render / filter |
|---|---|---|---|
| No synthetic load | 3 passed | 73.718, 73.099, 72.567 s | 16.173–16.382 s / 1.117–1.130 s |
| Synthetic CPU load | 3 timed out | 125.709, 125.703, 125.700 s including timeout teardown | none: the table phase was never reached |

The configured limit remained 120 s, not 125 s. All three failures were inside
`openList`: one at `keyboard.press("Control+Shift+P")`, two waiting to fill the
palette combobox. Their traces also retain slow preparation: the explorer
Expand-all click takes 44.4–46.7 s including the work awaited after the click.
The Done click (including locator wait) takes 8.4–44.1 s; that duration is not
pure click-handler CPU time. No JavaScript/layout/paint profile was collected.

The first series did not acquire the common-directory gate leases. The load
averages and synthetic inputs were retained, but absence of foreign contention
was not established. This reproduces load sensitivity in this harness, not the
cause of the historical S2-03 failure or a portable performance bound.

## Build-mode and disclosure counterfactual

A follow-up held the three shared gate leases in the normal order for all four
runs, with no own CPU load. The production bundle was built first. One sample
per cell, same machine and dependency set, separate server/browser startup,
trace enabled, one worker, zero retries. The only test-source variant was the
existing `openProject(page, "en", false)` option, leaving navigation/explorer
closed; all 5000-device table, search and API assertions stayed unchanged.
The original test bytes were restored and checked against the retained source.

| Build / explorer state | Case duration | Whole command wall time | Table render attachment | Filter attachment |
|---|---:|---:|---:|---:|
| Development / expanded (original) | 74.257 s | 76.44 s | 16.193 s | 1.108 s |
| Development / closed (counterfactual) | 37.136 s | 39.33 s | 12.205 s | 0.851 s |
| Production / expanded (original) | 42.018 s | 43.69 s | 10.394 s | 1.075 s |
| Production / closed (counterfactual) | 20.745 s | 22.35 s | 9.000 s | 0.897 s |

All four passed. These are single-sample observations, not accepted speedups.
Render means the existing test's wall interval from opening the command palette
to seeing 5000 rows, not isolated React/rendering CPU time.

In the original-state follow-ups the Expand-all click occupies 24.5 s in dev
and 11.7 s in production. Omitting expansion changes both preparation and the
DOM state during subsequent table interaction. Its whole-case delta cannot be
attributed only to that click. The closed variant does not retain the original
expanded-explorer stress coverage and is **not** a drop-in gate replacement.
The published 9742/1264 ms production numbers in the
[device-navigation dossier](../limitations/current-project-device-navigation.md)
remain dated observations; comparing them with the dev series would be invalid.
No production-under-CPU-load series was run.

## Disposition

GATE-01 remains open. The case spends substantial time outside its table-render
attachment and can exhaust the shared 120 s budget under contention. This is
not a proven new production regression or an explanation of older cold-Vite,
SIGTERM or file-menu failures. Do not raise the timeout, remove expansion,
change lane membership or increase concurrency to hide this finding.

A future bounded runtime investigation should profile project creation and
Expand all in the production build (script vs. layout/paint), then retain the
expanded-state assertions in any before/after comparison. A deliberately
separate table-only benchmark would need to declare its different state and
leave the expanded case covered. No native WebKitGTK, real bus, backend
throughput or hardware evidence follows from these intercepted browser cases.

## Provenance and replay

Locked tools: Playwright 1.63.0, Vite 8.3.5, React 19.2.8; Node v26.8.1,
Chromium 155.0.8059.84. Original spec SHA-256:
`9673428b679e8e9d1c509e4e97a46ebf1f92047597bed6823bf56009988fd79e`.
Private evidence: `<profile>/evidence/gate01-devices-load-20261028/` contains
raw JSON reports, failure/success traces, the probe and counterfactual driver,
source binding, aggregation and runtime identity. Reports distinguish all six
series; counters are read from JSON, not inferred from wrapper exits.

For a faithful replay, install locked dependencies and build in an owned
worktree at the recorded revision. Acquire `knx-workspace-gates.lock`,
`knxbench-alpha-gate.lock`, then `knxbench-browser-fixture.lock` in the Git common
directory for the entire experiment, including any synthetic load. Run the
following in an isolated loopback-only namespace, from `apps/knx-web`:

```sh
node node_modules/@playwright/test/cli.js test \
  --config=playwright.config.ts --workers=1 --retries=0 --repeat-each=3 \
  --trace=on --reporter=json --output=<evidence>/artifacts \
  e2e/devices-navigation.e2e.ts:425 > <evidence>/report.json
```

Use `playwright.devices.config.ts` for the distinct production series. Keep
stdout, stderr, traces, build/source identity, load conditions and command exits
before cleanup; a receipt alone cannot recover a discarded failing wait.
