//! `/api/device-address/by-serial`: MP §2.5 without a programming button, and MP §2.4's read.
//!
//! One request, one procedure: MP §2.5 is four exchanges and no wait for a
//! person, so there is no session to poll. The rules match
//! `address_programming_routes` (ADR-0046) and are checked in this order,
//! before any tunnel opens:
//!
//! 1. the new address parses and is not excluded;
//! 2. the confirmation phrase is the one for individual-address programming
//!    of that address (MP §2.5 sends no restart, so none is derived);
//! 3. the serial number is given, or the open project records it for the
//!    named device (Project Schema `DeviceInstance/@SerialNumber`), never
//!    guessed;
//! 4. the durable pre-write recovery gate currently refuses every confirmed
//!    write (including a possible no-op) before any tunnel opens. Read-only
//!    serial lookup remains available. The protocol procedure stays in the
//!    simulator for recovery-gate development.

use std::net::SocketAddrV4;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::serial_number::SerialNumber;
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::serial_number_write::{
    serial_number_read, serial_number_write, SerialNumberWriteError,
};

use crate::bus_scan::LineScanStatus;
use crate::device_download::TunnelTransport;
use crate::errors::ApiError;
use crate::SharedState;

pub fn serial_address_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/device-address/by-serial", post(write))
        .route("/api/device-address/find-serial", get(find))
}

fn conflict(message: impl Into<String>) -> ApiError {
    ApiError::with_status(StatusCode::CONFLICT, message)
}

fn parse_serial(text: &str) -> Result<SerialNumber, ApiError> {
    text.parse()
        .map_err(|e| ApiError::bad_request(format!("serialNumber {text:?}: {e}")))
}

