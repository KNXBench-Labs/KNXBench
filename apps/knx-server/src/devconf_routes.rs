//! Adapts device configuration profile previews to confirmed authenticated HTTP edits.

use crate::{domain, errors::ApiError, SharedState};
use axum::{
    extract::{DefaultBodyLimit, State},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use knx_app::devconf_edit::{self as edit, DevconfEditError, DevconfRevision};
use knx_core::DeviceId;
use knx_store::project_history::NativeSnapshot;
use serde::{Deserialize, Serialize};
use std::{
    sync::{atomic::Ordering, Arc, Mutex},
    time::{Duration, Instant},
};

const PREVIEW_TTL: Duration = Duration::from_secs(300);

struct Pending {
    token: String,
    created: Instant,
    plan: edit::DevconfEditPreview,
}

struct ApiState {
    app: SharedState,
    pending: Mutex<Option<Pending>>,
}
type Api = Arc<ApiState>;

#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ProfileDraft {
    device: u32,
    module_id: String,
    module_version: String,
    module_sha256: String,
    /// Base64url (no padding) encoded module document bytes.
    document_b64: String,
}
impl ProfileDraft {
    fn normalized(&self) -> Result<edit::DevconfDraft, ApiError> {
        let document = URL_SAFE_NO_PAD
            .decode(&self.document_b64)
            .map_err(|_| ApiError::refused("devconfInvalid", "Profile document is not valid base64url"))?;
        Ok(edit::DevconfDraft {
            device: DeviceId(self.device),
            module_id: self.module_id.clone(),
            module_version: self.module_version.clone(),
            module_sha256: self.module_sha256.clone(),
            document,
        })
    }
}

