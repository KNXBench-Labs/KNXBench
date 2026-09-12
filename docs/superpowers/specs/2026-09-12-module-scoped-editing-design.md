# module-scoped (per-channel) parameter editing — T18, fourth slice

## Status and scope

T18's third slice shipped the parameter editor: `GET`/`POST
/api/device/{id}/parameters`, a real panel, top-level writes with undo. It
closed with one deliberate hole, decision **D25** (`docs/superpowers/specs/
2026-09-11-parameter-editor-design.md`): a module-scoped field — one
belonging to a `Module` instantiation, i.e. a channel — is *displayed*
with its own per-channel stored value but is **read-only**, because

> the evaluator's flat `ValueMap` has no scope in its key
> (`evaluate.rs:797`/`:348`), so a module-scoped write could never affect
> the same response's recomputed activation set the way a top-level write
> does.

This slice closes that hole, and with it the read-side half of **D16**
(`docs/superpowers/specs/2026-09-11-module-expansion-design.md`: "all
instantiations of one `ModuleDef` evaluate against the same values").

In scope:

1. A scope-aware `ValueMap` in `knx-productdb`, so each instantiation
   evaluates its `choose` elements against *its own* stored values.
2. Retaining the `ModuleInstance/@Id` at import, so the `MI-` component of
   a module-qualified `ParameterInstanceRef` id comes from the project's own
   data instead of being invented.
3. Making module-scoped fields editable, writing the verbatim
   module-qualified `ets_id` the project already uses.
4. Turning the two places that currently discard or silently collapse data
   — the dropped `@Id`, and a `HashMap` insert that lets one stored row
   overwrite another — into retained data and a loud report.

Out of scope, and still out: `Module` *arguments* (`NumericArg`/`TextArg`/
`AllocatorRef`) remain stored-but-uninterpreted; repeated instantiation with
`MI-` > 1 is *rejected*, not supported (§D40); nothing here claims ETS
behavioural parity.

## Evidence

Every fact below was measured in this repository at `31c60e5`, or quoted
from the KNX Standard v3.0.0 extraction at
`/mnt/daten-i/Sourcecode/knx-spec-kb/extracted/The KNX Standard v3.0.0/`.
Markers follow RESEARCH.md: **[D]** documented in the Standard, **[V]**
verified against the corpus, **[A]** assumption.

### E1. The id grammar, both sides of it — [D] for the tokens, [V] for the mangling

`Project Schema23 v01.00.00.md` §1.2.5.18, `complexType ModuleInstance_t`,
defines the project-side element's id verbatim:

> `Id` … required … The shortened id of the module instance. For Modules:
> `MD-ModuleDefUniqueNumber_M-ModuleUnqiueNumber_MI-ModuleInstance@RepeatIndex`

and describes `@RepeatIndex` as a list of `XmlOrder x repeat counter` order
infos, e.g. `37x2`.

`OriginalData/DemoProjects/KV v2.5 - demo.knxproj`, `P-03DE/0.xml`, agrees
on the shape and disagrees with the naming — [V]:

```xml
<ModuleInstance Id="MD-2_M-2_MI-1" RefId="MD-2_M-2" RepeatIndex="10x1">
```

The `MI-` token is the literal digit `1`; `@RepeatIndex` is `"10x1"`. So the
embedded `MI-<k>` is *not* the `@RepeatIndex` string. It is consistent with
being that string's second ("repeat counter") component — which is `1` in all
32 `ModuleInstance` elements of this project — but nothing in the extraction
says so, and RESEARCH.md's "sharpest unknown #1" (`docs/RESEARCH.md:838-845`)
stays open. This slice is built so that it does not need the answer (§D40).

The application-program side is corpus-observed only — [V], and the
extraction contains no normative `ModuleDef`/`Module`/`Dynamic` schema at
all (confirmed by search: `ModuleDef` appears in exactly one file, and only
as the project-side `ModuleInstance_t` and the `ModuleDefArgType_t` enum).
The three id forms line up like this, for KV's switch actuator:

| | value |
|---|---|
| declared `ParameterRef/@Id` (product db) | `M-00FA_A-2504-10-C071_MD-2_P-1_R-1` |
| program-side `Module/@Id` (product db) | `M-00FA_A-2504-10-C071_MD-2_M-4` |
| project-side `ModuleInstance/@Id` | `MD-2_M-4_MI-1` |
| stored `ParameterInstanceRef/@RefId` | `M-00FA_A-2504-10-C071_MD-2_M-4_MI-1_P-1_R-1` |

i.e. the stored id is `<application program id>_<ModuleInstance/@Id>_<the
declared ref's own suffix>`, and `Module/@Id` is the same string with the
`_MI-<k>` component removed. This is the bridge the whole slice rests on.

