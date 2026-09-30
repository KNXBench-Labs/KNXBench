# Data model

## 1. Scope

This describes the target domain model of `knx-core`. It is the reference
Session 2 implements against.

Session 1 implemented two of these types: `Layer` and `Resolved<T>`. Session 2
implemented the rest of the domain model described here, in `knx-core`, plus
the migration-chain skeleton in `knx-store`. Session 3 added one more type,
`Override<T>` — see the amendment in section 3 and
[ADR-0010](adr/0010-per-attribute-override-representation.md) — and built
the importer (`knx-etsproj`) that actually constructs this model from a
real project; it added no other new `knx-core` type. Session 3 also built a
`.knxproj` writer, which was withdrawn on 2026-09-20
([ADR-0028](adr/0028-no-knxproj-export.md)) — the model is unchanged by
that, but sentences below that explain a design choice by what "export
writes" are explaining a decision made when one existed. Every section below states whether it is **implemented**,
**planned** or **retained but uninterpreted**, so that the document can be
read as a status as well as a design.

Every count cited here was measured on the reference project in Session 0 and
is reproducible with [tools/inspect_knxproj.py](../tools/inspect_knxproj.py).
Section references point into [RESEARCH.md](RESEARCH.md).

## 2. Identity

*Implemented: the `*Id` newtypes and `SourceRef` in `knx-core/src/ids.rs`.*

Two kinds of identifier, never conflated.

**Internal IDs** — `DeviceId`, `GroupAddressId`, `ComObjectInstanceId`, and so
on — are stable, project-unique and persisted. These are the primary keys.

**`SourceRef { path, ets_id }`** carries the original ETS identifier string,
for example `M-006A_A-0001-22-26C0_O-0_R-10001`, together with the path in the
source document it came from.

ETS identifiers are never used as primary keys. They collide across projects
and they change. They are also never discarded: provenance needs them to
explain where a value came from, retained attributes are keyed by them
(`knx_etsproj::xpath`), and the project diff uses them to recognize the same
entity across two imports.

Note that ETS ids are not opaque. `ComObjectInstanceRef/@RefId` is a compound
key — application program id, `_O-<ComObject number>`, `_R-<ComObjectRef id>` —
which identifies the base object and the variant at once (RESEARCH §3.2). It is
parsed on import to reconstruct the override chain, and preserved as written.

## 3. The override chain

*Implemented: `Layer`, `Resolved<T>` (Session 1, `knx-core/src/provenance.rs`);
`Override<T>` (Session 3, `knx-core/src/provenance.rs` — see the amendment
below and [ADR-0010](adr/0010-per-attribute-override-representation.md));
the command layer that produces `UserEdit` values and undoes back to the
originating layer lives in `knx-core/src/command.rs` (Session 2); the
`Program`/`ProgramRef` layers are now populated — see the Session 4
amendment below.*

**Amendment (Session 4):** `Program` and `ProgramRef` are no longer
theoretical layers this crate merely defines and never produces.
`knx_productdb::enrich(&mut Project, &Connection)` fills them by resolving
each device's `Hardware2ProgramRefId` to an application program in the
shared product database, then each communication object's
`ComObjectInstanceRef` source id (`ComObjectInstance.source.ets_id`,
already the full `ComObjectRef` id, `map.rs:660`) to that program's
`ComObjectRef`/`ComObject` pair — no change to `knx-core` itself was
needed for the second step.

Enrichment writes **only into `Override::Absent` slots**
([ADR-0012](adr/0012-enrichment-into-absent-slots.md)): `Empty`,
`Malformed` and an instance-level `Value` are what the project file
actually said, and overwriting any of them would destroy the record of
that. A space-separated, multi-alternative `ComObjectRef/
@DatapointType` (RESEARCH §4.2) fills nothing and is reported rather than
guessed. `ComObjectInstance.size` is the one field enrichment fills
without this restriction, exactly per its own doc comment's stated
purpose.

**Amendment (Session 3):** the chain resolves **per attribute**, not per
object. `ComObjectInstanceRef` alone can leave `Text` absent while stating
`DatapointType` as an empty string and `Description` with a real value —
three different states on three different attributes of the same instance.
A single `Resolved<T>` per object cannot express that; `Override<T>` below
is what does.

This is the single most constraining finding of Session 0 (RESEARCH §3.2). A
communication object's effective properties resolve through three layers in the
source data:

