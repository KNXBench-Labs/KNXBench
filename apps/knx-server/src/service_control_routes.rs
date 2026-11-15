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
//!
//! Both operations also exist as strict explicit-TCP siblings
//! (`POST .../tcp/read`, `POST .../tcp`): a distinct body with a required project
//! binding and no unknown fields, so an old server cannot silently read or write
//! over UDP. The legacy routes and bodies stay UDP. A TCP operation opens TCP only,
//! never retries, and reports the sampled `connection` facts (also on failure).

use std::net::SocketAddrV4;
use std::sync::atomic::Ordering;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use knx_app::access_key::project_access_key;
use knx_app::operation_recording::{OperationKind, OperationStart};
use knx_app::service_control_backup::write_backup;
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::service_control::{
    read_service_control, set_individual_address_write_enable, ServiceControl, ServiceControlError,
};
use knx_net::TunnelAckWait;

use crate::device_compare_routes::ComparisonConnection;
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
    Router::new()
        .route("/api/device/service-control", get(read).post(write))
        .route("/api/device/service-control/tcp", post(write_tcp))
        .route("/api/device/service-control/tcp/read", post(read_tcp))
}

/// The project snapshot an explicit-TCP operation is bound to.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Binding {
    server_incarnation: String,
    project_incarnation: u64,
    revision: u64,
}

/// Refuses a stale or foreign snapshot before any I/O: the plan's key comes from the project.
fn check_binding(state: &SharedState, binding: &Binding) -> Result<(), ApiError> {
    if binding.server_incarnation != state.server_incarnation
        || binding.project_incarnation != state.project_incarnation.load(Ordering::SeqCst)
        || binding.revision != state.project_revision.load(Ordering::SeqCst)
    {
        return Err(ApiError::with_kind(
            StatusCode::CONFLICT,
            "serviceControlContextChanged",
            "the project changed since this review; read the device again",
        ));
    }
    Ok(())
}

/// A TCP response: the legacy body plus the transport facts, sampled before disconnect.
#[derive(Serialize)]
struct WithConnection<T> {
    #[serde(flatten)]
    body: T,
    connection: ComparisonConnection,
}

/// A failure of a TCP operation carries the facts too; legacy failures stay plain.
fn failure(
    status: StatusCode,
    detail: String,
    connection: Option<&ComparisonConnection>,
) -> ApiError {
    match connection {
        None => ApiError::with_status(status, detail),
        Some(c) => ApiError::with_status(status, detail).with_detail(
            "serviceControlFailed",
            "connection",
            serde_json::to_value(c).unwrap_or_default(),
        ),
    }
}

/// Connect failure: the legacy error, plus unavailable facts for a TCP operation.
fn connect_failure(
    gateway: SocketAddrV4,
    error: &crate::bus::BusSessionError,
    transport: knx_net::TunnelTransport,
) -> ApiError {
    let error = crate::tunnel_holder::gateway_connect_failed(gateway, error);
    match transport {
        knx_net::TunnelTransport::Udp => error,
        _ => error.with_detail(
            "gatewayConnectFailed",
            "connection",
            serde_json::to_value(ComparisonConnection::unopened()).unwrap_or_default(),
        ),
    }
}

/// Facts of a TCP operation, sampled while the real handle exists; recorded as a note.
fn sample(
    transport: knx_net::TunnelTransport,
    recorder: &knx_app::operation_recording::OperationRecorder,
    tunnel: &dyn crate::bus::BusTunnel,
) -> Option<ComparisonConnection> {
    (transport == knx_net::TunnelTransport::Tcp).then(|| {
        let connection = ComparisonConnection::sampled(transport, tunnel);
        recorder.note(
            "connection",
            serde_json::to_value(&connection).unwrap_or_default(),
        );
        connection
    })
}

