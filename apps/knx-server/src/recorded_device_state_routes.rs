//! No bus/device/product/source IO: only the current project's already held model.
use crate::{errors::ApiError, SharedState};
use axum::{
    extract::{Path, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    Json,
};
use serde::Serialize;
use std::sync::atomic::Ordering;
#[derive(Serialize)]
pub(crate) struct DeviceStateView {
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
    state: knx_app::recorded_device_state::RecordedDeviceState,
}
pub(crate) async fn current(
    State(state): State<SharedState>,
    Path(id): Path<u32>,
) -> Result<(HeaderMap, Json<DeviceStateView>), ApiError> {
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    let device = project
        .devices
        .get(knx_core::ids::DeviceId(id))
        .ok_or_else(|| ApiError::with_status(StatusCode::NOT_FOUND, "device not found"))?;
    let data = knx_app::recorded_device_state::device(device)
        .map_err(|error| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, error))?;
    let mut headers = HeaderMap::new();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok((
        headers,
        Json(DeviceStateView {
            server_incarnation: state.server_incarnation.clone(),
            project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
            snapshot_revision: state.project_revision.load(Ordering::Relaxed),
            state: data,
        }),
    ))
}
