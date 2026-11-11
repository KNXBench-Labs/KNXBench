# WP-32 — Download dialog: warn about pending changes outside the chosen scope (#39)

External report by the issue reporter, 10 November 2026; triaged 11 November 2026.
Status: **implemented 11 November 2026** (warning in `DownloadScopeAdvice`,
`uncoveredParts` in `downloadScopeAdvice.ts`). Owner decision 11 November 2026:
a warning is sufficient, no extra confirmation.

## Requested outcome

When the selected scope does not cover everything the project marks as
pending for the device (for example parameters only while a group-address
change is pending), the dialog names the uncovered part before start. The
warning derives from the same pending flags as the device-list hint and the
existing scope advice (`DownloadScopeAdvice`, `downloadScopeAdvice.ts`).
Starting stays possible.
