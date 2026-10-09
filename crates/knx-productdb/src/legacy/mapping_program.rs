//! Maps EX-IM application programs: types, grouped parameters, objects and the visibility tree.
//!
//! Rules and their evidence: docs/research/legacy-vd-mapping.md.

use std::collections::{BTreeMap, BTreeSet};

use super::exim::ExImDocument;
use super::mapping::{
    manufacturer_id, rows, skip, MappedComObject, MappedComObjectRef, MappedDynamicNode,
    MappedEnumeration, MappedParameter, MappedParameterRef, MappedParameterType, MappedProgram,
    MappingDiagnostic, Row, NO_MANUFACTURER,
};

/// Where one `text_attribute` row lands.
#[derive(Debug, Clone)]
pub(super) struct TranslationTarget {
    pub(super) scope: String,
    pub(super) scope_id: String,
    pub(super) element: String,
    pub(super) attribute: String,
    /// For a group member: the representative's entity. The member gets a
    /// translation of its own only in a language where its text differs
    /// from the representative's (measured against ETS's conversion).
    pub(super) unless_same_as: Option<String>,
}

/// Translation targets per (`COLUMN_ID`, `ENTITY_ID`).
pub(super) type TranslationTargets = BTreeMap<(String, String), Vec<TranslationTarget>>;

/// A sort key for numbers written as text: numeric first, then verbatim.
fn number_key(text: &str) -> (i64, String) {
    (text.trim().parse().unwrap_or(i64::MAX), text.to_string())
}

fn flag(value: Option<String>) -> Option<String> {
    value.map(|v| {
        if v.trim() == "1" {
            "Enabled"
        } else {
            "Disabled"
        }
        .to_string()
    })
}

/// `HIGH_ACCESS` → `Access_t`. Measured: 0 → `None`, 2 → `ReadWrite`;
/// 1 → `Read` is assumed (no file at hand uses it).
fn access(level: Option<&str>) -> Option<&'static str> {
    match level.map(str::trim) {
        Some("0") => Some("None"),
        Some("1") => Some("Read"),
        Some("2") => Some("ReadWrite"),
        _ => None,
    }
}

/// ETS's `ObjectSize_t` spelling for a length in bits.
fn object_size(bits: i64) -> Option<String> {
    match bits {
        1..=7 => Some(format!("{bits} Bit")),
        8 => Some("1 Byte".into()),
        b if b > 8 && b % 8 == 0 => Some(format!("{} Bytes", b / 8)),
        _ => None,
    }
}

/// The range of an integer of `size` bits, as text. `None` for sizes
/// outside 1..=63.
fn natural_bounds(signed: bool, size: Option<i64>) -> (Option<String>, Option<String>) {
    match size {
        Some(bits @ 1..=63) if signed => (
            Some((-(1i64 << (bits - 1))).to_string()),
            Some(((1i64 << (bits - 1)) - 1).to_string()),
        ),
        Some(bits @ 1..=63) => (Some("0".into()), Some(((1i64 << bits) - 1).to_string())),
        _ => (None, None),
    }
}

/// One VD parameter row, read once.
struct VdParameter<'a> {
    row: Row<'a>,
    id: String,
    number: String,
    type_id: String,
    atomic: String,
    address: Option<i64>,
    bit: i64,
    size: Option<i64>,
    access: &'static str,
    default: Option<String>,
    text: Option<String>,
    parent: Option<String>,
    parent_value: Option<String>,
    order: i64,
}

struct VdObject {
    ref_id: String,
    unique: String,
    number: i64,
    parent: Option<String>,
    parent_value: Option<String>,
    order: i64,
}