### E2. The corpus — [V], re-measured, not quoted

- Module-qualified `ParameterInstanceRef` rows: KV v2.5 demo **9/9**; Unser
  Zuhause ETS 6.3.0 **0/1343**; Unser Zuhause ETS 4 **0/1390**.
- Five of KV's nine share one declared `ParameterRef`
  (`…_MD-2_P-1_R-1`) with five genuinely different values — `17`, `33`,
  `49`, `32`, `48` — under `M-2`…`M-6`. This is the shape the whole slice
  exists for.
- `_MI-` occurs only ever as `_MI-1_`, across all three projects.
- `Module/@Id` is present on **102/102** `<Module>` elements in the seven
  module-bearing program files in the corpus (`prod3`'s three, `kv25`'s
  four). `module_id: None` is therefore unexercised, not impossible.
- No test in the workspace installs a module-bearing program from a real
  `.knxproj`; `apps/knx-server/tests/http_parameter_panel.rs`'s
  `KV_SHAPE_PROGRAM` is a hand-built mirror of KV's shape.
- **No fixture anywhere has a module-scoped `ParameterRef` that both holds
  divergent per-channel values and controls a `choose` in the same
  `ModuleDef`.** The bug this slice fixes is therefore proven from the code
  path, not from a currently-wrong observed activation. Building that
  fixture is a task in the plan, not an optional extra.

### E3. The single read site — [V], code

`crates/knx-productdb/src/dynamic/evaluate.rs`:

- `:348` `pub type ValueMap = HashMap<String, String>;` — keyed by declared
  `ParameterRef` id alone.
- `:797`, inside `evaluate_comparable_choose`, is the **only** place any
  value is read: `node.ref_id.as_deref().and_then(|id| values.get(id))`.
- That function already has `scope: Option<&ModuleScope>` as a parameter
  (`:785-795`), passed down from `walk`'s `Module` arm (`:678-682`) through
  `evaluate_choose` unchanged. **No new threading is needed** to make the
  read scope-aware; only the map's key and lookup change.
- `:671-704`, the `Module` arm, passes the *same* `&ValueMap` into every
  instantiation's `walk`. That is the exact site where D16 is load-bearing:
  for KV's five channels, a `choose` gated on `…_MD-2_P-1_R-1` would today
  resolve identically in all five, using the program default — because the
  five stored values never enter the map at all (D21).

Callers to update: `apps/knx-server/src/domain.rs:2013` (the one production
caller of `resolve_values`), `crates/knx-productdb/tests/dynamic_tree.rs:
1541,1817,1949` (empty `supplied`), and the literal-map test helper at
`dynamic_tree.rs:629` used by five tests.

### E4. The write path — [V], code

`Command::SetParameterValue`/`RestoreParameterValue` target a
`ParameterInstance` by `(device: DeviceId, ets_id: String)` with **verbatim
string equality** (`crates/knx-core/src/command.rs:454-482,638-671,673-720`);
`knx-core` deliberately does not know what a program declares (its own
comment, `:104-107`). `RestoreParameterValue` is only ever constructed as
the inverse of an already-validated `SetParameterValue`, so undo inherits a
validated id and needs no new logic.

