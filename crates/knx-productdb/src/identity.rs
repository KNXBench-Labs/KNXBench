//! Package identity: element digests per candidate, their parser agreement, and queries.
//!
//! ADR-0043. For every element of the six identity-bearing kinds in every
//! parsed member blob, `source_identity` records `(blob, kind, id,
//! occurrence, digest)`. The rows are a pure function of the blob's bytes,
//! so the recorded set does not depend on install order, even though the
//! first-installed winner does.
//!
//! The scan mirrors the domain parsers' dispatch exactly (`parse/catalog.rs`,
//! `parse/hardware.rs`, `parse/program.rs`), and every ingest checks it
//! against what the parser actually did: a mismatch fails the ingest.
//!
//! The digest is SHA-256 over a canonical token stream. Every token is one
//! tag byte followed by fields; a field is its UTF-8 byte length as a `u64`
//! little-endian, then its bytes. Tokens:
//!
//! - `S` qualified name, attribute count (decimal), then per attribute
//!   sorted by raw qualified name: raw name, value normalized exactly as
//!   `crate::xml::attrs` normalizes it. `<X/>` is `S` then `E`, as `<X></X>`.
//! - `E` qualified name.
//! - `T` text: the character data between two element tags, with
//!   references resolved, CDATA taken literally and line endings of the
//!   literal pieces normalized to `\n`; a run of only XML whitespace is
//!   dropped. Comments and processing instructions do not split a run.
//! - `K` table name, logical id: a nested tracked element, in place of its
//!   content, which goes to that element's own digest.
//!
//! Comments, processing instructions, the XML declaration and DOCTYPE are
//! not hashed. Names are compared as written; namespaces are not resolved.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::{NsReader, Reader, XmlVersion};
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};

use crate::ingest::FileKind;
use crate::report::IdConflict;
use crate::ProductDbError;

/// The six package-content kinds that carry a first-winner identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IdentityKind {
    CatalogSection,
    CatalogItem,
    Hardware,
    Product,
    Hardware2Program,
    ApplicationProgram,
}

impl IdentityKind {
    pub const ALL: [IdentityKind; 6] = [
        IdentityKind::CatalogSection,
        IdentityKind::CatalogItem,
        IdentityKind::Hardware,
        IdentityKind::Product,
        IdentityKind::Hardware2Program,
        IdentityKind::ApplicationProgram,
    ];

    /// The typed table holding this kind's winning rows, which is also the
    /// `source_identity.table_name` value.
    pub const fn as_table(self) -> &'static str {
        match self {
            IdentityKind::CatalogSection => "catalog_section",
            IdentityKind::CatalogItem => "catalog_item",
            IdentityKind::Hardware => "hardware",
            IdentityKind::Product => "product",
            IdentityKind::Hardware2Program => "hardware2program",
            IdentityKind::ApplicationProgram => "application_program",
        }
    }

    pub fn parse(table: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_table() == table)
    }
}

/// One element of an identity-bearing kind found in one blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub table: IdentityKind,
    pub logical_id: String,
    pub occurrence: u32,
    pub digest: String,
}

/// `(kind, logical id, occurrence)` of one recorded candidate.
pub(crate) type CandidateKey = (IdentityKind, String, u32);

/// Which parser's dispatch the scan mirrors, decided exactly as ingest
/// decides which parser runs (`ingest::classify`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Dispatch {
    Catalog,
    Hardware,
    Program,
}

/// One open tracked element. Its hasher receives every token until the
/// element closes, except the content of a nested tracked element.
struct Frame {
    hasher: Sha256,
    /// Element depth of the tracked element itself.
    depth: usize,
    /// Index into the candidate list, whose digest is filled on close.
    index: usize,
}

const TOKEN_START: u8 = b'S';
const TOKEN_END: u8 = b'E';
const TOKEN_TEXT: u8 = b'T';
const TOKEN_MARKER: u8 = b'K';

fn field(hasher: &mut Sha256, value: &str) {
    hasher.update((value.len() as u64).to_le_bytes());
    hasher.update(value.as_bytes());
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn scan_error(source_path: &str, cause: impl std::fmt::Display) -> ProductDbError {
    ProductDbError::Xml {
        source_path: source_path.to_string(),
        cause: format!("identity scan: {cause}"),
    }
}

/// `\r\n` and a lone `\r` become `\n` (XML 1.0 §2.11). Applied to each
/// literal piece, so a `&#13;` character reference stays a `\r`.
fn push_normalized(out: &mut String, literal: &str) {
    let mut chars = literal.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(ch);
        }
    }
}

/// The five predefined entities and legal decimal or hexadecimal character
/// references. Anything else is a scan error: the parsers honour no DTD.
fn resolve_reference(name: &str) -> Option<char> {
    match name {
        "lt" => return Some('<'),
        "gt" => return Some('>'),
        "amp" => return Some('&'),
        "apos" => return Some('\''),
        "quot" => return Some('"'),
        _ => {}
    }
    let code = if let Some(hex) = name.strip_prefix("#x") {
        if hex.is_empty() || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return None;
        }
        u32::from_str_radix(hex, 16).ok()?
    } else {
        let decimal = name.strip_prefix('#')?;
        if decimal.is_empty() || !decimal.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        decimal.parse().ok()?
    };
    let legal = matches!(code, 0x9 | 0xA | 0xD)
        || (0x20..=0xD7FF).contains(&code)
        || (0xE000..=0xFFFD).contains(&code)
        || (0x10000..=0x10FFFF).contains(&code);
    if legal {
        char::from_u32(code)
    } else {
        None
    }
}

fn is_xml_whitespace(text: &str) -> bool {
    text.bytes()
        .all(|b| matches!(b, b' ' | b'\t' | b'\n' | b'\r'))
}

/// Hands the pending character data to the innermost open frame, unless it
/// is only XML whitespace, and clears it.
fn flush_text(text: &mut String, frames: &mut [Frame]) {
    if !text.is_empty() && !is_xml_whitespace(text) {
        if let Some(frame) = frames.last_mut() {
            frame.hasher.update([TOKEN_TEXT]);
            field(&mut frame.hasher, text);
        }
    }
    text.clear();
}

fn start_token(hasher: &mut Sha256, qname: &str, attributes: &crate::xml::Attrs) {
    hasher.update([TOKEN_START]);
    field(hasher, qname);
    let names: Vec<&str> = attributes.names().collect();
    field(hasher, &names.len().to_string());
    // `names()` iterates the raw qualified names in byte order.
    for name in names {
        field(hasher, name);
        field(hasher, attributes.evidence_value(name).unwrap_or_default());
    }
}

