//! Prepared atomic parameter adoption using existing command and P3 rules.
//!
//! This is not device evidence. Only the App service can establish that a value
//! came from a genuine comparison; Core validates the whole project mutation.
use super::{stored_parameter_instance, Command, CommandError, CommandStack};
use crate::{DeviceId, Project};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParameterAdoption {
    pub reference: String,
    pub raw: String,
}

#[derive(Debug)]
pub enum AdoptionCommandError {
    Stale,
    EmptySelection,
    DuplicateReference(String),
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
        if stack.comparison_revision(device) != revision || revision == u64::MAX {
            return Err(AdoptionCommandError::Stale);
        }
        if project.devices.get(device).is_none() {
            return Err(CommandError::DeviceNotFound(device).into());
        }
        if values.is_empty() {
            return Err(AdoptionCommandError::EmptySelection);
        }
        // Admit both history directions, not only the current edit.
        stack.visit_snapshot_states(project, |_, _, _| true)?;
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
        let changed = !edits.is_empty();
        let mut candidate = project.clone();
        let mut candidate_history = stack.clone();
        if changed {
            edits.insert(0, Command::ReserveIds { through: ids });
            candidate_history.do_command(&mut candidate, Command::Batch(edits))?;
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