Validation lives at the `knx-server` layer, pre-command
(`apps/knx-server/src/domain.rs:2224-2283`): check 1 rejects any `ets_id`
not in `ref_ids` (which is every module-qualified id), check 2 rejects a
bare declared id that is only reachable inside a `scope: Some(_)` section.
Nothing today mis-targets a write silently — module-qualified ids are
refused outright.

### E5. The `MI-` component is imported, then dropped — [V], code

`crates/knx-etsproj/src/parse/installation_v21.rs:764-769` parses
`ModuleInstance` with **both** `Id` (required) and `RefId` (required) into
`SourceModuleInstance`. But `crates/knx-etsproj/src/map.rs:1038-1049` uses
`mi.id` only as a local `BTreeMap` key for wiring com objects, and stores
`source.ets_id = mi.ref_id` — so `knx_core::ModuleInstance` keeps
`MD-2_M-4` and loses `MD-2_M-4_MI-1`. `crates/knx-store`'s
`module_instance` table (`module_instance.rs:17`) persists
`source_ets_id`/`repeat_index`, so the loss is in the domain model, not only
in SQL. Export is unaffected (the verbatim `<ModuleInstances>` subtree is
retained, `crates/knx-etsproj/src/source.rs:103-106`), so this is a
*domain-model* loss, not a roundtrip loss — but it is exactly the datum the
write path needs, and `CLAUDE.md`'s "never silently discard information"
covers it either way.

## Decisions

### D35. `ValueMap` becomes a type with a scope dimension, not a `HashMap` alias

`ValueMap` stops being `HashMap<String, String>` and becomes a struct
holding two maps: unscoped values keyed by declared `ParameterRef` id, and
scoped values keyed by `(module_id, declared_ref_id)` — where `module_id` is
the program-side `Module/@Id`, the same string `ModuleScope::module_id`
already carries (`evaluate.rs:680`, from `Module/@Id` via
`dynamic/parse.rs:394`).

Why the program-side `Module/@Id` and not `ModuleScope::module_node`: the
`i64` node id is a product-database row number, meaningless to the project
side, and it changes when a package is reinstalled. `module_id` is the datum
both sides can name. `ScopeKey = (Option<i64>, String)` (`:518`) stays as it
is — it keys `Activation`'s internal dedup, a different job.

The public surface:

```rust
pub struct ValueMap { /* unscoped: HashMap, scoped: HashMap */ }

impl ValueMap {
    /// Scoped value for this scope if there is one, otherwise the
    /// program-level value. Never another instantiation's value.
    pub fn get(&self, scope: Option<&ModuleScope>, ref_id: &str) -> Option<&str>;
    /// The program-level value, ignoring scope — the display path's
    /// `ProgramDefault` fallback.
    pub fn get_unscoped(&self, ref_id: &str) -> Option<&str>;
    pub fn insert_scoped(&mut self, module_id: String, ref_id: String, value: String);
    pub fn len_scoped(&self) -> usize;
}

impl From<HashMap<String, String>> for ValueMap { /* unscoped only */ }
```

`From<HashMap<_, _>>` keeps the five literal-map tests (`dynamic_tree.rs`
via the `values()` helper at `:629`) readable: `evaluate(&trees,
&values(...).into())`.

### D36. Lookup falls back to the program level, never sideways

`get(Some(scope), id)` tries `(scope.module_id, id)` first, then the
unscoped map. `get(None, id)` tries the unscoped map only. It never consults
another instantiation's value, and it never consults a scoped value for a
top-level read. A channel with no stored value of its own behaves exactly as
it does today — the program default — which is also what the panel already
labels `ProgramDefault`.

Cost if wrong: a channel that should have inherited something other than the
program default shows and evaluates the program default. That is today's
behaviour for every channel, so the change cannot regress it.

### D37. A `Module` with no `@Id` is reported, once, at its expansion site

`ModuleScope::module_id` is `Option<String>` because `Module/@Id` is an
optional XML attribute read (`parse.rs:394`); it is `None` in 0/102 corpus
elements (E2) but nothing guarantees that for a package we have not seen.
When `walk`'s `Module` arm expands a node whose `element_id` is `None`, it
pushes a new diagnostic:

