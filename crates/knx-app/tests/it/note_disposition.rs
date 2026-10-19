//! Note-aware deletes: every stranded note gets an explicit choice in one undo step.

use knx_app::note_disposition::{self as disposition, Choice, Deletion, PlanError};
use knx_core::note::{ProjectNote, ProjectNoteTarget as Target};
use knx_core::{
    Area, AreaId, BuildingPart, BuildingPartId, BuildingPartType, ComObjectInstance,
    ComObjectInstanceId, Command, CommandStack, CommissioningState, CompletionStatus, DeviceId,
    DeviceInstance, GroupAddress, GroupAddressEntry, GroupAddressId, GroupRange, GroupRangeId,
    Installation, InstallationId, Language, Line, LineId, Override, Project, ProjectNoteId,
    ResolvedFlags, SourceRef, Topology,
};

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: id.into(),
        ets_id: id.into(),
    }
}
fn line(id: u32, address: u8, devices: Vec<DeviceId>) -> Line {
    Line {
        id: LineId(id),
        source: source("line"),
        name: format!("line {id}"),
        address,
        medium_ref: "MT-0".into(),
        domain_address: None,
        domain_address_is_checked: None,
        ip_routing_multicast_address: None,
        multicast_ttl: None,
        completion: CompletionStatus::Editing,
        devices,
    }
}
fn room(id: u32) -> BuildingPart {
    BuildingPart {
        id: BuildingPartId(id),
        source: source("room"),
        name: format!("room {id}"),
        number: None,
        kind: BuildingPartType::Room,
        default_line: None,
        completion: CompletionStatus::Editing,
        children: vec![],
        devices: vec![],
        parent: None,
    }
}
fn group(id: u32) -> GroupAddressEntry {
    GroupAddressEntry {
        id: GroupAddressId(id),
        source: source("ga"),
        name: format!("group {id}"),
        address: GroupAddress::from_raw(u16::try_from(id).unwrap()),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Override::Absent,
    }
}
fn range(id: u32) -> GroupRange {
    GroupRange {
        id: GroupRangeId(id),
        source: source("range"),
        name: format!("range {id}"),
        start: GroupAddress::from_raw(0x0800 * u16::try_from(id).unwrap()),
        end: GroupAddress::from_raw(0x0800 * u16::try_from(id).unwrap() + 0x07ff),
        parent: None,
        children: vec![],
    }
}

/// Every deletable kind twice over: device 1 (object 1) and device 2 on
/// line 1, empty line 2, empty area 2, rooms 1 and 2, ranges 1 and 2,
/// group addresses 1 and 2. Nothing is linked, so every delete is admitted.
fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    for (device, object) in [(1, Some(1)), (2, None)] {
        project.devices.insert(DeviceInstance {
            id: DeviceId(device),
            source: source("device"),
            name: format!("device {device}"),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "A".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: object.map(ComObjectInstanceId).into_iter().collect(),
            binary_data: vec![],
        });
    }
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: source("object"),
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
        completion: CompletionStatus::Editing,
        topology: Topology {
            areas: vec![
                Area {
                    id: AreaId(1),
                    source: source("area"),
                    name: "area 1".into(),
                    address: 1,
                    completion: CompletionStatus::Editing,
                    lines: vec![LineId(1), LineId(2)],
                },
                Area {
                    id: AreaId(2),
                    source: source("area"),
                    name: "area 2".into(),
                    address: 2,
                    completion: CompletionStatus::Editing,
                    lines: vec![],
                },
            ],
            lines: vec![
                line(1, 1, vec![DeviceId(1), DeviceId(2)]),
                line(2, 2, vec![]),
            ],
            unassigned: vec![],
        },
        buildings: vec![room(1), room(2)],
        group_ranges: vec![range(1), range(2)],
        group_addresses: vec![group(1), group(2)],
        parameters: vec![],
    });
    project.ids = knx_core::project::IdAllocators::from_counts(2, 2, 2, 1, 2, 2, 2, 0, 0);
    project
}
fn with_notes(mut project: Project, notes: &[(u32, Target, &str)]) -> Project {
    for (id, target, title) in notes {
        project.notes.push(
            ProjectNote::new(
                ProjectNoteId(*id),
                *target,
                (*title).into(),
                "body\r\n".into(),
            )
            .unwrap()
            .with_documentation(*id % 2 == 0),
        );
    }
    let highest = notes.iter().map(|(id, ..)| *id).max().unwrap_or(0);
    project.ids = project.ids.clone().with_project_note_count(highest);
    project
}
fn ids(values: &[u32]) -> Vec<ProjectNoteId> {
    values.iter().copied().map(ProjectNoteId).collect()
}

