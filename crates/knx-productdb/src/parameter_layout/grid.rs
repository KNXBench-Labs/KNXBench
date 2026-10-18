//! Declared parameter-block grids, read from the retained source (ADR-0137).
//!
//! `dynamic_node` keeps `Rows`/`Columns` structurally but `Layout`, `Cell` and
//! `Width` only in its audit string, which nothing re-parses. This reader
//! applies the caption reader's exact-ownership rules to the hash-checked
//! source instead. It never derives a cell from text, names or identifiers.
use super::ownership::{is_dynamic_container, is_dynamic_root, is_program_owner};
use quick_xml::{events::Event, name::ResolveResult, NsReader};
use std::collections::{HashMap, HashSet};

/// Work budget of one grid read. Every XML event consumes at least one input
/// byte, so the source ceiling (`MAX_EDITOR_SOURCE_BYTES`) bounds the walk;
/// exceeding it reports the source as unavailable rather than claiming that
/// no grid exists.
pub const MAX_GRID_EVENTS: usize = crate::parameter_editor::MAX_EDITOR_SOURCE_BYTES;
const MAX_DEPTH: usize = 64;
/// Deepest `ParameterRefRef/@IndentLevel` shown. The collection declares 1 to 5
/// (census P4); a larger or malformed value indents nothing.
pub const MAX_INDENT_LEVEL: u8 = 8;

/// A 1-based `row,col` position (Project Schema 23 §1.1.2.35 `CellRef_t`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub row: u32,
    pub column: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridStatus {
    /// `Table`/`Grid` with exactly one non-empty `Rows` and `Columns`.
    Grid,
    /// A `Layout` token outside `Table`/`Grid`/`List`; kept verbatim.
    UnknownLayout,
    /// Incomplete or contradictory grid declarations.
    InvalidGrid,
    /// `Rows`/`Columns` exist structurally but the source could not be read.
    SourceUnavailable,
}

