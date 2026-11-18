//! `nested-block-cell-passive-v1`: a program-owned `ParameterBlock` placed in
//! its parent grid by its own `@Cell` shows its literal `@Text` as a passive
//! cell caption, exactly like a `Button` literal. Display only: the nested
//! block's own contents stay in the row view, and nothing here is a write
//! target or an editor.

use std::collections::{BTreeMap, HashMap, HashSet};

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;

use super::grid::{parse_cell, BlockGrid, GridStatus, MAX_GRID_EVENTS};
use super::ownership::{is_dynamic_container, is_dynamic_root, is_program_owner};
use super::Group;
use crate::parameter_layout::Node;

/// Why no nested-block cell could be bound. A refusal binds no cell at all,
/// never a partial set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NestedBinding {
    #[default]
    Bound,
    SourceUnavailable,
    StructuralDrift,
    /// Two program-owned nested `ParameterBlock`s declare the same `@Id`.
    DuplicateBlockId,
}

/// One admitted nested block placed in its parent's grid.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NestedBlockCell {
    /// The parent grid block, a renderable grid.
    pub group: Group,
    pub cell: super::grid::Cell,
    /// The nested block's `@Id`, verbatim from the source.
    pub element_id: String,
    /// `@Text`, XML-decoded, when it is a displayable literal; `None` when
    /// absent, blank, `#` or placeholder-carrying (the cell then shows no
    /// caption, exactly as an absent `Button/@Text` would).
    pub text: Option<String>,
}

/// The attributes this form admits for the placement itself. A nested block
/// may of course also declare `Text`, `Layout` and its own `Rows`/`Columns`;
/// those describe the block's own children and never place it, so they stay
/// unadmitted here and the block then binds no cell.
const REQUIRED: [&str; 2] = ["Id", "Cell"];

/// One admitted source candidate, before it is bound to stored nodes.
struct Candidate {
    node: i64,
    element_id: String,
    text: Option<String>,
    cell: String,
    block_node: i64,
    block_id: String,
}

/// The program's own `Dynamic` tree as the parser numbered it, plus candidates.
struct SourceTree {
    /// `(kind, parent, position)` indexed by node id.
    nodes: Vec<(String, Option<i64>, i64)>,
    candidates: Vec<Candidate>,
}

struct Entry {
    kind: String,
    active: bool,
    dynamic: bool,
    /// Node id in the program's tree, when inside it.
    node: Option<i64>,
    /// Next child's position below `node`.
    children: i64,
    /// Nearest enclosing exact-namespace `Dynamic` `ParameterBlock`.
    block: Option<(i64, Option<String>)>,
}

const MAX_DEPTH: usize = 64;

/// Binds every admitted program-owned nested cell of `program`, or none.
pub(super) fn bind(
    bytes: Option<&[u8]>,
    program: &str,
    stored: Option<&BTreeMap<i64, Node>>,
    grids: &HashMap<i64, BlockGrid>,
    captions: &HashMap<String, String>,
) -> (NestedBinding, HashMap<i64, NestedBlockCell>) {
    // Nothing to bind: no stored nested `ParameterBlock` with a grid parent.
    if !stored.is_some_and(|nodes| nodes.values().any(|node| node.kind == "ParameterBlock")) {
        return (NestedBinding::Bound, HashMap::new());
    }
    if stored.is_some_and(|nodes| !any_nested_may_have_a_grid(nodes, grids)) {
        return (NestedBinding::Bound, HashMap::new());
    }
    let Some(bytes) = bytes else {
        return (NestedBinding::SourceUnavailable, HashMap::new());
    };
    match read(bytes, program, MAX_GRID_EVENTS)
        .and_then(|source| bind_source(source, stored.unwrap_or(&BTreeMap::new()), grids, captions))
    {
        Ok(cells) => (NestedBinding::Bound, cells),
        Err(refusal) => (refusal, HashMap::new()),
    }
}

/// The nearest stored `ParameterBlock` above `node`, stopping at a cycle or
/// a missing ancestor.
fn nearest_block(stored: &BTreeMap<i64, Node>, node: i64) -> Option<(i64, &Node)> {
    let mut seen = HashSet::new();
    let mut parent = stored.get(&node).and_then(|node| node.parent);
    while let Some(at) = parent {
        let ancestor = stored.get(&at).filter(|_| seen.insert(at))?;
        if ancestor.kind == "ParameterBlock" {
            return Some((at, ancestor));
        }
        parent = ancestor.parent;
    }
    None
}

