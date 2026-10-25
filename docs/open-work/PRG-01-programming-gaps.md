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

- Post-download read-only observation and its CLI, server, web, report and
  history consumers, published as `8b5edad7f`
  ([acceptance](../status/2026-10-25-post-download-check.md),
  [contract](../contracts/post-download-check.md)). Runtime state remains unknown
  and physical-exemplar identity is not verified.

- Backup prior-application identity: a backed-up run records the pre-write object 3/PID 13
  answer (backup format 2) and `restore_plan` refuses a plan of another or unrecorded
  application; legacy partial backups keep their precondition route
  ([contract](../contracts/backup-application-identity.md)). Published as `08db292f3`
  after independent reviews and the maintained 16-stage integrated package gate
  ([acceptance](../status/2026-10-25-backup-application-identity.md)); it does not
  prove whole-device recovery.

- Source-bound programming settlement replans the original request at completion
  and withholds written-memory credit after a selected-plan source change
  ([contract](../contracts/download-settlement-source-freshness.md)). Source
  reviews and all 14 selected integrated package stages passed; published as
  `fa4c18213` ([acceptance](../status/2026-10-25-download-source-freshness.md)).
  Other parts and later catalogue edits are not
  monitored; this does not close PRG-01.

- Region-bound memory reads use one selected service and respect the user-memory
  count limit for backup, comparison and byte-order probes
  ([contract](../contracts/memory-region-read-service.md)). Independent review and
  all 14 integrated package stages passed; published as `a4a4ebcf1`
  ([acceptance](../status/2026-10-25-memory-region-reads.md)).
  This adds no hardware or interface/coupler budget claim.

- Property load-control response-state handling preserves an actual LoadCompleting
  observation and rejects invalid responses before polling
  ([contract](../contracts/load-control-response-state.md)). Independent review
  and all 14 integrated package stages passed; published as `e4f5e7c6f`
  ([acceptance](../status/2026-10-25-load-control-response.md)).
  No inferred silent state, new retry, common timeout or hardware support.

- Memory request/response service pairing now requires the requested service as
  well as its address, without changing the answer ladder or retry policy
  ([contract](../contracts/memory-response-service-pairing.md)). Independent reviews
  and all 14 integrated package stages passed; published as `53400189e`
  ([acceptance](../status/2026-10-25-memory-response-pairing.md)). Synthetic
  nonconforming-peer evidence is not hardware acceptance.

- Declared TypeNumber bound admission refuses unreadable limits instead of silently
  dropping them, without altering stored text or encoding
  ([contract](../contracts/parameter-number-bound-admission.md)). Independent review,
  correction re-review and all 14 integrated stages passed; published as `87d71d4aa`
  ([acceptance](../status/2026-10-25-parameter-number-bounds.md)). Selected corpus
  regression is not an exhaustive compatibility census; parameter policies remain open.

## Open

- Broader target/exemplar identity and independently verified runtime-state
  observation; the delivered identity/load check does not prove either.
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
