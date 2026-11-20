//! Read-only parameter groups, grids and object controls from product data.
use crate::ProductDbError;
use quick_xml::{events::Event, name::ResolveResult, NsReader};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet, HashMap};

mod button;
mod grid;
mod nested;
pub use button::{ButtonBinding, ButtonLiteral};
pub(crate) use grid::{layout_attribute_is_known, layout_element_is_known};
pub use grid::{BlockGrid, Cell, GridAxis, GridIssue, GridIssueKind, GridStatus, MAX_GRID_EVENTS};
pub use nested::{NestedBinding, NestedBlockCell};

/// Exact program ownership shared by every presentation reader of the source.
mod ownership {
    const PROGRAMS: [&str; 4] = [
        "KNX",
        "ManufacturerData",
        "Manufacturer",
        "ApplicationPrograms",
    ];

    pub(super) fn is_program_owner(path: &[&str]) -> bool {
        path == PROGRAMS
    }

    /// The program's own `Dynamic` or one of its `ModuleDef`s'.
    pub(super) fn is_dynamic_root(kind: &str, path: &[&str]) -> bool {
        kind == "Dynamic"
            && (path.strip_suffix(&["ApplicationProgram"]) == Some(&PROGRAMS)
                // WP-05h: a `ModuleDef` may sit under `Static` (or directly under
                // the program); both are module tree roots.
                || path.strip_suffix(&["Static", "ModuleDefs", "ModuleDef"])
                    == Some(&[PROGRAMS.as_slice(), &["ApplicationProgram"]].concat())
                || path.strip_suffix(&["ApplicationProgram", "ModuleDefs", "ModuleDef"])
                    == Some(&PROGRAMS))
    }

