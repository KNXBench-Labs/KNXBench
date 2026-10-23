# WP-04.3 stage D — preview/apply orchestration

Claimed 22 October 2026. The closing stage-D slice under
[ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md) and the
[managed-binding contract](managed-binding.md) sections 5–7. It composes the
already-delivered primitives — `capture_stamp` (immutable catalogue stamp +
full live witness), `copy_pinned_current` (bounded W/S/D copy with the fresh
zero-to-one destination-cookie rule), the shared noncommitting installer
(`install_admitted_product_file`, already `Cancellation`-aware), the typed
commit gate (`PublicationGate`) and the complete `PreviewIdentity` codec —
into the two orchestration entry points the server worker later calls:

- `ManagedCatalogue::preview_current_controlled` — a **read-only** rehearsal
  that changes no live catalogue bytes and returns a
  `ControlledPreview` (a `PreviewBinding` the server retains plus the full
  immutable rehearsal result).
- `ManagedCatalogue::apply_current_controlled` — the live, stale-checked,
  commit-gated install that returns a `ControlledApplySettlement`.

Neither is wired to a server route, worker or job. Construction-only, mirroring
the prior stage-D increments.

## Scope of this increment

- New `orchestration` module (`knx-productdb`, private, `#[allow(dead_code)]`):
  - `OperationContext<'a> { gate: &'a PublicationGate, policy: ControlledPolicy }`
    — the shared cancellation gate and the effective limit policy.
  - `CatalogueArtifacts` — a minimal trait the orchestrator uses to allocate a
    fresh, equal-page-size rehearsal destination and a fresh recovery
    destination. The server's `ArtifactRoot` (already delivered) implements it
    later; this increment defines the seam, not the filesystem ownership.
  - `ControlledPreview { preview: AdmittedPreviewArtifact, binding: PreviewBinding }`.
  - `AdmittedPreviewArtifact { identity: PreviewIdentity, install: ProductFileInstall }`
    — the full immutable raw rehearsal result and its exact identity; the safe
    public projection is a later server-side concern (P6).
  - `OrchestrationError` — typed failures (not healthy, sqlite, copy, install,
    preview identity, stop, source/policy disagreement). Never a string or a
    partial digest.
  - `ControlledApplySettlement { failure, commit, rollback, health, artifacts }`
    with `CommitEvidence { NotCommitted, Committed, Unknown }`,
    `RollbackDisposition` and `ArtifactDisposition`.
- `PreviewBinding` gains a `preview: PreviewIdentity` field and `BINDING_VERSION`
  bumps to 2; `bind` now takes the preview identity.
- The reopened-D `copy_content_witness` equality (section 4 copy validation) is
  a later slice; `copy_content_witness` stays private.

### Preview recipe (managed-binding §6)

`preview_current_controlled(source, op, artifacts)`:

1. Require `Healthy`, checkpoint, and refuse a `source.limits` that disagrees
   with `op.policy.limits()`.
2. `capture_stamp` (W IMMEDIATE reservation + full live witness, explicit
   rollback, autocommit verified). This is the **original acquisition stamp**.
3. Open a distinct read-only S (`open_read_only`) to the managed path, pin a
   real read snapshot (`BEGIN` + a main-schema read).
4. Allocate a fresh equal-page-size D via `artifacts`, copy S→D with
   `copy_pinned_current` (observed Done + cookie 0→1). The explicit reopened-D
   `copy_content_witness` equality against S is deferred to the later
   copy-validation slice; the pinned byte-exact equal-page copy and the cookie
   0→1 receipt already guard this lane.
5. Run the shared noncommitting installer on D with
   `Cancellation::Controlled(op.gate)`, then `preview_identity` on the result.
6. End S's read transaction, close S, and return the binding composed from the
   original stamp + `SourceIdentity` + `PolicyIdentity` + `PreviewIdentity`.

No W row changes; no generation increment; no live DML.

### Apply recipe (managed-binding §6/§7)