pub(super) fn map_programs(
    document: &ExImDocument,
    ns: &str,
    default_language: Option<&str>,
    diagnostics: &mut Vec<MappingDiagnostic>,
) -> Vec<MappedProgram> {
    let masks: BTreeMap<String, String> = rows(document, "mask")
        .iter()
        .filter_map(|r| {
            let version: u16 = r.get("MASK_VERSION")?.trim().parse().ok()?;
            Some((r.get("MASK_ID")?, format!("MV-{version:04X}")))
        })
        .collect();
    let mut programs = Vec::new();
    for r in rows(document, "application_program") {
        let (Some(pid), Some(mid)) = (
            r.get("PROGRAM_ID"),
            r.get("MANUFACTURER_ID").and_then(|m| manufacturer_id(&m)),
        ) else {
            let reason = if r.get("PROGRAM_ID").is_none() {
                "PROGRAM_ID is empty"
            } else {
                NO_MANUFACTURER
            };
            skip(diagnostics, "application_program", reason);
            continue;
        };
        let id = format!("{mid}_A-{ns}-{pid}");
        let mask_version = match r.get("MASK_ID") {
            Some(mask) => {
                let found = masks.get(&mask).cloned();
                if found.is_none() {
                    diagnostics.push(MappingDiagnostic::DanglingReference {
                        table: "application_program".into(),
                        column: "MASK_ID".into(),
                        value: mask,
                    });
                }
                found
            }
            None => None,
        };
        let mut program = MappedProgram {
            id,
            exim_program_id: pid,
            manufacturer_id: mid,
            name: r.get("PROGRAM_NAME"),
            application_number: r.get("DEVICE_TYPE"),
            application_version: r.get("PROGRAM_VERSION"),
            mask_version,
            pei_type: r.get("PEI_TYPE"),
            linkable: r.get("LINKABLE").map(|v| v.trim() == "1"),
            original_manufacturer: r
                .get("ORIGINAL_MANUFACTURER_ID")
                .and_then(|m| manufacturer_id(&m)),
            default_language: default_language.map(str::to_string),
            parameter_types: Vec::new(),
            enumerations: Vec::new(),
            parameters: Vec::new(),
            parameter_refs: Vec::new(),
            com_objects: Vec::new(),
            com_object_refs: Vec::new(),
            dynamic: Vec::new(),
        };
        let types = map_types(document, &mut program, diagnostics);
        let parameters = map_parameters(document, &mut program, &types, diagnostics);
        let objects = map_objects(document, &mut program, diagnostics);
        program.dynamic = build_tree(&program.id, &parameters, &objects, diagnostics);
        programs.push(program);
    }
    programs
}

/// Returns `PARAMETER_TYPE_ID` → (atomic type, size) for the mapped types.
fn map_types(
    document: &ExImDocument,
    program: &mut MappedProgram,
    diagnostics: &mut Vec<MappingDiagnostic>,
) -> BTreeMap<String, (String, Option<i64>)> {
    let mut types = BTreeMap::new();
    for r in rows(document, "parameter_type") {
        if r.get("PROGRAM_ID").as_deref() != Some(program.exim_program_id.as_str()) {
            continue;
        }
        let (Some(tid), Some(atomic)) = (r.get("PARAMETER_TYPE_ID"), r.get("ATOMIC_TYPE_NUMBER"))
        else {
            skip(
                diagnostics,
                "parameter_type",
                "PARAMETER_TYPE_ID or ATOMIC_TYPE_NUMBER is empty",
            );
            continue;
        };
        let size = r.int("PARAMETER_TYPE_SIZE");
        let (kind, size_in_bit, base, number_type) = match atomic.trim() {
            "0" => ("None", None, None, None),
            "1" => ("Number", size, None, Some("unsignedInt")),
            "2" => ("Number", size, None, Some("signedInt")),
            "4" => ("Restriction", size, Some("Value"), None),
            _ => {
                diagnostics.push(MappingDiagnostic::UnknownAtomicType {
                    parameter_type: tid,
                    atomic_type: atomic,
                });
                continue;
            }
        };
        let (min_inclusive, max_inclusive) = if kind == "Number" {
            // An empty bound is the type's own (measured for signed 16 bit:
            // ETS writes -32768).
            let (low, high) = natural_bounds(number_type == Some("signedInt"), size);
            (
                r.get("PARAMETER_MINIMUM_VALUE").or(low),
                r.get("PARAMETER_MAXIMUM_VALUE").or(high),
            )
        } else {
            (None, None)
        };
        program.parameter_types.push(MappedParameterType {
            id: format!("{}_PT-{tid}", program.id),
            name: r.get("PARAMETER_TYPE_NAME"),
            kind: kind.into(),
            size_in_bit,
            base: base.map(str::to_string),
            min_inclusive,
            max_inclusive,
            number_type: number_type.map(str::to_string),
        });
        types.insert(tid, (atomic.trim().to_string(), size));
    }

    let mut values: BTreeMap<String, Vec<Row>> = BTreeMap::new();
    for r in rows(document, "parameter_list_of_values") {
        if let Some(tid) = r.get("PARAMETER_TYPE_ID") {
            if types.get(&tid).is_some_and(|(a, _)| a == "4") {
                values.entry(tid).or_default().push(r);
            }
        }
    }
    for (tid, mut list) in values {
        // Measured: ETS orders by DISPLAY_ORDER, an empty one counting as
        // 0, and keeps the file's row order among equals (a stable sort).
        list.sort_by_key(|r| r.int("DISPLAY_ORDER").unwrap_or(0));
        let type_id = format!("{}_PT-{tid}", program.id);
        let mut used = BTreeSet::new();
        for (order, r) in list.iter().enumerate() {
            let Some(value) = r.get("REAL_VALUE") else {
                skip(
                    diagnostics,
                    "parameter_list_of_values",
                    "REAL_VALUE is empty",
                );
                continue;
            };
            // ETS keys an enumeration by its value; a repeated value keeps
            // its own row id so neither is lost.
            let mut id = format!("{type_id}_EN-{value}");
            if !used.insert(id.clone()) {
                id = format!("{id}-{}", r.get("PARAMETER_VALUE_ID").unwrap_or_default());
                used.insert(id.clone());
            }
            program.enumerations.push(MappedEnumeration {
                parameter_type_id: type_id.clone(),
                id,
                value,
                text: r.get("DISPLAYED_VALUE"),
                display_order: order as i64,
            });
        }
    }
    types
}

