# P1 — Object-local function choices: rescue and publish

Planning only; part of the [17 October package set](README.md). Follow its
shared rules.

**Starter:** `/goal Run package P1 from docs/goals/open-work-2026-10-17/p1-object-function-rescue.md on fresh origin/main; follow README.md's rules.`

## Entry conditions (both required)

1. The **declared parameter grids** package is on `main` (look for its
   handover entry and ADR "declared-parameter-grids" under `docs/adr/`). Until
   then both packages edit `ParameterPanel.tsx`,
   `crates/knx-productdb/src/parameter_layout.rs` and the Devices browser spec.
2. **Ownership is handed over.** The feature belongs to a Codex session. Start
   only after a handover entry by that session or an explicit user assignment
   ("P1 übernehmen"). Without it, stop and report.

## What exists

- Feature commit on the rewritten history: `99dd6e65` "Put function choices
  where the object lives" (`refs/kl185-rewritten/heads/fix/object-function-dropdown-20261017`),
  parent `1ee19383`, 25 files, about 420 lines. It contains its own contract
  update (`docs/contracts/parameter-object-refresh.md`), user-guide text and the
  Codex log `.ai/logs/2026-10-17_codex_object-function-dropdown.md`.
- Three later merges on `refs/kl185-rewritten/heads/integrate/object-function-dropdown-20261017`
  (`a2990adc`): useful as a record of earlier conflict resolutions, not as a
  base.
- Worktrees `KNXBench-object-function-20261017` (`cd346ce3`) and
  `KNXBench-object-function-integration-20261017` (`c13e54f7`) are on the
  **old** history. Never push or merge from them.
- Last gate (`gate5`) was interrupted by the disk move at 12:55 with zero
  stages; earlier gates on older merges were green. No result is valid for the
  new base.

## Scope

1. New worktree off fresh `origin/main`; cherry-pick `99dd6e65` (resolve, do not
   squash away its log or contract changes).
2. Adapt the Devices spec (`apps/knx-web/e2e/devices-navigation.e2e.ts`) to the
   area register of the parameter workspace: `.parameter-workspace-nav button`,
   `aria-current`, `.parameter-workspace-closed`, and the shared helper
   `openParameterSections` (opens one area, throws on several). Do **not**
   weaken the assertion that the Parameters tab starts closed on the first
   untouched visit.
3. Integrate with the parameter grids: the object-local function controls sit
   below the DPT field and reuse the shared bound parameter rows; a grid must
   not duplicate or hide them. Missing, ambiguous or module-local mappings stay
   explicit, never guessed.
4. Full gate (owning Rust: `knx-productdb`, `knx-server`; workspace Clippy; web
   profile with all 63 stages; EN/DE wide/narrow screenshots of the Devices
   view inspected).
5. Publish, read back, then retire **both** old worktrees, their branches and
   the rewritten refs of those branches (the integration worktree's build
   directory alone was about 74 GB). Archive nothing new: the cherry-pick on
   `main` is the record.

## Not in scope

New mapping rules, module-instance construction, any change to the parameter
grids' own behaviour.

## Done when

`99dd6e65`'s behaviour is on `main` under a green full gate, the Devices spec
asserts the area register, both old worktrees are gone, and `OPEN_WORK.md` no
longer lists the package as unpublished.
