# WP-04.3 stage D — managed catalogue stamp + witness binding

Claimed 22 October 2026. First slice of the stage-D "route wiring + worker
execution" increment under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
and the [managed-binding contract](../catalogue-jobs/managed-binding.md)
sections 3 (persistent W observations and ordinary external ABA) and 5 (exact
proposed signatures). It delivers the immutable binding a future
preview/apply compares, without any installer orchestration or HTTP route.

## Scope of this increment

- Expose the delivered live witness as a crate-private capability:
  `CatalogueWitness`, `WitnessError` and `witness_current` become `pub(crate)`
  in `catalogue_schema/witness` (with the `Limit`/`AdmissionError` error
  causes they reference). The live `CatalogueWitness` stream already includes
  `schema_version`; the copy witness (`CopyContentWitness`/`copy_content_witness`)
  stays private for the later copy-validation slice. No public API, quiet
  signature or schema change.
- New `catalogue_stamp` module (`knx-productdb`):
  - `CatalogueStamp { incarnation, generation, data_version, total_changes,
    file, witness }` — the actual W values at one snapshot, never a global
    latest overwritten by a later read.
  - `CatalogueIncarnation` and `FileStamp` are promoted from `pub(crate)`
    implementation details to the stamp's typed fields (still crate-private).
  - `StampError` — typed acquisition failures, preserving
    `NotHealthy`/`Witness`/`Observation`/`Stop`/`Overflow` causes distinctly;
    never a string, never a partial digest.
- `ManagedCatalogue::capture_stamp(&mut self, gate: &PublicationGate)`:
  - Requires `Healthy`, increments **no** generation (a read-only reservation),
    and takes one W `transaction_with_behavior(Immediate)` with
    `DropBehavior::Ignore`.
  - Establishes a real main read, samples `PRAGMA data_version` and the file
    stamp, streams the full live witness on that transaction, then samples
    `data_version` again. Any intra-reservation change or read inconsistency
    refuses (`StampError::Observation`) instead of resetting the baseline.
  - Ends the reservation with one explicit `ROLLBACK`, verifies
    `is_autocommit()` afterwards, and returns the bound stamp. The connection
    is never left with an active transaction; a poisoned guard is not
    introduced for this read-only path.
- No production route, Ready consent, job slot, installer, W/S/D copy or
  commit gate is exercised by this increment.

## Explicitly out of scope (later slices)

- `preview_current_controlled` / `apply_current_controlled` and their
  `OperationContext` / `ControlledSource` / `ControlledPolicy` /
  `CatalogueArtifacts` orchestration.
- `SourceIdentity`, `EffectiveLimits`, `PolicyIdentity`, `PreviewBinding`,
  `ControlledPreview` and `ControlledApplySettlement`.
- Server HTTP route wiring, the worker/supervisor execution, the
  `EmbeddedServerHandle` and desktop exit interception.
- The S/D copy recipe, destination-cookie validation and the artifact owner.

## Done when

- `capture_stamp` compiles with test-first coverage: a stamp binds the exact
  witness, generation, incarnation, `data_version` and file stamp; an
  external write between two captures changes `data_version` and a same-W
  managed write changes `generation` (the complementary ABA detectors, both
  changing the witness when bytes change); a non-Healthy owner refuses; the
  connection is left autocommit after an explicit rollback; a stopped gate
  surfaces a typed `Stop`.
- The live witness still includes `schema_version` (unchanged equality
  semantics, now reachable crate-internally).
- Focused tests green and the owning ProductDB suite passes; no public quiet
  API, schema or response shape changes; no production controlled route
  becomes reachable.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [managed-binding.md](../catalogue-jobs/managed-binding.md) (authoritative, §3 + §5)
- [witness-increment.md](../catalogue-jobs/witness-increment.md)
- [copy-increment.md](../catalogue-jobs/copy-increment.md)
- Delivered primitives: `crates/knx-productdb/src/managed_catalogue.rs`
  (`ManagedCatalogue`), `catalogue_schema/witness.rs` (`witness_current`),
  `operation.rs` (`PublicationGate`, `Stop`).
