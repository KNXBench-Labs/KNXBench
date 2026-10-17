//! Plans explicit note dispositions so a delete never strands or drops a note.
//!
//! A delete that removes an entity a project note points at is refused by the
//! core ([`CommandError::NotesOrphaned`]). This module is the explicit way
//! through (ADR-0138): [`preview`] runs the whole delete on a private copy
//! without notes and lists every note it would strand, cascades included (a
//! device takes its communication objects with it); [`plan`] turns one choice
//! per stranded note into a single `Batch` of note edits followed by the
//! delete, so the change is one undo step and undo restores the entity before
//! its notes. Nothing here retargets or deletes a note on its own.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use knx_core::note::{NoteError, ProjectNoteTarget};
use knx_core::{
    AreaId, BuildingPartId, Command, CommandError, DeviceId, GroupAddressId, GroupRangeId, LineId,
    Project, ProjectNoteId,
};

use crate::note_targets::note_target_catalogue;

/// One delete the web and CLI already offer, by entity kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Deletion {
    Devices(Vec<DeviceId>),
    GroupAddresses(Vec<GroupAddressId>),
    Area(AreaId),
    Line(LineId),
    BuildingPart(BuildingPartId),
    GroupRange(GroupRangeId),
}

/// What happens to one stranded note. Keeping it on the project is
/// `MoveTo(ProjectNoteTarget::Project)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    MoveTo(ProjectNoteTarget),
    Delete,
}

/// A note the delete would strand, with the text the user recognises it by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffectedNote {
    pub id: ProjectNoteId,
    pub target: ProjectNoteTarget,
    pub title: String,
}

/// What a delete would do to notes, computed without changing the project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Preview {
    /// Stranded notes in collection order.
    pub notes: Vec<AffectedNote>,
    /// Note targets the delete removes, in catalogue order: a note cannot be
    /// moved to any of these. Empty when no note is stranded.
    pub removed_targets: Vec<ProjectNoteTarget>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    EmptyDeletion,
    DuplicateTarget,
    /// The delete itself is refused, independent of notes.
    Delete(CommandError),
    DuplicateChoice(ProjectNoteId),
    /// A choice names a note this delete does not strand.
    NotAffected(ProjectNoteId),
    /// Stranded notes without a choice, in collection order.
    MissingChoice(Vec<ProjectNoteId>),
    /// The chosen target is removed by the same delete.
    RemovedTarget {
        note: ProjectNoteId,
        target: ProjectNoteTarget,
    },
    InvalidTarget {
        note: ProjectNoteId,
        error: NoteError,
    },
}

impl fmt::Display for PlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyDeletion => f.write_str("nothing to delete"),
            Self::DuplicateTarget => f.write_str("an entity is named twice in one delete"),
            Self::Delete(error) => write!(f, "{error}"),
            Self::DuplicateChoice(id) => write!(f, "note {id} has more than one choice"),
            Self::NotAffected(id) => write!(f, "note {id} is not affected by this delete"),
            Self::MissingChoice(ids) => {
                let list: Vec<_> = ids.iter().map(ToString::to_string).collect();
                write!(f, "notes without a choice: {}", list.join(", "))
            }
            Self::RemovedTarget { note, target } => write!(
                f,
                "note {note} cannot move to {target}, which this delete removes"
            ),
            Self::InvalidTarget { note, error } => {
                write!(f, "note {note} cannot move there: {error}")
            }
        }
    }
}

impl std::error::Error for PlanError {}

impl Deletion {
    /// The core command the existing delete routes run for this deletion.
    pub fn command(&self) -> Result<Command, PlanError> {
        fn many<T: Copy + Ord>(
            ids: &[T],
            one: impl Fn(T) -> Command,
        ) -> Result<Command, PlanError> {
            if ids.is_empty() {
                return Err(PlanError::EmptyDeletion);
            }
            if ids.iter().collect::<BTreeSet<_>>().len() != ids.len() {
                return Err(PlanError::DuplicateTarget);
            }
            Ok(match ids {
                [id] => one(*id),
                _ => Command::Batch(ids.iter().map(|id| one(*id)).collect()),
            })
        }
        match self {
            Self::Devices(ids) => many(ids, |id| Command::DeleteDevice { id }),
            Self::GroupAddresses(ids) => many(ids, |id| Command::DeleteGroupAddress { id }),
            Self::Area(id) => Ok(Command::DeleteArea { id: *id }),
            Self::Line(id) => Ok(Command::DeleteLine { id: *id }),
            Self::BuildingPart(id) => Ok(Command::DeleteBuildingPart { id: *id }),
            Self::GroupRange(id) => Ok(Command::DeleteGroupRange { id: *id }),
        }
    }
}

