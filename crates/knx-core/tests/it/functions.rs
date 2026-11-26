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
        description: None,
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

#[test]
fn project_rename_validates_new_names_preserves_other_fields_and_restores_historical_names_exactly()
{
    use knx_core::address::GroupAddressStyle;
    use knx_core::names::{NameError, MAX_EDITED_NAME_CODEPOINTS};

    // Works on a minimal project with zero installations too.
    let mut empty_project = Project::new(Language("en".into()));
    let inverse = Command::RenameProject {
        name: "Minimal Project".into(),
    }
    .apply(&mut empty_project)
    .unwrap();
    assert_eq!(empty_project.info.name, "Minimal Project");
    assert_eq!(
        inverse,
        Command::RestoreProjectName {
            name: String::new()
        }
    );
    inverse.apply(&mut empty_project).unwrap();
    assert_eq!(empty_project.info.name, "");

    let historical_names = [
        String::new(),
        "   ".into(),
        "\t \u{2003} ".into(),
        "x".repeat(MAX_EDITED_NAME_CODEPOINTS + 1),
        "Imported\tproject\u{7}".into(),
    ];
    for original in historical_names {
        let mut project = with_light();
        project.info.project_id = "P-0511".into();
        project.info.name = original.clone();
        project.info.project_number = Some("PRJ-51".into());
        project.info.group_address_style = GroupAddressStyle::TwoLevel;
        project.info.completion = CompletionStatus::FinishedDesign;
        project.info.ets_schema_version = 23;
        project.info.unlifted_group_address_dpt_declarations = 2;
        let before = project.clone();

        let new_name = "  Haus 🛠  ".to_string();
        let rename = Command::RenameProject {
            name: new_name.clone(),
        };
        assert_eq!(
            rename.rename_target_name(&project),
            Ok(Some(original.as_str()))
        );
        assert_eq!(rename.rename_value(), Some(new_name.as_str()));

        // Direct apply/restore inverse check.
        let mut direct = before.clone();
        let inverse = rename.apply(&mut direct).unwrap();
        assert_eq!(
            inverse,
            Command::RestoreProjectName {
                name: original.clone()
            }
        );
        let mut expected = before.clone();
        expected.info.name = new_name.clone();
        assert_eq!(direct, expected, "non-name project fields must not change");
        let redo_cmd = inverse.apply(&mut direct).unwrap();
        assert_eq!(
            direct, before,
            "restore inverse must recover exact historical name"
        );
        assert_eq!(
            redo_cmd,
            Command::RestoreProjectName {
                name: new_name.clone()
            }
        );
        redo_cmd.apply(&mut direct).unwrap();
        assert_eq!(direct, expected);

        // CommandStack undo/redo and no-op preserving redo.
        let mut stack = CommandStack::new();
        stack.do_command(&mut project, rename).unwrap();
        assert_eq!(project, expected);
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
        stack
            .do_command(
                &mut project,
                Command::RenameProject {
                    name: original.clone(),
                },
            )
            .unwrap();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
        stack.redo(&mut project).unwrap();
        assert_eq!(project, expected);
        let (undo_states, redo_states) = stack.snapshot_states(&project).unwrap();
        assert_eq!(undo_states, vec![before]);
        assert!(redo_states.is_empty());
    }

    // Boundary length (1024 Unicode scalars) is accepted; invalid inputs are refused cleanly.
    let mut project = with_light();
    project.info.name = "Existing".into();
    let max_valid = "🛠".repeat(MAX_EDITED_NAME_CODEPOINTS);
    round_trip(
        &mut project,
        Command::RenameProject {
            name: max_valid.clone(),
        },
    );
    assert_eq!(project.info.name, max_valid);

    let invalid_cases = [
        (String::new(), NameError::Blank),
        ("   ".into(), NameError::Blank),
        ("\u{85}\u{2003}".into(), NameError::Blank),
        (
            "🛠".repeat(MAX_EDITED_NAME_CODEPOINTS + 1),
            NameError::TooLong,
        ),
        ("bad\nname".into(), NameError::ControlCharacter),
        ("bad\tname".into(), NameError::ControlCharacter),
        ("bad\0name".into(), NameError::ControlCharacter),
        ("bad\u{2028}name".into(), NameError::ControlCharacter),
    ];
    for (bad, expected_error) in invalid_cases {
        let mut stack = CommandStack::new();
        let before = project.clone();
        assert_eq!(
            refused(&project, Command::RenameProject { name: bad.clone() }),
            CommandError::InvalidName(expected_error)
        );
        assert_eq!(
            stack.do_command(&mut project, Command::RenameProject { name: bad }),
            Err(CommandError::InvalidName(expected_error))
        );
        assert_eq!(project, before);
        assert!(!stack.can_undo());
    }
}

