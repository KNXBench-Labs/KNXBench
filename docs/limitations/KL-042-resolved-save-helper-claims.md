# Resolved save-helper claims

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-042)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 42. `command_sync.rs`'s module doc overstates its own role — pre-existing, not introduced by T12

**Resolved by AR04.** The exported compatibility helper explicitly uses the
existing transactional whole-project-save fallback. Its former successful no-op
arms and row-only persistence claims are removed; adjacent low-level helper
comments and tests no longer imply a live incremental engine. Current production
save paths remain complete saves and do not call this helper.

The historical description moved verbatim to [resolved limitations](../history/KNOWN_LIMITATIONS_resolved.md#42-command_syncrss-module-doc-overstates-its-own-role--pre-existing-not-introduced-by-t12) (AR14D D5).
<!-- END retained catalogue record -->

<a id="earlier-resolved-description"></a>

<!-- BEGIN earlier resolved record -->
## 42. `command_sync.rs`'s module doc overstates its own role — pre-existing, not introduced by T12

**Resolved by AR04.** The exported compatibility helper explicitly uses the
existing transactional whole-project-save fallback. Its former successful no-op
arms and row-only persistence claims are removed; adjacent low-level helper
comments and tests no longer imply a live incremental engine. Current production
save paths remain complete saves and do not call this helper.

**Evidence.** A previously unsupported parameter edit failed behaviorally before
the fix. File-backed regressions verify nested structural batches, exact reopened
models/high-water marks, undo/redo, middle-sibling order, unchanged opaque and
manufacturer rows, and a late SQL failure retaining the prior durable state.
Three compiled behavioral mutants were caught and all touched source hashes
restored. See [the storage contract](../history/../contracts/STORAGE_COMMAND_CONTRACT.md) and the AR04
receipt in `.ai/logs/2026-10-01_codex_alpha-storage-contract.md` for final gates
and publication; a planned gate is not a passing result.

**Retained boundary, not this former defect.** No incremental-performance claim,
automatic in-memory/history rollback, combined opaque/model transaction or new
multi-user conflict policy is introduced. Use `save_project_if_unchanged` for
the existing expected-state contract. UI/editor and commissioning scopes remain
with their original owners. This heading/fragment is a historical waypoint.
<!-- END earlier resolved record -->
