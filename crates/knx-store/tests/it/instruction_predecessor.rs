//! Authentic schema12 history fixture producer; never rewrites by default.
use knx_core::{CommandStack, Language, Project};
use knx_store::project_history::{self as history, HistoryAction, NativeSnapshot};

#[test]
#[ignore = "one-time opt-in producer, unchanged schema12 only; never overwrite frozen evidence"]
fn freeze_authentic_v12_instruction_history_predecessor() {
    assert_eq!(
        knx_store::CURRENT_SCHEMA_VERSION,
        12,
        "must use the predecessor writer"
    );
    let path = std::path::PathBuf::from(
        std::env::var_os("KNXBENCH_FREEZE_V12_INSTRUCTIONS")
            .expect("explicit output path required"),
    );
    assert!(!path.exists(), "frozen evidence is never overwritten");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let mut root = Project::new(Language("en".into()));
    root.info.project_id = "KB-SYNTHETIC-INSTRUCTIONS".into();
    root.info.name = "Saved predecessor".into();
    let id = root.ids.next_device_id().unwrap();
    root.devices.insert(knx_core::DeviceInstance {
        id,
        source: knx_core::SourceRef {
            path: "synthetic/0.xml".into(),
            ets_id: "SYNTHETIC-DI-1".into(),
        },
        name: "Synthetic device".into(),
        description: Some("Grüße predecessor".into()),
        address: None,
        product_ref: "synthetic product".into(),
        program_ref: "synthetic program".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    root.installations.push(knx_core::Installation {
        id: knx_core::InstallationId(0),
        name: "Synthetic installation".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::FinishedDesign,
        topology: knx_core::Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![id],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    let opaque = vec![knx_store::StoredOpaqueEntry {
        source_path: "synthetic/original.txt".into(),
        xpath: String::new(),
        kind: "ContainerEntry".into(),
        name: String::new(),
        bytes: b"retained original synthetic evidence".to_vec(),
        sha256: "4496c6b5bf370b9f37fec2d3dd0991c712157b86fe34027f5b4e76c7098f15f9".into(),
    }];
    let mut snapshot = NativeSnapshot {
        project: root.clone(),
        opaque,
        manufacturer_refs: vec![],
    };
    let generation =
        history::save_editor(&conn, &snapshot, &CommandStack::new(), None, true).unwrap();
    snapshot.project.info.name = "Working predecessor".into();
    let mut redo = root.clone();
    redo.info.name = "Redo predecessor".into();
    let stack = CommandStack::from_snapshot_states(vec![root], vec![redo]);
    let generation =
        history::save_editor(&conn, &snapshot, &stack, Some(generation), false).unwrap();
    history::change_history(
        &conn,
        &snapshot,
        generation,
        HistoryAction::Create("Named predecessor"),
    )
    .unwrap();
    let editor = history::load_editor(&conn).unwrap().unwrap();
    assert_eq!(editor.working.project.info.name, "Working predecessor");
    assert_eq!((editor.undo.len(), editor.redo.len()), (1, 1));
    let versions = history::list_versions(&conn).unwrap();
    assert_eq!(versions.len(), 1);
    assert_eq!(versions[0].label, "Named predecessor");
    conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM;")
        .unwrap();
    drop(conn);
}
