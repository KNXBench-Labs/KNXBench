//! Parses one captured schema-23 `Function` subtree into source records.
//!
//! Parses one captured schema-23 `Function` subtree (*Project Schema23*
//! §1.2.6.9/§1.2.6.10; ADR-0124).
//!
//! The schema-≥20 walker captures the whole element verbatim (that raw copy
//! is retained in the opaque store) and hands it here, so the main state
//! machine needs no Function frames. Attributes the known-element table
//! does not list, and any child other than `GroupAddressRef`, are reported
//! as unknown exactly as the walker would report them; their bytes survive
//! in the raw copy.

use quick_xml::events::Event;
use quick_xml::Reader;

use super::{attr_map, ParseError, UnknownAggregator, UnknownKind};
use crate::known::KnownSchema;
use crate::source::{RetainedElement, SourceFunction, SourceFunctionMember};

fn known<'s>(schema: &'s KnownSchema, path: &str) -> &'s [&'s str] {
    schema
        .elements
        .iter()
        .find(|e| e.path == path)
        .map(|e| e.attributes)
        .unwrap_or(&[])
}

/// Takes the modeled attributes out of `attrs`, reporting every attribute
/// the table does not list. Known but unmodeled ones (`Puid`,
/// `Implements`, `DefaulGroupRange`) stay in the raw copy only.
fn classify(
    attrs: Vec<(String, String)>,
    known_names: &[&str],
    xpath: &str,
    aggregator: &mut UnknownAggregator,
) -> Vec<(String, String)> {
    let mut kept = Vec::new();
    for (name, value) in attrs {
        if known_names.contains(&name.as_str()) {
            kept.push((name, value));
        } else {
            aggregator.record(xpath, UnknownKind::Attribute, &name, Some(value));
        }
    }
    kept
}

fn take(attrs: &mut Vec<(String, String)>, name: &str) -> Option<String> {
    let index = attrs.iter().position(|(k, _)| k == name)?;
    Some(attrs.remove(index).1)
}

fn require(
    attrs: &mut Vec<(String, String)>,
    name: &str,
    xpath: &str,
) -> Result<String, ParseError> {
    take(attrs, name).ok_or_else(|| ParseError::MissingRequiredAttribute {
        xpath: xpath.to_string(),
        name: name.to_string(),
    })
}

/// `raw` is the captured `Function` element, `xpath` its real path and
/// `table_path` the collapsed path the known-element table lists it under.
pub(crate) fn parse_function(
    raw: Vec<u8>,
    xpath: &str,
    table_path: &str,
    source_path: &str,
    schema: &KnownSchema,
    aggregator: &mut UnknownAggregator,
) -> Result<SourceFunction, ParseError> {
    let member_table_path = format!("{table_path}/GroupAddressRef");
    let member_xpath = format!("{xpath}/GroupAddressRef");
    let mut function: Option<SourceFunction> = None;
    let mut depth = 0usize;
    let mut reader = Reader::from_reader(raw.as_slice());
    loop {
        let position = reader.buffer_position();
        let event = reader.read_event().map_err(|e| ParseError::Xml {
            source_path: source_path.to_string(),
            position,
            cause: e.to_string(),
        })?;
        let (start, is_empty) = match &event {
            Event::Start(start) => (start, false),
            Event::Empty(start) => (start, true),
            Event::End(_) => {
                depth = depth.saturating_sub(1);
                continue;
            }
            Event::Eof => break,
            _ => continue,
        };
        let local = start.name().local_name().as_ref().to_string();
        if function.is_none() {
            let mut attrs = classify(
                attr_map(start, source_path, position)?,
                known(schema, table_path),
                xpath,
                aggregator,
            );
            function = Some(SourceFunction {
                id: require(&mut attrs, "Id", xpath)?,
                name: take(&mut attrs, "Name"),
                function_type: take(&mut attrs, "Type"),
                number: take(&mut attrs, "Number"),
                comment: take(&mut attrs, "Comment"),
                description: take(&mut attrs, "Description"),
                completion_status: take(&mut attrs, "CompletionStatus"),
                members: Vec::new(),
                raw: RetainedElement {
                    xpath: xpath.to_string(),
                    name: local,
                    raw: raw.clone(),
                },
            });
            if !is_empty {
                depth = 1;
            }
            continue;
        }
        let function = function.as_mut().expect("set above");
        if depth == 1 && local == "GroupAddressRef" {
            let mut attrs = classify(
                attr_map(start, source_path, position)?,
                known(schema, &member_table_path),
                &member_xpath,
                aggregator,
            );
            function.members.push(SourceFunctionMember {
                id: require(&mut attrs, "Id", &member_xpath)?,
                ref_id: require(&mut attrs, "RefId", &member_xpath)?,
                name: take(&mut attrs, "Name"),
                role: take(&mut attrs, "Role"),
            });
            if !is_empty {
                reader
                    .read_to_end(start.to_end().name())
                    .map_err(|e| ParseError::Xml {
                        source_path: source_path.to_string(),
                        position: reader.buffer_position(),
                        cause: e.to_string(),
                    })?;
            }
            continue;
        }
        // Anything else inside a Function is not in the table: reported,
        // skipped, and kept in the raw copy.
        let child_xpath = format!("{xpath}/{local}");
        aggregator.record(&child_xpath, UnknownKind::Element, &local, None);
        if !is_empty {
            reader
                .read_to_end(start.to_end().name())
                .map_err(|e| ParseError::Xml {
                    source_path: source_path.to_string(),
                    position: reader.buffer_position(),
                    cause: e.to_string(),
                })?;
        }
    }
    function.ok_or_else(|| ParseError::Xml {
        source_path: source_path.to_string(),
        position: 0,
        cause: "empty Function capture".to_string(),
    })
}
