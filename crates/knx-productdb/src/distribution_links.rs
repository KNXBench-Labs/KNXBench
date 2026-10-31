//! Resolves split ZIP declarations without collapsing original manufacturer or member identities.

use super::*;
use std::collections::{BTreeMap, HashMap};

type OriginalKey = (Option<u32>, u32);
fn key(c: &DistributionSourceRef) -> OriginalKey {
    (c.member_ordinal, c.location)
}

/// Local source scope takes precedence. Only dangling catalogue/hardware joins may use other
/// members with the same declared manufacturer and exact namespace. This is not identity proof.
pub(super) fn resolve_archive(
    report: &mut DistributionInspection,
    limits: &DistributionLimits,
) -> Result<()> {
    let namespaces = report
        .documents
        .iter()
        .map(|d| (d.member_ordinal, d.namespace.as_str()))
        .collect::<HashMap<_, _>>();
    // Indexed once: a linear lookup per metadata row would be quadratic in members.
    let mut documents = HashMap::new();
    for d in &report.documents {
        documents.entry(d.member_ordinal).or_insert(d);
    }
    let mut manufacturers = HashMap::new();
    let mut counts = HashMap::new();
    for metadata in &report.declared_metadata {
        if metadata.attribute != "RefId"
            || metadata.value.is_empty()
            || !documents
                .get(&metadata.source.member_ordinal)
                .and_then(|d| {
                    d.locations
                        .get(metadata.source.location.checked_sub(1)? as usize)
                        .and_then(|l| d.names.get(l.name as usize))
                })
                .is_some_and(|n| n.local == "Manufacturer")
        {
            continue;
        }
        let Some(&namespace) = namespaces.get(&metadata.source.member_ordinal) else {
            continue;
        };
        manufacturers.insert(key(&metadata.source), (namespace, metadata.value.as_str()));
        *counts
            .entry((
                metadata.source.member_ordinal,
                namespace,
                metadata.value.as_str(),
            ))
            .or_insert(0usize) += 1;
    }
    // Grouped by member, so skipping the reference's own member costs nothing per identity.
    type ByMember<'a> = BTreeMap<Option<u32>, Vec<&'a DistributionIdentity>>;
    let mut identities = HashMap::<(&str, &str, &str, &str), ByMember<'_>>::new();
    for identity in &report.identities {
        if identity.application_scope.is_some()
            || identity.declared_id.is_empty()
            || !matches!(
                identity.kind.as_str(),
                "Product" | "Hardware2Program" | "ApplicationProgram"
            )
        {
            continue;
        }
        if let Some(&(namespace, manufacturer)) =
            manufacturers.get(&key(&identity.manufacturer_source))
        {
            identities
                .entry((
                    namespace,
                    manufacturer,
                    identity.kind.as_str(),
                    identity.declared_id.as_str(),
                ))
                .or_default()
                .entry(identity.source.member_ordinal)
                .or_default()
                .push(identity);
        }
    }
    let mut references = report.references.len().saturating_add(
        report
            .references
            .iter()
            .map(|r| r.candidates.len())
            .sum::<usize>(),
    );
    if references > limits.max_references {
        return Err(limit("cross-member references"));
    }
    for reference in &mut report.references {
        if reference.state != DistributionReferenceState::Dangling {
            continue;
        }
        let Some(kind) = reference
            .target_kind
            .as_deref()
            .filter(|k| matches!(*k, "Product" | "Hardware2Program" | "ApplicationProgram"))
        else {
            continue;
        };
        let Some(&(namespace, manufacturer)) =
            manufacturers.get(&key(&reference.manufacturer_source))
        else {
            continue;
        };
        let Some(all) = identities.get(&(
            namespace,
            manufacturer,
            kind,
            reference.declared_target.as_str(),
        )) else {
            continue;
        };
        let mut ambiguous_scope = counts
            .get(&(reference.source.member_ordinal, namespace, manufacturer))
            .copied()
            .unwrap_or(0)
            > 1;
        for identity in all
            .iter()
            .filter(|(member, _)| **member != reference.source.member_ordinal)
            .flat_map(|(_, ids)| ids)
        {
            references = references
                .checked_add(1)
                .filter(|&n| n <= limits.max_references)
                .ok_or_else(|| limit("cross-member reference candidates"))?;
            ambiguous_scope |= counts
                .get(&(identity.source.member_ordinal, namespace, manufacturer))
                .copied()
                .unwrap_or(0)
                > 1;
            reference.candidates.push(identity.source);
        }
        if !reference.candidates.is_empty() {
            reference.resolution_basis =
                DistributionResolutionBasis::DeclaredManufacturerAcrossMembers;
            reference.state = if reference.candidates.len() == 1 && !ambiguous_scope {
                DistributionReferenceState::Resolved
            } else {
                DistributionReferenceState::Ambiguous
            };
        }
    }
    super::readers::output_limit(report, limits)
}