struct PlacedParameter {
    /// `PARAMETER_ID` in the file.
    vd_id: String,
    number: String,
    ref_id: String,
    atomic: String,
    text: Option<String>,
    parent: Option<String>,
    parent_value: Option<String>,
    order: i64,
}

fn map_parameters(
    document: &ExImDocument,
    program: &mut MappedProgram,
    types: &BTreeMap<String, (String, Option<i64>)>,
    diagnostics: &mut Vec<MappingDiagnostic>,
) -> Vec<PlacedParameter> {
    let mut vd = Vec::new();
    for r in rows(document, "parameter") {
        if r.get("PROGRAM_ID").as_deref() != Some(program.exim_program_id.as_str()) {
            continue;
        }
        let (Some(id), Some(number), Some(type_id)) = (
            r.get("PARAMETER_ID"),
            r.get("PARAMETER_NUMBER"),
            r.get("PARAMETER_TYPE_ID"),
        ) else {
            let reason = match (r.get("PARAMETER_ID"), r.get("PARAMETER_NUMBER")) {
                (None, _) => "PARAMETER_ID is empty",
                (_, None) => "PARAMETER_NUMBER is empty",
                _ => "PARAMETER_TYPE_ID is empty",
            };
            skip(diagnostics, "parameter", reason);
            continue;
        };
        let Some((atomic, type_size)) = types.get(&type_id) else {
            diagnostics.push(MappingDiagnostic::DanglingReference {
                table: "parameter".into(),
                column: "PARAMETER_TYPE_ID".into(),
                value: type_id,
            });
            continue;
        };
        let level = r.get("PARAMETER_HIGH_ACCESS");
        let mapped_access = access(level.as_deref()).unwrap_or_else(|| {
            diagnostics.push(MappingDiagnostic::UnknownAccessLevel {
                parameter: number.clone(),
                level: level.clone().unwrap_or_default(),
            });
            "None"
        });
        vd.push(VdParameter {
            row: r,
            id,
            type_id,
            atomic: atomic.clone(),
            // `[V]` Address 0 means "no memory": ETS4's conversion of the
            // Siemens `.vd5` places none of its 3,576 such rows, and
            // grouping them as one memory cell merged unrelated parameters
            // (ADR-0094, L4).
            address: r.int("PARAMETER_ADDRESS").filter(|address| *address != 0),
            bit: r.int("PARAMETER_BITOFFSET").unwrap_or(0),
            size: r.int("PARAMETER_SIZE").or(*type_size),
            access: mapped_access,
            default: r.text("PARAMETER_DEFAULT_LONG"),
            text: r.text("PARAMETER_DESCRIPTION"),
            parent: r.get("PAR_PARAMETER_ID"),
            parent_value: r.get("PARENT_PARM_VALUE"),
            order: r.int("PARAMETER_DISPLAY_ORDER").unwrap_or(0),
            number,
        });
    }
    vd.sort_by_key(|p| number_key(&p.number));

    // Group by memory cell and type; parameters without memory stand alone.
    let mut groups: BTreeMap<(i64, i64, i64, String), Vec<usize>> = BTreeMap::new();
    let mut singles = Vec::new();
    for (i, p) in vd.iter().enumerate() {
        match p.address {
            Some(address) => groups
                .entry((address, p.bit, p.size.unwrap_or(0), p.type_id.clone()))
                .or_default()
                .push(i),
            None => singles.push(i),
        }
    }
    // Groups starting at the same bit form a union; overlap from another
    // start cannot be expressed and is reported.
    // (size, type id, member indexes) per (address, bit offset).
    type CellGroup = (i64, String, Vec<usize>);
    let mut by_start: BTreeMap<(i64, i64), Vec<CellGroup>> = BTreeMap::new();
    for ((address, bit, size, type_id), members) in groups {
        by_start
            .entry((address, bit))
            .or_default()
            .push((size, type_id, members));
    }
    let vd_ref = &vd;
    let spans: Vec<(i64, i64, String)> = by_start
        .iter()
        .flat_map(|((address, bit), list)| {
            list.iter().map(move |(size, _, members)| {
                let start = address * 8 + bit;
                (
                    start,
                    start + size.max(&1),
                    vd_ref[members[0]].number.clone(),
                )
            })
        })
        .collect();
    let mut overlapping = BTreeSet::new();
    for (i, a) in spans.iter().enumerate() {
        for b in &spans[i + 1..] {
            if a.0 != b.0 && a.0 < b.1 && b.0 < a.1 {
                overlapping.insert(a.2.clone());
                overlapping.insert(b.2.clone());
            }
        }
    }
    if !overlapping.is_empty() {
        diagnostics.push(MappingDiagnostic::OverlappingMemory {
            parameters: overlapping.into_iter().collect(),
        });
    }

    let segment = format!("{}_AS-0000", program.id);
    let mut placed = Vec::new();
    let mut union_seq = 0;
    let emit_group = |members: &[usize],
                      union: Option<(i64, i64)>,
                      program: &mut MappedProgram,
                      placed: &mut Vec<PlacedParameter>| {
        let rep = &vd[members[0]];
        let prefix = if union.is_some() { "UP" } else { "P" };
        let parameter_id = format!("{}_{prefix}-{}", program.id, rep.number);
        program.parameters.push(MappedParameter {
            id: parameter_id.clone(),
            name: rep.row.text("PARAMETER_NAME"),
            text: rep.text.clone(),
            parameter_type_id: format!("{}_PT-{}", program.id, rep.type_id),
            access: Some(rep.access.to_string()),
            value: rep.default.clone(),
            code_segment: rep.address.map(|_| segment.clone()),
            offset: rep.address,
            bit_offset: rep.address.map(|_| rep.bit),
            union_id: union.map(|u| u.0),
            union_size_in_bit: union.map(|u| u.1),
        });
        for &m in members {
            let p = &vd[m];
            let ref_id = format!("{parameter_id}_R-{}", p.number);
            program.parameter_refs.push(MappedParameterRef {
                id: ref_id.clone(),
                parameter_id: parameter_id.clone(),
                display_order: Some(p.order),
                tag: Some(p.number.clone()),
                text: p.text.clone().filter(|t| Some(t) != rep.text.as_ref()),
                value: p
                    .default
                    .clone()
                    .filter(|v| Some(v) != rep.default.as_ref()),
                access: Some(p.access.to_string()).filter(|a| a != rep.access),
            });
            placed.push(PlacedParameter {
                vd_id: p.id.clone(),
                number: p.number.clone(),
                ref_id,
                atomic: p.atomic.clone(),
                text: p.text.clone(),
                parent: p.parent.clone(),
                parent_value: p.parent_value.clone(),
                order: p.order,
            });
        }
    };
    for list in by_start.values() {
        if list.len() > 1 {
            union_seq += 1;
            let size = list.iter().map(|(s, _, _)| *s).max().unwrap_or(0);
            let mut ordered: Vec<_> = list.iter().collect();
            ordered.sort_by_key(|(_, _, m)| number_key(&vd[m[0]].number));
            for (_, _, members) in ordered {
                emit_group(members, Some((union_seq, size)), program, &mut placed);
            }
        } else {
            emit_group(&list[0].2, None, program, &mut placed);
        }
    }
    for i in singles {
        emit_group(&[i], None, program, &mut placed);
    }
    program
        .parameters
        .sort_by_key(|p| number_key(p.id.rsplit('-').next().unwrap_or("")));
    program
        .parameter_refs
        .sort_by_key(|r| number_key(r.tag.as_deref().unwrap_or("")));
    placed.sort_by_key(|p| number_key(&p.number));
    placed
}

