//! Exercises typed note integrity without IO or changing project schema.

use knx_core::note::{
    validate_notes, NoteError, OrphanedNote, ProjectNote, ProjectNoteTarget as Target,
    MAX_NOTES_TEXT_BYTES, MAX_NOTE_BODY_BYTES, MAX_NOTE_TITLE_BYTES, MAX_PROJECT_NOTES,
};
use knx_core::{
    AreaId, BuildingPartId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId,
    InstallationId, Language, LineId, Project, ProjectNoteId,
};

fn note(id: u32) -> ProjectNote {
    ProjectNote::new(
        ProjectNoteId(id),
        Target::Project,
        " title ".into(),
        "Literal Grüße🙂\r\n<img src=\"https://untrusted.invalid/never\">\n\t".into(),
    )
    .unwrap()
}
#[test]
fn note_text_is_literal_and_documentation_is_explicitly_opted_in() {
    let n = note(1);
    assert_eq!(n.title(), " title ");
    assert!(n.body().contains("\r\n<img"));
    assert!(!n.include_in_documentation());
    let included = n.clone().with_documentation(true);
    assert!(included.include_in_documentation());
    assert!(!n.include_in_documentation());
    assert_eq!(n.id(), ProjectNoteId(1));
    assert_eq!(n.target(), Target::Project);
}
#[test]
fn invalid_identity_and_blank_title_are_refused_without_normalizing_valid_text() {
    assert_eq!(
        ProjectNote::new(ProjectNoteId(0), Target::Project, "x".into(), String::new()),
        Err(NoteError::InvalidId)
    );
    assert_eq!(
        ProjectNote::new(
            ProjectNoteId(1),
            Target::Project,
            " \t\r\n".into(),
            String::new()
        ),
        Err(NoteError::EmptyTitle)
    );
    assert_eq!(
        ProjectNote::new(
            ProjectNoteId(1),
            Target::Project,
            "x".repeat(MAX_NOTE_TITLE_BYTES + 1),
            String::new()
        ),
        Err(NoteError::TitleBudget)
    );
    assert_eq!(
        ProjectNote::new(
            ProjectNoteId(1),
            Target::Project,
            "x".into(),
            "x".repeat(MAX_NOTE_BODY_BYTES + 1)
        ),
        Err(NoteError::BodyBudget)
    );
    for body in ["\0", "\u{1b}", "\u{7f}"] {
        assert_eq!(
            ProjectNote::new(ProjectNoteId(1), Target::Project, "x".into(), body.into()),
            Err(NoteError::ControlCharacter)
        );
    }
}
#[test]
fn every_absent_typed_target_is_refused_not_resolved_by_its_numeric_id() {
    let project = Project::new(Language("en".into()));
    assert_eq!(Target::Project.validate(&project), Ok(()));
    for target in [
        Target::Installation(InstallationId(0)),
        Target::Area(AreaId(1)),
        Target::Line(LineId(1)),
        Target::BuildingPart(BuildingPartId(1)),
        Target::Device(DeviceId(1)),
        Target::GroupRange(GroupRangeId(1)),
        Target::GroupAddress(GroupAddressId(1)),
        Target::ComObject(ComObjectInstanceId(1)),
    ] {
        assert_eq!(
            target.validate(&project),
            Err(NoteError::MissingTarget(target))
        );
    }
}
#[test]
fn collection_rejects_duplicate_ids_and_bounded_size() {
    let project = Project::new(Language("en".into()));
    assert_eq!(
        validate_notes(&project, &[note(1), note(1)]),
        Err(NoteError::DuplicateId(ProjectNoteId(1)))
    );
    let notes = (0..=MAX_PROJECT_NOTES)
        .map(|i| note(u32::try_from(i + 1).unwrap()))
        .collect::<Vec<_>>();
    assert_eq!(
        validate_notes(&project, &notes),
        Err(NoteError::NoteCountBudget)
    );
    let notes = (1..=(MAX_NOTES_TEXT_BYTES / MAX_NOTE_BODY_BYTES + 1))
        .map(|i| {
            ProjectNote::new(
                ProjectNoteId(u32::try_from(i).unwrap()),
                Target::Project,
                "x".into(),
                "x".repeat(MAX_NOTE_BODY_BYTES),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        validate_notes(&project, &notes),
        Err(NoteError::TotalTextBudget)
    );
}

fn source() -> knx_core::SourceRef {
    knx_core::SourceRef {
        path: "synthetic".into(),
        ets_id: "synthetic".into(),
    }
}
fn target_project() -> Project {
    use knx_core::{
        Area, BuildingPart, BuildingPartType, ComObjectInstance, CommissioningState,
        CompletionStatus, DeviceInstance, GroupAddress, GroupAddressEntry, GroupRange,
        Installation, Override, ResolvedFlags, Topology,
    };
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source(),
        name: "device".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "A".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: source(),
        device: DeviceId(1),
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
    });
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "install".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: source(),
                name: "area".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![LineId(1)],
            }],
            lines: vec![knx_core::Line {
                id: LineId(1),
                source: source(),
                name: "line".into(),
                address: 1,
                medium_ref: "MT-0".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::FinishedDesign,
                devices: vec![DeviceId(1)],
            }],
            unassigned: vec![],
        },
        buildings: vec![BuildingPart {
            id: BuildingPartId(1),
            source: source(),
            name: "room".into(),
            number: None,
            kind: BuildingPartType::Room,
            default_line: None,
            completion: CompletionStatus::FinishedDesign,
            children: vec![],
            devices: vec![DeviceId(1)],
            parent: None,
        }],
        group_ranges: vec![GroupRange {
            id: GroupRangeId(1),
            source: source(),
            name: "range".into(),
            start: GroupAddress::from_raw(1),
            end: GroupAddress::from_raw(2),
            parent: None,
            children: vec![],
        }],
        group_addresses: vec![GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "group".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(1)),
            declared_dpt: Override::Absent,
        }],
        parameters: vec![],
    });
    project
}
fn targets() -> [Target; 9] {
    [
        Target::Project,
        Target::Installation(InstallationId(0)),
        Target::Area(AreaId(1)),
        Target::Line(LineId(1)),
        Target::BuildingPart(BuildingPartId(1)),
        Target::Device(DeviceId(1)),
        Target::GroupRange(GroupRangeId(1)),
        Target::GroupAddress(GroupAddressId(1)),
        Target::ComObject(ComObjectInstanceId(1)),
    ]
}
#[test]
fn valid_exact_targets_and_collection_order_are_preserved() {
    let project = target_project();
    let before = project.clone();
    let notes = targets()
        .into_iter()
        .enumerate()
        .map(|(i, target)| {
            ProjectNote::new(
                ProjectNoteId(u32::try_from(i + 1).unwrap()),
                target,
                "title".into(),
                "body".into(),
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let before_notes = notes.clone();
    assert_eq!(validate_notes(&project, &notes), Ok(()));
    assert_eq!(notes, before_notes);
    assert_eq!(project, before);
}
#[test]
fn duplicate_stored_target_ids_are_ambiguous_not_first_wins() {
    let mut project = target_project();
    let duplicate = project.installations[0].clone();
    project.installations.push(duplicate);
    for target in targets().into_iter().filter(|t| {
        !matches!(
            t,
            Target::Project | Target::Device(_) | Target::ComObject(_)
        )
    }) {
        assert_eq!(
            target.validate(&project),
            Err(NoteError::AmbiguousTarget(target))
        );
    }
}
#[test]
fn a_communication_object_with_no_owning_device_is_not_a_valid_note_target() {
    let mut project = target_project();
    project.devices.remove(DeviceId(1));
    let target = Target::ComObject(ComObjectInstanceId(1));
    assert_eq!(
        target.validate(&project),
        Err(NoteError::MissingTarget(target))
    );
}
#[test]
fn utf8_limits_are_bytes_and_exact_boundary_text_stays_verbatim() {
    let title = "🙂".repeat(MAX_NOTE_TITLE_BYTES / 4);
    let body = "🙂".repeat(MAX_NOTE_BODY_BYTES / 4);
    let note = ProjectNote::new(
        ProjectNoteId(u32::MAX),
        Target::Project,
        title.clone(),
        body.clone(),
    )
    .unwrap();
    assert_eq!(note.title(), title);
    assert_eq!(note.body(), body);
    assert_eq!(
        ProjectNote::new(
            ProjectNoteId(1),
            Target::Project,
            title + "x",
            String::new()
        ),
        Err(NoteError::TitleBudget)
    );
    assert_eq!(
        ProjectNote::new(ProjectNoteId(1), Target::Project, "x".into(), body + "x"),
        Err(NoteError::BodyBudget)
    );
}

#[test]
fn note_commands_restore_exact_order_text_target_and_inclusion() {
    use knx_core::Command;
    let mut project = target_project();
    let first = note(1);
    let second = note(2);
    let remove_first = Command::InsertProjectNote {
        note: first.clone(),
        position: 0,
    }
    .apply(&mut project)
    .unwrap();
    Command::InsertProjectNote {
        note: second.clone(),
        position: 1,
    }
    .apply(&mut project)
    .unwrap();
    assert_eq!(project.ids.peek_project_note(), 2);
    let before = project.clone();
    let inverse = Command::MoveProjectNote {
        id: ProjectNoteId(1),
        position: 1,
    }
    .apply(&mut project)
    .unwrap();
    assert_eq!(project.notes, [second.clone(), first.clone()]);
    inverse.apply(&mut project).unwrap();
    assert_eq!(project, before);
    let replacement = ProjectNote::new(
        ProjectNoteId(1),
        Target::Device(DeviceId(1)),
        "edited".into(),
        "body\r\n".into(),
    )
    .unwrap()
    .with_documentation(true);
    let inverse = Command::ReplaceProjectNote {
        note: replacement.clone(),
    }
    .apply(&mut project)
    .unwrap();
    assert_eq!(project.notes[0], replacement);
    inverse.apply(&mut project).unwrap();
    assert_eq!(project, before);
    let inverse = Command::RemoveProjectNote {
        id: ProjectNoteId(1),
    }
    .apply(&mut project)
    .unwrap();
    assert_eq!(project.notes, [second]);
    inverse.apply(&mut project).unwrap();
    assert_eq!(project, before);
    remove_first.apply(&mut project).unwrap();
    assert_eq!(project.ids.peek_project_note(), 2);
    assert_eq!(
        project.ids.next_project_note_id().unwrap(),
        ProjectNoteId(3)
    );
}
#[test]
fn invalid_note_command_leaves_content_and_counters_exactly_unchanged() {
    use knx_core::Command;
    let mut project = target_project();
    let before = project.clone();
    let invalid = ProjectNote::new(
        ProjectNoteId(5),
        Target::Device(DeviceId(99)),
        "x".into(),
        "x".into(),
    )
    .unwrap();
    assert!(Command::InsertProjectNote {
        note: invalid,
        position: 0
    }
    .apply(&mut project)
    .is_err());
    assert_eq!(project, before);
    assert!(Command::InsertProjectNote {
        note: note(1),
        position: 1
    }
    .apply(&mut project)
    .is_err());
    assert_eq!(project, before);
}
#[test]
fn every_existing_delete_command_refuses_an_implicitly_orphaned_note_atomically() {
    use knx_core::{Command, CommandError};
    for (target, delete) in [
        (
            Target::Area(AreaId(1)),
            Command::DeleteArea { id: AreaId(1) },
        ),
        (
            Target::Line(LineId(1)),
            Command::DeleteLine { id: LineId(1) },
        ),
        (
            Target::BuildingPart(BuildingPartId(1)),
            Command::DeleteBuildingPart {
                id: BuildingPartId(1),
            },
        ),
        (
            Target::Device(DeviceId(1)),
            Command::DeleteDevice { id: DeviceId(1) },
        ),
        (
            Target::GroupRange(GroupRangeId(1)),
            Command::DeleteGroupRange {
                id: GroupRangeId(1),
            },
        ),
        (
            Target::GroupAddress(GroupAddressId(1)),
            Command::DeleteGroupAddress {
                id: GroupAddressId(1),
            },
        ),
        (
            Target::ComObject(ComObjectInstanceId(1)),
            Command::DeleteDevice { id: DeviceId(1) },
        ),
    ] {
        let mut project = target_project();
        match target {
            Target::Area(_) => {
                project.installations[0].topology.areas[0].lines.clear();
                project.installations[0].topology.lines.clear();
                project.installations[0]
                    .topology
                    .unassigned
                    .push(DeviceId(1));
            }
            Target::Line(_) => {
                project.installations[0].topology.lines[0].devices.clear();
                project.installations[0]
                    .topology
                    .unassigned
                    .push(DeviceId(1));
            }
            Target::BuildingPart(_) | Target::Device(_) | Target::ComObject(_) => {
                project.installations[0].buildings[0].devices.clear()
            }
            Target::GroupRange(_) => project.installations[0].group_addresses[0].range = None,
            _ => {}
        }
        project.notes.push(
            ProjectNote::new(ProjectNoteId(1), target, "retain".into(), "reason".into()).unwrap(),
        );
        project.ids = project.ids.clone().with_project_note_count(1);
        let before = project.clone();
        assert_eq!(
            delete.apply(&mut project),
            Err(CommandError::NotesOrphaned(vec![OrphanedNote {
                id: ProjectNoteId(1),
                target,
                title: "retain".into(),
            }])),
            "{target:?}"
        );
        assert_eq!(project, before);
    }
}
/// A device and its objects carry three notes; a fourth sits on the project.
fn noted_device_project() -> Project {
    let mut project = target_project();
    project.installations[0].buildings[0].devices.clear();
    for (id, target, title) in [
        (1, Target::Device(DeviceId(1)), "Device note"),
        (2, Target::Project, "Project note"),
        (
            3,
            Target::ComObject(ComObjectInstanceId(1)),
            "Object \"quoted\"\nnote",
        ),
    ] {
        project.notes.push(
            ProjectNote::new(ProjectNoteId(id), target, title.into(), "body".into()).unwrap(),
        );
    }
    project.ids = project.ids.clone().with_project_note_count(3);
    project
}
#[test]
fn a_device_delete_names_every_note_it_would_orphan_including_its_objects() {
    use knx_core::{Command, CommandError};
    let mut project = noted_device_project();
    let before = project.clone();
    let error = Command::DeleteDevice { id: DeviceId(1) }
        .apply(&mut project)
        .unwrap_err();
    assert_eq!(
        error,
        CommandError::NotesOrphaned(vec![
            OrphanedNote {
                id: ProjectNoteId(1),
                target: Target::Device(DeviceId(1)),
                title: "Device note".into(),
            },
            OrphanedNote {
                id: ProjectNoteId(3),
                target: Target::ComObject(ComObjectInstanceId(1)),
                title: "Object \"quoted\"\nnote".into(),
            },
        ])
    );
    assert_eq!(project, before);
    let text = error.to_string();
    assert!(
        text.contains("would leave 2 project notes without their target"),
        "{text}"
    );
    assert!(
        text.contains("note 1 \"Device note\" on device 1"),
        "{text}"
    );
    assert!(
        text.contains("note 3 \"Object \\\"quoted\\\"\\nnote\" on communication object 1"),
        "{text}"
    );
    assert!(text.contains("nothing was changed"), "{text}");
    assert!(!text.contains("Project note"), "{text}");
    assert!(!text.contains("note validation refused"), "{text}");
}
#[test]
fn a_batch_names_the_orphaning_step_and_its_notes() {
    use knx_core::{Command, CommandError};
    let mut project = noted_device_project();
    let before = project.clone();
    let error = Command::Batch(vec![
        Command::RemoveProjectNote {
            id: ProjectNoteId(2),
        },
        Command::DeleteDevice { id: DeviceId(1) },
    ])
    .apply(&mut project)
    .unwrap_err();
    let CommandError::BatchItem { index: 1, source } = &error else {
        panic!("{error:?}");
    };
    assert!(matches!(**source, CommandError::NotesOrphaned(ref notes) if notes.len() == 2));
    assert_eq!(project, before);
}
#[test]
fn a_new_note_with_a_missing_target_stays_an_invalid_note_in_plain_words() {
    use knx_core::{Command, CommandError};
    let mut project = target_project();
    let note = ProjectNote::new(
        ProjectNoteId(1),
        Target::Device(DeviceId(99)),
        "x".into(),
        String::new(),
    )
    .unwrap();
    let error = Command::InsertProjectNote { note, position: 0 }
        .apply(&mut project)
        .unwrap_err();
    assert_eq!(
        error,
        CommandError::InvalidNote(NoteError::MissingTarget(Target::Device(DeviceId(99))))
    );
    assert_eq!(
        error.to_string(),
        "note refused: its target device 99 does not exist"
    );
}
#[test]
fn every_note_error_reads_as_a_sentence_not_a_debug_dump() {
    for error in [
        NoteError::InvalidId,
        NoteError::EmptyTitle,
        NoteError::TitleBudget,
        NoteError::BodyBudget,
        NoteError::ControlCharacter,
        NoteError::NoteCountBudget,
        NoteError::TotalTextBudget,
        NoteError::DuplicateId(ProjectNoteId(4)),
        NoteError::MissingTarget(Target::GroupRange(GroupRangeId(2))),
        NoteError::AmbiguousTarget(Target::BuildingPart(BuildingPartId(3))),
    ] {
        let text = error.to_string();
        assert!(!text.is_empty());
        assert!(
            !text.contains('(') && !text.contains("Id"),
            "{error:?} reads as {text}"
        );
    }
    assert_eq!(Target::Project.to_string(), "the project");
    assert_eq!(
        Target::Installation(InstallationId(0)).to_string(),
        "installation 0"
    );
    assert_eq!(
        Target::GroupAddress(GroupAddressId(7)).to_string(),
        "group address 7"
    );
}
#[test]
fn with_target_moves_a_note_and_keeps_identity_text_and_inclusion() {
    let note = note(4).with_documentation(true);
    let moved = note.clone().with_target(Target::Device(DeviceId(1)));
    assert_eq!(moved.id(), note.id());
    assert_eq!(moved.title(), note.title());
    assert_eq!(moved.body(), note.body());
    assert!(moved.include_in_documentation());
    assert_eq!(moved.target(), Target::Device(DeviceId(1)));
}
#[test]
fn explicit_note_disposition_and_target_delete_share_one_undo_step() {
    use knx_core::{Command, CommandStack};
    let mut project = target_project();
    project.installations[0].buildings[0].devices.clear();
    let n = ProjectNote::new(
        ProjectNoteId(1),
        Target::ComObject(ComObjectInstanceId(1)),
        "retain".into(),
        "literal".into(),
    )
    .unwrap();
    project.notes.push(n.clone());
    project.ids = project.ids.clone().with_project_note_count(1);
    let before = project.clone();
    let mut history = CommandStack::default();
    history
        .do_command(
            &mut project,
            Command::Batch(vec![
                Command::RemoveProjectNote { id: n.id() },
                Command::DeleteDevice { id: DeviceId(1) },
            ]),
        )
        .unwrap();
    assert!(project.notes.is_empty());
    assert!(project.devices.get(DeviceId(1)).is_none());
    history.undo(&mut project).unwrap();
    assert_eq!(project, before);
    history.redo(&mut project).unwrap();
    assert!(project.notes.is_empty());
}
#[test]
fn note_id_exhaustion_refuses_without_rewinding_or_panicking() {
    let mut ids = knx_core::IdAllocators::default().with_project_note_count(u32::MAX - 1);
    assert_eq!(ids.next_project_note_id().unwrap(), ProjectNoteId(u32::MAX));
    let before = ids.clone();
    assert!(ids.next_project_note_id().is_err());
    assert_eq!(ids, before);
}

#[test]
fn note_noop_preserves_existing_redo_and_adds_no_undo_step() {
    use knx_core::{Command, CommandStack};
    let mut project = target_project();
    project.notes.push(note(1));
    project.ids = project.ids.clone().with_project_note_count(1);
    let mut history = CommandStack::default();
    history
        .do_command(
            &mut project,
            Command::InsertProjectNote {
                note: note(2),
                position: 1,
            },
        )
        .unwrap();
    history.undo(&mut project).unwrap();
    let before = project.clone();
    let counts = history.history_lengths();
    history
        .do_command(&mut project, Command::ReplaceProjectNote { note: note(1) })
        .unwrap();
    assert_eq!(history.history_lengths(), counts);
    assert_eq!(project, before);
    history
        .do_command(
            &mut project,
            Command::MoveProjectNote {
                id: ProjectNoteId(1),
                position: 0,
            },
        )
        .unwrap();
    assert_eq!(history.history_lengths(), counts);
    history.redo(&mut project).unwrap();
    assert_eq!(project.notes, [note(1), note(2)]);
}
#[test]
fn failed_note_batch_preserves_originals_ids_and_both_history_stacks() {
    use knx_core::{Command, CommandStack};
    let mut project = target_project();
    project.notes.push(note(1));
    project.ids = project.ids.clone().with_project_note_count(1);
    project.instructions.insert(
        DeviceId(1),
        knx_core::instruction::InstructionDocument::new(vec![]).unwrap(),
    );
    let mut history = CommandStack::default();
    history
        .do_command(
            &mut project,
            Command::InsertProjectNote {
                note: note(2),
                position: 1,
            },
        )
        .unwrap();
    history.undo(&mut project).unwrap();
    let before = project.clone();
    let counts = history.history_lengths();
    let bad = ProjectNote::new(
        ProjectNoteId(5),
        Target::Device(DeviceId(99)),
        "bad".into(),
        String::new(),
    )
    .unwrap();
    assert!(history
        .do_command(
            &mut project,
            Command::Batch(vec![
                Command::RemoveProjectNote {
                    id: ProjectNoteId(1)
                },
                Command::InsertProjectNote {
                    note: bad,
                    position: 0
                }
            ])
        )
        .is_err());
    assert_eq!(project, before);
    assert_eq!(history.history_lengths(), counts);
    history.redo(&mut project).unwrap();
    assert_eq!(project.notes, [note(1), note(2)]);
}
