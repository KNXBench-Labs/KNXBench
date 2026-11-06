//! Reads bounded literal folder declarations from the exact installed product source.

use std::collections::BTreeMap;

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::{NsReader, XmlVersion};
use rusqlite::{Connection, OptionalExtension};

use crate::dynamic::TextProjectionBudget;
use crate::{IdentityKind, ProductDbError};

#[cfg(test)]
mod tests;

pub const MAX_SOURCE_BYTES: usize = 32 * 1024 * 1024;
pub use crate::identity::admission::MAX_IDENTITY_ADMISSION_ROWS;
// Display-admission policy, not format limits. Visit every unscoped row once,
// including unrelated kinds; refuse the whole overlay rather than a prefix.
const MAX_NORMALIZED_ROWS: usize = 65_536;
const MAX_NORMALIZED_METADATA_BYTES: usize = 8 * 1024 * 1024;
const MAX_EVENTS: usize = 1_000_000;
const MAX_DEPTH: usize = 64;
const MAX_IDENTITIES: usize = 8192;
const MAX_ATTRIBUTE_BYTES: usize = 4096;
const NS: &str = "http://knx.org/xml/project/20";
const PROGRAM_PATH: [&str; 5] = [
    "KNX",
    "ManufacturerData",
    "Manufacturer",
    "ApplicationPrograms",
    "ApplicationProgram",
];

/// Saved identity components only; labels never participate in a join.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FolderPathNode {
    pub kind: String,
    pub ref_id: String,
}

/// Current installed-source declaration, not import-time or active-placement evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderCaption {
    pub text: String,
    pub application_program_id: String,
    pub parameter_block_id: String,
    pub source_sha256: String,
}

pub type FolderCaptions = BTreeMap<Vec<FolderPathNode>, FolderCaption>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptionIssue {
    MissingSource,
    SourceIdentity,
    UnsupportedXml,
    ResourceLimit,
    UnsupportedSchema,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CaptionEvidence {
    Declared(FolderCaptions),
    Unavailable(CaptionIssue),
}

/// Reads once per device request, after releasing the project/opaque locks.
/// Optional evidence failures never change saved placement; SQLite failures remain errors.
pub fn folder_captions(
    conn: &Connection,
    product: &str,
    hardware2program: &str,
    project_schema: u32,
) -> Result<CaptionEvidence, ProductDbError> {
    if !matches!(project_schema, 20 | 23) {
        return Ok(CaptionEvidence::Unavailable(
            CaptionIssue::UnsupportedSchema,
        ));
    }
    match read_owned(conn, product, hardware2program) {
        Ok(v) => Ok(v),
        Err(ProductDbError::Xml { .. }) => {
            Ok(CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity))
        }
        Err(e) => Err(e),
    }
}

