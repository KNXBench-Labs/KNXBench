# Project diff/compare — design

**Task:** T14 in `docs/GAP_ANALYSIS_ETS.md`. Closes gap **C1** ("No project
comparison/diff"), partially mitigates
[`KNOWN_LIMITATIONS.md` §9](../../KNOWN_LIMITATIONS.md#9-project-files-are-not-diffable)
("Project files are not diffable" — lifted, per that entry's own "Lifted
when", by "a textual export and import format", which this is not; T14 lifts
it a different way, by giving the *application* a diff instead of a text
format a version-control tool could diff on its own).

**Goal:** given two KNXBench projects, compute and show what a human changed
between them — entities added, removed, or modified — as a feature reachable
from the CLI, the HTTP API and the web UI, not merely as a test assertion.

---

## 1. The honesty problem, stated first

ETS can compare two project versions structurally. This design **does not
claim to reproduce that feature**, for the same reason T13's design would not
claim to reproduce ETS's printed reports: nothing in this repository describes
what ETS's compare output contains, and `OriginalData/` holds no ETS-produced
comparison of any kind to check against.

So, as with T13:

1. The output is **"KNXBench project diff"** or **"what changed"**, never
   described as an ETS comparison or a replacement for one, in code, docs, UI
   text or CLI output.
2. Its content is chosen from what this domain model can actually match and
   compare (§3), not from a remembered ETS comparison layout.
3. Where two entities cannot be matched at all, or two candidates tie on the
   only key available, the diff says so explicitly (§3) rather than guessing
   or staying silent — CLAUDE.md: never silently discard information.

If an ETS-produced comparison sample is ever obtained, comparing content sets
becomes possible. Until then, parity is unmeasurable and therefore unclaimable.

## 2. Why a new crate, and why `knx-etsproj::compare` stays exactly as it is

`knx-etsproj::compare` already answers a comparison question — but it is the
wrong one for a user-facing feature, and it lives in the wrong place.

**The wrong question.** `compare.rs`'s own module doc states its purpose:
"the declared semantic-equality relation (ADR-0007, IMPORT_EXPORT §9): what
'the roundtrip didn't change anything' means in this repository"
(`crates/knx-etsproj/src/compare.rs:1-2`). `semantic_view` and
`describe_difference` exist to answer one narrow question — *did import →
export → import preserve everything a roundtrip is supposed to preserve* —
for `roundtrip_model_is_semantically_equal` and its siblings
(`docs/IMPORT_EXPORT.md:293-298`, ADR-0007). Changing what fields
`SemanticDevice`/`SemanticComObject`/etc. include, or what "equal" means for
them, changes what fidelity means for every roundtrip test in the repository.
T14 asks a different question — *what did a human change between two saves* —
and answering it wants a different, and looser, notion of "the same entity":
matched primarily by identity (`SourceRef`), not by "every modelled field is
byte-identical". Folding both questions into one type would make every future
change to either question risk silently breaking the other.

**The wrong place.** `compare.rs` lives in `knx-etsproj`, the `.knxproj`
import/export crate. A user-facing diff feature has no business dragging in
a ZIP/XML archive parser, and — per the orchestrator's ruling — must not
depend on `knx-etsproj` at all, so it structurally cannot reuse `compare.rs`
even if reuse were otherwise desirable.

**The decision.** A new pure crate, `crates/knx-diff`, models the *matching*
approach `compare.rs` already pioneered (key every entity by an identity,
normalize collection order, diff field-by-field) but as its own declared
type, independent of the roundtrip crate, free to diverge where a diff's
job and a roundtrip oracle's job genuinely differ (§3 covers the concrete
divergences). This is the same "one crate per output-format/purpose"
precedent `knx-csv` and `knx-report` already set
(`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
§6).

**The duplication, stated plainly.** `knx-diff` and `knx-etsproj::compare`
will independently re-derive some of the same shapes — resolving a
communication object's `Override<Text>`/`Override<DptRef>` down to "the
value that was actually set and would be exported" is logic both crates need
(`compare.rs:356-379`'s `semantic_text`/`semantic_dpt`, reimplemented for
`knx-diff` in §3). This is accepted, not overlooked: the two crates answer
different questions for different audiences (an internal test oracle vs. a
user-facing report) and the alternative — one shared type serving both —
is exactly the coupling §2's "wrong question" paragraph explains is the
worse failure mode. Six lines of duplicated text-resolution logic per
comparison type is a small price for two independently changeable
questions.

## 3. Entity matching — the heart of the design

### 3.1 Why internal ids cannot be the key

`Project`'s internal ids (`DeviceId`, `GroupAddressId`, `GroupRangeId`,
`BuildingPartId`, `ComObjectInstanceId`, `ParameterInstanceId`,
`ModuleInstanceId`, `AreaId`, `LineId`) are synthetic counters, allocated
fresh by `IdAllocators` on every import or `Command::Create*`
(`crates/knx-core/src/project.rs:24-33`) — two independent loads of the "same"
project in a general sense need not agree on them at all. `InstallationId` is
the one exception: its own doc comment states it "mirrors ETS's own
installation number... rather than a synthetic counter: it is already stable
and project-unique in the source data" (`crates/knx-core/src/ids.rs:47-49`).
Installations are therefore matched directly by `InstallationId` — the only
entity in this design that is.

### 3.2 `SourceRef` and its persistence

Every other entity carries a `SourceRef { path, ets_id }`
(`crates/knx-core/src/ids.rs:15-19`), preserved end to end:

- Imported entities keep ETS's own `RefId` string as `ets_id`
  (`crates/knx-etsproj/src/map.rs`, throughout).
- `knx-store` persists it verbatim as `source_ets_id`
  (`crates/knx-store/src/group.rs:24-25`, `crates/knx-store/src/devices.rs:32`
  `upsert_device`'s `source_path`/`source_ets_id` columns).
- Entities created inside the application (no ETS origin) get a synthetic
  but *stable* `ets_id` of the form `KB-<Type>-<id>` —
  `KB-GA-<id>` for a group address (`apps/knx-server/src/domain.rs:865-866`),
  and the same pattern for areas, lines, ranges, building parts
  (`domain.rs:904-905`, `:942-943`, `:1001-1002`, `:1070-1071`) and devices
  (`domain.rs:1416-1417`).

So `ets_id` is always present and — for a given `.knxdb` file reloaded, or a
project re-saved after edits without touching that entity — stable. It is
**not** guaranteed stable across an independent *re-import* of the same
source `.knxproj`, since ETS may regenerate `RefId`s; T14 does not attempt to
detect that case specially (§9).

### 3.3 The matching algorithm

For each entity collection (areas within an installation, lines within an
installation, devices within an installation, group ranges within an
installation, group addresses within an installation, building parts within
an installation; communication objects and parameters within a matched
device — §3.5):

1. **Ets_id pass.** Build a map from `ets_id` to entity for each side. Any
   `ets_id` present on both sides is a match — regardless of whether every
   field still agrees; field agreement is what `changed` vs. unchanged
   means, not what matching means.
2. **Natural-key pass**, over whatever is left unmatched on both sides after
   step 1. Build a map from the entity's natural key (§3.4) to entity, for
   each side's *leftover* set only. A key present in both leftover sets with
   **exactly one** candidate on each side is a match.
3. **Ambiguity.** A natural key with more than one leftover candidate on
   either side is not guessed at. Every one of those candidates is instead
   reported as `removed` (left side) / `added` (right side) individually,
   *plus* one `AmbiguityNote` naming the key and how many candidates
   collided on each side — so the person reading the diff knows "these looks
   like adds and removes" might actually be one rename the natural key
   could not disambiguate, rather than the diff staying silent about the
   possibility.
4. **Leftover.** Whatever is still unmatched after steps 1–3 is `removed`
   (left only) or `added` (right only).
5. **Matched pairs** are diffed field-by-field (§3.5); a pair with zero
   differing fields produces no output at all — it is not "unchanged", it is
   simply absent from the diff, the same convention `git diff` uses.

### 3.4 The natural key, per entity type

| Entity | Key | Notes |
| --- | --- | --- |
| Installation | `InstallationId` (u8) | Not `ets_id`/natural-key matched at all — §3.1. |
| Area | `address` (u8) | Unique within an installation's topology. |
| Line | `(area address, line address)` | A `LineId` does not store its area; resolved via `Topology::area_of` (`topology.rs:53-55`), the same lookup `compare.rs` does not need but this design does, because a line's *placement* is exactly the kind of thing a diff — unlike a roundtrip oracle — must be able to say moved. Scoped by area because a line address (e.g. `1`) repeats across areas. |
| Device | individual `address` (`IndividualAddress`) | **Only when `Some`.** A device with no individual address has no natural key at all; if its `ets_id` doesn't match either, it cannot be correlated and surfaces as an unrelated add+remove pair. Recorded as a named limitation (§9), not hidden. |
| Group address | the 16-bit `address` value | Always present (it is the address itself), unique within an installation. |
| Group range | `(start, end)` | Both main and middle ranges live in one flat `Installation::group_ranges` list (`group.rs:12-23`); a range's *own* span is a stronger, always-present identity than trying to scope middle ranges under an already-matched parent (which would fail entirely for a range whose parent itself didn't match). A range's `parent` is instead a **diffable field**, resolved to the parent's own matched key, exactly like a line's area (§3.5) — so "range moved to a different parent" is reported as a field change, not a match failure. |
| Building part | **path of names from the root**, e.g. `["Building A", "Floor 1", "Room 3"]` | Built by walking `parent` to the root through a `BTreeMap<BuildingPartId, &BuildingPart>` lookup, the same technique `compare.rs:429-459`'s `semantic_building_part` already uses for a single parent hop, extended here to the full chain. Two siblings sharing a name under the same matched parent produce the same path and therefore collide under the ambiguity rule (§3.3 step 3) — this is a real, if narrow, limitation of a name-based key, recorded in §9 alongside the identical limitation `compare.rs` accepts for the single-hop case. |
| Communication object | `(matched device's key, number)` | Requires the owning device to already be matched (§3.5); an object under an unmatched device is not diffed individually — its device's own add/remove already accounts for it. `number` comes from the `RefId`'s `_O-<n>` segment (`device.rs:58`) and is stable per device regardless of module vs. monolithic program shape. |
| Parameter instance | `(matched device's key, parameter's own `ets_id`)` | Not one of the orchestrator's explicitly enumerated keys; reasoned out below (§3.6). `ParameterInstance.source.ets_id` is the imported `RefId` itself (`parameter.rs:12-14`), already the "ets_id" case, so there is no separate natural-key fallback for parameters — an unmatched `ets_id` here is a genuine add/remove, not a candidate for guessing. |

### 3.5 What gets diffed once two entities are matched

Field lists mirror `compare.rs`'s `SemanticX` types where the entity's
question is the same ("what changed" needs the same modelled-attribute set
"is this the same after a roundtrip" needs), with two deliberate additions
neither `compare.rs` nor a roundtrip question needs, both already justified
above: a matched line's/range's **placement** (area / parent), and a matched
device's **line and building-part placement**, resolved the same way
(`Topology::area_of`, and a reverse lookup from `BuildingPart::devices` — no
such reverse index exists yet in any crate; `knx-diff` builds its own, a
`HashMap<DeviceId, BuildingPartId>` used only internally, never driving
output order per §4).

- **Project info** (singleton, no matching needed): `name`, `project_number`,
  `group_address_style`, `completion` (`project.rs:143-165`). `project_id`
  is excluded — it is the container's own identity string, comparing two
  files that are not the same container is the entire point of a diff, so
  it would differ on every legitimate comparison and add only noise.
- **Installation**: `name`, `multicast_address`, `completion`, `default_line`
  (resolved to the line's own matched key).
- **Area**: `name`, `completion`.
- **Line**: `name`, `medium_ref`, `domain_address`,
  `domain_address_is_checked`, `ip_routing_multicast_address`,
  `multicast_ttl`, `completion`, **placement** (area address).
- **Device**: `name`, `description`, `address`, `product_ref`, `program_ref`
  (printed as opaque identifiers, exactly like T13 — resolving them needs
  `knx-productdb`, which `knx-diff` must not reach), `commissioning`
  (`CommissioningState`, compared as a whole struct — `completion` plus the
  five `*_loaded` flags plus `broken` are all engineering-relevant to "what
  changed"), **line placement**, **building-part placement** (resolved to
  the building part's own path key, `None` for a device in no building
  part).
- **Group range**: `name`, `start`, `end`, **parent placement**.
- **Group address**: `name`, `central`, `unfiltered`, **range placement**
  (resolved to the range's own matched key, `None` for no range).
- **Building part**: `name`, `number`, `kind`, `completion`, `default_line`
  (resolved to the line's own matched key). `devices` is **not** a field
  here, for the same reason `compare.rs` excludes it from
  `SemanticBuildingPart`'s comparison basis (a relationship owned by the
  device side, not the building part) — a device's *own* placement change
  is what reports a device moving building parts, once per device, not once
  per building part.
- **Communication object**: `text`, `description`, `dpt` — each resolved
  through `Override<T>`/`Layer::is_exported()` exactly like
  `compare.rs:356-379`'s `semantic_text`/`semantic_dpt`, reimplemented
  directly against `knx_core::StringTable` (five lines; see §7 for why this
  does not justify a `knx-projection` dependency), the five flags
  (`ResolvedFlags`, same `Override<bool>` collapse `compare.rs:389-393`
  documents and accepts), `links` (each resolved to `(group address's own
  matched key, Direction)`, sorted), `module_instance` (resolved to the
  module instance's own `ets_id`, `None` for a schema-11 device).
- **Parameter instance**: `raw` only — an uninterpreted string diff, nothing
  more (§3.6).

### 3.6 Deciding parameter values are in scope

The brief's default is out-of-scope "if the model does not carry them
reliably". Checked: `crates/knx-core/src/device.rs` has no `ParameterInstance`
at all (it lives in `crates/knx-core/src/parameter.rs`, referenced from
`Installation::parameters`, `installation.rs:26`). The one `parameters:
vec![]` hit in `crates/knx-etsproj/src/infer.rs:226` is inside that module's
own `empty_project()` test fixture, not evidence that real import skips
parameters — the actual mapper, `crates/knx-etsproj/src/map.rs:627-637` (and
again at `:1096-1106` for the module-based shape), populates
`ParameterInstance` for every real import, and `compare.rs`'s own
`SemanticParameter`/`semantic_parameter` already treats `raw` as reliable
enough to gate roundtrip-equality tests on (`compare.rs:146-151`,
`:461-471`). So the model does carry these values reliably, from real
imports, in both device shapes.

Given that, excluding them would be the "silently discard information"
CLAUDE.md forbids: a parameter's raw value is exactly the kind of thing that
can legitimately change between two saves, and `parameter.rs`'s own doc
already sets the precedent for what "reliable but uninterpreted" data looks
like in this codebase (RESEARCH R3) — the same posture T13 takes toward
`product_ref`/`program_ref` (§3.5 above): show it verbatim, label it as
what it is, never claim to interpret it. So `knx-diff` reports a changed
`raw` value as `raw` on both sides, nothing decoded, no parameter name
resolved (none exists yet — `RefId`-to-name resolution needs the product
database's `Dynamic` parameter grammar, unresearched per RESEARCH R3).

### 3.7 The testable property

`diff_projects(&p, &p)` — the same `Project` value, or two independent loads
of the same unmodified `.knxdb` file — produces a `ProjectDiff` in which
every `added`, `removed`, `changed` and `ambiguous` list is empty, at every
level, for every entity type. This is the design's own correctness anchor
and is named explicitly as a test in §8.

## 4. Determinism and ordering

Same discipline as `knx-report` (`crates/knx-report/src/lib.rs:8-15`) and the
same reason: the CLI's plain text and the HTTP route's JSON must be
byte-identical for the same two inputs on every run.

- Every `added`/`removed`/`changed`/`ambiguous` list is sorted by the
  entity's own display key — the matched pair's `ets_id` (left side's, when
  it was the basis of the match; either side's, arbitrarily but
  consistently the left's, when matched by natural key with differing
  `ets_id`s) for `ets_id`-keyed entities, the natural key's own `Ord` for
  everything else (an address, a `(area, line)` pair, a path of names —
  all of which are already `Ord` types or trivially made so). Never
  `HashMap` iteration order.
- `diff_projects` takes no clock, no filesystem, no RNG: `&Project` in,
  `ProjectDiff` out, nothing else. There is nothing in this design that
  could need a caller-supplied "now" the way `knx-report::ReportOptions`
  does — a diff describes two projects, not a moment in time — so there is
  no `DiffOptions` type at all.
- The internal `HashMap<DeviceId, BuildingPartId>` reverse index (§3.5) is a
  lookup structure, never an iteration source for output — the same
  distinction `knx-report`'s design draws (§4 there) between an index and a
  walk that emits document text.

## 5. Rendering is a separate question from computing

`knx-diff` renders nothing. `diff_projects` returns typed Rust values;
turning them into text is each surface's own job, matching how `knx-csv` and
`knx-report` already keep their own domain types serde-free (neither
`knx_report::HtmlReport` nor `ReportWarning` derives `Serialize` — the server
wraps them in its own DTOs, `apps/knx-server/src/routes.rs:437-455`).

- **CLI: plain text.** One line per change, grouped by section, in the
  `print_doc_export_report`/`print_export_report` style
  (`apps/knx-cli/src/main.rs:467-473`, `:562-572`) — `+`/`-`/`~` prefixes for
  added/removed/changed, one `changed_fields` summary per changed line, one
  line per ambiguity note. No JSON on the CLI: nothing there consumes it
  programmatically today, and hand-rolled text is what every other CLI
  report in this codebase already does.
- **HTTP: JSON.** The web UI needs structured data to group and (eventually)
  filter a result set that can run into hundreds of entries on the reference
  project's scale (36 devices, 907 communication objects, 514 group
  addresses); DTOs live in `apps/knx-server/src/routes.rs`, `#[derive(Serialize)]`,
  `camelCase`, one `From<&knx_diff::X>` per `knx-diff` type — the exact
  pattern `DocumentationWarningDto`/`DocumentationExportReportDto` already
  establish (`routes.rs:437-457`).
- **No HTML, and no `knx-report` dependency, in `knx-diff` itself.** Nothing
  in the T14 backlog line asks for a printable/exportable diff document —
  it asks for "a report" in the sense of a feature surfacing the
  information, which JSON-into-a-web-panel and plain-text-on-a-terminal
  already are. Growing an HTML renderer inside `knx-diff` the way
  `knx-report` has one would be exactly the scope creep CLAUDE.md and this
  brief both warn against, and would additionally force a `chrono`/timestamp
  concept into a crate that currently needs neither. **If** a "printable
  diff document" is wanted later, the honest seam is the other direction:
  `knx-report` (which may depend on other pure crates) gains
  `render_diff_html(&knx_diff::ProjectDiff, &ReportOptions) -> String` and a
  `knx-diff` dependency of its own — the computation stays in `knx-diff`,
  only the rendering choice moves. Recorded in §9, not designed further
  here.

## 6. Where the code lives

A new crate, `crates/knx-diff`, depending on **`knx-core` only**.

**Why not `knx-projection`, even though the orchestrator flagged it as
possible "if genuinely useful".** T13 needed `knx-projection` for one
specific reason: `build_device_detail`/`build_com_object_node`
(`crates/knx-projection/src/lib.rs:314`, `:330`) already resolve a
communication object's text/description/DPT through the string table, and
re-deriving that in `knx-report` would have duplicated genuinely fiddly
logic. `knx-diff` needs the identical five-line resolution
(`strings.text(&resolved.value, strings.default_language())`) — but that
exact logic already has a second, independent implementation directly
against `knx_core::StringTable`, with no complaint about duplication: 
`compare.rs:356-379`'s `semantic_text`/`semantic_dpt`, in a crate that also
does not depend on `knx-projection`. That is the closest sibling precedent
there is for this exact question, and it answers it: resolving `Override<T>`
down to its exported value is small enough, and stable enough, to write
directly against `knx-core` twice rather than add a dependency once. Nothing
else in this design needs `knx-projection`'s per-`DeviceId`-shaped lookups
(`build_device_detail` takes one device id and returns one detail record;
`knx-diff` needs to walk and match *pairs* across two independent
`Project`s, a shape `knx-projection`'s API does not offer and there is no
reason to bend it toward). So: `knx-core` only.

`xtask check-layering` (`xtask/src/main.rs`) gains a matching rule, in the
same shape as `knx-csv`'s and `knx-report`'s two-part rule
(`xtask/src/main.rs:74-97`):

- `knx-diff` must reach none of `knx-store`, `knx-etsproj`, `knx-productdb`
  — the ruling's own constraint, restated as an enforced gate, not just a
  `Cargo.toml` omission someone could reintroduce by accident.
- `knx-diff` must reach none of `layering::CORE_FORBIDDEN` — same reasoning
  as `knx-report`'s: a comparison engine has no business with a database, an
  archive parser, or an async runtime.

Internally the crate splits into `key.rs` (natural-key computation and the
matching algorithm of §3.3, generic enough to serve every entity table
without six copy-pasted loops), `semantic.rs` (the per-entity field
extraction of §3.5, playing the same role `compare.rs`'s free functions do,
independently), and `diff.rs` (the top-level `diff_projects` entry point
that ties installations, then each entity table within each installation,
together). `lib.rs` stays a thin two-line wrapper, matching `knx-report`'s
own `lib.rs:30-32`.

Public surface (illustrative — the shared `added`/`removed`/`changed`/
`ambiguous` shape repeats once per entity table; only `Device` and
`GroupAddress` are spelled out fully below, the rest follow the field lists
in §3.5 and the same generic shape):

```rust
pub fn diff_projects(left: &Project, right: &Project) -> ProjectDiff;

pub struct ProjectDiff {
    /// Empty when `left`/`right` agree on every field in §3.5's "Project
    /// info" list.
    pub info_changes: Vec<FieldChange>,
    pub installations: Vec<InstallationDiff>,
}

pub struct InstallationDiff {
    pub id: u8,
    pub status: EntityStatus, // Added | Removed | Matched
    /// Only populated when `status == Matched`.
    pub field_changes: Vec<FieldChange>,
    pub areas: EntityTable<AreaKey, AreaFields>,
    pub lines: EntityTable<LineKey, LineFields>,
    pub devices: EntityTable<DeviceKey, DeviceFields>,
    pub group_ranges: EntityTable<GroupRangeKey, GroupRangeFields>,
    pub group_addresses: EntityTable<GroupAddressKey, GroupAddressFields>,
    pub buildings: EntityTable<BuildingPartKey, BuildingPartFields>,
}

/// One row's shared shape, repeated per entity type per §3.3.
pub struct EntityTable<K, F> {
    pub added: Vec<(K, F)>,
    pub removed: Vec<(K, F)>,
    pub changed: Vec<EntityChange<K, F>>,
    pub ambiguous: Vec<AmbiguityNote<K>>,
}

pub struct EntityChange<K, F> {
    pub key: K,
    pub matched_by: MatchKind, // EtsId | NaturalKey
    pub left: F,
    pub right: F,
    /// Names of the `F` fields that actually differ, e.g. `["name",
    /// "commissioning"]` — the quick-glance summary every surface prints
    /// first (§5).
    pub changed_fields: Vec<&'static str>,
}

pub struct AmbiguityNote<K> {
    pub key: K,
    pub left_candidates: usize,
    pub right_candidates: usize,
}

pub enum EntityStatus { Added, Removed, Matched }
pub enum MatchKind { EtsId, NaturalKey }

pub struct DeviceKey {
    pub ets_id: Option<String>,
    pub address: Option<String>, // formatted `IndividualAddress`, when present
}

pub struct DeviceFields {
    pub name: String,
    pub description: Option<String>,
    pub address: Option<String>,
    pub product_ref: String,
    pub program_ref: String,
    pub commissioning: CommissioningState,
    pub line: Option<LineKey>,
    pub building: Option<Vec<String>>, // the matched building part's path
    pub com_objects: EntityTable<ComObjectKey, ComObjectFields>,
    pub parameters: EntityTable<ParameterKey, ParameterFields>,
}

pub struct GroupAddressKey {
    pub ets_id: Option<String>,
    pub address: String, // formatted per each side's own GroupAddressStyle
}

pub struct GroupAddressFields {
    pub name: String,
    pub central: bool,
    pub unfiltered: bool,
    pub range: Option<GroupRangeKey>,
}
```

`ComObjectKey`/`ComObjectFields`/`ParameterKey`/`ParameterFields` follow
§3.4/§3.5 the same way; nested inside `DeviceFields` rather than at
`InstallationDiff` level, per §3.4's cascading rule.

One address-formatting note worth stating outright: `GroupAddress::format`
takes the project's own `GroupAddressStyle` (`address.rs:164`). Left and
right sides format their own addresses under their own project's style —
if the two projects disagree on style, every address string differs
cosmetically even when the underlying 16-bit value is identical. `knx-diff`
matches by the raw 16-bit value (§3.4), not the formatted string, so this
cannot cause a false add/remove — but a formatted `GroupAddressKey.address`
that looks different between `left`/`right` DTOs on an otherwise-unchanged
address is expected, not a bug, and worth a one-line note in whatever UI
renders it.

## 7. What the two sides of the comparison are

The brief's phrasing ("two `.knxbench` project files") maps to this
repository's actual extension: `.knxdb`, the SQLite file `knx-store` reads
and writes (`domain.rs:1-14`'s own module doc: "`open_native_project`
persist/restore this app's own project state as a `.knxdb` SQLite file").

**In scope for T14:**

- **CLI: two `.knxdb` files.** `knx diff <a.knxdb> <b.knxdb>` loads both
  independently via `knx_store::open_and_migrate` + `knx_store::load_project`
  — the exact two calls `run_doc_export` already makes for one store
  (`apps/knx-cli/src/main.rs:523-537`), made twice.
- **Server: the open project (left) against a `.knxdb` file (right).**
  `POST /api/project/diff { path }` compares `AppState`'s live, possibly
  edited, in-memory project — not a re-read from `store_path` — against the
  `.knxdb` at `path`. This is deliberate and, unlike `export_project`'s
  pre-T10 bug (`GAP_ANALYSIS_ETS.md` C4's account of stale-disk-read), not a
  staleness risk here: comparing "what I have open, edits and all" against
  "what's on disk" is a real, useful question ("what would Save change"),
  distinct from comparing two files. `path` is resolved with
  `resolve_project_path` (a *read* of an existing file, same function
  `import_project`/`open_native_project` use, `routes.rs:182`, `:192` —
  **not** `resolve_new_project_path`, which is for write targets that need
  not exist yet).
- **Web: a button that picks a `.knxdb` file and calls the route above.**

**Deliberately out of scope for T14** (§9 restates these):

- **Either side being a raw `.knxproj`.** Would need the surface layer (not
  `knx-diff`, which must not depend on `knx-etsproj`) to import it first,
  doubling the failure modes a comparison route has to explain (a bad
  `.knxproj` fails for import reasons; a bad `.knxdb` fails for store
  reasons) for a use case the backlog line does not ask for ("what changed
  between these two **saves**" — a save is a `.knxdb`, not a `.knxproj`).
- **Comparing the open project against itself under a different name**, or
  any other combination not listed above.

**A concrete correctness gotcha found while designing this, not invented:**
`knx_store::open_and_migrate` "Opens (creating if absent) the SQLite file at
`path`" (`crates/knx-store/src/migration.rs:357-361`). Handed a `path` that
does not exist, both the CLI and the server route must check
existence themselves *before* calling it — otherwise a typo'd comparison
target silently becomes an empty, freshly created `.knxdb`, and the diff
reports every entity in `left` as "removed" instead of failing with "file
not found". This is a pre-existing sharp edge `open_native_project` and
`run_doc_export` already carry; T14 does not fix it for them, but must not
inherit it for its own two new call sites.

## 8. Testing

- **`key.rs` unit tests**: the three-pass algorithm (§3.3) over small
  hand-built collections — an `ets_id` match wins even when every other
  field differs; a natural-key match is found only among leftovers; two
  leftover candidates sharing a natural key produce an `AmbiguityNote` and
  land both entities in `added`/`removed`, never in `changed`; an
  address-less device with no `ets_id` match is an unrelated add+remove.
- **`semantic.rs` unit tests**: each entity's field extraction against a
  hand-built `Project` (a `testutil.rs`, same convention `knx-csv`/
  `knx-report` each keep); `Override::Value` at `Layer::Program`/`Inferred`
  resolves to `None` in `DeviceFields`/`ComObjectFields` the same way
  `compare.rs`'s tests already lock for its own types; `Layer::Instance`/
  `UserEdit` resolve to `Some`.
- **`diff_projects` unit tests, the design's own anchor property (§3.7):**
  `diff_projects(&p, &p)` is empty at every level, for a project containing
  at least one of every entity type this design covers, including a
  module-based device (schema ≥ 21) and a monolithic one (schema 11).
- **Determinism test**: `diff_projects` run twice on the same two inputs
  produces equal `ProjectDiff` values (derive `PartialEq` throughout) —
  catches a `HashMap`-driven ordering bug the same way `knx-report`'s
  equivalent test does.
- **A genuine two-project test**: hand-build `left`, clone it, apply one
  change per entity kind to the clone with `knx_core::Command` where a
  `Command` exists for that field and a direct struct mutation where it
  does not (e.g. `product_ref`, which has no `Command`), and assert each
  change appears exactly once, in the right table, with the right
  `changed_fields`.
- **Corpus-gated integration test**, in `crates/knx-app/tests/` and not in
  `knx-diff`, for the identical reason T13's does
  (`docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`
  §8's citation of `xtask/src/layering.rs:94-98`: the layering gate walks
  dev-dependency edges too, so a `knx-etsproj` dev-dependency on `knx-diff`
  to build its "two independently imported copies of the reference project"
  fixture would trip `knx-diff`'s own layering rule if placed inside
  `knx-diff` itself). Imports the reference `.knxproj` twice into two
  independent `Project`s and asserts `diff_projects` between them is empty
  — the strongest form of §3.7's property, run against real, large,
  ETS-shaped data (36 devices, 907 communication objects, 514 group
  addresses) instead of a hand-built fixture.
- **Server test**: the route against a live project plus a written `.knxdb`
  fixture, including "no project open" and "comparison path does not exist"
  rejections.
- **CLI test**: the built binary against two small hand-built `.knxdb`
  files, both the "identical" and "one field changed" cases, asserting the
  printed text names the change.
- **Frontend test**: the button and result panel — cancelled picker calls
  nothing, a successful diff with zero changes says so, a diff with changes
  renders grouped counts, a rejected comparison surfaces the error.

## 9. Out of scope, and recorded as such

- **Any ETS-comparison parity claim** — no sample exists (§1).
- **Comparing against a raw `.knxproj`** — needs an importer step outside
  `knx-diff`'s reach and outside what "these two saves" asks for (§7).
- **Merging or applying a diff.** T14 computes and shows; it does not turn a
  `ProjectDiff` back into a `Command` sequence that could replay one
  project's changes onto another. A real feature, a materially larger one
  (every field-level change would need an inverse `Command`, and some — a
  device's `product_ref`/`program_ref` — have none today), not asked for by
  the backlog line.
- **Three-way comparison** (a common ancestor plus two divergent saves,
  the way a VCS merge does it). Nothing in this codebase tracks project
  ancestry or a common base to diff against.
- **Version history / time travel inside the store.** `knx-store` has no
  concept of "the project as of save N" — only "the project as it is now".
  A diff needs two whole files today; a history feature is a separate,
  much larger undertaking (a new storage concern, not a comparison one).
- **Detecting an ETS re-import's regenerated `RefId`s as "the same
  entity"** — §3.2 already flags this as a case the natural-key fallback
  may or may not catch, depending on whether the natural key still lines
  up; no special-case re-import detection is built.
- **An HTML/printable diff document** — the seam for adding one later is
  named in §5, not built now.
- **A CI-friendly "exit nonzero if anything differs" CLI flag** — `knx diff`
  exits `0` whenever it successfully produces a comparison, mirroring
  `run_doc_export`'s reasoning (`apps/knx-cli/src/main.rs:554-559`): a diff
  with changes is not a failed diff. A real, small feature request the day
  someone actually wants scripted gating; not this task.
- **A rich visual/side-by-side diff view in the web UI.** The panel
  specified in §8 is a grouped list, the same visual register the existing
  Log tab already uses — no tree view, no inline before/after text
  highlighting.
- **Fixing `open_native_project`/`run_doc_export`'s own pre-existing
  create-if-absent sharp edge** (§7) — named, not silently inherited for
  T14's own new call sites, not retrofitted onto the existing ones.
