# WP-25 — Import findings missing from the session log after reopening

Owner report: 2 November 2026. Owner Go for implementation 2 November 2026
(supersedes the earlier recorded-only status). Implemented and independently
reviewed (APPROVE); scoped package gates, main-first merge and publication pending.

## Observed

When an older project is reopened whose import warnings were shown before,
clicking the warning counter no longer shows them in the session log.

## Cause (reproduced)

The counters come from the import report persisted with the project
(`ProjectTree.errors`/`warnings`), while the `import:*` session-log entries are
written only at import time and live for one server run
([`importFindings.ts`](../../apps/knx-web/src/importFindings.ts)).

## Requested outcome

Reproduce and confirm the cause, then decide between re-projecting the
persisted findings into the log on open or a dedicated findings view. A counter
must never link to an empty list without saying why.

## Decision and result

Reproduced by `import_restore_tests` (server): reopening a native file with a
stored report gave counters `(1, 4)` but no `import:*` entries in the log.
Chosen: re-project on open, because the counters, filter, search and export
already live in the session log and a second findings view would duplicate
them. `historical_import_log::project` rebuilds the counted entries from
the stored report JSON (same grouping as a fresh import; a test pins the two
projections equal), and `open_native_project` adds them plus one info line
saying they were restored at opening. Their timestamps are the time of
opening: the stored report carries no import time, so none is invented, and no
`import` summary line is replayed. Opening resets the log, so a current import
cannot be duplicated. Informational notes (opaque, observations) are not
restored; they stay in the full report. A project without a stored report gets
no entries and, with zero counters, no counter link. If the log cap (16,000)
drops findings, the scope note still names the mismatch.

### Repair after the design review

- The projection lives in `historical_import_log.rs`, built from the raw stored
  JSON, which stays untouched. Admission only validates the counter witnesses,
  so a row may lack display fields (even `{}`). Missing or unreadable fields show
  "not recorded" (never 0, `null` or an empty text) and the original row is kept
  in the entry's detail.
- A manufacturer group's occurrence total uses checked `u64` addition; a missing,
  non-integer or overflowing member withholds the total ("not recorded").
- The projection is checked against the admitted counts before the log cap can
  evict anything. A disagreement lists nothing and logs a warning that names it;
  the counters and the full report stay authoritative.
- It is bound to the loaded (recovered) working image and appended only after the
  replacement succeeded: a refused open (unsaved edits) changes neither project nor log.
- The focused log scope says, in EN and DE, that the findings are restored at
  opening (not freshly imported, times are the opening) and offers the full-report
  action (`ImportEvidencePanel`), also on a cap or count mismatch.
- Not tested: the 16,000 cap interplay beyond one error-only report; the
  fallback wording is English in the server log, as all server log text is.
