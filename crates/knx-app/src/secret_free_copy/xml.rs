//! Blanks secret attribute values and secret element text in one XML document or fragment.
//!
//! The rewrite splices: every byte outside a changed start tag or a blanked
//! text run is copied from the input unchanged. A start tag with a secret
//! attribute is rebuilt from its own raw attribute bytes, with the secret
//! value emptied (one space between attributes, `"` quotes unless the raw
//! value contains one). A document that is not well-formed, declares a
//! DTD (entities could carry a value outside any attribute) or is not
//! UTF-8-compatible is refused, and the caller withholds it.

use std::borrow::Cow;

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use super::{is_secret_name, is_secret_value, Findings};

/// What a payload looks like, by its first bytes only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Content {
    Xml,
    Zip,
    /// UTF-16 or UTF-32 text: names would not match byte-wise, so it
    /// cannot be inspected here.
    WideText,
    Other,
}

pub(super) fn sniff(bytes: &[u8]) -> Content {
    if bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"PK\x05\x06") {
        return Content::Zip;
    }
    // Byte-order marks, or a `<` encoded in UTF-16/32 without one. Plain
    // binary that merely starts with zero bytes stays `Other`.
    if [
        &[0xFF, 0xFE][..],
        &[0xFE, 0xFF],
        &[0x00, 0x00, 0xFE, 0xFF],
        b"<\0",
        b"\0<",
        b"\0\0\0<",
    ]
    .iter()
    .any(|prefix| bytes.starts_with(prefix))
    {
        return Content::WideText;
    }
    let body = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match body.iter().find(|b| !b.is_ascii_whitespace()) {
        Some(b'<') => Content::Xml,
        _ => Content::Other,
    }
}

/// Where an element sits: `base` names the member or row; with
/// `root_in_base` the fragment's root element is already part of `base`
/// (a retained element's XPath ends in it).
pub(super) struct Base<'a> {
    pub text: &'a str,
    pub root_in_base: bool,
}

/// The redacted bytes, or `None` when nothing secret was found.
pub(super) fn redact(
    input: &[u8],
    base: &Base<'_>,
    findings: &mut Findings,
) -> Result<Option<Vec<u8>>, &'static str> {
    let mut reader = Reader::from_reader(input);
    let mut out: Vec<u8> = Vec::new();
    let mut copied = 0usize;
    let mut changed = false;
    // Element name, whether it is secret, whether its text was counted.
    let mut stack: Vec<(Vec<u8>, bool, bool)> = Vec::new();
    loop {
        let start = position(&reader)?;
        let event = reader.read_event().map_err(|_| "not well-formed XML")?;
        let end = position(&reader)?;
        match event {
            Event::Start(ref tag) | Event::Empty(ref tag) => {
                let local = tag.local_name().as_ref().as_bytes().to_vec();
                let path = element_path(base, &stack, &local);
                let empty = matches!(event, Event::Empty(_));
                if let Some(rebuilt) = redact_tag(tag, empty, &local, &path, findings)? {
                    out.extend_from_slice(&input[copied..start]);
                    out.extend_from_slice(&rebuilt);
                    copied = end;
                    changed = true;
                }
                if matches!(event, Event::Start(_)) {
                    let secret = is_secret_name(&local);
                    stack.push((local, secret, false));
                }
            }
            Event::End(_) => {
                stack.pop().ok_or("unmatched XML end tag")?;
            }
            Event::Text(_) | Event::CData(_) | Event::GeneralRef(_)
                if stack.iter().any(|(_, secret, _)| *secret) =>
            {
                let raw = &input[start..end];
                if raw.iter().all(u8::is_ascii_whitespace) {
                    continue;
                }
                findings.remember(raw);
                out.extend_from_slice(&input[copied..start]);
                copied = end;
                changed = true;
                // Counted once per secret element, however its text is split.
                if let Some(index) = stack.iter().rposition(|(_, secret, _)| *secret) {
                    if !stack[index].2 {
                        let path = element_path(base, &stack[..index], &stack[index].0);
                        let kind = String::from_utf8_lossy(&stack[index].0).into_owned();
                        findings.redact(path, kind);
                        stack[index].2 = true;
                    }
                }
            }
            Event::DocType(_) => {
                return Err("XML with a document type declaration is not inspected")
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return Err("XML ends inside an element");
    }
    if !changed {
        return Ok(None);
    }
    out.extend_from_slice(&input[copied..]);
    Ok(Some(out))
}

fn position(reader: &Reader<&[u8]>) -> Result<usize, &'static str> {
    usize::try_from(reader.buffer_position()).map_err(|_| "XML position out of range")
}