    pub(super) fn is_dynamic_container(kind: &str) -> bool {
        matches!(
            kind,
            "Channel" | "ChannelIndependentBlock" | "ParameterBlock" | "choose" | "when" | "Repeat"
        )
    }
}
use ownership::{is_dynamic_container, is_dynamic_root, is_program_owner};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub node_id: i64,
    pub element_id: Option<String>,
    pub text: Option<String>,
    pub caption_ref: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct FieldLayout {
    pub group: Option<Group>,
    pub controls_com_objects: bool,
    /// Declared position in the group's grid; only for a `GridStatus::Grid`.
    pub cell: Option<Cell>,
    /// Declared references reachable through structural choice dependencies.
    pub controlled_object_refs: Vec<String>,
    /// `ParameterRefRef/@IndentLevel` (census P4), only when the reference has
    /// one group and every occurrence there declares the same valid level.
    /// Presentation only.
    pub indent_level: Option<u8>,
}
/// An activated separator's declared position, for grid captions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SeparatorPlacement {
    pub block_node: i64,
    pub cell: Cell,
}
/// What a separator declares for the row view (ADR-0139), verbatim.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SeparatorDeclaration {
    /// The nearest enclosing `ParameterBlock`, as for fields.
    pub group: Option<Group>,
    pub element_id: Option<String>,
    pub ui_hint: Option<String>,
    /// The separator's own `Access`; enclosing blocks are the evaluator's.
    pub access: Option<String>,
}
#[derive(Debug, Default)]
pub struct ParameterLayout {
    fields: HashMap<(String, String), FieldLayout>,
    grids: HashMap<(String, i64), BlockGrid>,
    separators: HashMap<(String, i64), SeparatorPlacement>,
    declarations: HashMap<(String, i64), SeparatorDeclaration>,
    /// `(module_def_id, node)` → literal; module trees bind too (WP-05h).
    buttons: HashMap<(String, i64), ButtonLiteral>,
    /// `nested-block-cell-passive-v1`: program-owned nested blocks placed in
    /// their parent grid, bound by structure.
    nested_cells: HashMap<i64, NestedBlockCell>,
    nested_binding: NestedBinding,
    button_binding: ButtonBinding,
    axis_program: String,
    /// `(module_def_id, axis @Id)` of every bound literal grid axis;
    /// `""` is the program's own tree (WP-05n: module/Repeat-scoped grids
    /// bind per scope).
    literal_grid_axes: std::collections::HashSet<(String, String)>,
    /// `inline-section-flatten-v1`: `(definition, inline block node)` → the
    /// outer group its heading renders in.
    inline_label_groups: HashMap<(String, i64), Group>,
}
impl ParameterLayout {
    /// The bound literal axis ids of `definition`'s tree (`""` = the
    /// program's own tree), when this layout was loaded for `program`.
    pub fn literal_grid_axes<'a>(
        &'a self,
        program: &str,
        definition: &'a str,
    ) -> Option<impl Iterator<Item = &'a str> + use<'a>> {
        (self.axis_program == program).then(|| {
            self.literal_grid_axes
                .iter()
                .filter(move |(scope, _)| scope == definition)
                .map(|(_, id)| id.as_str())
        })
    }
    /// `button-literal-passive-v1`: the admitted literal of the `Button` at
    /// `node` in `definition`'s tree; since WP-05h module trees bind too.
    pub fn button(&self, definition: &str, node: i64) -> Option<&ButtonLiteral> {
        self.buttons.get(&(definition.to_owned(), node))
    }
    /// Whether the program's `Button` literals could be bound to stored nodes at all.
    /// `nested-block-cell-passive-v1`: the admitted cell of the program-owned
    /// nested block `node`, when its parent grid can render it.
    pub fn nested_cell(&self, definition: &str, node: i64) -> Option<&NestedBlockCell> {
        if !definition.is_empty() {
            return None;
        }
        self.nested_cells.get(&node)
    }

    pub fn nested_binding(&self) -> NestedBinding {
        self.nested_binding
    }

    /// `inline-section-flatten-v1`: the outer group an activated `Inline`
    /// block's heading renders in, if the walk found one.
    pub fn inline_label_group(&self, definition: &str, node: i64) -> Option<&Group> {
        self.inline_label_groups.get(&(definition.to_owned(), node))
    }

    pub fn button_binding(&self) -> ButtonBinding {
        self.button_binding
    }
    pub fn field(&self, definition: &str, reference: &str) -> FieldLayout {
        self.fields
            .get(&(definition.to_owned(), reference.to_owned()))
            .cloned()
            .unwrap_or_default()
    }
    /// The grid declaration of `block_node` in `definition`'s tree, if any.
    pub fn grid(&self, definition: &str, block_node: i64) -> Option<&BlockGrid> {
        self.grids.get(&(definition.to_owned(), block_node))
    }

    /// ADR-0177: every grid declared in the program's own tree, for resolving
    /// axis `TextParameterRefId` captions.
    pub fn program_grids(&self) -> impl Iterator<Item = &BlockGrid> {
        self.grids_for("")
    }

    /// WP-05n: every grid declared in `definition`'s tree (`""` = the
    /// program's own tree).
    pub fn grids_for<'a>(
        &'a self,
        definition: &'a str,
    ) -> impl Iterator<Item = &'a BlockGrid> + use<'a> {
        self.grids
            .iter()
            .filter(move |((scope, _), _)| scope == definition)
            .map(|(_, grid)| grid)
    }
    pub fn separator(&self, definition: &str, node: i64) -> Option<&SeparatorPlacement> {
        self.separators.get(&(definition.to_owned(), node))
    }
    /// Every separator of `definition`'s tree, placed in a grid or not.
    pub fn separator_declaration(
        &self,
        definition: &str,
        node: i64,
    ) -> Option<&SeparatorDeclaration> {
        self.declarations.get(&(definition.to_owned(), node))
    }
}
#[derive(Debug)]
struct Node {
    parent: Option<i64>,
    /// Sibling position below `parent`, as the parser numbered it.
    position: i64,
    kind: String,
    reference: Option<String>,
    id: Option<String>,
    text: Option<String>,
    access: Option<String>,
    ui_hint: Option<String>,
    /// `ParameterBlock/@Inline = true/1` (v34, inline-section-flatten-v1).
    inline: bool,
}

