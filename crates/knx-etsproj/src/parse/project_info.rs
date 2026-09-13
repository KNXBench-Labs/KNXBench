//! The `Project.xml` / `project.xml` parser: three elements deep (`KNX` >
//! `Project` > `ProjectInformation`), no lists, no frame stack needed. Unlike
//! `parse_installation`, this narrower interface has no `retained_elements`
//! output — `Project.xml` carries only project metadata, so an unknown
//! element's subtree is skipped rather than kept verbatim; it is still
//! reported, never silently dropped. Unknown attributes and known-but-
//! unmodelled attributes are handled exactly as in the installation parser:
//! the former reported, the latter kept in `other` for export.

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

use crate::known::KnownSchema;
use crate::source::{RetainedAttribute, RetainedElement, SourceProjectInfo};

use super::{
    attr_map, skip_and_capture, ParseError, UnknownAggregator, UnknownConstruct, UnknownKind,
};

/// Parses one `Project.xml` document into a [`SourceProjectInfo`],
/// tolerantly: an unknown element or attribute is reported, not fatal.
/// Only malformed XML fails the whole parse — `ProjectInformation` has no
/// required attribute of its own.
pub fn parse_project_info(
    bytes: &[u8],
    source_path: &str,
    schema: &KnownSchema,
) -> Result<(SourceProjectInfo, Vec<UnknownConstruct>), ParseError> {
    // See `installation.rs`'s identical stripping for why: `Reader` silently
    // absorbs a leading UTF-8 BOM into its own position bookkeeping, so
    // `ProjectTraces`'s raw-capture slice must be indexed against a `bytes`
    // view with the same three bytes already removed.
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    let mut reader = Reader::from_reader(bytes);
    let mut path_stack: Vec<String> = Vec::new();
    let mut info = SourceProjectInfo::default();
    let mut aggregator = UnknownAggregator::default();

    loop {
        let pos_before = reader.buffer_position();
        let event = reader.read_event().map_err(|e| ParseError::Xml {
            source_path: source_path.to_string(),
            position: reader.buffer_position(),
            cause: e.to_string(),
        })?;

        match event {
            Event::Eof => break,

            Event::Start(start) => {
                open_element(
                    &start,
                    false,
                    &mut reader,
                    bytes,
                    pos_before,
                    source_path,
                    schema,
                    &mut path_stack,
                    &mut info,
                    &mut aggregator,
                )?;
            }

            Event::Empty(start) => {
                open_element(
                    &start,
                    true,
                    &mut reader,
                    bytes,
                    pos_before,
                    source_path,
                    schema,
                    &mut path_stack,
                    &mut info,
                    &mut aggregator,
                )?;
            }

            Event::End(_) => {
                path_stack.pop().expect("End without matching Start");
            }

            _ => {} // Text, Comment, PI, Decl, CData, DocType: not structural, ignored.
        }
    }

    if !path_stack.is_empty() {
        return Err(ParseError::Xml {
            source_path: source_path.to_string(),
            position: reader.buffer_position(),
            cause: format!(
                "unexpected end of document inside <{}>",
                path_stack.join("/")
            ),
        });
    }

    Ok((info, aggregator.into_sorted_vec(source_path)))
}