/// What the domain parser for this blob's kind dispatches `element` as.
/// Advances the program parser's `Dynamic` skip and parameter-type state
/// exactly as `parse/program.rs` does.
fn dispatch_element(
    dispatch: Dispatch,
    local: &str,
    attributes: &crate::xml::Attrs,
    is_start: bool,
    depth: usize,
    skipped_dynamic: &mut Option<usize>,
    expecting_type_child: &mut bool,
) -> Option<IdentityKind> {
    if skipped_dynamic.is_some() {
        return None;
    }
    match dispatch {
        Dispatch::Catalog => match local {
            "CatalogSection" => Some(IdentityKind::CatalogSection),
            "CatalogItem" => Some(IdentityKind::CatalogItem),
            _ => None,
        },
        // The outer `<Hardware>` is the collection, the inner the entity.
        Dispatch::Hardware => match local {
            "Hardware" if attributes.get("Id").is_some() => Some(IdentityKind::Hardware),
            "Product" => Some(IdentityKind::Product),
            "Hardware2Program" => Some(IdentityKind::Hardware2Program),
            _ => None,
        },
        Dispatch::Program => {
            if is_start && local == "Dynamic" {
                // The parser's first match arm skips the subtree before the
                // parameter-type child is ever looked at, so a skipped
                // `Dynamic` does not consume that child.
                *skipped_dynamic = Some(depth);
                None
            } else if *expecting_type_child {
                *expecting_type_child = false;
                None
            } else {
                match local {
                    "ApplicationProgram" => Some(IdentityKind::ApplicationProgram),
                    "ParameterType" => {
                        *expecting_type_child = true;
                        None
                    }
                    _ => None,
                }
            }
        }
    }
}

/// Streams one blob and returns every identity candidate in document
/// order, each with its element digest. Mirrors the parser that ingest runs
/// for this blob's kind; a blob of any other kind has no candidates. One
/// pass, no tree: memory is bounded by the nesting depth.
pub(crate) fn scan_identities(
    source_path: &str,
    bytes: &[u8],
) -> Result<Vec<Candidate>, ProductDbError> {
    let dispatch = match crate::ingest::classify(bytes) {
        FileKind::Catalog => Dispatch::Catalog,
        FileKind::Hardware => Dispatch::Hardware,
        FileKind::ApplicationProgram => Dispatch::Program,
        _ => return Ok(Vec::new()),
    };
    // The parsers' own reader and configuration, so both see one tokenization.
    let mut reader = Reader::from_reader(bytes);
    let mut buf = Vec::new();
    let mut candidates: Vec<Candidate> = Vec::new();
    let mut occurrences: HashMap<(IdentityKind, String), u32> = HashMap::new();
    let mut frames: Vec<Frame> = Vec::new();
    let mut depth = 0usize;
    let mut text = String::new();
    let mut skipped_dynamic: Option<usize> = None;
    let mut expecting_type_child = false;

    loop {
        buf.clear();
        let event = reader
            .read_event_into(&mut buf)
            .map_err(|e| scan_error(source_path, e))?;
        let (element, is_start): (BytesStart, bool) = match event {
            Event::Start(e) => (e, true),
            Event::Empty(e) => (e, false),
            Event::End(e) => {
                flush_text(&mut text, &mut frames);
                if let Some(frame) = frames.last_mut() {
                    frame.hasher.update([TOKEN_END]);
                    field(&mut frame.hasher, e.name().as_ref());
                }
                if let Some(frame) = frames.pop_if(|frame| frame.depth == depth) {
                    candidates[frame.index].digest = hex(&frame.hasher.finalize());
                }
                if skipped_dynamic == Some(depth) {
                    skipped_dynamic = None;
                }
                depth = depth
                    .checked_sub(1)
                    .ok_or_else(|| scan_error(source_path, "end tag outside the root"))?;
                continue;
            }
            Event::Text(piece) => {
                push_normalized(&mut text, &piece);
                continue;
            }
            Event::CData(piece) => {
                push_normalized(&mut text, &piece);
                continue;
            }
            Event::GeneralRef(reference) => {
                let name: &str = &reference;
                let resolved = resolve_reference(name).ok_or_else(|| {
                    scan_error(source_path, format!("undeclared XML entity {name:?}"))
                })?;
                text.push(resolved);
                continue;
            }
            Event::Eof => break,
            Event::Comment(_) | Event::PI(_) | Event::Decl(_) | Event::DocType(_) => continue,
        };
        flush_text(&mut text, &mut frames);
        if is_start {
            depth += 1;
        }
        let qname = element.name().as_ref().to_string();
        let local = crate::xml::local_name(&element);
        let attributes = crate::xml::attrs(&element, source_path)?;
        let tracked = dispatch_element(
            dispatch,
            &local,
            &attributes,
            is_start,
            depth,
            &mut skipped_dynamic,
            &mut expecting_type_child,
        );
        if let Some(kind) = tracked {
            let logical_id = attributes.get("Id").unwrap_or_default().to_string();
            let occurrence = {
                let count = occurrences.entry((kind, logical_id.clone())).or_insert(0);
                *count = count
                    .checked_add(1)
                    .ok_or_else(|| scan_error(source_path, "occurrence counter overflow"))?;
                *count
            };
            if let Some(parent) = frames.last_mut() {
                parent.hasher.update([TOKEN_MARKER]);
                field(&mut parent.hasher, kind.as_table());
                field(&mut parent.hasher, &logical_id);
            }
            candidates.push(Candidate {
                table: kind,
                logical_id,
                occurrence,
                digest: String::new(),
            });
            frames.push(Frame {
                hasher: Sha256::new(),
                depth,
                index: candidates.len() - 1,
            });
        }
        if let Some(frame) = frames.last_mut() {
            start_token(&mut frame.hasher, &qname, &attributes);
            if !is_start {
                frame.hasher.update([TOKEN_END]);
                field(&mut frame.hasher, &qname);
            }
        }
        if !is_start && tracked.is_some() {
            let frame = frames.pop().expect("pushed for this element");
            candidates[frame.index].digest = hex(&frame.hasher.finalize());
        }
    }
    if !frames.is_empty() || depth != 0 {
        return Err(scan_error(source_path, "incomplete XML document"));
    }
    Ok(candidates)
}

/// Outcome of looking at a blob's recorded identity scan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ScanStatus {
    Measured,
    Unavailable(String),
}

