# Public community recognition

## Boundary

Owner-approved repository recognition, 2026-10-13. The README and
[Hall of Fame](../community/HALL_OF_FAME.md) list reporters and issue-thread
repliers and PR authors, including external AI agents and GitHub Bot accounts (owner amendment,
15 October 2026). Maintainer accounts, explicit opt-outs and the scoreboard's own
`github-actions[bot]` account are **excluded entirely**, not given a second table.
PR discussion/review comments are excluded; PR authorship and merge outcomes count
(owner amendment, 16 October 2026). English titles and humour are part of this
acknowledgement, not a measure of technical competence.

This has no connection to the application's local achievements, customer projects,
bus operations or analytics. `tools/community_hall_of_fame.py` uses only the public
GitHub issue/comment/pull-request APIs via the existing `gh` CLI. No new Python dependency.

## Evidence and scoring

Versioned data: `docs/community/hall-of-fame.json`, schema 2 (schema-1 migration
preserves all completion awards and notification flags). Explicit policy:
`docs/community/hall-of-fame-policy.json`. Count all open/closed issues and all
current issue-thread replies and all PRs, with full pagination. Fetch failures, malformed
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
- Cumulative report medals at every ten authored issues, through 100 (including
  a new milestone at 20); the existing ten-report medal is preserved.
- Cumulative resolution medals at every five retained completed-report awards,
  through 100. `not_planned`/unspecified closures never advance this track.
  Reopen/reclose cannot duplicate an award; deletion/transfer retains completion
  progress, while authored-issue medals follow the live authored counter.
- Medals do not alter XP or the sixteen existing rank/title thresholds.
- Public goals give cadence and hints, never an unearned medal-name catalogue.
  Generated README/page/JSON and completion notices show only earned names.
  The source and tests are public: this is presentation-level surprise, not a
  secrecy/security promise. No new notification ledger or snapshot schema.
- +5 XP per submitted PR (open or draft); +20 merge bonus (**25 total**) or
  +10 closed-unmerged bonus (**15 total**). The two outcome bonuses never stack.
  A closed-unmerged PR is not necessarily a rejected contribution.
- `pull_requests`, `pull_requests_open`, `pull_requests_merged` and
  `pull_requests_closed` (closed **without** merge) are separate counters.
  The three state counters sum to the submitted total. PRs never count as issues.
- `xp_breakdown` separates issue/reply/completion/PR XP, summing to total XP.
- PR and merge medals each unlock at 1/5/10/20/50/100. Only earned names appear;
  closed-unmerged PRs advance submission participation, not merge medals.
- PR XP/counters/medals are live: reopening restores submission-only XP; later
  merging replaces the closed bonus. Removed PRs stop counting. Unlike completed
  issue awards, PR credit is not retained in a historical ledger. Historical PRs
  are included; one maintained PR credit comment is reconciled without replaying
  issue notices (notification amendment below).
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
issue-comment create/edit/delete, PR open/close/reopen (`pull_request_target`),
a daily schedule and manual dispatch. It only
runs for this repository and executes trusted default-branch code. No issue body,
login or PR checkout is interpolated into shell commands. Only the trusted default
branch is checked out; no fork/PR head executes. Issue and PR comment write
permissions are explicit and limited to this trusted workflow.
Contents/issue write
permissions are restricted to this workflow; token values are never logged.

Concurrency has no active-run cancellation; each run reconciles **all** pending
eligible awards, not just its triggering issue. This recovers a close event whose
pending run GitHub superseded. Default-branch pushes retry from a fresh disposable
Actions checkout, never force-push or reset a developer worktree. Publication uses
`KNXBench <github@knxbench.com>` with no co-author trailer. External bot replies
count and trigger refreshes; only the scoreboard bot's own replies are ignored
by the event guard and counter, preventing self-scoring/notification loops.
Bot handles may include a bounded literal `[bot]` suffix; profile URL path
segments are percent-encoded. Unknown actor types still fail closed. Being an
eligible participant does not make an external bot's award marker trusted.

## Completion acknowledgement

Eligible new completed issues get a separate automatic issue reply:

```text
Field-test credit for @tester: +10 XP for this completed issue.
Total: 15 XP · Title: Bus Bug Scout
1 issues · 0 replies · 1 completed reports.
Rank up: Telegram Trainee → Bus Bug Scout.
```

