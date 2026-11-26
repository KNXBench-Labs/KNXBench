# Archived restart-snapshot tracks, 2026-11


<!-- archived 2026-11-18 from .ai/CURRENT_STATE.md -->
## UI-polish intake WP-44/45/46 (DONE; owner WebUI ui-polish; observed 2026-11-17)
- Owner UI feedback 17 Nov consolidated into three open-work packages on `origin/main` `9af5c698a`: WP-44 (bus monitor sticky telegram details, status-bar session rx/tx counters, connected animation), WP-45 (compact selects 20-25 ch with hover marquee), WP-46 (calmer copy: Standard/Default priority label, explanations behind ?, one-time manufacturer hints). Committed as `9c9badceb` on `work/ui-polish-open-work-20261117`, worktree `<sources>/KNXBench-ui-polish-20261117`, pushed and remote-readback verified; docs-only, check_local_info/check_documentation/check-anchors/check-ledger PASS. No implementation started; no bus/hardware/VM/release/deploy. Root stale/dirty and foreign tracks preserved; .ai/CURRENT_STATE.md copied from root unchanged.
# Restart snapshot — Issue #53 taken by WebUI issue53 (2026-11-16): offline report/source/plan validation on `work/issue53-hager-evidence-20261116` in `<sources>/KNXBench-issue53-20261116`; no bus/hardware/VM; promotion, independent review and gates pending. Issues #49–#52 remain owned by issue-packages.

<!-- archived 2026-11-18 from .ai/CURRENT_STATE.md -->
## UI-polish intake WP-44/45/46 (DONE; owner WebUI ui-polish; observed 2026-11-17)
- Owner UI feedback 17 Nov consolidated into three open-work packages: WP-44 (bus monitor sticky telegram details, status-bar session rx/tx counters, connected animation), WP-45 (compact selects 20-25 ch with hover marquee), WP-46 (calmer copy: Standard/Default priority label, explanations behind ?, one-time manufacturer hints). Docs-only intake on `work/ui-polish-open-work-20261117`, pushed and remote-readback verified; no implementation started; no bus/hardware/VM/release/deploy. Archived to `.ai/archive/`; the open-work docs carry the live status.

<!-- archived 2026-11-26 from .ai/CURRENT_STATE.md -->
## Catalogue keyboard selection (DONE; owner WebUI; observed 2026-11-26)
- Delivered on `origin/main` via PR #154 (merge `ca93fa4d5`, tree-identical to gated commit `13e95e411`): when `Product` is hidden, each row's native selection button moves to its first visible metadata cell with product identity in the accessible name, preserving valid column preferences and reload behavior.
- Change-scoped 11-stage package gate passed (`inputsFrozen=True`, `userDataTouched=[]`, `web-gate` 36 stages / 1,553 browser tests: 1,454 functional, 99 exclusive; earlier load timeout in `background-activity.e2e.ts:215` passed cleanly in an isolated targeted run). Review used documented coordinator self-review while external CLI reviewers were at usage limits. Root checkout and foreign tracks preserved; no release, bus, hardware or VM authority.