fn map_objects(
    document: &ExImDocument,
    program: &mut MappedProgram,
    diagnostics: &mut Vec<MappingDiagnostic>,
) -> Vec<VdObject> {
    let sizes: BTreeMap<String, i64> = rows(document, "object_type")
        .iter()
        .filter_map(|r| Some((r.get("OBJECT_TYPE_CODE")?, r.int("LENGTH_IN_BIT")?)))
        .collect();
    let priorities: BTreeMap<String, String> = rows(document, "object_priority")
        .iter()
        .filter_map(|r| {
            Some((
                r.get("OBJECT_PRIORITY_CODE")?,
                r.get("OBJECT_PRIORITY_NAME")?,
            ))
        })
        .collect();
    let mut by_number: BTreeMap<i64, Vec<(String, Row)>> = BTreeMap::new();
    let mut datapoint_rows = 0;
    for r in rows(document, "communication_object") {
        if r.get("PROGRAM_ID").as_deref() != Some(program.exim_program_id.as_str()) {
            continue;
        }
        let (Some(number), Some(unique)) = (r.int("OBJECT_NUMBER"), r.get("OBJECT_UNIQUE_NUMBER"))
        else {
            skip(
                diagnostics,
                "communication_object",
                "OBJECT_NUMBER or OBJECT_UNIQUE_NUMBER is empty",
            );
            continue;
        };
        if r.get("EIB_DATA_TYPE_CODE").is_some() {
            datapoint_rows += 1;
        }
        by_number.entry(number).or_default().push((unique, r));
    }
    if datapoint_rows > 0 {
        diagnostics.push(MappingDiagnostic::UnmappedColumn {
            table: "communication_object".into(),
            column: "EIB_DATA_TYPE_CODE".into(),
            rows: datapoint_rows,
        });
    }
    let size_of = |r: &Row, diagnostics: &mut Vec<MappingDiagnostic>| {
        let code = r.get("OBJECT_TYPE")?;
        let found = sizes.get(&code).and_then(|bits| object_size(*bits));
        if found.is_none() {
            diagnostics.push(MappingDiagnostic::DanglingReference {
                table: "communication_object".into(),
                column: "OBJECT_TYPE".into(),
                value: code,
            });
        }
        found
    };
    let mut placed = Vec::new();
    for (number, mut members) in by_number {
        members.sort_by_key(|(unique, _)| number_key(unique));
        let rep = members[0].1;
        let id = format!("{}_O-{number}", program.id);
        let fields = |r: &Row, size: Option<String>| {
            [
                r.text("OBJECT_NAME"),
                r.text("OBJECT_FUNCTION"),
                size,
                r.get("OBJECT_PRIORITY")
                    .and_then(|p| priorities.get(&p).cloned()),
                flag(r.get("OBJECT_READENABLED")),
                flag(r.get("OBJECT_WRITEENABLED")),
                flag(r.get("OBJECT_TRANSENABLED")),
                flag(r.get("OBJECT_UPDATEENABLED")),
                flag(r.get("OBJECT_COMMENABLED")),
                flag(r.get("OBJECT_READONINITENABLED")),
            ]
        };
        let rep_size = size_of(&rep, diagnostics);
        let base = fields(&rep, rep_size);
        let [name, function, size, priority, read, write, transmit, update, communication, read_on_init] =
            base.clone();
        program.com_objects.push(MappedComObject {
            id: id.clone(),
            number: Some(number),
            name: name.clone(),
            text: name,
            function_text: function,
            object_size: size,
            priority,
            read_flag: read,
            write_flag: write,
            transmit_flag: transmit,
            update_flag: update,
            communication_flag: communication,
            read_on_init_flag: read_on_init,
        });
        for (unique, r) in &members {
            let own_size = size_of(r, diagnostics);
            let own = fields(r, own_size);
            let differs = |i: usize| own[i].clone().filter(|_| own[i] != base[i]);
            let ref_id = format!("{id}_R-{unique}");
            program.com_object_refs.push(MappedComObjectRef {
                id: ref_id.clone(),
                com_object_id: id.clone(),
                tag: Some(unique.clone()),
                text: differs(0),
                function_text: differs(1),
                object_size: differs(2),
                priority: differs(3),
                read_flag: differs(4),
                write_flag: differs(5),
                transmit_flag: differs(6),
                update_flag: differs(7),
                communication_flag: differs(8),
                read_on_init_flag: differs(9),
            });
            placed.push(VdObject {
                ref_id,
                unique: unique.clone(),
                number,
                parent: r.get("PARAMETER_ID"),
                parent_value: r.get("PARENT_PARAMETER_VALUE"),
                order: r.int("OBJECT_DISPLAY_ORDER").unwrap_or(0),
            });
        }
    }
    placed
}