```text
ApplicationProgram/.../ComObject       defaults: Name, Text, FunctionText,
                                       ObjectSize, Priority, DatapointType, flags
        ↓ overridden by
ApplicationProgram/.../ComObjectRef    per-variant override: Text, FunctionText,
                                       ObjectSize, DatapointType, flags, Tag
        ↓ overridden by
0.xml  ComObjectInstanceRef            per-device override: Text, Description,
                                       DatapointType, five of the six flags,
                                       IsActive
```

Measured override frequencies in the reference project: `DatapointType` 758×
(of which 497 are the empty string and 261 carry a value — see the amendment
above), `Description` 691×, `Text` 121×, `ReadFlag` 39×, `UpdateFlag` 30×,
`TransmitFlag` 27×, `WriteFlag` 18×, `CommunicationFlag` 8× — against 907
communication object instances in total. `ReadOnInitFlag`, the sixth flag,
occurs 0× at instance level — in this project and in both others measured
(KNOWN_LIMITATIONS §117). It is a program-layer attribute in every file
sampled here, which is why the instance row above names five and
`ResolvedFlags` still models six.

758 of 907 instances carry a `DatapointType` attribute at instance level at
all (149 do not). An importer that reads only the application program is
therefore wrong for the large majority of objects, and a model without
provenance cannot tell the user which of the three layers a value it is
showing actually came from.