#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Action {
    Set { draft: ProfileDraft },
    Remove { device: u32 },
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewRequest {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    action: Action,
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
struct ProfileView {
    device: u32,
    module_id: String,
    module_version: String,
    module_sha256: String,
    document_bytes: usize,
}
impl From<&knx_core::devconf::DeviceConfigProfile> for ProfileView {
    fn from(profile: &knx_core::devconf::DeviceConfigProfile) -> Self {
        Self {
            device: profile.device().0,
            module_id: profile.module_id().into(),
            module_version: profile.module_version().into(),
            module_sha256: profile.module_sha256().into(),
            document_bytes: profile.document().len(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    profiles: Vec<ProfileView>,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    persistence: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewView {
    token: String,
    action: Action,
    previous: Option<ProfileView>,
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

fn conflict() -> ApiError {
    ApiError::conflict(
        "devconfStale",
        "Profile preview no longer matches the open project; refresh and preview again",
    )
}
fn error(error: DevconfEditError) -> ApiError {
    match error {
        DevconfEditError::Conflict => conflict(),
        DevconfEditError::InvalidRevision => conflict(),
        DevconfEditError::Persistence(_) => ApiError::refused(
            "devconfPersistenceFailed",
            "Profile edit was not applied because native recovery persistence failed",
        ),
        DevconfEditError::Profile(_) | DevconfEditError::Command(_) => ApiError::refused(
            "devconfInvalid",
            "Profile edit refused; check the device, module identity and document bytes",
        ),
    }
}
fn persistence(native: bool) -> &'static str {
    if native {
        "native-working-history"
    } else {
        "session-only"
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
fn revision(state: &SharedState) -> DevconfRevision {
    DevconfRevision {
        project_incarnation: format!(
            "{}:{}",
            state.server_incarnation,
            state.project_incarnation.load(Ordering::Relaxed)
        ),
        generation: domain::current_project_revision(state),
    }
}

fn read_blocking(state: &SharedState) -> Result<Json<View>, ApiError> {
    let lock = state.project.lock().expect("state mutex poisoned");
    let project = lock
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("No project open"))?;
    Ok(Json(View {
        profiles: project
            .devices
            .config_profiles()
            .map(ProfileView::from)
            .collect(),
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
        snapshot_revision: domain::current_project_revision(state),
        persistence: persistence(state.store_path.known().is_some()),
    }))
}

async fn read(State(api): State<Api>) -> Result<Json<View>, ApiError> {
    crate::routes::off_runtime(move || read_blocking(&api.app)).await?
}

async fn preview(
    State(api): State<Api>,
    Json(request): Json<PreviewRequest>,
) -> Result<Json<PreviewView>, ApiError> {
    crate::routes::off_runtime(move || {
        let state = &api.app;
        let lock = state.project.lock().expect("state mutex poisoned");
        let project = lock
            .as_ref()
            .ok_or_else(|| ApiError::bad_request("No project open"))?;
        if request.server_incarnation != state.server_incarnation
            || request.project_incarnation != state.project_incarnation.load(Ordering::Relaxed)
            || request.snapshot_revision != domain::current_project_revision(state)
        {
            return Err(conflict());
        }
        let plan = match &request.action {
            Action::Set { draft } => {
                edit::preview_devconf_edit(&snapshot(state, project), &revision(state), &draft.normalized()?)
            }
            Action::Remove { device } => edit::preview_devconf_removal(
                &snapshot(state, project),
                &revision(state),
                DeviceId(*device),
            ),
        }
        .map_err(error)?;
        let previous = match &request.action {
            Action::Set { draft } => project
                .devices
                .config_profile(DeviceId(draft.device))
                .map(ProfileView::from),
            Action::Remove { device } => project
                .devices
                .config_profile(DeviceId(*device))
                .map(ProfileView::from),
        };
        let mut random = [0_u8; 24];
        getrandom::fill(&mut random)
            .map_err(|_| ApiError::internal("Unable to allocate profile preview identity"))?;
        let token = URL_SAFE_NO_PAD.encode(random);
        let response = PreviewView {
            token: token.clone(),
            action: request.action,
            previous,
            no_op: plan.is_no_op(),
            expires_in_seconds: PREVIEW_TTL.as_secs(),
        };
        *api.pending.lock().expect("preview mutex poisoned") = Some(Pending {
            token,
            created: Instant::now(),
            plan,
        });
        Ok(Json(response))
    })
    .await?
}

async fn apply(
    State(api): State<Api>,
    Json(request): Json<ApplyRequest>,
) -> Result<Json<ApplyView>, ApiError> {
    crate::routes::off_runtime(move || {
        if !request.confirmed {
            return Err(ApiError::refused(
                "devconfConfirmationRequired",
                "Confirm the displayed profile change before applying",
            ));
        }
        let state = &api.app;
        let mut lock = state.project.lock().expect("state mutex poisoned");
        let project = lock
            .as_mut()
            .ok_or_else(|| ApiError::bad_request("No project open"))?;
        let mut pending = api.pending.lock().expect("preview mutex poisoned");
        let plan = pending.as_ref().ok_or_else(conflict)?;
        if plan.token != request.token || plan.created.elapsed() >= PREVIEW_TTL {
            return Err(conflict());
        }
        let mut candidate = snapshot(state, project);
        let mut stack = state.command_stack.lock().expect("state mutex poisoned");
        let path = state.store_path.lock().expect("state mutex poisoned");
        let mut generation = state
            .history_generation
            .lock()
            .expect("state mutex poisoned");
        let target = path.as_deref().map(|path| edit::DevconfNativeTarget {
            path,
            history_generation: *generation,
        });
        let outcome = edit::apply_devconf_edit(
            &mut candidate,
            &mut stack,
            &revision(state),
            &plan.plan,
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
        *pending = None;
        Ok(Json(ApplyView {
            changed: outcome.changed,
            native_generation: outcome.native_generation,
            snapshot_revision: domain::current_project_revision(state),
            persistence: persistence(path.is_some()),
        }))
    })
    .await?
}

async fn cancel(
    State(api): State<Api>,
    Json(request): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut pending = api.pending.lock().expect("preview mutex poisoned");
    let plan = pending.as_ref().ok_or_else(conflict)?;
    if plan.token != request.token {
        return Err(conflict());
    }
    *pending = None;
    Ok(Json(serde_json::json!({"cancelled":true})))
}

pub fn router(app: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/api/project/device-config-profiles", get(read))
        .route("/api/project/device-config-profiles/preview", post(preview))
        .route("/api/project/device-config-profiles/apply", post(apply))
        .route("/api/project/device-config-profiles/cancel", post(cancel))
        .layer(DefaultBodyLimit::max(2 * 1024 * 1024))
        .with_state(Arc::new(ApiState {
            app,
            pending: Mutex::new(None),
        }))
}
