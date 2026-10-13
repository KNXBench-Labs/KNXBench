//! MODEL-07 / ADR-0124: project Functions as a core model.
//!
//! A Function is placed in one building part, lists group addresses of that
//! part's installation in order, and owns nothing else. Every command is
//! undoable, refuses before mutating, and no command connects two
//! installations. Deleting a referenced group address or building part is
//! refused rather than cascading.

use knx_core::function::{
    validate_functions, Function, FunctionDetails, FunctionError, FunctionInvariantError,
    FunctionMember, MAX_FUNCTION_SHORT_TEXT_CODEPOINTS,
};
use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, Command, CommandError, CommandStack,
    CompletionStatus, FunctionId, FunctionMemberId, GroupAddress, GroupAddressEntry,
    GroupAddressId, IdAllocators, IdKind, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};

fn native() -> SourceRef {
    SourceRef {
        path: String::new(),
        ets_id: String::new(),
    }
}

fn ga(id: u32, raw: u16) -> GroupAddressEntry {
    GroupAddressEntry {
        id: GroupAddressId(id),
        source: native(),
        name: format!("G{id}"),
        address: GroupAddress::from_raw(raw),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Default::default(),
    }
}

fn part(id: u32) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: native(),
        name: format!("Room {id}"),
        number: None,
        kind: BuildingPartType::Room,
        default_line: None,
        completion: CompletionStatus::Undefined,
        children: vec![],
        devices: vec![],
        parent: None,
    }
}

fn installation(id: u8) -> Installation {
    Installation {
        id: InstallationId(id),
        name: format!("I{id}"),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::Undefined,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    }
}

/// Installation 0: rooms 1 and 2, group addresses 1–3.
/// Installation 1: room 3, group address 4.
fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    let mut home = installation(0);
    home.buildings.extend([part(1), part(2)]);
    home.group_addresses.extend([ga(1, 1), ga(2, 2), ga(3, 3)]);
    let mut garage = installation(1);
    garage.buildings.push(part(3));
    garage.group_addresses.push(ga(4, 4));
    project.installations.extend([home, garage]);
    project
}

fn details(name: &str) -> FunctionDetails {
    FunctionDetails {
        name: name.into(),
        ..Default::default()
    }
}

fn member(id: u32, address: u32, role: Option<&str>) -> FunctionMember {
    FunctionMember {
        id: FunctionMemberId(id),
        source: native(),
        group_address: GroupAddressId(address),
        name: String::new(),
        role: role.map(Into::into),
    }
}

fn function(id: u32, location: u32, members: Vec<FunctionMember>) -> Function {
    Function {
        id: FunctionId(id),
        source: native(),
        location: BuildingPartId(location),
        details: details("Living light"),
        members,
    }
}

/// Applies, undoes and redoes `command` through the stack; the project must
/// return exactly to its state before and after.
fn round_trip(project: &mut Project, command: Command) {
    let before = project.clone();
    let mut stack = CommandStack::new();
    stack.do_command(project, command).unwrap();
    let after = project.clone();
    stack.undo(project).unwrap();
    assert_eq!(*project, before, "undo restores the exact project");
    stack.redo(project).unwrap();
    assert_eq!(*project, after, "redo repeats the exact change");
}

fn refused(project: &Project, command: Command) -> CommandError {
    let mut candidate = project.clone();
    let error = command.apply(&mut candidate).unwrap_err();
    assert_eq!(&candidate, project, "a refused command changes nothing");
    error
}

fn with_light() -> Project {
    let mut project = project();
    Command::CreateFunction {
        function: function(
            1,
            1,
            vec![member(1, 1, Some("SwitchOnOff")), member(2, 2, None)],
        ),
    }
    .apply(&mut project)
    .unwrap();
    project
}

