//! Stored bus access and IP configuration over HTTP: read-only, hidden until revealed (Goal B3).
//!
//! `GET /api/project/stored-bus-access` lists every stored `BusAccess` and
//! `IPConfig` record of the open project with every endpoint value hidden;
//! `POST /api/project/stored-bus-access/reveal` answers one record with all
//! of its values, on the user's explicit request. Neither route writes,
//! logs a value, connects anywhere or touches the gateway settings.

use crate::{errors::ApiError, SharedState};
use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use knx_app::stored_bus_access::{stored_bus_access, StoredBusAccess, StoredRecord};
use serde::Deserialize;

pub(crate) fn router() -> Router<SharedState> {
    Router::new()
        .route("/api/project/stored-bus-access", get(list))
        .route("/api/project/stored-bus-access/reveal", post(reveal))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RevealRequest {
    record_id: String,
}

fn view(state: &SharedState, reveal: Option<&str>) -> Result<StoredBusAccess, ApiError> {
    // Same order as every other reader of both: project, then opaque.
    let project = state.project.lock().expect("state mutex poisoned");
    let project = project
        .as_ref()
        .ok_or_else(|| ApiError::bad_request("no project open"))?;
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    Ok(stored_bus_access(project, &opaque, reveal))
}

async fn list(State(state): State<SharedState>) -> Result<Json<StoredBusAccess>, ApiError> {
    view(&state, None).map(Json)
}

async fn reveal(
    State(state): State<SharedState>,
    Json(request): Json<RevealRequest>,
) -> Result<Json<StoredRecord>, ApiError> {
    view(&state, Some(&request.record_id))?
        .records
        .into_iter()
        .find(|record| record.revealed)
        .map(Json)
        // The id is not echoed: it is harmless, but an error is not the place
        // to repeat request content.
        .ok_or_else(|| {
            ApiError::with_kind(
                StatusCode::NOT_FOUND,
                "storedBusAccessChanged",
                "this stored record is no longer in the open project; reload the list",
            )
        })
}
