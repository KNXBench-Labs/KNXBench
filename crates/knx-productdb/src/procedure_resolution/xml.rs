//! Parses bounded namespace-aware XML for declarative procedure inspection.
use super::{Node, MAX_XML_BYTES};
use quick_xml::{
    events::{BytesStart, Event},
    name::ResolveResult,
    NsReader, XmlVersion,
};
use std::collections::BTreeMap;
const MAX_EVENTS: usize = 200_000;
const MAX_DEPTH: usize = 64;
const MAX_NODES: usize = 65_536;
// Charge expanded names as well as input bytes: one namespace declaration
// can otherwise be copied into thousands of retained node identities.
const MAX_TREE_BYTES: usize = 64 * 1024 * 1024;
const MAX_ATTRIBUTES_PER_NODE: usize = 256;
fn charge(total: &mut usize, bytes: usize) -> Result<(), &'static str> {
    *total = total
        .checked_add(bytes)
        .filter(|n| *n <= MAX_TREE_BYTES)
        .ok_or("xml-work-limit")?;
    Ok(())
}

fn namespace(ns: ResolveResult<'_>) -> Result<String, &'static str> {
    match ns {
        ResolveResult::Bound(n) => Ok(n.as_ref().to_owned()),
        _ => Err("unbound-namespace"),
    }
}
pub(super) fn parse(bytes: &[u8]) -> Result<Node, &'static str> {
    if bytes.len() > MAX_XML_BYTES {
        return Err("source-byte-limit");
    }
    std::str::from_utf8(bytes).map_err(|_| "non-utf8-source")?;
    let mut reader = NsReader::from_reader(bytes);
    let mut stack: Vec<Node> = Vec::new();
    let mut root = None;
    let mut nodes = 0;
    let mut tree_bytes = 0;
    let mut seen_content = false;
    let mut seen_decl = false;
    for _ in 0..MAX_EVENTS {
        let start = reader.buffer_position();
        let (ns, event) = reader.read_resolved_event().map_err(|_| "malformed-xml")?;
        let is_decl = matches!(&event, Event::Decl(_));
        match event {
            Event::Decl(ref decl) => {
                if seen_decl || seen_content {
                    return Err("malformed-xml-declaration");
                }
                seen_decl = true;
                let header = BytesStart::from_content(decl.as_ref(), 3);
                let mut previous = 0;
                for attribute in header.attributes().with_checks(true) {
                    let attribute = attribute.map_err(|_| "malformed-xml-declaration")?;
                    let rank = match attribute.key.as_ref() {
                        "version" => 1,
                        "encoding" => 2,
                        "standalone" => 3,
                        _ => return Err("malformed-xml-declaration"),
                    };
                    if rank <= previous {
                        return Err("malformed-xml-declaration");
                    }
                    previous = rank;
                    if rank == 3 && !matches!(attribute.value.as_ref(), "yes" | "no") {
                        return Err("malformed-xml-declaration");
                    }
                }
                if decl
                    .version()
                    .map_err(|_| "malformed-xml-declaration")?
                    .as_ref()
                    != "1.0"
                {
                    return Err("unsupported-xml-version");
                }
                if let Some(encoding) = decl.encoding() {
                    if !encoding
                        .map_err(|_| "malformed-xml-declaration")?
                        .eq_ignore_ascii_case("UTF-8")
                    {
                        return Err("unsupported-xml-encoding");
                    }
                }
            }
            Event::Start(ref e) | Event::Empty(ref e) => {
                nodes += 1;
                if nodes > MAX_NODES || stack.len() >= MAX_DEPTH {
                    return Err("xml-work-limit");
                }
                let ns = namespace(ns)?;
                charge(
                    &mut tree_bytes,
                    256 + ns.len() + e.local_name().as_ref().len(),
                )?;
                let mut attributes = BTreeMap::new();
                for (index, a) in e.attributes().enumerate() {
                    if index >= MAX_ATTRIBUTES_PER_NODE {
                        return Err("xml-work-limit");
                    }
                    let a = a.map_err(|_| "duplicate-or-invalid-attribute")?;
                    if a.key.as_ref() == "xmlns" || a.key.as_ref().starts_with("xmlns:") {
                        continue;
                    }
                    let (ans, local) = reader.resolver().resolve_attribute(a.key);
                    let key = match ans {
                        ResolveResult::Unbound => local.as_ref().to_owned(),
                        ResolveResult::Bound(n) => format!("{{{}}}{}", n.as_ref(), local.as_ref()),
                        ResolveResult::Unknown(_) => return Err("unbound-attribute"),
                    };
                    let value = a
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|_| "malformed-attribute")?
                        .into_owned();
                    charge(&mut tree_bytes, 96 + key.len() + value.len())?;
                    if attributes.insert(key, value).is_some() {
                        return Err("duplicate-expanded-attribute");
                    }
                }
                let node = Node {
                    namespace: ns,
                    name: e.local_name().as_ref().to_owned(),
                    attributes,
                    children: vec![],
                    text: String::new(),
                    byte_start: start,
                    byte_end: reader.buffer_position(),
                };
                if matches!(event, Event::Empty(_)) {
                    attach(node, &mut stack, &mut root)?;
                } else {
                    stack.push(node);
                }
            }
            Event::End(_) => {
                let mut node = stack.pop().ok_or("unmatched-end")?;
                node.byte_end = reader.buffer_position();
                attach(node, &mut stack, &mut root)?;
            }
            Event::Text(t) => {
                if let Some(n) = stack.last_mut() {
                    charge(&mut tree_bytes, t.as_ref().len())?;
                    n.text.push_str(t.as_ref());
                } else if !t.as_ref().trim().is_empty() {
                    return Err("text-outside-root");
                }
            }
            Event::CData(t) => {
                charge(&mut tree_bytes, t.as_ref().len())?;
                stack
                    .last_mut()
                    .ok_or("text-outside-root")?
                    .text
                    .push_str(t.as_ref());
            }
            Event::GeneralRef(r) => {
                let s = quick_xml::escape::unescape(&format!("&{};", r.as_ref()))
                    .map_err(|_| "invalid-entity")?
                    .into_owned();
                charge(&mut tree_bytes, s.len())?;
                stack
                    .last_mut()
                    .ok_or("text-outside-root")?
                    .text
                    .push_str(&s);
            }
            Event::DocType(_) => return Err("dtd-refused"),
            Event::Eof => {
                if !stack.is_empty() {
                    return Err("incomplete-xml");
                }
                let root: Node = root.ok_or("missing-root")?;
                let tail = root
                    .namespace
                    .strip_prefix("http://knx.org/xml/project/")
                    .ok_or("unsupported-namespace")?;
                if root.name != "KNX"
                    || !matches!(tail, "10" | "11" | "12" | "13" | "14" | "20" | "21" | "23")
                {
                    return Err("unsupported-namespace");
                }
                return Ok(root);
            }
            _ => {}
        }
        if !is_decl {
            seen_content = true;
        }
    }
    Err("xml-event-limit")
}
fn attach(node: Node, stack: &mut [Node], root: &mut Option<Node>) -> Result<(), &'static str> {
    if let Some(parent) = stack.last_mut() {
        parent.children.push(node);
    } else if root.replace(node).is_some() {
        return Err("multiple-roots");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse;
    const ROOT: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"/>"#;
    #[test]
    fn wide_attribute_lists_have_a_positive_boundary_and_named_refusal() {
        let attributes = |count: usize| {
            (0..count)
                .map(|i| format!(" a{i}=\"value\""))
                .collect::<String>()
        };
        let valid = format!(
            "<KNX xmlns=\"http://knx.org/xml/project/20\"{}/>",
            attributes(255)
        );
        assert!(parse(valid.as_bytes()).is_ok());
        let excessive = format!(
            "<KNX xmlns=\"http://knx.org/xml/project/20\"{}/>",
            attributes(256)
        );
        assert!(matches!(parse(excessive.as_bytes()), Err("xml-work-limit")));
    }
    #[test]
    fn repeated_long_namespaces_are_bounded_before_tree_amplification() {
        let namespace = format!("urn:{}", "x".repeat(65536));
        let xml = format!(
            "<KNX xmlns=\"http://knx.org/xml/project/20\" xmlns:f=\"{namespace}\">{}</KNX>",
            "<f:Unknown/>".repeat(1100)
        );
        assert!(matches!(parse(xml.as_bytes()), Err("xml-work-limit")));
    }
    #[test]
    fn late_xml_declarations_are_not_tolerated() {
        assert!(parse(format!("{ROOT}<?xml version=\"1.0\"?>").as_bytes()).is_err());
    }
    #[test]
    fn non_utf8_declared_encoding_is_not_silently_interpreted_as_utf8() {
        assert!(
            parse(format!("<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>{ROOT}").as_bytes())
                .is_err()
        );
    }
    #[test]
    fn xml_11_is_not_processed_using_xml_10_attribute_rules() {
        assert!(parse(format!("<?xml version=\"1.1\"?>{ROOT}").as_bytes()).is_err());
    }
    #[test]
    fn malformed_declaration_attributes_are_not_first_wins() {
        for header in [
            r#"<?xml version="1.0" encoding="UTF-8" encoding="ISO-8859-1"?>"#,
            r#"<?xml version="1.0" standalone="maybe"?>"#,
            r#"<?xml version="1.0" Future="ignored"?>"#,
            r#"<?xml version="1.0" standalone="yes" encoding="UTF-8"?>"#,
        ] {
            assert!(parse(format!("{header}{ROOT}").as_bytes()).is_err());
        }
    }
    #[test]
    fn explicit_utf8_xml_10_and_namespace_prefixes_remain_supported() {
        assert!(
            parse(format!("<?xml version=\"1.0\" encoding=\"UTF-8\"?>{ROOT}").as_bytes()).is_ok()
        );
        assert!(parse(br#"<k:KNX xmlns:k="http://knx.org/xml/project/20"/>"#).is_ok());
    }
}
