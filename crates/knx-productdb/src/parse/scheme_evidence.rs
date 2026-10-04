//! Targeted evidence for accepted product schemes whose semantics stay unmodelled.

use quick_xml::events::{BytesStart, Event};
use quick_xml::name::ResolveResult;
use quick_xml::{NsReader, XmlVersion};
use std::collections::{HashMap, HashSet};
use std::io::BufRead;

use crate::report::{UnknownCollector, UnknownConstruct, UnknownKind};
use crate::xml::local_name;
use crate::ProductDbError;

pub(crate) const MAX_EVIDENCE_DEPTH: usize = 1024;
// Coupled scan-work ceilings, not a bound on process RSS. Raw attributes,
// repeated paths, and expanded namespace names remain charged independently.
// KL-152 measurement and policy rationale: docs/PRODUCT_DATABASE_CORPUS.md.
pub(crate) const MAX_EVIDENCE_ITEMS: usize = 1_048_576;
pub(crate) const MAX_EVIDENCE_BYTES: usize = 256 * 1024 * 1024;
const SCHEME_12_NAMESPACE: &str = "http://knx.org/xml/project/12";
const SCHEME_14_NAMESPACE: &str = "http://knx.org/xml/project/14";
const SCHEME_21_NAMESPACE: &str = "http://knx.org/xml/project/21";
const SCHEME_23_NAMESPACE: &str = "http://knx.org/xml/project/23";

fn xml_error(source_path: &str, cause: impl ToString) -> ProductDbError {
    ProductDbError::Xml {
        source_path: source_path.to_string(),
        cause: cause.to_string(),
    }
}

#[derive(Debug)]
struct PathElement {
    local_name: String,
    rendered_name: String,
    in_project_namespace: bool,
    module_def_active: bool,
}

struct ElementIdentity {
    expanded_name: String,
    rendered_name: String,
    extended_scheme: bool,
}

fn xpath(path: &[PathElement]) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        format!(
            "/{}",
            path.iter()
                .map(|element| element.rendered_name.as_str())
                .collect::<Vec<_>>()
                .join("/")
        )
    }
}

fn local_xpath(path: &[PathElement]) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        format!(
            "/{}",
            path.iter()
                .map(|element| element.local_name.as_str())
                .collect::<Vec<_>>()
                .join("/")
        )
    }
}

const APPLICATION_PROGRAM_PARENT: [&str; 4] = [
    "KNX",
    "ManufacturerData",
    "Manufacturer",
    "ApplicationPrograms",
];

fn is_application_program_parent(path: &[PathElement]) -> bool {
    path.len() == APPLICATION_PROGRAM_PARENT.len()
        && path.iter().all(|element| element.in_project_namespace)
        && path
            .iter()
            .map(|element| element.local_name.as_str())
            .eq(APPLICATION_PROGRAM_PARENT)
}

fn is_inside_application_program(path: &[PathElement]) -> bool {
    const COMMON: [&str; 5] = [
        "KNX",
        "ManufacturerData",
        "Manufacturer",
        "ApplicationPrograms",
        "ApplicationProgram",
    ];
    path.len() >= COMMON.len()
        && path
            .iter()
            .take(APPLICATION_PROGRAM_PARENT.len())
            .all(|element| element.in_project_namespace)
        && path
            .iter()
            .take(COMMON.len())
            .map(|element| element.local_name.as_str())
            .eq(COMMON)
}

fn is_application_program_context(path: &[PathElement], current_name: &str) -> bool {
    const COMMON_LEN: usize = 5;
    if current_name == "ApplicationProgram" && is_application_program_parent(path) {
        return true;
    }
    path.len() >= COMMON_LEN
        && is_inside_application_program(path)
        && path.iter().all(|element| element.in_project_namespace)
        && ((path.len() == COMMON_LEN && matches!(current_name, "Static" | "Dynamic"))
            || (path.len() > COMMON_LEN
                && (matches!(path[COMMON_LEN].local_name.as_str(), "Static" | "Dynamic")
                    || (path.last().is_some_and(|element| element.module_def_active)
                        && (current_name == "Dynamic"
                            || path
                                .iter()
                                .skip(COMMON_LEN)
                                .any(|element| element.local_name == "Dynamic"))))))
}

type AliasKey = (String, bool, String);

fn parser_alias_xpath(path: &[PathElement], local_name: &str) -> String {
    const PROGRAM: &str =
        "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram";
    let dynamic_index = path
        .iter()
        .position(|item| item.local_name == "Dynamic")
        .or_else(|| (local_name == "Dynamic").then_some(path.len()));
    if let Some(dynamic_index) = dynamic_index {
        let module_dynamic = path[..dynamic_index]
            .last()
            .is_some_and(|item| item.module_def_active);
        if module_dynamic {
            return format!("{PROGRAM}/ModuleDefs/ModuleDef/Dynamic/{local_name}");
        }
        return format!("{PROGRAM}/Dynamic/{local_name}");
    }
    let parent = local_xpath(path);
    if parent == "/" {
        format!("/{local_name}")
    } else {
        format!("{parent}/{local_name}")
    }
}

fn add_alias(aliases: &mut HashMap<AliasKey, u32>, xpath: String, kind: UnknownKind, name: String) {
    let key = (xpath, kind == UnknownKind::Attribute, name);
    *aliases.entry(key).or_default() += 1;
}

fn unknown_key(item: &UnknownConstruct) -> AliasKey {
    (
        item.xpath.clone(),
        item.kind == UnknownKind::Attribute,
        item.name.clone(),
    )
}

