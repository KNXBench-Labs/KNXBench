//! Adds newly configured communication objects without replacing existing instances.
use super::{Command, CommandError, IdKind};
use crate::{ComObjectInstance, ComObjectInstanceId, DeviceId, ProgramDefaults, Project};
use std::collections::HashSet;

pub(super) fn add(
    project: &mut Project,
    device: DeviceId,
    objects: &[ComObjectInstance],
    defaults: &[(ComObjectInstanceId, ProgramDefaults)],
) -> Result<Command, CommandError> {
    let owner = project
        .devices
        .get(device)
        .ok_or(CommandError::DeviceNotFound(device))?;
    let mut ids = HashSet::new();
    for object in objects {
        // The inverse is an unlinked append removal; initial links would
        // make a freshly applied command impossible to undo safely.
        if object.device != device
            || !object.links.is_empty()
            || object.module_instance.is_some_and(|id| {
                project
                    .devices
                    .module_instance(id)
                    .is_none_or(|module| module.device != device)
            })
            || owner.com_objects.contains(&object.id)
        {
            return Err(CommandError::InvalidComObjectBatch(device));
        }
        if !ids.insert(object.id) || project.devices.com_object(object.id).is_some() {
            return Err(CommandError::IdInUse {
                kind: IdKind::ComObjectInstance,
                id: object.id.0,
            });
        }
    }
    let mut default_ids = HashSet::new();
    if defaults
        .iter()
        .any(|(id, _)| !ids.contains(id) || !default_ids.insert(*id))
    {
        return Err(CommandError::InvalidComObjectBatch(device));
    }
    let added = objects.iter().map(|o| o.id).collect::<Vec<_>>();
    project
        .devices
        .get_mut(device)
        .expect("checked")
        .com_objects
        .extend(&added);
    for object in objects {
        project.devices.insert_com_object(object.clone());
    }
    for (id, defaults) in defaults {
        project.devices.set_program_defaults(*id, defaults.clone());
    }
    Ok(Command::RemoveAddedComObjects { device, ids: added })
}

pub(super) fn remove(
    project: &mut Project,
    device: DeviceId,
    ids: &[ComObjectInstanceId],
) -> Result<Command, CommandError> {
    let owner = project
        .devices
        .get(device)
        .ok_or(CommandError::DeviceNotFound(device))?;
    // Undo removes only the exact appended suffix, never reorders old objects.
    if !owner.com_objects.ends_with(ids) || ids.iter().collect::<HashSet<_>>().len() != ids.len() {
        return Err(CommandError::InvalidComObjectBatch(device));
    }
    let mut objects = Vec::with_capacity(ids.len());
    let mut defaults = Vec::new();
    for id in ids {
        let object = project
            .devices
            .com_object(*id)
            .ok_or(CommandError::ComObjectNotFound(*id))?;
        if object.device != device {
            return Err(CommandError::InvalidComObjectBatch(device));
        }
        if !object.links.is_empty() {
            return Err(CommandError::DeviceHasLinks(device));
        }
        objects.push(object.clone());
        if let Some(value) = project.devices.program_defaults(*id) {
            defaults.push((*id, value.clone()));
        }
    }
    let owner = project.devices.get_mut(device).expect("checked");
    owner
        .com_objects
        .truncate(owner.com_objects.len() - ids.len());
    for id in ids {
        project.devices.remove_com_object(*id);
    }
    Ok(Command::AddComObjects {
        device,
        objects,
        program_defaults: defaults,
    })
}
