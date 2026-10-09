//! Guarded name-only routes; no protocol or catalogue mutation.
use crate::{
    domain::{self, RenameError, RenameExpectation},
    errors::ApiError,
    SharedState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RenameBody {
    name: String,
    expected_name: String,
    server_incarnation: String,
    snapshot_revision: u64,
    project_incarnation: u64,
}
impl RenameBody {
    fn expectation(&self) -> RenameExpectation {
        RenameExpectation {
            server_incarnation: self.server_incarnation.clone(),
            snapshot_revision: self.snapshot_revision,
            project_incarnation: self.project_incarnation,
            expected_name: self.expected_name.clone(),
        }
    }
}
fn map_error(error: RenameError) -> ApiError {
    match error {
        RenameError::Conflict(message) => ApiError::conflict("renameConflict", message),
        RenameError::Invalid(message) => ApiError::refused("renameInvalid", message),
        RenameError::Failure(message) => ApiError::internal(message),
    }
}
pub(crate) async fn device(
    State(state): State<SharedState>,
    Path(id): Path<u32>,
    Json(body): Json<RenameBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let expectation = body.expectation();
    domain::rename_impl(
        &state,
        knx_core::Command::RenameDevice {
            device: knx_core::DeviceId(id),
            name: body.name,
        },
        expectation,
    )
    .map(Json)
    .map_err(map_error)
}
pub(crate) async fn group_address(
    State(state): State<SharedState>,
    Path(id): Path<u32>,
    Json(body): Json<RenameBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let expectation = body.expectation();
    domain::rename_impl(
        &state,
        knx_core::Command::RenameGroupAddress {
            id: knx_core::GroupAddressId(id),
            name: body.name,
        },
        expectation,
    )
    .map(Json)
    .map_err(map_error)
}
