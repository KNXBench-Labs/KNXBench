use std::path::Path;

use axum::extract::Path as AxumPath;
use axum::extract::State;
use axum::routing::post;
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
