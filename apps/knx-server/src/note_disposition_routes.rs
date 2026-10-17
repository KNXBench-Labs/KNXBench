//! Adapts note-aware deletes (ADR-0138) to HTTP: a named 409, then one bound apply.
//!
//! The entity delete routes ask [`refusal`] first. When the delete would
//! strand project notes, they answer `409 noteDispositionRequired` with every
//! affected note, the note targets the delete removes and the snapshot the
//! answer describes; nothing changes. `POST /api/project/delete-with-notes`
//! takes that snapshot back with one choice per note and runs the choices and
//! the delete as one command (one undo step) through the revision-bound
//! command path, so a project that moved on in between is refused as stale.

use std::sync::atomic::Ordering;

use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use knx_app::note_disposition::{self as disposition, Choice, Deletion};
use knx_core::note::{OrphanedNote, ProjectNoteTarget};
use knx_core::{
    AreaId, BuildingPartId, CommandError, DeviceId, GroupAddressId, GroupRangeId, LineId,
    ProjectNoteId,
};
use serde::Deserialize;
use serde_json::{json, Map};

use crate::domain::{self, BoundCommandError, SnapshotBinding};
use crate::errors::ApiError;
use crate::note_routes::Target;
use crate::{AppState, SharedState};

const REQUIRED: &str = "noteDispositionRequired";

pub(crate) fn router() -> Router<SharedState> {
    Router::new().route("/api/project/delete-with-notes", post(apply))
}

/// The structured refusal for a delete that would strand notes, computed
/// under the project lock; `None` lets the route run its delete as before
/// (including a delete the core refuses for another reason).
pub(crate) fn refusal(state: &AppState, deletion: &Deletion) -> Option<ApiError> {
    let lock = state.project.lock().expect("state mutex poisoned");
    let project = lock.as_ref()?;
    if project.notes.is_empty() {
        return None;
    }
    let preview = disposition::preview(project, deletion).ok()?;
    if preview.notes.is_empty() {
        return None;
    }
    let message = CommandError::NotesOrphaned(
        preview
            .notes
            .iter()
            .map(|note| OrphanedNote {
                id: note.id,
                target: note.target,
                title: note.title.clone(),
            })
            .collect(),
    )
    .to_string();
    let mut details = Map::new();
    details.insert(
        "notes".into(),
        preview
            .notes
            .iter()
            .map(|note| {
                json!({"id": note.id.0, "title": note.title, "target": Target::from(note.target)})
            })
            .collect(),
    );
    details.insert(
        "removedTargets".into(),
        preview
            .removed_targets
            .iter()
            .map(|target| json!(Target::from(*target)))
            .collect(),
    );
    details.insert(
        "serverIncarnation".into(),
        state.server_incarnation.clone().into(),
    );
    details.insert(
        "projectIncarnation".into(),
        state.project_incarnation.load(Ordering::Relaxed).into(),
    );
    details.insert(
        "snapshotRevision".into(),
        domain::current_project_revision(state).into(),
    );
    Some(ApiError::with_details(
        StatusCode::CONFLICT,
        REQUIRED,
        message,
        details,
    ))
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum DeleteWire {
    Devices { ids: Vec<u32> },
    GroupAddresses { ids: Vec<u32> },
    Area { id: u32 },
    Line { id: u32 },
    BuildingPart { id: u32 },
    GroupRange { id: u32 },
}

impl DeleteWire {
    fn deletion(&self) -> Deletion {
        match self {
            Self::Devices { ids } => Deletion::Devices(ids.iter().copied().map(DeviceId).collect()),
            Self::GroupAddresses { ids } => {
                Deletion::GroupAddresses(ids.iter().copied().map(GroupAddressId).collect())
            }
            Self::Area { id } => Deletion::Area(AreaId(*id)),
            Self::Line { id } => Deletion::Line(LineId(*id)),
            Self::BuildingPart { id } => Deletion::BuildingPart(BuildingPartId(*id)),
            Self::GroupRange { id } => Deletion::GroupRange(GroupRangeId(*id)),
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum ActionWire {
    /// Keep the note, attached to the project.
    Project,
    Move {
        target: Target,
    },
    Delete,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChoiceWire {
    note: u32,
    action: ActionWire,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyRequest {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    delete: DeleteWire,
    choices: Vec<ChoiceWire>,
}

async fn apply(
    State(state): State<SharedState>,
    Json(request): Json<ApplyRequest>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let binding = SnapshotBinding {
        server_incarnation: request.server_incarnation,
        project_incarnation: request.project_incarnation,
        snapshot_revision: request.snapshot_revision,
    };
    let deletion = request.delete.deletion();
    let choices: Vec<_> = request
        .choices
        .iter()
        .map(|choice| {
            let action = match &choice.action {
                ActionWire::Project => Choice::MoveTo(ProjectNoteTarget::Project),
                ActionWire::Move { target } => Choice::MoveTo(target.normalized()),
                ActionWire::Delete => Choice::Delete,
            };
            (ProjectNoteId(choice.note), action)
        })
        .collect();
    let detail = format!("{deletion:?} with note choices {choices:?}");
    domain::apply_bound(
        &state,
        &binding,
        "DeleteWithNoteChoices",
        None,
        detail,
        |project| disposition::plan(project, &deletion, &choices).map_err(|e| e.to_string()),
    )
    .map_err(|error| match error {
        BoundCommandError::NoProject => ApiError::bad_request("No project open"),
        BoundCommandError::Stale => ApiError::conflict(
            "noteDispositionStale",
            "The project changed since the affected notes were listed; nothing was deleted. Delete again to see the current notes.",
        ),
        BoundCommandError::Refused(message) => {
            ApiError::refused("noteDispositionRefused", message)
        }
        BoundCommandError::Failure(message) => ApiError::internal(format!(
            "The delete was not applied because it could not be persisted: {message}"
        )),
    })
    .map(Json)
}
