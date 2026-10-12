//! Authenticated catalogue/read adapters for retained selective-source import reports.

use crate::{errors::ApiError, SharedState};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use knx_app::import_evidence::source_reports;
use serde::Serialize;
use std::sync::atomic::Ordering;

#[derive(Serialize)]
pub(crate) struct SourceCatalogue {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    records: Vec<source_reports::SourceReportSummary>,
}

#[derive(Serialize)]
pub(crate) struct SourceReportView {
    scope: &'static str,
    id: String,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    report_json: String,
}

pub(crate) async fn catalogue(
    State(state): State<SharedState>,
) -> Result<Json<SourceCatalogue>, ApiError> {
    let project = state.project.lock().expect("state mutex poisoned");
    project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    let records = source_reports::catalogue(&opaque)
        .map_err(|message| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, message))?;
    Ok(Json(SourceCatalogue {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
        snapshot_revision: state.project_revision.load(Ordering::Relaxed),
        records,
    }))
}

pub(crate) async fn read(
    State(state): State<SharedState>,
    Path(id): Path<String>,
) -> Result<Json<SourceReportView>, ApiError> {
    let project = state.project.lock().expect("state mutex poisoned");
    project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    let report = source_reports::read(&opaque, &id)
        .map_err(|message| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, message))?;
    Ok(Json(SourceReportView {
        scope: "selectiveSource",
        id,
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
        snapshot_revision: state.project_revision.load(Ordering::Relaxed),
        report_json: report.report_json,
    }))
}