fn charge_evidence_budget(
    item_count: &mut usize,
    byte_count: &mut usize,
    path: &[PathElement],
    element: &BytesStart<'_>,
    source_path: &str,
) -> Result<(), ProductDbError> {
    let path_bytes = path.iter().try_fold(1_usize, |total, item| {
        total.checked_add(1 + item.rendered_name.len())
    });
    let path_bytes = path_bytes
        .and_then(|total| total.checked_add(1 + element.name().as_ref().len()))
        .ok_or_else(|| xml_error(source_path, "XML evidence path size overflow"))?;
    let mut attributes = 0_usize;
    let mut estimated_bytes = path_bytes;
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| xml_error(source_path, error))?;
        attributes = attributes
            .checked_add(1)
            .ok_or_else(|| xml_error(source_path, "XML evidence item count overflow"))?;
        estimated_bytes = estimated_bytes
            .checked_add(path_bytes)
            .and_then(|total| total.checked_add(attribute.key.as_ref().len()))
            .and_then(|total| total.checked_add(attribute.value.as_ref().len()))
            .ok_or_else(|| xml_error(source_path, "XML evidence byte count overflow"))?;
    }
    *item_count = item_count
        .checked_add(1 + attributes)
        .ok_or_else(|| xml_error(source_path, "XML evidence item count overflow"))?;
    *byte_count = byte_count
        .checked_add(estimated_bytes)
        .ok_or_else(|| xml_error(source_path, "XML evidence byte count overflow"))?;
    if *item_count > MAX_EVIDENCE_ITEMS {
        return Err(xml_error(
            source_path,
            format!("XML evidence exceeds item limit {MAX_EVIDENCE_ITEMS}"),
        ));
    }
    if *byte_count > MAX_EVIDENCE_BYTES {
        return Err(xml_error(
            source_path,
            format!("XML evidence exceeds byte limit {MAX_EVIDENCE_BYTES}"),
        ));
    }
    Ok(())
}

fn charge_expanded_name_budget<R: BufRead>(
    reader: &NsReader<R>,
    byte_count: &mut usize,
    element: &BytesStart<'_>,
    identity: &ElementIdentity,
    source_path: &str,
) -> Result<(), ProductDbError> {
    let mut additional = identity.expanded_name.len();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| xml_error(source_path, error))?;
        additional = additional
            .checked_add(identity.expanded_name.len())
            .and_then(|total| total.checked_add(expanded_attribute_name(reader, &attribute).len()))
            .ok_or_else(|| xml_error(source_path, "XML expanded-name byte count overflow"))?;
    }
    *byte_count = byte_count
        .checked_add(additional)
        .ok_or_else(|| xml_error(source_path, "XML evidence byte count overflow"))?;
    if *byte_count > MAX_EVIDENCE_BYTES {
        return Err(xml_error(
            source_path,
            format!("XML evidence exceeds byte limit {MAX_EVIDENCE_BYTES}"),
        ));
    }
    Ok(())
}

fn expanded_attribute_name<R: BufRead>(
    reader: &NsReader<R>,
    attribute: &quick_xml::events::attributes::Attribute<'_>,
) -> String {
    let (namespace, local) = reader.resolver().resolve_attribute(attribute.key);
    match namespace {
        ResolveResult::Bound(namespace) => {
            format!("{{{}}}{}", namespace.as_ref(), local.as_ref())
        }
        _ => attribute.key.as_ref().to_string(),
    }
}

fn is_targeted_attribute(xpath: &str, name: &str) -> bool {
    (name == "AppliesTo" && xpath.ends_with("/LdCtrlWriteProp"))
        || (name == "Occurrence" && xpath.ends_with("/Property"))
        || (xpath.ends_with("/ParameterSeparator") && name == "UIHint")
        || (xpath.ends_with("/ApplicationProgram") && name == "HardwareType")
        || (xpath.ends_with("/DatapointType") && name == "VariableLength")
        || (xpath.ends_with("/Hardware2Program")
            && matches!(
                name,
                "CouplerCapabilities" | "RFRxCapabilities" | "RFTxCapabilities"
            ))
        || (xpath.ends_with("/InterfaceObjectProperty") && name == "AccessPolicy")
        || (xpath.ends_with("/Resource") && name == "Optional")
        || (xpath.ends_with("/String") && matches!(name, "NullTerminated" | "VariableLength"))
        || xpath.ends_with("/LdCtrlDeclarePropDesc")
}

fn is_extended_scheme_non_program_context(path: &[PathElement], name: &str) -> bool {
    if !path.iter().all(|element| element.in_project_namespace) {
        return false;
    }
    let names: Vec<&str> = path
        .iter()
        .map(|element| element.local_name.as_str())
        .collect();
    match name {
        "DatapointType" => names == ["KNX", "MasterData", "DatapointTypes"],
        "Resource" => {
            names == ["KNX", "MasterData", "Resources"]
                || names
                    == [
                        "KNX",
                        "MasterData",
                        "MaskVersions",
                        "MaskVersion",
                        "HawkConfigurationData",
                        "Resources",
                    ]
        }
        "InterfaceObjectProperty" => {
            names == ["KNX", "MasterData", "InterfaceObjectProperties"]
                || names
                    == [
                        "KNX",
                        "MasterData",
                        "InterfaceObjectTypes",
                        "InterfaceObjectType",
                        "InterfaceObjectProperties",
                    ]
        }
        "String" => {
            names
                == [
                    "KNX",
                    "MasterData",
                    "DatapointTypes",
                    "DatapointType",
                    "DatapointSubtypes",
                    "DatapointSubtype",
                    "Format",
                ]
        }
        "Hardware2Program" => {
            names
                == [
                    "KNX",
                    "ManufacturerData",
                    "Manufacturer",
                    "Hardware",
                    "Hardware",
                    "Hardware2Programs",
                ]
        }
        _ => false,
    }
}

