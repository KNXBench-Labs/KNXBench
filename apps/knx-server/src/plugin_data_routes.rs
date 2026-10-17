//! Adapts plug-in data adoption (ADR-0136) to HTTP: list, preview, token-only apply.
//!
//! The server keeps the one plan it last previewed, as `device_download_plan`
//! keeps a shown download plan, so apply writes exactly what the user saw:
//! the client sends only the confirmation token. The plan is used at most
//! once; a newer preview replaces it and cancel drops it.
//!
//! The opaque store is outside command Undo, so adoption is not a command.
//! Apply first records the current working state as a named project version
//! (the way back), then persists the adopted working state with the undo
//! stack unchanged (ADR-0136, amendment of 17 October 2026).

use crate::{domain, errors::ApiError, paths::resolve_project_path, AppState, SharedState};
use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
use knx_app::plugin_data::{self as adopt, Refusal};
use knx_app::selective_import as import;
use knx_store::project_history::{self as history, HistoryError};
use serde::Deserialize;
use serde_json::{json, Value};

/// Label of the version that keeps the state before an adoption.
pub(crate) const ROLLBACK_LABEL: &str = "Before plug-in data adoption";
/// Domain-separates web tokens from the CLI's file-bound tokens.
const TOKEN_SCOPE: &str = "web-adopt-v1";
const STALE: &str = "pluginDataStale";
const STALE_TEXT: &str =
    "the project or the preview changed since it was shown; nothing was adopted, preview again";

pub(crate) fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/project/plugin-data/candidates", post(candidates))
        .route("/api/project/plugin-data/preview", post(preview))
        .route("/api/project/plugin-data/apply", post(apply))
        .route("/api/project/plugin-data/cancel", post(cancel))
}

/// The plan last previewed and the binding it was shown under. Opaque
/// outside this module: only its routes read or replace it.
pub struct ShownAdoption {
    token: String,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    plan: adopt::Plan,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SourceRequest {
    path: String,
    #[serde(default)]
    password: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreviewRequest {
    path: String,
    #[serde(default)]
    password: Option<String>,
    source_hash: String,
    request: adopt::Request,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TokenRequest {
    confirmation_token: String,
}

fn no_project() -> ApiError {
    ApiError::refused("pluginDataTarget", "no project open")
}

fn native_required() -> ApiError {
    ApiError::refused(
        "pluginDataNativeRequired",
        "save the project as .knxdb first: the state before an adoption is kept as a project version",
    )
}

fn refused(refusal: Refusal) -> ApiError {
    if refusal == Refusal::Stale {
        return ApiError::conflict(STALE, STALE_TEXT);
    }
    let mut details = serde_json::Map::new();
    details.insert(
        "refusal".into(),
        serde_json::to_value(refusal.detail()).unwrap_or(Value::Null),
    );
    ApiError::with_details(
        StatusCode::UNPROCESSABLE_ENTITY,
        "pluginDataRefused",
        refusal.to_string(),
        details,
    )
}

fn rollback_error(error: HistoryError) -> ApiError {
    match error {
        HistoryError::Stale => ApiError::conflict(
            "projectHistoryStale",
            "project history changed in another editor; reopen before adopting. Nothing was adopted.",
        ),
        HistoryError::Invalid(_) | HistoryError::Limit(_) | HistoryError::ImageLimit(_) => {
            ApiError::refused(
                "pluginDataRollbackUnavailable",
                format!(
                    "the version that keeps the state before the adoption could not be written ({error}); nothing was adopted"
                ),
            )
        }
        other => ApiError::internal(other.to_string()),
    }
}

fn read_source(
    path: &std::path::Path,
    password: Option<String>,
) -> Result<import::Source, ApiError> {
    let password = password
        .filter(|p| !p.is_empty())
        .map(knx_etsproj::ProjectPassword::new);
    import::read_source(path, password.as_ref())
        .map_err(|e| ApiError::refused("pluginDataSource", e))
}

fn has_project(state: &AppState) -> bool {
    state
        .project
        .lock()
        .expect("state mutex poisoned")
        .is_some()
}

fn is_native(state: &AppState) -> bool {
    state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .is_some()
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, ApiError> + Send + 'static,
) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| ApiError::internal(e.to_string()))?
}

pub(crate) async fn candidates(
    State(state): State<SharedState>,
    Json(body): Json<SourceRequest>,
) -> Result<Json<Value>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &body.path)?;
    blocking(move || {
        if !has_project(&state) {
            return Err(no_project());
        }
        let source = read_source(&path, body.password)?;
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or_else(no_project)?;
        let target = crate::project_history::snapshot(&state, project);
        Ok(Json(json!({
            "sourceHash": source.source_hash,
            "persistence": if is_native(&state) { "native" } else { "session" },
            "candidates": adopt::candidates(&target, &source),
        })))
    })
    .await
}

