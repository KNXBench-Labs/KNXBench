//! Applies the Function commands of ADR-0124 and returns their inverses.
//!
//! Applies the Function commands of ADR-0124. Each returns its inverse and
//! leaves the project untouched on refusal: every check runs before the
//! first mutation.

use super::{
    check_id_free, require_unique_building_part, require_unique_group_address, Command,
    CommandError, IdKind,
};
use crate::function::{Function, FunctionDetails, FunctionError, FunctionMember};
use crate::{BuildingPartId, FunctionId, FunctionMemberId, Project};

fn index_of(project: &Project, id: FunctionId) -> Result<usize, CommandError> {
    project
        .functions
        .iter()
        .position(|f| f.id == id)
        .ok_or(CommandError::FunctionNotFound(id))
}

/// The installation index owning `location`, the Function's scope.
fn scope(project: &Project, location: BuildingPartId) -> Result<usize, CommandError> {
    require_unique_building_part(project, location)
}

fn invalid(error: FunctionError) -> CommandError {
    CommandError::InvalidFunction(error)
}

/// A member that may join a Function scoped to installation `scope`:
/// unused member id, the group address in that installation, and not
/// already a member of `existing`.
fn check_member(
    project: &Project,
    scope: usize,
    existing: &[FunctionMember],
    member: &FunctionMember,
) -> Result<(), CommandError> {
    member.validate().map_err(invalid)?;
    if member.id.0 == 0 || member_id_taken(project, member.id) {
        return Err(CommandError::IdInUse {
            kind: IdKind::FunctionMember,
            id: member.id.0,
        });
    }
    let owner = require_unique_group_address(project, member.group_address)?;
    if owner != scope {
        return Err(CommandError::CrossInstallation {
            from: project.installations[scope].id,
            to: project.installations[owner].id,
        });
    }
    if existing
        .iter()
        .any(|m| m.group_address == member.group_address)
    {
        return Err(invalid(FunctionError::DuplicateMember(
            member.group_address,
        )));
    }
    Ok(())
}

fn member_id_taken(project: &Project, id: FunctionMemberId) -> bool {
    project
        .functions
        .iter()
        .flat_map(|f| &f.members)
        .any(|m| m.id == id)
}

pub(super) fn create(project: &mut Project, function: &Function) -> Result<Command, CommandError> {
    if function.id.0 == 0 {
        return Err(CommandError::IdInUse {
            kind: IdKind::Function,
            id: 0,
        });
    }
    check_id_free(project, IdKind::Function, function.id.0)?;
    function.details.validate().map_err(invalid)?;
    let scope = scope(project, function.location)?;
    let mut accepted: Vec<FunctionMember> = Vec::new();
    for member in &function.members {
        check_member(project, scope, &accepted, member)?;
        if accepted.iter().any(|m| m.id == member.id) {
            return Err(CommandError::IdInUse {
                kind: IdKind::FunctionMember,
                id: member.id.0,
            });
        }
        accepted.push(member.clone());
    }
    project.functions.push(function.clone());
    Ok(Command::DeleteFunction { id: function.id })
}

pub(super) fn delete(project: &mut Project, id: FunctionId) -> Result<Command, CommandError> {
    let position = index_of(project, id)?;
    let function = project.functions.remove(position);
    Ok(Command::RestoreFunction { function, position })
}

/// Undo-only: imported text is restored as it was, without edit admission.
pub(super) fn restore(
    project: &mut Project,
    function: &Function,
    position: usize,
) -> Result<Command, CommandError> {
    check_id_free(project, IdKind::Function, function.id.0)?;
    if position > project.functions.len() {
        return Err(invalid(FunctionError::InvalidPosition(position)));
    }
    if let Some(taken) = function
        .members
        .iter()
        .find(|m| member_id_taken(project, m.id))
    {
        return Err(CommandError::IdInUse {
            kind: IdKind::FunctionMember,
            id: taken.id.0,
        });
    }
    scope(project, function.location)?;
    project.functions.insert(position, function.clone());
    Ok(Command::DeleteFunction { id: function.id })
}

pub(super) fn update(
    project: &mut Project,
    id: FunctionId,
    details: &FunctionDetails,
    admit: bool,
) -> Result<Command, CommandError> {
    let index = index_of(project, id)?;
    if admit {
        details.validate().map_err(invalid)?;
    }
    let previous = std::mem::replace(&mut project.functions[index].details, details.clone());
    Ok(Command::RestoreFunctionDetails {
        id,
        details: previous,
    })
}

pub(super) fn relocate(
    project: &mut Project,
    id: FunctionId,
    location: BuildingPartId,
) -> Result<Command, CommandError> {
    let index = index_of(project, id)?;
    let from = scope(project, project.functions[index].location)?;
    let to = scope(project, location)?;
    if from != to {
        return Err(CommandError::CrossInstallation {
            from: project.installations[from].id,
            to: project.installations[to].id,
        });
    }
    let previous = std::mem::replace(&mut project.functions[index].location, location);
    Ok(Command::MoveFunction {
        id,
        location: previous,
    })
}

pub(super) fn add_member(
    project: &mut Project,
    function: FunctionId,
    member: &FunctionMember,
    position: usize,
    admit: bool,
) -> Result<Command, CommandError> {
    let index = index_of(project, function)?;
    let owner = scope(project, project.functions[index].location)?;
    if admit {
        check_member(project, owner, &project.functions[index].members, member)?;
    } else if member_id_taken(project, member.id) {
        return Err(CommandError::IdInUse {
            kind: IdKind::FunctionMember,
            id: member.id.0,
        });
    }
    if position > project.functions[index].members.len() {
        return Err(invalid(FunctionError::InvalidPosition(position)));
    }
    project.functions[index]
        .members
        .insert(position, member.clone());
    Ok(Command::RemoveFunctionMember {
        function,
        member: member.id,
    })
}

pub(super) fn remove_member(
    project: &mut Project,
    function: FunctionId,
    member: FunctionMemberId,
) -> Result<Command, CommandError> {
    let index = index_of(project, function)?;
    let position = project.functions[index]
        .members
        .iter()
        .position(|m| m.id == member)
        .ok_or(CommandError::FunctionMemberNotFound(member))?;
    let member = project.functions[index].members.remove(position);
    Ok(Command::RestoreFunctionMember {
        function,
        member,
        position,
    })
}
