# The parameter editor (T18, third slice)

## Status and scope

**Status:** design, 2026-09-11. Follows the first slice
(`2026-09-11-dynamic-tree-parse-and-evaluate-design.md`, decisions D1-D11)
and the second (`2026-09-11-module-expansion-design.md`, decisions D12-D19).
Both are merged to `main`; nothing outside `knx-productdb`'s own tests calls
`evaluate` today (`grep -rn "dynamic::" crates/ apps/ --include=*.rs` finds
callers only inside `crates/knx-productdb`).

Decision numbering continues at **D20**.

**In scope — the smallest coherent vertical slice a user can see:** open a
device that has a program, see its parameters (including every `Module`
instantiation, each shown as its own channel), change one value, see which
other parameters and communication objects are now active as a result, and
see any diagnostic the evaluator raised, without it reading like a debugger.

**This is not the whole editor, and it does not try to be.** The brief's
own instinct is right: a full parameter editor is too large for one branch.
This design draws the boundary at **read everything, write only what the
current data model can store unambiguously** — decided in D24/D25 below,
not assumed here. Two further, explicitly out-of-scope pieces this
boundary produces:

- **Editing a `Module`-scoped (per-channel) value.** The premise this
  bullet opened with in the first draft of this design — that
  `ParameterInstance` "has nowhere to store twelve independent channel
  values" — is false, and re-measuring it changes what this bullet says.
  ETS already puts the instantiation *inside* the stored `ets_id` string,
  not in a separate column: `<Module/@Id>_MI-<k>_<declared ParameterRef's
  own suffix>` — e.g. `M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1` for
  `Module/@Id M-00FA_A-2504-10-C071_MD-2_M-4`
  (`RefId="M-00FA_A-2504-10-C071_MD-2"`) and declared `ParameterRef`
  `M-00FA_A-2504-10-C071_MD-2_P-1_R-1`. **[V], corpus-observed:** the KV
  v2.5 demo project's `ParameterInstance` table already stores exactly
  this shape for all nine of its `ParameterInstanceRef` rows, and five of
  the nine are five *distinct* values — 17, 33, 49, 32, 48 — for the
  single declared `ParameterRef` `M-00FA_A-2504-10-C071_MD-2_P-1_R-1`, in
  five different `Module` instantiations (`_M-2_` through `_M-6_`).
  Per-channel values are not a hypothetical this slice has to design
  storage for; they are already in the repository's own corpus, in the
  table this slice already reads (`crates/knx-store` `ParameterInstance`,
  keyed by `(device, ets_id)`, `crates/knx-core/src/parameter.rs`,
  unchanged). The two other demo projects (Unser Zuhause ETS 6.3.0 and
  ETS 4, 1343 and 1390 `ParameterInstanceRef` rows respectively) store
  none of theirs module-qualified — every one matches a declared
  `ParameterRef` id verbatim — so this is a real but bounded corpus shape,
  not the common case; see D21 for the exact decomposition this slice
  adds to read it.

  What actually blocks a *write* is one level down, in the evaluator, not
  in storage: `crates/knx-productdb/src/dynamic/evaluate.rs:797` resolves
  a `choose`'s controlling value with `values.get(id)`, where `id` is the
  bare declared `ParameterRef` id and `ValueMap` is `HashMap<String,
  String>` (`evaluate.rs:348`) — one slot per declared id, project-wide,
  with no scope in the key. A flat map cannot hold KV's five independent
  values for one declared ref at once, so a `choose` inside a `ModuleDef`
  takes the same branch in every instantiation regardless of what the
  project stores per channel. Making a per-channel value actually *drive*
  evaluation means changing the merged, tested evaluator's value lookup —
  a separate slice with its own design (see D21's ruling on `supplied`,
  and the Non-goals entry below). This slice reads and displays a
  per-channel value where the corpus already provides one (D21's
  decomposition, D22's per-section attachment); it does not write one
  (D25) — not for lack of somewhere to put it, but because a write the
  evaluator cannot see is worse than no write. Named as follow-on work
  below.
- **Deep format validation for `Float`/`Text`/`IPAddress`/`Picture`/`Raw`
  parameter kinds.** Only `Number` (bounds) and `Restriction` (enum
  membership) have columns this database actually carries
  (`min_inclusive`/`max_inclusive`, `parameter_type_enum`). The other five
  kinds get a non-empty-string check only. Inventing IPv4/picture/raw-byte
  format rules with no spike behind them is exactly the kind of unresearched
  assumption the brief asks to keep out of acceptance criteria.

**Also explicitly out of scope**, forced by the brief rather than by new
research:

- **Schema changes.** The product database stays at v3. The device-side
  `parameter_instance` table (already migrated, `crates/knx-store`) is
  unchanged. If a later slice needs the module-scope key above, that is its
  own decision with its own migration.
- **The import path.** ADR-0014 stands; import still never evaluates
  `choose`/`when`, and this slice does not touch `crates/knx-etsproj`.
- **Commissioning, download, anything needing KNX hardware.**

## Evidence

The `Dynamic` structural grammar is corpus material, not Standard text
(RESEARCH.md §4.3/§4.4); this slice adds no new spike, and the two
knowledge bases under `/mnt/daten-i/Sourcecode/knx-spec-kb/knowledge_base/`
were not queried for it — a UI-and-wiring slice over an already-researched
evaluator has no new grammar question to ask them, and padding this section
with unrelated Standard citations would misrepresent what grounds these
decisions. The facts below are either already-cited corpus evidence
(carried forward, not re-derived) or plain code facts, each marked which.

- **[V], carried forward from RESEARCH.md §4.4 Q3.** One `ModuleDef`
  (`prod3`, `M-0083_A-0317-31-7DC6_MD-1`) declares 208 `ParameterRef` ids
  and 3 `Argument`s; the owning application program has four such
  `ModuleDef`s. A single device's active parameter set is therefore on the
  order of hundreds of fields, not the low thousands the *project-wide*
  tables (514 group addresses, 907 communication object instances,
  ARCHITECTURE.md §7) are. That scale difference is why this design does
  not propose pagination for one device's parameter panel — see D22.
- **[V], carried forward from GAP_ANALYSIS_ETS.md's T18 entry.** Slice 2's
  own corpus regression: `prod3`'s three module-bearing programs go from
  22/18/14 activations (program tree only) to 382/258/134 (with expansion).
  Confirms a device with modules genuinely needs the per-scope grouping
  D23 designs, not just a flat list — the ungrouped count is over an order
  of magnitude larger than the scoped one per program.
- **Code fact.** `resolve_values(conn, program_id, supplied) -> ValueMap`
  already implements exactly the precedence D21 needs (`supplied` wins,
  then `parameter_ref.value`, then `parameter.value`) — nothing about
  precedence is a new decision, only which map plays the role of
  `supplied`.
- **Code fact.** `crates/knx-productdb/src/query.rs` has no
  parameter-shaped read function today (`ComObjectView`/`com_object_view`
  is the only merged view; grepped for `struct Parameter`, `fn parameter`
  — nothing). D20/D22 add one, following `com_object_view`'s own
  `pick()`/`ValueLayer` shape.
- **Code fact.** `apps/knx-server/src/routes.rs` already has a precedent
  for a response DTO that is *not* a `knx-projection` type and carries no
  `ts-rs` binding: `CatalogInstallReportDto`, a plain
  `#[derive(serde::Serialize)] #[serde(rename_all = "camelCase")]` struct
  converted from a `knx-productdb` type with a hand-written `From` impl.
  D20 follows this precedent rather than inventing a new one.
- **Code fact.** `xtask/src/main.rs`'s `check_layering` forbids
  `knx-productdb` from reaching `knx-etsproj` or `knx-store`, and forbids
  `knx-projection` from reaching `CORE_FORBIDDEN`
  (`serde_json`/`quick-xml`/`rusqlite`/`tokio`/`axum`/`tower`) — and
  `knx-productdb` itself depends on `rusqlite`. `knx-projection` can
  therefore never depend on `knx-productdb`, transitively or otherwise.
  `apps/knx-server` has zero entries in `check_layering` — it is
  Infrastructure's consumer, the same position ADR-0017 argues from for
  the `knx-net` edge. `apps/knx-server/Cargo.toml` already lists both
  `knx-productdb` and `knx-store` as dependencies (the catalog routes and
  the project routes, respectively) — no new crate edge is needed at all.
- **Code fact.** `knx_core::Command` is the only place mutations to
  `Project` happen (ARCHITECTURE.md §6); `knx-core` is one of the four
  crates `check_layering` forbids from reaching `rusqlite`
  (`CORE_FORBIDDEN`), so `Command::apply` can never itself query
  `parameter_type`/`parameter`/`parameter_type_enum` — type/kind/min/max/
  enum validation is structurally impossible inside `knx-core`. D24 states
  the consequence: that validation has to happen in `knx-server`, before a
  `Command` is even constructed.
- **Code fact, corrected during Task 2.** `Override<T>` exists in
  `crates/knx-core/src/provenance.rs:51` and is the undo-payload shape for
  `RestoreComObjectDescription` (`Override<Text>`), `RestoreComObjectDpt`
  (`Override<DptRef>`) and `RestoreComObjectFlag` (`Override<bool>`) in
  `crates/knx-core/src/command.rs`. Its variants are **not** `Absent`/
  `Present`, as an earlier revision of this design asserted; they are
  `Absent`, `Empty`, `Value(Resolved<T>)` and `Malformed(String)` — it
  models a *source attribute's* four real ETS states, which is why those
  three commands use it: the fields they restore are themselves
  `Override`s. `ParameterInstance.raw` is a plain `String`
  (`crates/knx-core/src/parameter.rs:21`), so the state D24's inverse
  command restores is only "a row existed carrying this string" versus "no
  row existed at all". That is `Option<String>`, and D24 uses it; reusing
  `Override<String>` here would have meant inventing an `Override` for a
  field that is not one.
- **Code fact.** `knx_core::device::DeviceInstance.source: SourceRef {
  path, ets_id }` already exists per device, and every existing
  `ParameterInstance` created by import shares its owning device's file
  path (a `ParameterInstanceRef` is a child of the same `DeviceInstance`
  element in the same source file). D24 reuses `device.source.path`
  verbatim for a `ParameterInstance` the editor creates, rather than
  inventing a sentinel path — this is the existing convention, not a new
  one.
- **[V], measured for this revision, all three demo projects
  (`OriginalData/DemoProjects/`, extracted read-only).**
  `ParameterInstanceRef` row counts: KV v2.5 demo 9, Unser Zuhause ETS
  6.3.0 1343, Unser Zuhause ETS 4 1390. Module-qualified (id matches
  `_M-\d+_MI-\d+_`): KV 9/9, both Unser Zuhause projects 0/0. Union-typed
  (`_UP-`): KV 0, UZ 6.3.0 208, UZ 4 216. Verbatim match against a
  declared `ParameterRef` id: KV 0/9, UZ 6.3.0 1343/1343 (all), UZ 4
  1390/1390 (all) — every UZ instance row matches a declared id exactly,
  including all 208/216 union ones; the gap between that row count and
  the 788 *distinct* ids behind it is the same bare declared id reused
  under different `DeviceInstance` elements (each device gets its own
  copy of the id), not a module qualifier. Reproduced with `grep -o
  '<ParameterInstanceRef RefId="[^"]*"' <project>/0.xml`, filtered per
  column above, against each `.knxproj` zip's extracted `0.xml`.
- **[V], same measurement.** KV's `M-00FA_A-2504-10-C071` application
  program declares 6 `ParameterRef` ids total (2 top-level, 4 under
  `MD-2`), none containing `_M-\d+_MI-\d+_`, and its `Dynamic` declares 8
  `Module` elements under `MD-2` (`M-1` through `M-8`). KV's other three
  `M-00FA` programs (`A-2502-10-8698`, `A-2500-10-51CB`,
  `A-2507-10-0DE5`) each likewise declare 8 `Module` elements — 32 in
  total across the four programs the KV project activates. **Correction
  to the brief that produced this revision:** the brief's figure "32 such
  Module ids in that program" is the sum across those four programs, not
  the count for `A-2504-10-C071` alone (8) — re-measured and corrected
  here; it does not change any decision below. Across all three demo
  projects' declared `parameter_ref` id spaces (24 KV rows, 18,843 rows
  shared by both Unser Zuhause projects), zero contain the
  `_M-\d+_MI-\d+_` substring — the decomposition regex in D21 cannot
  mistake a declared id for a module-qualified one anywhere in this
  corpus.
