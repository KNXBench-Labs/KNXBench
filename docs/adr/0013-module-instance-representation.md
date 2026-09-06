# ADR 0013: `ModuleInstance` is a first-class entity; its arguments stay uninterpreted

Date: 2026-09-06
Status: Accepted
Session: 7 (reopened into Session 2/3 territory)

## Context

Schema 21 (and, by RESEARCH §3.4's inference, schema 23) introduces modular
application programs: a `DeviceInstance` no longer has one monolithic
`ApplicationProgram`, it instantiates one or more `ModuleDef`s, each
`ModuleInstance` carrying a `RepeatIndex` (e.g. `"6x1"`, shape unresearched)
and `Arguments` (e.g. `argCH=1`, a formal-argument-to-value map). Measured on
the KV demo project (RESEARCH §3.4, §2.5): every `DeviceInstance` in the
sample uses this shape, none use a monolithic program.

`ModuleDef`'s internal structure — `Static/ComObjects` + `ComObjectRefs`,
`Dynamic/Channel/ParameterBlock/choose/when` — mirrors the existing
`ApplicationProgram/Static/ComObjects`+`ComObjectRefs` shape exactly, one
level deeper. The `Dynamic/choose/when` grammar that picks which
`ComObjectRef`s are active for given argument/parameter values is the same
grammar already flagged unresearched in KNOWN_LIMITATIONS §3 — this ADR does
not resolve that gap.

Multiple `ComObjectInstance`s hang off one `ModuleInstance` (verified: the KV
sample's device 1 has 8 `ModuleInstance`s covering between 1 and 3 live
communication objects each). `DeviceInstance` today already owns its
`ComObjectInstance`s directly (DATA_MODEL §4); a modular device needs an
intermediate grouping the same shape as `DeviceInstance` → `ComObjectInstance`
already has, or that grouping is lost on import and cannot be reconstructed
for export.

## Decision

`ModuleInstance` becomes a first-class `knx-core` entity, mirroring
`DeviceInstance`/`ComObjectInstanceRef`'s existing precedent rather than being
folded into an attribute bag on `ComObjectInstance`:

```rust
id_type!(ModuleInstanceId, u32);

pub struct ModuleInstance {
    pub id: ModuleInstanceId,
    pub device: DeviceId,
    pub source: SourceRef,          // the ModuleDef this instance refers to
    pub repeat_index: String,       // retained opaque, see below
    pub arguments: Vec<(SourceRef, String)>,  // retained uninterpreted
}
```

`ComObjectInstance` gains `pub module_instance: Option<ModuleInstanceId>` —
`None` for schema-11-shaped monolithic-program devices, `Some` for
module-based ones.

`repeat_index` and `arguments` are retained **uninterpreted**, the same
policy DATA_MODEL §10 already applies to `ParameterInstanceRef::value`: we
know the ETS id and the raw value, we do not evaluate what it means. This is
deliberate, not a shortcut deferred out of laziness — `arguments` feeds the
`Dynamic/choose/when` grammar to decide which `ComObjectRef`s are active, and
[ADR-0014](0014-group-object-tree-authoritative-source.md) established that
import never needs to run that evaluation itself, because `GroupObjectTree`
already carries ETS's own answer. Interpreting `arguments` would be work with
no import-time consumer.

## Alternatives considered

**Fold module data into `ComObjectInstance` as extra fields (no separate
entity).** Rejected: loses the one-`ModuleInstance`-to-many-`ComObjectInstance`
grouping (repeat index, argument set) that a device with 8 module instances
and ~15 live communication objects actually has. Re-deriving it at export time
from scratch, without ever having modelled it, risks writing a
`ModuleInstances` block ETS cannot parse back.

**Treat the whole `ModuleInstances` subtree as opaque passthrough (like
`BinaryData`, DATA_MODEL §10).** Rejected: `ComObjectInstance.module_instance`
needs a real id to resolve DPT/Text through the module chain
([ADR-0014](0014-group-object-tree-authoritative-source.md),
`knx-productdb`'s `ModuleDef` enrichment) — an opaque blob cannot be joined
against by the rest of the domain model. Full opacity would leave every
modular communication object's DPT unknown, which the KV sample shows is the
majority case (instance-level overrides are essentially absent).

**Interpret `Dynamic/choose/when` to compute active `ComObjectRef`s from
`arguments` ourselves.** Rejected in [ADR-0014](0014-group-object-tree-authoritative-source.md);
restated here because it's the alternative that would have made `arguments`
worth parsing.

## Consequences

`knx-store` gains a `module_instance` table (one row-writer module, same
shape as the existing entity-persistence modules) and a schema-version bump,
same pattern as every prior entity addition. `ComObjectInstance`'s existing
`number: u16` (from `_O-<n>`) stays as-is; module-based `RefId`s parse the
same way once the `MD-*_M-*_MI-*_` prefix is stripped (Session 3's existing
`com_object_number` parser, extended, not replaced).

Export must write `ModuleInstance`/`Arguments` back verbatim from the
retained `SourceRef`/value pairs — round-trip depends on never having
discarded them, same discipline ADR-0007 already requires for
`ParameterInstanceRef`.

If a future session researches the `Dynamic/choose/when` grammar (lifting
KNOWN_LIMITATIONS §3), `arguments` is already positioned as that evaluator's
input — this ADR does not have to be revisited, only extended.