fn read_owned(
    conn: &Connection,
    product: &str,
    hardware2program: &str,
) -> Result<CaptionEvidence, ProductDbError> {
    type Owner = (
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
        String,
    );
    if product.len() > MAX_ATTRIBUTE_BYTES || hardware2program.len() > MAX_ATTRIBUTE_BYTES {
        return Ok(CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit));
    }
    let owner: Option<Result<Owner, CaptionIssue>> = conn
        .query_row(
            "SELECT p.hardware_id,hp.hardware_id,hp.application_program_ref,
         p.source_sha256,hp.source_sha256,h.source_sha256,ap.source_sha256,h.id,ap.manufacturer_id
         FROM product p JOIN hardware2program hp ON hp.id=?2
         JOIN hardware h ON h.id=p.hardware_id
         JOIN application_program ap ON ap.id=hp.application_program_ref WHERE p.id=?1",
            [product, hardware2program],
            |r| {
                // Inspect borrowed values before copying any owner metadata.
                for column in 0..9 {
                    let rusqlite::types::ValueRef::Text(value) = r.get_ref(column)? else {
                        return Ok(Err(CaptionIssue::SourceIdentity));
                    };
                    if value.len() > MAX_ATTRIBUTE_BYTES {
                        return Ok(Err(CaptionIssue::ResourceLimit));
                    }
                }
                Ok(Ok((
                    r.get(0)?,
                    r.get(1)?,
                    r.get(2)?,
                    r.get(3)?,
                    r.get(4)?,
                    r.get(5)?,
                    r.get(6)?,
                    r.get(7)?,
                    r.get(8)?,
                )))
            },
        )
        .optional()?;
    let Some(owner) = owner else {
        return Ok(CaptionEvidence::Unavailable(CaptionIssue::MissingSource));
    };
    let (ph, hh, program, psha, hpsha, hsha, sha, hardware, manufacturer) = match owner {
        Ok(owner) => owner,
        Err(issue) => return Ok(CaptionEvidence::Unavailable(issue)),
    };
    if ph != hh {
        return Ok(CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity));
    }
    for (kind, id, source) in [
        (IdentityKind::Product, product, psha.as_str()),
        (
            IdentityKind::Hardware2Program,
            hardware2program,
            hpsha.as_str(),
        ),
        (IdentityKind::Hardware, hardware.as_str(), hsha.as_str()),
        (
            IdentityKind::ApplicationProgram,
            program.as_str(),
            sha.as_str(),
        ),
    ] {
        use crate::identity::admission::{admit, Admission};
        match admit(conn, kind, id, source)? {
            Admission::Admitted => {}
            Admission::SourceIdentity => {
                return Ok(CaptionEvidence::Unavailable(CaptionIssue::SourceIdentity));
            }
            Admission::ResourceLimit => {
                return Ok(CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit));
            }
        }
    }
    let hardware_bytes = match verified_source(conn, &hpsha)? {
        Ok(bytes) => bytes,
        Err(issue) => return Ok(CaptionEvidence::Unavailable(issue)),
    };
    // Product and H2P can have distinct winning members; never substitute one
    // member's otherwise valid declarations for the other's source authority.
    let product_link = if psha == hpsha {
        hardware_owner_link(
            &hardware_bytes,
            HardwareOwner::Product(product),
            &hardware,
            &manufacturer,
        )
    } else {
        let product_bytes = match verified_source(conn, &psha)? {
            Ok(bytes) => bytes,
            Err(issue) => return Ok(CaptionEvidence::Unavailable(issue)),
        };
        hardware_owner_link(
            &product_bytes,
            HardwareOwner::Product(product),
            &hardware,
            &manufacturer,
        )
    };
    if let Err(issue) = product_link {
        return Ok(CaptionEvidence::Unavailable(issue));
    }
    if let Err(issue) = hardware_program_link(
        &hardware_bytes,
        hardware2program,
        &hardware,
        &program,
        &manufacturer,
    ) {
        return Ok(CaptionEvidence::Unavailable(issue));
    }
    let bytes = match verified_source(conn, &sha)? {
        Ok(bytes) => bytes,
        Err(issue) => return Ok(CaptionEvidence::Unavailable(issue)),
    };
    let parsed = match read_source(&bytes, &program, &manufacturer) {
        Ok(v) => v,
        Err(issue) => return Ok(CaptionEvidence::Unavailable(issue)),
    };
    let corroborated = match corroborate(conn, &program, &parsed)? {
        Ok(v) => v,
        Err(issue) => return Ok(CaptionEvidence::Unavailable(issue)),
    };
    let mut output = BTreeMap::new();
    let mut budget = TextProjectionBudget::default();
    for (candidate, consistent) in parsed.into_iter().zip(corroborated) {
        if !consistent {
            continue;
        }
        for text in [&candidate.text, &program, &candidate.id, &sha] {
            if budget.admit_copy(text).is_err() {
                return Ok(CaptionEvidence::Unavailable(CaptionIssue::ResourceLimit));
            }
        }
        output.insert(
            candidate.path,
            FolderCaption {
                text: candidate.text,
                application_program_id: program.clone(),
                parameter_block_id: candidate.id,
                source_sha256: sha.clone(),
            },
        );
    }
    Ok(CaptionEvidence::Declared(output))
}