/// Each delete kind with the note target it strands and a surviving target
/// of a different kind to move the note to.
fn kinds() -> Vec<(Deletion, Target, Target)> {
    vec![
        (
            Deletion::Devices(vec![DeviceId(2)]),
            Target::Device(DeviceId(2)),
            Target::Line(LineId(1)),
        ),
        (
            Deletion::GroupAddresses(vec![GroupAddressId(1)]),
            Target::GroupAddress(GroupAddressId(1)),
            Target::GroupAddress(GroupAddressId(2)),
        ),
        (
            Deletion::Area(AreaId(2)),
            Target::Area(AreaId(2)),
            Target::Area(AreaId(1)),
        ),
        (
            Deletion::Line(LineId(2)),
            Target::Line(LineId(2)),
            Target::Device(DeviceId(1)),
        ),
        (
            Deletion::BuildingPart(BuildingPartId(2)),
            Target::BuildingPart(BuildingPartId(2)),
            Target::BuildingPart(BuildingPartId(1)),
        ),
        (
            Deletion::GroupRange(GroupRangeId(2)),
            Target::GroupRange(GroupRangeId(2)),
            Target::ComObject(ComObjectInstanceId(1)),
        ),
    ]
}

#[test]
fn preview_names_the_stranded_note_for_every_delete_kind_and_changes_nothing() {
    for (deletion, stranded, _) in kinds() {
        let project = with_notes(
            project(),
            &[(1, stranded, "stranded"), (2, Target::Project, "safe")],
        );
        let before = project.clone();
        let preview = disposition::preview(&project, &deletion).unwrap();
        assert_eq!(preview.notes.len(), 1, "{deletion:?}");
        assert_eq!(preview.notes[0].id, ProjectNoteId(1));
        assert_eq!(preview.notes[0].target, stranded);
        assert_eq!(preview.notes[0].title, "stranded");
        assert!(preview.removed_targets.contains(&stranded), "{deletion:?}");
        assert_eq!(project, before);
    }
}

#[test]
fn a_device_delete_strands_the_notes_of_its_objects_too() {
    let project = with_notes(
        project(),
        &[
            (1, Target::ComObject(ComObjectInstanceId(1)), "object"),
            (2, Target::Device(DeviceId(2)), "other device"),
            (3, Target::Device(DeviceId(1)), "device"),
        ],
    );
    let preview = disposition::preview(&project, &Deletion::Devices(vec![DeviceId(1)])).unwrap();
    assert_eq!(
        preview.notes.iter().map(|n| n.id).collect::<Vec<_>>(),
        ids(&[1, 3])
    );
    assert_eq!(
        preview.removed_targets,
        [
            Target::Device(DeviceId(1)),
            Target::ComObject(ComObjectInstanceId(1)),
        ]
    );
}

#[test]
fn a_delete_without_noted_targets_previews_empty_and_plans_the_plain_delete() {
    let project = with_notes(project(), &[(1, Target::Project, "kept")]);
    let deletion = Deletion::GroupAddresses(vec![GroupAddressId(1), GroupAddressId(2)]);
    let preview = disposition::preview(&project, &deletion).unwrap();
    assert!(preview.notes.is_empty());
    assert!(
        preview.removed_targets.is_empty(),
        "no catalogue walk without a dialog"
    );
    let command = disposition::plan(&project, &deletion, &[]).unwrap();
    let mut after = project.clone();
    command.apply(&mut after).unwrap();
    assert!(after.installations[0].group_addresses.is_empty());
    assert_eq!(after.notes, project.notes);
}

