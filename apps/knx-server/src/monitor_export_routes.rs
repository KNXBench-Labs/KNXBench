//! Snapshot-bound monitoring ZIP export without saved-file or bus access.

use crate::{domain::snapshot_is_current, errors::ApiError, SharedState};
use axum::{
    body::Body,
    extract::{rejection::JsonRejection, DefaultBodyLimit, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use knx_app::monitor_export::{self, ExportBundle, ExportOptions, ExportReport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const REQUEST_BYTES: usize = 4096;
const LANGUAGE_BYTES: usize = 128;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct PreviewRequest {
    server_incarnation: String,
    project_incarnation: u64,
    project_revision: u64,
    installation: Option<u8>,
    language: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct DownloadRequest {
    server_incarnation: String,
    project_incarnation: u64,
    project_revision: u64,
    installation: Option<u8>,
    language: Option<String>,
    bundle_sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Binding {
    server_incarnation: String,
    project_incarnation: u64,
    project_revision: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PreviewResponse {
    // Serialize the application report directly: its snake_case contract is intentional.
    report: ExportReport,
    binding: Binding,
    bundle_sha256: String,
}

pub(crate) fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/project/monitor-export/preview", post(preview))
        .route("/api/project/monitor-export/download", post(download))
        .layer(DefaultBodyLimit::max(REQUEST_BYTES))
}

fn json_error(error: JsonRejection) -> ApiError {
    ApiError::with_status(
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            StatusCode::PAYLOAD_TOO_LARGE
        } else {
            StatusCode::BAD_REQUEST
        },
        error.body_text(),
    )
}

impl PreviewRequest {
    fn validate(&self) -> Result<(), ApiError> {
        if self.server_incarnation.is_empty() || self.server_incarnation.len() > 128 {
            return Err(ApiError::bad_request("invalid server incarnation"));
        }
        if let Some(language) = &self.language {
            if language.is_empty()
                || language.len() > LANGUAGE_BYTES
                || language.parse::<language_tags::LanguageTag>().is_err()
            {
                return Err(ApiError::bad_request(
                    "invalid or oversized export language",
                ));
            }
        }
        Ok(())
    }

    fn binding(&self) -> Binding {
        Binding {
            server_incarnation: self.server_incarnation.clone(),
            project_incarnation: self.project_incarnation,
            project_revision: self.project_revision,
        }
    }
}

fn stale() -> ApiError {
    ApiError::with_status(
        StatusCode::CONFLICT,
        "reviewed monitoring export has changed; preview it again",
    )
}

fn build(state: &SharedState, request: &PreviewRequest) -> Result<ExportBundle, ApiError> {
    let options = ExportOptions {
        installation: request.installation.map(knx_core::InstallationId),
        language: request.language.clone(),
    };
    let project = {
        let guard = state
            .project
            .lock()
            .map_err(|_| ApiError::internal("project lock poisoned"))?;
        let project = guard
            .as_ref()
            .ok_or_else(|| ApiError::bad_request("no project open"))?;
        if !snapshot_is_current(
            state,
            &request.server_incarnation,
            request.project_incarnation,
            request.project_revision,
        ) {
            return Err(stale());
        }
        // Borrowed admission must precede the full snapshot clone. The App
        // helper owns its exact input-budget policy; no catalogue lock is held.
        monitor_export::validate_request(project, &options)
            .map_err(|message| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, message))?;
        project.clone()
    }; // Always release project before acquiring products; build cannot mutate live state.
    if state.product_db_unavailable.is_some() {
        return Err(ApiError::internal(
            "product database unavailable; monitoring export refused",
        ));
    }
    match state.product_db.as_ref() {
        None => monitor_export::build(&project, None, &options),
        Some(owner) => owner
            .with_read(|products| monitor_export::build(&project, Some(products), &options))
            .map_err(|_| ApiError::internal("product database lock poisoned"))?,
    }
    .map_err(|message| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, message))
}

fn private_response(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

async fn preview(
    State(state): State<SharedState>,
    request: Result<Json<PreviewRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(request) = request.map_err(json_error)?;
    // Request/language budgets must refuse before even acquiring/cloning the project.
    request.validate()?;
    tokio::task::spawn_blocking(move || {
        let bundle = build(&state, &request)?;
        let bundle_sha256 = format!("{:x}", Sha256::digest(&bundle.bytes));
        Ok(private_response(
            Json(PreviewResponse {
                report: bundle.report,
                binding: request.binding(),
                bundle_sha256,
            })
            .into_response(),
        ))
    })
    .await
    .map_err(|_| ApiError::internal("monitoring export worker failed"))?
}

fn validate_digest(digest: &str) -> Result<(), ApiError> {
    if digest.len() != 64
        || !digest
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ApiError::bad_request(
            "export hashes must be lowercase SHA-256 hex",
        ));
    }
    Ok(())
}

async fn download(
    State(state): State<SharedState>,
    request: Result<Json<DownloadRequest>, JsonRejection>,
) -> Result<Response, ApiError> {
    let Json(request) = request.map_err(json_error)?;
    validate_digest(&request.bundle_sha256)?;
    let preview = PreviewRequest {
        server_incarnation: request.server_incarnation,
        project_incarnation: request.project_incarnation,
        project_revision: request.project_revision,
        installation: request.installation,
        language: request.language,
    };
    preview.validate()?;
    tokio::task::spawn_blocking(move || {
        let bundle = build(&state, &preview)?;
        if format!("{:x}", Sha256::digest(&bundle.bytes)) != request.bundle_sha256 {
            return Err(stale());
        }
        let mut response = Body::from(bundle.bytes).into_response();
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/zip"),
        );
        response.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_static("attachment; filename=\"monitoring-export.zip\""),
        );
        Ok(private_response(response))
    })
    .await
    .map_err(|_| ApiError::internal("monitoring export worker failed"))?
}
