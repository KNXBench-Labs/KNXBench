//! Command persistence through the transactional whole-project-save fallback.
//!
//! This compatibility entry point is not an incremental persistence engine.
//! Server and CLI save paths use complete project saves directly; they do not
//! call this helper. Complete snapshot persistence avoids successful no-op
//! arms for structural commands and preserves allocator high-water marks.

use rusqlite::Connection;

use knx_core::command::Command;
use knx_core::project::Project;

use crate::StoreError;

/// Call only after a successful `Command::apply(&mut project)`, passing the
/// complete authoritative post-command snapshot. Undo/redo use the same
/// whole-project-save fallback. The command is retained in the public
/// signature for compatibility; it is not reapplied, inspected to choose
/// partial writes, or replayed recursively when it is a batch.
///
/// A storage failure rolls back the durable write, not the already-applied
/// in-memory command or its history: that recovery remains the caller's job.
/// This is not optimistic conflict detection; callers comparing an expected
/// saved state must use `save_project_if_unchanged` instead.
pub fn sync_after_command(
    conn: &Connection,
    project: &Project,
    _command: &Command,
) -> Result<(), StoreError> {
    crate::save_project(conn, project)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::open_and_migrate_in_memory;
    use knx_core::address::{GroupAddress, IndividualAddress};
    use knx_core::commissioning::{CommissioningState, CompletionStatus};
    use knx_core::device::DeviceInstance;
    use knx_core::group::GroupAddressEntry;
    use knx_core::ids::{DeviceId, InstallationId, SourceRef};
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
        crate::project::cover_ids_in_use(&mut project);
        project
    }

    #[test]
    fn command_sync_persists_a_previously_unsupported_parameter_edit() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_unassigned_device();
        crate::save_project(&conn, &project).unwrap();
        let command = Command::SetParameterValue {
            id: project.ids.next_parameter_instance_id().unwrap(),
            device: DeviceId(1),
            ets_id: "P-1".into(),
            raw: "7".into(),
        };
        command.apply(&mut project).unwrap();

        sync_after_command(&conn, &project, &command).unwrap();

        assert_eq!(crate::load_project(&conn).unwrap(), project);
    }

    #[test]
    fn undo_restores_an_imported_coupler_address_in_the_device_row() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = project_with_one_unassigned_device();
        let original = IndividualAddress::new(1, 1, 0).unwrap();
        project.devices.get_mut(DeviceId(1)).unwrap().address = Some(original);
        crate::save_project(&conn, &project).unwrap();
        let edit = Command::SetIndividualAddress {
            device: DeviceId(1),
            address: Some(IndividualAddress::new(1, 1, 18).unwrap()),
        };
        let restore = edit.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &edit).unwrap();
        restore.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &restore).unwrap();
        assert_eq!(
            crate::load_project(&conn)
                .unwrap()
                .devices
                .get(DeviceId(1))
                .unwrap()
                .address,
            Some(original)
        );
    }

    #[test]
    fn set_individual_address_persists_the_address() {
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

    /// The authoritative snapshot retains the second installation's membership;
    /// persistence must not route this edit into the first installation.
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
        crate::project::cover_ids_in_use(&mut project);
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
    fn create_and_delete_group_address_persist_through_whole_project_fallback() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(knx_core::string_table::Language("en".into()));
        project.installations.push(installation());
        crate::save_project(&conn, &project).unwrap();

        let ga_id = project.ids.next_group_address_id().unwrap();
        let entry = GroupAddressEntry {
            id: ga_id,
            source: source(),
            name: "New GA".into(),
            address: GroupAddress::from_raw(5),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        };
        let create = Command::CreateGroupAddress {
            entry: entry.clone(),
            installation: None,
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

    /// Project-wide metadata belongs to the complete saved snapshot too.
    #[test]
    fn set_group_address_style_persists_the_project_metadata() {
        let conn = open_and_migrate_in_memory().unwrap();
        let mut project = Project::new(knx_core::string_table::Language("en".into()));
        project.installations.push(installation());
        crate::save_project(&conn, &project).unwrap();

        let command = Command::SetGroupAddressStyle {
            style: knx_core::address::GroupAddressStyle::Free,
        };
        command.apply(&mut project).unwrap();
        sync_after_command(&conn, &project, &command).unwrap();

        let loaded = crate::load_project(&conn).unwrap();
        assert_eq!(
            loaded.info.group_address_style,
            knx_core::address::GroupAddressStyle::Free
        );
    }

    #[test]
    fn set_device_description_persists_the_description() {
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
    fn set_com_object_description_preserves_the_description_override_provenance() {
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
    fn set_com_object_dpt_preserves_the_dpt_override_provenance() {
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
