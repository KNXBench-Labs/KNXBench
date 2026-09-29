# ADR 0014: `GroupObjectTree` is the authoritative communication-object list for schema ≥ 21

Date: 2026-09-06
Status: Accepted
Session: 7 (reopened into Session 2/3 territory)

## Context

RESEARCH §3.3 measured, for schema 23, that `ComObjectInstanceRef` elements
are omitted whenever they carry no override and no link — a 24% undercount
if `ComObjectInstanceRef`s are treated as the full object list — and named
`GroupObjectTree` as the authoritative source instead. That was a one-off
observation on one project shape, not yet a rule the parser enforces.

Session 7 measured the same relationship on the KV demo project (schema 21,
a second, independent installation, RESEARCH §2.5): for every one of its 4
devices, every `ComObjectInstanceRef/@RefId` is a member of
`GroupObjectTree`'s id set, with zero exceptions, and `GroupObjectTree`
carries substantially more ids than `ComObjectInstanceRefs` does (e.g. one
device: 5 `ComObjectInstanceRef`s against 26 `GroupObjectTree` ids). Schema
21's `GroupObjectTree` shape itself differs from schema 23's — nested
`Nodes/Node[@Type=Channel]/@GroupObjectInstances`, grouped per module
channel, rather than schema 23's flat `DeviceInstance/GroupObjectTree/
@GroupObjectInstances` — but the *role* it plays is identical: the full,
authoritative object list, with `ComObjectInstanceRef` as the overlay of
overrides and links on top.

Separately: for module-based devices (ADR-0013), `ComObjectInstanceRef`
essentially never carries `DatapointType`/`Text`/flag overrides in the KV
sample (measured: `Links`, `ChannelId`, `RefId` only, across every instance).
Resolving those objects' semantics requires walking down through
`ModuleInstance` → `ModuleDef` → `ComObjectRef` → `ComObject`
(`knx-productdb`), which in turn requires knowing *which* `ComObjectRef` is
active — the very question `Dynamic/choose/when` answers inside the
application program, and which KNOWN_LIMITATIONS §3 already flags as an
unresearched grammar.

## Decision

Import treats `GroupObjectTree` as the authoritative enumeration of a
device's communication objects for schema ≥ 21, schema-version-generic (not
a schema-21-only special case): walk `GroupObjectTree` first to get the full
id list, then look up each id's `ComObjectInstanceRef` (if any) for instance
overrides, then its module/program chain for defaults. `ComObjectInstanceRef`
elements are never enumerated directly as "the device's objects" for these
schema versions — only schema 11, which has no `GroupObjectTree` at all,
keeps `ComObjectInstanceRef` enumeration as the primary path.

Because `GroupObjectTree` already reflects ETS's own resolution of
`Dynamic/choose/when` — it is the *output* of that evaluation, written into
the file — import never evaluates the `choose`/`when` grammar itself. An id
present in `GroupObjectTree` is active; one absent is not. This is not an
assumption standing in for missing research: it is what "authoritative" was
already measured to mean in the paragraph above.

## Alternatives considered

**Keep `ComObjectInstanceRef` enumeration as primary, treat `GroupObjectTree`
as supplementary.** Rejected: reproduces the exact 24%+ undercount RESEARCH
§3.3 already identified as a defect, now measured worse for module-based
devices (KV sample: `ComObjectInstanceRef` covers as little as 1/5 of a
device's live objects).

**Evaluate `Dynamic/choose/when` ourselves to compute the active set,
independent of `GroupObjectTree`.** Rejected: unresearched grammar
(KNOWN_LIMITATIONS §3), and redundant — `GroupObjectTree` gives the same
answer without needing the grammar at all. Revisit only if a future project
sample shows `GroupObjectTree` and a from-scratch evaluation disagreeing,
which would mean this ADR's core assumption (ETS always keeps them in sync)
is wrong.

**Reconstruct a schema-11-shaped `ComObjectInstance` list and drop
`GroupObjectTree` after parsing (don't model it).** Rejected: the objects
`GroupObjectTree` names but `ComObjectInstanceRef` doesn't are real,
addressable communication objects (they can still be group-linked later by a
user editing the project) — dropping them contradicts CLAUDE.md's "never
silently discard information."

## Consequences

`knx-etsproj`'s schema-≥21 mapper path enumerates devices differently from
schema 11's: `map_device` for schema 11 stays exactly as it is (no
`GroupObjectTree` to read), a new sibling path added for schema ≥ 21 rather
than a single function branching on schema version internally — keeping the
two paths textually separate is deliberate, since conflating them risks the
undercount above leaking back into schema 11 by accident.

An object present in `GroupObjectTree` but with no `ComObjectInstanceRef`
override and no product-database entry for its module/program still gets a
`ComObjectInstance` — DPT/Text simply stay `Override::Absent`/unknown, which
`ImportReport` must surface as such rather than silently rendering as a
zero-value default.

This rule is schema-version-generic by design: schema 23's flat
`GroupObjectTree` shape and schema 21's nested `Nodes/Node` shape both feed
the same authoritative-id-list concept, only the XML shape of *reading* that
list differs per schema.

**Amendment 2026-09-29 (ISSUE-08).** "Present in `GroupObjectTree` is
active" now reaches the mapped value too. Until then the schema-≥21 mapper
read a missing `ComObjectInstanceRef/@IsActive` as `false`. It imported
every overridden object as inactive: 691 of 867 in the ETS 6.3.0 reference
project and 26 of 75 in the KV demo. Project Schema23 §1.2.5.13 declares no
such attribute on that element, and neither sample writes one. A stated
value is still honoured (IMPORT_EXPORT §9.3).