/// Builds the `Dynamic` tree (ADR-0094). A parameter's children with an
/// empty parent value follow it inline (visible whenever it is); children
/// with a value sit under `choose` on it, one `when` per value. Pages are
/// parameters of atomic type 0 without a parent; other roots join the
/// first page, as ETS's conversion does.
fn build_tree(
    program_id: &str,
    parameters: &[PlacedParameter],
    objects: &[VdObject],
    diagnostics: &mut Vec<MappingDiagnostic>,
) -> Vec<MappedDynamicNode> {
    let known: BTreeSet<&str> = parameters.iter().map(|p| p.vd_id.as_str()).collect();
    let mut child_params: BTreeMap<&str, Vec<&PlacedParameter>> = BTreeMap::new();
    let mut child_objects: BTreeMap<&str, Vec<&VdObject>> = BTreeMap::new();
    let mut root_params = Vec::new();
    let mut root_objects = Vec::new();
    for p in parameters {
        match p.parent.as_deref() {
            Some(parent) if known.contains(parent) => {
                child_params.entry(parent).or_default().push(p)
            }
            Some(parent) => {
                diagnostics.push(MappingDiagnostic::DanglingReference {
                    table: "parameter".into(),
                    column: "PAR_PARAMETER_ID".into(),
                    value: parent.into(),
                });
                root_params.push(p);
            }
            None => root_params.push(p),
        }
    }
    for o in objects {
        match o.parent.as_deref() {
            Some(parent) if known.contains(parent) => {
                child_objects.entry(parent).or_default().push(o)
            }
            Some(parent) => {
                diagnostics.push(MappingDiagnostic::DanglingReference {
                    table: "communication_object".into(),
                    column: "PARAMETER_ID".into(),
                    value: parent.into(),
                });
                root_objects.push(o);
            }
            None => root_objects.push(o),
        }
    }
    let param_key = |p: &&PlacedParameter| (p.order, number_key(&p.number));
    let object_key = |o: &&VdObject| (o.order, o.number, number_key(&o.unique));
    root_params.sort_by_key(param_key);
    root_objects.sort_by_key(object_key);
    for list in child_params.values_mut() {
        list.sort_by_key(param_key);
    }
    for list in child_objects.values_mut() {
        list.sort_by_key(object_key);
    }

    let mut tree = Tree {
        nodes: Vec::new(),
        child_params: &child_params,
        child_objects: &child_objects,
        placed: BTreeSet::new(),
        diagnostics,
    };
    let root = tree.push(None, "Dynamic", None, None, None, false, None);
    let channel = tree.push(Some(root), "Channel", None, None, None, false, None);
    for o in &root_objects {
        tree.object(channel, o);
    }
    let (pages, others): (Vec<_>, Vec<_>) = root_params.into_iter().partition(|p| p.atomic == "0");
    if pages.is_empty() && !others.is_empty() {
        tree.diagnostics.push(MappingDiagnostic::NoPage {
            program: program_id.into(),
        });
    }
    for (i, page) in pages.iter().enumerate() {
        tree.placed.insert(page.vd_id.clone());
        let block = tree.push(
            Some(channel),
            "ParameterBlock",
            Some(format!("{program_id}_PB-{}", page.number)),
            Some(page.ref_id.clone()),
            None,
            false,
            page.text.clone(),
        );
        tree.children(block, page, true, 1);
        if i == 0 {
            for p in &others {
                tree.parameter(block, p, 1);
            }
        }
    }
    if pages.is_empty() {
        for p in &others {
            tree.parameter(channel, p, 1);
        }
    }
    for p in parameters {
        if !tree.placed.contains(&p.vd_id) {
            tree.diagnostics.push(MappingDiagnostic::UnplacedParameter {
                parameter: p.number.clone(),
            });
        }
    }
    tree.nodes
}