No resolved scalar exists without its layer, and no overridable attribute
exists without its three-state presence:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Layer {
    /// Default from the application program's `ComObject`.
    Program,
    /// Per-variant override from the application program's `ComObjectRef`.
    ProgramRef,
    /// Per-device override that was present in the imported project.
    Instance,
    /// Derived by this application, for example a datapoint type taken from
    /// linked communication objects. Never written back as if the user set it.
    Inferred,
    /// Changed in this application.
    UserEdit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resolved<T> {
    pub value: T,
    pub layer: Layer,
}

/// A source attribute in one of its four real states — Session 3,
/// ADR-0010. `Override::Empty` and `Override::Absent` are distinct:
/// collapsing them loses exactly the 497-vs-149 distinction above.
/// `Override::Malformed` keeps the raw text of a present value that
/// could not be parsed, so the file's own text is never silently
/// replaced by a guess.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum Override<T> {
    #[default]
    Absent,
    Empty,
    Value(Resolved<T>),
    Malformed(String),
}
```

Whether a value is the project's **own** follows from the layer alone,
which is what `Layer::is_exported()` returns:

| Layer | Origin | The project's own value |
| --- | --- | --- |
| `Program`, `ProgramRef` | Product database | No |
| `Instance` | Present in the source project | Yes |
| `UserEdit` | Changed in this application | Yes |
| `Inferred` | Derived by us, for example a DPT from linked objects | No — shown in the UI as inferred |

The method's name is a fossil of the `.knxproj` writer that first needed
the distinction (ADR-0028 withdrew it on 2026-09-20). The distinction
itself did not go anywhere: `knx-diff` uses it to decide what is worth
comparing, and the CSV and documentation exports use it to decide what is
worth writing. All four `Override` states keep their meanings — `Absent`
is "never stated", `Empty` is "stated as an empty string", `Malformed`
holds the unparsable raw text, and `Value` carries its layer with it —
because the point of the distinction was always to record what the source
file said, not to drive one writer. See [ADR-0004](adr/0004-provenance-model.md) for the layer model and
[ADR-0010](adr/0010-per-attribute-override-representation.md) for why it is
wrapped in `Override<T>` per attribute rather than applied once per object.

**Module-based objects (schema ≥21):** a module-based device resolves its
`ComObjectInstance` defaults through one extra hop before reaching the
product database — `ModuleInstance` → `ModuleDef` → `ComObjectRef` →
`ComObject`, rather than the direct `ApplicationProgram` chain above, which
schema 11 (and any non-module device) still uses unchanged. This section
states only the chain's *shape*; see [ADR-0013](adr/0013-module-instance-representation.md)
for why `ModuleInstance` is a first-class entity and
[ADR-0014](adr/0014-group-object-tree-authoritative-source.md) for why
`GroupObjectTree`, not `ComObjectInstanceRef`, is the authoritative source
for which module instance a given object belongs to. The `ModuleDef` side
of the chain — its own `Arguments`/`Static`/`Dynamic`, argument binding,
and the id-mangling rule connecting its internal ids to a project's
instance-level ones — is now researched at the application-program level;
see [RESEARCH.md](RESEARCH.md) §4.4 (R4 spike, 2026-09-11).
`knx-productdb`'s `Dynamic` evaluator now expands a `Module` node into its
`ModuleDef`'s own stored tree at the application-program level (T18 slice
2, 2026-09-11), but this domain model and the import path are untouched
by that — per-instantiation `ModuleInstance` resolution stays a
project-side concern this model still does not evaluate
([KNOWN_LIMITATIONS.md §3](KNOWN_LIMITATIONS.md#3-device-parameters-are-preserved-but-not-interpreted)).

## 4. Entities

*Implemented: `knx-core/src/{device,parameter,devices,group,building,topology,
installation,project}.rs`.*

The attribute sets below are those actually observed in the reference project
(RESEARCH §3), not the full schema — no authoritative XSD is available, so the
model is built from evidence and extended when an import reports something
unknown.

| Entity | n | Observed attributes | Treatment |
| --- | --- | --- | --- |
| `Project` | 1 | `Id` | Modelled; `schema_version` added by us |
| `Installation` | 1 | `InstallationId`, `Name`, `BCUKey`, `DefaultLine`, `IPRoutingMulticastAddress`, `SplitType`, `CompletionStatus` | Modelled; `BCUKey` and `SplitType` retained opaque |
| `Area` | 1 | `Id`, `Name`, `Address`, `CompletionStatus` | Modelled |
| `Line` | 1 | `Id`, `Name`, `Address`, `MediumTypeRefId`, `DomainAddress`, `DomainAddressIsChecked`, `IPRoutingMulticastAddress`, `MulticastTTL`, `CompletionStatus` | Modelled |
| `BusAccess` | 1 | `Name`, `Edi`, `Parameter` | Opaque — ETS tool configuration, see section 10 |
| `DeviceInstance` | 36 | `Id`, `Name`, `Description`, `Address`, `ProductRefId`, `Hardware2ProgramRefId`, `LastModified`, `LastDownload`, `CompletionStatus`, the five `*Loaded` flags, `IsCommunicationObjectVisibilityCalculated`, `Broken` | Modelled; commissioning attributes in section 7 |
| `ComObjectInstanceRef` | 907 | `RefId`, `IsActive`, `DatapointType`, `Description`, `Text`, `ReadFlag`, `WriteFlag`, `TransmitFlag`, `UpdateFlag`, `CommunicationFlag` | Modelled with provenance, section 3 |
| `ParameterInstanceRef` | 1390 | `RefId`, `Value` | Retained uninterpreted, section 10 |
| `Send` / `Receive` | 569 / 27 | `GroupAddressRefId` | Modelled as directional links, section 6 |
| `GroupRange` | 35 | `Id`, `Name`, `RangeStart`, `RangeEnd` | Modelled |
| `GroupAddress` | 514 | `Id`, `Name`, `Address`, `Central`, `Unfiltered` | Modelled |
| `BuildingPart` | 22 | `Id`, `Name`, `Number`, `Type`, `DefaultLine`, `CompletionStatus` | Modelled, section 5 |
| `DeviceInstanceRef` | 29 | `RefId` | Modelled as a building-to-device reference |
| `BinaryData` | 6 | `Id`, `Name` | Opaque, section 10 |
| `ModuleInstance` | 4 devices' worth in the KV sample (schema 21; not present in the schema-11 reference project) | `RefId`, `RepeatIndex`, `Arguments`/`Argument` | Modelled as a first-class entity, retained-but-uninterpreted arguments/repeat_index, ADR-0013 |

`ComObjectInstance` is the entity the override chain hangs off. Every
overridable attribute is `Override<T>`, per the amendment above — not the
`Resolved<T>` / `Option<Resolved<T>>` mix this section originally sketched
before Session 3 measured what "resolved" actually needs to represent:

```rust
pub struct ComObjectInstance {
    pub id: ComObjectInstanceId,
    pub source: SourceRef,
    pub device: DeviceId,
    pub number: u16,                       // from _O-<n>
    pub text: Override<Text>,
    pub description: Override<Text>,
    pub dpt: Override<DptRef>,
    pub flags: ResolvedFlags,              // six independent Override<bool>
    pub size: Option<Resolved<ObjectSize>>,
    pub is_active: bool,
    pub links: Vec<GroupLink>,
    pub module_instance: Option<ModuleInstanceId>,
}
```

`module_instance` is `Some` only for objects belonging to a module-based
device (schema ≥21, [ADR-0013](adr/0013-module-instance-representation.md)):
such an object resolves its DPT/Text defaults through
`ModuleInstance` → `ModuleDef` → `ComObjectRef` → `ComObject` instead of the
direct `ApplicationProgram` chain schema 11 uses. It is `None` for every
schema-11 device.

`size` stays a plain `Option<Resolved<ObjectSize>>`, not `Override<T>`:
schema 11 never states an object size at instance level at all (Session 3
measurement), so there is no empty-vs-absent distinction to preserve for
this one field yet — only "known, from the product database" versus
"unknown". `text`/`description` hold `Text`, not `LocalizedString`, per
[ADR-0010](adr/0010-per-attribute-override-representation.md) and section
8's `Text::Literal`/`Text::Localized` split: schema 11's instance-level
overrides are literal strings ETS wrote directly into the project, never a
handle into the string table.

`Functions` are defined in the schema but absent from the reference project —
`xknxproject` reports an empty functions map. ETS5 and later projects do use
them, so the model must not be shaped in a way that assumes their absence. This
is an assumption, not a verified fact (RESEARCH §3.1).

**Decided, not implemented — project notes:**
[ADR-0031](adr/0031-project-notes-are-a-project-owned-collection.md) reserves a
future ordered `ProjectNote` collection on the `Project` aggregate. A note has
a stable ID and a typed target for either the project or a supported
user-facing entity; it is not a text field copied onto every entity. The
current six-field `Project` in `knx-core/src/project.rs` has no such collection,
and schema version 9 has no note table. Implementation therefore requires a
future schema bump, ordered migration, and frozen predecessor fixture. Existing
projects will migrate to an empty collection; this paragraph does not claim
that notes can currently be created, stored, projected, or reported.

## 5. Two orthogonal hierarchies

*Implemented: `knx-core/src/{topology,building,devices}.rs`.*

`Devices` is the sole owner of devices. `Topology` (Area → Line → devices) and
`Buildings` (recursive typed `BuildingPart` → devices) hold references only
(RESEARCH §3.1). A device is placed in a building and in a line independently;
neither placement owns it.

Observed `BuildingPart/@Type` values: `Building` (1), `Floor` (3), `Room` (14),
`Corridor` (2), `DistributionBoard` (1), `BuildingPart` (1). `BuildingPart`
nests recursively and carries `DefaultLine`. Beyond those six observed values,
`BuildingPartType` supports the complete documented Space vocabulary.
*Project Schema23 v01.00.00* §1.1.2.3 enumerates ten and §1.2.6.4
names eleven, including both `RoomPart` and `Segment`: only `RoomPart` is
absent from the enumeration. `Stairway`, `RoomPart`, `Area`, `Ground` and
`Segment` retain exact types through import, native save/load/re-save,
projection, API creation and localized UI. Synthetic tests verify these five;
the local reference projects do not contain them. Unknown ETS types still
produce a `MapProblem`; unknown native types cause a typed load error.
The unconstrained `kind TEXT NOT NULL` column needs no migration: schema v9
is unchanged. ETS project export remains absent (ADR-0028).

**A site/property is a `Ground` root, not a new kind**
([ADR-0038](adr/0038-site-is-a-ground-root-space.md), 2026-09-26). Several
buildings sharing one KNX infrastructure are `Building` children of one
parentless `Ground` part in one `Installation`. The installation owns the
shared topology and group addresses, so the site owns no device and
duplicates none. Schema23 §1.2.6.3 names `Ground` among the normal root
types. No `Site` kind exists, and an undocumented `Site` token on import
stays a reported `UnknownEnumValue` rather than becoming `Ground`. A level
above `Installation` (grouping *separate* infrastructures) is explicitly not
modelled and would need its own ADR.

**A device without a line is valid** and lives in `Topology::unassigned`. The
reference project contains exactly one, and it is precisely the device
`xknxproject` loses (RESEARCH §7.1). Any model that makes line membership
mandatory reproduces that bug.

**Both hierarchies are topological, and neither carries coordinates**
([ADR-0019](adr/0019-building-model-stays-topological.md), 2026-09-13). No
entity in this model has a position, an extent, a rotation or a floor-plan
reference, and none is planned for v1.0.0. The decision rests on evidence
rather than on omission: `Space_t` (schema 23 §1.2.6.4) and
`DeviceInstance_t` (§1.2.5.1) have no spatial attribute, an attribute
inventory of all three reference projects — schema 11's
`<Buildings><BuildingPart>` and schema 21/23's `<Locations><Space>` — finds
none either, and the KNX Standard's own spatial model (3/10/3 *KNX IoT
Information Model*) keeps geometry out of its location classes on purpose,
referencing IFC instead. Graphical topology and building views therefore
compute their layout at render time and persist nothing, consistent with
[ADR-0009](adr/0009-ui-boundary.md). ADR-0019 also
pre-commits the shape of a later spatial layer — separate `FloorPlan` and
`Placement` entities in their own tables, integer millimetres, origin at the
plan's own top-left, no `z` — so that it cannot be bolted onto these
entities as fields; building it needs its own ADR and its own store schema
version, and neither exists.

## 6. Directional links

*Implemented: `knx-core/src/flags.rs` (`Direction`, `GroupLink`).*

```rust
pub struct GroupLink {
    pub ga: GroupAddressId,
    pub direction: Direction,   // Send | Receive
}
```

Direction is semantically meaningful — which object writes the group address
versus which listens — and is never flattened into an undirected association.
The reference project has 569 send links against 27 receive links (RESEARCH
§3.1); collapsing them would discard that distinction for 596 links at once.

## 7. Commissioning state

*Implemented: `knx-core/src/commissioning.rs`.*

Commissioning state is domain data, not import metadata. It describes the delta
between the planned project and the physical installation, which is
engineering-critical (RESEARCH §3.1). `CompletionStatus` in the reference
project: 25× `FinishedDesign`, 11× `Editing`.

```rust
pub struct CommissioningState {
    pub completion: CompletionStatus,   // Undefined | Editing | FinishedDesign | Accepted
    pub individual_address_loaded: bool,
    pub application_program_loaded: bool,
    pub parameters_loaded: bool,
    pub communication_part_loaded: bool,
    pub medium_config_loaded: bool,
    pub last_modified: Option<DateTime<Utc>>,
    pub last_download: Option<DateTime<Utc>>,
    pub broken: bool,
}
```

Modelling these as data, rather than dropping them as ETS bookkeeping, is what
keeps a future commissioning path open — and, in v1, what lets the application
tell the user which devices are not in the state the project says they are.

## 8. Localized strings

*Implemented: `knx-core/src/string_table.rs`.*

`LocalizedString` is a handle into a `StringTable` keyed by `(key, language)`,
not a `String`. Import populates the table from the `TranslationUnit` trees:
one application program alone carries 5919 translation elements (RESEARCH
§4.1). Display resolves against the active language and falls back to
`DefaultLanguage`.

A field that can hold either form — `ComObjectInstance::text`/`description`,
for instance — is typed `Text`, not `LocalizedString` directly:

```rust
pub enum Text {
    Literal(String),
    Localized(LocalizedString),
}
```

**Amendment (Session 3):** schema 11's instance-level `ComObjectInstanceRef/
@Text`/`@Description` are always `Text::Literal` — ETS writes the string
directly into the project and keeps no translation for it (measured: this
session's importer never calls `StringTable::insert`, since it ingests no
application program yet). `Text::Localized` is reachable only once an
application program is ingested (Session 4) and resolves a `Program`/
`ProgramRef`-layer value — which `Layer::is_exported()` rejects as not the
project's own regardless, so the distinction matters for display, not for
what any writer emits.

This is retrofit-hostile — replacing `String` with a handle after the fact
touches every entity, every projection and every test — which is why it is in
the model from day one.

**Amendment (T32, 2026-09-12).** Product-data translations live in
`knx-productdb` — a separate crate and a separate schema from this model —
and that table changed shape. `translation` was keyed by `(program_id,
language, ref_id, attribute_name)`, which only a translation belonging to
an `ApplicationProgram` could satisfy; schema v4 replaces `program_id`
with `(scope, scope_id)`, where `scope` is `Program`, `Catalog`,
`Hardware` or `Master`. `Catalog`/`Hardware` rows key off the owning
`Manufacturer/@RefId`, `Program` rows off the application program's `@Id`
exactly as before, and `Master` rows — `knx_master.xml` has no owning
element at all — use `''` as their `scope_id`. The empty string is a
deliberate sentinel, not laziness: SQLite treats NULLs in a
non-`INTEGER` primary key as pairwise distinct, so a NULL `scope_id`
would silently permit duplicate master rows, the one thing the key
exists to prevent (`dynamic_node.module_def_id` carries the identical
sentinel for the identical reason). A v3→v4 backfill replays already
stored blobs so existing databases gain the rows without a reinstall.
The core model's own `StringTable` is untouched by this and still has no
resolver ([KNOWN_LIMITATIONS.md §37](KNOWN_LIMITATIONS.md#37-imported-translations-are-stored-but-never-read-and-the-ui-is-english-only--partially-resolved-2026-09-12)).

## 9. Addresses and datapoint types

*Implemented: `knx-core/src/address.rs`, `knx-core/src/dpt/` (a module
directory since T29, 2026-09-11: `mod.rs` for `DptRef` itself, `codec.rs`
for decoding/encoding a value against one, `resolve.rs` for inferring a
group address's DPT from its linked communication objects — see
[KNOWN_LIMITATIONS.md §61](KNOWN_LIMITATIONS.md#61-the-dpt-codec-covers-thirty-main-types-infers-rather-than-reads-its-input-and-leaves-several-encoding-questions-to-a-stated-ruling-rather-than-the-standard) for exactly what the
codec covers).*

Addresses are dedicated types, not integers. `IndividualAddress(u16)` exposes
area, line and device; `GroupAddress(u16)` is rendered according to the
project-wide `GroupAddressStyle` (Free, TwoLevel, ThreeLevel). Parsing and
formatting are pure functions with typed errors, which makes them exhaustively
unit-testable and keeps formatting decisions out of the UI. The raw `u16`
never changes when the style does — restyling
(`knx_core::Command::SetGroupAddressStyle`, undoable, `knx-server`'s
`POST /api/project/group-address-style`) only changes how every address is
subsequently rendered, and refuses to touch anything if even one existing
address would not survive the new style, though since TwoLevel's 5+11 bits
and ThreeLevel's 5+3+8 bits both exhaust the full 16, that refusal currently
has no way to trigger (`knx_projection::ProjectTree.group_address_style`
carries the current choice out to the UI's Inspector, read-only —
[KNOWN_LIMITATIONS.md §84](KNOWN_LIMITATIONS.md#84-a-projects-group-address-style-can-be-chosen-and-afterwards-never-seen--resolved-2026-09-14-t4)).

Two rules that an over-strict model would get wrong:

- **A group address without a datapoint type is normal, not an error** — 194 of
  514 in the reference project (RESEARCH §6.1).
- **A group address with no linked communication object at all is normal** —
  110 of 514.

Both must survive import unchanged and stay that way in `.knxdb`. Neither is
a validation failure; at most, both are findings in a report.

Datapoint types are referenced, not inlined: `knx_master.xml` defines 289 DPT
subtypes, which belong to the product database rather than to each project.

`DptRef` parses a single `DPST-<main>-<sub>` / `DPT-<main>` string. It does
not yet handle `ComObjectRef/@DatapointType` being a space-separated list of
alternatives, which the reference project's raw XML contains (RESEARCH
§4.2 — `M-0083/M-0083_A-0019-13-A892.xml`, e.g. `"DPST-9-21 DPST-9-21"`).
Deferred to `knx-productdb` (Session 4), which owns DPT compatibility
resolution.

## 10. Retained but uninterpreted

*Retained but uninterpreted.*

Kept in the model so that the commissioning path stays open (RESEARCH §8.3),
but not interpreted in v1:

- `ParameterInstance { ref: SourceRef, raw: String }` — 1390 values in the
  reference project, dropped entirely by `xknxproject`. Held as raw strings
  because interpreting them requires evaluating the `Dynamic` tree; its
  `@test` value grammar is now documented and its structural grammar is
  corpus-observed (RESEARCH §4.3, RESEARCH R3). `knx-productdb` — a
  separate crate and a separate schema from this one — now stores that tree
  losslessly in a `dynamic_node` table (schema v3, backfilled into
  existing databases from their stored blobs) and evaluates it headlessly
  into active `ParameterRef`/`ComObjectRef` sets (T18 slice 1, 2026-09-11).
  A `ModuleDef`'s own tree is stored under the same table as its own scope
  (`module_def_id` set to the `ModuleDef`'s own `@Id`, rather than the
  empty string the owning application program's own tree uses), and is
  now *evaluated* too: the evaluator follows a `Module` node in the
  program's tree into the referenced `ModuleDef`'s stored tree (T18 slice
  2, 2026-09-11) — no schema change and no v4, `dynamic_node` already
  stored everything this needed. This domain model was untouched by
  either slice; **T18 slice 3 (2026-09-11) gives `ParameterInstance` a
  reader and a writer** — `apps/knx-server`'s parameter routes
  (`GET`/`POST /api/device/{id}/parameters`) — without any schema change
  here either: `ParameterInstance` is still keyed by `(device, ets_id)`
  only and still holds a plain `raw: String`. That single-key shape is
  exactly why module-scoped (per-channel) editing is out of scope for this
  slice (design D25). **T18 task 12 (2026-09-14)** gives the product
  database its first structure for module *arguments*: `products.sqlite`
  schema **v11** adds a `module_def_argument` table (one row per
  `ModuleDef/Arguments/Argument`, keyed `(program_id, module_def_id, id)`,
  carrying `name`/`arg_type`/`allocates`/`position`) and a
  `dynamic_node.value` column holding a `NumericArg`/`TextArg`'s `@Value`
  in a column of its own rather than only inside the deliberately
  not-re-parseable `extra` audit string. `migrate_v10_to_v11` backfills both
  by replaying stored `ApplicationProgram` blobs ([ADR-0020](adr/0020-migrations-may-rederive-from-stored-bytes.md)
  rule E1). The evaluator uses them to substitute `{{ArgumentName}}` into
  `Channel`/`ParameterBlock`/`ParameterSeparator` text, so two
  instantiations of one `ModuleDef` no longer read identically. This
  domain model is again untouched: the project side still stores a
  `ParameterInstance` keyed by `(device, ets_id)`, and argument
  interpretation happens entirely on the product-database side of the
  fence ([KNOWN_LIMITATIONS.md §§68-71](KNOWN_LIMITATIONS.md) name what
  that leaves open). **Schema v18 (2026-09-30, [ADR-0052](adr/0052-channel-name-and-number-are-stored-verbatim.md))**
  adds `dynamic_node.name` and `dynamic_node.number`: a `Channel`'s `@Name`
  and `@Number`, verbatim (`number` is text; not every corpus value is a
  number). `migrate_v17_to_v18` backfills them from the stored blobs as v11
  did, but keeps every recorded unknown row except the retired
  `Channel/@Number` ones. Again product-database side only.
- `Memory`, `AbsoluteSegment`, `LoadProcedures`, mask and resource data — held
  in the product database, not in the project.
- `BusAccess`, `BCUKey`, `SplitType`, `BinaryData` and vendor baggage — held in
  the opaque store keyed by source path, written back byte-identical, never
  executed and never interpreted ([ADR-0006](adr/0006-opaque-passthrough-store.md)).

"Uninterpreted" is not the same as "discarded". Everything listed here
round-trips.

## 11. Versioning and migration

*Implemented: `Project::schema_version` and `CURRENT_SCHEMA_VERSION` (now
`6`, module-scoped parameter editing's id retention — see amendments below)
in `knx-core/src/project.rs`; the migration chain (`open_and_migrate`,
`migrate_v0_to_v1` through `migrate_v5_to_v6`, the frozen `v1-empty.sqlite`
through `v5-empty.sqlite` fixtures) in `knx-store/src/migration.rs`.*

**Amendment (Session 3):** `migrate_v1_to_v2` adds the opaque-passthrough
table (`opaque_entry`, section 10 / [ADR-0006](adr/0006-opaque-passthrough-store.md)).
`knx-store`'s own `CURRENT_SCHEMA_VERSION` and `knx-core`'s track each
other in lockstep by design (the migration.rs doc comment states this
directly) — bumping to 2 here does not mean the *Rust shape* of `Project`
changed in Session 3 (it did not: no new field, no new entity type), only
that a new table now exists for a version this schema number to describe.

**Amendment (Session 4):** `migrate_v2_to_v3` adds the manufacturer
manifest table (`manufacturer_ref`, [IMPORT_EXPORT.md §10](IMPORT_EXPORT.md)
/ [ADR-0011](adr/0011-product-database-storage.md)). Same lockstep
relationship as Session 3: bumping to 3 does not mean the Rust shape of
`Project` changed (it did not).

**Amendment (Session 5, cycle 2):** `migrate_v3_to_v4` adds one table (or
more) per `knx_core::Project` entity — installation/area/line, building
parts, devices and communication objects (including a normalized
`com_object_override` table for the `Override<T>` chain), group
ranges/addresses, parameter instances — plus `project_info` and
`id_allocators` for the project's own scalar/counter state. Full detail,
including the owned-list-vs-flat-list `position`-column reasoning, is in
[the design spec](superpowers/specs/2026-09-03-knx-entity-persistence-design.md),
not repeated here. Unlike Sessions 3 and 4, this one *does* change
`knx-core`'s Rust shape, though only by derive: `Project`, `Devices`,
`StringTable` and `IdAllocators` all gain `#[derive(PartialEq)]`, and
`StringTable` gains a public `iter()` — both additive, no behavior change,
needed so `knx-store`'s round-trip test can assert `Project == Project`
and so it can enumerate string-table entries to persist them.

