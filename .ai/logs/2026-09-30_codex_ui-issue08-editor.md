# UI U12 / ISSUE-08 UI half — 2026-09-30

Owner: UI session in isolated `KNXBench.worktrees/ui-dpt-editor`; Web lock transferred from completed ISSUE-05. No live KNX connection or device writes. Secrets and private product data do not enter this log.

## Prerequisites and scope

- ISSUE-05 feature `997af0b5` and closeout `95a6b494` are published/read back; only their task-owned artifacts were cleaned. Root foreign edits remain untouched.
- `goal-ui.md` U12 places ISSUE-08 UI half next, then §146 channel labels, ADR-0051 Debug toggle, monitor `control` fields, readiness/compare views. The goal.md data half P1–P3 is merged, explicitly announced in `.ai/CURRENT_STATE.md` (07:06 entry). Plan: `docs/superpowers/plans/2026-09-21-user-reported-issues.md` ISSUE-08; ADR-0050 for activation/ownership, ADR-0052 for later channel labels; KNOWN_LIMITATIONS §146.
- New server fields are intentionally `#[ts(skip)]` until this UI package owns the Web surface. Remove only the skips needed here, regenerate ts-rs bindings and assert the serialized contract. Do not infer names, DPTs, channel ownership or activation from UI strings. Preserve canonical identifiers alongside display text; `program_dpt` is not a chosen object DPT.
- Per-user standing instruction: no quota checks and no GPT subagents; continue packages in goal order. K15 commissioning session owns a separate live worktree and Cargo target; do not touch them.

## Progress

Web-lock handover committed/published before any `apps/knx-web` edit. Inspect existing implementation and tests next, write RED tests, then implement the smallest clean UI projection and verify every boundary. All claims must name executed tests and gates; no compatibility claim beyond evidence.
