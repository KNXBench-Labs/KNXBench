//! Source-bound literal `Button/@Text` cell labels (`button-literal-passive-v1`).
//!
//! `dynamic_node` keeps a `Button` as kind, parent and position only: its
//! `@Id` and `@Text` never reached a column. Rather than migrate or reimport,
//! this reader reopens the hash-checked source and rebuilds the program's
//! `Dynamic` tree exactly as the stored-tree parser numbered it: every
//! element counts, whatever its namespace. Only when that structure matches
//! the stored nodes completely are admitted literals bound to node ids. Any
//! mismatch, duplicate or exhausted budget binds nothing at all.
use super::grid::{parse_cell, BlockGrid, Cell, GridStatus, MAX_DEPTH};
use super::ownership::{is_dynamic_container, is_dynamic_root, is_program_owner};
use super::{Group, Node};
use quick_xml::{events::Event, name::ResolveResult, NsReader};
use std::collections::{BTreeMap, HashMap, HashSet};

/// An admitted program-owned `Button`: its literal text at its declared cell.
/// Presentation only; no handler, action or write target is ever derived.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ButtonLiteral {
    /// The nearest enclosing `ParameterBlock`, a renderable grid.
    pub group: Group,
    pub cell: Cell,
    /// `Button/@Id`, verbatim from the source.
    pub element_id: String,
    /// `Button/@Text`, XML-decoded, otherwise verbatim (never trimmed).
    pub text: String,
}

/// Whether the retained source could be bound to the stored `Dynamic` nodes.
/// Any refusal binds no label at all, never a partial set.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonBinding {
    /// Bound; every admitted `Button` has its [`ButtonLiteral`].
    Bound,
    /// No hash-checked source, no unique owner, or the read budget ran out.
    #[default]
    SourceUnavailable,
    /// Stored nodes and source structure differ, or roots are not unique.
    StructuralDrift,
    /// Two program-owned `Button`s declare the same `@Id`.
    DuplicateButtonId,
}

/// The attributes this form admits; the three handler ones are inert data.
const REQUIRED: [&str; 3] = ["Id", "Text", "Cell"];
const INERT: [&str; 3] = [
    "EventHandler",
    "EventHandlerOnline",
    "EventHandlerParameters",
];

/// One admitted source candidate, before it is bound to stored nodes.
struct Candidate {
    node: i64,
    element_id: String,
    text: String,
    cell: String,
    block_node: i64,
    block_id: String,
    /// Cleared when a child element or character data shows that it is no leaf.
    leaf: bool,
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
    candidate: Option<usize>,
}

/// Binds every admitted literal of `program`'s own tree, or none.
pub(super) fn bind(
    bytes: Option<&[u8]>,
    program: &str,
    stored: Option<&BTreeMap<i64, Node>>,
    grids: &HashMap<i64, BlockGrid>,
    captions: &HashMap<String, String>,
) -> (ButtonBinding, HashMap<i64, ButtonLiteral>) {
    // Nothing to bind: no stored `Button`, so no source read is needed.
    if !stored.is_some_and(|nodes| nodes.values().any(|node| node.kind == "Button")) {
        return (ButtonBinding::Bound, HashMap::new());
    }
    // Nor when no stored `Button`'s nearest block can render a grid: such a
    // `Button` never shows a label, so a refused read would hide nothing.
    // A block whose grid source is unreadable may still be one, so it reads.
    if stored.is_some_and(|nodes| !any_button_may_have_a_grid(nodes, grids)) {
        return (ButtonBinding::Bound, HashMap::new());
    }
    let Some(bytes) = bytes else {
        return (ButtonBinding::SourceUnavailable, HashMap::new());
    };
    match read(bytes, program, super::grid::MAX_GRID_EVENTS)
        .and_then(|source| bind_source(source, stored.unwrap_or(&BTreeMap::new()), grids, captions))
    {
        Ok(literals) => (ButtonBinding::Bound, literals),
        Err(refusal) => (refusal, HashMap::new()),
    }
}

/// The nearest stored `ParameterBlock` above `node`, stopping at a cycle or
/// a missing ancestor. Admission and the read skip share this one walk.
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

