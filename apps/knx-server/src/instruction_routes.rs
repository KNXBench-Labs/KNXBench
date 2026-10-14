//! Source-bound instruction routes without protocol operations or resource loading.
use crate::{domain, errors::ApiError, SharedState};
use axum::{
    extract::{DefaultBodyLimit, Path, State},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use knx_app::instruction_edit::{
    self as edit, InstructionEditError, InstructionEditPreview, InstructionNativeTarget,
    InstructionRevision,
};
use knx_app::instructions::InstructionOrigin;
use knx_core::{
    instruction::{InstructionDocument, InstructionRun, InstructionStyle},
    DeviceId,
};
use knx_store::project_history::NativeSnapshot;
use serde::{Deserialize, Serialize};
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::time::{Duration, Instant};

const PREVIEW_TTL: Duration = Duration::from_secs(300);
struct Pending {
    token: String,
    created: Instant,
    plan: InstructionEditPreview,
}
struct ApiState {
    app: SharedState,
    pending: Mutex<Option<Pending>>,
}
type Api = Arc<ApiState>;
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Style {
    bold: bool,
    italic: bool,
    underline: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Run {
    text: String,
    style: Style,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Document {
    runs: Vec<Run>,
}
impl From<&InstructionDocument> for Document {
    fn from(doc: &InstructionDocument) -> Self {
        Self {
            runs: doc
                .runs()
                .iter()
                .map(|run| Run {
                    text: run.text.clone(),
                    style: Style {
                        bold: run.style.bold,
                        italic: run.style.italic,
                        underline: run.style.underline,
                    },
                })
                .collect(),
        }
    }
}
impl Document {
    fn normalized(self) -> Result<InstructionDocument, ApiError> {
        InstructionDocument::new(
            self.runs
                .into_iter()
                .map(|run| InstructionRun {
                    text: run.text,
                    style: InstructionStyle {
                        bold: run.style.bold,
                        italic: run.style.italic,
                        underline: run.style.underline,
                    },
                })
                .collect(),
        )
        .map_err(|error| {
            ApiError::refused(
                "instructionInvalid",
                format!("Invalid normalized instruction document: {error:?}"),
            )
        })
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewRequest {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    document: Option<Document>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyRequest {
    token: String,
    confirmed: bool,
    confirm_conversion: bool,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CancelRequest {
    token: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    device: u32,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    /// `"imported"`, or `"native"` for a device made in KNXBench (Goal B4).
    origin: &'static str,
    /// The imported device XML's hash; `None` for a native device.
    original_xml_sha256: Option<String>,
    original_present: bool,
    original_bytes: usize,
    original_format: Option<&'static str>,
    unsupported: Vec<String>,
    preview_error: Option<String>,
    effective_document: Option<Document>,
    override_document: Option<Document>,
    persistence: &'static str,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewView {
    token: String,
    device: u32,
    replacement: Option<Document>,
    conversion_required: bool,
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
        "instructionStale",
        "Instruction preview no longer matches the open project; refresh and preview again",
    )
}
fn error(error: InstructionEditError) -> ApiError {
    match error {
        InstructionEditError::Conflict => conflict(),
        InstructionEditError::ConversionConsentRequired => ApiError::refused(
            "instructionConversionConsent",
            "Explicit conversion consent is required; the original will remain preserved",
        ),
        InstructionEditError::Source(source) => ApiError::refused(
            "instructionSourceUnavailable",
            format!("Original instruction source unavailable: {source:?}"),
        ),
        InstructionEditError::Persistence(_) => ApiError::refused(
            "instructionPersistenceFailed",
            "Instruction edit was not applied because native recovery persistence failed",
        ),
        _ => ApiError::refused("instructionInvalid", "Instruction edit refused"),
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
fn revision(state: &SharedState) -> InstructionRevision {
    InstructionRevision {
        project_incarnation: format!(
            "{}:{}",
            state.server_incarnation,
            state.project_incarnation.load(Ordering::Relaxed)
        ),
        generation: domain::current_project_revision(state),
    }
}
async fn read(State(api): State<Api>, Path(id): Path<u32>) -> Result<Json<View>, ApiError> {
    let state = &api.app;
    let lock = state.project.lock().expect("state mutex poisoned");
    let project = lock
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("No project open"))?;
    let origin = knx_app::instructions::instruction_origin(
        project,
        &state.opaque.lock().expect("state mutex poisoned"),
        DeviceId(id),
    )
    .map_err(|source| error(InstructionEditError::Source(source)))?;
    let original = match &origin {
        InstructionOrigin::Imported(original) => Some(original),
        InstructionOrigin::Native => None,
    };
    let hints = original.and_then(|original| original.hints.as_ref());
    let mut unsupported = vec![];
    let mut preview_error = None;
    let mut original_format = None;
    let mut effective_document = None;
    if let Some(hints) = hints {
        match &hints.preview {
            Ok(view) => {
                original_format = Some(match view.format {
                    knx_etsproj::rtf::TextFormat::Plain => "plain",
                    knx_etsproj::rtf::TextFormat::Rtf => "rtf",
                });
                unsupported = view.unsupported.clone();
                effective_document = Some(Document {
                    runs: view
                        .runs
                        .iter()
                        .map(|run| Run {
                            text: run.text.clone(),
                            style: Style {
                                bold: run.style.bold,
                                italic: run.style.italic,
                                underline: run.style.underline,
                            },
                        })
                        .collect(),
                });
            }
            Err(reason) => preview_error = Some(format!("{reason:?}")),
        }
    }
    let override_document = project.instructions.get(&DeviceId(id)).map(Document::from);
    if override_document.is_some() {
        effective_document = override_document.clone();
    }
    Ok(Json(View {
        device: id,
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
        snapshot_revision: domain::current_project_revision(state),
        origin: if original.is_some() {
            "imported"
        } else {
            "native"
        },
        original_xml_sha256: original.map(|original| original.xml_sha256.clone()),
        original_present: hints.is_some(),
        original_bytes: hints.map_or(0, |h| h.original.len()),
        original_format,
        unsupported,
        preview_error,
        effective_document,
        override_document,
        persistence: if state
            .store_path
            .lock()
            .expect("state mutex poisoned")
            .is_some()
        {
            "native-working-history"
        } else {
            "session-only"
        },
    }))
}
async fn preview(
    State(api): State<Api>,
    Path(id): Path<u32>,
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
    let replacement = request.document.map(Document::normalized).transpose()?;
    let plan = edit::preview_instruction_edit(
        &snapshot(state, project),
        &revision(state),
        DeviceId(id),
        replacement,
    )
    .map_err(error)?;
    let mut random = [0_u8; 24];
    getrandom::fill(&mut random)
        .map_err(|_| ApiError::internal("Unable to allocate instruction preview identity"))?;
    let token = URL_SAFE_NO_PAD.encode(random);
    let response = PreviewView {
        token: token.clone(),
        device: id,
        replacement: plan.replacement().map(Document::from),
        conversion_required: plan.conversion_required(),
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
    Path(id): Path<u32>,
    Json(request): Json<ApplyRequest>,
) -> Result<Json<ApplyView>, ApiError> {
    if !request.confirmed {
        return Err(ApiError::refused(
            "instructionConfirmationRequired",
            "Confirm the displayed instruction change before applying",
        ));
    }
    let state = &api.app;
    let mut lock = state.project.lock().expect("state mutex poisoned");
    let project = lock
        .as_mut()
        .ok_or_else(|| ApiError::bad_request("No project open"))?;
    let mut pending = api.pending.lock().expect("preview mutex poisoned");
    let plan = pending.as_ref().ok_or_else(conflict)?;
    if plan.token != request.token
        || plan.plan.device() != DeviceId(id)
        || plan.created.elapsed() >= PREVIEW_TTL
    {
        return Err(conflict());
    }
    let mut candidate = snapshot(state, project);
    let mut stack = state.command_stack.lock().expect("state mutex poisoned");
    let path = state.store_path.lock().expect("state mutex poisoned");
    let mut generation = state
        .history_generation
        .lock()
        .expect("state mutex poisoned");
    let target = path.as_deref().map(|path| InstructionNativeTarget {
        path,
        history_generation: *generation,
    });
    let outcome = edit::apply_instruction_edit(
        &mut candidate,
        &mut stack,
        &revision(state),
        &plan.plan,
        request.confirm_conversion,
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
        persistence: if path.is_some() {
            "native-working-history"
        } else {
            "session-only"
        },
    }))
}
async fn cancel(
    State(api): State<Api>,
    Path(id): Path<u32>,
    Json(request): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let mut pending = api.pending.lock().expect("preview mutex poisoned");
    let plan = pending.as_ref().ok_or_else(conflict)?;
    if plan.token != request.token || plan.plan.device() != DeviceId(id) {
        return Err(conflict());
    }
    *pending = None;
    Ok(Json(serde_json::json!({"cancelled":true})))
}
pub fn router(app: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/api/devices/{id}/instructions", get(read))
        .route("/api/devices/{id}/instructions/preview", post(preview))
        .route("/api/devices/{id}/instructions/apply", post(apply))
        .route("/api/devices/{id}/instructions/cancel", post(cancel))
        .layer(DefaultBodyLimit::max(1024 * 1024))
        .with_state(Arc::new(ApiState {
            app,
            pending: Mutex::new(None),
        }))
}
