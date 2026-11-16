//! Verifies device configuration profiles in versioned native storage and
//! through undoable domain commands (ADR-0175 decision 5, WP-42.2).

use knx_core::devconf::DeviceConfigProfile;
use knx_core::ids::SourceRef;
use knx_core::{
    Area, AreaId, Command, CommandError, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Line, LineId, Project,
};
use knx_store::project_history::NativeSnapshot;

fn profile(device: u32, module: &str) -> DeviceConfigProfile {
    DeviceConfigProfile::new(
        DeviceId(device),
        module,
        "1.0.0",
        &"a".repeat(64),
        b"{\"nodes\": []}".to_vec(),
    )
    .unwrap()
}

fn project_with_device() -> Project {
    let mut project = Project::new(Language("en".into()));
    let source = SourceRef {
        path: "synthetic".into(),
        ets_id: "D-1".into(),
    };
    project.installations.push(Installation {
        id: InstallationId(1),
        name: "Installation".into(),
        default_line: None,
        multicast_address: None,
        completion: knx_core::CompletionStatus::Undefined,
        topology: knx_core::Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: source.clone(),
                name: "Area".into(),
                address: 1,
                completion: knx_core::CompletionStatus::Undefined,
                lines: vec![LineId(1)],
            }],
            lines: vec![Line {
                id: LineId(1),
                source,
                name: "Line".into(),
                address: 1,
                medium_ref: "MT-1".into(),
                domain_address: None,
                domain_address_is_checked: None,
                ip_routing_multicast_address: None,
                multicast_ttl: None,
                completion: knx_core::CompletionStatus::Undefined,
                devices: vec![DeviceId(1)],
            }],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "D-1".into(),
        },
        name: "Device 1".into(),
        description: None,
        address: None,
        product_ref: "P-1".into(),
        program_ref: "H-1".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
        .ids
        .raise_to(&knx_core::IdAllocators::from_counts(2, 2, 2, 1, 1, 1, 1, 1, 1));
    project
}

#[test]
fn profile_round_trips_through_native_save_load_and_snapshot() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("profile.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    let mut project = project_with_device();
    Command::SetDeviceConfigProfile {
        profile: profile(1, "example.demo-panel"),
    }
    .apply(&mut project)
    .unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    assert_eq!(knx_store::load_project(&conn).unwrap(), project);

    let snapshot = NativeSnapshot::read(&conn).unwrap();
    let bytes = snapshot.encode().unwrap();
    assert_eq!(
        NativeSnapshot::decode(&bytes, &knx_store::project_history::image_hash(&bytes)).unwrap(),
        snapshot
    );

    drop(conn);
    let reopened = knx_store::open_existing_read_only(&path).unwrap();
    let loaded = knx_store::load_project(&reopened.conn).unwrap();
    assert_eq!(loaded, project);
    assert_eq!(
        reopened
            .conn
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        knx_store::CURRENT_SCHEMA_VERSION
    );
    assert_eq!(
        loaded
            .devices
            .config_profile(DeviceId(1))
            .unwrap()
            .module_id(),
        "example.demo-panel"
    );
}

#[test]
fn set_replace_and_remove_are_undoable() {
    let mut project = project_with_device();
    let mut stack = knx_core::CommandStack::new();

    stack
        .do_command(
            &mut project,
            Command::SetDeviceConfigProfile {
                profile: profile(1, "example.demo-panel"),
            },
        )
        .unwrap();
    assert!(project.devices.config_profile(DeviceId(1)).is_some());

    // Replace: inverse restores the previous profile.
    stack
        .do_command(
            &mut project,
            Command::SetDeviceConfigProfile {
                profile: profile(1, "example.other"),
            },
        )
        .unwrap();
    assert_eq!(
        project
            .devices
            .config_profile(DeviceId(1))
            .unwrap()
            .module_id(),
        "example.other"
    );
    stack.undo(&mut project).unwrap();
    assert_eq!(
        project
            .devices
            .config_profile(DeviceId(1))
            .unwrap()
            .module_id(),
        "example.demo-panel"
    );

    // Remove: inverse restores the exact profile.
    stack
        .do_command(
            &mut project,
            Command::RemoveDeviceConfigProfile { device: DeviceId(1) },
        )
        .unwrap();
    assert!(project.devices.config_profile(DeviceId(1)).is_none());
    stack.undo(&mut project).unwrap();
    assert_eq!(
        project
            .devices
            .config_profile(DeviceId(1))
            .unwrap()
            .module_id(),
        "example.demo-panel"
    );

    // Redo after undo works through the same mechanism.
    stack.redo(&mut project).unwrap();
    assert!(project.devices.config_profile(DeviceId(1)).is_none());
}

