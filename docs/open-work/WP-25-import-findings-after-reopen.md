# WP-25 — Import findings missing from the session log after reopening

Owner report: 2 November 2026. Recorded only; not claimed, no implementation Go.

## Observed

When an older project is reopened whose import warnings were shown before,
clicking the warning counter no longer shows them in the session log.

## Suspected cause (unverified)

The counters come from the import report persisted with the project
(`ProjectTree.errors`/`warnings`), while the `import:*` session-log entries are
written only at import time and live for one server run
([`importFindings.ts`](../../apps/knx-web/src/importFindings.ts)).

## Requested outcome

Reproduce and confirm the cause, then decide between re-projecting the
persisted findings into the log on open or a dedicated findings view. A counter
must never link to an empty list without saying why.
