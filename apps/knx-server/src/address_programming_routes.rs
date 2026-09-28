//! `/api/device-address/*`: program a device's individual address on the button loop, and poll it.
//!
//! Programming the individual address is not a download (docs/GLOSSARY.md).
//! ADR-0046 fixes the rules; in this order:
//!
//! 1. `start` refuses before any socket opens unless the confirmation phrase
//!    is the one `AddressProgrammingAuthorisation::for_hardware` demands for
//!    the new address (it covers MP §2.3 step 4's restart), the wait is
//!    within bounds, and no monitor, scan, download or other programming
//!    runs.
//! 2. `status` returns the rounds whose answer changed since `since`.
//! 3. `stop` ends the wait. Once exactly one device was found, MP §2.3
//!    runs to its end and `stop` answers `409`.
//!
//! There is no plan route: the procedure depends on no project data, so
//! there is nothing a project edit could change between reading and
//! confirming (ADR-0046 §2).

use std::net::SocketAddrV4;
use std::sync::atomic::Ordering;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use knx_core::commissioning::mutation::{required_confirmation_phrase, WriteScope};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::programming_button_wait::{
    AddressProgrammingAuthorisation, ButtonWait,
};

use crate::address_programming::{
    AddressProgrammingEvent, AddressProgrammingSession, AddressProgrammingStatus, StopOutcome,
};
use crate::bus_scan::LineScanStatus;
use crate::errors::ApiError;
use crate::SharedState;

/// How long to wait for a button when the request names no wait. The
/// CLI's default (`knx device program-address`).
pub const DEFAULT_WAIT_SECONDS: u64 = 120;
/// `[A]` The CLI's maximum: RES §4.26.1 lets a device leave programming
/// mode by itself after four minutes; ten covers a walk across a building.
pub const MAX_WAIT_SECONDS: u64 = 600;

pub fn address_programming_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/device-address/phrase", get(phrase))
        .route("/api/device-address/start", post(start))
        .route("/api/device-address/status", get(status))
        .route("/api/device-address/stop", post(stop))
}

fn conflict(message: impl Into<String>) -> ApiError {
    ApiError::with_status(StatusCode::CONFLICT, message)
}

