# Captured saved-instance priority

Status: implemented and independently reviewed; focused proofs pass; integrated
acceptance is tracked in the [delivery record](../status/2026-11-06-saved-instance-priority.md).
Scope: one WP-06 captured-source increment under [ADR-0158](../adr/0158-object-priority-provenance.md).

## Outcome and exclusions

Fresh, uniquely owned, non-module schema-23 project imports gain a normalized
observation of the saved instance's own declaration. Native save/reopen, working
history and the existing read-only device-detail projection preserve that fact.
It is not selected-product priority, effective inheritance, live state or a wire
encoding. Existing raw opaque retention and product-query provenance are already
delivered and are not reimplemented.

No priority edit command, UI column/editor, product resolution, activation rule,
image/device-writer change, new source schema/module support, report/diff feature,
bus action, native GUI acceptance, release or deployment is admitted.

## Core representation

Add `ComObjectInstance.saved_instance_priority: SavedInstancePriority`. The
core types have no XML, SQLite, UI or protocol dependency. Captured payloads are
boxed so an unavailable default does not carry a full inline source record.

```text
SavedInstancePriority =
  Unavailable { reason: SavedPriorityUnavailable }
  | Absent(Box<SavedPriorityAbsent>)
  | Declared(Box<SavedPriorityDeclared>)

SavedPriorityAbsent = { omission: SavedPriorityOmission, source: SavedPrioritySource }
SavedPriorityDeclared = { raw: String, source: SavedPrioritySource }
SavedPriorityOmission = AttributeOmitted | InstanceElementOmitted
SavedPriorityUnavailable = NotCaptured | UnsupportedSourceSchema | ModuleScope
  | AmbiguousOwner | AmbiguousDeclaration | SourceIntegrity | ResourceLimit
SavedPrioritySource = {
  source_schema: u32,
  device_source: SourceRef,
  object_source: SourceRef,
  owner_xpath: String,
  topology_sha256: String
}
```

Default is explicitly Unavailable(NotCaptured), never absence. Captured source
schema is exactly 23; paths agree and identify the original topology member;
owner/object identifiers and locator are nonempty; digest is 64 lowercase hex
characters. Native malformed/unknown state vocabulary refuses rather than
repairing or defaulting. `raw` is the XML reader's decoded attribute string,
not original XML octets. Empty, whitespace, unknown and case-different strings
remain exact. Lexical Known means only Low/High/Alert and grants no encoding.

## Source capture and integrity

Use the existing importer, after complete source mapping and opaque collection.
Pass the parsed source, original topology member and exact original bytes once.
Do not pull Priority out of `other` or add another raw-retention producer.

- Index all original devices before normalized maps can hide duplicate IDs;
  include every installation, Segment/line and UnassignedDevices branch.
  A duplicate original device identity in that member is AmbiguousOwner.
- Join by real normalized owner plus exact SourceRef/member/ref, not number,
  label, address or globally unique object RefId. Validate record ID/backlinks,
  object/list occurrence uniqueness and unique normalized owner/source binding.
- Refuse resolved or unresolved module-shaped references. Validate non-module
  reference syntax through existing helpers, never the mapper's fallback zero.
- Count source override occurrences independently of `over: Option<_>`: zero,
  one and duplicate are different. Duplicate declarations are ambiguous even
  when values agree, are empty or both omit Priority. Out-of-tree overrides
  create no new object and cannot supply another object's declaration.
- Match only exact unqualified `Priority`. One original element without that
  attribute is AttributeOmitted. A tree-only object with no original instance
  element is InstanceElementOmitted. Neither absence resolves inheritance.
- Authenticate exactly one whole source ContainerEntry by member, kind,
  empty locator/name, original bytes and SHA-256. A present declaration also
  needs exactly one matching owner-keyed RetainedAttribute with exact kind,
  member, locator, name, decoded UTF-8 bytes and hash. Missing, duplicate,
  wrong-owner or hash/byte mismatch is SourceIntegrity. Absence requires no
  conflicting retained Priority row. Preserve parser-error precedence.

