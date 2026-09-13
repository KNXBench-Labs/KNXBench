# ADR 0019: The building model stays topological — no spatial coordinates in v1.0.0

Date: 2026-09-13
Status: Accepted
Session: 7 (T21, the graphical half)

## Context

T21 shipped its first half on 2026-09-13: the workbench renders projected
areas, lines and devices, and nested building parts with their assigned
devices, as keyboard-navigable hierarchies. `apps/knx-web/src/StructureWorkspace.tsx`
says so in its own header — "no invented physical coordinates". What was
left open was a spatial canvas or floor-plan editor, and the reason it was
left open was that the domain model has no notion of *where* anything is.
[GAP_ANALYSIS_ETS.md](../GAP_ANALYSIS_ETS.md) D1/D2 and `goal.md` §3 both
record it as "needs a domain decision about coordinates before it needs a
UI". This is that decision.

Four questions had to be answered from evidence, not from taste: does ETS
store coordinates, does the KNX Standard model space at all, in what units
and from what origin would we store them, and is a floor plan an imported
asset or something KNXBench draws.

### E1 — schema 23's published *project-data* schema has nowhere to put geometry

The authoritative document is *Project Schema23 v01.00.00* (KNX Standard
v3.0.0, 64 pages). Its §1.2.6.4 `complexType Space_t` — "An element of the
building structure" — carries exactly these attributes: `Id`, `Name`,
`Type`, `Usage`, `Number`, `Comment`, `CompletionStatus`, `DefaultLine`,
`Description`, `Puid`. Its children are `Space`, `DeviceInstanceRef`,
`Function`. There is no coordinate, extent, rotation, elevation or plan
reference. §1.2.5.1 `complexType DeviceInstance_t` is the same story across
its ~30 attributes: addresses, load flags, checksums, APDU lengths, a
`SerialNumber`, a `UniqueId` — nothing positional.

A `grep -niE "coordinate|geometr|floor.?plan"` over the whole document's
`pdftotext -layout` output returns zero hits, as does `grep -ciE "\bplan\b"`.
The only length quantity anywhere in the format's vicinity is
`Product/@WidthInMillimeter`, on rail-mounted products — two of the five
manufacturer `Hardware.xml` files sampled carry it at all (e.g.
`WidthInMillimeter="1.4230000e+002"` next to `IsRailMounted="1"`), which is
a product's DIN-rail width — a catalogue property of the hardware, not a
placement of an instance.

### E2 — the three real projects agree, across schema 11, 21 and 23

An attribute inventory of every element in each project part confirms the
schema document on real data:

| Project | `CreatedBy` / `ToolVersion` | Namespace | Building container | Space attributes present |
| --- | --- | --- | --- | --- |
| `Unser Zuhause ets4 - 2025-12-15.knxproj` | `ETS4` / `ETS 4.1.8 (Build 3614)` | `http://knx.org/xml/project/11` | `<Buildings><BuildingPart …>` | `Id`, `Name`, `Type`, `Number`, `CompletionStatus`, `DefaultLine` |
| `KV v2.5 - demo.knxproj` | `ETS6` / `6.0.5030.0` | `http://knx.org/xml/project/21` | `<Locations><Space …>` | `Id`, `Name`, `Type`, `Puid` |
| `Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj` | `ETS6` / `6.3.7959.0` | `http://knx.org/xml/project/23` | `<Locations><Space …>` | `Id`, `Name`, `Type`, `Number`, `Puid`, `CompletionStatus`, `DefaultLine` |

Not one spatial attribute in any of the three, on `Space`/`BuildingPart`,
on `DeviceInstance`, on `Area` or on `Line`. ETS's answer to "where is this
room" is its position in a tree and, optionally, a `Number` and a `Usage`.

