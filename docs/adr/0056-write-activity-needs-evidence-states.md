# ADR-0056: Write activity needs evidence states, not generic success/failure

- Status: Accepted (2026-09-30)
- Scope: future server-side one-shot write telemetry, not permission to write to KNX hardware

## Context and verified behavior

`GET /api/bus/activity` currently retains bounded read-only comparisons,
serial-number lookups and Debug-gated property reads (ADR-0055). Its generic
`running` / `finished` / `failed` / `unknown` states describe whether a
**read request completed**, not whether a device was changed. Applying these
states to writes without observing the actual write boundary would mislead
both a user and a recovery workflow.

- `serial_address_routes::write` may return HTTP 200 with `wrote: false` when
  the device already has the requested address. A successful changed address
  is read back by serial number. `SerialNumberWriteError::NotVerified` and
  `Session { step: 3 or 4 }` may follow the broadcast write. The HTTP route
  currently persists no pre-write recovery record of the original address
  and serial number; a response carrying `previousAddress` is not durable
  evidence if the request is cancelled. These are observations of the local
  implementation, not a claim about undocumented device storage.
- `service_control_routes::write` may return HTTP 200 with `written: false`
  and no backup because the bit already has the requested value. A changed
  value has a same-session durable, read-back-checked backup of exactly the
  property value and mask **before** `A_PropertyValue_Write` (ADR-0051),
  followed by an exact-octet property readback. A session error after that
  write may leave the device changed but unverified; the backup is recovery
  evidence for this property, not a complete device image.
- `bus_routes::write_value` returns HTTP 200 after `BusSession::send`, which
  delegates to `BusTunnel::send`. It performs no per-device state readback and
  cannot prove that a group-address receiver changed state. Its payload echo
  is a codec check, not a device receipt. A failed or cancelled send can be
  ambiguous from the route alone.

The original simulated HTTP regressions verified that none of these routes
fabricated a generic one-shot success. The service-control route now has the
write-specific follow-up below; serial-address and group writes stay
`untracked`. Two HTTP 200 no-op outcomes remain separately tested.

## Decision

1. Do **not** adapt a write route by merely calling
   `ActionGuard::finish("finished" | "failed")`. For writes, `failed` must
   never imply no mutation, and HTTP 200 must never imply a write occurred.
2. A write observer exposes a typed outcome distinct from read completion.
   Its minimum evidence categories are: `running`, `noChange` (witnessed no
   write send), `notSent` (explicit proof of pre-send refusal),
   `effectUnverified` (the route entered a phase from which a write **may**
   have been sent, but has no device readback), `verified` (the route's exact
   device readback passed), and `unknown` (request dropped without a
   witnessed terminal result). An HTTP 200 or a backup alone never implies
   `verified`. `effectUnverified` does **not** claim a frame reached the bus;
   a transport failure before delivery may still belong there.
3. The write boundary must be observed in the application/protocol path,
   before the await that can transmit. Route-level `Result` mapping alone
   cannot distinguish a pre-send error from a post-send error or cancellation.
   A dropped future defaults to `unknown` unless a witnessed terminal state
   was already recorded. Only the existing detailed action response and its
   backup/readback evidence can justify a stronger category.
4. If a route's write requires a durable pre-write backup, its observer may
   report that backup **only after** persistence and readback succeeded.
   Activity must not contain a host path, credentials, a serial number, raw
   payload or memory/property octets. A future recovery reference must be a
   safe server-issued identifier resolved by a separate guarded endpoint,
   not an invented filename or a substitute for the backup. The full-backup
   policy and action-specific confirmation/opt-in gates remain independent
   prerequisites; telemetry cannot grant write permission.
5. `serviceControlWrite` leaves `untracked` only with the route-specific
   observer and simulated transport/cancellation regressions described below.
   Keep `serialAddress` and `groupWrite` in `untracked` and keep
   `coverage: "partial"`: the serial-address HTTP path needs a durable
   pre-write recovery design before any new live write is authorized. Group
   writes need an honest meaning for gateway acceptance without claiming
   receiver verification or a storage backup.

## Implemented follow-up: service-control write (2026-09-30)

`POST /api/device/service-control` is the first write-specific one-shot
observer. It starts **after** opt-in, target/confirmation/key-plan checks and
tunnel conflict checks; no observer grants those prerequisites. A `WriteGuard`
records no property bytes, mask, serial, project key, gateway or backup path.
Its `writeEvidence` flags are `backupRecorded` and `sendPossible`. Both become
true only after `write_backup` has durably persisted and read back the exact
property value and mask in the protocol's synchronous pre-write callback.
The callback immediately precedes `write_property`, but is **not** proof that
its async send succeeded or was even attempted. Thus a later error is
`effectUnverified`, not a false `notSent` or `verified`; an abandoned future
is `unknown` and retains the flags that were witnessed. A no-op is `noChange`;
a pre-callback error or backup failure is `notSent`; `verified` requires the
existing same-session exact-octet property response. The route response and
on-disk recovery record remain authoritative; the volatile, bounded activity
snapshot is not an audit trail or permission for live hardware work.

The former `with_tunnel!` helper was replaced by one shared reservation
function, holding its three lock guards across connection, action and
disconnect, so reads and writes preserve the same exclusion/lock order while
using distinct completion semantics. Simulated tests cover opt-in refusal,
no-op, backup failure, transport failure before actual delivery **and** after
the simulator observed a property mutation, pre-connect and pre-send
cancellation, and verified readback. Both transport failures produce
`effectUnverified` because the route cannot infer delivery from an error.
`coverage` remains `partial`: serial-address and group writes are still
untracked, and Web UI coverage is not part of this change.

## Follow-up: serial-address write fails closed (ADR-0057, 2026-09-30)

The public CLI and HTTP serial-address write paths now return failure before
opening a tunnel: no complete, durable pre-write recovery for affected device
storage has been established. Earlier simulated HTTP 200/no-op examples above
are historical design evidence, not a currently reachable HTTP write outcome.
`serialAddress` remains `untracked` conservatively; no write activity is
fabricated by the refusal, and the read-only serial lookup is unaffected.

## Consequences and next implementation slice

Do not expose a generic green "completed write" status from the other
routes. The next slices require a durable recovery design for serial-address
writes and honest group-write semantics before changing their live
availability or claiming coverage. No Web change, bus operation or new
hardware authorization is made by this ADR or its follow-up.
