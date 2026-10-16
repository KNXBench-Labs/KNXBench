# Community PR recognition — 16 October 2026

## Scope

Owner requested PR participation, differentiated accepted/declined XP, PR counters
and medals in the Bug Hunters’ Hall of Fame. Isolated branch
`feat/community-pr-xp-20261016`, base `dd4212e1`. The dirty/stale shared root and
other worktrees are not synchronized or modified, except additive handover.
No subagents, bus operations, releases, dependencies or application code changes.

## Contract and evidence

The latest About dialog is a deliberate external link, not the older bundled
snapshot implementation. Extend the existing Python generator and trusted
GitHub Actions workflow; do not restore a second application scoring engine.

Official GitHub REST documentation, retrieved 2026-10-16, documents
`GET /repos/{owner}/{repo}/pulls`, `state=all`, 100 records per page and the
`merged_at` field. The issue endpoint's PR stubs do not establish merge status.
The official event reference documents `pull_request_target`, its security risk
with untrusted PR execution, and `closed` for merge events. Sources:

- https://docs.github.com/en/rest/pulls/pulls#list-pull-requests
- https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#pull_request_target

The workflow checks out only the repository's default branch and executes no PR
head code. New PR lifecycle triggers use opened/closed/reopened; read-only PR
permission is explicit. PR discussion comments and reviews remain outside scope.

PR scoring is live, not a retained award ledger: 5 submission XP, plus 20 when
merged or 10 when closed unmerged. These bonuses are mutually exclusive. A reopen
returns to 5; later merge gives 25, never 35. Closed unmerged is not interpreted
as proven rejection. Live removals remove PR contribution, explicitly documented.
Schema 2 admits schema 1 and retains the exact existing completion-award ledger,
including notification flags; unknown schema versions fail. Add explicit XP
sources and PR total/open/merged/closed-unmerged counters. PR and merge medals at
1/5/10/20/50/100 show only earned names. Existing rank/report/completion policy
and notification trust remain intact.

## Test-first observations

- Initial RED: named new PR scoring test raises the expected missing
  `pull_requests` keyword TypeError; no existing PR counting API.
- First GREEN: original suite plus scoring test passes.
- Second RED: new medals/rendering/fetch/workflow tests fail for missing medal
  definitions, absent PR columns/fetch/trigger, and absent PR failure refusal;
  missing closed timestamp also exposes an AttributeError.
- Second GREEN: 59 community tests pass, including state transitions,
  deduplication, exclusions/external bots, schema migration/notification retention,
  malformed data, all medal thresholds through 101, output-preserving PR fetch
  failure and trusted default-branch workflow invariants.
- Actual public API generation succeeds without `--notify`: thengsty has one
  merged PR, 25 PR XP and 329 total XP; hagbardZ remains at 18 XP. No new public
  comments posted. Generated Markdown/JSON/README share the same snapshot.

## Separate in-session review

Reviewed the full generator/policy/workflow diff after implementation, separately
from editing. Traced last writers from paginated fetch through validation,
state buckets, XP sum, medals, sorted rows and all generated outputs; checked
schema-1 award migration and notification flags, own-bot suppression and
fork-event execution boundaries.

- CRITICAL: none.
- IMPORTANT: none in the reviewed slice. Retained PR awards and PR notifications
  are deliberately not claimed; live reopening/removal behavior is disclosed.
- MINOR: long existing Markdown table becomes wider with the requested counters;
  GitHub provides table scrolling. Keep all contributors rather than truncating.

## Delivery status

Frozen scoped branch gate passes all nine stages: 59 community tests, 175 full
tools tests (one inherited skip), documentation, five repository checks and
whitespace. All 2940 captured source path/mode/content identities are unchanged.
The first full-tools attempt lacked installed Playwright dependencies; `npm ci`
from the existing lock resolves it. The existing xtask binary was stale despite
matching checked-in sources and reported 128 false missing-heading findings;
a fresh offline build passes all five checks, including 205 ledger rows. These
failed environment attempts remain in private evidence, not counted as acceptance.
Static added-line security scan finds no secret assignments, shell injection,
dynamic execution or unsafe deserialization. Self-review has no open blocking
findings. Actual-merge gate, publication and live Actions execution are PENDING. Local unit success and real API reads do not certify remote write
permissions or workflow execution. Full Rust/web/private-corpus/hardware gates
are not claimed for this repository-tooling-only change.
