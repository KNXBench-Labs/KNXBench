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

- **Editing a `Module`-scoped (per-channel) value.** `ParameterInstance` is
  keyed by `(device, ets_id)` only (`crates/knx-core/src/parameter.rs`); it
  has no field for *which* instantiation of a `ModuleDef` a value belongs
  to. Slice 2's D16 already establishes that the evaluator reads every
  instantiation of one `ModuleDef` against identical values, because
  per-instantiation values are a project-side construct `knx-productdb`
  does not model. This slice inherits that limitation on the *write* side
  too: there is nowhere to store twelve independent channel values today.
  Fixing it needs a real data-model decision (does `ParameterInstance` grow
  a scope key? does the key point at the AP-level `Module`, matching
  `ModuleScope::module_node`, or at a project-side `ModuleInstance`?) that
  deserves its own design, not a side effect of this one. Module-scoped
  values are read and displayed per channel in this slice; they are not
  editable in this slice. Named as follow-on work below.
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
- **Code fact.** `Override<T>` (`Absent`/`Present`) already exists and is
  already the undo-payload shape for `RestoreComObjectDescription`
  (`Override<Text>`), `RestoreComObjectDpt` (`Override<DptRef>`) and
  `RestoreComObjectFlag` (`Override<bool>`) in `crates/knx-core/src/
  command.rs`. D24's inverse command reuses it rather than inventing a new
  "did this exist before" shape.
- **Code fact.** `knx_core::device::DeviceInstance.source: SourceRef {
  path, ets_id }` already exists per device, and every existing
  `ParameterInstance` created by import shares its owning device's file
  path (a `ParameterInstanceRef` is a child of the same `DeviceInstance`
  element in the same source file). D24 reuses `device.source.path`
  verbatim for a `ParameterInstance` the editor creates, rather than
  inventing a sentinel path — this is the existing convention, not a new
  one.

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

**The stale case.** A stored `ParameterInstance.source.ets_id` that names
no `parameter_ref` row for the device's current `program_ref` (the program
changed since the value was written, or the id was simply never valid) is
never visited by `resolve_values`'s own loop — that loop only iterates
`parameter_ref` rows for the program, so a stale `supplied` entry rides
along unused in the returned `ValueMap` and is never flagged by anything
inside `knx-productdb`. Per `CLAUDE.md`: never silently discard. The
assembly step in `knx-server` computes the diff itself — the set of
`ets_id`s in `supplied` minus the set of `parameter_ref` ids for the
program (a new bulk query, D22) — and returns every orphan explicitly, in
the read model's own `stale` list (D22), carrying its `ets_id` and its
`raw` value unchanged.

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

### D22. The read model: `query.rs` additions and the DTO shape