#[test]
fn function_rename_validates_name_only_preserves_other_details_and_restores_historical_names_exactly(
) {
    use knx_core::names::{NameError, MAX_EDITED_NAME_CODEPOINTS};

    let historical_names = [
        String::new(),
        "   ".into(),
        "\t \u{2003} ".into(),
        "x".repeat(MAX_FUNCTION_SHORT_TEXT_CODEPOINTS + 1),
        "x".repeat(MAX_EDITED_NAME_CODEPOINTS + 1),
        "Imported\u{7}function".into(),
    ];
    for original in historical_names {
        let mut project = with_light();
        // Add a second sibling Function to prove only the targeted Function changes.
        Command::CreateFunction {
            function: function(2, 2, vec![member(3, 3, Some("Dim"))]),
        }
        .apply(&mut project)
        .unwrap();
        // Seed non-name fields with historical values, including ones that would
        // not pass full `FunctionDetails::validate()`, proving name-only scope.
        project.functions[0].source = SourceRef {
            path: "P-0511/0.xml".into(),
            ets_id: "F-1".into(),
        };
        project.functions[0].details = FunctionDetails {
            name: original.clone(),
            function_type: Some("T".repeat(MAX_FUNCTION_SHORT_TEXT_CODEPOINTS + 5)),
            number: Some("1.2".into()),
            comment: Some("Imported\u{7}comment\r\nline 2".into()),
            description: Some("Existing description".into()),
            completion: CompletionStatus::FinishedDesign,
        };
        let before = project.clone();

        let new_name = "  Living light 🛠  ".to_string();
        let rename = Command::RenameFunction {
            id: FunctionId(1),
            name: new_name.clone(),
        };
        assert_eq!(
            rename.rename_target_name(&project),
            Ok(Some(original.as_str()))
        );
        assert_eq!(rename.rename_value(), Some(new_name.as_str()));

        let mut direct = before.clone();
        let inverse = rename.apply(&mut direct).unwrap();
        assert_eq!(
            inverse,
            Command::RestoreFunctionName {
                id: FunctionId(1),
                name: original.clone(),
            }
        );
        let mut expected = before.clone();
        expected.functions[0].details.name = new_name.clone();
        assert_eq!(
            direct, expected,
            "every non-name Function detail, member, sibling and project field must stay unchanged"
        );
        let redo_cmd = inverse.apply(&mut direct).unwrap();
        assert_eq!(
            direct, before,
            "restore inverse must recover exact historical name"
        );
        assert_eq!(
            redo_cmd,
            Command::RestoreFunctionName {
                id: FunctionId(1),
                name: new_name.clone(),
            }
        );
        redo_cmd.apply(&mut direct).unwrap();
        assert_eq!(direct, expected);

        let mut stack = CommandStack::new();
        stack.do_command(&mut project, rename).unwrap();
        assert_eq!(project, expected);
        stack.undo(&mut project).unwrap();
        assert_eq!(project, before);
        stack
            .do_command(
                &mut project,
                Command::RenameFunction {
                    id: FunctionId(1),
                    name: original.clone(),
                },
            )
            .unwrap();
        assert!(!stack.can_undo());
        assert!(stack.can_redo());
        stack.redo(&mut project).unwrap();
        assert_eq!(project, expected);
        let (undo_states, redo_states) = stack.snapshot_states(&project).unwrap();
        assert_eq!(undo_states, vec![before]);
        assert!(redo_states.is_empty());
    }

    // Boundary length (255 Unicode scalars) is accepted; invalid inputs are refused cleanly.
    let mut project = with_light();
    let max_valid = "🛠".repeat(MAX_FUNCTION_SHORT_TEXT_CODEPOINTS);
    round_trip(
        &mut project,
        Command::RenameFunction {
            id: FunctionId(1),
            name: max_valid.clone(),
        },
    );
    assert_eq!(project.functions[0].details.name, max_valid);

    let invalid_cases = [
        (String::new(), FunctionError::InvalidName(NameError::Blank)),
        ("   ".into(), FunctionError::InvalidName(NameError::Blank)),
        (
            "\u{85}\u{2003}".into(),
            FunctionError::InvalidName(NameError::Blank),
        ),
        (
            "🛠".repeat(MAX_FUNCTION_SHORT_TEXT_CODEPOINTS + 1),
            FunctionError::TooLong("name"),
        ),
        (
            "🛠".repeat(MAX_EDITED_NAME_CODEPOINTS + 1),
            FunctionError::InvalidName(NameError::TooLong),
        ),
        (
            "bad\nname".into(),
            FunctionError::InvalidName(NameError::ControlCharacter),
        ),
        (
            "bad\tname".into(),
            FunctionError::InvalidName(NameError::ControlCharacter),
        ),
        (
            "bad\0name".into(),
            FunctionError::InvalidName(NameError::ControlCharacter),
        ),
        (
            "bad\u{2028}name".into(),
            FunctionError::InvalidName(NameError::ControlCharacter),
        ),
    ];
    for (bad, expected_error) in invalid_cases {
        let mut stack = CommandStack::new();
        let before = project.clone();
        assert_eq!(
            refused(
                &project,
                Command::RenameFunction {
                    id: FunctionId(1),
                    name: bad.clone(),
                }
            ),
            CommandError::InvalidFunction(expected_error.clone())
        );
        assert_eq!(
            stack.do_command(
                &mut project,
                Command::RenameFunction {
                    id: FunctionId(1),
                    name: bad,
                }
            ),
            Err(CommandError::InvalidFunction(expected_error))
        );
        assert_eq!(project, before);
        assert!(!stack.can_undo());
    }
}

