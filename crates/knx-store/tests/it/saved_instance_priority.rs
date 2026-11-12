//! Verifies canonical native priority rows, semantic identity, history and atomic refusal.

use knx_core::*;
use knx_store::{
    project_history::{self as history, NativeSnapshot},
    StoreError,
};

fn source() -> SavedPrioritySource {
    SavedPrioritySource { source_schema:23,
        device_source:SourceRef {path:"P-SYNTHETIC/0.xml".into(),ets_id:"P-SYNTHETIC-0_DI-1".into()},
        object_source:SourceRef {path:"P-SYNTHETIC/0.xml".into(),ets_id:"O-7_R-1".into()},
        owner_xpath:"/KNX/Project/Installations/Installation/Topology/UnassignedDevices/DeviceInstance[@Id='P-SYNTHETIC-0_DI-1']/ComObjectInstanceRefs/ComObjectInstanceRef[@RefId='O-7_R-1']".into(),
        topology_sha256:"a".repeat(64) }
}
fn declared(raw: &str) -> SavedInstancePriority {
    SavedInstancePriority::Declared(Box::new(SavedPriorityDeclared {
        raw: raw.into(),
        source: source(),
    }))
}
fn seed(fact: SavedInstancePriority) -> NativeSnapshot {
    let mut p = Project::new(Language("en".into()));
    let owner = p.ids.next_device_id().unwrap();
    let id = p.ids.next_com_object_instance_id().unwrap();
    p.devices.insert(DeviceInstance {
        id: owner,
        source: source().device_source,
        name: "Saved synthetic".into(),
        description: None,
        address: None,
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![id],
        binary_data: vec![],
    });
    p.devices.insert_com_object(ComObjectInstance {
        id,
        device: owner,
        source: source().object_source,
        number: 7,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
        priority_edit: None,
        saved_instance_priority: fact,
    });
    p.installations.push(Installation {
        id: InstallationId(0),
        name: "Synthetic".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![owner],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    NativeSnapshot {
        project: p,
        opaque: vec![knx_store::StoredOpaqueEntry {
            source_path: "P-SYNTHETIC/0.xml".into(),
            xpath: String::new(),
            kind: "ContainerEntry".into(),
            name: String::new(),
            bytes: b"synthetic opaque original".to_vec(),
            sha256: "b".repeat(64),
        }],
        manufacturer_refs: vec![],
    }
}
fn facts() -> Vec<SavedInstancePriority> {
    let mut values = vec![
        Default::default(),
        declared("Low"),
        declared(""),
        declared(" future Ω "),
    ];
    for omission in [
        SavedPriorityOmission::AttributeOmitted,
        SavedPriorityOmission::InstanceElementOmitted,
    ] {
        values.push(SavedInstancePriority::Absent(Box::new(
            SavedPriorityAbsent {
                omission,
                source: source(),
            },
        )));
    }
    for reason in [
        SavedPriorityUnavailable::UnsupportedSourceSchema,
        SavedPriorityUnavailable::ModuleScope,
        SavedPriorityUnavailable::AmbiguousOwner,
        SavedPriorityUnavailable::AmbiguousDeclaration,
        SavedPriorityUnavailable::SourceIntegrity,
        SavedPriorityUnavailable::ResourceLimit,
    ] {
        values.push(SavedInstancePriority::Unavailable { reason });
    }
    values
}

#[test]
fn saved_instance_priority_native_roundtrip_single_batch_and_rebound() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("native.knxdb");
    for value in facts() {
        let mut snapshot = seed(value.clone());
        // Native stores original observations even if their current applicability changed.
        let owner = snapshot.project.devices.iter().next().unwrap().id;
        snapshot.project.devices.get_mut(owner).unwrap().source.path = "scoped/0.xml".into();
        let conn = knx_store::open_and_migrate(&path).unwrap();
        knx_store::save_project_with_passthrough(&conn, &snapshot.project, &snapshot.opaque, &[])
            .unwrap();
        drop(conn);
        for _ in 0..2 {
            let conn = knx_store::open_existing_and_migrate(&path).unwrap();
            let read = NativeSnapshot::read(&conn).unwrap();
            assert!(read == snapshot);
            let id = read.project.devices.com_objects().next().unwrap().id;
            assert_eq!(
                knx_store::devices::load_com_object_instance(&conn, id)
                    .unwrap()
                    .saved_instance_priority,
                value
            );
            let rows: i64 = conn
                .query_row("SELECT count(*) FROM com_object_saved_priority", [], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(rows, i64::from(value != SavedInstancePriority::default()));
            knx_store::save_project(&conn, &read.project).unwrap();
        }
    }
}

#[test]
fn saved_instance_priority_invalid_native_rows_sql_and_bypass_decoder() {
    let snapshot = seed(declared("High"));
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    knx_store::save_project(&conn, &snapshot.project).unwrap();
    let id = snapshot.project.devices.com_objects().next().unwrap().id;
    for set in ["state='future'","reason='Future'","reason='NotCaptured',state='unavailable',raw=NULL,source_schema=NULL,device_source_path=NULL,device_source_ets_id=NULL,object_source_path=NULL,object_source_ets_id=NULL,owner_xpath=NULL,topology_sha256=NULL",
        "state='unavailable',reason='Future',raw=NULL,source_schema=NULL,device_source_path=NULL,device_source_ets_id=NULL,object_source_path=NULL,object_source_ets_id=NULL,owner_xpath=NULL,topology_sha256=NULL",
        "raw=NULL","omission='AttributeOmitted'","source_schema=NULL","source_schema=24","device_source_path=NULL",
        "device_source_ets_id=''","object_source_path='different'","object_source_ets_id=NULL","owner_xpath=NULL",
        "topology_sha256=NULL","topology_sha256=upper(topology_sha256)","topology_sha256=replace(topology_sha256,'a','g')",
        "state='absent',raw=NULL,omission=NULL","state='absent',raw=NULL,omission='Future'",
        "state='unavailable',reason=NULL,raw=NULL,source_schema=NULL,device_source_path=NULL,device_source_ets_id=NULL,object_source_path=NULL,object_source_ets_id=NULL,owner_xpath=NULL,topology_sha256=NULL"] {
        conn.execute_batch("SAVEPOINT bad;").unwrap();
        assert!(conn.execute(&format!("UPDATE com_object_saved_priority SET {set}"),[]).is_err(),"SQL accepted {set}");
        conn.execute_batch("ROLLBACK TO bad; RELEASE bad;").unwrap();
        conn.execute_batch("PRAGMA ignore_check_constraints=ON; SAVEPOINT bad;").unwrap();
        conn.execute(&format!("UPDATE com_object_saved_priority SET {set}"),[]).unwrap();
        assert!(knx_store::load_project(&conn).is_err(),"batch accepted {set}");
        assert!(knx_store::devices::load_com_object_instance(&conn,id).is_err(),"single accepted {set}");
        conn.execute_batch("ROLLBACK TO bad; RELEASE bad; PRAGMA ignore_check_constraints=OFF;").unwrap();
        assert!(knx_store::load_project(&conn).unwrap()==snapshot.project);
    }
    assert!(conn
        .execute(
            "UPDATE com_object_saved_priority SET com_object_instance_id=999",
            []
        )
        .is_err());
    conn.execute_batch(
        "PRAGMA foreign_keys=OFF; UPDATE com_object_saved_priority SET com_object_instance_id=999;",
    )
    .unwrap();
    assert!(knx_store::load_project(&conn).is_err());
}

#[test]
fn saved_instance_priority_preflight_before_direct_mutation_and_transaction_rollback() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let baseline = seed(declared("High"));
    let g = history::save_editor(&conn, &baseline, &CommandStack::new(), None, true).unwrap();
    let before = history::load_editor(&conn).unwrap().unwrap();
    let mut changed = baseline.clone();
    let id = changed.project.devices.com_objects().next().unwrap().id;
    let object = changed.project.devices.com_object_mut(id).unwrap();
    object.number = 99;
    if let SavedInstancePriority::Declared(value) = &mut object.saved_instance_priority {
        value.source.topology_sha256 = "bad".into();
    }
    let object = changed.project.devices.com_object(id).unwrap();
    assert!(matches!(
        knx_store::devices::upsert_com_object_instance(&conn, object.device, 0, object),
        Err(StoreError::SavedPriorityIntegrity)
    ));
    assert!(NativeSnapshot::read(&conn).unwrap() == baseline);
    assert!(knx_store::save_project_with_passthrough(&conn, &changed.project, &[], &[]).is_err());
    assert!(NativeSnapshot::read(&conn).unwrap() == baseline);
    let mut valid = baseline.clone();
    valid.project.info.name = "Must roll back".into();
    conn.execute_batch("CREATE TRIGGER deny_saved_priority BEFORE INSERT ON com_object_saved_priority BEGIN SELECT RAISE(ABORT,'synthetic refusal'); END;").unwrap();
    assert!(knx_store::save_project_with_passthrough(&conn, &valid.project, &[], &[]).is_err());
    conn.execute_batch("DROP TRIGGER deny_saved_priority;")
        .unwrap();
    assert!(NativeSnapshot::read(&conn).unwrap() == baseline);
    let after = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(after.generation, g);
    assert_eq!(before.generation, after.generation);
    assert!(
        before.baseline == after.baseline
            && before.working == after.working
            && before.undo == after.undo
            && before.redo == after.redo
    );
}

#[test]
fn saved_instance_priority_fingerprints_all_semantic_columns_and_history_recovery() {
    let baseline = seed(declared("High"));
    let original = baseline.semantic_hash().unwrap();
    for fact in facts() {
        if fact != declared("High") {
            assert_ne!(seed(fact).semantic_hash().unwrap(), original);
        }
    }
    let mut source_changed = baseline.clone();
    let id = source_changed
        .project
        .devices
        .com_objects()
        .next()
        .unwrap()
        .id;
    if let SavedInstancePriority::Declared(value) = &mut source_changed
        .project
        .devices
        .com_object_mut(id)
        .unwrap()
        .saved_instance_priority
    {
        value.source.device_source.ets_id.push('X');
    }
    assert_ne!(source_changed.semantic_hash().unwrap(), original);
    for value in facts() {
        let snapshot = seed(value);
        let bytes = snapshot.encode().unwrap();
        assert!(NativeSnapshot::decode(&bytes, &history::image_hash(&bytes)).unwrap() == snapshot);
    }
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let g = history::save_editor(&conn, &baseline, &CommandStack::new(), None, true).unwrap();
    let mut working = baseline.clone();
    let mut stack = CommandStack::new();
    let device = working.project.devices.iter().next().unwrap().id;
    stack
        .do_command(
            &mut working.project,
            Command::RenameDevice {
                device,
                name: "Working".into(),
            },
        )
        .unwrap();
    stack
        .do_command(
            &mut working.project,
            Command::SetGroupAddressStyle {
                style: GroupAddressStyle::Free,
            },
        )
        .unwrap();
    stack.undo(&mut working.project).unwrap();
    let expected = stack.snapshot_states(&working.project).unwrap();
    assert_eq!(expected.0.len(), 1);
    assert_eq!(expected.1.len(), 1);
    let g = history::save_editor(&conn, &working, &stack, Some(g), false).unwrap();
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&path).unwrap();
    let recovered = history::load_editor(&conn).unwrap().unwrap();
    assert!(recovered.baseline == baseline && recovered.working == working);
    assert!(recovered.undo == expected.0 && recovered.redo == expected.1);
    let mut replay = CommandStack::from_snapshot_states(recovered.undo, recovered.redo);
    replay.redo(&mut working.project).unwrap();
    assert_eq!(
        working.project.info.group_address_style,
        GroupAddressStyle::Free
    );
    replay.undo(&mut working.project).unwrap();
    assert!(working == recovered.working);
    let change = history::change_history(
        &conn,
        &working,
        g,
        history::HistoryAction::Create("Priority witness"),
    )
    .unwrap();
    assert!(change.generation > g);
}