#[test]
fn a_function_is_created_in_its_room_with_ordered_members_and_undone_exactly() {
    let mut project = project();
    round_trip(
        &mut project,
        Command::CreateFunction {
            function: function(
                1,
                1,
                vec![member(1, 2, Some("InfoOnOff")), member(2, 1, None)],
            ),
        },
    );
    let created = &project.functions[0];
    assert_eq!(created.location, BuildingPartId(1));
    let order: Vec<_> = created.members.iter().map(|m| m.group_address).collect();
    assert_eq!(order, [GroupAddressId(2), GroupAddressId(1)]);
    assert_eq!(validate_functions(&project), Ok(()));
}

#[test]
fn creation_refuses_bad_identity_text_and_placement_before_changing_anything() {
    let project = with_light();
    let cases = [
        (function(0, 1, vec![]), "zero id"),
        (function(1, 1, vec![]), "function id in use"),
        (function(2, 99, vec![]), "missing room"),
        (function(2, 1, vec![member(1, 3, None)]), "member id in use"),
        (
            function(2, 1, vec![member(9, 99, None)]),
            "missing group address",
        ),
        (
            function(2, 1, vec![member(9, 4, None)]),
            "other installation",
        ),
        (
            function(2, 1, vec![member(8, 3, None), member(9, 3, None)]),
            "duplicate address",
        ),
        (
            function(2, 1, vec![member(8, 3, None), member(8, 1, None)]),
            "duplicate member id",
        ),
    ];
    for (function, case) in cases {
        let error = refused(&project, Command::CreateFunction { function });
        match case {
            "zero id" | "function id in use" => assert!(
                matches!(
                    error,
                    CommandError::IdInUse {
                        kind: IdKind::Function,
                        ..
                    }
                ),
                "{case}: {error:?}"
            ),
            "missing room" => assert_eq!(
                error,
                CommandError::BuildingPartNotFound(BuildingPartId(99))
            ),
            "member id in use" | "duplicate member id" => assert!(
                matches!(
                    error,
                    CommandError::IdInUse {
                        kind: IdKind::FunctionMember,
                        ..
                    }
                ),
                "{case}: {error:?}"
            ),
            "missing group address" => {
                assert_eq!(
                    error,
                    CommandError::GroupAddressNotFound(GroupAddressId(99))
                )
            }
            "other installation" => assert_eq!(
                error,
                CommandError::CrossInstallation {
                    from: InstallationId(0),
                    to: InstallationId(1)
                }
            ),
            "duplicate address" => assert_eq!(
                error,
                CommandError::InvalidFunction(FunctionError::DuplicateMember(GroupAddressId(3)))
            ),
            _ => unreachable!(),
        }
    }
    let mut blank = function(2, 1, vec![]);
    blank.details.name = " \t".into();
    assert!(matches!(
        refused(&project, Command::CreateFunction { function: blank }),
        CommandError::InvalidFunction(FunctionError::InvalidName(_))
    ));
    let mut long_type = function(2, 1, vec![]);
    long_type.details.function_type = Some("x".repeat(MAX_FUNCTION_SHORT_TEXT_CODEPOINTS + 1));
    assert_eq!(
        refused(
            &project,
            Command::CreateFunction {
                function: long_type
            }
        ),
        CommandError::InvalidFunction(FunctionError::TooLong("type"))
    );
}

