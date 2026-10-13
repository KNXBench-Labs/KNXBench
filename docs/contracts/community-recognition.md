# Public community recognition

## Boundary

Owner-approved repository recognition, 2026-10-13. The README and
[Hall of Fame](../community/HALL_OF_FAME.md) list every external human issue author
and issue-thread replier. Maintainer accounts are **excluded entirely**, not given
a second table. Bots and PR threads are excluded. English titles and humour are
part of this acknowledgement, not a measure of technical competence.

This has no connection to the application's local achievements, customer projects,
bus operations or analytics. `tools/community_hall_of_fame.py` uses only the public
GitHub issue/comment APIs via the existing `gh` CLI. No new Python dependency.

## Evidence and scoring

Versioned data: `docs/community/hall-of-fame.json`, schema 1. Explicit policy:
`docs/community/hall-of-fame-policy.json`. Count all open/closed issues and all
current issue-thread replies, with full pagination. Fetch failures, malformed
identifiers, unknown states/snapshot versions or conflicting duplicate records
fail rather than publish partial or invented counts. Deduplicate identical API
records by stable record id. This is a paginated observation, not a transactional
snapshot of a concurrently changing GitHub repository.

- +5 XP per issue, +1 XP per reply, counters separately disclosed.
- +10 XP to the reporter on a `completed` closure, once per issue number.
- No closure XP for `not_planned` or an unspecified closure reason.
- Retain the completion award across reopen/reclose/deletion/transfer.
- Reply deletion changes its live counter; edits do not create a second reply.
- Sixteen thresholded titles, listed in the public page/data.
- Golden Bus Probe at ten authored issues.
- Stable alphabetical tiebreak; equal XP has equal position.

The award ledger stores only issue number, public handle, first observed completed
closure time, awarded XP and notification state. Generated rows store handles,
counters, titles and medals. No report bodies, customer names/files or email
addresses are published. Exclusion is explicit and case-insensitive; a maintainer
can add an opt-out. Account-renaming reconciliation by immutable GitHub user id
is not implemented; retained historical awards use the observed public handle.

## Rendering and recovery

Only the bounded `HALL-OF-FAME` marker region in README is generated. The dedicated
page and JSON are generated in full. Validate/render all payloads before replacing
files; each replacement is staged adjacent to its destination. A working-tree
I/O failure can interrupt the three replacements; a Git commit publishes the
consistent completed set. Unchanged observations retain their timestamp to avoid
a daily no-op commit. Never truncate the scoreboard to a top-N list.

GitHub workflow `Community Hall of Fame` runs on issue open/close/reopen/transfer,
issue-comment create/edit/delete, a daily schedule and manual dispatch. It only
runs for this repository and executes trusted default-branch code. No issue body,
login or PR checkout is interpolated into shell commands. Contents/issue write
permissions are restricted to this workflow; token values are never logged.

Concurrency has no active-run cancellation; each run reconciles **all** pending
eligible awards, not just its triggering issue. This recovers a close event whose
pending run GitHub superseded. Default-branch pushes retry from a fresh disposable
Actions checkout, never force-push or reset a developer worktree. Publication uses
`KNXBench <github@knxbench.com>` with no co-author trailer. Bot replies do not count
or retrigger this workflow.

## Completion acknowledgement

Eligible new completed issues get a separate automatic issue reply:

```text
Field-test credit for @tester: +10 XP for this completed issue.
Total: 15 XP · Title: Bus Bug Scout
1 issues · 0 replies · 1 completed reports.
Rank up: Telegram Trainee → Bus Bug Scout.
```

Real comments also link the board and include the medal when earned. Do not invent
an extra rank-up when the bonus stays within the current band. Historical closures
before `notify_since` receive scoreboard credit without retroactive comment spam.

Before posting, re-read that the issue remains closed/completed. A per-issue marker
from GitHub Actions or an excluded maintainer recovers interrupted posting; an
external comment cannot spoof it. Mark notified only after exact target readback.
The persisted flag prevents duplicate notices after reopen or deletion of the
original notice. If posting succeeded but persistence failed, the trusted marker
recovers the next run. A deletion in that narrow crash window cannot be recovered
from a marker that no longer exists; no claim of distributed exactly-once delivery.

## About dialog (Goal B16, 14 October 2026)

Implemented as handed off. `apps/knx-web/src/bundledHallOfFame.ts` reads
`docs/community/hall-of-fame.json` at build time (`?raw`); there is no request to
GitHub or anywhere else for it. `hallOfFame.ts` admits it like untrusted input:
schema 1, a parseable `generated_at`, and per contributor a handle, a position
≥ 1, non-negative integer XP/issue/reply/completion counters, a title and text
medals. Any malformed or duplicate (case-insensitive) contributor refuses the
whole snapshot, so About never shows part of a board; it then says the bundled
board could not be read and still links the live one. Order, positions (as the
`<ol>` item values, so equal XP keeps an equal number), XP, titles and medals are
the generator's; About computes none of them. Titles and medals stay English in
every UI language. Handles are React text; only a handle matching GitHub's
grammar (1–39 letters, digits, single inner hyphens) links to
`https://github.com/<handle>`, and the live-board link is built from the
application's own repository constant, never from the file. The section shows the
snapshot time and "This is not a live count." No domain, storage or project
mutation is involved.

Because the file lies outside `apps/knx-web`, the Vite dev server, the browser
fixture server and Vitest's DOM environment each allow exactly
`docs/community` beyond the app (`devFileAccess` in `vite.config.ts`).

## Acceptance

Thirty-two new unit tests cover counting, reply-only contributors, maintainer/bot/PR
exclusion, pagination-record deduplication, once-only/retained completion credit,
thresholds, all-user rendering, malformed inputs, README bounds, unchanged data,
trusted notice recovery, post readback and stale/reopened closure handling.
Real API generation was exercised. Full tools suite must run with checkout-bound
`PYTHONPATH`; the initial unbound discovery imported the wrong tools namespace and
failed, not the counting feature. Remote workflow execution/acceptance is recorded
in the delivery log; local tests alone do not certify GitHub write permissions.
