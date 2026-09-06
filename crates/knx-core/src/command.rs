//! The command layer: every mutation is a `Command` whose `apply` returns
//! its own inverse, so undo and redo replay the same mechanism (ARCHITECTURE
//! §6). Validation lives here, not in the UI and not in storage. Any command
//! that changes a `Resolved<T>` sets its layer to `Layer::UserEdit`.

use std::fmt;

use crate::device::ComObjectInstance;
use crate::dpt::DptRef;
use crate::group::GroupAddressEntry;
use crate::ids::{AreaId, ComObjectInstanceId, DeviceId, GroupAddressId, LineId};
use crate::project::Project;
use crate::provenance::{Layer, Override, Resolved};
use crate::string_table::Text;
use crate::topology::{Area, Line};
use crate::validation::{
    check_no_duplicate_area_address, check_no_duplicate_group_address,
    check_no_duplicate_individual_address, check_no_duplicate_line_address, ValidationError,
};
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
    /// Sets a device's description as a user edit. Unlike
    /// `ComObjectInstance::description`, `DeviceInstance::description` is a
    /// bare `Option<String>`, not an `Override<Text>` — no provenance layer
    /// exists to preserve, so this doubles as its own undo/redo form (like
    /// `SetIndividualAddress`).
    SetDeviceDescription {
        device: DeviceId,
        description: Option<String>,
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
        dpt: Override<DptRef>,
    },
    /// Sets a communication object instance's description as a user edit.
    /// Always resolves to `Layer::UserEdit` — use
    /// `RestoreComObjectDescription` to put back an exact prior
    /// `Resolved<Text>` (that is what undo does; this variant is not it).
    SetComObjectDescription {
        com_object: ComObjectInstanceId,
        description: Option<String>,
    },
    /// The undo/redo form of `SetComObjectDescription` — see
    /// `RestoreComObjectDpt` for why this cannot share the bare-`String`
    /// shape.
    RestoreComObjectDescription {
        com_object: ComObjectInstanceId,
        description: Override<Text>,
    },
    /// `entry.id` is pre-allocated by the caller via
    /// `Project::ids::next_group_address_id`.
    CreateGroupAddress {
        entry: GroupAddressEntry,
    },
    DeleteGroupAddress {
        id: GroupAddressId,
    },
    /// `area.id` is pre-allocated by the caller via
    /// `Project::ids::next_area_id`.
    CreateArea {
        area: Area,
    },
    DeleteArea {
        id: AreaId,
    },
    /// `line.id` is pre-allocated by the caller via
    /// `Project::ids::next_line_id`. `area` names the owning area, which
    /// must already exist.
    CreateLine {
        area: AreaId,
        line: Line,
    },
    DeleteLine {
        id: LineId,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandError {
    Validation(ValidationError),
    DeviceNotFound(DeviceId),
    ComObjectNotFound(ComObjectInstanceId),
    GroupAddressNotFound(GroupAddressId),
    /// A `DeleteGroupAddress` was refused because at least one
    /// communication object still links to it — deleting it now would
    /// leave a dangling `GroupLink` (`ValidationError::DanglingGroupLink`
    /// exists for the reverse direction: a link created against a group
    /// address that is already gone).
    GroupAddressInUse(GroupAddressId),
    AreaNotFound(AreaId),
    /// A `DeleteArea` was refused because it still owns at least one line.
    AreaNotEmpty(AreaId),
    LineNotFound(LineId),
    /// A `DeleteLine` was refused because it still owns at least one
    /// device.
    LineNotEmpty(LineId),
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
            CommandError::GroupAddressInUse(id) => {
                write!(
                    f,
                    "group address {id} is still linked from a communication object"
                )
            }
            CommandError::AreaNotFound(id) => write!(f, "area {id} not found"),
            CommandError::AreaNotEmpty(id) => {
                write!(f, "area {id} still has lines, cannot delete")
            }
            CommandError::LineNotFound(id) => write!(f, "line {id} not found"),
            CommandError::LineNotEmpty(id) => {
                write!(f, "line {id} still has devices, cannot delete")
            }
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
            Command::SetDeviceDescription {
                device,
                description,
            } => {
                let device = *device;
                let target = project
                    .devices
                    .get_mut(device)
                    .ok_or(CommandError::DeviceNotFound(device))?;
                let previous = target.description.clone();
                target.description = description.clone();
                Ok(Command::SetDeviceDescription {
                    device,
                    description: previous,
                })
            }
            Command::SetComObjectDpt { com_object, dpt } => {
                let com_object = *com_object;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.dpt.clone();
                com.dpt = match dpt {
                    Some(value) => Override::Value(Resolved {
                        value: *value,
                        layer: Layer::UserEdit,
                    }),
                    // A user-initiated clear is a deliberate empty, mirroring
                    // ETS's own `DatapointType=""` convention; `Absent` would
                    // misrepresent a value the user just acted on.
                    None => Override::Empty,
                };
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
                let previous = com.dpt.clone();
                com.dpt = dpt.clone();
                Ok(Command::RestoreComObjectDpt {
                    com_object,
                    dpt: previous,
                })
            }
            Command::SetComObjectDescription {
                com_object,
                description,
            } => {
                let com_object = *com_object;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.description.clone();
                com.description = match description {
                    Some(value) => Override::Value(Resolved {
                        value: Text::Literal(value.clone()),
                        layer: Layer::UserEdit,
                    }),
                    // A user-initiated clear is a deliberate empty, mirroring
                    // `SetComObjectDpt`'s own convention (see its comment).
                    None => Override::Empty,
                };
                Ok(Command::RestoreComObjectDescription {
                    com_object,
                    description: previous,
                })
            }
            Command::RestoreComObjectDescription {
                com_object,
                description,
            } => {
                let com_object = *com_object;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.description.clone();
                com.description = description.clone();
                Ok(Command::RestoreComObjectDescription {
                    com_object,
                    description: previous,
                })
            }
            Command::CreateGroupAddress { entry } => {
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                check_no_duplicate_group_address(installation, entry.id, entry.address)?;
                let id = entry.id;
                installation.group_addresses.push(entry.clone());
                Ok(Command::DeleteGroupAddress { id })
            }
            Command::DeleteGroupAddress { id } => {
                let id = *id;
                if project
                    .devices
                    .com_objects()
                    .any(|com| com.links.iter().any(|link| link.ga == id))
                {
                    return Err(CommandError::GroupAddressInUse(id));
                }
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
            Command::CreateArea { area } => {
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                check_no_duplicate_area_address(&installation.topology, area.id, area.address)?;
                let id = area.id;
                installation.topology.areas.push(area.clone());
                Ok(Command::DeleteArea { id })
            }
            Command::DeleteArea { id } => {
                let id = *id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let pos = installation
                    .topology
                    .areas
                    .iter()
                    .position(|a| a.id == id)
                    .ok_or(CommandError::AreaNotFound(id))?;
                if !installation.topology.areas[pos].lines.is_empty() {
                    return Err(CommandError::AreaNotEmpty(id));
                }
                let area = installation.topology.areas.remove(pos);
                Ok(Command::CreateArea { area })
            }
            Command::CreateLine { area, line } => {
                let area_id = *area;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let area_ref = installation
                    .topology
                    .areas
                    .iter()
                    .find(|a| a.id == area_id)
                    .ok_or(CommandError::AreaNotFound(area_id))?;
                check_no_duplicate_line_address(
                    area_ref,
                    &installation.topology.lines,
                    line.id,
                    line.address,
                )?;
                let id = line.id;
                installation.topology.lines.push(line.clone());
                installation
                    .topology
                    .areas
                    .iter_mut()
                    .find(|a| a.id == area_id)
                    .unwrap()
                    .lines
                    .push(id);
                Ok(Command::DeleteLine { id })
            }
            Command::DeleteLine { id } => {
                let id = *id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let area_id = installation
                    .topology
                    .area_of(id)
                    .map(|a| a.id)
                    .ok_or(CommandError::LineNotFound(id))?;
                let pos = installation
                    .topology
                    .lines
                    .iter()
                    .position(|l| l.id == id)
                    .ok_or(CommandError::LineNotFound(id))?;
                if !installation.topology.lines[pos].devices.is_empty() {
                    return Err(CommandError::LineNotEmpty(id));
                }
                let line = installation.topology.lines.remove(pos);
                installation
                    .topology
                    .areas
                    .iter_mut()
                    .find(|a| a.id == area_id)
                    .unwrap()
                    .lines
                    .retain(|&l| l != id);
                Ok(Command::CreateLine {
                    area: area_id,
                    line,
                })
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
    use crate::flags::{Direction, GroupLink, ResolvedFlags};
    use crate::group::GroupRange;
    use crate::ids::{InstallationId, LineId, SourceRef};
    use crate::installation::Installation;
    use crate::string_table::Text;
    use crate::topology::{Area, Topology};
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
            binary_data: vec![],
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
            binary_data: vec![],
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
    fn create_group_address_rejects_a_duplicate_address_and_leaves_the_stack_untouched() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "Existing".into(),
                address: GroupAddress::from_raw(5),
                central: false,
                unfiltered: false,
                range: None,
            });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupAddress {
                entry: GroupAddressEntry {
                    id: GroupAddressId(2),
                    source: source(),
                    name: "New".into(),
                    address: GroupAddress::from_raw(5),
                    central: false,
                    unfiltered: false,
                    range: None,
                },
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DuplicateGroupAddress { .. }
            ))
        ));
        assert_eq!(project.installations[0].group_addresses.len(), 1);
        assert!(!stack.can_undo());
    }

    #[test]
    fn delete_group_address_is_rejected_while_a_com_object_still_links_to_it() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "GA".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: None,
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
            links: vec![GroupLink {
                ga: GroupAddressId(1),
                direction: Direction::Send,
            }],
        });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::DeleteGroupAddress {
                id: GroupAddressId(1),
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::GroupAddressInUse(GroupAddressId(1)))
        ));
        assert_eq!(project.installations[0].group_addresses.len(), 1);
        assert!(!stack.can_undo());
    }

    #[test]
    fn set_com_object_dpt_marks_layer_as_user_edit_and_undoes() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved {
                value: Text::Literal("t".into()),
                layer: Layer::Program,
            }),
            description: Override::Absent,
            dpt: Override::Value(Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::Program,
            }),
            flags: ResolvedFlags {
                read: Override::Value(Resolved {
                    value: true,
                    layer: Layer::Program,
                }),
                write: Override::Absent,
                transmit: Override::Absent,
                update: Override::Absent,
                communication: Override::Value(Resolved {
                    value: true,
                    layer: Layer::Program,
                }),
            },
            size: None,
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
        assert_eq!(updated.dpt.value().unwrap().value, new_dpt);
        assert_eq!(updated.dpt.value().unwrap().layer, Layer::UserEdit);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(
            restored.dpt.value().unwrap().value,
            DptRef {
                main: 1,
                sub: Some(1)
            }
        );
        assert_eq!(restored.dpt.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn set_device_description_then_undo_restores_previous_value() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetDeviceDescription {
                    device: DeviceId(1),
                    description: Some("new description".into()),
                },
            )
            .unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().description,
            Some("new description".to_string())
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(project.devices.get(DeviceId(1)).unwrap().description, None);
        stack.redo(&mut project).unwrap();
        assert_eq!(
            project.devices.get(DeviceId(1)).unwrap().description,
            Some("new description".to_string())
        );
    }

    #[test]
    fn set_com_object_description_marks_layer_as_user_edit_and_undoes() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Value(Resolved {
                value: Text::Literal("old".into()),
                layer: Layer::Program,
            }),
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectDescription {
                    com_object: ComObjectInstanceId(1),
                    description: Some("new".into()),
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(
            updated.description.value().unwrap().value,
            Text::Literal("new".into())
        );
        assert_eq!(updated.description.value().unwrap().layer, Layer::UserEdit);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(
            restored.description.value().unwrap().value,
            Text::Literal("old".into())
        );
        assert_eq!(restored.description.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn setting_com_object_description_to_none_writes_empty_not_absent() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Value(Resolved {
                value: Text::Literal("old".into()),
                layer: Layer::Program,
            }),
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectDescription {
                    com_object: ComObjectInstanceId(1),
                    description: None,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(updated.description, Override::Empty);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(restored.description.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn setting_com_object_dpt_to_none_writes_empty_not_absent() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved {
                value: Text::Literal("t".into()),
                layer: Layer::Program,
            }),
            description: Override::Absent,
            dpt: Override::Value(Resolved {
                value: DptRef {
                    main: 1,
                    sub: Some(1),
                },
                layer: Layer::Program,
            }),
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectDpt {
                    com_object: ComObjectInstanceId(1),
                    dpt: None,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(updated.dpt, Override::Empty);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert_eq!(restored.dpt.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn create_then_delete_area_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let area = Area {
            id: AreaId(1),
            source: source(),
            name: "Area 1".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        };
        stack
            .do_command(&mut project, Command::CreateArea { area: area.clone() })
            .unwrap();
        assert_eq!(project.installations[0].topology.areas.len(), 1);
        stack
            .do_command(&mut project, Command::DeleteArea { id: AreaId(1) })
            .unwrap();
        assert!(project.installations[0].topology.areas.is_empty());
        stack.undo(&mut project).unwrap(); // undoes the delete -> recreates
        assert_eq!(project.installations[0].topology.areas.len(), 1);
        stack.undo(&mut project).unwrap(); // undoes the create -> empty again
        assert!(project.installations[0].topology.areas.is_empty());
    }

    #[test]
    fn create_area_rejects_a_duplicate_address_and_leaves_the_stack_untouched() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "Existing".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateArea {
                area: Area {
                    id: AreaId(2),
                    source: source(),
                    name: "New".into(),
                    address: 1,
                    completion: CompletionStatus::FinishedDesign,
                    lines: vec![],
                },
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DuplicateAreaAddress { .. }
            ))
        ));
        assert!(!stack.can_undo());
    }

    #[test]
    fn delete_area_refuses_when_it_still_has_a_line() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteArea { id: AreaId(1) });
        assert_eq!(result, Err(CommandError::AreaNotEmpty(AreaId(1))));
        assert!(!stack.can_undo());
    }

    #[test]
    fn delete_unknown_area_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteArea { id: AreaId(99) });
        assert_eq!(result, Err(CommandError::AreaNotFound(AreaId(99))));
    }

    fn test_line(id: LineId, address: u8, devices: Vec<DeviceId>) -> Line {
        Line {
            id,
            source: source(),
            name: "L".into(),
            address,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices,
        }
    }

    #[test]
    fn create_then_delete_line_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        });
        let mut stack = CommandStack::new();
        let line = test_line(LineId(1), 1, vec![]);
        stack
            .do_command(
                &mut project,
                Command::CreateLine {
                    area: AreaId(1),
                    line: line.clone(),
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].topology.lines.len(), 1);
        assert_eq!(
            project.installations[0].topology.areas[0].lines,
            vec![LineId(1)]
        );
        stack
            .do_command(&mut project, Command::DeleteLine { id: LineId(1) })
            .unwrap();
        assert!(project.installations[0].topology.lines.is_empty());
        assert!(project.installations[0].topology.areas[0].lines.is_empty());
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].topology.lines.len(), 1);
        stack.undo(&mut project).unwrap();
        assert!(project.installations[0].topology.lines.is_empty());
    }

    #[test]
    fn create_line_rejects_a_duplicate_address_in_the_same_area() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(1), 1, vec![]));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateLine {
                area: AreaId(1),
                line: test_line(LineId(2), 1, vec![]),
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DuplicateLineAddress { .. }
            ))
        ));
        assert!(!stack.can_undo());
    }

    #[test]
    fn create_line_rejects_an_unknown_area() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateLine {
                area: AreaId(99),
                line: test_line(LineId(1), 1, vec![]),
            },
        );
        assert_eq!(result, Err(CommandError::AreaNotFound(AreaId(99))));
    }

    #[test]
    fn delete_line_refuses_when_it_still_has_a_device() {
        let mut project = test_project_with_one_device(None);
        project.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(1), 1, vec![DeviceId(1)]));
        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteLine { id: LineId(1) });
        assert_eq!(result, Err(CommandError::LineNotEmpty(LineId(1))));
    }

    #[test]
    fn delete_unknown_line_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteLine { id: LineId(99) });
        assert_eq!(result, Err(CommandError::LineNotFound(LineId(99))));
    }
}
