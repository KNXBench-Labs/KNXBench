//! Marks the parts an edit changes as pending, in the edit's own undo step (P3, ADR-0129).
//!
//! [`candidates`] names the devices a command can change the programming
//! content of; the command stack compares their
//! [`ProgrammingContent`](crate::commissioning::pending::ProgrammingContent)
//! before and after and records [`Command::SetDeviceProgramming`] for every
//! part that differs. The match lists every command on purpose: a new command
//! does not compile until someone decides what it changes on a device.

use std::collections::{BTreeMap, BTreeSet};

use super::{Command, CommandError};
use crate::commissioning::pending::{
    devices_linked_to, programming_content, ProgrammingChange, ProgrammingContent, ProgrammingParts,
};
use crate::ids::{ComObjectInstanceId, DeviceId};
use crate::Project;

/// Applies a belief change to one device and returns the change that undoes it.
pub(super) fn set(
    project: &mut Project,
    device: DeviceId,
    change: ProgrammingChange,
) -> Result<Command, CommandError> {
    let instance = project
        .devices
        .get_mut(device)
        .ok_or(CommandError::DeviceNotFound(device))?;
    let undo = change.apply_to(&mut instance.commissioning);
    Ok(Command::SetDeviceProgramming {
        device,
        change: undo,
    })
}

fn owner(project: &Project, com_object: ComObjectInstanceId, out: &mut BTreeSet<DeviceId>) {
    if let Some(co) = project.devices.com_object(com_object) {
        out.insert(co.device);
    }
}

/// Adds the devices `cmd` may change the programming content of, as far as
/// `project` shows them. Called before and after the command, so objects
/// and links a batch creates or removes are found on one side or the other.
pub(super) fn candidates(cmd: &Command, project: &Project, out: &mut BTreeSet<DeviceId>) {
    match cmd {
        Command::SetIndividualAddress { device, .. }
        | Command::SetCouplerIndividualAddress { device, .. }
        | Command::RestoreIndividualAddress { device, .. }
        | Command::SetParameterValue { device, .. }
        | Command::RestoreParameterValue { device, .. }
        | Command::AddComObjects { device, .. }
        | Command::RemoveAddedComObjects { device, .. } => {
            out.insert(*device);
        }
        Command::SetComObjectFlag { com_object, .. }
        | Command::RestoreComObjectFlag { com_object, .. }
        | Command::LinkComObject { com_object, .. }
        | Command::UnlinkComObject { com_object, .. }
        | Command::RestoreGroupLink { com_object, .. } => owner(project, *com_object, out),
        Command::ReaddressGroupAddress { id, .. } | Command::DeleteGroupAddress { id } => {
            out.extend(devices_linked_to(project, *id));
        }
        Command::RestoreGroupAddress { entry, .. } => {
            out.extend(devices_linked_to(project, entry.id));
        }
        Command::Batch(commands) => {
            for command in commands {
                candidates(command, project, out);
            }
        }
        Command::CreateDevice { device, .. } | Command::RestoreDevice { device, .. } => { out.insert(device.id); }
        Command::DeleteDevice { id } => { out.insert(*id); }
        // The belief itself, not content.
        Command::SetDeviceProgramming { .. }
        // Datapoint types and texts are not programmed (the group object
        // table carries the product's object types).
        | Command::SetComObjectDpt { .. }
        | Command::RestoreComObjectDpt { .. }
        | Command::SetComObjectDescription { .. }
        | Command::RestoreComObjectDescription { .. }
        // Names, descriptions, notes, instructions, Functions: documentation.
        | Command::RenameDevice { .. }
        | Command::RestoreDeviceName { .. }
        | Command::RenameGroupAddress { .. }
        | Command::RestoreGroupAddressName { .. }
        | Command::SetDeviceDescription { .. }
        | Command::SetDeviceInstructions { .. }
        | Command::RenameArea { .. }
        | Command::RenameLine { .. }
        | Command::RenameBuildingPart { .. }
        | Command::RenameGroupRange { .. }
        | Command::RenameInstallation { .. }
        | Command::InsertProjectNote { .. }
        | Command::RemoveProjectNote { .. }
        | Command::ReplaceProjectNote { .. }
        | Command::MoveProjectNote { .. }
        | Command::CreateFunction { .. }
        | Command::DeleteFunction { .. }
        | Command::RestoreFunction { .. }
        | Command::UpdateFunction { .. }
        | Command::RestoreFunctionDetails { .. }
        | Command::MoveFunction { .. }
        | Command::AddFunctionMember { .. }
        | Command::RemoveFunctionMember { .. }
        | Command::RestoreFunctionMember { .. }
        // A new or unlinked group address is in no device's tables; central
        // and unfiltered concern couplers, not the linked devices; the style
        // only changes how the same number is written.
        | Command::CreateGroupAddress { .. }
        | Command::UpdateGroupAddress { .. }
        | Command::SetGroupAddressStyle { .. }
        // Structure and placement do not change an address (a line move
        // keeps it; `SetIndividualAddress` is the re-address).
        | Command::CreateArea { .. }
        | Command::DeleteArea { .. }
        | Command::RestoreArea { .. }
        | Command::CreateLine { .. }
        | Command::DeleteLine { .. }
        | Command::RestoreLine { .. }
        | Command::MoveLineToArea { .. }
        | Command::RestoreLinePlacement { .. }
        | Command::MoveDeviceToLine { .. }
        | Command::RepairDevicePlacement { .. }
        | Command::RestoreDevicePlacements { .. }
        | Command::RepairLineOwner { .. }
        | Command::RestoreLineOwners { .. }
        | Command::CreateBuildingPart { .. }
        | Command::DeleteBuildingPart { .. }
        | Command::RestoreBuildingPart { .. }
        | Command::MoveBuildingPart { .. }
        | Command::RestoreBuildingPartPlacement { .. }
        | Command::MoveDeviceToBuildingPart { .. }
        | Command::RestoreDeviceBuildingPlacement { .. }
        | Command::CreateGroupRange { .. }
        | Command::DeleteGroupRange { .. }
        | Command::RestoreGroupRange { .. }
        | Command::MoveGroupRange { .. }
        | Command::RestoreGroupRangePlacement { .. }
        | Command::SetIdAllocators { .. }
        | Command::ReserveIds { .. } => {}
    }
}

