//! Streaming helpers over `quick-xml`, shared by every parser in this crate.
//!
//! Deliberately small: this crate stores each file whole as a blob, so its
//! parsers do not need `knx-etsproj`'s retained-fragment machinery — the
//! bytes are the retention mechanism (ADR-0011). What is needed is exactly
//! attribute lookup, subtree skipping (the `Dynamic` tree, RESEARCH §4.1),
//! and errors that name the file they came from.

use std::collections::{BTreeMap, HashSet};

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use crate::ProductDbError;

/// One element's attributes, owned and decoded, keyed by local name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attrs {
    values: BTreeMap<String, String>,
    evidence: BTreeMap<String, String>,
}

impl Attrs {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.values.get(name).map(String::as_str)
    }

    /// Every attribute name present, so a parser can report the ones it
    /// does not know instead of dropping them.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.evidence.keys().map(String::as_str)
    }

    pub fn evidence_value(&self, name: &str) -> Option<&str> {
        self.evidence.get(name).map(String::as_str)
    }
}

pub fn local_name(e: &BytesStart) -> String {
    e.local_name().as_ref().to_string()
}

pub fn attrs(e: &BytesStart, source_path: &str) -> Result<Attrs, ProductDbError> {
    let mut values = BTreeMap::new();
    let mut evidence = BTreeMap::new();
    let mut unqualified = HashSet::new();
    for attr in e.attributes() {
        let attr = attr.map_err(|err| ProductDbError::Xml {
            source_path: source_path.to_string(),
            cause: err.to_string(),
        })?;
        let raw_key = attr.key.as_ref().to_string();
        let key = attr.key.local_name().as_ref().to_string();
        let value = attr
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|err| ProductDbError::Xml {
                source_path: source_path.to_string(),
                cause: err.to_string(),
            })?
            .into_owned();
        evidence.insert(raw_key, value.clone());
        let is_unqualified = !attr.key.as_ref().contains(':');
        if is_unqualified {
            unqualified.insert(key.clone());
            values.insert(key, value);
        } else if !unqualified.contains(&key) {
            values.insert(key, value);
        }
    }
    Ok(Attrs { values, evidence })
}

/// Consumes events until the element named `name` closes, counting nested
/// elements of the same name so a `<choose>` inside a `<choose>` does not
/// end the skip early.
pub fn skip_subtree(
    reader: &mut Reader<&[u8]>,
    name: &str,
    source_path: &str,
) -> Result<(), ProductDbError> {
    let mut depth = 1usize;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) if e.name().as_ref() == name => depth += 1,
            Ok(Event::End(e)) if e.name().as_ref() == name => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            Ok(Event::Eof) => {
                return Err(ProductDbError::Xml {
                    source_path: source_path.to_string(),
                    cause: format!("unexpected end of file while skipping <{name}>"),
                })
            }
            Ok(_) => {}
            Err(e) => {
                return Err(ProductDbError::Xml {
                    source_path: source_path.to_string(),
                    cause: e.to_string(),
                })
            }
        }
    }
}