/// One indexed program/scope range walk, never one scan per caption. Only
/// borrowed, size-checked PB metadata is compared; retain two small flags per
/// already bounded candidate, not normalized strings or diagnostic payloads.
fn corroborate(
    conn: &Connection,
    program: &str,
    candidates: &[Candidate],
) -> Result<Result<Vec<bool>, CaptionIssue>, ProductDbError> {
    let mut matches: BTreeMap<&str, (&str, usize, bool)> = candidates
        .iter()
        .map(|c| (c.id.as_str(), (c.text.as_str(), 0, false)))
        .collect();
    let mut stmt = conn.prepare(
        "SELECT kind='ParameterBlock',
         CASE WHEN kind='ParameterBlock' AND length(CAST(element_id AS BLOB))<=4096 THEN element_id END,
         CASE WHEN kind='ParameterBlock' AND length(CAST(text AS BLOB))<=4096 THEN text END,
         text_parameter_ref_id IS NULL,ref_id IS NULL,
         CASE WHEN kind='ParameterBlock' THEN length(CAST(element_id AS BLOB)) END,
         CASE WHEN kind='ParameterBlock' THEN length(CAST(text AS BLOB)) END
         FROM dynamic_node WHERE program_id=?1 AND module_def_id='' LIMIT ?2",
    )?;
    let mut rows = stmt.query(rusqlite::params![program, (MAX_NORMALIZED_ROWS + 1) as i64])?;
    let mut visited = 0;
    let mut metadata_bytes = 0usize;
    while let Some(row) = rows.next()? {
        visited += 1;
        if visited > MAX_NORMALIZED_ROWS {
            return Ok(Err(CaptionIssue::ResourceLimit));
        }
        if !row.get::<_, bool>(0)? {
            continue;
        }
        for column in [5, 6] {
            let bytes: Option<i64> = row.get(column)?;
            if let Some(bytes) = bytes {
                if !(0..=MAX_ATTRIBUTE_BYTES as i64).contains(&bytes) {
                    return Ok(Err(CaptionIssue::ResourceLimit));
                }
                metadata_bytes += bytes as usize;
                if metadata_bytes > MAX_NORMALIZED_METADATA_BYTES {
                    return Ok(Err(CaptionIssue::ResourceLimit));
                }
            }
        }
        let Some(id) = row.get_ref(1)?.as_str_or_null().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(1, rusqlite::types::Type::Text, Box::new(e))
        })?
        else {
            continue;
        };
        let Some((expected, count, consistent)) = matches.get_mut(id) else {
            continue;
        };
        *count = (*count + 1).min(2);
        let text = row.get_ref(2)?.as_str_or_null().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(2, rusqlite::types::Type::Text, Box::new(e))
        })?;
        // Lookup uses the exact full element identity, never a suffix or extra.
        *consistent = text == Some(*expected) && row.get::<_, bool>(3)? && row.get::<_, bool>(4)?;
    }
    Ok(Ok(candidates
        .iter()
        .map(|c| {
            let (_, count, consistent) = matches[c.id.as_str()];
            count == 1 && consistent
        })
        .collect()))
}