fn parse_new_address(address: &str) -> Result<IndividualAddress, ApiError> {
    let parsed: IndividualAddress = address.parse().map_err(|e| {
        ApiError::bad_request(format!("invalid individual address {address:?}: {e}"))
    })?;
    ContactableAddress::new(parsed).map_err(|e| ApiError::bad_request(e.to_string()))?;
    Ok(parsed)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PhraseQuery {
    address: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PhraseResponse {
    address: String,
    /// The phrase `start` demands. It names this address and this scope;
    /// it is not proof that a person read anything (ADR-0045 §2).
    confirmation_phrase: String,
    default_wait_seconds: u64,
    max_wait_seconds: u64,
}

/// What `start` will demand, for the address the user typed. Nothing is
/// sent and nothing is remembered.
async fn phrase(Query(query): Query<PhraseQuery>) -> Result<Json<PhraseResponse>, ApiError> {
    let address = parse_new_address(&query.address)?;
    Ok(Json(PhraseResponse {
        address: address.to_string(),
        confirmation_phrase: required_confirmation_phrase(
            address,
            WriteScope::IndividualAddressProgramming,
        ),
        default_wait_seconds: DEFAULT_WAIT_SECONDS,
        max_wait_seconds: MAX_WAIT_SECONDS,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartRequest {
    address: String,
    gateway: String,
    confirmation: String,
    wait_seconds: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartResponse {
    programming_id: u64,
}

async fn start(
    State(state): State<SharedState>,
    Json(body): Json<StartRequest>,
) -> Result<Json<StartResponse>, ApiError> {
    let gateway: SocketAddrV4 = body
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request("gateway is not a host:port IPv4 address"))?;
    let new_address = parse_new_address(&body.address)?;
    let wait_seconds = body.wait_seconds.unwrap_or(DEFAULT_WAIT_SECONDS);
    if wait_seconds == 0 || wait_seconds > MAX_WAIT_SECONDS {
        return Err(ApiError::bad_request(format!(
            "waitSeconds must be between 1 and {MAX_WAIT_SECONDS}"
        )));
    }
    let authorisation =
        AddressProgrammingAuthorisation::for_hardware(new_address, &body.confirmation)
            .map_err(|e| ApiError::bad_request(format!("not written: {e}")))?;

    // Lock order download → programming → monitor → scan: a lock is only
    // ever awaited downward and `try_lock`ed upward, so no wait can form a
    // cycle. Held until the programming is registered.
    let mut programming = state.address_programming.lock().await;
    if crate::device_download_routes::download_in_progress(&state) {
        return Err(conflict(
            "a download to a device is running: the gateway serves one tunnel",
        ));
    }
    if let Some(existing) = programming.as_ref() {
        if existing.is_active() {
            return Err(conflict(format!(
                "programming {} is already running",
                existing.new_address()
            )));
        }
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
    let id = state
        .next_address_programming_id
        .fetch_add(1, Ordering::SeqCst);
    *programming = Some(AddressProgrammingSession::start(
        id,
        tunnel,
        new_address,
        authorisation,
        state.address_programming_timing,
        ButtonWait {
            give_up_after: Duration::from_secs(wait_seconds),
            pause_between_rounds: state.address_programming_pause,
        },
    ));
    drop(scan);
    drop(monitor);
    Ok(Json(StartResponse { programming_id: id }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusQuery {
    #[serde(default)]
    since: usize,
    programming_id: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusResponse {
    programming_id: u64,
    address: String,
    wait_seconds: u64,
    status: AddressProgrammingStatus,
    next_since: usize,
    events: Vec<AddressProgrammingEvent>,
}

fn with_session<R>(
    state: &SharedState,
    id: Option<u64>,
    f: impl FnOnce(&AddressProgrammingSession) -> Result<R, ApiError>,
) -> Result<R, ApiError> {
    // `try_lock`: a start holds this lock while it connects, and a poll
    // must not queue behind a gateway time-out.
    let Ok(guard) = state.address_programming.try_lock() else {
        return Err(conflict("a programming is being started"));
    };
    let Some(session) = guard.as_ref() else {
        return Err(ApiError::with_status(
            StatusCode::NOT_FOUND,
            "no individual-address programming yet",
        ));
    };
    if let Some(id) = id.filter(|id| *id != session.id()) {
        return Err(ApiError::with_status(
            StatusCode::NOT_FOUND,
            format!("programming {id} has been replaced"),
        ));
    }
    f(session)
}

async fn status(
    State(state): State<SharedState>,
    Query(query): Query<StatusQuery>,
) -> Result<Json<StatusResponse>, ApiError> {
    with_session(&state, query.programming_id, |session| {
        let (status, next_since, events) = session.snapshot_since(query.since);
        Ok(Json(StatusResponse {
            programming_id: session.id(),
            address: session.new_address().to_string(),
            wait_seconds: session.wait_seconds(),
            status,
            next_since,
            events,
        }))
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StopRequest {
    programming_id: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StopResponse {
    stopping: bool,
}

async fn stop(
    State(state): State<SharedState>,
    Json(body): Json<StopRequest>,
) -> Result<Json<StopResponse>, ApiError> {
    with_session(&state, Some(body.programming_id), |session| {
        match session.request_stop() {
            StopOutcome::WillStop => Ok(Json(StopResponse { stopping: true })),
            StopOutcome::TooLate => Err(conflict(
                "too late to stop: the device was found and MP §2.3 runs to its end, or it is over",
            )),
        }
    })
}

/// For the download, monitor and scan starts: whether a programming is
/// starting, waiting or programming. Never waits.
pub fn address_programming_in_progress(state: &SharedState) -> bool {
    match state.address_programming.try_lock() {
        Err(_) => true,
        Ok(guard) => guard
            .as_ref()
            .is_some_and(AddressProgrammingSession::is_active),
    }
}
