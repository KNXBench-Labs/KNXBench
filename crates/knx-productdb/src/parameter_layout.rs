//! Read-only parameter groups and structural communication-object controls from product data.
use crate::ProductDbError;
use quick_xml::{events::Event, name::ResolveResult, NsReader};
use rusqlite::{Connection, OptionalExtension};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub node_id: i64,
    pub element_id: Option<String>,
    pub text: Option<String>,
    pub caption_ref: Option<String>,
}
#[derive(Debug, Clone, Default)]
pub struct FieldLayout {
    pub group: Option<Group>,
    pub controls_com_objects: bool,
}
#[derive(Debug, Default)]
pub struct ParameterLayout {
    fields: HashMap<(String, String), FieldLayout>,
}
impl ParameterLayout {
    pub fn field(&self, definition: &str, reference: &str) -> FieldLayout {
        self.fields
            .get(&(definition.to_owned(), reference.to_owned()))
            .cloned()
            .unwrap_or_default()
    }
}
#[derive(Debug)]
struct Node {
    parent: Option<i64>,
    kind: String,
    reference: Option<String>,
    id: Option<String>,
    text: Option<String>,
}

/// Uses normalized parent relationships, never names, numbers or manufacturer-specific IDs.
/// A reference appearing in several blocks keeps no claimed single group.
pub fn load(conn: &Connection, program: &str) -> Result<ParameterLayout, ProductDbError> {
    let captions = source_captions(conn, program)?;
    let mut stmt = conn.prepare("SELECT module_def_id,node_id,parent_id,kind,ref_id,element_id,text FROM dynamic_node WHERE program_id=?1 ORDER BY module_def_id,node_id")?;
    let rows = stmt.query_map([program], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, i64>(1)?,
            Node {
                parent: r.get(2)?,
                kind: r.get(3)?,
                reference: r.get(4)?,
                id: r.get(5)?,
                text: r.get(6)?,
            },
        ))
    })?;
    let mut trees: BTreeMap<String, BTreeMap<i64, Node>> = BTreeMap::new();
    for row in rows {
        let (scope, id, node) = row?;
        trees.entry(scope).or_default().insert(id, node);
    }
    let mut result = ParameterLayout::default();
    for (scope, nodes) in trees {
        let mut controls = BTreeSet::new();
        let mut dependencies: HashMap<String, BTreeSet<String>> = HashMap::new();
        let mut groups: HashMap<String, Option<Group>> = HashMap::new();
        for node in nodes.values() {
            if !matches!(node.kind.as_str(), "ParameterRefRef" | "ComObjectRefRef") {
                continue;
            }
            let mut parents = BTreeSet::new();
            let mut group = None;
            let mut seen = BTreeSet::new();
            let mut parent = node.parent;
            while let Some(at) = parent {
                if !seen.insert(at) || seen.len() > 64 {
                    break;
                }
                let Some(ancestor) = nodes.get(&at) else {
                    break;
                };
                if ancestor.kind == "choose" {
                    if let Some(reference) = &ancestor.reference {
                        parents.insert(reference.clone());
                    }
                }
                if group.is_none() && ancestor.kind == "ParameterBlock" {
                    group = Some(Group {
                        node_id: at,
                        element_id: ancestor.id.clone(),
                        text: ancestor.text.clone(),
                        caption_ref: ancestor
                            .id
                            .as_ref()
                            .and_then(|id| captions.get(id))
                            .cloned(),
                    });
                }
                parent = ancestor.parent;
            }
            if node.kind == "ComObjectRefRef" {
                controls.extend(parents);
            } else if let Some(reference) = &node.reference {
                dependencies
                    .entry(reference.clone())
                    .or_default()
                    .extend(parents);
                groups
                    .entry(reference.clone())
                    .and_modify(|previous| {
                        if *previous != group {
                            *previous = None;
                        }
                    })
                    .or_insert(group);
            }
        }
        // A mode can gate a parameter that gates objects elsewhere. Follow
        // those declaration dependencies, without evaluating a second tree.
        let mut pending = controls.iter().cloned().collect::<Vec<_>>();
        while let Some(reference) = pending.pop() {
            if let Some(parents) = dependencies.get(&reference) {
                for parent in parents {
                    if controls.insert(parent.clone()) {
                        pending.push(parent.clone());
                    }
                }
            }
        }
        for (reference, group) in groups {
            let controls_com_objects = controls.contains(&reference);
            result.fields.insert(
                (scope.clone(), reference),
                FieldLayout {
                    group,
                    controls_com_objects,
                },
            );
        }
    }
    Ok(result)
}

