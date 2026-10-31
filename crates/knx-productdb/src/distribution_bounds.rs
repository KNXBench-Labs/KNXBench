//! Bounds clonable Dynamic scalars and conservative module-expanded work before evaluation.

use super::{xml::Document, *};
use std::collections::{HashMap, HashSet};

/// Largest identity or value the Dynamic evaluator may clone per step; work is charged per copy.
const MAX_DYNAMIC_SCALAR_BYTES: usize = 4096;

/// This is resource admission over every declared branch, not branch execution or initialization.
/// Existing evaluator depth/work/activation budgets remain in force after admission.
pub(super) fn admit_dynamic(
    doc: &Document<'_>,
    application: usize,
    included: &HashSet<usize>,
    fallbacks: &[DistributionParameterFallback],
    max_work_bytes: usize,
) -> Result<()> {
    let mut scalar = 0usize;
    for fallback in fallbacks {
        scalar = scalar.max(fallback.value.as_ref().map_or(0, String::len));
    }
    let definitions = included
        .iter()
        .filter(|&&i| doc.nodes[i].local == "ModuleDef" && doc.declaration(i))
        .filter_map(|&i| doc.nodes[i].attr("Id").map(|name| (name, i)))
        .collect::<HashMap<_, _>>();
    let mut groups = HashMap::<usize, (usize, Vec<usize>)>::new();
    for &id in included {
        let node = &doc.nodes[id];
        let mut next = Some(id);
        let mut dynamic = false;
        while let Some(i) = next {
            if i == application {
                break;
            }
            if doc.nodes[i].local == "Dynamic" {
                dynamic = true;
                break;
            }
            next = doc.nodes[i].parent;
        }
        if !dynamic {
            continue;
        }
        let owner = doc.entity_scope(id).unwrap_or(application);
        let group = groups.entry(owner).or_default();
        group.0 += 1;
        for attr in &node.attrs {
            if matches!(
                attr.raw.as_str(),
                "Value" | "test" | "Id" | "RefId" | "ParamRefId" | "TextParameterRefId"
            ) {
                scalar = scalar.max(attr.value.len());
            }
        }
        if node.local == "Module" {
            if let Some(target) = node
                .attr("RefId")
                .and_then(|id| definitions.get(&id).copied())
            {
                group.1.push(target);
            }
        }
    }
    // Separate from XML text retention: one scalar can be cloned at every evaluation step.
    if scalar > MAX_DYNAMIC_SCALAR_BYTES {
        return Err(limit(&format!(
            "Dynamic scalar bytes ({MAX_DYNAMIC_SCALAR_BYTES})"
        )));
    }
    let cap = crate::dynamic::evaluate::MAX_EVALUATION_WORK;
    let mut costs = groups
        .iter()
        .map(|(&id, (base, _))| (id, *base))
        .collect::<HashMap<_, _>>();
    // Bounded dynamic programming, including cycles; no recursive source expansion or code runs.
    for _ in 0..crate::dynamic::evaluate::MAX_MODULE_NESTING_DEPTH {
        let next = groups
            .iter()
            .map(|(&id, (base, edges))| {
                let total = edges.iter().fold(*base, |sum, target| {
                    sum.saturating_add(costs.get(target).copied().unwrap_or(0))
                        .min(cap)
                });
                (id, total)
            })
            .collect();
        costs = next;
    }
    let units = costs.get(&application).copied().unwrap_or(0);
    // This quantity bounds normalized UTF-8 scalar payload copied over admitted work units.
    // JSON escaping and fixed evaluator structures are different quantities: the former is
    // counted by the serializer, the latter by the evaluator's existing work/activation caps.
    let copies = units
        .checked_mul(scalar)
        .ok_or_else(|| limit("Dynamic scalar-copy bytes"))?;
    if copies > max_work_bytes {
        Err(limit(&format!(
            "Dynamic scalar-copy bytes: {copies} > {max_work_bytes}"
        )))
    } else {
        Ok(())
    }
}
