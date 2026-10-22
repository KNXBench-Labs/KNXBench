# WP-04.3 stage D — complete preview identity

Claimed 22 October 2026. First slice of the stage-D "preview/apply
orchestration" increment under [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
and the [managed-binding contract](../catalogue-jobs/managed-binding.md)
sections 4 and 5 (the `PreviewIdentity` type). It implements the *complete
preview identity* — the versioned typed encoding over every admitted field of
one completed rehearsal install result — that the preview orchestration
computes and that apply later compares for exact preview/report agreement.

The `preview_current_controlled` / `apply_current_controlled` orchestration
(and the `ControlledPreview` / `AdmittedPreviewArtifact` types, the W/S/D copy
composition and the `PreviewBinding.preview` field) are later slices.

## Scope of this increment

- New `preview_identity` module (`knx-productdb`, private, `#[allow(dead_code)]`):
  - `PREVIEW_IDENTITY_VERSION: u32 = 1` — the encoding format version.
  - `PreviewIdentity { version, sha256, package_count, other_member_count,
    encoded_bytes }` — the digest plus exact scan accounting of one install
    result. It is *not* a `PartialEq`-free summary: `sha256` is the equality
    discriminator, the counts are exact bookkeeping, never a redacted
    projection.
  - `preview_identity(install: &ProductFileInstall) ->
    Result<PreviewIdentity, PreviewIdentityError>` — a deterministic, checked,
    lossless SHA-256 stream over the complete result.
- The stream is a distinct versioned domain
  `KNXBench/PreviewIdentity/1` + NUL + `u32` BE version, followed by explicitly
  tagged records. Every field is encoded:
  - `ProductFileInstall.sha256` (exact bytes) and `wrapper` (explicit null for
    a standalone package, else every `other_members` member in order with its
    `path`/`size`/`sha256`/`kind`).
  - Every `packages` entry **in archive order** with its optional `path`
    (explicit null) and its full `InstallReport`: `sha256`, `scheme`, `skipped`,
    every `members` member (`path`/`role`/`sha256`/`size`), `unknown`, every
    `conflicts` entry (`table`/`id`/`kept_sha256`/`other_sha256`/`occurrence`),
    the four `translations` counts, `dropped_datapoint_types`, `source_names`,
    and the optional `facts` and `baggage` subtrees with explicit nulls.
  - `facts` (when present): every `counts` entry
    (`category`/`disposition`/`count`), every `unknown_constructs` entry
    (`xpath`/`kind`/`name`/`occurrences`/explicit-null `sample`),
    `unknown_occurrences`, and every `diagnostics` entry
    (`kind`/`archive_path`/`xml_path`/`detail`/`occurrences`).
  - `baggage` (when present): every `declarations` entry
    (`index_path`/`ordinal`/the six declaration attributes with explicit
    nulls/`resolution`/explicit-null `member_path`/explicit-null `detail`) and
    every `payloads` entry
    (`member_path`/`sha256`/`size`/`media_class`/`nested` archive facts with
    explicit `Read` variant/`declarations`).
- Enumerations (`WrapperMemberKind`, `InstallCategory`, `InstallDisposition`,
  `InstallDiagnosticKind`, `UnknownKind`, `MediaClass`, `NestedArchive`,
  `Resolution`) are emitted as one fixed discriminant tag via an explicit
  `match`, never `as_str()` or `Debug` formatting, so the mapping is pinned by
  the versioned codec and documented in the module.
- All arithmetic is checked; lengths are `u64`-framed; a `Vec` is `u64` length
  then elements; a `bool` is a single tag byte; `Option` is an explicit
  none/some tag. No `format!`, no `Debug`/serde coercion, no delimiter
  concatenation, no first-N filtering, no field omission.

## Explicitly out of scope (later slices)

- `ControlledPreview`, `AdmittedPreviewArtifact`, `OperationContext`,
  `CatalogueArtifacts`, `ControlledError`, `ControlledApplySettlement` and the
  `preview_current_controlled` / `apply_current_controlled` orchestration.
- The W/S/D copy composition, the rehearsal install, the commit gate and
  settlement classification.
- The `PreviewBinding.preview` field (the binding gains it when the
  orchestration slice computes a `PreviewIdentity` from a real rehearsal).
- The immutable source-artifact digest/count is carried by the binding's
  existing `SourceIdentity` (`sha256` + `length`); this codec is the
  install-*result* identity, distinct from the source identity.
- Server HTTP route wiring and the worker/supervisor.

## Done when

- `preview_identity` is deterministic: identical `ProductFileInstall` values
  encode to equal digests, and every field discriminates — a different source
  `sha256`, scheme, `skipped` flag, member, conflict, translation count,
  dropped datapoint type, source name, fact, diagnostic, baggage declaration
  or payload each yields a different digest.
- Explicit nulls are encoded: a standalone package (`wrapper == None`), a
  package with no path, a missing `facts`/`baggage`, a `None` sample,
  `member_path` or `detail` each remain distinguishable from their empty
  `Some` counterparts and from an absent field.
- Ordering is preserved: packages in archive order, members, conflicts, counts,
  diagnostics, declarations and payloads in their stored order.
- Focused tests green and the owning ProductDB suite passes; no public quiet
  API, schema or response shape changes; no production controlled route becomes
  reachable.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [managed-binding.md](../catalogue-jobs/managed-binding.md) (authoritative, §4/§5)
- [preview-binding-increment.md](preview-binding-increment.md) (prior slice)
- [stamp-binding-increment.md](stamp-binding-increment.md) (prior slice)
- Existing types: `crates/knx-productdb/src/wrapper.rs` (`ProductFileInstall`,
  `InstalledPackage`, `WrapperContents`, `WrapperMember`, `WrapperMemberKind`),
  `package.rs` (`InstallReport`, `InstallFacts`, `InstallCount`,
  `InstallDiagnostic`, `InstallCategory`, `InstallDisposition`, `PackageMember`,
  `TranslationCounts`), `report.rs` (`IdConflict`, `UnknownConstruct`,
  `UnknownKind`), `baggage.rs` (`BaggageInventory`, `BaggagePayload`,
  `ResolvedDeclaration`, `Resolution`, `MediaClass`, `NestedArchive`),
  `parse/baggage.rs` (`BaggageDeclaration`).
