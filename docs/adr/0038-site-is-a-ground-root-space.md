# ADR 0038: A site is a `Ground` space at the root of the building structure — no new kind, no new level

Date: 2026-09-26
Status: Accepted (2026-09-28; independent UI-track review and user acceptance)
Session: DIN-16 (Paperclip), research and decision only — no UI
Review: UI-track U1 checked the cited Project Schema23 and KNX IoT source PDFs,
`knx-etsproj/tests/site_hierarchy.rs`, native storage and core command
characterization tests, and KNOWN_LIMITATIONS §127. No blocking contradiction.
`Ground` is a documented root space, not an asserted synonym for `loc:Site`;
real ETS `Ground` exports and editing non-first installations remain unverified.

## Context

A user reported (ISSUE-06, `docs/superpowers/plans/2026-09-21-user-reported-issues.md`)
that "a property/site is missing; multiple buildings using one KNX
infrastructure should be groupable beneath it." The plan names three
allowed outcomes: an evidenced external type, a KNXBench-native hierarchy
node, or no domain change with a documented explanation. It also fixes two
interface rules. No `BuildingPartType` or storage value is added until the
representation, import mapping and compatibility behaviour are documented.
Unknown external `Space/@Type` values stay reported instead of silently
becoming Site. This decision blocks ISSUE-05's site UI (DIN-3 plan §5).

Evidence below is marked **[D]** for a verbatim statement in a KNX document
and **[M]** for a measurement on repository data. Interpretation is marked
**[I]**. Source PDFs are under
`/mnt/daten-i/Sourcecode/knx-spec-kb/sources/The KNX Standard v3.0.0/`,
read with `pdftotext -layout`. The first 16 hex digits of each SHA-256 are
given.

### E1 — The project schema already has a root-level site type: `Ground`

*Project Schema23 v01.00.00* (`8fe9193e70135064…`):

- **[D]** §1.1.2.3 `simpleType SpaceType_t` (PDF p. 7) enumerates
  `Building`, `BuildingPart`, `Floor`, `Stairway`, `Room`, `Corridor`,
  `DistributionBoard`, `Area`, `Ground`, `Segment`. It has no `Site`,
  `Property`, `Campus` or `Premises` value.
- **[D]** §1.2.6.3 `element Locations_t/Space` (p. 54): "Space elements
  directly below Locations_t will nromally have Type "Area" or "Building"
  or “Ground”" (the typo "nromally" is in the source).
