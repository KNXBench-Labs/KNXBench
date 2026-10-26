# Regular-download failure diagnostic delivery (DIAG-02)

Observed: 26 October 2026.

## Delivered boundary

Published main-first merge `4209645b4` on base `2bf48de53` (feature commits
`a83c367a5`, `7bf867af2`, `3ac9f9b39`, `b09654d69`, `c15b4ee38`). A failed server/web
regular memory download keeps one payload-free `DownloadFailure` in the server status,
one session-log entry, the durable activity history and the tester's report
([contract](../contracts/download-failure-diagnostic.md)). The CLI and the System-2
worker were unchanged at this delivery and recorded failed rows without a cause.
The [CLI follow-up](2026-10-26-cli-download-failure-diagnostic.md) subsequently
extended the same diagnostic to download and restore; System-2 remains unchanged.

## Acceptance

- Independent review: BLOCK on one Important finding (the status line named the last
  started step after a backup failure) and ten Minor ones; all fixed in `3ac9f9b39` /
  `b09654d69`. Focused re-review of the fix round: APPROVE; its three Minor wording and
  coverage notes fixed in `c15b4ee38`.
- Maintained change-scoped package gate on merge `47de8efe6` (base `678082047`): 15/15
  stages green, inputs frozen, no touched or unchecked user data. Rust 2,686 passed,
  127 ignored, 14 binaries; corpus 91/91; browser functional 852, exclusive 69,
  production device-download-support 47, none failed. The new close/reopen case ran in
  EN and DE.
- `main` then gained docs-only `2bf48de53` (two UP258 status files). The re-merge
  `4209645b4` differs from the gated merge only in those two files (exact path-scoped
  `git diff --quiet`); documentation, anchors, ledger, diff and local-info checks rerun
  green on it. Rust and browser results carry over by that byte equality.
- Local, tracking and live `main` read back equal to `4209645b4`.

## Not claimed

No hardware run, no repair or explanation of the original field failure, no retry,
restart, restore, timeout or protocol change. An older KNXBench refuses a history page
that holds a recorded cause, as it already does for a read-only check.