#[test]
fn edited_text_passes_admission_and_undo_restores_imported_text_verbatim() {
    let mut project = with_light();
    // Imported text is never normalised or re-admitted: an ETS name with a
    // control character stays exactly as it was across edit and undo.
    project.functions[0].details.name = "Imported\u{7}name".into();
    project.functions[0].details.comment = Some("line 1\r\nline 2".into());
    let edited = FunctionDetails {
        name: "  Living light  ".into(),
        function_type: Some("FT-1".into()),
        number: Some("1.2".into()),
        comment: Some("tab\there".into()),
        description: Some(String::new()),
        completion: CompletionStatus::FinishedDesign,
    };
    round_trip(
        &mut project,
        Command::UpdateFunction {
            id: FunctionId(1),
            details: edited.clone(),
        },
    );
    assert_eq!(
        project.functions[0].details, edited,
        "text is stored exactly as given"
    );
    let mut undone = project.clone();
    let inverse = Command::UpdateFunction {
        id: FunctionId(1),
        details: details("x"),
    }
    .apply(&mut undone)
    .unwrap();
    assert!(matches!(inverse, Command::RestoreFunctionDetails { .. }));
    assert_eq!(
        refused(
            &project,
            Command::UpdateFunction {
                id: FunctionId(1),
                details: details("bad\nname"),
            }
        ),
        CommandError::InvalidFunction(FunctionError::InvalidName(
            knx_core::names::NameError::ControlCharacter
        ))
    );
    assert_eq!(
        refused(
            &project,
            Command::UpdateFunction {
                id: FunctionId(7),
                details: details("x"),
            }
        ),
        CommandError::FunctionNotFound(FunctionId(7))
    );
}

#[test]
fn a_function_moves_within_its_installation_only() {
    let mut project = with_light();
    round_trip(
        &mut project,
        Command::MoveFunction {
            id: FunctionId(1),
            location: BuildingPartId(2),
        },
    );
    assert_eq!(project.functions[0].location, BuildingPartId(2));
    assert_eq!(
        refused(
            &project,
            Command::MoveFunction {
                id: FunctionId(1),
                location: BuildingPartId(3),
            }
        ),
        CommandError::CrossInstallation {
            from: InstallationId(0),
            to: InstallationId(1)
        }
    );
}

#[test]
fn members_are_inserted_and_removed_in_place_and_undo_restores_the_order() {
    let mut project = with_light();
    round_trip(
        &mut project,
        Command::AddFunctionMember {
            function: FunctionId(1),
            member: member(3, 3, Some("Dim")),
            position: 1,
        },
    );
    let order: Vec<_> = project.functions[0]
        .members
        .iter()
        .map(|m| m.id.0)
        .collect();
    assert_eq!(order, [1, 3, 2]);
    round_trip(
        &mut project,
        Command::RemoveFunctionMember {
            function: FunctionId(1),
            member: FunctionMemberId(1),
        },
    );
    let order: Vec<_> = project.functions[0]
        .members
        .iter()
        .map(|m| m.id.0)
        .collect();
    assert_eq!(order, [3, 2]);
    assert_eq!(
        refused(
            &project,
            Command::AddFunctionMember {
                function: FunctionId(1),
                member: member(4, 1, None),
                position: 3,
            }
        ),
        CommandError::InvalidFunction(FunctionError::InvalidPosition(3))
    );
    assert_eq!(
        refused(
            &project,
            Command::AddFunctionMember {
                function: FunctionId(1),
                member: member(4, 2, None),
                position: 0,
            }
        ),
        CommandError::InvalidFunction(FunctionError::DuplicateMember(GroupAddressId(2)))
    );
    assert_eq!(
        refused(
            &project,
            Command::RemoveFunctionMember {
                function: FunctionId(1),
                member: FunctionMemberId(1),
            }
        ),
        CommandError::FunctionMemberNotFound(FunctionMemberId(1))
    );
}

#[test]
fn deleting_a_function_removes_only_it_and_undo_puts_it_back_in_place() {
    let mut project = with_light();
    Command::CreateFunction {
        function: function(2, 2, vec![member(5, 3, None)]),
    }
    .apply(&mut project)
    .unwrap();
    round_trip(&mut project, Command::DeleteFunction { id: FunctionId(1) });
    let ids: Vec<_> = project.functions.iter().map(|f| f.id.0).collect();
    assert_eq!(ids, [2]);
    assert_eq!(
        project.installations[0].group_addresses.len(),
        3,
        "addresses stay"
    );
}

