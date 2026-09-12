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
//! Nothing here has been validated against ETS. The no-match policy (D8)
//! and the `@default`-is-fallback reading (below) are inferences from
//! corpus consistency, not documented rules — see RESEARCH.md §4.3's
//! `[D]`/`[V]`/`[A]` markers, preserved here in the same spirit.

use std::collections::{HashMap, HashSet};

use rusqlite::{params, Connection, OptionalExtension};

use crate::ProductDbError;

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

/// One `dynamic_node` row, loaded for evaluation. `text` and `extra` play
/// no role in evaluation and are left out on purpose, not by oversight.
/// `element_id` (`Module/@Id`) is the one field slice 1 deliberately left
/// unloaded because nothing needed it yet; slice 2 needs it to fill
/// `ModuleScope::module_id` (design D14) for human-readable diagnostic and
/// activation reporting — it is never relied on for identity, which is
/// `module_node` (this row's own `node_id`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicNode {
    pub node_id: i64,
    pub parent_id: Option<i64>,
    pub kind: String,
    pub element_id: Option<String>,
    pub ref_id: Option<String>,
    pub test: Option<String>,
    pub is_default: bool,
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
        "SELECT node_id, parent_id, kind, element_id, ref_id, test, is_default
         FROM dynamic_node
         WHERE program_id = ?1 AND module_def_id = ?2
         ORDER BY node_id",
    )?;
    type Row = (
        i64,
        Option<i64>,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<i64>,
    );
    let rows: Vec<Row> = stmt
        .query_map(params![program_id, module_def_id], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
                r.get(6)?,
            ))
        })?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    let mut nodes = Vec::with_capacity(rows.len());
    for (node_id, parent_id, kind, element_id, ref_id, test, is_default) in rows {
        let control_kind = if kind == "choose" {
            match ref_id.as_deref() {
                Some(rid) => resolve_control_kind(conn, program_id, rid)?,
                None => None,
            }
        } else {
            None
        };
        nodes.push(DynamicNode {
            node_id,
            parent_id,
            kind,
            element_id,
            ref_id,
            test,
            is_default: is_default == Some(1),
            control_kind,
        });
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
}

impl ProgramTrees {
    /// Builds a `ProgramTrees` from an already-loaded program tree and its
    /// already-loaded `ModuleDef` trees. The database-touching counterpart
    /// is `load_program_trees`.
    pub fn from_parts(program: DynamicTree, modules: HashMap<String, DynamicTree>) -> ProgramTrees {
        ProgramTrees { program, modules }
    }

    /// A program with no `ModuleDef` trees at all — every existing
    /// hand-built-tree unit test from before this slice becomes
    /// `evaluate(&ProgramTrees::single(tree), &values)`, a mechanical
    /// change (plan Task 1 step 4).
    pub fn single(program: DynamicTree) -> ProgramTrees {
        ProgramTrees {
            program,
            modules: HashMap::new(),
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
    Ok(ProgramTrees::from_parts(program, modules))
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
    /// (design D10). Its subtree is not descended.
    UnrecognizedNode { node_id: i64, kind: String },
    /// Design D17: a `Module/@RefId` is absent, or names a `ModuleDef`
    /// with no stored tree for this program. The `Module` is not
    /// descended — not even its own `NumericArg`/`TextArg` children
    /// (design D19), which are argument bindings, not activations, in
    /// either outcome.
    ModuleDefNotFound {
        node_id: i64,
        ref_id: Option<String>,
    },
    /// Design D15: a `Module` node reached while already walking inside an
    /// expanded `ModuleDef`'s own tree. Nesting is rejected by policy, not
    /// recursed into and not resolved — the corpus has zero nested
    /// `Module` elements and nothing documents a cycle rule to implement
    /// against, so one level of expansion plus this diagnostic is what
    /// stays complete for everything observed and honest about everything
    /// that is not. The subtree is not descended.
    NestedModuleNotExpanded {
        node_id: i64,
        ref_id: Option<String>,
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
    /// `dynamic_node.node_id` of the `Module` element in the program's own
    /// tree. Always present, unique within that tree, and the field the
    /// dedup key and `HashSet`/`HashMap` identity actually use.
    pub module_node: i64,
    /// `Module/@Id`, for human-readable reporting. `None` if the element
    /// carries no `@Id`. Never relied on for identity — `module_node` is.
    pub module_id: Option<String>,
    /// `Module/@RefId`, i.e. the `ModuleDef/@Id` whose tree was walked.
    pub module_def_id: String,
}

/// One activated `ParameterRefRef`/`ComObjectRefRef` id, qualified by
/// where it was found (design D14).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActiveRef {
    pub scope: Option<ModuleScope>,
    pub ref_id: String,
}

/// One `Diagnostic`, qualified by where it was found (design D14).
/// `Diagnostic`'s own variants are unchanged — the scope wraps them, it
/// does not move into them, so a diagnostic's `node_id`/`choose_node`/
/// `when_node` keeps meaning "a `node_id` within the tree named by this
/// `scope`", same contract as slice 1, now with the scope actually present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedDiagnostic {
    pub scope: Option<ModuleScope>,
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
    pub diagnostics: Vec<ScopedDiagnostic>,
}

/// The dedup key design D18 specifies: `module_node` (not the whole
/// `ModuleScope` — `module_id` plays no part in identity) paired with the
/// raw `ref_id`.
type ScopeKey = (Option<i64>, String);

impl Activation {
    fn activate_parameter_ref(
        &mut self,
        seen: &mut HashSet<ScopeKey>,
        scope: Option<&ModuleScope>,
        id: String,
    ) {
        let key = (scope.map(|s| s.module_node), id.clone());
        if seen.insert(key) {
            self.parameter_refs.push(ActiveRef {
                scope: scope.cloned(),
                ref_id: id,
            });
        }
    }

