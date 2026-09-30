//! Evaluates a stored `Dynamic` tree: given a set of parameter values,
//! which `ParameterRefRef`s and `ComObjectRefRef`s are active, and what
//! could not be decided.
//!
//! This is design D6-D11
//! (`docs/superpowers/specs/2026-09-11-dynamic-tree-parse-and-evaluate-design.md`)
//! plus D12-D19
//! (`docs/superpowers/specs/2026-09-11-module-expansion-design.md`), which
//! adds `Module` expansion. `evaluate` itself is a pure function of
//! `(&ProgramTrees, &ValueMap)` — no database, no I/O, no logging.
//! `load_tree`/`load_program_trees` and `resolve_values` are the
//! DB-touching helpers that assemble its inputs; they are kept
//! deliberately separate so `evaluate`'s own logic can be unit-tested
//! against hand-built trees with no database at all (plan Task 2 step 1).
//!
//! D15's "exactly one level of expansion" policy is superseded by the
//! nested-expansion addendum in the same design document (goal-completion
//! task 11, 2026-09-14): a `Module` found while already inside an
//! expanded `ModuleDef`'s tree is now itself expanded, up to
//! [`MAX_MODULE_NESTING_DEPTH`], with a self-referential chain refused as
//! [`Diagnostic::ModuleCycleDetected`] rather than recursed into. The
//! corpus measured for that addendum still has zero nested `Module`
//! elements (see the design doc and `docs/RESEARCH.md` §4.4), so this
//! path is exercised only by hand-built unit tests today, not by
//! anything in `OriginalData/`.
//!
//! Nothing here has been validated against ETS. The no-match policy (D8)
//! and the `@default`-is-fallback reading (below) are inferences from
//! corpus consistency, not documented rules — see RESEARCH.md §4.3's
//! `[D]`/`[V]`/`[A]` markers, preserved here in the same spirit.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use rusqlite::{params, Connection, OptionalExtension};

use crate::ProductDbError;

/// Upper bound on `Module` nesting depth `evaluate` will expand. A
/// program's own top-level `Module` is depth 1; a `Module` found inside
/// that expansion's `ModuleDef` tree is depth 2; and so on. Reaching a
/// `Module` that would sit at depth `MAX_MODULE_NESTING_DEPTH + 1`
/// produces [`Diagnostic::ModuleNestingTooDeep`] instead of a further
/// recursive `walk` call — this is what keeps the recursion in `walk`
/// bounded even if the cycle check below ever has a bug, and it is the
/// only thing standing between a pathological (or malicious) manufacturer
/// file and a *stack* overflow. It does **not**, by itself, bound total
/// work: fan-out multiplies per level, and nothing here dedups distinct
/// nesting chains, so a `ModuleDef` tree with `k` sibling `Module`s each
/// pointing at the next can reach on the order of `k^depth` expansions
/// without ever repeating a chain or tripping this check. See
/// [`MAX_MODULE_EXPANSIONS`] for the budget that bounds *that* case —
/// this constant alone is not "the only thing standing between a
/// pathological file and" trouble; it is the only thing standing between
/// one and a stack overflow specifically.
///
/// **[A]** No source states a numeric bound for AP-side `Module` nesting.
/// The KNX Standard extraction available to this project defines no
/// application-program-side `ModuleDef`/`Module` complexType at all
/// (`docs/RESEARCH.md` §4.4 Q2, re-confirmed by a fresh `pdftotext
/// -layout` extraction of `Project Schema23 v01.00.00.pdf` for this task:
/// grepping every `complexType`/`element`/`simpleType` heading containing
/// "module" finds only `ModuleDefArgType_t` (§1.1.2.38) and the
/// project-instance-side `ModuleInstance_t` family (§1.2.5.16-20) — no
/// AP-side `ModuleDef` complexType). The Standard is not silent about
/// module nesting everywhere, though: `ModuleInstance_t/@Id`'s documented
/// grammar (§1.2.5.18, project-instance side, **[D]**, already recorded
/// at `docs/RESEARCH.md` §4.4 Q6) gives exactly one extra level —
/// `MD-<n>_M-<m>_MI-<k>` for a plain instantiation,
/// `MD-<n>_M-<m>_MI-<k>_SM-<n>_M-<m>_MI-<k>` for a `SubModule` one, never
/// a second `SM-` segment. That is project-side, not AP-side, so it does
/// not settle this constant's value on its own — but it is the one
/// documented data point that speaks to nesting depth at all, and it
/// says 2, not 16. `16` is this project's own choice, deliberately far
/// above that one documented neighbour: deep enough that no legitimate
/// hand-authored product is expected to reach it, shallow enough that
/// hitting it is always worth a diagnostic rather than more silent
/// recursion. The installed corpus under `OriginalData/ProductDatabases/`
/// has zero nested `Module` elements, measured for this task (see the
/// corpus test below and the design doc addendum's D44/D45). Revisit if
/// a genuine corpus sample ever needs more.
pub const MAX_MODULE_NESTING_DEPTH: usize = 16;

/// Upper bound on the total number of `Module` expansions `evaluate` will
/// perform in one call, counted across the *whole* walk — every nesting
/// chain and every branch, not just the deepest one.
/// [`MAX_MODULE_NESTING_DEPTH`] bounds how deep any single chain goes and
/// is what stops a stack overflow; it does not bound how many chains
/// there are. Fan-out multiplies per level with nothing to dedup it —
/// every distinct nesting chain is its own `ScopeKey` by construction, so
/// `seen_params`/`seen_coms` cannot collapse them the way they collapse
/// repeats within one scope. A `ModuleDef` tree with `k` sibling
/// `Module`s each pointing at the next, walked `depth` levels deep,
/// produces on the order of `k^depth` expansions — no cycle, never as
/// deep as `MAX_MODULE_NESTING_DEPTH`'s own ceiling of 16 needs to be to
/// still be catastrophic: a measured probe (`depth=12, fanout=4`, a
/// 44-node input, goal-completion task 11 fix round 1) reached 4,194,304
/// `Module` activations, 11.5s wall time and a peak RSS of 8,170 MiB.
/// Once expanding a `Module` would be the `MAX_MODULE_EXPANSIONS + 1`th
/// expansion this call has performed, `walk` refuses it with
/// [`Diagnostic::ModuleExpansionBudgetExhausted`] instead — the same
/// contract as [`MAX_MODULE_NESTING_DEPTH`]: refuse loudly, never crash,
/// never truncate the rest of the walk silently.
///
/// **Correction (goal-completion task 11, fix round 2):** round 1's own
/// doc comment here claimed this constant alone stood between a
/// pathological file and the 8,170 MiB blowup above. That overstated it.
/// This constant bounds how many `Module` *expansions* happen; it does
/// nothing to bound how much a single expansion costs. Before round 2,
/// every activated `ParameterRefRef`/`ComObjectRefRef` deep-cloned its
/// entire `ModuleScope` ancestor chain into `ActiveRef::scope`, so a
/// `ModuleDef` tree with many ref children, expanded even a modest
/// number of times within this budget, still reached multi-GB peak RSS —
/// measured, standalone binary over the public API, `opt-level = 2`: a
/// 488-node file (fanout 4, 9 module levels, 50 refs/level) hit
/// 1,325,196 activations and 1,946 MiB; a 1,388-node file (150
/// refs/level) hit 3,825,596 activations and 5,691 MiB, well inside this
/// budget's 100,000-expansion ceiling the whole time. Round 2 closes
/// that gap two ways, together: `ActiveRef`/`ScopedDiagnostic` now store
/// `Rc<ModuleScope>` (a pointer-sized refcount bump, `size_of::<ActiveRef>()
/// == 32` bytes, measured) instead of a deep clone, and
/// [`MAX_MODULE_ACTIVATIONS`] separately bounds the total ref count so a
/// single wide expansion can no longer multiply unboundedly against this
/// budget's expansion count. What this constant delivers on its own is
/// exactly what its name says: a ceiling on expansion *count*. What
/// bounds total memory is both constants together — see
/// [`MAX_MODULE_ACTIVATIONS`] for that half.
///
/// **[A]** This project's own choice, not derived from any source.
/// Picked against the measured legitimate ceiling: the largest single
/// program in the installed corpus tops out at 382 total activations
/// (`prod3`'s `M-0083_A-0317-31-7DC6.xml`, `docs/IMPLEMENTATION_STATUS.md`
/// §"Corpus regression coverage"), from only 44 stored `Module` rows —
/// real files sit nowhere near this budget. `100_000` is roughly 260x
/// that measured ceiling, enormous headroom before it can bite a
/// legitimate file, while still stopping the fan-out probe above early:
/// a running total crosses 100,000 partway through the ninth level of a
/// fanout-4 tree (`(4^9 - 4) / 3 = 87,380` at the end of level eight,
/// `(4^10 - 4) / 3 = 349,525` at the end of level nine), nowhere near the
/// depth-12 level where the measured 8,170 MiB case occurs. Revisit if a
/// genuine corpus sample ever needs more.
pub const MAX_MODULE_EXPANSIONS: usize = 100_000;

