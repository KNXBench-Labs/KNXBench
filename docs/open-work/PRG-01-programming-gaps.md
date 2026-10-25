# PRG-01 — Close non-Secure programming gaps

**Status:** In progress. Listed in [OPEN_WORK](../OPEN_WORK.md); per-ID status of record is the [ledger](../status/LEDGER.md).

**Owner:** another session; see the restart snapshot
[`.ai/CURRENT_STATE.md`](../../.ai/CURRENT_STATE.md) before touching shared
commissioning code.

## Task

The owner authorised (25 October 2026) closing existing non-Secure gaps in the
regular download/restore path, one bounded increment at a time, without live
bus or device operations.

## Delivered so far

- Hardware identity response width: Device Object PID 78 needs exactly six
  response octets before backup and mutation, published as `07fe2e67f`
  ([delivery](../status/2026-10-25-hardware-identity-width.md),
  [contract](../contracts/hardware-identity-comparison.md)).
- Partial-application identity response width: the existing object 3/PID 13
  precondition requires exactly five response octets before backup and mutation,
  published as `02c437110` ([contract](../contracts/partial-identity-comparison.md)).
- Scope advice across core, app, CLI, HTTP and web chooses the concrete pending
  scope or prepares nothing, published as `78fd9d780`
  ([contract](../contracts/device-programming-support-guidance.md)).

[Integrated acceptance and publication](../status/2026-10-25-programming-scope-and-identity.md)
record the boundaries separately; these increments do not close PRG-01.

## Open

- Post-download observation, including its CLI, server, web and history consumers.
- Parameter policies: the defining inactive, `Union` and customer-write rules
  are unresolved.
- Complete mutation effects and whole-mutation recovery contracts.

## Boundaries

KNX Secure is excluded. Existing family and recovery gates stay in force; a
partial download never escalates to a full one implicitly. No hardware
support is inferred from software increments.

## Done when (per increment)

Independent review, the maintained integrated package gate and publication
are recorded; hardware acceptance needs its own operation-specific consent
([WP-12](WP-12-field-evidence-and-owner-acceptance.md)).
