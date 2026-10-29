# Support live read no longer disables activity history — 29 October 2026

## Defect

`POST /api/support/diagnose` (the optional read-only live read of the
[support report](../contracts/universal-device-support.md)) starts a one-shot
activity of kind `supportDiagnostics`. The durable history validator in
`knx-app::commissioning_activity` did not list that kind. With the persistent
history every server build uses, the start row was refused, history was
latched `unavailable`, and every later write admission (device downloads,
System-2 operations, service-control writes) refused with "activity history is
unavailable; not sent" until the server restarted. The live read itself still
succeeded, so nothing pointed at the cause. The volatile `GET /api/bus/activity`
snapshot still carried the in-memory row, which the Web admission rejected as a
whole, so the Live panel showed no snapshot either.

Affected: every release since Alpha 12 (`v0.1.0-alpha.12` to `v0.1.0-alpha.18`),
established by reading the tagged sources, not by rerunning them. Route tests use
the persistent factory but never asserted history health after a live read.

## Fix

`supportDiagnostics` is a first-class read-only kind: added to the storage
validator (target address required, read states only, no write or download
evidence), to the Web `HISTORY_KINDS` admission and to the English and German
kind labels. The kind is not renamed to `deviceDiagnostics`, which names the
separate [diagnostic export](../contracts/device-diagnostic-export.md); history
should say which of the two read the device. No row with the old kind could have
been stored, so nothing needs migrating. Storage/wire format stays 2, the same
additive rule as `deviceDiagnostics`: an older build refuses a page that
contains the new kind ([contract](../contracts/COMMISSIONING_ACTIVITY_HISTORY.md#coverage-and-outcomes)).

## Tests

Red first on the unmodified code, then green:

- `knx-app` `a_support_live_read_is_recorded_without_latching_history_failure`:
  persistent history in a temporary directory; history must stay `configured`,
  write admission must succeed and the row must read back. Red:
  `left: "unavailable"`, `right: "configured"`.
- `knx-server` `http_support_diagnostics::a_live_read_is_durable_history_and_never_blocks_later_writes`:
  a simulated device behind the real router; after the live read, history stays
  `configured`, write admission succeeds, and `GET /api/bus/history` and
  `GET /api/bus/activity` both return the row. Red: same assertion.
- Web: `activityHistory.test.ts` admits the kind in all four read states,
  refuses it without an address or with write evidence, and requires an English
  and German label for every recorded kind; `BusActivityLive.test.tsx` renders a
  live snapshot that contains it. Red: `activity history malformed` and an
  absent live row.

Simulator and intercepted HTTP only; no bus, gateway or device was contacted.