#[test]
fn invalid_commands_are_refused_without_touching_the_project() {
    let mut project = project_with_device();
    let before = project.clone();

    // Unknown device.
    let mut bad = profile(1, "example.demo-panel");
    bad.device = DeviceId(99);
    assert_eq!(
        Command::SetDeviceConfigProfile { profile: bad }.apply(&mut project),
        Err(CommandError::DeviceNotFound(DeviceId(99)))
    );
    assert_eq!(project, before);

    // Invalid profile content (empty document).
    let invalid = DeviceConfigProfile {
        device: DeviceId(1),
        module_id: "example.demo-panel".into(),
        module_version: "1.0.0".into(),
        module_sha256: "a".repeat(64),
        document: vec![],
    };
    assert!(matches!(
        Command::SetDeviceConfigProfile {
            profile: invalid
        }
        .apply(&mut project),
        Err(CommandError::InvalidConfigProfile(_))
    ));
    assert_eq!(project, before);

    // Removing a profile that does not exist.
    assert_eq!(
        Command::RemoveDeviceConfigProfile { device: DeviceId(1) }.apply(&mut project),
        Err(CommandError::ConfigProfileNotFound(DeviceId(1)))
    );
    assert_eq!(project, before);
}

#[test]
fn profile_survives_device_delete_and_undo() {
    let mut project = project_with_device();
    let mut stack = knx_core::CommandStack::new();
    stack
        .do_command(
            &mut project,
            Command::SetDeviceConfigProfile {
                profile: profile(1, "example.demo-panel"),
            },
        )
        .unwrap();
    stack
        .do_command(&mut project, Command::DeleteDevice { id: DeviceId(1) })
        .unwrap();
    assert!(project.devices.config_profile(DeviceId(1)).is_none());
    stack.undo(&mut project).unwrap();
    assert_eq!(
        project
            .devices
            .config_profile(DeviceId(1))
            .unwrap()
            .module_id(),
        "example.demo-panel"
    );
}

#[test]
fn invalid_stored_profile_refuses_load_without_changing_saved_bytes() {
    let conn = knx_store::open_and_migrate_in_memory().unwrap();
    let project = project_with_device();
    knx_store::save_project(&conn, &project).unwrap();
    // Inject an over-budget document directly, bypassing the model check.
    conn.execute(
        "INSERT INTO device_config_profile (device_id, module_id, module_version, module_sha256, document) VALUES (1, 'm', '1.0.0', ?1, ?2)",
        rusqlite::params!["a".repeat(64), vec![0u8; 2_000_000]],
    )
    .unwrap();
    assert!(knx_store::load_project(&conn).is_err());
}

#[test]
fn authentic_native21_project_migrates_to_v22_without_profiles() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("predecessor.knxdb");
    std::fs::write(
        &path,
        include_bytes!("../fixtures/native21-before-devconf-profile.sqlite"),
    )
    .unwrap();
    let read = knx_store::open_existing_read_only(&path).unwrap();
    assert_eq!(read.migrated_from, Some(21));
    // The read-only opener migrates an in-memory copy, not the fixture file.
    let project = knx_store::load_project(&read.conn).unwrap();
    // A pre-v22 project has no profiles; absence means "no configuration",
    // never an empty document.
    assert!(project.devices.config_profiles().next().is_none());
    assert_eq!(
        read.conn
            .query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        knx_store::CURRENT_SCHEMA_VERSION
    );
    // And the migrated copy re-saves through a fresh writable copy exactly.
    let dir2 = tempfile::tempdir().unwrap();
    let path2 = dir2.path().join("migrated.knxdb");
    std::fs::write(&path2, std::fs::read(&path).unwrap()).unwrap();
    let writable = knx_store::open_and_migrate(&path2).unwrap();
    // The fixture's own history predates this save; save the migrated model
    // on a fresh store instead of over the fixture's history.
    let fresh = knx_store::open_and_migrate(&dir2.path().join("fresh.knxdb")).unwrap();
    knx_store::save_project(&fresh, &project).unwrap();
    assert_eq!(knx_store::load_project(&fresh).unwrap(), project);
    drop(writable);
}
