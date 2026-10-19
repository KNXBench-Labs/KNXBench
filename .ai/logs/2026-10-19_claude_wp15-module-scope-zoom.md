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
