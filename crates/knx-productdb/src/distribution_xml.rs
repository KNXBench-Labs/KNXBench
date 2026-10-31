//! Admits bounded XML and binds scoped reader views to immutable expanded-name occurrences.

use super::*;
use quick_xml::{events::Event, Reader};
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::{BTreeMap, HashSet};
use std::mem::size_of;
use std::rc::Rc;

const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
/// Escapes for the derived view so re-parsing normalization yields the same value: literal
/// tab, line feed and carriage return would otherwise turn into spaces.
fn escape_attribute(value: &str) -> String {
    let escaped = quick_xml::escape::escape(value);
    if !escaped.contains(['\t', '\n', '\r']) {
        return escaped.into_owned();
    }
    escaped
        .replace('\t', "&#9;")
        .replace('\n', "&#10;")
        .replace('\r', "&#13;")
}
/// First capacity step; later steps double so a large member is not copied once per step.
const MIN_GROWTH: usize = 4096;
const XMLNS_NS: &str = "http://www.w3.org/2000/xmlns/";

#[derive(Debug)]
pub(super) struct Attribute<'a> {
    pub raw: String,
    pub expanded: String,
    pub value: Cow<'a, str>,
}
#[derive(Debug)]
pub(super) struct Node<'a> {
    pub local: &'a str,
    pub raw: &'a str,
    pub ns: Rc<str>,
    pub attrs: Box<[Attribute<'a>]>,
    pub parent: Option<usize>,
    pub end_node: usize,
    pub start: usize,
    pub end: usize,
    pub first_token: usize,
    pub last_token: usize,
    sibling: usize,
    declarations: BTreeMap<String, Rc<str>>,
}
impl Node<'_> {
    pub fn attr(&self, name: &str) -> Option<String> {
        self.attrs
            .iter()
            .find(|a| a.raw == name)
            .map(|a| a.value.to_string())
    }
}
#[derive(Debug)]
enum Token {
    Open(usize),
    Close(usize),
    Data(usize, usize),
}
pub(super) struct Document<'a> {
    pub bytes: &'a [u8],
    pub sha256: String,
    pub nodes: Vec<Node<'a>>,
    tokens: Vec<Token>,
    pub namespace: String,
    pub member: Option<u32>,
    projection_scopes: Vec<Option<usize>>,
    work_bytes: Cell<usize>,
    xml_bytes: Cell<usize>,
    max_xml_bytes: usize,
    max_report_bytes: usize,
}
fn malformed(e: impl std::fmt::Display) -> DistributionInspectionError {
    DistributionInspectionError::MalformedXml(e.to_string())
}
fn namespace(e: &str) -> DistributionInspectionError {
    DistributionInspectionError::Namespace(e.into())
}
// Rebind reader-borrowed strings to the immutable input, without unsafe or XML reinterpretation.
fn original<'a>(bytes: &'a [u8], value: &str) -> Result<&'a str> {
    let offset = (value.as_ptr() as usize)
        .checked_sub(bytes.as_ptr() as usize)
        .ok_or_else(|| malformed("XML reader slice outside original"))?;
    let slice = bytes
        .get(
            offset
                ..offset
                    .checked_add(value.len())
                    .ok_or_else(|| limit("XML slice"))?,
        )
        .ok_or_else(|| malformed("XML reader slice outside original"))?;
    std::str::from_utf8(slice).map_err(malformed)
}
fn xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | '\u{20}'..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..='\u{10ffff}')
}
fn name_start(c: char) -> bool {
    matches!(c, 'A'..='Z' | '_' | 'a'..='z' | '\u{c0}'..='\u{d6}' | '\u{d8}'..='\u{f6}' | '\u{f8}'..='\u{2ff}' | '\u{370}'..='\u{37d}' | '\u{37f}'..='\u{1fff}' | '\u{200c}'..='\u{200d}' | '\u{2070}'..='\u{218f}' | '\u{2c00}'..='\u{2fef}' | '\u{3001}'..='\u{d7ff}' | '\u{f900}'..='\u{fdcf}' | '\u{fdf0}'..='\u{fffd}' | '\u{10000}'..='\u{effff}')
}
fn ncname(s: &str) -> bool {
    let mut chars = s.chars();
    chars.next().is_some_and(name_start) && chars.all(|c| name_start(c) || matches!(c, '-' | '.' | '0'..='9' | '\u{b7}' | '\u{300}'..='\u{36f}' | '\u{203f}'..='\u{2040}'))
}
fn qname(s: &str) -> Result<(&str, &str)> {
    let (prefix, local) = s.split_once(':').unwrap_or(("", s));
    if s.len() > 512 {
        return Err(limit("XML name bytes"));
    }
    if !ncname(local)
        || (!prefix.is_empty() && !ncname(prefix))
        || (s.contains(':') && prefix.is_empty())
    {
        return Err(namespace("invalid QName"));
    }
    Ok((prefix, local))
}
fn encoding(bytes: &[u8]) -> Result<()> {
    if bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]) {
        let little = bytes[0] == 0xff;
        let units = bytes[2..]
            .as_chunks::<2>()
            .0
            .iter()
            .take(256)
            .map(|p| {
                if little {
                    u16::from_le_bytes([p[0], p[1]])
                } else {
                    u16::from_be_bytes([p[0], p[1]])
                }
            })
            .collect::<Vec<_>>();
        // Decode only a bounded diagnostic prolog. No repaired/UTF-16 document is admitted.
        let prolog = String::from_utf16_lossy(&units);
        let mut reader = Reader::from_reader(prolog.as_bytes());
        match reader.read_event() {
            Ok(Event::Decl(declaration))
                if declaration
                    .encoding()
                    .is_some_and(|enc| enc.is_ok_and(|enc| enc.eq_ignore_ascii_case("UTF-8"))) =>
            {
                return Err(DistributionInspectionError::EncodingConflict)
            }
            _ => {}
        }
        return Err(DistributionInspectionError::UnsupportedEncoding(
            "UTF-16".into(),
        ));
    }
    if bytes.starts_with(&[0, 0, 0xfe, 0xff])
        || bytes.starts_with(&[0, 0, 0, b'<'])
        || bytes.starts_with(&[b'<', 0, 0, 0])
    {
        return Err(DistributionInspectionError::UnsupportedEncoding(
            "UTF-32".into(),
        ));
    }
    if bytes.starts_with(&[0, b'<']) || bytes.starts_with(&[b'<', 0]) {
        return Err(DistributionInspectionError::UnsupportedEncoding(
            "UTF-16 without BOM".into(),
        ));
    }
    let text =
        std::str::from_utf8(bytes).map_err(|_| DistributionInspectionError::EncodingConflict)?;
    if !text.chars().all(xml_char) {
        return Err(malformed("illegal XML character"));
    }
    Ok(())
}

