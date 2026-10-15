# Community Hall-of-Fame medal expansion — 2026-10-15

## Owner request and scope

Extend community recognition through 100 issues, acknowledge the existing
20-report contributor, award resolution milestones every five completed reports,
keep unearned medal identities a presentation surprise, reorder columns to
Place → Bug hunter → Rank & title → Medal → XP → Issues → Replies → Resolved,
and admit external AI agents without human-only wording.

Implemented isolated from the selected stale/dirty root and foreign P4, story
and support-reporter worktrees. The current About dialog links to the public
board; it does not import the snapshot. No application/UI/domain/storage or
KNXnet/IP behavior changed.

## Behavior

- Cumulative report medals every ten issues through 100, preserving the
  original ten-report award and adding the requested 20-report milestone.
- Cumulative resolution medals every five retained completed-report awards
  through 100. `not_planned` and unspecified closes earn no completion progress.
- All earned names appear in the table and future ordinary completion notices.
  Public goals provide only cadence/hints; generated JSON has no unearned medal
  catalogue. Public Python source is inspectable, not a secrecy guarantee.
- Existing XP/ranks, historical completion awards and notified flags retained.
  Backfill updates the board without historical comment spam.
- External GitHub Bot accounts count alongside other reporters/repliers;
  the scoreboard's own Actions account remains excluded and cannot self-trigger
  comment refresh. Profile URL segments are encoded for `[bot]` handles.
- Self-review found scoring exclusions were also granting notification-marker
  trust. Added a separate optional validated `trusted_notice_logins` policy
  list, preserving maintainer recovery without granting opt-outs that authority.

## Actual verification before publication

- Test-first RED/GREEN for milestone boundaries (0 through 100 and beyond),
  render order/empty state, spoiler exclusion, external Bot participation and
  workflow guard. Opt-out marker spoofing reproduced before the trust fix.
- 47 community tests pass; complete tools discovery runs 163 tests with one
  inherited skip and no failures after installing locked npm dependencies.
  The first full run lacked Playwright's local CLI; retained as an environment
  prerequisite failure, not accepted as a pass.
- Documentation checker passes; fresh offline xtask build plus check-layering,
  check-headers, check-anchors, check-ledger and check-corpus-gates pass.
- Live public GitHub generation succeeds: two contributors, 21 authored issues,
  16 replies. Leading reporter has 20 issues, 19 completions, five earned medals,
  303 XP. README/page/JSON agree exactly, with unchanged prior award ledger and XP.
- Separate in-session source/security self-review; no independent review claimed.

Publication, actual remote workflow execution and final cleanup are recorded
only after they happen in the closing handover/private evidence receipt.
No Rust/frontend runtime suite, hardware operation, release or deployment claimed.
