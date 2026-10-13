# Community recognition and issues #17/#18

Date: 2026-10-13. Agent: codex, session `922727d3ed74`.

## Decisions and boundaries

Owner requested an English public scoreboard with issue/reply counters, increasing
KNX-themed titles, a medal for the frequent tester and a listing for every
reporter/replier. Owner explicitly excluded maintainer accounts, approved the
README/dedicated page plus automatic refresh, requested an About-dialog surface
through Goal B, and requested earned XP and rank/title in closure follow-ups.
Scoring: 5 per issue, 1 per reply, 10 once per completed closure. Sixteen titles;
Golden Bus Probe at ten issues. No separate maintainer table.

Repository tooling is independent of application achievements/project storage.
Public schema-1 snapshot is suitable for a build-time About import; no app network
access, telemetry or web/domain change is implemented in this package.

## Implemented

- Real paginated issue/comment collection via existing `gh` CLI.
- Human-only non-PR participation, explicit maintainer/opt-out policy, deduplication,
  deterministic ordering/ties, every participant shown.
- Once-only retained completed-issue awards; no bonus for not-planned closures.
- Marker-bounded README table, dedicated page, public minimal JSON/policy.
- Default-branch-only workflow, issue/comment events, daily/manual reconciliation,
  pending-award recovery, non-force push retries in disposable Actions checkout.
- Completion-credit comments with XP, total, title and true rank-up announcement,
  exact post readback, trusted-marker retry recovery and persistent notification flag.
- Contract documents freshness/privacy, deletion/rename/recovery limits and About handoff.

## Triage

`docs/status/2026-10-13-issues-17-18-triage.md` contains source lines and proposed
acceptance. #17: positional text-template family unsupported; no claim it is a
translation key and no unverified frontend brace-stripping fix. #18: handoff waits
for first download-status response; a pending-response component probe reproduced
the empty selection and a 404 control restored it. The reporter's 30 seconds is
not locally measured. Status backend uses try_lock; project-wide readiness remains
a performance candidate to measure. Both acknowledged/assigned/bug-labelled with
exact remote readback (comments 6154991930 and 6155169770).

Goal B request: separate #17/#18 packages (suggest B14/B15) and About recognition
(suggest B16), ordering/dependency resolution stays with that session. No actual
bus traffic, private project/tester fixture or product implementation was performed.

## Executed evidence

- Missing new module established initial TDD RED.
- Repository admission regression additionally proved a named RED assertion before
  the constructor validation fix; complete new suite now 32/32.
- All tool tests with checkout-bound PYTHONPATH: 122 tests, 0 failures/errors,
  one existing skip. Initial namespace-unbound discovery failed on
  `tools.run_corpus_tests`; corrected environment, not a product change.
- Real GitHub API generation succeeded; counters are not fixtures.
- Workflow YAML event inventory and extracted bash syntax checked.
- Source self-review performed separately (not independent approval): no reporter
  text in shell, no untrusted PR execution, explicit publication scope, retained
  awards, post readback. Handle-rename limitation disclosed.
- #18: baseline 25/25 pass, control 1/1 pass, immediate-selection expectation fails
  at named assertion. Temporary probe removed; no failed test in the package.
- Documentation checker and xtask header/anchor checks passed before final metadata;
  final merged-input rerun and actual remote workflow receipt recorded below.

## Delivery receipt

PENDING: final merged-input tool/docs/xtask checks, publication readback and real
GitHub workflow execution. Do not treat local success as GitHub permission proof.
About dialog and fixes for #17/#18 remain handoff requests, not completed features.