/// Whether some stored `Button`'s nearest `ParameterBlock` is a grid or a
/// grid whose source could not be read.
fn any_button_may_have_a_grid(
    stored: &BTreeMap<i64, Node>,
    grids: &HashMap<i64, BlockGrid>,
) -> bool {
    stored
        .iter()
        .filter(|(_, node)| node.kind == "Button")
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
) -> Result<HashMap<i64, ButtonLiteral>, ButtonBinding> {
    // Complete structural correspondence, every node and none extra.
    if stored.len() != source.nodes.len() {
        return Err(ButtonBinding::StructuralDrift);
    }
    for ((id, node), (index, (kind, parent, position))) in
        stored.iter().zip(source.nodes.iter().enumerate())
    {
        if *id != index as i64
            || node.kind != *kind
            || node.parent != *parent
            || node.position != *position
        {
            return Err(ButtonBinding::StructuralDrift);
        }
    }
    let mut literals = HashMap::new();
    for candidate in source.candidates.into_iter().filter(|c| c.leaf) {
        // The nearest stored block must be the same node with the same `@Id`.
        let Some((block_node, block)) = nearest_block(stored, candidate.node) else {
            continue;
        };
        if block_node != candidate.block_node || block.id.as_deref() != Some(&candidate.block_id) {
            return Err(ButtonBinding::StructuralDrift);
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
        literals.insert(
            candidate.node,
            ButtonLiteral {
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
    Ok(literals)
}

/// Text this literal form shows: present, not blank, not `#`, no placeholder delimiter.
fn literal_text(text: &str) -> bool {
    !text.trim().is_empty() && text.trim() != "#" && !text.contains("{{") && !text.contains("}}")
}

/// Exactly the admitted unqualified attributes; namespace declarations are not behaviour.
fn admitted_attributes(attr: &crate::xml::Attrs) -> bool {
    attr.names().all(|name| {
        name == "xmlns"
            || name.starts_with("xmlns:")
            || REQUIRED.contains(&name)
            || INERT.contains(&name)
    }) && REQUIRED
        .iter()
        .all(|name| attr.evidence_value(name).is_some())
}

fn read(bytes: &[u8], program: &str, budget: usize) -> Result<SourceTree, ButtonBinding> {
    let unavailable = ButtonBinding::SourceUnavailable;
    let drift = ButtonBinding::StructuralDrift;
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
    let mut button_ids = HashSet::new();
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
                    "ApplicationProgram" | "ModuleDef" | "ParameterBlock" | "Button"
                );
                let attr = if relevant {
                    crate::xml::attrs(&e, "parameter button").map_err(|_| unavailable)?
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
                    if let Some(index) = top.candidate {
                        candidates[index].leaf = false;
                    }
                    node = Some(id);
                }
                if parser_depth > 0 || kind == "Dynamic" {
                    parser_depth += usize::from(!empty);
                }
                let parent = stack.last();
                let inherited = parent.and_then(|p| p.block.clone());
                let block = if dynamic && kind == "ParameterBlock" {
                    node.map(|node| (node, attr.evidence_value("Id").map(str::to_owned)))
                } else {
                    inherited
                };
                let mut candidate = None;
                if let (true, Some(button_node)) = (same && kind == "Button", node) {
                    if let Some(id) = attr.evidence_value("Id") {
                        duplicate |= !button_ids.insert(id.to_owned());
                    }
                    let text = attr.evidence_value("Text").unwrap_or_default();
                    let ancestry = parent.is_some_and(|p| p.dynamic);
                    if let (true, true, true, Some((block_node, Some(block_id)))) = (
                        ancestry,
                        admitted_attributes(&attr),
                        literal_text(text),
                        block.clone(),
                    ) {
                        candidates.push(Candidate {
                            node: button_node,
                            element_id: attr.evidence_value("Id").unwrap_or_default().to_owned(),
                            text: text.to_owned(),
                            cell: attr.evidence_value("Cell").unwrap_or_default().to_owned(),
                            block_node,
                            block_id,
                            leaf: true,
                        });
                        candidate = Some(candidates.len() - 1);
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
                        candidate,
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
            // Character data inside a candidate is content, so no leaf form;
            // XML whitespace, comments and processing instructions are not.
            Event::Text(text) => {
                if text.chars().any(|c| !matches!(c, ' ' | '\t' | '\r' | '\n')) {
                    clear_leaf(&stack, &mut candidates);
                }
            }
            Event::CData(_) | Event::GeneralRef(_) => clear_leaf(&stack, &mut candidates),
            Event::DocType(_) => return Err(unavailable),
            Event::Eof => {
                if !stack.is_empty() || owners != 1 {
                    return Err(unavailable);
                }
                if duplicate {
                    return Err(ButtonBinding::DuplicateButtonId);
                }
                return Ok(SourceTree { nodes, candidates });
            }
            _ => {}
        }
    }
    Err(unavailable)
}

/// The open candidate, if any, has content and so is no leaf.
fn clear_leaf(stack: &[Entry], candidates: &mut [Candidate]) {
    if let Some(index) = stack.last().and_then(|entry| entry.candidate) {
        candidates[index].leaf = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(dynamic: &str) -> String {
        format!(
            r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A"><Dynamic>{dynamic}</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
        )
    }

    const GRID: &str = r#"<ParameterBlock Id="B" Layout="Grid"><Rows><Row Id="R"/></Rows><Columns><Column Id="C"/></Columns><Button Id="X" Text="Label" Cell="1,1"/></ParameterBlock>"#;

    #[test]
    fn the_tree_is_numbered_like_the_parser_and_candidates_need_a_block_id() {
        let source = read(document(GRID).as_bytes(), "A", 1_000).ok().unwrap();
        let kinds: Vec<&str> = source
            .nodes
            .iter()
            .map(|(kind, _, _)| kind.as_str())
            .collect();
        assert_eq!(
            kinds,
            [
                "Dynamic",
                "ParameterBlock",
                "Rows",
                "Row",
                "Columns",
                "Column",
                "Button"
            ]
        );
        assert_eq!(source.nodes[6], ("Button".into(), Some(1), 2));
        assert_eq!(source.candidates.len(), 1);
        assert_eq!(source.candidates[0].block_id, "B");
        let anonymous = read(
            document(&GRID.replace(r#"Id="B" "#, "")).as_bytes(),
            "A",
            1_000,
        )
        .ok()
        .unwrap();
        assert!(anonymous.candidates.is_empty());
    }

    #[test]
    fn exhausted_budgets_doctype_and_missing_owners_refuse_the_read() {
        let xml = document(GRID);
        assert_eq!(
            read(xml.as_bytes(), "A", 8).err(),
            Some(ButtonBinding::SourceUnavailable)
        );
        assert!(read(xml.as_bytes(), "A", 64).is_ok());
        assert_eq!(
            read(xml.as_bytes(), "Other", 1_000).err(),
            Some(ButtonBinding::SourceUnavailable)
        );
        let doctype = format!("<!DOCTYPE KNX>{xml}");
        assert_eq!(
            read(doctype.as_bytes(), "A", 1_000).err(),
            Some(ButtonBinding::SourceUnavailable)
        );
        let shadow = document(GRID).replace(
            "</Dynamic>",
            r#"</Dynamic><x:Dynamic xmlns:x="urn:x"><x:Button Id="X" Text="Y" Cell="1,1"/></x:Dynamic>"#,
        );
        assert_eq!(
            read(shadow.as_bytes(), "A", 1_000).err(),
            Some(ButtonBinding::StructuralDrift)
        );
        let nested = document(&format!("<choose><when><ModuleDef/>{GRID}</when></choose>"));
        assert_eq!(
            read(nested.as_bytes(), "A", 1_000).err(),
            Some(ButtonBinding::StructuralDrift)
        );
    }

    #[test]
    fn a_second_same_namespace_program_root_is_drift() {
        let second = GRID.replace(r#"Id="X""#, r#"Id="Y""#);
        let xml = document(GRID).replace(
            "</Dynamic>",
            &format!("</Dynamic><Dynamic>{second}</Dynamic>"),
        );
        assert_eq!(
            read(xml.as_bytes(), "A", 1_000).err(),
            Some(ButtonBinding::StructuralDrift)
        );
    }

    #[test]
    fn nesting_beyond_the_shared_grid_depth_refuses_the_read() {
        // `document` opens six frames (KNX to `Dynamic`), `GRID` two more.
        let nested = |depth: usize| {
            let inner = depth - 8;
            document(&format!(
                "{}{GRID}{}",
                "<Wrap>".repeat(inner),
                "</Wrap>".repeat(inner)
            ))
        };
        assert!(read(nested(MAX_DEPTH).as_bytes(), "A", 10_000).is_ok());
        assert_eq!(
            read(nested(MAX_DEPTH + 1).as_bytes(), "A", 10_000).err(),
            Some(ButtonBinding::SourceUnavailable)
        );
    }
}
