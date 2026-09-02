# Data model

## 1. Scope

This describes the target domain model of `knx-core`. It is the reference
Session 2 implements against.

Session 1 implemented two of these types: `Layer` and `Resolved<T>`. Session 2
implemented the rest of the domain model described here, in `knx-core`, plus
the migration-chain skeleton in `knx-store`. Every section below states
whether it is **implemented**, **planned** or **retained but uninterpreted**,
so that the document can be read as a status as well as a design.

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
and they change. They are also never discarded: export needs them to write a
file ETS can read, and provenance needs them to explain where a value came
from.

Note that ETS ids are not opaque. `ComObjectInstanceRef/@RefId` is a compound
key — application program id, `_O-<ComObject number>`, `_R-<ComObjectRef id>` —
which identifies the base object and the variant at once (RESEARCH §3.2). It is
parsed on import to reconstruct the override chain, and preserved as written.

## 3. The override chain

*Implemented: `Layer`, `Resolved<T>` (Session 1, `knx-core/src/provenance.rs`);
the command layer that produces `UserEdit` values and undoes back to the
originating layer lives in `knx-core/src/command.rs` (Session 2).*

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
                                       DatapointType, all five flags, IsActive
```

Measured override frequencies in the reference project: `DatapointType` 758×,
`Description` 691×, `Text` 121×, `ReadFlag` 39×, `UpdateFlag` 30×,
`TransmitFlag` 27×, `WriteFlag` 18×, `CommunicationFlag` 8× — against 907
communication object instances in total.

758 of 907 instances override the datapoint type. An importer that reads only
the application program is therefore wrong for the large majority of objects,
and a model without provenance cannot decide what to write back on export.

No resolved scalar exists without its layer:

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
```

Export semantics follow from the layer alone, which is what
`Layer::is_exported()` returns:

| Layer | Origin | Written to `0.xml` on export |
| --- | --- | --- |
| `Program`, `ProgramRef` | Product database | No |
| `Instance` | Present in the source project | Yes |
| `UserEdit` | Changed in this application | Yes |
| `Inferred` | Derived by us, for example a DPT from linked objects | No — shown in the UI as inferred |

See [ADR-0004](adr/0004-provenance-model.md).

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

`ComObjectInstance` is the entity the override chain hangs off:

```rust
pub struct ComObjectInstance {
    pub id: ComObjectInstanceId,
    pub source: SourceRef,
    pub device: DeviceId,
    pub number: u16,                       // from _O-<n>
    pub text: Resolved<LocalizedString>,
    pub description: Option<Resolved<LocalizedString>>,
    pub dpt: Option<Resolved<DptRef>>,
    pub flags: Resolved<ComFlags>,
    pub size: Resolved<ObjectSize>,
    pub is_active: bool,
    pub links: Vec<GroupLink>,
}
```

`Functions` are defined in the schema but absent from the reference project —
`xknxproject` reports an empty functions map. ETS5 and later projects do use
them, so the model must not be shaped in a way that assumes their absence. This
is an assumption, not a verified fact (RESEARCH §3.1).

## 5. Two orthogonal hierarchies

*Implemented: `knx-core/src/{topology,building,devices}.rs`.*

`Devices` is the sole owner of devices. `Topology` (Area → Line → devices) and
`Buildings` (recursive typed `BuildingPart` → devices) hold references only
(RESEARCH §3.1). A device is placed in a building and in a line independently;
neither placement owns it.

Observed `BuildingPart/@Type` values: `Building` (1), `Floor` (3), `Room` (14),
`Corridor` (2), `DistributionBoard` (1), `BuildingPart` (1). `BuildingPart`
nests recursively and carries `DefaultLine`.

**A device without a line is valid** and lives in `Topology::unassigned`. The
reference project contains exactly one, and it is precisely the device
`xknxproject` loses (RESEARCH §7.1). Any model that makes line membership
mandatory reproduces that bug.

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

This is retrofit-hostile — replacing `String` with a handle after the fact
touches every entity, every projection and every test — which is why it is in
the model from day one.

## 9. Addresses and datapoint types

*Implemented: `knx-core/src/address.rs`, `knx-core/src/dpt.rs`.*

Addresses are dedicated types, not integers. `IndividualAddress(u16)` exposes
area, line and device; `GroupAddress(u16)` is rendered according to the
project-wide `GroupAddressStyle` (Free, TwoLevel, ThreeLevel). Parsing and
formatting are pure functions with typed errors, which makes them exhaustively
unit-testable and keeps formatting decisions out of the UI.

Two rules that an over-strict model would get wrong:

- **A group address without a datapoint type is normal, not an error** — 194 of
  514 in the reference project (RESEARCH §6.1).
- **A group address with no linked communication object at all is normal** —
  110 of 514.

Both must survive import and export unchanged. Neither is a validation failure;
at most, both are findings in a report.

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
  because interpreting them requires the `Dynamic` tree grammar, which is
  unresearched (RESEARCH R3).
- `Memory`, `AbsoluteSegment`, `LoadProcedures`, mask and resource data — held
  in the product database, not in the project.
- `BusAccess`, `BCUKey`, `SplitType`, `BinaryData` and vendor baggage — held in
  the opaque store keyed by source path, written back byte-identical, never
  executed and never interpreted ([ADR-0006](adr/0006-opaque-passthrough-store.md)).

"Uninterpreted" is not the same as "discarded". Everything listed here
round-trips.

## 11. Versioning and migration

*Implemented: `Project::schema_version` and `CURRENT_SCHEMA_VERSION` in
`knx-core/src/project.rs`; the migration chain skeleton
(`open_and_migrate`, `migrate_v0_to_v1`, the frozen `v1-empty.sqlite`
fixture) in `knx-store/src/migration.rs`.*

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
