# Overrides outside the active object tree

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-173)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 173. Instance overrides outside the active object tree: retained and reported

**Repair 2026-10-10.** The import-integrity package (`d8346579`, integrated
acceptance `367405f1`) reports instance overrides not consumed by the declared
active object tree and preserves both original project XMLs byte-exact through
native Save/Reopen/Re-Save. It does not invent active communication objects or
apply those overrides to program defaults. Conflicting duplicate overrides are
reported as errors rather than resolved by a last-writer guess.

**Evidence.** Synthetic `orphan_object_overrides_are_reported_not_silently_lost`
and `conflicting_object_overrides_are_not_resolved_by_last_winner`, the explicit
private-source production/native test and independent source-to-persisted-content
checks meet the diagnostic/preservation condition. See
[merged-source acceptance](../status/2026-10-10-import-integrity-integration.md).
Project-specific identifiers, values and measurements remain private.

**Independent service/native witness (2026-10-10, `01e6be99`).**
`outside_tree_overrides_are_reported_and_preserved_without_activation` exercises
both empty and nonempty active trees through the genuine project import service.
It pins the warning and original reference, full XML source bytes/hash/owner,
native Save/Reopen/Re-Save equality, and absence of the outside override in the
typed model. An active neighbour keeps its own reference, no links and no text
override: retention cannot masquerade as activation or leak into siblings.
[Receipt](../evidence/kl173-native-evidence-2026-10-10.json).

**Remaining boundary.** Applying an override after reactivation needs documented
activation semantics. Retention and warning-level diagnostics are not typed
inactive-object modelling or new download authority. Read/Mapped counters remain
mapper-visit counts, not proof of complete raw-source interpretation. The original
omission described the pre-repair mapper at `2764b3b7`.

**Lifted condition met.** Every otherwise unconsumed override survives native
persistence as original source and is explicitly reported; automatic reactivation
semantics are outside this condition. Status of record: ledger `KL-173`.
<!-- END retained catalogue record -->