/// One declared `Row` or `Column`, attribute values verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridAxis {
    pub id: String,
    pub text: Option<String>,
    pub text_parameter_ref_id: Option<String>,
    /// `Column/@Width` only.
    pub width: Option<String>,
    /// `Row/@CollapseIfEmpty` only; preserved, not applied.
    pub collapse_if_empty: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridIssueKind {
    CellMalformed,
    CellOutOfRange,
    MissingCell,
    AmbiguousCell,
    /// A placed element this build does not show (`Button`, nested block, ...).
    UnshownContent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GridIssue {
    pub kind: GridIssueKind,
    /// Element local name, verbatim.
    pub element: String,
    /// `@RefId` of a reference, else `@Id`.
    pub reference: Option<String>,
    /// The raw `@Cell`, when one was declared.
    pub cell: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockGrid {
    pub declared_layout: Option<String>,
    pub status: GridStatus,
    pub rows: Vec<GridAxis>,
    pub columns: Vec<GridAxis>,
    pub issues: Vec<GridIssue>,
}

impl BlockGrid {
    pub(super) fn unavailable() -> Self {
        BlockGrid {
            declared_layout: None,
            status: GridStatus::SourceUnavailable,
            rows: Vec::new(),
            columns: Vec::new(),
            issues: Vec::new(),
        }
    }
}

/// Everything the source declares, keyed by document-unique element `@Id`.
#[derive(Debug, Default)]
pub(super) struct SourceGrids {
    pub blocks: HashMap<String, BlockGrid>,
    /// `(block @Id, ParameterRefRef @RefId)` → its single unambiguous cell.
    pub references: HashMap<(String, String), Cell>,
    /// Separator `@Id` → `(block @Id, cell)`.
    pub separators: HashMap<String, (String, Cell)>,
    /// `(block @Id, ParameterRefRef @RefId)` → the one valid `@IndentLevel`
    /// every occurrence in that block declares (census P4).
    pub indents: HashMap<(String, String), u8>,
}

struct Frame {
    id: String,
    layout: Option<String>,
    row_containers: usize,
    column_containers: usize,
    rows: Vec<GridAxis>,
    columns: Vec<GridAxis>,
    /// A `Row`/`Column` without its required `@Id`.
    malformed_axis: bool,
    /// `(element, reference, raw cell)` in document order.
    placed: Vec<(String, Option<String>, Option<String>)>,
    /// `(ParameterRefRef @RefId, raw @IndentLevel)` in document order.
    indents: Vec<(String, Option<String>)>,
}

#[derive(Clone, Copy, PartialEq)]
enum Role {
    Other,
    Block(usize),
    Rows(usize),
    Columns(usize),
}

struct Entry {
    kind: String,
    active: bool,
    dynamic: bool,
    role: Role,
    /// Nearest enclosing grid frame, inherited by descendants.
    owner: Option<usize>,
}

pub(super) fn read_grids(bytes: &[u8], program: &str) -> Option<SourceGrids> {
    read_grids_within(bytes, program, MAX_GRID_EVENTS)
}

fn read_grids_within(bytes: &[u8], program: &str, budget: usize) -> Option<SourceGrids> {
    let mut reader = NsReader::from_reader(bytes);
    let mut stack: Vec<Entry> = Vec::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut block_ids = HashSet::new();
    let mut namespace = None;
    let mut owners = 0;
    for _ in 0..budget {
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
                let path: Vec<&str> = stack.iter().map(|entry| entry.kind.as_str()).collect();
                let parent = stack.last();
                // Attributes matter only for the owner, blocks, axes and
                // anything below a block; skipping the rest (mostly `Static`)
                // is the measured hot path of this read.
                let relevant = kind == "ApplicationProgram"
                    || kind == "ParameterBlock"
                    || parent.is_some_and(|p| p.owner.is_some() || p.role != Role::Other);
                let attr = if relevant {
                    crate::xml::attrs(&e, "parameter grid").ok()?
                } else {
                    crate::xml::Attrs::default()
                };
                let owner_here = same
                    && kind == "ApplicationProgram"
                    && is_program_owner(&path)
                    && attr.evidence_value("Id") == Some(program);
                if owner_here {
                    owners += 1;
                }
                let active = same && (owner_here || parent.is_some_and(|p| p.active));
                let parent_dynamic = parent.is_some_and(|p| p.dynamic);
                let dynamic = active
                    && (is_dynamic_root(&kind, &path)
                        || (parent_dynamic && is_dynamic_container(&kind)));
                let inherited = parent.and_then(|p| p.owner);
                let mut role = Role::Other;
                let mut owner = inherited;
                if dynamic && kind == "ParameterBlock" {
                    // A nested block's own `@Cell` places it in its parent's grid.
                    if let (Some(frame), Some(cell)) = (inherited, attr.evidence_value("Cell")) {
                        frames[frame].placed.push((
                            kind.clone(),
                            attr.evidence_value("Id").map(str::to_owned),
                            Some(cell.to_owned()),
                        ));
                    }
                    if let Some(id) = attr.evidence_value("Id") {
                        if !block_ids.insert(id.to_owned()) {
                            return None;
                        }
                        frames.push(Frame {
                            id: id.to_owned(),
                            layout: attr.evidence_value("Layout").map(str::to_owned),
                            row_containers: 0,
                            column_containers: 0,
                            rows: Vec::new(),
                            columns: Vec::new(),
                            malformed_axis: false,
                            placed: Vec::new(),
                            indents: Vec::new(),
                        });
                        role = Role::Block(frames.len() - 1);
                        owner = Some(frames.len() - 1);
                    } else {
                        owner = None;
                    }
                } else if let (true, Some(Role::Block(frame))) = (same, parent.map(|p| p.role)) {
                    match kind.as_str() {
                        "Rows" => {
                            frames[frame].row_containers += 1;
                            role = Role::Rows(frame);
                        }
                        "Columns" => {
                            frames[frame].column_containers += 1;
                            role = Role::Columns(frame);
                        }
                        _ => place(&mut frames, Some(frame), &kind, &attr),
                    }
                } else if let (true, Some(Role::Rows(frame))) = (same, parent.map(|p| p.role)) {
                    if kind == "Row" {
                        match axis(&attr) {
                            Some(row) => frames[frame].rows.push(row),
                            None => frames[frame].malformed_axis = true,
                        }
                    }
                } else if let (true, Some(Role::Columns(frame))) = (same, parent.map(|p| p.role)) {
                    if kind == "Column" {
                        match axis(&attr) {
                            Some(column) => frames[frame].columns.push(column),
                            None => frames[frame].malformed_axis = true,
                        }
                    }
                } else if same && parent_dynamic {
                    place(&mut frames, inherited, &kind, &attr);
                }
                if !empty {
                    stack.push(Entry {
                        kind,
                        active,
                        dynamic,
                        role,
                        owner,
                    });
                    if stack.len() > MAX_DEPTH {
                        return None;
                    }
                }
            }
            Event::End(_) => {
                stack.pop()?;
            }
            Event::DocType(_) => return None,
            Event::Eof => {
                return (stack.is_empty() && owners == 1).then(|| finish(frames));
            }
            _ => {}
        }
    }
    None
}