Primary basis is Project Schema23 v01.00.00, sections 1.1.2.4 and 1.2.5.11–13,
printed pages 7 and 48. Its inspected original PDF identity is recorded in
[the source investigation](../research/2026-11-06-object-priority-provenance.md).
No older-source omission rule or malformed-token execution is inferred.

## Bounded work and allocation policy

These are KNXBench observation limits, not format limits: at most 100,000
objects examined for captured metadata, 4,096 UTF-8 bytes per raw/source string,
and 16 MiB total charged captured payload/index work. Use named constants and
checked arithmetic. Preflight lengths/counts before allocating indexes, cloning
strings, hashing diagnostic copies or expanding device-qualified locators.
Charge source/ref/path/locator/hash strings and index entries, not only `raw`.

Use linear occurrence indexes; do not scan all source or opaque entries for
each object. Projection similarly builds one ownership/source index per device
view, not a full-project scan for every row. Include counted-work and budget
boundary tests, not wall-clock ratios. Capture budgets must not refuse an
otherwise supported project import or discard opaque data: withhold the
eligible captured-metadata population as ResourceLimit, not an admitted prefix.
Unsupported-source/module reasons remain explicit. No default or partial raw
value may be emitted after incomplete budget admission.

## Native 16 to 17, without guessed backfill

Add one STRICT `com_object_saved_priority` table, PK/FK to the object. Migration
creates it empty and never reparses/rewrites opaque or history rows. Canonical
NotCaptured has no row; explicit noncanonical NotCaptured rows refuse. Other
unavailable reasons, absence and declaration have closed, mutually exclusive
shapes. SQL CHECKs must reject NULL/invalid shapes rather than passing through
SQL three-valued logic; strict decoding repeats validation after CHECK bypass.

One shared codec serves direct upsert, full transactional save, single reader
and batch reader. Validate model shape before any upsert mutation. Delete child
rows before parent objects. Batch loading must not add one query per object.
Preserve existing representability, reachability and transaction rollback.

Keep explicit snapshot versions 12,13,14,15,16 and current17 admitted. Shape
validation subtracts the new table for predecessors while retaining rejection
of unknown objects/columns. When the new table is empty, omit it from semantic
fingerprints and retain the existing identity-version choice byte for byte.
When nonempty, include all canonical rows/columns under identity version17.

Original snapshot hashes and schema shapes are checked before any owned memory
migration. Every old root, working image, undo/redo entry and named version
independently becomes NotCaptured while original opaque/context/image facts
remain unchanged. Read-only open migrates only an owned memory copy, preserving
the file's bytes/version. Writable upgrade and later operation failures remain
atomic; never clear history or rewrite old hashes to make admission succeed.
Older builds may refuse newly written schema17 files; document that boundary.

Produce the new object-bearing V16 predecessor fixture with the exact unchanged
V16 writer from the admitted base, not a current writer followed by DROP/PRAGMA
downgrade. Retain producer source/base/input/argv/file/image/context hashes.
The fixture includes genuine baseline, divergent working state, both stacks,
a named version and keyed/unkeyed/empty/unknown synthetic opaque attributes.

## Audited selective-import mapping (native model 17)

The selective-import model-17 correction and its focused regressions are authored;
execution and independent acceptance remain pending. It is a dedicated source-
reference mapping, not permission for generic clone/rebind repair.

- Before copying a selected captured fact, use the shared `SavedPriorityBindings`
  index against the complete **original source project**, including unselected
  owners/claims. Require structural validity and unique current owner, record,
  object/list and source binding. Module-bound or already mismatched, ambiguous
  or malformed captured facts refuse the **whole plan**, without changing either
  project or rewriting the fact to make it applicable. Keeping an unscoped bad
  fact is not safe: an already-prefixed mismatch could accidentally match after
  the owning records are scoped. No replacement unavailable reason is invented.