/// Upper bound on the total number of activated
/// `ParameterRefRef`/`ComObjectRefRef` occurrences (`Activation::parameter_refs.len() +
/// Activation::com_object_refs.len()`, combined) `evaluate` will record
/// in one call. The complementary budget to [`MAX_MODULE_EXPANSIONS`]
/// (goal-completion task 11, fix round 2, blocking finding 1's residual):
/// that constant bounds how many times a `Module` gets expanded; this one
/// bounds how much each expansion is allowed to cost, in the currency
/// that actually drove the round-1 blowup — total activated refs, each
/// one an `ActiveRef` pushed onto a `Vec` for the lifetime of the call. A
/// `ModuleDef` tree with many sibling `ParameterRefRef`/`ComObjectRefRef`
/// children, expanded repeatedly (but still within the expansion budget),
/// multiplies refs-per-expansion by expansion-count with nothing else
/// bounding the product — this is exactly the shape of the round-1
/// residual (see [`MAX_MODULE_EXPANSIONS`]'s doc comment for the measured
/// numbers). Checked inside `Activation::activate_parameter_ref`/
/// `activate_com_object_ref` — ahead of the per-scope dedup, so that once
/// the budget is spent nothing further is allocated for a ref that cannot
/// be recorded anyway — and again in `walk`'s `Module` arm, where a spent
/// budget stops further expansion outright (fix round 3; round 2 checked
/// only at the activation sites and only after the dedup, which left the
/// `seen` sets growing unbounded while the walk ran on to the expansion
/// budget). A ref already seen in its scope still never counts against
/// the budget; once the combined count would be the
/// `MAX_MODULE_ACTIVATIONS + 1`th,
/// a single [`Diagnostic::ModuleExpansionBudgetExhausted`] is recorded
/// (the same variant `MAX_MODULE_EXPANSIONS` uses, distinguished only by
/// which constant appears in its `budget` field, per fix round 2's
/// instruction to reuse it rather than add a new variant) and every
/// further ref that would have crossed it is dropped without a repeat
/// diagnostic — unlike the expansion budget, activation sites are leaf
/// nodes with no subtree to refuse descending into, so without this
/// single-diagnostic rule the diagnostics `Vec` itself would grow
/// unboundedly in exactly the scenario this budget exists to prevent.
///
/// **[A]** This project's own choice, not derived from any source. Same
/// measured legitimate ceiling as [`MAX_MODULE_EXPANSIONS`] (382 total
/// activations, `docs/IMPLEMENTATION_STATUS.md`), but a wider multiplier
/// against it: a single legitimate expansion can already contribute all
/// 382 by itself, so this budget has to clear the expansion budget's own
/// worst case, not just one file's totals. `1_000_000` is roughly 2,600x
/// the measured ceiling. Peak memory at that cap is bounded and small
/// now that `ActiveRef` stores an `Rc` instead of a deep clone:
/// `size_of::<ActiveRef>() == 32` bytes (measured), plus a `ScopeKey`
/// entry in the relevant `seen` set (`(Vec<i64>, String)`, stack size 48
/// bytes, heap-bounded by [`MAX_MODULE_NESTING_DEPTH`] node ids plus one
/// short ref-id string) — measured directly (the test that trips this
/// budget, `a_wide_module_def_trips_the_activation_budget_without_tripping_the_expansion_budget`
/// in `dynamic_tree.rs`, `fanout=4, module_levels=6, refs_per_level=800`,
/// crossing this budget at 1,092,000 attempted activations from just
/// 5,461 `Module` expansions): 314,712 KiB (307 MiB) peak `VmHWM`, 1.96s
/// wall time. Nowhere near the multi-GB regime `MAX_MODULE_EXPANSIONS`
/// alone left open. Revisit if a genuine corpus sample ever needs more.
pub const MAX_MODULE_ACTIVATIONS: usize = 1_000_000;

/// [D] `Condition_t`'s three alternatives (`Project Schema23 v01.00.00.md`
/// §1.1.3.18): a single number, a space-separated list of numbers, or a
/// comparison. All six operators are implemented even though the
/// researched corpus (RESEARCH.md §4.3) only ever exercises `>` (13 of
/// 22630 `when` elements) — this is the normative grammar, not a
/// corpus-shaped subset of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Test {
    Single(i64),
    List(Vec<i64>),
    Compare(Op, i64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Op {
    Eq,
    Ne,
    Gt,
    Lt,
    Ge,
    Le,
}

/// `Test::parse` failed: the raw string is not a legal `Condition_t`
/// literal. Carries no detail of its own — the caller already has the raw
/// string that failed, and `Diagnostic::UnparsableTest` is where it is
/// reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnparsableTest;

impl Test {
    /// Parses one `@test` value per `Condition_t`. `quick-xml` has already
    /// decoded `&lt;`/`&gt;` to `<`/`>` by the time anything in this crate
    /// sees the attribute value (Task 1 stores `test` verbatim from that
    /// already-decoded string), so only the plain operator glyphs are
    /// matched here. Longer operators (`!=`, `>=`, `<=`) are tried before
    /// their single-character prefixes (`=`, `>`, `<`) so `>=` is never
    /// misread as `>` followed by a stray `=`.
    pub fn parse(raw: &str) -> Result<Test, UnparsableTest> {
        let raw = raw.trim();
        if raw.is_empty() {
            return Err(UnparsableTest);
        }
        for (glyph, op) in [
            ("!=", Op::Ne),
            (">=", Op::Ge),
            ("<=", Op::Le),
            ("=", Op::Eq),
            (">", Op::Gt),
            ("<", Op::Lt),
        ] {
            if let Some(rest) = raw.strip_prefix(glyph) {
                let n: i64 = rest.trim().parse().map_err(|_| UnparsableTest)?;
                return Ok(Test::Compare(op, n));
            }
        }
        let parts: Vec<&str> = raw.split_whitespace().collect();
        if parts.len() == 1 {
            let n: i64 = parts[0].parse().map_err(|_| UnparsableTest)?;
            return Ok(Test::Single(n));
        }
        let numbers: Result<Vec<i64>, _> = parts.iter().map(|p| p.parse::<i64>()).collect();
        numbers.map(Test::List).map_err(|_| UnparsableTest)
    }

    fn matches(&self, value: i64) -> bool {
        match self {
            Test::Single(n) => value == *n,
            Test::List(ns) => ns.contains(&value),
            Test::Compare(op, n) => match op {
                Op::Eq => value == *n,
                Op::Ne => value != *n,
                Op::Gt => value > *n,
                Op::Lt => value < *n,
                Op::Ge => value >= *n,
                Op::Le => value <= *n,
            },
        }
    }
}

/// Whether a `choose`'s controlling parameter's `ParameterType` is
/// `TypeNone` (design D9's own code path — no comparison is ever
/// attempted) or anything else this build treats as comparable
/// (`TypeNumber`/`TypeRestriction` per the Standard, and — because this
/// build does not reject a type the Standard does not expect here either —
/// any other kind `parameter_type.kind` may hold). `None` on the owning
/// `DynamicNode` (not this enum) means the controlling reference could not
/// be resolved at all; see `Diagnostic::UnresolvedParamRef`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlKind {
    TypeNone,
    Comparable,
}

/// One `dynamic_node` row, loaded for evaluation. `extra` plays no role in
/// evaluation and is left out on purpose, not by oversight — it is a
/// human-readable audit trail, documented as not re-parseable.
/// `element_id` (`Module/@Id`) is the one field slice 1 deliberately left
/// unloaded because nothing needed it yet; slice 2 needs it to fill
/// `ModuleScope::module_id` (design D14) for human-readable diagnostic and
/// activation reporting — it is never relied on for identity, which is
/// `module_node` (this row's own `node_id`). `text` and `value` joined it
/// in goal-completion task 12 (design D49/D47): `text` is what a `{{Name}}`
/// placeholder is substituted into, `value` is what a `NumericArg`/
/// `TextArg` binding binds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicNode {
    pub node_id: i64,
    pub parent_id: Option<i64>,
    pub kind: String,
    pub element_id: Option<String>,
    pub ref_id: Option<String>,
    pub test: Option<String>,
    pub is_default: bool,
    /// `Channel`/`ParameterBlock`/`ParameterSeparator`'s `@Text`, verbatim
    /// — placeholders included. Substitution happens in `evaluate`, not
    /// here: the stored row must keep saying what the file said.
    pub text: Option<String>,
    /// `NumericArg`/`TextArg`'s `@Value`, verbatim.
    pub value: Option<String>,
    /// `Channel`'s `@Name`, verbatim (ADR-0052). `None` for every other
    /// kind and for a channel without the attribute.
    pub name: Option<String>,
    /// `Channel`'s `@Number`, verbatim: text, since not every corpus value
    /// is a number (ADR-0052).
    pub number: Option<String>,
    /// Only meaningful when `kind == "choose"`. `load_tree` resolves this
    /// from `parameter_ref`/`parameter`/`parameter_type`; hand-built trees
    /// (unit tests) set it directly.
    pub control_kind: Option<ControlKind>,
}

/// A `Dynamic` tree (one `(program_id, module_def_id)` scope, design D2),
/// indexed for evaluation. Built either by `load_tree` from the database or
/// directly from a hand-built `Vec<DynamicNode>` (`from_nodes`) for unit
/// tests that need no database at all.
#[derive(Debug, Clone, Default)]
pub struct DynamicTree {
    nodes: HashMap<i64, DynamicNode>,
    // Keyed by `parent_id` (`None` for the tree's own roots — normally
    // exactly the one root `<Dynamic>` element). Children are kept in the
    // order they were supplied, which callers must supply in document
    // order (ascending `node_id`) for that to mean anything; `load_tree`'s
    // `ORDER BY node_id` query guarantees this for database-backed trees.
    children: HashMap<Option<i64>, Vec<i64>>,
}

impl DynamicTree {
    pub fn from_nodes(nodes: Vec<DynamicNode>) -> DynamicTree {
        let mut children: HashMap<Option<i64>, Vec<i64>> = HashMap::new();
        for n in &nodes {
            children.entry(n.parent_id).or_default().push(n.node_id);
        }
        let nodes = nodes.into_iter().map(|n| (n.node_id, n)).collect();
        DynamicTree { nodes, children }
    }

    fn node(&self, id: i64) -> Option<&DynamicNode> {
        self.nodes.get(&id)
    }

    fn children_of(&self, parent: Option<i64>) -> &[i64] {
        self.children.get(&parent).map(Vec::as_slice).unwrap_or(&[])
    }

    fn roots(&self) -> &[i64] {
        self.children_of(None)
    }
}

/// Loads one `Dynamic` scope (`module_def_id = ""` for an
/// `ApplicationProgram`'s own tree, or a `ModuleDef`'s `@Id`) from
/// `dynamic_node`, resolving each `choose`'s controlling `ControlKind`
/// against `parameter_ref`/`parameter`/`parameter_type` as it goes (design
/// D9 needs to know this before it can pick a code path, and `evaluate`
/// itself must not touch the database to find out).
pub fn load_tree(
    conn: &Connection,
    program_id: &str,
    module_def_id: &str,
) -> Result<DynamicTree, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT node_id, parent_id, kind, element_id, ref_id, test, is_default,
                text, value, name, number
         FROM dynamic_node
         WHERE program_id = ?1 AND module_def_id = ?2
         ORDER BY node_id",
    )?;
    // `control_kind` is resolved below, per `choose` row, once the
    // statement is done.
    let rows: Vec<DynamicNode> = stmt
        .query_map(params![program_id, module_def_id], |r| {
            Ok(DynamicNode {
                node_id: r.get(0)?,
                parent_id: r.get(1)?,
                kind: r.get(2)?,
                element_id: r.get(3)?,
                ref_id: r.get(4)?,
                test: r.get(5)?,
                is_default: r.get::<_, Option<i64>>(6)? == Some(1),
                text: r.get(7)?,
                value: r.get(8)?,
                name: r.get(9)?,
                number: r.get(10)?,
                control_kind: None,
            })
        })?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    let mut nodes = Vec::with_capacity(rows.len());
    for mut node in rows {
        if node.kind == "choose" {
            if let Some(rid) = node.ref_id.as_deref() {
                node.control_kind = resolve_control_kind(conn, program_id, rid)?;
            }
        }
        nodes.push(node);
    }
    Ok(DynamicTree::from_nodes(nodes))
}