/// The project after `deletion`, computed on a copy without notes so the
/// note refusal cannot hide what the delete itself does.
fn without_notes_after(project: &Project, deletion: &Deletion) -> Result<Project, PlanError> {
    let command = deletion.command()?;
    let mut candidate = project.clone();
    candidate.notes.clear();
    command.apply(&mut candidate).map_err(PlanError::Delete)?;
    Ok(candidate)
}

/// Lists the notes `deletion` would strand and, when there are any, the
/// targets it removes. Refuses with the delete's own reason when the delete
/// is not admitted.
pub fn preview(project: &Project, deletion: &Deletion) -> Result<Preview, PlanError> {
    let after = without_notes_after(project, deletion)?;
    let notes = stranded(project, &after);
    // The catalogue walk is only worth it when a dialog will offer moves.
    let removed_targets = if notes.is_empty() {
        Vec::new()
    } else {
        note_target_catalogue(project)
            .into_iter()
            .map(|option| option.target)
            .filter(|target| target.validate(&after).is_err())
            .collect()
    };
    Ok(Preview {
        notes,
        removed_targets,
    })
}

/// Notes that resolve in `project` and point at an entity `after` lacks.
fn stranded(project: &Project, after: &Project) -> Vec<AffectedNote> {
    project
        .notes
        .iter()
        .filter(|note| {
            note.target().validate(project).is_ok()
                && matches!(
                    note.target().validate(after),
                    Err(NoteError::MissingTarget(_))
                )
        })
        .map(|note| AffectedNote {
            id: note.id(),
            target: note.target(),
            title: note.title().to_owned(),
        })
        .collect()
}

/// One `Batch`: each chosen note edit in collection order, then the delete.
/// Every stranded note needs exactly one choice and no other note may have
/// one; a move must name a target that exists and survives the delete.
pub fn plan(
    project: &Project,
    deletion: &Deletion,
    choices: &[(ProjectNoteId, Choice)],
) -> Result<Command, PlanError> {
    let after = without_notes_after(project, deletion)?;
    let affected = stranded(project, &after);
    let mut chosen = BTreeMap::new();
    for (id, choice) in choices {
        if chosen.insert(*id, *choice).is_some() {
            return Err(PlanError::DuplicateChoice(*id));
        }
        if !affected.iter().any(|note| note.id == *id) {
            return Err(PlanError::NotAffected(*id));
        }
    }
    let missing: Vec<_> = affected
        .iter()
        .map(|note| note.id)
        .filter(|id| !chosen.contains_key(id))
        .collect();
    if !missing.is_empty() {
        return Err(PlanError::MissingChoice(missing));
    }
    let mut steps = Vec::with_capacity(affected.len() + 1);
    for note in project.notes.iter() {
        let Some(choice) = chosen.get(&note.id()) else {
            continue;
        };
        steps.push(match *choice {
            Choice::Delete => Command::RemoveProjectNote { id: note.id() },
            Choice::MoveTo(target) => {
                if let Err(error) = target.validate(&after) {
                    return Err(if target.validate(project).is_ok() {
                        PlanError::RemovedTarget {
                            note: note.id(),
                            target,
                        }
                    } else {
                        PlanError::InvalidTarget {
                            note: note.id(),
                            error,
                        }
                    });
                }
                Command::ReplaceProjectNote {
                    note: note.clone().with_target(target),
                }
            }
        });
    }
    let delete = deletion.command()?;
    if steps.is_empty() {
        return Ok(delete);
    }
    steps.push(delete);
    Ok(Command::Batch(steps))
}