- **[D]** §1.2.6.4 `complexType Space_t` (p. 55): a `Space` has child
  `Space` elements, so the structure nests recursively. The `Type`
  attribute is required, and its prose lists eleven values (the ten above
  plus `RoomPart`, see [KNOWN_LIMITATIONS.md §89](../KNOWN_LIMITATIONS.md#89-five-documented-spacetype-values-are-coarsened-to-buildingpart-on-import)).
  None of them is a site type other than `Ground`.
- **[D]** §1.2.6.1: `Locations` sits under
  `Project_t/Installations/Installation`. §1.2.3.13 lists that
  `Installation`'s children as `Topology`, `Buildings` ("Contains the
  building structure"), `GroupAddresses`, `Trades` and `SplitInfos`. The
  document is inconsistent with itself here: §1.2.3.13 calls the building
  structure `Buildings`, §1.2.6.1 heads it `…/Installation/Locations` and
  lists its child as `BuildingPart`, while §1.2.6.2 `Locations_t` holds
  `Space`. Real exports use `Locations`/`Space`, which is what the importer
  reads. §1.2.3.12: a project holds "Up to 16" installations.
- **[M]** A whole-document grep for `site|premises|campus` finds only the
  two `Ground` sentences above. The word `property` appears only as KNX
  interface-object property terms (e.g. `SystemProperty`, `AppProperty`),
  never as real estate.

**[I]** The schema's answer to "several buildings on one KNX
infrastructure" is: one `Installation` (one topology, one group-address
structure), whose `Locations` root is a `Ground` space holding several
`Building` spaces. The schema does not say this in so many words: it names
`Ground` as a normal root type (§1.2.6.3) and places no constraint on the
type of a child `Space` (§1.2.6.4), so `Building` under `Ground` is *not
forbidden* rather than prescribed, and the schema never defines what
`Ground` means. `Area` is also named as a root type. Its meaning is
not defined in the schema, and it must not be confused with a topology
area (§1.2.4).

### E2 — The KNX location model has a Site, and maps it onto ETS's building-structure root, not onto a new ETS type

*3/10/3 KNX IoT Information Model* v2.0.0 (`cf06f9f4ee69fa10…`):

- **[D]** §1.2.3.5 `loc:Site`: "A loc:Site represents a collection of
  buildings and grounds that belong to a given institution." Main object
  properties: `loc:hasBuilding`, `loc:hasSiteSegment`. Disjoint with
  `loc:Building`, `loc:Floor`, `loc:Space`. "The concept can be mapped to
  the IFC concept IfcSite."
- **[D]** §1.2.3.5.1 `loc:SiteSegment`: "a part of a ground, land or of a
  campus. It subdivides a site. A site segment is usually occupied by a
  building."
- **[D]** §1.2.1 (line 501–503): "A loc:Site is usually at the top of a
  location hierarchy". §1.5.1: `loc:hasBuilding/loc:isBuildingOf` is "A
  strong OP relationship to express a relation to building contained in a
  site", with Domain `loc:Building` and Range `loc:Site`, as printed.
- **[D]** §1.2.2 "Alternative Hierarchies": the common superclass allows
  "not strictly sticking to the usual hierarchy".

*3/10/4 KNX IoT 3rd Party API* v2.0.0 (`21f6788b5aacb27d…`), §1.2.5.2.2,
Table 10 "Location Types", column "Concept in MaC ETS":

- **[D]** `urn:knx:loc.site` / `loc:Site` → "Buildings (MaC root node)".
  `loc:Building` → "Building". `loc:Room` → "Room, Corridor, Stairway".
  `loc:Space` → "Building Part, Cabinet". The same clause says "the used
  type SHALL conform to the most specific MaC ETS concept as expressed in
  the last column."

*3/10/2 KNX IoT Constants* (`0946d7221cc40198…`), §2.2, Table 1 "Mapping of
terms":

- **[D]** Row "building/floor/room/site" → KIM "loc:Building/ loc:Floor/
  loc:Room/ loc:Site" → KNX Classic Installer "Building/Floor/ Room/-". The
  site column is a dash: KNX Classic (ETS) terminology has no named site
  term.

**[I]** The Standard knows the site concept, and it maps it onto ETS's
root of the building structure, the per-installation `Locations`
container, not onto a `Space/@Type`. KNXBench already has that root: the
list `Installation::buildings`, whose parentless entries are the roots. It
also already has `Ground` as a typed node. A user who wants a named site
node beneath the root can use `Ground`, the only site-like value the schema
offers. Neither document defines a KNX Classic site type that ETS would
write or read.

**Evidence boundary.** Tables 10 and 1 belong to the KNX IoT documents, not
to the project schema. They describe how an IoT server should present ETS
data. They do not define `.knxproj` content. The schema-23 PDF is the only
authority for the file format, and it offers `Ground`, not `Site`.

### E3 — The corpus: three real projects, each one installation with one `Building` root, no `Ground`

**[M]** A read-only inventory of each project part (`Installation`,
top-level `Space`/`BuildingPart` types, parent→child type edges):

| Project (sha256/16) | Schema | Installations | Root spaces | Type histogram |
| --- | --- | --- | --- | --- |
| `KV v2.5 - demo.knxproj` (`8f96b695e9f9d837`) | 21 | 1 | `Building` ×1 | `Building` 1 |
| `Unser Zuhause ets4 - 2025-12-15.knxproj` (`b3e61923488ad07e`) | 11 | 1 | `Building` ×1 | `Building` 1, `BuildingPart` 1, `Floor` 3, `Room` 14, `Corridor` 2, `DistributionBoard` 1 |
| `Unser Zuhause ets 6.3.0 - 2026-09-02.knxproj` (`6d3c1eb33b28b2e9`) | 23 | 1 | `Building` ×1 | same as schema 11 |

No real sample contains `Ground`, `Area`, more than one root, or more than
one installation. This matches [KNOWN_LIMITATIONS.md §1](../KNOWN_LIMITATIONS.md)'s
single-sample bias. Every claim about a `Ground` root is therefore backed by
the schema text (E1) plus synthetic tests, never by a real ETS export.

### E4 — KNXBench already represents all of it

**[M]** Code state at `main` `7b64496`:

- `knx_core::BuildingPartType` has had `Ground` since T13
  (`9b93c0a`, KNOWN_LIMITATIONS §89 resolved). Import maps `Ground`
  exactly. Native storage persists it as `kind TEXT` (store schema v9).
  Projection, the creation API and the EN/DE UI label it (German:
  "Grundstück", i.e. plot/property). `ProjectExplorer.tsx` already offers
  `Ground` in the "new building part" selector, at the root or nested.
- `BuildingPart` nests without a depth limit and references devices without
  owning them. `Devices` is the sole owner (DATA_MODEL §5).
  `knx-core`'s commands enforce no parent/child type rules, so a `Ground`
  root with `Building` children is already a valid, creatable, persistable
  structure.
- An `Installation` owns one `Topology` and one group-address structure.
  Several buildings under one site in one installation therefore share one
  KNX infrastructure by construction.
- Unknown `Space/@Type` values (e.g. `Site`) produce a `MapProblem`
  (`UnknownEnumValue { kind: "BuildingPart/@Type" }`), fall back to
  `BuildingPart`, and appear in the import report's errors. Unknown persisted
  kinds refuse to load (`StoreError::UnknownBuildingPartType`).
- **Gap found on the way (not decided here):** nothing can rename an
  `Installation` after creation. `Installation.name` is only set by
  `NewProjectDialog` and import, and no command changes it. The ETS
  `Installation/@Name` may be empty "if the project contains just one
  installation" (schema 23 §1.2.3.13).

## Decision

**No domain change. A site/property is represented as a `Space` of type
`Ground` at the root of an installation's building structure, with its
buildings as `Building` children.** This is outcome 3 of ISSUE-06 ("no
domain change with a documented explanation"), grounded in outcome 1's
evidence: the external type exists, is documented as a root type
(schema 23 §1.2.6.3), and is already modelled.

Concretely:

1. **No new `BuildingPartType` variant, no `Site` storage value, no new
   entity, no migration.** `CURRENT_SCHEMA_VERSION` and store schema v9 are
   unchanged. `Ground` is the site.
2. **One KNX infrastructure means one `Installation`.** Buildings that share
   a topology and group addresses sit under one `Ground` root of one
   installation. Buildings with separate infrastructures are separate
   installations (up to 16 per project, §1.2.3.12). The site is **not** a
   level above `Installation`.
3. **The site never owns devices, and is never a second owner of them.**
   Devices are owned by `Devices` and referenced by the building parts that
   contain them (DATA_MODEL §5). A device may be referenced directly by the
   `Ground` node, because the schema allows `DeviceInstanceRef` on any
   `Space`, e.g. an outdoor device on the plot. Grouping buildings under a
   site copies no device and moves no line membership.
4. **Import mapping.** `Space/@Type="Ground"` → `BuildingPartType::Ground`,
   exactly (unchanged). No other token is read as a site. In particular an
   undocumented `Site`, `Property` or `Campus` value stays an
   `UnknownEnumValue` map problem, with the existing reported `BuildingPart`
   fallback. It does not silently become `Ground`, per ISSUE-06's interface
   rule, now pinned by a test.
5. **Export.** None: KNXBench writes no `.knxproj`
   ([ADR-0028](0028-no-knxproj-export.md)). If an exporter ever returns,
   `Ground` is a schema-23 value and needs no loss warning. Schema 11 has
   the same element under `Buildings/BuildingPart`, and no schema-11
   enumeration was checked. That is left to that exporter's own evidence.
6. **Tree placement (for ISSUE-05, the UI that follows).** A `Ground` node
   is an ordinary root building part and renders where roots render today,
   under the installation's "Buildings" branch. The UI may *call* a `Ground`
   node "Site / Property" or "Grundstück/Liegenschaft" as a display label.
   That label is display text. It adds no model value and must not be
   written back as a type. ISSUE-05 must create sites and move buildings
   through the existing validated commands (`CreateBuildingPart`,
   `MoveDeviceToBuildingPart`, and the structural-move command that B10
   defines). It must not add a parallel path.

### What this decision does not claim

- It does not claim ETS 5/6 displays `Ground` as "site" or restricts where
  `Ground` may appear. No ETS sample contains a `Ground` space (E3). The
  schema says root spaces "will normally" be `Area`, `Building` or
  `Ground`. That is a description, not a constraint, and KNXBench enforces
  no nesting rule (unchanged).
- It does not claim that `Ground` and `loc:Site` are the same concept. E2's
  mapping is the IoT documents' mapping, and it points at the ETS *root
  node*, not at `Ground`. The decision rests on the *schema*'s naming of
  `Ground` as a top-level type. That is the smallest faithful use of a
  documented value. It is not a claim of semantic identity.
- It does not add a KNXBench-native `Site` above installations. See the
  alternatives below.

## Alternatives considered

**Add `BuildingPartType::Site` (a KNXBench-native kind).** Rejected. No
project schema defines `Site` (E1), so it would be a native-only type
duplicating a documented one (`Ground`). Once any exporter exists, it would
need either a loss warning or a lossy mapping back to `Ground`. Its only
benefit, the display word "site", is a label question that ISSUE-05 can
answer without touching the model.

**Add a native `Site` entity above `Installation` (grouping installations).**
Rejected for this issue, but recorded as the open alternative. The user's
case is "multiple buildings using *one* KNX infrastructure". One
infrastructure is one `Installation` (E1 §1.2.3.13), so a level above
installations would group *separate* infrastructures, a different request.
It would also be native-only data with no counterpart in any `.knxproj`, a
new table and migration, and a second tree root in every view. If a user
later asks to group separate installations, that needs its own ADR, with
the IoT "Buildings (MaC root node)" mapping (E2) as its starting evidence.

**Read an unknown `Site`/`Property` token as `Ground` on import.** Rejected
by ISSUE-06's interface rule and by AGENTS.md's rule never to discard
information silently. Guessing a meaning for an undocumented token is
inventing KNX behaviour. The token stays reported, and a test now pins that.

**Enforce "a `Ground` may only be a root and only hold `Building`s".**
Rejected. The schema says "normally", not "shall". 3/10/3 §1.2.2
explicitly allows alternative hierarchies. Rejecting a real ETS project
over a nesting rule the format does not impose would be data loss by
validation.

## Consequences

- **No code or schema change is required by this decision.** The branch
  adds characterization tests only. They pin the decision's premises so that
  a later change cannot quietly break them:
  - `knx-etsproj/tests/site_hierarchy.rs`: a schema-23 `Ground` root with
    two `Building` children and one shared line imports with no errors, one
    installation, both devices on one line, each referenced once. An
    undocumented `Site` root type is reported and never becomes `Ground`
    (mutation-checked: aliasing `"Site"` to `Ground` in
    `parse_building_part_type` makes it fail).
  - `knx-store` `a_ground_site_over_two_buildings_on_one_line_round_trips`:
    native save/load/re-save keeps the `Ground` root, both buildings and
    exactly two device references.
  - `knx-core` `a_ground_site_holds_two_buildings_and_a_device_moves_between_them`:
    the command path the UI will use creates the site, and moving a device
    between the two buildings leaves exactly one reference.
- **ISSUE-05 is unblocked with a fixed contract:** a site is a `Ground` root.
  Label it, do not re-type it. Prove shared-installation ownership through
  the commands above.
- **Recorded, not fixed:** an installation cannot be renamed in KNXBench
  today (E4). This is relevant when several installations are used for
  separate infrastructures. It is flagged for ISSUE-05/B10 triage, not
  treated as part of this decision.
- **Recorded, not fixed — and larger than the rename gap:** decision item 2
  describes a project shape KNXBench can *import and store* but not *edit*.
  Every `Command` mutates `installations[0]` only (`knx-core` `command.rs`,
  the `Command` doc comment; `CreateBuildingPart` uses
  `installations.first_mut()`). A second installation from an import is
  kept and saved, but no command can create, move or delete anything in it.
  Routing commands to a chosen installation is prerequisite to offering
  "separate infrastructures are separate installations" as an editing
  workflow; until then, it is a representation, not a feature.
- **No compatibility claim changes.** `Ground` has been covered by
  COMPATIBILITY.md since T13. This ADR adds only a documented *usage*, with
  synthetic (not real-sample) evidence.