- **[V], same measurement.** Regex-stripping `_M-\d+_MI-\d+_` → `_` from
  each of KV's 9 stored `ParameterInstanceRef` ids yields a string that
  matches a declared `ParameterRef` id in the corresponding program, 9/9.
  The `MI` index observed is `1` in every one of those 9 rows, and `_MI-`
  never occurs anywhere in either Unser Zuhause project — this corpus
  never exercises an `MI` value other than 1, so `MI`'s full semantics
  (does it ever exceed 1? what would a second instance of one `Module`
  reference mean?) are unattested. D25 relies on this gap staying
  unresearched territory for the *write* side; it does not affect the
  read side, where `MI`'s value is carried through unparsed.
- **Code fact, re-verified for this revision.**
  `crates/knx-productdb/src/dynamic/evaluate.rs:797`,
  `evaluate_comparable_choose`, resolves a `choose`'s controlling value
  with `values.get(id)` where `id` is `node.ref_id` (a bare declared
  `ParameterRef` id) and `ValueMap` is `HashMap<String, String>`
  (`evaluate.rs:348`) — confirmed no scope enters that lookup at all.
- **Code fact.** `ModuleScope::module_id` (`evaluate.rs:475`) is set from
  `node.element_id` when a `Module` node is walked (`evaluate.rs:680`) —
  the same `@Id` `dynamic/parse.rs` already stores on every `DynamicNode`
  (`evaluate.rs:130`). A program's declared `Module/@Id` set is therefore
  already reachable two ways with no new SQL query: walking the
  already-loaded `ProgramTrees` for `Module`-kind nodes directly, or
  collecting the distinct `ModuleScope`s already present on
  `Activation::parameter_refs` once D23's own grouping runs. D21's
  decomposition step uses the latter, since D23 already computes it.

