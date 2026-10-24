# Communication-object folders: source and identity boundary

## Status and scope

WP-06A is a **retention characterization and source contract**, not folder
implementation. It changes no production parser, model, storage schema,
activation, projection, UI, priority or protocol behavior. WP-06's source-backed
folder display remains open. Assertions below describe inspected KNXBench
implementation and public synthetic inputs; they do not independently establish
KNX/ETS schema conformance or manufacturer-folder semantics.

**Update 24 October 2026:** the WP-06a slice now displays saved project folders
from the retained tree, projection-only; see
[Saved project folder display](#saved-project-folder-display-24-october-2026).
The sections up to it record the 20 October characterization unchanged.

## Three distinct sources

| Source | Current representation | What it does not establish |
| --- | --- | --- |
| Saved project `GroupObjectTree` and nested `Node Type="Folder"` | Raw complete tree plus flattened object-reference list, qualified by owning device/source member | A typed folder, current activation, a manufacturer parameter block, a stable UI key or unique product reference |
| Manufacturer `ParameterBlock` / `ShowInComObjectTree` declarations | Separate product-source investigation; this slice admits no mapping | Equality with saved project Folder `RefId`, folder labels, conflicting ownership or module-instance authority |
| Evaluated application `Dynamic` channels | Existing `ComObjectNode.channel` from `Channel` / `ChannelIndependentBlock` activation ownership | A reconstruction of the saved folder hierarchy or a folder assignment for inactive/undetermined/unevaluated objects |

### Saved source retention

`SourceDevice.group_object_tree` is the flattened authoritative object-reference
list; `group_object_tree_raw` carries the complete tree bytes. The installation
parser unions root and nested `GroupObjectInstances` in encounter order,
deduplicating references. It does not model `Node/@Type` or `@RefId` as a typed
folder; their declarations remain inside the raw tree. The domain mapper creates
objects from the flat references, not from an inferred folder classification.

The importer returns an opaque `RetainedElement` named `GroupObjectTree` with
its topology source member and owning device's locator. For the schema-23
synthetic line-device witness the member is `P-0001/0.xml` and the locator is:

```text
/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance[@Id='P-0001-0_DI-1']/GroupObjectTree
```

This is an existing source locator, not a new folder identity. The exact retained
tree includes hierarchy, `Type`, reference attributes when actually declared,
and their absence when not declared. A missing Folder `RefId` stays absent:
never fill it from a neighboring channel, another fixture or a product block.
This test scope does not prove behavior for duplicate device identities or every
namespace, schema, source spelling or malformed folder declaration.

### Evaluated ownership is not saved membership

The product evaluator's `is_channel` accepts `Channel` and
`ChannelIndependentBlock`; traversing `ParameterBlock` does not make that node
a channel owner. The server overlays activation and channel from evaluated hits.
Its channel key contains the module-node chain and evaluator node ID, not a
saved Folder `RefId`. The projection keeps evaluated activation separate from
the project's stored `is_active` claim. Existing table grouping uses evaluated
channel keys, with inactive/undetermined/unevaluated objects unassigned.

Do not equate these identity domains, infer current activation from saved tree
membership, declare Folder `RefId` globally unique, derive a label from an ID,
or invent product/module/UI keys from the retained hierarchy.

## Public evidence and acceptance matrix

| Evidence | What it establishes | Boundary |
| --- | --- | --- |
| `schema23_reads_nested_nodes_and_root_objects_in_encounter_order` | Two tracked synthetic trees, both containing a Folder without `RefId`; flat reference order and namespace controls nearby | Existing importer test, not an independently verified vendor export |
| `schema23_retains_literal_folder_trees_for_each_owning_device` | Exact tree bytes, opaque kind/name/member/device locator, complete cardinality and per-device object order | Characterizes current behavior; no missing-feature RED claimed |
| `modern_node_and_root_styles_keep_unassigned_configuration_across_resave` | Existing separate native project/opaque persistence witness | Unmodified; no fresh save/reopen acceptance implied by this slice |
| `saved_oracle_separates_tree_membership_channel_ids_and_explicit_channel_activity` | Separate saved-state oracle declares `PB-TEST` Folder under `CH-TEST` | Its `O-TEST-*` IDs are oracle material, not copied or repaired into an error-free production-import witness |

The new test must use both original importer tree literals unchanged. Their
Folder `RefId` absence is part of the assertion. It must compare exact counts
and ownership before constructing maps, so duplicate keys cannot overwrite a
missing tree. Expected bytes/locators come from the input and literal contract,
not the production locator or flattening helper. Discrimination controls, when
executed, belong in the result record and must fail at the named runtime
assertion; compile/setup failures do not count as killed controls.

## Focused execution (not delivery acceptance)

The named characterization and existing encounter-order/namespace controls
execute through the compiled owning integration-test binary: three exact tests,
three passes, zero failures/ignored. Its compiled inventory contains 421 tests.
The parent reran the same three exact cases with isolated data/config homes and
network namespace; all pass. Existing test source is byte-preserved as an exact
prefix; only one test is added, and both input tree literals are unchanged.
The new test additionally checks the complete matching import-report inventory,
per-device object ownership and the existing group-address link.

Original production source already passes: this closes a coverage/contract gap,
not a reproduced production bug. In an authenticated isolated source snapshot,
the exact test passes before/after two separately compiled production mutations:
a fixed wrong-owner locator and different well-formed raw tree each fail at the
intended runtime assertion. Canonical source is never mutated. Independent
whole-change review accepts with zero findings; its three exact tests pass and
14 scratch-only post-import corruption controls are rejected. Those 14 are
assertion calibration, not additional production-source mutants. Initial reviewer
harness compilation failed before execution and remains separate from passes.

**Witness limit:** the two literal object-reference sequences are also
lexicographically sorted. They detect reversal but do not distinguish encounter
order from lexicographic sorting. No broader ordering discrimination is claimed.
No new native save/reopen or folder-rendering execution is implied.

### Verified package delivery

Published as main-first `9145b03f` (feature `04754bc3`) onto `77601107`.
The maintained scoped gate passed all 13 ordered stages: owning `knx-etsproj`
and checker `xtask`, 579 Rust passes (58 ignored), 55/55 selected corpus cases
and 340 registered tooling tests (one existing optional skip). Warning-denied
Clippy, format/documentation/repository checks also passed. The three named
importer cases appear as actual passes in the gate's raw Rust log.

The parent independently checked complete/success/plan/freeze flags, exact
ordered stage proofs, unchanged reviewed test bytes and frozen input hashes,
raw Rust/corpus summaries and empty touched/unchecked-user-data lists. Local,
tracking and live publication tips agreed. The selected scope includes no web,
full-workspace or release acceptance. Corpus passes are regressions, not new
manufacturer-folder or schema-conformance proof.

The compact private archive was verified member-by-member: 275 evidence files
plus one manifest, including original logs, independent review, exact source
manifests/overlays and both compiled discrimination controls. Reproducible
exports and build/dependency caches are excluded explicitly. Implementation
worktree, two merged branches and scratch are retired; a rebuildable target
remains only for the result-documentation gate. Foreign work and the selected
stale/dirty checkout were preserved. This releases the WP-06A claim, not the
remaining WP-06 display or priority requirements.

## Remaining admission requirements

**Amended 24 October 2026 (WP-06a slice):** the saved-project half is now
admitted for display; see [Saved project folder display](#saved-project-folder-display-24-october-2026).
Manufacturer `ShowInComObjectTree` mapping, folder labels, module-instance
binding and priority remain open as listed below.

Full folder implementation needs independently admissible project/manufacturer
identity witnesses, primary format semantics, deterministic conflict rules,
activation/access behavior, module-instance binding and loss/unsupported-data
reporting. Synthetic retention alone admits none of these. Existing repository
schema/private-corpus research remains a historical lead, not fresh primary
verification in this package. Priority representation remains a separate
ADR-gated slice. No manufacturer contact or new source acquisition is required
for this retention-only contract.

## Saved project folder display (24 October 2026)

**Primary format semantics.** Project Schema 23 v01.00.00 §1.2.5.21–23
(printed pages 51–52), inspected in the original PDF: `GroupObjectTree` holds
`Node` elements nested through `Nodes`; `Node/@Type` is `Folder` (a
`ParameterBlock` with `ShowInComObjectTree`) or `Channel`; `Node/@RefId` is
*required* and names that channel or block by its shortened id; a node's
`@GroupObjectInstances` lists its objects; the tree's own attribute lists the
objects of the `ChannelIndependentBlock`. No label attribute is defined, so a
folder's caption lives only in product data.

**Decision.** The retained tree already reaches native storage, save/reopen and
undo as an opaque per-device row, so this slice is **projection-only**: no
domain, storage-schema, import or command change. `knx-app`'s
`com_object_folders` reads the device's retained row (matched by exact device
`SourceRef` path plus `DeviceInstance[@Id]` locator) and admits an object only
when its full `SourceRef` belongs uniquely to that device and source member.
Before parsing, the reader verifies the retained bytes against the row's
SHA-256. The server adds
`ComObjectNode.project_folder` to the device detail. A folder's identity is the
declared `Type`/`RefId` chain from the root, per device; the key is opaque and
length-prefixed. The table shows the chain's `Folder` ids verbatim
("Project folder: PB-3"), the full saved path in the tooltip, and makes the
folder ids searchable. No `RefId` is resolved against the product, so no
manufacturer label is shown or guessed.

**Refusals (never a folder):**

| Status | When |
| --- | --- |
| `Ambiguous` | the object is listed more than once in the device's tree (also twice in one list) |
| `UnidentifiedNode` | its node or an ancestor lacks `Type` `Channel`/`Folder` or a nonempty `RefId`, or repeats another node's declared path |
| `TreeUnresolved` | several trees match; another device shares the source identity; an object has a foreign owner, another source member or a project-duplicate full source identity; retained bytes fail their SHA-256; or the tree is malformed, foreign, mixes explicit and unbound project elements, has an unbound prefix, a DTD, an element outside `Nodes`/`Node` positions, or exceeds the read bounds |
| `ModuleObject` | a module-instance object the tree places in a folder (WP-07 binding not admitted) |
| `Inactive` | a stored-inactive object, or one the current evaluation reports `Inactive`, that the tree places in a folder |

`NoFolder` (root or directly in a `Channel`) and `NotInSavedTree` (no retained
tree, schema 11, KNXBench-created/source-less device or object, unlisted object) show
nothing. Both unchanged public importer trees above therefore yield no folder
(`Ambiguous`, `UnidentifiedNode`), as they must.

**Evidence.** `knx-app` unit tests: declared single/nested folders, root and
channel objects, module/inactive refusal, both unchanged WP-06A trees,
unidentified/duplicate paths, seven malformed/foreign tree cases, explicit
project namespaces, wholly unbound inherited fragments, mixed-namespace
refusal, retained-hash mutation, and exact device/object source ownership
including project-wide duplicate identities. `knx-server`: overlay key and
refusal mapping, evaluated-inactive downgrade, and a route test that imports a
synthetic schema-23 project and gets identical placements after import, native
save-as/reopen and an undoable object edit plus its undo (no product database). Vitest covers
the view helpers, search and the rendered note. All inputs are synthetic and
built from the spec's declared structure.

**Boundaries.** The saved tree describes the import-time state. KNXBench edits
do not rewrite it, so objects created later show nothing and objects
deactivated later are refused, but a parameter change that would move an active
object to another folder is not reflected. Folder captions (product
`ParameterBlock` text), grouping the table by folder, module objects, schema 20
`Node/@Text`, the selective-import source path and priority remain open. No real
vendor export with folder `RefId`s is in the public repository; acceptance is
synthetic and spec-derived, not an ETS compatibility claim.

## Source anchors

- [Source representation](../../crates/knx-etsproj/src/source.rs),
  [installation parser](../../crates/knx-etsproj/src/parse/installation_v21.rs),
  [opaque ownership](../../crates/knx-etsproj/src/lib.rs) and
  [object mapper](../../crates/knx-etsproj/src/map.rs).
- [Importer witnesses](../../crates/knx-etsproj/tests/it/import_integrity.rs),
  [existing native persistence test](../../crates/knx-app/tests/it/import_integrity_roundtrip.rs)
  and [saved-state oracle](../../apps/knx-server/tests/common/saved_activation.rs).
- [Saved folder reader](../../crates/knx-app/src/com_object_folders.rs),
  [device-detail overlay](../../apps/knx-server/src/com_object_folders.rs),
  [route test](../../apps/knx-server/tests/it/http_com_object_folders.rs) and
  [table note](../../apps/knx-web/src/ComObjectTable.tsx).
- [Dynamic evaluator](../../crates/knx-productdb/src/dynamic/evaluate.rs),
  [server overlay](../../apps/knx-server/src/com_object_activation.rs),
  [projection](../../crates/knx-projection/src/lib.rs) and
  [table contract](../COMMUNICATION_OBJECT_TABLE.md#presentation-and-derived-data).
- [WP-06](../OPEN_WORK.md#wp-06--communication-object-organisation),
  [existing gap census](../research/2026-10-17-product-database-gap-census.md) and
  [source import boundary](IMPORT_BOUNDARY_CONTRACT.md).
