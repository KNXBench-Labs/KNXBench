# `Module` expansion in the `Dynamic` evaluator (T18, second slice)

## Status and scope

**Status:** design, 2026-09-11. Follows the first slice
(`2026-09-11-dynamic-tree-parse-and-evaluate-design.md`, decisions D1-D11,
merged as `1e0d073`) and the R4 research spike (`docs/RESEARCH.md` §4.4,
merged as `4f278e4`).

Decision numbering continues at **D12** so that a reference to "design D10"
in a source comment keeps meaning exactly one thing across both documents.

**In scope.** Make the evaluator follow a `Module` node into the referenced
`ModuleDef`'s own stored `Dynamic` tree, so that a modular application
program's active `ParameterRef`/`ComObjectRef` set is complete rather than
truncated at the module boundary. Qualify every activation and every
diagnostic by the instantiating `Module`, so that twelve instantiations of
one `ModuleDef` are twelve results and not one.

**Explicitly out of scope**, each for a stated reason:

- **Structured argument values.** `Module/NumericArg` and `Module/TextArg`
  are stored losslessly today (as `dynamic_node` rows with their attributes
  in `extra`) and stay that way. §4.4 Q3 establishes that `choose` never
  branches on an `Argument`, so activation-set computation does not need
  them. There is no `argument` or `module_def` table and this slice does not
  add one.
- **Memory-offset placement and text-template substitution.** The
  `LParameters`/`RParameters`/`ParameterCalculations`/`Union`/`Memory`
  mechanism §4.4 Q2 flags inside `ModuleDef/Static` is unresearched. Any
  work that needs it needs its own spike first.
- **Per-instantiation parameter *values*.** See D16 — this is a real,
  documented limitation of this slice, not an oversight.
- **Project-side `ModuleInstance` resolution.** `ModuleInstance`,
  `@RepeatIndex` and the `_M-<m>_MI-<k>_` id mangling are a `.knxproj`
  concern. Per ADR-0014 import reads `GroupObjectTree` and never evaluates
  this tree at all; `knx-productdb` stays at the
  `ApplicationProgram`+`ModuleDef`+`Module` level (§4.4 Q8d).
- **Schema changes.** `dynamic_node` already stores everything this slice
  reads. The product database stays at **v3**; there is no v3→v4 migration
  in this slice.
- **Any UI.** There is still no parameter editor. Nothing here is reachable
  from `knx-app` or `knx-web`.

## Evidence

Every structural fact below is `[V]` — observed in the corpus — unless
marked `[D]`. `docs/RESEARCH.md` §4.4 carries the counts and the method;
this section states only what the decisions rest on.

- **[V]** A `Module` names its `ModuleDef` through `@RefId`, carrying the
  full id (`RefId="M-0083_A-0317-31-7DC6_MD-1"`), never a short id needing
  recovery through an owning element. 0 of 102 `Module` elements across 7
  module-bearing application-program files cross an `ApplicationProgram`
  boundary.
- **[V]** A `ModuleDef`'s `Dynamic` tree uses exactly the same element
  vocabulary as the program's own (`Channel`, `ParameterBlock`,
  `choose`/`when`, `ParameterRefRef`, `ComObjectRefRef`,
  `ParameterSeparator`), 3-4 levels deep.
- **[V]** `choose` inside a `ModuleDef` branches on a `ParameterRef`
  declared in that `ModuleDef`'s own `Static`, never on an `Argument`:
  57/57 distinct `choose/@ParamRefId` in
  `M-0083_A-0317-31-7DC6_MD-1` match a `ParameterRef`, 0/57 match an
  `Argument`, reproduced in the archive's two other application programs.
  Those `ParameterRef` rows are already ingested under the owning
  program's `program_id` by slice 1.
- **[V]** A `ModuleDef`'s local `ParameterRef`/`ComObjectRef` ids are
  declared **once** and reused verbatim by every instantiating `Module`.
  `M-0083_A-0317-31-7DC6_MD-1` declares 41 `ComObjectRef` ids and is
  referenced by 12 sibling `Module` elements.
- **[V]** No nesting anywhere in the corpus: zero `Module` elements inside
  any `ModuleDef/Dynamic`, and no `SubModuleDef` element on the
  application-program side at all.
- **[D]** The Standard defines no application-program-side complexType for
  `ModuleDef` or `Module` in the available extraction. There is therefore
  no `[D]`-strength statement about nesting, cycles, or expansion order to
  appeal to — which is precisely why D15 makes an explicit policy instead
  of inheriting one.
- **Implementation fact, not corpus:** `dynamic_node.node_id` is monotonic
  **per `(program_id, module_def_id)`** and resets at each `Dynamic` root
  (`dynamic/parse.rs`, `handle_dynamic_element`). Node ids therefore
  collide across trees. D14 exists because of this.

## Decisions

