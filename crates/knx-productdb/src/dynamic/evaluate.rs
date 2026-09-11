//! Evaluates a stored `Dynamic` tree: given a set of parameter values,
//! which `ParameterRefRef`s and `ComObjectRefRef`s are active, and what
//! could not be decided.
//!
//! This is design D6-D11
//! (`docs/superpowers/specs/2026-09-11-dynamic-tree-parse-and-evaluate-design.md`).
//! `evaluate` itself is a pure function of `(&DynamicTree, &ValueMap)` — no
//! database, no I/O, no logging. `load_tree` and `resolve_values` are the
//! two DB-touching helpers that assemble its inputs; they are kept
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

/// One `dynamic_node` row, loaded for evaluation. A strict subset of what
/// Task 1 stores — `element_id`, `text` and `extra` play no role in
/// evaluation and are left out on purpose, not by oversight.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DynamicNode {
    pub node_id: i64,
    pub parent_id: Option<i64>,
    pub kind: String,
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
        "SELECT node_id, parent_id, kind, ref_id, test, is_default
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
            ))
        })?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    let mut nodes = Vec::with_capacity(rows.len());
    for (node_id, parent_id, kind, ref_id, test, is_default) in rows {
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
            ref_id,
            test,
            is_default: is_default == Some(1),
            control_kind,
        });
    }
    Ok(DynamicTree::from_nodes(nodes))
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

/// Keyed by `ParameterRef` id — the same id `choose/@ParamRefId` and
/// `ParameterRefRef/@RefId` use, and the same one a project's
/// `ParameterInstance` carries (`DATA_MODEL.md` §10). A plain map, not an
/// opaque type, so a unit test can build one with a literal without going
/// through `resolve_values`.
pub type ValueMap = HashMap<String, String>;

/// Assembles a `ValueMap` per design D6's resolution order: `supplied`
/// wins where present; everything else falls back to `parameter_ref.value`,
/// then `parameter.value`. A ref with none of the three is simply absent
/// from the result — `evaluate` turns that absence into `Diagnostic::MissingValue`
/// only for the refs it actually needs during a given walk, rather than
/// pre-flagging every parameter a tree happens to declare, most of which a
/// given evaluation may never reach.
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
    Ok(values)
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
    /// A `Module` node (design D10): recognized, but this slice does not
    /// follow it into the referenced `ModuleDef`'s own tree. How that
    /// expansion would work — naming, argument binding, repetition,
    /// id-mangling, and a required dedup fix — is researched but not yet
    /// implemented; see RESEARCH.md §4.4 (the R4 spike, 2026-09-11).
    ModuleNotExpanded {
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
}

/// The result of evaluating a `Dynamic` tree against a `ValueMap`: which
/// `ParameterRefRef`s and `ComObjectRefRef`s are active, and every
/// diagnostic encountered along the way. Both id lists are in document
/// order, deduplicated by first occurrence (design D11) — an id legitimately
/// reachable through more than one active branch appears once, at its
/// first position.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Activation {
    pub parameter_refs: Vec<String>,
    pub com_object_refs: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl Activation {
    fn activate_parameter_ref(&mut self, seen: &mut HashSet<String>, id: String) {
        if seen.insert(id.clone()) {
            self.parameter_refs.push(id);
        }
    }

    fn activate_com_object_ref(&mut self, seen: &mut HashSet<String>, id: String) {
        if seen.insert(id.clone()) {
            self.com_object_refs.push(id);
        }
    }
}