/// The serial number from the request, or from the open project.
fn resolve(
    state: &SharedState,
    serial_number: Option<&str>,
    device: Option<&str>,
) -> Result<SerialNumber, ApiError> {
    match (serial_number, device) {
        (Some(text), None) => parse_serial(text),
        (None, Some(device)) => {
            let project = state.project.lock().expect("state mutex poisoned");
            let opaque = state.opaque.lock().expect("state mutex poisoned");
            knx_app::serial_number::project_serial_number_from_project(
                &opaque,
                project.as_ref(),
                device,
            )
            .map_err(|e| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?
            .ok_or_else(|| {
                ApiError::with_status(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    format!(
                        "the open project records no serial number for device {device}; \
                             give serialNumber (from the device label)"
                    ),
                )
            })
        }
        (None, None) => Err(ApiError::bad_request(
            "name the device: serialNumber, or device (its DeviceInstance id)",
        )),
        (Some(_), Some(_)) => Err(ApiError::bad_request(
            "serialNumber and device name the device twice",
        )),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteRequest {
    address: String,
    gateway: String,
    confirmation: String,
    serial_number: Option<String>,
    device: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WriteResponse {
    serial_number: String,
    previous_address: String,
    address: String,
    wrote: bool,
}

async fn write(
    State(state): State<SharedState>,
    Json(body): Json<WriteRequest>,
) -> Result<Json<WriteResponse>, ApiError> {
    let gateway: SocketAddrV4 = body
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request("gateway is not a host:port IPv4 address"))?;
    let new_address: IndividualAddress = body.address.parse().map_err(|e| {
        ApiError::bad_request(format!(
            "invalid individual address {:?}: {e}",
            body.address
        ))
    })?;
    ContactableAddress::new(new_address).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let authorisation = WriteAuthorisation::for_hardware(
        new_address,
        WriteScope::IndividualAddressProgramming,
        &body.confirmation,
    )
    .map_err(|e| ApiError::bad_request(format!("not written: {e}")))?;
    let serial = resolve(
        &state,
        body.serial_number.as_deref(),
        body.device.as_deref(),
    )?;

    // A phrase and a project serial are not durable recovery evidence. Refuse
    // even a possible no-op before opening a tunnel: that cannot be known yet.
    knx_app::serial_address_recovery::require_persistent_pre_write_recovery()
        .map_err(|message| ApiError::with_status(StatusCode::PRECONDITION_FAILED, message))?;

    // Same lock order as `address_programming_routes::start`.
    let programming = state.address_programming.lock().await;
    if crate::device_download_routes::download_in_progress(&state) {
        return Err(conflict(
            "a download to a device is running: the gateway serves one tunnel",
        ));
    }
    if programming.as_ref().is_some_and(|p| p.is_active()) {
        return Err(conflict("an individual-address programming is running"));
    }
    let monitor = state.bus_session.lock().await;
    if monitor.is_some() {
        return Err(conflict(
            "stop the bus monitor first: the gateway serves one tunnel",
        ));
    }
    let scan = state.line_scan_session.lock().await;
    if matches!(
        scan.as_ref().map(|scan| scan.status()),
        Some(LineScanStatus::Running)
    ) {
        return Err(conflict(
            "a line scan is running: the gateway serves one tunnel",
        ));
    }
    let tunnel = state
        .connector
        .connect_tunnel(gateway)
        .await
        .map_err(|e| ApiError::with_status(StatusCode::BAD_GATEWAY, e.to_string()))?;
    let outcome = serial_number_write(
        &TunnelTransport(tunnel.as_ref()),
        state.address_programming_timing,
        serial,
        authorisation,
    )
    .await;
    let _ = tunnel.disconnect().await;
    drop(scan);
    drop(monitor);
    drop(programming);
    match outcome {
        Ok(report) => Ok(Json(WriteResponse {
            serial_number: report.serial_number.to_string(),
            previous_address: report.previous_address.to_string(),
            address: report.verified_address.to_string(),
            wrote: report.wrote,
        })),
        Err(err) => {
            let status = match err {
                SerialNumberWriteError::NotFound(_) => StatusCode::NOT_FOUND,
                SerialNumberWriteError::OccupiedByAnotherDevice { .. }
                | SerialNumberWriteError::Excluded(_) => StatusCode::CONFLICT,
                SerialNumberWriteError::NotVerified { .. }
                | SerialNumberWriteError::Session { .. } => StatusCode::BAD_GATEWAY,
            };
            Err(ApiError::with_status(status, err.to_string()))
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FindQuery {
    gateway: String,
    serial_number: Option<String>,
    device: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FindResponse {
    serial_number: String,
    /// `None`: MP §2.4, no device with this serial number answered.
    address: Option<String>,
}

/// MP §2.4, read-only. Refused while anything else holds the gateway.
async fn find(
    State(state): State<SharedState>,
    Query(query): Query<FindQuery>,
) -> Result<Json<FindResponse>, ApiError> {
    let gateway: SocketAddrV4 = query
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request("gateway is not a host:port IPv4 address"))?;
    let serial = resolve(
        &state,
        query.serial_number.as_deref(),
        query.device.as_deref(),
    )?;
    let Ok(programming) = state.address_programming.try_lock() else {
        return Err(conflict("an individual-address programming is running"));
    };
    if crate::device_download_routes::download_in_progress(&state)
        || programming.as_ref().is_some_and(|p| p.is_active())
    {
        return Err(conflict(
            "the gateway is in use by a download or a programming",
        ));
    }
    let monitor = state.bus_session.lock().await;
    if monitor.is_some() {
        return Err(conflict(
            "stop the bus monitor first: the gateway serves one tunnel",
        ));
    }
    // A scan may hold the only usable tunnel. Do not wait behind a held
    // holder lock or open a second tunnel while a scan is running.
    let Ok(scan) = state.line_scan_session.try_lock() else {
        return Err(conflict("a line scan is using the gateway"));
    };
    if matches!(
        scan.as_ref().map(|scan| scan.status()),
        Some(LineScanStatus::Running)
    ) {
        return Err(conflict(
            "a line scan is running: the gateway serves one tunnel",
        ));
    }
    // No physical address is known yet: never substitute a serial number or
    // guessed address in the activity record. A dropped future is unknown.
    let activity = state.one_shot_activity.start("serialLookup", None);
    let tunnel = match state.connector.connect_tunnel(gateway).await {
        Ok(tunnel) => tunnel,
        Err(e) => {
            activity.finish("failed");
            return Err(ApiError::with_status(
                StatusCode::BAD_GATEWAY,
                e.to_string(),
            ));
        }
    };
    let found = serial_number_read(
        &TunnelTransport(tunnel.as_ref()),
        serial,
        state.address_programming_timing,
    )
    .await;
    let _ = tunnel.disconnect().await;
    activity.finish(if found.is_ok() { "finished" } else { "failed" });
    drop(scan);
    drop(monitor);
    drop(programming);
    let found = found.map_err(|e| ApiError::with_status(StatusCode::BAD_GATEWAY, e.to_string()))?;
    Ok(Json(FindResponse {
        serial_number: serial.to_string(),
        address: found.map(|address| address.to_string()),
    }))
}