fn axis(attr: &crate::xml::Attrs) -> Option<GridAxis> {
    let value = |name: &str| attr.evidence_value(name).map(str::to_owned);
    Some(GridAxis {
        id: value("Id")?,
        text: value("Text"),
        text_parameter_ref_id: value("TextParameterRefId"),
        width: value("Width"),
        collapse_if_empty: value("CollapseIfEmpty"),
    })
}

/// Records an element below a grid block: every reference (a missing cell is
/// an issue) and any other element that declares a cell.
fn place(frames: &mut [Frame], owner: Option<usize>, kind: &str, attr: &crate::xml::Attrs) {
    let Some(frame) = owner else {
        return;
    };
    let cell = attr.evidence_value("Cell").map(str::to_owned);
    if kind == "ParameterRefRef" {
        if let Some(reference) = attr.evidence_value("RefId") {
            frames[frame].indents.push((
                reference.to_owned(),
                attr.evidence_value("IndentLevel").map(str::to_owned),
            ));
        }
    }
    if kind != "ParameterRefRef" && cell.is_none() {
        return;
    }
    let reference = attr
        .evidence_value(if kind == "ParameterRefRef" {
            "RefId"
        } else {
            "Id"
        })
        .or_else(|| attr.evidence_value("RefId"))
        .map(str::to_owned);
    frames[frame]
        .placed
        .push((kind.to_owned(), reference, cell));
}

fn parse_cell(raw: &str, rows: usize, columns: usize) -> Result<Cell, GridIssueKind> {
    let (row, column) = raw.split_once(',').ok_or(GridIssueKind::CellMalformed)?;
    let digits = |s: &str| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit());
    if !digits(row) || !digits(column) {
        return Err(GridIssueKind::CellMalformed);
    }
    let (Ok(row), Ok(column)) = (row.parse::<u32>(), column.parse::<u32>()) else {
        return Err(GridIssueKind::CellOutOfRange);
    };
    let fits = |value: u32, count: usize| value >= 1 && (value as usize) <= count;
    if fits(row, rows) && fits(column, columns) {
        Ok(Cell { row, column })
    } else {
        Err(GridIssueKind::CellOutOfRange)
    }
}

fn finish(frames: Vec<Frame>) -> SourceGrids {
    let mut result = SourceGrids::default();
    for frame in frames {
        place_indents(&frame, &mut result);
        let axes = frame.row_containers > 0 || frame.column_containers > 0;
        let status = match frame.layout.as_deref() {
            Some("Table" | "Grid") => {
                if frame.row_containers == 1
                    && frame.column_containers == 1
                    && !frame.malformed_axis
                    && !frame.rows.is_empty()
                    && !frame.columns.is_empty()
                {
                    GridStatus::Grid
                } else {
                    GridStatus::InvalidGrid
                }
            }
            None | Some("List") if axes => GridStatus::InvalidGrid,
            None | Some("List") => continue,
            Some(_) => GridStatus::UnknownLayout,
        };
        let mut issues = Vec::new();
        if status == GridStatus::Grid {
            place_cells(&frame, &mut issues, &mut result);
        }
        result.blocks.insert(
            frame.id,
            BlockGrid {
                declared_layout: frame.layout,
                status,
                rows: frame.rows,
                columns: frame.columns,
                issues,
            },
        );
    }
    result
}