pub(crate) async fn preview(
    State(state): State<SharedState>,
    Json(body): Json<PreviewRequest>,
) -> Result<Json<Value>, ApiError> {
    let path = resolve_project_path(&state.data_dir, &body.path)?;
    blocking(move || {
        if !has_project(&state) {
            return Err(no_project());
        }
        let source = read_source(&path, body.password)?;
        if source.source_hash != body.source_hash {
            return Err(ApiError::conflict(
                "pluginDataSourceChanged",
                "the source file changed since it was listed; choose it again",
            ));
        }
        let project = state.project.lock().expect("state mutex poisoned");
        let project = project.as_ref().ok_or_else(no_project)?;
        if !domain::snapshot_is_current(
            &state,
            &body.server_incarnation,
            body.project_incarnation,
            body.snapshot_revision,
        ) {
            return Err(ApiError::conflict(STALE, STALE_TEXT));
        }
        if !is_native(&state) {
            return Err(native_required());
        }
        let target = crate::project_history::snapshot(&state, project);
        let plan = adopt::plan(&target, &source, body.request).map_err(refused)?;
        let token = plan
            .confirmation_token(&format!(
                "{TOKEN_SCOPE}:{}:{}:{}",
                body.server_incarnation, body.project_incarnation, body.snapshot_revision
            ))
            .map_err(ApiError::internal)?;
        let preview = plan.preview.clone();
        *state.plugin_data_plan.lock().expect("state mutex poisoned") = Some(ShownAdoption {
            token: token.clone(),
            server_incarnation: body.server_incarnation,
            project_incarnation: body.project_incarnation,
            snapshot_revision: body.snapshot_revision,
            plan,
        });
        Ok(Json(
            json!({"preview": preview, "confirmationToken": token}),
        ))
    })
    .await
}

pub(crate) async fn apply(
    State(state): State<SharedState>,
    Json(body): Json<TokenRequest>,
) -> Result<Json<Value>, ApiError> {
    blocking(move || apply_locked(&state, &body.confirmation_token).map(Json)).await
}

fn apply_locked(state: &AppState, token: &str) -> Result<Value, ApiError> {
    let mut project = state.project.lock().expect("state mutex poisoned");
    // Taken only when the token matches: a wrong token leaves another
    // client's shown plan alone; a matching one is used at most once.
    let shown = {
        let mut slot = state.plugin_data_plan.lock().expect("state mutex poisoned");
        if slot.as_ref().is_none_or(|shown| shown.token != token) {
            return Err(ApiError::conflict(STALE, STALE_TEXT));
        }
        slot.take().expect("checked above")
    };
    let project = project.as_mut().ok_or_else(no_project)?;
    if !domain::snapshot_is_current(
        state,
        &shown.server_incarnation,
        shown.project_incarnation,
        shown.snapshot_revision,
    ) {
        return Err(ApiError::conflict(STALE, STALE_TEXT));
    }
    let path = state
        .store_path
        .lock()
        .expect("state mutex poisoned")
        .clone()
        .ok_or_else(native_required)?;
    let before = crate::project_history::snapshot(state, project);
    let mut after = before.clone();
    let preview = adopt::apply(&mut after, shown.plan).map_err(refused)?;

    let stack = state.command_stack.lock().expect("state mutex poisoned");
    let mut generation = state
        .history_generation
        .lock()
        .expect("state mutex poisoned");
    // The way back is written before the change it undoes.
    let conn =
        history::open_editor_store(&path, false).map_err(|e| ApiError::internal(e.to_string()))?;
    let change = history::change_history(
        &conn,
        &before,
        *generation,
        history::HistoryAction::Create(ROLLBACK_LABEL),
    )
    .map_err(rollback_error)?;
    drop(conn);
    *generation = change.generation;
    let rollback_id = change
        .versions
        .iter()
        .filter(|v| v.reason == "named" && v.label == ROLLBACK_LABEL)
        .map(|v| v.id)
        .max();
    match knx_app::project_history::persist_working(&path, &after, &stack, change.generation) {
        Ok(next) => *generation = next,
        Err(error) => {
            // The version is durable; publish its new generation, nothing else.
            domain::next_project_revision(state);
            return Err(ApiError::internal(format!(
                "the plug-in data was not adopted ({error}); a version of the unchanged project was kept"
            )));
        }
    }
    drop(generation);
    // Durable admission precedes publication; the model and refs are unchanged.
    *state.opaque.lock().expect("state mutex poisoned") = after.opaque;
    let revision = domain::next_project_revision(state);
    let clean = state.clean_project.lock().expect("state mutex poisoned");
    let last_saved = state.last_saved_at.lock().expect("state mutex poisoned");
    let counts = *state.import_counts.lock().expect("state mutex poisoned");
    let tree = domain::tree_with_state(
        project,
        clean.as_ref(),
        &stack,
        counts,
        revision,
        state,
        last_saved.as_deref(),
    );
    Ok(json!({
        "preview": preview,
        "project": tree,
        "rollbackVersion": {"id": rollback_id, "label": ROLLBACK_LABEL},
    }))
}

pub(crate) async fn cancel(
    State(state): State<SharedState>,
    Json(body): Json<TokenRequest>,
) -> Result<Json<Value>, ApiError> {
    let mut slot = state.plugin_data_plan.lock().expect("state mutex poisoned");
    let cancelled = slot
        .as_ref()
        .is_some_and(|shown| shown.token == body.confirmation_token);
    if cancelled {
        *slot = None;
    }
    Ok(Json(json!({ "cancelled": cancelled })))
}
