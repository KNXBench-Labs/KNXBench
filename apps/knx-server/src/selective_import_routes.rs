//! Adapts read-only source inspection and bound selective-import confirmation to HTTP.

use crate::{domain, errors::ApiError, paths::resolve_project_path, SharedState};
use axum::{extract::State, Json};
use knx_app::selective_import::{self as import, Selection};
use knx_store::project_history::NativeSnapshot;
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SourceRequest {
    path: String,
    #[serde(default)]
    password: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MergeRequest {
    path: String,
    #[serde(default)]
    password: Option<String>,
    source_hash: String,
    selection: Selection,
    #[serde(default)]
    confirmation_token: Option<String>,
}

pub(crate) async fn source(
    State(state): State<SharedState>,
    Json(request): Json<SourceRequest>,
) -> Result<Json<Value>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &request.path)?;
    let source = tokio::task::spawn_blocking(move || {
        let password = request
            .password
            .filter(|p| !p.is_empty())
            .map(knx_etsproj::ProjectPassword::new);
        import::read_source(&path, password.as_ref())
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))?
    .map_err(|e| ApiError::refused("selectiveImportSource", e))?;
    Ok(Json(
        serde_json::to_value(source.inventory()).map_err(|e| ApiError::internal(e.to_string()))?,
    ))
}

pub(crate) async fn preview(
    State(state): State<SharedState>,
    Json(request): Json<MergeRequest>,
) -> Result<Json<Value>, ApiError> {
    process(state, request, false).await
}
pub(crate) async fn apply(
    State(state): State<SharedState>,
    Json(request): Json<MergeRequest>,
) -> Result<Json<Value>, ApiError> {
    process(state, request, true).await
}

async fn process(
    state: SharedState,
    request: MergeRequest,
    apply: bool,
) -> Result<Json<Value>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &request.path)?;
    let result = tokio::task::spawn_blocking(move || {
        let password = request
            .password
            .filter(|p| !p.is_empty())
            .map(knx_etsproj::ProjectPassword::new);
        let source = import::read_source(&path, password.as_ref())
            .map_err(|e| ApiError::refused("selectiveImportSource", e))?;
        if source.source_hash != request.source_hash {
            return Err(ApiError::conflict(
                "selectiveImportStale",
                "source bytes changed; inspect the source again",
            ));
        }
        let mut project = state.project.lock().expect("state mutex poisoned");
        let project = project
            .as_mut()
            .ok_or_else(|| ApiError::refused("selectiveImportTarget", "no project open"))?;
        let mut snapshot = NativeSnapshot {
            project: project.clone(),
            opaque: state.opaque.lock().expect("state mutex poisoned").clone(),
            manufacturer_refs: state
                .manufacturer_refs
                .lock()
                .expect("state mutex poisoned")
                .clone(),
        };
        let plan = import::plan(&snapshot, &source, request.selection)
            .map_err(|e| ApiError::refused("selectiveImportConflict", e))?;
        let token = plan
            .confirmation_token(&format!(
                "{}:{}",
                state.server_incarnation,
                domain::current_project_revision(&state)
            ))
            .map_err(ApiError::internal)?;
        if !apply {
            return Ok(json!({"preview":plan.preview,"confirmationToken":token}));
        }
        if request.confirmation_token.as_deref() != Some(token.as_str()) {
            return Err(ApiError::conflict(
                "selectiveImportStale",
                "preview is missing or stale; preview again before confirming",
            ));
        }
        let mut stack = state.command_stack.lock().expect("state mutex poisoned");
        let mut candidate_stack = stack.clone();
        let preview = import::apply(&mut snapshot, &mut candidate_stack, plan)
            .map_err(|e| ApiError::conflict("selectiveImportStale", e))?;
        // Durable admission precedes every in-memory publication, context included.
        let path = state.store_path.lock().expect("state mutex poisoned");
        let mut generation = state
            .history_generation
            .lock()
            .expect("state mutex poisoned");
        if let Some(path) = path.as_deref() {
            *generation = knx_app::project_history::persist_working(
                path,
                &snapshot,
                &candidate_stack,
                *generation,
            )
            .map_err(ApiError::internal)?;
        }
        *project = snapshot.project;
        *state.opaque.lock().expect("state mutex poisoned") = snapshot.opaque;
        *state
            .manufacturer_refs
            .lock()
            .expect("state mutex poisoned") = snapshot.manufacturer_refs;
        *stack = candidate_stack;
        let revision = domain::next_project_revision(&state);
        let clean = state.clean_project.lock().expect("state mutex poisoned");
        let last_saved = state.last_saved_at.lock().expect("state mutex poisoned");
        let counts = *state.import_counts.lock().expect("state mutex poisoned");
        let tree = domain::tree_with_state(
            project,
            clean.as_ref(),
            &stack,
            counts,
            revision,
            &state,
            last_saved.as_deref(),
        );
        Ok(json!({"preview":preview,"project":tree}))
    })
    .await
    .map_err(|e| ApiError::internal(e.to_string()))??;
    Ok(Json(result))
}
