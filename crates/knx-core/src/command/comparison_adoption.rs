//! Prepared atomic parameter adoption using existing command and P3 rules.
//!
//! This is not device evidence. Only the App service can establish that a value
//! came from a genuine comparison; Core validates the whole project mutation.
use super::{stored_parameter_instance, Command, CommandError, CommandStack};
use crate::{ComObjectInstanceId, DeviceId, Direction, GroupAddressId, Project};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterAdoption {
    pub reference: String,
    pub raw: String,
}

/// One link edit on an object of the adopted device. The App derives these
/// only from attributed observations; Core still validates every command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LinkAdoption {
    /// `true` adds the link, `false` removes the existing one.
    pub add: bool,
    pub com_object: ComObjectInstanceId,
    pub ga: GroupAddressId,
    pub direction: Direction,
}

#[derive(Debug)]
pub enum AdoptionCommandError {
    Stale,
    EmptySelection,
    DuplicateReference(String),
    /// The same link edit twice, or an add and a remove of one link.
    DuplicateLink(ComObjectInstanceId, GroupAddressId),
    /// A link edit on an object of another device.
    ForeignComObject(ComObjectInstanceId),
    Command(CommandError),
    Allocation(String),
}
impl std::fmt::Display for AdoptionCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Stale => f.write_str("comparison project or complete command history changed"),
            Self::EmptySelection => f.write_str("select at least one supported comparison row"),
            Self::DuplicateReference(reference) => {
                write!(f, "ambiguous parameter reference: {reference}")
            }
            Self::DuplicateLink(object, ga) => write!(
                f,
                "duplicate link adoption for object {} and group address {}",
                object.0, ga.0
            ),
            Self::ForeignComObject(object) => write!(
                f,
                "communication object {} does not belong to the compared device",
                object.0
            ),
            Self::Command(error) => write!(f, "comparison adoption command: {error}"),
            Self::Allocation(error) => write!(f, "comparison adoption allocation: {error}"),
        }
    }
}
impl std::error::Error for AdoptionCommandError {}
impl From<CommandError> for AdoptionCommandError {
    fn from(value: CommandError) -> Self {
        Self::Command(value)
    }
}

/// Exact before/after states from a single actual stack command. No caller can
/// replace the command, candidate, or expected history after preparation.
#[derive(Debug)]
pub struct PreparedComparisonAdoption {
    before: Project,
    history: CommandStack,
    candidate: Project,
    candidate_history: CommandStack,
    changed: bool,
}
impl PreparedComparisonAdoption {
    pub fn prepare(
        project: &Project,
        stack: &CommandStack,
        device: DeviceId,
        revision: u64,
        values: Vec<ParameterAdoption>,
    ) -> Result<Self, AdoptionCommandError> {
        if values.is_empty() {
            return Err(AdoptionCommandError::EmptySelection);
        }
        Self::prepare_with_links(project, stack, device, revision, values, Vec::new())
    }

    /// Parameters and link edits as one command. Removals run first, then
    /// additions in the given order, then parameters. Empty input is a no-op
    /// candidate: the App already refused an empty row selection.
    pub fn prepare_with_links(
        project: &Project,
        stack: &CommandStack,
        device: DeviceId,
        revision: u64,
        values: Vec<ParameterAdoption>,
        links: Vec<LinkAdoption>,
    ) -> Result<Self, AdoptionCommandError> {
        if stack.comparison_revision(device) != revision || revision == u64::MAX {
            return Err(AdoptionCommandError::Stale);
        }
        if project.devices.get(device).is_none() {
            return Err(CommandError::DeviceNotFound(device).into());
        }
        // Admit both history directions, not only the current edit.
        stack.visit_snapshot_states(project, |_, _, _| true)?;
        let mut seen = BTreeSet::new();
        let mut removals = Vec::new();
        let mut additions = Vec::new();
        for link in links {
            let owner = project
                .devices
                .com_object(link.com_object)
                .ok_or(CommandError::ComObjectNotFound(link.com_object))?
                .device;
            if owner != device {
                return Err(AdoptionCommandError::ForeignComObject(link.com_object));
            }
            if !seen.insert((link.com_object, link.ga)) {
                return Err(AdoptionCommandError::DuplicateLink(
                    link.com_object,
                    link.ga,
                ));
            }
            let command = if link.add {
                Command::LinkComObject {
                    com_object: link.com_object,
                    ga: link.ga,
                    direction: link.direction,
                }
            } else {
                Command::UnlinkComObject {
                    com_object: link.com_object,
                    ga: link.ga,
                    direction: link.direction,
                }
            };
            if link.add {
                additions.push(command);
            } else {
                removals.push(command);
            }
        }
        let mut references = BTreeSet::new();
        let mut ids = project.ids.clone();
        let mut edits = Vec::new();
        for value in values {
            if value.reference.is_empty() || !references.insert(value.reference.clone()) {
                return Err(AdoptionCommandError::DuplicateReference(value.reference));
            }
            let existing = stored_parameter_instance(project, device, &value.reference)?;
            if existing.is_some_and(|id| {
                project
                    .installations
                    .iter()
                    .flat_map(|i| &i.parameters)
                    .any(|row| row.id == id && row.raw == value.raw)
            }) {
                continue;
            }
            let id = match existing {
                Some(id) => id,
                None => ids
                    .next_parameter_instance_id()
                    .map_err(|e| AdoptionCommandError::Allocation(e.to_string()))?,
            };
            edits.push(Command::SetParameterValue {
                id,
                device,
                ets_id: value.reference,
                raw: value.raw,
            });
        }
        let mut commands = removals;
        commands.extend(additions);
        commands.extend(edits);
        let changed = !commands.is_empty();
        let mut candidate = project.clone();
        let mut candidate_history = stack.clone();
        if changed {
            commands.insert(0, Command::ReserveIds { through: ids });
            candidate_history.do_command(&mut candidate, Command::Batch(commands))?;
        }
        candidate_history.visit_snapshot_states(&candidate, |_, _, _| true)?;
        Ok(Self {
            before: project.clone(),
            history: stack.clone(),
            candidate,
            candidate_history,
            changed,
        })
    }
    pub fn candidate(&self) -> (&Project, &CommandStack) {
        (&self.candidate, &self.candidate_history)
    }
    pub fn changed(&self) -> bool {
        self.changed
    }
    pub fn is_current(&self, project: &Project, stack: &CommandStack) -> bool {
        *project == self.before && *stack == self.history
    }
    /// Publishes only the command-generated candidate, after exact before-state
    /// comparison. The App persists/preflights this candidate before calling.
    pub fn apply(
        &self,
        project: &mut Project,
        stack: &mut CommandStack,
    ) -> Result<(), AdoptionCommandError> {
        if !self.is_current(project, stack) {
            return Err(AdoptionCommandError::Stale);
        }
        *project = self.candidate.clone();
        *stack = self.candidate_history.clone();
        Ok(())
    }
}
