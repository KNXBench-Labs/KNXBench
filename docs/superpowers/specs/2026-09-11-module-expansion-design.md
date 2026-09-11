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