/// Handles one `Start` or `Empty` event. An element the known-element table
/// does not list at all is reported and its subtree skipped, without being
/// pushed onto `path_stack` (there is nothing to attach a child to). A known
/// element pushes `path_stack` (unless `is_empty`, which has no matching
/// `End` to pop it) and, for `ProjectInformation`, fills `info` from its
/// modelled attributes.
#[allow(clippy::too_many_arguments)]
fn open_element<'a>(
    start: &BytesStart<'a>,
    is_empty: bool,
    reader: &mut Reader<&'a [u8]>,
    bytes: &'a [u8],
    pos_before: u64,
    source_path: &str,
    schema: &KnownSchema,
    path_stack: &mut Vec<String>,
    info: &mut SourceProjectInfo,
    aggregator: &mut UnknownAggregator,
) -> Result<(), ParseError> {
    let local = start.name().local_name().as_ref().to_string();
    path_stack.push(local.clone());
    let xpath = format!("/{}", path_stack.join("/"));

    // `ProjectTraces` (schema ≥21, an audit log — RESEARCH §3.4: "purpose
    // not investigated") is known but deliberately not modeled beyond
    // verbatim retention, mirroring `installation.rs`'s `BusAccess` special
    // case exactly: whole subtree captured raw, not walked, never reported
    // as unknown.
    if local == "ProjectTraces" {
        let raw = if is_empty {
            bytes[pos_before as usize..reader.buffer_position() as usize].to_vec()
        } else {
            skip_and_capture(reader, bytes, start, pos_before, source_path)?
        };
        info.project_traces_raw = Some(RetainedElement {
            xpath,
            name: local,
            raw,
        });
        path_stack.pop();
        return Ok(());
    }

    let known_names = schema
        .elements
        .iter()
        .find(|e| e.path == xpath)
        .map(|e| e.attributes);

    let Some(known_names) = known_names else {
        aggregator.record(&xpath, UnknownKind::Element, &local, None);
        if !is_empty {
            reader
                .read_to_end(start.to_end().name())
                .map_err(|e| ParseError::Xml {
                    source_path: source_path.to_string(),
                    position: reader.buffer_position(),
                    cause: e.to_string(),
                })?;
        }
        path_stack.pop();
        return Ok(());
    };

    let attrs = attr_map(start, source_path, pos_before)?;
    let mut other = Vec::new();
    for (name, value) in attrs {
        if !known_names.contains(&name.as_str()) {
            aggregator.record(&xpath, UnknownKind::Attribute, &name, Some(value.clone()));
            other.push(RetainedAttribute {
                xpath: xpath.clone(),
                name,
                value,
            });
            continue;
        }
        if local != "ProjectInformation" {
            // `KNX`'s `CreatedBy`/`ToolVersion` and `Project`'s `Id` carry no
            // information this narrower parser returns; being known (not
            // unknown) is all that matters for them.
            continue;
        }
        match name.as_str() {
            "Name" => info.name = Some(value),
            "LastModified" => info.last_modified = Some(value),
            "ProjectStart" => info.project_start = Some(value),
            "ProjectId" => info.project_number = Some(value),
            "GroupAddressStyle" => info.group_address_style = Some(value),
            "CompletionStatus" => info.completion_status = Some(value),
            // Known to the table, but `SourceProjectInfo` has no dedicated
            // field for it (e.g. `ProjectTracingLevel`,
            // `Hide16BitGroupsFromLegacyPlugins`): retained, not reported.
            _ => other.push(RetainedAttribute {
                xpath: xpath.clone(),
                name,
                value,
            }),
        }
    }
    if local == "ProjectInformation" {
        info.other = other;
    }

    if is_empty {
        path_stack.pop();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::known::known_schema;
    use crate::Container;

    fn reference_ets4_bytes() -> Vec<u8> {
        std::fs::read(knx_testsupport::reference_ets4_path())
            .expect("reference ETS4 project lives under OriginalData/ (gitignored); see knx_testsupport::corpus_available")
    }

    #[test]
    fn the_reference_project_information_is_read_verbatim() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let bytes = c.read("P-0512/Project.xml").unwrap();
        let (info, unknown) =
            parse_project_info(&bytes, "P-0512/Project.xml", known_schema(11).unwrap()).unwrap();
        assert_eq!(info.name.as_deref(), Some("Unser Zuhause"));
        assert_eq!(info.group_address_style.as_deref(), Some("ThreeLevel"));
        assert_eq!(info.completion_status.as_deref(), Some("Editing"));
        assert_eq!(info.last_modified.as_deref(), Some("2025-12-15T07:07:12"));
        assert_eq!(unknown, vec![]);
    }

    #[test]
    fn tool_state_attributes_are_retained_rather_than_modelled() {
        if !crate::testutil::corpus_available() {
            eprintln!("skip: OriginalData/ corpus not present (gitignored, local-only)");
            return;
        }
        let mut c = Container::open(reference_ets4_bytes()).unwrap();
        let bytes = c.read("P-0512/Project.xml").unwrap();
        let (info, _) =
            parse_project_info(&bytes, "P-0512/Project.xml", known_schema(11).unwrap()).unwrap();
        assert!(info.other.iter().any(|a| a.name == "ProjectTracingLevel"));
        assert!(info
            .other
            .iter()
            .any(|a| a.name == "Hide16BitGroupsFromLegacyPlugins" && a.value == "1"));
    }
}
