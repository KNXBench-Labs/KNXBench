# Independent sample coverage

[Current summary and ledger status](../KNOWN_LIMITATIONS.md#catalogue-kl-001)

This dossier preserves the technical detail and dated history. Earlier
headings, failures and lifting conditions are historical where later
updates supersede them; they are not new work assignments.

<!-- BEGIN retained catalogue record -->

## 1. Single-sample bias

**Narrowed 2026-10-10: independent module-using schema-23 import evidenced.**
Import-integrity's `GroupObjectTree/Nodes/Node` and `UnassignedDevices` repairs
are integrated. The unchanged independent source passes the genuine project
and embedded-product import, product database verification, native save/reopen
and resave, and the independent production RefId gate. The new opt-in witness
`private_original_unknown_report_is_backed_by_exact_retained_sources`
(`crates/knx-app/tests/private_schema23_reconciliation.rs`, `6f8ac4332c79`)
independently inventories original XML members and requires each to survive
byte-exactly under its original source identity in native/project or product
source storage. Duplicate identities refuse rather than overwrite.

**Final source admission:** `382ab577` passed the full source/Web/corpus/private/
browser chain. New upstream-only regression tests pass on `dd4b8165`; runtime
inputs are unchanged. This evidence narrows preservation coverage only.

**Owner-approved reconciliation boundary.** Reported, byte-exact
*retained-uninterpreted* data counts as source-preservation reconciliation,
not as understood semantics. The unknown report remains nonempty and
`ImportReport.has_losses()` deliberately remains true: its conservative
meaning includes unknown or misunderstood semantics. No warning is hidden
or downgraded by this test. Each unknown report source must resolve to its
original retained bytes; import errors and inference conflicts must be absent.
The previous requirement to reconcile unknowns "to empty" is superseded by
this explicitly accepted, narrower preservation contract, not by deleting
unknown entries.

**Scope.** Independent schema-23 module/RefId handling is now evidenced for the
imported sample, rather than inferred only from the schema-21 reference. The
runtime's broader conservative module-support warning is unchanged. This
single additional sample does not establish complete ETS semantics, writable
Repeat activation, or coverage of arbitrary module/program shapes. Device
extensions and project metadata can remain uninterpreted while their original
source bytes are retained. Native persistence is verified, not an ETS export
roundtrip (the historical schema-21 writer was withdrawn by ADR-0028).

**Remaining boundary.** Schemas 12/13/14/22 still need independent source
and format evidence (§6); wider schema-21/23 feature and manufacturer coverage
still needs further independent samples. KL-1 remains `ACCEPTED_BOUNDARY` for
that residual coverage, but the independent schema-23 import/reconciliation
prerequisite is complete. No ETS, Secure or hardware compatibility claim.
[Compatibility](../COMPATIBILITY.md) and the
[package receipt](../evidence/kl1-source-reconciliation-2026-10-10.json) separate
source preservation from semantics and publication.
<!-- END retained catalogue record -->