async fn open_tunnel(
    state: &SharedState,
    operation: OperationStart,
    gateway: SocketAddrV4,
    transport: knx_net::TunnelTransport,
) -> (
    knx_app::operation_recording::OperationRecorder,
    Result<Box<dyn crate::bus::BusTunnel>, crate::bus::BusSessionError>,
) {
    match transport {
        knx_net::TunnelTransport::Udp => {
            crate::operation_recordings::open(
                state,
                operation,
                gateway,
                TunnelAckWait::Standard,
                &state.address_programming_timing,
            )
            .await
        }
        transport => {
            crate::operation_recordings::open_with_transport(
                state,
                operation,
                gateway,
                TunnelAckWait::Standard,
                &state.address_programming_timing,
                transport,
            )
            .await
        }
    }
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
    // Goal A3: no new tunnel once a stop has begun (KL-163).
    state.stop_admission.admit()?;
    let programming = state.address_programming.lock().await;
    if let Some(holder) = crate::tunnel_holder::download_holder(state) {
        return Err(crate::tunnel_holder::gateway_busy(
            holder,
            "a download to a device is running: the gateway serves one tunnel",
        ));
    }
    if let Some(holder) = programming
        .as_ref()
        .and_then(crate::tunnel_holder::programming_holder_of)
    {
        return Err(crate::tunnel_holder::gateway_busy(
            holder,
            "an individual-address programming is running",
        ));
    }
    let monitor = state.bus_session.lock().await;
    if let Some(existing) = monitor.as_ref() {
        return Err(crate::tunnel_holder::gateway_busy(
            crate::tunnel_holder::monitor_holder_of(existing),
            "stop the bus monitor first: the gateway serves one tunnel",
        ));
    }
    let scan = state.line_scan_session.lock().await;
    if let Some(holder) = scan.as_ref().and_then(crate::tunnel_holder::scan_holder_of) {
        return Err(crate::tunnel_holder::gateway_busy(
            holder,
            "a line scan is running: the gateway serves one tunnel",
        ));
    }
    Ok((programming, monitor, scan))
}

