//! Proves one parsed ApplicationProgram caption-value span without normalizing source bytes.
use quick_xml::{events::Event, Reader};
use std::{io, ops::Range};
#[cfg(test)]
#[path = "settlement_caption_review.rs"]
mod review;

fn invalid() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "source is not one literal caption-value change",
    )
}

// Slice-reader attributes borrow the ORIGINAL input. Never search for a value or tag substring.
fn span(bytes: &[u8], program: &str) -> io::Result<Range<usize>> {
    if bytes.len() > 16 * 1024 * 1024 {
        return Err(invalid());
    }
    let mut reader = Reader::from_reader(bytes);
    let mut path = Vec::new();
    let mut found = None;
    let mut events = 0usize;
    loop {
        events += 1;
        if events > 250_000 || path.len() > 64 {
            return Err(invalid());
        }
        let event = reader.read_event().map_err(io::Error::other)?;
        let empty = matches!(event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let name = e.name().as_ref().to_owned();
                if name == "ApplicationProgram"
                    && path
                        == [
                            "KNX",
                            "ManufacturerData",
                            "Manufacturer",
                            "ApplicationPrograms",
                        ]
                {
                    let mut id = None;
                    let mut value = None;
                    for attribute in e.attributes() {
                        let attribute = attribute.map_err(io::Error::other)?;
                        match attribute.key.as_ref() {
                            "Id" => {
                                id = Some(
                                    attribute
                                        .normalized_value(quick_xml::XmlVersion::Implicit1_0)
                                        .map_err(io::Error::other)?
                                        .into_owned(),
                                )
                            }
                            "Name" => {
                                let literal: &str = attribute.value.as_ref();
                                let start = (literal.as_ptr() as usize)
                                    .checked_sub(bytes.as_ptr() as usize)
                                    .ok_or_else(invalid)?;
                                let end = start.checked_add(literal.len()).ok_or_else(invalid)?;
                                if bytes.get(start..end) != Some(literal.as_bytes()) {
                                    return Err(invalid());
                                }
                                value = Some(start..end);
                            }
                            _ => {}
                        }
                    }
                    if id.as_deref() == Some(program) {
                        if found.is_some() {
                            return Err(invalid());
                        }
                        let value = value.ok_or_else(invalid)?;
                        let literal = &bytes[value.clone()];
                        if literal.is_empty()
                            || literal.len() > 1024
                            || !literal
                                .iter()
                                .all(|b| (0x20..=0x7e).contains(b) && !b"&<>\"'".contains(b))
                            || value.start == 0
                            || bytes.get(value.start - 1) != Some(&b'"')
                            || bytes.get(value.end) != Some(&b'"')
                        {
                            return Err(invalid());
                        }
                        found = Some(value);
                    }
                }
                if !empty {
                    path.push(name);
                }
            }
            Event::End(_) => {
                path.pop().ok_or_else(invalid)?;
            }
            Event::DocType(_) => return Err(invalid()),
            Event::Eof => break,
            _ => {}
        }
    }
    found.ok_or_else(invalid)
}

/// Value-only proof after the caller's full-document XML/namespace and measured-owner admission.
/// This proves byte consistency, not execution, authenticity or a start/recovery capability.
pub fn caption_value_only(old: &[u8], new: &[u8], program: &str) -> io::Result<()> {
    let a = span(old, program)?;
    let b = span(new, program)?;
    if old[a.clone()] == new[b.clone()]
        || old[..a.start] != new[..b.start]
        || old[a.end..] != new[b.end..]
    {
        return Err(invalid());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn xml(value: &str) -> Vec<u8> {
        format!("<KNX><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram Id=\"A\" Name=\"{value}\"/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>").into_bytes()
    }
    #[test]
    fn selected_literal_caption_and_exact_limit() {
        for value in ["B".to_owned(), "x".repeat(1024)] {
            caption_value_only(&xml("A"), &xml(&value), "A").unwrap();
        }
    }
    #[test]
    fn caption_lexical_attacks_and_unchanged_value_refuse() {
        for value in [
            "",
            "A",
            "&amp;",
            "'",
            ">",
            "<",
            "é",
            "\n",
            &"x".repeat(1025),
        ] {
            assert!(
                caption_value_only(&xml("A"), &xml(value), "A").is_err(),
                "{value:?}"
            );
        }
    }
    #[test]
    fn comments_other_owners_quotes_and_other_bytes_are_not_spans() {
        let old = xml("A");
        for new in [
            String::from_utf8(xml("B"))
                .unwrap()
                .replace("Name=\"B\"", "Name='B'"),
            String::from_utf8(xml("B"))
                .unwrap()
                .replace("Id=\"A\"", "Id=\"C\""),
            String::from_utf8(xml("B"))
                .unwrap()
                .replace(" Name=", "  Name="),
            String::from_utf8(old.clone()).unwrap().replace(
                "<ApplicationProgram ",
                "<!-- Name=\"B\" --><ApplicationProgram ",
            ),
        ] {
            assert!(caption_value_only(&old, new.as_bytes(), "A").is_err());
        }
    }
}