fn verified_source(
    conn: &Connection,
    sha: &str,
) -> Result<Result<Vec<u8>, CaptionIssue>, ProductDbError> {
    let lengths: Option<(i64, i64)> = conn
        .query_row(
            "SELECT len,length(bytes) FROM source_file WHERE sha256=?1",
            [sha],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let Some((declared, actual)) = lengths else {
        return Ok(Err(CaptionIssue::MissingSource));
    };
    if actual < 0 || actual > MAX_SOURCE_BYTES as i64 {
        return Ok(Err(CaptionIssue::ResourceLimit));
    }
    if declared != actual {
        return Ok(Err(CaptionIssue::SourceIdentity));
    }
    let bytes: Option<Vec<u8>> = conn.query_row(
        "SELECT bytes FROM source_file WHERE sha256=?1 AND length(bytes)=?2 AND length(bytes)<=?3",
        rusqlite::params![sha,actual,MAX_SOURCE_BYTES as i64], |r| r.get(0),
    ).optional()?;
    let Some(bytes) = bytes else {
        return Ok(Err(CaptionIssue::SourceIdentity));
    };
    if bytes.len() != actual as usize || crate::sha256_hex(&bytes) != sha {
        return Ok(Err(CaptionIssue::SourceIdentity));
    }
    Ok(Ok(bytes))
}

enum HardwareOwner<'a> {
    Product(&'a str),
    Program { hp: &'a str, program: &'a str },
}

/// Keep the existing H2P discriminator on the same canonical bounded walker.
fn hardware_program_link(
    bytes: &[u8],
    hp: &str,
    hardware: &str,
    program: &str,
    manufacturer: &str,
) -> Result<(), CaptionIssue> {
    hardware_owner_link(
        bytes,
        HardwareOwner::Program { hp, program },
        hardware,
        manufacturer,
    )
}

/// Measured local-name/parser-context rows alone do not authorize either edge.
fn hardware_owner_link(
    bytes: &[u8],
    owner: HardwareOwner<'_>,
    hardware: &str,
    manufacturer: &str,
) -> Result<(), CaptionIssue> {
    let (container, element, id, expected_program) = match owner {
        HardwareOwner::Product(id) => ("Products", "Product", id, None),
        HardwareOwner::Program { hp, program } => {
            ("Hardware2Programs", "Hardware2Program", hp, Some(program))
        }
    };
    let owner_path = [
        "KNX",
        "ManufacturerData",
        "Manufacturer",
        "Hardware",
        "Hardware",
        container,
        element,
    ];
    let mut reader = NsReader::from_reader(bytes);
    reader.config_mut().check_comments = true;
    let mut stack = Vec::<Frame>::new();
    let mut owners = 0usize;
    let mut references = 0usize;
    for _ in 0..MAX_EVENTS {
        let (ns, event) = reader
            .read_resolved_event()
            .map_err(|_| CaptionIssue::UnsupportedXml)?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::DocType(_) => return Err(CaptionIssue::UnsupportedXml),
            Event::Start(e) | Event::Empty(e) => {
                if stack.len() >= MAX_DEPTH {
                    return Err(CaptionIssue::ResourceLimit);
                }
                stack.push(read_frame(
                    &e,
                    matches!(ns,ResolveResult::Bound(ref uri) if uri.as_ref()==NS),
                )?);
                let current = stack.last().expect("pushed frame");
                let selected = stack.len() >= 7
                    && path(&stack[..7], &owner_path)
                    && stack[6].attrs.get("Id").map(String::as_str) == Some(id);
                if current.name == element
                    && current.attrs.get("Id").map(String::as_str) == Some(id)
                {
                    if !selected
                        || qualified(&current.attrs)
                        || stack[2].attrs.get("RefId").map(String::as_str) != Some(manufacturer)
                        || qualified(&stack[2].attrs)
                        || stack[4].attrs.get("Id").map(String::as_str) != Some(hardware)
                        || qualified(&stack[4].attrs)
                    {
                        return Err(CaptionIssue::SourceIdentity);
                    }
                    owners += 1;
                }
                if expected_program.is_some()
                    && selected
                    && stack.len() == 8
                    && current.name == "ApplicationProgramRef"
                {
                    references += 1;
                    if !current.known
                        || qualified(&current.attrs)
                        || current.attrs.get("RefId").map(String::as_str) != expected_program
                    {
                        return Err(CaptionIssue::SourceIdentity);
                    }
                }
                if empty {
                    stack.pop();
                }
            }
            Event::End(_) => {
                if stack.pop().is_none() {
                    return Err(CaptionIssue::UnsupportedXml);
                }
            }
            Event::Eof => {
                return if owners == 1
                    && references == usize::from(expected_program.is_some())
                    && stack.is_empty()
                    && crate::xml::validate_complete_document(
                        "folder caption hardware owner",
                        bytes,
                    )
                    .is_ok()
                {
                    Ok(())
                } else {
                    Err(CaptionIssue::SourceIdentity)
                }
            }
            _ => {}
        }
    }
    Err(CaptionIssue::ResourceLimit)
}

fn read_frame(e: &quick_xml::events::BytesStart<'_>, known: bool) -> Result<Frame, CaptionIssue> {
    let mut attrs = BTreeMap::new();
    for a in e.attributes().with_checks(true) {
        let a = a.map_err(|_| CaptionIssue::UnsupportedXml)?;
        if attrs.len() >= 64 || a.key.as_ref().len() > 256 || a.value.len() > MAX_ATTRIBUTE_BYTES {
            return Err(CaptionIssue::ResourceLimit);
        }
        let value = a
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|_| CaptionIssue::UnsupportedXml)?;
        attrs.insert(a.key.as_ref().to_owned(), value.into_owned());
    }
    Ok(Frame {
        name: e.local_name().as_ref().to_owned(),
        attrs,
        known,
    })
}

