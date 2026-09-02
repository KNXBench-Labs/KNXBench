//! The command layer: every mutation is a `Command` whose `apply` returns
//! its own inverse, so undo and redo replay the same mechanism (ARCHITECTURE
//! §6). Validation lives here, not in the UI and not in storage. Any command
//! that changes a `Resolved<T>` sets its layer to `Layer::UserEdit`.

use std::fmt;

use crate::device::ComObjectInstance;
use crate::dpt::DptRef;
use crate::group::GroupAddressEntry;
use crate::ids::{ComObjectInstanceId, DeviceId, GroupAddressId};
use crate::project::Project;
use crate::provenance::{Layer, Resolved};
use crate::validation::{check_no_duplicate_individual_address, ValidationError};
use crate::IndividualAddress;

/// A single reversible mutation. `apply` performs the mutation on
/// `installations[0]` — the model supports multiple installations, but no
/// current command targets any other; routing a command to a specific
/// installation is future work, not silently assumed solved here.
#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    SetIndividualAddress {
        device: DeviceId,
        address: Option<IndividualAddress>,
    },
    /// Sets a communication object instance's datapoint type as a user
    /// edit. Always resolves to `Layer::UserEdit` — use `RestoreComObjectDpt`
    /// to put back an exact prior `Resolved<DptRef>` (that is what undo
    /// does; this variant is not it).
    SetComObjectDpt {
        com_object: ComObjectInstanceId,
        dpt: Option<DptRef>,
    },
    /// The undo/redo form of `SetComObjectDpt`: restores an exact resolved
    /// value, layer included. Not constructed directly by UI code — a
    /// `Layer::Program` value undone back into place must stay
    /// `Layer::Program`, not become `Layer::UserEdit` again, so this cannot
    /// share `SetComObjectDpt`'s bare-`DptRef` shape.
    RestoreComObjectDpt {
        com_object: ComObjectInstanceId,
        dpt: Option<Resolved<DptRef>>,
    },
    /// `entry.id` is pre-allocated by the caller via
    /// `Project::ids::next_group_address_id`.
    CreateGroupAddress {
        entry: GroupAddressEntry,
    },
    DeleteGroupAddress {
        id: GroupAddressId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    Validation(ValidationError),
    DeviceNotFound(DeviceId),
    ComObjectNotFound(ComObjectInstanceId),
    GroupAddressNotFound(GroupAddressId),
    InstallationNotFound,
    NothingToUndo,
    NothingToRedo,
}

impl std::error::Error for CommandError {}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Validation(e) => write!(f, "{e}"),
            CommandError::DeviceNotFound(id) => write!(f, "device {id} not found"),
            CommandError::ComObjectNotFound(id) => {
                write!(f, "communication object instance {id} not found")
            }
            CommandError::GroupAddressNotFound(id) => write!(f, "group address {id} not found"),
            CommandError::InstallationNotFound => write!(f, "project has no installation"),
            CommandError::NothingToUndo => write!(f, "nothing to undo"),
            CommandError::NothingToRedo => write!(f, "nothing to redo"),
        }
    }
}

impl From<ValidationError> for CommandError {
    fn from(e: ValidationError) -> Self {
        CommandError::Validation(e)
    }
}

impl Command {
    /// Applies the command to `project`, returning its inverse on success.
    /// On failure, `project` is left untouched.
    pub fn apply(&self, project: &mut Project) -> Result<Command, CommandError> {
        match self {
            Command::SetIndividualAddress { device, address } => {
                let device = *device;
                let address = *address;
                let previous = project
                    .devices
                    .get(device)
                    .ok_or(CommandError::DeviceNotFound(device))?
                    .address;
                if let Some(addr) = address {
                    check_no_duplicate_individual_address(&project.devices, device, addr)?;
                }
                project.devices.get_mut(device).unwrap().address = address;
                Ok(Command::SetIndividualAddress {
                    device,
                    address: previous,
                })
            }
            Command::SetComObjectDpt { com_object, dpt } => {
                let com_object = *com_object;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.dpt;
                com.dpt = dpt.map(|value| Resolved {
                    value,
                    layer: Layer::UserEdit,
                });
                Ok(Command::RestoreComObjectDpt {
                    com_object,
                    dpt: previous,
                })
            }
            Command::RestoreComObjectDpt { com_object, dpt } => {
                let com_object = *com_object;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.dpt;
                com.dpt = *dpt;
                Ok(Command::RestoreComObjectDpt {
                    com_object,
                    dpt: previous,
                })
            }
            Command::CreateGroupAddress { entry } => {
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let id = entry.id;
                installation.group_addresses.push(entry.clone());
                Ok(Command::DeleteGroupAddress { id })
            }
            Command::DeleteGroupAddress { id } => {
                let id = *id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let pos = installation
                    .group_addresses
                    .iter()
                    .position(|e| e.id == id)
                    .ok_or(CommandError::GroupAddressNotFound(id))?;
                let entry = installation.group_addresses.remove(pos);
                Ok(Command::CreateGroupAddress { entry })
            }
        }
    }
}