/// How deep a parent chain may nest in the tree. Real files nest about 6
/// levels; the bound keeps a hostile file from exhausting the stack, and
/// whatever lies deeper is reported as unplaced.
const MAX_TREE_DEPTH: usize = 64;

struct Tree<'a, 'd> {
    nodes: Vec<MappedDynamicNode>,
    child_params: &'a BTreeMap<&'a str, Vec<&'a PlacedParameter>>,
    child_objects: &'a BTreeMap<&'a str, Vec<&'a VdObject>>,
    placed: BTreeSet<String>,
    diagnostics: &'d mut Vec<MappingDiagnostic>,
}

impl Tree<'_, '_> {
    #[allow(clippy::too_many_arguments)]
    fn push(
        &mut self,
        parent: Option<i64>,
        kind: &str,
        element_id: Option<String>,
        ref_id: Option<String>,
        test: Option<String>,
        is_default: bool,
        text: Option<String>,
    ) -> i64 {
        let node_id = self.nodes.len() as i64 + 1;
        let position = self.nodes.iter().filter(|n| n.parent_id == parent).count() as i64;
        self.nodes.push(MappedDynamicNode {
            node_id,
            parent_id: parent,
            position,
            kind: kind.into(),
            element_id,
            ref_id,
            test,
            is_default,
            text,
        });
        node_id
    }

