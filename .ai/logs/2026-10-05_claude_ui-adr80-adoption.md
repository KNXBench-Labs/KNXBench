# ADR-0080 adoption in the parameter panel (goal-ui owner, 2026-10-05)

Worktree `ui-adr80`, rebased onto `origin/main` `2f2a6892` (KL-142) with a
temporary WIP commit (not `git stash`: the stash list is shared by every
session's worktree). Web lock taken in `435b25ed`, released by this commit.

## Asked by Alpha (AR07 handover)

Adopt `parameterAccessReadOnly`, `manufacturerCalculation`,
`writeAuthorityUnavailable` and decide how `access: "None"` fields are shown.

## Change

- `api.ts` `ParameterDiagnosticKind` gains the three tokens plus ADR-0061's
  `unsupportedControlKind` and ADR-0062's `evaluationWorkBudgetExhausted`
  (both had only the server's English fallback); en/de catalogue entries,
  wording following the server's English messages.
- Presentation decision (recorded in ADR-0080 "UI presentation"): fields whose
  effective access is `None` are folded per section behind a button with their
  count and `aria-expanded`; shown read-only on request; never dropped. `Read`
  fields stay visible.
- Follow-on fix found while testing: every disabled field said "Shared across
  every instantiation of this module"; since ADR-0080 device-level fields are
  refused too, they now read "Not editable here — see the warnings for why."
- Docs: ADR-0080 section, KNOWN_LIMITATIONS (Access bullet; ADR-0061 token
  adopted), PARAMETER_SEMANTICS_BOUNDARY, IMPLEMENTATION_STATUS.

## Evidence

- RED first: 7 of 8 new cases failed on the old panel (the "no fold without
  None fields" case is green by construction); the caption case RED → GREEN.
- Mutants 8/8: none-not-folded, read-folded-too, toggle-dead, plural-swapped,
  access-kind-untranslated, authority-kind-untranslated, no-aria-expanded,
  no-toggle.
- First gate on the pre-KL-142 base was stopped and rerun after the rebase
  (shared files `api.ts`, catalogues). Gate (leases 7/8/9, inputs frozen):
  build, flow-study, theme-fixtures 0; Vitest 2,024 / 116; Chromium 134;
  anchors 549; ledger 188; diff-check clean; check-headers 1 only for the base's
  161 vs 157 (since `9b232142`, see the KL-142 log) — this package adds none.

No server change. No hardware, no real bus, no KNX socket.
