//! Native schema 15: project Functions persist exactly, older files gain
//! empty tables, history images keep their identity, and malformed rows are
//! refused instead of repaired by guessing (ADR-0124).

use knx_core::function::{Function, FunctionDetails, FunctionMember};
use knx_core::{
    BuildingPart, BuildingPartId, BuildingPartType, Command, CompletionStatus, FunctionId,
    FunctionMemberId, GroupAddress, GroupAddressEntry, GroupAddressId, Installation,
    InstallationId, Language, Project, SourceRef, Topology,
};
use knx_store::project_history::{self as history, NativeSnapshot};

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: "P-0001/0.xml".into(),
        ets_id: id.into(),
    }
}

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    let mut installations = Vec::new();
    for (index, (room, addresses)) in [(1u32, [1u32, 2]), (2, [3, 4])].into_iter().enumerate() {
        installations.push(Installation {
            id: InstallationId(index as u8),
            name: format!("I{index}"),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::Undefined,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![BuildingPart {
                id: BuildingPartId(room),
                source: source(&format!("BP-{room}")),
                name: format!("Room {room}"),
                number: None,
                kind: BuildingPartType::Room,
                default_line: None,
                completion: CompletionStatus::Undefined,
                children: vec![],
                devices: vec![],
                parent: None,
            }],
            group_ranges: vec![],
            group_addresses: addresses
                .iter()
                .map(|&id| GroupAddressEntry {
                    id: GroupAddressId(id),
                    source: source(&format!("GA-{id}")),
                    name: format!("G{id}"),
                    address: GroupAddress::from_raw(id as u16),
                    central: false,
                    unfiltered: false,
                    range: None,
                    declared_dpt: Default::default(),
                })
                .collect(),
            parameters: vec![],
        });
    }
    project.installations = installations;
    project.ids.raise_to(&knx_core::IdAllocators::from_counts(
        0, 0, 0, 0, 0, 4, 2, 0, 0,
    ));
    project
}

fn with_functions() -> Project {
    let mut project = project();
    let mut ids = project.ids.clone();
    let light = ids.next_function_id().unwrap();
    let blind = ids.next_function_id().unwrap();
    let members: Vec<_> = (0..3)
        .map(|_| ids.next_function_member_id().unwrap())
        .collect();
    project.ids = ids;
    for function in [
        Function {
            id: light,
            source: source("F-1"),
            location: BuildingPartId(1),
            details: FunctionDetails {
                // Imported text is stored verbatim, control characters too.
                name: " Living\u{7}light ".into(),
                function_type: Some("FT-1".into()),
                number: Some("1".into()),
                comment: Some("line 1\r\nline 2".into()),
                description: Some(String::new()),
                completion: CompletionStatus::FinishedDesign,
            },
            members: vec![
                FunctionMember {
                    id: members[0],
                    source: source("GF-1"),
                    group_address: GroupAddressId(2),
                    name: String::new(),
                    role: Some("SwitchOnOff".into()),
                },
                FunctionMember {
                    id: members[1],
                    source: source("GF-2"),
                    group_address: GroupAddressId(1),
                    name: "Status".into(),
                    role: None,
                },
            ],
        },
        Function {
            id: blind,
            source: SourceRef {
                path: String::new(),
                ets_id: String::new(),
            },
            location: BuildingPartId(2),
            details: FunctionDetails {
                name: "Blind".into(),
                ..Default::default()
            },
            members: vec![FunctionMember {
                id: members[2],
                source: source("GF-3"),
                group_address: GroupAddressId(4),
                name: String::new(),
                role: None,
            }],
        },
    ] {
        Command::RestoreFunction {
            position: project.functions.len(),
            function,
        }
        .apply(&mut project)
        .unwrap();
    }
    project
}

#[test]
fn functions_text_order_and_allocators_survive_save_reopen_and_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("functions.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let project = with_functions();
    knx_store::save_project(&conn, &project).unwrap();
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);
    let snapshot = NativeSnapshot::read(&conn).unwrap();
    let bytes = snapshot.encode().unwrap();
    assert_eq!(
        NativeSnapshot::decode(&bytes, &history::image_hash(&bytes)).unwrap(),
        snapshot
    );
    drop(conn);
    let reopened = knx_store::open_existing_read_only(&path).unwrap();
    let loaded = knx_store::load_project(&reopened.conn).unwrap();
    assert_eq!(loaded, project);
    assert_eq!(
        (
            loaded.ids.peek_function(),
            loaded.ids.peek_function_member()
        ),
        (2, 3)
    );
    assert_eq!(
        reopened
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
}

#[test]
fn a_deleted_function_does_not_rewind_its_allocators() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let mut project = with_functions();
    Command::DeleteFunction { id: FunctionId(2) }
        .apply(&mut project)
        .unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    let mut loaded = knx_store::load_project(&conn).unwrap();
    assert_eq!(loaded.ids.next_function_id().unwrap(), FunctionId(3));
    assert_eq!(
        loaded.ids.next_function_member_id().unwrap(),
        FunctionMemberId(4)
    );
}