#[test]
fn function_rename_and_restore_refuse_missing_and_ambiguous_ids_atomically() {
    let base = with_light();
    for missing in [FunctionId(0), FunctionId(99)] {
        let rename = Command::RenameFunction {
            id: missing,
            name: "Valid".into(),
        };
        let restore = Command::RestoreFunctionName {
            id: missing,
            name: String::new(),
        };
        assert_eq!(
            rename.rename_target_name(&base),
            Err(CommandError::FunctionNotFound(missing))
        );
        assert_eq!(
            refused(&base, rename.clone()),
            CommandError::FunctionNotFound(missing)
        );
        assert_eq!(
            refused(&base, restore),
            CommandError::FunctionNotFound(missing)
        );
        let mut candidate = base.clone();
        let mut stack = CommandStack::new();
        assert_eq!(
            stack.do_command(&mut candidate, rename),
            Err(CommandError::FunctionNotFound(missing))
        );
        assert_eq!(candidate, base);
        assert!(!stack.can_undo());
    }

    // Duplicate FunctionId(1) in `project.functions` must be refused unambiguously.
    let mut duplicated = with_light();
    duplicated.functions.push(function(1, 2, vec![]));
    duplicated.functions.push(function(2, 2, vec![]));
    let before = duplicated.clone();

    let duplicate_err =
        CommandError::FunctionInvariant(FunctionInvariantError::DuplicateFunctionId(FunctionId(1)));
    let rename_dup = Command::RenameFunction {
        id: FunctionId(1),
        name: "Renamed".into(),
    };
    assert_eq!(
        rename_dup.rename_target_name(&duplicated),
        Err(duplicate_err.clone())
    );
    assert_eq!(refused(&duplicated, rename_dup), duplicate_err);
    assert_eq!(
        refused(
            &duplicated,
            Command::RestoreFunctionName {
                id: FunctionId(1),
                name: String::new(),
            }
        ),
        duplicate_err
    );
    // Even if `name` matches the first duplicate's current name, `do_command` must refuse, not no-op.
    let mut stack = CommandStack::new();
    let same_name = duplicated.functions[0].details.name.clone();
    assert_eq!(
        stack.do_command(
            &mut duplicated,
            Command::RenameFunction {
                id: FunctionId(1),
                name: same_name,
            }
        ),
        Err(duplicate_err.clone())
    );
    assert_eq!(duplicated, before);
    assert!(!stack.can_undo());

    // Renaming a distinct Function while another ID is duplicated is also refused atomically.
    assert_eq!(
        refused(
            &duplicated,
            Command::RenameFunction {
                id: FunctionId(2),
                name: "Other".into(),
            }
        ),
        duplicate_err
    );
}

