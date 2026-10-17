//! Adapts private note previews to confirmed authenticated HTTP edits.

use crate::{domain, errors::ApiError, SharedState};
use axum::{
    extract::{DefaultBodyLimit, State},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use knx_app::note_edit::{
    self as edit, NoteEditError, NoteEditPreview, NoteNativeTarget, NoteRevision,
};
use knx_app::note_targets::note_target_catalogue;
use knx_core::{
    note::{ProjectNote, ProjectNoteTarget},
    AreaId, BuildingPartId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId,
    InstallationId, LineId,
};
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
    plan: NoteEditPreview,
}
struct ApiState {
    app: SharedState,
    pending: Mutex<Option<Pending>>,
}
type Api = Arc<ApiState>;

#[derive(Clone, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    content = "id",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum Target {
    Project,
    Installation(u8),
    Area(u32),
    Line(u32),
    BuildingPart(u32),
    Device(u32),
    GroupRange(u32),
    GroupAddress(u32),
    ComObject(u32),
}
impl Target {
    pub(crate) fn normalized(&self) -> ProjectNoteTarget {
        match *self {
            Self::Project => ProjectNoteTarget::Project,
            Self::Installation(id) => ProjectNoteTarget::Installation(InstallationId(id)),
            Self::Area(id) => ProjectNoteTarget::Area(AreaId(id)),
            Self::Line(id) => ProjectNoteTarget::Line(LineId(id)),
            Self::BuildingPart(id) => ProjectNoteTarget::BuildingPart(BuildingPartId(id)),
            Self::Device(id) => ProjectNoteTarget::Device(DeviceId(id)),
            Self::GroupRange(id) => ProjectNoteTarget::GroupRange(GroupRangeId(id)),
            Self::GroupAddress(id) => ProjectNoteTarget::GroupAddress(GroupAddressId(id)),
            Self::ComObject(id) => ProjectNoteTarget::ComObject(ComObjectInstanceId(id)),
        }
    }
}
impl From<ProjectNoteTarget> for Target {
    fn from(value: ProjectNoteTarget) -> Self {
        match value {
            ProjectNoteTarget::Project => Self::Project,
            ProjectNoteTarget::Installation(id) => Self::Installation(id.0),
            ProjectNoteTarget::Area(id) => Self::Area(id.0),
            ProjectNoteTarget::Line(id) => Self::Line(id.0),
            ProjectNoteTarget::BuildingPart(id) => Self::BuildingPart(id.0),
            ProjectNoteTarget::Device(id) => Self::Device(id.0),
            ProjectNoteTarget::GroupRange(id) => Self::GroupRange(id.0),
            ProjectNoteTarget::GroupAddress(id) => Self::GroupAddress(id.0),
            ProjectNoteTarget::ComObject(id) => Self::ComObject(id.0),
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Draft {
    target: Target,
    title: String,
    body: String,
    include_in_documentation: bool,
}
impl Draft {
    fn normalized(&self) -> edit::NoteDraft {
        edit::NoteDraft {
            target: self.target.normalized(),
            title: self.title.clone(),
            body: self.body.clone(),
            include_in_documentation: self.include_in_documentation,
        }
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Action {
    Create { draft: Draft, position: usize },
    Replace { id: u32, draft: Draft },
    Remove { id: u32 },
    Move { id: u32, position: usize },
}
impl Action {
    fn normalized(&self) -> edit::NoteAction {
        match self {
            Self::Create { draft, position } => edit::NoteAction::Create {
                draft: draft.normalized(),
                position: *position,
            },
            Self::Replace { id, draft } => edit::NoteAction::Replace {
                id: knx_core::ProjectNoteId(*id),
                draft: draft.normalized(),
            },
            Self::Remove { id } => edit::NoteAction::Remove {
                id: knx_core::ProjectNoteId(*id),
            },
            Self::Move { id, position } => edit::NoteAction::Move {
                id: knx_core::ProjectNoteId(*id),
                position: *position,
            },
        }
    }
    fn previous(&self, project: &knx_core::Project) -> Option<NoteView> {
        let id = match self {
            Self::Create { .. } => return None,
            Self::Replace { id, .. } | Self::Remove { id } | Self::Move { id, .. } => *id,
        };
        project
            .notes
            .iter()
            .enumerate()
            .find(|(_, note)| note.id().0 == id)
            .map(|(position, note)| NoteView::from_note(note, position))
    }
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
struct NoteView {
    id: u32,
    target: Target,
    title: String,
    body: String,
    include_in_documentation: bool,
    position: usize,
}
impl NoteView {
    fn from_note(note: &ProjectNote, position: usize) -> Self {
        Self {
            id: note.id().0,
            target: note.target().into(),
            title: note.title().into(),
            body: note.body().into(),
            include_in_documentation: note.include_in_documentation(),
            position,
        }
    }
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    notes: Vec<NoteView>,
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
    previous: Option<NoteView>,
    created_id: Option<u32>,
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
        "noteStale",
        "Note preview no longer matches the open project; refresh and preview again",
    )
}
fn error(error: NoteEditError) -> ApiError {
    match error {
        NoteEditError::Conflict => conflict(),
        NoteEditError::Persistence(_) => ApiError::refused(
            "notePersistenceFailed",
            "Note edit was not applied because native recovery persistence failed",
        ),
        _ => ApiError::refused(
            "noteInvalid",
            "Note edit refused; check the typed target, text and position",
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
fn revision(state: &SharedState) -> NoteRevision {
    NoteRevision {
        project_incarnation: format!(
            "{}:{}",
            state.server_incarnation,
            state.project_incarnation.load(Ordering::Relaxed)
        ),
        generation: domain::current_project_revision(state),
    }
}
async fn read(State(api): State<Api>) -> Result<Json<View>, ApiError> {
    let state = &api.app;
    let lock = state.project.lock().expect("state mutex poisoned");
    let project = lock
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("No project open"))?;
    Ok(Json(View {
        notes: project
            .notes
            .iter()
            .enumerate()
            .map(|(i, n)| NoteView::from_note(n, i))
            .collect(),
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
        snapshot_revision: domain::current_project_revision(state),
        persistence: persistence(
            state
                .store_path
                .lock()
                .expect("state mutex poisoned")
                .is_some(),
        ),
    }))
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetOptionView {
    target: Target,
    owner: Vec<String>,
    parts: Vec<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TargetsView {
    targets: Vec<TargetOptionView>,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
}
/// The service-owned target catalogue (`knx_app::note_targets`), stamped with
/// the same snapshot identity as the notes read so a host can drop a stale one.
async fn targets(State(api): State<Api>) -> Result<Json<TargetsView>, ApiError> {
    let state = &api.app;
    let lock = state.project.lock().expect("state mutex poisoned");
    let project = lock
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("No project open"))?;
    Ok(Json(TargetsView {
        targets: note_target_catalogue(project)
            .into_iter()
            .map(|option| TargetOptionView {
                target: option.target.into(),
                owner: option.owner,
                parts: option.parts,
            })
            .collect(),
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
        snapshot_revision: domain::current_project_revision(state),
    }))
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
        return Err(conflict());
    }
    let plan = edit::preview_note_edit(
        &snapshot(state, project),
        &revision(state),
        request.action.normalized(),
    )
    .map_err(error)?;
    let mut random = [0_u8; 24];
    getrandom::fill(&mut random)
        .map_err(|_| ApiError::internal("Unable to allocate note preview identity"))?;
    let token = URL_SAFE_NO_PAD.encode(random);
    let response = PreviewView {
        token: token.clone(),
        previous: request.action.previous(project),
        action: request.action,
        created_id: plan.created_id().map(|id| id.0),
        no_op: plan.is_no_op(),
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
            "noteConfirmationRequired",
            "Confirm the displayed note change before applying",
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
    let target = path.as_deref().map(|path| NoteNativeTarget {
        path,
        history_generation: *generation,
    });
    let outcome = edit::apply_note_edit(
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
        .route("/api/project/notes", get(read))
        .route("/api/project/notes/targets", get(targets))
        .route("/api/project/notes/preview", post(preview))
        .route("/api/project/notes/apply", post(apply))
        .route("/api/project/notes/cancel", post(cancel))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .with_state(Arc::new(ApiState {
            app,
            pending: Mutex::new(None),
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{http::StatusCode, response::IntoResponse};
    use serde_json::json;

    #[test]
    fn every_typed_target_keeps_its_exact_wire_kind_and_domain_id() {
        for value in [
            json!({"kind":"project"}),
            json!({"kind":"installation","id":255}),
            json!({"kind":"area","id":1}),
            json!({"kind":"line","id":2}),
            json!({"kind":"buildingPart","id":3}),
            json!({"kind":"device","id":4}),
            json!({"kind":"groupRange","id":5}),
            json!({"kind":"groupAddress","id":6}),
            json!({"kind":"comObject","id":4294967295_u64}),
        ] {
            let parsed: Target = serde_json::from_value(value.clone()).unwrap();
            let roundtrip = Target::from(parsed.normalized());
            assert_eq!(serde_json::to_value(roundtrip).unwrap(), value);
        }
        assert!(serde_json::from_value::<Target>(json!({"kind":"installation","id":256})).is_err());
        assert!(
            serde_json::from_value::<Target>(json!({"kind":"project","unexpected":true})).is_err()
        );
    }

    #[tokio::test]
    async fn an_expired_matching_plan_refuses_before_any_live_mutation() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState::new(dir.path().to_path_buf()));
        *state.project.lock().unwrap() =
            Some(knx_core::Project::new(knx_core::Language("en".into())));
        let before = state.project.lock().unwrap().clone().unwrap();
        let plan = edit::preview_note_edit(
            &snapshot(&state, &before),
            &revision(&state),
            edit::NoteAction::Create {
                draft: edit::NoteDraft {
                    target: ProjectNoteTarget::Project,
                    title: "pending".into(),
                    body: "literal".into(),
                    include_in_documentation: false,
                },
                position: 0,
            },
        )
        .unwrap();
        let api = Arc::new(ApiState {
            app: state.clone(),
            pending: Mutex::new(Some(Pending {
                token: "synthetic-expired-plan".into(),
                created: Instant::now() - PREVIEW_TTL,
                plan,
            })),
        });
        match apply(
            State(api),
            Json(ApplyRequest {
                token: "synthetic-expired-plan".into(),
                confirmed: true,
            }),
        )
        .await
        {
            Err(error) => assert_eq!(error.into_response().status(), StatusCode::CONFLICT),
            Ok(_) => panic!("expired matching note preview must refuse"),
        }
        assert_eq!(state.project.lock().unwrap().as_ref(), Some(&before));
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            (0, 0)
        );
    }
}
