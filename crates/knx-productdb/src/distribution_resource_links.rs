//! Resolves only canonical resource consumers in original namespace and manufacturer scopes.

use super::{xml::Document, *};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DistributionResourceConsumer {
    pub source: DistributionSourceRef,
    pub manufacturer_source: DistributionSourceRef,
    pub application_source: Option<DistributionSourceRef>,
    pub namespace: String,
    pub declared_manufacturer_id: Option<String>,
    pub attribute: String,
    pub role: String,
    pub declared_baggage_ref: Option<String>,
    pub reference_state: Option<DistributionReferenceState>,
    pub declaration_candidates: Vec<DistributionSourceRef>,
    pub reason: String,
}

fn below(doc: &Document<'_>, mut id: usize, scope: usize, path: &[&str]) -> bool {
    for &name in path.iter().rev() {
        let n = &doc.nodes[id];
        if n.local != name || n.ns.as_ref() != doc.namespace {
            return false;
        }
        let Some(parent) = n.parent else {
            return false;
        };
        id = parent;
    }
    id == scope
}

pub(super) fn collect(doc: &Document<'_>, report: &mut DistributionInspection) -> Result<()> {
    for (i, n) in doc.nodes.iter().enumerate() {
        let Some(app) = doc.application(i) else {
            continue;
        };
        let Some(manufacturer) = doc.manufacturer(i) else {
            continue;
        };
        let mut roles = Vec::new();
        if i == app {
            for (attr, role) in [
                ("ContextHelpFile", "declaredContextHelp"),
                ("IconFile", "declaredIconArchive"),
            ] {
                if n.attr(attr).is_some() {
                    roles.push((attr, role));
                }
            }
        } else if n.local == "TypePicture"
            && n.parent.is_some_and(|p| doc.declaration(p))
            && doc.entity_scope(i).is_some_and(|scope| {
                below(
                    doc,
                    i,
                    scope,
                    &["Static", "ParameterTypes", "ParameterType", "TypePicture"],
                )
            })
        {
            roles.push(("RefId", "rasterPicture"));
        } else if below(doc, i, app, &["Static", "Extension", "Baggage"]) {
            roles.push(("RefId", "resourceReference"));
        }
        for (attribute, role) in roles {
            if n.attr(attribute)
                .is_some_and(|v| v.len() > MAX_IDENTITY_BYTES)
            {
                return Err(limit("resource reference bytes"));
            }
            doc.push(
                &mut report.resource_consumers,
                DistributionResourceConsumer {
                    source: doc.coordinate(i)?,
                    manufacturer_source: doc.coordinate(manufacturer)?,
                    application_source: Some(doc.coordinate(app)?),
                    namespace: doc.namespace.clone(),
                    declared_manufacturer_id: doc.nodes[manufacturer].attr("RefId"),
                    attribute: attribute.into(),
                    role: role.into(),
                    declared_baggage_ref: n.attr(attribute),
                    reference_state: None,
                    declaration_candidates: Vec::new(),
                    reason: "pending".into(),
                },
            )?;
        }
    }
    Ok(())
}

/// Units already charged by ordinary references, dependency endpoints and candidate targets.
fn charged_references(report: &DistributionInspection) -> usize {
    let units = |n: usize| 1 + n;
    let ordinary = report.references.iter().map(|r| units(r.candidates.len()));
    let endpoints = report
        .dependency_evidence
        .iter()
        .flat_map(|d| &d.endpoints)
        .map(|e| units(e.candidates.len()));
    let targets = report
        .source_candidates
        .iter()
        .flat_map(|c| [&c.parameter, &c.parameter_type])
        .flatten()
        .map(|t| units(t.candidates.len()));
    ordinary
        .chain(endpoints)
        .chain(targets)
        .fold(0, usize::saturating_add)
}