/// Uses normalized parent relationships, never names, numbers or manufacturer-specific IDs.
/// A reference appearing in several blocks keeps no claimed single group.
pub fn load(conn: &Connection, program: &str) -> Result<ParameterLayout, ProductDbError> {
    let source = retained_source(conn, program)?;
    let captions = source
        .as_deref()
        .and_then(|bytes| read_captions(bytes, program))
        .unwrap_or_default();
    // `None`: unreadable, so a structural grid is reported unavailable.
    let grids = source
        .as_deref()
        .and_then(|bytes| grid::read_grids(bytes, program));
    let mut stmt = conn.prepare("SELECT module_def_id,node_id,parent_id,kind,ref_id,element_id,text,access,ui_hint,position,COALESCE(inline,0) FROM dynamic_node WHERE program_id=?1 ORDER BY module_def_id,node_id")?;
    let rows = stmt.query_map([program], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            Node {
                parent: r.get(2)?,
                position: r.get(9)?,
                kind: r.get(3)?,
                reference: r.get(4)?,
                id: r.get(5)?,
                text: r.get(6)?,
                access: r.get(7)?,
                ui_hint: r.get(8)?,
                inline: r.get::<_, i64>(10)? == 1,
            },
        ))
    })?;
    // WP-05h: renumber each scope's tree from 0 (per-scope ids, matching
    // button::read), keeping parent relationships within the scope.
    let mut trees: BTreeMap<String, BTreeMap<i64, Node>> = BTreeMap::new();
    {
        let mut remaps: HashMap<String, HashMap<i64, i64>> = HashMap::new();
        let mut raw: BTreeMap<String, Vec<(i64, Node)>> = BTreeMap::new();
        for row in rows {
            let (scope, id, node) = row?;
            raw.entry(scope.clone()).or_default().push((id, node));
            let remap = remaps.entry(scope).or_default();
            remap.insert(id, remap.len() as i64 - 1 + 1); // placeholder, fixed below
        }
        for (scope, entries) in raw {
            let remap = remaps.get_mut(&scope).unwrap();
            remap.clear();
            for (index, (old_id, _)) in entries.iter().enumerate() {
                remap.insert(*old_id, index as i64);
            }
            let tree: BTreeMap<i64, Node> = entries
                .into_iter()
                .map(|(old_id, node)| {
                    let new_id = remap[&old_id];
                    let node = Node {
                        parent: node.parent.and_then(|p| remap.get(&p).copied()),
                        ..node
                    };
                    (new_id, node)
                })
                .collect();
            trees.insert(scope, tree);
        }
    }
    let mut result = ParameterLayout {
        button_binding: ButtonBinding::Bound,
        nested_binding: NestedBinding::Bound,
        axis_program: program.to_owned(),
        ..ParameterLayout::default()
    };
    for (scope, nodes) in &trees {
        let mut structural = BTreeSet::new();
        for node in nodes.values() {
            if matches!(node.kind.as_str(), "Rows" | "Columns") {
                structural.extend(node.parent);
            }
        }
        let mut scope_grids: HashMap<i64, BlockGrid> = HashMap::new();
        for (id, node) in nodes.iter() {
            if node.kind != "ParameterBlock" {
                continue;
            }
            let declared = match (&grids, &node.id) {
                (Some(grids), Some(element)) => grids.blocks.get(element).cloned(),
                _ => None,
            };
            if let Some(grid) =
                declared.or_else(|| structural.contains(id).then(BlockGrid::unavailable))
            {
                scope_grids.insert(*id, grid);
            }
        }
        // The group's block `@Id`, only when that block is a renderable grid.
        let placed_in = |group: &Option<Group>| -> Option<String> {
            let group = group.as_ref()?;
            let is_grid = scope_grids
                .get(&group.node_id)
                .is_some_and(|grid| grid.status == GridStatus::Grid);
            is_grid.then(|| group.element_id.clone()).flatten()
        };
        let mut separators = Vec::new();
        let mut targets: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        let mut dependencies: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut groups: HashMap<String, Option<Group>> = HashMap::new();
        // inline-section-flatten-v1: an `Inline` block's heading renders in the
        // group of its nearest non-inline block ancestor. Recorded for every
        // inline block, before the field walk below skips non-field kinds.
        for (&node_id, node) in nodes.iter() {
            if node.kind == "ParameterBlock" && node.inline {
                let mut outer = None;
                let mut parent = node.parent;
                let mut seen = BTreeSet::new();
                while let Some(at) = parent {
                    if !seen.insert(at) || seen.len() > 64 {
                        break;
                    }
                    let Some(ancestor) = nodes.get(&at) else {
                        break;
                    };
                    if ancestor.kind == "ParameterBlock" && !ancestor.inline {
                        outer = Some(Group {
                            node_id: at,
                            element_id: ancestor.id.clone(),
                            text: ancestor.text.clone(),
                            caption_ref: ancestor
                                .id
                                .as_ref()
                                .and_then(|id| captions.get(id))
                                .cloned(),
                        });
                        break;
                    }
                    parent = ancestor.parent;
                }
                if let Some(group) = outer {
                    result
                        .inline_label_groups
                        .insert((scope.clone(), node_id), group);
                }
            }
        }
        for (&node_id, node) in nodes.iter() {
            if !matches!(
                node.kind.as_str(),
                "ParameterRefRef" | "ComObjectRefRef" | "ParameterSeparator"
            ) {
                continue;
            }
            let mut parents = BTreeSet::new();
            let mut group = None;
            let mut seen = BTreeSet::new();
            let mut parent = node.parent;
            while let Some(at) = parent {
                if !seen.insert(at) || seen.len() > 64 {
                    break;
                }
                let Some(ancestor) = nodes.get(&at) else {
                    break;
                };
                if ancestor.kind == "choose" {
                    if let Some(reference) = &ancestor.reference {
                        parents.insert(reference.clone());
                    }
                }
                // inline-section-flatten-v1: an `Inline` block renders inside
                // its parent page, so it is not the group owner below it.
                if group.is_none() && ancestor.kind == "ParameterBlock" && !ancestor.inline {
                    group = Some(Group {
                        node_id: at,
                        element_id: ancestor.id.clone(),
                        text: ancestor.text.clone(),
                        caption_ref: ancestor
                            .id
                            .as_ref()
                            .and_then(|id| captions.get(id))
                            .cloned(),
                    });
                }
                parent = ancestor.parent;
            }
            if node.kind == "ComObjectRefRef" {
                if let Some(reference) = &node.reference {
                    for parent in parents {
                        targets.entry(parent).or_default().insert(reference.clone());
                    }
                }
            } else if node.kind == "ParameterSeparator" {
                result.declarations.insert(
                    (scope.clone(), node_id),
                    SeparatorDeclaration {
                        group: group.clone(),
                        element_id: node.id.clone(),
                        ui_hint: node.ui_hint.clone(),
                        access: node.access.clone(),
                    },
                );
                separators.push((node_id, node.id.clone(), group));
            } else if let Some(reference) = &node.reference {
                dependencies
                    .entry(reference.clone())
                    .or_default()
                    .extend(parents);
                groups
                    .entry(reference.clone())
                    .and_modify(|previous| {
                        if *previous != group {
                            *previous = None;
                        }
                    })
                    .or_insert(group);
            }
        }
        // A mode can gate a parameter that gates objects elsewhere. Follow
        // those declaration dependencies, without evaluating a second tree.
        let mut pending = targets.keys().cloned().collect::<Vec<_>>();
        while let Some(reference) = pending.pop() {
            if let Some(parents) = dependencies.get(&reference) {
                let objects = targets.get(&reference).cloned().unwrap_or_default();
                for parent in parents {
                    let inherited = targets.entry(parent.clone()).or_default();
                    let before = inherited.len();
                    inherited.extend(objects.iter().cloned());
                    if inherited.len() != before {
                        pending.push(parent.clone());
                    }
                }
            }
        }
        for (node_id, element, group) in separators {
            let (Some(block), Some(element), Some(grids)) = (placed_in(&group), element, &grids)
            else {
                continue;
            };
            if let Some((owner, cell)) = grids.separators.get(&element) {
                if *owner == block {
                    let block_node = group.as_ref().map_or(0, |group| group.node_id);
                    let placement = SeparatorPlacement {
                        block_node,
                        cell: *cell,
                    };
                    result
                        .separators
                        .insert((scope.clone(), node_id), placement);
                }
            }
        }
        for (reference, group) in groups {
            let controlled_object_refs: Vec<_> = targets
                .remove(&reference)
                .unwrap_or_default()
                .into_iter()
                .collect();
            let controls_com_objects = !controlled_object_refs.is_empty();
            let cell = match (placed_in(&group), &grids) {
                (Some(block), Some(grids)) => {
                    grids.references.get(&(block, reference.clone())).copied()
                }
                _ => None,
            };
            let indent_level = match (&group, &grids) {
                (
                    Some(Group {
                        element_id: Some(block),
                        ..
                    }),
                    Some(grids),
                ) => grids
                    .indents
                    .get(&(block.clone(), reference.clone()))
                    .copied(),
                _ => None,
            };
            result.fields.insert(
                (scope.clone(), reference),
                FieldLayout {
                    group,
                    controls_com_objects,
                    cell,
                    controlled_object_refs,
                    indent_level,
                },
            );
        }
        if let Some(source_grids) = &grids {
            // WP-05n: every scope's grid axes bind against its own stored
            // per-scope tree; a scope whose source/stored correspondence
            // fails contributes no axes, never another scope's.
            let bound = grid::scoped_axis_caption_ids(source_grids, scope, nodes);
            let axes: std::collections::HashSet<String> = scope_grids
                .values()
                .filter(|grid| grid.status == GridStatus::Grid)
                .flat_map(|grid| grid.rows.iter().chain(&grid.columns))
                // WP-05n: an axis declaring `TextParameterRefId` is a
                // parameter-driven caption host (ADR-0177), never an axis
                // translation candidate — in any scope.
                .filter(|axis| axis.text_parameter_ref_id.is_none())
                .filter(|axis| bound.as_ref().is_some_and(|bound| bound.contains(&axis.id)))
                .map(|axis| axis.id.clone())
                .collect();
            for id in axes {
                result.literal_grid_axes.insert((scope.clone(), id));
            }
        }
        // button-literal-passive-v1 / module-scoped-buttons-v1 (WP-05h):
        // every tree binds its own Buttons; a refusal in any tree refuses
        // the whole program's labels, never a subset.
        // WP-05h: the bind walk checks every scope's nearest block, so
        // the grid lookup must cover all trees, not only this scope's.
        // Keyed by (scope, node) so per-scope ids never collide.
        // The stored per-scope tree is renumbered from 0, but the source
        // grid map is keyed by element `@Id`; look blocks up by `@Id` and
        // re-key with this scope's renumbered node id.
        // Merge every scope's structural fallback (unavailable grids for
        // blocks with stored axes) with the source-declared grids.
        let mut all_grids: HashMap<(String, i64), BlockGrid> = trees
            .iter()
            .flat_map(|(tree_scope, tree)| {
                tree.iter()
                    .filter(|(_, n)| n.kind == "ParameterBlock")
                    .filter_map(|(id, n)| {
                        grids
                            .as_ref()
                            .and_then(|g| n.id.as_ref().and_then(|e| g.blocks.get(e)))
                            .map(|grid| ((tree_scope.clone(), *id), grid.clone()))
                    })
            })
            .collect();
        for (tree_scope, tree) in trees.iter() {
            let mut structural = BTreeSet::new();
            for node in tree.values() {
                if matches!(node.kind.as_str(), "Rows" | "Columns") {
                    structural.extend(node.parent);
                }
            }
            for (id, node) in tree.iter() {
                if node.kind == "ParameterBlock"
                    && !all_grids.contains_key(&(tree_scope.clone(), *id))
                    && structural.contains(id)
                {
                    all_grids.insert((tree_scope.clone(), *id), BlockGrid::unavailable());
                }
            }
        }
        let (binding, buttons) = button::bind(
            source.as_deref(),
            program,
            Some(nodes),
            &all_grids,
            &captions,
            scope.as_str(),
        );
        if binding != ButtonBinding::Bound && result.button_binding == ButtonBinding::Bound {
            result.button_binding = binding;
        }
        for (node, literal) in buttons {
            result.buttons.insert((scope.clone(), node), literal);
        }
        // nested-block-cell-passive-v1: program-owned nested blocks only.
        if scope.is_empty() {
            let (nested_binding, nested_cells) = nested::bind(
                source.as_deref(),
                program,
                Some(nodes),
                &scope_grids,
                &captions,
            );
            result.nested_binding = nested_binding;
            result.nested_cells = nested_cells;
        }
        for (id, grid) in scope_grids {
            result.grids.insert((scope.clone(), id), grid);
        }
    }
    Ok(result)
}