## Decisions

### D20. Evaluation lives in `apps/knx-server`; no new crate

`apps/knx-server` assembles the `ValueMap` and calls `evaluate`. Forced by
the layering rule in Evidence: `knx-productdb` cannot reach `knx-store`
(so it cannot read a project's `ParameterInstance` rows itself), and
`knx-core`/`knx-productdb` cannot reach each other's storage layer either.
Something above both crates has to hold a project connection and a
product-db connection at once and pass values between them —
`apps/knx-server` is the only crate in the workspace that already depends
on both `knx-store` and `knx-productdb` for exactly this reason (the
catalog routes and the project routes).

**Rejected alternative: a new `knx-parameter-service` crate** interposed
between `knx-server` and the two infrastructure crates. Rejected for the
same reason ADR-0017 rejected `knx-bus-service`: `knx-server` is this
seam's only consumer today, and `CLAUDE.md`'s "avoid speculative
abstractions" argues directly against a crate boundary drawn for a
hypothetical second consumer that does not exist. If `knx-cli` ever needs
headless parameter editing, that is the day this decision gets revisited,
not before.

The read model is a set of plain `#[derive(serde::Serialize)]` DTOs
defined in `apps/knx-server` (D22), not a `knx-projection` type. Evidence
above shows why `knx-projection` structurally cannot host it: it would
need `knx-productdb`'s parameter/type/enum tables, and `knx-projection` is
forbidden from reaching `rusqlite` at all. This is not a compromise; it
follows the same shape `CatalogInstallReportDto` already uses for a
product-db-shaped response with no `knx-projection` involvement.

### D21. `ValueMap` assembly: stored values win, program defaults fill the rest, and a stale value is reported, never dropped

Precedence, using `resolve_values` exactly as it already exists:

1. Build `supplied: HashMap<String, String>` from the device's stored
   `ParameterInstance` rows (`crates/knx-store::load_parameters_for_
   installation`, filtered to the one device, or a new device-scoped
   loader — see the plan), keyed by `source.ets_id`, valued by `raw`.
2. Call `resolve_values(conn, program_id, &supplied)`. Its own precedence
   (supplied, then `parameter_ref.value`, then `parameter.value`) is
   unchanged and is exactly D21's precedence — this decision is naming
   what `supplied` means for this consumer, not changing `resolve_values`.

**Decomposing a stored `ets_id` (added by this revision).** A stored
`ParameterInstance.source.ets_id` is not always a bare declared
`ParameterRef` id — Evidence shows ETS encodes a `Module` instantiation
*inside* the id string itself:
`<Module/@Id>_MI-<k>_<declared ParameterRef's own suffix>`. Before an
`ets_id` can be matched against the program, it must be decomposed:

1. If `ets_id` is already a member of `parameter_ref_ids` (D22) verbatim,
   it is unscoped — the common case (1343/1343 and 1390/1390 in the two
   Unser Zuhause demo projects, Evidence).
2. Otherwise, apply `^(.*)_M-(\d+)_MI-(\d+)_(.*)$` to `ets_id`. No match:
   decomposition fails (see the corrected stale definition below). A
   match yields candidate `module_id = "{prefix}_M-{n}"` and candidate
   declared id `"{prefix}_{suffix}"`. Both must be validated before
   either is trusted — a regex match alone is a coincidence, not a
   decomposition: `module_id` must equal the `module_id` of some
   `ModuleScope` the program's `Dynamic` tree actually declares
   (Evidence: already reachable from the loaded `ProgramTrees`/
   `Activation`, no new query), and the declared id must be a member of
   `parameter_ref_ids` (D22). Both checks pass on all 9 of KV's rows
   (Evidence); on this corpus the regex never matches a genuinely
   unscoped id by accident, since no declared `parameter_ref` id in any
   of the three demo projects contains `_M-\d+_MI-\d+_` (Evidence).

Chosen over prefix-matching the `Module/@Id` set first: both routes
decompose KV's 9 rows correctly (Evidence), but the regex route reads the
section-routing key (`module_id`) and the `ValueMap`/`parameter_ref_ids`
key (the declared id) out of one pattern match, so it does not need the
`Module/@Id` set enumerated before it can even attempt a split — only to
*validate* a candidate it already has. Validation against the known
`Module/@Id` set and against `parameter_ref_ids` still runs either way;
the difference is only which piece of data decides where to cut the
string.

Successfully decomposed, unscoped entries enter `supplied` exactly as
step 1 above describes. Successfully decomposed, module-scoped entries do
**not** enter `supplied` — see the flat-`ValueMap` ruling below — they
feed D22's per-section value attachment instead.

**The stale case, corrected.** `stale` means: decomposition failed
outright (no verbatim `parameter_ref_ids` match and no regex match
either — the program changed since the value was written, or the id was
simply never valid), **or** decomposition (verbatim or via the regex
above) recovered a candidate that names no `parameter_ref`/`Module` row
the current program actually declares. `stale` does **not** mean "the id
carries a module segment" — a module-qualified id that decomposes
successfully (KV's shape, 9/9) is not stale; it is a per-channel value
with a home (D22). A stale entry keeps carrying its original `ets_id` and
`raw` unchanged, exactly as before this revision.

Per `CLAUDE.md`: never silently discard. The assembly step in `knx-server`
runs the decomposition above over every stored `ParameterInstance` for
the device — not a separate diff query; the decomposition already visits
every row — and sorts each into exactly one of three outcomes: unscoped
(feeds `supplied`), module-scoped-and-valid (feeds D22's per-section
attachment), or stale (decomposition failed, or its candidate is
undeclared). Every orphan lands in the read model's own `stale` list
(D22), carrying its `ets_id` and its `raw` value unchanged.

**Rejected alternative: silently drop the stale row from the read
model.** Rejected outright — this is precisely the case `CLAUDE.md` names.
**Rejected alternative: silently keep it in the displayed field list as if
it still named a real parameter.** Rejected — it would mislead the user
into believing an edit to a phantom field does something, when writing to
that `ets_id` is (D24) never possible again once no `parameter_ref` claims
it, since the write path validates against the declared `parameter_ref`
set.

The stale row is never deleted by this slice. Deleting project data based
on a product-database observation is an even larger data-integrity
decision than surfacing it, and nothing in the brief asks for a delete
path — reporting it is the whole requirement.

**The flat-`ValueMap` conflict: what `supplied` receives (added by this
revision).** Five stored values (KV's shape) can map to one `ValueMap`
key — `resolve_values` and `evaluate` (Evidence: `evaluate.rs:797`/
`:348`) have exactly one flat slot per declared id, project-wide, with no
scope. `supplied` cannot receive more than one of the five without
silently picking a winner, and `CLAUDE.md` forbids that outright.

**Ruling: `supplied` receives only unscoped, successfully-matched stored
values.** Module-scoped, successfully-decomposed values never enter
`supplied`/`ValueMap` at all; they are attached directly to their own
`ParameterSectionDto`'s field at the D22 assembly step, after `evaluate`
has already run, using the per-channel map the decomposition step already
built (`(module_id, declared id) -> raw`). The evaluator never sees a
module-scoped value in this slice, in either direction.

**Rejected alternative: feed one (arbitrary) module-scoped value per
declared id into `supplied` and report the conflict elsewhere.** Rejected.
A value the evaluator uses to decide a `choose` branch for *every*
instantiation, chosen from among several channels' worth of values by
"whichever came first" or "the last one processed," is a silently-picked
winner regardless of whether a diagnostic elsewhere also names the
conflict — a report next to a silent decision is not the same as not
making the silent decision. It would also make evaluation depend on
processing order over data with no natural order (five stored rows for
five channels do not rank), a new nondeterminism `CLAUDE.md`'s
"deterministic behavior" rules out.

**Consequence, stated plainly.** Because no module-scoped value ever
reaches `ValueMap`, a `choose` inside a `ModuleDef` that is controlled by
a module-scoped `ParameterRef` is evaluated against that parameter's
*program-level default* (`parameter_ref.value`/`parameter.value`) in
every instantiation, never against what a specific channel actually has
stored. The *value shown* for a module-scoped field is correct per
channel (D22 attaches it directly from the stored row); the *active field
set* for that channel — which fields even appear, because a `choose`
upstream of them took one branch or another — is computed as if every
channel used the default. A channel whose `choose` decision ETS would
make differently, based on its own stored value, can therefore show a
different active field set here than ETS would show. This is real and
belongs in Non-goals and in the `KNOWN_LIMITATIONS.md` text Task 5
writes, not swept into "module values are read-only" as if that already
covered it.

### D22. The read model: `query.rs` additions and the DTO shape

Two additions to `crates/knx-productdb/src/query.rs`, following
`com_object_view`'s own idiom (a bulk, single-query fetch, not N calls per
field — 208 fields per `ModuleDef` alone, Evidence, makes N calls the
wrong shape):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterView {
    pub id: String,               // parameter_ref.id, the ValueMap/ets_id key
    pub display_order: Option<i64>, // ParameterRef/@DisplayOrder is genuinely optional in shipped
                                     // packages — [V] measured: all 543 parameter_ref rows for
                                     // prod3's program M-0083_A-0317-31-7DC6 omit it. None means
                                     // "the package declared no order", distinct from Some(0);
                                     // a plain i64 with a 0 fallback would erase that distinction.
    pub tag: Option<String>,
    pub name: Option<String>,     // parameter.name
    pub text: Option<String>,     // pick(parameter.text, parameter_ref.text)
    pub text_layer: ValueLayer,   // reuses the existing enum
    pub kind: String,             // parameter_type.kind, verbatim
    pub access: Option<String>,   // parameter.access, verbatim — display only, D24 does not gate on it
    pub min_inclusive: Option<String>,
    pub max_inclusive: Option<String>,
    pub enum_options: Vec<(String, Option<String>)>, // (value, text), only non-empty when kind == "Restriction"
}

/// Every `ParameterView` declared by a program, in `parameter_ref.
/// display_order`. One query, not one per field.
pub fn parameter_views(conn: &Connection, program_id: &str)
    -> Result<Vec<ParameterView>, ProductDbError>;

/// The bare id set, for the stale-value diff (D21). A thin projection of
/// the same rows `parameter_views` reads; kept separate because the
/// stale check needs only the id set and runs before the full view is
/// worth building.
pub fn parameter_ref_ids(conn: &Connection, program_id: &str)
    -> Result<std::collections::HashSet<String>, ProductDbError>;
```

`apps/knx-server` combines one `parameter_views` call, one `Activation`
(D20), one `supplied` map and one stale diff (D21) into:

```rust
struct ParameterPanelDto {
    program_id: Option<String>,           // None: device has no resolvable program_ref (mirrors CreationDiagnostic::ProgramlessProduct)
    sections: Vec<ParameterSectionDto>,
    stale: Vec<StaleParameterDto>,
    diagnostics: Vec<ParameterDiagnosticDto>,
}
struct ParameterSectionDto {
    scope: Option<ModuleScopeDto>,        // None: the program's own top-level Static
    fields: Vec<ParameterFieldDto>,
}
struct ModuleScopeDto { module_node: i64, module_id: Option<String>, module_def_id: String }
struct ParameterFieldDto {
    ets_id: String,
    name: Option<String>,
    text: Option<String>,
    kind: String,
    value: Option<String>,
    value_source: String,                 // "Stored" | "ProgramDefault" — was ets_id a key in `supplied`?
    editable: bool,                       // false when the field's scope != None (D25)
    min: Option<String>,
    max: Option<String>,
    enum_options: Vec<EnumOptionDto>,
}
struct EnumOptionDto { value: String, text: Option<String> }
struct StaleParameterDto { ets_id: String, raw: String }
struct ParameterDiagnosticDto { scope: Option<ModuleScopeDto>, message: String, detail: String }
```

`value_source` is new provenance this slice adds, not a `resolve_values`
output — `resolve_values` returns only the merged map, so `knx-server`
derives it itself: `if supplied.contains_key(ets_id) { "Stored" } else {
"ProgramDefault" }`, computed before the call, since `supplied` is already
assembled. This is the same reasoning `ComObjectView`'s `ValueLayer`
follows: a value without knowing which layer produced it cannot be edited
correctly, and `value_source` is what lets the panel say "this is the
program's own default" versus "you already changed this."

**Per-section value sourcing (added by this revision).** The paragraph
above is exactly right for the top-level section (`scope: None`): every
field there draws `value`/`value_source` straight from the shared
`ValueMap`, unchanged. A module-scoped section's fields do not: D21's
decomposition step already sorts each stored, module-scoped value into a
per-channel map keyed by `(module_id, declared id)`. When `knx-server`
builds a `ParameterFieldDto` inside a section whose `scope` is `Some`, it
looks up that section's `module_id` and the field's `id` in that map
first; found, `value` is the decomposed `raw` and `value_source` is
`"Stored"`; not found, it falls back to the same shared `ValueMap`
default `resolve_values` already computed, `value_source`
`"ProgramDefault"` — the same rule the top-level section uses, just
per-section rather than project-wide. `ParameterView`/`query.rs` above
are unchanged by this — the per-channel map is assembled in
`apps/knx-server`, over rows `query.rs` already returns, not a new
productdb query.

**Rejected alternative: reuse `ValueLayer` (`Program`/`ProgramRef`) for
`value_source`.** Rejected — that enum names *display-text* provenance
inside the product database (which of two product-db-side layers supplied
a string), a different axis from *which side of the project/product-db
boundary* supplied a parameter's live value. Reusing the name for a
different meaning would be the kind of silent semantic drift `CLAUDE.md`
warns against under "explicit behavior."

Only one field, `dpt`-equivalent for parameters, is present per row: there
is no per-field override chain the way `ComObjectRef`/`ComObjectInstanceRef`
has three layers (ARCHITECTURE.md §5) — a `parameter_ref` overrides its
`parameter` at exactly one layer, matching `pick()`'s existing two-argument
shape untouched.

### D23. `ModuleScope`-qualified display: one section per instantiation, addressed by `module_node`

The read model groups `Activation::parameter_refs` by `ActiveRef::scope`
into `ParameterSectionDto`s: one section for `scope: None` (the program's
own top-level parameters), and one section per distinct `ModuleScope`
(keyed by `module_node`, matching the evaluator's own dedup key, D18). A
section's label is `module_id` when present, falling back to `"Module
#{module_node}"` — `module_id` is `Module/@Id`, human-authored and often
readable (e.g. "Channel A" in the Evidence example from §4.4), but is not
guaranteed present, so the fallback must exist.

Each section lists the *same* `ParameterView`s (D22) — a `ModuleDef`'s
`ParameterRef` ids are declared once and reused verbatim by every
instantiating `Module` (RESEARCH.md §4.4, carried forward) — joined
against that scope's slice of the `Activation`'s active ids, so a section
shows only the fields active for that specific channel, using the shared
metadata (name/text/kind/min/max/enum) but each channel's own evaluated
`value`/`value_source` (which, after D21's decomposition and D22's
per-section attachment, are **not** guaranteed identical across sections —
KV's corpus shape stores five distinct values, 17/33/49/32/48, for the
same declared `ParameterRef` across five `Module` instantiations, and
each now surfaces in its own section. What *is* still shared across
sections, per D16 and D21's flat-`ValueMap` ruling, is the *evaluated
activation*: a `choose` upstream of a module-scoped field runs against
the program-level default in every instantiation, since no per-channel
value ever reaches `ValueMap` — so two channels can show a different set
of *active* fields than ETS would, even though each channel's own field,
once active, shows its own real stored value. D25 names the write-side
consequence of the same flat map; this is its read-side counterpart).

Wire representation: `module_node` (a `dynamic_node.node_id`, stable for a
given `program_id`'s stored tree, D14) is the section's identity on the
wire, carried in `ModuleScopeDto.module_node`, an `i64`. It is not used to
address a *write* — D24/D25 write by `ets_id` alone, since that is the
only key `ParameterInstance` supports; a module-scoped field's `ets_id` is
therefore ambiguous as a write target across sections, which is exactly
why D25 makes those fields `editable: false`.

**Rejected alternative: flatten all sections into one list, `ref_id`
deduplicated.** Rejected — it is precisely the collapse D14/D18 (slice 2)
exists to prevent; the whole point of `ModuleScope` is that "12
instantiations are 12 results," and a UI over it must not re-introduce the
bug the evaluator was built to fix.

### D24. Writing a value: request, validation, response, timing

**Request.** `POST /api/device/{device_id}/parameters`, body
`{ etsId: String, raw: String }` (camelCase on the wire, matching the
existing convention throughout `routes.rs`).

**Validation, before a `Command` is constructed** (forced by the layering
fact in Evidence: `knx-core` cannot reach `parameter_type`/`parameter`/
`parameter_type_enum`, so this cannot live in `Command::apply`):

1. The device must exist and resolve to a `program_id` (`resolve_program`)
   — else 404/400, matching the existing `set_individual_address`-style
   error shape.
2. `etsId` must name a `parameter_ref` declared by that program (via the
   `parameter_ref_ids` set, D22) — else 400. **This check does not require
   the parameter to be currently active.** A parameter hidden behind an
   unmatched `choose` branch today may become visible after a different
   edit, and its previously-written value must still be there when it
   does — the same "values persist under a hidden branch" behaviour any
   ETS-like parameter form needs, and nothing here has to invent it: it
   falls out of `ParameterInstance` already being keyed by `ets_id` alone,
   independent of `Activation`.
3. The named `parameter_ref`/`parameter`/`parameter_type` chain's `kind`
   decides the check: `Number` — `raw` parses as the same integer type
   `resolve_values`'s callers already assume and falls within
   `min_inclusive`/`max_inclusive` when present; `Restriction` — `raw`
   equals one `parameter_type_enum.value` for that `parameter_type_id`;
   `None` — rejected outright, a `TypeNone` parameter carries no writable
   value (slice 1's D9); every other kind (`Text`/`Float`/`IPAddress`/
   `Picture`/`Raw`) — non-empty-string only, per this design's stated
   non-goal above. A validation failure is 400 with a message naming the
   `kind` and the bound or set that was violated.
4. The field's current `Activation`-derived section must be `scope: None`
   — else 400 "read-only in this slice" (D25).

**On success**, `knx-server` builds and applies exactly one
`knx_core::Command::SetParameterValue`:

```rust
// crates/knx-core/src/command.rs, alongside SetComObjectFlag
SetParameterValue {
    id: ParameterInstanceId,   // existing row's id if one exists for (device, ets_id); freshly allocated via Project::ids::next_parameter_instance_id() otherwise — same split as CreateDevice: this command does not allocate
    device: DeviceId,
    ets_id: String,
    raw: String,
},
RestoreParameterValue {       // undo/redo form, same reason RestoreComObjectFlag exists rather than reusing the bare-String shape
    id: ParameterInstanceId,
    device: DeviceId,
    ets_id: String,
    raw: Option<String>,      // None: no ParameterInstance existed before, undo deletes the row; Some(prior): restore that exact string
},
```

`apply` validates only what `knx-core` can see: the device exists
(reusing `CommandError::DeviceNotFound`). It finds an existing
`ParameterInstance` for `(device, ets_id)` in `installations[0]`'s
`parameters` list — every existing `Command::apply` targets that one
installation and none resolves which installation owns a device
(`crates/knx-core/src/command.rs:30-33`); this command inherits that scope
rather than inventing routing — and overwrites `raw` in place, or, if none exists,
pushes a new one with `source: SourceRef { path: device.source.path.
clone(), ets_id }` (Evidence: this mirrors the path every
import-created `ParameterInstance` already carries, since a
`ParameterInstanceRef` lives inside its `DeviceInstance` in one file).

**Response.** The *same* `ParameterPanelDto` D22 defines, freshly
assembled after the write — re-evaluation is part of the same request,
not a subsequent `GET`. **Rejected alternative: return only the write's
own echo and require a follow-up `GET`.** Rejected because "changing one
value changes which other parameters exist" (the brief's own framing) —
a client that forgets the follow-up `GET` would display a stale active
set with no indication anything is wrong, which is the UI-level version of
the same silently-stale-data problem D21 refuses to allow at the model
level. One round trip per user action, always internally consistent, is
worth the (small, D22-Evidence-bounded) cost of recomputing the whole
panel on every write.

### D25. Module-scoped fields are read-only in this slice

The conclusion stands; the reason changes. `ParameterInstance` is *not*
blocked from storing a per-channel value — Evidence shows ETS already
writes a scope-qualified `ets_id`, and `crates/knx-store`'s `(device,
ets_id)` key already stores five distinct such rows for one declared
`ParameterRef` in the KV corpus today, unmodified by this revision. The
real blocker is two things, both in the evaluator, not the schema:

1. **The flat `ValueMap` (D21's ruling, `evaluate.rs:797`/`:348`).** Even
   a correctly-written, scope-qualified value can never reach
   `evaluate`'s `choose` logic in this slice's design, because `supplied`
   only ever receives unscoped values (D21). Accepting a module-scoped
   write would silently violate D24's own "no second `GET` needed"
   guarantee: the freshly re-assembled `ParameterPanelDto` returned in
   the same response would not reflect the edit's effect on the active
   field set, because nothing propagates a module-scoped value into the
   evaluation that built that set. A write the evaluator cannot see is a
   worse UX than no write — the user would have to already distrust the
   response they just got back.
2. **The write-side validation this would need is unresearched.**
   Accepting a module-scoped `etsId` on `POST` would mean either
   decomposing and validating it the way D21 now does for reads (a real
   open question: does the `MI` index ever legitimately exceed `1`?
   Evidence: every occurrence in all three demo projects is `MI-1`; its
   full semantics are unattested) or accepting the fully-qualified string
   as an opaque write target with no cross-check against the program at
   all. Neither is designed here, and `CLAUDE.md`'s "do not invent
   technical facts" rules out guessing at `MI`'s semantics from a corpus
   that never exercises a second value.

Given both, the honest and simplest choice (`CLAUDE.md`: "prefer the
simpler [approach] unless there is a measurable reason not to") is the
one already reached in the first draft: module-scoped fields stay
`editable: false`, checked server-side. D24's write path needs **no code
change** for this revision, only its stated justification does — its
verbatim-match check against `parameter_ref_ids` (D24 step 2) already
rejects a module-scoped `etsId` on its own, since Evidence shows no
declared `parameter_ref` id in this corpus ever contains the
`_M-\d+_MI-\d+_` pattern.

**Explicitly:** module-scoped values are read, decomposed, displayed per
channel (D21/D22/D23), and preserved untouched. No write in this slice
may modify, delete or normalise one — not the top-level write path
(which cannot even name one, per above), and nothing else in this slice
touches `ParameterInstance` rows outside `Command::SetParameterValue`/
`RestoreParameterValue`'s own top-level-only reach (D24, Task 2,
unchanged).

**Rejected alternative: allow the write and let it visibly affect every
channel.** Rejected — technically simple, but it would present a
misleading affordance (a field drawn inside "Channel A"'s section,
editable, that actually edits "Channel B" through "Channel L" too) with
no warning. A UI that looks like per-channel editing but is not is worse
than no per-channel editing. (This rejection stands regardless of which
justification above is cited — it was never about *where* the value
would be stored.)
**Rejected alternative: silently no-op the write for module-scoped
fields.** Rejected outright by the same never-silently-discard reading
D21 already applies — `editable: false` in the DTO, checked server-side
too (D24 step 4), is the honest version of the same refusal.

This is the line item that makes T18 slice 3 a bounded, single-branch
slice rather than the full editor: the harder half of "the editor" —
per-channel values driving evaluation, and a validated write path for
them — is a real, separately-decidable evaluator question (not the
data-model question the first draft named; Non-goals below corrects
this), named explicitly, not solved here.

### D26. Diagnostics surface as a summarized, expandable list — never dropped, never a debugger

`Activation::diagnostics` (`Vec<ScopedDiagnostic>`) maps 1:1, in order,
into `ParameterPanelDto::diagnostics: Vec<ParameterDiagnosticDto>` — every
diagnostic the evaluator produced is present in the response, satisfying
"never logged, never swallowed" at the wire boundary, same as `Activation`
itself already guarantees inside the crate.

Each `ParameterDiagnosticDto` carries two strings: `message`, a short,
fixed, hand-authored sentence per `Diagnostic` variant (e.g.
`NoBranchMatched` → `"A choice did not match any of its options."`,
`ModuleDefNotFound` → `"A module could not be found in this program."`),
naming the user-facing situation without exposing a `node_id`; and
`detail`, the variant's own `{:?}` `Debug` rendering (the internal ids
intact) — present for anyone attaching a diagnostic to a bug report, not
shown by default.

The frontend renders the list as a single collapsed banner headed with a
count ("3 issues found while evaluating this device's parameters"),
expandable to the per-item `message` text; `detail` is behind a further
"copy details" affordance, not printed inline. A diagnostic whose
`Diagnostic` variant carries a `param_ref` that matches a currently
displayed field additionally gets a small marker on that field (not a
blocking one — D24/D25 do not gate writes on diagnostics existing
elsewhere in the same activation, a stated non-goal below).

**Rejected alternative: render `Diagnostic`'s `Debug` form directly,
unsummarized, as the primary UI.** Rejected — this is the "turns the panel
into a debugger" the brief warns against by name; a user editing a
lighting channel does not need to see `UnexpectedTypeNoneShape {
choose_node: 4821 }` to understand that something about a choice could
not be evaluated.
**Rejected alternative: summarize only a count, drop the per-item
detail.** Rejected — this is "never swallowed" being violated in spirit
even if the `Vec` is not literally shortened; a user who wants to know
*which* choice failed and report it has to be able to get there.

## Acceptance criteria

1. `GET /api/device/{id}/parameters` on a device with a resolvable program
   returns a `ParameterPanelDto` whose top-level section's fields' values
   match a hand-computed `resolve_values` result for a fixture with a
   known, non-empty `supplied` map (at least one stored, at least one
   defaulted field, asserted by `value_source`).
2. A device with a stored `ParameterInstance` whose `ets_id` decomposes to
   nothing valid — no verbatim `parameter_ref_ids` match, no successful
   regex decomposition, or a regex match whose candidate names no
   `parameter_ref`/`Module` the current program declares — appears in
   `stale`, not in any section's `fields`, and the device's other valid
   stored values (unscoped or successfully module-scoped) are unaffected
   (D21, corrected by this revision).
3. A device whose program has a `Module` instantiated twice produces two
   `ParameterSectionDto`s with distinct `scope.module_node`, both
   containing the same `ets_id`s, exactly matching D14/D18's two-
   instantiations-two-results guarantee, now at the DTO layer (D23). This
   criterion is about the `ets_id` *sets* matching, not the values —
   criterion 4 below checks that values may legitimately differ per
   section.
4. A device whose stored `ParameterInstance` rows are the KV v2.5 demo
   shape — `ParameterInstanceRef` ids like
   `M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1` — decomposes all of them
   successfully: the five distinct stored values 17, 33, 49, 32, 48 (for
   `Module` instantiations `M-2` through `M-6` of `ModuleDef`
   `M-00FA_A-2504-10-C071_MD-2`, declared `ParameterRef`
   `M-00FA_A-2504-10-C071_MD-2_P-1_R-1`) each appear as that field's
   `value` with `value_source == "Stored"` in their own
   `ParameterSectionDto`, and `stale` is empty (D21/D22, this revision).
5. `POST /api/device/{id}/parameters` with a valid top-level `etsId`/`raw`
   pair returns 200 and a `ParameterPanelDto` whose relevant field's
   `value` equals the posted `raw` and whose `value_source` is `"Stored"`,
   in the same response — no second request needed to observe it (D24).
6. The same request, repeated with a `raw` outside `min_inclusive`/
   `max_inclusive` for a `Number`-kind field, and with a `raw` absent from
   `parameter_type_enum` for a `Restriction`-kind field, both return 400
   and leave the previously stored value unchanged (verified by a
   follow-up `GET`).
7. A `POST` naming an `etsId` whose field is currently `scope: Some(_)`
   returns 400 and changes nothing (D25).
8. A `POST` naming an `etsId` that is not declared by the device's program
   at all (not even stale — never valid) returns 400.
9. A write and its `Command::SetParameterValue`/`RestoreParameterValue`
   round-trip through `/api/undo` and `/api/redo`: after undo, a
   previously-absent `ParameterInstance` is gone again (not present with
   an empty string), and a previously-present one is restored to its
   exact prior `raw`.
10. `ParameterPanelDto::diagnostics.len()` equals the underlying
    `Activation::diagnostics.len()` for the same evaluation, for a fixture
    constructed to produce at least one diagnostic (D26) — nothing is
    filtered out between `evaluate` and the wire.
11. All six gates pass: `cargo fmt --all --check`, `cargo clippy
    --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
    `cargo run -p xtask -- check-layering`, `cargo deny check`, and
    `npm run test` from `apps/knx-web`.
12. `docs/KNOWN_LIMITATIONS.md` §3, `docs/GAP_ANALYSIS_ETS.md`'s A3 row
    and T18 entry, and `docs/DATA_MODEL.md` §10 all state, in the same
    document set, both what this slice closes (a UI and a write path exist
    now, and module-scoped values are correctly read and displayed) and
    what it deliberately still does not do (module-scoped editing, the
    flat-`ValueMap` activation limitation D21/D23 name, deep format
    validation) — no standing limitation is downgraded past what D20-D26
    actually deliver.

## Non-goals and follow-ups

Specific and generous, per the brief — this list is the material for the
new `KNOWN_LIMITATIONS.md` entry, not a summary of it.

- **Per-channel (`Module`-instantiation) value editing.** Named in
  "Status and scope" and D25. Storage is not the open question — Evidence
  shows `ParameterInstance`'s existing `(device, ets_id)` key already
  carries a module-qualified `ets_id` in the corpus today, unmodified by
  this revision. What is open, and needs its own design: (1) a
  scope-aware value lookup in `knx-productdb` — `resolve_values`/
  `evaluate`'s flat `ValueMap` (`HashMap<String, String>`,
  `evaluate.rs:348`) would need to become scope-aware (most plausibly
  keyed by something like `(Option<i64 module_node>, String)`, mirroring
  `ScopeKey`'s own existing dedup key) so a `choose` can see a channel's
  own value instead of the program default; and (2) a validated write
  path for a module-scoped `etsId`, including settling what the `MI`
  index means when it is ever observed above `1` (unattested in this
  corpus, D25). Neither is a schema change on present evidence; both are
  deliberately not bundled into this slice.
- **Deep validation for `Float`/`Text`/`IPAddress`/`Picture`/`Raw`.** Only
  a non-empty-string check. In particular: no IPv4 dotted-quad parsing for
  `IPAddress`, no byte-length check against `size_in_bit` for `Raw`, no
  fractional-format check for `Float`. None of these have a research spike
  behind them; inventing rules now would be exactly the untested
  assumption the brief asks to keep out of acceptance criteria.
- **`Access` is display-only.** RESEARCH.md §4.3 found no usable
  correlation for `Access` (`Access="None"` alongside a `Memory` child came
  back roughly 50/50 in the corpus, and `Visible` was never observed at
  all). This slice shows `access` verbatim in `ParameterFieldDto` and does
  not use it to block, hide, or grey out a write.
- **Diagnostics never gate a write.** A `NoBranchMatched` (or any other)
  diagnostic present elsewhere in the same `Activation` does not prevent
  writing a different, valid field. Deciding when a diagnostic *should*
  block an edit is unresearched territory this slice does not enter.
- **Argument (`NumericArg`/`TextArg`) values.** Unreadable and unwritable
  in this slice, same as slice 2 left them — they are placement/template
  mechanics (§4.4 Q2/Q3), not user-facing parameters, and remain stored
  losslessly and uninterpreted.
- **Union parameters (`_UP-<n>_R-<n>` ids, `parameter.union_id`/
  `union_size_in_bit`).** Displayed and writable like any other parameter
  in this slice (raw string in, raw string out), with no union-aware
  cross-field validation — the bit-packed shared-storage semantics are
  unresearched (RESEARCH.md never covers them).
- **No bulk/multi-field write.** One `POST` per value (D24). If per-device
  panels prove slow to edit field-by-field in practice, a batched
  endpoint is a named follow-on, not a speculative addition now.
- **No pagination or virtualization of the field list.** Evidence bounds
  a single device's active field count at roughly the low hundreds
  (208 `ParameterRef`s in one `ModuleDef` alone) — a different scale from
  the *project-wide* tables ARCHITECTURE.md §7 does paginate. If a real
  program is found whose single-device field count rivals those tables,
  that is new evidence this design did not have, and pagination becomes
  a real decision then, not now.
- **No product-database (`parameter`/`parameter_ref`/`parameter_type`)
  editing.** This is a device-instance (project-side) editor only —
  the product database stays read-only infrastructure, matching every
  other slice touching it so far.
- **No search or filter UI.** A hundred-plus flat field list per section
  is left exactly that; a search box is presentation-layer polish with no
  data-model consequence and is left to a follow-up.
- **Commissioning, download, anything needing KNX hardware.** Unchanged
  from the brief's own exclusions.
