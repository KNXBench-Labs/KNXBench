# 2026-10-04 — U20 part 2: the Flow view in the bus monitor

Agent: Claude, goal-ui.md owner session; Web lock `5dcd429e` (released in this delivery).

- Wiring: `useFlowFeed` in `BusMonitorPanel` gets every admitted batch (poll and
  reattach backlog). It resets on Connect and when another session answers, and
  keeps running while the table tab is shown. Table filters are shown only in the
  table view. Tabs follow the tablist pattern with arrow keys.
- Order honesty: the view component was written before its tests. RED was shown
  against a stub (7/7 failed) and 11 view mutants, all caught.
- Test-harness findings (not product bugs):
  - Panels left mounted by earlier tests in `BusMonitorPanel.test.tsx` poll the
    shared mock, so one-shot answers were consumed by them. The flow tests now
    answer by argument (status and `since`).
  - Absolute poll and fetch counts are not valid there. The poll proof compares
    two equal windows (table vs flow); fetch-once is proven in `flowFeed.test.tsx`.
  - A mutant baseline with a red test makes counts worthless. It was re-run on a
    green baseline: two survivors (reattach, new empty session) led to two tests.
- Guards updated with justification: the companion import inventory (+4 flow
  modules, each proven API-free; +`fetchFlowSnapshot`, a read) and
  `diagnosticShell` class ownership (tab/panel ids renamed out of the class
  namespace instead of adding exceptions).
- Visual check: an unthemed fixture showed no lines, so a themed fixture was
  added. Arrowheads were hidden under text (fixed with a text clearance gap and
  a halo), and the flag table was too wide (now one narrow table per object).
  The e2e asserts the theme stroke before and after a live theme switch.
- In-session review: IMPORTANT, count texts read "1 telegrams" (made
  count-neutral); MINOR, `aria-controls` on the inactive tab pointed at nothing
  (now only on the selected tab); MINOR, whole-panel re-render per batch/expiry
  (documented for U21 measurement).
- Final sweeps on the reviewed code: reducer 23/23, view 11/11, panel 7/7, feed
  1/1, all restored byte-exact. Old panel vs new e2e: 6/6 failed.
- Full gate (first attempt): web build, fmt, clippy -D warnings, workspace tests 3,196 passed / 0 failed / 177 ignored in 175 blocks with 0 skip markers, five repository gates (headers 510 ok; anchors 453 ok; ledger 186 rows), tsc, `check:flow-study`, Vitest 1,963/113 files, complete Chromium suite 125/125, whitespace; source frozen.