/// The full set of `Dynamic` trees one program's evaluation can reach:
/// the program's own tree (`module_def_id = ""`), plus every `ModuleDef`
/// tree stored under this `program_id`, keyed by `ModuleDef/@Id` (design
/// D13). Nothing is copied or pre-expanded (design D12) — `evaluate`
/// follows a `Module` node into the matching entry here at walk time.
#[derive(Debug, Clone, Default)]
pub struct ProgramTrees {
    program: DynamicTree,
    modules: HashMap<String, DynamicTree>,
    /// Every `ModuleDef/Arguments/Argument` declared anywhere in this
    /// program, keyed by the declaration's own `@Id` — the id a
    /// `Module`'s `NumericArg`/`TextArg` binding names in its `@RefId`
    /// (design D48). Empty for a `ProgramTrees` built by `single` or by
    /// `from_parts` without `with_arguments`, which is what every
    /// pre-task-12 hand-built test tree is: no declarations, so no
    /// substitution, so those trees evaluate exactly as they did before.
    arguments: HashMap<String, ModuleDefArgument>,
}

/// One `ModuleDef/Arguments/Argument` declaration (design D47). The
/// `ModuleDef`-scoped *name* is the interesting field: it is the token a
/// `{{Name}}` placeholder in that `ModuleDef`'s own `Dynamic` tree spells,
/// and the published schema gives it a type of its own — `Identifier50_t`,
/// "This type is for specifying the name of `ModuleDef\Arguments\Argument`"
/// (`Project Schema23 v01.00.00` §1.1.3.9, **[D]**).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDefArgument {
    /// `Argument/@Id`.
    pub id: String,
    /// The `ModuleDef/@Id` this declaration belongs to.
    pub module_def_id: String,
    /// `Argument/@Name`. `None` when the file omitted it, which no file in
    /// the installed corpus does (**[V]**) and which leaves the argument
    /// permanently unresolvable by name.
    pub name: Option<String>,
    /// `Argument/@Type`, verbatim. `None` means the attribute was absent,
    /// which is how every numeric declaration in the installed corpus is
    /// written (**[V]**). See [`ArgumentKind`] for how it is read.
    pub arg_type: Option<String>,
    /// `Argument/@Allocates`, the memory-allocation facet. Stored, loaded,
    /// and deliberately not interpreted: the constructs that would consume
    /// it (`Memory/@BaseOffset`, `ComObject/@BaseNumber`) live in the
    /// `Static` half of the file, which `evaluate` cannot reach and this
    /// crate does not model.
    pub allocates: Option<i64>,
}

/// How an argument declaration's `@Type` is read (design D50). The
/// published `ModuleDefArgType_t` enumeration has exactly three facets —
/// `Numeric`, `Text`, `AllocatorRef` (`Project Schema23 v01.00.00`
/// §1.1.2.38, **[D]**) — and this build interprets two of them.
///
/// `AllocatorRef` is **unattested**: zero occurrences in the whole
/// installed corpus and zero hits in either KNX specification knowledge
/// base (**[V]**, goal-completion task 12 — see the design document's task
/// 12 addendum for what was searched). No source anywhere states what it
/// means, so nothing here guesses; a declaration that spells it is
/// reported as [`Diagnostic::UnsupportedModuleArgumentKind`] and its
/// bindings are not substituted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArgumentKind {
    /// `@Type` absent, or spelled `Numeric`.
    Numeric,
    /// `@Type="Text"`.
    Text,
    /// Anything else, `AllocatorRef` included.
    Unsupported,
}

impl ModuleDefArgument {
    pub fn kind(&self) -> ArgumentKind {
        match self.arg_type.as_deref() {
            None | Some("Numeric") => ArgumentKind::Numeric,
            Some("Text") => ArgumentKind::Text,
            Some(_) => ArgumentKind::Unsupported,
        }
    }
}

impl ProgramTrees {
    /// Whether the program's own `Dynamic` tree has any node at all
    /// (ISSUE-08). An empty tree activates nothing, which says nothing
    /// about the program's objects: there was nothing to evaluate.
    pub fn has_program_tree(&self) -> bool {
        !self.program.nodes.is_empty()
    }

    /// Builds a `ProgramTrees` from an already-loaded program tree and its
    /// already-loaded `ModuleDef` trees. The database-touching counterpart
    /// is `load_program_trees`.
    pub fn from_parts(program: DynamicTree, modules: HashMap<String, DynamicTree>) -> ProgramTrees {
        ProgramTrees {
            program,
            modules,
            arguments: HashMap::new(),
        }
    }

    /// Attaches the program's `ModuleDef` argument declarations. A separate
    /// step rather than a third `from_parts` parameter so that every
    /// pre-task-12 caller keeps compiling and keeps meaning what it meant:
    /// a tree set with no declarations substitutes nothing.
    pub fn with_arguments(
        mut self,
        arguments: impl IntoIterator<Item = ModuleDefArgument>,
    ) -> ProgramTrees {
        self.arguments = arguments.into_iter().map(|a| (a.id.clone(), a)).collect();
        self
    }

    fn argument(&self, id: &str) -> Option<&ModuleDefArgument> {
        self.arguments.get(id)
    }

    /// A program with no `ModuleDef` trees at all — every existing
    /// hand-built-tree unit test from before this slice becomes
    /// `evaluate(&ProgramTrees::single(tree), &values)`, a mechanical
    /// change (plan Task 1 step 4).
    pub fn single(program: DynamicTree) -> ProgramTrees {
        ProgramTrees {
            program,
            modules: HashMap::new(),
            arguments: HashMap::new(),
        }
    }

    fn module(&self, module_def_id: &str) -> Option<&DynamicTree> {
        self.modules.get(module_def_id)
    }
}

/// Loads every `Dynamic` scope this program's evaluation can reach: its
/// own tree, plus one tree per distinct `module_def_id` already stored for
/// it (design D13). `load_tree` itself needs no change beyond step 1 —
/// it is already scoped by `(program_id, module_def_id)`, which is exactly
/// what a `ModuleDef`'s own tree is keyed by (slice 1's D2).
pub fn load_program_trees(
    conn: &Connection,
    program_id: &str,
) -> Result<ProgramTrees, ProductDbError> {
    let program = load_tree(conn, program_id, "")?;

    let mut stmt =
        conn.prepare("SELECT DISTINCT module_def_id FROM dynamic_node WHERE program_id = ?1")?;
    let module_def_ids: Vec<String> = stmt
        .query_map([program_id], |r| r.get(0))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    let mut modules = HashMap::with_capacity(module_def_ids.len());
    for module_def_id in module_def_ids {
        // `""` is the program's own scope, already loaded above as
        // `program` — not a `ModuleDef` id, so it is not re-inserted here.
        if module_def_id.is_empty() {
            continue;
        }
        let tree = load_tree(conn, program_id, &module_def_id)?;
        modules.insert(module_def_id, tree);
    }
    Ok(ProgramTrees::from_parts(program, modules)
        .with_arguments(load_module_def_arguments(conn, program_id)?))
}