struct Frame {
    name: String,
    attrs: BTreeMap<String, String>,
    known: bool,
}
struct Candidate {
    id: String,
    text: String,
    path: Vec<FolderPathNode>,
    ancestors: Vec<String>,
}
fn path(stack: &[Frame], names: &[&str]) -> bool {
    stack.len() == names.len()
        && stack
            .iter()
            .zip(names)
            .all(|(f, n)| f.known && f.name == *n)
}
fn qualified(attrs: &BTreeMap<String, String>) -> bool {
    attrs.keys().any(|name| {
        name.split_once(':').is_some_and(|(prefix, local)| {
            prefix != "xmlns"
                && matches!(
                    local,
                    "Id" | "RefId"
                        | "Text"
                        | "Access"
                        | "ShowInComObjectTree"
                        | "TextParameterRefId"
                )
        })
    })
}
fn accessible(f: &Frame) -> bool {
    !qualified(&f.attrs)
        && matches!(
            f.attrs.get("Access").map(String::as_str),
            None | Some("Read" | "ReadWrite")
        )
}
fn relative<'a>(id: &'a str, program: &str) -> Option<&'a str> {
    id.strip_prefix(program)?
        .strip_prefix('_')
        .filter(|s| !s.is_empty())
}

fn read_source(
    bytes: &[u8],
    program: &str,
    manufacturer: &str,
) -> Result<Vec<Candidate>, CaptionIssue> {
    let text = std::str::from_utf8(bytes).map_err(|_| CaptionIssue::UnsupportedXml)?;
    if text.chars().any(|c| !matches!(c, '\t'|'\n'|'\r'|'\u{20}'..='\u{D7FF}'|'\u{E000}'..='\u{FFFD}'|'\u{10000}'..='\u{10FFFF}')) {
        return Err(CaptionIssue::UnsupportedXml);
    }
    let mut reader = NsReader::from_reader(bytes);
    reader.config_mut().check_comments = true;
    let mut stack = Vec::<Frame>::new();
    let mut programs = 0usize;
    let mut dynamics = 0usize;
    let mut roots = 0usize;
    let mut identities = BTreeMap::<String, usize>::new();
    let mut candidates = Vec::<Candidate>::new();
    let mut budget = TextProjectionBudget::default();
    for _ in 0..MAX_EVENTS {
        let (ns, event) = reader
            .read_resolved_event()
            .map_err(|_| CaptionIssue::UnsupportedXml)?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::DocType(_) => return Err(CaptionIssue::UnsupportedXml),
            Event::Decl(d) => {
                if roots != 0
                    || d.version().ok().as_deref() != Some("1.0")
                    || d.encoding()
                        .is_some_and(|e| e.map_or(true, |v| !v.eq_ignore_ascii_case("utf-8")))
                {
                    return Err(CaptionIssue::UnsupportedXml);
                }
            }
            Event::Start(e) | Event::Empty(e) => {
                if stack.len() >= MAX_DEPTH {
                    return Err(CaptionIssue::ResourceLimit);
                }
                let known = matches!(ns, ResolveResult::Bound(ref uri) if uri.as_ref() == NS);
                if stack.is_empty() {
                    roots += 1;
                    if roots != 1 || !known || e.local_name().as_ref() != "KNX" {
                        return Err(CaptionIssue::UnsupportedXml);
                    }
                }
                stack.push(read_frame(&e, known)?);
                let owner = stack.len() >= 5
                    && path(&stack[..5], &PROGRAM_PATH)
                    && stack[4].attrs.get("Id").is_some_and(|id| id == program);
                if stack.last().is_some_and(|f| {
                    f.name == "ApplicationProgram"
                        && f.attrs.get("Id").is_some_and(|id| id == program)
                }) && !owner
                {
                    return Err(CaptionIssue::SourceIdentity);
                }
                if owner {
                    if stack[2].attrs.get("RefId").map(String::as_str) != Some(manufacturer)
                        || qualified(&stack[2].attrs)
                        || qualified(&stack[4].attrs)
                    {
                        return Err(CaptionIssue::SourceIdentity);
                    }
                    if stack.len() == 5 {
                        programs += 1;
                    }
                    if stack.len() == 6 && stack[5].known && stack[5].name == "Dynamic" {
                        dynamics += 1;
                    }
                    if stack.len() > 6 && stack[5].known && stack[5].name == "Dynamic" {
                        let current = stack.last().expect("pushed frame");
                        if current.known
                            && matches!(current.name.as_str(), "Rename" | "ParameterBlockRename")
                        {
                            return Err(CaptionIssue::UnsupportedXml);
                        }
                        if matches!(current.name.as_str(), "Channel" | "ParameterBlock") {
                            if let Some(id) = current.attrs.get("Id") {
                                if identities.len() >= MAX_IDENTITIES
                                    && !identities.contains_key(id)
                                {
                                    return Err(CaptionIssue::ResourceLimit);
                                }
                                *identities.entry(id.clone()).or_default() += 1;
                            }
                        }
                        if current.name == "ParameterBlock" && current.known {
                            if let Some(candidate) = candidate(&stack, program) {
                                for s in std::iter::once(&candidate.text)
                                    .chain(candidate.ancestors.iter())
                                    .chain(candidate.path.iter().flat_map(|n| [&n.kind, &n.ref_id]))
                                {
                                    budget
                                        .admit_copy(s)
                                        .map_err(|_| CaptionIssue::ResourceLimit)?;
                                }
                                if candidates.len() >= MAX_IDENTITIES {
                                    return Err(CaptionIssue::ResourceLimit);
                                }
                                candidates.push(candidate);
                            }
                        }
                    }
                }
                if empty {
                    stack.pop();
                }
            }
            Event::End(_) => {
                if stack.pop().is_none() {
                    return Err(CaptionIssue::UnsupportedXml);
                }
            }
            Event::Eof => {
                if !stack.is_empty()
                    || roots != 1
                    || programs != 1
                    || dynamics != 1
                    || crate::xml::validate_complete_document("folder caption source", bytes)
                        .is_err()
                {
                    return Err(CaptionIssue::UnsupportedXml);
                }
                let mut paths = BTreeMap::<Vec<FolderPathNode>, usize>::new();
                for c in &candidates {
                    *paths.entry(c.path.clone()).or_default() += 1;
                }
                candidates.retain(|c| {
                    paths.get(&c.path) == Some(&1)
                        && c.ancestors.iter().all(|id| identities.get(id) == Some(&1))
                });
                return Ok(candidates);
            }
            _ => {}
        }
    }
    Err(CaptionIssue::ResourceLimit)
}

