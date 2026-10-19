# WP-15 module scope identity and two zoom follow-ups (Claude, 2026-10-19)

## Request

Alpha 15 package WP-15: adopt ADR-0063 `nodeChain` in the web consumers, then
the two UISET zoom follow-ups (pane drags under application zoom; one table
case driven through the real `uiScale` setting). Web only, separate commits.

## Part A: module scope identity

- Found: `parameterPresentation.ts` grouped diagnostics and
  `parameterWorkspace.ts` keyed sections by `moduleNode`/`moduleId`/
  `moduleDefId` only. The workspace separated equal legacy scopes by
  occurrence, so a sibling that vanished after a write handed its position
  (chosen area, row key, draft editor) to the survivor. The Restricted and
  Functions section views were keyed by list index. `sameScope()` named in
  KL-003 no longer exists. The parameter route has no strict client admission
  (plain `request()` cast), so the optional field needs no admission change.
- Decision: one rule in `moduleScope.ts`. A chain counts only with the
  published shape (nonempty safe integers ending in `moduleNode`); otherwise a
  tagged legacy identity that never equals a chained one. Malformed is treated
  as absent, never a refusal.
- RED: 5 unit cases (presentation 3, workspace 2) and 2 component cases
  (`ParameterPanel.test.tsx`) failed at their semantic assertions before the
  fix; all pass after.
- Mutants (each restored byte-exact, sha256 checked): chain ignored, clipped
  to two nodes, sorted, untagged fallback, shape check removed, index keys —
  every one fails a named case.
- Full Vitest under load: two unrelated heavy tests (LogPanel 16000 entries,
  ResizableTable large table) timed out at 5 s with load average about 20;
  both files pass alone (88/88).

## Part B: zoom follow-ups

- `ResizablePane` and `PaneSplitter` divide the pointer delta (and the
  splitter's DOMRect height) by `appUiScale()`; the pane width is rounded.
- RED: 6 Vitest cases (Workbench 4, PaneSplitter 2) and the browser cases
  at 0.8/1.5 failed (navigation edge off by 12 and 30 visual px); 100% passed.
  GREEN: 22/22 Vitest, 6/6 splitter browser cases.
- Setting-driven table case passed 1/1 in a focused run.
- Owner pause stopped work: no full Vitest, typecheck, mutation proofs, web
  gate or rerun of the timeout classification for this part.

## Resume after the rebase onto the test-speed main

- Focused Vitest on the rebased tree (Part A and B files): 124/124.
- All 22 `tsconfig*.json` typecheck clean; `workbench-splitters.e2e.ts` is in
  no committed config, so a scratch config typechecked it (a planted error
  proved the config sees the spec).
- Four Part B mutants, each restored byte-exact: pane delta, splitter delta,
  splitter observed height and interaction-time height left unscaled; each
  fails the named scale cases. Pane-width rounding is not discriminated.
- `tools/browser_lanes.py`: no untagged real-time use; the new cases poll
  for state, assert no elapsed time or frames, so no `@exclusive`.
- Wip commit reworded into a normal commit; package gate run on the commit.
