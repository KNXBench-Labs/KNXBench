use std::path::Path;

use axum::extract::Path as AxumPath;
use axum::extract::State;
use axum::routing::{delete, post};
use axum::{Json, Router};
use serde::Deserialize;

use crate::domain;
use crate::errors::ApiError;
use crate::SharedState;

pub fn project_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/project/import", post(import_project))
        .route("/api/project/open", post(open_native_project))
        .route("/api/project/save", post(save_project))
        .route("/api/project/save-as", post(save_project_as))
        .route("/api/device/{id}", axum::routing::get(device_detail))
        .route("/api/individual-address", post(set_individual_address))
        .route("/api/com-object-dpt", post(set_com_object_dpt))
        .route("/api/group-addresses", post(create_group_address))
        .route("/api/group-addresses/{id}", delete(delete_group_address))
        .route("/api/undo", post(undo))
        .route("/api/redo", post(redo))
}

#[derive(Deserialize)]
pub(crate) struct PathBody {
    pub(crate) path: String,
}

async fn import_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::open_project(&state, Path::new(&body.path))
        .map(Json)
        .map_err(ApiError::internal)
}

async fn open_native_project(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::open_native_project(&state, Path::new(&body.path))
        .map(Json)
        .map_err(ApiError::internal)
}

async fn save_project(State(state): State<SharedState>) -> Result<(), ApiError> {
    domain::save_project(&state).map_err(ApiError::internal)
}

async fn save_project_as(
    State(state): State<SharedState>,
    Json(body): Json<PathBody>,
) -> Result<(), ApiError> {
    domain::save_project_as(&state, Path::new(&body.path)).map_err(ApiError::internal)
}

async fn device_detail(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::DeviceDetail>, ApiError> {
    domain::device_detail(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetIndividualAddressBody {
    device_id: u32,
    address: Option<String>,
}

async fn set_individual_address(
    State(state): State<SharedState>,
    Json(body): Json<SetIndividualAddressBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_individual_address_impl(&state, body.device_id, body.address)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SetComObjectDptBody {
    com_object_id: u32,
    dpt: Option<String>,
}

async fn set_com_object_dpt(
    State(state): State<SharedState>,
    Json(body): Json<SetComObjectDptBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::set_com_object_dpt_impl(&state, body.com_object_id, body.dpt)
        .map(Json)
        .map_err(ApiError::bad_request)
}

#[derive(Deserialize)]
struct CreateGroupAddressBody {
    name: String,
    address: String,
}

async fn create_group_address(
    State(state): State<SharedState>,
    Json(body): Json<CreateGroupAddressBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::create_group_address_impl(&state, body.name, body.address)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn delete_group_address(
    State(state): State<SharedState>,
    AxumPath(id): AxumPath<u32>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::delete_group_address_impl(&state, id)
        .map(Json)
        .map_err(ApiError::bad_request)
}

async fn undo(State(state): State<SharedState>) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::undo_impl(&state).map(Json).map_err(ApiError::bad_request)
}

async fn redo(State(state): State<SharedState>) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    domain::redo_impl(&state).map(Json).map_err(ApiError::bad_request)
}
