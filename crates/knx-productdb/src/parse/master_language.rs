//! Captures byte-derived evidence for unconsumed master Languages attributes.
//!
//! This deliberately owns no manufacturer/DPT declarations
//! and performs no writes. Exact ancestor paths and resolved namespaces decide
//! which unqualified attributes are consumed; every other field stays named.

use quick_xml::events::Event;
use quick_xml::name::ResolveResult;
use quick_xml::{NsReader, XmlVersion};

use super::scheme_evidence::MAX_EVIDENCE_DEPTH;
use crate::report::{UnknownCollector, UnknownConstruct};
use crate::ProductDbError;

// Dedicated master-language scanning and retained-source classification keep
// their original ceilings independently of package scheme-evidence policy.
pub(crate) const MAX_MASTER_LANGUAGE_BYTES: usize = 64 * 1024 * 1024;
const MAX_MASTER_LANGUAGE_ITEMS: usize = 262_144;

const LANGUAGE_PATH: &[&str] = &[
    "KNX",
    "MasterData",
    "Languages",
    "Language",
    "TranslationUnit",
    "TranslationElement",
    "Translation",
];

struct Frame {
    local: String,
    rendered: String,
    in_namespace: bool,
}

fn error(cause: impl Into<String>) -> ProductDbError {
    ProductDbError::Xml {
        source_path: "knx_master.xml".into(),
        cause: cause.into(),
    }
}

fn charge(total: &mut usize, bytes: usize) -> Result<(), ProductDbError> {
    *total = total
        .checked_add(bytes)
        .filter(|size| *size <= MAX_MASTER_LANGUAGE_BYTES)
        .ok_or_else(|| error("master-language evidence byte budget exceeded"))?;
    Ok(())
}

fn supported_namespace(namespace: &str) -> bool {
    // Empty is the historical dedicated master API's namespace-free fixture
    // form, not a newly admitted product-package scheme. Others are exact.
    matches!(
        namespace,
        "" | "http://knx.org/xml/project/11"
            | "http://knx.org/xml/project/12"
            | "http://knx.org/xml/project/13"
            | "http://knx.org/xml/project/14"
            | "http://knx.org/xml/project/20"
            | "http://knx.org/xml/project/21"
            | "http://knx.org/xml/project/23"
    )
}

/// Shared by current master ingest and byte-only re-derivation. Nothing is
/// published until the complete, bounded pass succeeds. This is not a typed
/// namespace-admission validator and never changes package scheme admission.
pub(crate) fn master_language_unknowns(
    bytes: &[u8],
) -> Result<Vec<UnknownConstruct>, ProductDbError> {
    if bytes.len() > MAX_MASTER_LANGUAGE_BYTES {
        return Err(error("master-language input byte budget exceeded"));
    }
    let mut reader = NsReader::from_reader(bytes);
    let mut buffer = Vec::new();
    let mut path = Vec::<Frame>::new();
    let mut root_namespace: Option<String> = None;
    let mut collector = UnknownCollector::default();
    let mut items = 0usize;
    let mut evidence_bytes = 0usize;
    let mut skipped_depth = 0usize;
    loop {
        buffer.clear();
        let (namespace, event) = reader
            .read_resolved_event_into(&mut buffer)
            .map_err(|cause| error(cause.to_string()))?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                // This pass owns Languages only. Retain depth, not paths or
                // evidence budgets, for opaque sections outside that branch.
                if skipped_depth > 0 {
                    if matches!(event, Event::Start(_)) {
                        skipped_depth += 1;
                    }
                    continue;
                }
                let local = element.local_name().as_ref().to_string();
                if (path.len() == 1 && local != "MasterData")
                    || (path.len() == 2 && local != "Languages")
                {
                    if matches!(event, Event::Start(_)) {
                        skipped_depth = 1;
                    }
                    continue;
                }
                let namespace_uri = match &namespace {
                    ResolveResult::Bound(namespace) => namespace.as_ref(),
                    ResolveResult::Unbound => "",
                    ResolveResult::Unknown(_) => {
                        return Err(error("master-language element namespace unresolved"))
                    }
                };
                if root_namespace.is_none() {
                    if local != "KNX" {
                        return Ok(Vec::new());
                    }
                    root_namespace = Some(namespace_uri.to_string());
                }
                let in_namespace = root_namespace.as_deref() == Some(namespace_uri)
                    && supported_namespace(namespace_uri);
                let rendered = if in_namespace {
                    local.clone()
                } else {
                    element.name().as_ref().to_string()
                };
                let in_languages = path.len() >= 2
                    && path[0].local == "KNX"
                    && path[1].local == "MasterData"
                    && ((path.len() == 2 && local == "Languages")
                        || (path.len() >= 3 && path[2].local == "Languages"));
                if in_languages {
                    items += 1;
                    if items > MAX_MASTER_LANGUAGE_ITEMS || path.len() >= MAX_EVIDENCE_DEPTH {
                        return Err(error("master-language evidence depth/item budget exceeded"));
                    }
                    let canonical = in_namespace
                        && path.iter().all(|frame| frame.in_namespace)
                        && path.len() < LANGUAGE_PATH.len()
                        && path
                            .iter()
                            .map(|frame| frame.local.as_str())
                            .eq(LANGUAGE_PATH[..path.len()].iter().copied())
                        && local == LANGUAGE_PATH[path.len()];
                    let consumed: &[&str] = if canonical {
                        match local.as_str() {
                            "Language" => &["Identifier"],
                            "TranslationElement" => &["RefId"],
                            "Translation" => &["AttributeName", "Text"],
                            _ => &[],
                        }
                    } else {
                        &[]
                    };
                    let mut owning_path = path
                        .iter()
                        .map(|frame| frame.rendered.as_str())
                        .collect::<Vec<_>>();
                    owning_path.push(&rendered);
                    let xpath = format!("/{}", owning_path.join("/"));
                    for attribute in element.attributes() {
                        let attribute = attribute.map_err(|cause| error(cause.to_string()))?;
                        let raw_name = attribute.key.as_ref();
                        if raw_name == "xmlns"
                            || raw_name.starts_with("xmlns:")
                            || consumed.contains(&raw_name)
                        {
                            continue;
                        }
                        let (namespace, local) = reader.resolver().resolve_attribute(attribute.key);
                        let name = match namespace {
                            ResolveResult::Bound(namespace) => {
                                charge(&mut evidence_bytes, namespace.as_ref().len())?;
                                format!("{{{}}}{}", namespace.as_ref(), local.as_ref())
                            }
                            ResolveResult::Unbound => raw_name.to_string(),
                            ResolveResult::Unknown(_) => {
                                return Err(error("master-language attribute namespace unresolved"))
                            }
                        };
                        let value = attribute
                            .normalized_value(XmlVersion::Implicit1_0)
                            .map_err(|cause| error(cause.to_string()))?;
                        charge(&mut evidence_bytes, xpath.len())?;
                        charge(&mut evidence_bytes, name.len())?;
                        charge(&mut evidence_bytes, value.len())?;
                        collector.attribute(&xpath, &name, &value);
                    }
                }
                if matches!(event, Event::Start(_)) {
                    path.push(Frame {
                        local,
                        rendered,
                        in_namespace,
                    });
                }
            }
            Event::End(_) => {
                if skipped_depth > 0 {
                    skipped_depth -= 1;
                } else {
                    path.pop();
                }
            }
            Event::Eof => {
                if !path.is_empty() || skipped_depth != 0 {
                    return Err(error("master-language document incomplete"));
                }
                break;
            }
            _ => {}
        }
    }
    Ok(collector.into_vec())
}