- For an originally bound declared/absent fact only, prepend the exact existing
  `source.prefix` plus `/` to `device_source.path` and `object_source.path`, just
  as for its owning Device/ComObject SourceRefs and retained member paths. Keep
  both ETS identifiers, `source_schema`, `owner_xpath`, original topology digest,
  raw decoded text and omission kind exact. Lexical token classification remains
  derived from the same untrimmed raw string. This reference scoping does not
  rewrite XML/archive/retained-attribute bytes or their hashes and does not infer
  anything from program/product defaults.
- Preserve every unavailable reason exactly, including canonical NotCaptured;
  do not promote it to declared or absent. Existing destination captured facts
  remain byte-for-byte semantic values, including any pre-existing mismatch.
- Admit the complete candidate through final `NativeSnapshot::encode`, including
  captured string bounds **after** prefix expansion and native resource/shape
  limits. Recheck imported captured bindings against the full merged destination;
  collision with an existing scoped owner/source refuses, rather than lending
  apparently unique authority to either fact. No partial plan is returned.
- The selective guard admits normalized model 17 on **both** sides only. Source
  ETS schema support stays with the complete importer; future/unknown normalized
  fields still require a new audit. Generic rebound clones outside this explicit
  mapping retain the projection mismatch behavior below. No digest reauthentication
  or new priority semantics are claimed by this mapping.

## Read-only projection

Add required `ComObjectNode.saved_instance_priority` with tagged JSON/TS states:

- unavailable: a closed reason, including projection-only SourceBindingMismatch;
- absent: exact omission and CapturedSavedInstance source;
- declared: exact raw, Known/Empty/Unknown token status, CapturedSavedInstance source.

Captured source metadata identifies the original archive-member owner/object,
locator, schema and hash. It does not claim that projection reverified retained
bytes or observed current device state. Before exposing applicable raw/absence,
check current source/owner/list/record binding and uniqueness. Rebound clones
retain the underlying fact through save/reopen but withhold raw applicability
as SourceBindingMismatch. Names, addresses, parameters, activation and language
do not themselves rewrite captured source identity. Product/server overlays
must leave this field untouched. No extra endpoint or operation is added.

Update generated bindings and all direct typed fixture literals explicitly;
do not make the field optional, use `any`, exclude a configuration or weaken
production types. Unrelated checked-date work remains with its existing owner.

## Executable acceptance obligations

All are pending until actual execution is recorded. Use production entry points.

1. App/import: schema23 Segment and UnassignedDevices; three known literals;
   empty/whitespace/case/unknown/entity-decoded Unicode; both absence causes;
   same RefId on distinct owners; product/enrichment independence.
2. Parser/capture: original fatal malformed/duplicate-attribute errors; foreign
   namespace/position/case controls; duplicate owners/declarations; module and
   malformed-ref withholding; out-of-tree diagnostics; all independent original
   member/retained attribute ownership/hash/byte checks and a valid control.
3. Resource: independently vary object count, raw/owner/path/locator lengths,
   index work and total bytes; below-bound success, no incomplete prefix,
   unchanged opaque data and linear counted work.
4. Store: exact semantic and opaque preservation through repeated close/reopen;
   shared single/batch codec; nonempty existing-data save rollback; invalid SQL
   states and decoder tests after constraint bypass; retained rebound facts.
5. Predecessors/history: real V16 object-bearing writer fixture plus authentic
   existing V12/V13 fixtures; read-only byte/version preservation; baseline,
   working, both stacks and named restore; corruption/future-schema refusal;
   post-migration failure rollback; empty/nonempty fingerprint controls.
6. Commands/projection/API: full object/device delete/restore and add/remove
   inverses retain the field; working-state projection after reopen; real mounted
   GET device route returns every required tagged state; no product/wire alias;
   binding mismatch withholds raw; all generated and standalone fixture types.

Owning targets are the existing consolidated `it` targets for knx-app,
knx-store and knx-server, and the core/etsproj/projection library tests. Final
acceptance uses the maintained change-scoped package gate on frozen sources,
with its selected corpus/bindings/type/browser stages. All tests and fixture
production use task-owned synthetic files, isolated homes, offline loopback,
low concurrency and the maintained shared leases. No real user file or bus
is an acceptance input. An authored contract or fixture is not feature delivery.