**Amendment (schema 21/23 import support):** `migrate_v4_to_v5` bumps
`CURRENT_SCHEMA_VERSION` 4 → 5, adding new `module_instance`/
`module_instance_argument` tables plus a nullable
`com_object_instance.module_instance_id` column and a
`project_info.ets_schema_version` column. Unlike the table-only Session 3/4
amendments above, this one *does* change `knx-core`'s Rust shape: a new
`ModuleInstance` entity (`knx-core/src/module.rs`), one new field on
`ComObjectInstance` (`module_instance: Option<ModuleInstanceId>`, section 4),
and one new field on `ProjectInfo` (`ets_schema_version: u32`, defaulting to
11). See [ADR-0013](adr/0013-module-instance-representation.md).

**Amendment (module-scoped parameter editing, D38):** `migrate_v5_to_v6`
bumps `CURRENT_SCHEMA_VERSION` 5 → 6, adding `module_instance.
instance_ets_id TEXT NOT NULL DEFAULT ''` — the verbatim project-side
`ModuleInstance/@Id` (e.g. `"MD-2_M-4_MI-1"`), which schema-≥21 import
already parses but previously discarded after using it only as a local
wiring key. `knx_core::ModuleInstance` gains the matching
`instance_ets_id: String` field, retained uninterpreted next to
`repeat_index`; existing rows migrate to `''`, treated identically to a
missing instance by the read/write path (design D39): every module-scoped
section for such a `ModuleInstance` stays read-only until the project is
re-imported, since the migration cannot invent the id
([KNOWN_LIMITATIONS.md §71](KNOWN_LIMITATIONS.md#71-a-project-imported-before-store-schema-6-has-no-module-instance-ids-to-write-with)).
`source.ets_id` keeps holding the `@RefId` unchanged — the two differ by
exactly the `_MI-<k>` suffix, which is the entire point of keeping both.
The server reads `instance_ets_id` to reconstruct the exact id a write
must target, never a guessed `MI-1` (design D38/D39). See the design doc
at `docs/superpowers/specs/2026-09-12-module-scoped-editing-design.md`
(Evidence E1/E5, Decisions D35-D43).

**Note (spatial coordinates, 2026-09-13):**
[ADR-0019](adr/0019-building-model-stays-topological.md) adds **no**
migration and no version — `CURRENT_SCHEMA_VERSION` stays 6 — because it
decides *not* to model coordinates in v1.0.0 (section 5). It does constrain
the migration a later spatial layer would need: two new tables (`floor_plan`,
`placement`) and no altered column, so every existing project migrates with
zero rows. Zero rows is a complete project rather than a project missing
something, which is what makes that migration safe in a way §71's invented
module ids were not.

```rust
pub struct Project {
    pub schema_version: u32,
    // ...
}
```

The schema version is stored in SQLite's `user_version` pragma and mirrored in
the model. Migration is an ordered chain `v_n → v_n+1` implemented in
`knx-store`; there is no version-skipping path and no downgrade.

Every schema version gets a frozen fixture file, committed once and never
regenerated, which must keep loading. A migration that cannot open its
predecessor's fixture is a failing test, not a release note.

See [ADR-0003](adr/0003-sqlite-project-format.md).