/// Loads every `ModuleDef/Arguments/Argument` declaration stored for one
/// program (design D48). Declarations live in `module_def_argument`, not in
/// any `Dynamic` tree — `Arguments` sits outside `Dynamic` — so this is a
/// query of its own rather than another `load_tree` scope.
pub fn load_module_def_arguments(
    conn: &Connection,
    program_id: &str,
) -> Result<Vec<ModuleDefArgument>, ProductDbError> {
    let mut stmt = conn.prepare(
        "SELECT id, module_def_id, name, arg_type, allocates
         FROM module_def_argument
         WHERE program_id = ?1
         ORDER BY module_def_id, position",
    )?;
    let rows = stmt
        .query_map([program_id], |r| {
            Ok(ModuleDefArgument {
                id: r.get(0)?,
                module_def_id: r.get(1)?,
                name: r.get(2)?,
                arg_type: r.get(3)?,
                allocates: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// `choose/@ParamRefId` -> `parameter_ref.parameter_id` ->
/// `parameter.parameter_type_id` -> `parameter_type.kind`. `None` when any
/// link in that chain is missing — a dangling reference, which the corpus
/// (RESEARCH.md §4.3) never shows but real manufacturer data is not
/// guaranteed to avoid.
fn resolve_control_kind(
    conn: &Connection,
    program_id: &str,
    param_ref_id: &str,
) -> Result<Option<ControlKind>, ProductDbError> {
    let kind: Option<String> = conn
        .query_row(
            "SELECT pt.kind
             FROM parameter_ref pr
             JOIN parameter p ON p.program_id = pr.program_id AND p.id = pr.parameter_id
             JOIN parameter_type pt ON pt.program_id = p.program_id AND pt.id = p.parameter_type_id
             WHERE pr.program_id = ?1 AND pr.id = ?2",
            params![program_id, param_ref_id],
            |r| r.get(0),
        )
        .optional()?;
    Ok(kind.map(|k| {
        if k == "None" {
            ControlKind::TypeNone
        } else {
            ControlKind::Comparable
        }
    }))
}

/// Parameter values available to a `Dynamic` tree evaluation, in two
/// scopes (design D35). `unscoped` is keyed by declared `ParameterRef`
/// id — the same id `choose/@ParamRefId` and `ParameterRefRef/@RefId`
/// use, and the same one a project's program-level `ParameterInstance`
/// carries (`DATA_MODEL.md` §10). `scoped` is keyed by `(module_id,
/// declared_ref_id)`, where `module_id` is the program-side `Module/@Id`
/// (`ModuleScope::module_id`) — one `Module` instantiation's own stored
/// values, held apart from every other instantiation of the same
/// `ModuleDef` and from the program default (design D36). Not an opaque
/// type: `From<HashMap<String, String>>` lets a unit test build the
/// unscoped half with a literal, without going through `resolve_values`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ValueMap {
    unscoped: HashMap<String, String>,
    scoped: HashMap<(String, String), String>,
}

impl ValueMap {
    /// The value for `ref_id` within `scope`: the scope's own stored value
    /// if it has one, otherwise the program-level value (design D36).
    /// `scope: None` reads the program-level value only. Never falls back
    /// to another instantiation's scoped value, and a scope with no
    /// `module_id` (design D37) can never have a scoped value to find.
    pub fn get(&self, scope: Option<&ModuleScope>, ref_id: &str) -> Option<&str> {
        if let Some(module_id) = scope.and_then(|s| s.module_id.as_deref()) {
            if let Some(v) = self
                .scoped
                .get(&(module_id.to_string(), ref_id.to_string()))
            {
                return Some(v.as_str());
            }
        }
        self.unscoped.get(ref_id).map(String::as_str)
    }

    /// The program-level value for `ref_id`, ignoring scope entirely —
    /// the display path's `ProgramDefault` fallback.
    pub fn get_unscoped(&self, ref_id: &str) -> Option<&str> {
        self.unscoped.get(ref_id).map(String::as_str)
    }

    /// Stores one instantiation's own value, keyed by `(module_id,
    /// ref_id)` — never `module_node` (design D35: that `i64` is a
    /// product-database row number, meaningless to the project side and
    /// unstable across a package reinstall).
    pub fn insert_scoped(&mut self, module_id: String, ref_id: String, value: String) {
        self.scoped.insert((module_id, ref_id), value);
    }

    /// Number of stored scoped values, across every `module_id`.
    pub fn len_scoped(&self) -> usize {
        self.scoped.len()
    }
}

impl From<HashMap<String, String>> for ValueMap {
    /// Wraps a flat map as the unscoped half only — the shape every
    /// pre-T18-slice-4 caller already had.
    fn from(unscoped: HashMap<String, String>) -> Self {
        ValueMap {
            unscoped,
            scoped: HashMap::new(),
        }
    }
}

/// Assembles a `ValueMap` per design D6's resolution order: `supplied`
/// wins where present; everything else falls back to `parameter_ref.value`,
/// then `parameter.value`. A ref with none of the three is simply absent
/// from the result — `evaluate` turns that absence into `Diagnostic::MissingValue`
/// only for the refs it actually needs during a given walk, rather than
/// pre-flagging every parameter a tree happens to declare, most of which a
/// given evaluation may never reach. Returns unscoped values only — no
/// per-`Module`-instantiation value enters here (that is Task 3's write
/// path); `evaluate` sees an empty `scoped` half for every caller of this
/// function today.
pub fn resolve_values(
    conn: &Connection,
    program_id: &str,
    supplied: &HashMap<String, String>,
) -> Result<ValueMap, ProductDbError> {
    let mut values = supplied.clone();
    let mut stmt = conn.prepare(
        "SELECT pr.id, pr.value, p.value
         FROM parameter_ref pr
         JOIN parameter p ON p.program_id = pr.program_id AND p.id = pr.parameter_id
         WHERE pr.program_id = ?1",
    )?;
    let rows: Vec<(String, Option<String>, Option<String>)> = stmt
        .query_map([program_id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
        .collect::<Result<_, _>>()?;
    for (id, ref_value, param_value) in rows {
        if let std::collections::hash_map::Entry::Vacant(e) = values.entry(id) {
            if let Some(v) = ref_value.or(param_value) {
                e.insert(v);
            }
        }
    }
    Ok(values.into())
}

/// Everything `evaluate` could not decide. Returned, never logged, never
/// swallowed (design D6). `choose_node`/`when_node`/`node_id` are the
/// `dynamic_node.node_id` of the element in question, within the tree
/// `evaluate` was called with — meaningful only alongside that tree's
/// `(program_id, module_def_id)`, which this type does not itself carry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diagnostic {
    /// [A] Inferred from RESEARCH.md §4.3: 5570 of 8732 default-less
    /// `choose` elements in the corpus have a legal value no `when` covers.
    /// The conservative reading — activate nothing rather than guess a
    /// branch — is this project's data-integrity rule, not a documented
    /// KNX rule. Also covers the case where the controlling value could be
    /// resolved and parsed but simply matched nothing.
    NoBranchMatched {
        choose_node: i64,
        param_ref: Option<String>,
        observed_value: String,
    },
    /// A `when/@test` did not parse as `Condition_t` (D7). That `when` is
    /// treated as never matching — not as always matching, and not as an
    /// error that aborts evaluating its siblings.
    UnparsableTest { when_node: i64, raw: String },
    /// A `choose/@ParamRefId` does not resolve to a `parameter_ref` backed
    /// by a `parameter`/`parameter_type` this build can read. Nothing under
    /// the `choose` activates; unlike `MissingValue`, this is not about the
    /// *value* being absent, but about not knowing what kind of comparison
    /// (if any) even applies.
    UnresolvedParamRef {
        choose_node: i64,
        param_ref: Option<String>,
    },
    /// The controlling value resolved but is not a valid `Condition_t`
    /// number.
    NonNumericValue {
        choose_node: i64,
        param_ref: Option<String>,
        raw: String,
    },
    /// [A] Design D9: a `TypeNone`-controlled `choose` whose children are
    /// not exactly one `when default="true"` — the one shape all 604
    /// corpus occurrences share. The Standard states a `TypeNone` parameter
    /// cannot control a `choose` at all, so there is no defined behaviour
    /// for any other shape; nothing activates.
    UnexpectedTypeNoneShape { choose_node: i64 },
    /// An element kind this build does not recognize in this position
    /// (design D10). Its subtree is not evaluated; every reference inside
    /// it is named by a [`Diagnostic::RefBelowSkippedNode`] (ADR-0041).
    UnrecognizedNode { node_id: i64, kind: String },
    /// ADR-0041 (PDB-9), amending D10: a `ParameterRefRef`,
    /// `ComObjectRefRef` or `Module` below a node the walk refuses for a
    /// *structural* reason. `skipped_node` is that node: an unrecognized
    /// kind, a non-`when` child of a `choose`, a `choose` with
    /// `UnresolvedParamRef` or `UnexpectedTypeNoneShape`, a recognized
    /// layout container (`Rows`/`Columns`), or a recognized leaf
    /// (`ParameterRefRef`, `ComObjectRefRef`, `ParameterSeparator`,
    /// `Assign`) that unexpectedly has children. The reference is *not*
    /// activated (that would guess at the skipped node's meaning) but it no
    /// longer vanishes. Reported in document order, every `choose` branch
    /// included; a `Module`'s own argument bindings are not references and
    /// are not descended (D19). Counted against `MAX_MODULE_ACTIVATIONS`.
    ///
    /// Deliberately *not* emitted for value-dependent refusals
    /// (`MissingValue`, `NonNumericValue`, `NoBranchMatched`): there the
    /// branches are conditionally hidden by design and the `choose` itself
    /// is already named. Nor for a `Module` that is not expanded
    /// (`ModuleDefNotFound`, cycle, depth, expansion budget): the `Module`
    /// is named by its own diagnostic, but its `ModuleDef`'s contents are
    /// not enumerated.
    RefBelowSkippedNode {
        skipped_node: i64,
        ref_node: i64,
        kind: String,
        ref_id: Option<String>,
    },
    /// Design D17: a `Module/@RefId` is absent, or names a `ModuleDef`
    /// with no stored tree for this program. The `Module` is not
    /// descended — not even its own `NumericArg`/`TextArg` children
    /// (design D19), which are argument bindings, not activations, in
    /// either outcome.
    ModuleDefNotFound {
        node_id: i64,
        ref_id: Option<String>,
    },
    /// A `Module` node whose expansion would repeat a `ModuleDef` already
    /// active somewhere up its own nesting chain — a genuine cycle, not
    /// merely a depth limit. Detected by walking the enclosing
    /// `ModuleScope` chain (`ModuleScope::parent`) and comparing
    /// `module_def_id`s *before* attempting to expand, so a cycle is
    /// caught at its first repeat rather than only once
    /// `MAX_MODULE_NESTING_DEPTH` is reached. The subtree is not
    /// descended — recursing into a cycle is exactly the stack-overflow
    /// risk this diagnostic exists to refuse instead of hitting.
    ModuleCycleDetected {
        node_id: i64,
        ref_id: Option<String>,
    },
    /// A `Module` node whose expansion would sit deeper than
    /// [`MAX_MODULE_NESTING_DEPTH`] allows. `depth` is the nesting depth
    /// the expansion would have occupied had it proceeded (always
    /// `MAX_MODULE_NESTING_DEPTH + 1` today, since `walk` checks before
    /// each recursive step rather than after). The subtree is not
    /// descended.
    ModuleNestingTooDeep {
        node_id: i64,
        ref_id: Option<String>,
        depth: usize,
    },
    /// One of two total-work budgets tripped, distinguished by which
    /// constant `budget` carries:
    ///
    /// - `budget == MAX_MODULE_EXPANSIONS`: `node_id` names a `Module`
    ///   node whose expansion would be the `MAX_MODULE_EXPANSIONS + 1`th
    ///   `Module` expansion performed by this `evaluate` call — not a
    ///   per-chain depth bound (see [`MAX_MODULE_EXPANSIONS`]'s own doc
    ///   comment for why the depth bound alone cannot catch this: fan-out
    ///   multiplies per level across many distinct, non-cyclic chains,
    ///   none of which individually reaches [`MAX_MODULE_NESTING_DEPTH`]).
    ///   The subtree is not descended, and no further `Module` anywhere
    ///   in this call is descended either once this budget is spent.
    /// - `budget == MAX_MODULE_ACTIVATIONS` (fix round 2): `node_id`
    ///   names the node at which the spent budget was first noticed —
    ///   either a `ParameterRefRef`/`ComObjectRefRef` whose activation
    ///   would be the `MAX_MODULE_ACTIVATIONS + 1`th recorded by this
    ///   call, combined across `parameter_refs` and `com_object_refs`,
    ///   or, since fix round 3, the `Module` node whose expansion was
    ///   refused because the budget was already spent when the walk
    ///   reached it. Which of the two it is depends on DFS order and on
    ///   nothing else; both are ordinary. See [`MAX_MODULE_ACTIVATIONS`]'s
    ///   own doc comment. Recorded once; every further ref or expansion
    ///   that would also cross the budget is dropped without a repeat
    ///   diagnostic.
    ///
    /// Either way: refused loudly, never crashed into, never silently
    /// truncated wholesale — only this one diagnostic marks where the
    /// budget was reached.
    ModuleExpansionBudgetExhausted {
        node_id: i64,
        ref_id: Option<String>,
        budget: usize,
    },
    /// A `choose`'s controlling `ParameterRef` resolved to a real
    /// comparable parameter, but no value for it exists anywhere in the
    /// resolution chain (supplied, `parameter_ref.value`, `parameter.value`).
    MissingValue {
        choose_node: i64,
        param_ref: Option<String>,
    },
    /// [V] `Module/@Id` is present on 102/102 corpus elements, but it is an
    /// optional attribute. Without it, this instantiation cannot be matched
    /// to a project-side `ModuleInstance`, so its stored per-channel values
    /// are unreachable and it evaluates against program defaults.
    ModuleWithoutId { node_id: i64 },
    /// A `Module` child that binds an argument (`NumericArg`/`TextArg`)
    /// but cannot be resolved to a declaration: no `@RefId`, no `@Value`,
    /// an `@RefId` naming no `ModuleDef/Arguments/Argument` this program
    /// declares, or a declaration with no `@Name` to bind to. The binding
    /// is reported, never dropped — an argument the evaluator cannot
    /// interpret is a compatibility fact, not a rounding error (design
    /// D50).
    ModuleArgumentNotBound {
        /// The binding element's own `node_id`, not the `Module`'s.
        node_id: i64,
        ref_id: Option<String>,
    },
    /// An argument construct this build does not interpret: a `Module`
    /// child that is neither `NumericArg` nor `TextArg`, or a declaration
    /// whose `@Type` is outside the two facets of `ModuleDefArgType_t`
    /// this build reads — `AllocatorRef` being the one such facet the
    /// published schema names (§1.1.2.38, **[D]**) and the one with zero
    /// attestation anywhere (**[V]**, task 12). Reported, never guessed at
    /// (design D50).
    UnsupportedModuleArgumentKind {
        node_id: i64,
        /// The element name, or the `@Type` spelling, that was met.
        kind: String,
    },
    /// A `{{Name}}` placeholder in an activated element's `@Text` whose
    /// `Name` matches no argument bound in the enclosing scope. The text
    /// keeps the placeholder verbatim — substituting an empty string would
    /// destroy the only evidence that something was meant to go there
    /// (design D50).
    ///
    /// Purely numeric placeholders (`{{0}}`, `{{1}}`) are a different,
    /// separately-attested family — 948 occurrences corpus-wide against
    /// 978 named ones (**[V]**, task 12) — tied to `TextParameterRefId`,
    /// not to module arguments. They are left verbatim and are *not*
    /// reported here: nothing about them is unresolved by this mechanism,
    /// because they were never this mechanism's to resolve.
    UnresolvedTextPlaceholder { node_id: i64, name: String },
}

impl Diagnostic {
    /// Whether this diagnostic can stand between a ref and its activation
    /// (ISSUE-08): after it, "not activated" may be wrong, not just
    /// conditional. `false` only for the two that cannot:
    ///
    /// - `NoBranchMatched`: the `choose` read a known value and no `when`
    ///   covers it. Activating nothing is the evaluator's `[A]` rule, and
    ///   a device read back octet for octet agreed with it (RESEARCH
    ///   §19.2 "Unmatched `choose`"); the download planner accepts it on
    ///   the same evidence.
    /// - `UnresolvedTextPlaceholder`: about label text, never about which
    ///   refs activate.
    ///
    /// Every other variant means some subtree was not walked or not walked
    /// with a known value, so an object not activated may still be.
    pub fn may_hide_refs(&self) -> bool {
        match self {
            Diagnostic::NoBranchMatched { .. } | Diagnostic::UnresolvedTextPlaceholder { .. } => {
                false
            }
            Diagnostic::UnparsableTest { .. }
            | Diagnostic::UnresolvedParamRef { .. }
            | Diagnostic::NonNumericValue { .. }
            | Diagnostic::UnexpectedTypeNoneShape { .. }
            | Diagnostic::UnrecognizedNode { .. }
            | Diagnostic::RefBelowSkippedNode { .. }
            | Diagnostic::ModuleDefNotFound { .. }
            | Diagnostic::ModuleCycleDetected { .. }
            | Diagnostic::ModuleNestingTooDeep { .. }
            | Diagnostic::ModuleExpansionBudgetExhausted { .. }
            | Diagnostic::MissingValue { .. }
            | Diagnostic::ModuleWithoutId { .. }
            | Diagnostic::ModuleArgumentNotBound { .. }
            | Diagnostic::UnsupportedModuleArgumentKind { .. } => true,
        }
    }
}

/// Which expansion produced a given activation or diagnostic (design D14).
/// `None` means the application program's own tree; `Some` names the
/// `Module` element in that tree whose expansion is being walked. Node ids
/// collide across `Dynamic` trees (`dynamic_node.node_id` restarts at
/// every `Dynamic` root, `dynamic/parse.rs`'s `handle_dynamic_element`) and
/// a `ModuleDef`'s local `ParameterRef`/`ComObjectRef` ids are reused
/// verbatim by every instantiating `Module` (RESEARCH.md §4.4) — an
/// unqualified result is ambiguous in both ways at once, which is why this
/// type exists and why every `ActiveRef`/`ScopedDiagnostic` carries one.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ModuleScope {
    /// `dynamic_node.node_id` of the `Module` element that produced this
    /// scope, within the tree named by `parent` (the program's own tree
    /// when `parent` is `None`, otherwise `parent`'s own `module_def_id`
    /// tree). Unique within *that* tree, but — since nested expansion —
    /// no longer unique on its own across the whole evaluation: two
    /// different nesting chains can each reuse the same `node_id` at
    /// their own respective depth, because `dynamic_node.node_id` resets
    /// per `(program_id, module_def_id)`. Identity therefore needs the
    /// full chain (`ScopeKey` below), not `module_node` alone.
    pub module_node: i64,
    /// `Module/@Id`, for human-readable reporting. `None` if the element
    /// carries no `@Id`. Never relied on for identity.
    pub module_id: Option<String>,
    /// `Module/@RefId`, i.e. the `ModuleDef/@Id` whose tree was walked.
    pub module_def_id: String,
    /// This instantiation's resolved argument bindings, in the document
    /// order of the `Module`'s own `NumericArg`/`TextArg` children (design
    /// D48): each declaration's `@Name` paired with the `@Value` this
    /// `Module` bound it to. A `Vec`, not a map, because it holds three
    /// entries in the widest corpus sample and a linear scan of three is
    /// cheaper than hashing — and because `ModuleScope` is `Hash`, which a
    /// `HashMap` field would not be.
    ///
    /// This is what makes two instantiations of one `ModuleDef` differ:
    /// everything else in the scope describes *where* the expansion
    /// happened, this describes what it was told.
    pub arguments: Vec<BoundArgument>,
    /// The enclosing scope one level up the nesting chain: `None` when
    /// this `Module` was found in the application program's own tree
    /// (nesting depth 1); `Some` when it was found inside another
    /// expanded `ModuleDef`'s tree (nested-expansion addendum, task 11).
    /// `Rc`, not `Box` (fix round 2, blocking finding 1's residual): every
    /// activated `ParameterRefRef`/`ComObjectRefRef` clones its enclosing
    /// scope into `ActiveRef::scope`, and with a boxed chain that clone
    /// walked and reallocated every ancestor — `Rc::clone` is a refcount
    /// bump instead, turning a ~1.5 kB deep clone per ref into 8 bytes.
    /// The chain can never grow past [`MAX_MODULE_NESTING_DEPTH`] `Rc`s
    /// deep, because `walk` refuses to expand a `Module` any deeper than
    /// that.
    pub parent: Option<Rc<ModuleScope>>,
}

/// One resolved argument binding: a `ModuleDef/Arguments/Argument`'s
/// declared `@Name` and the `@Value` one `Module` bound to it.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BoundArgument {
    pub name: String,
    pub value: String,
}

impl ModuleScope {
    /// The value bound to `name` in *this* scope. Deliberately not
    /// recursive up the `parent` chain: `Identifier50_t` names a
    /// `ModuleDef`'s own argument (**[D]**, §1.1.3.9), the corpus has no
    /// nested `Module` at all to show otherwise (**[V]**), and inventing
    /// outer-scope inheritance would be inventing semantics.
    fn argument(&self, name: &str) -> Option<&str> {
        self.arguments
            .iter()
            .find(|a| a.name == name)
            .map(|a| a.value.as_str())
    }

    /// Nesting depth of this scope: 1 for a `Module` found directly in
    /// the application program's own tree, 2 for one found inside that
    /// expansion's `ModuleDef` tree, and so on.
    pub fn depth(&self) -> usize {
        1 + self.parent.as_deref().map_or(0, ModuleScope::depth)
    }

    /// Whether `module_def_id` already appears somewhere in this scope's
    /// own chain (this scope's own `module_def_id` included) — the cycle
    /// test: expanding `module_def_id` again from here would re-enter a
    /// `ModuleDef` already being walked.
    fn chain_contains(&self, module_def_id: &str) -> bool {
        self.module_def_id == module_def_id
            || self
                .parent
                .as_deref()
                .is_some_and(|p| p.chain_contains(module_def_id))
    }

    /// The `module_node` chain, outermost first, ending with this scope's
    /// own — the full-identity key nested dedup needs (see `module_node`'s
    /// own doc comment for why `module_node` alone is not enough). `pub`
    /// (fix round 1, blocking finding 4, goal-completion task 11) so
    /// callers outside this crate — `apps/knx-server`'s parameter-panel
    /// section grouping, specifically — can key on the same full chain
    /// this crate's own `Activation` dedup uses, instead of falling back
    /// to the flat `module_node` that two different nesting chains can
    /// share.
    pub fn node_chain(&self) -> Vec<i64> {
        let mut chain = self
            .parent
            .as_deref()
            .map_or_else(Vec::new, ModuleScope::node_chain);
        chain.push(self.module_node);
        chain
    }
}

/// One activated `ParameterRefRef`/`ComObjectRefRef` id, qualified by
/// where it was found (design D14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRef {
    pub scope: Option<Rc<ModuleScope>>,
    pub ref_id: String,
    /// The innermost `Channel` or `ChannelIndependentBlock` this ref was
    /// activated under (ISSUE-08), or `None` if the walk reached it
    /// outside every channel element. A `Module` instantiated inside a
    /// channel passes that channel on to the refs of its `ModuleDef` tree,
    /// unless that tree opens a channel of its own.
    pub channel: Option<ChannelOwner>,
}

/// The channel element that owns an activated ref (ISSUE-08).
///
/// Structural, not heuristic: it is the nearest enclosing `Channel`/
/// `ChannelIndependentBlock` on the evaluation path. Corpus: in the three
/// corpus projects' programs every `ComObjectRefRef` sits under exactly
/// one channel element (5,630 / 5,630 / 8), so the owner is unambiguous
/// there. Its display text is the matching [`ActiveLabel`] (same `scope`
/// and `node_id`), when the element carries a non-empty `@Text`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelOwner {
    /// The scope the channel element was found in. Not necessarily the
    /// ref's own scope: a module that opens no channel inherits the one
    /// around its `Module` element.
    pub scope: Option<Rc<ModuleScope>>,
    /// `dynamic_node.node_id` of the channel element within `scope`'s tree.
    pub node_id: i64,
    /// `Channel` or `ChannelIndependentBlock`.
    pub kind: String,
    /// The element's `@Id`, as stored.
    pub element_id: Option<String>,
    /// `Channel/@Name`, verbatim (ADR-0052). `None` for a
    /// `ChannelIndependentBlock` and for a channel without the attribute.
    pub name: Option<String>,
    /// `Channel/@Number`, verbatim (ADR-0052).
    pub number: Option<String>,
}

/// One activated label: the `@Text` of an activated `Channel`,
/// `ParameterBlock` or `ParameterSeparator`, with every `{{Name}}`
/// placeholder resolved against the enclosing scope's argument bindings
/// (design D49).
///
/// This is the evaluator's answer to "what does this module instance
/// actually say", and it is the one place where an argument value changes
/// the evaluated result: two `Module` elements naming the same `ModuleDef`
/// produce the same refs under different scopes, but different labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveLabel {
    pub scope: Option<Rc<ModuleScope>>,
    /// `dynamic_node.node_id` of the labelled element, within the tree
    /// named by `scope` — the same contract `Diagnostic`'s node ids have.
    pub node_id: i64,
    /// The element kind the label came from (`Channel`, `ParameterBlock`,
    /// `ParameterSeparator`).
    pub kind: String,
    /// `@Text` as stored, placeholders and all.
    pub raw_text: String,
    /// `@Text` after substitution. Equal to `raw_text` when there was
    /// nothing to substitute — the overwhelmingly common case, since only
    /// text inside a `ModuleDef` tree can carry an argument placeholder.
    pub text: String,
}

