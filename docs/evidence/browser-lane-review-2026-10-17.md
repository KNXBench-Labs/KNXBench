# Browser lane review, 17 October 2026

Package P4 item 1 ([goal file](../goals/open-work-2026-10-17/p4-gate-hygiene.md)).
Procedure: [web test gate contract](../contracts/web-test-gate.md#evidence-and-admission)
(reviewed scope, measured runs, exact case identity per lane, no skips or retries).

## Why the parallel lane was closed

`apps/knx-web/browser-lanes.json` recorded the fixture fingerprint of the
14 October admission (`f256e881`). Since then every input that
`fixture_fingerprint()` hashes, apart from the Playwright/Vite configs, moved,
so `lanes()` sent all 82 spec files to the one-worker exclusive lane. Separately,
23 of the 45 admitted specs were edited and would have dropped to the exclusive
lane on their own even with a matching fingerprint.

## Shared inputs reviewed (since `f256e881`)

| Input | Change | Parallel-safety verdict |
| --- | --- | --- |
| `tools/browser_tmp.py`, `tools/browser_tmp.cjs`, `browserTmp.ts` (`7eda04ff`, `beab0112`) | temp-root selection: symlinked scratch, private leaf reuse | Safe. Playwright workers inherit `KNXBENCH_BROWSER_TMP_DIR`, allocate with `--reuse-owned` (`created=false`) and never remove the parent's leaf; Chromium makes its own unique directories inside it. |
| `e2e/disclosure-fixture.ts` (`d38e734b`, `44fe2723`) | new helpers that open folded navigation, explorer and parameter sections | Safe. Clicks on the calling page only; no shared state, files, ports or clock. |
| `e2e/diff-virtual-fixture.tsx` (`236c64ae`) | adds `functions: empty` to fixture data | Safe. Used only by `diff-virtual.e2e.ts`, which is not admitted. |
| `package-lock.json` (`bbb573f4`, `237bc301`) | app version string `0.1.0-alpha.11` → `alpha.13` | Safe. No dependency changed. |
| `e2e/*.native.ts`, `e2e/*.shots.ts`, `e2e/*.integration.ts` | other configs' entry files | Not loaded by the default config (`testMatch: **/*.e2e.ts`); no effect on either lane. |

## Admitted specs whose bytes changed (23 of 45)

Twenty-one only add calls to the disclosure helpers above (opening folded
sections the 14 October layout showed expanded): `device-download-scope`,
`device-wizard`, `durable-import-evidence`, `ga-type-detail`,
`installation-node`, `installations`, `legacy-install`,
`native-import-diagnostics`, `new-project` (also asserts the folded state
first), `notes-workspace`, `parameter-editors`, `project-password`,
`project-payloads`, `range-ga-create` (one more page-local expand click),
`repair`, `save-overwrite-resume`, `source-import-reports`,
`stored-bus-access`, `structure-heading-hover`, `structure-new-menu`,
`workbench-splitters`. `group-address-drag` also enlarges its own viewport,
and `release-updates` waits for its own manual check to settle before
advancing its page-local fake clock. None adds a fixed port, a file written
outside Playwright's per-test output, an environment switch, a shared
server-side state or a real-time budget.

**Decision:** re-admit exactly the reviewed 45 files with their current hashes
and the current fixture fingerprint. No file is newly admitted; the other 37
stay exclusive.

## Measured runs

`python3 tools/run_web_gate.py --profile benchmark` on the candidate tree
(re-admitted `browser-lanes.json`, all other sources as on the P4 branch after
its item 2–4 commits): `success=True`, `complete=True`, `inputsFrozen=True`
(source fingerprint equal at start and end), default temp-root selection below
a symlinked `scratch` (no explicit root). Compiled inventory 596 cases: 276 in
the 45 functional files, 320 in the 37 exclusive files.

| Run | Workers | Wall (s) | CPU (s) | Cases | Expected | Unexpected / skipped / flaky | Retries |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | 1 | 321.25 | 342.96 | 276 | 276 | 0 / 0 / 0 | 0 |
| 2 | 2 | 185.96 | 391.92 | 276 | 276 | 0 / 0 / 0 | 0 |
| 3 | 4 | 138.72 | 489.45 | 276 | 276 | 0 / 0 / 0 | 0 |
| 4 | 4 | 141.59 | 491.16 | 276 | 276 | 0 / 0 / 0 | 0 |
| 5 | 2 | 215.36 | 429.80 | 276 | 276 | 0 / 0 / 0 | 0 |
| 6 | 1 | 333.81 | 358.79 | 276 | 276 | 0 / 0 / 0 | 0 |

Each run's case identities (file, title path, project) equal the inventory's
functional subset and each other; the runner's own proof check and an
independent recount of the six Playwright JSON reports agree. Means: 327.53 s
(1 worker), 200.66 s (2), 140.16 s (4), about 2.3× for this lane on this host.
Load average at start was 3.4; an unrelated desktop workload ran during part of
the matrix, so the timings are not a clean performance claim, only the case
identities are. The full web profile of the package gate then runs the
functional lane with the default worker count beside the exclusive lane.
