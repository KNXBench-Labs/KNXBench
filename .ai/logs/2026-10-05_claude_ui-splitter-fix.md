# Left-column splitters with a project loaded (goal-ui owner, 2026-10-05)

User report: with a project open, the separators in the left column (navigation
block / Project Explorer / diagnostics block) did not resize anything.

## Cause

`.workbench-pane-left .project-explorer` had `flex: 1 1 auto`. With a real tree
the explorer's whole content height entered the column's flex calculation, and
the column shrank both blocks — including any height a splitter set — to make
room. With 40 installations the navigation block was ~37 px before anyone
touched it; a drag changed `aria-valuenow` but not the rendered height. The
happy-dom unit tests (`PaneSplitter.test.tsx`) cannot see layout.

## Change (CSS only)

`flex: 1 1 0` for the explorer (it takes only what the blocks leave) and a 72 px
minimum, the splitters' own `STACK_BLOCK_MIN_PX`, so two blocks dragged to their
maximum cannot squeeze it out of sight.

## Evidence

- New `apps/knx-web/e2e/workbench-splitters.e2e.ts` (full app, `page.route`
  intercept, 40 installations, no unexpected API call): drag down/up and the
  keyboard change the rendered height by exactly the moved distance;
  `aria-valuenow` matches the drawn height; both blocks at maximum leave the
  explorer ≥ 72 px and the diagnostics block inside the column.
- RED on the previous CSS: both drag tests (117 expected / 35 received; 97 / 35).
  The maximum case was RED with a 0 px explorer minimum (29 px), GREEN at 72 px.
- Gate (leases 7/8/9, inputs frozen): build, flow-study, theme-fixtures, Vitest
  2,016 / 116, splitter spec ×5 15/15, check-anchors 539, check-ledger 186,
  check-headers 532 / 157, diff-check — all 0. Chromium first run 134 passed,
  1 failed: `telegram-flow-hub.e2e.ts` hit its 30 s test timeout under full-suite
  load (3/3 in isolation). Full Chromium rerun under the leases: 136 passed, rc 0
  — that count includes one read-only scratch probe (settings screenshot for the
  next package) that was in the e2e folder when the rerun started; it was
  removed before this commit.

No KNX/bus contact; intercepted traffic only.
