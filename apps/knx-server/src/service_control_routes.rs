//! `/api/device/service-control`: `PID_SERVICE_CONTROL` bit 2 (K12, ADR-0051).
//!
//! RES §4.2.8 bit 2, *"Individual Address Write Enable"*: a device that keeps
//! it clear ignores an address change by serial number (KNOWN_LIMITATIONS
//! §139). KNXBench never sets it on its own. These routes are the explicit,
//! opt-in operator action the user asked for on 2026-09-30:
//!
//! * `GET` reads the bit, read-only;
//! * `POST` sets or clears it, changing only bit 2.
//!
//! Both are refused unless the settings file carries
//! [`DEBUG_SETTING_KEY`] `= true`, which the Settings panel's Debug section
//! writes. It defaults to off: an unset key, a non-boolean value or an
//! unreadable/newer settings file all count as off. The gate is here, in
//! the server, not only in the UI, so a request that bypasses the panel is
//! refused too.
//!
//! `POST` further needs the device-specific confirmation phrase for
//! [`WriteScope::IndividualAddressWriteEnable`] and takes the same tunnel
//! lock order as `serial_address_routes`. The HTTP API takes no access key
//! (as `device_download_routes::keying`); the project's key is used when
//! the project has one.

use std::net::SocketAddrV4;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use knx_app::access_key::project_access_key;
use knx_app::service_control_backup::write_backup;
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::service_control::{
    read_service_control, set_individual_address_write_enable, ServiceControl, ServiceControlError,
};

use crate::bus_scan::LineScanStatus;
use crate::device_download::TunnelTransport;
use crate::errors::ApiError;
use crate::one_shot_activity::WriteOutcome;
use crate::settings;
use crate::SharedState;

/// The preference that opens these routes. Written by the Settings panel's
/// Debug section; everything else about the settings file stays opaque to
/// the server (`crate::settings`).
pub const DEBUG_SETTING_KEY: &str = "debugIndividualAddressWriteEnable";

pub fn service_control_routes() -> Router<SharedState> {
    Router::new().route("/api/device/service-control", get(read).post(write))
}

fn conflict(message: impl Into<String>) -> ApiError {
    ApiError::with_status(StatusCode::CONFLICT, message)
}

/// Whether the settings file says `true` for [`DEBUG_SETTING_KEY`].
/// Anything else, including a file this build refuses, is off. Takes
/// `settings_lock` because [`settings::load`] may write a migration back
/// (`settings_routes::read_settings`).
fn enabled(state: &SharedState) -> bool {
    let _guard = state.settings_lock.lock().expect("state mutex poisoned");
    let Ok(load) = settings::load(&state.data_dir) else {
        return false;
    };
    matches!(
        load.document().preferences.get(DEBUG_SETTING_KEY),
        Some(Value::Bool(true))
    )
}

fn require_enabled(state: &SharedState) -> Result<(), ApiError> {
    if enabled(state) {
        Ok(())
    } else {
        Err(ApiError::with_status(
            StatusCode::FORBIDDEN,
            format!(
                "Individual Address Write Enable is a debug action and is off; \
                 enable it under Settings → Debug ({DEBUG_SETTING_KEY})"
            ),
        ))
    }
}

fn parse_gateway(text: &str) -> Result<SocketAddrV4, ApiError> {
    text.parse()
        .map_err(|_| ApiError::bad_request("gateway is not a host:port IPv4 address"))
}

fn parse_address(text: &str) -> Result<IndividualAddress, ApiError> {
    let address: IndividualAddress = text
        .parse()
        .map_err(|e| ApiError::bad_request(format!("invalid individual address {text:?}: {e}")))?;
    ContactableAddress::new(address).map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(address)
}

fn plan(state: &SharedState) -> Result<AuthorisationPlan, ApiError> {
    let opaque = state.opaque.lock().expect("state mutex poisoned");
    let key = project_access_key(&opaque)
        .map_err(|e| ApiError::with_status(StatusCode::UNPROCESSABLE_ENTITY, e.to_string()))?;
    Ok(AuthorisationPlan::from_operator_key(key))
}