/// Whether some stored `ParameterBlock` has a stored `ParameterBlock` parent
/// whose grid could render a cell (or whose source could not be read).
fn any_nested_may_have_a_grid(
    stored: &BTreeMap<i64, Node>,
    grids: &HashMap<i64, BlockGrid>,
) -> bool {
    stored
        .iter()
        .filter(|(_, node)| node.kind == "ParameterBlock")
        .filter_map(|(id, _)| nearest_block(stored, *id))
        .any(|(block, _)| {
            grids.get(&block).is_some_and(|grid| {
                matches!(
                    grid.status,
                    GridStatus::Grid | GridStatus::SourceUnavailable
                )
            })
        })
}

fn bind_source(
    source: SourceTree,
    stored: &BTreeMap<i64, Node>,
    grids: &HashMap<i64, BlockGrid>,
    captions: &HashMap<String, String>,
) -> Result<HashMap<i64, NestedBlockCell>, NestedBinding> {
    // Complete structural correspondence, every node and none extra.
    if stored.len() != source.nodes.len() {
        return Err(NestedBinding::StructuralDrift);
    }
    for ((id, node), (index, (kind, parent, position))) in
        stored.iter().zip(source.nodes.iter().enumerate())
    {
        if *id != index as i64
            || node.kind != *kind
            || node.parent != *parent
            || node.position != *position
        {
            return Err(NestedBinding::StructuralDrift);
        }
    }
    let mut cells = HashMap::new();
    for candidate in source.candidates {
        // The candidate's own parent block must be the same stored node with
        // the same `@Id`.
        let Some((block_node, block)) = nearest_block(stored, candidate.node) else {
            continue;
        };
        if block_node != candidate.block_node || block.id.as_deref() != Some(&candidate.block_id) {
            return Err(NestedBinding::StructuralDrift);
        }
        let Some(grid) = grids
            .get(&block_node)
            .filter(|grid| grid.status == GridStatus::Grid)
        else {
            continue;
        };
        let Ok(cell) = parse_cell(&candidate.cell, grid.rows.len(), grid.columns.len()) else {
            continue;
        };
        cells.insert(
            candidate.node,
            NestedBlockCell {
                group: Group {
                    node_id: block_node,
                    element_id: block.id.clone(),
                    text: block.text.clone(),
                    caption_ref: captions.get(&candidate.block_id).cloned(),
                },
                cell,
                element_id: candidate.element_id,
                text: candidate.text,
            },
        );
    }
    Ok(cells)
}

/// Text this literal form shows: present, not blank, not `#`, no placeholder delimiter.
fn literal_text(text: &str) -> Option<String> {
    let trimmed = text.trim();
    (trimmed.is_empty() || trimmed == "#" || text.contains("{{") || text.contains("}}"))
        .then_some(())
        .is_none()
        .then(|| text.to_owned())
}

/// The admitted optional caption attribute; namespace declarations are not behaviour.
const OPTIONAL: [&str; 1] = ["Text"];

/// Exactly the admitted unqualified attributes: placement (`Id`, `Cell`) plus
/// the optional literal caption (`Text`); namespace declarations are not behaviour.
fn admitted_attributes(attr: &crate::xml::Attrs) -> bool {
    attr.names().all(|name| {
        name == "xmlns"
            || name.starts_with("xmlns:")
            || REQUIRED.contains(&name)
            || OPTIONAL.contains(&name)
    }) && REQUIRED
        .iter()
        .all(|name| attr.evidence_value(name).is_some())
}

