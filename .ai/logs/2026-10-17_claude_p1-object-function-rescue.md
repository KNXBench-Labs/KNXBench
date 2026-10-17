# 2026-10-17 — Claude — P1: object-local function choices rescued and published

Package P1 from `docs/goals/open-work-2026-10-17/p1-object-function-rescue.md`,
assigned explicitly by the owner after P4 was published. Claim `bbb3879a`,
worktree `KNXBench-object-function-rescue-20261017`, branch
`fix/object-function-rescue-20261017` off `541b1f48`.

## What happened

- **Cherry-pick `55bcb203`** of Codex's `99dd6e65` (from
  `refs/kl185-rewritten/heads/fix/object-function-dropdown-20261017`), with its
  log, contract and user-guide text. Nine files conflicted, all "both sides
  added next to each other": `FieldLayout` keeps the grid `cell` (ADR-0137) and
  gains `controlled_object_refs`; the old `controls` set became the feature's
  `targets` map (reference → object references, propagated along choice
  dependencies) while the separator declarations and grid placement of
  ADR-0139/0137 stay; the DTO and `api.ts` carry `cell` and
  `controlledComObjectIds`; the layout test file keeps both tests.
- **Devices spec** (`devices-navigation.e2e.ts`): the case now asserts the
  area register is closed on the first untouched visit (`aria-current` count 0,
  `.parameter-workspace-closed` visible, editor hidden), opens the one area with
  `openParameterSections`, sets the mode, changes it again below the object's
  DPT and finds that write back in the still-open area. The screenshot scrolls
  to the control and adds an editor-only capture.
- **Commit `4b961475`**: new HTTP test
  `module_objects_and_module_fields_get_no_inferred_object_controls` (a module
  instance's object and a module-scoped field whose references spell a
  top-level declaration stay unmapped), block-`Access` cases in
  `ComObjectFunctionControls.test.tsx`, contract text on grids/separators/block
  access, FEATURES row moved into "Newer source changes".

## Evidence

- Mutants (log in agent scratch): M1 no dependency propagation, M2 module
  objects mapped, M3 module sections mapped, M4 no restriction filter, M5
  field-only access check, M6 controls not passed to the object editor — all
  killed. M2 and M3 **survived** before the new HTTP test; that test was
  written for them and kills both by name.
- Package gate on `4b961475` (16 stages, 0 failures, inputs frozen, symlinked
  `TMPDIR`): web 64/64 stages (Vitest 3,732; browser 596 = 276 parallel + 320
  exclusive), fmt, Clippy `-D warnings` with freshly checked knx-desktop,
  workspace tests 4,857 / 0 / 212 ignored in 364 binaries, bindings unchanged,
  xtask layering/headers/anchors/ledger/corpus-gates, documentation, tools 220,
  private corpus 153/153 in 33 targets, diff-check.
- Screenshots EN/DE × 1440/400 inspected: control below DPT, within width, no
  overlap (the first capture showed only the page top; fixed by scrolling).

## Retired

Worktrees `KNXBench-object-function-20261017` (`cd346ce3`) and
`KNXBench-object-function-integration-20261017` (`c13e54f7`), both branches and
both `refs/kl185-rewritten/heads/*object-function-dropdown*` refs. The earlier
integration branch's resolutions were compared first: the same feature lines in
`Inspector.tsx`, `ParameterPanel.tsx` and `styles.css`, nothing lost.

## Left as is (not in scope)

- `ParameterView = "functions"` and the messages `parameters.functions*` are no
  longer rendered by the app (only `ParameterPanel.test.tsx` uses the view).
  Removing them is cleanup, not rescue.
- Module-local object mapping stays explicitly unavailable.
