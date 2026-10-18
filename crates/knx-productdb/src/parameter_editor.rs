//! Reads bounded, exact-owner parameter type evidence without mutating product data.
//!
//! Attribute preservation does not claim a value encoding or editor contract.

use std::collections::BTreeMap;

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::{NsReader, XmlVersion};
use rusqlite::{Connection, OptionalExtension};

use crate::{sha256_hex, ProductDbError};

/// Maximum source size inspected for this optional UI evidence: the storage
/// ceiling, so a retained program is never too large to inspect (ADR-0142).
/// 199 of 1,462 measured programs exceed the former 8 MiB; the largest is
/// 281.8 MiB.
pub const MAX_EDITOR_SOURCE_BYTES: usize = knx_core::limits::MAX_RETAINED_BYTES_USIZE;
/// Every XML event consumes at least one input byte, so the source bound is
/// also the walk bound.
const MAX_EVENTS: usize = MAX_EDITOR_SOURCE_BYTES;
const MAX_DEPTH: usize = 64;
const MAX_ATTRIBUTES: usize = 64;
const MAX_ATTRIBUTE_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EditorTypeSource {
    pub source_sha256: String,
    pub program_id: String,
    pub parameter_type_id: String,
    pub schema: u16,
    pub child: String,
    /// Attribute names remain qualified when the original name was qualified.
    pub attributes: BTreeMap<String, String>,
    pub parameter_type_attributes: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditorEvidenceIssue {
    MissingReference,
    MissingSource,
    SourceBudget,
    SourceIdentity,
    UnsupportedXml,
    AmbiguousDefinition,
    ModelMismatch,
    MissingDeclaration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterEditorEvidence {
    Declared(EditorTypeSource),
    Unavailable(EditorEvidenceIssue),
}

type TypeRow = (
    String,
    String,
    Option<i64>,
    Option<String>,
    Option<String>,
    String,
    String,
);

/// Resolves an exact program-local ParameterRef, not a display-name heuristic.
/// SQLite failures remain errors; optional source admission has named fallbacks.
pub fn parameter_editor_evidence(
    conn: &Connection,
    program_id: &str,
    parameter_ref_id: &str,
) -> Result<ParameterEditorEvidence, ProductDbError> {
    let row: Option<TypeRow> = conn
        .query_row(
            "SELECT pt.id,pt.kind,pt.size_in_bit,pt.min_inclusive,pt.max_inclusive,a.source_sha256,p.id
         FROM parameter_ref pr
         JOIN parameter p ON p.program_id=pr.program_id AND p.id=pr.parameter_id
         JOIN parameter_type pt ON pt.program_id=p.program_id AND pt.id=p.parameter_type_id
         JOIN application_program a ON a.id=pr.program_id
         WHERE pr.program_id=?1 AND pr.id=?2",
            [program_id, parameter_ref_id],
            |r| {
                Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                ))
            },
        )
        .optional()?;
    let Some((type_id, kind, size, min, max, sha, parameter_id)) = row else {
        return Ok(unavailable(EditorEvidenceIssue::MissingReference));
    };
    let lengths: Option<(i64, i64)> = conn
        .query_row(
            "SELECT len,length(bytes) FROM source_file WHERE sha256=?1",
            [&sha],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((declared, actual)) = lengths else {
        return Ok(unavailable(EditorEvidenceIssue::MissingSource));
    };
    if actual < 0 || actual > MAX_EDITOR_SOURCE_BYTES as i64 {
        return Ok(unavailable(EditorEvidenceIssue::SourceBudget));
    }
    if declared != actual {
        return Ok(unavailable(EditorEvidenceIssue::SourceIdentity));
    }
    let bytes: Vec<u8> = conn.query_row(
        "SELECT bytes FROM source_file WHERE sha256=?1",
        [&sha],
        |r| r.get(0),
    )?;
    if bytes.len() != actual as usize || sha256_hex(&bytes) != sha {
        return Ok(unavailable(EditorEvidenceIssue::SourceIdentity));
    }
    let parsed = match read_source(
        &bytes,
        program_id,
        &type_id,
        &parameter_id,
        parameter_ref_id,
    ) {
        Ok(v) => v,
        Err(issue) => return Ok(unavailable(issue)),
    };
    let expected = match kind.as_str() {
        "Time" => "TypeTime",
        "Number" => "TypeNumber",
        "Color" => "TypeColor",
        "Picture" => "TypePicture",
        "Float" => "TypeFloat",
        "Text" => "TypeText",
        "Restriction" => "TypeRestriction",
        "None" => "TypeNone",
        "IPAddress" => "TypeIPAddress",
        "Raw" => "TypeRawData",
        _ => return Ok(unavailable(EditorEvidenceIssue::ModelMismatch)),
    };
    let attrs = &parsed.attributes;
    let raw_size = attrs.get("SizeInBit").and_then(|s| s.parse::<i64>().ok());
    if parsed.child != expected
        || raw_size != size
        || attrs.get("minInclusive") != min.as_ref()
        || attrs.get("maxInclusive") != max.as_ref()
    {
        return Ok(unavailable(EditorEvidenceIssue::ModelMismatch));
    }
    Ok(ParameterEditorEvidence::Declared(EditorTypeSource {
        source_sha256: sha,
        program_id: program_id.into(),
        parameter_type_id: type_id,
        schema: parsed.schema,
        child: parsed.child,
        attributes: parsed.attributes,
        parameter_type_attributes: parsed.parent_attributes,
    }))
}

fn unavailable(issue: EditorEvidenceIssue) -> ParameterEditorEvidence {
    ParameterEditorEvidence::Unavailable(issue)
}

#[derive(Debug)]
struct Frame {
    name: String,
    known: bool,
    attributes: BTreeMap<String, String>,
}
struct Parsed {
    schema: u16,
    child: String,
    attributes: BTreeMap<String, String>,
    parent_attributes: BTreeMap<String, String>,
}
const PROGRAM_PATH: [&str; 5] = [
    "KNX",
    "ManufacturerData",
    "Manufacturer",
    "ApplicationPrograms",
    "ApplicationProgram",
];
const TYPE_PATH: [&str; 8] = [
    "KNX",
    "ManufacturerData",
    "Manufacturer",
    "ApplicationPrograms",
    "ApplicationProgram",
    "Static",
    "ParameterTypes",
    "ParameterType",
];
fn path(stack: &[Frame], names: &[&str]) -> bool {
    stack.len() == names.len()
        && stack
            .iter()
            .zip(names)
            .all(|(f, n)| f.known && f.name == *n)
}
fn raw_attributes(e: &BytesStart<'_>) -> Result<BTreeMap<String, String>, EditorEvidenceIssue> {
    let mut out = BTreeMap::new();
    for attr in e.attributes().with_checks(true) {
        let attr = attr.map_err(|_| EditorEvidenceIssue::UnsupportedXml)?;
        let name = attr.key.as_ref().to_owned();
        let value = attr
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|_| EditorEvidenceIssue::UnsupportedXml)?
            .into_owned();
        if out.len() >= MAX_ATTRIBUTES
            || name.len() > MAX_ATTRIBUTE_BYTES
            || value.len() > MAX_ATTRIBUTE_BYTES
        {
            return Err(EditorEvidenceIssue::SourceBudget);
        }
        if out.insert(name, value).is_some() {
            return Err(EditorEvidenceIssue::UnsupportedXml);
        }
    }
    Ok(out)
}
fn read_source(
    bytes: &[u8],
    program: &str,
    type_id: &str,
    parameter_id: &str,
    reference_id: &str,
) -> Result<Parsed, EditorEvidenceIssue> {
    let text = std::str::from_utf8(bytes).map_err(|_| EditorEvidenceIssue::UnsupportedXml)?;
    if text.chars().any(|c|!matches!(c,'\t'|'\n'|'\r'|'\u{20}'..='\u{D7FF}'|'\u{E000}'..='\u{FFFD}'|'\u{10000}'..='\u{10FFFF}')) {
        return Err(EditorEvidenceIssue::UnsupportedXml);
    }
    let mut reader = NsReader::from_reader(bytes);
    reader.config_mut().check_comments = true;
    let mut stack: Vec<Frame> = Vec::new();
    let mut namespace = Vec::new();
    let mut schema = 0;
    let mut programs = 0usize;
    let mut types = 0usize;
    let mut children = Vec::new();
    let mut references = Vec::new();
    let mut parameters = Vec::new();
    for _ in 0..MAX_EVENTS {
        let (resolved, event) = reader
            .read_resolved_event()
            .map_err(|_| EditorEvidenceIssue::UnsupportedXml)?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Decl(d) => {
                if d.version().ok().as_deref() != Some("1.0")
                    || d.encoding()
                        .is_some_and(|e| e.map_or(true, |v| !v.eq_ignore_ascii_case("utf-8")))
                {
                    return Err(EditorEvidenceIssue::UnsupportedXml);
                }
            }
            Event::DocType(_) => return Err(EditorEvidenceIssue::UnsupportedXml),
            Event::Start(e) | Event::Empty(e) => {
                let uri = match resolved {
                    ResolveResult::Bound(ns) => ns.as_ref().as_bytes().to_vec(),
                    _ => Vec::new(),
                };
                if stack.is_empty() {
                    namespace = uri.clone();
                    schema = match namespace.as_slice() {
                        b"http://knx.org/xml/project/20" => 20,
                        b"http://knx.org/xml/project/21" => 21,
                        b"http://knx.org/xml/project/22" => 22,
                        b"http://knx.org/xml/project/23" => 23,
                        _ => return Err(EditorEvidenceIssue::UnsupportedXml),
                    };
                }
                if stack.len() >= MAX_DEPTH {
                    return Err(EditorEvidenceIssue::SourceBudget);
                }
                stack.push(Frame {
                    name: e.local_name().as_ref().into(),
                    known: uri == namespace,
                    attributes: raw_attributes(&e)?,
                });
                if path(&stack, &PROGRAM_PATH)
                    && stack[4]
                        .attributes
                        .get("Id")
                        .is_some_and(|id| id == program)
                {
                    programs += 1;
                }
                let owner = stack.len() >= 5
                    && path(&stack[..5], &PROGRAM_PATH)
                    && stack[4]
                        .attributes
                        .get("Id")
                        .is_some_and(|id| id == program);
                if owner
                    && stack.len() == 8
                    && stack[..6]
                        .iter()
                        .zip(TYPE_PATH[..6].iter())
                        .all(|(f, n)| f.known && f.name == *n)
                    && stack[6].known
                    && stack[7].known
                {
                    let attrs = &stack[7].attributes;
                    if stack[6].name == "ParameterRefs"
                        && stack[7].name == "ParameterRef"
                        && attrs.get("Id").is_some_and(|id| id == reference_id)
                    {
                        references.push(attrs.get("RefId").cloned());
                    }
                    if stack[6].name == "Parameters"
                        && stack[7].name == "Parameter"
                        && attrs.get("Id").is_some_and(|id| id == parameter_id)
                    {
                        parameters.push(attrs.get("ParameterType").cloned());
                    }
                }
                if owner
                    && path(&stack, &TYPE_PATH)
                    && stack[7]
                        .attributes
                        .get("Id")
                        .is_some_and(|id| id == type_id)
                {
                    types += 1;
                }
                if owner
                    && stack.len() == 9
                    && path(&stack[..8], &TYPE_PATH)
                    && stack[7]
                        .attributes
                        .get("Id")
                        .is_some_and(|id| id == type_id)
                    && stack[8].known
                {
                    children.push(Parsed {
                        schema,
                        child: stack[8].name.clone(),
                        attributes: stack[8].attributes.clone(),
                        parent_attributes: stack[7].attributes.clone(),
                    });
                }
                if empty {
                    stack.pop();
                }
            }
            Event::End(_) => {
                if stack.pop().is_none() {
                    return Err(EditorEvidenceIssue::UnsupportedXml);
                }
            }
            Event::Eof => {
                if !stack.is_empty()
                    || crate::xml::validate_complete_document("parameter editor source", bytes)
                        .is_err()
                {
                    return Err(EditorEvidenceIssue::UnsupportedXml);
                }
                if programs > 1
                    || types > 1
                    || children.len() > 1
                    || references.len() > 1
                    || parameters.len() > 1
                {
                    return Err(EditorEvidenceIssue::AmbiguousDefinition);
                }
                if programs != 1 || types != 1 || children.len() != 1 {
                    return Err(EditorEvidenceIssue::MissingDeclaration);
                }
                if references.as_slice() != [Some(parameter_id.into())]
                    || parameters.as_slice() != [Some(type_id.into())]
                {
                    return Err(EditorEvidenceIssue::ModelMismatch);
                }
                return Ok(children.remove(0));
            }
            _ => {}
        }
    }
    Err(EditorEvidenceIssue::SourceBudget)
}
