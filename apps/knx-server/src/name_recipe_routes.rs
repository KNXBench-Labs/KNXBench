//! HTTP routes for the bounded literal group-address naming recipe (Goal B7).
//!
//! Explicit group address ids, a literal prefix/suffix, a binding preview, an
//! explicit confirmation, one undo step. Everything that decides is the existing
//! `knx_app::name_recipe` service (FUTURE-02, see
//! `docs/contracts/offline-name-recipe.md`); these routes only bind it to
//! the live session the way `instruction_routes` binds the instruction
//! service. No selection is generated here, nothing is interpolated, and
//! nothing reaches the bus.
use crate::{domain, errors::ApiError, SharedState};
use axum::{
    extract::{DefaultBodyLimit, State},
    routing::post,
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use knx_app::name_recipe::{
    self as recipe, NameRecipeError, NameRecipeIntent, NameRecipeNativeTarget, NameRecipePlan,
    NameRecipeRevision,
};
use knx_core::GroupAddressId;
use knx_store::project_history::NativeSnapshot;
use serde::{Deserialize, Serialize};
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a shown preview may be confirmed. After that the user previews
/// again, which costs one click and rules out applying a forgotten dialog.
const PREVIEW_TTL: Duration = Duration::from_secs(300);
/// Ids, affixes and the service's 1 MiB preview budget fit well inside.
const BODY_LIMIT: usize = 64 * 1024;

struct Pending {
    token: String,
    created: Instant,
    plan: NameRecipePlan,
}
/// One pending preview per server: a newer preview, from any window,
/// replaces it, and the older dialog's confirmation then asks for a new
/// preview instead of applying something nobody is looking at.
struct ApiState {
    app: SharedState,
    pending: Mutex<Option<Pending>>,
}
type Api = Arc<ApiState>;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewRequest {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    /// Stable ids in the order the user chose them; never expanded.
    group_addresses: Vec<u32>,
    prefix: String,
    suffix: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyRequest {
    token: String,
    confirmed: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CancelRequest {
    token: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RowView {
    id: u32,
    before: String,
    after: String,
    changed: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewView {
    token: String,
    rows: Vec<RowView>,
    changed: usize,
    no_op: bool,
    expires_in_seconds: u64,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyView {
    changed: bool,
    native_generation: Option<i64>,
    snapshot_revision: u64,
    persistence: &'static str,
}

fn stale() -> ApiError {
    ApiError::conflict(
        "nameRecipeStale",
        "The project changed since this preview; preview again",
    )
}
fn error(error: NameRecipeError) -> ApiError {
    match error {
        NameRecipeError::Conflict | NameRecipeError::InvalidRevision => stale(),
        NameRecipeError::InvalidSelection => ApiError::refused(
            "nameRecipeSelection",
            format!(
                "Choose between 1 and {} group addresses",
                recipe::MAX_RECIPE_SELECTION
            ),
        ),
        NameRecipeError::Budget => ApiError::refused(
            "nameRecipeBudget",
            "The prefix, suffix or preview is longer than a recipe may be",
        ),
        NameRecipeError::DuplicateSelection(GroupAddressId(id)) => ApiError::refused(
            "nameRecipeDuplicate",
            format!("Group address {id} is selected twice"),
        ),
        NameRecipeError::MissingTarget(GroupAddressId(id))
        | NameRecipeError::AmbiguousTarget(GroupAddressId(id)) => ApiError::refused(
            "nameRecipeTarget",
            format!("Group address {id} is not uniquely in the open project"),
        ),
        NameRecipeError::InvalidName(GroupAddressId(id), reason) => ApiError::refused(
            "nameRecipeInvalidName",
            format!("The new name of group address {id} is not allowed: {reason}"),
        ),
        NameRecipeError::Command(reason) => {
            ApiError::refused("nameRecipeRefused", format!("Renaming refused: {reason}"))
        }
        NameRecipeError::Persistence(_) => ApiError::refused(
            "nameRecipePersistenceFailed",
            "Nothing was renamed because the project file could not be updated",
        ),
    }
}
fn snapshot(state: &SharedState, project: &knx_core::Project) -> NativeSnapshot {
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
fn revision(state: &SharedState) -> NameRecipeRevision {
    NameRecipeRevision {
        project_incarnation: format!(
            "{}:{}",
            state.server_incarnation,
            state.project_incarnation.load(Ordering::Relaxed)
        ),
        generation: domain::current_project_revision(state),
    }
}

async fn preview(
    State(api): State<Api>,
    Json(request): Json<PreviewRequest>,
) -> Result<Json<PreviewView>, ApiError> {
    let state = &api.app;
    let lock = state.project.lock().expect("state mutex poisoned");
    let project = lock
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("No project open"))?;
    if request.server_incarnation != state.server_incarnation
        || request.project_incarnation != state.project_incarnation.load(Ordering::Relaxed)
        || request.snapshot_revision != domain::current_project_revision(state)
    {
        return Err(stale());
    }
    let plan = recipe::preview_name_recipe(
        &snapshot(state, project),
        &revision(state),
        NameRecipeIntent {
            ids: request
                .group_addresses
                .into_iter()
                .map(GroupAddressId)
                .collect(),
            prefix: request.prefix,
            suffix: request.suffix,
        },
    )
    .map_err(error)?;
    let mut random = [0_u8; 24];
    getrandom::fill(&mut random)
        .map_err(|_| ApiError::internal("Unable to allocate a preview identity"))?;
    let token = URL_SAFE_NO_PAD.encode(random);
    let shown = plan.preview();
    let response = PreviewView {
        token: token.clone(),
        rows: shown
            .rows
            .iter()
            .map(|row| RowView {
                id: row.id.0,
                before: row.before.clone(),
                after: row.after.clone(),
                changed: row.changed,
            })
            .collect(),
        changed: shown.changed,
        no_op: shown.changed == 0,
        expires_in_seconds: PREVIEW_TTL.as_secs(),
    };
    *api.pending.lock().expect("preview mutex poisoned") = Some(Pending {
        token,
        created: Instant::now(),
        plan,
    });
    Ok(Json(response))
}

async fn apply(
    State(api): State<Api>,
    Json(request): Json<ApplyRequest>,
) -> Result<Json<ApplyView>, ApiError> {
    if !request.confirmed {
        return Err(ApiError::refused(
            "nameRecipeConfirmationRequired",
            "Confirm the shown preview before renaming",
        ));
    }
    let state = &api.app;
    let mut lock = state.project.lock().expect("state mutex poisoned");
    let project = lock
        .as_mut()
        .ok_or_else(|| ApiError::bad_request("No project open"))?;
    let mut pending = api.pending.lock().expect("preview mutex poisoned");
    let shown = pending.as_ref().ok_or_else(stale)?;
    if shown.token != request.token || shown.created.elapsed() >= PREVIEW_TTL {
        return Err(stale());
    }
    let mut candidate = snapshot(state, project);
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    let path = state.store_path.lock().expect("state mutex poisoned");
    let mut generation = state
        .history_generation
        .lock()
        .expect("state mutex poisoned");
    let target = path.as_deref().map(|path| NameRecipeNativeTarget {
        path,
        history_generation: *generation,
    });
    let outcome = recipe::apply_name_recipe(
        &mut candidate,
        &mut stack,
        &revision(state),
        &shown.plan,
        target,
    )
    .map_err(error)?;
    if outcome.changed {
        *project = candidate.project;
        if let Some(next) = outcome.native_generation {
            *generation = next;
        }
        domain::next_project_revision(state);
    }
    // A preview is confirmed once; a replay previews again.
    *pending = None;
    Ok(Json(ApplyView {
        changed: outcome.changed,
        native_generation: outcome.native_generation,
        snapshot_revision: domain::current_project_revision(state),
        persistence: if path.is_some() {
            "native-working-history"
        } else {
            "session-only"
        },
    }))
}

async fn cancel(
    State(api): State<Api>,
    Json(request): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut pending = api.pending.lock().expect("preview mutex poisoned");
    match pending.as_ref() {
        Some(shown) if shown.token == request.token => *pending = None,
        // Someone else's preview, or none: nothing of ours to cancel.
        _ => return Ok(Json(serde_json::json!({ "cancelled": false }))),
    }
    Ok(Json(serde_json::json!({ "cancelled": true })))
}

pub fn router(app: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/api/name-recipe/preview", post(preview))
        .route("/api/name-recipe/apply", post(apply))
        .route("/api/name-recipe/cancel", post(cancel))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(Arc::new(ApiState {
            app,
            pending: Mutex::new(None),
        }))
}