fn candidate(stack: &[Frame], program: &str) -> Option<Candidate> {
    let current = stack.last()?;
    let text = current.attrs.get("Text")?;
    if text.trim().is_empty()
        || text.len() > 4096
        || text.contains("{{")
        || text.contains("}}")
        || current.attrs.contains_key("TextParameterRefId")
        || !matches!(
            current.attrs.get("ShowInComObjectTree").map(String::as_str),
            Some("true" | "1")
        )
    {
        return None;
    }
    let mut shown = Vec::new();
    let mut ancestors = Vec::new();
    for frame in &stack[5..] {
        if !frame.known
            || !accessible(frame)
            || !matches!(
                frame.name.as_str(),
                "Dynamic"
                    | "Channel"
                    | "ChannelIndependentBlock"
                    | "ParameterBlock"
                    | "choose"
                    | "when"
            )
        {
            return None;
        }
        if matches!(frame.name.as_str(), "Channel" | "ParameterBlock") {
            if frame.attrs.contains_key("RefId") {
                return None;
            }
            let id = frame.attrs.get("Id")?;
            let rel = relative(id, program)?;
            ancestors.push(id.clone());
            if frame.name == "Channel"
                || matches!(
                    frame.attrs.get("ShowInComObjectTree").map(String::as_str),
                    Some("true" | "1")
                )
            {
                shown.push(FolderPathNode {
                    kind: if frame.name == "Channel" {
                        "Channel"
                    } else {
                        "Folder"
                    }
                    .into(),
                    ref_id: rel.into(),
                });
            }
        }
    }
    Some(Candidate {
        id: current.attrs.get("Id")?.clone(),
        text: text.clone(),
        path: shown,
        ancestors,
    })
}
