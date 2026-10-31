//! Prove Dynamic expanded names from retained source, including pre-namespace database rows.
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::NsReader;
use rusqlite::Connection;

use super::Unproved;

#[derive(Clone, Copy, Default)]
struct Context {
    clean: bool,
    program: bool,
    dynamic: bool,
    module: bool,
}

/// Local names and NULL audit attributes cannot establish historical namespace provenance.
/// Resolve the original document without changing its rows or the read-only evaluator.
pub(super) fn prove<'a>(
    conn: &Connection,
    program_id: &str,
    kinds: impl Iterator<Item = &'a str>,
) -> Result<(), Unproved> {
    let unresolved = || Unproved;
    let (sha, bytes): (String, Vec<u8>) = conn
        .query_row(
            "SELECT s.sha256, s.bytes FROM application_program a
             JOIN source_file s ON s.sha256 = a.source_sha256 WHERE a.id = ?1",
            [program_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| unresolved())?;
    if crate::blob::sha256_hex(&bytes) != sha {
        return Err(unresolved());
    }
    let mut reader = NsReader::from_reader(bytes.as_slice());
    let mut namespace = None;
    let mut stack: Vec<Context> = Vec::new();
    let mut programs = 0;
    let mut source_kinds = Vec::new();
    loop {
        let (resolved, event) = reader.read_resolved_event().map_err(|_| unresolved())?;
        let is_start = matches!(event, Event::Start(_));
        match event {
            Event::Start(element) | Event::Empty(element) => {
                let local = element.local_name();
                if stack.is_empty() {
                    if namespace.is_some() || local.as_ref() != "KNX" {
                        return Err(unresolved());
                    }
                    let ResolveResult::Bound(uri) = &resolved else {
                        return Err(unresolved());
                    };
                    // The admitted product-file schemes; no URI or prefix guessing.
                    if !matches!(
                        uri.as_ref(),
                        "http://knx.org/xml/project/10"
                            | "http://knx.org/xml/project/11"
                            | "http://knx.org/xml/project/12"
                            | "http://knx.org/xml/project/13"
                            | "http://knx.org/xml/project/14"
                            | "http://knx.org/xml/project/20"
                            | "http://knx.org/xml/project/21"
                            | "http://knx.org/xml/project/23"
                    ) {
                        return Err(unresolved());
                    }
                    namespace = Some(uri.as_ref().to_owned());
                }
                let parent = stack.last().copied().unwrap_or(Context {
                    clean: true,
                    ..Context::default()
                });
                let clean = parent.clean
                    && matches!(&resolved, ResolveResult::Bound(uri) if Some(uri.as_ref()) == namespace.as_deref());
                let selected = local.as_ref() == "ApplicationProgram"
                    && element
                        .try_get_attribute("Id")
                        .map_err(|_| unresolved())?
                        .map(|a| a.normalized_value(quick_xml::XmlVersion::Implicit1_0))
                        .transpose()
                        .map_err(|_| unresolved())?
                        .is_some_and(|id| id == program_id);
                if selected {
                    programs += 1;
                    if !clean || parent.program {
                        return Err(unresolved());
                    }
                }
                let program = selected || parent.program;
                let module = parent.module || (program && local.as_ref() == "ModuleDef");
                let dynamic = parent.dynamic || (program && !module && local.as_ref() == "Dynamic");
                if dynamic {
                    if !clean {
                        return Err(unresolved());
                    }
                    source_kinds.push(local.as_ref().to_owned());
                }
                if is_start {
                    stack.push(Context {
                        clean,
                        program,
                        dynamic,
                        module,
                    });
                }
            }
            Event::End(_) => {
                stack.pop().ok_or_else(unresolved)?;
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if programs != 1 || !stack.is_empty() || !source_kinds.iter().map(String::as_str).eq(kinds) {
        return Err(unresolved());
    }
    Ok(())
}