fn unqualified_attributes(
    element: &BytesStart<'_>,
    source_path: &str,
) -> Result<Vec<(String, String)>, ProductDbError> {
    let mut attributes = Vec::new();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| xml_error(source_path, error))?;
        let name = attribute.key.as_ref();
        if name.contains(':') {
            continue;
        }
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| xml_error(source_path, error))?;
        attributes.push((name.to_string(), value.into_owned()));
    }
    Ok(attributes)
}

fn parser_attribute_value(
    element: &BytesStart<'_>,
    wanted: &str,
    source_path: &str,
) -> Result<Option<String>, ProductDbError> {
    let mut qualified = None;
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| xml_error(source_path, error))?;
        if attribute.key.local_name().as_ref() != wanted {
            continue;
        }
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| xml_error(source_path, error))?
            .into_owned();
        if !attribute.key.as_ref().contains(':') {
            return Ok(Some(value));
        }
        qualified = Some(value);
    }
    Ok(qualified)
}

fn path_element(
    path: &[PathElement],
    element: &BytesStart<'_>,
    rendered_name: String,
    in_project_namespace: bool,
    source_path: &str,
) -> Result<PathElement, ProductDbError> {
    let local_name = local_name(element);
    let inherited_module = path.last().is_some_and(|element| element.module_def_active);
    let opens_module = local_name == "ModuleDef"
        && parser_attribute_value(element, "Id", source_path)?.is_some_and(|id| !id.is_empty());
    Ok(PathElement {
        local_name,
        rendered_name,
        in_project_namespace,
        module_def_active: inherited_module || opens_module,
    })
}

fn record_element<R: BufRead>(
    reader: &NsReader<R>,
    collector: &mut UnknownCollector,
    aliases: &mut HashMap<AliasKey, u32>,
    path: &[PathElement],
    element: &BytesStart<'_>,
    source_path: &str,
) -> Result<(), ProductDbError> {
    let name = local_name(element);
    let parent = xpath(path);
    let current = if parent == "/" {
        format!("/{name}")
    } else {
        format!("{parent}/{name}")
    };
    let attributes = unqualified_attributes(element, source_path)?;

    if name == "LdCtrlDeclarePropDesc" && is_application_program_context(path, &name) {
        collector.element(&parent, &name);
        for (attribute, value) in &attributes {
            collector.attribute(&current, attribute, value);
        }
    } else {
        for (attribute, value) in &attributes {
            if is_targeted_attribute(&current, attribute)
                && (is_application_program_context(path, &name)
                    || is_extended_scheme_non_program_context(path, &name))
            {
                collector.attribute(&current, attribute, value);
            }
        }
    }
    let mut aliased_names = HashSet::new();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| xml_error(source_path, error))?;
        let raw_name = attribute.key.as_ref().to_string();
        if !raw_name.contains(':') {
            continue;
        }
        let expanded_name = expanded_attribute_name(reader, &attribute);
        if aliased_names.insert(raw_name.clone()) {
            add_alias(
                aliases,
                parser_alias_xpath(path, &name),
                UnknownKind::Attribute,
                raw_name,
            );
        }
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| xml_error(source_path, error))?;
        collector.attribute(&current, &expanded_name, &value);
    }
    Ok(())
}

fn record_namespace_lookalike<R: BufRead>(
    reader: &NsReader<R>,
    collector: &mut UnknownCollector,
    aliases: &mut HashMap<AliasKey, u32>,
    path: &[PathElement],
    element: &BytesStart<'_>,
    identity: &ElementIdentity,
    source_path: &str,
) -> Result<(), ProductDbError> {
    let local = local_name(element);
    if !(is_inside_application_program(path)
        || (is_application_program_parent(path) && local == "ApplicationProgram")
        || (identity.extended_scheme && is_extended_scheme_non_program_context(path, &local)))
    {
        return Ok(());
    }
    let parent = xpath(path);
    let qualified = &identity.expanded_name;
    let current = format!("{parent}/{}", identity.rendered_name);
    collector.element(&parent, qualified);
    add_alias(
        aliases,
        local_xpath(path),
        UnknownKind::Element,
        local.clone(),
    );
    let mut aliased_names = HashSet::new();
    for attribute in element.attributes() {
        let attribute = attribute.map_err(|error| xml_error(source_path, error))?;
        let name = format!(
            "{qualified}/@{}",
            expanded_attribute_name(reader, &attribute)
        );
        let value = attribute
            .normalized_value(XmlVersion::Implicit1_0)
            .map_err(|error| xml_error(source_path, error))?;
        collector.attribute(&current, &name, &value);
        let raw_attribute = attribute.key.as_ref().to_string();
        if aliased_names.insert(raw_attribute.clone()) {
            add_alias(
                aliases,
                parser_alias_xpath(path, &local),
                UnknownKind::Attribute,
                raw_attribute,
            );
        }
    }
    Ok(())
}