// Older catalogues did not model ParameterBlock/@ParamRefId, and `Layout`,
// `Cell` and `Width` were never typed. Read this presentation-only metadata
// from the retained, hash-checked source rather than reparsing
// dynamic_node.extra (an audit string, not an interchange format).
fn retained_source(conn: &Connection, program: &str) -> Result<Option<Vec<u8>>, ProductDbError> {
    let row:Option<(String,i64,i64)>=conn.query_row("SELECT s.sha256,s.len,length(s.bytes) FROM application_program a JOIN source_file s ON s.sha256=a.source_sha256 WHERE a.id=?1",[program],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
    let Some((sha, declared, actual)) = row else {
        return Ok(None);
    };
    if actual < 0
        || actual > crate::parameter_editor::MAX_EDITOR_SOURCE_BYTES as i64
        || actual != declared
    {
        return Ok(None);
    }

    let bytes: Vec<u8> = conn.query_row(
        "SELECT bytes FROM source_file WHERE sha256=?1",
        [&sha],
        |r| r.get(0),
    )?;
    Ok((crate::sha256_hex(&bytes) == sha).then_some(bytes))
}
fn read_captions(bytes: &[u8], program: &str) -> Option<HashMap<String, String>> {
    let mut reader = NsReader::from_reader(bytes);
    // Element name, exact selected program ownership, admitted Dynamic ancestry.
    let mut stack: Vec<(String, bool, bool)> = Vec::new();
    let mut result = HashMap::new();
    let mut namespace = None;
    let mut owners = 0;
    let mut seen = BTreeSet::new();
    for _ in 0..100_000 {
        let (ns, event) = reader.read_resolved_event().ok()?;
        let empty = matches!(&event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let resolved = match ns {
                    ResolveResult::Bound(ns) => Some(ns.as_ref().to_owned()),
                    _ => None,
                };
                let kind = e.local_name().as_ref().to_owned();
                if stack.is_empty() {
                    if kind != "KNX" {
                        return None;
                    }
                    namespace = resolved.clone();
                }
                let same = resolved == namespace;
                let path: Vec<&str> = stack.iter().map(|(name, _, _)| name.as_str()).collect();
                let attr = crate::xml::attrs(&e, "parameter presentation").ok()?;
                let owner = same
                    && kind == "ApplicationProgram"
                    && is_program_owner(&path)
                    && attr.evidence_value("Id") == Some(program);
                if owner {
                    owners += 1;
                }
                let active = same && (owner || stack.last().is_some_and(|(_, active, _)| *active));
                let kind_text = kind.as_str();
                let inside_dynamic = active
                    && (is_dynamic_root(kind_text, &path)
                        || (stack.last().is_some_and(|(_, _, dynamic)| *dynamic)
                            && is_dynamic_container(kind_text)));
                if inside_dynamic && kind == "ParameterBlock" {
                    // A prefixed extension attribute is not the source declaration.
                    if let (Some(id), Some(reference)) =
                        (attr.evidence_value("Id"), attr.evidence_value("ParamRefId"))
                    {
                        if !seen.insert(id.to_owned()) {
                            return None;
                        }
                        result.insert(id.to_owned(), reference.to_owned());
                    }
                }
                if !empty {
                    stack.push((kind, active, inside_dynamic));
                    if stack.len() > 64 {
                        return None;
                    }
                }
            }
            Event::End(_) => {
                stack.pop()?;
            }
            Event::DocType(_) => return None,
            Event::Eof => return (stack.is_empty() && owners == 1).then_some(result),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captions_require_the_exact_owner_and_do_not_accept_duplicate_or_foreign_blocks() {
        let xml=br#"<KNX xmlns="http://knx.org/xml/project/23"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A"><Dynamic><ParameterBlock Id="B" ParamRefId="P"/><x:ParameterBlock xmlns:x="urn:foreign" Id="X" ParamRefId="Q"/></Dynamic></ApplicationProgram><ApplicationProgram Id="Other"><Dynamic><ParameterBlock Id="B" ParamRefId="Other"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
        assert_eq!(
            read_captions(xml, "A").unwrap(),
            HashMap::from([("B".into(), "P".into())])
        );
        assert!(read_captions(xml, "missing").is_none());
        assert!(read_captions(br#"<!DOCTYPE KNX><KNX/>"#, "A").is_none());
    }
}
