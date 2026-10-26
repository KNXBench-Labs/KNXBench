# CLI download failure diagnostic delivery (DIAG-02 follow-up)

Observed: 26 October 2026.

## Delivered boundary

Published main-first merge `23e1ff3ec` on base `62089b033` (feature `a1b26ed06`,
documentation `a75e0a6dc`, review corrections `e5cfa1e7b`). `knx device download`
and `knx device restore` now build the same payload-free `DownloadFailure` as
the server worker, from the executor's own terminal error and progress:

- One `diagnostic:` terminal line precedes the written classification.
- The durable activity history keeps the cause, phase, step, read-back count and
  restart uncertainty; the optional download `--report` keeps the same value
  alongside `outcome.stoppedInStep`.
- An inconsistent diagnostic can fall back to a failed row without a cause, but
  now warns the operator. If both recording attempts fail, the original history
  error survives instead of being replaced by an already-recorded error.
- Regular download and restore call the same execution-and-recording helper.
  Cleanup, write admission and exit predicates remain unchanged.

[Contract](../contracts/download-failure-diagnostic.md) ·
[initial server/web delivery](2026-10-26-download-failure-diagnostic.md).

## Acceptance

- Independent whole-feature review: APPROVE with five Minor findings. Corrected
  error preservation, fallback warning, restore composition coverage, actor-neutral
  lagged-frame wording and output-order assertions. Focused re-review of the
  corrections: APPROVE, no remaining findings.
- Maintained change-scoped branch and merge gates: each 14/14 stages, frozen inputs,
  no touched or unchecked user data. The merge gate ran 2,702 Rust tests across
  14 binaries (127 ignored), selected corpus 91/91; tooling registered 431 tests,
  six skipped. Browser and bindings stages were not selected; the frontend bundle
  was built as the desktop prerequisite, not as browser acceptance.
- Local, tracking and live `main` read back equal to `23e1ff3ec`.
- Focused CLI selection: 30 passed, including 15 failure cases. Owning CLI runs:
  174 unit and 242 integration tests passed, 21 ignored. The app's focused
  diagnostic selection passed ten tests. These overlap the package gate and are
  not additional unique-test totals.
- Original semantic REDs preceded the corrections for lost history errors,
  missing fallback warnings and frontend-specific lagged wording. The new restore
  helper first failed to compile because its interface was absent; this is not an
  original runtime-defect reproduction. Three later calibration mutants exposed
  lost partial-write classification, reversed output ordering and omitted helper
  recording. The initial six mutation probes belong to the earlier feature
  revision; they are not executions on the corrected final sources.

## Remaining boundaries

Simulator and helper-composition coverage only: no complete restore CLI
admission/tunnel invocation and no real installation or hardware validation.
System-2 failure rows and never-connected CLI rows still have no structured cause;
older rows and reports are not backfilled. No CLI stop-on-request or unwitnessed-task
classification is introduced. The original field failure's cause remains unknown;
no new retry, restart, restore policy, protocol behaviour or compatibility claim.