#[test]
fn a_referenced_group_address_or_room_is_not_deleted_out_from_under_a_function() {
    let project = with_light();
    assert_eq!(
        refused(
            &project,
            Command::DeleteGroupAddress {
                id: GroupAddressId(2)
            }
        ),
        CommandError::GroupAddressInFunction(GroupAddressId(2))
    );
    assert_eq!(
        refused(
            &project,
            Command::DeleteBuildingPart {
                id: BuildingPartId(1)
            }
        ),
        CommandError::BuildingPartHasFunctions(BuildingPartId(1))
    );
    // Unreferenced ones still go.
    let mut free = project.clone();
    Command::DeleteGroupAddress {
        id: GroupAddressId(3),
    }
    .apply(&mut free)
    .unwrap();
    Command::DeleteBuildingPart {
        id: BuildingPartId(2),
    }
    .apply(&mut free)
    .unwrap();
}

#[test]
fn a_batch_that_would_orphan_a_member_is_refused_as_a_whole() {
    let project = project();
    let batch = Command::Batch(vec![
        Command::CreateFunction {
            function: function(1, 1, vec![member(1, 3, None)]),
        },
        Command::DeleteGroupAddress {
            id: GroupAddressId(3),
        },
    ]);
    match refused(&project, batch) {
        CommandError::BatchItem { index, source } => {
            assert_eq!(index, 1);
            assert_eq!(
                *source,
                CommandError::GroupAddressInFunction(GroupAddressId(3))
            );
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn moving_a_room_to_another_installation_cannot_strand_its_functions() {
    // Rooms cannot move between installations at all today; this pins that
    // the Function invariant would refuse it too if one tried via restore.
    let mut project = with_light();
    let mut room = project.installations[0].buildings.remove(0);
    room.parent = None;
    project.installations[1].buildings.push(room);
    assert!(matches!(
        validate_functions(&project),
        Err(FunctionInvariantError::ForeignMember { .. })
    ));
    let before = project.clone();
    let error = Command::RenameInstallation {
        id: InstallationId(0),
        name: "Home".into(),
    }
    .apply(&mut project)
    .unwrap_err();
    assert!(
        matches!(error, CommandError::FunctionInvariant(_)),
        "{error:?}"
    );
    assert_eq!(project, before);
}

#[test]
fn the_invariant_names_each_broken_rule() {
    let mut project = with_light();
    project.functions.push(function(1, 2, vec![]));
    assert_eq!(
        validate_functions(&project),
        Err(FunctionInvariantError::DuplicateFunctionId(FunctionId(1)))
    );
    let mut project = with_light();
    project
        .functions
        .push(function(2, 2, vec![member(1, 3, None)]));
    assert_eq!(
        validate_functions(&project),
        Err(FunctionInvariantError::DuplicateMemberId(FunctionMemberId(
            1
        )))
    );
    let mut project = with_light();
    project.functions.push(function(2, 42, vec![]));
    assert_eq!(
        validate_functions(&project),
        Err(FunctionInvariantError::UnplacedLocation {
            function: FunctionId(2),
            location: BuildingPartId(42)
        })
    );
    let mut project = with_light();
    project.functions[0].members[0].id = FunctionMemberId(0);
    assert_eq!(
        validate_functions(&project),
        Err(FunctionInvariantError::ZeroId)
    );
}

#[test]
fn allocators_issue_and_raise_function_ids_independently() {
    let mut ids = IdAllocators::default();
    assert_eq!(ids.next_function_id(), Ok(FunctionId(1)));
    assert_eq!(ids.next_function_member_id(), Ok(FunctionMemberId(1)));
    assert_eq!(ids.next_function_member_id(), Ok(FunctionMemberId(2)));
    let raised = IdAllocators::default().with_function_counts(7, 9);
    assert!(ids.raise_to(&raised));
    assert_eq!((ids.peek_function(), ids.peek_function_member()), (7, 9));
    assert!(!ids.raise_to(&IdAllocators::default()), "never lowered");
}
