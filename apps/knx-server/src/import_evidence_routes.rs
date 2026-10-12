//! Explicit authenticated read of complete original import findings for the open project.

use crate::{errors::ApiError, SharedState};
use axum::{extract::State, http::StatusCode, Json};
use serde::Serialize;
use std::sync::atomic::Ordering;

#[derive(Serialize)]
pub(crate) struct ImportEvidenceView {
    available: bool,
    historical: bool,
    server_incarnation: String,
    project_incarnation: u64,
    report_json: Option<String>,
}

pub(crate) async fn read(
    State(state): State<SharedState>,
) -> Result<Json<ImportEvidenceView>, ApiError> {
    // The same project lock that publishes metadata scopes the whole response.
    let project = state.project.lock().expect("state mutex poisoned");
    project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    let evidence = knx_app::import_evidence::read(&opaque)
        .map_err(|message| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, message))?;
    let available = evidence.is_some();
    Ok(Json(ImportEvidenceView {
        available,
        historical: available && state.import_report_historical.load(Ordering::Relaxed),
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
        report_json: evidence.map(|evidence| evidence.report_json),
    }))
}