/// Scans `bytes` and records the result under `sha256`: `measured` with
/// every candidate row, or `unavailable` with the scan's reason. A scan
/// failure is never an error here; a database failure is.
pub(crate) fn record_scan(
    conn: &Connection,
    sha256: &str,
    source_path: &str,
    bytes: &[u8],
) -> Result<ScanStatus, ProductDbError> {
    match scan_identities(source_path, bytes) {
        Ok(candidates) => {
            conn.execute(
                "INSERT INTO source_identity_scan (source_sha256, status, reason) VALUES (?1, 'measured', NULL)",
                [sha256],
            )?;
            let mut insert = conn.prepare(
                "INSERT INTO source_identity (source_sha256, table_name, logical_id, occurrence, digest)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )?;
            for candidate in candidates {
                insert.execute(params![
                    sha256,
                    candidate.table.as_table(),
                    candidate.logical_id,
                    candidate.occurrence,
                    candidate.digest,
                ])?;
            }
            Ok(ScanStatus::Measured)
        }
        Err(ProductDbError::Sqlite(error)) => Err(ProductDbError::Sqlite(error)),
        Err(error) => {
            let reason = error.to_string();
            record_unavailable(conn, sha256, &reason)?;
            Ok(ScanStatus::Unavailable(reason))
        }
    }
}

pub(crate) fn record_unavailable(
    conn: &Connection,
    sha256: &str,
    reason: &str,
) -> Result<(), ProductDbError> {
    conn.execute(
        "INSERT INTO source_identity_scan (source_sha256, status, reason) VALUES (?1, 'unavailable', ?2)",
        params![sha256, reason],
    )?;
    Ok(())
}

fn stored_status(conn: &Connection, sha256: &str) -> Result<Option<ScanStatus>, ProductDbError> {
    let row: Option<(String, Option<String>)> = conn
        .query_row(
            "SELECT status, reason FROM source_identity_scan WHERE source_sha256 = ?1",
            [sha256],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    row.map(|(status, reason)| match (status.as_str(), reason) {
        ("measured", None) => Ok(ScanStatus::Measured),
        ("unavailable", Some(reason)) => Ok(ScanStatus::Unavailable(reason)),
        _ => Err(corrupt("source_identity_scan row has an invalid status")),
    })
    .transpose()
}

fn corrupt(cause: impl Into<String>) -> ProductDbError {
    ProductDbError::Xml {
        source_path: "package identity".into(),
        cause: cause.into(),
    }
}

fn disagreement(source_path: &str, cause: impl std::fmt::Display) -> ProductDbError {
    ProductDbError::Xml {
        source_path: source_path.to_string(),
        cause: format!("identity scan disagrees with parser: {cause}"),
    }
}

fn stored_keys(conn: &Connection, sha256: &str) -> Result<HashSet<CandidateKey>, ProductDbError> {
    let rows = conn
        .prepare(
            "SELECT table_name, logical_id, occurrence FROM source_identity WHERE source_sha256 = ?1",
        )?
        .query_map([sha256], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    rows.into_iter()
        .map(|(table, id, occurrence)| {
            let kind = IdentityKind::parse(&table)
                .ok_or_else(|| corrupt(format!("unknown identity table {table:?}")))?;
            let occurrence = u32::try_from(occurrence)
                .ok()
                .filter(|o| *o >= 1)
                .ok_or_else(|| corrupt("identity occurrence out of range"))?;
            Ok((kind, id, occurrence))
        })
        .collect()
}

/// After a domain parser ran over `sha256`: records its identity scan if
/// none is recorded yet, then checks the measured candidates against what
/// the parser did (`check_agreement`). An `unavailable` scan is not
/// checked and never fails the ingest.
pub(crate) fn record_and_check(
    conn: &Connection,
    sha256: &str,
    source_path: &str,
    bytes: &[u8],
    conflicts: &[IdConflict],
) -> Result<(), ProductDbError> {
    let status = match stored_status(conn, sha256)? {
        Some(status) => status,
        None => record_scan(conn, sha256, source_path, bytes)?,
    };
    if status == ScanStatus::Measured {
        let candidates = stored_keys(conn, sha256)?;
        check_agreement(conn, sha256, source_path, &candidates, conflicts)?;
    }
    Ok(())
}

/// The parsers and the scan must agree (ADR-0043 §4):
///
/// a. every typed row this blob won, with a non-empty id, has the blob's
///    first-occurrence candidate;
/// b. every `IdConflict` this pass produced for this blob has the candidate
///    of exactly that occurrence;
/// c. every candidate with a non-empty id has a typed row of that id.
///
/// Empty ids are candidates too, but the parsers store them as `NULL` or
/// `''`, so a. and c. cannot pair them.
pub(crate) fn check_agreement(
    conn: &Connection,
    sha256: &str,
    source_path: &str,
    candidates: &HashSet<CandidateKey>,
    conflicts: &[IdConflict],
) -> Result<(), ProductDbError> {
    for kind in IdentityKind::ALL {
        let table = kind.as_table();
        let won = conn
            .prepare(&format!(
                "SELECT id FROM {table} WHERE source_sha256 = ?1 AND id IS NOT NULL AND id <> ''"
            ))?
            .query_map([sha256], |r| r.get::<_, String>(0))?
            .collect::<Result<Vec<_>, _>>()?;
        for id in won {
            if !candidates.contains(&(kind, id.clone(), 1)) {
                return Err(disagreement(
                    source_path,
                    format!("{table} row {id:?} won by this blob has no candidate"),
                ));
            }
        }
    }
    for conflict in conflicts.iter().filter(|c| c.other_sha256 == sha256) {
        let kind = IdentityKind::parse(&conflict.table).ok_or_else(|| {
            disagreement(
                source_path,
                format!("conflict names unknown table {:?}", conflict.table),
            )
        })?;
        if !candidates.contains(&(kind, conflict.id.clone(), conflict.occurrence)) {
            return Err(disagreement(
                source_path,
                format!(
                    "{} conflict {:?} occurrence {} has no candidate",
                    conflict.table, conflict.id, conflict.occurrence
                ),
            ));
        }
    }
    for (kind, id, _) in candidates.iter().filter(|(_, id, _)| !id.is_empty()) {
        let table = kind.as_table();
        let present: Option<i64> = conn
            .query_row(&format!("SELECT 1 FROM {table} WHERE id = ?1"), [id], |r| {
                r.get(0)
            })
            .optional()?;
        if present.is_none() {
            return Err(disagreement(
                source_path,
                format!("candidate {table} {id:?} has no typed row"),
            ));
        }
    }
    Ok(())
}

/// Records `KNX/@CreatedBy`, `@ToolVersion` and the root namespace of a
/// newly stored blob (ADR-0043 §5). Only unprefixed attributes count; a
/// blob whose first element is not `KNX`, or that is not XML, gets no row
/// and no error. The facts are source strings, never compared or ordered.
pub(crate) fn record_producer(
    conn: &Connection,
    sha256: &str,
    bytes: &[u8],
) -> Result<(), ProductDbError> {
    let Some((namespace, created_by, tool_version)) = producer_facts(bytes) else {
        return Ok(());
    };
    conn.execute(
        "INSERT OR IGNORE INTO source_producer (source_sha256, root_namespace, created_by, tool_version)
         VALUES (?1, ?2, ?3, ?4)",
        params![sha256, namespace, created_by, tool_version],
    )?;
    Ok(())
}

type ProducerFacts = (Option<String>, Option<String>, Option<String>);

fn producer_facts(bytes: &[u8]) -> Option<ProducerFacts> {
    let text = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    let mut reader = NsReader::from_reader(text);
    loop {
        let (resolved, event) = reader.read_resolved_event().ok()?;
        match event {
            Event::Start(root) | Event::Empty(root) => {
                if root.local_name().as_ref() != "KNX" {
                    return None;
                }
                let namespace = match resolved {
                    ResolveResult::Bound(namespace) => Some(namespace.as_ref().to_string()),
                    _ => None,
                };
                let mut created_by = None;
                let mut tool_version = None;
                for attribute in root.attributes() {
                    let attribute = attribute.ok()?;
                    let slot = match attribute.key.as_ref() {
                        "CreatedBy" => &mut created_by,
                        "ToolVersion" => &mut tool_version,
                        _ => continue,
                    };
                    *slot = Some(
                        attribute
                            .normalized_value(XmlVersion::Implicit1_0)
                            .ok()?
                            .into_owned(),
                    );
                }
                return Some((namespace, created_by, tool_version));
            }
            Event::Text(text) if is_xml_whitespace(&text) => {}
            Event::Decl(_) | Event::Comment(_) | Event::PI(_) | Event::DocType(_) => {}
            _ => return None,
        }
    }
}

// ---------------------------------------------------------------------------
// Queries
// ---------------------------------------------------------------------------

/// Every source name a package arrived under, sorted.
pub fn package_source_names(
    conn: &Connection,
    package_sha256: &str,
) -> Result<Vec<String>, ProductDbError> {
    Ok(conn
        .prepare(
            "SELECT source_name FROM package_source_name WHERE package_sha256 = ?1 ORDER BY source_name",
        )?
        .query_map([package_sha256], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?)
}

/// One recorded element of `(kind, logical_id)` in one blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityCandidate {
    pub source_sha256: String,
    pub source_path: String,
    pub occurrence: u32,
    pub digest: String,
    /// Whether this element's digest equals the winner's first occurrence.
    /// `None` when there is no winner or the winner has no measured
    /// candidate for this id.
    pub same_as_winner: Option<bool>,
    /// Every installed package containing this candidate's blob, sorted.
    pub packages: Vec<String>,
}

/// A blob known to hold this id whose elements could not be measured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmeasuredSource {
    pub source_sha256: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdentityReport {
    pub kind: IdentityKind,
    pub logical_id: String,
    /// The blob whose element supplied the stored row (first installed).
    pub winner: Option<String>,
    /// Ordered by `(source_sha256, occurrence)`.
    pub candidates: Vec<IdentityCandidate>,
    /// Blobs that hold the winner or a recorded conflict of this id but
    /// whose scan is `unavailable` (or missing), sorted.
    pub unmeasured: Vec<UnmeasuredSource>,
}

fn checked_digest(digest: String) -> Result<String, ProductDbError> {
    if digest.len() == 64
        && digest
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
    {
        Ok(digest)
    } else {
        Err(corrupt(
            "source_identity digest is not 64 lowercase hex digits",
        ))
    }
}

fn checked_occurrence(value: i64) -> Result<u32, ProductDbError> {
    u32::try_from(value)
        .ok()
        .filter(|o| *o >= 1)
        .ok_or_else(|| corrupt("source_identity occurrence out of range"))
}

fn blob_packages(conn: &Connection, sha256: &str) -> Result<Vec<String>, ProductDbError> {
    Ok(conn
        .prepare(
            "SELECT DISTINCT package_sha256 FROM package_member WHERE source_sha256 = ?1 ORDER BY package_sha256",
        )?
        .query_map([sha256], |r| r.get(0))?
        .collect::<Result<Vec<_>, _>>()?)
}

fn winner_of(
    conn: &Connection,
    kind: IdentityKind,
    logical_id: &str,
) -> Result<Option<String>, ProductDbError> {
    Ok(conn
        .query_row(
            &format!(
                "SELECT source_sha256 FROM {} WHERE id = ?1",
                kind.as_table()
            ),
            [logical_id],
            |r| r.get(0),
        )
        .optional()?)
}

/// Names the winner of `(kind, logical_id)`, every recorded candidate, the
/// packages that carry each, and whether each equals the winner. Fails
/// closed on a malformed persisted row.
pub fn identity_candidates(
    conn: &Connection,
    kind: IdentityKind,
    logical_id: &str,
) -> Result<IdentityReport, ProductDbError> {
    let winner = winner_of(conn, kind, logical_id)?;
    let rows = conn
        .prepare(
            "SELECT i.table_name, i.source_sha256, i.occurrence, i.digest, f.source_path
             FROM source_identity AS i
             JOIN source_file AS f ON f.sha256 = i.source_sha256
             WHERE i.table_name = ?1 AND i.logical_id = ?2
             ORDER BY i.source_sha256, i.occurrence",
        )?
        .query_map(params![kind.as_table(), logical_id], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut candidates = Vec::with_capacity(rows.len());
    for (table, source_sha256, occurrence, digest, source_path) in rows {
        if IdentityKind::parse(&table) != Some(kind) {
            return Err(corrupt(format!("unknown identity table {table:?}")));
        }
        candidates.push(IdentityCandidate {
            packages: blob_packages(conn, &source_sha256)?,
            source_sha256,
            source_path,
            occurrence: checked_occurrence(occurrence)?,
            digest: checked_digest(digest)?,
            same_as_winner: None,
        });
    }
    let winner_digest = winner.as_ref().and_then(|winner| {
        candidates
            .iter()
            .find(|c| &c.source_sha256 == winner && c.occurrence == 1)
            .map(|c| c.digest.clone())
    });
    if let Some(winner_digest) = &winner_digest {
        for candidate in &mut candidates {
            candidate.same_as_winner = Some(&candidate.digest == winner_digest);
        }
    }

    // Blobs known to hold this id: the winner and both sides of every
    // recorded conflict, package-level and single-file.
    let mut holders: BTreeSet<String> = winner.iter().cloned().collect();
    let conflict_rows = conn
        .prepare(
            "SELECT kept_sha256, other_sha256 FROM package_conflict WHERE table_name = ?1 AND logical_id = ?2
             UNION
             SELECT source_sha256, sample FROM ingest_unknown
             WHERE kind = 'IdConflict' AND xpath = ?1 AND name = ?2 AND sample IS NOT NULL",
        )?
        .query_map(params![kind.as_table(), logical_id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    for (kept, other) in conflict_rows {
        holders.insert(kept);
        holders.insert(other);
    }
    let mut unmeasured = Vec::new();
    for holder in holders {
        match stored_status(conn, &holder)? {
            Some(ScanStatus::Measured) => {}
            Some(ScanStatus::Unavailable(reason)) => unmeasured.push(UnmeasuredSource {
                source_sha256: holder,
                reason,
            }),
            None => unmeasured.push(UnmeasuredSource {
                source_sha256: holder,
                reason: "no identity scan recorded".into(),
            }),
        }
    }
    Ok(IdentityReport {
        kind,
        logical_id: logical_id.to_string(),
        winner,
        candidates,
        unmeasured,
    })
}

/// An id whose recorded elements do not all have the same digest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    pub kind: IdentityKind,
    pub logical_id: String,
    /// Recorded candidate rows of this id.
    pub candidates: usize,
    pub distinct_digests: usize,
}

/// Every `(kind, id)` with more than one distinct digest, sorted by kind
/// then id. Fails closed on a malformed persisted row.
pub fn identity_divergences(conn: &Connection) -> Result<Vec<Divergence>, ProductDbError> {
    let mut groups: BTreeMap<(IdentityKind, String), (usize, BTreeSet<String>)> = BTreeMap::new();
    let mut statement =
        conn.prepare("SELECT table_name, logical_id, occurrence, digest FROM source_identity")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let table: String = row.get(0)?;
        let kind = IdentityKind::parse(&table)
            .ok_or_else(|| corrupt(format!("unknown identity table {table:?}")))?;
        checked_occurrence(row.get(2)?)?;
        let digest = checked_digest(row.get(3)?)?;
        let entry = groups.entry((kind, row.get(1)?)).or_default();
        entry.0 += 1;
        entry.1.insert(digest);
    }
    Ok(groups
        .into_iter()
        .filter(|(_, (_, digests))| digests.len() > 1)
        .map(|((kind, logical_id), (candidates, digests))| Divergence {
            kind,
            logical_id,
            candidates,
            distinct_digests: digests.len(),
        })
        .collect())
}

/// `xs:unsignedShort` / `xs:unsignedByte` lexical form: surrounding XML
/// whitespace, an optional `+`, one or more ASCII digits, at most `max`.
fn parse_unsigned(raw: &str, max: u32, type_name: &str) -> Result<u32, String> {
    let trimmed = raw.trim_matches(|c| matches!(c, ' ' | '\t' | '\n' | '\r'));
    let digits = trimmed.strip_prefix('+').unwrap_or(trimmed);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{raw:?} is not an {type_name}"));
    }
    let significant = digits.trim_start_matches('0');
    if significant.len() > 5 {
        return Err(format!("{raw:?} exceeds {type_name}"));
    }
    let value: u32 = if significant.is_empty() {
        0
    } else {
        significant
            .parse()
            .map_err(|_| format!("{raw:?} is not an {type_name}"))?
    };
    if value > max {
        return Err(format!("{raw:?} exceeds {type_name}"));
    }
    Ok(value)
}

fn parse_unsigned_byte(raw: &str) -> Result<u8, String> {
    parse_unsigned(raw, u32::from(u8::MAX), "xs:unsignedByte")
        .map(|v| u8::try_from(v).expect("bounded by u8::MAX"))
}

fn parse_unsigned_short(raw: &str) -> Result<u16, String> {
    parse_unsigned(raw, u32::from(u16::MAX), "xs:unsignedShort")
        .map(|v| u16::try_from(v).expect("bounded by u16::MAX"))
}

/// A family is keyed by `(manufacturer, ApplicationNumber)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FamilyKey {
    Number(u16),
    /// `raw` is `None` when the attribute is absent.
    Unparsed {
        raw: Option<String>,
        reason: String,
    },
}

/// `ReplacesVersions` read as an `xs:list` of `xs:unsignedByte`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReplacesVersions {
    Absent,
    /// At least one token is not an `xs:unsignedByte`; nothing is linked.
    Unparsed {
        raw: String,
        reason: String,
    },
    /// One entry per token, in source order, duplicates kept: the listed
    /// version and the installed family members with that version, sorted.
    /// No member means *not installed*, never *does not exist*.
    Parsed {
        raw: String,
        entries: Vec<(u8, Vec<String>)>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FamilyMember {
    pub program_id: String,
    pub application_version: Option<String>,
    pub parsed_version: Option<u8>,
    pub replaces: ReplacesVersions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProgramFamily {
    pub program_id: String,
    pub manufacturer_id: String,
    pub key: FamilyKey,
    /// Sorted by (parsed version, id). Only the program itself when the
    /// key did not parse.
    pub members: Vec<FamilyMember>,
}

struct ProgramVersionRow {
    id: String,
    application_number: Option<String>,
    application_version: Option<String>,
    replaces_versions: Option<String>,
}

fn family_rows(
    conn: &Connection,
    manufacturer_id: &str,
) -> Result<Vec<ProgramVersionRow>, ProductDbError> {
    Ok(conn
        .prepare(
            "SELECT id, application_number, application_version, replaces_versions
             FROM application_program WHERE manufacturer_id = ?1 AND id IS NOT NULL ORDER BY id",
        )?
        .query_map([manufacturer_id], |r| {
            Ok(ProgramVersionRow {
                id: r.get(0)?,
                application_number: r.get(1)?,
                application_version: r.get(2)?,
                replaces_versions: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?)
}

fn replaces(raw: Option<&str>, members: &[(String, Option<u8>)]) -> ReplacesVersions {
    let Some(raw) = raw else {
        return ReplacesVersions::Absent;
    };
    let mut entries = Vec::new();
    for token in raw
        .split([' ', '\t', '\n', '\r'])
        .filter(|token| !token.is_empty())
    {
        match parse_unsigned_byte(token) {
            Ok(version) => {
                let ids = members
                    .iter()
                    .filter(|(_, v)| *v == Some(version))
                    .map(|(id, _)| id.clone())
                    .collect();
                entries.push((version, ids));
            }
            Err(reason) => {
                return ReplacesVersions::Unparsed {
                    raw: raw.to_string(),
                    reason,
                }
            }
        }
    }
    ReplacesVersions::Parsed {
        raw: raw.to_string(),
        entries,
    }
}

/// The winning programs sharing `program_id`'s manufacturer and parsed
/// `ApplicationNumber`, with `ReplacesVersions` resolved against them.
/// Derived at query time from winning rows; `None` for an unknown id.
pub fn program_family(
    conn: &Connection,
    program_id: &str,
) -> Result<Option<ProgramFamily>, ProductDbError> {
    let manufacturer: Option<String> = conn
        .query_row(
            "SELECT manufacturer_id FROM application_program WHERE id = ?1",
            [program_id],
            |r| r.get(0),
        )
        .optional()?;
    let Some(manufacturer_id) = manufacturer else {
        return Ok(None);
    };
    let rows = family_rows(conn, &manufacturer_id)?;
    let this = rows
        .iter()
        .find(|row| row.id == program_id)
        .ok_or_else(|| corrupt("program row vanished during the family query"))?;
    let key = match this.application_number.as_deref() {
        None => FamilyKey::Unparsed {
            raw: None,
            reason: "ApplicationNumber is absent".into(),
        },
        Some(raw) => match parse_unsigned_short(raw) {
            Ok(number) => FamilyKey::Number(number),
            Err(reason) => FamilyKey::Unparsed {
                raw: Some(raw.to_string()),
                reason,
            },
        },
    };
    let in_family: Vec<&ProgramVersionRow> = match key {
        FamilyKey::Number(number) => rows
            .iter()
            .filter(|row| {
                row.application_number
                    .as_deref()
                    .and_then(|raw| parse_unsigned_short(raw).ok())
                    == Some(number)
            })
            .collect(),
        FamilyKey::Unparsed { .. } => vec![this],
    };
    let versions: Vec<(String, Option<u8>)> = in_family
        .iter()
        .map(|row| {
            (
                row.id.clone(),
                row.application_version
                    .as_deref()
                    .and_then(|raw| parse_unsigned_byte(raw).ok()),
            )
        })
        .collect();
    let mut members: Vec<FamilyMember> = in_family
        .iter()
        .zip(&versions)
        .map(|(row, (_, parsed_version))| FamilyMember {
            program_id: row.id.clone(),
            application_version: row.application_version.clone(),
            parsed_version: *parsed_version,
            replaces: replaces(row.replaces_versions.as_deref(), &versions),
        })
        .collect();
    members
        .sort_by(|a, b| (a.parsed_version, &a.program_id).cmp(&(b.parsed_version, &b.program_id)));
    Ok(Some(ProgramFamily {
        program_id: program_id.to_string(),
        manufacturer_id,
        key,
        members,
    }))
}

/// One winning product carrying a looked-up order number.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrderNumberProduct {
    pub product_id: String,
    pub text: Option<String>,
    pub hardware_id: String,
    /// `Hardware2Program` program references of the product's hardware,
    /// sorted and distinct.
    pub programs: Vec<String>,
    pub source_sha256: String,
    /// Distinct schemes of the installed packages containing the source blob.
    pub schemes: Vec<u32>,
}

/// Exact-string lookup of a manufacturer's order number over winning
/// product rows, sorted by product id. An order number is never an identity.
pub fn products_by_order_number(
    conn: &Connection,
    manufacturer_id: &str,
    order_number: &str,
) -> Result<Vec<OrderNumberProduct>, ProductDbError> {
    let rows = conn
        .prepare(
            "SELECT id, text, hardware_id, source_sha256 FROM product
             WHERE manufacturer_id = ?1 AND order_number = ?2 AND id IS NOT NULL ORDER BY id",
        )?
        .query_map(params![manufacturer_id, order_number], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, Option<String>>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut out = Vec::with_capacity(rows.len());
    for (product_id, text, hardware_id, source_sha256) in rows {
        let programs = conn
            .prepare(
                "SELECT DISTINCT application_program_ref FROM hardware2program
                 WHERE hardware_id = ?1 AND application_program_ref IS NOT NULL
                 ORDER BY application_program_ref",
            )?
            .query_map([&hardware_id], |r| r.get(0))?
            .collect::<Result<Vec<String>, _>>()?;
        let schemes = conn
            .prepare(
                "SELECT DISTINCT p.scheme FROM package AS p
                 JOIN package_member AS m ON m.package_sha256 = p.sha256
                 WHERE m.source_sha256 = ?1 ORDER BY p.scheme",
            )?
            .query_map([&source_sha256], |r| r.get::<_, i64>(0))?
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|scheme| {
                u32::try_from(scheme).map_err(|_| corrupt("invalid stored package scheme"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        out.push(OrderNumberProduct {
            product_id,
            text,
            hardware_id,
            programs,
            source_sha256,
            schemes,
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hardware_file(inner: &str) -> String {
        format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware>{inner}</Hardware></Manufacturer></ManufacturerData></KNX>"#
        )
    }

    fn catalog_file(inner: &str) -> String {
        format!(
            r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Catalog>{inner}</Catalog></Manufacturer></ManufacturerData></KNX>"#
        )
    }

    fn program_file(inner: &str) -> String {
        format!(
            r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms>{inner}</ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
        )
    }

    fn scan(xml: &str) -> Vec<Candidate> {
        scan_identities("M-0001/T.xml", xml.as_bytes()).unwrap()
    }

    fn digest(xml: &str, kind: IdentityKind, id: &str) -> String {
        scan(xml)
            .into_iter()
            .find(|c| c.table == kind && c.logical_id == id && c.occurrence == 1)
            .unwrap_or_else(|| panic!("no candidate {kind:?} {id} in {xml}"))
            .digest
    }

    fn hw(inner_h1: &str) -> String {
        digest(&hardware_file(inner_h1), IdentityKind::Hardware, "H-1")
    }

    #[test]
    fn a_digest_is_sixty_four_lowercase_hex_digits() {
        let d = hw(r#"<Hardware Id="H-1"/>"#);
        assert_eq!(d.len(), 64);
        assert!(
            d.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')),
            "{d}"
        );
    }

    #[test]
    fn syntax_without_meaning_does_not_change_the_digest() {
        let base = hw(r#"<Hardware Id="H-1" A="1" B="2"><Note>a&amp;b</Note><Empty/></Hardware>"#);
        for variant in [
            // attribute order
            r#"<Hardware B="2" A="1" Id="H-1"><Note>a&amp;b</Note><Empty/></Hardware>"#,
            // <X/> vs <X></X>
            r#"<Hardware Id="H-1" A="1" B="2"><Note>a&amp;b</Note><Empty></Empty></Hardware>"#,
            // whitespace-only text between elements, with CRLF
            "<Hardware Id=\"H-1\" A=\"1\" B=\"2\">\r\n  <Note>a&amp;b</Note>\r\n\t<Empty/>\n</Hardware>",
            // comments and processing instructions
            r#"<Hardware Id="H-1" A="1" B="2"><!-- c --><Note>a<!--x-->&amp;b</Note><?pi x?><Empty/></Hardware>"#,
            // decimal and hex character references for &
            r#"<Hardware Id="H-1" A="1" B="2"><Note>a&#38;b</Note><Empty/></Hardware>"#,
            r#"<Hardware Id="H-1" A="1" B="2"><Note>a&#x26;b</Note><Empty/></Hardware>"#,
            // CDATA taken literally
            r#"<Hardware Id="H-1" A="1" B="2"><Note><![CDATA[a&b]]></Note><Empty/></Hardware>"#,
            r#"<Hardware Id="H-1" A="1" B="2"><Note>a<![CDATA[&]]>b</Note><Empty/></Hardware>"#,
            // attribute quoting
            r#"<Hardware Id='H-1' A='1' B='2'><Note>a&amp;b</Note><Empty/></Hardware>"#,
        ] {
            assert_eq!(hw(variant), base, "{variant}");
        }
    }

    #[test]
    fn line_endings_inside_text_are_normalized() {
        assert_eq!(
            hw("<Hardware Id=\"H-1\"><Note>a\r\nb\rc</Note></Hardware>"),
            hw("<Hardware Id=\"H-1\"><Note>a\nb\nc</Note></Hardware>")
        );
    }

    #[test]
    fn a_character_reference_to_carriage_return_is_not_a_line_ending() {
        assert_ne!(
            hw("<Hardware Id=\"H-1\"><Note>a&#13;b</Note></Hardware>"),
            hw("<Hardware Id=\"H-1\"><Note>a\nb</Note></Hardware>")
        );
    }

    #[test]
    fn meaningful_and_syntactic_changes_change_the_digest() {
        let base = hw(r#"<Hardware Id="H-1" A="1"><Note>text</Note><Other/></Hardware>"#);
        for variant in [
            // changed attribute value
            r#"<Hardware Id="H-1" A="2"><Note>text</Note><Other/></Hardware>"#,
            // added attribute
            r#"<Hardware Id="H-1" A="1" C=""><Note>text</Note><Other/></Hardware>"#,
            // changed non-whitespace text
            r#"<Hardware Id="H-1" A="1"><Note>text!</Note><Other/></Hardware>"#,
            // surrounding whitespace of real text is kept
            r#"<Hardware Id="H-1" A="1"><Note> text</Note><Other/></Hardware>"#,
            // added child
            r#"<Hardware Id="H-1" A="1"><Note>text</Note><Other/><Other/></Hardware>"#,
            // reordered children
            r#"<Hardware Id="H-1" A="1"><Other/><Note>text</Note></Hardware>"#,
            // different nesting of the same tokens
            r#"<Hardware Id="H-1" A="1"><Note>text<Other/></Note></Hardware>"#,
            // text moved to another element
            r#"<Hardware Id="H-1" A="1"><Note/><Other>text</Other></Hardware>"#,
            // renamed element
            r#"<Hardware Id="H-1" A="1"><Note2>text</Note2><Other/></Hardware>"#,
        ] {
            assert_ne!(hw(variant), base, "{variant}");
        }
    }

    #[test]
    fn a_different_namespace_prefix_is_reported_as_different() {
        assert_ne!(
            hw(r#"<Hardware Id="H-1"><a:Note xmlns:a="urn:x"/></Hardware>"#),
            hw(r#"<Hardware Id="H-1"><b:Note xmlns:b="urn:x"/></Hardware>"#)
        );
    }

    #[test]
    fn field_boundaries_cannot_be_shifted_between_names_and_values() {
        assert_ne!(
            hw(r#"<Hardware Id="H-1"><N a="bc"/></Hardware>"#),
            hw(r#"<Hardware Id="H-1"><N ab="c"/></Hardware>"#)
        );
        assert_ne!(
            hw(r#"<Hardware Id="H-1"><N>ab</N></Hardware>"#),
            hw(r#"<Hardware Id="H-1"><Na>b</Na></Hardware>"#)
        );
    }

    #[test]
    fn a_nested_product_is_a_marker_in_its_hardware_not_its_content() {
        let product = |text: &str| {
            format!(
                r#"<Hardware Id="H-1"><Products><Product Id="P-1" Text="{text}"/></Products></Hardware>"#
            )
        };
        let a = hardware_file(&product("one"));
        let b = hardware_file(&product("two"));
        assert_eq!(
            digest(&a, IdentityKind::Hardware, "H-1"),
            digest(&b, IdentityKind::Hardware, "H-1"),
            "a Product's own change does not reach its Hardware"
        );
        assert_ne!(
            digest(&a, IdentityKind::Product, "P-1"),
            digest(&b, IdentityKind::Product, "P-1")
        );

        let base = hw(
            r#"<Hardware Id="H-1"><Products><Product Id="P-1"/><Product Id="P-2"/></Products></Hardware>"#,
        );
        for variant in [
            r#"<Hardware Id="H-1"><Products><Product Id="P-1"/></Products></Hardware>"#,
            r#"<Hardware Id="H-1"><Products><Product Id="P-2"/><Product Id="P-1"/></Products></Hardware>"#,
            r#"<Hardware Id="H-1"><Products><Product Id="P-1"/><Product Id="P-2"/><Product Id="P-3"/></Products></Hardware>"#,
        ] {
            assert_ne!(hw(variant), base, "{variant}");
        }
    }

    #[test]
    fn a_nested_catalog_item_or_section_is_a_marker_in_its_section() {
        let file = |item_name: &str, order: bool| {
            let items = if order {
                format!(r#"<CatalogItem Id="CI-1" Name="{item_name}"/><CatalogItem Id="CI-2"/>"#)
            } else {
                format!(r#"<CatalogItem Id="CI-2"/><CatalogItem Id="CI-1" Name="{item_name}"/>"#)
            };
            catalog_file(&format!(
                r#"<CatalogSection Id="CS-1"><CatalogSection Id="CS-2">{items}</CatalogSection></CatalogSection>"#
            ))
        };
        let sec = |xml: &str, id: &str| digest(xml, IdentityKind::CatalogSection, id);
        assert_eq!(sec(&file("a", true), "CS-2"), sec(&file("b", true), "CS-2"));
        assert_ne!(
            digest(&file("a", true), IdentityKind::CatalogItem, "CI-1"),
            digest(&file("b", true), IdentityKind::CatalogItem, "CI-1")
        );
        assert_ne!(
            sec(&file("a", true), "CS-2"),
            sec(&file("a", false), "CS-2")
        );
        // The outer section sees only a marker for the inner one.
        assert_eq!(
            sec(&file("a", true), "CS-1"),
            sec(&file("a", false), "CS-1")
        );
    }

    #[test]
    fn candidates_are_in_document_order_with_per_id_occurrences() {
        let xml = hardware_file(
            r#"<Hardware Id="H-1"><Products><Product Id="P-1"/></Products><Hardware2Programs><Hardware2Program Id="HP-1"/></Hardware2Programs></Hardware><Hardware Id="H-1"><Products><Product/></Products></Hardware>"#,
        );
        let got: Vec<_> = scan(&xml)
            .into_iter()
            .map(|c| (c.table, c.logical_id, c.occurrence))
            .collect();
        assert_eq!(
            got,
            [
                (IdentityKind::Hardware, "H-1".to_string(), 1),
                (IdentityKind::Product, "P-1".to_string(), 1),
                (IdentityKind::Hardware2Program, "HP-1".to_string(), 1),
                (IdentityKind::Hardware, "H-1".to_string(), 2),
                (IdentityKind::Product, String::new(), 1),
            ]
        );
    }

    #[test]
    fn the_outer_hardware_collection_without_an_id_is_not_a_candidate() {
        let kinds: Vec<_> = scan(&hardware_file(r#"<Hardware Id="H-1"/>"#))
            .into_iter()
            .map(|c| c.table)
            .collect();
        assert_eq!(kinds, [IdentityKind::Hardware]);
    }

    #[test]
    fn a_prefixed_id_is_read_through_the_parsers_attribute_helper() {
        let xml = hardware_file(r#"<Hardware xmlns:x="urn:x" x:Id="H-9"/>"#);
        let ids: Vec<_> = scan(&xml).into_iter().map(|c| c.logical_id).collect();
        assert_eq!(ids, ["H-9"]);
    }

    #[test]
    fn a_program_inside_dynamic_is_part_of_its_parent_not_a_candidate() {
        let file = |dynamic: &str| {
            program_file(&format!(
                r#"<ApplicationProgram Id="A-1"><Static/><Dynamic><ApplicationProgram Id="A-X"/>{dynamic}</Dynamic></ApplicationProgram>"#
            ))
        };
        let ids: Vec<_> = scan(&file("")).into_iter().map(|c| c.logical_id).collect();
        assert_eq!(ids, ["A-1"]);
        assert_ne!(
            digest(&file(""), IdentityKind::ApplicationProgram, "A-1"),
            digest(&file("<Channel/>"), IdentityKind::ApplicationProgram, "A-1"),
            "the skipped Dynamic subtree is still hashed into its program"
        );
    }

    #[test]
    fn the_element_after_a_parameter_type_is_never_a_candidate() {
        let cases = [
            // the child of an open ParameterType
            (
                r#"<ApplicationProgram Id="A-1"><ParameterType Id="PT"><ApplicationProgram Id="A-2"/></ParameterType></ApplicationProgram>"#,
                vec!["A-1"],
            ),
            // an empty ParameterType consumes its next sibling
            (
                r#"<ApplicationProgram Id="A-1"><ParameterType Id="PT"/></ApplicationProgram><ApplicationProgram Id="A-2"/><ApplicationProgram Id="A-3"/>"#,
                vec!["A-1", "A-3"],
            ),
            // a skipped Dynamic does not consume it; the element after does
            (
                r#"<ApplicationProgram Id="A-1"><ParameterType Id="PT"/><Dynamic><x/></Dynamic></ApplicationProgram><ApplicationProgram Id="A-2"/><ApplicationProgram Id="A-3"/>"#,
                vec!["A-1", "A-3"],
            ),
            // an empty Dynamic is not skipped, so it is the consumed child
            (
                r#"<ApplicationProgram Id="A-1"><ParameterType Id="PT"/><Dynamic/></ApplicationProgram><ApplicationProgram Id="A-2"/>"#,
                vec!["A-1", "A-2"],
            ),
        ];
        for (inner, expected) in cases {
            let ids: Vec<_> = scan(&program_file(inner))
                .into_iter()
                .map(|c| c.logical_id)
                .collect();
            assert_eq!(ids, expected, "{inner}");
        }
    }

    #[test]
    fn a_catalog_or_other_file_kind_is_dispatched_by_content() {
        let catalog =
            catalog_file(r#"<CatalogSection Id="CS-1"><Hardware Id="H-1"/></CatalogSection>"#);
        let kinds: Vec<_> = scan(&catalog).into_iter().map(|c| c.table).collect();
        assert_eq!(kinds, [IdentityKind::CatalogSection]);
        let master = r#"<KNX><MasterData><Hardware Id="H-1"/></MasterData></KNX>"#;
        assert!(scan(master).is_empty());
        assert!(scan_identities("x.bin", b"MZ\x90\x00").unwrap().is_empty());
    }

    #[test]
    fn an_undeclared_entity_or_unclosed_element_is_a_scan_error() {
        for bad in [
            hardware_file(r#"<Hardware Id="H-1"><Note>&nbsp;</Note></Hardware>"#),
            hardware_file(r#"<Hardware Id="H-1">"#).replace("</KNX>", ""),
        ] {
            assert!(
                scan_identities("M-0001/Hardware.xml", bad.as_bytes()).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn identity_kinds_round_trip_through_their_table_names() {
        for kind in IdentityKind::ALL {
            assert_eq!(IdentityKind::parse(kind.as_table()), Some(kind));
        }
        assert_eq!(IdentityKind::parse("Hardware"), None);
        assert_eq!(IdentityKind::parse(""), None);
    }

    mod agreement {
        use super::*;
        use crate::{ingest_file, open_and_migrate, sha256_hex};

        const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/11"><ManufacturerData><Manufacturer RefId="M-0001"><Hardware><Hardware Id="H-1"><Products><Product Id="P-1"/><Product Id="P-1"/></Products></Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;

        fn ingested() -> (tempfile::TempDir, Connection, String) {
            let dir = tempfile::tempdir().unwrap();
            let conn = open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
            ingest_file(&conn, "M-0001/Hardware.xml", HARDWARE.as_bytes()).unwrap();
            (dir, conn, sha256_hex(HARDWARE.as_bytes()))
        }

        fn keys(conn: &Connection, sha: &str) -> HashSet<CandidateKey> {
            stored_keys(conn, sha).unwrap()
        }

        fn conflict(sha: &str) -> IdConflict {
            IdConflict {
                table: "product".into(),
                id: "P-1".into(),
                kept_sha256: sha.into(),
                other_sha256: sha.into(),
                occurrence: 2,
            }
        }

        fn check(conn: &Connection, sha: &str, candidates: &HashSet<CandidateKey>) -> String {
            match check_agreement(
                conn,
                sha,
                "M-0001/Hardware.xml",
                candidates,
                &[conflict(sha)],
            ) {
                Ok(()) => String::new(),
                Err(error) => error.to_string(),
            }
        }

        #[test]
        fn the_recorded_candidates_agree_with_the_parser() {
            let (_dir, conn, sha) = ingested();
            assert_eq!(
                keys(&conn, &sha),
                HashSet::from([
                    (IdentityKind::Hardware, "H-1".to_string(), 1),
                    (IdentityKind::Product, "P-1".to_string(), 1),
                    (IdentityKind::Product, "P-1".to_string(), 2),
                ])
            );
            assert_eq!(check(&conn, &sha, &keys(&conn, &sha)), "");
        }

        #[test]
        fn a_won_row_without_its_first_candidate_is_a_disagreement() {
            let (_dir, conn, sha) = ingested();
            let mut candidates = keys(&conn, &sha);
            candidates.remove(&(IdentityKind::Hardware, "H-1".to_string(), 1));
            // Rule c. cannot see this: only a. pairs rows with candidates.
            let error = check(&conn, &sha, &candidates);
            assert!(
                error.contains("identity scan disagrees with parser: hardware row \"H-1\" won by this blob has no candidate"),
                "{error}"
            );
        }

        #[test]
        fn a_conflict_without_its_occurrence_is_a_disagreement() {
            let (_dir, conn, sha) = ingested();
            let mut candidates = keys(&conn, &sha);
            candidates.remove(&(IdentityKind::Product, "P-1".to_string(), 2));
            let error = check(&conn, &sha, &candidates);
            assert!(
                error.contains("product conflict \"P-1\" occurrence 2 has no candidate"),
                "{error}"
            );
            // Another blob's conflict is not this blob's business.
            let mut foreign = conflict(&sha);
            foreign.other_sha256 = "f".repeat(64);
            check_agreement(&conn, &sha, "x", &candidates, &[foreign]).unwrap();
        }

        #[test]
        fn a_candidate_without_a_typed_row_is_a_disagreement() {
            let (_dir, conn, sha) = ingested();
            let mut candidates = keys(&conn, &sha);
            candidates.insert((IdentityKind::CatalogItem, "CI-9".to_string(), 1));
            let error = check(&conn, &sha, &candidates);
            assert!(
                error.contains("candidate catalog_item \"CI-9\" has no typed row"),
                "{error}"
            );
            // An empty id is recorded but cannot be paired with a row.
            let mut candidates = keys(&conn, &sha);
            candidates.insert((IdentityKind::CatalogItem, String::new(), 1));
            assert_eq!(check(&conn, &sha, &candidates), "");
        }
    }
}