/// One `Diagnostic`, qualified by where it was found (design D14).
/// `Diagnostic`'s own variants are unchanged — the scope wraps them, it
/// does not move into them, so a diagnostic's `node_id`/`choose_node`/
/// `when_node` keeps meaning "a `node_id` within the tree named by this
/// `scope`", same contract as slice 1, now with the scope actually present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedDiagnostic {
    pub scope: Option<Rc<ModuleScope>>,
    pub diagnostic: Diagnostic,
}

/// The result of evaluating a program's `Dynamic` trees against a
/// `ValueMap`: which `ParameterRefRef`s and `ComObjectRefRef`s are active,
/// and every diagnostic encountered along the way. Both id lists are in
/// document order — program tree first, then each `Module`'s expansion
/// inlined at its position (design D18) — deduplicated by first occurrence
/// per scope (design D11, qualified by D18): an id legitimately reachable
/// through more than one active branch *within the same scope* appears
/// once, at its first position; the same id under two different
/// instantiating `Module`s appears once per instantiation.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Activation {
    pub parameter_refs: Vec<ActiveRef>,
    pub com_object_refs: Vec<ActiveRef>,
    /// Activated `Channel`/`ParameterBlock`/`ParameterSeparator` labels in
    /// document order (design D49), one per activated element carrying a
    /// non-empty `@Text`.
    pub labels: Vec<ActiveLabel>,
    pub diagnostics: Vec<ScopedDiagnostic>,
    /// How many [`Diagnostic::RefBelowSkippedNode`]s were emitted. Counted
    /// against [`MAX_MODULE_ACTIVATIONS`] like a ref or a label (ADR-0041):
    /// a `ModuleDef` wrapping its refs in an unrecognized node multiplies
    /// them with fan-out exactly as unwrapped refs would.
    skipped_refs_reported: usize,
}