/// `base` plus `/name` per open element and `name` itself.
fn element_path(base: &Base<'_>, stack: &[(Vec<u8>, bool, bool)], name: &[u8]) -> String {
    let mut path = base.text.to_string();
    let names = stack.iter().map(|(n, ..)| n.as_slice()).chain([name]);
    for (depth, element) in names.enumerate() {
        if depth == 0 && base.root_in_base {
            continue;
        }
        path.push('/');
        path.push_str(&String::from_utf8_lossy(element));
    }
    path
}

/// The rebuilt start tag when one of its attributes was secret.
fn redact_tag(
    tag: &BytesStart<'_>,
    empty: bool,
    element: &[u8],
    path: &str,
    findings: &mut Findings,
) -> Result<Option<Vec<u8>>, &'static str> {
    let mut attributes: Vec<(Vec<u8>, Vec<u8>)> = Vec::new();
    let mut changed = false;
    for attribute in tag.attributes() {
        let attribute = attribute.map_err(|_| "malformed or duplicate XML attribute")?;
        let key = attribute.key.as_ref().as_bytes().to_vec();
        let local = attribute.key.local_name().as_ref().as_bytes().to_vec();
        let raw: &[u8] = attribute.value.as_bytes();
        let mut value = raw.to_vec();
        if is_secret_name(&local) && is_secret_value(&local, raw) {
            findings.remember(raw);
            if let Ok(plain) = attribute.normalized_value(XmlVersion::Implicit1_0) {
                findings.remember(plain.as_bytes());
            }
            let name = String::from_utf8_lossy(&local).into_owned();
            findings.redact(format!("{path}/@{name}"), name);
            value.clear();
            changed = true;
        } else if element == b"BusAccess" && local == b"Parameter" {
            let plain = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|_| "malformed XML attribute value")?;
            if let Some(redacted) =
                redact_connection_string(&plain, &format!("{path}/@Parameter"), findings)
            {
                value = quick_xml::escape::escape(redacted)
                    .into_owned()
                    .into_bytes();
                changed = true;
            }
        }
        attributes.push((key, value));
    }
    if !changed {
        return Ok(None);
    }
    let mut rebuilt = b"<".to_vec();
    rebuilt.extend_from_slice(tag.name().as_ref().as_bytes());
    for (key, value) in attributes {
        let quote = if value.contains(&b'"') { b'\'' } else { b'"' };
        rebuilt.push(b' ');
        rebuilt.extend_from_slice(&key);
        rebuilt.push(b'=');
        rebuilt.push(quote);
        rebuilt.extend_from_slice(&value);
        rebuilt.push(quote);
    }
    rebuilt.extend_from_slice(if empty { b"/>" } else { b">" });
    Ok(Some(rebuilt))
}

/// Part names of a `BusAccess/@Parameter` connection string whose values
/// are credentials (KNX IP Secure tunnelling passwords, authentication
/// codes, keys). Matched case-insensitively as substrings.
const CREDENTIAL_PARTS: &[&str] = &["password", "key", "authenticationcode", "secret"];

fn is_credential_part(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    CREDENTIAL_PARTS.iter().any(|part| lower.contains(part))
}

/// `name=value` parts separated by `;`: credential values are emptied,
/// every other part is kept verbatim. A string that is not all
/// `name=value` parts but mentions a credential is emptied whole.
pub(super) fn redact_connection_string(
    text: &str,
    location: &str,
    findings: &mut Findings,
) -> Option<String> {
    let parts: Vec<&str> = text.split(';').collect();
    let structured = parts
        .iter()
        .filter(|part| !part.trim().is_empty())
        .all(|part| part.contains('='));
    if !structured {
        if is_credential_part(text) && !text.trim().is_empty() {
            findings.remember(text.as_bytes());
            findings.redact(location.to_string(), "BusAccess/@Parameter".into());
            return Some(String::new());
        }
        return None;
    }
    let mut changed = false;
    let rebuilt: Vec<Cow<'_, str>> = parts
        .iter()
        .map(|part| match part.split_once('=') {
            Some((name, value)) if is_credential_part(name) && !value.trim().is_empty() => {
                findings.remember(value.as_bytes());
                findings.redact(
                    location.to_string(),
                    format!("BusAccess/@Parameter:{}", name.trim()),
                );
                changed = true;
                Cow::Owned(format!("{name}="))
            }
            _ => Cow::Borrowed(*part),
        })
        .collect();
    changed.then(|| rebuilt.join(";"))
}
