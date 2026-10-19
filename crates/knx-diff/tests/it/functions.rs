//! Per-Function diff table: matching, field changes and visible leftovers.
//!
//! ADR-0124: two projects that differ only in their Functions never diff
//! as equal; internal ids alone are no difference. Since the per-Function
//! table (MODEL-07, 17 October 2026) each installation lists its Functions as
//! added, removed, changed or ambiguous, matched by ETS id first and by
//! building-part path plus name second.
use knx_core::function::{Function, FunctionDetails, FunctionMember};
use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, CompletionStatus, FunctionId, FunctionMemberId,
    GroupAddress, GroupAddressEntry, GroupAddressId, Installation, InstallationId, Language,
    Project, SourceRef, Topology,
};
use knx_diff::{diff_projects, EntityStatus, FunctionKey, MatchKind, ProjectDiff};

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
        buildings: vec![
            BuildingPart {
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
            },
            BuildingPart {
                id: BuildingPartId(2),
                source: source("P-0001-0_BP-2"),
                name: "Hall".into(),
                number: None,
                kind: BuildingPartType::Room,
                default_line: None,
                completion: CompletionStatus::Undefined,
                children: vec![],
                devices: vec![],
                parent: None,
            },
        ],
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

fn functions(diff: &ProjectDiff) -> &knx_diff::EntityTable<FunctionKey, knx_diff::FunctionFields> {
    assert_eq!(diff.installations.len(), 1);
    &diff.installations[0].functions
}

fn unsourced() -> SourceRef {
    SourceRef {
        path: String::new(),
        ets_id: String::new(),
    }
}

/// The same Function as `left`'s, created in KNXBench (no source id).
fn knxbench_made(mut project: Project) -> Project {
    project.functions[0].source = unsourced();
    for member in &mut project.functions[0].members {
        member.source = unsourced();
    }
    project
}

#[test]
fn equal_functions_with_different_internal_ids_do_not_diff() {
    assert!(diff_projects(&project((1, 1)), &project((7, 40))).is_empty());
}

#[test]
fn placed_functions_are_a_per_function_table_not_a_project_info_blob() {
    let left = project((1, 1));
    let mut right = left.clone();
    right.functions[0].members[1].role = Some("Dim".into());
    let diff = diff_projects(&left, &right);
    assert!(!diff.is_empty());
    assert!(
        diff.info_changes
            .iter()
            .all(|c| !c.field.contains("unctions")),
        "{:?}",
        diff.info_changes
    );
    let table = functions(&diff);
    assert!(table.added.is_empty() && table.removed.is_empty() && table.ambiguous.is_empty());
    assert_eq!(table.changed.len(), 1);
    let change = &table.changed[0];
    assert_eq!(change.matched_by, MatchKind::EtsId);
    assert_eq!(change.key.ets_id.as_deref(), Some("P-0001-0_F-1"));
    assert_eq!(change.key.name, "Light <script>");
    assert_eq!(change.key.location, vec!["Living <room>".to_string()]);
    assert_eq!(change.changed_fields, vec!["members"]);
    let fields = change.field_changes();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].left, "1/0/1 [SwitchOnOff], 1/0/2 [InfoOnOff]");
    assert_eq!(fields[0].right, "1/0/1 [SwitchOnOff], 1/0/2 [Dim]");
}

#[test]
fn each_changed_attribute_is_named_with_plain_text_values() {
    let left = project((1, 1));
    let mut right = left.clone();
    right.functions[0].details.comment = None;
    right.functions[0].details.function_type = Some("FT-7".into());
    right.functions[0].details.completion = CompletionStatus::Tested;
    right.functions[0].location = BuildingPartId(2);
    right.functions[0].members.reverse();
    let diff = diff_projects(&left, &right);
    let change = &functions(&diff).changed[0];
    assert_eq!(
        change.changed_fields,
        vec![
            "function_type",
            "comment",
            "completion",
            "location",
            "members"
        ]
    );
    let fields = change.field_changes();
    let by_name = |name: &str| fields.iter().find(|f| f.field == name).unwrap();
    assert_eq!(by_name("comment").left, "first\nsecond");
    assert_eq!(by_name("comment").right, "-");
    assert_eq!(by_name("function_type").right, "FT-7");
    assert_eq!(by_name("location").left, "Living <room>");
    assert_eq!(by_name("location").right, "Hall");
    assert_eq!(by_name("completion").left, "Editing");
}