/// Reconciles parser-local sightings of the PDB-5 constructs with one complete
/// streaming scan over the preserved XML member. Parsers may skip an unknown
/// subtree after reporting its root; this pass keeps the named semantic fields
/// visible without pretending to interpret them. Exact parser evidence is
/// updated monotonically. Namespace-blind aliases for foreign elements and
/// attributes are replaced by namespace-qualified rows from this pass so one
/// physical sighting cannot survive under both identities.
pub(crate) fn reconcile_targeted_unknowns(
    bytes: &[u8],
    source_path: &str,
    unknown: &mut Vec<UnknownConstruct>,
) -> Result<(), ProductDbError> {
    reconcile_unknowns(bytes, source_path, unknown, false)
}

pub(crate) fn reconcile_package_unknowns(
    bytes: &[u8],
    source_path: &str,
    unknown: &mut Vec<UnknownConstruct>,
) -> Result<(), ProductDbError> {
    reconcile_unknowns(bytes, source_path, unknown, true)
}

fn reconcile_unknowns(
    bytes: &[u8],
    source_path: &str,
    unknown: &mut Vec<UnknownConstruct>,
    accept_extended_schemes: bool,
) -> Result<(), ProductDbError> {
    let text = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    if text.iter().find(|byte| !byte.is_ascii_whitespace()) != Some(&b'<') {
        return Ok(());
    }

    let mut reader = NsReader::from_reader(text);
    let mut buffer = Vec::new();
    let mut path = Vec::new();
    let mut collector = UnknownCollector::default();
    let mut aliases = HashMap::new();
    let mut evidence_items = 0;
    let mut evidence_bytes = 0;
    let mut project_namespace: Option<String> = None;
    loop {
        buffer.clear();
        let (namespace, event) = reader
            .read_resolved_event_into(&mut buffer)
            .map_err(|error| xml_error(source_path, error))?;
        match event {
            Event::Start(element) => {
                if project_namespace.is_none() {
                    let root_namespace = match namespace {
                        ResolveResult::Bound(namespace) => namespace.as_ref().to_string(),
                        _ => return Ok(()),
                    };
                    if element.local_name().as_ref() != "KNX"
                        || !(matches!(
                            root_namespace.as_str(),
                            SCHEME_12_NAMESPACE | SCHEME_14_NAMESPACE
                        ) || (accept_extended_schemes
                            && matches!(
                                root_namespace.as_str(),
                                SCHEME_21_NAMESPACE | SCHEME_23_NAMESPACE
                            )))
                    {
                        return Ok(());
                    }
                    project_namespace = Some(root_namespace);
                }
                if path.len() >= MAX_EVIDENCE_DEPTH {
                    return Err(xml_error(
                        source_path,
                        format!("XML nesting exceeds evidence limit {MAX_EVIDENCE_DEPTH}"),
                    ));
                }
                charge_evidence_budget(
                    &mut evidence_items,
                    &mut evidence_bytes,
                    &path,
                    &element,
                    source_path,
                )?;
                let in_project_namespace = matches!(
                    namespace,
                    ResolveResult::Bound(namespace)
                        if project_namespace.as_deref() == Some(namespace.as_ref())
                );
                let rendered_name = if in_project_namespace {
                    local_name(&element)
                } else {
                    element.name().as_ref().to_string()
                };
                let local = local_name(&element);
                let expanded_name = match namespace {
                    ResolveResult::Bound(namespace) => {
                        format!("{{{}}}{local}", namespace.as_ref())
                    }
                    _ => format!("{{}}{local}"),
                };
                let identity = ElementIdentity {
                    expanded_name,
                    rendered_name: rendered_name.clone(),
                    extended_scheme: matches!(
                        project_namespace.as_deref(),
                        Some(SCHEME_21_NAMESPACE | SCHEME_23_NAMESPACE)
                    ),
                };
                charge_expanded_name_budget(
                    &reader,
                    &mut evidence_bytes,
                    &element,
                    &identity,
                    source_path,
                )?;
                if in_project_namespace
                    && (is_application_program_context(&path, &local)
                        || (matches!(
                            project_namespace.as_deref(),
                            Some(SCHEME_21_NAMESPACE | SCHEME_23_NAMESPACE)
                        ) && is_extended_scheme_non_program_context(&path, &local)))
                {
                    record_element(
                        &reader,
                        &mut collector,
                        &mut aliases,
                        &path,
                        &element,
                        source_path,
                    )?;
                } else if !in_project_namespace
                    || (is_inside_application_program(&path)
                        && !path.iter().all(|element| element.in_project_namespace))
                {
                    record_namespace_lookalike(
                        &reader,
                        &mut collector,
                        &mut aliases,
                        &path,
                        &element,
                        &identity,
                        source_path,
                    )?;
                }
                path.push(path_element(
                    &path,
                    &element,
                    rendered_name,
                    in_project_namespace,
                    source_path,
                )?);
            }
            Event::Empty(element) => {
                if project_namespace.is_none() {
                    let root_namespace = match namespace {
                        ResolveResult::Bound(namespace) => namespace.as_ref().to_string(),
                        _ => return Ok(()),
                    };
                    if element.local_name().as_ref() != "KNX"
                        || !(matches!(
                            root_namespace.as_str(),
                            SCHEME_12_NAMESPACE | SCHEME_14_NAMESPACE
                        ) || (accept_extended_schemes
                            && matches!(
                                root_namespace.as_str(),
                                SCHEME_21_NAMESPACE | SCHEME_23_NAMESPACE
                            )))
                    {
                        return Ok(());
                    }
                    project_namespace = Some(root_namespace);
                }
                charge_evidence_budget(
                    &mut evidence_items,
                    &mut evidence_bytes,
                    &path,
                    &element,
                    source_path,
                )?;
                let in_project_namespace = matches!(
                    namespace,
                    ResolveResult::Bound(namespace)
                        if project_namespace.as_deref() == Some(namespace.as_ref())
                );
                let rendered_name = if in_project_namespace {
                    local_name(&element)
                } else {
                    element.name().as_ref().to_string()
                };
                let local = local_name(&element);
                let expanded_name = match namespace {
                    ResolveResult::Bound(namespace) => {
                        format!("{{{}}}{local}", namespace.as_ref())
                    }
                    _ => format!("{{}}{local}"),
                };
                let identity = ElementIdentity {
                    expanded_name,
                    rendered_name: rendered_name.clone(),
                    extended_scheme: matches!(
                        project_namespace.as_deref(),
                        Some(SCHEME_21_NAMESPACE | SCHEME_23_NAMESPACE)
                    ),
                };
                charge_expanded_name_budget(
                    &reader,
                    &mut evidence_bytes,
                    &element,
                    &identity,
                    source_path,
                )?;
                if in_project_namespace
                    && (is_application_program_context(&path, &local)
                        || (matches!(
                            project_namespace.as_deref(),
                            Some(SCHEME_21_NAMESPACE | SCHEME_23_NAMESPACE)
                        ) && is_extended_scheme_non_program_context(&path, &local)))
                {
                    record_element(
                        &reader,
                        &mut collector,
                        &mut aliases,
                        &path,
                        &element,
                        source_path,
                    )?;
                } else if !in_project_namespace
                    || (is_inside_application_program(&path)
                        && !path.iter().all(|element| element.in_project_namespace))
                {
                    record_namespace_lookalike(
                        &reader,
                        &mut collector,
                        &mut aliases,
                        &path,
                        &element,
                        &identity,
                        source_path,
                    )?;
                }
            }
            Event::End(_) => {
                if path.pop().is_none() {
                    return Err(xml_error(source_path, "unexpected XML closing element"));
                }
            }
            Event::Eof if path.is_empty() => break,
            Event::Eof => return Err(xml_error(source_path, "unexpected end of XML document")),
            _ => {}
        }
    }

    for item in unknown.iter_mut() {
        if let Some(alias_occurrences) = aliases.get(&unknown_key(item)) {
            item.occurrences = item.occurrences.saturating_sub(*alias_occurrences);
        }
    }
    unknown.retain(|item| item.occurrences > 0);

    let mut indices = unknown
        .iter()
        .enumerate()
        .map(|(index, item)| (unknown_key(item), index))
        .collect::<HashMap<_, _>>();
    for replacement in collector.into_vec() {
        let key = unknown_key(&replacement);
        if let Some(index) = indices.get(&key).copied() {
            let existing = &mut unknown[index];
            existing.occurrences = existing.occurrences.max(replacement.occurrences);
            if existing.sample.is_none() {
                existing.sample = replacement.sample;
            }
        } else {
            indices.insert(key, unknown.len());
            unknown.push(replacement);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::UnknownKind;

    #[test]
    fn extended_scheme_scanning_is_package_only() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram HardwareType="RF"/></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
        let mut generic = Vec::new();
        reconcile_targeted_unknowns(xml, "application.xml", &mut generic).unwrap();
        assert!(generic.is_empty());
        let mut package = Vec::new();
        reconcile_package_unknowns(xml, "application.xml", &mut package).unwrap();
        assert!(package
            .iter()
            .any(|item| item.name == "HardwareType" && item.sample.as_deref() == Some("RF")));
    }

    #[test]
    fn replaces_partial_sightings_with_complete_targeted_evidence() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/12" xmlns:e="urn:example"><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram><Static><Property Occurrence="2"/><Property Occurrence="2"/><LoadProcedure><LdCtrlWriteProp AppliesTo="load"/><LdCtrlDeclarePropDesc ObjIdx="1" Writable="false"/></LoadProcedure></Static><Dynamic><ParameterSeparator UIHint="Text" e:UIHint="extension" HorizontalRuler="true" Access="Read"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
        let mut unknown = vec![UnknownConstruct {
            xpath: "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/Property".into(),
            kind: UnknownKind::Attribute,
            name: "Occurrence".into(),
            occurrences: 1,
            sample: Some("2".into()),
        }];

        reconcile_targeted_unknowns(xml, "fixture.xml", &mut unknown).unwrap();

        let occurrence = unknown
            .iter()
            .find(|item| item.name == "Occurrence")
            .unwrap();
        assert_eq!(occurrence.occurrences, 2);
        assert_eq!(occurrence.sample.as_deref(), Some("2"));
        for name in ["AppliesTo", "ObjIdx", "Writable", "UIHint"] {
            assert!(unknown.iter().any(|item| item.name == name), "{name}");
        }
        assert!(unknown.iter().any(|item| {
            item.kind == UnknownKind::Element && item.name == "LdCtrlDeclarePropDesc"
        }));
    }

    #[test]
    fn leaves_same_named_attributes_on_unrelated_elements_alone() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/12"><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram><Static><Other Occurrence="2" AppliesTo="other"/></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
        let retained = UnknownConstruct {
            xpath: "/KNX/Other".into(),
            kind: UnknownKind::Attribute,
            name: "Occurrence".into(),
            occurrences: 1,
            sample: Some("2".into()),
        };
        let mut unknown = vec![retained.clone()];

        reconcile_targeted_unknowns(xml, "fixture.xml", &mut unknown).unwrap();

        assert_eq!(unknown, vec![retained]);
    }

    #[test]
    fn does_not_merge_ambiguous_suffix_paths() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/12"><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram><Static><A><Property Occurrence="2"/></A><B><Property Occurrence="3"/></B></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
        let alias = UnknownConstruct {
            xpath: "/Static/Property".into(),
            kind: UnknownKind::Attribute,
            name: "Occurrence".into(),
            occurrences: 2,
            sample: Some("legacy-alias".into()),
        };
        let mut unknown = vec![alias.clone()];

        reconcile_targeted_unknowns(xml, "fixture.xml", &mut unknown).unwrap();

        assert!(unknown.contains(&alias));
        assert_eq!(unknown.len(), 3);
        assert!(unknown.iter().any(|item| {
            item.xpath.ends_with("/Static/A/Property")
                && item.occurrences == 1
                && item.sample.as_deref() == Some("2")
        }));
        assert!(unknown.iter().any(|item| {
            item.xpath.ends_with("/Static/B/Property")
                && item.occurrences == 1
                && item.sample.as_deref() == Some("3")
        }));
    }

    #[test]
    fn ignores_other_schemes_but_reports_extension_namespace_lookalikes() {
        let retained = UnknownConstruct {
            xpath: "/KNX/ParameterSeparator".into(),
            kind: UnknownKind::Attribute,
            name: "UIHint".into(),
            occurrences: 1,
            sample: Some("extension".into()),
        };
        let mut other_scheme = vec![retained.clone()];
        reconcile_targeted_unknowns(
            br#"<KNX xmlns="http://knx.org/xml/project/13"><Property Occurrence="2"/></KNX>"#,
            "fixture.xml",
            &mut other_scheme,
        )
        .unwrap();
        assert_eq!(other_scheme, vec![retained.clone()]);

        let mut extension = vec![
            retained.clone(),
            UnknownConstruct {
                xpath: "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static".into(),
                kind: UnknownKind::Element,
                name: "Property".into(),
                occurrences: 1,
                sample: None,
            },
            UnknownConstruct {
                xpath: "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/Property".into(),
                kind: UnknownKind::Attribute,
                name: "Occurrence".into(),
                occurrences: 1,
                sample: Some("2".into()),
            },
        ];
        reconcile_targeted_unknowns(
            br#"<KNX xmlns="http://knx.org/xml/project/12" xmlns:e="urn:example"><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram><Static><e:Property Occurrence="2"/><e:LdCtrlDeclarePropDesc ObjIdx="1"/></Static><Dynamic><ParameterSeparator e:UIHint="not-knx"/><e:ParameterSeparator UIHint="also-not-knx"/></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#,
            "fixture.xml",
            &mut extension,
        )
        .unwrap();
        assert!(extension.contains(&retained));
        for name in [
            "{urn:example}Property",
            "{urn:example}LdCtrlDeclarePropDesc",
            "{urn:example}ParameterSeparator",
        ] {
            assert!(extension.iter().any(|item| item.name == name), "{name}");
        }
        assert!(extension.iter().any(|item| {
            item.kind == UnknownKind::Attribute
                && item.name == "{urn:example}UIHint"
                && item.sample.as_deref() == Some("not-knx")
        }));
        assert!(extension.iter().any(|item| {
            item.xpath.ends_with("/e:Property")
                && item.name == "{urn:example}Property/@Occurrence"
                && item.sample.as_deref() == Some("2")
        }));
        assert!(!extension.iter().any(|item| {
            item.xpath.ends_with("/Static/Property")
                && item.name == "Occurrence"
                && item.sample.as_deref() == Some("2")
        }));
        assert!(!extension.iter().any(|item| {
            item.xpath.ends_with("/ApplicationProgram/Static") && item.name == "Property"
        }));
    }

    #[test]
    fn subtracts_only_foreign_occurrences_from_shared_parser_aliases() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/12" xmlns:e="urn:example"><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram><Static><Other Flag="yes"/><e:Other Flag="foreign"/></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
        let parent =
            "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static";
        let mut unknown = vec![
            UnknownConstruct {
                xpath: parent.into(),
                kind: UnknownKind::Element,
                name: "Other".into(),
                occurrences: 2,
                sample: None,
            },
            UnknownConstruct {
                xpath: format!("{parent}/Other"),
                kind: UnknownKind::Attribute,
                name: "Flag".into(),
                occurrences: 2,
                sample: Some("yes".into()),
            },
        ];

        reconcile_targeted_unknowns(xml, "fixture.xml", &mut unknown).unwrap();

        assert!(unknown.iter().any(|item| {
            item.kind == UnknownKind::Element && item.name == "Other" && item.occurrences == 1
        }));
        assert!(unknown.iter().any(|item| {
            item.kind == UnknownKind::Attribute && item.name == "Flag" && item.occurrences == 1
        }));
        assert!(unknown.iter().any(|item| {
            item.kind == UnknownKind::Element && item.name == "{urn:example}Other"
        }));
    }

    #[test]
    fn qualified_attribute_collision_preserves_the_unqualified_sighting_separately() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/12" xmlns:e="urn:example" xmlns:f="urn:second"><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram><Static><Other Flag="canonical" e:Flag="foreign" f:Flag="second"/></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
        let path = "/KNX/ManufacturerData/Manufacturer/ApplicationPrograms/ApplicationProgram/Static/Other";
        let mut unknown = vec![
            UnknownConstruct {
                xpath: path.into(),
                kind: UnknownKind::Attribute,
                name: "Flag".into(),
                occurrences: 1,
                sample: Some("canonical".into()),
            },
            UnknownConstruct {
                xpath: path.into(),
                kind: UnknownKind::Attribute,
                name: "e:Flag".into(),
                occurrences: 1,
                sample: Some("foreign".into()),
            },
            UnknownConstruct {
                xpath: path.into(),
                kind: UnknownKind::Attribute,
                name: "f:Flag".into(),
                occurrences: 1,
                sample: Some("second".into()),
            },
        ];

        reconcile_targeted_unknowns(xml, "fixture.xml", &mut unknown).unwrap();

        assert!(unknown.iter().any(|item| {
            item.xpath == path
                && item.name == "Flag"
                && item.occurrences == 1
                && item.sample.as_deref() == Some("canonical")
        }));
        assert!(!unknown
            .iter()
            .any(|item| matches!(item.name.as_str(), "e:Flag" | "f:Flag")));
        assert!(unknown.iter().any(|item| item.name == "{urn:example}Flag"));
        assert!(unknown.iter().any(|item| item.name == "{urn:second}Flag"));
    }

    #[test]
    fn project_namespace_prefixes_do_not_change_canonical_paths() {
        let xml = br#"<KNX xmlns="http://knx.org/xml/project/12" xmlns:p="http://knx.org/xml/project/12"><ManufacturerData><Manufacturer><ApplicationPrograms><ApplicationProgram><Dynamic><p:ParameterBlock><p:ParameterSeparator UIHint="Text"/></p:ParameterBlock></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;
        let mut unknown = Vec::new();

        reconcile_targeted_unknowns(xml, "fixture.xml", &mut unknown).unwrap();

        let ui_hint = unknown.iter().find(|item| item.name == "UIHint").unwrap();
        assert!(ui_hint
            .xpath
            .ends_with("/Dynamic/ParameterBlock/ParameterSeparator"));
        assert!(!ui_hint.xpath.contains(':'));
    }

    #[test]
    fn dynamic_alias_paths_follow_parser_state_not_physical_nesting() {
        let path = |names: &[&str]| {
            names
                .iter()
                .map(|name| PathElement {
                    local_name: (*name).into(),
                    rendered_name: (*name).into(),
                    in_project_namespace: true,
                    module_def_active: false,
                })
                .collect::<Vec<_>>()
        };
        let program = [
            "KNX",
            "ManufacturerData",
            "Manufacturer",
            "ApplicationPrograms",
            "ApplicationProgram",
        ];
        let mut wrapped = path(&program);
        wrapped.push(PathElement {
            local_name: "Wrapper".into(),
            rendered_name: "Wrapper".into(),
            in_project_namespace: true,
            module_def_active: false,
        });
        assert!(parser_alias_xpath(&wrapped, "Dynamic").ends_with("/Dynamic/Dynamic"));
        wrapped.push(PathElement {
            local_name: "Dynamic".into(),
            rendered_name: "Dynamic".into(),
            in_project_namespace: false,
            module_def_active: false,
        });
        wrapped.push(PathElement {
            local_name: "ParameterBlock".into(),
            rendered_name: "e:ParameterBlock".into(),
            in_project_namespace: false,
            module_def_active: false,
        });
        assert!(parser_alias_xpath(&wrapped, "ParameterSeparator")
            .ends_with("/Dynamic/ParameterSeparator"));

        let mut module = path(&program);
        for name in ["ModuleDefs", "ModuleDef", "Wrapper"] {
            module.push(PathElement {
                local_name: name.into(),
                rendered_name: name.into(),
                in_project_namespace: true,
                module_def_active: name != "ModuleDefs",
            });
        }
        assert!(parser_alias_xpath(&module, "Dynamic")
            .ends_with("/ModuleDefs/ModuleDef/Dynamic/Dynamic"));
    }

    #[test]
    fn budget_policy_admits_one_million_attribute_and_element_items() {
        let mut element = BytesStart::new("X");
        element.push_attribute(("f", "v"));
        let mut count = 0;
        let mut bytes = 0;
        for _ in 0..524_288 {
            charge_evidence_budget(&mut count, &mut bytes, &[], &element, "fixture.xml")
                .expect("bounded item policy must admit one million charged items");
        }
        assert_eq!(count, 1_048_576);
        assert!(bytes < MAX_EVIDENCE_BYTES);
    }

    #[test]
    fn budget_policy_admits_bounded_repeated_attribute_bytes_above_64_mib() {
        let value = "v".repeat(1024 * 1024);
        let mut element = BytesStart::new("X");
        element.push_attribute(("f", value.as_str()));
        let mut count = 0;
        let mut bytes = 0;
        for _ in 0..128 {
            charge_evidence_budget(&mut count, &mut bytes, &[], &element, "fixture.xml")
                .expect("bounded byte policy must admit repeated evidence above 64 MiB");
        }
        assert_eq!(count, 256);
        assert!(bytes > 128 * 1024 * 1024);
        assert!(bytes < MAX_EVIDENCE_BYTES);
    }

    #[test]
    fn budget_policy_item_boundary_includes_attributes_and_rejects_next_element() {
        let mut element = BytesStart::new("X");
        element.push_attribute(("f", "v"));
        let mut count = MAX_EVIDENCE_ITEMS - 2;
        let mut bytes = 0;
        charge_evidence_budget(&mut count, &mut bytes, &[], &element, "fixture.xml")
            .expect("the exact item ceiling is inclusive");
        assert_eq!(count, MAX_EVIDENCE_ITEMS);
        let error = charge_evidence_budget(
            &mut count,
            &mut bytes,
            &[],
            &BytesStart::new("X"),
            "fixture.xml",
        )
        .expect_err("the next element must fail even without attributes");
        assert!(error.to_string().contains("evidence exceeds item limit"));
    }

    #[test]
    fn budget_policy_byte_boundary_is_inclusive_and_rejects_next_path() {
        let element = BytesStart::new("X");
        let mut count = 0;
        // The empty ancestor path plus /X charges three bytes.
        let mut bytes = MAX_EVIDENCE_BYTES - 3;
        charge_evidence_budget(&mut count, &mut bytes, &[], &element, "fixture.xml")
            .expect("the exact estimated-byte ceiling is inclusive");
        assert_eq!(bytes, MAX_EVIDENCE_BYTES);
        let error = charge_evidence_budget(&mut count, &mut bytes, &[], &element, "fixture.xml")
            .expect_err("the next charged path must fail");
        assert!(error.to_string().contains("evidence exceeds byte limit"));
    }

    #[test]
    fn budget_policy_late_item_refusal_preserves_existing_evidence() {
        let mut xml = String::from(r#"<KNX xmlns="http://knx.org/xml/project/14">"#);
        for _ in 0..=MAX_EVIDENCE_ITEMS / 2 {
            xml.push_str(r#"<Property Occurrence="2"/>"#);
        }
        xml.push_str("</KNX>");
        let retained = UnknownConstruct {
            xpath: "/old".into(),
            kind: UnknownKind::Element,
            name: "Old".into(),
            occurrences: 1,
            sample: None,
        };
        let mut unknown = vec![retained.clone()];
        let error = reconcile_targeted_unknowns(xml.as_bytes(), "fixture.xml", &mut unknown)
            .expect_err("late item exhaustion must reject the complete scan");
        assert!(error.to_string().contains("evidence exceeds item limit"));
        assert_eq!(unknown, vec![retained]);
    }

    #[test]
    fn budget_policy_late_repeated_path_byte_refusal_preserves_existing_evidence() {
        let name = "N".repeat(128);
        let mut xml = String::from(r#"<KNX xmlns="http://knx.org/xml/project/14">"#);
        for _ in 0..128 {
            xml.push_str(&format!("<{name}>"));
        }
        for _ in 0..32_768 {
            xml.push_str("<Property/>");
        }
        for _ in 0..128 {
            xml.push_str(&format!("</{name}>"));
        }
        xml.push_str("</KNX>");
        let retained = UnknownConstruct {
            xpath: "/old".into(),
            kind: UnknownKind::Element,
            name: "Old".into(),
            occurrences: 1,
            sample: None,
        };
        let mut unknown = vec![retained.clone()];
        let error = reconcile_targeted_unknowns(xml.as_bytes(), "fixture.xml", &mut unknown)
            .expect_err("repeated paths must not bypass the estimated-byte ceiling");
        assert!(error.to_string().contains("evidence exceeds byte limit"));
        assert_eq!(unknown, vec![retained]);
    }

    #[test]
    fn rejects_excessive_evidence_nesting_without_publishing_partial_rows() {
        let mut xml = String::from(r#"<KNX xmlns="http://knx.org/xml/project/12">"#);
        for _ in 0..=MAX_EVIDENCE_DEPTH {
            xml.push_str("<Nested>");
        }
        for _ in 0..=MAX_EVIDENCE_DEPTH {
            xml.push_str("</Nested>");
        }
        xml.push_str("</KNX>");
        let retained = UnknownConstruct {
            xpath: "/old".into(),
            kind: UnknownKind::Element,
            name: "Old".into(),
            occurrences: 1,
            sample: None,
        };
        let mut unknown = vec![retained.clone()];

        let error = reconcile_targeted_unknowns(xml.as_bytes(), "fixture.xml", &mut unknown)
            .expect_err("excessive nesting must be rejected");

        assert!(error.to_string().contains("nesting exceeds evidence limit"));
        assert_eq!(unknown, vec![retained]);
    }

    #[test]
    fn evidence_item_budget_is_checked_before_scanner_state_is_published() {
        let element = BytesStart::new("Property");
        let mut count = MAX_EVIDENCE_ITEMS;
        let mut bytes = 0;

        let error = charge_evidence_budget(&mut count, &mut bytes, &[], &element, "fixture.xml")
            .expect_err("one item beyond the budget must fail");

        assert!(error.to_string().contains("evidence exceeds item limit"));
    }

    #[test]
    fn evidence_byte_budget_counts_repeated_paths_before_publication() {
        let element = BytesStart::new("Property");
        let mut count = 0;
        let mut bytes = MAX_EVIDENCE_BYTES;

        let error = charge_evidence_budget(&mut count, &mut bytes, &[], &element, "fixture.xml")
            .expect_err("one path beyond the byte budget must fail");

        assert!(error.to_string().contains("evidence exceeds byte limit"));
    }

    #[test]
    fn expanded_namespace_names_are_charged_to_the_byte_budget() {
        let reader = NsReader::from_reader(b"".as_slice());
        let element = BytesStart::new("Foreign");
        let identity = ElementIdentity {
            expanded_name: "{a-very-long-namespace}Foreign".into(),
            rendered_name: "e:Foreign".into(),
            extended_scheme: false,
        };
        let mut bytes = MAX_EVIDENCE_BYTES;

        let error =
            charge_expanded_name_budget(&reader, &mut bytes, &element, &identity, "fixture.xml")
                .expect_err("expanded names beyond the byte budget must fail");

        assert!(error.to_string().contains("evidence exceeds byte limit"));
    }
}