/// Evaluates `tree` against `values`: a pure function, single-pass,
/// depth-first, document order (design D6). No database, no I/O, no
/// logging — every diagnostic is returned in `Activation::diagnostics`.
pub fn evaluate(tree: &DynamicTree, values: &ValueMap) -> Activation {
    let mut activation = Activation::default();
    let mut seen_params = HashSet::new();
    let mut seen_coms = HashSet::new();
    for &root in tree.roots() {
        walk(
            tree,
            root,
            values,
            &mut activation,
            &mut seen_params,
            &mut seen_coms,
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

#[allow(clippy::too_many_arguments)]
fn walk(
    tree: &DynamicTree,
    node_id: i64,
    values: &ValueMap,
    activation: &mut Activation,
    seen_params: &mut HashSet<String>,
    seen_coms: &mut HashSet<String>,
) {
    let Some(node) = tree.node(node_id) else {
        return;
    };
    if is_transparent_container(&node.kind) {
        for &child in tree.children_of(Some(node_id)) {
            walk(tree, child, values, activation, seen_params, seen_coms);
        }
        return;
    }
    match node.kind.as_str() {
        "choose" => evaluate_choose(tree, node, values, activation, seen_params, seen_coms),
        "ParameterRefRef" => {
            if let Some(id) = &node.ref_id {
                activation.activate_parameter_ref(seen_params, id.clone());
            }
        }
        "ComObjectRefRef" => {
            if let Some(id) = &node.ref_id {
                activation.activate_com_object_ref(seen_coms, id.clone());
            }
        }
        // Recognized and deliberately inert (design D10): presentation
        // (`ParameterSeparator`) and assignment (`Assign`) constructs whose
        // semantics this slice does not model. Neither activates anything
        // nor is expected to have children in the corpus, so there is
        // nothing to descend into.
        "ParameterSeparator" | "Assign" => {}
        "Module" => activation.diagnostics.push(Diagnostic::ModuleNotExpanded {
            node_id,
            ref_id: node.ref_id.clone(),
        }),
        other => activation.diagnostics.push(Diagnostic::UnrecognizedNode {
            node_id,
            kind: other.to_string(),
        }),
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_choose(
    tree: &DynamicTree,
    node: &DynamicNode,
    values: &ValueMap,
    activation: &mut Activation,
    seen_params: &mut HashSet<String>,
    seen_coms: &mut HashSet<String>,
) {
    let Some(control_kind) = node.control_kind else {
        activation.diagnostics.push(Diagnostic::UnresolvedParamRef {
            choose_node: node.node_id,
            param_ref: node.ref_id.clone(),
        });
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
                Some(child) => walk(tree, child, values, activation, seen_params, seen_coms),
                None => activation
                    .diagnostics
                    .push(Diagnostic::UnexpectedTypeNoneShape {
                        choose_node: node.node_id,
                    }),
            }
        }
        ControlKind::Comparable => {
            evaluate_comparable_choose(tree, node, values, activation, seen_params, seen_coms)
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn evaluate_comparable_choose(
    tree: &DynamicTree,
    node: &DynamicNode,
    values: &ValueMap,
    activation: &mut Activation,
    seen_params: &mut HashSet<String>,
    seen_coms: &mut HashSet<String>,
) {
    let param_ref = node.ref_id.clone();
    let Some(raw_value) = node.ref_id.as_deref().and_then(|id| values.get(id)) else {
        activation.diagnostics.push(Diagnostic::MissingValue {
            choose_node: node.node_id,
            param_ref,
        });
        return;
    };
    let Ok(observed) = raw_value.trim().parse::<i64>() else {
        activation.diagnostics.push(Diagnostic::NonNumericValue {
            choose_node: node.node_id,
            param_ref,
            raw: raw_value.to_string(),
        });
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
            activation.diagnostics.push(Diagnostic::UnrecognizedNode {
                node_id: child_id,
                kind: child.kind.clone(),
            });
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
            Err(UnparsableTest) => activation.diagnostics.push(Diagnostic::UnparsableTest {
                when_node: child_id,
                raw: raw_test.to_string(),
            }),
        }
    }

    match selected.or(default_child) {
        Some(child) => walk(tree, child, values, activation, seen_params, seen_coms),
        None => activation.diagnostics.push(Diagnostic::NoBranchMatched {
            choose_node: node.node_id,
            param_ref: node.ref_id.clone(),
            observed_value: observed.to_string(),
        }),
    }
}