fn status_of(error: &ServiceControlError) -> StatusCode {
    match error {
        ServiceControlError::InvertedMask | ServiceControlError::NotPresent => {
            StatusCode::UNPROCESSABLE_ENTITY
        }
        ServiceControlError::Malformed { .. } | ServiceControlError::Session { .. } => {
            StatusCode::BAD_GATEWAY
        }
        ServiceControlError::PreWriteBackup(_) => StatusCode::INSUFFICIENT_STORAGE,
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceControlDto {
    address: String,
    /// `PID_SERVICE_CONTROL` as read, four hex digits.
    raw: String,
    mask: String,
    individual_address_write_enabled: bool,
}

impl ServiceControlDto {
    fn new(address: IndividualAddress, value: ServiceControl) -> Self {
        Self {
            address: address.to_string(),
            raw: format!("{:04X}", value.raw),
            mask: format!("{:04X}", value.mask.0),
            individual_address_write_enabled: value.individual_address_write_enabled(),
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReadQuery {
    address: String,
    gateway: String,
}

/// Serialize management operations against retained sessions in the same
/// lock order as `serial_address_routes::write`, before opening a tunnel.
/// Hold all guards through disconnect: another operation must not borrow the
/// gateway while this request still owns it.
type TunnelReservation<'a> = (
    tokio::sync::MutexGuard<'a, Option<crate::AddressProgrammingSession>>,
    tokio::sync::MutexGuard<'a, Option<crate::bus::BusSession>>,
    tokio::sync::MutexGuard<'a, Option<crate::bus_scan::LineScanSession>>,
);

async fn reserve_tunnel(state: &SharedState) -> Result<TunnelReservation<'_>, ApiError> {
    let programming = state.address_programming.lock().await;
    if crate::device_download_routes::download_in_progress(state) {
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
    Ok((programming, monitor, scan))
}

async fn read(
    State(state): State<SharedState>,
    Query(query): Query<ReadQuery>,
) -> Result<Json<ServiceControlDto>, ApiError> {
    require_enabled(&state)?;
    let gateway = parse_gateway(&query.gateway)?;
    let address = parse_address(&query.address)?;
    let plan = plan(&state)?;
    let _reservation = reserve_tunnel(&state).await?;
    let activity = state
        .one_shot_activity
        .start("serviceControlRead", Some(address.to_string()));
    let connected = match state.connector.connect_tunnel(gateway).await {
        Ok(connected) => connected,
        Err(e) => {
            activity.finish("failed");
            return Err(ApiError::with_status(
                StatusCode::BAD_GATEWAY,
                e.to_string(),
            ));
        }
    };
    let tunnel = TunnelTransport(connected.as_ref());
    let outcome =
        read_service_control(&tunnel, address, plan, state.address_programming_timing).await;
    let _ = connected.disconnect().await;
    activity.finish(if outcome.is_ok() {
        "finished"
    } else {
        "failed"
    });
    outcome
        .map(|value| Json(ServiceControlDto::new(address, value)))
        .map_err(|e| ApiError::with_status(status_of(&e), e.to_string()))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteRequest {
    address: String,
    gateway: String,
    confirmation: String,
    enable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WriteResponse {
    before: ServiceControlDto,
    /// The value read back, four hex digits.
    after: String,
    individual_address_write_enabled: bool,
    /// `false`: bit 2 already had the requested value; nothing was sent.
    written: bool,
    /// Durable property recovery evidence, absent for a no-op.
    backup_path: Option<String>,
}

async fn write(
    State(state): State<SharedState>,
    Json(body): Json<WriteRequest>,
) -> Result<Json<WriteResponse>, ApiError> {
    require_enabled(&state)?;
    let gateway = parse_gateway(&body.gateway)?;
    let address = parse_address(&body.address)?;
    let authorisation = WriteAuthorisation::for_hardware(
        address,
        WriteScope::IndividualAddressWriteEnable,
        &body.confirmation,
    )
    .map_err(|e| ApiError::bad_request(format!("not written: {e}")))?;
    let plan = plan(&state)?;
    let _reservation = reserve_tunnel(&state).await?;
    let activity = state
        .one_shot_activity
        .start_write("serviceControlWrite", Some(address.to_string()));
    let connected = match state.connector.connect_tunnel(gateway).await {
        Ok(connected) => connected,
        Err(e) => {
            activity.finish(WriteOutcome::NotSent);
            return Err(ApiError::with_status(
                StatusCode::BAD_GATEWAY,
                e.to_string(),
            ));
        }
    };
    let tunnel = TunnelTransport(connected.as_ref());
    let mut backup_path = None;
    let outcome = set_individual_address_write_enable(
        &tunnel,
        plan,
        state.address_programming_timing,
        authorisation,
        body.enable,
        |before| {
            let path = write_backup(
                &state.data_dir.join("device-backups"),
                address,
                before.before.mask.0,
                before.before.raw,
                before.device_control,
            )
            .map_err(|e| e.to_string())?;
            backup_path = Some(path);
            // The protocol invokes this callback immediately before its
            // write. A transport failure after here cannot prove no send.
            activity.mark_send_possible();
            Ok(())
        },
    )
    .await;
    let _ = connected.disconnect().await;
    let write_outcome = match &outcome {
        Ok(change) if change.written => WriteOutcome::Verified,
        Ok(_) => WriteOutcome::NoChange,
        Err(_) if activity.send_possible() => WriteOutcome::EffectUnverified,
        Err(_) => WriteOutcome::NotSent,
    };
    activity.finish(write_outcome);
    match outcome {
        Ok(change) => Ok(Json(WriteResponse {
            before: ServiceControlDto::new(address, change.before),
            after: format!("{:04X}", change.after),
            individual_address_write_enabled: change.after
                & knx_core::commissioning::properties::SERVICE_CONTROL_IA_WRITE_ENABLE
                != 0,
            written: change.written,
            backup_path: backup_path.map(|p: std::path::PathBuf| p.display().to_string()),
        })),
        Err(e) => {
            let detail = match backup_path {
                Some(path) => format!("{e}; pre-write property backup: {}", path.display()),
                None => e.to_string(),
            };
            Err(ApiError::with_status(status_of(&e), detail))
        }
    }
}