/// A reference is indented only when every occurrence in the block declares
/// the same valid level; absent, contradictory or malformed values give none.
fn place_indents(frame: &Frame, result: &mut SourceGrids) {
    let mut levels: HashMap<&str, Option<u8>> = HashMap::new();
    for (reference, raw) in &frame.indents {
        let level = raw.as_deref().and_then(indent_level);
        levels
            .entry(reference.as_str())
            .and_modify(|previous| {
                if *previous != level {
                    *previous = None;
                }
            })
            .or_insert(level);
    }
    for (reference, level) in levels {
        if let Some(level) = level {
            result
                .indents
                .insert((frame.id.clone(), reference.to_owned()), level);
        }
    }
}

/// `1..=MAX_INDENT_LEVEL` written as plain decimal digits.
fn indent_level(raw: &str) -> Option<u8> {
    if raw.is_empty() || raw.len() > 3 || !raw.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    raw.parse::<u8>()
        .ok()
        .filter(|level| (1..=MAX_INDENT_LEVEL).contains(level))
}

/// An element gets a cell only when every occurrence names the same valid
/// one; any other combination is a named issue in first-occurrence order.
fn place_cells(frame: &Frame, issues: &mut Vec<GridIssue>, result: &mut SourceGrids) {
    struct Occurrences {
        first: usize,
        element: String,
        reference: Option<String>,
        cells: Vec<Cell>,
        problems: Vec<(GridIssueKind, Option<String>)>,
    }
    let mut order: Vec<Occurrences> = Vec::new();
    let mut index: HashMap<(String, Option<String>), usize> = HashMap::new();
    for (position, (element, reference, raw)) in frame.placed.iter().enumerate() {
        let key = (element.clone(), reference.clone());
        let slot = *index.entry(key).or_insert_with(|| {
            order.push(Occurrences {
                first: position,
                element: element.clone(),
                reference: reference.clone(),
                cells: Vec::new(),
                problems: Vec::new(),
            });
            order.len() - 1
        });
        let entry = &mut order[slot];
        let shown = matches!(element.as_str(), "ParameterRefRef" | "ParameterSeparator");
        let outcome = match (shown, raw) {
            (false, _) => Err(GridIssueKind::UnshownContent),
            (true, None) => Err(GridIssueKind::MissingCell),
            (true, Some(raw)) => parse_cell(raw, frame.rows.len(), frame.columns.len()),
        };
        match outcome {
            Ok(cell) if !entry.cells.contains(&cell) => entry.cells.push(cell),
            Ok(_) => {}
            Err(kind) if !entry.problems.contains(&(kind, raw.clone())) => {
                entry.problems.push((kind, raw.clone()))
            }
            Err(_) => {}
        }
    }
    let mut ordered: Vec<(usize, GridIssue)> = Vec::new();
    for entry in order {
        let issue = |kind, cell| GridIssue {
            kind,
            element: entry.element.clone(),
            reference: entry.reference.clone(),
            cell,
        };
        for (kind, cell) in &entry.problems {
            ordered.push((entry.first, issue(*kind, cell.clone())));
        }
        if entry.cells.len() > 1 {
            ordered.push((entry.first, issue(GridIssueKind::AmbiguousCell, None)));
        }
        let (Some(reference), [cell], true) = (
            &entry.reference,
            entry.cells.as_slice(),
            entry.problems.is_empty(),
        ) else {
            continue;
        };
        if entry.element == "ParameterRefRef" {
            result
                .references
                .insert((frame.id.clone(), reference.clone()), *cell);
        } else {
            result
                .separators
                .insert(reference.clone(), (frame.id.clone(), *cell));
        }
    }
    ordered.sort_by_key(|(first, _)| *first);
    issues.extend(ordered.into_iter().map(|(_, issue)| issue));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn document(program: &str, dynamic: &str) -> String {
        format!(
            r#"<KNX xmlns="http://knx.org/xml/project/23"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="{program}"><Dynamic>{dynamic}</Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
        )
    }

    const GRID: &str = r#"<ParameterBlock Id="B" Layout="Table"><Rows><Row Id="R"/></Rows><Columns><Column Id="C" Width="100%"/></Columns><ParameterRefRef RefId="P" Cell="1,1"/></ParameterBlock>"#;

    #[test]
    fn only_the_exact_program_and_unprefixed_source_attributes_count() {
        let xml = document("A", GRID);
        let grids = read_grids(xml.as_bytes(), "A").unwrap();
        assert_eq!(grids.blocks["B"].status, GridStatus::Grid);
        assert_eq!(
            grids.references[&("B".into(), "P".into())],
            Cell { row: 1, column: 1 }
        );
        assert!(read_grids(xml.as_bytes(), "Other").is_none(), "no owner");
        let prefixed = document(
            "A",
            &GRID.replace(r#"Layout="Table""#, r#"xmlns:x="urn:x" x:Layout="Table""#),
        );
        let grids = read_grids(prefixed.as_bytes(), "A").unwrap();
        assert_eq!(
            grids.blocks["B"].status,
            GridStatus::InvalidGrid,
            "rows without a source layout"
        );
        let foreign = document(
            "A",
            &GRID
                .replace("<Rows>", r#"<x:Rows xmlns:x="urn:x">"#)
                .replace("</Rows>", "</x:Rows>"),
        );
        let grids = read_grids(foreign.as_bytes(), "A").unwrap();
        assert_eq!(
            grids.blocks["B"].status,
            GridStatus::InvalidGrid,
            "foreign rows are not rows"
        );
        let outside = document("A", "").replace(
            "<Dynamic>",
            &format!("<Extension>{GRID}</Extension><Dynamic>"),
        );
        assert!(read_grids(outside.as_bytes(), "A")
            .unwrap()
            .blocks
            .is_empty());
    }

    #[test]
    fn duplicate_blocks_doctype_and_exhausted_budgets_refuse_the_whole_read() {
        assert!(read_grids(document("A", &format!("{GRID}{GRID}")).as_bytes(), "A").is_none());
        assert!(read_grids(
            format!("<!DOCTYPE KNX>{}", document("A", GRID)).as_bytes(),
            "A"
        )
        .is_none());
        let xml = document("A", GRID);
        assert!(read_grids_within(xml.as_bytes(), "A", 8).is_none());
        assert!(read_grids_within(xml.as_bytes(), "A", 64).is_some());
    }

    #[test]
    fn cells_belong_to_the_nearest_block_and_axes_need_identifiers() {
        let nested = r#"<ParameterBlock Id="B" Layout="Table"><Rows><Row Id="R"/></Rows><Columns><Column Id="C" Width="100%"/></Columns><ParameterBlock Id="N" Cell="1,1"><ParameterRefRef RefId="P" Cell="1,1"/></ParameterBlock></ParameterBlock>"#;
        let grids = read_grids(document("A", nested).as_bytes(), "A").unwrap();
        assert!(
            grids.references.is_empty(),
            "inner reference belongs to the list block N"
        );
        assert_eq!(
            grids.blocks["B"].issues[0].kind,
            GridIssueKind::UnshownContent
        );
        assert_eq!(grids.blocks["B"].issues[0].element, "ParameterBlock");
        assert!(!grids.blocks.contains_key("N"));
        let unnamed = document("A", &GRID.replace(r#"<Row Id="R"/>"#, "<Row/>"));
        assert_eq!(
            read_grids(unnamed.as_bytes(), "A").unwrap().blocks["B"].status,
            GridStatus::InvalidGrid
        );
    }

    #[test]
    fn cell_syntax_is_two_ascii_integers_within_the_declared_axes() {
        for raw in ["1,1", "01,1"] {
            assert_eq!(
                parse_cell(raw, 1, 1),
                Ok(Cell { row: 1, column: 1 }),
                "{raw}"
            );
        }
        for raw in ["1", "1,", ",1", "1,1,1", " 1,1", "+1,1", "1;1", "a,1"] {
            assert_eq!(
                parse_cell(raw, 1, 1),
                Err(GridIssueKind::CellMalformed),
                "{raw}"
            );
        }
        for raw in ["0,1", "1,0", "2,1", "1,2", "99999999999,1"] {
            assert_eq!(
                parse_cell(raw, 1, 1),
                Err(GridIssueKind::CellOutOfRange),
                "{raw}"
            );
        }
    }
}