/// Undo/redo stacks of applied commands' inverses. A failed `do_command`
/// leaves both stacks untouched.
#[derive(Debug, Default)]
pub struct CommandStack {
    undo: Vec<Command>,
    redo: Vec<Command>,
}

impl CommandStack {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn do_command(&mut self, project: &mut Project, cmd: Command) -> Result<(), CommandError> {
        let inverse = cmd.apply(project)?;
        self.undo.push(inverse);
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let cmd = self.undo.pop().ok_or(CommandError::NothingToUndo)?;
        let inverse = cmd.apply(project)?;
        self.redo.push(inverse);
        Ok(())
    }

    pub fn redo(&mut self, project: &mut Project) -> Result<(), CommandError> {
        let cmd = self.redo.pop().ok_or(CommandError::NothingToRedo)?;
        let inverse = cmd.apply(project)?;
        self.undo.push(inverse);
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::BuildingPart;
    use crate::commissioning::{CommissioningState, CompletionStatus};
    use crate::device::DeviceInstance;
    use crate::flags::ComFlags;
    use crate::flags::ObjectSize;
    use crate::group::GroupRange;
    use crate::ids::{InstallationId, SourceRef};
    use crate::installation::Installation;
    use crate::string_table::{LocalizedString, TranslationKey};
    use crate::topology::Topology;
    use crate::{GroupAddress, Language};

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn test_project_with_one_device(address: Option<IndividualAddress>) -> Project {
        let mut p = Project::new(Language("en".into()));
        p.installations.push(Installation {
            id: InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: Vec::<BuildingPart>::new(),
            group_ranges: Vec::<GroupRange>::new(),
            group_addresses: vec![],
            parameters: vec![],
        });
        p.devices.insert(DeviceInstance {
            id: DeviceId(1),
            source: source(),
            name: "D".into(),
            description: None,
            address,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
        });
        p
    }

    #[test]
    fn set_individual_address_then_undo_restores_previous_value() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let addr = IndividualAddress::new(1, 1, 1).unwrap();
        stack
            .do_command(
                &mut project,
                Command::SetIndividualAddress {
                    device: DeviceId(1),
                    address: Some(addr),
                },
            )
            .unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(addr)
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(project.devices.get(DeviceId(1)).unwrap().address, None);
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().address,
            Some(addr)
        );
    }

    #[test]
    fn set_individual_address_rejects_duplicate() {
        let mut project =
            test_project_with_one_device(Some(IndividualAddress::new(1, 1, 1).unwrap()));
        project.devices.insert(DeviceInstance {
            id: DeviceId(2),
            source: source(),
            name: "D2".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
        });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::SetIndividualAddress {
                device: DeviceId(2),
                address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DuplicateIndividualAddress { .. }
            ))
        ));
        assert!(!stack.can_undo());
    }

    #[test]
    fn create_then_delete_group_address_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let entry = GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "GA".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: None,
        };
        stack
            .do_command(
                &mut project,
                Command::CreateGroupAddress {
                    entry: entry.clone(),
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].group_addresses.len(), 1);
        stack
            .do_command(
                &mut project,
                Command::DeleteGroupAddress {
                    id: GroupAddressId(1),
                },
            )
            .unwrap();
        assert!(project.installations[0].group_addresses.is_empty());
        stack.undo(&mut project).unwrap(); // undoes the delete -> recreates
        assert_eq!(project.installations[0].group_addresses.len(), 1);
        stack.undo(&mut project).unwrap(); // undoes the create -> empty again
        assert!(project.installations[0].group_addresses.is_empty());
    }

    #[test]
    fn set_com_object_dpt_marks_layer_as_user_edit_and_undoes() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Resolved {
                value: LocalizedString(TranslationKey("t".into())),
                layer: Layer::Program,
            },
            description: None,
            dpt: Some(Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::Program,
            }),
            flags: Resolved {
                value: ComFlags {
                    read: true,
                    write: false,
                    transmit: false,
                    update: false,
                    communication: true,
                },
                layer: Layer::Program,
            },
            size: Resolved {
                value: ObjectSize::Bit(1),
                layer: Layer::Program,
            },
            is_active: true,
            links: vec![],
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        let new_dpt = DptRef {
            main: 5,
            sub: Some(1),
        };
        stack
            .do_command(
                &mut project,
                Command::SetComObjectDpt {
                    com_object: ComObjectInstanceId(1),
                    dpt: Some(new_dpt),
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(updated.dpt.as_ref().unwrap().value, new_dpt);
        assert_eq!(updated.dpt.as_ref().unwrap().layer, Layer::UserEdit);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(
            restored.dpt.as_ref().unwrap().value,
            DptRef {
                main: 1,
                sub: Some(1)
            }
        );
        assert_eq!(restored.dpt.as_ref().unwrap().layer, Layer::Program);
    }
}
