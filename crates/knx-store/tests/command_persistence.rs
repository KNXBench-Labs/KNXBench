//! Offline regression contract for command persistence, reopen and failure atomicity.
//!
//! Only the repository's synthetic archive is imported; no private corpus or bus.

use std::path::PathBuf;

use knx_core::{
    Area, BuildingPart, BuildingPartType, Command, CommandStack, CompletionStatus,
    DevicePlacementSlot, Direction, GroupAddress, GroupAddressEntry, IndividualAddress, Line,
    Project, SourceRef,
};
use knx_store::{
    insert_manufacturer_refs, insert_opaque, load_manufacturer_refs, load_opaque, load_project,
    open_and_migrate, save_project, sync_after_command, ManufacturerRef, StoredOpaqueEntry,
};

struct Fixture {
    _directory: tempfile::TempDir,
    path: PathBuf,
    project: Project,
    opaque: Vec<StoredOpaqueEntry>,
    manufacturers: Vec<ManufacturerRef>,
}

impl Fixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("command.knxdb");
        let project = knx_etsproj::import_knxproj_bytes(
            knx_testsupport::minimal_knxproj_bytes(),
            "synthetic.knxproj",
        )
        .unwrap()
        .project;
        let opaque = vec![StoredOpaqueEntry {
            source_path: "ar04/synthetic.bin".into(),
            xpath: String::new(),
            kind: "Entry".into(),
            name: "synthetic.bin".into(),
            bytes: b"\0AR04-opaque\xff\n".to_vec(),
            sha256: "2f455e8e89d1166aca73a851f5f1f159fb388dabe91b34aba38f1d7fd7029b10".into(),
        }];
        let manufacturers = vec![ManufacturerRef {
            source_path: opaque[0].source_path.clone(),
            sha256: opaque[0].sha256.clone(),
            len: opaque[0].bytes.len() as i64,
            kind: "SyntheticUninterpreted".into(),
        }];
        let conn = open_and_migrate(&path).unwrap();
        save_project(&conn, &project).unwrap();
        insert_opaque(&conn, &opaque).unwrap();
        insert_manufacturer_refs(&conn, &manufacturers).unwrap();
        assert_eq!(load_project(&conn).unwrap(), project);
        drop(conn);
        Self {
            _directory: directory,
            path,
            project,
            opaque,
            manufacturers,
        }
    }

    fn assert_reopened(&self, expected: &Project) {
        let conn = open_and_migrate(&self.path).unwrap();
        assert_eq!(&load_project(&conn).unwrap(), expected);
        assert_eq!(load_opaque(&conn).unwrap(), self.opaque);
        assert_eq!(load_manufacturer_refs(&conn).unwrap(), self.manufacturers);
    }

    fn sync(&self, command: &Command) {
        let conn = open_and_migrate(&self.path).unwrap();
        sync_after_command(&conn, &self.project, command).unwrap();
        drop(conn);
        self.assert_reopened(&self.project);
    }

    fn apply(&mut self, command: &Command) -> Command {
        let inverse = command.apply(&mut self.project).unwrap();
        self.sync(command);
        inverse
    }
}

fn source() -> SourceRef {
    SourceRef {
        path: "ar04/synthetic.xml".into(),
        ets_id: "synthetic".into(),
    }
}

#[test]
fn structural_nested_batch_and_history_reopen_the_complete_post_state() {
    let mut fixture = Fixture::new();
    let before = fixture.project.clone();
    let device = fixture.project.installations[0].topology.lines[0].devices[0];
    let com_object = fixture.project.devices.get(device).unwrap().com_objects[0];
    let area = fixture.project.ids.next_area_id().unwrap();
    let line = fixture.project.ids.next_line_id().unwrap();
    let part = fixture.project.ids.next_building_part_id().unwrap();
    let ga = fixture.project.ids.next_group_address_id().unwrap();
    let parameter = fixture.project.ids.next_parameter_instance_id().unwrap();
    let range = fixture.project.installations[0].group_ranges[1].id;
    let command = Command::Batch(vec![
        Command::CreateArea {
            area: Area {
                id: area,
                source: source(),
                name: "Second area".into(),
                address: 2,
                completion: CompletionStatus::Undefined,
                lines: vec![],
            },
            installation: None,
        },
        Command::CreateLine {
            area,
            line: Line {
                id: line,
                source: source(),
                name: "Second line".into(),
                address: 1,
                medium_ref: "MT-0".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: CompletionStatus::Undefined,
                devices: vec![],
            },
        },
        Command::CreateBuildingPart {
            part: BuildingPart {
                id: part,
                source: source(),
                name: "Room".into(),
                number: None,
                kind: BuildingPartType::Room,
                default_line: Some(line),
                completion: CompletionStatus::Undefined,
                children: vec![],
                devices: vec![],
                parent: None,
            },
            installation: None,
        },
        Command::Batch(vec![
            Command::SetIndividualAddress {
                device,
                address: None,
            },
            Command::MoveDeviceToLine {
                device,
                line: Some(line),
            },
            Command::SetIndividualAddress {
                device,
                address: Some(IndividualAddress::new(2, 1, 1).unwrap()),
            },
            Command::MoveDeviceToBuildingPart {
                device,
                part: Some(part),
            },
        ]),
        Command::CreateGroupAddress {
            entry: GroupAddressEntry {
                id: ga,
                source: source(),
                name: "Second GA".into(),
                address: GroupAddress::from_raw(7),
                central: true,
                unfiltered: false,
                range: Some(range),
                declared_dpt: Default::default(),
            },
            installation: None,
        },
        Command::LinkComObject {
            com_object,
            ga,
            direction: Direction::Receive,
        },
        Command::SetParameterValue {
            id: parameter,
            device,
            ets_id: "P-AR04".into(),
            raw: "7".into(),
        },
    ]);
    let mut stack = CommandStack::new();
    stack
        .do_command(&mut fixture.project, command.clone())
        .unwrap();
    let after = fixture.project.clone();
    fixture.sync(&command);
    stack.undo(&mut fixture.project).unwrap();
    assert!(fixture.project.same_user_content_as(&before));
    assert_eq!(fixture.project.ids, after.ids);
    fixture.sync(&command);
    stack.redo(&mut fixture.project).unwrap();
    assert_eq!(fixture.project, after);
    fixture.sync(&command);
}