#[test]
fn saved_instance_priority_default_removes_existing_row_without_touching_opaque() {
    let snapshot = seed(declared("High"));
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    knx_store::save_project_with_passthrough(&conn, &snapshot.project, &snapshot.opaque, &[])
        .unwrap();
    let mut object = snapshot
        .project
        .devices
        .com_objects()
        .next()
        .unwrap()
        .clone();
    object.saved_instance_priority = Default::default();
    knx_store::devices::upsert_com_object_instance(&conn, object.device, 0, &object).unwrap();
    assert_eq!(
        conn.query_row("SELECT count(*) FROM com_object_saved_priority", [], |r| {
            r.get::<_, i64>(0)
        })
        .unwrap(),
        0
    );
    assert_eq!(
        knx_store::devices::load_com_object_instance(&conn, object.id)
            .unwrap()
            .saved_instance_priority,
        SavedInstancePriority::default()
    );
    assert_eq!(knx_store::load_opaque(&conn).unwrap(), snapshot.opaque);
}

#[test]
fn saved_instance_priority_snapshot_hash_unknown_shape_and_future_schema_refuse() {
    let snapshot = seed(SavedInstancePriority::default());
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    knx_store::save_project(&conn, &snapshot.project).unwrap();
    conn.execute_batch("DROP TABLE device_parameter_baseline_value; DROP TABLE device_parameter_baseline; DROP TABLE group_address_description; DROP TABLE com_object_priority_edit; DROP TABLE com_object_saved_priority; PRAGMA user_version=16;")
        .unwrap();
    let original = conn.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
    assert!(NativeSnapshot::decode(&original, &history::image_hash(&original)).is_ok());
    assert!(NativeSnapshot::decode(&original, &"0".repeat(64)).is_err());
    for ddl in [
        "CREATE TABLE unexpected (id INTEGER) STRICT;",
        "ALTER TABLE com_object_instance ADD COLUMN unexpected TEXT;",
        "PRAGMA user_version=18;",
    ] {
        let mut bad = rusqlite::Connection::open_in_memory().unwrap();
        bad.deserialize_read_exact(
            rusqlite::MAIN_DB,
            original.as_slice(),
            original.len(),
            false,
        )
        .unwrap();
        bad.execute_batch(ddl).unwrap();
        let image = bad.serialize(rusqlite::MAIN_DB).unwrap().to_vec();
        assert!(
            NativeSnapshot::decode(&image, &history::image_hash(&image)).is_err(),
            "rehashing must not admit {ddl}"
        );
    }
}

