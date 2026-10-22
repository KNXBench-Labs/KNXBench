# WP-04.3 stage C — transitive installer cancellation controls

Claimed 22 October 2026. The first increment of WP-04.3 stage C under
[ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md) and the
[managed-binding contract](../catalogue-jobs/managed-binding.md) section 5
(stage C: shared installer transitive controls), reusing the accepted
cancellation-reader decisions of stage A
([control-admission.md](../catalogue-jobs/control-admission.md)).

## Scope of this increment

Thread one explicit typed `Stop` check through the product-package installer
and its parser pipeline, so that a future controlled apply can stop a
long-running install at every seam. The existing public quiet API is driven
through the same code but with a control that never stops, so its behaviour
is byte-for-byte unchanged. No controlled preview/apply route, Ready consent
or job slot becomes reachable.

- New cancellation control in `knx_productdb::operation`:
  `Cancellation<'a>` with `Quiet` and `Controlled(&'a PublicationGate)`
  variants and a `checkpoint() -> Result<(), Stop>` that `Quiet` never fails
  and `Controlled` forwards to the gate. It is `Copy` and carries no
  allocation, no callback and no SQL.
- Typed cancellation errors: `ProductDbError::Cancelled(Stop)` and
  `PackageError::Cancelled(Stop)`, with `From<Stop>` conversions. `Stop`
  stays the one shared token; no string error, no swallowed stop, no
  resumable half-install.
- Every internal installer seam accepts the control and checks it at the
  named boundaries — both ZIP views, read, hash, decode, skip, identity,
  classification, ingest, report and final admission:
  - `wrapper.rs`: `install_product_file`,
    `install_admitted_product_file`, `install_wrapper` (per inner member).
  - `package.rs` — first ZIP view (streaming `CheckedArchive`):
    `open_checked_archive` (per entry), `read_checked_member` (per member),
    `check_deflate_stream` (per input block), `install_package`,
    `install_package_with_limits`, `install_admitted_package` (per member
    read/hash/classify/ingest and before final admission), `admit_package_bytes`.
  - `package.rs` — second ZIP view (checked central-directory metadata):
    `validated_zip_metadata`, `validated_zip_members`, `validate_central_directory`,
    `preflight_zip` (per entry/record where bounded loops already exist).
  - `ingest.rs`: `ingest_file`, `ingest_file_in_transaction` (hash, skip
    check, classify — checkpoints around the call, `classify` itself stays a
    bounded root-element predicate — each parser call, report and evidence
    writes).
  - Parser entry points (`parse/`): `catalog`, `hardware`, `program`,
    `translation`, `baggage`, `master`, `write_authority`, `scheme_evidence`,
    `dynamic` — each checks at least once per file and at its existing
    bounded loop boundaries.
  - Report/identity seams: `report::insert_unknown`, `report::insert_conflicts`,
    `identity::record_producer`, `identity::record_and_check`,
    `blob::store_source_file` — checked before each write.
- Public quiet wrappers (`install_package`, `install_package_with_limits`,
  `install_product_file`, `ingest_file`) pass `Cancellation::Quiet`; their
  signatures, response shapes, error mappings and schema are unchanged.
- The `pub(crate)` internal entry points (`install_admitted_package`,
  `install_admitted_product_file`, `ingest_file_in_transaction`) gain the
  `Cancellation` parameter so the future controlled apply in
  `managed_catalogue` can drive a cancellable install on an admitted
  transaction without going through a committing quiet wrapper.

## Explicitly out of scope (later increments)

- Any controlled preview/apply route, Ready consent, job slot, upload spool
  or `catalogue_job_control` wiring. Stage D owns the job/lifetime API.
- The W/S/D apply recipe, reservation, backup or commit gate: cancellation
  here only stops the parser/installer before its caller settles, it never
  claims rollback or a committed outcome.
- Resource admission (page/result/record caps) and the complete artifact
  comparator: these belong to the separate artifact/result policy
  ([protocol-artifacts.md](../catalogue-jobs/protocol-artifacts.md)) and are
  wired before any controlled preview/apply is exposed, not afterwards.
- The legacy ETS import pipeline (`legacy/publish.rs`) and the path-based
  update/apply helpers (`update.rs`): stage B already fenced their server
  mutators; their internals are not the product-package installer.

## Done when

- A controlled install can be stopped at each named seam, and a stop leaves
  no partial rows behind (the caller's transaction rolls back exactly as a
  parse error would today).
- `Cancellation::Quiet` drives every existing public wrapper to the same
  result, error and database bytes as before this change.
- Focused tests green and the owning productdb suite passes; no public quiet
  signature, schema or response shape changes; no production controlled
  route becomes reachable.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [managed-binding.md](../catalogue-jobs/managed-binding.md) (authoritative)
- [control-admission.md](../catalogue-jobs/control-admission.md)
- Delivered stage-A primitive: `crates/knx-productdb/src/operation.rs`
  (`PublicationGate`, `Stop`)
