//! Exercises typed note integrity without IO or changing project schema.

use knx_core::note::{
    validate_notes, NoteError, ProjectNote, ProjectNoteTarget as Target, MAX_NOTES_TEXT_BYTES,
    MAX_NOTE_BODY_BYTES, MAX_NOTE_TITLE_BYTES, MAX_PROJECT_NOTES,
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
