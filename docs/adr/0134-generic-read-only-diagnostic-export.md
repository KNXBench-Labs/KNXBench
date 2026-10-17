# ADR 0134: Read-only diagnostic export uses one generic identity recipe

Date: 2026-10-17
Status: Accepted (offline/simulator scope; no hardware acceptance)
Session: device-diagnostic export

## Context

Reporters often cannot run ETS, a paid device reader or developer tooling, but
an investigation needs a few original device replies. The existing universal
support report ([ADR-0131](0131-universal-device-support.md)) deliberately
withholds raw bytes and limits live reads to reviewed family recipes; unknown
or absent masks stay descriptor-only (its decision 6). Download, compare and
byte-order probes need an executable plan, which an unsupported device
precisely lacks.

Verified facts this decision relies on, all already cited in the repository:

- Device Descriptor Type 0 is read by every management procedure (`read_mask_version`).
- `PID_MANUFACTURER_ID` (12) of the Device Object is read by CP §3.5.2 step 03
  in every load procedure, independent of the device family
  (`knx_core::commissioning::properties`).
- `PID_HARDWARE_TYPE` (78 decimal, six octets) is the optional Device Object
  capability check of the same step; Resources §4.3.28 (ADR-0131).
- AL §3.4.4.2: a zero-element property response means *object/property absent
  or insufficient access*; the three causes are not distinguishable.

Assumption, explicitly not verified: devices without property services answer
a property read with silence or a zero-element response, not a state change.
No hardware has exercised this recipe.

## Decision

1. A separate export, `generic-identity-v1`, performs exactly: connect,
   descriptor type 0, Device Object PID12, Device Object PID78 (index 1, one
   element each), disconnect. It runs for every selected device, independent of
   catalogue data, mask family or download admission. This narrows ADR-0131
   decision 6 **only for this export**: these two properties are generic
   Device Object properties, not a borrowed family mapping. The support report
   keeps its existing descriptor-only rule and recipes unchanged.
2. The session is `ManagementSession::read_only` with `AuthorisationPlan::Skip`:
   no key, authorisation, Verify Mode, memory/property write, restart, reset
   or group telegram. Simulator tests assert the exhaustive outgoing service
   set, including setup, failure and cancellation paths.
3. Identity asserted by project/catalogue data is compared before further
   reads. A conflict, or an unverified descriptor, stops the sequence. Agreement
   is not a product/application match and never grants programming.
4. **No memory is read.** No independently admitted safe memory profile
   exists; product source offsets are not runtime addresses. `memoryStatus` is
   always `not-requested` in format 1. A future profile needs its own evidence
   and decision.
5. Results are loss-aware per item (`read`, `partial`, `refused-ambiguous`,
   `error`, `not-read`) with raw bytes, received length, returned element
   count, timestamps and error token. Cancellation and later failures keep
   earlier bytes; refusal is never an empty success or zeros.
6. One application service (`knx_app::device_diagnostics`) serves HTTP and the
   CLI; neither has its own protocol path. The archive is a fixed four-member
   ZIP with SHA-256 manifest, create-only, owner-only (0600) and read back before
   success is reported. It carries an opaque random selection reference instead
   of project identity.
7. Gateway exclusivity reuses the server's tunnel locks in their established
   order and adds one per-user nonblocking file lease shared by the CLI and
   server diagnostic paths. Busy refuses; it never queues or forces a second
   tunnel.

## Alternatives considered

- **Extend the support report with raw bytes.** Rejected: ADR-0131's privacy
  envelope explicitly excludes raw replies, and its consumers rely on that.
- **Descriptor only for unknown masks.** Rejected for this export: the
  descriptor alone cannot distinguish manufacturers, which is the minimum
  identity an investigation needs; PID12 is read by every load procedure anyway.
- **Bounded memory reads from product source segments.** Rejected: source
  offsets are not proven runtime addresses, and configuration memory can hold
  sensitive data. Collecting first and redacting later is not acceptable.
- **Reuse the byte-order probe / compare.** Rejected: both require an
  executable download plan.

## Consequences

- `knx-app` gains a normal dependency on `knx-net` (already in the workspace);
  `knx-mcp` still cannot reach either (layering check).
- Every device can produce an honest bounded export; most exports will be
  partial, which is the intended, documented outcome.
- Hardware behaviour (especially for devices without interface objects) stays
  unverified until a separately authorised read-only test on a named device.
