# Schema 21/23 import support — design

**Status.** Approved, ready for implementation planning.

## Context

[`ideas.md`](../../../ideas.md) flags this as the next major task (2026-09-06):
"Schema 21/23 vollständiger Import-Support." [KNOWN_LIMITATIONS.md §1](../../KNOWN_LIMITATIONS.md#1-single-sample-bias)
and [ROADMAP.md](../../ROADMAP.md) both already call for "its own
brainstorming/design pass" before implementation, not a drive-by table entry
— this is that pass.

**Where this picks up.** Session 7 cycle 1 (2026-09-06) added a second,
independent `.knxproj` sample (`KV v2.5 - demo.knxproj`, schema 21, KNX
Association demo project — RESEARCH §2.5) and proved schema 21 is
detected-and-refused by name, same mechanism as schema 23
(`ImportFailure::NoKnownSchemaTable`). Neither schema has a `known_schema`
table or a mapper path; both are still `known_schema(21) == None` /
`known_schema(23) == None`. RESEARCH §3.4 measured the schema-11-vs-21
content-model diff and found two things blocking real implementation:
`ModuleInstances` (modular application programs) has no domain-model
representation, and `GroupObjectTree` vs. `ComObjectInstanceRef` needed a
resolution rule. This session's brainstorming dug further into the KV
sample (walking `GroupObjectTree/Nodes`, the `ModuleDef` chain in
`M-00FA`'s application programs) and resolved both — see
[ADR-0013](../../adr/0013-module-instance-representation.md) and
[ADR-0014](../../adr/0014-group-object-tree-authoritative-source.md), and
the amended RESEARCH §3.4.

**Scope decision: backend only.** `knx-etsproj` (import/export) +
`knx-core` (domain model) + `knx-productdb` (enrichment). No new UI this
cycle — `ModuleInstance` data flows into `ComObjectInstance` exactly as
schema-11 data does today, and existing UI (Inspector, device tree) renders
it unchanged. A module/channel-grouped view is a legitimate follow-on cycle,
not a blocker for calling schema 21/23 import "supported." Same split
Session 5 cycle 2 (entity persistence, no UI) → cycle 3 (UI) already used.

**Scope decision: schema 21 gets a round-trip claim, schema 23 does not.**
The KV sample is schema 21. The existing schema-23 sample (`Unser Zuhause`,
re-exported from the same installation ETS4→ETS6, RESEARCH §2.4/§3.3) has
**no** `ModuleInstances` at all — every device in it uses a monolithic
application program. There is no independent evidence for what a
module-based schema-23 project's `GroupObjectTree`/`ModuleDef` shape looks
like; RESEARCH §3.4's "every structural delta §3.3 attributed to schema 23
is already present at schema 21" finding covers `Segment`, `Puid`,
`Locations`/`Space` — it does not and cannot cover modules, since §3.3's
sample never exercises them. Schema 23's `known_schema` table and mapper
path are still built (module-based devices should not silently misparse),
but KNOWN_LIMITATIONS must keep saying schema 23's module handling is
inferred, not evidenced, until an independent module-using schema-23 sample
exists.

## Core resolution algorithm (ADR-0014 applied)

For schema ≥ 21, per device:

1. Read `GroupObjectTree` (schema 21: walk `Nodes/Node/@GroupObjectInstances`
   across all nodes and union the ids; schema 23: read the flat
   `@GroupObjectInstances` attribute directly) → the authoritative id list.
2. For each id, look up a matching `ComObjectInstanceRef` by `@RefId`, if one
   exists → instance-layer overrides (`DatapointType`, `Text`, flags,
   `Links`), same `Override<T>` machinery as schema 11.
3. Resolve the id's `ComObjectRef`/`ComObject` defaults:
   - Monolithic program (no `ModuleInstances` on this device): existing
     `ApplicationProgram/Static/ComObjects`+`ComObjectRefs` path, unchanged.
   - Module-based: strip the `MD-*_M-*_MI-*_` instance prefix to recover the
     `ModuleDef`-relative `O-*_R-*` id, resolve `ModuleInstance/@RefId` →
     `ModuleDef/@Id` (via the owning device's `Hardware2ProgramRefId`, same
     short-id-recovery pattern as schema 23's `ComObjectInstanceRef/@RefId`,
     RESEARCH §3.3/§3.4), then the same `ComObjectRef`/`ComObject` join
     `knx-productdb` already does for monolithic programs, one level deeper.
4. `Dynamic/choose/when` is never evaluated. Step 1 already is that
   evaluation's result, per ETS.

An id present in `GroupObjectTree` with no `ComObjectInstanceRef` override
and no product-database entry for its program/module still produces a
`ComObjectInstance` — DPT/Text/flags stay `Override::Absent`, surfaced by
`ImportReport`, never defaulted to a guess (CLAUDE.md: never silently
discard, never invent; ADR-0012's absent-slot discipline extends unchanged).

## `knx-core` changes

- `ids.rs`: `id_type!(ModuleInstanceId, u32)`.
- New `module.rs` (mirrors `parameter.rs`'s shape): `ModuleInstance` struct
  per ADR-0013 — `id`, `device: DeviceId`, `source: SourceRef`,
  `repeat_index: String` (opaque), `arguments: Vec<(SourceRef, String)>`
  (uninterpreted, same policy as `ParameterInstanceRef::value`).
- `device.rs`/`ComObjectInstance`: add `pub module_instance:
  Option<ModuleInstanceId>`.
- `Devices` (`devices.rs`): gains a `module_instances: Vec<ModuleInstance>`
  collection and accessor, same shape as its existing `com_objects()`
  enumerator.
- `IdAllocators` (`project.rs`): `next_module_instance_id`, same pattern as
  every other counter.
- Schema version bump (`knx-core`'s own versioning, DATA_MODEL §11) — additive
  only, no existing field changes shape.
- DATA_MODEL.md §4 entity table gains a `ModuleInstance` row; §3 (override
  chain) gets a note that module-based objects resolve through one extra
  hop, cross-referencing ADR-0013/0014 rather than re-deriving them.

## `knx-etsproj` changes

- `known.rs`: `known_schema(21)` and `known_schema(23)` tables, built from
  measured evidence only (same policy as `SCHEMA_11` — no guessed
  attributes). Schema 21's table is built from the KV sample directly.
  Schema 23's table reuses schema 21's entries for everything RESEARCH
  confirmed present at both (`Segment`, `Puid`, `Locations`/`Space`,
  `Security`) plus schema 23's own confirmed deltas (§3.3: short `RefId`,
  `Links` attribute, flat `GroupObjectTree`, `"true"`/`"false"` booleans) —
  and is explicitly commented as carrying the module-related entries
  (`ModuleInstances`, `ModuleDef`, nested vs. flat `GroupObjectTree`) *by
  inference from schema 21*, not independent schema-23 evidence, per the
  scope decision above.
- `map.rs`: new device-mapping path for schema ≥ 21, implementing the
  resolution algorithm above. Schema 11's `map_device`/`map_com_object`
  stay untouched (ADR-0014's consequence: kept textually separate rather
  than one function branching on schema version).
- `RefId` parsing: extend the existing `com_object_number` parser (Session 3)
  to strip a leading `MD-*_M-*_MI-*_` module-instance prefix before applying
  the existing `_O-<n>_R-<n>` extraction — additive, schema-11 `RefId`s have
  no such prefix and are unaffected.
- Export: `ModuleInstance`/`Arguments`/`GroupObjectTree` written back from
  retained `SourceRef`s, verbatim structure. Round-trip test target: zero
  unknown-construct entries on the KV sample, same bar schema 11 already
  meets.

## `knx-productdb` changes

- `ingest.rs`: application-program ingestion also walks
  `ApplicationProgram/ModuleDefs/ModuleDef/Static/{ComObjects,ComObjectRefs}`
  — same row shape as the existing top-level walk, keyed additionally by the
  owning `ModuleDef` id.
- `enrich.rs`: enrichment lookup gains the module hop (`ModuleInstance` →
  `ModuleDef` → `ComObjectRef` → `ComObject`) as an alternate join path,
  selected when `ComObjectInstance.module_instance.is_some()`. ADR-0012's
  rule is unchanged: fills only `Override::Absent` slots, never overwrites
  `Empty`/`Malformed`/instance-layer `Value`.
- No schema-23-specific productdb work: manufacturer data for a
  differently-shaped schema-23 project is already blocked on a real sample
  (`2026-09-03-product-database-design.md` §46, "same blocker as schema 23
  project data, risk R1") — unaffected by this design, not reopened here.

## Testing

- `knx-etsproj`: round-trip test against the KV demo project (schema 21),
  target zero unknown-construct entries. Unit tests for the new `RefId`
  prefix-stripping, the `GroupObjectTree` walk (both nested-`Nodes` schema-21
  shape and flat schema-23 shape), and the `Absent`-when-unresolvable case.
- `knx-core`: `ModuleInstance` construction/equality tests, same shape as
  existing entity tests.
- `knx-productdb`: enrichment tests for the module-hop join, mirroring
  `enrich.rs`'s existing four ADR-0012 tests but through a `ModuleDef`.
- `knx-store`: migration test for the new schema version (frozen fixture,
  same pattern as `v1`–`v4`), persistence round-trip for `ModuleInstance`.
- Regression: existing schema-11 fixtures and round-trip tests must stay
  green, unchanged — this design adds a sibling path, not a replacement.

## Documentation updates (post-implementation)

- KNOWN_LIMITATIONS.md §1: schema 21 import moves from "detected and
  refused" to "implemented, round-trip verified on one sample"; schema 23's
  module handling stays flagged as inferred-not-evidenced.
- COMPATIBILITY.md: schema 21 entry, scoped to what the KV sample verifies.
- ROADMAP.md: "next major format-support task" line closed out, replaced
  with whatever residual gap remains (schema 12/13/14/20/22 still
  unsampled; `Dynamic/choose/when` grammar still unresearched, now provably
  avoidable rather than blocking).

## Explicitly out of scope

- Interpreting `Dynamic/choose/when` (KNOWN_LIMITATIONS §3) — not needed
  (ADR-0014), not attempted.
- `repeat_index`'s `"NxM"` shape — retained opaque (ADR-0013), not parsed.
- Schema 12/13/14/20/22 — no samples exist, unaffected by this design.
- UI surfacing of `ModuleInstance` grouping — deferred, see scope decision
  above.
- Schema-23 manufacturer/product database ingestion — pre-existing R1
  blocker, unaffected.