    fn object(&mut self, parent: i64, o: &VdObject) {
        self.push(
            parent.into(),
            "ComObjectRefRef",
            None,
            Some(o.ref_id.clone()),
            None,
            false,
            None,
        );
    }

    fn parameter(&mut self, parent: i64, p: &PlacedParameter, depth: usize) {
        // A cycle in the parent chain would recurse forever and a long chain
        // would exhaust the stack; a parameter is placed once and only up to
        // the depth bound, and one left out is reported as unplaced.
        if depth > MAX_TREE_DEPTH || !self.placed.insert(p.vd_id.clone()) {
            return;
        }
        self.push(
            Some(parent),
            "ParameterRefRef",
            None,
            Some(p.ref_id.clone()),
            None,
            false,
            None,
        );
        self.children(parent, p, false, depth + 1);
    }

    /// Places `p`'s children under `parent`: the unconditional ones inline,
    /// the conditional ones under one `choose` on `p`.
    fn children(&mut self, parent: i64, p: &PlacedParameter, is_page: bool, depth: usize) {
        let params = self
            .child_params
            .get(p.vd_id.as_str())
            .cloned()
            .unwrap_or_default();
        let objects = self
            .child_objects
            .get(p.vd_id.as_str())
            .cloned()
            .unwrap_or_default();
        let conditional = |v: &Option<String>| v.is_some() && !is_page;
        if is_page
            && (params.iter().any(|c| c.parent_value.is_some())
                || objects.iter().any(|c| c.parent_value.is_some()))
        {
            self.diagnostics.push(MappingDiagnostic::ValueOnPageParent {
                parameter: p.number.clone(),
            });
        }
        for c in params.iter().filter(|c| !conditional(&c.parent_value)) {
            self.parameter(parent, c, depth);
        }
        for o in objects.iter().filter(|o| !conditional(&o.parent_value)) {
            self.object(parent, o);
        }
        let mut values: BTreeSet<(i64, String)> = BTreeSet::new();
        values.extend(
            params
                .iter()
                .filter_map(|c| c.parent_value.as_deref())
                .map(number_key),
        );
        values.extend(
            objects
                .iter()
                .filter_map(|o| o.parent_value.as_deref())
                .map(number_key),
        );
        if values.is_empty() || is_page {
            return;
        }
        let choose = self.push(
            Some(parent),
            "choose",
            None,
            Some(p.ref_id.clone()),
            None,
            false,
            None,
        );
        for (_, value) in values {
            let when = self.push(
                Some(choose),
                "when",
                None,
                None,
                Some(value.clone()),
                false,
                None,
            );
            for c in params
                .iter()
                .filter(|c| c.parent_value.as_deref() == Some(value.as_str()))
            {
                self.parameter(when, c, depth);
            }
            for o in objects
                .iter()
                .filter(|o| o.parent_value.as_deref() == Some(value.as_str()))
            {
                self.object(when, o);
            }
        }
    }
}

