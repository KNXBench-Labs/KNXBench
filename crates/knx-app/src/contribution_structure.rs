//! Bounded XML shapes, not a competing typed KNX parser or an XSD validator.
use super::contribution::{Analysis, AnalysisError, Check, Member};
use quick_xml::{events::Event, name::ResolveResult, NsReader};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_EXPANDED_BYTES: u64 = 64 * 1024 * 1024;
pub const MAX_XML_BYTES: u64 = 8 * 1024 * 1024;
const MAX_MEMBERS: usize = 4096;
const MAX_EVENTS: usize = 200_000;
const MAX_SHAPES: usize = 10_000;
const MAX_DEPTH: usize = 128;
const MAX_IDENTITY_BYTES: usize = 1024;
const MAX_PATH_BYTES: usize = 16 * 1024;
const MAX_SHAPE_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Shape {
    pub member_id: String,
    pub path: Vec<String>,
    pub namespace: String,
    pub name: String,
    pub kind: String,
    pub occurrences: u64,
}
struct Scan {
    shapes: Vec<Shape>,
    scheme: Option<u32>,
    sensitive: bool,
}
fn namespace(value: ResolveResult<'_>) -> Result<String, &'static str> {
    match value {
        ResolveResult::Bound(ns) => Ok(ns.as_ref().to_owned()),
        ResolveResult::Unbound => Ok(String::new()),
        ResolveResult::Unknown(_) => Err("unbound XML prefix"),
    }
}
fn secret(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("password")
        || n.contains("secret")
        || n.ends_with("key")
        || n.contains("keyring")
        || n.contains("credential")
}
fn scan(
    bytes: &[u8],
    id: &str,
    events: &mut usize,
    shape_count: &mut usize,
    shape_bytes: &mut usize,
) -> Result<Scan, &'static str> {
    let mut reader = NsReader::from_reader(bytes);
    let mut stack = Vec::new();
    let mut shapes = BTreeMap::<(Vec<String>, String, String, String), u64>::new();
    let mut scheme = None;
    let mut sensitive = false;
    let mut roots = 0;
    loop {
        *events += 1;
        if *events > MAX_EVENTS {
            return Err("XML event budget exceeded");
        }
        let (ns, event) = reader.read_resolved_event().map_err(|_| "malformed XML")?;
        match event {
            Event::Start(ref element) | Event::Empty(ref element) => {
                let ns = namespace(ns)?;
                let local = element.local_name();
                let name = local.as_ref().to_owned();
                if ns.len() > MAX_IDENTITY_BYTES || name.len() > MAX_IDENTITY_BYTES {
                    return Err("XML identity budget exceeded");
                }
                if stack.is_empty() {
                    roots += 1;
                    if roots != 1 {
                        return Err("multiple XML roots");
                    }
                    if name == "KNX" {
                        if let Some(tail) = ns.strip_prefix("http://knx.org/xml/project/") {
                            scheme = tail.parse::<u32>().ok().filter(|v| v.to_string() == tail);
                        }
                    }
                }
                sensitive |= secret(&name);
                stack.push(format!("{{{ns}}}{name}"));
                if stack.len() > MAX_DEPTH {
                    return Err("XML depth budget exceeded");
                }
                let path_bytes = stack.iter().map(String::len).sum::<usize>();
                if path_bytes > MAX_PATH_BYTES {
                    return Err("XML path budget exceeded");
                }
                let mut add =
                    |namespace: String, name: String, kind: &str| -> Result<(), &'static str> {
                        if namespace.len() > MAX_IDENTITY_BYTES || name.len() > MAX_IDENTITY_BYTES {
                            return Err("XML attribute identity budget exceeded");
                        }
                        let retained_bytes = path_bytes + namespace.len() + name.len() + kind.len();
                        // Charge work before cloning attacker-controlled paths, including repeats.
                        *shape_bytes += retained_bytes;
                        if *shape_bytes > MAX_SHAPE_BYTES {
                            return Err("XML shape memory/work budget exceeded");
                        }
                        let key = (stack.clone(), namespace, name, kind.to_owned());
                        if !shapes.contains_key(&key) {
                            *shape_count += 1;
                            if *shape_count > MAX_SHAPES {
                                return Err("XML shape budget exceeded");
                            }
                        }
                        *shapes.entry(key).or_default() += 1;
                        Ok(())
                    };
                add(ns, name, "element")?;
                for attribute in element.attributes() {
                    let a = attribute.map_err(|_| "malformed or duplicate XML attribute")?;
                    if a.key.as_ref() == "xmlns" || a.key.as_ref().starts_with("xmlns:") {
                        continue;
                    }
                    let (ns, local) = reader.resolver().resolve_attribute(a.key);
                    let ns = namespace(ns)?;
                    let name = local.as_ref().to_owned();
                    sensitive |= secret(&name);
                    // Validate entities without retaining any source value.
                    a.normalized_value(quick_xml::XmlVersion::Implicit1_0)
                        .map_err(|_| "malformed attribute value")?;
                    add(ns, name, "attribute")?;
                }
                if matches!(event, Event::Empty(_)) {
                    stack.pop();
                }
            }
            Event::End(_) => {
                stack.pop().ok_or("unmatched XML end")?;
            }
            Event::DocType(_) => return Err("DTD is not accepted for contribution samples"),
            Event::Text(text) => {
                if stack.is_empty() && text.as_ref().bytes().any(|b| !b.is_ascii_whitespace()) {
                    return Err("text outside root");
                }
            }
            Event::Eof => {
                if !stack.is_empty() || roots == 0 {
                    return Err("incomplete XML");
                }
                break;
            }
            _ => {}
        }
    }
    Ok(Scan {
        scheme,
        sensitive,
        shapes: shapes
            .into_iter()
            .map(|((path, namespace, name, kind), occurrences)| Shape {
                member_id: id.into(),
                path,
                namespace,
                name,
                kind,
                occurrences,
            })
            .collect(),
    })
}

