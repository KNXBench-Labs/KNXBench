//! Display-shaped view of project Functions (ADR-0124): each Function with
//! its resolved location and its members' group addresses, in model order.
//! Read-only; edits go through `knx_core` commands.

use knx_core::{function::Function, CompletionStatus, Project};
use serde::Serialize;
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FunctionsView {
    /// In model order, which is the order of the source project.
    pub functions: Vec<FunctionNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FunctionNode {
    pub id: u32,
    pub installation_id: u8,
    pub location: FunctionLocationNode,
    pub name: String,
    /// The ETS function type reference as written (e.g. `"FT-1"`); not
    /// resolved to a master-data name.
    pub function_type: Option<String>,
    pub number: Option<String>,
    pub comment: Option<String>,
    pub description: Option<String>,
    /// `CompletionStatus` spelled as in the ETS schema (`"Editing"`, …).
    pub completion: String,
    pub members: Vec<FunctionMemberNode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FunctionLocationNode {
    pub building_part_id: u32,
    pub name: String,
    /// `BuildingPartType` as in [`crate::BuildingNode::kind`].
    pub kind: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct FunctionMemberNode {
    pub id: u32,
    pub group_address_id: u32,
    /// Formatted in the project's group-address style.
    pub address: String,
    pub group_address_name: String,
    /// The member's own name (ETS writes `""` when unset).
    pub name: String,
    pub role: Option<String>,
}

/// Builds the view. Functions whose references do not resolve cannot exist
/// in a valid project (`function::validate_functions`); if one is passed
/// anyway it is skipped rather than shown with invented data, and a member
/// whose address is gone is skipped the same way.
pub fn build_functions_view(project: &Project) -> FunctionsView {
    FunctionsView {
        functions: project
            .functions
            .iter()
            .filter_map(|function| build_function_node(project, function))
            .collect(),
    }
}

fn build_function_node(project: &Project, function: &Function) -> Option<FunctionNode> {
    let installation = project
        .installations
        .iter()
        .find(|i| i.buildings.iter().any(|b| b.id == function.location))?;
    let part = installation
        .buildings
        .iter()
        .find(|b| b.id == function.location)?;
    let style = project.info.group_address_style;
    let members = function
        .members
        .iter()
        .filter_map(|member| {
            let entry = installation
                .group_addresses
                .iter()
                .find(|g| g.id == member.group_address)?;
            Some(FunctionMemberNode {
                id: member.id.0,
                group_address_id: member.group_address.0,
                address: entry.address.format(style),
                group_address_name: entry.name.clone(),
                name: member.name.clone(),
                role: member.role.clone(),
            })
        })
        .collect();
    let details = &function.details;
    Some(FunctionNode {
        id: function.id.0,
        installation_id: installation.id.0,
        location: FunctionLocationNode {
            building_part_id: part.id.0,
            name: part.name.clone(),
            kind: crate::building_kind_str(part.kind).to_string(),
        },
        name: details.name.clone(),
        function_type: details.function_type.clone(),
        number: details.number.clone(),
        comment: details.comment.clone(),
        description: details.description.clone(),
        completion: completion_str(details.completion).to_string(),
        members,
    })
}

fn completion_str(status: CompletionStatus) -> &'static str {
    match status {
        CompletionStatus::Undefined => "Undefined",
        CompletionStatus::Editing => "Editing",
        CompletionStatus::FinishedDesign => "FinishedDesign",
        CompletionStatus::FinishedCommissioning => "FinishedCommissioning",
        CompletionStatus::Tested => "Tested",
        CompletionStatus::Locked => "Locked",
        CompletionStatus::Accepted => "Accepted",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::function::{FunctionDetails, FunctionMember};
    use knx_core::{
        BuildingPart, BuildingPartId, BuildingPartType, FunctionId, FunctionMemberId, GroupAddress,
        GroupAddressEntry, GroupAddressId, GroupAddressStyle, Installation, InstallationId,
        Language, SourceRef, Topology,
    };

    fn source() -> SourceRef {
        SourceRef {
            path: String::new(),
            ets_id: String::new(),
        }
    }

    fn part(id: u32, name: &str, kind: BuildingPartType, parent: Option<u32>) -> BuildingPart {
        BuildingPart {
            id: BuildingPartId(id),
            source: source(),
            name: name.into(),
            number: None,
            kind,
            default_line: None,
            completion: CompletionStatus::Undefined,
            children: vec![],
            devices: vec![],
            parent: parent.map(BuildingPartId),
        }
    }

    fn member(id: u32, ga: u32, role: &str) -> FunctionMember {
        FunctionMember {
            id: FunctionMemberId(id),
            source: source(),
            group_address: GroupAddressId(ga),
            name: String::new(),
            role: Some(role.into()),
        }
    }

    /// Two installations; the Function sits in a room nested under a floor
    /// of the second one.
    fn project() -> Project {
        let mut project = Project::new(Language("en".into()));
        project.info.group_address_style = GroupAddressStyle::ThreeLevel;
        for (id, ga_base) in [(0_u8, 10_u32), (1, 20)] {
            let mut floor = part(ga_base, "Ground floor", BuildingPartType::Floor, None);
            floor.children.push(BuildingPartId(ga_base + 1));
            project.installations.push(Installation {
                id: InstallationId(id),
                name: format!("Installation {id}"),
                default_line: None,
                multicast_address: None,
                completion: CompletionStatus::Undefined,
                topology: Topology {
                    areas: vec![],
                    lines: vec![],
                    unassigned: vec![],
                },
                buildings: vec![
                    floor,
                    part(
                        ga_base + 1,
                        "Kitchen",
                        BuildingPartType::Room,
                        Some(ga_base),
                    ),
                ],
                group_ranges: vec![],
                group_addresses: [
                    (ga_base, 0x0801_u16, "Light"),
                    (ga_base + 1, 0x0802, "Light status"),
                ]
                .into_iter()
                .map(|(ga, raw, name)| GroupAddressEntry {
                    id: GroupAddressId(ga),
                    source: source(),
                    name: name.into(),
                    address: GroupAddress::from_raw(raw),
                    central: false,
                    unfiltered: false,
                    range: None,
                    declared_dpt: Default::default(),
                })
                .collect(),
                parameters: vec![],
            });
        }
        project.functions.push(Function {
            id: FunctionId(5),
            source: source(),
            location: BuildingPartId(21),
            details: FunctionDetails {
                name: "Kitchen light".into(),
                function_type: Some("FT-1".into()),
                completion: CompletionStatus::FinishedDesign,
                ..Default::default()
            },
            members: vec![member(8, 21, "InfoOnOff"), member(9, 20, "SwitchOnOff")],
        });
        project
    }

    #[test]
    fn a_function_shows_its_nested_room_and_ordered_member_addresses() {
        let view = build_functions_view(&project());
        assert_eq!(
            view,
            FunctionsView {
                functions: vec![FunctionNode {
                    id: 5,
                    installation_id: 1,
                    location: FunctionLocationNode {
                        building_part_id: 21,
                        name: "Kitchen".into(),
                        kind: "Room".into(),
                    },
                    name: "Kitchen light".into(),
                    function_type: Some("FT-1".into()),
                    number: None,
                    comment: None,
                    description: None,
                    completion: "FinishedDesign".into(),
                    members: vec![
                        FunctionMemberNode {
                            id: 8,
                            group_address_id: 21,
                            address: "1/0/2".into(),
                            group_address_name: "Light status".into(),
                            name: String::new(),
                            role: Some("InfoOnOff".into()),
                        },
                        FunctionMemberNode {
                            id: 9,
                            group_address_id: 20,
                            address: "1/0/1".into(),
                            group_address_name: "Light".into(),
                            name: String::new(),
                            role: Some("SwitchOnOff".into()),
                        },
                    ],
                }],
            }
        );
    }

    #[test]
    fn a_member_resolves_only_inside_the_function_installation() {
        let mut project = project();
        // Group address 10 exists, but in installation 0.
        project.functions[0].members.push(member(10, 10, "Dim"));
        let view = build_functions_view(&project);
        assert_eq!(view.functions[0].members.len(), 2);
        project.functions[0].location = BuildingPartId(99);
        assert!(build_functions_view(&project).functions.is_empty());
    }
}