/// The content of `devices` before a command runs.
pub(super) fn before(
    project: &Project,
    devices: &BTreeSet<DeviceId>,
) -> BTreeMap<DeviceId, ProgrammingContent> {
    programming_content(project, devices)
}

/// Advances only changed device content, reusing P3's exact comparison semantics.
/// Revisions are session-only: undo never resurrects an earlier bus observation.
pub(super) fn record_revisions(
    project: &Project,
    before: &BTreeMap<DeviceId, ProgrammingContent>,
    devices: &BTreeSet<DeviceId>,
    revisions: &mut BTreeMap<DeviceId, u64>,
) {
    let after = programming_content(project, devices);
    for device in devices {
        if before.get(device) != after.get(device) {
            let revision = revisions.entry(*device).or_default();
            *revision = revision.saturating_add(1);
        }
    }
}

/// Marks every part whose content changed between `before` and `project` as
/// pending, whatever the belief was, and folds the undo into `inverse`, so
/// that one undo step reverts the edit and its marks together. A part that
/// was already pending is recorded too: if a download clears it before the
/// undo, the undo must make it pending again.
pub(super) fn mark_changed(
    project: &mut Project,
    before: BTreeMap<DeviceId, ProgrammingContent>,
    devices: &BTreeSet<DeviceId>,
    inverse: Command,
) -> Command {
    let after = programming_content(project, devices);
    let mut undo = Vec::new();
    for (device, content) in before {
        let Some(now) = after.get(&device) else {
            continue;
        };
        let parts = content.changed_parts(now);
        if parts.is_empty() {
            continue;
        }
        undo.push(
            set(project, device, ProgrammingChange::mark(parts, false))
                .expect("a device present after the command accepts a belief change"),
        );
    }
    if undo.is_empty() {
        return inverse;
    }
    undo.reverse();
    undo.push(inverse);
    Command::Batch(undo)
}

/// Rewrites a history command so that it never claims `parts` of `device`
/// programmed. Devices restored whole (`RestoreDevice`/`CreateDevice`) cannot
/// be the device just written: it exists, so no entry recreates it.
pub(super) fn drop_programmed_claims(cmd: &mut Command, device: DeviceId, parts: ProgrammingParts) {
    match cmd {
        Command::SetDeviceProgramming { device: d, change } if *d == device => {
            *change = change.without_programmed_claims(parts);
        }
        Command::Batch(commands) => {
            for command in commands {
                drop_programmed_claims(command, device, parts);
            }
        }
        _ => {}
    }
}