#[test]
fn every_choice_for_every_kind_is_one_undo_step_with_exact_undo_and_redo() {
    for (deletion, stranded, elsewhere) in kinds() {
        for choice in [
            Choice::MoveTo(Target::Project),
            Choice::MoveTo(elsewhere),
            Choice::Delete,
        ] {
            let mut project = with_notes(
                project(),
                &[
                    (1, Target::Project, "first"),
                    (2, stranded, "stranded"),
                    (3, Target::Project, "last"),
                ],
            );
            let before = project.clone();
            let command =
                disposition::plan(&project, &deletion, &[(ProjectNoteId(2), choice)]).unwrap();
            let mut history = CommandStack::default();
            history.do_command(&mut project, command).unwrap();
            assert_eq!(history.history_lengths().0, 1, "{deletion:?} {choice:?}");
            assert!(stranded.validate(&project).is_err(), "{deletion:?}");
            match choice {
                Choice::MoveTo(target) => {
                    assert_eq!(project.notes.len(), 3);
                    let moved = &project.notes[1];
                    assert_eq!(moved.id(), ProjectNoteId(2));
                    assert_eq!(moved.target(), target);
                    assert_eq!(moved.title(), "stranded");
                    assert_eq!(moved.body(), "body\r\n");
                    assert!(moved.include_in_documentation());
                }
                Choice::Delete => {
                    assert_eq!(
                        project.notes.iter().map(|n| n.id()).collect::<Vec<_>>(),
                        ids(&[1, 3])
                    );
                }
            }
            let after = project.clone();
            history.undo(&mut project).unwrap();
            assert_eq!(project, before, "{deletion:?} {choice:?}");
            history.redo(&mut project).unwrap();
            assert_eq!(project, after, "{deletion:?} {choice:?}");
            assert_eq!(project.ids.peek_project_note(), 3);
        }
    }
}

#[test]
fn undo_restores_the_entity_before_its_notes() {
    let mut project = with_notes(project(), &[(1, Target::Device(DeviceId(2)), "device")]);
    let command = disposition::plan(
        &project,
        &Deletion::Devices(vec![DeviceId(2)]),
        &[(ProjectNoteId(1), Choice::Delete)],
    )
    .unwrap();
    let Command::Batch(steps) = &command else {
        panic!("one batch expected: {command:?}");
    };
    assert!(matches!(
        steps.first(),
        Some(Command::RemoveProjectNote { .. })
    ));
    assert!(matches!(steps.last(), Some(Command::DeleteDevice { .. })));
    let inverse = command.apply(&mut project).unwrap();
    let Command::Batch(undo) = inverse else {
        panic!("batch inverse expected");
    };
    assert!(
        matches!(undo.first(), Some(Command::RestoreDevice { .. })),
        "{undo:?}"
    );
    assert!(matches!(
        undo.last(),
        Some(Command::InsertProjectNote { .. })
    ));
}

#[test]
fn several_devices_and_notes_resolve_in_one_batch() {
    let mut project = with_notes(
        project(),
        &[
            (1, Target::Device(DeviceId(1)), "a"),
            (2, Target::ComObject(ComObjectInstanceId(1)), "b"),
            (3, Target::Device(DeviceId(2)), "c"),
        ],
    );
    let before = project.clone();
    let deletion = Deletion::Devices(vec![DeviceId(1), DeviceId(2)]);
    assert_eq!(
        disposition::preview(&project, &deletion)
            .unwrap()
            .notes
            .len(),
        3
    );
    let command = disposition::plan(
        &project,
        &deletion,
        &[
            (ProjectNoteId(3), Choice::MoveTo(Target::Line(LineId(1)))),
            (ProjectNoteId(1), Choice::Delete),
            (ProjectNoteId(2), Choice::MoveTo(Target::Project)),
        ],
    )
    .unwrap();
    let mut history = CommandStack::default();
    history.do_command(&mut project, command).unwrap();
    assert!(project.devices.get(DeviceId(1)).is_none());
    assert!(project.devices.get(DeviceId(2)).is_none());
    assert_eq!(
        project
            .notes
            .iter()
            .map(|n| (n.id(), n.target()))
            .collect::<Vec<_>>(),
        [
            (ProjectNoteId(2), Target::Project),
            (ProjectNoteId(3), Target::Line(LineId(1))),
        ]
    );
    history.undo(&mut project).unwrap();
    assert_eq!(project, before);
}

