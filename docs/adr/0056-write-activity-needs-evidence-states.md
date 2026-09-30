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

Simulated HTTP regression tests deliberately verify that these three write
routes remain named in `untracked` and do not fabricate a generic one-shot
success. They also exercise two HTTP 200 no-op outcomes.

## Decision

1. Do **not** adapt a write route by merely calling
   `ActionGuard::finish("finished" | "failed")`. For writes, `failed` must
   never imply no mutation, and HTTP 200 must never imply a write occurred.
2. A future write observer must expose a typed, versioned outcome distinct
   from read completion. Its minimum evidence categories are: `running`,
   `noChange` (witnessed no send), `notSent` (explicit proof of pre-send
   refusal), `sentUnverified` (send was attempted/accepted without a device
   readback), `verified` (the route's exact device readback passed), and
   `unknown` (request dropped or the boundary cannot be established).
   `sentUnverified` is **not** a success receipt; a send failure can still
   belong there. The labels are a design contract, not an implemented API.
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
5. Keep `serviceControlWrite`, `serialAddress` and `groupWrite` in
   `untracked`, and keep `coverage: "partial"`, until the actual writer for
   each route supplies the required proof. In particular, the serial-address
   HTTP path needs a durable pre-write recovery design before any new live
   write is authorized. Group writes need an honest meaning for gateway
   acceptance without claiming receiver verification or a storage backup.

## Consequences and next implementation slice

Do not expose a generic green "completed write" status from these routes.
The next safe slice is to introduce a write-specific evidence type and test
its phase transitions with a simulated transport and cancellation **before**
wiring any write route. Verify `noChange`, explicit pre-send refusal, a
post-send readback mismatch, and an aborted future separately. Address the
serial-address recovery gap and group-write semantics before changing their
live availability or claiming coverage. No Web change, bus operation or
hardware authorization is made by this ADR.
