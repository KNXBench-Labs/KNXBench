# ADR 0186: The download-report summary keeps the run-state reason

Date: 2026-11-21
Status: Accepted
Session: PRG-01-N9

## Context

The CLI and the web view state why a run state stayed `unknown` (undefined value,
unavailable read, candidate mismatch, not qualified). The report summary, which
testers paste into public issues, printed only `application running: unknown`, so
the reason existed in three consumers but not in the fourth. The report file also
read an explicit `postDownloadCheck: null` as an absent check, unlike the activity
history, which refuses `null`.

## Decision

- `runtime_summary()` keeps the recorded reason beside an unknown runtime:
  `application running: unknown (no defined run state observed; run-state read: …)`.
  A record without a continuation keeps the legacy text exactly.
- The report file keeps reading `null` as absent: that only ever yields "not
  checked", never proof (reconcile refuses it), and existing report producers
  rely on it. This is pinned as a characterisation, not widened or narrowed.
- The report file is pinned as a third reader of the shared web/Rust case table
  (28 accepted, 130 refused): any malformed or contradictory check fails the
  whole report parse instead of degrading to "not checked".

## Consequences

Only wording of one summary clause changes; no wire format, no new claim. Facts
stay as before: the reported property is not device health or download success.
Hardware behaviour is untested; this is simulator/fixture evidence only.
