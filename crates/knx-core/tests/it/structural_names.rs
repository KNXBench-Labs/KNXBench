//! Verifies structural name admission, exact inverses, identity safety and atomic history.

use knx_core::names::NameError;
use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, Command, CommandError,
    CommandStack, CommissioningState, CompletionStatus, DeviceId, DeviceInstance, GroupAddress,
    GroupRange, GroupRangeId, Installation, InstallationId, Language, Line, LineId, Project,
    SourceRef, Topology,
};

const BUILDING_KINDS: [BuildingPartType; 11] = [
    BuildingPartType::Building,
    BuildingPartType::Floor,
    BuildingPartType::Room,
    BuildingPartType::Corridor,
    BuildingPartType::DistributionBoard,
    BuildingPartType::BuildingPart,
    BuildingPartType::Stairway,
    BuildingPartType::RoomPart,
    BuildingPartType::Area,
    BuildingPartType::Ground,
    BuildingPartType::Segment,
];

#[derive(Clone, Copy, Debug)]
enum Target {
    Installation,
    Area,
    Line,
    Part(usize),
    Range(usize),
}

fn targets() -> Vec<Target> {
    let mut targets = vec![
        Target::Installation,
        Target::Area,
        Target::Line,
        Target::Range(0),
        Target::Range(1),
    ];
    targets.extend((0..BUILDING_KINDS.len()).map(Target::Part));
    targets
}

impl Target {
    fn rename(self, name: &str) -> Command {
        let name = name.into();
        match self {
            Self::Installation => Command::RenameInstallation {
                id: InstallationId(2),
                name,
            },
            Self::Area => Command::RenameArea {
                id: AreaId(2),
                name,
            },
            Self::Line => Command::RenameLine {
                id: LineId(2),
                name,
            },
            Self::Part(index) => Command::RenameBuildingPart {
                id: BuildingPartId(101 + index as u32),
                name,
            },
            Self::Range(index) => Command::RenameGroupRange {
                id: GroupRangeId(2 + index as u32 * 2),
                name,
            },
        }
    }

    fn restore(self, name: &str) -> Command {
        let name = name.into();
        match self {
            Self::Installation => Command::RestoreInstallationName {
                id: InstallationId(2),
                name,
            },
            Self::Area => Command::RestoreAreaName {
                id: AreaId(2),
                name,
            },
            Self::Line => Command::RestoreLineName {
                id: LineId(2),
                name,
            },
            Self::Part(index) => Command::RestoreBuildingPartName {
                id: BuildingPartId(101 + index as u32),
                name,
            },
            Self::Range(index) => Command::RestoreGroupRangeName {
                id: GroupRangeId(2 + index as u32 * 2),
                name,
            },
        }
    }

    fn name_mut(self, project: &mut Project) -> &mut String {
        let installation = &mut project.installations[1];
        match self {
            Self::Installation => &mut installation.name,
            Self::Area => &mut installation.topology.areas[0].name,
            Self::Line => &mut installation.topology.lines[0].name,
            Self::Part(index) => &mut installation.buildings[index].name,
            Self::Range(index) => &mut installation.group_ranges[index].name,
        }
    }

    fn missing_error(self) -> CommandError {
        match self {
            Self::Installation => CommandError::InstallationNotFound,
            Self::Area => CommandError::AreaNotFound(AreaId(2)),
            Self::Line => CommandError::LineNotFound(LineId(2)),
            Self::Part(index) => {
                CommandError::BuildingPartNotFound(BuildingPartId(101 + index as u32))
            }
            Self::Range(index) => {
                CommandError::GroupRangeNotFound(GroupRangeId(2 + index as u32 * 2))
            }
        }
    }

    fn ambiguous_error(self) -> CommandError {
        match self {
            Self::Installation => CommandError::InstallationIdentityAmbiguous(InstallationId(2)),
            Self::Area => CommandError::AreaPlacementAmbiguous(AreaId(2)),
            Self::Line => CommandError::LinePlacementAmbiguous(LineId(2)),
            Self::Part(index) => {
                CommandError::BuildingPartPlacementAmbiguous(BuildingPartId(101 + index as u32))
            }
            Self::Range(index) => {
                CommandError::GroupRangePlacementAmbiguous(GroupRangeId(2 + index as u32 * 2))
            }
        }
    }