// Older catalogues did not model ParameterBlock/@ParamRefId. Read this
// presentation-only reference from the retained, hash-checked source rather
// than reparsing dynamic_node.extra (an audit string, not an interchange format).
fn source_captions(
    conn: &Connection,
    program: &str,
) -> Result<HashMap<String, String>, ProductDbError> {
    let row:Option<(String,i64,i64)>=conn.query_row("SELECT s.sha256,s.len,length(s.bytes) FROM application_program a JOIN source_file s ON s.sha256=a.source_sha256 WHERE a.id=?1",[program],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional()?;
    let Some((sha, declared, actual)) = row else {
        return Ok(HashMap::new());
    };
    if actual < 0
        || actual > crate::parameter_editor::MAX_EDITOR_SOURCE_BYTES as i64
        || actual != declared
    {
        return Ok(HashMap::new());
    }
    let bytes: Vec<u8> = conn.query_row(
        "SELECT bytes FROM source_file WHERE sha256=?1",
        [&sha],
        |r| r.get(0),
    )?;
    if crate::sha256_hex(&bytes) != sha {
        return Ok(HashMap::new());
    }
    Ok(read_captions(&bytes, program).unwrap_or_default())
}
fn read_captions(bytes: &[u8], program: &str) -> Option<HashMap<String, String>> {
    let mut reader = NsReader::from_reader(bytes);
    // Element name, exact selected program ownership, admitted Dynamic ancestry.
    let mut stack: Vec<(String, bool, bool)> = Vec::new();
    let mut result = HashMap::new();
    let mut namespace = None;
    let mut owners = 0;
    let mut seen = BTreeSet::new();
    for _ in 0..100_000 {
        let (ns, event) = reader.read_resolved_event().ok()?;
        let empty = matches!(&event, Event::Empty(_));
        match event {
            Event::Start(e) | Event::Empty(e) => {
                let resolved = match ns {
                    ResolveResult::Bound(ns) => Some(ns.as_ref().to_owned()),
                    _ => None,
                };
                let kind = e.local_name().as_ref().to_owned();
                if stack.is_empty() {
                    if kind != "KNX" {
                        return None;
                    }
                    namespace = resolved.clone();
                }
                let same = resolved == namespace;
                let path: Vec<&str> = stack.iter().map(|(name, _, _)| name.as_str()).collect();
                let attr = crate::xml::attrs(&e, "parameter presentation").ok()?;
                let owner = same
                    && kind == "ApplicationProgram"
                    && path
                        == [
                            "KNX",
                            "ManufacturerData",
                            "Manufacturer",
                            "ApplicationPrograms",
                        ]
                    && attr.evidence_value("Id") == Some(program);
                if owner {
                    owners += 1;
                }
                let active = same && (owner || stack.last().is_some_and(|(_, active, _)| *active));
                let dynamic_root = kind == "Dynamic"
                    && (path
                        == [
                            "KNX",
                            "ManufacturerData",
                            "Manufacturer",
                            "ApplicationPrograms",
                            "ApplicationProgram",
                        ]
                        || path
                            == [
                                "KNX",
                                "ManufacturerData",
                                "Manufacturer",
                                "ApplicationPrograms",
                                "ApplicationProgram",
                                "ModuleDefs",
                                "ModuleDef",
                            ]);
                let inside_dynamic = active
                    && (dynamic_root
                        || (stack.last().is_some_and(|(_, _, dynamic)| *dynamic)
                            && matches!(
                                kind.as_str(),
                                "Channel"
                                    | "ChannelIndependentBlock"
                                    | "ParameterBlock"
                                    | "choose"
                                    | "when"
                                    | "Repeat"
                            )));
                if inside_dynamic && kind == "ParameterBlock" {
                    // A prefixed extension attribute is not the source declaration.
                    if let (Some(id), Some(reference)) =
                        (attr.evidence_value("Id"), attr.evidence_value("ParamRefId"))
                    {
                        if !seen.insert(id.to_owned()) {
                            return None;
                        }
                        result.insert(id.to_owned(), reference.to_owned());
                    }
                }
                if !empty {
                    stack.push((kind, active, inside_dynamic));
                    if stack.len() > 64 {
                        return None;
                    }
                }
            }
            Event::End(_) => {
                stack.pop()?;
            }
            Event::DocType(_) => return None,
            Event::Eof => return (stack.is_empty() && owners == 1).then_some(result),
            _ => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn captions_require_the_exact_owner_and_do_not_accept_duplicate_or_foreign_blocks() {
        let xml=br#"<KNX xmlns="http://knx.org/xml/project/23"><ManufacturerData><Manufacturer RefId="M"><ApplicationPrograms><ApplicationProgram Id="A"><Dynamic><ParameterBlock Id="B" ParamRefId="P"/><x:ParameterBlock xmlns:x="urn:foreign" Id="X" ParamRefId="Q"/></Dynamic></ApplicationProgram><ApplicationProgram Id="Other"><Dynamic><ParameterBlock Id="B" ParamRefId="Other"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
        assert_eq!(
            read_captions(xml, "A").unwrap(),
            HashMap::from([("B".into(), "P".into())])
        );
        assert!(read_captions(xml, "missing").is_none());
        assert!(read_captions(br#"<!DOCTYPE KNX><KNX/>"#, "A").is_none());
    }
}