/// The dedup key design D18 specifies, qualified for nested expansion
/// (task 11): the *full* `module_node` chain (`ModuleScope::node_chain`,
/// not `module_node` alone — see that field's own doc comment for why a
/// single node id is not unique across different nesting chains), paired
/// with the raw `ref_id`. `module_id` plays no part in identity, same as
/// before.
type ScopeKey = (Vec<i64>, String);

fn scope_key_chain(scope: Option<&Rc<ModuleScope>>) -> Vec<i64> {
    scope.map(|s| s.node_chain()).unwrap_or_default()
}

impl Activation {
    /// Combined `parameter_refs.len() + com_object_refs.len()`, checked
    /// against [`MAX_MODULE_ACTIVATIONS`] by `activate_parameter_ref`/
    /// `activate_com_object_ref` before pushing (fix round 2, blocking
    /// finding 1's residual).
    /// Task 12 (design D51) folds `labels` in: a label is produced per
    /// activated element per expansion, so it multiplies with fan-out
    /// exactly the way a ref does, and a budget that counted refs but not
    /// labels would be a budget with a hole in it the width of a
    /// `ModuleDef` whose tree is all `Channel`s.
    fn activations_recorded(&self) -> usize {
        self.parameter_refs.len()
            + self.com_object_refs.len()
            + self.labels.len()
            + self.skipped_refs_reported
    }

    /// Whether the activation budget has already produced its one
    /// diagnostic. Needed because `activations_recorded()` freezes at
    /// `MAX_MODULE_ACTIVATIONS` once the budget is hit (nothing pushes
    /// past it), so a bare `total == MAX_MODULE_ACTIVATIONS` check would
    /// be true on *every* subsequent refused ref, not just the first —
    /// this scan (cheap: at most one matching entry ever exists, so it's
    /// O(1) in practice once the budget has tripped) is what keeps it to
    /// exactly one.
    fn activation_budget_already_diagnosed(&self) -> bool {
        self.diagnostics.iter().any(|d| {
            matches!(
                d.diagnostic,
                Diagnostic::ModuleExpansionBudgetExhausted { budget, .. }
                    if budget == MAX_MODULE_ACTIVATIONS
            )
        })
    }

    /// Whether the activation budget is spent, and — on the first call
    /// that finds it spent — the place its one diagnostic is recorded.
    /// `true` means the caller must stop: stop activating, and stop
    /// expanding.
    ///
    /// Fix round 3 (scoped re-review of round 2) moved this ahead of
    /// everything that allocates. Round 2 checked the budget *after*
    /// `seen.insert(key)`, and `walk`'s `Module` arm did not consult it
    /// at all, so a run that had spent its activation budget went right
    /// on expanding — up to the far larger [`MAX_MODULE_EXPANSIONS`] —
    /// and every ref it met still built and inserted a `ScopeKey`
    /// (`(Vec<i64>, String)`, two heap allocations) into a `seen` set no
    /// budget bounded. The refs stopped being recorded; the work did
    /// not. That is the same defect round 2 was dispatched to fix,
    /// relocated from `ActiveRef` cloning into `seen`-set growth.
    fn activation_budget_spent(
        &mut self,
        scope: Option<&Rc<ModuleScope>>,
        node_id: i64,
        ref_id: Option<&str>,
    ) -> bool {
        if self.activations_recorded() < MAX_MODULE_ACTIVATIONS {
            return false;
        }
        if !self.activation_budget_already_diagnosed() {
            self.diagnose(
                scope,
                Diagnostic::ModuleExpansionBudgetExhausted {
                    node_id,
                    ref_id: ref_id.map(str::to_string),
                    budget: MAX_MODULE_ACTIVATIONS,
                },
            );
        }
        true
    }

    fn activate_parameter_ref(
        &mut self,
        seen: &mut HashSet<ScopeKey>,
        scope: Option<&Rc<ModuleScope>>,
        channel: Option<&ChannelOwner>,
        node_id: i64,
        id: String,
    ) {
        if self.activation_budget_spent(scope, node_id, Some(&id)) {
            return;
        }
        let key = (scope_key_chain(scope), id.clone());
        if !seen.insert(key) {
            return;
        }
        self.parameter_refs.push(ActiveRef {
            scope: scope.cloned(),
            ref_id: id,
            channel: channel.cloned(),
        });
    }

    fn activate_com_object_ref(
        &mut self,
        seen: &mut HashSet<ScopeKey>,
        scope: Option<&Rc<ModuleScope>>,
        channel: Option<&ChannelOwner>,
        node_id: i64,
        id: String,
    ) {
        if self.activation_budget_spent(scope, node_id, Some(&id)) {
            return;
        }
        let key = (scope_key_chain(scope), id.clone());
        if !seen.insert(key) {
            return;
        }
        self.com_object_refs.push(ActiveRef {
            scope: scope.cloned(),
            ref_id: id,
            channel: channel.cloned(),
        });
    }

    fn diagnose(&mut self, scope: Option<&Rc<ModuleScope>>, diagnostic: Diagnostic) {
        self.diagnostics.push(ScopedDiagnostic {
            scope: scope.cloned(),
            diagnostic,
        });
    }
}

/// Evaluates `trees` against `values`: a pure function, single-pass,
/// depth-first, document order (design D6, extended by D12-D19 to a tree
/// set). No database, no I/O, no logging — every diagnostic is returned in
/// `Activation::diagnostics`. The walk starts at the program's own tree
/// (`module_def_id = ""`, unscoped); a `Module` node found there is
/// expanded into `trees`' matching `ModuleDef` tree, scoped to that
/// `Module` (design D19). Also bounded by [`MAX_MODULE_ACTIVATIONS`]
/// (fix round 2), the budget on total recorded refs complementing
/// [`MAX_MODULE_EXPANSIONS`]'s budget on total `Module` expansions.
pub fn evaluate(trees: &ProgramTrees, values: &ValueMap) -> Activation {
    let mut activation = Activation::default();
    let mut seen_params = HashSet::new();
    let mut seen_coms = HashSet::new();
    let mut expansions_used = 0usize;
    for &root in trees.program.roots() {
        walk(
            trees,
            &trees.program,
            root,
            values,
            &mut activation,
            &mut seen_params,
            &mut seen_coms,
            &mut expansions_used,
            None,
            None,
        );
    }
    activation
}

/// Kinds recognized as transparent containers (design D10): they carry no
/// activation meaning of their own, so evaluation simply descends into
/// every child in document order.
fn is_transparent_container(kind: &str) -> bool {
    matches!(
        kind,
        "Dynamic" | "ChannelIndependentBlock" | "Channel" | "ParameterBlock" | "when"
    )
}

/// The transparent containers that own what is activated below them
/// (ISSUE-08, [`ChannelOwner`]).
fn is_channel(kind: &str) -> bool {
    matches!(kind, "Channel" | "ChannelIndependentBlock")
}

/// ADR-0041: a `ParameterBlock`'s table layout. Corpus (304 distinct
/// programs): `Rows` and `Columns` 4,267 each, always directly under a
/// `ParameterBlock` carrying `@Layout`, holding only `Row`/`Column`, never
/// a reference. Recognized presentation — no diagnostic, no activation.
fn is_layout_container(kind: &str) -> bool {
    matches!(kind, "Rows" | "Columns")
}

/// The element kinds [`Diagnostic::RefBelowSkippedNode`] names.
fn is_reference(kind: &str) -> bool {
    matches!(kind, "ParameterRefRef" | "ComObjectRefRef" | "Module")
}

/// Names every reference strictly below `skipped`, depth-first in document
/// order, against `skipped` itself (ADR-0041). Nothing is evaluated, so a
/// `choose` contributes all of its branches and nested unrecognized kinds
/// are not separately reported; a `Module`'s children are argument
/// bindings (D19) and are not descended. Iterative, so a hostile nesting
/// depth cannot exhaust the stack. Each report counts against
/// [`MAX_MODULE_ACTIVATIONS`]; once that budget is spent the walk stops
/// here and the budget's own diagnostic says output was truncated.
fn report_refs_below(
    tree: &DynamicTree,
    skipped: i64,
    activation: &mut Activation,
    scope: Option<&Rc<ModuleScope>>,
) {
    let mut pending: Vec<i64> = tree
        .children_of(Some(skipped))
        .iter()
        .rev()
        .copied()
        .collect();
    while let Some(id) = pending.pop() {
        let Some(node) = tree.node(id) else {
            continue;
        };
        if is_reference(&node.kind) {
            if activation.activation_budget_spent(scope, id, node.ref_id.as_deref()) {
                return;
            }
            activation.skipped_refs_reported += 1;
            activation.diagnose(
                scope,
                Diagnostic::RefBelowSkippedNode {
                    skipped_node: skipped,
                    ref_node: id,
                    kind: node.kind.clone(),
                    ref_id: node.ref_id.clone(),
                },
            );
            if node.kind == "Module" {
                continue;
            }
        }
        pending.extend(tree.children_of(Some(id)).iter().rev().copied());
    }
}