Where ETS *does* say more about a space, it says it as taxonomy rather than
as geometry: schema 23's `simpleType SpaceType_t` (§1.1.2.3) enumerates ten
values — `Building`, `BuildingPart`, `Floor`, `Stairway`, `Room`,
`Corridor`, `DistributionBoard`, `Area`, `Ground`, `Segment` — and §1.2.6.4's
own prose for `Space_t/@Type` adds a contradictory eleventh, `RoomPart`,
which §1.1.2.3's own facet list does not contain — the document disagrees
with itself about that one value, and this ADR resolves nothing about it.
Even `Ground`, the one value that is unmistakably about a site rather than
a structure, carries no extent. (Five of those eleven have no
`BuildingPartType` variant
here and are coarsened on import; that is a separate defect, recorded on the
way past as [KNOWN_LIMITATIONS.md §89](../KNOWN_LIMITATIONS.md#89-five-documented-spacetype-values-are-coarsened-to-buildingpart-on-import),
not a coordinate question.)

### E3 — the only places a third party could hide a plan are declared out of scope

Two channels in a `.knxproj` carry data the schema does not describe:
device-level `BinaryData` (schema 23 §1.2.5.28-30, described as "For use by
plugins") and an `ExtraData/` directory, which the schema document never
mentions at all — zero occurrences of the string `ExtraData` in 64 pages.
Schema 23 §4.2.1 puts the former outside the interoperable content
explicitly, listing "BinaryData and BinaryDataRef data within device
instance data" among data "not stored within the XML files but as external
files".

In the schema-23 reference project all three `BinaryData` entries are named
`244_Info` and sit inside a `DeviceInstance`, alongside `ExtraData/*.azp`
and `ExtraData/*.rbg` members — vendor plugin state for one device, not a
building plan. KNXBench already treats both channels as opaque and
byte-preserved ([ADR-0006](0006-opaque-passthrough-store.md)).

### E4 — the KNX Standard models space, and deliberately keeps geometry out

The one place in the Standard that formally models physical space is
*3/10/3 KNX IoT Information Model* v2.0.0, whose glossary defines the
"location model aspect" as "the actual spatial building structure of an
Installation which is independent from the KNX System". Its location
classes carry only *relational* properties — `loc:hasFloor`, `loc:hasRoom`,
`loc:hasSpace`, `loc:hasLowerFloor`, `loc:hasUpperFloor` — plus an optional
postal address as an external `vcard:Address`. A grep for
`loc:(area|size|height|width|length|shape|polygon|point|boundary|centroid)`
returns nothing.

Geometry is instead delegated by *reference*. Clause 1.2.2, "Bridging to
BIM": "KNX `loc:Building`, `loc:Floor`, `loc:Site` and `loc:Space`
expresses a reference to an IFC `:IfcBuilding`, `:IfcBuildingStorey`,
`:IfcSite` and `:IfcSpace` and thus make bridging to a BIM model possible."
The standard's own architecture is: name the space, relate it, and point at
the model that owns its shape.

Across the whole 179-document extraction of the KNX Standard, "floor plan",
"floorplan", "site plan", "DXF" and "gbXML" appear in zero documents, and
the only "coordinate" in the Standard is a colour coordinate
(`DPT_Colour_xyY`'s x-axis/y-axis, 3/7/2 *Datapoint Types* v02.02.01). 140
of the corpus's 146 "BIM" hits (`pdftotext -layout`, whole-word, 17 files) are
Bus Interface Modules (9/4/2 alone holds 28), not Building Information
Modelling. The other **six** are the only Building-Information-Modelling
mentions in 179 documents: four in 3/10/3's *"Bridging to BIM"* clause, cited
above, and two in 3/10/2's constant list — its glossary entry *"BIM  Building
Information Model, a digital process to describe and document a building in
all its life cycle phases, from its planning, construction, operation up to
its demolition."* and an IFC cross-reference **[D]**. Six mentions, no
schema, no property, no datapoint type.

### What this evidence does not say

It does not say ETS 5/6 has no floor-plan feature in its own database or in
a paid ETS App; it says no such data appears in an exported `.knxproj` at
schema 11, 21 or 23, and that schema 23's published schema has nowhere to
put it. E1 rests on one document, covering project data only: *Project
Schema23 v01.00.00* types the Project, General, Topology, Device Data,
Building Structure, Group Address and SplitInfo sections and does **not**
type `ManufacturerData` — there is no `Hardware_t` or `Product_t` section
in it, which is why `Product/@WidthInMillimeter` in E1 had to be read out
of the project files rather than the schema. No KNX project-schema `.xsd`
exists anywhere in this repository or in `knx-spec-kb`, so this PDF is the
only published-schema source there is, and E2's archive inventory stands in
for the manufacturer side.
It also does not cover schema 12-14, 20 or 22, for which no sample exists
([KNOWN_LIMITATIONS.md §1](../KNOWN_LIMITATIONS.md)). If a coordinate
attribute exists in one of those, this ADR's *evidence* changes but its
*decision* does not, because the decision does not rest on ETS's choice —
see the first alternative below.

## Decision

**The building model stays topological for v1.0.0.** No `knx-core` entity
gains a positional field: not `BuildingPart`, not `Device`, not `Area`, not
`Line`. `CURRENT_SCHEMA_VERSION` stays at 6 and no migration is written for
this ADR. The graphical topology and building views keep computing their
layout from the hierarchy at render time and keep persisting nothing —
layout is a projection ([ADR-0009](0009-ui-boundary.md)), and a computed
layout is not a domain fact.

Spatial coordinates are therefore **not a gap against ETS** — there is
nothing in an exported `.knxproj` at schema 11, 21 or 23 to be compatible
with — but a **feature KNXBench does not have**,
which is a materially different claim and is recorded as such.

**If and when geometry is wanted, it arrives as its own additive layer,
never as fields on existing entities.** This ADR pre-commits the shape so
that a future session cannot invent it ad hoc, while leaving the decision
to build it open:

- **Two new entities, in their own tables.** A `FloorPlan` — an imported
  asset (raster, or a single-page vector) held as a blob with its declared
  real-world extent in millimetres — and a `Placement` binding a
  `BuildingPartId` or a `DeviceId` to a `(FloorPlanId, x_mm, y_mm,
  rotation_mdeg)`. Nothing is added to `BuildingPart` or `Device`. A
  `FloorPlan` does not itself record which `BuildingPart` it depicts; that
  association is deferred to the follow-up ADR along with the decision to
  build the layer at all. This pre-commitment also covers point placement
  only — pinning a device or a building part's label to a location on a
  plan — and defers a building part's extent or outline to that same
  follow-up ADR; a `Placement` names where a point sits, not the shape of
  a room.
- **Units: integer millimetres, and millidegrees for rotation, clockwise
  positive in the y-down frame.** Integers because equality, diff and
  round-trip must stay deterministic ([ADR-0007](0007-roundtrip-fidelity.md));
  millimetres because it is the only length unit the format itself uses
  anywhere (`Product/@WidthInMillimeter`, E1).
- **Origin: per plan, at the plan asset's own top-left corner, x right, y
  down.** Not a site datum and not geographic. Nothing in a `.knxproj`
  supplies either, and a surveyed origin KNXBench cannot obtain is a fact
  we would be inventing.
- **No z coordinate.** A `Placement` carries a `FloorPlanId`, so elevation
  is implied by which plan a point sits on; floors are also already a
  level of the hierarchy, and an elevation field would be a second,
  disagreeing representation of the same thing.
- **A floor plan is imported, never drawn.** KNXBench is not becoming a
  CAD tool; the plan is someone else's artefact, positioned and scaled, and
  the KNX Standard's own bridging model (E4) points the same way.
- **The absence of coordinates is a permanently valid state**, not a
  degraded one — every project imported from ETS will be in it, forever.
- **Coordinates are native-only data.** Since no `.knxproj` schema can
  carry them, a `.knxproj` export of a project that has them must emit an
  explicit loss warning. Writing them into device `BinaryData` or
  `ExtraData/` to smuggle them past ETS is forbidden by
  [ADR-0006](0006-opaque-passthrough-store.md).

Building the layer needs its own ADR — the same rule
[ROADMAP.md](../ROADMAP.md) already applies to T20's `Functions` and to the
project-notes idea — because "we have decided what it would look like" is
not "we have decided to have it".

## Alternatives considered

**Add optional `x`/`y` (and rotation) to `BuildingPart` and `Device` now.**
Rejected on three counts. It has no consumer: no view reads them, so the
first version would be a schema migration in support of nothing, which is
the speculative abstraction `CLAUDE.md` forbids. It puts the value on the
wrong object: a coordinate without a plan reference names no space, so
"where is the device" is only answerable relative to a plan, and a bare
`x`/`y` on the entity silently assumes one
([DATA_MODEL.md §5](../DATA_MODEL.md)). And it costs a store migration plus
a frozen fixture ([DATA_MODEL.md §11](../DATA_MODEL.md)) for a field every
imported project leaves empty.

Note what this argument is *not*: it is not "ETS does not store
coordinates, so we do not". Import formats must not dictate the internal
model (`CLAUDE.md`), and if schema 24 shipped an `X`/`Y` on `Space`
tomorrow, the answer would still be that a computed layout is not a domain
fact and an unused field is not a model. ETS's silence is why there is no
*compatibility* pressure; the reasons above are why there is no *design*
pressure.

**Keep the plan and its placements in the opaque passthrough store, or in
device `BinaryData`, so ETS plugins might read them.** Rejected. That store
is defined by ADR-0006 as data we preserve byte-identically and never
interpret; writing our own structured data into it inverts its one
invariant and makes a diff of it meaningless. It would also not achieve
interoperability: schema 23 §4.2.1 places that data outside the
interoperable content (E3), so no ETS feature is obliged to look at it, and
we would be guessing at a private format to be read by nothing.

**Adopt an IFC/BIM or geographic coordinate model now.** Rejected as
premature, not as wrong. The Standard's own location model already tells us
the correct shape — reference the external model, do not copy its geometry
(E4) — and that is the shape to implement *if* an installation with an IFC
model ever turns up. Today there is no such sample in the repository, no
IFC parser, and no user asking; an IFC bridge is a project of its own, and
claiming one on the strength of a mapping table in a specification would be
claiming a capability we have not built.

**Persist the layout the existing views already compute.** Rejected. It
would make the current layout algorithm a migration liability: change the
algorithm and every stored project disagrees with the screen, with no way
to tell a stale computed value from a deliberate user placement. Derived
data belongs in the projection, recomputed ([ADR-0009](0009-ui-boundary.md)).

## Consequences

**Nothing to migrate today.** `knx-core`'s Rust shape is unchanged,
`CURRENT_SCHEMA_VERSION` stays 6, `knx-store` gains no table, and no new
fixture is frozen. This ADR is a recorded decision with no code change,
which is the outcome it was allowed to have.

**The future migration is constrained to be additive.** A `FloorPlan`
plus `Placement` layer becomes store schema 7, two new tables and no
altered column; existing projects migrate with zero rows, and zero rows is
a valid project rather than a project missing something — unlike
[KNOWN_LIMITATIONS.md §71](../KNOWN_LIMITATIONS.md)'s pre-schema-6 module
ids, there is no value the migration would have to invent.

**One rule to enforce by review, not by a gate.** `knx-core` entities must
not grow positional fields. `cargo run -p xtask -- check-layering` keeps
layout code out of the core but would not notice an `x_mm` on
`BuildingPart`; this ADR is the thing a reviewer cites instead.

**Export is unaffected now and pre-decided later.** No export warning is
needed today because there is nothing to lose. The future layer ships with
one, so that "native-only" never becomes "quietly dropped"
([ADR-0015](0015-native-output-drops-ets-reimport-goal.md) already makes
native-only data an accepted category).

**T21's graphical half is answered rather than built.** The hierarchy views
stay as they are for v1.0.0; the canvas becomes a post-v1.0.0 roadmap item
conditioned on a follow-up ADR. `GAP_ANALYSIS_ETS.md` D2's floor-plan
overlay moves from "open gap" to "recorded non-goal for v1.0.0, with the
shape of the eventual answer written down".

**No compatibility claim changes.** COMPATIBILITY.md gains nothing: ETS
stores no coordinates at schema 11, 21 or 23 (E1, E2), so there is no ETS
behaviour here to verify or to claim.