fn read(bytes: &[u8], program: &str, budget: usize) -> Result<SourceTree, NestedBinding> {
    let unavailable = NestedBinding::SourceUnavailable;
    let drift = NestedBinding::StructuralDrift;
    let mut reader = NsReader::from_reader(bytes);
    let mut stack: Vec<Entry> = Vec::new();
    let mut namespace = None;
    let mut owners = 0;
    // The stored-tree parser's own view, namespace-blind (`dynamic::parse`).
    let mut parser_program = String::new();
    let mut parser_modules: Vec<String> = Vec::new();
    let mut parser_depth = 0usize;
    let mut program_roots = 0;
    let mut nodes: Vec<(String, Option<i64>, i64)> = Vec::new();
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut block_ids = HashSet::new();
    let mut duplicate = false;
    for _ in 0..budget {
        let (ns, event) = reader.read_resolved_event().map_err(|_| unavailable)?;
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
                        return Err(unavailable);
                    }
                    namespace = resolved.clone();
                }
                let same = resolved == namespace;
                let path: Vec<&str> = stack.iter().map(|entry| entry.kind.as_str()).collect();
                let parent = stack.last();
                let relevant = matches!(
                    kind.as_str(),
                    "ApplicationProgram" | "ModuleDef" | "ParameterBlock"
                );
                let attr = if relevant {
                    crate::xml::attrs(&e, "parameter nested block").map_err(|_| unavailable)?
                } else {
                    crate::xml::Attrs::default()
                };
                // A tree holding these names desynchronizes the parser's frames.
                if parser_depth > 0 && matches!(kind.as_str(), "ApplicationProgram" | "ModuleDef") {
                    return Err(drift);
                }
                if kind == "ApplicationProgram" {
                    parser_program = attr.get("Id").unwrap_or_default().to_owned();
                    parser_modules.clear();
                } else if kind == "ModuleDef" && !empty {
                    parser_modules.push(attr.get("Id").unwrap_or_default().to_owned());
                }
                let owner_here = same
                    && kind == "ApplicationProgram"
                    && is_program_owner(&path)
                    && attr.evidence_value("Id") == Some(program);
                if owner_here {
                    owners += 1;
                }
                let active = same && (owner_here || parent.is_some_and(|p| p.active));
                let dynamic = active
                    && (is_dynamic_root(&kind, &path)
                        || (parent.is_some_and(|p| p.dynamic) && is_dynamic_container(&kind)));
                // The parser starts a tree at any `Dynamic` outside a tree.
                let mut node = None;
                if parser_depth == 0 && kind == "Dynamic" {
                    let program_scope = parser_program == program
                        && parser_modules.last().is_none_or(|id| id.is_empty());
                    if program_scope {
                        program_roots += 1;
                        let canonical = dynamic && path.last() == Some(&"ApplicationProgram");
                        if program_roots > 1 || !canonical {
                            return Err(drift);
                        }
                        nodes.push((kind.clone(), None, 0));
                        node = Some(0);
                    }
                } else if let Some(parent_node) = parent.and_then(|p| p.node) {
                    let id = nodes.len() as i64;
                    let top = stack.last_mut().expect("parent present");
                    nodes.push((kind.clone(), Some(parent_node), top.children));
                    top.children += 1;
                    node = Some(id);
                }
                if parser_depth > 0 || kind == "Dynamic" {
                    parser_depth += usize::from(!empty);
                }
                let parent = stack.last();
                let inherited = parent.and_then(|p| p.block.clone());
                // A nested block's own `@Cell` places it in its parent's grid,
                // so its candidate parent is the inherited block, not itself.
                let block = if dynamic && kind == "ParameterBlock" {
                    inherited.clone().or_else(|| {
                        node.map(|node| (node, attr.evidence_value("Id").map(str::to_owned)))
                    })
                } else {
                    inherited.clone()
                };
                // A nested program-owned block with a cell in a dynamic tree is
                // a candidate; only its own `@Cell` places it in the parent grid.
                // The candidate's parent is the *inherited* block, not this one.
                if let Some(node) = (same && dynamic && kind == "ParameterBlock")
                    .then_some(node)
                    .flatten()
                {
                    if let Some(id) = attr.evidence_value("Id") {
                        duplicate |= !block_ids.insert(id.to_owned());
                    }
                    let ancestry = parent.is_some_and(|p| p.dynamic);
                    if let (true, true, Some((block_node, Some(block_id)))) =
                        (ancestry, admitted_attributes(&attr), inherited.clone())
                    {
                        candidates.push(Candidate {
                            node,
                            element_id: attr.evidence_value("Id").unwrap_or_default().to_owned(),
                            text: attr.evidence_value("Text").and_then(literal_text),
                            cell: attr.evidence_value("Cell").unwrap_or_default().to_owned(),
                            block_node,
                            block_id,
                        });
                    }
                }
                if !empty {
                    stack.push(Entry {
                        kind,
                        active,
                        dynamic,
                        node,
                        children: 0,
                        block,
                    });
                    if stack.len() > MAX_DEPTH {
                        return Err(unavailable);
                    }
                }
            }
            Event::End(e) => {
                if e.local_name().as_ref() == "ModuleDef" {
                    parser_modules.pop();
                }
                parser_depth = parser_depth.saturating_sub(1);
                stack.pop().ok_or(unavailable)?;
            }
            Event::DocType(_) => return Err(unavailable),
            Event::Eof => {
                if !stack.is_empty() || owners != 1 {
                    return Err(unavailable);
                }
                if duplicate {
                    return Err(NestedBinding::DuplicateBlockId);
                }
                return Ok(SourceTree { nodes, candidates });
            }
            _ => {}
        }
    }
    Err(unavailable)
}

#[cfg(test)]
mod tests {
    // integration coverage lives in tests/it/parameter_layout/passive_nested_cells.rs
}
