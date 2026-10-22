//! Reads optional parameter source evidence without granting any write authority.

use crate::{domain, errors::ApiError, SharedState};
use axum::{
    extract::{Path, Query, State},
    routing::get,
    Json, Router,
};
use knx_productdb::parameter_editor::{
    parameter_editor_evidence, EditorEvidenceIssue, ParameterEditorEvidence,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, sync::atomic::Ordering};

pub(crate) fn router() -> Router<SharedState> {
    Router::new().route("/api/device/{id}/parameter-editor-source", get(read))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SourceQuery {
    ets_id: String,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    device: u32,
    parameter_ref_id: String,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    evidence: Evidence,
}
#[derive(Serialize)]
#[serde(tag = "status", rename_all = "camelCase")]
enum Evidence {
    Declared { source: Source },
    Unavailable { reason: &'static str },
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Source {
    source_sha256: String,
    program_id: String,
    parameter_type_id: String,
    schema: u16,
    child: String,
    attributes: BTreeMap<String, String>,
    parent_attributes: BTreeMap<String, String>,
}
fn unavailable(reason: &'static str) -> Evidence {
    Evidence::Unavailable { reason }
}
fn issue_name(issue: EditorEvidenceIssue) -> &'static str {
    match issue {
        EditorEvidenceIssue::MissingReference => "missingReference",
        EditorEvidenceIssue::MissingSource => "missingSource",
        EditorEvidenceIssue::SourceBudget => "sourceBudget",
        EditorEvidenceIssue::SourceIdentity => "sourceIdentity",
        EditorEvidenceIssue::UnsupportedXml => "unsupportedXml",
        EditorEvidenceIssue::AmbiguousDefinition => "ambiguousDefinition",
        EditorEvidenceIssue::ModelMismatch => "modelMismatch",
        EditorEvidenceIssue::MissingDeclaration => "missingDeclaration",
    }
}
fn identity(state: &SharedState, device: u32) -> Result<(String, u64, u64), ApiError> {
    let guard = state.project.lock().expect("state mutex poisoned");
    let project = guard
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("No project open"))?;
    let device = project
        .devices
        .get(knx_core::DeviceId(device))
        .ok_or_else(|| ApiError::bad_request("Device not found"))?;
    Ok((
        device.program_ref.clone(),
        state.project_incarnation.load(Ordering::Relaxed),
        domain::current_project_revision(state),
    ))
}
async fn read(
    State(state): State<SharedState>,
    Path(device): Path<u32>,
    Query(query): Query<SourceQuery>,
) -> Result<Json<View>, ApiError> {
    if query.ets_id.is_empty() || query.ets_id.len() > 1024 {
        return Err(ApiError::bad_request(
            "Parameter reference must contain 1 to 1024 bytes",
        ));
    }
    let before = identity(&state, device)?;
    // Never hold the project and product database mutexes together.
    let evidence = if let Some(owner) = state.product_db.as_ref() {
        owner
            .with_read(|products| -> Result<_, ApiError> {
                match knx_productdb::query::resolve_program(products, &before.0)
                    .map_err(|e| ApiError::internal(e.to_string()))?
                {
                    None => Ok(unavailable("missingProgram")),
                    Some(program) => {
                        match parameter_editor_evidence(products, &program, &query.ets_id)
                            .map_err(|e| ApiError::internal(e.to_string()))?
                        {
                            ParameterEditorEvidence::Unavailable(issue) => {
                                Ok(unavailable(issue_name(issue)))
                            }
                            ParameterEditorEvidence::Declared(source) => Ok(Evidence::Declared {
                                source: Source {
                                    source_sha256: source.source_sha256,
                                    program_id: source.program_id,
                                    parameter_type_id: source.parameter_type_id,
                                    schema: source.schema,
                                    child: source.child,
                                    attributes: source.attributes,
                                    parent_attributes: source.parameter_type_attributes,
                                },
                            }),
                        }
                    }
                }
            })
            .map_err(crate::catalogue_owner::owner_error)??
    } else {
        unavailable("missingProductDatabase")
    };
    let after = identity(&state, device)?;
    if before != after {
        return Err(ApiError::with_status(
            axum::http::StatusCode::CONFLICT,
            "Project changed while reading parameter evidence",
        ));
    }
    Ok(Json(View {
        device,
        parameter_ref_id: query.ets_id,
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: before.1,
        snapshot_revision: before.2,
        evidence,
    }))
}