```rust
/// [V] `Module/@Id` is present on 102/102 corpus elements, but it is an
/// optional attribute. Without it, this instantiation cannot be matched to
/// a project-side `ModuleInstance`, so its stored per-channel values are
/// unreachable and it evaluates against program defaults.
ModuleWithoutId { node_id: i64 },
```

It is emitted per instantiation, not per read, and unconditionally — a
`Module` we cannot name is worth saying out loud even when no value would
have applied. The server surfaces it as a section diagnostic, and the
section's fields stay read-only (D39).

Why not synthesise an id from `module_node`: it would be a fabricated
identifier that looks like project data and matches nothing. Fail loud.

### D38. The `MI-` component comes from the project, via a retained `@Id`

`knx_core::ModuleInstance` gains one field:

```rust
/// The verbatim `ModuleInstance/@Id` — e.g. `"MD-2_M-4_MI-1"`, the
/// `RefId` (`source.ets_id`) plus the `_MI-<k>` component the id grammar
/// puts on it ([D] Project Schema23 §1.2.5.18, [V] KV v2.5 demo).
/// Retained uninterpreted, same policy as `repeat_index`; the parameter
/// editor reads it to reconstruct the module-qualified `ets_id` a write
/// must target, rather than guessing `MI-1`.
pub instance_ets_id: String,
```

populated from `SourceModuleInstance::id`, which the parser already requires
(E5), and persisted as a new `module_instance.instance_ets_id` column with a
`knx-store` migration. Existing rows migrate to `''` (empty), which D39
treats exactly like a missing instance: read-only, reported, never guessed.

Why not default the `MI-` component to `1`: it is right in 100% of the
corpus and it is still a guess about a component whose semantics
RESEARCH.md explicitly lists as unresolved. Writing a wrong `ets_id` puts a
row into a saved project that no program will ever match again — a
data-integrity cost, paid silently, forever. Reading the project's own
answer costs one column.

### D39. Editability of a module-scoped field is earned per section

`let editable = section.scope.is_none();`
(`apps/knx-server/src/domain.rs:2065`) becomes: a section is editable if it
has no scope, **or** all of the following hold, checked in this order and
each failure reported as a section diagnostic:

1. the scope has a `module_id` (D37);
2. the device has exactly one imported `ModuleInstance` whose
   `source.ets_id` matches the trailing `MD-<d>_M-<n>` of that `module_id`
   after a `_` separator;
3. that instance's `instance_ets_id` is non-empty and decomposes as
   `<its own RefId>_MI-<digits>` (D38).

Then the write target is `format!("{module_id}_MI-{k}_{suffix}")`, where
`suffix` is the declared ref id with the `module_id`'s own prefix (its text
before `_M-<n>`) and one `_` stripped — which reproduces the corpus's stored
ids exactly (E1), and is asserted to do so against KV's five real ids.

Two `ModuleInstance`s sharing a `RefId` (a genuinely repeated module, MI > 1)
fails rule 2 → read-only with a diagnostic. That is D40.

### D40. `MI-` > 1 is refused, not supported

`ValueMap`'s scoped key is `(module_id, ref_id)` with no `MI-` dimension,
because a program-side `Module` node carries no repeat index at all — the
evaluator has nothing to key it by. So a device with two `ModuleInstance`s
sharing one `RefId` cannot have its channels told apart on the read side,
and this slice does not pretend otherwise:

- its module-scoped fields are read-only, with a diagnostic naming the
  duplicate `RefId`;
- a stored row whose `MI-` digits do not equal the one authoritative
  instance's is reported `stale` (D41), not merged;
- a new limitation in `KNOWN_LIMITATIONS.md` says so, pointing at
  RESEARCH.md's sharpest unknown #1 as the thing that would have to be
  researched first.

0/32 `ModuleInstance` elements in the corpus need this. Supporting it on
present evidence would mean inventing the key.

### D41. Stored-row validation gains the `MI-` check, and stops overwriting silently

