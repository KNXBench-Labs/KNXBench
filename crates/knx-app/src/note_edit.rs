//! Previews project-owned notes and publishes edits only after durable admission.

use knx_core::note::ProjectNoteTarget;
use knx_core::{CommandStack, ProjectNoteId};
use knx_store::project_history::NativeSnapshot;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteRevision {
    pub project_incarnation: String,
    pub generation: u64,
}

#[derive(Clone, PartialEq, Eq)]
pub struct NoteDraft {
    pub target: ProjectNoteTarget,
    pub title: String,
    pub body: String,
    pub include_in_documentation: bool,
}

#[derive(Clone, PartialEq, Eq)]
pub enum NoteAction {
    Create { draft: NoteDraft, position: usize },
    Replace { id: ProjectNoteId, draft: NoteDraft },
    Remove { id: ProjectNoteId },
    Move { id: ProjectNoteId, position: usize },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NoteEditError {
    InvalidRevision,
    Conflict,
    Note(knx_core::note::NoteError),
    Allocation(knx_core::project::IdAllocationError),
    Command(knx_core::CommandError),
    Persistence(String),
}

pub struct NoteEditPreview {
    before: NativeSnapshot,
    revision: NoteRevision,
    command: knx_core::Command,
    created_id: Option<ProjectNoteId>,
    no_op: bool,
}

impl NoteEditPreview {
    pub fn created_id(&self) -> Option<ProjectNoteId> {
        self.created_id
    }
    pub fn is_no_op(&self) -> bool {
        self.no_op
    }
}

#[derive(Debug, Clone, Copy)]
pub struct NoteNativeTarget<'a> {
    pub path: &'a Path,
    pub history_generation: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NoteEditOutcome {
    pub changed: bool,
    pub native_generation: Option<i64>,
}

pub fn preview_note_edit(
    snapshot: &NativeSnapshot,
    revision: &NoteRevision,
    action: NoteAction,
) -> Result<NoteEditPreview, NoteEditError> {
    if revision.project_incarnation.is_empty()
        || revision.project_incarnation.len() > 128
        || revision.generation > 9_007_199_254_740_991
    {
        return Err(NoteEditError::InvalidRevision);
    }
    let (command, created_id) = match action {
        NoteAction::Create { draft, position } => {
            let mut ids = snapshot.project.ids.clone();
            let id = ids
                .next_project_note_id()
                .map_err(NoteEditError::Allocation)?;
            let note = knx_core::note::ProjectNote::new(id, draft.target, draft.title, draft.body)
                .map_err(NoteEditError::Note)?
                .with_documentation(draft.include_in_documentation);
            (
                knx_core::Command::InsertProjectNote { note, position },
                Some(id),
            )
        }
        NoteAction::Replace { id, draft } => {
            let note = knx_core::note::ProjectNote::new(id, draft.target, draft.title, draft.body)
                .map_err(NoteEditError::Note)?
                .with_documentation(draft.include_in_documentation);
            (knx_core::Command::ReplaceProjectNote { note }, None)
        }
        NoteAction::Remove { id } => (knx_core::Command::RemoveProjectNote { id }, None),
        NoteAction::Move { id, position } => {
            (knx_core::Command::MoveProjectNote { id, position }, None)
        }
    };
    let mut candidate = snapshot.project.clone();
    command
        .apply(&mut candidate)
        .map_err(NoteEditError::Command)?;
    Ok(NoteEditPreview {
        before: snapshot.clone(),
        revision: revision.clone(),
        command,
        created_id,
        no_op: candidate == snapshot.project,
    })
}

pub fn apply_note_edit(
    snapshot: &mut NativeSnapshot,
    stack: &mut CommandStack,
    revision: &NoteRevision,
    preview: &NoteEditPreview,
    native: Option<NoteNativeTarget<'_>>,
) -> Result<NoteEditOutcome, NoteEditError> {
    if snapshot != &preview.before || revision != &preview.revision {
        return Err(NoteEditError::Conflict);
    }
    if preview.no_op {
        return Ok(NoteEditOutcome {
            changed: false,
            native_generation: None,
        });
    }
    let mut candidate = snapshot.clone();
    let mut history = stack.clone();
    history
        .do_command(&mut candidate.project, preview.command.clone())
        .map_err(NoteEditError::Command)?;
    let native_generation = if let Some(target) = native {
        let expected_hash = preview
            .before
            .semantic_hash()
            .map_err(|error| NoteEditError::Persistence(error.to_string()))?;
        let conn = knx_store::project_history::open_editor_store(target.path, false)
            .map_err(|error| NoteEditError::Persistence(error.to_string()))?;
        Some(
            knx_store::project_history::save_editor_if_unchanged(
                &conn,
                &candidate,
                &history,
                target.history_generation,
                &expected_hash,
                false,
            )
            .map_err(|error| NoteEditError::Persistence(error.to_string()))?,
        )
    } else {
        None
    };
    *snapshot = candidate;
    *stack = history;
    Ok(NoteEditOutcome {
        changed: true,
        native_generation,
    })
}