pub(super) fn resolve(
    report: &mut DistributionInspection,
    limits: &DistributionLimits,
) -> Result<()> {
    let mut local = HashMap::<(DistributionSourceRef, &str), Vec<usize>>::new();
    type ByMember = std::collections::BTreeMap<Option<u32>, Vec<usize>>;
    let mut cross = HashMap::<(&str, &str, &str), ByMember>::new();
    for (i, d) in report.resource_declarations.iter().enumerate() {
        if let Some(id) = d.id.as_deref().filter(|s| !s.is_empty()) {
            local
                .entry((d.manufacturer_source, id))
                .or_default()
                .push(i);
            if let Some(manufacturer) = d
                .declared_manufacturer_id
                .as_deref()
                .filter(|s| !s.is_empty())
            {
                cross
                    .entry((&d.namespace, manufacturer, id))
                    .or_default()
                    .entry(d.source.member_ordinal)
                    .or_default()
                    .push(i);
            }
        }
    }
    let documents = report
        .documents
        .iter()
        .map(|d| (d.member_ordinal, d))
        .collect::<HashMap<_, _>>();
    let mut scopes = HashMap::new();
    for m in report
        .declared_metadata
        .iter()
        .filter(|m| m.attribute == "RefId")
    {
        let Some(doc) = documents.get(&m.source.member_ordinal) else {
            continue;
        };
        let mut current = m.source.location.checked_sub(1);
        let mut canonical = true;
        for expected in ["Manufacturer", "ManufacturerData", "KNX"] {
            let Some(location) = current.and_then(|i| doc.locations.get(i as usize)) else {
                canonical = false;
                break;
            };
            let Some(name) = doc.names.get(location.name as usize) else {
                canonical = false;
                break;
            };
            if name.namespace != doc.namespace || name.local != expected {
                canonical = false;
                break;
            }
            current = location.parent;
        }
        if canonical && current.is_none() {
            *scopes
                .entry((
                    m.source.member_ordinal,
                    doc.namespace.as_str(),
                    m.value.as_str(),
                ))
                .or_insert(0usize) += 1;
        }
    }
    let mut references = charged_references(report);
    let mut bytes = super::provenance::json_size(report, limits.max_report_bytes)?;
    let mut records = super::readers::output_count(report);
    for c in &mut report.resource_consumers {
        let old = super::provenance::json_size(c, limits.max_report_bytes)?;
        let Some(raw) = c.declared_baggage_ref.as_deref() else {
            c.reason = "missingReference".into();
            continue;
        };
        references = references.saturating_add(1);
        if references > limits.max_references {
            return Err(limit("resource references"));
        }
        let mut ambiguous_scope = false;
        let across_members = c
            .declared_manufacturer_id
            .as_deref()
            .and_then(|m| cross.get(&(c.namespace.as_str(), m, raw)));
        let candidates = local
            .get(&(c.manufacturer_source, raw))
            .into_iter()
            .flatten()
            .chain(across_members.into_iter().flat_map(|members| {
                members
                    .iter()
                    .filter(|(member, _)| **member != c.source.member_ordinal)
                    .flat_map(|(_, rows)| rows)
            }));
        for &i in candidates {
            let d = &report.resource_declarations[i];
            let same_scope = d.manufacturer_source == c.manufacturer_source;
            let across = d.source.member_ordinal != c.source.member_ordinal
                && c.declared_manufacturer_id.as_deref().is_some_and(|m| {
                    !m.is_empty() && d.declared_manufacturer_id.as_deref() == Some(m)
                });
            if !same_scope && !across {
                continue;
            }
            if across {
                ambiguous_scope |= scopes
                    .get(&(
                        c.source.member_ordinal,
                        c.namespace.as_str(),
                        c.declared_manufacturer_id.as_deref().unwrap_or(""),
                    ))
                    .copied()
                    .unwrap_or(0)
                    > 1
                    || scopes
                        .get(&(
                            d.source.member_ordinal,
                            d.namespace.as_str(),
                            d.declared_manufacturer_id.as_deref().unwrap_or(""),
                        ))
                        .copied()
                        .unwrap_or(0)
                        > 1;
            }
            references = references.saturating_add(1);
            if references > limits.max_references {
                return Err(limit("resource reference candidates"));
            }
            records = records.saturating_add(1);
            if records > limits.max_output_records {
                return Err(limit("resource output records"));
            }
            c.declaration_candidates.push(d.source);
        }
        c.declaration_candidates
            .sort_unstable_by_key(|s| (s.member_ordinal, s.location));
        let (state, reason) = match c.declaration_candidates.len() {
            n if n > 0 && ambiguous_scope => (
                DistributionReferenceState::Ambiguous,
                "ambiguousManufacturerScope",
            ),
            0 => (DistributionReferenceState::Dangling, "missingDeclaration"),
            1 => (
                DistributionReferenceState::Resolved,
                "uniqueDeclaredResource",
            ),
            _ => (
                DistributionReferenceState::Ambiguous,
                "ambiguousDeclaration",
            ),
        };
        c.reference_state = Some(state);
        c.reason = reason.into();
        let new = super::provenance::json_size(c, limits.max_report_bytes)?;
        bytes = bytes
            .saturating_sub(old)
            .checked_add(new)
            .filter(|&n| n <= limits.max_report_bytes)
            .ok_or_else(|| limit("serialized report bytes"))?;
    }
    Ok(())
}
