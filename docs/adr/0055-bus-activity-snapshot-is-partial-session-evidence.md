# ADR-0055: The bus-activity snapshot reports only observed sessions

- Status: Accepted (2026-09-30)
- Scope: KNXBench server commissioning activity, not KNX protocol behavior

## Context

The UI needs a status bar for bus actions. The server retains the last
session for device downloads, button-based individual-address programming and
line scans; the bus-monitor session is held only until it is stopped. Other
routes perform one-shot work without a retained session. A global `idle` or
complete activity history inferred from these four holders would be false: an
operation can begin and end between two polls, and a lock held while connecting
or sending does not identify the operation or its target.

## Decision

`GET /api/bus/activity` returns a guarded, read-only server snapshot. It never
opens a tunnel or writes to a device. It reports the session still held per
kind with a typed status and aggregated progress; session IDs are scoped by
`serverIncarnation`. It uses `try_lock` on the existing session holders and
reports any unavailable holder in `busyLocks`, without waiting behind a gateway
timeout or guessing which operation owns the lock. In particular, a held
address-programming lock can belong to a serial-number action or another
management operation, not necessarily button programming.

The response always declares `coverage: "partial"` and names the known
untracked one-shot routes. It does not claim an atomic cross-session snapshot,
chronological ordering of retained sessions, proof the bus is idle, a durable
audit trail, or an action history. Raw telegrams, memory bytes, access keys and
host paths are excluded from the aggregate response. Existing per-action
endpoints remain the authoritative detailed status and recovery evidence.

## Consequences

The UI may show observed session progress, but must not label this endpoint a
complete global activity ledger or infer all actions are absent when its
`sessions` array is empty. A later shared activity registry needs explicit
begin/end instrumentation at *every* bus route, defined lifetime/retention and
an atomic read model before claiming global coverage. No UI code or live bus
operation is part of this decision.

## Follow-up: first one-shot observer (2026-09-30)

The read-only device-compare route is now the first instrumented one-shot
operation. Once target and gateway conflicts are checked, it starts a
server-lifetime activity record before connecting. A witnessed result records
`finished` or `failed`; dropping the request before a result records `unknown`
rather than inventing success. A guard does not own, reopen or write through
the KNX tunnel. Records have monotonically increasing per-incarnation IDs,
target address and UTC start/end times, not a gateway, credentials, memory
blocks or comparison differences.

The in-memory view keeps at most 64 records under normal single-tunnel use,
never evicts a running action, and reports the number of evicted terminal
records in `oneShotDropped`. A server restart loses this view; it must not be
treated as persistent audit or recovery evidence. The other one-shot routes
remain explicitly `untracked`, and `coverage` remains `partial`. The detailed
device-compare response remains the authoritative result.