`apply_current_controlled(source, shown, op, artifacts)`:

1. Re-verify the retained binding version and complete effective policy against
   the current operation, then the source's exact name bytes, length and digest
   against `shown.source`, and `source.limits` against `op.policy.limits()`.
   Any difference is a refusal before copy or live work.
2. Acquire one W IMMEDIATE reservation and capture the **unchanged original**
   incarnation/generation/data_version/total_changes/file/witness under it.
   Compare against `shown.catalogue`; a difference is `stale` — no recovery
   backup, no live install. Retain this same reservation through copying,
   live DML and settlement; the standalone `capture_stamp` helper's released
   reservation cannot serve as apply admission.
3. With W still reserved, open S, allocate fresh D and B, copy both from the
   unchanged S (observed Done + cookie rule each), close S before any W DML.
4. Under the retained W IMMEDIATE transaction, arm the managed mutation guard
   (checked generation increment, `Mutating`, quarantine-on-unwind), run the
   shared noncommitting installer on W, compute `preview_identity` of the live
   result and require exact agreement with `shown.preview`.
5. Commit gate: `try_claim_commit` wins → one live `COMMIT`; a latched
   cancellation wins → explicit `ROLLBACK`. No second retry, no reopen-to-decide.
6. Classify settlement from the complete trace: `Committed` only for a
   successful COMMIT with autocommit=true; `NotCommitted` for no issued COMMIT
   or a proved noncommitting settlement; `Unknown` for ambiguous COMMIT
   outcomes. Rollback disposition is independent: no issued COMMIT does not
   prove successful rollback. Failed/unresolved rollback quarantines W and
   retains any recovery artifact even when commit evidence is `NotCommitted`.
   Never report `Explicit` or authorize disposal from an unchecked rollback.

`apply_current_controlled` returns a `ControlledApplySettlement` (it never
panics on a business refusal); the caller inspects the settlement to decide
recovery-backup retention and worker disposition.

## Explicitly out of scope (later slices)

- The server `CatalogueArtifacts` implementation (`ArtifactRoot` filesystem
  ownership, lease, inventory, sidecar adoption, sync) — P8.
- The safe public NDJSON projection and artifact records — P6.
- The server HTTP route wiring, the `POST …/apply` worker/supervisor and the
  `GET …/details` reader.
- `EmbeddedServerHandle` and the desktop/headless drain — P9.
- The live W growth cap (`max_page_count`) and the 64 GiB main-growth
  reassertion described in protocol-artifacts P5; those are asserted when the
  worker wires the live install, not a new parser/limit here.
- Whole raw source `Vec` streaming; the existing bounded whole-`Vec` installer
  remains the shared body (P5 note: first implementation retains the whole
  bounded raw `Vec`).

## Done when

- `preview_current_controlled` rehearses on D, never mutates W, and returns a
  binding whose `preview` identity equals a re-derived `preview_identity` of
  the same install; a committed catalogue change between two previews yields a
  different binding (stale discriminator).
- `apply_current_controlled` refuses a source that differs from the shown
  binding in name/length/digest or a catalogue that changed since preview
  (`stale`), without writing; a matching apply installs once and returns
  `Committed`; a latched cancellation rolls back and returns `NotCommitted`.
- Focused tests green, the owning ProductDB suite passes, warning-denied
  Clippy, `cargo fmt` and header checks clean. No public quiet API, schema or
  response shape changes; no production controlled route becomes reachable.

## References

- [ADR-0150](../../adr/0150-cancellable-catalogue-jobs.md)
- [managed-binding.md](managed-binding.md) (authoritative, §5–§7)
- [preview-identity-increment.md](preview-identity-increment.md) (prior slice)
- [preview-binding-increment.md](preview-binding-increment.md) (prior slice)
- [stamp-binding-increment.md](stamp-binding-increment.md) (prior slice)
- [copy-increment.md](copy-increment.md) (prior slice)
