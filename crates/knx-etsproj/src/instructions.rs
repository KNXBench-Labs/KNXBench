//! Original installation-instruction reads, distinct from derived RTF views.
use crate::rtf::{TextError, TextInspection};

pub const MAX_XML_BYTES: usize = 8 * 1024 * 1024;
pub const MAX_XML_EVENTS: usize = 100_000;
pub const MAX_XML_DEPTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallationHints {
    pub original: String,
    pub preview: Result<TextInspection, TextError>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InstructionReadError {
    SourceLimit,
    EventLimit,
    DepthLimit,
    UnsupportedNamespace,
    Malformed,
    MissingDevice,
    AmbiguousDevice,
    DuplicateField,
}

/// Reads only an exact source owner, without modifying imported bytes.
///
/// Initial admission is Schema23 with unprefixed element names. Whole XML is
/// validated and budgeted before the existing source parser is used. Preview
/// refusal is kept alongside the exact logical original, not returned as a
/// missing value. This function never reads files, URLs or external entities.
pub fn read_installation_hints(
    xml: &[u8],
    source_path: &str,
    ets_id: &str,
) -> Result<Option<InstallationHints>, InstructionReadError> {
    validate_xml(xml)?;
    let parsed = crate::parse::parse_installation_v21(xml, source_path, &crate::known::SCHEMA_23)
        .map_err(|_| InstructionReadError::Malformed)?;
    let mut matches = parsed
        .document
        .installations
        .iter()
        .flat_map(|installation| {
            installation.unassigned_devices.iter().chain(
                installation
                    .areas
                    .iter()
                    .flat_map(|area| area.lines.iter().flat_map(|line| line.devices.iter())),
            )
        })
        .filter(|device| device.id == ets_id);
    let device = matches.next().ok_or(InstructionReadError::MissingDevice)?;
    if matches.next().is_some() {
        return Err(InstructionReadError::AmbiguousDevice);
    }
    let mut hints = device
        .other
        .iter()
        .filter(|attribute| attribute.name == "InstallationHints");
    let Some(hint) = hints.next() else {
        return Ok(None);
    };
    if hints.next().is_some() {
        return Err(InstructionReadError::DuplicateField);
    }
    Ok(Some(InstallationHints {
        original: hint.value.clone(),
        preview: crate::rtf::inspect_text(&hint.value),
    }))
}

fn xml_character(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
}

// XML1.0 Fifth Edition §2.3 NameStartChar/NameChar, excluding colon for
// the deliberately unprefixed admission boundary (NCName).
fn name_start(c: char) -> bool {
    matches!(c, 'A'..='Z' | '_' | 'a'..='z' | '\u{c0}'..='\u{d6}' | '\u{d8}'..='\u{f6}'
        | '\u{f8}'..='\u{2ff}' | '\u{370}'..='\u{37d}' | '\u{37f}'..='\u{1fff}'
        | '\u{200c}'..='\u{200d}' | '\u{2070}'..='\u{218f}' | '\u{2c00}'..='\u{2fef}'
        | '\u{3001}'..='\u{d7ff}' | '\u{f900}'..='\u{fdcf}' | '\u{fdf0}'..='\u{fffd}'
        | '\u{10000}'..='\u{effff}')
}
fn ncname(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(name_start) && chars.all(|c| name_start(c)
        || matches!(c, '-' | '.' | '0'..='9' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}'))
}
fn namespace_attribute(name: &str, value: &str) -> bool {
    const XML: &str = "http://www.w3.org/XML/1998/namespace";
    const XMLNS: &str = "http://www.w3.org/2000/xmlns/";
    if name == "xmlns" {
        return value != XML && value != XMLNS;
    }
    let Some(prefix) = name.strip_prefix("xmlns:") else {
        return true;
    };
    ncname(prefix)
        && prefix != "xmlns"
        && !value.is_empty()
        && value != XMLNS
        && ((prefix == "xml") == (value == XML))
}

fn validate_xml(xml: &[u8]) -> Result<(), InstructionReadError> {
    use quick_xml::events::Event;
    use quick_xml::name::ResolveResult;
    use quick_xml::{NsReader, XmlVersion};
    use InstructionReadError as E;
    if xml.len() > MAX_XML_BYTES {
        return Err(E::SourceLimit);
    }
    // UTF-8 XML1.0 only: other encodings remain retained but uninterpreted.
    let text = std::str::from_utf8(xml).map_err(|_| E::Malformed)?;
    if !text.chars().all(xml_character) {
        return Err(E::Malformed);
    }
    let xml = xml.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(xml);
    let mut reader = NsReader::from_reader(xml);
    reader.config_mut().check_comments = true;
    let mut depth = 0_usize;
    let mut roots = 0_usize;
    let mut events = 0_usize;
    loop {
        events += 1;
        if events > MAX_XML_EVENTS {
            return Err(E::EventLimit);
        }
        let before = reader.buffer_position();
        let (namespace, event) = reader.read_resolved_event().map_err(|_| E::Malformed)?;
        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                if !matches!(namespace, ResolveResult::Bound(ref ns) if ns.as_ref() == "http://knx.org/xml/project/23")
                    || start.name().as_ref().contains(':')
                {
                    return Err(E::UnsupportedNamespace);
                }
                if !ncname(start.name().as_ref()) {
                    return Err(E::Malformed);
                }
                if depth == 0 {
                    roots += 1;
                    if roots != 1 || start.name().as_ref() != "KNX" {
                        return Err(E::Malformed);
                    }
                }
                if depth + 1 > MAX_XML_DEPTH {
                    return Err(E::DepthLimit);
                }
                for attribute in start.attributes() {
                    let attribute = attribute.map_err(|_| E::Malformed)?;
                    let name = attribute.key.as_ref();
                    if name.contains(':') && !name.starts_with("xmlns:") {
                        return Err(E::UnsupportedNamespace);
                    }
                    let value = attribute
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|_| E::Malformed)?;
                    if !value.chars().all(xml_character)
                        || !namespace_attribute(name, &value)
                        || !(ncname(name) || name.strip_prefix("xmlns:").is_some_and(ncname))
                    {
                        return Err(E::Malformed);
                    }
                }
                if matches!(event, Event::Start(_)) {
                    depth += 1;
                }
            }
            Event::End(_) => {
                depth = depth.checked_sub(1).ok_or(E::Malformed)?;
            }
            Event::DocType(_) => return Err(E::Malformed),
            Event::Decl(declaration) => {
                if before != 0 || roots != 0 || depth != 0 {
                    return Err(E::Malformed);
                }
                let start = quick_xml::events::BytesStart::from_content(declaration.as_ref(), 3);
                let mut position = 0;
                for attribute in start.attributes() {
                    let attribute = attribute.map_err(|_| E::Malformed)?;
                    match (position, attribute.key.as_ref(), attribute.value.as_ref()) {
                        (0, "version", "1.0") => position = 1,
                        (1, "encoding", value) if value.eq_ignore_ascii_case("UTF-8") => {
                            position = 2
                        }
                        (1 | 2, "standalone", "yes" | "no") => position = 3,
                        _ => return Err(E::Malformed),
                    }
                }
                if position == 0 {
                    return Err(E::Malformed);
                }
            }
            Event::Text(text) => {
                if depth == 0
                    && !text
                        .as_ref()
                        .chars()
                        .all(|c| matches!(c, ' ' | '\t' | '\r' | '\n'))
                {
                    return Err(E::Malformed);
                }
            }
            Event::GeneralRef(reference) => {
                if depth == 0 {
                    return Err(E::Malformed);
                }
                match reference.resolve_char_ref().map_err(|_| E::Malformed)? {
                    Some(c) if xml_character(c) => {}
                    None if matches!(reference.as_ref(), "lt" | "gt" | "amp" | "apos" | "quot") => {
                    }
                    _ => return Err(E::Malformed),
                }
            }
            Event::CData(_) if depth == 0 => return Err(E::Malformed),
            Event::PI(instruction) => {
                if instruction.target().eq_ignore_ascii_case("xml") || !ncname(instruction.target())
                {
                    return Err(E::Malformed);
                }
            }
            Event::Eof => {
                return if depth == 0 && roots == 1 {
                    Ok(())
                } else {
                    Err(E::Malformed)
                };
            }
            _ => {}
        }
    }
}