Two additions to `crates/knx-productdb/src/query.rs`, following
`com_object_view`'s own idiom (a bulk, single-query fetch, not N calls per
field — 208 fields per `ModuleDef` alone, Evidence, makes N calls the
wrong shape):

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterView {
    pub id: String,               // parameter_ref.id, the ValueMap/ets_id key
    pub display_order: i64,
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
`value`/`value_source` (which, per D16 and this slice's own read-model
construction, are identical across sections for now, since `ValueMap` has
one entry per `ets_id`, not one per `(ets_id, module_node)` — D25 names
this on the write side, this decision names it on the read side: the
*evaluated activation* differs per channel, e.g. one channel's `choose`
took a different branch than another's, but a *shown value* for the same
`ets_id` is currently identical everywhere it appears, because there is
exactly one stored value for it).

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
    raw: Override<String>,    // Absent: no ParameterInstance existed before, undo deletes the row; Present(prior): restore that exact string
},
```

`apply` validates only what `knx-core` can see: the device exists
(reusing `CommandError::DeviceNotFound`). It finds an existing
`ParameterInstance` for `(device, ets_id)` in the owning installation's
`parameters` list and overwrites `raw` in place, or, if none exists,
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

Direct consequence of D24 validation step 4 and the data-model fact this
whole design opens with: `ParameterInstance` cannot distinguish which
`Module` instantiation a value belongs to, so there is no correct thing
for a write to `ets_id` under `scope: Some(_)` to do — writing it would
silently change the value shown in every other instantiation of the same
`ModuleDef` too, which is not what a user editing "Channel A" would
expect from a field displayed as "Channel A"'s own setting.

**Rejected alternative: allow the write and let it visibly affect every
channel.** Rejected — technically simple, but it would present a
misleading affordance (a field drawn inside "Channel A"'s section,
editable, that actually edits "Channel B" through "Channel L" too) with
no warning. A UI that looks like per-channel editing but is not is worse
than no per-channel editing.
**Rejected alternative: silently no-op the write for module-scoped
fields.** Rejected outright by the same never-silently-discard reading
D21 already applies — `editable: false` in the DTO, checked server-side
too (D24 step 4), is the honest version of the same refusal.

This is the line item that makes T18 slice 3 a bounded, single-branch
slice rather than the full editor: the harder half of "the editor" —
per-channel values — is a real, separately-decidable data-model question,
named explicitly under Non-goals and follow-ups, not solved here.

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
2. A device with a stored `ParameterInstance` whose `ets_id` names no
   `parameter_ref` in its current program appears in `stale`, not in any
   section's `fields`, and the device's other, valid stored values are
   unaffected (D21).
3. A device whose program has a `Module` instantiated twice produces two
   `ParameterSectionDto`s with distinct `scope.module_node`, both
   containing the same `ets_id`s, exactly matching D14/D18's two-
   instantiations-two-results guarantee, now at the DTO layer (D23).
4. `POST /api/device/{id}/parameters` with a valid top-level `etsId`/`raw`
   pair returns 200 and a `ParameterPanelDto` whose relevant field's
   `value` equals the posted `raw` and whose `value_source` is `"Stored"`,
   in the same response — no second request needed to observe it (D24).
5. The same request, repeated with a `raw` outside `min_inclusive`/
   `max_inclusive` for a `Number`-kind field, and with a `raw` absent from
   `parameter_type_enum` for a `Restriction`-kind field, both return 400
   and leave the previously stored value unchanged (verified by a
   follow-up `GET`).
6. A `POST` naming an `etsId` whose field is currently `scope: Some(_)`
   returns 400 and changes nothing (D25).
7. A `POST` naming an `etsId` that is not declared by the device's program
   at all (not even stale — never valid) returns 400.
8. A write and its `Command::SetParameterValue`/`RestoreParameterValue`
   round-trip through `/api/undo` and `/api/redo`: after undo, a
   previously-absent `ParameterInstance` is gone again (not present with
   an empty string), and a previously-present one is restored to its
   exact prior `raw`.
9. `ParameterPanelDto::diagnostics.len()` equals the underlying
   `Activation::diagnostics.len()` for the same evaluation, for a fixture
   constructed to produce at least one diagnostic (D26) — nothing is
   filtered out between `evaluate` and the wire.
10. All six gates pass: `cargo fmt --all --check`, `cargo clippy
    --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
    `cargo run -p xtask -- check-layering`, `cargo deny check`, and
    `npm run test` from `apps/knx-web`.
11. `docs/KNOWN_LIMITATIONS.md` §3, `docs/GAP_ANALYSIS_ETS.md`'s A3 row
    and T18 entry, and `docs/DATA_MODEL.md` §10 all state, in the same
    document set, both what this slice closes (a UI and a write path exist
    now) and what it deliberately still does not do (module-scoped
    editing, deep format validation) — no standing limitation is
    downgraded past what D20-D26 actually deliver.

## Non-goals and follow-ups

Specific and generous, per the brief — this list is the material for the
new `KNOWN_LIMITATIONS.md` entry, not a summary of it.

- **Per-channel (`Module`-instantiation) value editing.** Named in
  "Status and scope" and D25. Needs its own design decision — most likely
  a `ParameterInstance` scope key — which is itself a schema change and
  therefore needs its own justification, deliberately not bundled into
  this slice's "no v4" constraint.
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
