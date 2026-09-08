//! Incremental persistence: after `Command::apply(&mut project)` succeeds,
//! `sync_after_command` writes only the row(s) that command's own target
//! id(s) name, reading the resulting state out of the already-mutated
//! `project` rather than re-deriving `command.rs`'s own mutation logic
//! (design doc, "Incremental command sync"). Grows as `command.rs` grows —
//! every `Command` variant has a match arm here, but only the original
//! eight (device address/description, com-object DPT/description and their
//! undo/redo forms, group-address create/delete) actually persist; the rest
//! (topology/group-range/group-link variants, plus device create/delete)
//! are no-op stubs awaiting a future incremental-sync pass (see each arm's
//! own "persistence layer not yet implemented" comment).

use rusqlite::Connection;

use knx_core::command::Command;
use knx_core::ids::InstallationId;
use knx_core::project::Project;

use crate::devices::{
    set_device_line, upsert_com_object_description_override, upsert_com_object_dpt_override,
    upsert_device,
};
use crate::group::{delete_group_address, upsert_group_address};
use crate::StoreError;

/// Call only after a successful `Command::apply(&mut project)`, passing the
/// resulting `project`. Applies equally to undo/redo, since both replay
/// through this same `Command` enum — a `RestoreComObjectDpt` produced by
/// undoing a `SetComObjectDpt` is itself a `Command`, synced the same way.
///
/// The owning installation is never a parameter: for a device it is read
/// back off the device's own stored row, and for a group address it is
/// `installations[0]`, the only installation `Command::apply` itself ever
/// touches (`command.rs` uses `installations.first_mut()`). A caller
/// passing the wrong one would silently move a device — or file a new group
/// address — into another installation, so there is nothing to pass.
/// `SetIndividualAddress` does not move a device between lines either, so
/// its existing line/position is looked up and re-asserted rather than
/// changed.
pub fn sync_after_command(
    conn: &Connection,
    project: &Project,
    command: &Command,
) -> Result<(), StoreError> {
    let tx = conn.unchecked_transaction()?;
    match command {
        Command::SetIndividualAddress { device, .. } => {
            let d = project
                .devices
                .get(*device)
                .expect("Command::apply already proved this device exists");
            let (installation_id, line_id, position): (u8, Option<i64>, i64) = tx.query_row(
                "SELECT installation_id, line_id, topology_position FROM device WHERE id = ?1",
                [d.id.0],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
            upsert_device(&tx, InstallationId(installation_id), position, d)?;
            set_device_line(
                &tx,
                d.id,
                line_id.map(|l| knx_core::ids::LineId(l as u32)),
                position,
            )?;
        }
        Command::SetDeviceDescription { device, .. } => {
            let d = project
                .devices
                .get(*device)
                .expect("Command::apply already proved this device exists");
            let (installation_id, line_id, position): (u8, Option<i64>, i64) = tx.query_row(
                "SELECT installation_id, line_id, topology_position FROM device WHERE id = ?1",
                [d.id.0],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )?;
            upsert_device(&tx, InstallationId(installation_id), position, d)?;
            set_device_line(
                &tx,
                d.id,
                line_id.map(|l| knx_core::ids::LineId(l as u32)),
                position,
            )?;
        }
        Command::SetComObjectDpt { com_object, .. }
        | Command::RestoreComObjectDpt { com_object, .. } => {
            let com = project
                .devices
                .com_object(*com_object)
                .expect("Command::apply already proved this com object exists");
            upsert_com_object_dpt_override(&tx, com.id, &com.dpt)?;
        }
        Command::SetComObjectDescription { com_object, .. }
        | Command::RestoreComObjectDescription { com_object, .. } => {
            let com = project
                .devices
                .com_object(*com_object)
                .expect("Command::apply already proved this com object exists");
            upsert_com_object_description_override(&tx, com.id, &com.description)?;
        }
        Command::CreateGroupAddress { entry } => {
            // `Command::apply` pushes onto `installations.first_mut()`, so
            // that is the installation the new row belongs to — deriving it
            // here is what keeps the two from ever disagreeing.
            let installation_id = project
                .installations
                .first()
                .expect("Command::apply already proved this project has an installation")
                .id;
            let position: i64 = tx
                .query_row(
                    "SELECT COALESCE(MAX(position) + 1, 0) FROM group_address WHERE installation_id = ?1",
                    [installation_id.0],
                    |row| row.get(0),
                )?;
            upsert_group_address(&tx, installation_id, position, entry)?;
        }
        Command::DeleteGroupAddress { id } => {
            delete_group_address(&tx, *id)?;
        }
        Command::CreateDevice { .. } => {
            // Device create/delete persistence layer not yet implemented
            // (out of scope for this plan's Task 1, which only added the
            // `Command` variants and in-memory `apply` logic).
        }
        Command::DeleteDevice { .. } => {
            // Device create/delete persistence layer not yet implemented
            // (out of scope for this plan's Task 1, which only added the
            // `Command` variants and in-memory `apply` logic).
        }
        Command::CreateArea { .. } => {
            // Area persistence layer not yet implemented (Task 2 scope).
        }
        Command::DeleteArea { .. } => {
            // Area persistence layer not yet implemented (Task 2 scope).
        }
        Command::CreateLine { .. } => {
            // Line persistence layer not yet implemented (Task 3 scope).
        }
        Command::DeleteLine { .. } => {
            // Line persistence layer not yet implemented (Task 3 scope).
        }
        Command::MoveDeviceToLine { .. } => {
            // Line/device-membership persistence layer not yet implemented (Task 4 scope).
        }
        Command::CreateGroupRange { .. } => {
            // Group-range persistence layer not yet implemented (Task 5 scope).
        }
        Command::DeleteGroupRange { .. } => {
            // Group-range persistence layer not yet implemented (Task 5 scope).
        }
        Command::RenameGroupRange { .. } => {
            // Group-range persistence layer not yet implemented (Task 5 scope).
        }
        Command::LinkComObject { .. } => {
            // Group-link persistence layer not yet implemented (Task 6 scope).
        }
        Command::UnlinkComObject { .. } => {
            // Group-link persistence layer not yet implemented (Task 6 scope).
        }
        Command::SetComObjectFlag { .. } | Command::RestoreComObjectFlag { .. } => {
            // Com-object-flag persistence layer not yet implemented.
        }
    }
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use knx_core::address::{GroupAddress, IndividualAddress};
    use knx_core::commissioning::{CommissioningState, CompletionStatus};
    use knx_core::device::DeviceInstance;
    use knx_core::group::GroupAddressEntry;
    use knx_core::ids::{DeviceId, SourceRef};
    use knx_core::installation::Installation;
    use knx_core::topology::Topology;

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn installation() -> Installation {
        Installation {
            id: InstallationId(0),
            name: "H".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
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

    fn device_one() -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(1),
            source: source(),
            name: "D".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    fn project_with_one_unassigned_device() -> Project {
        let mut project = Project::new(knx_core::string_table::Language("en".into()));
        let mut installation = installation();
        installation.topology.unassigned = vec![DeviceId(1)];
        project.installations.push(installation);
        project.devices.insert(device_one());
        project
    }

    #[test]
    fn set_individual_address_syncs_only_the_device_row() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_unassigned_device();
        crate::save_project(&conn, &project).unwrap();

        let command = Command::SetIndividualAddress {
            device: DeviceId(1),
            address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
        };
        command.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &command).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        assert_eq!(
            loaded.devices.get(DeviceId(1)).unwrap().address,
            Some(IndividualAddress::new(1, 1, 1).unwrap())
        );
    }

    /// A device's installation is read back off its own stored row rather
    /// than taken from the caller, so a device belonging to a second
    /// installation stays there. With the removed `installation_id`
    /// parameter, a caller passing the wrong value silently moved the
    /// device: `upsert_device`'s
    /// `ON CONFLICT ... installation_id = excluded.installation_id`
    /// overwrote it without complaint.
    #[test]
    fn set_individual_address_keeps_a_device_in_its_own_installation() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(knx_core::string_table::Language("en".into()));
        project.installations.push(installation()); // InstallationId(0), empty
        let mut second = installation();
        second.id = InstallationId(1);
        second.topology.unassigned = vec![DeviceId(1)];
        project.installations.push(second);
        project.devices.insert(device_one());
        crate::save_project(&conn, &project).unwrap();

        let command = Command::SetIndividualAddress {
            device: DeviceId(1),
            address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
        };
        command.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &command).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        assert_eq!(
            loaded.installations[1].topology.unassigned,
            vec![DeviceId(1)]
        );
        assert!(loaded.installations[0].topology.unassigned.is_empty());
        assert_eq!(loaded, project);
    }

    #[test]
    fn create_and_delete_group_address_sync_incrementally() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(knx_core::string_table::Language("en".into()));
        project.installations.push(installation());
        crate::save_project(&conn, &project).unwrap();

        let ga_id = project.ids.next_group_address_id();
        let entry = GroupAddressEntry {
            id: ga_id,
            source: source(),
            name: "New GA".into(),
            address: GroupAddress::from_raw(5),
            central: false,
            unfiltered: false,
            range: None,
        };
        let create = Command::CreateGroupAddress {
            entry: entry.clone(),
        };
        create.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &create).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        assert_eq!(loaded.installations[0].group_addresses, vec![entry]);

        let delete = Command::DeleteGroupAddress { id: ga_id };
        delete.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &delete).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        assert_eq!(loaded.installations[0].group_addresses, vec![]);
    }

    #[test]
    fn set_device_description_syncs_only_the_device_row() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_unassigned_device();
        crate::save_project(&conn, &project).unwrap();

        let command = Command::SetDeviceDescription {
            device: DeviceId(1),
            description: Some("Schaltaktor Keller".into()),
        };
        command.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &command).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        assert_eq!(
            loaded.devices.get(DeviceId(1)).unwrap().description,
            Some("Schaltaktor Keller".to_string())
        );
    }

    #[test]
    fn set_com_object_description_syncs_only_the_description_override_row() {
        use knx_core::device::ComObjectInstance;
        use knx_core::flags::ResolvedFlags;
        use knx_core::ids::ComObjectInstanceId;
        use knx_core::provenance::Override;

        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_unassigned_device();
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
        project
            .devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));
        crate::save_project(&conn, &project).unwrap();

        let set = Command::SetComObjectDescription {
            com_object: ComObjectInstanceId(1),
            description: Some("Aktoreingang 1".into()),
        };
        let inverse = set.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &set).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        let com = loaded.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(
            com.description.value().unwrap().value,
            knx_core::string_table::Text::Literal("Aktoreingang 1".into())
        );

        // Undo replays through the same mechanism.
        inverse.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &inverse).unwrap();
        let loaded = crate::load_project(&conn).unwrap();
        let com = loaded.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(com.description, Override::Absent);
    }

    #[test]
    fn set_com_object_dpt_syncs_only_the_dpt_override_row() {
        use knx_core::device::ComObjectInstance;
        use knx_core::dpt::DptRef;
        use knx_core::flags::ResolvedFlags;
        use knx_core::ids::ComObjectInstanceId;
        use knx_core::provenance::Override;

        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_unassigned_device();
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
        project
            .devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));
        crate::save_project(&conn, &project).unwrap();

        let set = Command::SetComObjectDpt {
            com_object: ComObjectInstanceId(1),
            dpt: Some(DptRef {
                main: 1,
                sub: Some(1),
            }),
        };
        let inverse = set.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &set).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        let com = loaded.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(com.dpt.value().is_some());

        // Undo replays through the same mechanism.
        inverse.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &inverse).unwrap();
        let loaded = crate::load_project(&conn).unwrap();
        let com = loaded.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(com.dpt, Override::Absent);
    }
}