#[test]
fn project_and_function_rename_batch_applies_atomically_and_rolls_back_completely_on_failure() {
    use knx_core::names::NameError;

    let mut project = with_light();
    project.info.name = String::new();
    project.functions[0].details.name = " \t ".into();
    let before = project.clone();

    // Valid batch applies both as one undoable step and restores exact historical names.
    round_trip(
        &mut project,
        Command::Batch(vec![
            Command::RenameProject {
                name: "Building A".into(),
            },
            Command::RenameFunction {
                id: FunctionId(1),
                name: "Living Light".into(),
            },
        ]),
    );
    assert_eq!(project.info.name, "Building A");
    assert_eq!(project.functions[0].details.name, "Living Light");

    // Failed batches must roll back earlier valid project and function renames atomically.
    let mut rollback_project = before.clone();
    let failing_batches = [
        (
            Command::Batch(vec![
                Command::RenameProject {
                    name: "Should roll back".into(),
                },
                Command::RenameFunction {
                    id: FunctionId(1),
                    name: "Should also roll back".into(),
                },
                Command::RenameFunction {
                    id: FunctionId(99),
                    name: "Missing".into(),
                },
            ]),
            2,
            CommandError::FunctionNotFound(FunctionId(99)),
        ),
        (
            Command::Batch(vec![
                Command::RenameProject {
                    name: "Should roll back".into(),
                },
                Command::RenameFunction {
                    id: FunctionId(1),
                    name: "   ".into(),
                },
            ]),
            1,
            CommandError::InvalidFunction(FunctionError::InvalidName(NameError::Blank)),
        ),
        (
            Command::Batch(vec![
                Command::RenameFunction {
                    id: FunctionId(1),
                    name: "Should roll back".into(),
                },
                Command::RenameProject { name: "   ".into() },
            ]),
            1,
            CommandError::InvalidName(NameError::Blank),
        ),
    ];
    for (batch, expected_index, expected_source) in failing_batches {
        let mut stack = CommandStack::new();
        let err = stack.do_command(&mut rollback_project, batch).unwrap_err();
        assert_eq!(
            err,
            CommandError::BatchItem {
                index: expected_index,
                source: Box::new(expected_source),
            }
        );
        assert_eq!(
            rollback_project, before,
            "failed batch must leave project untouched"
        );
        assert!(!stack.can_undo(), "failed batch must leave stack untouched");
    }

    // Batch against a project with a duplicated function id is refused atomically
    // at step 0 by `Command::apply`'s function-invariant guard before publishing.
    let mut dup_project = before.clone();
    dup_project.functions.push(function(1, 2, vec![]));
    let dup_before = dup_project.clone();
    let mut stack = CommandStack::new();
    let err = stack
        .do_command(
            &mut dup_project,
            Command::Batch(vec![
                Command::RenameProject {
                    name: "Should roll back".into(),
                },
                Command::RenameFunction {
                    id: FunctionId(1),
                    name: "Ambiguous".into(),
                },
            ]),
        )
        .unwrap_err();
    assert_eq!(
        err,
        CommandError::BatchItem {
            index: 0,
            source: Box::new(CommandError::FunctionInvariant(
                FunctionInvariantError::DuplicateFunctionId(FunctionId(1)),
            )),
        }
    );
    assert_eq!(dup_project, dup_before);
    assert!(!stack.can_undo());
}
