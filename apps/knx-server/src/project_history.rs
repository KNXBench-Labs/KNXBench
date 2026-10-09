//! Native project history HTTP adapters with revision-bound destructive consent.

use axum::{
    extract::{Path, State},
    Json,
};
use knx_store::project_history::{self as history, HistoryError, NativeSnapshot};
use serde::{Deserialize, Serialize};

use crate::{domain, errors::ApiError, AppState, SharedState};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VersionView {
    id: i64,
    created_at: String,
    reason: String,
    label: String,
    bytes: i64,
    image_hash: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryLimits {
    max_image_bytes: usize,
    max_history_bytes: i64,
    max_stack_states: usize,
    max_versions: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryView {
    format_version: i64,
    persistence: &'static str,
    server_incarnation: String,
    snapshot_revision: u64,
    generation: i64,
    undo_steps: usize,
    redo_steps: usize,
    total_bytes: i64,
    limits: HistoryLimits,
    versions: Vec<VersionView>,
    project: knx_projection::ProjectTree,
    #[serde(skip)]
    context_to_publish: Option<crate::GroupAddressContext>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct HistoryRequest {
    server_incarnation: String,
    snapshot_revision: u64,
    generation: i64,
    confirmed: bool,
    label: Option<String>,
}

fn storage_error(error: HistoryError) -> ApiError {
    match error {
        HistoryError::Stale => ApiError::conflict("projectHistoryStale", error.to_string()),
        HistoryError::Invalid(_) | HistoryError::Limit(_) => {
            ApiError::refused("projectHistoryUnavailable", error.to_string())
        }
        _ => ApiError::internal(error.to_string()),
    }
}

fn snapshot(state: &AppState, project: &knx_core::Project) -> NativeSnapshot {
    NativeSnapshot {
        project: project.clone(),
        opaque: state.opaque.lock().expect("state mutex poisoned").clone(),
        manufacturer_refs: state
            .manufacturer_refs
            .lock()
            .expect("state mutex poisoned")
            .clone(),
    }
}

fn version_views(versions: Vec<history::ProjectVersion>) -> Vec<VersionView> {
    versions
        .into_iter()
        .map(|v| VersionView {
            id: v.id,
            created_at: v.created_at,
            reason: v.reason,
            label: v.label,
            bytes: v.bytes,
            image_hash: v.image_hash,
        })
        .collect()
}

fn view_from_parts(
    state: &AppState,
    project: &knx_core::Project,
    persistence: &'static str,
    versions: Vec<VersionView>,
    total_bytes: i64,
) -> HistoryView {
    let stack = state.command_stack.lock().expect("state mutex poisoned");
    let generation = *state
        .history_generation
        .lock()
        .expect("state mutex poisoned");
    let clean = state.clean_project.lock().expect("state mutex poisoned");
    let last_saved = state.last_saved_at.lock().expect("state mutex poisoned");
    let revision = domain::current_project_revision(state);
    let tree = domain::tree_with_state(
        project,
        clean.as_ref(),
        &stack,
        *state.import_counts.lock().expect("state mutex poisoned"),
        revision,
        &state.server_incarnation,
        last_saved.as_deref(),
    );
    let (undo_steps, redo_steps) = stack.history_lengths();
    HistoryView {
        format_version: history::FORMAT_VERSION,
        persistence,
        server_incarnation: state.server_incarnation.clone(),
        snapshot_revision: revision,
        generation,
        undo_steps,
        redo_steps,
        total_bytes,
        limits: HistoryLimits {
            max_image_bytes: history::MAX_IMAGE_BYTES,
            max_history_bytes: history::MAX_HISTORY_BYTES,
            max_stack_states: history::MAX_STACK_STATES,
            max_versions: history::MAX_VERSIONS,
        },
        versions,
        project: tree,
        context_to_publish: None,
    }
}

fn view_locked(state: &AppState, project: &knx_core::Project) -> Result<HistoryView, ApiError> {
    let path = state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .clone();
    let generation = *state
        .history_generation
        .lock()
        .expect("state mutex poisoned");
    let (persistence, versions, total_bytes) = match path.as_deref() {
        None => ("session", Vec::new(), 0),
        Some(path) => {
            let admitted = knx_store::open_existing_read_only(path)
                .map_err(|e| ApiError::internal(e.to_string()))?;
            let conn = admitted.conn;
            let read = conn
                .unchecked_transaction()
                .map_err(|e| ApiError::internal(e.to_string()))?;
            let recovered = history::load_editor(&read).map_err(storage_error)?;
            if recovered.as_ref().map_or(0, |h| h.generation) != generation {
                return Err(ApiError::conflict(
                    "projectHistoryStale",
                    "project history changed in another editor; reopen before editing",
                ));
            }
            let versions = version_views(history::list_versions(&read).map_err(storage_error)?);
            let total_bytes = history::stored_history_bytes(&read).map_err(storage_error)?;
            read.commit()
                .map_err(|e| ApiError::internal(e.to_string()))?;
            ("native", versions, total_bytes)
        }
    };
    Ok(view_from_parts(
        state,
        project,
        persistence,
        versions,
        total_bytes,
    ))
}

pub async fn get(State(state): State<SharedState>) -> Result<Json<HistoryView>, ApiError> {
    tokio::task::spawn_blocking(move || {
        let project = state.project.lock().expect("state mutex poisoned");
        view_locked(
            &state,
            project
                .as_ref()
                .ok_or_else(|| ApiError::bad_request("no project open"))?,
        )
        .map(Json)
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?
}

enum Operation {
    Create,
    Restore(i64),
    Delete(i64),
    ClearUndo,
}

fn mutate(
    state: &AppState,
    request: HistoryRequest,
    operation: Operation,
) -> Result<HistoryView, ApiError> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    let current = project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    if request.server_incarnation != state.server_incarnation
        || request.snapshot_revision != domain::current_project_revision(state)
        || request.generation
            != *state
                .history_generation
                .lock()
                .expect("state mutex poisoned")
    {
        return Err(ApiError::conflict(
            "projectHistoryStale",
            "project or history changed since this action was shown; refresh before retrying",
        ));
    }
    if !matches!(operation, Operation::Create) && !request.confirmed {
        return Err(ApiError::refused(
            "projectHistoryConsentRequired",
            "explicit confirmation is required for restore, deletion or clearing undo history",
        ));
    }
    if state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .is_none()
        && matches!(operation, Operation::ClearUndo)
    {
        *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
        domain::next_project_revision(state);
        return Ok(view_from_parts(state, current, "session", Vec::new(), 0));
    }
    let path = state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .clone()
        .ok_or_else(|| {
            ApiError::refused(
                "projectHistoryNativeRequired",
                "save the project as .knxdb before creating or restoring persistent versions",
            )
        })?;
    let current_snapshot = snapshot(state, current);
    let conn =
        history::open_editor_store(&path, false).map_err(|e| ApiError::internal(e.to_string()))?;
    let action = match operation {
        Operation::Create => history::HistoryAction::Create(
            request
                .label
                .as_deref()
                .ok_or_else(|| ApiError::bad_request("version label is required"))?,
        ),
        Operation::Delete(id) => history::HistoryAction::Delete(id),
        Operation::ClearUndo => history::HistoryAction::ClearUndo,
        Operation::Restore(id) => history::HistoryAction::Restore(id),
    };
    let change = history::change_history(&conn, &current_snapshot, request.generation, action)
        .map_err(storage_error)?;
    if matches!(operation, Operation::ClearUndo | Operation::Restore(_)) {
        *state.command_stack.lock().expect("state mutex poisoned") = knx_core::CommandStack::new();
    }
    if let Some(restored) = change.restored {
        let restored_snapshot = restored.working;
        *project = Some(restored_snapshot.project.clone());
        *state.clean_project.lock().expect("state mutex poisoned") =
            Some(restored_snapshot.project);
        *state.opaque.lock().expect("state mutex poisoned") = restored_snapshot.opaque;
        *state
            .manufacturer_refs
            .lock()
            .expect("state mutex poisoned") = restored_snapshot.manufacturer_refs;
        *state.import_counts.lock().expect("state mutex poisoned") = (0, 0);
        *state.last_saved_at.lock().expect("state mutex poisoned") =
            Some(crate::session_log::now());
        state
            .catalog_requests
            .lock()
            .expect("state mutex poisoned")
            .clear();
        *state
            .device_download_plan
            .lock()
            .expect("state mutex poisoned") = None;
    }
    *state
        .history_generation
        .lock()
        .expect("state mutex poisoned") = change.generation;
    domain::next_project_revision(state);
    // Store response facts were collected before commit. No fallible file read
    // occurs after the durable mutation and its in-memory publication.
    let mut view = view_from_parts(
        state,
        project.as_ref().expect("project remains present"),
        "native",
        version_views(change.versions),
        change.total_bytes,
    );
    if matches!(operation, Operation::Restore(_)) {
        // Whole-project replacement republishes the already-existing monitor's
        // interpretation context, not protocol traffic or hardware state.
        view.context_to_publish = Some(crate::GroupAddressContext::from_project(project.as_ref()));
    }
    Ok(view)
}

async fn run(
    state: SharedState,
    body: HistoryRequest,
    operation: Operation,
) -> Result<Json<HistoryView>, ApiError> {
    // Share the ordinary open/new/undo/style publication ordering. Never hold a
    // synchronous project lock while awaiting the monitor's async session lock.
    let _publication = if matches!(operation, Operation::Restore(_)) {
        Some(state.group_address_style_publication.lock().await)
    } else {
        None
    };
    let worker_state = state.clone();
    let mut view = tokio::task::spawn_blocking(move || mutate(&worker_state, body, operation))
        .await
        .map_err(|e| ApiError::internal(e.to_string()))??;
    if let Some(context) = view.context_to_publish.take() {
        if let Some(session) = state.bus_session.lock().await.as_ref() {
            session.update_group_address_context(context);
            view.project.group_address_context_session_id = Some(session.id());
        }
    }
    Ok(Json(view))
}

pub async fn create(
    State(state): State<SharedState>,
    Json(body): Json<HistoryRequest>,
) -> Result<Json<HistoryView>, ApiError> {
    run(state, body, Operation::Create).await
}
pub async fn restore(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Json(body): Json<HistoryRequest>,
) -> Result<Json<HistoryView>, ApiError> {
    run(state, body, Operation::Restore(id)).await
}
pub async fn delete(
    State(state): State<SharedState>,
    Path(id): Path<i64>,
    Json(body): Json<HistoryRequest>,
) -> Result<Json<HistoryView>, ApiError> {
    run(state, body, Operation::Delete(id)).await
}
pub async fn clear_undo(
    State(state): State<SharedState>,
    Json(body): Json<HistoryRequest>,
) -> Result<Json<HistoryView>, ApiError> {
    run(state, body, Operation::ClearUndo).await
}
