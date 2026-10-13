//! Maps schema-23 Functions after every installation is mapped, so each
//! location and group address already has its internal id (ADR-0124).

use knx_core::function::{location_installation, Function, FunctionDetails, FunctionMember};
use std::collections::BTreeMap;

use knx_core::{IdAllocationError, Project, SourceRef};

use super::{
    required_completion, resolve_one, EntityCounts, IdTables, MapProblem, MapProblemDetail,
};
use crate::source::{SourceBuildingPart, SourceDocument};

/// Walks the building parts in the same document order as pass 2 and
/// claims each part's id the same way (`IdTable::own` is single-use, so a
/// local occurrence count over `all` stands in for it): a repeated `Space`
/// id still resolves to the element that lists the Function.
struct Walk<'a> {
    source_path: &'a str,
    tables: &'a IdTables,
    seen: BTreeMap<String, usize>,
}

pub(super) fn map_functions(
    document: &SourceDocument,
    source_path: &str,
    tables: &IdTables,
    project: &mut Project,
    problems: &mut Vec<MapProblem>,
    counts: &mut EntityCounts,
) -> Result<(), IdAllocationError> {
    let mut walk = Walk {
        source_path,
        tables,
        seen: BTreeMap::new(),
    };
    for installation in &document.installations {
        for part in &installation.buildings {
            map_part(&mut walk, part, project, problems, counts)?;
        }
    }
    Ok(())
}

fn map_part(
    walk: &mut Walk<'_>,
    part: &SourceBuildingPart,
    project: &mut Project,
    problems: &mut Vec<MapProblem>,
    counts: &mut EntityCounts,
) -> Result<(), IdAllocationError> {
    let occurrence = walk.seen.entry(part.id.clone()).or_default();
    let location = walk.tables.building_parts.all(&part.id)[*occurrence];
    *occurrence += 1;
    let (source_path, tables) = (walk.source_path, walk.tables);
    for source in &part.functions {
        counts.functions.read += 1;
        let xpath = crate::xpath::function(&source.id);
        let scope = location_installation(project, location);
        let id = project.ids.next_function_id()?;
        let mut members = Vec::new();
        for member in &source.members {
            counts.function_members.read += 1;
            let Some(address) = resolve_one(
                &member.ref_id,
                &tables.group_addresses,
                "GroupAddressRef/@RefId",
                &xpath,
                problems,
            ) else {
                continue;
            };
            if scope.is_none()
                || knx_core::function::group_address_installation(project, address) != scope
            {
                problems.push(MapProblem {
                    xpath: xpath.clone(),
                    detail: MapProblemDetail::CrossInstallationReference {
                        kind: "GroupAddressRef/@RefId",
                        target: member.ref_id.clone(),
                    },
                });
                continue;
            }
            members.push(FunctionMember {
                id: project.ids.next_function_member_id()?,
                source: SourceRef {
                    path: source_path.to_string(),
                    ets_id: member.id.clone(),
                },
                group_address: address,
                name: member.name.clone().unwrap_or_default(),
                role: member.role.clone(),
            });
            counts.function_members.mapped += 1;
        }
        project.functions.push(Function {
            id,
            source: SourceRef {
                path: source_path.to_string(),
                ets_id: source.id.clone(),
            },
            location,
            details: FunctionDetails {
                name: source.name.clone().unwrap_or_default(),
                function_type: source.function_type.clone(),
                number: source.number.clone(),
                comment: source.comment.clone(),
                description: source.description.clone(),
                completion: required_completion(&source.completion_status, &xpath, problems),
            },
            members,
        });
        counts.functions.mapped += 1;
    }
    for child in &part.children {
        map_part(walk, child, project, problems, counts)?;
    }
    Ok(())
}
