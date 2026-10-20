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
    use super::{
        parse, Event, Node, NsReader, MAX_ATTRIBUTES_PER_NODE, MAX_DEPTH, MAX_EVENTS, MAX_NODES,
        MAX_TREE_BYTES, MAX_XML_BYTES,
    };
    const NS: &str = "http://knx.org/xml/project/20";
    const ROOT: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"/>"#;
    const ROOT_OPEN: &str = r#"<KNX xmlns="http://knx.org/xml/project/20">"#;
    const ROOT_CLOSE: &str = "</KNX>";

    // Count the actual fixture events, including EOF and Empty element levels,
    // independently of parse's counters. This catches fixture-shape mistakes
    // before a shared xml-work-limit code could disguise another first refusal.
    fn assert_fixture_shape(
        xml: &str,
        expected_nodes: usize,
        expected_depth: usize,
        expected_events: usize,
        expected_attributes: usize,
    ) -> (usize, usize, usize) {
        assert!(xml.len() < MAX_XML_BYTES);
        let mut reader = NsReader::from_reader(xml.as_bytes());
        let (mut nodes, mut depth, mut max_depth, mut events, mut max_attributes) = (0, 0, 0, 0, 0);
        loop {
            let (_, event) = reader.read_resolved_event().expect("well-formed fixture");
            events += 1;
            match event {
                Event::Start(ref e) | Event::Empty(ref e) => {
                    nodes += 1;
                    max_depth = max_depth.max(depth + 1);
                    let attributes = e.attributes().map(Result::unwrap).count();
                    max_attributes = max_attributes.max(attributes);
                    if matches!(event, Event::Start(_)) {
                        depth += 1;
                    }
                }
                Event::End(_) => depth -= 1,
                Event::Eof => break,
                _ => {}
            }
        }
        assert_eq!(depth, 0);
        assert_eq!(nodes, expected_nodes);
        assert_eq!(max_depth, expected_depth);
        assert_eq!(events, expected_events);
        assert_eq!(max_attributes, expected_attributes);
        assert!(max_attributes < MAX_ATTRIBUTES_PER_NODE);
        (nodes, max_depth, events)
    }

    fn assert_identity(node: &Node, name: &str, namespace: &str) {
        assert_eq!(node.name, name);
        assert_eq!(node.namespace, namespace);
        assert!(node.attributes.is_empty());
    }

    #[test]
    fn node_limit_has_an_independent_inclusive_boundary() {
        assert_eq!(MAX_NODES, 65_536);
        const LEAF: &str = "<Leaf/>";
        for child_count in [MAX_NODES - 1, MAX_NODES] {
            let xml = format!("{ROOT_OPEN}{}{ROOT_CLOSE}", LEAF.repeat(child_count));
            let events = child_count + 3; // root Start, Empty leaves, root End, EOF
            let (_, depth, actual_events) =
                assert_fixture_shape(&xml, child_count + 1, 2, events, 1);
            assert!(depth < MAX_DEPTH);
            assert!(actual_events < MAX_EVENTS);
            let charge = 256 + NS.len() + 3 + child_count * (256 + NS.len() + 4);
            assert!(charge < MAX_TREE_BYTES);
            if child_count == MAX_NODES {
                assert!(matches!(parse(xml.as_bytes()), Err("xml-work-limit")));
                continue;
            }
            let root = parse(xml.as_bytes()).expect("exact node allowance is inclusive");
            assert_identity(&root, "KNX", NS);
            assert!(root.text.is_empty());
            assert_eq!(root.children.len(), 65_535);
            assert_eq!((root.byte_start, root.byte_end), (0, xml.len() as u64));
            for (index, leaf) in root.children.iter().enumerate() {
                assert_identity(leaf, "Leaf", NS);
                assert!(leaf.text.is_empty());
                assert!(leaf.children.is_empty());
                let start = ROOT_OPEN.len() + index * LEAF.len();
                assert_eq!(
                    (leaf.byte_start, leaf.byte_end),
                    (start as u64, (start + LEAF.len()) as u64)
                );
                assert_eq!(&xml[start..start + LEAF.len()], LEAF);
            }
        }
    }

    #[test]
    fn depth_limit_counts_root_and_empty_leaves() {
        assert_eq!(MAX_DEPTH, 64);
        const OPEN: &str = "<Branch>";
        const CLOSE: &str = "</Branch>";
        for leaf_xml in ["<Leaf/>", "<Leaf>edge</Leaf>"] {
            let empty = leaf_xml == "<Leaf/>";
            for depth in [MAX_DEPTH, MAX_DEPTH + 1] {
                let xml = format!(
                    "{ROOT_OPEN}{}{leaf_xml}{}{ROOT_CLOSE}",
                    OPEN.repeat(depth - 2),
                    CLOSE.repeat(depth - 2)
                );
                let events = if empty { 2 * depth } else { 2 * depth + 2 };
                let (nodes, _, actual_events) = assert_fixture_shape(&xml, depth, depth, events, 1);
                assert!(nodes < MAX_NODES);
                assert!(actual_events < MAX_EVENTS);
                let charge = 256
                    + NS.len()
                    + 3
                    + (depth - 2) * (256 + NS.len() + "Branch".len())
                    + 256
                    + NS.len()
                    + "Leaf".len()
                    + if empty { 0 } else { "edge".len() };
                assert!(charge < MAX_TREE_BYTES);
                if depth == MAX_DEPTH + 1 {
                    assert!(matches!(parse(xml.as_bytes()), Err("xml-work-limit")));
                    continue;
                }
                let root = parse(xml.as_bytes()).expect("exact depth allowance is inclusive");
                let mut node = &root;
                for level in 0..depth {
                    let leaf = level == depth - 1;
                    let name = if level == 0 {
                        "KNX"
                    } else if leaf {
                        "Leaf"
                    } else {
                        "Branch"
                    };
                    assert_identity(node, name, NS);
                    assert_eq!(node.text, if leaf && !empty { "edge" } else { "" });
                    assert_eq!(node.children.len(), usize::from(!leaf));
                    let (start, end) = if level == 0 {
                        (0, xml.len())
                    } else if leaf {
                        let start = ROOT_OPEN.len() + (depth - 2) * OPEN.len();
                        assert_eq!(&xml[start..start + leaf_xml.len()], leaf_xml);
                        (start, start + leaf_xml.len())
                    } else {
                        (
                            ROOT_OPEN.len() + (level - 1) * OPEN.len(),
                            xml.len() - ROOT_CLOSE.len() - (level - 1) * CLOSE.len(),
                        )
                    };
                    assert_eq!((node.byte_start, node.byte_end), (start as u64, end as u64));
                    if !leaf {
                        node = &node.children[0];
                    }
                }
            }
        }
    }

    #[test]
    fn event_limit_reserves_a_slot_for_eof() {
        assert_eq!(MAX_EVENTS, 200_000);
        const COMMENT: &str = "<!--event-->";
        for comments in [MAX_EVENTS - 2, MAX_EVENTS - 1] {
            let xml = format!("{ROOT}{}", COMMENT.repeat(comments));
            let (nodes, depth, _) = assert_fixture_shape(&xml, 1, 1, comments + 2, 1);
            assert!(nodes < MAX_NODES);
            assert!(depth < MAX_DEPTH);
            assert!(nodes * (256 + NS.len() + "KNX".len()) < MAX_TREE_BYTES);
            if comments == MAX_EVENTS - 1 {
                // The last permitted call reads a comment, leaving no EOF slot.
                assert!(matches!(parse(xml.as_bytes()), Err("xml-event-limit")));
                continue;
            }
            let root = parse(xml.as_bytes()).expect("EOF in event slot 200000 is admitted");
            assert_identity(&root, "KNX", NS);
            assert!(root.text.is_empty());
            assert!(root.children.is_empty());
            assert_eq!((root.byte_start, root.byte_end), (0, ROOT.len() as u64));
            assert_eq!(&xml[..root.byte_end as usize], ROOT);
        }
    }

    #[test]
    fn tree_charge_has_a_positive_exact_boundary() {
        assert_eq!(MAX_TREE_BYTES, 67_108_864);
        const CHILDREN: usize = 1000;
        const LEAF: &str = "<f:Unknown/>";
        let namespace = format!("urn:{}", "x".repeat(65_536));
        assert_eq!(namespace.len(), 65_540);
        let root_charge = 256 + NS.len() + "KNX".len();
        let child_charge = 256 + namespace.len() + "Unknown".len();
        let text_len = MAX_TREE_BYTES - root_charge - CHILDREN * child_charge;
        assert_eq!(
            (root_charge, child_charge, text_len),
            (288, 65_803, 1_305_576)
        );
        assert_eq!(root_charge + CHILDREN * child_charge + text_len, 67_108_864);
        let open = format!(r#"<KNX xmlns="{NS}" xmlns:f="{namespace}">"#);
        let text = "t".repeat(text_len);
        let mut xml = format!("{open}{}{text}{ROOT_CLOSE}", LEAF.repeat(CHILDREN));
        assert_eq!(
            xml.len(),
            open.len() + CHILDREN * LEAF.len() + text_len + ROOT_CLOSE.len()
        );
        let (nodes, depth, events) = assert_fixture_shape(&xml, CHILDREN + 1, 2, CHILDREN + 4, 2);
        assert!(nodes < MAX_NODES);
        assert!(depth < MAX_DEPTH);
        assert!(events < MAX_EVENTS);
        {
            let root = parse(xml.as_bytes()).expect("exact retained-tree charge is inclusive");
            assert_identity(&root, "KNX", NS);
            assert_eq!(root.text, text);
            assert_eq!(root.children.len(), CHILDREN);
            assert_eq!((root.byte_start, root.byte_end), (0, xml.len() as u64));
            for (index, leaf) in root.children.iter().enumerate() {
                assert_identity(leaf, "Unknown", &namespace);
                assert!(leaf.text.is_empty());
                assert!(leaf.children.is_empty());
                let start = open.len() + index * LEAF.len();
                assert_eq!(
                    (leaf.byte_start, leaf.byte_end),
                    (start as u64, (start + LEAF.len()) as u64)
                );
                assert_eq!(&xml[start..start + LEAF.len()], LEAF);
            }
        }
        // One retained text byte changes no element/attribute/event/depth count.
        // In particular this is not a source-byte, node or event-limit refusal.
        xml.insert(xml.len() - ROOT_CLOSE.len(), 't');
        assert_eq!(
            root_charge + CHILDREN * child_charge + text_len + 1,
            67_108_865
        );
        assert_fixture_shape(&xml, CHILDREN + 1, 2, CHILDREN + 4, 2);
        assert!(matches!(parse(xml.as_bytes()), Err("xml-work-limit")));
    }
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