impl<'a> Document<'a> {
    fn parse(bytes: &'a [u8], member: Option<u32>, limits: &DistributionLimits) -> Result<Self> {
        if bytes.len() as u64 > limits.max_member_bytes {
            return Err(limit("XML member bytes"));
        }
        encoding(bytes)?;
        let mut doc = Self {
            bytes,
            sha256: crate::sha256_hex(bytes),
            nodes: Vec::new(),
            tokens: Vec::new(),
            namespace: String::new(),
            member,
            projection_scopes: Vec::new(),
            work_bytes: Cell::new(0),
            xml_bytes: Cell::new(0),
            max_xml_bytes: limits.max_xml_working_bytes,
            max_report_bytes: limits.max_report_bytes,
        };
        // quick-xml drops a UTF-8 BOM without advancing its offset; keep original-byte coordinates.
        let bom = if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
            3
        } else {
            0
        };
        let mut reader = Reader::from_reader(&bytes[bom..]);
        reader.config_mut().check_comments = true;
        let mut stack = Vec::<usize>::new();
        let mut siblings = vec![BTreeMap::<(Rc<str>, &'a str), usize>::new()];
        let mut roots = 0;
        let mut text_bytes = 0usize;
        loop {
            let start = bom + reader.buffer_position() as usize;
            let event = reader.read_event().map_err(malformed)?;
            let end = bom + reader.buffer_position() as usize;
            match &event {
                Event::DocType(_) => return Err(DistributionInspectionError::DtdForbidden),
                Event::Decl(decl) => {
                    if let Some(enc) = decl.encoding() {
                        let enc = enc.map_err(malformed)?;
                        if !enc.eq_ignore_ascii_case("UTF-8") {
                            if bytes.starts_with(&[0xef, 0xbb, 0xbf])
                                || enc.eq_ignore_ascii_case("UTF-16")
                            {
                                return Err(DistributionInspectionError::EncodingConflict);
                            }
                            return Err(DistributionInspectionError::UnsupportedEncoding(
                                enc.into_owned(),
                            ));
                        }
                    }
                    if decl.version().map_err(malformed)?.as_ref() != "1.0" {
                        return Err(malformed("unsupported XML version"));
                    }
                }
                Event::Start(e) | Event::Empty(e) => {
                    if doc.nodes.len() >= limits.max_elements {
                        return Err(limit("elements"));
                    }
                    if stack.len() >= limits.max_depth {
                        return Err(limit("depth"));
                    }
                    let raw = original(bytes, e.name().as_ref())?;
                    let (prefix, local) = qname(raw)?;
                    if prefix == "xmlns" {
                        return Err(namespace("reserved xmlns element prefix"));
                    }
                    let mut attrs = Vec::new();
                    let mut declarations = BTreeMap::<String, Rc<str>>::new();
                    for (count, attr) in e.attributes().with_checks(true).enumerate() {
                        if count >= limits.max_attributes_per_element {
                            return Err(limit("attributes per element"));
                        }
                        let attr = attr.map_err(malformed)?;
                        let key = attr.key.as_ref().to_string();
                        qname(&key)?;
                        let value = attr
                            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                            .map_err(malformed)?;
                        let value = match value {
                            Cow::Borrowed(s) => Cow::Borrowed(original(bytes, s)?),
                            Cow::Owned(s) => {
                                doc.xml_charge(s.capacity())?;
                                Cow::Owned(s)
                            }
                        };
                        doc.xml_charge(key.capacity())?;
                        if !key.contains(':')
                            && (matches!(key.as_str(), "Id" | "RefId" | "ParameterType")
                                || key.ends_with("RefId"))
                            && value.len() > MAX_IDENTITY_BYTES
                        {
                            return Err(limit("identity/reference bytes"));
                        }
                        if !value.chars().all(xml_char) {
                            return Err(malformed("illegal attribute character"));
                        }
                        text_bytes = text_bytes
                            .checked_add(value.len())
                            .ok_or_else(|| limit("text bytes"))?;
                        if key == "xmlns" || key.starts_with("xmlns:") {
                            let binding = key.strip_prefix("xmlns:").unwrap_or("");
                            if binding == "xmlns"
                                || value == XMLNS_NS
                                || (binding == "xml") != (value == XML_NS)
                                || (!binding.is_empty() && value.is_empty())
                            {
                                return Err(namespace(
                                    "invalid reserved or empty namespace binding",
                                ));
                            }
                            doc.xml_charge(
                                binding.len() + value.len() + size_of::<(String, Rc<str>)>(),
                            )?;
                            declarations.insert(binding.to_string(), Rc::from(value.as_ref()));
                        } else {
                            attrs.push(Attribute {
                                raw: key,
                                expanded: String::new(),
                                value,
                            });
                        }
                    }
                    let lookup = |prefix: &str| -> Result<Rc<str>> {
                        if prefix == "xml" {
                            return Ok(XML_NS.into());
                        }
                        if let Some(uri) = declarations.get(prefix) {
                            return Ok(uri.clone());
                        }
                        for &ancestor in stack.iter().rev() {
                            if let Some(uri) = doc.nodes[ancestor].declarations.get(prefix) {
                                return Ok(uri.clone());
                            }
                        }
                        if prefix.is_empty() {
                            Ok(Rc::from(""))
                        } else {
                            Err(namespace("unbound namespace prefix"))
                        }
                    };
                    let ns = lookup(prefix)?;
                    let mut expanded_attrs = HashSet::new();
                    for attr in &mut attrs {
                        let (prefix, local) = qname(&attr.raw)?;
                        let uri = if prefix.is_empty() {
                            Rc::from("")
                        } else {
                            lookup(prefix)?
                        };
                        attr.expanded = format!("{{{uri}}}{local}");
                        doc.xml_charge(attr.expanded.capacity())?;
                        if !expanded_attrs.insert(attr.expanded.clone()) {
                            return Err(namespace("duplicate expanded attribute"));
                        }
                    }
                    let parent = stack.last().copied();
                    let sibling = siblings
                        .last_mut()
                        .expect("root sibling frame")
                        .entry((ns.clone(), local))
                        .or_default();
                    *sibling += 1;
                    if parent.is_none() {
                        roots += 1;
                    }
                    let id = doc.nodes.len();
                    let first_token = doc.tokens.len();
                    if doc.nodes.len() == doc.nodes.capacity() {
                        let more = doc.nodes.capacity().max(MIN_GROWTH);
                        doc.xml_charge(more * size_of::<Node<'_>>())?;
                        doc.nodes.reserve_exact(more);
                    }
                    doc.xml_charge(attrs.len() * size_of::<Attribute<'_>>())?;
                    doc.nodes.push(Node {
                        local,
                        raw,
                        ns,
                        attrs: attrs.into_boxed_slice(),
                        parent,
                        end_node: id + 1,
                        start,
                        end,
                        first_token,
                        last_token: first_token + 2,
                        sibling: *sibling,
                        declarations,
                    });
                    doc.token(Token::Open(id))?;
                    if matches!(event, Event::Empty(_)) {
                        doc.token(Token::Close(id))?;
                    } else {
                        stack.push(id);
                        siblings.push(BTreeMap::new());
                    }
                }
                Event::End(_) => {
                    siblings.pop();
                    let id = stack
                        .pop()
                        .ok_or_else(|| malformed("end tag outside root"))?;
                    doc.nodes[id].end = end;
                    doc.nodes[id].end_node = doc.nodes.len();
                    doc.token(Token::Close(id))?;
                    doc.nodes[id].last_token = doc.tokens.len();
                }
                Event::Text(_) | Event::CData(_) | Event::GeneralRef(_) => {
                    text_bytes = text_bytes
                        .checked_add(end - start)
                        .ok_or_else(|| limit("text bytes"))?;
                    doc.token(Token::Data(start, end))?;
                }
                Event::Eof => break,
                _ => {}
            }
            if text_bytes > limits.max_text_bytes {
                return Err(limit("text bytes"));
            }
        }
        if roots != 1 || !stack.is_empty() {
            return Err(malformed("incomplete XML document"));
        }
        // Also checks trailing content, entity legality and declaration placement through EOF.
        crate::xml::validate_complete_document("distribution XML", bytes)
            .map_err(|e| malformed(e.to_string()))?;
        let root = &doc.nodes[0];
        if root.local != "KNX" {
            return Err(DistributionInspectionError::UnrecognizedSource);
        }
        let version = root
            .ns
            .strip_prefix("http://knx.org/xml/project/")
            .unwrap_or("");
        if !matches!(
            version,
            "10" | "11" | "12" | "13" | "14" | "20" | "21" | "23"
        ) {
            return Err(DistributionInspectionError::UnsupportedSchema(
                root.ns.to_string(),
            ));
        }
        doc.namespace = root.ns.to_string();
        doc.xml_charge(doc.nodes.len() * size_of::<Option<usize>>())?;
        doc.projection_scopes.reserve_exact(doc.nodes.len());
        for id in 0..doc.nodes.len() {
            let n = &doc.nodes[id];
            let own_scope = (n.local == "ApplicationProgram"
                && doc.is_path(
                    id,
                    &[
                        "KNX",
                        "ManufacturerData",
                        "Manufacturer",
                        "ApplicationPrograms",
                        "ApplicationProgram",
                    ],
                ))
                || (matches!(n.local, "Catalog" | "Hardware")
                    && doc.is_path(id, &["KNX", "ManufacturerData", "Manufacturer", n.local]));
            let scope = if own_scope {
                Some(id)
            } else {
                n.parent
                    .and_then(|i| doc.projection_scopes[i])
                    .filter(|&scope| doc.role_node(id, scope))
            };
            doc.projection_scopes.push(scope);
        }
        Ok(doc)
    }
    pub fn path(&self, id: usize, indexed: bool) -> String {
        let mut chain = Vec::new();
        let mut next = Some(id);
        while let Some(i) = next {
            chain.push(i);
            next = self.nodes[i].parent;
        }
        let mut path = String::new();
        for i in chain.into_iter().rev() {
            let n = &self.nodes[i];
            path.push('/');
            path.push_str(n.local);
            if indexed {
                path.push_str(&format!("[{}]", n.sibling));
            }
        }
        path
    }
    pub fn charge(&self, bytes: usize) -> Result<()> {
        let total = self
            .work_bytes
            .get()
            .checked_add(bytes)
            .filter(|&n| n <= self.max_report_bytes)
            .ok_or_else(|| limit("serialized report bytes"))?;
        self.work_bytes.set(total);
        Ok(())
    }
    pub fn uncharge(&self, bytes: usize) {
        self.work_bytes.set(self.work_bytes.get() - bytes);
    }
    pub fn coordinate(&self, id: usize) -> Result<DistributionSourceRef> {
        if id >= self.nodes.len() {
            return Err(malformed("invalid element reference"));
        }
        Ok(DistributionSourceRef {
            member_ordinal: self.member,
            location: u32::try_from(id + 1).map_err(|_| limit("source locations"))?,
        })
    }
    fn xml_charge(&self, bytes: usize) -> Result<()> {
        let total = self
            .xml_bytes
            .get()
            .checked_add(bytes)
            .filter(|n| *n <= self.max_xml_bytes)
            .ok_or_else(|| limit("XML working bytes"))?;
        self.xml_bytes.set(total);
        Ok(())
    }
    fn token(&mut self, token: Token) -> Result<()> {
        if self.tokens.len() == self.tokens.capacity() {
            let more = self.tokens.capacity().max(2 * MIN_GROWTH);
            self.xml_charge(more * size_of::<Token>())?;
            self.tokens.reserve_exact(more);
        }
        self.tokens.push(token);
        Ok(())
    }
    pub fn push<T: Serialize>(&self, rows: &mut Vec<T>, row: T) -> Result<()> {
        self.charge(super::provenance::json_size(&row, self.max_report_bytes)?.saturating_add(1))?;
        rows.push(row);
        Ok(())
    }
    pub fn is_path(&self, mut id: usize, expected: &[&str]) -> bool {
        for (position, name) in expected.iter().rev().enumerate() {
            let n = &self.nodes[id];
            if n.ns.as_ref() != self.namespace || n.local != *name {
                return false;
            }
            match n.parent {
                Some(parent) => id = parent,
                None => return position + 1 == expected.len(),
            }
        }
        false
    }
    pub fn children(&self, id: usize) -> impl Iterator<Item = usize> + '_ {
        (id + 1..self.nodes[id].end_node).filter(move |&i| self.nodes[i].parent == Some(id))
    }
    pub fn manufacturer(&self, id: usize) -> Option<usize> {
        let mut next = Some(id);
        while let Some(i) = next {
            if self.is_path(i, &["KNX", "ManufacturerData", "Manufacturer"]) {
                return Some(i);
            }
            next = self.nodes[i].parent;
        }
        None
    }
    pub fn application(&self, id: usize) -> Option<usize> {
        let mut next = Some(id);
        while let Some(i) = next {
            if self.is_path(
                i,
                &[
                    "KNX",
                    "ManufacturerData",
                    "Manufacturer",
                    "ApplicationPrograms",
                    "ApplicationProgram",
                ],
            ) {
                return Some(i);
            }
            next = self.nodes[i].parent;
        }
        None
    }
    /// Parameter/ref/type identities are scoped by the closest ModuleDef, otherwise application.
    pub fn entity_scope(&self, id: usize) -> Option<usize> {
        let mut next = self.nodes[id].parent;
        while let Some(i) = next {
            if self.nodes[i].ns.as_ref() == self.namespace && self.nodes[i].local == "ModuleDef" {
                return Some(i);
            }
            next = self.nodes[i].parent;
        }
        self.application(id)
    }
    fn source_map(&self, report: &mut DistributionInspection) -> Result<()> {
        let before = self.work_bytes.get();
        let mut names = Vec::new();
        let mut intern = BTreeMap::new();
        let mut locations = Vec::new();
        for n in &self.nodes {
            let key = (n.ns.clone(), n.local, n.raw);
            let name = if let Some(&id) = intern.get(&key) {
                id
            } else {
                let id = names.len() as u32;
                self.push(
                    &mut names,
                    DistributionSourceName {
                        namespace: n.ns.to_string(),
                        local: n.local.into(),
                        raw: n.raw.into(),
                    },
                )?;
                intern.insert(key, id);
                id
            };
            self.push(
                &mut locations,
                DistributionLocation {
                    parent: n.parent.map(|p| p as u32),
                    name,
                    sibling: n.sibling as u32,
                    start: n.start as u64,
                    end: n.end as u64,
                    text_token: None,
                },
            )?;
        }
        let mut content_bytes = 0usize;
        let mut stack = Vec::new();
        for (ordinal, token) in self.tokens.iter().enumerate() {
            match *token {
                Token::Open(i) => stack.push(i),
                Token::Close(_) => {
                    stack.pop();
                }
                Token::Data(start, end) => {
                    let data = &self.bytes[start..end];
                    if data.iter().all(|b| b.is_ascii_whitespace()) {
                        continue;
                    }
                    let Some(&owner) = stack.last() else {
                        continue;
                    };
                    let source = DistributionSourceRef {
                        member_ordinal: self.member,
                        location: (locations.len() + 1) as u32,
                    };
                    let owner_location = &locations[owner];
                    let location = DistributionLocation {
                        parent: Some(owner as u32),
                        name: owner_location.name,
                        sibling: owner_location.sibling,
                        start: start as u64,
                        end: end as u64,
                        text_token: Some(ordinal as u64 + 1),
                    };
                    self.push(&mut locations, location)?;
                    let kind = if data.starts_with(b"<![CDATA[") {
                        "cdata"
                    } else if data.starts_with(b"&") {
                        "entityReference"
                    } else {
                        "text"
                    };
                    let content = DistributionContent {
                        source,
                        sha256: crate::sha256_hex(data),
                        kind: kind.into(),
                    };
                    let cost = super::provenance::json_size(&content, self.max_report_bytes)? + 1;
                    content_bytes = content_bytes
                        .checked_add(cost)
                        .ok_or_else(|| limit("serialized report bytes"))?;
                    self.push(&mut report.preserved_content, content)?;
                }
            }
        }
        // Replace the nested-table tally with the exact document representation once.
        self.work_bytes.set(before);
        self.charge(content_bytes)?;
        self.push(
            &mut report.documents,
            DistributionDocument {
                member_ordinal: self.member,
                namespace: self.namespace.clone(),
                sha256: self.sha256.clone(),
                size: self.bytes.len() as u64,
                names,
                locations,
            },
        )
    }
    fn role_node(&self, id: usize, scope: usize) -> bool {
        let n = &self.nodes[id];
        if n.ns.as_ref() != self.namespace {
            return false;
        }
        if id == scope {
            return true;
        }
        let parent = n.parent.map(|i| self.nodes[i].local).unwrap_or("");
        match self.nodes[scope].local {
            "Catalog" => matches!(
                (n.local, parent),
                ("CatalogSection", "Catalog" | "CatalogSection")
                    | ("CatalogItem", "Catalog" | "CatalogSection")
            ),
            "Hardware" => match n.local {
                "Hardware" => n.parent == Some(scope),
                "Products" | "Hardware2Programs" => parent == "Hardware" && n.parent != Some(scope),
                "Product" => parent == "Products",
                "Hardware2Program" => parent == "Hardware2Programs",
                "ApplicationProgramRef" => parent == "Hardware2Program",
                _ => false,
            },
            "ApplicationProgram" => {
                let mut next = Some(id);
                while let Some(i) = next {
                    if i == scope {
                        break;
                    }
                    if self.nodes[i].local == "Dynamic" {
                        return !matches!(
                            n.local,
                            "KNX"
                                | "ManufacturerData"
                                | "Manufacturer"
                                | "ApplicationPrograms"
                                | "ApplicationProgram"
                                | "Static"
                                | "ModuleDefs"
                                | "ModuleDef"
                                | "Catalog"
                                | "Hardware"
                                | "Baggages"
                        );
                    }
                    next = self.nodes[i].parent;
                }
                match n.local {
                    "Static" | "Dynamic" => matches!(parent, "ApplicationProgram" | "ModuleDef"),
                    "ModuleDefs" => matches!(parent, "Static" | "ApplicationProgram" | "ModuleDef"),
                    "ModuleDef" => parent == "ModuleDefs",
                    "ParameterTypes" | "Parameters" | "ParameterRefs" | "ComObjectTable"
                    | "ComObjectRefs" => parent == "Static",
                    "ParameterType" => parent == "ParameterTypes",
                    name if name.starts_with("Type") => parent == "ParameterType",
                    "Enumeration" => matches!(parent, "TypeRestriction" | "Restriction"),
                    "Restriction" => parent == "ParameterType",
                    "Union" => parent == "Parameters",
                    "Parameter" => matches!(parent, "Parameters" | "Union"),
                    "ParameterRef" => parent == "ParameterRefs",
                    "Memory" | "Property" => matches!(parent, "Parameter" | "Union"),
                    "ComObject" => parent == "ComObjectTable",
                    "ComObjectRef" => parent == "ComObjectRefs",
                    "Arguments" => parent == "ModuleDef",
                    "Argument" => parent == "Arguments",
                    _ => false,
                }
            }
            _ => false,
        }
    }
    pub fn projected(&self, id: usize, scope: usize) -> bool {
        self.projection_scopes[id] == Some(scope)
    }
    pub fn semantic_node(&self, id: usize) -> bool {
        self.projection_scopes[id].is_some()
    }
    pub fn declaration(&self, id: usize) -> bool {
        if !self.semantic_node(id) {
            return false;
        }
        let n = &self.nodes[id];
        if !matches!(
            n.local,
            "ApplicationProgram"
                | "Hardware"
                | "Product"
                | "Hardware2Program"
                | "CatalogItem"
                | "CatalogSection"
                | "Parameter"
                | "ParameterRef"
                | "ParameterType"
                | "ComObject"
                | "ComObjectRef"
                | "ModuleDef"
                | "Argument"
        ) {
            return false;
        }
        let mut next = n.parent;
        while let Some(i) = next {
            if self.nodes[i].local == "Dynamic" {
                return false;
            }
            next = self.nodes[i].parent;
        }
        true
    }
    /// Sanitized role wrapper v1: only admitted role paths and unqualified KNX attributes.
    pub fn view(
        &self,
        scope: usize,
        limits: &DistributionLimits,
    ) -> Result<(Vec<u8>, HashSet<usize>)> {
        let node = &self.nodes[scope];
        let manufacturer = self
            .manufacturer(scope)
            .and_then(|i| self.nodes[i].attr("RefId"))
            .unwrap_or_default();
        let is_app = node.local == "ApplicationProgram";
        let mut out = format!(
            "<KNX xmlns=\"{}\"><ManufacturerData><Manufacturer RefId=\"{}\">{}",
            self.namespace,
            escape_attribute(&manufacturer),
            if is_app { "<ApplicationPrograms>" } else { "" }
        );
        let mut included = HashSet::new();
        let mut active = Vec::new();
        for token in &self.tokens[node.first_token..node.last_token] {
            match *token {
                Token::Open(i) => {
                    let n = &self.nodes[i];
                    let include =
                        active.last().copied().unwrap_or(true) && self.projected(i, scope);
                    active.push(include);
                    if include {
                        included.insert(i);
                        out.push('<');
                        out.push_str(n.local);
                        for a in &n.attrs {
                            if !a.raw.contains(':') {
                                out.push(' ');
                                out.push_str(&a.raw);
                                out.push_str("=\"");
                                out.push_str(&escape_attribute(a.value.as_ref()));
                                out.push('"');
                            }
                        }
                        out.push('>');
                    }
                }
                Token::Close(i) => {
                    if active.pop() == Some(true) {
                        out.push_str("</");
                        out.push_str(self.nodes[i].local);
                        out.push('>');
                    }
                }
                Token::Data(start, end) => {
                    if active.last() == Some(&true) {
                        out.push_str(
                            std::str::from_utf8(&self.bytes[start..end]).map_err(malformed)?,
                        );
                    }
                }
            }
            if out.len() > limits.max_derived_bytes {
                return Err(limit("derived view bytes"));
            }
        }
        if is_app {
            out.push_str("</ApplicationPrograms>");
        }
        out.push_str("</Manufacturer></ManufacturerData></KNX>");
        if out.len() > limits.max_derived_bytes {
            return Err(limit("derived view bytes"));
        }
        Ok((out.into_bytes(), included))
    }
}

pub(super) fn inspect(
    bytes: &[u8],
    member: Option<u32>,
    limits: &DistributionLimits,
    report: &mut DistributionInspection,
) -> Result<usize> {
    let doc = Document::parse(bytes, member, limits)?;
    super::readers::inspect(&doc, limits, report)?;
    doc.source_map(report)?;
    super::readers::output_limit(report, limits)?;
    Ok(doc.work_bytes.get())
}

#[cfg(test)]
mod tests {
    use super::escape_attribute;

    #[test]
    fn derived_view_escaping_survives_attribute_value_normalization() {
        for value in ["a\tb\nc\rd", "<&\"'>", "plain", ""] {
            let xml = format!("<x v=\"{}\"/>", escape_attribute(value));
            let mut reader = quick_xml::Reader::from_str(&xml);
            let quick_xml::events::Event::Empty(e) = reader.read_event().unwrap() else {
                panic!("{xml}");
            };
            let attr = e.attributes().next().unwrap().unwrap();
            let reparsed = attr
                .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                .unwrap();
            assert_eq!(reparsed, value, "{xml}");
        }
    }
}
