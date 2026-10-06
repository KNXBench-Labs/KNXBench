# UI closure residue: UI-04 Web half and KL-61 binding wording (goal-ui owner, 2026-10-06)

Found by the closing review for the UI closure receipt (`RELEASE-03` / AR16).
Worktree `ui-closure`; web lock taken in `03d8a7fb`, released by this commit.
Rebased over Alpha's AR16 slice 1 and the commissioning D5 docs (base `1f4aca11`).

## UI-04 (COMMISSIONING_ALPHA_LEDGER "Handoff to the UI owner")

- `liveActivity.ts`: strict admission of `GET /api/bus/activity` — every field,
  session kind/state, busy lock, untracked kind, counts and bounds; refuses the
  whole snapshot; one-shot records go through the history admission on a
  synthetic one-entry page so both readers share one rule.
- `BusActivityLive.tsx`: new *Live activity* tab; polls every 2 s only while
  the page is visible, stops on tab change; sessions with state/progress,
  one-shot records, eviction count, busy locks ("operation unknown"),
  history-storage state (`unavailable` as alert), untracked kinds, restart note
  on a new `serverIncarnation`; a failed poll clears the last snapshot and
  shows no server text. Partial/volatile wording throughout (ADR-0055).
- `BusActivityHistory.tsx`: first window reloads every 3 s while it shows a
  running row; after paging, a note points to "Refresh from beginning".
- No global status bar (owner decision, KNOWN_LIMITATIONS).
- `DiagnosticsCompanion.test.tsx` guard: `liveActivity.ts` added to the
  companion's import inventory with its own no-API/no-fetch assertion
  (negative control: a planted `fetch(` turns it red; restored byte-exact).

## KL-61

`GroupAddressNode.dpts` doc comment reworded to the effective type (ADR-0078
hand-over) and regenerated; only comment lines change in the binding
(generator whitespace churn in four other bindings restored). The
declared-versus-linked display needs a projection field (Alpha).

## Evidence

- RED first: live tests (module absent) and two history cases; the "no reload
  without running rows" case is green by construction.
- Mutants 16/16 (first sweep 15/16: `octets-unbounded` survived → three bound
  cases added, rerun killed).
- Chromium `activity-live.e2e.ts` 5 + `activity-history.e2e.ts` 8 = 13 passed;
  negative control against the previous `BusDiagnosticsPanel`: 5 failed,
  restored (sha256 OK).
- Closing gate attempt 1 (head `03d8a7fb` + candidate): Rust 188 suites
  3,311 / 0 / 177 ignored, corpus linked; bindings regenerate identically
  (`diff -r -Z`, 17 files); five xtask checks 0; tsc 0; **Vitest 1 failed**
  (the companion import-inventory guard above) → fixed; Chromium 0; refused
  (inputs changed during the run).
- Attempt 2 (same head, fixed candidate, inputs frozen): fmt 0, clippy 0
  (incremental in this scratch target: 350 units at `4459e310`, 6 after the
  knx-projection comment, 1 here), Rust 188 / 3,311 / 0 / 177, bindings
  identical, layering/headers 155/155/anchors 603/ledger 190/corpus-gates 0,
  diff-check 0, `tsc --noEmit` 0, `tsc -b` 0, Vitest 2,064 / 117, Chromium 139.
- After the rebase (upstream: docs, Python tools and two manual-screenshot
  tooling files outside `src/` and outside the suites; no Rust): headers
  155/155, anchors 606, ledger 190, diff-check 0, `tsc -b` 0, Vitest
  2,064 / 117. Rust and Chromium results carry over.

No hardware, no bus, no KNX socket.