Real comments also link the board. They name only a medal that this completion
unlocked (see "Medals in bot comments" below); the full collection lives in the
board table. New milestones are reconciled retroactively on the board, without reposting
already-notified issue comments or creating separate milestone-comment spam. Do not invent
an extra rank-up when the bonus stays within the current band. Historical closures
before `notify_since` receive scoreboard credit without retroactive comment spam.

Before posting, re-read that the issue remains closed/completed. A per-issue marker
from GitHub Actions or an explicitly trusted maintainer recovers interrupted
posting; an external comment cannot spoof it. The optional policy list
`trusted_notice_logins` is validated separately from `excluded_logins`; opting
out of scoring grants no notification-marker trust. Without that list only the
scoreboard bot is trusted. Mark notified only after exact target readback.
The persisted flag prevents duplicate notices after reopen or deletion of the
original notice. If posting succeeded but persistence failed, the trusted marker
recovers the next run. A deletion in that narrow crash window cannot be recovered
from a marker that no longer exists; no claim of distributed exactly-once delivery.

## PR credit acknowledgement (16 October 2026 follow-up)

`--notify`, already invoked by the maintained workflow, also reconciles one
comment per eligible PR, including historical and open/draft PRs. The comment
shows this PR's total XP and submission/outcome calculation, its current status,
the author's board total and title, issue/reply/completion/PR counters, XP source
breakdown, any medal this PR unlocked and the board link. No unearned names are
revealed and no extra XP is awarded for the comment.

A trusted `<!-- knxbench-pr-credit:<number> -->` marker identifies the existing
comment. Create if absent; PATCH that exact comment when the generated body
changes; perform no write when unchanged. Reopen, close without merge and merge
update the same comment, with live 5/15/25 PR XP rather than accumulating bonuses.
Other contribution changes can also update the displayed board total.
All eligible PRs reconcile on every run, recovering superseded event runs.

Only explicitly trusted notice accounts or the correctly typed scoreboard bot
can own this marker; an external participant/opt-out cannot forge it. Multiple
trusted marked comments fail rather than guessing a target. Before a write,
re-read the PR id/number, author identity, state and merge/close timestamps; if
these differ from the counted observation, skip the stale write until the next
refresh. Unrelated user metadata such as avatar changes does not block it.
POST/PATCH must pass an exact id/body readback; PATCH also binds the response to
the originally requested comment id. API/permission failures fail the job.

There is no new snapshot schema or PR notice ledger. Retries discover an already
posted/edited trusted comment from GitHub, even if the preceding run failed
before committing outputs. Deleting the comment permits recreation; concurrent
GitHub changes are not transactional and exactly-once delivery is not promised.
Workflow serialization reduces duplicate risk; no unrelated comments are deleted.
The completed-issue ledger and its notification semantics remain unchanged.

