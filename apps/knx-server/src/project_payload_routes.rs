//! Explicit current-bound retained payload catalogue and streamed inert local downloads.

use crate::{errors::ApiError, SharedState};
use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::Response,
    Json,
};
use serde::{Deserialize, Serialize};
use std::{io::Write, sync::atomic::Ordering};

#[derive(Serialize)]
pub(crate) struct Catalogue {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    max_download_bytes: usize,
    records: Vec<knx_app::project_payloads::PayloadSummary>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Binding {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
}

pub(crate) async fn catalogue(
    State(state): State<SharedState>,
) -> Result<(HeaderMap, Json<Catalogue>), ApiError> {
    let project = state.project.lock().expect("state mutex poisoned");
    project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    let records = knx_app::project_payloads::catalogue(&opaque)
        .map_err(|e| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, e))?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((
        headers,
        Json(Catalogue {
            server_incarnation: state.server_incarnation.clone(),
            project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
            snapshot_revision: state.project_revision.load(Ordering::Relaxed),
            max_download_bytes: knx_app::project_payloads::MAX_DOWNLOAD_BYTES,
            records,
        }),
    ))
}

pub(crate) async fn download(
    State(state): State<SharedState>,
    Path(id): Path<u32>,
    Query(binding): Query<Binding>,
) -> Result<Response, ApiError> {
    let tmp = {
        let project = state.project.lock().expect("state mutex poisoned");
        project
            .as_ref()
            .ok_or_else(|| ApiError::bad_request("no project open"))?;
        if binding.server_incarnation != state.server_incarnation
            || binding.project_incarnation != state.project_incarnation.load(Ordering::Relaxed)
            || binding.snapshot_revision != state.project_revision.load(Ordering::Relaxed)
        {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                "retained payload catalogue is stale; read it again",
            ));
        }
        let opaque = state.opaque.lock().expect("state mutex poisoned");
        let bytes = knx_app::project_payloads::download(&opaque, id)
            .map_err(|e| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, e))?;
        // Temporary file is only the admitted body, never a source-derived path.
        let mut tmp =
            tempfile::NamedTempFile::new().map_err(|e| ApiError::internal(e.to_string()))?;
        tmp.write_all(bytes)
            .map_err(|e| ApiError::internal(e.to_string()))?;
        tmp.into_temp_path()
    };
    let mut response = crate::fs_routes::stream_temp_file(tmp, 64 * 1024).await?;
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"retained-payload.bin\""),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("sandbox; default-src 'none'"),
    );
    Ok(response)
}
