//! Read-only parameter groups, grids and object controls from product data.
use crate::ProductDbError;
use quick_xml::{events::Event, name::ResolveResult, NsReader};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet, HashMap};

mod grid;
pub use grid::{BlockGrid, Cell, GridAxis, GridIssue, GridIssueKind, GridStatus, MAX_GRID_EVENTS};

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
}
impl ParameterLayout {
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
    kind: String,
    reference: Option<String>,
    id: Option<String>,
    text: Option<String>,
    access: Option<String>,
    ui_hint: Option<String>,
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
    let mut stmt = conn.prepare("SELECT module_def_id,node_id,parent_id,kind,ref_id,element_id,text,access,ui_hint FROM dynamic_node WHERE program_id=?1 ORDER BY module_def_id,node_id")?;
    let rows = stmt.query_map([program], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            Node {
                parent: r.get(2)?,
                kind: r.get(3)?,
                reference: r.get(4)?,
                id: r.get(5)?,
                text: r.get(6)?,
                access: r.get(7)?,
                ui_hint: r.get(8)?,
            },
        ))
    })?;
    let mut trees: BTreeMap<String, BTreeMap<i64, Node>> = BTreeMap::new();
    for row in rows {
        let (scope, id, node) = row?;
        trees.entry(scope).or_default().insert(id, node);
    }
    let mut result = ParameterLayout::default();
    for (scope, nodes) in trees {
        let mut structural = BTreeSet::new();
        for node in nodes.values() {
            if matches!(node.kind.as_str(), "Rows" | "Columns") {
                structural.extend(node.parent);
            }
        }
        let mut scope_grids: HashMap<i64, BlockGrid> = HashMap::new();
        for (id, node) in &nodes {
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
        for (&node_id, node) in &nodes {
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
                if group.is_none() && ancestor.kind == "ParameterBlock" {
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
            result.fields.insert(
                (scope.clone(), reference),
                FieldLayout {
                    group,
                    controls_com_objects,
                    cell,
                    controlled_object_refs,
                },
            );
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
