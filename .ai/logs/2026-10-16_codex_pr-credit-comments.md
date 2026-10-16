# PR credit comments — 16 October 2026

## Scope and design

Owner explicitly requests the Actions-authored issue-style XP acknowledgement on
PRs too. Base `32b4bb81`, isolated `feat/community-pr-notices-20261016` worktree.
Current recognition contract/source/status read before implementation. No
application/domain/scoring/schema/dependency/hardware/release change. Shared root
and foreign worktrees remain untouched except additive handover.

Use the existing generator's `--notify` boundary and comment APIs. One trusted
marker `knxbench-pr-credit:<number>` identifies a maintained comment per eligible
PR, including historical PRs and open/draft PRs. Reconcile points/state, board
total, English title, all counters, XP sources and only earned medals. Live
status/author/identity must still match the counted observation before any write.
Create once, PATCH on changes, no write if unchanged; never add per-event spam.
Comment deletion allows recreation; there is no separate durable PR notification
ledger, and no distributed exactly-once guarantee. Existing issue completion
ledger/once-only notices stay unchanged. Public comments and bot self-events
remain excluded from scoring/self-triggering. Explicitly trusted maintainers or
the correctly typed scoreboard bot can own markers; scoring opt-out grants no
marker trust. Multiple trusted marked comments fail rather than guessing one.

## Test-first evidence

Initial named RED raises expected missing `notify_pull_requests` AttributeError.
First GREEN covers merged PR text, exact comment readback, title and medals.
Second RED shows missing real API get/PATCH adapter and unwired CLI notification
call. Both corrected; full community suite then passes. Additional lifecycle,
retry, stale-source, excluded/external-bot and earned-only presentation regressions
cover sibling paths. Real API/workflow and complete scoped gates remain PENDING.

## Separate in-session self-review

Full code diff traced independently from editing, from fetched PRs and existing
score rows to comment discovery, trusted-author admission, live re-read, create/
PATCH and readback; existing issue notice behavior unchanged. No subagents.

- CRITICAL: none.
- IMPORTANT: none in intended scoring/notification behavior.
- MINOR: edit readback compares response id to its own receipt, not to the
  requested existing comment id. Add explicit target-id regression and enforce
  it before accepting an update.
- MINOR: comparing whole public `user` objects can skip otherwise valid updates
  if nonidentity GitHub metadata changes. Restrict identity comparison to
  id/login/type while preserving status and PR id/number equality.

Both review findings reproduced as named failing assertions before correction.
PATCH now validates the response against the requested existing comment id; actor
comparison uses id/login/type rather than mutable public metadata. All 72
community tests pass after correction; separate recheck finds no open blocking
or cheap minor finding. Real read-only generation preserves the entire issue
award ledger and current 329/18 XP. No public comment has been written yet.
Frozen branch gate passes all nine planned stages: 72 community tests, full tools
188 tests (one inherited skip), docs, five freshly built repository checks and
whitespace. All 2948 source path/mode/content identities remain unchanged.
Added-line security scan has zero secret/shell-injection/dynamic-execution/pickle
candidates. At this branch checkpoint actual-merge acceptance and remote workflow
were pending; their subsequent outcomes follow below. Public GitHub comments are
external writes: only the published maintained workflow exercises them after
branch and actual-merge acceptance. Scope includes existing PR #24.

## Live workflow correction

Published initial runtime `ce8dc746` after nine-stage branch and actual-merge
acceptance. Real workflow `38906647856` passed tests but failed creating the
PR #24 comment. No PR credit comment existed after that run. The captured
CalledProcessError omitted stderr; its HTTP status/server reason is unknown,
not asserted as a verified 403. Added named failing regressions for explicit PR
write capability and sanitized API failures, then corrected both. Final workflow
uses `issues: write` and `pull-requests: write`, while retaining trusted default
checkout and no PR code execution. API failures show only an extracted HTTP
status, never stderr/body/token text. Fresh acceptance and real workflow retry
remain PENDING. The correction branch executes all runtime/documentation stages
successfully but its final whitespace check refuses one extra log EOF blank line;
that layout-only defect is removed before commit and actual-merge acceptance.