/// `tree` is the `Dynamic` tree currently being walked — the program's own
/// tree while `scope` is `None`, or the `ModuleDef` tree named by the
/// innermost `Module` in `scope`'s chain once one has been expanded.
/// `trees` is consulted at *every* `Module` node, nesting included (D44
/// supersedes D15's one-level policy), to resolve its `@RefId` against the
/// program's other stored scopes. `expansions_used` is the running count of
/// `Module` expansions performed by this whole `evaluate` call, checked
/// against [`MAX_MODULE_EXPANSIONS`] before each one — see that constant's
/// own doc comment for why the per-chain depth bound alone cannot do this
/// job.
#[allow(clippy::too_many_arguments)]
fn walk(
    trees: &ProgramTrees,
    tree: &DynamicTree,
    node_id: i64,
    values: &ValueMap,
    activation: &mut Activation,
    seen_params: &mut HashSet<ScopeKey>,
    seen_coms: &mut HashSet<ScopeKey>,
    expansions_used: &mut usize,
    scope: Option<&Rc<ModuleScope>>,
    channel: Option<&ChannelOwner>,
) {
    let Some(node) = tree.node(node_id) else {
        return;
    };
    if is_transparent_container(&node.kind) {
        record_label(activation, scope, node);
        // ISSUE-08: a channel element becomes the owner of everything
        // activated below it; any other container passes the current one on.
        let opened = is_channel(&node.kind).then(|| ChannelOwner {
            scope: scope.cloned(),
            node_id,
            kind: node.kind.clone(),
            element_id: node.element_id.clone(),
            name: node.name.clone(),
            number: node.number.clone(),
        });
        let channel = opened.as_ref().or(channel);
        for &child in tree.children_of(Some(node_id)) {
            walk(
                trees,
                tree,
                child,
                values,
                activation,
                seen_params,
                seen_coms,
                expansions_used,
                scope,
                channel,
            );
        }
        return;
    }
    match node.kind.as_str() {
        "choose" => evaluate_choose(
            trees,
            tree,
            node,
            values,
            activation,
            seen_params,
            seen_coms,
            expansions_used,
            scope,
            channel,
        ),
        "ParameterRefRef" => {
            if let Some(id) = &node.ref_id {
                activation.activate_parameter_ref(seen_params, scope, channel, node_id, id.clone());
            }
            report_refs_below(tree, node_id, activation, scope);
        }
        "ComObjectRefRef" => {
            if let Some(id) = &node.ref_id {
                activation.activate_com_object_ref(seen_coms, scope, channel, node_id, id.clone());
            }
            report_refs_below(tree, node_id, activation, scope);
        }
        // Recognized and deliberately inert (design D10): presentation
        // (`ParameterSeparator`) and assignment (`Assign`) constructs whose
        // semantics this slice does not model. Neither activates anything
        // nor is expected to have children in the corpus, so there is
        // nothing to descend into.
        // `ParameterSeparator` is still inert as an activation, but it
        // carries `@Text` and therefore a label (design D49). `Assign`
        // does not and stays entirely inert.
        // ADR-0041: none of these is expected to have children (none does
        // in the corpus), and none is descended; should one ever hold a
        // reference, it is named rather than lost.
        "ParameterSeparator" => {
            record_label(activation, scope, node);
            report_refs_below(tree, node_id, activation, scope);
        }
        "Assign" => report_refs_below(tree, node_id, activation, scope),
        kind if is_layout_container(kind) => report_refs_below(tree, node_id, activation, scope),
        // Design D19: `Module` is dispatched here like any other node kind,
        // no special container handling. Its own children (`NumericArg`/
        // `TextArg`) are argument bindings, not activations, and are never
        // descended into, in either branch below.
        // Nested-expansion addendum (task 11, superseding D15): a
        // `Module` is handled the same way whether `scope` is `None`
        // (found in the program's own tree) or `Some` (found inside an
        // already-expanded `ModuleDef`'s tree) — the only difference is
        // what `scope` is when a diagnostic or a new nested `ModuleScope`
        // is built.
        "Module" => {
            let hit = node
                .ref_id
                .as_deref()
                .and_then(|rid| trees.module(rid).map(|t| (rid.to_string(), t)));
            match hit {
                Some((module_def_id, module_tree)) => {
                    // Cycle check first, and strictly before the depth
                    // check: a self-referential chain is caught at its
                    // first repeat, with a diagnostic naming the actual
                    // cause, rather than being left to run up against
                    // MAX_MODULE_NESTING_DEPTH and get the less specific
                    // "too deep" diagnostic instead.
                    if scope.is_some_and(|s| s.chain_contains(&module_def_id)) {
                        activation.diagnose(
                            scope,
                            Diagnostic::ModuleCycleDetected {
                                node_id,
                                ref_id: node.ref_id.clone(),
                            },
                        );
                        return;
                    }
                    let depth = scope.map_or(0, |s| s.depth()) + 1;
                    if depth > MAX_MODULE_NESTING_DEPTH {
                        activation.diagnose(
                            scope,
                            Diagnostic::ModuleNestingTooDeep {
                                node_id,
                                ref_id: node.ref_id.clone(),
                                depth,
                            },
                        );
                        return;
                    }
                    // Blocking finding 1 (goal-completion task 11, fix
                    // round 1): the depth bound above refuses one chain
                    // going too deep; it does not refuse many chains each
                    // staying shallow. This is the total-work backstop —
                    // checked last, after the more specific cycle/depth
                    // diagnoses, so a cycle or a too-deep chain still gets
                    // its own precise diagnostic first.
                    if *expansions_used >= MAX_MODULE_EXPANSIONS {
                        activation.diagnose(
                            scope,
                            Diagnostic::ModuleExpansionBudgetExhausted {
                                node_id,
                                ref_id: node.ref_id.clone(),
                                budget: MAX_MODULE_EXPANSIONS,
                            },
                        );
                        return;
                    }
                    // Fix round 3: the other budget stops the walk here
                    // too. Expanding a `ModuleDef` whose every activation
                    // will be refused is pure cost — the subtree still
                    // gets walked, its conditions still get evaluated,
                    // and its refs still reach `activate_*`. Nothing
                    // downstream can record a result, so there is nothing
                    // to gain by descending.
                    if activation.activation_budget_spent(scope, node_id, node.ref_id.as_deref()) {
                        return;
                    }
                    *expansions_used += 1;
                    // Design D37: a nameless instantiation can never be
                    // matched to a project-side `ModuleInstance`, so it
                    // is worth saying out loud even when — as here —
                    // nothing downstream has asked for its value yet.
                    // Emitted once per instantiation, unconditionally,
                    // before descending; `scope` is the enclosing scope
                    // this `Module` element itself was found in, not the
                    // (nameless) scope it would create.
                    if node.element_id.is_none() {
                        activation.diagnose(scope, Diagnostic::ModuleWithoutId { node_id });
                    }
                    let arguments =
                        bind_arguments(trees, tree, node_id, &module_def_id, activation, scope);
                    let new_scope = Rc::new(ModuleScope {
                        module_node: node_id,
                        module_id: node.element_id.clone(),
                        module_def_id,
                        arguments,
                        parent: scope.cloned(),
                    });
                    for &root in module_tree.roots() {
                        walk(
                            trees,
                            module_tree,
                            root,
                            values,
                            activation,
                            seen_params,
                            seen_coms,
                            expansions_used,
                            Some(&new_scope),
                            channel,
                        );
                    }
                }
                None => activation.diagnose(
                    scope,
                    Diagnostic::ModuleDefNotFound {
                        node_id,
                        ref_id: node.ref_id.clone(),
                    },
                ),
            }
        }
        other => {
            activation.diagnose(
                scope,
                Diagnostic::UnrecognizedNode {
                    node_id,
                    kind: other.to_string(),
                },
            );
            report_refs_below(tree, node_id, activation, scope);
        }
    }
}

/// Reads one `Module` element's argument bindings into the resolved
/// `(name, value)` pairs its scope will carry (design D48).
///
/// Design D19 said a `Module`'s own children are not descended into, being
/// argument bindings rather than activations. That still holds — this is
/// not a `walk` call and nothing here activates anything. What changes in
/// task 12 is that the bindings are now *read* on the way past instead of
/// being left where they lay.
///
/// Every binding that cannot be resolved produces a diagnostic rather than
/// silence, and the diagnostics carry the *binding element's* `node_id`,
/// which lives in `tree` — the enclosing tree the `Module` was found in —
/// so they are reported against the enclosing `scope`, not the scope about
/// to be created.
fn bind_arguments(
    trees: &ProgramTrees,
    tree: &DynamicTree,
    module_node_id: i64,
    module_def_id: &str,
    activation: &mut Activation,
    scope: Option<&Rc<ModuleScope>>,
) -> Vec<BoundArgument> {
    let mut bound = Vec::new();
    for &child_id in tree.children_of(Some(module_node_id)) {
        let Some(child) = tree.node(child_id) else {
            continue;
        };
        match child.kind.as_str() {
            "NumericArg" | "TextArg" => {}
            // Not a binding spelling this build knows. The published
            // `ModuleDefArgType_t` has a third facet with no attested
            // element spelling at all, so anything else found here is
            // reported and left alone rather than guessed at.
            other => {
                activation.diagnose(
                    scope,
                    Diagnostic::UnsupportedModuleArgumentKind {
                        node_id: child_id,
                        kind: other.to_string(),
                    },
                );
                continue;
            }
        }
        let declaration = child.ref_id.as_deref().and_then(|r| trees.argument(r));
        let resolved = match (declaration, child.value.as_deref()) {
            // A declaration belonging to a different `ModuleDef` than the
            // one being instantiated is not a binding this build claims to
            // understand: nothing in the corpus does it, so it is reported
            // rather than honoured.
            (Some(decl), Some(value)) if decl.module_def_id == module_def_id => {
                match (decl.kind(), decl.name.as_deref()) {
                    (ArgumentKind::Unsupported, _) => {
                        activation.diagnose(
                            scope,
                            Diagnostic::UnsupportedModuleArgumentKind {
                                node_id: child_id,
                                kind: decl.arg_type.clone().unwrap_or_default(),
                            },
                        );
                        None
                    }
                    (_, Some(name)) => Some(BoundArgument {
                        name: name.to_string(),
                        value: value.to_string(),
                    }),
                    // Declared, typed, and nameless: there is no token a
                    // placeholder could spell, so the value can never be
                    // reached.
                    (_, None) => {
                        activation.diagnose(
                            scope,
                            Diagnostic::ModuleArgumentNotBound {
                                node_id: child_id,
                                ref_id: child.ref_id.clone(),
                            },
                        );
                        None
                    }
                }
            }
            _ => {
                activation.diagnose(
                    scope,
                    Diagnostic::ModuleArgumentNotBound {
                        node_id: child_id,
                        ref_id: child.ref_id.clone(),
                    },
                );
                None
            }
        };
        if let Some(binding) = resolved {
            bound.push(binding);
        }
    }
    bound
}