#[test]
fn incomplete_extra_duplicate_or_dangling_choices_are_refused_without_effect() {
    let project = with_notes(
        project(),
        &[
            (1, Target::Device(DeviceId(1)), "device"),
            (2, Target::ComObject(ComObjectInstanceId(1)), "object"),
            (3, Target::Project, "project"),
        ],
    );
    let before = project.clone();
    let deletion = Deletion::Devices(vec![DeviceId(1)]);
    let delete = |id| (ProjectNoteId(id), Choice::Delete);
    assert_eq!(
        disposition::plan(&project, &deletion, &[delete(1)]).unwrap_err(),
        PlanError::MissingChoice(ids(&[2]))
    );
    assert_eq!(
        disposition::plan(&project, &deletion, &[delete(1), delete(2), delete(3)]).unwrap_err(),
        PlanError::NotAffected(ProjectNoteId(3))
    );
    assert_eq!(
        disposition::plan(&project, &deletion, &[delete(1), delete(2), delete(1)]).unwrap_err(),
        PlanError::DuplicateChoice(ProjectNoteId(1))
    );
    assert_eq!(
        disposition::plan(
            &project,
            &deletion,
            &[
                (
                    ProjectNoteId(1),
                    Choice::MoveTo(Target::ComObject(ComObjectInstanceId(1)))
                ),
                delete(2),
            ],
        )
        .unwrap_err(),
        PlanError::RemovedTarget {
            note: ProjectNoteId(1),
            target: Target::ComObject(ComObjectInstanceId(1)),
        }
    );
    let missing = Target::Device(DeviceId(99));
    assert!(matches!(
        disposition::plan(
            &project,
            &deletion,
            &[(ProjectNoteId(1), Choice::MoveTo(missing)), delete(2)]
        )
        .unwrap_err(),
        PlanError::InvalidTarget {
            note: ProjectNoteId(1),
            ..
        }
    ));
    assert_eq!(project, before);
}

#[test]
fn malformed_deletions_and_refused_deletes_are_reported_as_such() {
    let project = with_notes(project(), &[(1, Target::Device(DeviceId(2)), "device")]);
    assert_eq!(
        disposition::plan(&project, &Deletion::Devices(vec![]), &[]).unwrap_err(),
        PlanError::EmptyDeletion
    );
    assert_eq!(
        disposition::plan(
            &project,
            &Deletion::Devices(vec![DeviceId(2), DeviceId(2)]),
            &[]
        )
        .unwrap_err(),
        PlanError::DuplicateTarget
    );
    // Deleting a non-empty area is refused by the core for its own reason,
    // which the plan passes through instead of inventing a note problem.
    let error = disposition::plan(&project, &Deletion::Area(AreaId(1)), &[]).unwrap_err();
    assert!(matches!(error, PlanError::Delete(_)), "{error:?}");
    assert!(disposition::preview(&project, &Deletion::Area(AreaId(1))).is_err());
}

#[test]
fn planned_dispositions_survive_save_and_reopen() {
    let mut project = with_notes(
        project(),
        &[
            (1, Target::ComObject(ComObjectInstanceId(1)), "moved"),
            (2, Target::Device(DeviceId(1)), "deleted"),
        ],
    );
    let command = disposition::plan(
        &project,
        &Deletion::Devices(vec![DeviceId(1)]),
        &[
            (ProjectNoteId(1), Choice::MoveTo(Target::Line(LineId(1)))),
            (ProjectNoteId(2), Choice::Delete),
        ],
    )
    .unwrap();
    command.apply(&mut project).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("dispositions.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let (reopened, _) = knx_store::load_project_reporting(&conn).unwrap();
    assert_eq!(reopened.notes, project.notes);
    assert_eq!(reopened.notes[0].target(), Target::Line(LineId(1)));
    assert_eq!(reopened.ids.peek_project_note(), 2);
    assert!(reopened.devices.get(DeviceId(1)).is_none());
}