#[test]
fn added_and_removed_functions_are_listed_with_their_fields() {
    let left = project((1, 1));
    let mut right = left.clone();
    right.functions.clear();
    let removed = diff_projects(&left, &right);
    let table = functions(&removed);
    assert_eq!(table.removed.len(), 1);
    assert_eq!(table.removed[0].0.name, "Light <script>");
    assert_eq!(table.removed[0].1.members.len(), 2);
    let added = diff_projects(&right, &left);
    assert_eq!(functions(&added).added.len(), 1);
    assert!(functions(&added).removed.is_empty());
}

#[test]
fn knxbench_made_functions_match_by_room_and_name() {
    let left = knxbench_made(project((1, 1)));
    let mut right = knxbench_made(project((5, 9)));
    assert!(diff_projects(&left, &right).is_empty());
    right.functions[0].details.number = Some("7".into());
    let diff = diff_projects(&left, &right);
    let change = &functions(&diff).changed[0];
    assert_eq!(change.matched_by, MatchKind::NaturalKey);
    assert_eq!(change.key.ets_id, None);
    assert_eq!(change.changed_fields, vec!["number"]);
    // Without a source id a rename cannot be told from a replacement.
    let mut renamed = left.clone();
    renamed.functions[0].details.name = "Ceiling".into();
    let diff = diff_projects(&left, &renamed);
    assert_eq!(functions(&diff).removed.len(), 1);
    assert_eq!(functions(&diff).added.len(), 1);
}

#[test]
fn same_named_functions_in_different_rooms_match_their_own_room() {
    let mut left = knxbench_made(project((1, 1)));
    let mut hall = left.functions[0].clone();
    hall.id = FunctionId(2);
    hall.location = BuildingPartId(2);
    for (i, member) in hall.members.iter_mut().enumerate() {
        member.id = FunctionMemberId(10 + i as u32);
    }
    left.functions.push(hall);
    let mut right = left.clone();
    right.functions[1].details.comment = Some("hall only".into());
    let diff = diff_projects(&left, &right);
    let table = functions(&diff);
    assert!(table.ambiguous.is_empty(), "{:?}", table.ambiguous);
    assert_eq!(table.changed.len(), 1);
    assert_eq!(table.changed[0].key.location, vec!["Hall".to_string()]);
    assert_eq!(table.changed[0].changed_fields, vec!["comment"]);
}

#[test]
fn twin_functions_without_source_ids_are_ambiguous_not_guessed() {
    let mut left = knxbench_made(project((1, 1)));
    let mut twin = left.functions[0].clone();
    twin.id = FunctionId(2);
    for (i, member) in twin.members.iter_mut().enumerate() {
        member.id = FunctionMemberId(10 + i as u32);
    }
    left.functions.push(twin);
    let mut right = left.clone();
    right.functions[1].details.comment = Some("changed".into());
    let diff = diff_projects(&left, &right);
    let table = functions(&diff);
    assert_eq!(table.ambiguous.len(), 1);
    assert_eq!(table.ambiguous[0].left_candidates, 2);
    assert_eq!(table.ambiguous[0].right_candidates, 2);
    assert!(table.changed.is_empty());
    assert!(!diff.is_empty());
}

#[test]
fn a_different_group_address_notation_alone_changes_no_function() {
    let left = project((1, 1));
    let mut right = left.clone();
    right.info.group_address_style = knx_core::GroupAddressStyle::TwoLevel;
    let diff = diff_projects(&left, &right);
    assert!(
        functions(&diff).changed.is_empty(),
        "{:?}",
        functions(&diff).changed
    );
}

#[test]
fn a_function_outside_every_installation_stays_visible() {
    let left = project((1, 1));
    let mut right = left.clone();
    right.functions[0].location = BuildingPartId(99);
    let diff = diff_projects(&left, &right);
    assert!(!diff.is_empty());
    let change = diff
        .info_changes
        .iter()
        .find(|c| c.field == "unplaced_functions")
        .expect("an unplaced Function is reported");
    assert_eq!(change.left, "");
    assert!(change.right.contains("Light <script>"));
    assert_eq!(functions(&diff).removed.len(), 1);
}

#[test]
fn an_added_installation_accounts_for_its_functions() {
    let left = project((1, 1));
    let mut right = left.clone();
    right.installations[0].id = InstallationId(1);
    let diff = diff_projects(&left, &right);
    let statuses: Vec<_> = diff
        .installations
        .iter()
        .map(|i| i.status.clone())
        .collect();
    assert_eq!(statuses, vec![EntityStatus::Removed, EntityStatus::Added]);
    assert!(diff
        .installations
        .iter()
        .all(|i| i.functions.added.is_empty() && i.functions.removed.is_empty()));
}
