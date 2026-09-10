//! The command layer: every mutation is a `Command` whose `apply` returns
//! its own inverse, so undo and redo replay the same mechanism (ARCHITECTURE
//! §6). Validation lives here, not in the UI and not in storage. Any command
//! that changes a `Resolved<T>` sets its layer to `Layer::UserEdit`.

use std::fmt;

use crate::device::{ComObjectInstance, DeviceInstance};
use crate::dpt::DptRef;
use crate::flags::{ComFlagKind, Direction, GroupLink};
use crate::group::{GroupAddressEntry, GroupRange};
use crate::ids::{AreaId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId, LineId};
use crate::installation::Installation;
use crate::project::Project;
use crate::provenance::{Layer, Override, Resolved};
use crate::string_table::Text;
use crate::topology::{Area, Line};
use crate::validation::{
    check_group_address_in_range, check_group_link_target_exists,
    check_group_range_is_well_ordered, check_group_range_nests_in_parent,
    check_no_duplicate_area_address, check_no_duplicate_group_address,
    check_no_duplicate_individual_address, check_no_duplicate_line_address,
    check_no_overlapping_group_range, ValidationError,
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
    /// Sets one of a communication object instance's five flags as a user
    /// edit. Always resolves to `Layer::UserEdit` — use
    /// `RestoreComObjectFlag` to put back an exact prior `Override<bool>`
    /// (what undo does). Unlike `SetComObjectDpt`/`SetComObjectDescription`,
    /// a flag checkbox has only two states, so `value` is a bare `bool`,
    /// never `Option<bool>` — there is no "clear the override" gesture here.
    SetComObjectFlag {
        com_object: ComObjectInstanceId,
        flag: ComFlagKind,
        value: bool,
    },
    /// The undo/redo form of `SetComObjectFlag` — see `RestoreComObjectDpt`
    /// for why this cannot share the bare-`bool` shape.
    RestoreComObjectFlag {
        com_object: ComObjectInstanceId,
        flag: ComFlagKind,
        value: Override<bool>,
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
    /// Moves a device to `line`, or to `Topology::unassigned` if `None`.
    /// Does not touch `DeviceInstance::address` — a line move and a
    /// re-address are two separate user intents; `SetIndividualAddress`
    /// is the command for the latter.
    MoveDeviceToLine {
        device: DeviceId,
        line: Option<LineId>,
    },
    /// Creates a device with its communication-object instances already
    /// attached, placed in `line` or, if `None`, `Topology::unassigned` —
    /// mirrors `MoveDeviceToLine`'s own placement rule, since a device is
    /// placed in a line xor left unassigned the same way in both commands.
    /// `device.id` and every entry in `com_objects` carry ids
    /// pre-allocated by the caller via `Project::ids::next_device_id`/
    /// `next_com_object_instance_id`; `device.com_objects` already lists
    /// their ids, so no separate id list is threaded through twice.
    CreateDevice {
        device: DeviceInstance,
        com_objects: Vec<ComObjectInstance>,
        line: Option<LineId>,
    },
    /// Refuses (`CommandError::DeviceHasLinks`) if any of the device's
    /// communication objects still links to a group address — the
    /// device equivalent of `DeleteGroupAddress`'s `GroupAddressInUse`
    /// check.
    DeleteDevice {
        id: DeviceId,
    },
    /// `range.id` is pre-allocated by the caller via
    /// `Project::ids::next_group_range_id`.
    CreateGroupRange {
        range: GroupRange,
    },
    DeleteGroupRange {
        id: GroupRangeId,
    },
    RenameGroupRange {
        id: GroupRangeId,
        name: String,
    },
    /// Adds a directional link from a communication object instance to a
    /// group address. `direction` distinguishes a send link from a
    /// receive link — a comm object may hold both for the same `ga` as
    /// two distinct `GroupLink`s (DATA_MODEL §6).
    LinkComObject {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
        direction: Direction,
    },
    UnlinkComObject {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
        direction: Direction,
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
    /// A `DeleteDevice` was refused because at least one of the device's
    /// communication object instances still links to a group address —
    /// deleting it now would leave a dangling `GroupLink`, the device
    /// equivalent of `GroupAddressInUse`.
    DeviceHasLinks(DeviceId),
    GroupRangeNotFound(GroupRangeId),
    /// A `DeleteGroupRange` was refused because it still has nested
    /// (middle) ranges.
    GroupRangeNotEmpty(GroupRangeId),
    /// A `DeleteGroupRange` was refused because at least one group
    /// address still names it as its `range`.
    GroupRangeInUse(GroupRangeId),
    LinkAlreadyExists {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
        direction: Direction,
    },
    LinkNotFound {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
        direction: Direction,
    },
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
            CommandError::DeviceHasLinks(id) => {
                write!(f, "device {id} still has linked communication objects, cannot delete")
            }
            CommandError::GroupRangeNotFound(id) => write!(f, "group range {id} not found"),
            CommandError::GroupRangeNotEmpty(id) => {
                write!(f, "group range {id} still has nested ranges, cannot delete")
            }
            CommandError::GroupRangeInUse(id) => write!(
                f,
                "group range {id} still has group addresses assigned to it"
            ),
            CommandError::LinkAlreadyExists {
                com_object,
                ga,
                direction,
            } => write!(
                f,
                "communication object {com_object} already links to group address {ga} ({direction:?})"
            ),
            CommandError::LinkNotFound {
                com_object,
                ga,
                direction,
            } => write!(
                f,
                "communication object {com_object} has no {direction:?} link to group address {ga}"
            ),
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

/// Removes `device` from wherever it currently sits in `installation`'s
/// topology — `unassigned` or a line's `devices` — returning the line it
/// was in, if any. Shared by `MoveDeviceToLine` (which repositions the
/// device elsewhere) and `DeleteDevice` (which needs the same lookup to
/// know what line its own inverse `CreateDevice` should name).
fn remove_device_from_topology(
    installation: &mut Installation,
    device: DeviceId,
) -> Result<Option<LineId>, CommandError> {
    if let Some(pos) = installation
        .topology
        .unassigned
        .iter()
        .position(|&d| d == device)
    {
        installation.topology.unassigned.remove(pos);
        Ok(None)
    } else if let Some(current_line) = installation
        .topology
        .lines
        .iter_mut()
        .find(|l| l.devices.contains(&device))
    {
        let id = current_line.id;
        current_line.devices.retain(|&d| d != device);
        Ok(Some(id))
    } else {
        Err(CommandError::DeviceNotFound(device))
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
            Command::SetComObjectFlag {
                com_object,
                flag,
                value,
            } => {
                let com_object = *com_object;
                let flag = *flag;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.flags.get(flag).clone();
                *com.flags.get_mut(flag) = Override::Value(Resolved {
                    value: *value,
                    layer: Layer::UserEdit,
                });
                Ok(Command::RestoreComObjectFlag {
                    com_object,
                    flag,
                    value: previous,
                })
            }
            Command::RestoreComObjectFlag {
                com_object,
                flag,
                value,
            } => {
                let com_object = *com_object;
                let flag = *flag;
                let com: &mut ComObjectInstance = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let previous = com.flags.get(flag).clone();
                *com.flags.get_mut(flag) = value.clone();
                Ok(Command::RestoreComObjectFlag {
                    com_object,
                    flag,
                    value: previous,
                })
            }
            Command::CreateGroupAddress { entry } => {
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                check_no_duplicate_group_address(installation, entry.id, entry.address)?;
                if let Some(range_id) = entry.range {
                    let range = installation
                        .group_ranges
                        .iter()
                        .find(|r| r.id == range_id)
                        .ok_or(CommandError::GroupRangeNotFound(range_id))?;
                    check_group_address_in_range(range, entry.address)?;
                }
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
            Command::MoveDeviceToLine { device, line } => {
                let device = *device;
                let line = *line;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                if let Some(line_id) = line {
                    if !installation.topology.lines.iter().any(|l| l.id == line_id) {
                        return Err(CommandError::LineNotFound(line_id));
                    }
                }
                let previous = remove_device_from_topology(installation, device)?;
                match line {
                    Some(line_id) => {
                        installation
                            .topology
                            .lines
                            .iter_mut()
                            .find(|l| l.id == line_id)
                            .unwrap()
                            .devices
                            .push(device);
                    }
                    None => installation.topology.unassigned.push(device),
                }
                Ok(Command::MoveDeviceToLine {
                    device,
                    line: previous,
                })
            }
            Command::CreateDevice {
                device,
                com_objects,
                line,
            } => {
                let device = device.clone();
                let com_objects = com_objects.clone();
                let line = *line;
                let device_id = device.id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                if let Some(line_id) = line {
                    if !installation.topology.lines.iter().any(|l| l.id == line_id) {
                        return Err(CommandError::LineNotFound(line_id));
                    }
                }
                for com in &com_objects {
                    project.devices.insert_com_object(com.clone());
                }
                project.devices.insert(device);
                match line {
                    Some(line_id) => {
                        installation
                            .topology
                            .lines
                            .iter_mut()
                            .find(|l| l.id == line_id)
                            .unwrap()
                            .devices
                            .push(device_id);
                    }
                    None => installation.topology.unassigned.push(device_id),
                }
                Ok(Command::DeleteDevice { id: device_id })
            }
            Command::DeleteDevice { id } => {
                let id = *id;
                let device_ref = project
                    .devices
                    .get(id)
                    .ok_or(CommandError::DeviceNotFound(id))?;
                let has_links = device_ref.com_objects.iter().any(|&com_id| {
                    project
                        .devices
                        .com_object(com_id)
                        .is_some_and(|c| !c.links.is_empty())
                });
                if has_links {
                    return Err(CommandError::DeviceHasLinks(id));
                }
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let line = remove_device_from_topology(installation, id)?;
                let device = project.devices.remove(id).unwrap();
                let com_objects = device
                    .com_objects
                    .iter()
                    .filter_map(|&com_id| project.devices.remove_com_object(com_id))
                    .collect();
                Ok(Command::CreateDevice {
                    device,
                    com_objects,
                    line,
                })
            }
            Command::CreateGroupRange { range } => {
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                check_group_range_is_well_ordered(range.id, range.start, range.end)?;
                if let Some(parent_id) = range.parent {
                    let parent = installation
                        .group_ranges
                        .iter()
                        .find(|r| r.id == parent_id)
                        .ok_or(CommandError::GroupRangeNotFound(parent_id))?;
                    check_group_range_nests_in_parent(parent, range.id, range.start, range.end)?;
                }
                check_no_overlapping_group_range(
                    installation
                        .group_ranges
                        .iter()
                        .filter(|r| r.parent == range.parent),
                    range.id,
                    range.start,
                    range.end,
                )?;
                let id = range.id;
                if let Some(parent_id) = range.parent {
                    installation
                        .group_ranges
                        .iter_mut()
                        .find(|r| r.id == parent_id)
                        .unwrap()
                        .children
                        .push(id);
                }
                installation.group_ranges.push(range.clone());
                Ok(Command::DeleteGroupRange { id })
            }
            Command::DeleteGroupRange { id } => {
                let id = *id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let pos = installation
                    .group_ranges
                    .iter()
                    .position(|r| r.id == id)
                    .ok_or(CommandError::GroupRangeNotFound(id))?;
                if !installation.group_ranges[pos].children.is_empty() {
                    return Err(CommandError::GroupRangeNotEmpty(id));
                }
                if installation
                    .group_addresses
                    .iter()
                    .any(|ga| ga.range == Some(id))
                {
                    return Err(CommandError::GroupRangeInUse(id));
                }
                let range = installation.group_ranges.remove(pos);
                if let Some(parent_id) = range.parent {
                    installation
                        .group_ranges
                        .iter_mut()
                        .find(|r| r.id == parent_id)
                        .unwrap()
                        .children
                        .retain(|&c| c != id);
                }
                Ok(Command::CreateGroupRange { range })
            }
            Command::RenameGroupRange { id, name } => {
                let id = *id;
                let installation = project
                    .installations
                    .first_mut()
                    .ok_or(CommandError::InstallationNotFound)?;
                let range = installation
                    .group_ranges
                    .iter_mut()
                    .find(|r| r.id == id)
                    .ok_or(CommandError::GroupRangeNotFound(id))?;
                let previous = std::mem::replace(&mut range.name, name.clone());
                Ok(Command::RenameGroupRange { id, name: previous })
            }
            Command::LinkComObject {
                com_object,
                ga,
                direction,
            } => {
                let com_object = *com_object;
                let ga = *ga;
                let direction = *direction;
                let installation = project
                    .installations
                    .first()
                    .ok_or(CommandError::InstallationNotFound)?;
                check_group_link_target_exists(installation, com_object, ga)?;
                let com = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                if com
                    .links
                    .iter()
                    .any(|l| l.ga == ga && l.direction == direction)
                {
                    return Err(CommandError::LinkAlreadyExists {
                        com_object,
                        ga,
                        direction,
                    });
                }
                com.links.push(GroupLink { ga, direction });
                Ok(Command::UnlinkComObject {
                    com_object,
                    ga,
                    direction,
                })
            }
            Command::UnlinkComObject {
                com_object,
                ga,
                direction,
            } => {
                let com_object = *com_object;
                let ga = *ga;
                let direction = *direction;
                let com = project
                    .devices
                    .com_object_mut(com_object)
                    .ok_or(CommandError::ComObjectNotFound(com_object))?;
                let pos = com
                    .links
                    .iter()
                    .position(|l| l.ga == ga && l.direction == direction)
                    .ok_or(CommandError::LinkNotFound {
                        com_object,
                        ga,
                        direction,
                    })?;
                com.links.remove(pos);
                Ok(Command::LinkComObject {
                    com_object,
                    ga,
                    direction,
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
    use crate::flags::ResolvedFlags;
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
            module_instance: None,
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
            module_instance: None,
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
            module_instance: None,
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
            module_instance: None,
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
            module_instance: None,
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
    fn set_com_object_flag_marks_layer_as_user_edit_and_undoes() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags {
                read: Override::Value(Resolved {
                    value: true,
                    layer: Layer::Program,
                }),
                ..ResolvedFlags::none()
            },
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectFlag {
                    com_object: ComObjectInstanceId(1),
                    flag: ComFlagKind::Read,
                    value: false,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(!updated.flags.read.value().unwrap().value);
        assert_eq!(updated.flags.read.value().unwrap().layer, Layer::UserEdit);
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(restored.flags.read.value().unwrap().value);
        assert_eq!(restored.flags.read.value().unwrap().layer, Layer::Program);
    }

    #[test]
    fn setting_a_never_stated_com_object_flag_writes_value_not_absent() {
        let mut project = test_project_with_one_device(None);
        let com = ComObjectInstance {
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
        };
        project.devices.insert_com_object(com);
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::SetComObjectFlag {
                    com_object: ComObjectInstanceId(1),
                    flag: ComFlagKind::Communication,
                    value: true,
                },
            )
            .unwrap();
        let updated = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(updated.flags.communication.value().unwrap().value);
        assert_eq!(
            updated.flags.communication.value().unwrap().layer,
            Layer::UserEdit
        );
        stack.undo(&mut project).unwrap();
        let restored = project.devices.com_object(ComObjectInstanceId(1)).unwrap();
        assert!(!restored.flags.communication.is_present());
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

    fn project_with_line_and_unassigned_device() -> Project {
        let mut p = test_project_with_one_device(None);
        p.installations[0].topology.areas.push(Area {
            id: AreaId(1),
            source: source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        });
        p.installations[0]
            .topology
            .lines
            .push(test_line(LineId(1), 1, vec![]));
        p.installations[0].topology.unassigned.push(DeviceId(1));
        p
    }

    #[test]
    fn move_device_from_unassigned_to_a_line_and_back_via_undo() {
        let mut project = project_with_line_and_unassigned_device();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(1)),
                },
            )
            .unwrap();
        assert!(project.installations[0].topology.unassigned.is_empty());
        assert_eq!(
            project.installations[0].topology.lines[0].devices,
            vec![DeviceId(1)]
        );
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project.installations[0].topology.unassigned,
            vec![DeviceId(1)]
        );
        assert!(project.installations[0].topology.lines[0]
            .devices
            .is_empty());
    }

    #[test]
    fn move_device_between_two_lines() {
        let mut project = project_with_line_and_unassigned_device();
        project.installations[0].topology.areas[0]
            .lines
            .push(LineId(2));
        project.installations[0]
            .topology
            .lines
            .push(test_line(LineId(2), 2, vec![]));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(1)),
                },
            )
            .unwrap();
        stack
            .do_command(
                &mut project,
                Command::MoveDeviceToLine {
                    device: DeviceId(1),
                    line: Some(LineId(2)),
                },
            )
            .unwrap();
        assert!(project.installations[0].topology.lines[0]
            .devices
            .is_empty());
        assert_eq!(
            project.installations[0].topology.lines[1].devices,
            vec![DeviceId(1)]
        );
    }

    #[test]
    fn move_device_to_a_nonexistent_line_is_rejected_and_leaves_the_device_in_place() {
        let mut project = project_with_line_and_unassigned_device();
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::MoveDeviceToLine {
                device: DeviceId(1),
                line: Some(LineId(99)),
            },
        );
        assert_eq!(result, Err(CommandError::LineNotFound(LineId(99))));
        assert_eq!(
            project.installations[0].topology.unassigned,
            vec![DeviceId(1)]
        );
        assert!(!stack.can_undo());
    }

    #[test]
    fn move_an_unknown_device_is_rejected() {
        let mut project = project_with_line_and_unassigned_device();
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::MoveDeviceToLine {
                device: DeviceId(99),
                line: Some(LineId(1)),
            },
        );
        assert_eq!(result, Err(CommandError::DeviceNotFound(DeviceId(99))));
    }

    fn test_com_object_instance(id: ComObjectInstanceId, device: DeviceId) -> ComObjectInstance {
        ComObjectInstance {
            id,
            source: source(),
            device,
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        }
    }

    fn test_device_instance(id: DeviceId, com_objects: Vec<ComObjectInstanceId>) -> DeviceInstance {
        DeviceInstance {
            id,
            source: source(),
            name: "New device".into(),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects,
            binary_data: vec![],
        }
    }

    #[test]
    fn create_device_lands_in_unassigned_when_no_line_is_given_and_undo_removes_it() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let com = test_com_object_instance(ComObjectInstanceId(10), DeviceId(2));
        let device = test_device_instance(DeviceId(2), vec![ComObjectInstanceId(10)]);
        stack
            .do_command(
                &mut project,
                Command::CreateDevice {
                    device: device.clone(),
                    com_objects: vec![com.clone()],
                    line: None,
                },
            )
            .unwrap();
        assert!(project.devices.get(DeviceId(2)).is_some());
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(10))
            .is_some());
        assert_eq!(
            project.installations[0].topology.unassigned,
            vec![DeviceId(2)]
        );
        stack.undo(&mut project).unwrap();
        assert!(project.devices.get(DeviceId(2)).is_none());
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(10))
            .is_none());
        assert!(project.installations[0].topology.unassigned.is_empty());
    }

    #[test]
    fn create_device_on_a_line_places_it_there_and_rejects_an_unknown_line() {
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
        stack
            .do_command(
                &mut project,
                Command::CreateDevice {
                    device: test_device_instance(DeviceId(2), vec![]),
                    com_objects: vec![],
                    line: Some(LineId(1)),
                },
            )
            .unwrap();
        assert_eq!(
            project.installations[0].topology.lines[0].devices,
            vec![DeviceId(2)]
        );

        let result = stack.do_command(
            &mut project,
            Command::CreateDevice {
                device: test_device_instance(DeviceId(3), vec![]),
                com_objects: vec![],
                line: Some(LineId(99)),
            },
        );
        assert_eq!(result, Err(CommandError::LineNotFound(LineId(99))));
        assert!(project.devices.get(DeviceId(3)).is_none());
    }

    #[test]
    fn delete_device_refuses_while_a_com_object_still_has_a_link() {
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
        let mut com = test_com_object_instance(ComObjectInstanceId(1), DeviceId(1));
        com.links.push(GroupLink {
            ga: GroupAddressId(1),
            direction: Direction::Send,
        });
        project.devices.insert_com_object(com);
        project
            .devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));

        let mut stack = CommandStack::new();
        let result = stack.do_command(&mut project, Command::DeleteDevice { id: DeviceId(1) });
        assert_eq!(result, Err(CommandError::DeviceHasLinks(DeviceId(1))));
        assert!(project.devices.get(DeviceId(1)).is_some());
        assert!(!stack.can_undo());
    }

    #[test]
    fn deleting_then_undoing_and_redoing_preserves_values_captured_at_delete_time() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let com = test_com_object_instance(ComObjectInstanceId(10), DeviceId(2));
        let device = test_device_instance(DeviceId(2), vec![ComObjectInstanceId(10)]);
        stack
            .do_command(
                &mut project,
                Command::CreateDevice {
                    device,
                    com_objects: vec![com],
                    line: None,
                },
            )
            .unwrap();

        // Stand-in for `knx_productdb::enrich::apply` filling an `Absent`
        // slot after creation (design doc §3, step 3) — a direct
        // mutation, not a `Command`, exactly like the real enrichment
        // pass.
        project
            .devices
            .com_object_mut(ComObjectInstanceId(10))
            .unwrap()
            .dpt = Override::Value(Resolved {
            value: DptRef {
                main: 1,
                sub: Some(1),
            },
            layer: Layer::Program,
        });

        stack
            .do_command(&mut project, Command::DeleteDevice { id: DeviceId(2) })
            .unwrap();
        assert!(project.devices.get(DeviceId(2)).is_none());
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(10))
            .is_none());

        stack.undo(&mut project).unwrap(); // undoes the delete -> recreates
        let restored = project.devices.com_object(ComObjectInstanceId(10)).unwrap();
        assert_eq!(
            restored.dpt.value().unwrap().value,
            DptRef {
                main: 1,
                sub: Some(1)
            }
        );
        assert_eq!(restored.dpt.value().unwrap().layer, Layer::Program);

        stack.redo(&mut project).unwrap(); // re-deletes
        assert!(project.devices.get(DeviceId(2)).is_none());

        stack.undo(&mut project).unwrap(); // undoes the re-delete -> recreates again
        let restored_again = project.devices.com_object(ComObjectInstanceId(10)).unwrap();
        assert_eq!(
            restored_again.dpt.value().unwrap().layer,
            Layer::Program,
            "the enriched value must survive a delete/undo/redo/undo cycle unchanged"
        );
    }

    fn test_range(
        id: GroupRangeId,
        start: u16,
        end: u16,
        parent: Option<GroupRangeId>,
    ) -> GroupRange {
        GroupRange {
            id,
            source: source(),
            name: "R".into(),
            start: GroupAddress::from_raw(start),
            end: GroupAddress::from_raw(end),
            parent,
            children: vec![],
        }
    }

    #[test]
    fn create_then_delete_group_range_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let range = test_range(GroupRangeId(1), 0, 2047, None);
        stack
            .do_command(
                &mut project,
                Command::CreateGroupRange {
                    range: range.clone(),
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].group_ranges.len(), 1);
        stack
            .do_command(
                &mut project,
                Command::DeleteGroupRange {
                    id: GroupRangeId(1),
                },
            )
            .unwrap();
        assert!(project.installations[0].group_ranges.is_empty());
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges.len(), 1);
    }

    #[test]
    fn create_nested_group_range_registers_with_its_parent_and_undo_deregisters_it() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::CreateGroupRange {
                    range: test_range(GroupRangeId(2), 0, 255, Some(GroupRangeId(1))),
                },
            )
            .unwrap();
        assert_eq!(
            project.installations[0].group_ranges[0].children,
            vec![GroupRangeId(2)]
        );
        stack.undo(&mut project).unwrap();
        assert!(project.installations[0].group_ranges[0].children.is_empty());
    }

    #[test]
    fn create_group_range_rejects_a_span_outside_its_parent() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 255, None));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupRange {
                range: test_range(GroupRangeId(2), 0, 2047, Some(GroupRangeId(1))),
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::GroupRangeOutsideParent { .. }
            ))
        ));
    }

    #[test]
    fn create_group_range_rejects_an_inverted_span() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupRange {
                range: test_range(GroupRangeId(1), 255, 0, None),
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::GroupRangeInverted { .. }
            ))
        ));
        assert!(!stack.can_undo());
    }

    #[test]
    fn create_group_range_rejects_an_unknown_parent() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupRange {
                range: test_range(GroupRangeId(1), 0, 255, Some(GroupRangeId(99))),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::GroupRangeNotFound(GroupRangeId(99)))
        );
    }

    #[test]
    fn create_group_range_rejects_overlap_with_a_sibling() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 255, None));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupRange {
                range: test_range(GroupRangeId(2), 200, 500, None),
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::OverlappingGroupRange { .. }
            ))
        ));
    }

    #[test]
    fn delete_group_range_refuses_when_it_still_has_children() {
        let mut project = test_project_with_one_device(None);
        let mut range = test_range(GroupRangeId(1), 0, 2047, None);
        range.children.push(GroupRangeId(2));
        project.installations[0].group_ranges.push(range);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::DeleteGroupRange {
                id: GroupRangeId(1),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::GroupRangeNotEmpty(GroupRangeId(1)))
        );
    }

    #[test]
    fn delete_group_range_refuses_when_a_group_address_still_uses_it() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        project.installations[0]
            .group_addresses
            .push(GroupAddressEntry {
                id: GroupAddressId(1),
                source: source(),
                name: "GA".into(),
                address: GroupAddress::from_raw(1),
                central: false,
                unfiltered: false,
                range: Some(GroupRangeId(1)),
            });
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::DeleteGroupRange {
                id: GroupRangeId(1),
            },
        );
        assert_eq!(result, Err(CommandError::GroupRangeInUse(GroupRangeId(1))));
    }

    #[test]
    fn rename_group_range_round_trips_through_undo() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 2047, None));
        project.installations[0].group_ranges[0].name = "Old name".into();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::RenameGroupRange {
                    id: GroupRangeId(1),
                    name: "New name".into(),
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].group_ranges[0].name, "New name");
        stack.undo(&mut project).unwrap();
        assert_eq!(project.installations[0].group_ranges[0].name, "Old name");
    }

    #[test]
    fn rename_unknown_group_range_is_rejected() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::RenameGroupRange {
                id: GroupRangeId(99),
                name: "X".into(),
            },
        );
        assert_eq!(
            result,
            Err(CommandError::GroupRangeNotFound(GroupRangeId(99)))
        );
    }

    #[test]
    fn create_group_address_rejects_an_address_outside_its_stated_range() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 100, None));
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupAddress {
                entry: GroupAddressEntry {
                    id: GroupAddressId(1),
                    source: source(),
                    name: "GA".into(),
                    address: GroupAddress::from_raw(200),
                    central: false,
                    unfiltered: false,
                    range: Some(GroupRangeId(1)),
                },
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::GroupAddressOutsideRange { .. }
            ))
        ));
    }

    #[test]
    fn create_group_address_accepts_an_address_inside_its_stated_range() {
        let mut project = test_project_with_one_device(None);
        project.installations[0]
            .group_ranges
            .push(test_range(GroupRangeId(1), 0, 100, None));
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::CreateGroupAddress {
                    entry: GroupAddressEntry {
                        id: GroupAddressId(1),
                        source: source(),
                        name: "GA".into(),
                        address: GroupAddress::from_raw(50),
                        central: false,
                        unfiltered: false,
                        range: Some(GroupRangeId(1)),
                    },
                },
            )
            .unwrap();
        assert_eq!(project.installations[0].group_addresses.len(), 1);
    }

    #[test]
    fn create_group_address_rejects_an_unknown_range() {
        let mut project = test_project_with_one_device(None);
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::CreateGroupAddress {
                entry: GroupAddressEntry {
                    id: GroupAddressId(1),
                    source: source(),
                    name: "GA".into(),
                    address: GroupAddress::from_raw(50),
                    central: false,
                    unfiltered: false,
                    range: Some(GroupRangeId(99)),
                },
            },
        );
        assert_eq!(
            result,
            Err(CommandError::GroupRangeNotFound(GroupRangeId(99)))
        );
    }

    fn test_project_with_one_com_object() -> Project {
        let mut p = test_project_with_one_device(None);
        p.devices
            .get_mut(DeviceId(1))
            .unwrap()
            .com_objects
            .push(ComObjectInstanceId(1));
        p.devices.insert_com_object(ComObjectInstance {
            id: ComObjectInstanceId(1),
            source: source(),
            device: DeviceId(1),
            number: 0,
            text: Override::Value(Resolved {
                value: Text::Literal("t".into()),
                layer: Layer::Program,
            }),
            description: Override::Absent,
            dpt: Override::Absent,
            flags: ResolvedFlags::none(),
            size: None,
            is_active: true,
            links: vec![],
            module_instance: None,
        });
        p.installations[0].group_addresses.push(GroupAddressEntry {
            id: GroupAddressId(1),
            source: source(),
            name: "GA".into(),
            address: GroupAddress::from_raw(1),
            central: false,
            unfiltered: false,
            range: None,
        });
        p
    }

    #[test]
    fn link_then_unlink_com_object_round_trips_through_undo() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::LinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Send,
                },
            )
            .unwrap();
        assert_eq!(
            project
                .devices
                .com_object(ComObjectInstanceId(1))
                .unwrap()
                .links,
            vec![GroupLink {
                ga: GroupAddressId(1),
                direction: Direction::Send
            }]
        );
        stack
            .do_command(
                &mut project,
                Command::UnlinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Send,
                },
            )
            .unwrap();
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(1))
            .unwrap()
            .links
            .is_empty());
        stack.undo(&mut project).unwrap();
        assert_eq!(
            project
                .devices
                .com_object(ComObjectInstanceId(1))
                .unwrap()
                .links
                .len(),
            1
        );
        stack.undo(&mut project).unwrap();
        assert!(project
            .devices
            .com_object(ComObjectInstanceId(1))
            .unwrap()
            .links
            .is_empty());
    }

    #[test]
    fn link_com_object_rejects_a_nonexistent_group_address() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::LinkComObject {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(99),
                direction: Direction::Send,
            },
        );
        assert!(matches!(
            result,
            Err(CommandError::Validation(
                ValidationError::DanglingGroupLink { .. }
            ))
        ));
    }

    #[test]
    fn link_com_object_rejects_an_exact_duplicate_link() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::LinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Send,
                },
            )
            .unwrap();
        let result = stack.do_command(
            &mut project,
            Command::LinkComObject {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(1),
                direction: Direction::Send,
            },
        );
        assert_eq!(
            result,
            Err(CommandError::LinkAlreadyExists {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(1),
                direction: Direction::Send,
            })
        );
    }

    #[test]
    fn link_com_object_allows_send_and_receive_on_the_same_group_address() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        stack
            .do_command(
                &mut project,
                Command::LinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Send,
                },
            )
            .unwrap();
        stack
            .do_command(
                &mut project,
                Command::LinkComObject {
                    com_object: ComObjectInstanceId(1),
                    ga: GroupAddressId(1),
                    direction: Direction::Receive,
                },
            )
            .unwrap();
        assert_eq!(
            project
                .devices
                .com_object(ComObjectInstanceId(1))
                .unwrap()
                .links
                .len(),
            2
        );
    }

    #[test]
    fn unlink_a_nonexistent_link_is_rejected() {
        let mut project = test_project_with_one_com_object();
        let mut stack = CommandStack::new();
        let result = stack.do_command(
            &mut project,
            Command::UnlinkComObject {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(1),
                direction: Direction::Send,
            },
        );
        assert_eq!(
            result,
            Err(CommandError::LinkNotFound {
                com_object: ComObjectInstanceId(1),
                ga: GroupAddressId(1),
                direction: Direction::Send,
            })
        );
    }
}
