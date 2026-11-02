# Catalogue polling and disposed-owner lifetime — acceptance, 3 November 2026

## Delivered source

Main-first source `5296e56c5` is published. Local, tracking and live `main`
were read back as the same revision after publication. The controller/fixture
correction comprises `09833e8b7`, `cb5155b77` and `1213e6ff9`; the final
combined source, not the first intermediate fix, is the accepted candidate.
The separate locale-readiness fixture correction is `f3366cfe9`.

This is a correctness prerequisite found while accepting WP-27, not delivery
of WP-27 itself. It changes no server endpoint, project storage, device command,
comparison flow, download operation or UI layout.

## Corrected contracts

- A queued automatic catalogue poll is removed after valid terminal-status
  adoption. Its callback checks its own timer ticket before changing the timer
  reference, then rechecks connection, operation identity, running state and
  existing error gates. Explicit manual terminal refresh remains available.
- Late refresh errors from a retired operation cannot change an empty or newer
  operation's snapshot. An already pending read retains its exclusivity until
  actual settlement; normal polling for the new owner then resumes. No global
  transport abort or replayed upload/apply is used as a substitute.
- The synthetic fixture retains only actually admitted, successfully disposed
  handle/capability pairs. Their exact authenticated, query-free, body-free GET
  receives typed **410 `outcomeUnavailable`**, JSON and `Cache-Control: no-store`,
  matching the existing server contract. Unknown IDs, extra paths, wrong methods
  and wrong/missing authority remain rejected. There is no real-network fallback.
- The footer fixture waits for the requested document language before dismissing
  startup toasts. It retains the explicit-connect, session-bound-stop, unexpected
  request, live-toast overlap and hit-target assertions.

The historical strict-fixture failure observed a known old job GET during a
DELETE transition. It did not prove a new poll started after client-side Dispose
completed, new-job corruption, an install mutation, or an exact JavaScript timer
initiator. The deterministic regressions establish the separate client defects.

## Source-bound verification

Independent whole-fix review approved the final four-file catalogue candidate;
the locale fix has its own source review. Its reviewed Git blob was verified
identical before adoption. All catalogue source hashes remained unchanged.

| Execution | Result |
| --- | --- |
| Original baseline with final regression test bytes | 14 failed, 29 passed; semantic assertion failures, no skipped cases |
| Intermediate timer-ticket discriminator | 1 failed, 22 passed; separate self-review defect, not an original-baseline claim |
| Final focused controller/fixture/API/protocol/consumer tests | 57/57 passed; app and catalogue-fixture TypeScript checks passed |
| Focused footer, development / production communication | 20 / 20 passed |
| Focused catalogue update, development / production catalogue | 22 / 22 passed |
| Complete selected branch package gate | 11/11 stages passed, frozen inputs |
| Main-first selected package gate | 11/11 stages passed, frozen inputs; no touched or unchecked user data |
| Main-first full Vitest suite | 5,034/5,034 passed |
| Main-first default Chromium | 845 functional and 46 exclusive passed |
| Main-first production catalogue | 22 functional passed; compiled exclusive lane empty |
| Main-first production communication | 87 functional and 1 exclusive passed |

Browser reports have no unexpected, skipped or flaky outcomes. Counts describe
separate executions/configurations and are not added into a unique-test total.
The selected package includes build, all derived TypeScript checks, full Vitest,
formatting, documentation, browser-lane policy, layering, headers, anchors,
ledger, corpus-policy checks, tool tests and clean diff. Corpus-policy checks
are not corpus execution. Historical failures remain retained, not relabelled
as green. Early TDD phase labels were not source fingerprints; the final pinned
baseline replay and final candidate carry actual source bindings.

## Remaining boundaries

All browser replies and requests were synthetic and isolated. This is not a
native HTTP/disposal integration run: the unchanged server's 410 contract was
checked against its source and existing service tests, not rerun as a native
endpoint. No new Rust/corpus runtime, real device, bus, manufacturer, VM,
WebKitGTK, release or deployment acceptance is claimed. Final WP-27 integration
and its own acceptance remain separate work.