#[test]
fn a_project_without_functions_keeps_its_semantic_identity_after_the_v15_tables_appear() {
    // The identity of a function-free image must not depend on whether the
    // empty v15 tables exist, or every predecessor history image would
    // suddenly look changed. A schema-14 shape is made by dropping the v15
    // tables, then the real upgrade path recreates them.
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v14.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &project()).unwrap();
    let plain = NativeSnapshot::read(&conn)
        .unwrap()
        .semantic_hash()
        .unwrap();
    conn.execute_batch(
        "DROP TABLE project_function_member; DROP TABLE project_function; \
         DROP TABLE project_function_state; PRAGMA user_version=14;",
    )
    .unwrap();
    drop(conn);
    let upgraded = knx_store::open_existing_and_migrate(&path).unwrap();
    assert_eq!(
        upgraded
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        15
    );
    assert_eq!(
        NativeSnapshot::read(&upgraded)
            .unwrap()
            .semantic_hash()
            .unwrap(),
        plain
    );
    let with = knx_store::open_and_migrate_in_memory().unwrap();
    knx_store::save_project(&with, &with_functions()).unwrap();
    assert_ne!(
        NativeSnapshot::read(&with)
            .unwrap()
            .semantic_hash()
            .unwrap(),
        plain
    );
}

#[test]
fn authentic_schema13_files_gain_empty_function_tables_and_keep_their_history() {
    for bytes in [
        include_bytes!("fixtures/native13-before-notes-saved.sqlite").as_slice(),
        include_bytes!("fixtures/native13-before-notes-working.sqlite").as_slice(),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("predecessor.knxdb");
        std::fs::write(&path, bytes).unwrap();
        let read = knx_store::open_existing_read_only(&path).unwrap();
        let root = NativeSnapshot::read(&read.conn).unwrap();
        assert!(root.project.functions.is_empty());
        let editor = history::load_editor(&read.conn).unwrap().unwrap();
        assert!(editor.working.project.functions.is_empty());
        drop(read);
        assert_eq!(
            std::fs::read(&path).unwrap(),
            bytes,
            "reading never upgrades"
        );
        let migrated = knx_store::open_existing_and_migrate(&path).unwrap();
        let state: (i64, i64, i64) = migrated
            .query_row(
                "SELECT (SELECT count(*) FROM project_function),\
                        (SELECT count(*) FROM project_function_member),\
                        (SELECT last_function_id + last_member_id FROM project_function_state)",
                [],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();
        assert_eq!(state, (0, 0, 0));
        assert_eq!(
            history::load_editor(&migrated).unwrap().unwrap().working,
            editor.working
        );
        assert_eq!(NativeSnapshot::read(&migrated).unwrap(), root);
    }
}

#[test]
fn malformed_function_rows_refuse_without_guessing() {
    for mutation in [
        "UPDATE project_function SET position=7 WHERE id=1",
        "UPDATE project_function_member SET position=5 WHERE id=1",
        // Location in the other installation than the members.
        "UPDATE project_function SET location=2 WHERE id=1",
        // A member pointing into the other installation.
        "UPDATE project_function_member SET group_address=3 WHERE id=1",
        "UPDATE project_function SET completion='Sometime' WHERE id=1",
        "DELETE FROM project_function_state",
        "INSERT INTO project_function_state(id,last_function_id,last_member_id) VALUES(1,0,0)",
    ] {
        let conn = knx_store::open_and_migrate_in_memory().unwrap();
        knx_store::save_project(&conn, &with_functions()).unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints=ON; PRAGMA foreign_keys=OFF")
            .unwrap();
        conn.execute_batch(mutation).unwrap();
        let bytes = conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
        assert!(knx_store::load_project(&conn).is_err(), "{mutation}");
        assert_eq!(conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec(), bytes);
    }
}

#[test]
fn a_stale_function_counter_goes_through_the_explicit_repair_report() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    knx_store::save_project(&conn, &with_functions()).unwrap();
    conn.execute(
        "UPDATE project_function_state SET last_function_id=0,last_member_id=1",
        [],
    )
    .unwrap();
    let (mut loaded, report) = knx_store::load_project_reporting(&conn).unwrap();
    let repair = report.expect("a stale counter is reported, not hidden");
    assert_eq!(repair.stored.peek_function(), 0);
    assert_eq!(repair.repaired.peek_function(), 2);
    assert_eq!(repair.repaired.peek_function_member(), 3);
    assert_eq!(loaded.ids.next_function_id().unwrap(), FunctionId(3));
}

#[test]
fn an_inconsistent_function_set_is_not_saved() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let mut project = with_functions();
    project.functions[0].members[0].group_address = GroupAddressId(3);
    let error = knx_store::save_project(&conn, &project).unwrap_err();
    assert!(
        matches!(error, knx_store::StoreError::FunctionIntegrity(_)),
        "{error}"
    );
}
