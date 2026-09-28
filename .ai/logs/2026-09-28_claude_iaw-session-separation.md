# 2026-09-28 — Claude: separating the commissioning session from the goal.md session

## Problem

Two Hermes sessions work in the same repository:

- **goal.md/PDB session:** root checkout `/mnt/daten-i/Sourcecode/KNXBench`, currently on branch `pdb-11-package-identity`.
- **Commissioning session:** worktree `KNXBench.worktrees/iaw-settling-delay`, branch `iaw-settling-delay`. The target is the MDT push button `1.1.67`.

They got in each other's way in three places:

1. **Root `.ai/CURRENT_STATE.md` (uncommitted).** The commissioning session had written four entries (06:46, 04:11, 22:44 and 21:50) into the root checkout's working copy. They sat between the PDB entries there. Two writers on one uncommitted file means lost updates.
2. **Shared docs.** Both branches change `.ai/CURRENT_STATE.md`, `docs/IMPLEMENTATION_STATUS.md` and `docs/KNOWN_LIMITATIONS.md`. A `git merge-tree --write-tree main HEAD` probe (no writes) found exactly these 3 conflicts, all in docs and none in code.
3. **Scratch.** Both sessions used `~/.hermes/profiles/knxbench/cache/scratch/` with generic file names.

`goal.md` is identical in both trees, and the scope does not overlap: goal.md excludes commissioning.

## What was done

- The commissioning scratch moved to `scratch/iaw/` (`mdt/`, `backup/`, `spec/`, `live-target/`, `live_props.txt`).
  - `mdt_env.py` now points `S` at the new directory.
  - Verified after the move: the option-C image still equals `target_4400.hex`.
  - Only this session's own loose probe files (`api*.txt`, `share*.txt`, `state.txt`, …) were deleted. The other session's files (`pdb11/`, `x.txt`, …) are untouched.
- The root `.ai/CURRENT_STATE.md` was reset to `HEAD` plus one handover notice.
  - Before the change, a difflib check showed that the uncommitted diff consisted only of the four commissioning entries (0 PDB lines).
  - Backups: `scratch/iaw/root_CURRENT_STATE.{before.md,HEAD.md,diff}`.
- The notice asks the goal.md session to add a do-not-touch line to goal.md §12 itself. goal.md was not edited, because it is tracked on their branch.

## Ownership rules

| Artifact | Owner |
|---|---|
| Root `.ai/CURRENT_STATE.md`, `goal.md` | goal.md session |
| Worktree `.ai/CURRENT_STATE.md`, `.ai/logs/*_claude_iaw-*.md`, `scratch/iaw/` | commissioning session |
| Branch/worktree `iaw-settling-delay` | commissioning session; others must not merge, rebase or prune it |
| Root checkout, PDB/DIN branches | goal.md session |

## Merge plan

Rebase `iaw-settling-delay` onto the then-current `main` only immediately before merging. Resolve the three doc conflicts by keeping both sides, with `CURRENT_STATE.md` entries in chronological order.

## Limitation

Hermes has no tool for sending a message directly to another session. The notice therefore travels through the file the other session must read first (`.ai/CURRENT_STATE.md` in the root checkout, per the handover protocol).