    fn remove(self, project: &mut Project) {
        match self {
            Self::Installation => {
                project.installations.remove(1);
            }
            Self::Area => project.installations[1].topology.areas.clear(),
            Self::Line => project.installations[1].topology.lines.clear(),
            Self::Part(index) => {
                project.installations[1].buildings.remove(index);
            }
            Self::Range(index) => {
                project.installations[1].group_ranges.remove(index);
            }
        }
    }

    fn duplicate(self, project: &mut Project, across_installations: bool) {
        let source = project.installations[1].clone();
        let destination = usize::from(!across_installations);
        match self {
            Self::Installation => project.installations.push(source),
            Self::Area => project.installations[destination]
                .topology
                .areas
                .push(source.topology.areas[0].clone()),
            Self::Line => project.installations[destination]
                .topology
                .lines
                .push(source.topology.lines[0].clone()),
            Self::Part(index) => project.installations[destination]
                .buildings
                .push(source.buildings[index].clone()),
            Self::Range(index) => project.installations[destination]
                .group_ranges
                .push(source.group_ranges[index].clone()),
        }
    }
}

fn source(id: u32) -> SourceRef {
    SourceRef {
        path: "synthetic.xml".into(),
        ets_id: format!("source-{id}"),
    }
}

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    for number in 1..=2 {
        let line = LineId(number);
        let device = DeviceId(number);
        project.installations.push(Installation {
            id: InstallationId(number as u8),
            name: "duplicate label".into(),
            default_line: Some(line),
            multicast_address: Some("224.0.23.12".parse().unwrap()),
            completion: CompletionStatus::Accepted,
            topology: Topology {
                areas: vec![Area {
                    id: AreaId(number),
                    source: source(number),
                    name: "duplicate label".into(),
                    address: number as u8,
                    completion: CompletionStatus::Tested,
                    lines: vec![line],
                }],
                lines: vec![Line {
                    id: line,
                    source: source(number),
                    name: "duplicate label".into(),
                    address: 1,
                    medium_ref: "MT-5".into(),
                    domain_address: Some("123456".into()),
                    domain_address_is_checked: Some(false),
                    ip_routing_multicast_address: Some("224.0.23.12".parse().unwrap()),
                    multicast_ttl: Some(16),
                    completion: CompletionStatus::FinishedCommissioning,
                    devices: vec![device],
                }],
                unassigned: vec![],
            },
            buildings: BUILDING_KINDS
                .iter()
                .enumerate()
                .map(|(index, &kind)| BuildingPart {
                    id: BuildingPartId((number - 1) * 100 + index as u32 + 1),
                    source: source(index as u32),
                    name: "duplicate label".into(),
                    number: Some(format!("room-{index}")),
                    kind,
                    default_line: Some(line),
                    completion: CompletionStatus::Locked,
                    children: if index + 1 < BUILDING_KINDS.len() {
                        vec![BuildingPartId((number - 1) * 100 + index as u32 + 2)]
                    } else {
                        vec![]
                    },
                    parent: if index > 0 {
                        Some(BuildingPartId((number - 1) * 100 + index as u32))
                    } else {
                        None
                    },
                    devices: if index == 0 { vec![device] } else { vec![] },
                })
                .collect(),
            group_ranges: vec![
                GroupRange {
                    id: GroupRangeId(number),
                    source: source(number),
                    name: "duplicate label".into(),
                    start: GroupAddress::from_raw(100),
                    end: GroupAddress::from_raw(200),
                    parent: None,
                    children: vec![GroupRangeId(number + 2)],
                },
                GroupRange {
                    id: GroupRangeId(number + 2),
                    source: source(number + 2),
                    name: "duplicate label".into(),
                    start: GroupAddress::from_raw(100),
                    end: GroupAddress::from_raw(150),
                    parent: Some(GroupRangeId(number)),
                    children: vec![],
                },
            ],
            group_addresses: vec![],
            parameters: vec![],
        });
        project.devices.insert(DeviceInstance {
            id: device,
            source: source(number),
            name: "Programmed witness".into(),
            description: Some("keep description".into()),
            address: Some(knx_core::IndividualAddress::new(number as u8, 1, 1).unwrap()),
            product_ref: "product".into(),
            program_ref: "program".into(),
            commissioning: CommissioningState {
                completion: CompletionStatus::Accepted,
                individual_address_loaded: true,
                application_program_loaded: true,
                parameters_loaded: true,
                communication_part_loaded: true,
                medium_config_loaded: true,
                ..Default::default()
            },
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
    project
}

#[test]
fn unicode_names_and_duplicate_labels_change_only_the_target_name() {
    for target in targets() {
        for name in [
            "  Küche 🛠 e\u{301}  ".into(),
            "🛠".repeat(1024),
            "duplicate label".into(),
        ] {
            let mut project = project();
            *target.name_mut(&mut project) = "old name".into();
            let before = project.clone();
            let mut expected = before.clone();
            *target.name_mut(&mut expected) = name.clone();
            let command = target.rename(&name);
            assert_eq!(command.rename_value(), Some(name.as_str()));
            assert_eq!(command.rename_target_name(&project), Ok(Some("old name")));
            let inverse = command.apply(&mut project).unwrap();
            assert_eq!(inverse, target.restore("old name"));
            assert_eq!(project, expected, "{target:?}");
            let redo = inverse.apply(&mut project).unwrap();
            assert_eq!(project, before);
            redo.apply(&mut project).unwrap();
            assert_eq!(project, expected);
        }
    }
}

#[test]
fn invalid_new_names_are_refused_without_changing_project_or_history() {
    let mut invalid = vec![
        ("".into(), NameError::Blank),
        (" \u{2003} ".into(), NameError::Blank),
        ("🛠".repeat(1025), NameError::TooLong),
    ];
    invalid.extend(
        [
            '\0', '\t', '\n', '\r', '\u{7f}', '\u{9f}', '\u{2028}', '\u{2029}',
        ]
        .map(|c| (format!("name{c}"), NameError::ControlCharacter)),
    );
    for target in targets() {
        for (name, error) in &invalid {
            let mut project = project();
            let mut stack = populated_history(&mut project);
            let before = project.clone();
            let history = stack.snapshot_states(&project).unwrap();
            assert_eq!(
                target.rename(name).apply(&mut project),
                Err(CommandError::InvalidName(*error))
            );
            assert_eq!(project, before, "{target:?}: {name:?}, apply");
            assert_eq!(
                stack.do_command(&mut project, target.rename(name)),
                Err(CommandError::InvalidName(*error))
            );
            assert_eq!(project, before, "{target:?}: {name:?}");
            assert_eq!(
                stack.snapshot_states(&project).unwrap(),
                history,
                "{target:?}: {name:?}, invalid-name refusal changed history"
            );
            assert!(stack.can_undo());
            assert!(stack.can_redo());
        }
    }
}

#[test]
fn undo_and_redo_restore_legacy_invalid_names_exactly() {
    for target in targets() {
        for legacy in [
            String::new(),
            " \t\u{2003} ".into(),
            "x".repeat(1025),
            "old\u{7}name".into(),
        ] {
            let mut project = project();
            *target.name_mut(&mut project) = legacy.clone();
            let before = project.clone();
            let mut expected = before.clone();
            *target.name_mut(&mut expected) = "New 🌍".into();
            let mut stack = CommandStack::new();
            stack
                .do_command(&mut project, target.rename("New 🌍"))
                .unwrap();
            assert_eq!(project, expected, "{target:?}");
            assert_eq!(stack.comparison_revision(DeviceId(2)), 0);
            stack.undo(&mut project).unwrap();
            assert_eq!(project, before, "{target:?}: {legacy:?}");
            stack.redo(&mut project).unwrap();
            assert_eq!(project, expected);
        }
    }
}

#[test]
fn missing_and_duplicate_ids_refuse_rename_restore_and_name_lookup() {
    for target in targets() {
        for damage in 0..3 {
            let mut project = project();
            let mut stack = populated_history(&mut project);
            let error = if damage == 0 {
                target.remove(&mut project);
                target.missing_error()
            } else {
                target.duplicate(&mut project, damage == 2);
                target.ambiguous_error()
            };
            let before = project.clone();
            let history = stack.snapshot_states(&project).unwrap();
            // A matching label must not bypass identity admission as a no-op.
            let rename = target.rename("duplicate label");
            assert_eq!(rename.rename_target_name(&project), Err(error.clone()));
            for command in [rename, target.restore("")] {
                assert_eq!(command.apply(&mut project), Err(error.clone()));
                assert_eq!(
                    project, before,
                    "{target:?}, damage {damage}, apply {command:?}"
                );
                assert_eq!(
                    stack.do_command(&mut project, command.clone()),
                    Err(error.clone())
                );
                assert_eq!(project, before, "{target:?}, damage {damage}, {command:?}");
                assert_eq!(
                    stack.snapshot_states(&project).unwrap(),
                    history,
                    "{target:?}, damage {damage}, identity refusal changed history: {command:?}"
                );
                assert!(stack.can_undo());
                assert!(stack.can_redo());
            }
        }
    }
}

fn populated_history(project: &mut Project) -> CommandStack {
    let mut stack = CommandStack::new();
    for name in ["First edit", "Second edit"] {
        stack
            .do_command(project, Command::RenameProject { name: name.into() })
            .unwrap();
    }
    stack.undo(project).unwrap();
    stack
}

#[test]
fn same_name_is_a_noop_preserving_both_history_directions() {
    for target in targets() {
        for name in ["duplicate label", "", " \t "] {
            let mut project = project();
            *target.name_mut(&mut project) = name.into();
            let mut stack = populated_history(&mut project);
            let before = project.clone();
            let history = stack.snapshot_states(&project).unwrap();
            stack.do_command(&mut project, target.rename(name)).unwrap();
            assert_eq!(project, before);
            assert_eq!(
                stack.snapshot_states(&project).unwrap(),
                history,
                "{target:?}"
            );
            assert!(stack.can_undo());
            assert!(stack.can_redo());
            assert_eq!(stack.comparison_revision(DeviceId(2)), 0);
        }
    }
}

#[test]
fn failed_mixed_batch_restores_every_field_and_both_history_directions() {
    for failing_target in targets() {
        let mut project = project();
        for (index, target) in targets().into_iter().enumerate() {
            *target.name_mut(&mut project) = if index % 2 == 0 { "" } else { " \t " }.into();
        }
        let mut stack = populated_history(&mut project);
        let before = project.clone();
        let history = stack.snapshot_states(&project).unwrap();
        let mut commands = vec![Command::RenameDevice {
            device: DeviceId(2),
            name: "Device renamed".into(),
        }];
        commands.extend(
            targets()
                .into_iter()
                .map(|target| target.rename("Valid structural name")),
        );
        commands.push(Command::RenameProject {
            name: "Project renamed".into(),
        });
        let failure_index = commands.len();
        commands.push(failing_target.rename("\n"));
        let command = Command::Batch(commands);
        let error = CommandError::BatchItem {
            index: failure_index,
            source: Box::new(CommandError::InvalidName(NameError::Blank)),
        };
        assert_eq!(command.apply(&mut project), Err(error.clone()));
        assert_eq!(project, before);
        assert_eq!(stack.do_command(&mut project, command), Err(error));
        assert_eq!(project, before, "{failing_target:?}");
        assert_eq!(stack.snapshot_states(&project).unwrap(), history);
        assert_eq!(stack.comparison_revision(DeviceId(2)), 0);
        assert!(stack.can_undo());
        assert!(stack.can_redo());
    }
}

#[test]
fn installation_create_resolution_keeps_its_legacy_first_match() {
    let mut project = project();
    project.installations[1].id = InstallationId(1);
    let mut area = project.installations[0].topology.areas[0].clone();
    area.id = AreaId(3);
    area.address = 3;
    area.lines.clear();
    Command::CreateArea {
        area: area.clone(),
        installation: Some(InstallationId(1)),
    }
    .apply(&mut project)
    .unwrap();
    assert_eq!(project.installations[0].topology.areas.last(), Some(&area));
    assert_eq!(project.installations[1].topology.areas.len(), 1);
}