    fn activate_com_object_ref(
        &mut self,
        seen: &mut HashSet<ScopeKey>,
        scope: Option<&ModuleScope>,
        id: String,
    ) {
        let key = (scope.map(|s| s.module_node), id.clone());
        if seen.insert(key) {
            self.com_object_refs.push(ActiveRef {
                scope: scope.cloned(),
                ref_id: id,
            });
        }
    }

    fn diagnose(&mut self, scope: Option<&ModuleScope>, diagnostic: Diagnostic) {
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
/// `Module` (design D19).
pub fn evaluate(trees: &ProgramTrees, values: &ValueMap) -> Activation {
    let mut activation = Activation::default();
    let mut seen_params = HashSet::new();
    let mut seen_coms = HashSet::new();
    for &root in trees.program.roots() {
        walk(
            trees,
            &trees.program,
            root,
            values,
            &mut activation,
            &mut seen_params,
            &mut seen_coms,
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

/// `tree` is the `Dynamic` tree currently being walked — the program's own
/// tree while `scope` is `None`, or the one `ModuleDef` tree `scope` names
/// once a `Module` has been expanded (design D15's one level). `trees` is
/// only consulted at a top-level `Module` node, to resolve its `@RefId`
/// against the program's other stored scopes.
#[allow(clippy::too_many_arguments)]
fn walk(
    trees: &ProgramTrees,
    tree: &DynamicTree,
    node_id: i64,
    values: &ValueMap,
    activation: &mut Activation,
    seen_params: &mut HashSet<ScopeKey>,
    seen_coms: &mut HashSet<ScopeKey>,
    scope: Option<&ModuleScope>,
) {
    let Some(node) = tree.node(node_id) else {
        return;
    };
    if is_transparent_container(&node.kind) {
        for &child in tree.children_of(Some(node_id)) {
            walk(
                trees,
                tree,
                child,
                values,
                activation,
                seen_params,
                seen_coms,
                scope,
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
            scope,
        ),
        "ParameterRefRef" => {
            if let Some(id) = &node.ref_id {
                activation.activate_parameter_ref(seen_params, scope, id.clone());
            }
        }
        "ComObjectRefRef" => {
            if let Some(id) = &node.ref_id {
                activation.activate_com_object_ref(seen_coms, scope, id.clone());
            }
        }
        // Recognized and deliberately inert (design D10): presentation
        // (`ParameterSeparator`) and assignment (`Assign`) constructs whose
        // semantics this slice does not model. Neither activates anything
        // nor is expected to have children in the corpus, so there is
        // nothing to descend into.
        "ParameterSeparator" | "Assign" => {}
        // Design D19: `Module` is dispatched here like any other node kind,
        // no special container handling. Its own children (`NumericArg`/
        // `TextArg`) are argument bindings, not activations, and are never
        // descended into, in either branch below.
        "Module" => match scope {
            // Design D15: already inside an expanded `ModuleDef`'s tree —
            // this `Module` is not followed, regardless of whether its
            // `@RefId` would otherwise resolve.
            Some(current) => activation.diagnose(
                Some(current),
                Diagnostic::NestedModuleNotExpanded {
                    node_id,
                    ref_id: node.ref_id.clone(),
                },
            ),
            None => {
                let hit = node
                    .ref_id
                    .as_deref()
                    .and_then(|rid| trees.module(rid).map(|t| (rid.to_string(), t)));
                match hit {
                    Some((module_def_id, module_tree)) => {
                        // Design D37: a nameless instantiation can never be
                        // matched to a project-side `ModuleInstance`, so it
                        // is worth saying out loud even when — as here —
                        // nothing downstream has asked for its value yet.
                        // Emitted once per instantiation, unconditionally,
                        // before descending; the scope it would name is
                        // the one that has no name.
                        if node.element_id.is_none() {
                            activation.diagnose(None, Diagnostic::ModuleWithoutId { node_id });
                        }
                        let new_scope = ModuleScope {
                            module_node: node_id,
                            module_id: node.element_id.clone(),
                            module_def_id,
                        };
                        for &root in module_tree.roots() {
                            walk(
                                trees,
                                module_tree,
                                root,
                                values,
                                activation,
                                seen_params,
                                seen_coms,
                                Some(&new_scope),
                            );
                        }
                    }
                    None => activation.diagnose(
                        None,
                        Diagnostic::ModuleDefNotFound {
                            node_id,
                            ref_id: node.ref_id.clone(),
                        },
                    ),
                }
            }
        },
        other => activation.diagnose(
            scope,
            Diagnostic::UnrecognizedNode {
                node_id,
                kind: other.to_string(),
            },
        ),
    }
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
    scope: Option<&ModuleScope>,
) {
    let Some(control_kind) = node.control_kind else {
        activation.diagnose(
            scope,
            Diagnostic::UnresolvedParamRef {
                choose_node: node.node_id,
                param_ref: node.ref_id.clone(),
            },
        );
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
                    scope,
                ),
                None => activation.diagnose(
                    scope,
                    Diagnostic::UnexpectedTypeNoneShape {
                        choose_node: node.node_id,
                    },
                ),
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
            scope,
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
    scope: Option<&ModuleScope>,
) {
    let param_ref = node.ref_id.clone();
    let Some(raw_value) = node.ref_id.as_deref().and_then(|id| values.get(scope, id)) else {
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
            scope,
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