#[test]
fn deleting_and_restoring_a_middle_group_address_preserves_sibling_order() {
    let mut fixture = Fixture::new();
    let first = fixture.project.ids.next_group_address_id().unwrap();
    let second = fixture.project.ids.next_group_address_id().unwrap();
    for (id, address) in [(first, 7), (second, 8)] {
        fixture.apply(&Command::CreateGroupAddress {
            entry: GroupAddressEntry {
                id,
                source: source(),
                name: "Sibling".into(),
                address: GroupAddress::from_raw(address),
                central: false,
                unfiltered: false,
                range: None,
                declared_dpt: Default::default(),
            },
            installation: None,
        });
    }
    let before = fixture.project.clone();
    let restore = fixture.apply(&Command::DeleteGroupAddress { id: first });
    let redo = fixture.apply(&restore);
    assert_eq!(fixture.project, before);
    fixture.apply(&redo);
    fixture.apply(&restore);
    assert_eq!(fixture.project, before);
}

#[test]
fn a_late_sql_failure_preserves_the_saved_state_but_not_the_already_applied_memory_edit() {
    let mut fixture = Fixture::new();
    let before = fixture.project.clone();
    let device = fixture.project.installations[0].topology.lines[0].devices[0];
    let command = Command::SetParameterValue {
        id: fixture.project.ids.next_parameter_instance_id().unwrap(),
        device,
        ets_id: "P-AR04".into(),
        raw: "7".into(),
    };
    let inverse = command.apply(&mut fixture.project).unwrap();
    let applied = fixture.project.clone();
    let conn = open_and_migrate(&fixture.path).unwrap();
    conn.execute_batch(
        "CREATE TRIGGER ar04_failure BEFORE INSERT ON parameter_instance
         BEGIN SELECT RAISE(ABORT, 'ar04 injected failure'); END;",
    )
    .unwrap();
    let error = sync_after_command(&conn, &fixture.project, &command).unwrap_err();
    assert!(error.to_string().contains("ar04 injected failure"));
    assert_eq!(fixture.project, applied);
    drop(conn);
    fixture.assert_reopened(&before);
    assert!(!fixture.project.same_user_content_as(&before));
    let conn = open_and_migrate(&fixture.path).unwrap();
    conn.execute_batch("DROP TRIGGER ar04_failure").unwrap();
    drop(conn);
    fixture.apply(&inverse);
    assert!(fixture.project.same_user_content_as(&before));
}

/// MODEL-02: the schema holds one placement per device, so an ambiguous
/// topology is refused on save (the saved file stays as it was) instead of
/// silently keeping the last placement. After an explicit repair the
/// project saves and reopens exactly; undoing the repair is refused again.
#[test]
fn an_ambiguous_topology_is_refused_on_save_and_saves_after_repair() {
    let mut fixture = Fixture::new();
    let saved = fixture.project.clone();
    let device = fixture.project.installations[0].topology.lines[0].devices[0];
    let line = fixture.project.installations[0].topology.lines[0].id;
    fixture.project.installations[0]
        .topology
        .unassigned
        .push(device);
    let conn = open_and_migrate(&fixture.path).unwrap();
    let error = save_project(&conn, &fixture.project).unwrap_err();
    assert!(
        matches!(&error, knx_store::StoreError::AmbiguousTopology { devices, lines }
            if devices == &[device] && lines.is_empty()),
        "{error}"
    );
    drop(conn);
    fixture.assert_reopened(&saved);

    let ambiguous = fixture.project.clone();
    let inverse = fixture.apply(&Command::RepairDevicePlacement {
        device,
        keep: DevicePlacementSlot::Line(line),
    });
    assert_eq!(
        fixture.project, saved,
        "keeping the line restores the clean state"
    );
    inverse.apply(&mut fixture.project).unwrap();
    assert_eq!(fixture.project, ambiguous);
    let conn = open_and_migrate(&fixture.path).unwrap();
    assert!(save_project(&conn, &fixture.project).is_err());
    drop(conn);
    fixture.assert_reopened(&saved);
}

/// A line listed by two areas is refused the same way.
#[test]
fn a_line_with_two_owners_is_refused_on_save() {
    let fixture = Fixture::new();
    let mut project = fixture.project.clone();
    let line = project.installations[0].topology.lines[0].id;
    let mut second = project.installations[0].topology.areas[0].clone();
    second.id = project.ids.next_area_id().unwrap();
    second.address = 9;
    project.installations[0].topology.areas.push(second);
    let conn = open_and_migrate(&fixture.path).unwrap();
    let error = save_project(&conn, &project).unwrap_err();
    assert!(
        matches!(&error, knx_store::StoreError::AmbiguousTopology { devices, lines }
            if devices.is_empty() && lines == &[line]),
        "{error}"
    );
    drop(conn);
    fixture.assert_reopened(&fixture.project);
}
