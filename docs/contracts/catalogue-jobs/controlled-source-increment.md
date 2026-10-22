# WP-04.3 stage D — controlled source + policy identity

Claimed 22 October 2026. Second slice of the stage-D "route wiring + worker
execution" increment under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
and the [managed-binding contract](../catalogue-jobs/managed-binding.md)
section 5 (the `ControlledSource` / `SourceIdentity` / `PolicyIdentity` /
`EffectiveLimits` types and the source admission that authenticates a sealed
source). It delivers the binding's identity inputs; the `PreviewBinding` /
`ControlledPreview` composition and the preview/apply orchestration are later
slices.

## Scope of this increment

- New `controlled` module (`knx-productdb`, private, `#[allow(dead_code)]`):
  - `ControlledPolicy { Standard, Large }` with `limits() -> PackageLimits`
    (reusing the existing `PackageLimits` STANDARD/LARGE constants; no new
    limit constants).
  - `ControlledSource<'a> { name, bytes, sha256, limits }` — a bounded borrow
    authenticated from a sealed source, never a client path.
  - `SourceIdentity { name, length, sha256 }` — exact name bytes, actual byte
    length and the digest of the actual bytes, not a claimed hash or length.
  - `PolicyIdentity { version, max_member_size, max_expanded_size }` — the
    effective policy values actually admitted.
  - `ControlledError` — typed admission failures: an invalid name (with the
    exact rule violated), an over-limit source, and a digest/length mismatch;
    never a string-only refusal and never a truncated prefix.
- `ControlledSource::admit(name, bytes, policy) -> Result<Self, ControlledError>`:
  - Validates the source name against the P3 grammar: 1..512 UTF-8 bytes,
    basename only — rejects `/`, `\`, `:`, NUL, Unicode Cc/Cf controls, the
    exact names `.` and `..`, and leading/trailing ASCII whitespace. It never
    silently basenames or renames.
  - Requires `bytes.len() <= MAX_RETAINED_BYTES`; an empty source is
    syntactically admitted (the installer's package refusal stays a later,
    separate refusal).
  - Computes SHA-256 over the actual bytes once; `SourceIdentity` and the
    retained `sha256` are derived from that one digest, never a second hash.
- `SourceIdentity::from_source` and `PolicyIdentity::from_policy` are the only
  constructors; both are infallible projections of already-admitted values.

## Explicitly out of scope (later slices)

- `PreviewBinding`, `ControlledPreview`, `ControlledApplySettlement`,
  `OperationContext`, `CatalogueArtifacts` and the `preview_current_controlled`
  / `apply_current_controlled` orchestration.
- The W/S/D copy, the rehearsal install, the commit gate and settlement.
- Server HTTP route wiring and the worker/supervisor.

## Done when

- `admit` compiles with test-first coverage: each rejected name rule (separator,
  colon, NUL, control, `.`/`..`, leading/trailing whitespace, overlong, empty)
  is refused with the exact rule; a valid basename is admitted; a source over
  `MAX_RETAINED_BYTES` is refused; the identity's `length`/`sha256` are the
  actual bytes' values and the digest is stable across identical bytes; both
  policies project the exact existing STANDARD/LARGE limits.
- Focused tests green and the owning ProductDB suite passes; no public quiet
  API, schema or response shape changes; no production controlled route
  becomes reachable.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [managed-binding.md](../catalogue-jobs/managed-binding.md) (authoritative, §5)
- [protocol-artifacts.md](../catalogue-jobs/protocol-artifacts.md) (P3 name grammar, P2)
- [stamp-binding-increment.md](stamp-binding-increment.md) (prior slice)
- Existing `PackageLimits`: `crates/knx-productdb/src/package.rs`
- Existing ceiling: `crates/knx-core/src/limits.rs` (`MAX_RETAINED_BYTES`)