#[test]
fn saved_instance_priority_batch_reader_uses_one_metadata_query_for_many_objects() {
    use std::{
        ffi::{c_void, CStr},
        sync::atomic::{AtomicUsize, Ordering},
    };
    unsafe extern "C" fn count_statement(
        kind: u32,
        context: *mut c_void,
        statement: *mut c_void,
        _: *mut c_void,
    ) -> i32 {
        if kind == rusqlite::ffi::SQLITE_TRACE_STMT {
            // SQLite supplies a live sqlite3_stmt for this synchronous trace callback.
            let sql = unsafe { rusqlite::ffi::sqlite3_sql(statement.cast()) };
            if !sql.is_null()
                && unsafe { CStr::from_ptr(sql) }
                    .to_bytes()
                    .windows(b"FROM com_object_saved_priority".len())
                    .any(|s| s == b"FROM com_object_saved_priority")
            {
                // The counter outlives the registered callback and is only accessed atomically.
                unsafe { &*context.cast::<AtomicUsize>() }.fetch_add(1, Ordering::Relaxed);
            }
        }
        0
    }
    for count in [1, 128] {
        let mut snapshot = seed(declared("High"));
        let template = snapshot
            .project
            .devices
            .com_objects()
            .next()
            .unwrap()
            .clone();
        for n in 1..count {
            let mut com = template.clone();
            com.id = snapshot.project.ids.next_com_object_instance_id().unwrap();
            com.number = n as u16;
            com.source.ets_id = format!("O-{n}_R-99");
            if let SavedInstancePriority::Declared(value) = &mut com.saved_instance_priority {
                value.source.object_source = com.source.clone();
            }
            snapshot
                .project
                .devices
                .get_mut(com.device)
                .unwrap()
                .com_objects
                .push(com.id);
            snapshot.project.devices.insert_com_object(com);
        }
        let conn = knx_store::open_and_migrate_in_memory().unwrap();
        knx_store::save_project(&conn, &snapshot.project).unwrap();
        let counter = AtomicUsize::new(0);
        // The connection is not shared; unregister before dropping the stack-owned counter.
        let status = unsafe {
            rusqlite::ffi::sqlite3_trace_v2(
                conn.handle(),
                rusqlite::ffi::SQLITE_TRACE_STMT,
                Some(count_statement),
                (&counter as *const AtomicUsize).cast_mut().cast(),
            )
        };
        assert_eq!(status, rusqlite::ffi::SQLITE_OK);
        let loaded = knx_store::load_project(&conn);
        unsafe {
            rusqlite::ffi::sqlite3_trace_v2(conn.handle(), 0, None, std::ptr::null_mut());
        }
        assert!(loaded.unwrap() == snapshot.project);
        eprintln!(
            "saved_instance_priority batch-query: objects={count} metadata_queries={}",
            counter.load(Ordering::Relaxed)
        );
        assert_eq!(
            counter.load(Ordering::Relaxed),
            1,
            "one metadata batch query, independent of object count {count}"
        );
    }
}