/// Registers where each `text_attribute` row of this program lands.
pub(super) fn translation_targets(
    document: &ExImDocument,
    _ns: &str,
    program: &MappedProgram,
    targets: &mut TranslationTargets,
) {
    let pid = program.exim_program_id.as_str();
    let program_scope = |element: &str, attribute: &str, unless: Option<&str>| TranslationTarget {
        scope: "Program".into(),
        scope_id: program.id.clone(),
        element: element.into(),
        attribute: attribute.into(),
        unless_same_as: unless.map(str::to_string),
    };
    targets.insert(
        ("80".into(), pid.to_string()),
        vec![program_scope(&program.id, "Name", None)],
    );
    // VD PARAMETER_ID of each group's representative, by parameter id.
    let mut representative_entity: BTreeMap<String, String> = BTreeMap::new();
    let refs: BTreeMap<&str, &MappedParameterRef> = program
        .parameter_refs
        .iter()
        .filter_map(|r| Some((r.tag.as_deref()?, r)))
        .collect();
    let mut members = Vec::new();
    for r in rows(document, "parameter") {
        if r.get("PROGRAM_ID").as_deref() != Some(pid) {
            continue;
        }
        let (Some(id), Some(number)) = (r.get("PARAMETER_ID"), r.get("PARAMETER_NUMBER")) else {
            continue;
        };
        let Some(pref) = refs.get(number.as_str()) else {
            continue;
        };
        if pref.parameter_id.rsplit('-').next() == Some(number.as_str()) {
            representative_entity.insert(pref.parameter_id.clone(), id.clone());
        }
        members.push((id, pref));
    }
    // A member translates its own ref. Its translation is left out only
    // where it adds nothing: the ref keeps the shared text (no override) and
    // the translation equals the representative's. A ref that overrides the
    // text keeps every translation, or another language would show the
    // override's language.
    for (id, pref) in members {
        let target = match representative_entity.get(&pref.parameter_id) {
            Some(rep) if *rep == id => program_scope(&pref.parameter_id, "Text", None),
            _ if pref.text.is_some() => program_scope(&pref.id, "Text", None),
            rep => program_scope(&pref.id, "Text", rep.map(String::as_str)),
        };
        targets.insert(("10".into(), id), vec![target]);
    }
    let enum_ids: BTreeMap<(String, String), &str> = program
        .enumerations
        .iter()
        .map(|e| {
            (
                (e.parameter_type_id.clone(), e.value.clone()),
                e.id.as_str(),
            )
        })
        .collect();
    for r in rows(document, "parameter_list_of_values") {
        let (Some(tid), Some(value), Some(value_id)) = (
            r.get("PARAMETER_TYPE_ID"),
            r.get("REAL_VALUE"),
            r.get("PARAMETER_VALUE_ID"),
        ) else {
            continue;
        };
        let type_id = format!("{}_PT-{tid}", program.id);
        if let Some(id) = enum_ids.get(&(type_id, value)) {
            targets.insert(
                ("11".into(), value_id),
                vec![program_scope(id, "Text", None)],
            );
        }
    }
    // The representative of each object group is its first ref; its VD
    // OBJECT_ID is the entity members compare against.
    let mut object_rows = Vec::new();
    for r in rows(document, "communication_object") {
        if r.get("PROGRAM_ID").as_deref() != Some(pid) {
            continue;
        }
        if let (Some(id), Some(unique)) = (r.get("OBJECT_ID"), r.get("OBJECT_UNIQUE_NUMBER")) {
            object_rows.push((id, unique));
        }
    }
    let by_unique: BTreeMap<&str, &str> = object_rows
        .iter()
        .map(|(id, unique)| (unique.as_str(), id.as_str()))
        .collect();
    for (id, unique) in &object_rows {
        let Some(oref) = program
            .com_object_refs
            .iter()
            .find(|c| c.tag.as_deref() == Some(unique.as_str()))
        else {
            continue;
        };
        let first = program
            .com_object_refs
            .iter()
            .find(|c| c.com_object_id == oref.com_object_id)
            .expect("a ref's own group");
        let rep_entity = first.tag.as_deref().and_then(|t| by_unique.get(t)).copied();
        for (column, attribute, overridden) in [
            ("20", "Text", oref.text.is_some()),
            ("22", "FunctionText", oref.function_text.is_some()),
        ] {
            let target = if first.id == oref.id {
                program_scope(&oref.com_object_id, attribute, None)
            } else if overridden {
                program_scope(&oref.id, attribute, None)
            } else {
                program_scope(&oref.id, attribute, rep_entity)
            };
            targets.insert((column.into(), id.clone()), vec![target]);
        }
    }
}
