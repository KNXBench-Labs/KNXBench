//! HTTP routes for missing-name discovery, reevaluation, preview, and atomic apply.
//!
//! Exposes read-only scan and Q16 parent-change reevaluation backed by read-only
//! product-DB `product_text` resolution, plus a bounded, revision-bound preview,
//! explicit confirmation, cancel, and atomic single-undo apply backed by
//! `knx_app::missing_name_apply`.

use crate::{domain, errors::ApiError, SharedState};
use axum::{
    extract::{DefaultBodyLimit, State},
    routing::post,
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use knx_app::missing_name_apply::{
    self as apply_svc, MissingNameApplyError, MissingNameApplyIntent, MissingNameApplyPlan,
    MissingNameApplyPreview, MissingNameEntry, MissingNameReevaluation, MAX_MISSING_NAME_SELECTION,
};
use knx_app::missing_name_recipes::{
    self as recipes, MissingNameEvidence, MissingNameProposal, TextAssociation,
};
use knx_app::missing_names::{
    MissingNameScanRequest, MissingNameScanResult, NameTargetId, DEFAULT_PAGE_SIZE,
    DEFAULT_SCAN_LIMIT,
};
use knx_app::name_recipe::{NameRecipeNativeTarget, NameRecipeRevision};
use knx_core::{DeviceId, Project};
use knx_store::project_history::NativeSnapshot;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{atomic::Ordering, Arc, Mutex};
use std::time::{Duration, Instant};

/// How long a shown preview may be confirmed before the user must preview again.
const PREVIEW_TTL: Duration = Duration::from_secs(300);
/// Bounded request body limit for scan, reevaluation, preview, apply, and cancel.
const BODY_LIMIT: usize = 256 * 1024;
/// Maximum accepted length of a BCP 47 language tag.
const LANGUAGE_BYTES: usize = 128;
/// Fixed server authority bound into the plan confirmation token.
const APPLY_AUTHORITY: &str = "knx-server-missing-names-v1";

struct Pending {
    token: String,
    confirmation_token: String,
    created: Instant,
    plan: MissingNameApplyPlan,
}

/// One pending preview per server: a newer preview replaces any earlier one.
struct ApiState {
    app: SharedState,
    pending: Mutex<Option<Pending>>,
}

type Api = Arc<ApiState>;

fn default_scan_limit() -> Option<usize> {
    Some(DEFAULT_SCAN_LIMIT)
}

fn default_page_size() -> usize {
    DEFAULT_PAGE_SIZE
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ScanRequest {
    #[serde(default = "default_scan_limit")]
    max_entities_scanned: Option<usize>,
    #[serde(default)]
    offset: usize,
    #[serde(default = "default_page_size")]
    limit: usize,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanView {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    persistence: &'static str,
    discovery: MissingNameScanResult,
    items: Vec<MissingNameProposal>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReevaluateRequest {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    #[serde(default)]
    changed_parents: Vec<NameTargetId>,
    #[serde(default)]
    draft_parents: Vec<MissingNameEntry>,
    #[serde(default)]
    language: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewRequest {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    entries: Vec<MissingNameEntry>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewView {
    token: String,
    expires_in_seconds: u64,
    persistence: &'static str,
    preview: MissingNameApplyPreview,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ApplyRequest {
    token: String,
    confirmed: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ApplyView {
    changed: bool,
    native_generation: Option<i64>,
    snapshot_revision: u64,
    persistence: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CancelRequest {
    token: String,
}

fn persistence_label(has_store_path: bool) -> &'static str {
    if has_store_path {
        "native-working-history"
    } else {
        "session-only"
    }
}

fn stale() -> ApiError {
    ApiError::conflict(
        "missingNameStale",
        "The project changed since this preview; scan or preview again",
    )
}

fn validate_language(language: Option<String>) -> Result<Option<String>, ApiError> {
    match language {
        None => Ok(None),
        Some(lang) => {
            if lang.is_empty()
                || lang.len() > LANGUAGE_BYTES
                || lang.parse::<language_tags::LanguageTag>().is_err()
            {
                return Err(ApiError::bad_request("Invalid or oversized language tag"));
            }
            Ok(Some(lang))
        }
    }
}

fn check_binding(
    state: &SharedState,
    server_incarnation: &str,
    project_incarnation: u64,
    snapshot_revision: u64,
) -> Result<(), ApiError> {
    if server_incarnation != state.server_incarnation
        || project_incarnation != state.project_incarnation.load(Ordering::Relaxed)
        || snapshot_revision != domain::current_project_revision(state)
    {
        return Err(stale());
    }
    Ok(())
}

fn map_error(error: MissingNameApplyError) -> ApiError {
    match error {
        MissingNameApplyError::Conflict
        | MissingNameApplyError::InvalidRevision
        | MissingNameApplyError::InvalidConfirmation
        | MissingNameApplyError::InvalidAuthority => stale(),
        MissingNameApplyError::InvalidSelection => ApiError::refused(
            "missingNameSelection",
            format!("Choose between 1 and {MAX_MISSING_NAME_SELECTION} blank targets"),
        ),
        MissingNameApplyError::Budget => ApiError::refused(
            "missingNameBudget",
            "The preview exceeds the maximum allowed payload size",
        ),
        MissingNameApplyError::DuplicateSelection(target) => ApiError::refused(
            "missingNameDuplicate",
            format!("Target {target:?} is selected more than once"),
        ),
        MissingNameApplyError::MissingTarget(target)
        | MissingNameApplyError::AmbiguousTarget(target) => ApiError::refused(
            "missingNameTarget",
            format!("Target {target:?} is not uniquely present in the open project"),
        ),
        MissingNameApplyError::ReservedTarget(target) => ApiError::refused(
            "missingNameReservedTarget",
            format!("Target {target:?} has a reserved identifier and cannot be renamed"),
        ),
        MissingNameApplyError::TargetNotBlank(target) => ApiError::refused(
            "missingNameTargetNotBlank",
            format!("Target {target:?} already has a non-empty name"),
        ),
        MissingNameApplyError::InvalidName(target, reason) => ApiError::refused(
            "missingNameInvalidName",
            format!("The new name for {target:?} is not allowed: {reason}"),
        ),
        MissingNameApplyError::InvalidFunctionName(target, reason) => ApiError::refused(
            "missingNameInvalidName",
            format!("The new name for {target:?} is not allowed: {reason}"),
        ),
        MissingNameApplyError::Command(reason) => {
            ApiError::refused("missingNameRefused", format!("Renaming refused: {reason}"))
        }
        MissingNameApplyError::Persistence(_) => ApiError::refused(
            "missingNamePersistenceFailed",
            "Nothing was renamed because the project file could not be updated",
        ),
    }
}

fn snapshot(state: &SharedState, project: &Project) -> NativeSnapshot {
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

/// Resolve read-only `product_text` evidence for project devices without holding `state.project`.
///
/// Uses only `row.product_text`, never catalog, hardware, application, or manufacturer names.
/// Known hardware conflicts remain unclear evidence, even without text, so description fallback
/// cannot hide the contradiction. An absent database or lookup still degrades gracefully.
fn resolve_product_evidence(
    state: &SharedState,
    project: &Project,
    language: Option<&str>,
) -> Vec<MissingNameEvidence> {
    let inputs: Vec<(DeviceId, String, String)> = project
        .devices
        .iter()
        .filter(|device| !device.product_ref.trim().is_empty())
        .map(|device| {
            (
                device.id,
                device.product_ref.clone(),
                device.program_ref.clone(),
            )
        })
        .collect();
    if inputs.is_empty() {
        return Vec::new();
    }
    let Some(owner) = state.product_db.as_ref() else {
        return Vec::new();
    };
    let resolved = owner.with_read(|conn| {
        let mut manufacturer_names = knx_productdb::query::ManufacturerNameCache::default();
        let mut cache: HashMap<(String, String), Option<(String, TextAssociation)>> =
            HashMap::new();
        let mut evidence = Vec::new();
        for (device_id, product_ref, program_ref) in inputs {
            let key = (product_ref.clone(), program_ref.clone());
            let product_text = cache
                .entry(key)
                .or_insert_with(|| {
                    match knx_productdb::query::device_product_cached(
                        conn,
                        &product_ref,
                        &program_ref,
                        language,
                        &mut manufacturer_names,
                    ) {
                        Ok(Some(row)) => {
                            if row.program_relation
                                == knx_productdb::query::DeviceProgramRelation::HardwareMismatch
                            {
                                // Keep explicit conflict evidence even when product text is absent.
                                Some((
                                    row.product_text.unwrap_or_default(),
                                    TextAssociation::Unclear,
                                ))
                            } else {
                                // A missing program link is not a proven hardware contradiction:
                                // the product row still establishes its own designation.
                                row.product_text
                                    .filter(|text| !text.trim().is_empty())
                                    .map(|text| (text, TextAssociation::Confirmed))
                            }
                        }
                        Ok(None) | Err(_) => None,
                    }
                })
                .clone();
            if let Some((product_text, association)) = product_text {
                evidence.push(MissingNameEvidence::ResolvedProduct {
                    device: device_id,
                    product_ref,
                    program_ref,
                    product_text,
                    association,
                });
            }
        }
        Ok::<_, ()>(evidence)
    });
    resolved.ok().and_then(Result::ok).unwrap_or_default()
}

async fn scan(
    State(api): State<Api>,
    Json(request): Json<ScanRequest>,
) -> Result<Json<ScanView>, ApiError> {
    crate::routes::off_runtime(move || {
        let language = validate_language(request.language)?;
        let state = &api.app;
        let (project, project_incarnation, snapshot_revision) = {
            let lock = state.project.lock().expect("state mutex poisoned");
            let project = lock
                .as_ref()
                .ok_or_else(|| ApiError::bad_request("No project open"))?
                .clone();
            (
                project,
                state.project_incarnation.load(Ordering::Relaxed),
                domain::current_project_revision(state),
            )
        };
        let persistence = persistence_label(state.store_path.known().is_some());
        let evidence = resolve_product_evidence(state, &project, language.as_deref());
        let scan_request = MissingNameScanRequest {
            max_entities_scanned: request.max_entities_scanned,
            offset: request.offset,
            limit: request.limit,
        };
        let proposed = recipes::propose_missing_names(&project, &evidence, scan_request);
        Ok(Json(ScanView {
            server_incarnation: state.server_incarnation.clone(),
            project_incarnation,
            snapshot_revision,
            persistence,
            discovery: proposed.discovery,
            items: proposed.items,
        }))
    })
    .await?
}

async fn reevaluate(
    State(api): State<Api>,
    Json(request): Json<ReevaluateRequest>,
) -> Result<Json<MissingNameReevaluation>, ApiError> {
    crate::routes::off_runtime(move || {
        let language = validate_language(request.language)?;
        let state = &api.app;
        let snap = {
            let lock = state.project.lock().expect("state mutex poisoned");
            let project = lock
                .as_ref()
                .ok_or_else(|| ApiError::bad_request("No project open"))?;
            check_binding(
                state,
                &request.server_incarnation,
                request.project_incarnation,
                request.snapshot_revision,
            )?;
            snapshot(state, project)
        };
        let evidence = resolve_product_evidence(state, &snap.project, language.as_deref());
        let result = apply_svc::reevaluate_affected_proposals(
            &snap,
            &evidence,
            &request.changed_parents,
            &request.draft_parents,
        )
        .map_err(map_error)?;
        Ok(Json(result))
    })
    .await?
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
        check_binding(
            state,
            &request.server_incarnation,
            request.project_incarnation,
            request.snapshot_revision,
        )?;
        let plan = apply_svc::preview_missing_name_apply(
            &snapshot(state, project),
            &revision(state),
            MissingNameApplyIntent {
                entries: request.entries,
            },
        )
        .map_err(map_error)?;
        let confirmation_token = plan
            .confirmation_token(APPLY_AUTHORITY)
            .map_err(map_error)?;
        let mut random = [0_u8; 24];
        getrandom::fill(&mut random)
            .map_err(|_| ApiError::internal("Unable to allocate a preview identity"))?;
        let token = URL_SAFE_NO_PAD.encode(random);
        let response = PreviewView {
            token: token.clone(),
            expires_in_seconds: PREVIEW_TTL.as_secs(),
            persistence: persistence_label(state.store_path.known().is_some()),
            preview: plan.preview().clone(),
        };
        *api.pending.lock().expect("preview mutex poisoned") = Some(Pending {
            token,
            confirmation_token,
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
                "missingNameConfirmationRequired",
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
        let path = state.store_path.lock_unless_saving()?;
        let mut generation = state
            .history_generation
            .lock()
            .expect("state mutex poisoned");
        let target = path.as_deref().map(|path| NameRecipeNativeTarget {
            path,
            history_generation: *generation,
        });
        let outcome = apply_svc::apply_missing_name_apply(
            &mut candidate,
            &mut stack,
            &revision(state),
            &shown.plan,
            APPLY_AUTHORITY,
            &shown.confirmation_token,
            target,
        )
        .map_err(map_error)?;
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
            persistence: persistence_label(path.is_some()),
        }))
    })
    .await?
}

async fn cancel(
    State(api): State<Api>,
    Json(request): Json<CancelRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    crate::routes::off_runtime(move || {
        let mut pending = api.pending.lock().expect("preview mutex poisoned");
        match pending.as_ref() {
            Some(shown) if shown.token == request.token => *pending = None,
            // Someone else's preview, or none: nothing of ours to cancel.
            _ => return Ok(Json(serde_json::json!({ "cancelled": false }))),
        }
        Ok(Json(serde_json::json!({ "cancelled": true })))
    })
    .await?
}

pub fn router(app: SharedState) -> Router<SharedState> {
    Router::new()
        .route("/api/missing-names/scan", post(scan))
        .route("/api/missing-names/reevaluate", post(reevaluate))
        .route("/api/missing-names/preview", post(preview))
        .route("/api/missing-names/apply", post(apply))
        .route("/api/missing-names/cancel", post(cancel))
        .layer(DefaultBodyLimit::max(BODY_LIMIT))
        .with_state(Arc::new(ApiState {
            app,
            pending: Mutex::new(None),
        }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use knx_core::Project;

    #[tokio::test(flavor = "current_thread")]
    async fn mounted_cancel_waits_off_runtime_and_preserves_nonmatching_preview() {
        use axum::{body::Body, http::Request};
        use std::sync::mpsc;
        use tower::ServiceExt;

        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState::new(dir.path().to_path_buf()));
        let mut project = Project::new(knx_core::Language("en".into()));
        project.info.name = String::new();
        *state.project.lock().unwrap() = Some(project.clone());
        let api = Arc::new(ApiState {
            app: state.clone(),
            pending: Mutex::new(None),
        });
        let shown = preview(
            State(api.clone()),
            Json(PreviewRequest {
                server_incarnation: state.server_incarnation.clone(),
                project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
                snapshot_revision: domain::current_project_revision(&state),
                entries: vec![MissingNameEntry {
                    target: NameTargetId::Project,
                    name: "Renamed Project".into(),
                }],
            }),
        )
        .await
        .unwrap()
        .0;
        let app = Router::new()
            .route("/api/missing-names/cancel", post(cancel))
            .with_state(api.clone());

        for (token, expected_cancelled) in [("foreign-token", false), (shown.token.as_str(), true)]
        {
            let (held_tx, held_rx) = mpsc::sync_channel(1);
            let (release_tx, release_rx) = mpsc::sync_channel(1);
            let holder_api = api.clone();
            let holder = std::thread::spawn(move || {
                let guard = holder_api.pending.lock().unwrap();
                held_tx.send(()).unwrap();
                // An inline-lock regression cannot deadlock the test: this independent
                // thread releases on timeout, recording that the runtime made no progress.
                let released_by_runtime = release_rx.recv_timeout(Duration::from_secs(2)).is_ok();
                drop(guard);
                released_by_runtime
            });
            held_rx.recv().unwrap();
            let request = Request::post("/api/missing-names/cancel")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({"token": token}).to_string()))
                .unwrap();
            let request_app = app.clone();
            let task = tokio::spawn(async move { request_app.oneshot(request).await });
            // The mounted request must actually have been polled before the progress probe.
            tokio::task::yield_now().await;
            let waiting_before_timer = !task.is_finished();
            tokio::time::sleep(Duration::from_millis(20)).await;
            let waiting_after_timer = !task.is_finished();
            let release_sent = release_tx.send(()).is_ok();
            // Release and join before every assertion, including task/result assertions.
            let holder_result = holder.join();
            let response_result = task.await;

            assert!(
                release_sent,
                "runtime could not release the independently held slot"
            );
            assert!(
                holder_result.unwrap(),
                "cancel blocked the current-thread runtime"
            );
            assert!(waiting_before_timer && waiting_after_timer);
            let response = response_result.unwrap().unwrap();
            assert_eq!(response.status(), axum::http::StatusCode::OK);
            let body = axum::body::to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap();
            assert_eq!(
                serde_json::from_slice::<serde_json::Value>(&body).unwrap(),
                serde_json::json!({"cancelled": expected_cancelled})
            );
            let pending = api.pending.lock().unwrap();
            if expected_cancelled {
                assert!(pending.is_none());
            } else {
                assert_eq!(pending.as_ref().unwrap().token, shown.token);
            }
        }
        assert_eq!(state.project.lock().unwrap().as_ref(), Some(&project));
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            (0, 0)
        );
    }

    #[tokio::test]
    async fn an_expired_matching_preview_is_refused_without_mutating_project() {
        let dir = tempfile::tempdir().unwrap();
        let state = Arc::new(crate::AppState::new(dir.path().to_path_buf()));
        let mut project = Project::new(knx_core::Language("en".into()));
        project.info.name = String::new();
        *state.project.lock().unwrap() = Some(project.clone());
        let snap = snapshot(&state, &project);
        let plan = apply_svc::preview_missing_name_apply(
            &snap,
            &revision(&state),
            MissingNameApplyIntent {
                entries: vec![MissingNameEntry {
                    target: NameTargetId::Project,
                    name: "Renamed Project".into(),
                }],
            },
        )
        .unwrap();
        let confirmation_token = plan.confirmation_token(APPLY_AUTHORITY).unwrap();
        let api = Arc::new(ApiState {
            app: state.clone(),
            pending: Mutex::new(Some(Pending {
                token: "expired-token".into(),
                confirmation_token,
                created: Instant::now() - PREVIEW_TTL,
                plan,
            })),
        });

        let result = apply(
            State(api),
            Json(ApplyRequest {
                token: "expired-token".into(),
                confirmed: true,
            }),
        )
        .await;
        let err = match result {
            Ok(_) => panic!("expired matching preview must be refused"),
            Err(err) => err,
        };
        assert_eq!(err.kind(), Some("missingNameStale"));
        assert_eq!(
            state.project.lock().unwrap().as_ref().unwrap().info.name,
            ""
        );
        assert_eq!(
            state.command_stack.lock().unwrap().history_lengths(),
            (0, 0)
        );
    }
}