async fn read(
    State(state): State<SharedState>,
    Query(query): Query<ReadQuery>,
) -> Result<Json<ServiceControlDto>, ApiError> {
    read_with(
        &state,
        &query.address,
        &query.gateway,
        knx_net::TunnelTransport::Udp,
    )
    .await
    .map(|(dto, _)| Json(dto))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TcpReadRequest {
    address: String,
    gateway: String,
    binding: Binding,
}

async fn read_tcp(
    State(state): State<SharedState>,
    Json(body): Json<TcpReadRequest>,
) -> Result<Json<WithConnection<ServiceControlDto>>, ApiError> {
    require_enabled(&state)?;
    check_binding(&state, &body.binding)?;
    let (dto, connection) = read_with(
        &state,
        &body.address,
        &body.gateway,
        knx_net::TunnelTransport::Tcp,
    )
    .await?;
    Ok(Json(WithConnection {
        body: dto,
        connection: connection.expect("a TCP read samples its connection"),
    }))
}

async fn read_with(
    state: &SharedState,
    address: &str,
    gateway: &str,
    transport: knx_net::TunnelTransport,
) -> Result<(ServiceControlDto, Option<ComparisonConnection>), ApiError> {
    require_enabled(state)?;
    let gateway = parse_gateway(gateway)?;
    let address = parse_address(address)?;
    let plan = plan(state)?;
    let _reservation = reserve_tunnel(state).await?;
    let activity = state
        .one_shot_activity
        .start("serviceControlRead", Some(address.to_string()));
    let (recorder, connected) = open_tunnel(
        state,
        OperationStart::new(OperationKind::ServiceControl).target(
            serde_json::json!({ "individualAddress": address.to_string(), "write": false }),
        ),
        gateway,
        transport,
    )
    .await;
    let connected = match connected {
        Ok(connected) => connected,
        Err(e) => {
            activity.finish("failed");
            return Err(connect_failure(gateway, &e, transport));
        }
    };
    let tunnel = TunnelTransport(connected.as_ref());
    let outcome =
        read_service_control(&tunnel, address, plan, state.address_programming_timing).await;
    // Sampled while the real handle exists, independently of optional recording.
    let connection = sample(transport, &recorder, connected.as_ref());
    let _ = connected.disconnect().await;
    crate::operation_recordings::finish_result(
        &recorder,
        &outcome,
        serde_json::json!({ "value": outcome.as_ref().ok().map(|v| format!("{v:?}")) }),
    );
    activity.finish(if outcome.is_ok() {
        "finished"
    } else {
        "failed"
    });
    match outcome {
        Ok(value) => Ok((ServiceControlDto::new(address, value), connection)),
        Err(e) => Err(failure(status_of(&e), e.to_string(), connection.as_ref())),
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteRequest {
    address: String,
    gateway: String,
    confirmation: String,
    enable: bool,
    /// Unit-test stall at the file-write boundary; never accepted over HTTP.
    #[cfg(test)]
    #[serde(skip)]
    before_backup: Option<Box<dyn FnOnce() + Send>>,
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
    write_with(&state, body, knx_net::TunnelTransport::Udp)
        .await
        .map(|(response, _)| Json(response))
}

/// Same body as the legacy write, plus the required project binding; nothing else.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TcpWriteRequest {
    address: String,
    gateway: String,
    confirmation: String,
    enable: bool,
    binding: Binding,
}

async fn write_tcp(
    State(state): State<SharedState>,
    Json(body): Json<TcpWriteRequest>,
) -> Result<Json<WithConnection<WriteResponse>>, ApiError> {
    require_enabled(&state)?;
    check_binding(&state, &body.binding)?;
    let (response, connection) = write_with(
        &state,
        WriteRequest {
            address: body.address,
            gateway: body.gateway,
            confirmation: body.confirmation,
            enable: body.enable,
            #[cfg(test)]
            before_backup: None,
        },
        knx_net::TunnelTransport::Tcp,
    )
    .await?;
    Ok(Json(WithConnection {
        body: response,
        connection: connection.expect("a TCP write samples its connection"),
    }))
}

async fn write_with(
    state: &SharedState,
    body: WriteRequest,
    transport: knx_net::TunnelTransport,
) -> Result<(WriteResponse, Option<ComparisonConnection>), ApiError> {
    require_enabled(state)?;
    let gateway = parse_gateway(&body.gateway)?;
    let address = parse_address(&body.address)?;
    let authorisation = WriteAuthorisation::for_hardware(
        address,
        WriteScope::IndividualAddressWriteEnable,
        &body.confirmation,
    )
    .map_err(|e| ApiError::bad_request(format!("not written: {e}")))?;
    let plan = plan(state)?;
    let _reservation = reserve_tunnel(state).await?;
    let activity = state
        .one_shot_activity
        .start_write("serviceControlWrite", Some(address.to_string()))
        .map_err(|_| {
            ApiError::with_status(
                StatusCode::SERVICE_UNAVAILABLE,
                "activity history unavailable; not sent",
            )
        })?;
    if state.one_shot_activity.ensure_write_available().is_err() {
        activity.finish(WriteOutcome::NotSent);
        return Err(ApiError::with_status(
            StatusCode::SERVICE_UNAVAILABLE,
            "activity history unavailable; not sent",
        ));
    }
    let (recorder, connected) = open_tunnel(
        state,
        OperationStart::new(OperationKind::ServiceControl).target(serde_json::json!({
            "individualAddress": address.to_string(),
            "write": true,
            "enable": body.enable,
        })),
        gateway,
        transport,
    )
    .await;
    let connected = match connected {
        Ok(connected) => connected,
        Err(e) => {
            activity.finish(WriteOutcome::NotSent);
            return Err(connect_failure(gateway, &e, transport));
        }
    };
    let tunnel = TunnelTransport(connected.as_ref());
    let mut backup_path = None;
    #[cfg(test)]
    let mut before_backup = body.before_backup;
    let outcome = set_individual_address_write_enable(
        &tunnel,
        plan,
        state.address_programming_timing,
        authorisation,
        body.enable,
        |before| {
            // Both records are durable (fsync) while the tunnel is open: keep
            // them off the tunnel's runtime workers (S2-04, issue #30), but
            // only them, so the request stays cancellable everywhere else.
            crate::tunnel_workers::blocking_off_tunnel_workers(|| {
                #[cfg(test)]
                if let Some(hook) = before_backup.take() {
                    hook();
                }
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
                activity
                    .mark_send_possible()
                    .map_err(|_| "activity write intent unavailable; not sent".to_string())
            })
        },
    )
    .await;
    // Sampled while the real handle exists, independently of optional recording.
    let connection = sample(transport, &recorder, connected.as_ref());
    let _ = connected.disconnect().await;
    let write_outcome = match &outcome {
        Ok(change) if change.written => WriteOutcome::Verified,
        Ok(_) => WriteOutcome::NoChange,
        Err(_) if activity.send_possible() => WriteOutcome::EffectUnverified,
        Err(_) => WriteOutcome::NotSent,
    };
    crate::operation_recordings::finish_result(
        &recorder,
        &outcome,
        serde_json::json!({
            "writeOutcome": match write_outcome {
                WriteOutcome::Verified => "verified",
                WriteOutcome::NoChange => "noChange",
                WriteOutcome::EffectUnverified => "effectUnverified",
                WriteOutcome::NotSent => "notSent",
            },
            "change": outcome.as_ref().ok().map(|c| format!("{c:?}")),
            "backupRecorded": backup_path.is_some(),
        }),
    );
    activity.finish(write_outcome);
    match outcome {
        Ok(change) => Ok((
            WriteResponse {
                before: ServiceControlDto::new(address, change.before),
                after: format!("{:04X}", change.after),
                individual_address_write_enabled: change.after
                    & knx_core::commissioning::properties::SERVICE_CONTROL_IA_WRITE_ENABLE
                    != 0,
                written: change.written,
                backup_path: backup_path.map(|p: std::path::PathBuf| p.display().to_string()),
            },
            connection,
        )),
        Err(e) => {
            let detail = match backup_path {
                Some(path) => format!("{e}; pre-write property backup: {}", path.display()),
                None => e.to_string(),
            };
            Err(failure(status_of(&e), detail, connection.as_ref()))
        }
    }
}

#[cfg(test)]
mod tests;