/// Complete XML validation shared by package admission and v12 backfill.
/// Domain parsers may ignore unknown elements; validation may not.
pub(crate) fn validate_complete_document(path: &str, bytes: &[u8]) -> Result<(), ProductDbError> {
    let error = |cause: String| ProductDbError::Xml {
        source_path: path.to_string(),
        cause,
    };
    let mut reader = Reader::from_reader(bytes);
    reader.config_mut().check_comments = true;
    let mut depth = 0;
    let mut roots = 0;
    let mut seen_content = false;
    let mut seen_decl = false;
    let mut seen_doctype = false;
    loop {
        let event = reader.read_event().map_err(|e| error(e.to_string()))?;
        match &event {
            Event::Decl(_) if seen_content || seen_decl => {
                return Err(error("XML declaration is not first".to_string()));
            }
            Event::Decl(_) => seen_decl = true,
            Event::DocType(_) if seen_doctype || roots != 0 || depth != 0 => {
                return Err(error("DOCTYPE outside XML prolog".to_string()));
            }
            Event::DocType(_) => seen_doctype = true,
            Event::Start(element) => {
                for attribute in element.attributes().with_checks(true) {
                    let attribute = attribute.map_err(|e| error(e.to_string()))?;
                    attribute
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|e| error(e.to_string()))?;
                }
                if depth == 0 {
                    roots += 1;
                }
                depth += 1;
            }
            Event::Empty(element) => {
                for attribute in element.attributes().with_checks(true) {
                    let attribute = attribute.map_err(|e| error(e.to_string()))?;
                    attribute
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|e| error(e.to_string()))?;
                }
                if depth == 0 {
                    roots += 1;
                }
            }
            Event::End(_) if depth > 0 => depth -= 1,
            Event::End(_) => return Err(error("end tag outside XML root".to_string())),
            Event::Text(text)
                if depth == 0 && !text.as_ref().bytes().all(|b| b.is_ascii_whitespace()) =>
            {
                return Err(error("text outside XML root".to_string()));
            }
            Event::CData(_) if depth == 0 => {
                return Err(error("CDATA outside XML root".to_string()));
            }
            Event::GeneralRef(reference) => {
                let name = reference.as_ref();
                let predefined = matches!(name, "lt" | "gt" | "amp" | "apos" | "quot");
                let numeric = name
                    .strip_prefix("#x")
                    .filter(|digits| {
                        !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_hexdigit())
                    })
                    .map(|digits| u32::from_str_radix(digits, 16).ok())
                    .or_else(|| {
                        name.strip_prefix('#')
                            .filter(|digits| {
                                !digits.is_empty() && digits.bytes().all(|b| b.is_ascii_digit())
                            })
                            .map(|digits| digits.parse().ok())
                    })
                    .flatten()
                    .filter(|code| {
                        matches!(code, 0x9 | 0xA | 0xD)
                            || (0x20..=0xD7FF).contains(code)
                            || (0xE000..=0xFFFD).contains(code)
                            || (0x10000..=0x10FFFF).contains(code)
                    })
                    .is_some();
                if depth == 0 || (!predefined && !numeric) {
                    return Err(error(format!("undeclared XML entity {name:?}")));
                }
            }
            Event::Eof => break,
            _ => {}
        }
        if !matches!(event, Event::Decl(_)) {
            seen_content = true;
        }
    }
    if depth != 0 || roots != 1 {
        return Err(error("incomplete XML document".to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::events::Event;
    use quick_xml::Reader;

    #[test]
    fn attributes_are_read_by_local_name_regardless_of_namespace_prefix() {
        let mut r = Reader::from_reader(
            br#"<Hardware Id="M-006A_H-1" SerialNumber="EM12102" BusCurrent="1.5e+001"/>"#.as_ref(),
        );
        let mut buf = Vec::new();
        let start = match r.read_event_into(&mut buf).unwrap() {
            Event::Empty(e) => e,
            other => panic!("expected an empty element, got {other:?}"),
        };
        let a = attrs(&start, "t.xml").unwrap();
        assert_eq!(a.get("Id"), Some("M-006A_H-1"));
        assert_eq!(a.get("SerialNumber"), Some("EM12102"));
        assert_eq!(a.get("Missing"), None);
    }

    #[test]
    fn unqualified_attributes_win_local_name_collisions_in_both_orders() {
        for xml in [
            br#"<when xmlns:e="urn:example" default="true" e:default="false"/>"#.as_slice(),
            br#"<when xmlns:e="urn:example" e:default="false" default="true"/>"#.as_slice(),
        ] {
            let mut reader = Reader::from_reader(xml);
            let mut buf = Vec::new();
            let start = match reader.read_event_into(&mut buf).unwrap() {
                Event::Empty(element) => element,
                other => panic!("expected an empty element, got {other:?}"),
            };

            let attributes = attrs(&start, "t.xml").unwrap();
            assert_eq!(attributes.get("default"), Some("true"));
            let evidence_names = attributes.names().collect::<Vec<_>>();
            assert!(evidence_names.contains(&"default"));
            assert!(evidence_names.contains(&"e:default"));
        }
    }

    #[test]
    fn a_skipped_subtree_leaves_the_reader_after_its_end_tag() {
        let xml = br#"<Root><Dynamic><Channel><Block/></Channel></Dynamic><After Id="x"/></Root>"#;
        let mut r = Reader::from_reader(xml.as_ref());
        let mut buf = Vec::new();
        // <Root>
        r.read_event_into(&mut buf).unwrap();
        buf.clear();
        // <Dynamic>
        match r.read_event_into(&mut buf).unwrap() {
            Event::Start(e) => skip_subtree(&mut r, e.name().as_ref(), "t.xml").unwrap(),
            other => panic!("expected <Dynamic>, got {other:?}"),
        }
        buf.clear();
        match r.read_event_into(&mut buf).unwrap() {
            Event::Empty(e) => assert_eq!(local_name(&e), "After"),
            other => panic!("expected <After/>, got {other:?}"),
        }
    }

    #[test]
    fn a_nested_element_of_the_same_name_does_not_end_the_skip_early() {
        let xml = br#"<Root><choose><choose><when/></choose></choose><After Id="x"/></Root>"#;
        let mut r = Reader::from_reader(xml.as_ref());
        let mut buf = Vec::new();
        r.read_event_into(&mut buf).unwrap();
        buf.clear();
        match r.read_event_into(&mut buf).unwrap() {
            Event::Start(e) => skip_subtree(&mut r, e.name().as_ref(), "t.xml").unwrap(),
            other => panic!("expected <choose>, got {other:?}"),
        }
        buf.clear();
        match r.read_event_into(&mut buf).unwrap() {
            Event::Empty(e) => assert_eq!(local_name(&e), "After"),
            other => panic!("expected <After/>, got {other:?}"),
        }
    }

    #[test]
    fn truncated_xml_is_an_error_naming_its_source_path() {
        let mut r = Reader::from_reader(br#"<Root><Dynamic>"#.as_ref());
        let mut buf = Vec::new();
        r.read_event_into(&mut buf).unwrap();
        buf.clear();
        let e = match r.read_event_into(&mut buf).unwrap() {
            Event::Start(e) => e,
            other => panic!("expected <Dynamic>, got {other:?}"),
        };
        let err = skip_subtree(&mut r, e.name().as_ref(), "M-0083/A.xml").unwrap_err();
        assert!(format!("{err}").contains("M-0083/A.xml"));
    }
}
