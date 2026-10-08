# ADR-0075: Commissioning callers share an application-layer activity lifecycle

- Date: 2026-10-04
- Status: Proposed (all three CLI write callers locally implemented; integrated acceptance pending)
- Scope: Offline caller/client lifecycle coverage; no new KNX protocol behavior

## Context and observed gaps

[ADR-0064](0064-durable-activity-history-is-not-recovery.md) and
[ADR-0067](0067-download-lifecycle-preserves-uncertainty.md) define payload-free
history, durable start/intent and independent device/restart/cleanup evidence.
The published baseline lived in `apps/knx-server/src/one_shot_activity.rs`;
the local Caller candidate extracts it to the application layer.
`knx-app` already depends on `knx-store`, `chrono` and `serde`; it does not depend
on either client or the KNX transport adapter.

The published CLI baseline had these sibling boundaries outside that server lifecycle:

| Caller | Baseline boundary | Evidence lost by the baseline return shape |
| --- | --- | --- |
| `run_device_download` | Opens its tunnel before `device_download::execute`; disconnects separately | Helper returns only `Written`, not restart or recording outcome |
| `run_device_restore` | Uses the same download helper and its own tunnel/cleanup | Restore must not bypass the ordinary-download lifecycle |
| `run_device_service_control` | Opens its tunnel before helper dispatch; property keeper saves original values | Helper returns only a boolean; no durable write-intent journal |

These are source observations, not offline runtime acceptance of these callers.
Read-only CLI observations and long-lived bus sessions need a separate coverage
inventory; this first package must not claim they are already tracked.

## Proposed decision

Move the existing payload-free vocabulary, validation and journal guards into
`knx-app::commissioning_activity`. Keep SQLite admission/schema in `knx-store`.
Keep connection ownership, async execution, recovery payloads, stdout/HTTP
presentation and client-specific diagnostics in their adapters. The application
service must not import server types, HTTP, UI or KNX transport code.

Use minimal typed download witnesses (written, restart, terminal classification)
rather than a server response carrying exception strings. Preserve format-2
field names, closed enum tokens and existing sequence/incarnation identity.
A CLI producer incarnation is distinct from a server process; the legacy
`serverIncarnation` wire field is retained, not silently renamed. Before choosing
an identity generator, inspect existing workspace dependencies and prove
cross-process identity uniqueness with the selected implementation.

CLI execution uses an explicit `--activity-history <path>` option independent
of its project/product databases and immutable recovery files. Confirmed write
mode requires that option and durable storage admission; there is no implicit
current-directory or volatile-only fallback. Read-only mode may omit it, but
must not claim history coverage. Plan-only commands remain offline and do not
open or modify even a supplied activity store, or connect a tunnel.

A write caller must durably persist start **before** requesting its connector.
After immutable original-value backup, it must persist possible-send intent
inside the existing keeper callback, before any device mutation. Backup or
intent persistence failure prevents the mutation. Service-control backup retains
both original `PID_SERVICE_CONTROL` and `PID_DEVICE_CONTROL` before Verify Mode.
The journal is metadata only and cannot substitute for either backup.

Capture the witnessed device outcome before independent cleanup. Recording or
cleanup failure cannot rewrite a witnessed result into `notSent`, nor promote
it to success. Dropped/pending work remains uncertain; process interruption is
not proof of no write. Do not introduce automatic retry, restore or cancellation
that claims a safe abort without an operation-specific recovery contract.

## Required offline verification before acceptance

1. Preserve every existing server lifecycle/schema regression while extracting
   the service; compare the serialized format-2 contract, not just compilation.
2. Real CLI entry-point refusal with an unavailable/foreign history store occurs
   before the injected connector is called. No real gateway is contacted.
3. Download and restore share the same backup/intent gate; failing either gate
   produces zero device-changing transport events with original files retained.
4. Service-control keeps both original properties before Verify Mode; either
   backup or journal failure prevents every property write.
5. Terminal recording failure, cleanup error/panic, and pending-runtime drop
   preserve independently witnessed outcome and uncertainty without raw payloads
   or host paths in history.
6. Read-only history/query and plan-only CLI commands cannot create bus traffic
   or authority to retry/write. Test ordinary output and failure classifications.
7. Audit remaining clients and long-lived session callers explicitly; keep
   partial coverage visible until their own contracts and tests are implemented.

Tests precede production changes. Compilation-only or a missing-module RED is
not behavioral evidence. Acceptance requires named runtime assertions, relevant
server/CLI regressions, current integrated gates and publication readback.

## Limits

The local implementation is not yet accepted or published, does not close
SAFE-03/AUDIT-01 and does not
change the device-specific fail-closed recovery boundaries. New hardware,
power-loss, vendor and ETS experiments are accepted out of the goal per the
[user scope decision](../archive/alpha-0.1/goal-commission.md#user-scope-decision--2026-10-04).
Their absence remains a user notice, not a pending operator task. No compatibility,
certification, universal recovery or native-package approval follows.
