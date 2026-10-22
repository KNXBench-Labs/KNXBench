# WP-04.3 stage D — preview binding composition

Claimed 22 October 2026. Third slice of the stage-D "route wiring + worker
execution" increment under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
and the [managed-binding contract](../catalogue-jobs/managed-binding.md)
section 5 (the `PreviewBinding` type). It composes the three already-delivered
identity primitives — `CatalogueStamp`, `SourceIdentity` and `PolicyIdentity` —
into the immutable binding the server retains between preview and apply. The
`preview_current_controlled` / `apply_current_controlled` orchestration (and
the `ControlledPreview` / `AdmittedPreviewArtifact` / `PreviewIdentity` types it
requires) are later slices.

## Scope of this increment

- New `preview_binding` module (`knx-productdb`, private, `#[allow(dead_code)]`):
  - `BINDING_VERSION: u32 = 1` — the binding format version.
  - `PreviewBinding { version, catalogue, source, policy }` — an immutable
    composition of the `CatalogueStamp` (from `capture_stamp`), the
    `SourceIdentity` and the `PolicyIdentity` (both from the `controlled`
    admission projections).
  - `PreviewBinding::bind(catalogue, source, policy) -> Self` — an infallible
    constructor; the inputs are already-admitted values, so composition
    performs no second hash and no re-admission.
- Exact equality (`PartialEq`/`Eq`, plus `Debug`/`Clone`): any committed
  catalogue change (via `generation`, `data_version` or the full witness
  inside the stamp), any difference in source name/length/digest, or any
  different policy yields a different binding. This is what makes an
  apply-time comparison stale, not a durable commit history or a partial
  summary.
- The effective limits are carried by `policy` (`PolicyIdentity` already
  projects `max_member_size` and `max_expanded_size`); no separate
  `EffectiveLimits` field is duplicated.

## Explicitly out of scope (later slices)

- `ControlledPreview`, `AdmittedPreviewArtifact`, `PreviewIdentity`,
  `ControlledApplySettlement`, `OperationContext`, `CatalogueArtifacts` and the
  `preview_current_controlled` / `apply_current_controlled` orchestration.
- The W/S/D copy, the rehearsal install, the commit gate and settlement.
- Server HTTP route wiring and the worker/supervisor.

## Done when

- `bind` composes the stamp, source identity and policy identity exactly, and
  the binding pins `BINDING_VERSION == 1`.
- Equality is exact and each field discriminates: identical inputs bind equal;
  a different source name or source bytes, a different policy, or a committed
  catalogue change (managed same-W write consuming a generation) each produce a
  different binding.
- Focused tests green and the owning ProductDB suite passes; no public quiet
  API, schema or response shape changes; no production controlled route becomes
  reachable.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [managed-binding.md](../catalogue-jobs/managed-binding.md) (authoritative, §5)
- [controlled-source-increment.md](controlled-source-increment.md) (prior slice)
- [stamp-binding-increment.md](stamp-binding-increment.md) (prior slice)
- Existing primitives: `crates/knx-productdb/src/catalogue_stamp.rs`
  (`CatalogueStamp`), `crates/knx-productdb/src/controlled.rs`
  (`SourceIdentity`, `PolicyIdentity`).
