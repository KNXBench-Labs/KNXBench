# WP-13 remaining documentation sweep — 20 October 2026

## Scope and evidence boundary

Checked against published source `1039a78793` (the WP-13 claim, based on
`2a4b60b6`). This is a bounded source/documentation reconciliation, not a new
application, native, browser, corpus, hardware or release acceptance. Existing
receipts retain their original revision and counts. Formal ledger dispositions,
catalogue identities and retained historical blocks are unchanged.

## Reconciliation

| Topic | Current evidence | Disposition |
| --- | --- | --- |
| Installation rename | `Inspector.tsx` uses `RenameNameField`; the editor calls `api.renameEntity` with the captured tree and expected name. The current installation dossier already describes this. | Verified unchanged; no second rename implementation. |
| Product placement and binary values | `parse/program.rs` inserts `own_offset`/`own_bit_offset` and `parameter_type_enum.binary_value`; KL-128 already distinguishes modern product data from unmapped legacy binary values. | Verified unchanged; earlier migration repair remains separately owned. |
| Note editing and delete disposition | `NoteDispositionDialog`, its inspector/bulk consumers and `note_disposition_routes.rs` implement the explicit workflow. The current catalogue summaries are correct; `IMPORT_EXPORT.md` still called the service/editor pending. | Correct the import/export entry and historical-foundation dossier's current preface. |
| Selective import | `selective_import.rs` guards v16; `merge.rs` preserves destination Functions and refuses source Functions. `source_notes.rs` maps selected targets and requires an explicit outside-selection choice. | Replace the current v12/no-source-notes description; distinguish normalized service support from the ETS-only source workflow. |
| Functions consumers | [Function contract](../contracts/project-functions.md), `features/functions/FunctionsPanel.tsx`, `knx-diff` and MCP `tools/functions.rs` describe delivered consumers, not a missing backend. | Verified unchanged; source-Function selective import remains refused. |
| Plug-in web routes | `App.tsx` hosts project-level `PluginDataDialog`; `Inspector.tsx` hosts the device button. [Adoption contract](../contracts/plugin-data-adoption.md) retains the no-vendor-execution boundary. | Verified unchanged; no plug-in execution or originating-tool acceptance inferred. |
| UI waves | Current [feature inventory](../FEATURES.md) labels File menu, table columns, topology metrics, captions, units, comparison display and nested scope identity as Alpha 15. | Verified unchanged; not another UI implementation queue. |
| Lifecycle publication | [Ledger](LEDGER.md) reconciles the delivered CLI/server callers and Web history; `AUDIT-01` is DONE and `SAFE-03` ACCEPTED_BOUNDARY. | Add current contract/dossier prefaces; preserve every dated candidate statement below them. Remove the architecture table's stale candidate label. |
| Property planner versus memory-partial | [Normalized property-plan simulation](../contracts/offline-property-plan-simulation.md) and [KL-007](../limitations/KL-007-bounded-commissioning-support.md#normalized-simulation-boundary-16-october-2026-source) already distinguish simulation from public memory download, source translation and complete recovery. | Verified unchanged; simulator success does not admit hardware or close source/recovery gaps. |
| Portable manufacturer files | [KL-183](../limitations/KL-183-knxdb-manufacturer-files.md) and the manual already scope embedding to available manifest entries; the feature table still implied all new saves are self-contained. | Narrow that feature row and link its existing boundary. The catalogue-create/Save/empty-product-database case remains source-traced only, not runtime-reproduced or repaired. |

## Reader routes and preservation

- Root README and installation entry: verified unchanged; no changed operation,
  installation command or new feature to advertise.
- Operating guide and FAQ: existing notes, Functions, plug-in, rename and
  commissioning routes remain correct. The selective-import technical guide is
  updated in place; its old acceptance receipts stay historical.
- Feature inventory: only the overbroad portability row changes. Release labels
  and source feature availability remain unchanged.
- Technical contracts/status: current lifecycle prefaces and this record point
  at original evidence. This records source tracing, not fresh runtime proof.
- The old authored-note heading remains an explicit anchor. Dossier retained
  blocks and the formal ledger are preserved byte-for-byte.

## Verification and remaining work

The delivered docs candidate `d4a5c4fc` passed the maintained scoped gate10/10:
323 registered tooling tests with one existing optional skip, valid proofs,
frozen inputs and clear user-data guards. Local/tracking/live publication
readback matched. Review is a separate in-session self-review, not independent
approval; no application runtime or release acceptance is inferred.

This closes the named remaining WP-13 prose sweep, not every possible future
documentation discrepancy. The KL-183 authoring-path runtime regression is a
separate follow-up. Alpha 16 successor, exact-UP258 and directory encryption
header packages retain their existing owners.