/// Records one activated element's `@Text` as a label, substituted against
/// the enclosing scope's argument bindings (design D49). An element with no
/// `@Text`, or an empty one, produces nothing — an empty label says less
/// than no label and would cost a budget slot to say it.
fn record_label(activation: &mut Activation, scope: Option<&Rc<ModuleScope>>, node: &DynamicNode) {
    let Some(raw) = node.text.as_deref().filter(|t| !t.is_empty()) else {
        return;
    };
    if activation.activation_budget_spent(scope, node.node_id, None) {
        return;
    }
    let text = substitute_arguments(raw, scope, node.node_id, activation);
    activation.labels.push(ActiveLabel {
        scope: scope.cloned(),
        node_id: node.node_id,
        kind: node.kind.clone(),
        raw_text: raw.to_string(),
        text,
    });
}

/// The opening and closing delimiters of the placeholder syntax every
/// `@Text` in the researched corpus uses. **[A]**: no published schema in
/// either knowledge base states a substitution rule for application-program
/// text — the inference rests on corpus consistency, all 978 named
/// placeholders resolving to an `Argument/@Name` declared by the enclosing
/// `ModuleDef`, with none left over (**[V]**, task 12).
const PLACEHOLDER_OPEN: &str = "{{";
const PLACEHOLDER_CLOSE: &str = "}}";

/// Replaces every `{{Name}}` in `raw` with the value `scope` bound to the
/// argument called `Name` (design D49).
///
/// Three things deliberately do not happen here:
///
/// * A name with no binding is **left verbatim** and reported as
///   [`Diagnostic::UnresolvedTextPlaceholder`]. Substituting an empty
///   string would erase the only trace that a value was meant to appear.
/// * A purely numeric placeholder (`{{0}}`) is left verbatim and *not*
///   reported: it belongs to the positional family tied to
///   `TextParameterRefId`, which this build does not model and which was
///   never an argument reference to begin with.
/// * The substituted value is not re-scanned. A value that itself contains
///   `{{...}}` is inserted as-is, so no input can make this loop feed
///   itself — the scan advances strictly left to right over `raw` and
///   terminates in one pass.
fn substitute_arguments(
    raw: &str,
    scope: Option<&Rc<ModuleScope>>,
    node_id: i64,
    activation: &mut Activation,
) -> String {
    substitute_with(raw, scope.map(Rc::as_ref), |name| {
        activation.diagnose(
            scope,
            Diagnostic::UnresolvedTextPlaceholder {
                node_id,
                name: name.to_string(),
            },
        );
    })
}

/// [`substitute_arguments`]' rule for text that is read *outside* the
/// walk (ISSUE-08): a `ComObject/@FunctionText` or a translated channel
/// `@Text` inside a `ModuleDef` carries the same `{{Name}}` placeholders as
/// the labels the walk records. Same three rules — unbound names and
/// numeric placeholders stay verbatim, nothing is re-scanned — but there is
/// no `Activation` to report into, so an unbound name is only left
/// visible, not diagnosed. `scope: None` returns `raw` unchanged apart
/// from that: the program's own tree binds no arguments.
pub fn substitute_text(raw: &str, scope: Option<&ModuleScope>) -> String {
    substitute_with(raw, scope, |_| {})
}

fn substitute_with(
    raw: &str,
    scope: Option<&ModuleScope>,
    mut unresolved: impl FnMut(&str),
) -> String {
    if !raw.contains(PLACEHOLDER_OPEN) {
        return raw.to_string();
    }
    let mut out = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(open) = rest.find(PLACEHOLDER_OPEN) {
        let after_open = &rest[open + PLACEHOLDER_OPEN.len()..];
        let Some(close) = after_open.find(PLACEHOLDER_CLOSE) else {
            break;
        };
        let name = &after_open[..close];
        out.push_str(&rest[..open]);
        match (is_argument_name(name), scope.and_then(|s| s.argument(name))) {
            (true, Some(value)) => out.push_str(value),
            (true, None) => {
                out.push_str(PLACEHOLDER_OPEN);
                out.push_str(name);
                out.push_str(PLACEHOLDER_CLOSE);
                unresolved(name);
            }
            (false, _) => {
                out.push_str(PLACEHOLDER_OPEN);
                out.push_str(name);
                out.push_str(PLACEHOLDER_CLOSE);
            }
        }
        rest = &after_open[close + PLACEHOLDER_CLOSE.len()..];
    }
    out.push_str(rest);
    out
}

/// Whether `name` is shaped like an argument name — `Identifier50_t`'s
/// published pattern, "a letter or underscore followed by letters, digits
/// or underscores" (`Project Schema23 v01.00.00` §1.1.3.9, **[D]**). This
/// is what separates `{{ChNo}}` from `{{0}}`: the second cannot be an
/// argument name under that pattern, so it is not treated as one and not
/// reported as an unresolved one.
fn is_argument_name(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[allow(clippy::too_many_arguments)]
fn evaluate_choose(
    trees: &ProgramTrees,
    tree: &DynamicTree,
    node: &DynamicNode,
    values: &ValueMap,
    activation: &mut Activation,
    seen_params: &mut HashSet<ScopeKey>,
    seen_coms: &mut HashSet<ScopeKey>,
    expansions_used: &mut usize,
    scope: Option<&Rc<ModuleScope>>,
    channel: Option<&ChannelOwner>,
) {
    let Some(control_kind) = node.control_kind else {
        activation.diagnose(
            scope,
            Diagnostic::UnresolvedParamRef {
                choose_node: node.node_id,
                param_ref: node.ref_id.clone(),
            },
        );
        // ADR-0041: a structural refusal — the product data itself is not
        // understood — so every reference in every branch is named.
        report_refs_below(tree, node.node_id, activation, scope);
        return;
    };

    match control_kind {
        // Design D9: no comparison is attempted at all. The sole shape all
        // 604 corpus occurrences share is exactly one `when default="true"`
        // child; anything else has no defined behaviour, because the
        // Standard says a `TypeNone` parameter cannot control a `choose`
        // in the first place.
        ControlKind::TypeNone => {
            let children = tree.children_of(Some(node.node_id));
            let sole_default = match children {
                [only] => tree
                    .node(*only)
                    .filter(|n| n.kind == "when" && n.is_default)
                    .map(|_| *only),
                _ => None,
            };
            match sole_default {
                Some(child) => walk(
                    trees,
                    tree,
                    child,
                    values,
                    activation,
                    seen_params,
                    seen_coms,
                    expansions_used,
                    scope,
                    channel,
                ),
                None => {
                    activation.diagnose(
                        scope,
                        Diagnostic::UnexpectedTypeNoneShape {
                            choose_node: node.node_id,
                        },
                    );
                    // ADR-0041: structural refusal, as for
                    // `UnresolvedParamRef` above.
                    report_refs_below(tree, node.node_id, activation, scope);
                }
            }
        }
        ControlKind::Comparable => evaluate_comparable_choose(
            trees,
            tree,
            node,
            values,
            activation,
            seen_params,
            seen_coms,
            expansions_used,
            scope,
            channel,
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_comparable_choose(
    trees: &ProgramTrees,
    tree: &DynamicTree,
    node: &DynamicNode,
    values: &ValueMap,
    activation: &mut Activation,
    seen_params: &mut HashSet<ScopeKey>,
    seen_coms: &mut HashSet<ScopeKey>,
    expansions_used: &mut usize,
    scope: Option<&Rc<ModuleScope>>,
    channel: Option<&ChannelOwner>,
) {
    let param_ref = node.ref_id.clone();
    let Some(raw_value) = node
        .ref_id
        .as_deref()
        .and_then(|id| values.get(scope.map(|s| s.as_ref()), id))
    else {
        activation.diagnose(
            scope,
            Diagnostic::MissingValue {
                choose_node: node.node_id,
                param_ref,
            },
        );
        return;
    };
    let Ok(observed) = raw_value.trim().parse::<i64>() else {
        activation.diagnose(
            scope,
            Diagnostic::NonNumericValue {
                choose_node: node.node_id,
                param_ref,
                raw: raw_value.to_string(),
            },
        );
        return;
    };

    // First (and only) matching test wins; `@default="true"` covers the
    // rest. [A] This is an inference from 22630 consistent corpus
    // observations (RESEARCH.md §4.3), not a stated rule: `@default` never
    // co-occurs with `@test`, no `choose` has more than one default `when`,
    // and no `choose` has two `when` children with an identical `@test`.
    let mut selected: Option<i64> = None;
    let mut default_child: Option<i64> = None;
    for &child_id in tree.children_of(Some(node.node_id)) {
        let Some(child) = tree.node(child_id) else {
            continue;
        };
        if child.kind != "when" {
            // Never observed in the researched corpus (choose's children
            // are always `when`); reported the same way any other
            // unrecognized-in-this-position element is (design D10).
            activation.diagnose(
                scope,
                Diagnostic::UnrecognizedNode {
                    node_id: child_id,
                    kind: child.kind.clone(),
                },
            );
            report_refs_below(tree, child_id, activation, scope);
            continue;
        }
        if child.is_default {
            // Only the first default is kept, matching the corpus fact
            // that no `choose` in the sample ever has more than one; a
            // second one is not a documented shape and is silently
            // ignored rather than guessed at.
            if default_child.is_none() {
                default_child = Some(child_id);
            }
            continue;
        }
        let Some(raw_test) = child.test.as_deref() else {
            // Neither `@test` nor `@default="true"` — undocumented shape,
            // never observed; treated as never matching, with no
            // diagnostic invented for it.
            continue;
        };
        match Test::parse(raw_test) {
            Ok(test) if test.matches(observed) => {
                selected = Some(child_id);
                break;
            }
            Ok(_) => {}
            Err(UnparsableTest) => activation.diagnose(
                scope,
                Diagnostic::UnparsableTest {
                    when_node: child_id,
                    raw: raw_test.to_string(),
                },
            ),
        }
    }

    match selected.or(default_child) {
        Some(child) => walk(
            trees,
            tree,
            child,
            values,
            activation,
            seen_params,
            seen_coms,
            expansions_used,
            scope,
            channel,
        ),
        None => activation.diagnose(
            scope,
            Diagnostic::NoBranchMatched {
                choose_node: node.node_id,
                param_ref: node.ref_id.clone(),
                observed_value: observed.to_string(),
            },
        ),
    }
}