/// Failure stops the typed path; unexamined members never gain sample permission.
pub(super) fn inspect(bytes: &[u8], analysis: &mut Analysis) -> Result<bool, AnalysisError> {
    let mut container = knx_etsproj::container::Container::open(bytes.to_vec()).map_err(|_| {
        AnalysisError::Input("invalid, protected or unsafe archive; no analysis performed")
    })?;
    let entries = container.entries().to_vec();
    if entries.len() > MAX_MEMBERS
        || entries
            .iter()
            .try_fold(0u64, |sum, e| sum.checked_add(e.size))
            .is_none_or(|n| n > MAX_EXPANDED_BYTES)
    {
        return Err(AnalysisError::Input(
            "analysis archive exceeds 4096 members or 64 MiB expanded",
        ));
    }
    if entries.iter().any(|e| {
        e.path.starts_with('/')
            || e.path.contains(['\\', ':', '\0'])
            || e.path
                .split('/')
                .any(|part| matches!(part, "" | "." | ".."))
    }) {
        return Err(AnalysisError::Input(
            "unsafe archive-relative member identity",
        ));
    }
    analysis.original_allowed = !entries.is_empty();
    let mut events = 0;
    let mut shapes = 0;
    let mut shape_bytes = 0;
    let mut complete = true;
    for (i, e) in entries.iter().enumerate() {
        let id = format!("member-{}", i + 1);
        let mut member = Member {
            id: id.clone(),
            path: e.path.clone(),
            size: e.size,
            sample_allowed: false,
            reason: "Not an examined plain XML member".into(),
        };
        if e.path.to_ascii_lowercase().ends_with(".xml") && e.size <= MAX_XML_BYTES {
            match container
                .read(&e.path)
                .ok()
                .and_then(|data| scan(&data, &id, &mut events, &mut shapes, &mut shape_bytes).ok())
            {
                Some(scan) => {
                    if e.path.eq_ignore_ascii_case("knx_master.xml") {
                        analysis.scheme = scan.scheme;
                    }
                    member.sample_allowed = !scan.sensitive;
                    member.reason = if scan.sensitive {
                        "Known credential/key-like field detected; sharing blocked"
                    } else {
                        "Unmodified XML context; may contain private or proprietary values"
                    }
                    .into();
                    analysis.structure.extend(scan.shapes);
                }
                None => {
                    complete = false;
                    member.reason = "XML malformed, unreadable, DTD or scan budget exceeded".into();
                }
            }
        } else if e.path.to_ascii_lowercase().ends_with(".xml") {
            complete = false;
        }
        analysis.original_allowed &= member.sample_allowed;
        analysis.members.push(member);
    }
    analysis.checks.push(Check { name: "xml-structure".into(), status: if complete { "measured" } else { "partial" }.into(), detail: "Expanded namespace/full ancestor shapes only; not XSD validity or typed semantics. Non-XML members remain unexamined".into() });
    analysis.checks.push(Check {
        name: "xsd-validation".into(),
        status: "unavailable".into(),
        detail: "No authoritative XSD validator in this analysis service".into(),
    });
    analysis.checks.push(Check {
        name: "hardware".into(),
        status: "not-examined".into(),
        detail: "No bus connection or hardware verification".into(),
    });
    Ok(complete)
}

pub(super) fn sample(bytes: &[u8], path: &str) -> Result<Vec<u8>, AnalysisError> {
    let mut container = knx_etsproj::container::Container::open(bytes.to_vec())
        .map_err(|_| AnalysisError::Input("source archive unavailable"))?;
    let bytes = container
        .read(path)
        .map_err(|_| AnalysisError::Input("sample member unavailable"))?;
    std::str::from_utf8(&bytes).map_err(|_| {
        AnalysisError::Input("sample has no UTF-8 text preview; not shareable as context")
    })?;
    Ok(bytes)
}
