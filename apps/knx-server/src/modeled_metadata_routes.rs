//! Explicit authenticated views of current modeled project metadata, never raw-source guesses.

use crate::{errors::ApiError, SharedState};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    Json,
};
use serde::Serialize;
use std::sync::atomic::Ordering;

#[derive(Serialize)]
pub(crate) struct MetadataView {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    metadata: knx_app::modeled_metadata::ProjectMetadata,
}
pub(crate) async fn current(
    State(state): State<SharedState>,
) -> Result<(HeaderMap, Json<MetadataView>), ApiError> {
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    let metadata = knx_app::modeled_metadata::project(project)
        .map_err(|error| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, error))?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((
        headers,
        Json(MetadataView {
            server_incarnation: state.server_incarnation.clone(),
            project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
            snapshot_revision: state.project_revision.load(Ordering::Relaxed),
            metadata,
        }),
    ))
}
