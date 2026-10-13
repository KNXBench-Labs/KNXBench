//! ADR-0124: two projects that differ only in their Functions never diff
//! as equal; internal ids alone are no difference.
use knx_core::function::{Function, FunctionDetails, FunctionMember};
use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, CompletionStatus, FunctionId, FunctionMemberId,
    GroupAddress, GroupAddressEntry, GroupAddressId, Installation, InstallationId, Language,
    Project, SourceRef, Topology,
};
use knx_diff::diff_projects;

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: "P-0001/0.xml".into(),
        ets_id: id.into(),
    }
}

/// One installation with a room, two group addresses and one Function.
fn project(function_ids: (u32, u32)) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Home".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Undefined,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![BuildingPart {
            id: BuildingPartId(1),
            source: source("P-0001-0_BP-1"),
            name: "Living <room>".into(),
            number: None,
            kind: BuildingPartType::Room,
            default_line: None,
            completion: CompletionStatus::Undefined,
            children: vec![],
            devices: vec![],
            parent: None,
        }],
        group_ranges: vec![],
        group_addresses: [(1, 0x0801, "Switch"), (2, 0x0802, "Status")]
            .into_iter()
            .map(|(id, raw, name)| GroupAddressEntry {
                id: GroupAddressId(id),
                source: source(&format!("P-0001-0_GA-{id}")),
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
    let (function, first_member) = function_ids;
    project.functions.push(Function {
        id: FunctionId(function),
        source: source("P-0001-0_F-1"),
        location: BuildingPartId(1),
        details: FunctionDetails {
            name: "Light <script>".into(),
            function_type: Some("FT-1".into()),
            number: None,
            comment: Some("first\nsecond".into()),
            description: None,
            completion: CompletionStatus::Editing,
        },
        members: vec![
            FunctionMember {
                id: FunctionMemberId(first_member),
                source: source("P-0001-0_GF-1"),
                group_address: GroupAddressId(1),
                name: String::new(),
                role: Some("SwitchOnOff".into()),
            },
            FunctionMember {
                id: FunctionMemberId(first_member + 1),
                source: source("P-0001-0_GF-2"),
                group_address: GroupAddressId(2),
                name: String::new(),
                role: Some("InfoOnOff".into()),
            },
        ],
    });
    project
}

#[test]
fn equal_functions_with_different_internal_ids_do_not_diff() {
    assert!(diff_projects(&project((1, 1)), &project((7, 40))).is_empty());
}

#[test]
fn a_changed_role_order_location_or_text_is_a_visible_functions_change() {
    let left = project((1, 1));
    let mut role = left.clone();
    role.functions[0].members[1].role = Some("Dim".into());
    let mut order = left.clone();
    order.functions[0].members.reverse();
    let mut text = left.clone();
    text.functions[0].details.comment = None;
    let mut gone = left.clone();
    gone.functions.clear();
    for right in [role, order, text, gone] {
        let diff = diff_projects(&left, &right);
        assert!(!diff.is_empty());
        let change = diff
            .info_changes
            .iter()
            .find(|c| c.field == "functions")
            .expect("a functions change");
        assert!(change.left.contains("Light <script>"));
        assert!(change.left.contains("[SwitchOnOff]"));
    }
}