The [official shared issue/PR comment API](https://docs.github.com/en/rest/issues/comments)
documents POST creation and PATCH updates through the Issues endpoints and accepts
Issues **write** or Pull requests **write** permission. The initial live Actions
attempt failed at PR comment creation while only `pull-requests: read` was set;
its captured subprocess error did not expose an HTTP status, so no exact server
reason is claimed. The workflow now explicitly grants `pull-requests: write`
alongside existing `issues: write`. API failures expose only an HTTP status when
available, never raw response or submitted comment contents. It still executes
only default-branch code, never PR code, and bot comments remain excluded from
scoring and self-triggering.

## Medals in bot comments (owner amendment, 19 October 2026)

The owner reported that every XP update repeated the contributor's whole medal
list. The board table is now the only place that shows the collection; a bot
comment shows a medal line only when the event it acknowledges reached a
milestone, and shows no medal line otherwise.

- **Completion note:** the reporter's retained awards are ordered by closure time
  (`closed_at`, then issue number); the n-th award unlocks the resolution medal
  whose threshold is n. Two completions counted in one refresh therefore still
  credit the milestone to the right issue.
- **PR credit comment:** the author's counted PRs are ordered by PR number; the
  n-th PR unlocks the PR medal with threshold n. A merged PR additionally takes
  its place among the author's merged PRs (`merged_at`, then number) for the
  merge track. A first PR that is merged therefore names both first medals.
- Report-count medals (every ten authored issues) belong to no single event that
  the bot acknowledges, so they appear only on the board.

Medal thresholds, XP, ranks and the snapshot schema are unchanged. Because PR
credit comments are maintained, the first refresh after this change edits each
existing PR credit comment once to drop the old collection line; already-posted
completion notes are not edited or reposted.

## About dialog (issue #19, owner amendment, 14 October 2026)

The owner replaced Goal B16's bundled snapshot with a direct link to the
[public Hall of Fame document](../community/HALL_OF_FAME.md). About renders one
translated link, built from the application's fixed repository constant and the
canonical `blob/main/docs/community/HALL_OF_FAME.md` path. It uses
`target="_blank"` and `rel="noopener noreferrer"`; opening About makes no request
for the board. Only deliberate link activation opens GitHub. There is no JSON
import, snapshot parser, local participant list, counter or scoring logic in the
app, and no external-directory exception in Vitest. The generator, public JSON,
policy, awards and repository workflow remain unchanged.

The earlier source imported a file outside `apps/knx-web`, which the Docker
frontend stage does not copy; the actual frontend target reproduced an
`UNRESOLVED_IMPORT` for `bundledHallOfFame.ts`. Removing that dependency fixes the
app-only frontend build without enlarging the Docker context or adding another
copy/generation step. The link needs an Internet connection when opened; there
is deliberately no offline board. No domain, storage or project mutation and
no release or deployment is part of this change. The original B16 implementation
and acceptance remain historical in the delivery archive.

## PR evidence and limitations (16 October 2026)

The [REST pull-request list](https://docs.github.com/en/rest/pulls/pulls#list-pull-requests)
with `state=all`, `per_page=100` and complete pagination supplies `state` plus
`merged_at`. Issue-list PR stubs cannot establish merge status. Missing or
contradictory states/times and duplicate/conflicting identities fail before
outputs or notifications. A failed PR fetch cannot publish an issue-only board.
The [workflow event reference](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request_target)
documents the trusted-context trigger and the risk of executing untrusted PR
code. The workflow never executes a PR head, downloads PR artifacts or interpolates
PR fields into shell code. Merge/close notifications are not fabricated from
reviews or labels. PR review/discussion participation and retained PR awards
remain unsupported. The board is a paginated live observation, not an immutable
or transactional history; account-renaming limitations above remain.

## Acceptance

The original thirty-two unit tests covered counting, reply-only contributors,
maintainer/bot/PR exclusion under the original policy, pagination-record
deduplication, once-only/retained completion credit,
thresholds, all-user rendering, malformed inputs, README bounds, unchanged data,
trusted notice recovery, post readback and stale/reopened closure handling.
Real API generation was exercised. Full tools suite must run with checkout-bound
`PYTHONPATH`; the initial unbound discovery imported the wrong tools namespace and
failed, not the counting feature. Remote workflow execution/acceptance is recorded
in the delivery log; local tests alone do not certify GitHub write permissions.

15 October 2026 amendment: 47 community tests pass, including all integer medal
boundaries through 100 and the beyond-cap cases, all earned-name rendering,
unearned-name exclusion, external Bot scoring, own-bot event suppression,
backfill without duplicate notices and scoring-opt-out/marker-trust separation.
The full tools run has 163 tests with one inherited skip. Branch and merged
scoped gates pass. Real GitHub workflow `38694177327` completes successfully
on published source `fe1f4101`, including verification and live reconciliation.

16 October 2026 PR amendment: 59 community tests and 175 full tools tests
(one inherited skip) pass. Frozen branch and actual-merge scoped gates each
pass all nine stages, with all 2940 source identities unchanged. Freshly built
repository checks pass (the pre-existing xtask binary was stale). Actual GitHub
workflow `38885828890` succeeds on published runtime merge `7c67bb8d`, including
59-test verification and real reconciliation. Public Chromium checks at
1440/400px verify all 13 table headers, both observed contributors, PR counters,
earned medals, totals and reachable horizontal table scrolling. Existing
completion awards/notification flags are exactly preserved during real migration.
No application build, Rust workspace, private-corpus, hardware or release gate
is claimed by this repository-tooling-only acceptance.

PR-comment follow-up: 73 community tests and 189 tools tests (one inherited skip)
pass; all nine corrected actual-merge scoped stages pass with 2948 unchanged
source identities. Live workflow `38907119595` on runtime `45091f56` creates
bot comment `6216417065` on PR #24 with exact id/body readback. A second live run
`38907250544` succeeds without another comment or any body/updated-at change.
The initial live POST failure is retained as failed evidence, not relabelled.
No new application/domain/schema/scoring/dependency or hardware/release scope.

Medal-notice amendment (19 October 2026): 81 community tests pass, including
completion milestones 5/10/15/20 named on exactly their issue, closure-order
attribution, two completions in one refresh, report medals kept to the board,
first merged PR naming both tracks, merge-time ordering across authors and a
posted PR comment without a medal line. The live workflow run is not part of
this local acceptance.
