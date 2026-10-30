//! Blanks secret attribute values and secret element text in one XML document or fragment.
//!
//! The rewrite splices: every byte outside a changed start tag or a blanked
//! text run is copied from the input unchanged. A start tag with a secret
//! attribute is rebuilt from its own raw attribute bytes, with the secret
//! value emptied (one space between attributes, `"` quotes unless the raw
//! value contains one). A document that is not well-formed, declares a
//! DTD (entities could carry a value outside any attribute) or is not
//! UTF-8-compatible is refused, and the caller withholds it.

use quick_xml::events::{BytesStart, Event};
use quick_xml::{Reader, XmlVersion};

use super::{is_free_access_key, is_secret_name, is_secret_value, Findings};

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

struct SecretText {
    depth: usize,
    name: Vec<u8>,
    value: Vec<u8>,
    saw_nonempty: bool,
    tentative_observation: Option<(bool, u64)>,
}

fn observe_secret_text(secret: &mut SecretText, findings: &mut Findings) {
    if secret.saw_nonempty {
        return;
    }
    secret.saw_nonempty = true;
    if secret.name.as_slice() == b"BCUKey" {
        secret.tentative_observation = Some(findings.begin_tentative_credential_observation());
    } else {
        findings.observe_credential_material();
    }
}

/// The redacted bytes, or `None` when nothing secret was found.
pub(super) fn redact(
    input: &[u8],
    base: &Base<'_>,
    findings: &mut Findings,
) -> Result<Option<Vec<u8>>, &'static str> {
    if base.text.len() > super::budget::XML_PATH {
        return Err("XML path budget exceeded");
    }
    findings.budget.output(input.len().saturating_mul(2))?;
    let mut path = base.text.to_string();
    let mut reader = Reader::from_reader(input);
    let mut out: Vec<u8> = Vec::new();
    let mut copied = 0usize;
    let mut changed = false;
    // Element name, whether it is secret, whether its text was counted.
    let mut stack: Vec<(Vec<u8>, bool, bool)> = Vec::new();
    let mut secrets: Vec<SecretText> = Vec::new();
    loop {
        let start = position(&reader)?;
        let event = reader.read_event().map_err(|_| "not well-formed XML")?;
        let end = position(&reader)?;
        findings
            .budget
            .work(end.saturating_sub(start).saturating_add(1))?;
        match event {
            Event::Start(ref tag) | Event::Empty(ref tag) => {
                if stack.len() >= super::budget::XML_DEPTH {
                    return Err("XML depth budget exceeded");
                }
                let local_name = tag.local_name();
                let local = local_name.as_ref().as_bytes();
                let old_path = path.len();
                if !(stack.is_empty() && base.root_in_base) {
                    if path.len().saturating_add(1).saturating_add(local.len())
                        > super::budget::XML_PATH
                    {
                        return Err("XML path budget exceeded");
                    }
                    path.push('/');
                    path.push_str(std::str::from_utf8(local).map_err(|_| "invalid XML name")?);
                }
                let empty = matches!(event, Event::Empty(_));
                if let Some(rebuilt) = redact_tag(tag, empty, local, &path, findings)? {
                    out.extend_from_slice(&input[copied..start]);
                    out.extend_from_slice(&rebuilt);
                    copied = end;
                    changed = true;
                }
                if matches!(event, Event::Start(_)) {
                    let secret = is_secret_name(local);
                    stack.push((local.to_vec(), secret, false));
                    if secret {
                        secrets.push(SecretText {
                            depth: stack.len(),
                            name: local.to_vec(),
                            value: Vec::new(),
                            saw_nonempty: false,
                            tentative_observation: None,
                        });
                    }
                } else {
                    path.truncate(old_path);
                }
            }
            Event::End(_) => {
                if secrets
                    .last()
                    .is_some_and(|secret| secret.depth == stack.len())
                {
                    let secret = secrets.pop().ok_or("missing secret element")?;
                    if let Some(prior) = secret.tentative_observation {
                        findings.resolve_tentative_credential_observation(
                            prior,
                            is_free_access_key(&secret.name, &secret.value),
                        );
                    }
                    findings.remember(&secret.value);
                }
                let (name, ..) = stack.pop().ok_or("unmatched XML end tag")?;
                if !(stack.is_empty() && base.root_in_base) {
                    path.truncate(path.len().saturating_sub(name.len() + 1));
                }
            }
            Event::Text(_) | Event::CData(_) | Event::GeneralRef(_) if !secrets.is_empty() => {
                if end.saturating_sub(start) > 1024 * 1024 {
                    return Err("secret element exceeds decoded value budget");
                }
                if matches!(event, Event::GeneralRef(_)) {
                    for secret in &mut secrets {
                        observe_secret_text(secret, findings);
                    }
                }
                let decoded = match &event {
                    Event::Text(text) => text.xml_content(XmlVersion::Implicit1_0).into_owned(),
                    Event::CData(text) => text.xml_content(XmlVersion::Implicit1_0).into_owned(),
                    Event::GeneralRef(reference) => {
                        if let Some(ch) = reference
                            .resolve_char_ref()
                            .map_err(|_| "invalid character reference")?
                        {
                            ch.to_string()
                        } else {
                            quick_xml::escape::resolve_predefined_entity(reference.as_ref())
                                .ok_or("unresolved XML reference")?
                                .to_string()
                        }
                    }
                    _ => unreachable!(),
                };
                findings
                    .budget
                    .work(decoded.len().saturating_mul(secrets.len()))?;
                for secret in &mut secrets {
                    if !decoded.is_empty() {
                        observe_secret_text(secret, findings);
                    }
                    if secret.value.len().saturating_add(decoded.len()) > 1024 * 1024 {
                        return Err("secret element exceeds decoded value budget");
                    }
                    secret.value.extend_from_slice(decoded.as_bytes());
                }
                let raw = &input[start..end];
                findings.remember(raw);
                out.extend_from_slice(&input[copied..start]);
                copied = end;
                changed = true;
                // Counted once per secret element, however its text is split.
                if let Some(index) = secrets.last().map(|secret| secret.depth - 1) {
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
    findings.budget.check()?;
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
    // Avoid the parser's duplicate-name policy doing unmetered pairwise work.
    for attribute in tag.attributes().with_checks(false) {
        if attributes.len() >= 4096 {
            return Err("XML attribute count budget exceeded");
        }
        let attribute = attribute.map_err(|_| "malformed XML attribute")?;
        let key_bytes = attribute.key.as_ref().as_bytes();
        if key_bytes.len() > super::budget::XML_PATH {
            return Err("XML attribute name budget exceeded");
        }
        for (previous, _) in &attributes {
            findings.budget.work(1)?;
            if previous.len() == key_bytes.len() {
                findings.budget.work(key_bytes.len())?;
                if previous.as_slice() == key_bytes {
                    return Err("duplicate XML attribute");
                }
            }
        }
        let key = key_bytes.to_vec();
        let local = attribute.key.local_name().as_ref().as_bytes().to_vec();
        let raw: &[u8] = attribute.value.as_bytes();
        let mut value = raw.to_vec();
        let secret_name = is_secret_name(&local);
        let secret_value = secret_name && is_secret_value(&local, raw);
        let tentative_observation = if secret_value && local.as_slice() == b"BCUKey" {
            Some(findings.begin_tentative_credential_observation())
        } else {
            if secret_name {
                findings.observe_credential(&local, raw);
            }
            None
        };
        if secret_value {
            findings.remember(raw);
            let plain = attribute
                .normalized_value(XmlVersion::Implicit1_0)
                .map_err(|_| "undecodable secret attribute")?;
            if let Some(prior) = tentative_observation {
                findings.resolve_tentative_credential_observation(
                    prior,
                    is_free_access_key(&local, plain.as_bytes()),
                );
            }
            findings.remember(plain.as_bytes());
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
    // ASCII-only case folding, without an owned lowercase copy of untrusted text.
    CREDENTIAL_PARTS.iter().any(|part| {
        name.as_bytes()
            .windows(part.len())
            .any(|window| window.eq_ignore_ascii_case(part.as_bytes()))
    })
}

/// `name=value` parts separated by `;`: credential values are emptied,
/// every other part is kept verbatim. A string that is not all
/// `name=value` parts but mentions a credential is emptied whole.
pub(super) fn redact_connection_string(
    text: &str,
    location: &str,
    findings: &mut Findings,
) -> Option<String> {
    // Reserve before even scanning separators or credential names. 64 work units
    // per byte cover the two linear passes, trims/splits, and all ASCII keyword
    // comparisons (keyword lengths sum to 35). Iterators retain O(1) borrowed
    // slices, not one slot per delimiter; only changed output is ever owned.
    if findings
        .budget
        .work(text.len().saturating_mul(64).saturating_add(1))
        .is_err()
    {
        return None; // Sticky failure: every enclosing publication checks the budget.
    }
    let structured = text
        .split(';')
        .filter(|part| !part.trim().is_empty())
        .all(|part| part.contains('='));
    if !structured {
        let credential = is_credential_part(text);
        if credential && !text.is_empty() {
            findings.observe_credential_material();
        }
        if credential && !text.trim().is_empty() {
            findings.remember(text.as_bytes());
            findings.redact(location.to_string(), "BusAccess/@Parameter".into());
            return Some(String::new());
        }
        return None;
    }
    // Lazily copy the prefix on the first redaction. Output never exceeds input.
    let mut rebuilt: Option<String> = None;
    let mut offset = 0;
    for part in text.split(';') {
        let credential = part
            .split_once('=')
            .filter(|(name, _)| is_credential_part(name));
        if credential.is_some_and(|(_, value)| !value.is_empty()) {
            findings.observe_credential_material();
        }
        let secret = credential.filter(|(_, value)| !value.trim().is_empty());
        if let Some((name, value)) = secret {
            if rebuilt.is_none() {
                if findings.budget.output(text.len()).is_err() {
                    return None;
                }
                let mut output = String::with_capacity(text.len());
                output.push_str(&text[..offset]);
                rebuilt = Some(output);
            }
            findings.remember(value.as_bytes());
            findings.redact(
                location.to_string(),
                format!("BusAccess/@Parameter:{}", name.trim()),
            );
            // Do not continue materializing output after any sticky refusal.
            if findings.budget.check().is_err() {
                return None;
            }
            let output = rebuilt.as_mut()?;
            output.push_str(name);
            output.push('=');
        } else if let Some(output) = rebuilt.as_mut() {
            output.push_str(part);
        }
        offset += part.len();
        if offset < text.len() {
            if let Some(output) = rebuilt.as_mut() {
                output.push(';');
            }
            offset += 1;
        }
    }
    rebuilt
}
