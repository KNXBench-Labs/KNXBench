//! Stage 3: the tolerant streaming parser.
//!
//! Reads a project part's XML documents into the [`crate::source`] shape.
//! Nothing here fails the whole parse because of one unrecognized element or
//! attribute: unknown constructs are retained and reported, never silently
//! dropped and never fatal on their own (CLAUDE.md's data-integrity rule).

mod installation;
mod project_info;

pub use installation::parse_installation;
pub use project_info::parse_project_info;

use crate::source::{RetainedElement, SourceDocument};
use quick_xml::events::BytesStart;
use quick_xml::{Reader, XmlVersion};

/// The result of parsing one XML document: the document shape, plus
/// everything the known-element table for this schema version did not
/// recognize.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOutput {
    pub document: SourceDocument,
    pub unknown: Vec<UnknownConstruct>,
    pub retained_elements: Vec<RetainedElement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnknownKind {
    Element,
    Attribute,
}

/// One element or attribute the known-element table did not list, with an
/// occurrence count so a project that repeats the same unknown attribute
/// hundreds of times produces one report line, not hundreds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownConstruct {
    /// Container entry this was read from, e.g. `"P-0512/0.xml"`.
    pub source_path: String,
    /// Absolute element path, e.g. `"/KNX/Project/.../DeviceInstance"`.
    pub xpath: String,
    pub kind: UnknownKind,
    pub name: String,
    pub occurrences: u32,
    /// One example value, for an attribute.
    pub sample: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    Xml {
        source_path: String,
        position: u64,
        cause: String,
    },
    /// Not raised by `parse_installation` itself, which always receives an
    /// already-resolved `KnownSchema`. Shared vocabulary for callers that
    /// resolve `known_schema(version)` before invoking it and need a typed
    /// error for the `None` case.
    UnsupportedSchemaVersion(u32),
    MissingRequiredAttribute {
        xpath: String,
        name: String,
    },
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Xml {
                source_path,
                position,
                cause,
            } => write!(f, "{source_path}:{position}: {cause}"),
            ParseError::UnsupportedSchemaVersion(v) => {
                write!(f, "no known-element table for schema version {v}")
            }
            ParseError::MissingRequiredAttribute { xpath, name } => {
                write!(f, "{xpath}: missing required attribute {name}")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// Aggregates unknown constructs by `(xpath, kind, name)`, so repeats
/// collapse into one entry with a growing occurrence count instead of one
/// entry per occurrence.
#[derive(Default)]
pub(crate) struct UnknownAggregator {
    entries: std::collections::HashMap<(String, UnknownKind, String), (u32, Option<String>)>,
}

impl UnknownAggregator {
    pub(crate) fn record(
        &mut self,
        xpath: &str,
        kind: UnknownKind,
        name: &str,
        sample: Option<String>,
    ) {
        let key = (xpath.to_string(), kind, name.to_string());
        let entry = self.entries.entry(key).or_insert((0, None));
        entry.0 += 1;
        if entry.1.is_none() {
            entry.1 = sample;
        }
    }

    pub(crate) fn into_sorted_vec(self, source_path: &str) -> Vec<UnknownConstruct> {
        let mut out: Vec<UnknownConstruct> = self
            .entries
            .into_iter()
            .map(
                |((xpath, kind, name), (occurrences, sample))| UnknownConstruct {
                    source_path: source_path.to_string(),
                    xpath,
                    kind,
                    name,
                    occurrences,
                    sample,
                },
            )
            .collect();
        out.sort_by(|a, b| {
            (a.xpath.as_str(), a.kind, a.name.as_str()).cmp(&(
                b.xpath.as_str(),
                b.kind,
                b.name.as_str(),
            ))
        });
        out
    }
}

/// Decodes a start tag's attributes into owned `(name, value)` pairs, with
/// entity references expanded and namespace declarations (`xmlns`,
/// `xmlns:*`) excluded — the known-element table never lists those, and
/// every real project file carries them, so counting them as unknown would
/// make every import report noisy for no reason.
pub(crate) fn attr_map(
    start: &BytesStart,
    source_path: &str,
    position: u64,
) -> Result<Vec<(String, String)>, ParseError> {
    let mut out = Vec::new();
    for attr in start.attributes() {
        let attr = attr.map_err(|e| ParseError::Xml {
            source_path: source_path.to_string(),
            position,
            cause: e.to_string(),
        })?;
        let key = attr.key.into_inner();
        if key == "xmlns" || key.starts_with("xmlns:") {
            continue;
        }
        let value = attr
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|e| ParseError::Xml {
                source_path: source_path.to_string(),
                position,
                cause: e.to_string(),
            })?;
        out.push((key.to_string(), value.into_owned()));
    }
    Ok(out)
}

/// Skips a `Start` element's subtree via `read_to_end` and returns its raw
/// bytes, tags included, exactly as they appeared in the source — used to
/// retain an element verbatim without walking or understanding its content.
pub(crate) fn skip_and_capture<'a>(
    reader: &mut Reader<&'a [u8]>,
    bytes: &'a [u8],
    start_tag: &BytesStart<'a>,
    pos_before: u64,
    source_path: &str,
) -> Result<Vec<u8>, ParseError> {
    reader
        .read_to_end(start_tag.to_end().name())
        .map_err(|e| ParseError::Xml {
            source_path: source_path.to_string(),
            position: reader.buffer_position(),
            cause: e.to_string(),
        })?;
    let end = reader.buffer_position();
    Ok(bytes[pos_before as usize..end as usize].to_vec())
}

/// Orders `UnknownKind` for deterministic sorting; the exact order has no
/// meaning beyond stability.
impl PartialOrd for UnknownKind {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for UnknownKind {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        let rank = |k: &UnknownKind| match k {
            UnknownKind::Element => 0,
            UnknownKind::Attribute => 1,
        };
        rank(self).cmp(&rank(other))
    }
}