Pass B (`domain.rs:2027-2040`) keeps its two existing conditions —
`module_ids.contains(module_id) && ref_ids.contains(declared_id)` — and
gains:

- **`MI-` agreement**: if an authoritative `instance_ets_id` exists for that
  `module_id` (D39 rules 2-3) and its `MI-` digits differ from the row's,
  the row is `stale`. Where no authority exists, the `MI-` component is not
  checked — display keeps working exactly as it does today for projects
  imported before this slice, and the section is read-only anyway.
- **no silent collapse**: `module_scoped.insert((module_id, declared_id),
  raw)` today lets a second row overwrite the first with no trace. Two rows
  reaching the same key now produce a diagnostic naming both `ets_id`s, and
  the loser is listed `stale` rather than vanishing. With the `MI-` check in
  place this is unreachable through the decomposition route, which is
  precisely why it is cheap to make loud.

### D42. The validated scoped values feed `evaluate`, and the panel's display reads through the same map

`assemble_parameter_panel`'s order changes, because Pass B's validation
needs `module_ids`, which needs an activation, which now needs the scoped
values:

1. Pass A, unchanged: sort stored rows into `supplied` / `candidates` /
   `stale`.
2. `resolve_values(&products, &program_id, &supplied)` → `ValueMap` with
   unscoped values only, then `evaluate` → a **provisional** activation,
   used for nothing but its `module_id` set.
3. Pass B: validate candidates against that set (D41) → scoped values.
4. `insert_scoped` them into the `ValueMap`, then `evaluate` **again** → the
   activation the response is built from.

Two evaluations, not one. Measured cost: `evaluate` over the corpus's
largest module-bearing program is well under the ~1.26 ms figure T34 is
about, and the alternative — resolving `Module/@Id`s without evaluating —
would mean a second, parallel implementation of module expansion. Simpler
beats faster here, and the second pass is skipped entirely when there are no
scoped candidates, which is every project in the corpus except KV.

The display path then reads `values.get(Some(scope), ref_id)` instead of
consulting `module_scoped` directly, so *one* lookup rule governs both what
the user sees and what the evaluator decided. `value_source` stays
`"Stored"`/`"ProgramDefault"`, decided by which of the two maps answered.

### D43. The field DTO carries the id a write must use

`ParameterFieldDto` gains `write_ets_id: Option<String>` —
`Some(field.ets_id)` for an unscoped field, `Some(module_qualified)` for an
editable module-scoped one, `None` when the field is not editable. `ets_id`
keeps meaning "the declared `ParameterRef` id", which is what the panel
labels, keys its rows by, and joins views on.

`ParameterPanel.tsx` writes `field.writeEtsId` and disables the control when
it is `null`, which is already the shape of its `editable` check. Server-side
write validation (E4's checks 1 and 2) is replaced by: accept `ets_id`
verbatim if it is in `ref_ids` **and** reachable in an unscoped section;
otherwise accept it if it is the `write_ets_id` of an editable field in the
panel this request just assembled; otherwise reject with the reason. The
panel is the single authority on what is writable — no second id-shape
parser on the write path.

## Non-goals and follow-ups

- **`Module` arguments stay uninterpreted.** `NumericArg`/`TextArg` are
  stored; `AllocatorRef` is unattested in the corpus. Nothing here reads
  them.
- **Repeated instantiation (`MI-` > 1) stays unsupported** (D40), and the
  `MI-`↔`@RepeatIndex` question stays RESEARCH.md's open #1.
- **No ETS parity claim.** ETS's own per-channel activation is not
  observable here; this slice makes KNXBench's activation follow
  KNXBench's documented rules, per channel.
- **Nested modules.** A `Module` inside a `ModuleDef`'s own tree is not
  expanded (design D15; `walk`'s `Some(_)` scope arm reports
  `Diagnostic::NestedModuleNotExpanded` instead of descending). This slice
  does not change that, and the scoped key would need a scope *path*, not a
  single `module_id`, if it ever does.