### D12. Expansion is a property of the walk, not of storage

`dynamic_node` is unchanged. A `ModuleDef`'s tree is already stored as its
own scope (`module_def_id = <the ModuleDef's @Id>`, slice 1's D2), and the
program's own tree is already stored with `module_def_id = ''`. Expansion
means: when the walk reaches a `Module` node in the program's tree, it
continues into the tree stored under that `Module`'s `@RefId`.

Nothing is copied, flattened, or pre-expanded into the database. A
pre-expanded table would have to be invalidated whenever a value changes,
and would bake this build's expansion policy into stored data — the exact
mistake slice 1 avoided by storing the tree verbatim and interpreting it
in a pure function.

### D13. `evaluate` stays pure; the input grows from one tree to a tree set

Slice 1's D6 keeps `evaluate` free of database access so its logic can be
unit-tested against hand-built trees. Expansion needs more than one tree,
so the input grows rather than the purity being given up:

```rust
pub struct ProgramTrees {
    program: DynamicTree,                     // module_def_id = ""
    modules: HashMap<String, DynamicTree>,    // keyed by ModuleDef @Id
}

impl ProgramTrees {
    pub fn from_parts(program: DynamicTree, modules: HashMap<String, DynamicTree>) -> Self;
    pub fn single(program: DynamicTree) -> Self;   // no modules; for unit tests
}

pub fn load_program_trees(conn: &Connection, program_id: &str)
    -> Result<ProgramTrees, ProductDbError>;

pub fn evaluate(trees: &ProgramTrees, values: &ValueMap) -> Activation;
```

`load_program_trees` is the only new database-touching function: it lists
the distinct `module_def_id` values present for the program and calls the
existing `load_tree` once per scope. `load_tree`, `resolve_control_kind`
and `resolve_values` are **unchanged in behaviour** — they are already
`program_id`-scoped with no `module_def_id` filter on the parameter side,
which is exactly what a `ModuleDef`-internal `choose` needs (§4.4 Q8a).

`evaluate`'s signature changes from `&DynamicTree` to `&ProgramTrees`.
This is a deliberate breaking change to a function with no callers outside
this crate (`grep -rn "dynamic::" crates/ --include=*.rs` returns nothing
outside `knx-productdb`); `ProgramTrees::single` keeps every existing
hand-built-tree unit test a one-line change.

### D14. Every activation and every diagnostic carries its module scope

The load-bearing decision of this slice. A `ModuleDef`'s local ref ids are
reused verbatim by every instantiating `Module` (Evidence), and node ids
collide across trees (Evidence). An unqualified result is therefore
ambiguous in two independent ways at once.

```rust
/// Which expansion produced this result. `None` = the application
/// program's own tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleScope {
    /// `dynamic_node.node_id` of the `Module` element in the program's own
    /// tree. Always present, unique within that tree, and the component
    /// dedup actually keys on.
    pub module_node: i64,
    /// `Module/@Id`, for reporting. `None` if the element carries none.
    pub module_id: Option<String>,
    /// `Module/@RefId`, i.e. the `ModuleDef/@Id` whose tree was walked.
    pub module_def_id: String,
}

pub struct ActiveRef {
    pub scope: Option<ModuleScope>,
    pub ref_id: String,
}

pub struct ScopedDiagnostic {
    pub scope: Option<ModuleScope>,
    pub diagnostic: Diagnostic,
}

pub struct Activation {
    pub parameter_refs: Vec<ActiveRef>,
    pub com_object_refs: Vec<ActiveRef>,
    pub diagnostics: Vec<ScopedDiagnostic>,
}
```

`Diagnostic`'s own variants are **not** touched — the scope wraps them,
so a diagnostic's `node_id`/`choose_node`/`when_node` keeps meaning "a
`node_id` within the tree named by the accompanying scope", which is the
same contract slice 1 documented, now with the scope actually present.

`module_node` rather than `module_id` is the dedup key because it is
total: it exists for every `Module` element whether or not the element
carries an `@Id`, and it is unique by construction. `module_id` is carried
alongside for human-readable reporting and is never relied on for
identity.

### D15. Exactly one level of expansion; a nested `Module` is reported, not followed

A `Module` node inside a `ModuleDef`'s tree is **not** expanded. It
produces `Diagnostic::NestedModuleNotExpanded` and its subtree is not
descended.

The corpus has zero nested modules and the Standard extraction defines no
application-program-side `ModuleDef` complexType at all, so there is
nothing to implement against and nothing to validate against. The
alternatives were:

- *Recurse until it terminates* — rejected. With no documented nesting
  rule there is no documented cycle rule either, and an evaluator that can
  be made to loop by a manufacturer file is a defect regardless of how
  unlikely the file is.
- *Recurse with a depth cap* — rejected. A cap is a number nothing in the
  evidence supports, and it would produce partially-expanded results that
  look complete.
- *Silently ignore* — rejected outright by the project's data-integrity
  rule. A gap that is visibly a gap is acceptable; one that is invisible
  is not.

One level plus a loud diagnostic is the only option that is complete for
everything the corpus contains, honest about everything it does not, and
incapable of looping. If a nested sample ever appears, the diagnostic is
what surfaces it.

### D16. All instantiations of one `ModuleDef` evaluate against the same values — stated as a limitation

`ValueMap` is keyed by `ParameterRef` id, and a `ModuleDef`'s
`ParameterRef` ids are shared by all its instantiations. This slice
therefore evaluates every `Module` instantiating a given `ModuleDef`
against **identical** parameter values, so their activations differ only
in scope, not in content.

That is correct for what a product database contains: at the
application-program level there is exactly one set of declared parameter
values, and genuinely per-channel values are a *project*-side construct
(`ParameterInstanceRef`, mangled `_M-<m>_MI-<k>_` ids, §4.4 Q5) that
`knx-productdb` does not model and ADR-0014 keeps out of the import path.

It is nevertheless a real limitation on what the evaluator can answer, and
it goes in `docs/KNOWN_LIMITATIONS.md` in those words. No API is added for
per-instantiation value supply in this slice: there is no caller for it,
and inventing a key spelling for a consumer that does not exist is how
speculative abstractions get in.

### D17. `Module` diagnostics, replacing `ModuleNotExpanded`

`Diagnostic::ModuleNotExpanded` is **removed**. It said "this slice does
not follow `Module`", which stops being true. Three variants take its
place, and they are not interchangeable:

| Variant | Means | Subtree |
| --- | --- | --- |
| `ModuleDefNotFound { node_id, ref_id }` | `Module` has no `@RefId`, or names a `ModuleDef` with no stored tree in this program | not descended |
| `NestedModuleNotExpanded { node_id, ref_id }` | a `Module` inside a `ModuleDef`'s tree (D15) | not descended |
| *(no diagnostic)* | expanded normally | walked, scoped |

A `Module` whose `ModuleDef` tree exists but is *empty* is not a
diagnostic: an empty tree legitimately activates nothing, and there is no
evidence that it is malformed.

### D18. Dedup is per scope; ordering is program tree first, then each module in document order

Slice 1's D11 (document order, deduplicated by first occurrence) is kept
and qualified: the dedup key becomes
`(Option<module_node>, ref_id)`. Within one module's expansion an id
reachable through two active branches still appears once; across two
instantiations of the same `ModuleDef` the id appears once per
instantiation, which is the entire point of D14.

Traversal order is unchanged — single-pass, depth-first, document order —
with a module's expansion inlined at the position of its `Module` node, so
the result reads in the order the file does.

### D19. `Module` nodes in the program tree are reached the same way as any other node

No special container handling: `Module` is dispatched in `walk`'s `match`
exactly where `ModuleNotExpanded` is pushed today. `Module`'s own children
in the program tree (`NumericArg`, `TextArg`) are **not** descended into —
they are argument bindings, not activations, and descending would report
them as `UnrecognizedNode` for no benefit. They remain stored and
reportable; they are simply not part of the activation walk.

## Acceptance criteria

1. `evaluate` expands a `Module` in the program's own tree into the tree
   stored under its `@RefId`, and the resulting activations carry a
   `ModuleScope` naming the instantiating `Module`.
2. Two `Module` elements instantiating the same `ModuleDef` produce two
   separate sets of activations for the same local ref ids — no collapse
   (D14/D18). A unit test asserts exactly this against a hand-built
   `ProgramTrees`, with no database.
3. A `Module` inside a `ModuleDef`'s tree yields
   `NestedModuleNotExpanded` and contributes no activations (D15).
4. A `Module` with no `@RefId`, and a `Module` naming an absent
   `ModuleDef`, both yield `ModuleDefNotFound` (D17).
5. `Diagnostic::ModuleNotExpanded` no longer exists anywhere in the crate.
6. A corpus test over `OriginalData/ProductDatabases` asserts, for the
   module-bearing programs (`prod3`'s three, `kv25`'s four — the only ones
   in the corpus, §4.4 Q7): zero `ModuleDefNotFound`, zero
   `NestedModuleNotExpanded`, and per-program activation counts that are
   strictly greater than slice 1's, with the expected numbers **derived
   from the corpus and recorded in the test**, not fitted afterwards to
   whatever the implementation happens to print. It must skip loudly when
   the corpus is absent, matching the existing corpus tests' idiom.
7. The non-module corpus programs' activation counts are **unchanged** by
   this slice; a regression test pins at least one of them.
8. `evaluate` remains pure: no `Connection` parameter, no I/O, no logging.
9. All five gates pass: `cargo fmt --all --check`, `cargo clippy
   --workspace --all-targets -- -D warnings`, `cargo test --workspace`,
   `cargo run -p xtask -- check-layering`, `cargo deny check`.
10. The documentation set states what now exists and what still does not,
    without downgrading a standing limitation and without claiming ETS
    parity or KNX certification anywhere.

## Non-goals and follow-ups

- **T18 slice 3** — a parameter editor — remains the next slice, and is
  where per-instantiation values (D16) would first have a real consumer.
- **Argument semantics** (`Memory/@BaseOffset` placement, `{{ChNo}}` text
  substitution, the `ModuleDef/Static` scaled-memory constructs) need the
  research spike §4.4 Q2 flags before anything is built on them.
- **`AllocatorRef`** (`ModuleDefArgType_t`'s third facet) has zero corpus
  occurrences and stays unimplemented and undocumented beyond that fact.
- **`RepeatIndex`'s `"NxM"` encoding** stays an opaque string in
  `knx-core` per ADR-0013.

## Addendum (goal.md T18, task 11): D15 superseded — bounded recursive expansion

D15 stood on "the corpus has zero nested modules and there is nothing to
implement against." `goal.md` §3 T18 asked for the gap to be closed anyway,
bounded and cycle-safe rather than left as a one-level policy limit. This
addendum supersedes D15 and D17's `NestedModuleNotExpanded` row; D12-D14,
D16, D19 are unaffected.

### D44. `Module` nesting is expanded recursively, bounded by named constants

A `Module` found while walking an already-expanded `ModuleDef`'s tree is now
expanded the same way a top-level `Module` is: its `@RefId` is looked up in
`ProgramTrees`, and if found, its tree is walked with a new `ModuleScope`
whose `parent` is the enclosing scope. `ModuleScope` becomes a chain
(`parent: Option<Box<ModuleScope>>`) rather than a single flat record, and
carries a `depth()` method (1 + parent depth, 0 if none).

Expansion stops — with a diagnostic, not a panic and not silent truncation —
once `ModuleScope::depth()` would exceed
`evaluate::MAX_MODULE_NESTING_DEPTH = 16`. **[A]** No source states a
numeric bound for AP-side nesting; see the constant's own doc comment for
the inference and the fresh `pdftotext -layout` re-extraction of `Project
Schema23 v01.00.00.pdf` confirming (again) that the KNX Standard defines no
application-program-side `ModuleDef`/`Module` complexType at all. The
Standard is not silent about module nesting everywhere, though:
`ModuleInstance_t/@Id`'s documented grammar (§1.2.5.18, project-instance
side, **[D]**, already recorded at `docs/RESEARCH.md` §4.4 Q6) gives exactly
one extra level, never a second `SubModule` segment. That text is
project-side, not AP-side, so it does not settle this constant's value on
its own — but it is the one documented data point that speaks to nesting
depth at all, and it says 2, not 16; `16` is chosen deliberately far above
that one documented neighbour, not found in the Standard and not fitted to
any known file.

**Fix round 1, blocking finding 1 (goal-completion task 11):** the depth
bound alone does not bound *total* work — a non-cyclic tree with fan-out at
every level can multiply activations combinatorially without any single
chain exceeding `MAX_MODULE_NESTING_DEPTH`. A measured probe
(`depth=12, fanout=4`, 44-node input) reached 4,194,304 `Module`
activations, 11.5s and 8,170 MiB peak RSS — well short of depth 16. A
second constant, `evaluate::MAX_MODULE_EXPANSIONS = 100_000`, now caps the
total number of `Module` expansions any one `evaluate` call will perform,
checked alongside the depth bound and refused the same way, with
`Diagnostic::ModuleExpansionBudgetExhausted`. See that constant's own doc
comment for where `100_000` comes from (roughly 260x the corpus's measured
legitimate ceiling of 382 activations, recorded at
`docs/IMPLEMENTATION_STATUS.md`'s "Corpus regression coverage" entry, not
RESEARCH.md — fix round 2 correction, the citation was wrong in round 1).

**Fix round 2, and its own residual in round 3:** an expansion count is not
a work count. A `ModuleDef` carrying many `ParameterRefRef` children,
expanded a modest number of times well inside the 100,000 ceiling, still
reached multi-GB peak RSS — measured, standalone binary over the public
API at `opt-level = 2`: 1,325,196 activations at 1,946 MiB, and 3,825,596
at 5,691 MiB, both inside the expansion budget throughout **[V]**. Round 2
added a third constant, `evaluate::MAX_MODULE_ACTIVATIONS = 1_000_000`,
bounding the combined `parameter_refs`/`com_object_refs` count one
`evaluate` call may record, refused with the same
`Diagnostic::ModuleExpansionBudgetExhausted` variant distinguished by its
`budget` field, and emitted exactly once rather than once per refused ref.

Round 3 (from the scoped re-review of round 2) makes that budget stop the
walk rather than only the recording. Round 2 checked it at the two
activation sites and *after* the per-scope dedup, and `walk`'s `Module` arm
never consulted it, so a run that had spent its activation budget kept
expanding until the 100,000-times-larger expansion budget stopped it, and
every ref it met on the way still allocated a `ScopeKey` into a `seen` set
no budget bounded. The check now sits ahead of the dedup and is repeated in
the `Module` arm, where a spent budget refuses further expansion outright.

### D45. Cycles are detected by scanning the whole ancestor chain, not just the immediate parent

Before expanding a `Module`'s target `ModuleDef`, `chain_contains` walks
`self` and every `parent` comparing `module_def_id`. A `ModuleDef` that
(directly or through intermediate `ModuleDef`s) contains a `Module` naming
itself again is refused with `Diagnostic::ModuleCycleDetected { node_id,
ref_id }` and its subtree is not descended — never a stack overflow, never a
silent stop. This check runs *before* the depth-bound check, so a cycle is
reported as a cycle even when it would also have crossed the depth bound.

A non-cyclic chain that is simply too deep is refused with
`Diagnostic::ModuleNestingTooDeep { node_id, ref_id, depth }`, where `depth`
is the depth the expansion would have reached.

`Diagnostic::NestedModuleNotExpanded` (D17's row) is **removed** — a `Module`
inside a `ModuleDef`'s own tree is no longer a standing diagnostic by
itself; it is either expanded, refused as a cycle, or refused as too deep.

### D46. The dedup/scope key is qualified by the full ancestor node chain

D18's key, `(Option<module_node>, ref_id)`, assumed at most one enclosing
`Module`. A `node_id` is only unique within one `(program_id,
module_def_id)` tree (`dynamic_node`'s own storage, design D1-D5), so two
different nesting chains can reuse the same `node_id` at the same depth
under different ancestors — a flat `Option<i64>` would collide them. The key
becomes `(Vec<i64>, String)`: the full chain of `module_node` ids from the
program root down, plus `ref_id`. `ModuleScope::node_chain()` builds this
vector by walking `parent` outward-in.

**Fix round 1, blocking finding 4 (goal-completion task 11):** this crate's
own `evaluate`/`Activation` dedup got the widened key above, but
`apps/knx-server/src/domain.rs`'s parameter-panel section grouping did not
— it kept keying `section_order`/`sections_by_key` on the flat
`Option<module_node>` D18 used, so two distinct nesting chains sharing a
`module_node` (the branch's own two-level test constructs exactly this
pair) collided into one section instead of two. `node_chain()` is now
`pub`, and the server keys sections on `Option<Vec<i64>>` the same way this
crate does.

### Corpus measurement (task 11, required deliverable)

Measured, not guessed, against every `.knxprod` file present under
`OriginalData/ProductDatabases/` (all 4 archive files currently present
locally: `646704-04_ETS4_2012_47_DE_EN`, `Dummy_Applikation_Secure`,
`MDT_KP_AMI_AMS_03_Switch_Actuator_V31a` (3 application programs),
`Weinzierl_730_KNX_IP_Interface_ETS4`), two independent ways:

1. A raw XML scan (`xml.etree.ElementTree`, scratch script, outside the
   repo) over every extracted application-program XML file counted `Module`
   elements found inside a `ModuleDef` element's own subtree: **0** across
   all 6 application-program files.
2. `crates/knx-productdb/tests/dynamic_tree.rs`'s
   `corpus_nested_module_measurement_task_11` installs the same 4 archive
   files into a fresh database and runs
   `SELECT COUNT(*) FROM dynamic_node WHERE kind = 'Module' AND
   module_def_id != ''` (a `Module` row stored under a non-empty
   `module_def_id`, i.e. inside a `ModuleDef`'s own tree rather than the
   program's): **0**, out of a total of 86 stored `Module` rows overall
   (`kind = 'Module'`, any `module_def_id`; both counts are now asserted by
   the test, not merely printed).

Both measurements agree: **zero products in the installed database nest
modules.** D44/D45's bounded recursion is therefore exercised, in this
corpus, only by the new synthetic unit tests — the corpus itself gives it
nothing to expand.

### Acceptance criteria addendum

11. A unit test with two genuine nesting levels (`Module` inside a
    `ModuleDef`'s tree naming a *different* `ModuleDef`, no cycle) is
    expanded fully, with a two-deep `ModuleScope` chain on the resulting
    activations.
12. A unit test with a cycle (a `ModuleDef`'s tree, directly or through one
    intermediate `ModuleDef`, names a `ModuleDef` already in the ancestor
    chain) yields `ModuleCycleDetected` and no activation from the cyclic
    branch — and does not overflow the stack.
13. A unit test at exactly `MAX_MODULE_NESTING_DEPTH` expands fully with no
    diagnostic; one level deeper yields `ModuleNestingTooDeep { depth:
    MAX_MODULE_NESTING_DEPTH + 1, .. }` and no activation from the refused
    branch.
14. The corpus measurement above is recorded here and in
    `docs/RESEARCH.md`, not asserted from memory.
15. All six gates pass (the deny/layering/headers set, not just the five
    D-era ones): `cargo fmt --all -- --check`, `cargo clippy --workspace
    --all-targets -- -D warnings`, `cargo test --workspace --no-fail-fast`,
    `cargo run -p xtask -- check-layering`, `cargo run -p xtask --
    check-headers`, `cargo deny check`.
16. *(Fix round 1, blocking finding 1.)* A unit test with a non-cyclic
    fan-out tree that would exceed `MAX_MODULE_EXPANSIONS` before it would
    exceed `MAX_MODULE_NESTING_DEPTH` yields
    `ModuleExpansionBudgetExhausted { budget: MAX_MODULE_EXPANSIONS, .. }`
    and `evaluate` returns rather than continuing to expand.
16b. *(Fix round 2, and extended in round 3.)* A unit test with a fan-out
    tree whose refs-per-expansion exceed `MAX_MODULE_ACTIVATIONS` while its
    expansion count stays well under `MAX_MODULE_EXPANSIONS` yields exactly
    one `ModuleExpansionBudgetExhausted { budget: MAX_MODULE_ACTIVATIONS, .. }`
    and stops recording refs at the budget
    (`a_wide_module_def_trips_the_activation_budget_without_tripping_the_expansion_budget`).
    A second test, whose unrefused expansion count would overrun
    `MAX_MODULE_EXPANSIONS` 223 times over, proves the spent activation
    budget ends the walk: the expansion budget's own diagnostic never
    appears (`a_spent_activation_budget_stops_further_module_expansion`,
    `crates/knx-productdb/tests/dynamic_tree.rs`). Both were confirmed to
    fail without their fix.
17. *(Fix round 1, blocking finding 4; citation corrected fix round 2 —
    round 1 named the wrong test here.)* A unit test with two distinct
    nesting chains that reuse the same `module_node` at the same depth
    under different ancestors produces two separate parameter-panel
    sections in `apps/knx-server`, not one collided section:
    `two_nesting_chains_sharing_a_module_node_do_not_merge_into_one_section`
    (`apps/knx-server/tests/http_parameter_panel.rs`) is that test — not
    `a_module_inside_a_module_def_naming_a_different_module_def_is_expanded_two_levels`
    (`crates/knx-productdb/tests/dynamic_tree.rs`), which builds a single
    chain whose two scopes both happen to use node id 0 and never
    produces two sections in the first place.

---

## Addendum (goal.md T18, task 12, 2026-09-14): argument interpretation

Task 11 left `Module` arguments exactly where slice 2 put them: stored,
reportable, and consumed by nobody. The out-of-scope bullet at the top of
this document ("**Structured argument values.** … are stored losslessly
today … and stay that way. … There is no `argument` or `module_def` table
and this slice does not add one") is **superseded by this addendum** —
there is now a `module_def_argument` table, and D19 is narrowed rather
than reversed: a `Module`'s `NumericArg`/`TextArg` children are *read* on
the way past, and are still never descended into as activations.

### Evidence (measured 2026-09-14, `[V]` unless marked otherwise)

Corpus, counted over every `ApplicationProgram` member of every archive
under `OriginalData/`:

| Where | `Module` | `NumericArg` | `TextArg` | `AllocatorRef` | `Argument` decls |
|---|---|---|---|---|---|
| `ProductDatabases/` | 86 | 172 | 86 | **0** | 36 |
| `DemoProjects/` | 32 | 96 | 0 | **0** | 12 |

* Every one of the 118 `Module` elements carries at least one binding.
  Arguments are not a rare corner of the format; they are how modular
  products are written.
* The 36 `ProductDatabases` declarations are all MDT `M-0083`: 12×
  `Name="ParamOffsBase"` (no `@Type`, `Allocates="132"`), 12×
  `Name="ObjNumberBase"` (no `@Type`, `Allocates="20"`), 12×
  `Name="ChNo" Type="Text"`. The 12 `DemoProjects` ones are all KV25
  `M-00FA`: `argCH`/`argObj`/`argPar`, all numeric, all `@Type`-less.
  **No declaration anywhere spells `Numeric` explicitly** — numeric is the
  absent case.
* Attested consumption sites for an argument id, corpus-wide:
  `Memory/@BaseOffset` → a numeric argument, **705**;
  `ComObject/@BaseNumber` → a numeric argument, **157**; `{{Name}}` text
  placeholders, **978**, of which **978 resolve to an `Argument/@Name`
  declared by the enclosing `ModuleDef` and 0 do not** (resolved through
  `TranslationElement/@RefId` for the 775 that live in a translation unit;
  203 are direct).
* Separately, **948** purely numeric `{{<digits>}}` placeholders (e.g.
  `Text="Channel {{ChNo}}: {{0}}"`) match no argument name in any file.
  A different family, tied to `TextParameterRefId`, and not this
  mechanism's to resolve.
* Placeholders the `Dynamic` evaluator can actually *reach* — i.e. stored
  in `dynamic_node.text` inside a `ModuleDef`'s own tree: **24** (12×
  `Channel/@Text` `{{ChNo}}`, 4× `Channel/@Text` `{{argCH}}`, 4×
  `ParameterBlock/@Name`, 4× `ParameterBlock/@Text`).

Published schema, from the project's extraction of
`The KNX Standard v3.0.0 / Project Schema23 v01.00.00`:

* **[D]** §1.1.2.38 `simpleType ModuleDefArgType_t` — "Enum that can be
  used to define the argument in a module definition. Required for modular
  application programs." Facets: `Numeric`, `Text`, `AllocatorRef`.
* **[D]** §1.1.3.9 `simpleType Identifier50_t` — "This type is for
  specifying the name of `ModuleDef\Arguments\Argument`", pattern
  `[A-Za-z_][A-Za-z0-9_]*`. This is the documented proof that a `ModuleDef`
  argument is addressed *by name*, which is what makes `{{Name}}`
  resolution more than a guess.
* **[D]** `Value_t` — "TypeAllocatorRefId — A module allocator refId as
  string." One line, no semantics.
* **[A]** The extraction defines **no** AP-side `ModuleDef`/`Module`/
  `Arguments` complexType and **no** `{{…}}` substitution rule anywhere.
  The substitution semantics below therefore rest on the 978/978 corpus
  consistency above, not on a published rule, and are marked `[A]` in the
  code that implements them.

`AllocatorRef`, searched and not found — **the bases searched, named**:
`OriginalData/` in full (`.knxprod` and `.knxproj`, element and attribute
spellings, every readable archive member: 0 hits; the only 3 unreadable
members are the encrypted contents of the one `.vd2`, a format already out
of scope); `knx_spec_kb_programming.sqlite` (2,207 facts over 27
programming PDFs with figures); `knx_spec_kb_full179_clean.sqlite` (16,536
facts over 177 PDFs, text only) — both across `content`, `title`,
`keywords` and `evidenceText`, 0 hits; the only `Allocator` matches in
either base are "heat cost allocator" in DPT documents. It stays
unimplemented, and is reported rather than ignored.

### Decisions

**D47 — storage: one new table, one new column.** `ModuleDef/Arguments/
Argument` gets `module_def_argument (program_id, module_def_id, id, name,
arg_type, allocates, position, extra)`. It cannot be a `dynamic_node` row:
`Arguments` sits *outside* `Dynamic`, so it has no node id and belongs to
no tree. `NumericArg`/`TextArg`'s `@Value` gets `dynamic_node.value`.
The ADR-0020 "is it already stored?" check was asked and answered
honestly: the value *was* stored, in `extra` — which design D2's own
schema comment declares is a human-readable audit trail and explicitly
**not** re-parseable. Stored, yes; readable, no.

**D48 — resolution belongs to the scope.** `ProgramTrees` carries the
declarations (`with_arguments`, loaded by `load_program_trees`);
`ModuleScope` carries the resolved `Vec<BoundArgument>` — the `@Name`s and
the `@Value`s this instantiation bound them to. The task brief's
expectation that arguments "almost certainly belong there rather than in a
new mechanism" held up: nothing new was threaded through `walk`, because
`ModuleScope` was already on every activation and every diagnostic.
Lookup is deliberately **not** recursive up the `parent` chain: a name is
scoped to its own `ModuleDef` (`Identifier50_t`, `[D]`), and the corpus
has zero nested `Module`s to show otherwise.

**D49 — `{{Name}}` substitution, the one consumption site `evaluate` can
reach.** `Activation` gains `labels: Vec<ActiveLabel>` — one per activated
`Channel`/`ParameterBlock`/`ParameterSeparator` with a non-empty `@Text`,
carrying both `raw_text` (as stored) and `text` (substituted). This is
what makes two instantiations of one `ModuleDef` differ. The rejected
alternative was interpreting the *numeric* consumption sites instead
(`Memory/@BaseOffset`, `ComObject/@BaseNumber`, 862 occurrences between
them): both are `Static`-side constructs the pure `evaluate` cannot reach
without a much larger cross-cut — `com_object` has no `base_number` column
and memory placement is not modelled at all — so the interpretation with
the most corpus evidence behind it is also the one furthest out of this
slice's reach.

**D50 — nothing is discarded, and nothing is invented.** Three new
diagnostics, all scoped like every other:
`ModuleArgumentNotBound` (a binding with no resolvable declaration),
`UnsupportedModuleArgumentKind` (a `Module` child that is not
`NumericArg`/`TextArg`, or a declaration whose `@Type` is outside the two
facets read — `AllocatorRef` being the one such facet the schema names),
`UnresolvedTextPlaceholder` (a name-shaped placeholder with no binding).
An unresolved placeholder is left **verbatim**: substituting an empty
string would erase the only evidence that a value was meant to be there.
A purely numeric placeholder is left verbatim and *not* reported — it was
never an argument reference, and reporting it would be a false positive
948 occurrences wide.

**D51 — arguments do not multiply expansions, but labels multiply with
them.** An argument binding costs O(children of one `Module`) per
expansion and adds no fan-out of its own. A *label*, though, is produced
per activated element per expansion — exactly the way a ref is — so
`labels.len()` folds into `Activation::activations_recorded()` and counts
against `MAX_MODULE_ACTIVATIONS`. A budget that counted refs but not
labels would be a budget with a hole in it the width of a `ModuleDef`
whose tree is all `Channel`s. No new constant was needed.

**Deliberately not done:** the `Static` pass keeps reporting
`ModuleDef/Arguments`/`Argument` as unmodelled constructs. That is still
true of the facet it means — `@Allocates` and the memory placement it
feeds — and silencing the report would claim an interpretation this build
does not have.

### Migration

v9 → v10: create `module_def_argument`, `ALTER TABLE dynamic_node ADD
COLUMN value TEXT`, then replay every stored `ApplicationProgram` blob
through `dynamic::parse::parse_dynamic_trees` after **clearing** what the
previous parse of the same blob wrote — `parse_dynamic_trees` skips a
program that already has rows, so without the delete the backfill would be
an elaborate no-op on precisely the databases it exists for. The stale
`ingest_unknown` rows from the previous `Dynamic` pass are retired and
rewritten too, so a migrated database stops claiming `NumericArg/@Value`
is unmodelled the moment it acquires a column for it. Permitted by
ADR-0020 E1: every value re-derived is a pure function of bytes the
database already holds. `migrate_v2_to_v3`'s own backfill is the direct
precedent.

### Corpus measurement (task 12, required deliverable)

Re-measured on every run by `corpus_argument_measurement_task_12`
(`crates/knx-productdb/tests/dynamic_tree.rs`), over the installed
`OriginalData/ProductDatabases`:

* `AllocatorRef`: **0** occurrences across every readable archive member;
  3 members unreadable (encrypted, all inside the one `.vd2`), counted and
  named rather than dropped from the denominator.
* Stored rows for the one module-bearing package: 86 `Module`, 172
  `NumericArg`, 86 `TextArg`, 36 declarations (12 `Text`, 24 `@Type`-less,
  0 `AllocatorRef`), names `{ChNo, ObjNumberBase, ParamOffsBase}`.
* Evaluated under the corpus's own default values: **12 / 8 / 4**
  substituted labels for the three programs, every one of them distinct
  from the others — where before task 12 all twelve read
  `Channel {{ChNo}}: {{0}}`. Zero `UnresolvedTextPlaceholder`, zero
  `ModuleArgumentNotBound`, zero `UnsupportedModuleArgumentKind` on real
  data.

### Acceptance criteria addendum

18. A test in which an **argument value** changes the evaluated result:
    two `Module` elements naming one `ModuleDef`, identical except for
    their binding `@Value`s, evaluate to two different labels
    (`an_argument_value_changes_the_evaluated_label_of_each_module_instantiation`).
    Its mirror, `without_the_argument_declarations_the_same_trees_leave_every_placeholder_standing`,
    evaluates the same trees with no declarations attached and gets the
    pre-task-12 result — identical labels, every binding reported as
    `ModuleArgumentNotBound` — so the claim is about the argument, not
    about the tree.
19. A **measured** statement of `AllocatorRef`'s presence in the installed
    corpus, asserted rather than remembered
    (`corpus_argument_measurement_task_12`), together with the bases
    searched, named above.
20. An unresolved name-shaped placeholder is left verbatim and reported; a
    numeric placeholder is left verbatim and not reported; a declaration
    typed `AllocatorRef` is reported once per instantiation and never
    interpreted. All three asserted by the acceptance test.
21. The v9 → v10 migration recreates `dynamic_node` and
    `module_def_argument` from stored blobs with no re-install, and the
    pre-existing migration tests still pass after being taught that v10
    has structure to rewind.
22. All six gates pass, judged by exit status:
    `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets
    -j 2 -- -D warnings`, `cargo test --workspace --no-fail-fast -j 2`,
    `cargo run -p xtask -- check-layering`, `cargo run -p xtask --
    check-headers`, `cargo deny check`.
