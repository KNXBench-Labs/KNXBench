//! Transient, browser-owned opaque evidence and selection-only project edits.
use super::DecodedDto;
use crate::{errors::ApiError, SharedState};
use axum::{
    extract::{Extension, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    Json,
};
use knx_app::comparison_adoption::{
    self as service, AdoptionError, ComparisonScope, ObservedComparison, PreparedAdoption,
};
use knx_store::project_history::NativeSnapshot;
use serde::{Deserialize, Serialize};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const COOKIE: &str = "knx_comparison_session";
const OBSERVATION_TTL: Duration = Duration::from_secs(600);
const PREVIEW_TTL: Duration = Duration::from_secs(120);
const MAX_OBSERVATIONS: usize = 32;
const MAX_PREVIEWS: usize = 16;
pub(super) type Sessions = Arc<Mutex<Retained>>;

struct Entry<T> {
    token: String,
    owner: String,
    created: Instant,
    scope: ComparisonScope,
    value: T,
}
#[derive(Default)]
pub(super) struct Retained {
    observations: VecDeque<Entry<ObservedComparison>>,
    previews: VecDeque<Entry<PreparedAdoption>>,
}
impl Retained {
    fn prune(&mut self, scope: &ComparisonScope) {
        let now = Instant::now();
        self.observations
            .retain(|e| e.scope == *scope && now.duration_since(e.created) < OBSERVATION_TTL);
        self.previews
            .retain(|e| e.scope == *scope && now.duration_since(e.created) < PREVIEW_TTL);
    }
    pub(super) fn observe(
        &mut self,
        owner: String,
        value: ObservedComparison,
    ) -> Result<String, ApiError> {
        let scope = value.preparation().scope().clone();
        self.prune(&scope);
        let token = entropy()?;
        while self.observations.len() >= MAX_OBSERVATIONS {
            self.observations.pop_front();
        }
        self.observations.push_back(Entry {
            token: token.clone(),
            owner,
            created: Instant::now(),
            scope,
            value,
        });
        Ok(token)
    }
}
pub(super) fn scope(state: &SharedState) -> ComparisonScope {
    ComparisonScope {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state
            .project_incarnation
            .load(std::sync::atomic::Ordering::Relaxed),
    }
}
fn entropy() -> Result<String, ApiError> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes)
        .map_err(|_| ApiError::internal("Comparison entropy unavailable"))?;
    Ok(bytes.iter().map(|b| format!("{b:02x}")).collect())
}
fn owner(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(';'))
        .find_map(|part| {
            let (key, value) = part.trim().split_once('=')?;
            (key == COOKIE && value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit()))
                .then(|| value.to_owned())
        })
}
pub(super) fn browser(headers: &HeaderMap) -> Result<(String, Option<HeaderValue>), ApiError> {
    if let Some(owner) = owner(headers) {
        return Ok((owner, None));
    }
    let owner = entropy()?;
    let cookie = HeaderValue::from_str(&format!(
        "{COOKIE}={owner}; Path=/api/device-compare; HttpOnly; SameSite=Strict"
    ))
    .map_err(|_| ApiError::internal("Comparison cookie unavailable"))?;
    Ok((owner, Some(cookie)))
}
fn missing() -> ApiError {
    ApiError::conflict("deviceComparisonExpired", "Comparison/preview expired, unavailable or belongs to another browser. Compare and preview again; a missing response does not prove rollback.")
}
pub(super) fn error(error: AdoptionError) -> ApiError {
    match error {
        AdoptionError::Compare(e) => super::compare_error(e),
        AdoptionError::Stale | AdoptionError::SourceChanged | AdoptionError::InvalidScope => {
            ApiError::conflict(
                "deviceCompareStale",
                "Project, history or source changed. Compare and preview again.",
            )
        }
        AdoptionError::Native(_) => ApiError::conflict(
            "deviceAdoptionPersistenceFailed",
            "Native history preflight/persistence refused. No project edit was published.",
        ),
        e => ApiError::with_kind(
            StatusCode::UNPROCESSABLE_ENTITY,
            "deviceAdoptionRefused",
            e.to_string(),
        ),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct PreviewRequest {
    observation_token: String,
    selected: Vec<usize>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PreviewResponse {
    preview_token: String,
    selected: Vec<usize>,
    changed: bool,
    rows: Vec<DecodedDto>,
    expires_in_seconds: u64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ApplyRequest {
    preview_token: String,
    confirmation: Confirmation,
}
#[derive(Deserialize)]
pub(super) enum Confirmation {
    AdoptDeviceComparison,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ApplyResponse {
    changed: bool,
    tree: knx_projection::ProjectTree,
}

pub(super) async fn preview(
    State(state): State<SharedState>,
    Extension(sessions): Extension<Sessions>,
    headers: HeaderMap,
    Json(body): Json<PreviewRequest>,
) -> Result<Json<PreviewResponse>, ApiError> {
    state.stop_admission.admit()?;
    let owner = owner(&headers).ok_or_else(missing)?;
    // All editors/replacement/save serialize on project first. No lock crosses await.
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_ref().ok_or_else(missing)?;
    let stack = state.command_stack.lock().expect("state mutex poisoned");
    let scope = scope(&state);
    let snapshot = NativeSnapshot {
        project: project.clone(),
        opaque: state.opaque.lock().expect("state mutex poisoned").clone(),
        manufacturer_refs: state
            .manufacturer_refs
            .lock()
            .expect("state mutex poisoned")
            .clone(),
    };
    let mut sessions = sessions.lock().expect("comparison mutex poisoned");
    sessions.prune(&scope);
    let observed = &sessions
        .observations
        .iter()
        .find(|e| e.token == body.observation_token && e.owner == owner)
        .ok_or_else(missing)?
        .value;
    let products = state.product_db.as_ref().ok_or_else(|| {
        ApiError::conflict("downloadProductDataMissing", "Product data unavailable")
    })?;
    let preview = products
        .with_read(|conn| {
            service::preview_adoption(conn, &snapshot, &stack, &scope, observed, &body.selected)
        })
        .map_err(crate::catalogue_owner::owner_error)?
        .map_err(error)?;
    let token = entropy()?;
    let response = PreviewResponse {
        preview_token: token.clone(),
        selected: preview.selected().to_vec(),
        changed: preview.changed(),
        rows: super::decoded(preview.rows()),
        expires_in_seconds: PREVIEW_TTL.as_secs(),
    };
    while sessions.previews.len() >= MAX_PREVIEWS {
        sessions.previews.pop_front();
    }
    sessions.previews.push_back(Entry {
        token,
        owner,
        created: Instant::now(),
        scope,
        value: preview,
    });
    Ok(Json(response))
}

pub(super) async fn apply(
    State(state): State<SharedState>,
    Extension(sessions): Extension<Sessions>,
    headers: HeaderMap,
    Json(body): Json<ApplyRequest>,
) -> Result<Json<ApplyResponse>, ApiError> {
    state.stop_admission.admit()?;
    let Confirmation::AdoptDeviceComparison = body.confirmation;
    let owner = owner(&headers).ok_or_else(missing)?;
    let mut project = state.project.lock().expect("state mutex poisoned");
    let project = project.as_mut().ok_or_else(missing)?;
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    let counts = *state.import_counts.lock().expect("state mutex poisoned");
    let path = state.store_path.lock().expect("state mutex poisoned");
    let clean = state.clean_project.lock().expect("state mutex poisoned");
    let saved = state.last_saved_at.lock().expect("state mutex poisoned");
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    let refs = state
        .manufacturer_refs
        .lock()
        .expect("state mutex poisoned");
    let mut generation = state
        .history_generation
        .lock()
        .expect("state mutex poisoned");
    let scope = scope(&state);
    let mut sessions = sessions.lock().expect("comparison mutex poisoned");
    // Expiry checked at apply admission, not by an eventual background timer.
    sessions.prune(&scope);
    let preview = &sessions
        .previews
        .iter()
        .find(|e| e.token == body.preview_token && e.owner == owner)
        .ok_or_else(missing)?
        .value;
    let mut snapshot = NativeSnapshot {
        project: project.clone(),
        opaque: opaque.clone(),
        manufacturer_refs: refs.clone(),
    };
    let products = state.product_db.as_ref().ok_or_else(|| {
        ApiError::conflict("downloadProductDataMissing", "Product data unavailable")
    })?;
    let native = path.as_deref().map(|path| service::AdoptionNativeTarget {
        path,
        history_generation: *generation,
    });
    let outcome = products
        .with_read(|conn| {
            service::apply_adoption(conn, &mut snapshot, &mut stack, &scope, preview, native)
        })
        .map_err(crate::catalogue_owner::owner_error)?
        .map_err(error)?;
    if outcome.changed {
        // Service persisted the candidate/history transaction before this publication.
        *project = snapshot.project;
        if let Some(next) = outcome.native_generation {
            *generation = next;
        }
        crate::domain::next_project_revision(&state);
        state.session_log.lock().expect("state mutex poisoned").push(crate::session_log::LogEntry {
            timestamp: crate::session_log::now(), severity: crate::session_log::Severity::Info,
            source: "AdoptDeviceComparison".into(), message: "Adopted observed named parameters into the project (one undoable edit; no device write)".into(), location: None, detail: None, diagnostic: None,
        });
    }
    // Consume only after success. A failed native write leaves preview retryable.
    sessions.previews.retain(|e| e.token != body.preview_token);
    if outcome.changed {
        sessions.observations.clear();
        sessions.previews.clear();
    }
    let tree = crate::domain::tree_with_state(
        project,
        clean.as_ref(),
        &stack,
        counts,
        crate::domain::current_project_revision(&state),
        &state,
        saved.as_deref(),
    );
    Ok(Json(ApplyResponse {
        changed: outcome.changed,
        tree,
    }))
}
