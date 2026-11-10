//! Route bus connections and group traffic through the single shared bus session.
//! `/api/bus/*` — T15 task 3: the HTTP layer over `bus.rs`'s session/buffer/
//! drain-task machinery (design spec `docs/superpowers/specs/
//! 2026-09-11-group-monitor-design.md` §4.3), plus T25's interface search.
//! Five routes, registered by [`bus_routes`] and merged into `app()` in
//! `lib.rs` next to `routes::project_routes()`/`fs_routes::fs_routes()`.
//!
//! A new module, not a section of `routes.rs`: `routes.rs` is already large
//! (1500+ lines) and entirely project/device/catalog-shaped; these
//! routes share nothing with it except the same `#[serde(rename_all =
//! "camelCase")]` DTO / `Result<Json<T>, ApiError>` idiom, which this module
//! follows exactly rather than inventing a variant of it. The design spec's
//! own §4.3 offers this as one of two acceptable placements ("a new
//! `bus_routes()` ... or a sibling module") — this is the sibling-module
//! choice.
//!
//! `AppState.bus_session` is a `tokio::sync::Mutex`, not a `std::sync::
//! Mutex` (see `domain.rs`'s doc comment on that field for why) — `/start`
//! and `/write` hold the guard across `BusSession::start`/`BusSession::send`'s
//! own `.await` on purpose: at most one session exists, so serializing
//! concurrent attempts through this one lock is correct, not a cost to
//! avoid. `/start` alone takes it with `try_lock` and refuses `gatewayBusy`
//! when it is held (WP-26) instead of queueing behind another holder.

use std::net::SocketAddrV4;
use std::time::Duration;

use axum::extract::{Query, RawQuery, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use knx_app::property_validation::System2Settlement;
use knx_core::scan::{ScanPlan, ScanPlanBuilder};
use knx_core::{GroupAddress, GroupAddressDpt, GroupAddressStyle, IndividualAddress};
use knx_net::{
    compare_with_project, ApplicationService, Destination, ProbeOutcome, ProbePolicy, ScanEstimate,
    ScannedRange,
};

use crate::bus::{
    self, decode_single, BusSession, BusSessionError, DecodeFailureReason, DecodedValue,
    GroupAddressContext, SessionStatus, TelegramRow,
};
use crate::bus_scan::{LineScanResult, LineScanSession, LineScanStatus, LineScanTarget};
use crate::domain;
use crate::errors::ApiError;
use crate::SharedState;

pub fn bus_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/bus/monitor/start", post(start_monitor))
        .route("/api/bus/monitor/start-serial", post(start_serial_monitor))
        .route("/api/bus/monitor/start-tcp", post(start_tcp_monitor))
        .route("/api/bus/monitor/stop", post(stop_monitor))
        .route("/api/bus/monitor/stop-bound", post(stop_bound_monitor))
        .route("/api/bus/monitor/telegrams", get(poll_telegrams))
        .route("/api/bus/monitor/flow-snapshot", get(flow_snapshot))
        .route("/api/bus/write", post(write_value))
        .route("/api/bus/write-bound", post(write_bound_value))
        .route("/api/bus/read", post(read_value))
        .route("/api/bus/discover", post(discover_interfaces))
        .route(
            "/api/bus/serial-candidates",
            get(crate::serial_candidates::list),
        )
        .route("/api/bus/scan/estimate", post(estimate_scan))
        .route("/api/bus/scan/start", post(start_scan))
        .route("/api/bus/scan/start-tcp", post(start_tcp_scan))
        .route("/api/bus/scan/results", get(poll_scan))
        .route("/api/bus/scan/cancel", post(cancel_scan))
        .route("/api/bus/scan/cancel-bound", post(cancel_bound_scan))
        .route("/api/bus/scan/comparison", get(scan_comparison))
        .route("/api/bus/scan/comparison-bound", get(scan_comparison_bound))
        .route("/api/bus/scan/reconcile", post(reconcile_scan))
        .route("/api/bus/scan/reconcile-bound", post(reconcile_scan_bound))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanRequest {
    gateway: Option<String>,
    area: u8,
    line: u8,
    first_device: u8,
    last_device: u8,
    #[serde(default)]
    excluded: Vec<String>,
    response_timeout_ms: u64,
    inter_probe_pause_ms: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanEstimateResponse {
    candidate_count: usize,
    omitted_addresses: Vec<String>,
    response_timeout_ms: u64,
    vacant_confirmations: u8,
    inter_probe_pause_ms: u64,
    worst_case_ms: u64,
}

fn build_scan(
    body: &ScanRequest,
) -> Result<(ScanPlan, ProbePolicy, ScanEstimate, ScannedRange), ApiError> {
    if body.response_timeout_ms == 0 {
        return Err(ApiError::bad_request(
            "responseTimeoutMs must be greater than zero",
        ));
    }
    if body.first_device == 0 || body.last_device == 0 {
        return Err(ApiError::bad_request(
            "device 0 is the line coupler address, not a device probe target",
        ));
    }
    let first = IndividualAddress::new(body.area, body.line, body.first_device)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let last = IndividualAddress::new(body.area, body.line, body.last_device)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let mut builder = ScanPlanBuilder::new();
    for value in &body.excluded {
        let address = value.parse::<IndividualAddress>().map_err(|error| {
            ApiError::bad_request(format!("invalid excluded address {value:?}: {error}"))
        })?;
        builder = builder.exclude(address);
    }
    let plan = builder
        .range(first, last)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let policy = ProbePolicy::new(
        Duration::from_millis(body.response_timeout_ms),
        ProbePolicy::default().vacant_confirmations(),
        Duration::from_millis(body.inter_probe_pause_ms),
    )
    .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let estimate = ScanEstimate::new(&plan, &policy);
    let range = ScannedRange {
        area: body.area,
        line: body.line,
        first_device: body.first_device,
        last_device: body.last_device,
    };
    Ok((plan, policy, estimate, range))
}

fn estimate_response(
    plan: &ScanPlan,
    policy: &ProbePolicy,
    estimate: ScanEstimate,
) -> ScanEstimateResponse {
    ScanEstimateResponse {
        candidate_count: estimate.candidate_count(),
        omitted_addresses: plan.omitted().iter().map(ToString::to_string).collect(),
        response_timeout_ms: policy.response_timeout().as_millis() as u64,
        vacant_confirmations: policy.vacant_confirmations(),
        inter_probe_pause_ms: policy.inter_probe_pause().as_millis() as u64,
        worst_case_ms: estimate.worst_case().as_millis() as u64,
    }
}

async fn estimate_scan(
    Json(body): Json<ScanRequest>,
) -> Result<Json<ScanEstimateResponse>, ApiError> {
    let (plan, policy, estimate, _) = build_scan(&body)?;
    Ok(Json(estimate_response(&plan, &policy, estimate)))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanStartResponse {
    session_id: u64,
    server_incarnation: String,
    transport: &'static str,
    skipped_frames: Option<u64>,
    estimate: ScanEstimateResponse,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TcpScanRequest {
    gateway: String,
    area: u8,
    line: u8,
    first_device: u8,
    last_device: u8,
    excluded: Vec<String>,
    response_timeout_ms: u64,
    inter_probe_pause_ms: u64,
}

async fn start_tcp_scan(
    State(state): State<SharedState>,
    body: Result<Json<TcpScanRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<ScanStartResponse>, ApiError> {
    let Json(body) = body
        .map_err(|_| ApiError::bad_request("TCP scan requires exactly the complete scan plan"))?;
    let gateway = parse_tcp_gateway(&body.gateway)?;
    let body = ScanRequest {
        gateway: Some(body.gateway),
        area: body.area,
        line: body.line,
        first_device: body.first_device,
        last_device: body.last_device,
        excluded: body.excluded,
        response_timeout_ms: body.response_timeout_ms,
        inter_probe_pause_ms: body.inter_probe_pause_ms,
    };
    start_scan_owned(state, body, gateway, knx_net::TunnelTransport::Tcp).await
}

fn scan_reservation_busy() -> ApiError {
    crate::tunnel_holder::gateway_busy(
        crate::tunnel_holder::TunnelHolder::Connecting {
            session: "lineScan",
        },
        "a line scan reservation is held; nothing was started or cancelled",
    )
}

async fn start_scan(
    State(state): State<SharedState>,
    Json(body): Json<ScanRequest>,
) -> Result<Json<ScanStartResponse>, ApiError> {
    // Goal A3: no new tunnel once a stop has begun (KL-163).
    state.stop_admission.admit()?;
    let gateway = body
        .gateway
        .as_deref()
        .ok_or_else(|| ApiError::bad_request("gateway is required"))?
        .parse::<SocketAddrV4>()
        .map_err(|_| ApiError::bad_request("gateway is not a host:port IPv4 address"))?;
    start_scan_owned(state, body, gateway, knx_net::TunnelTransport::Udp).await
}

async fn start_scan_owned(
    state: SharedState,
    body: ScanRequest,
    gateway: SocketAddrV4,
    transport: knx_net::TunnelTransport,
) -> Result<Json<ScanStartResponse>, ApiError> {
    let (plan, policy, estimate, range) = build_scan(&body)?;
    let response_estimate = estimate_response(&plan, &policy, estimate);
    let mut guard = state
        .line_scan_session
        .clone()
        .try_lock_owned()
        .map_err(|_| scan_reservation_busy())?;
    state.stop_admission.admit()?;
    if system2_running(&state) {
        return Err(session_lock_busy(&state, true));
    }
    // Checked while holding this session's lock, never before it: a
    // download start holds its own lock and then waits for this one, so
    // this `try_lock` sees it (ADR-0045 §4).
    if let Some(holder) = crate::tunnel_holder::download_holder(&state) {
        return Err(crate::tunnel_holder::gateway_busy(
            holder,
            "a download to a device is running: the gateway serves one tunnel",
        ));
    }
    if let Some(holder) = crate::tunnel_holder::programming_holder(&state) {
        return Err(crate::tunnel_holder::gateway_busy(
            holder,
            "an individual-address programming is running: the gateway serves one tunnel",
        ));
    }
    if let Some(existing) = guard.as_mut() {
        if let Some(holder) = crate::tunnel_holder::scan_holder_of(existing) {
            return Err(crate::tunnel_holder::gateway_busy(
                holder,
                format!("line scan session {} is already running", existing.id()),
            ));
        }
    }
    let (done, answer) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let result = async {
            // Admission may close after handler reservation but before this task runs.
            state.stop_admission.admit()?;
            if let Some(existing) = guard.as_mut() {
                existing.cancel().await;
            }
            state.stop_admission.admit()?;
            if system2_running(&state) {
                return Err(session_lock_busy(&state, true));
            }
            let id = state
                .next_line_scan_session_id
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let mut session = LineScanSession::start_with_transport(
                id,
                LineScanTarget {
                    range,
                    gateway,
                    transport,
                },
                state.connector.as_ref(),
                plan,
                policy,
                crate::operation_recordings::start(
                    &state,
                    knx_app::operation_recording::OperationStart::new(
                        knx_app::operation_recording::OperationKind::LineScan,
                    )
                    .target(serde_json::json!({
                        "area": range.area, "line": range.line,
                        "firstDevice": range.first_device, "lastDevice": range.last_device,
                    }))
                    .gateway(gateway)
                    .related("lineScanSessionId", id),
                ),
            )
            .await
            .map_err(|e| connect_error_to_api_error(gateway, e))?;
            // Shutdown during connect still owns and releases its late tunnel.
            if let Err(error) = state.stop_admission.admit() {
                session.cancel().await;
                *guard = Some(session);
                return Err(error);
            }
            let snapshot = session.snapshot(0);
            let response = Json(ScanStartResponse {
                session_id: id,
                server_incarnation: state.server_incarnation.clone(),
                transport: session.transport().as_str(),
                skipped_frames: snapshot.skipped_frames,
                estimate: response_estimate,
            });
            *guard = Some(session);
            Ok(response)
        }
        .await;
        // Keep this exact reservation through disposal of an unclaimed successful start.
        if let Err(Ok(_)) = done.send(result) {
            if let Some(session) = guard.as_mut() {
                session.cancel().await;
            }
        }
    });
    answer.await.unwrap_or_else(|_| {
        Err(ApiError::with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "line scan start owner ended without an answer",
        ))
    })
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanSessionQuery {
    session_id: u64,
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum ScanOutcomeDto {
    Occupied { mask_version: Option<u16> },
    OccupiedBusy,
    OccupiedSilent,
    Vacant,
    Indeterminate,
    SelfAddress,
}

impl From<ProbeOutcome> for ScanOutcomeDto {
    fn from(value: ProbeOutcome) -> Self {
        match value {
            ProbeOutcome::Occupied { mask_version } => Self::Occupied { mask_version },
            ProbeOutcome::OccupiedBusy => Self::OccupiedBusy,
            ProbeOutcome::OccupiedSilent => Self::OccupiedSilent,
            ProbeOutcome::Vacant => Self::Vacant,
            ProbeOutcome::Indeterminate => Self::Indeterminate,
            ProbeOutcome::SelfAddress => Self::SelfAddress,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResultDto {
    address: String,
    outcome: ScanOutcomeDto,
}

impl From<LineScanResult> for ScanResultDto {
    fn from(value: LineScanResult) -> Self {
        Self {
            address: value.address.to_string(),
            outcome: value.outcome.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanResultsResponse {
    session_id: u64,
    server_incarnation: String,
    transport: &'static str,
    skipped_frames: Option<u64>,
    disconnect_confirmed: Option<bool>,
    cleanup_error: Option<String>,
    status: &'static str,
    error: Option<String>,
    next_since: usize,
    completed_count: usize,
    total_count: usize,
    omitted_addresses: Vec<String>,
    excluded_addresses: Vec<String>,
    results: Vec<ScanResultDto>,
}

fn scan_status(status: LineScanStatus) -> (&'static str, Option<String>) {
    match status {
        LineScanStatus::Running => ("running", None),
        LineScanStatus::Completed => ("completed", None),
        LineScanStatus::Cancelled => ("cancelled", None),
        LineScanStatus::Failed(error) => ("failed", Some(error)),
    }
}

fn scan_response(
    session: &LineScanSession,
    since: usize,
    incarnation: &str,
) -> ScanResultsResponse {
    let snapshot = session.snapshot(since);
    let (status, error) = scan_status(snapshot.status);
    let next_since = snapshot.next_since;
    ScanResultsResponse {
        session_id: session.id(),
        server_incarnation: incarnation.to_owned(),
        transport: session.transport().as_str(),
        skipped_frames: snapshot.skipped_frames,
        disconnect_confirmed: snapshot.disconnect_confirmed,
        cleanup_error: snapshot.cleanup_error,
        status,
        error,
        next_since,
        completed_count: next_since,
        total_count: session.total_count(),
        omitted_addresses: session.omitted().iter().map(ToString::to_string).collect(),
        excluded_addresses: session.excluded().iter().map(ToString::to_string).collect(),
        results: snapshot.results.into_iter().map(Into::into).collect(),
    }
}

fn scan_changed() -> ApiError {
    ApiError::conflict(
        "lineScanSessionChanged",
        "the named line scan is not held by this server; nothing was changed",
    )
}

fn check_scan_binding(
    state: &SharedState,
    session: Option<&LineScanSession>,
    bound: &BoundSession,
) -> Result<(), ApiError> {
    if bound.server_incarnation != state.server_incarnation
        || !session.is_some_and(|held| held.id() == bound.session_id)
    {
        return Err(scan_changed());
    }
    Ok(())
}

fn require_legacy_udp(session: &LineScanSession) -> Result<(), ApiError> {
    if session.transport() == knx_net::TunnelTransport::Tcp {
        return Err(ApiError::conflict(
            "lineScanSessionBindingRequired",
            "TCP scans require the strict identity-bound route; nothing was changed",
        ));
    }
    Ok(())
}

async fn poll_scan(
    State(state): State<SharedState>,
    RawQuery(raw): RawQuery,
) -> Result<Json<ScanResultsResponse>, ApiError> {
    let mut since = None;
    let mut binding = Vec::new();
    let mut has_incarnation = false;
    for pair in raw.as_deref().unwrap_or("").split('&') {
        if pair.is_empty() {
            if raw.as_deref().is_none_or(str::is_empty) {
                continue;
            }
            return Err(ApiError::bad_request("empty scan query field"));
        }
        let (name, value) = pair
            .split_once('=')
            .ok_or_else(|| ApiError::bad_request("scan query field needs a value"))?;
        if name == "since" {
            if since.is_some()
                || value.is_empty()
                || (value.len() > 1 && value.starts_with('0'))
                || !value.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(ApiError::bad_request(
                    "since must be one canonical nonnegative integer",
                ));
            }
            since = Some(
                value
                    .parse::<usize>()
                    .map_err(|_| ApiError::bad_request("since out of range"))?,
            );
        } else {
            has_incarnation |= name == "serverIncarnation";
            binding.push(pair);
        }
    }
    let legacy = !binding.is_empty() && !has_incarnation;
    let bound = if binding.is_empty() {
        None
    } else {
        let mut query = binding.join("&");
        if legacy {
            query.push_str(&format!("&serverIncarnation={}", state.server_incarnation));
        }
        Some(parse_bound_session(Some(&query))?)
    };
    let guard = state
        .line_scan_session
        .try_lock()
        .map_err(|_| scan_reservation_busy())?;
    if let Some(bound) = &bound {
        check_scan_binding(&state, guard.as_ref(), bound)?;
    }
    let session = guard.as_ref().ok_or_else(|| {
        ApiError::with_status(StatusCode::NOT_FOUND, "no line scan session exists")
    })?;
    if legacy {
        require_legacy_udp(session)?;
    }
    Ok(Json(scan_response(
        session,
        since.unwrap_or(0),
        &state.server_incarnation,
    )))
}

async fn cancel_scan(
    State(state): State<SharedState>,
    Query(query): Query<ScanSessionQuery>,
) -> Result<Json<ScanResultsResponse>, ApiError> {
    let guard = state
        .line_scan_session
        .clone()
        .try_lock_owned()
        .map_err(|_| scan_reservation_busy())?;
    let session = guard.as_ref().ok_or_else(scan_changed)?;
    if query.session_id != session.id() {
        return Err(scan_changed());
    }
    require_legacy_udp(session)?;
    cancel_scan_owned(guard, state.server_incarnation.clone()).await
}

async fn cancel_bound_scan(
    State(state): State<SharedState>,
    RawQuery(query): RawQuery,
) -> Result<Json<ScanResultsResponse>, ApiError> {
    let bound = parse_scan_bound(query.as_deref())?;
    let guard = state
        .line_scan_session
        .clone()
        .try_lock_owned()
        .map_err(|_| scan_reservation_busy())?;
    check_scan_binding(&state, guard.as_ref(), &bound)?;
    cancel_scan_owned(guard, state.server_incarnation.clone()).await
}

fn parse_scan_bound(query: Option<&str>) -> Result<BoundSession, ApiError> {
    if query.is_none_or(|q| q.split('&').any(str::is_empty)) {
        return Err(ApiError::bad_request(
            "exact scan session binding is required",
        ));
    }
    parse_bound_session(query)
}

async fn cancel_scan_owned(
    mut guard: tokio::sync::OwnedMutexGuard<Option<LineScanSession>>,
    incarnation: String,
) -> Result<Json<ScanResultsResponse>, ApiError> {
    let (done, answer) = tokio::sync::oneshot::channel();
    tokio::spawn(async move {
        let session = guard.as_mut().expect("matched held scan");
        session.cancel().await;
        let response = scan_response(session, 0, &incarnation);
        let _ = done.send(Ok(Json(response)));
    });
    answer.await.unwrap_or_else(|_| {
        Err(ApiError::with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "line scan cancel owner ended without an answer",
        ))
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ScanComparisonResponse {
    unexpected: Vec<String>,
    missing: Vec<String>,
    excluded_in_project: Vec<String>,
}

async fn scan_comparison(
    State(state): State<SharedState>,
    Query(query): Query<ScanSessionQuery>,
) -> Result<Json<ScanComparisonResponse>, ApiError> {
    let guard = state
        .line_scan_session
        .try_lock()
        .map_err(|_| scan_reservation_busy())?;
    let session = guard.as_ref().ok_or_else(scan_changed)?;
    if query.session_id != session.id() {
        return Err(scan_changed());
    }
    require_legacy_udp(session)?;
    scan_comparison_impl(&state, session)
}

async fn scan_comparison_bound(
    State(state): State<SharedState>,
    RawQuery(query): RawQuery,
) -> Result<Json<ScanComparisonResponse>, ApiError> {
    let bound = parse_scan_bound(query.as_deref())?;
    let guard = state
        .line_scan_session
        .try_lock()
        .map_err(|_| scan_reservation_busy())?;
    check_scan_binding(&state, guard.as_ref(), &bound)?;
    scan_comparison_impl(&state, guard.as_ref().expect("matched scan"))
}

fn scan_comparison_impl(
    state: &SharedState,
    session: &LineScanSession,
) -> Result<Json<ScanComparisonResponse>, ApiError> {
    let (status, _, results) = session.snapshot_since(0);
    if status != LineScanStatus::Completed {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "only a completed line scan can be compared with the project",
        ));
    }

    let project = state.project.lock().expect("state mutex poisoned");
    let project = project
        .as_ref()
        .ok_or_else(|| ApiError::with_status(StatusCode::CONFLICT, "no project open"))?;
    let project_addresses: Vec<_> = project
        .devices
        .iter()
        .filter_map(|device| device.address)
        .collect();
    let excluded = session.excluded().iter().copied().collect();
    let evidence: Vec<_> = results
        .into_iter()
        .map(|result| (result.address, result.outcome))
        .collect();
    let comparison =
        compare_with_project(&session.range(), &evidence, &excluded, &project_addresses);

    Ok(Json(ScanComparisonResponse {
        unexpected: comparison
            .unexpected
            .into_iter()
            .map(|address| address.to_string())
            .collect(),
        missing: comparison
            .missing
            .into_iter()
            .map(|address| address.to_string())
            .collect(),
        excluded_in_project: comparison
            .excluded_in_project
            .into_iter()
            .map(|address| address.to_string())
            .collect(),
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanReconcileBody {
    session_id: u64,
    #[serde(default)]
    unexpected: Vec<String>,
    #[serde(default)]
    missing: Vec<String>,
}

fn parse_scan_selection(
    values: Vec<String>,
    field: &str,
) -> Result<Vec<IndividualAddress>, ApiError> {
    values
        .into_iter()
        .map(|value| {
            value.parse::<IndividualAddress>().map_err(|error| {
                ApiError::bad_request(format!("invalid {field} address {value:?}: {error}"))
            })
        })
        .collect()
}

async fn reconcile_scan(
    State(state): State<SharedState>,
    Json(body): Json<ScanReconcileBody>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let guard = state
        .line_scan_session
        .try_lock()
        .map_err(|_| scan_reservation_busy())?;
    let session = guard.as_ref().ok_or_else(scan_changed)?;
    if body.session_id != session.id() {
        return Err(scan_changed());
    }
    require_legacy_udp(session)?;
    reconcile_scan_impl(&state, session, body.unexpected, body.missing)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoundScanReconcileBody {
    unexpected: Vec<String>,
    missing: Vec<String>,
}

async fn reconcile_scan_bound(
    State(state): State<SharedState>,
    RawQuery(query): RawQuery,
    body: Result<Json<BoundScanReconcileBody>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let bound = parse_scan_bound(query.as_deref())?;
    let Json(body) = body.map_err(|_| {
        ApiError::bad_request(
            "bound reconciliation requires exactly unexpected and missing selections",
        )
    })?;
    let guard = state
        .line_scan_session
        .try_lock()
        .map_err(|_| scan_reservation_busy())?;
    check_scan_binding(&state, guard.as_ref(), &bound)?;
    reconcile_scan_impl(
        &state,
        guard.as_ref().expect("matched scan"),
        body.unexpected,
        body.missing,
    )
}

fn reconcile_scan_impl(
    state: &SharedState,
    session: &LineScanSession,
    unexpected: Vec<String>,
    missing: Vec<String>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    let (status, _, results) = session.snapshot_since(0);
    if status != LineScanStatus::Completed {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "only a completed line scan can be reconciled",
        ));
    }

    let unexpected = parse_scan_selection(unexpected, "unexpected")?;
    let missing = parse_scan_selection(missing, "missing")?;
    let evidence: Vec<_> = results
        .into_iter()
        .map(|result| (result.address, result.outcome))
        .collect();
    let excluded = session.excluded().iter().copied().collect();
    let tree = domain::reconcile_scan_impl(
        state,
        session.range(),
        &evidence,
        &excluded,
        unexpected,
        missing,
    )
    .map_err(ApiError::bad_request)?;
    Ok(Json(tree))
}

/// Maps a [`BusSessionError`] to its HTTP status for the three mutating
/// routes (`/start`, `/stop`, `/write`), where "no active session" is a
/// `409` (design spec §4.3: all three mutate session state, so a missing
/// session is a conflict with what the caller asked for, not a not-found —
/// the opposite reading applies only to `GET /telegrams`, which never
/// constructs a [`BusSessionError`] at all, handled separately below).
/// For the two starts that open a tunnel: a transport failure there is the
/// gateway refusing or not answering the connect, named for the client
/// (issue #10); everything else maps as below.
fn connect_error_to_api_error(gateway: SocketAddrV4, error: BusSessionError) -> ApiError {
    match error {
        BusSessionError::Transport(_) => {
            crate::tunnel_holder::gateway_connect_failed(gateway, &error)
        }
        other => session_error_to_api_error(other),
    }
}

fn session_error_to_api_error(error: BusSessionError) -> ApiError {
    match error {
        BusSessionError::Transport(e) => {
            ApiError::with_status(StatusCode::BAD_GATEWAY, e.to_string())
        }
        BusSessionError::PassiveSerial => ApiError::conflict("passiveSerial", error.to_string()),
        BusSessionError::AlreadyActive | BusSessionError::NoActiveSession => {
            ApiError::with_status(StatusCode::CONFLICT, error.to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// POST /api/bus/monitor/start
// ---------------------------------------------------------------------------

/// Whether an admitted System-2 operation owns the tunnel right now.
fn system2_running(state: &SharedState) -> bool {
    matches!(
        state.device_system2.control.snapshot().settlement,
        System2Settlement::Running { .. }
    )
}

/// The `gatewayBusy` answer for a held (or System-2 owned) session lock.
/// System-2 is named first; the other helpers never wait either and keep a
/// known download or management holder before an unnamed session lock.
fn session_lock_busy(state: &SharedState, system2_running: bool) -> ApiError {
    let holder = if system2_running {
        crate::tunnel_holder::TunnelHolder::Connecting {
            session: "system2Operation",
        }
    } else {
        crate::tunnel_holder::download_holder(state)
            .or_else(|| crate::tunnel_holder::programming_holder(state))
            .unwrap_or(crate::tunnel_holder::TunnelHolder::Connecting {
                session: "busMonitorOrGroupWrite",
            })
    };
    crate::tunnel_holder::gateway_busy(
        holder,
        "the gateway's tunnel is reserved right now: the gateway serves one tunnel",
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StartRequest {
    gateway: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StartResponse {
    session_id: u64,
    server_incarnation: String,
    assigned_address: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    serial: Option<crate::bus_serial::SerialTarget>,
    /// The IP transport the session really runs on; absent for serial.
    #[serde(skip_serializing_if = "Option::is_none")]
    transport: Option<&'static str>,
    /// TCP only: the cumulative skipped-frame count at start (see `StopResponse`).
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped_frames: Option<u64>,
}

async fn start_monitor(
    State(state): State<SharedState>,
    Json(body): Json<StartRequest>,
) -> Result<Json<StartResponse>, ApiError> {
    // Goal A3: no new tunnel once a stop has begun (KL-163).
    state.stop_admission.admit()?;
    let gateway: SocketAddrV4 = body
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request(format!("not a gateway address: {:?}", body.gateway)))?;

    start_monitor_target(state, MonitorTarget::Ip(gateway)).await
}

enum MonitorTarget {
    Ip(SocketAddrV4),
    Tcp(SocketAddrV4),
    Serial(crate::bus_serial::SerialTarget),
}

/// Exactly a gateway; unknown, repeated or missing fields are refused.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct TcpStartRequest {
    gateway: String,
}

/// A unicast IPv4 `host:port` with a nonzero port, the CLI's rule for `--gateway`.
fn parse_tcp_gateway(value: &str) -> Result<SocketAddrV4, ApiError> {
    let refuse = || {
        ApiError::bad_request(format!(
            "TCP start requires a unicast IPv4 host:port with a nonzero port, got {value:?}; nothing was opened"
        ))
    };
    let gateway: SocketAddrV4 = value.parse().map_err(|_| refuse())?;
    if gateway.port() == 0
        || gateway.ip().is_unspecified()
        || gateway.ip().is_broadcast()
        || gateway.ip().is_multicast()
    {
        return Err(refuse());
    }
    Ok(gateway)
}

/// The explicit TCP start. A separate strict route, not a field on the UDP
/// start: a server without it answers 404 and the client never connects.
async fn start_tcp_monitor(
    State(state): State<SharedState>,
    body: Result<Json<TcpStartRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<StartResponse>, ApiError> {
    let Json(body) = body.map_err(|_| {
        ApiError::bad_request("TCP start requires exactly a gateway host:port; nothing was opened")
    })?;
    let gateway = parse_tcp_gateway(&body.gateway)?;
    state.stop_admission.admit()?;
    // The connect and its cleanup run in a task of their own: a dropped
    // request must not release the reservation while a socket is still
    // being opened or closed.
    let (done, answer) = tokio::sync::oneshot::channel();
    let task_state = state.clone();
    tokio::spawn(async move {
        let result = start_monitor_target(task_state.clone(), MonitorTarget::Tcp(gateway)).await;
        if let Err(Ok(Json(unclaimed))) = done.send(result) {
            release_unclaimed(&task_state, unclaimed.session_id).await;
        }
    });
    answer.await.unwrap_or_else(|_| {
        Err(ApiError::with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the TCP start task ended without an answer",
        ))
    })
}

/// Stops a session nobody is waiting for, holding the reservation until the
/// tunnel is released. A session already replaced or stopped is left alone.
async fn release_unclaimed(state: &SharedState, session_id: u64) {
    let mut guard = state.bus_session.lock().await;
    if guard.as_ref().is_some_and(|held| held.id() == session_id) {
        if let Some(session) = guard.take() {
            session.stop().await;
        }
    }
}

/// Finishes the stop of the TCP session held under `guard` in a task of its own. The guard
/// moves into the task, so the identity the caller checked, the removal and the cleanup all
/// happen under the one reservation, which is released only after the tunnel is, even when
/// the HTTP request is dropped meanwhile.
async fn stop_tcp_session(
    mut guard: tokio::sync::OwnedMutexGuard<Option<BusSession>>,
    state: &SharedState,
) -> Result<Json<StopResponse>, ApiError> {
    let (done, answer) = tokio::sync::oneshot::channel();
    let incarnation = state.server_incarnation.clone();
    tokio::spawn(async move {
        let result = match guard.take() {
            Some(session) => {
                let session_id = session.id();
                Ok(stop_response(
                    &incarnation,
                    session_id,
                    session.stop().await,
                ))
            }
            None => Err(session_error_to_api_error(BusSessionError::NoActiveSession)),
        };
        drop(guard);
        let _ = done.send(result);
    });
    answer.await.unwrap_or_else(|_| {
        Err(ApiError::with_status(
            StatusCode::INTERNAL_SERVER_ERROR,
            "the TCP stop task ended without an answer",
        ))
    })
}

fn stop_response(
    server_incarnation: &str,
    session_id: u64,
    summary: crate::bus::BusSessionSummary,
) -> Json<StopResponse> {
    Json(StopResponse {
        session_id,
        server_incarnation: server_incarnation.to_owned(),
        telegram_count: summary.telegram_count,
        dropped_count: summary.dropped_count,
        warning: summary.drain_panic,
        skipped_frames: summary.skipped_frames,
    })
}

async fn start_serial_monitor(
    State(state): State<SharedState>,
    body: Result<Json<crate::bus_serial::SerialTarget>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<StartResponse>, ApiError> {
    let Json(target) = body.map_err(|_| {
        ApiError::bad_request(
            "serial start requires exactly path, source and mode (passive or active)",
        )
    })?;
    target.validate().map_err(ApiError::bad_request)?;
    if !cfg!(target_os = "linux") {
        return Err(ApiError::with_details(
            StatusCode::NOT_IMPLEMENTED,
            "serialUnsupported",
            "serial TP-UART requires a Linux server",
            serde_json::Map::new(),
        ));
    }
    state.stop_admission.admit()?;
    start_monitor_target(state, MonitorTarget::Serial(target)).await
}

async fn start_monitor_target(
    state: SharedState,
    target: MonitorTarget,
) -> Result<Json<StartResponse>, ApiError> {
    // WP-26: a start never queues behind a held reservation (a long management
    // or System-2 run would turn it into a late, surprising connect). It is
    // refused at once, before any connector call. The System-2 check follows
    // the `try_lock` so an operation admitted before it holds its locks is
    // still seen; an unknown holder of the lock stays unnamed.
    let held = state.bus_session.try_lock();
    let system2_running = system2_running(&state);
    if system2_running {
        return Err(session_lock_busy(&state, system2_running));
    }
    let mut guard = held.map_err(|_| session_lock_busy(&state, system2_running))?;
    // The handler admitted before this task (or the lock) was reached; an orderly stop may
    // have closed admission since. Under the reservation, before any connector call, a
    // closed admission means nothing opens: the stop's release then cannot miss a late session.
    state.stop_admission.admit()?;
    // Checked while holding this session's lock, never before it: a
    // download start holds its own lock and then waits for this one, so
    // this `try_lock` sees it (ADR-0045 §4).
    if let Some(holder) = crate::tunnel_holder::download_holder(&state) {
        return Err(crate::tunnel_holder::gateway_busy(
            holder,
            "a download to a device is running: the gateway serves one tunnel",
        ));
    }
    if let Some(holder) = crate::tunnel_holder::programming_holder(&state) {
        return Err(crate::tunnel_holder::gateway_busy(
            holder,
            "an individual-address programming is running: the gateway serves one tunnel",
        ));
    }
    if let Some(existing) = guard.as_ref() {
        // Design spec §4.3 / brief item 2: the `409` body must *name* the
        // existing session, not just say "one exists": in the text (id and
        // gateway) and, since issue #10, as the structured `holder`.
        return Err(crate::tunnel_holder::gateway_busy(
            crate::tunnel_holder::monitor_holder_of(existing),
            match existing.gateway() {
                Some(gateway) => format!(
                    "a monitor session is already active (id {}, gateway {})",
                    existing.id(),
                    gateway
                ),
                None => format!(
                    "a serial monitor session is already active (id {})",
                    existing.id()
                ),
            },
        ));
    }

    // Build an owned `GroupAddressContext` snapshot from whatever project is
    // open right now, then drop the project lock before `BusSession::start`
    // — `knx_core::Project` does not derive `Clone`, and a
    // `std::sync::MutexGuard` cannot be held across that call's `.await`
    // (not `Send`, and holding the whole app's project mutex for the
    // duration of a real gateway connect would stall every other
    // project-touching route for no reason connected to this session). See
    // `bus.rs`'s `BusSession::start` doc comment — this is exactly the
    // pattern it asks callers to follow.
    let ctx = {
        let project_guard = state.project.lock().expect("project mutex poisoned");
        GroupAddressContext::from_project(project_guard.as_ref())
    };

    let id = state
        .next_bus_session_id
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);

    let session = match target {
        MonitorTarget::Ip(gateway) => BusSession::start(id, gateway, state.connector.as_ref(), ctx)
            .await
            .map_err(|error| connect_error_to_api_error(gateway, error))?,
        MonitorTarget::Tcp(gateway) => {
            BusSession::start_tcp(id, gateway, state.connector.as_ref(), ctx)
                .await
                .map_err(|error| connect_error_to_api_error(gateway, error))?
        }
        MonitorTarget::Serial(target) => {
            let active = target.mode == crate::bus_serial::SerialMode::Active;
            BusSession::start_serial(id, target, state.connector.as_ref(), ctx)
                .await
                .map_err(|error| {
                    ApiError::with_details(
                        StatusCode::BAD_GATEWAY,
                        "serialConnectFailed",
                        error.to_string(),
                        serde_json::Map::from_iter([(
                            "activeAutoAckStateUncertain".into(),
                            active.into(),
                        )]),
                    )
                })?
        }
    };

    let assigned_address = session.assigned_address().to_string();
    let serial = session.serial().cloned();
    let transport = session.transport().map(knx_net::TunnelTransport::as_str);
    let skipped_frames = session.skipped_frames();
    *guard = Some(session);

    Ok(Json(StartResponse {
        session_id: id,
        server_incarnation: state.server_incarnation.clone(),
        assigned_address,
        serial,
        transport,
        skipped_frames,
    }))
}

// ---------------------------------------------------------------------------
// POST /api/bus/monitor/stop
// ---------------------------------------------------------------------------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StopResponse {
    session_id: u64,
    server_incarnation: String,
    telegram_count: usize,
    dropped_count: u64,
    /// Drain panic/cancellation or serial cleanup error; omitted on clean cleanup.
    /// A serial error can leave the auto-ACK state unknown. The existing
    /// warning field preserves that uncertainty. Omitted in the ordinary case
    /// (`skip_serializing_if`). The additive `serverIncarnation` binds the
    /// reusable numeric id to this process lifetime. `200`, not `500`: the
    /// stop itself genuinely succeeded (the
    /// session is gone, the tally is accurate) — only the teardown that
    /// followed misbehaved. See `BusSession::stop`'s doc comment for the
    /// full reasoning and the task report for why `500` was rejected.
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
    /// TCP only: cumulative frames the stream decoder skipped (unknown service or above the
    /// receive cap), sampled just before the DISCONNECT_REQUEST. Frames, not telegram rows,
    /// and not part of `droppedCount`; those skipped during cleanup are not covered.
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped_frames: Option<u64>,
}

async fn stop_monitor(State(state): State<SharedState>) -> Result<Json<StopResponse>, ApiError> {
    let mut guard = state
        .bus_session
        .clone()
        .try_lock_owned()
        .map_err(|_| session_lock_busy(&state, system2_running(&state)))?;
    if guard.as_ref().is_some_and(|held| held.is_tcp()) {
        return stop_tcp_session(guard, &state).await;
    }
    let Some(session) = guard.take() else {
        return Err(session_error_to_api_error(BusSessionError::NoActiveSession));
    };
    let session_id = session.id();
    let serial = session.serial().is_some();
    // Serial owns the reservation through reset/disconnect, even when cleanup fails.
    // IP retains its existing release-before-disconnect behavior.
    if !serial {
        drop(guard);
    }
    let summary = session.stop().await;
    Ok(stop_response(
        &state.server_incarnation,
        session_id,
        summary,
    ))
}

// ---------------------------------------------------------------------------
// POST /api/bus/monitor/stop-bound?sessionId=<id>&serverIncarnation=<token>
// ---------------------------------------------------------------------------

/// `kind` of the `409` when the named session is not the one held (WP-26).
const BUS_SESSION_CHANGED: &str = "busSessionChanged";

/// The session a bound stop names: both fields, each exactly once, nothing
/// else, the id in canonical decimal. Nothing is defaulted or normalised.
#[derive(Debug, PartialEq, Eq)]
struct BoundSession {
    session_id: u64,
    server_incarnation: String,
}

fn parse_bound_session(query: Option<&str>) -> Result<BoundSession, ApiError> {
    let refuse = |why: &str| ApiError::bad_request(format!("bound stop refused: {why}"));
    let mut session_id = None;
    let mut incarnation = None;
    for pair in query
        .unwrap_or("")
        .split('&')
        .filter(|pair| !pair.is_empty())
    {
        let (name, value) = pair
            .split_once('=')
            .ok_or_else(|| refuse("a field has no value"))?;
        let slot = match name {
            "sessionId" => &mut session_id,
            "serverIncarnation" => &mut incarnation,
            _ => return Err(refuse("unknown query field")),
        };
        if slot.replace(value).is_some() {
            return Err(refuse("a field is repeated"));
        }
    }
    let (Some(id), Some(incarnation)) = (session_id, incarnation) else {
        return Err(refuse("sessionId and serverIncarnation are both required"));
    };
    let canonical = id == "0" || (!id.starts_with('0') && !id.is_empty());
    if !canonical || !id.bytes().all(|b| b.is_ascii_digit()) {
        return Err(refuse("sessionId must be a canonical decimal integer"));
    }
    let session_id = id
        .parse::<u64>()
        .map_err(|_| refuse("sessionId is out of range"))?;
    // The incarnation is unpadded base64url: no byte needs percent-encoding,
    // so an encoded or empty value cannot be one this server ever issued.
    let url_safe = |b: u8| b.is_ascii_alphanumeric() || b == b'-' || b == b'_';
    if incarnation.is_empty() || !incarnation.bytes().all(url_safe) {
        return Err(refuse("serverIncarnation is malformed"));
    }
    Ok(BoundSession {
        session_id,
        server_incarnation: incarnation.to_owned(),
    })
}

/// Stops only the named session, and never waits for the session lock.
///
/// Order: the identity is decoded first, so a malformed request has no
/// effect and never looks at a lock. Then, like `/start`, System-2 and a
/// held lock answer `gatewayBusy` at once (no late stop after a queue).
/// Only under the lock is the held session compared; a mismatch or an
/// empty slot is `409 busSessionChanged` and the slot stays as it is. Stop
/// admission is not consulted: it refuses new bus jobs, while releasing a
/// tunnel is what the orderly shutdown does itself, as the legacy stop does.
async fn stop_bound_monitor(
    State(state): State<SharedState>,
    RawQuery(query): RawQuery,
) -> Result<Json<StopResponse>, ApiError> {
    let bound = parse_bound_session(query.as_deref())?;
    let held = state.bus_session.clone().try_lock_owned();
    let system2_running = system2_running(&state);
    if system2_running {
        return Err(session_lock_busy(&state, system2_running));
    }
    let mut guard = held.map_err(|_| session_lock_busy(&state, system2_running))?;
    let matches = bound.server_incarnation == state.server_incarnation
        && guard
            .as_ref()
            .is_some_and(|session| session.id() == bound.session_id);
    if !matches {
        return Err(ApiError::conflict(
            BUS_SESSION_CHANGED,
            format!(
                "monitor session {} of the named server is not the session held now; nothing was stopped",
                bound.session_id
            ),
        ));
    }
    if guard.as_ref().is_some_and(|held| held.is_tcp()) {
        return stop_tcp_session(guard, &state).await;
    }
    let session = guard.take().expect("matched a held session");
    if session.serial().is_none() {
        drop(guard);
    }
    let summary = session.stop().await;
    Ok(stop_response(
        &state.server_incarnation,
        bound.session_id,
        summary,
    ))
}

// ---------------------------------------------------------------------------
// GET /api/bus/monitor/telegrams?since=<seq>
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelegramsQuery {
    since: Option<u64>,
    #[serde(default)]
    context_only: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecodedValueDto {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dpt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    reason: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    program_default_dpts: Option<Vec<String>>,
    text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl From<&DecodedValue> for DecodedValueDto {
    fn from(v: &DecodedValue) -> Self {
        match v {
            DecodedValue::Value { dpt, text } => Self {
                kind: "value",
                dpt: Some(dpt.clone()),
                reason: None,
                program_default_dpts: None,
                text: text.clone(),
                error: None,
            },
            DecodedValue::Unresolved { reason, text } => Self {
                kind: "unresolved",
                dpt: None,
                reason: Some(reason.as_str()),
                program_default_dpts: reason.program_default_dpts(),
                text: text.clone(),
                error: None,
            },
            DecodedValue::Conflict { text } => Self {
                kind: "conflict",
                dpt: None,
                reason: None,
                program_default_dpts: None,
                text: text.clone(),
                error: None,
            },
            DecodedValue::Error {
                dpt,
                reason,
                text,
                error,
            } => Self {
                kind: "error",
                dpt: Some(dpt.clone()),
                reason: Some(match reason {
                    DecodeFailureReason::UnsupportedDpt => "unsupportedDpt",
                    DecodeFailureReason::DecodeFailed => "decodeFailed",
                }),
                program_default_dpts: None,
                text: text.clone(),
                error: Some(error.clone()),
            },
        }
    }
}

#[cfg(test)]
mod decoded_value_dto_tests {
    use super::*;

    #[test]
    fn codec_failures_keep_the_dpt_and_distinguish_unsupported_from_malformed_payloads() {
        let unsupported = decode_single(
            knx_core::DptRef {
                main: 40,
                sub: Some(1),
            },
            &knx_core::GroupValue::Short(1),
        );
        let malformed = decode_single(
            knx_core::DptRef {
                main: 1,
                sub: Some(1),
            },
            &knx_core::GroupValue::Bytes(vec![1, 2]),
        );
        let unsupported = serde_json::to_value(DecodedValueDto::from(&unsupported)).unwrap();
        let malformed = serde_json::to_value(DecodedValueDto::from(&malformed)).unwrap();
        assert_eq!(unsupported["kind"], "error");
        assert_eq!(unsupported["dpt"], "DPST-40-1");
        assert_eq!(unsupported["reason"], "unsupportedDpt");
        assert_eq!(malformed["kind"], "error");
        assert_eq!(malformed["dpt"], "DPST-1-1");
        assert_eq!(malformed["reason"], "decodeFailed");
        assert!(malformed["error"]
            .as_str()
            .unwrap()
            .contains("wrong payload length"));
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TelegramRowDto {
    seq: u64,
    timestamp: String,
    source: String,
    destination: String,
    destination_name: Option<String>,
    service: String,
    raw_payload: Option<String>,
    decoded: Option<DecodedValueDto>,
    /// KNOWN_LIMITATIONS §147: Ctrl1/Ctrl2 as received; `null` on the
    /// closed-session marker.
    control: Option<ReceivedControlDto>,
    /// AR20: unformatted sender address (0–65535); `null` on the marker.
    source_raw: Option<u16>,
    /// AR20: unformatted group address (0–65535); `null` on the marker.
    destination_raw: Option<u16>,
    /// AR20: milliseconds between admission and this response, on the
    /// server's monotonic clock. Always present today; clients must still
    /// treat `null` as "age unknown" (contract).
    observed_age_ms: Option<u64>,
    /// AR20: decimal string of the context generation the row was decoded
    /// with; `null` on the marker.
    flow_generation: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ReceivedControlDto {
    /// `"system"`, `"urgent"`, `"normal"` or `"low"`.
    priority: &'static str,
    /// `null` unless the frame is an `L_Data.ind`; see `ReceivedControl`.
    repeated: Option<bool>,
    hop_count: u8,
}

impl From<&crate::bus::ReceivedControl> for ReceivedControlDto {
    fn from(control: &crate::bus::ReceivedControl) -> Self {
        Self {
            priority: crate::bus::priority_name(control.priority),
            repeated: control.repeated,
            hop_count: control.hop_count,
        }
    }
}

impl TelegramRowDto {
    /// `now` is the response instant every row's age is measured against.
    fn at(row: &TelegramRow, now: std::time::Instant) -> Self {
        let age = now.saturating_duration_since(row.observed_at).as_millis();
        Self {
            source_raw: row.source_raw,
            destination_raw: row.destination_raw,
            observed_age_ms: Some(u64::try_from(age).unwrap_or(u64::MAX)),
            flow_generation: row.flow_generation.map(|g| g.to_string()),
            seq: row.seq,
            timestamp: row.timestamp.clone(),
            source: row.source.clone(),
            destination: row.destination.clone(),
            destination_name: row.destination_name.clone(),
            service: row.service.clone(),
            raw_payload: row.raw_payload.clone(),
            decoded: row.decoded.as_ref().map(DecodedValueDto::from),
            control: row.control.as_ref().map(ReceivedControlDto::from),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TelegramsResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    serial: Option<crate::bus_serial::SerialTarget>,
    /// The IP transport the session really runs on; absent for serial.
    #[serde(skip_serializing_if = "Option::is_none")]
    transport: Option<&'static str>,
    /// TCP only: cumulative frames the stream decoder skipped (unknown service or above
    /// the receive cap). Frames, not telegram rows, and not part of `droppedBefore`.
    #[serde(skip_serializing_if = "Option::is_none")]
    skipped_frames: Option<u64>,
    session_id: u64,
    server_incarnation: String,
    context_status: &'static str,
    project_open: Option<bool>,
    status: &'static str,
    next_since: u64,
    dropped_before: u64,
    /// AR20: decimal string of the session's current context generation.
    flow_generation: String,
    telegrams: Vec<TelegramRowDto>,
}

async fn poll_telegrams(
    State(state): State<SharedState>,
    Query(q): Query<TelegramsQuery>,
) -> Result<Json<TelegramsResponse>, ApiError> {
    let since = q.since.unwrap_or(0);
    let guard = state.bus_session.lock().await;
    let Some(session) = guard.as_ref() else {
        // Uniformly 404 whenever no session exists, regardless of whether
        // `since` was given — design spec §4.3: a `GET` against a session
        // that was never started is a not-found, not a conflict (contrast
        // the mutating routes' 409 for the same underlying fact).
        return Err(ApiError::with_status(
            StatusCode::NOT_FOUND,
            "no monitor session is active",
        ));
    };
    let session_id = session.id();
    let buffer = session.buffer();
    // Compare before locking the buffer: the drain reads context before buffer.
    // No await under the project mutex; contention/poison is unavailable, not fresh.
    let (context_status, project_open) = match state.project.try_lock() {
        Ok(project) => (
            match session.project_context_matches(project.as_ref()) {
                Some(true) => "current",
                Some(false) => "stale",
                None => "unavailable",
            },
            Some(project.is_some()),
        ),
        Err(_) => ("unavailable", None),
    };
    // Read before the buffer lock, never under it: the drain task holds
    // the buffer while it reads the context.
    let flow_generation = session.flow_generation().to_string();
    let buffer = buffer.lock().expect("bus session buffer poisoned");
    let now = std::time::Instant::now();
    // `status`/`droppedBefore`/`telegrams` are all read from the same
    // locked `buffer` above, in one snapshot — never observed from two
    // different instants (design spec §4.3, `TelegramBuffer`'s own doc
    // comment on why this matters).
    let status = match buffer.status() {
        SessionStatus::Active => "active",
        SessionStatus::Closed => "closed",
    };
    let response = TelegramsResponse {
        serial: session.serial().cloned(),
        transport: session.transport().map(knx_net::TunnelTransport::as_str),
        skipped_frames: session.skipped_frames(),
        session_id,
        server_incarnation: state.server_incarnation.clone(),
        context_status,
        project_open,
        status,
        next_since: if q.context_only {
            since
        } else {
            buffer.next_seq()
        },
        dropped_before: buffer.dropped_before(),
        flow_generation,
        telegrams: if q.context_only {
            Vec::new()
        } else {
            buffer
                .telegrams_since(since)
                .iter()
                .map(|row| TelegramRowDto::at(row, now))
                .collect()
        },
    };
    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// GET /api/bus/monitor/flow-snapshot?sessionId=<n>&generation=<n>
// ---------------------------------------------------------------------------

/// Both optional and decimal; strings so a value beyond JavaScript's safe
/// integers is refused here rather than rounded on the way in.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct FlowSnapshotQuery {
    session_id: Option<String>,
    generation: Option<String>,
}

fn parse_decimal(name: &str, value: Option<&str>) -> Result<Option<u64>, ApiError> {
    value
        .map(|text| {
            if text.is_empty() || !text.bytes().all(|b| b.is_ascii_digit()) {
                return Err(ApiError::bad_request(format!(
                    "{name} must be a non-negative decimal integer"
                )));
            }
            text.parse::<u64>()
                .map_err(|_| ApiError::bad_request(format!("{name} is out of range")))
        })
        .transpose()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FlowSnapshotResponse {
    server_incarnation: String,
    session_id: u64,
    generation: String,
    /// `"current"`: lists of `generation`, the session's current context.
    /// `"historical"`: the requested session or generation is not current;
    /// the server keeps no older snapshot, so the lists are empty.
    /// `"unavailable"`: no project context; the lists are empty.
    status: &'static str,
    /// `"Free"`, `"TwoLevel"` or `"ThreeLevel"`; `null` without a project.
    group_address_style: Option<String>,
    #[serde(flatten)]
    lists: crate::flow::FlowListsDto,
}

/// AR20: configured participants of the monitor session's interpretation
/// context — configuration evidence, not observed delivery. Read-only and
/// bounded ([`crate::flow::FLOW_LIMITS`]); takes no project lock and never
/// touches the tunnel.
async fn flow_snapshot(
    State(state): State<SharedState>,
    Query(q): Query<FlowSnapshotQuery>,
) -> Result<Json<FlowSnapshotResponse>, ApiError> {
    let requested_session = parse_decimal("sessionId", q.session_id.as_deref())?;
    let requested_generation = parse_decimal("generation", q.generation.as_deref())?;
    let guard = state.bus_session.lock().await;
    let Some(session) = guard.as_ref() else {
        return Err(ApiError::with_status(
            StatusCode::NOT_FOUND,
            "no monitor session is active",
        ));
    };
    let session_id = session.id();
    let (generation, style, participants) = session.flow_snapshot();
    drop(guard);
    let other_session = requested_session.is_some_and(|id| id != session_id);
    let other_generation = requested_generation.is_some_and(|g| g != generation);
    let (status, lists) = if other_session || other_generation {
        ("historical", crate::flow::FlowListsDto::default())
    } else if style.is_none() {
        ("unavailable", crate::flow::FlowListsDto::default())
    } else {
        (
            "current",
            crate::flow::FlowListsDto::bounded(&participants, crate::flow::FLOW_LIMITS),
        )
    };
    Ok(Json(FlowSnapshotResponse {
        server_incarnation: state.server_incarnation.clone(),
        session_id,
        generation: requested_generation.unwrap_or(generation).to_string(),
        status,
        group_address_style: style.map(|s| format!("{s:?}")),
        lists,
    }))
}

// ---------------------------------------------------------------------------
// POST /api/bus/write
// ---------------------------------------------------------------------------

/// `destination` is parsed using the active session's current shared
/// `GroupAddressStyle` (see
/// [`BusSession::group_address_style`]) — Free, TwoLevel or ThreeLevel,
/// whichever the open project currently configures
/// with — falling back to [`GroupAddressStyle::ThreeLevel`] only when no
/// session is active yet, or a session is active but no project was open
/// when it started.
///
/// **Task 5 fix**, previously a defect: this route used to parse
/// `destination` with [`GroupAddressStyle::ThreeLevel`] unconditionally,
/// mirroring `apps/knx-cli/src/main.rs`'s `run_bus_write` (which has the
/// same bug, out of scope here — see the task report). But `GET
/// /api/bus/monitor/telegrams` renders every row's `destination` in the
/// *project's actual* style, so for a Free- or TwoLevel-style project, the
/// string that route just emitted could never parse back through this one —
/// clicking a table row and sending it failed with a `400` on an address
/// this very server had just produced. Reading the style from the same
/// session's snapshot closes that round trip without changing what
/// `/telegrams` emits (fixing this by making `/telegrams` always emit
/// three-level addresses was considered and rejected — that would break the
/// display for two of the three project styles to make this one form
/// easier).
///
/// `dpt`, if given, is parsed with `knx_core::DptRef::parse` — which only
/// accepts `DPST-<main>-<sub>`/`DPT-<main>`, not the dotted `"1.001"` form
/// the design spec's own §4.3 JSON example uses for this same field. This
/// is not a deviation invented here: `resolve_write_value`'s explicit
/// `--dpt` path in the CLI (`apps/knx-cli/src/main.rs:1602`) already calls
/// this very same `DptRef::parse`, so the spec's own worked example does
/// not actually round-trip through the mechanism the spec itself says this
/// route must reuse. Flagged in the task report, not silently "fixed" by
/// inventing a second parser.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WriteRequest {
    destination: String,
    dpt: Option<String>,
    input_format: Option<String>,
    value: String,
}

/// `decodedEcho` (task 27, requested mid-run: payloads should show the
/// actual data, not only bytes, and the confirmation line was the one
/// surface still missing it — `apps/knx-web/src/BusMonitorPanel.tsx`'s
/// Decoded column already covers the monitor). Reuses [`DecodedValueDto`]
/// as-is rather than a dedicated struct: [`decode_single`] only ever
/// returns its `Value`/`Error` variants here (the DPT this route decodes
/// against is always already resolved — never `Unresolved`/`Conflict`),
/// and that shape already carries exactly `dpt`, `text` and an optional
/// `error`, so a second struct would just be this one with two unused
/// variants deleted.
///
/// Decoded from `value` — the bytes [`knx_core::encode`] actually produced
/// and [`BusSession::send`] put on the wire — never from `body.value`.
/// Echoing the request's own input back would prove nothing about the
/// codec; decoding the wire bytes is what actually demonstrates a
/// round trip. A decode failure here does not fail the request: the
/// telegram already reached the bus by the time this runs, so the
/// response stays `200` and [`decode_single`]'s `DecodedValue::Error`
/// carries the mismatch as text instead — see [`DecodedValueDto::from`].
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WriteResponse {
    encoded_payload: String,
    service: &'static str,
    decoded_echo: DecodedValueDto,
    /// The IP transport the write went out on. Over TCP a successful send is
    /// the socket accepting the frame, not a device's confirmation.
    #[serde(skip_serializing_if = "Option::is_none")]
    transport: Option<&'static str>,
}

/// Both identity fields are required in the body; duplicate/extra fields fail decoding.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReadRequest {
    session_id: u64,
    server_incarnation: String,
    destination: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct BoundWriteRequest {
    session_id: u64,
    server_incarnation: String,
    destination: String,
    dpt: Option<String>,
    input_format: Option<String>,
    value: String,
}

fn validate_group_identity(id: u64, incarnation: &str) -> Result<(), ApiError> {
    if id == 0
        || id > 9_007_199_254_740_991
        || incarnation.is_empty()
        || incarnation.len() > 256
        || !incarnation
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(ApiError::bad_request(
            "a safe positive sessionId and serverIncarnation are required; nothing was sent",
        ));
    }
    Ok(())
}

fn matching_group_session<'a>(
    state: &SharedState,
    held: &'a Option<BusSession>,
    id: u64,
    incarnation: &str,
) -> Result<&'a BusSession, ApiError> {
    held.as_ref()
        .filter(|session| session.id() == id && state.server_incarnation == incarnation)
        .ok_or_else(|| {
            ApiError::conflict(
                BUS_SESSION_CHANGED,
                "the named bus session is no longer held; nothing was sent",
            )
        })
}

async fn read_value(
    State(state): State<SharedState>,
    body: Result<Json<ReadRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let Json(body) = body.map_err(|_| {
        ApiError::bad_request(
            "group read requires sessionId, serverIncarnation and destination; nothing was sent",
        )
    })?;
    validate_group_identity(body.session_id, &body.server_incarnation)?;
    let guard = state
        .bus_session
        .try_lock()
        .map_err(|_| session_lock_busy(&state, system2_running(&state)))?;
    if system2_running(&state) {
        return Err(session_lock_busy(&state, true));
    }
    let session =
        matching_group_session(&state, &guard, body.session_id, &body.server_incarnation)?;
    let ga = GroupAddress::parse(
        &body.destination,
        session
            .group_address_style()
            .unwrap_or(GroupAddressStyle::ThreeLevel),
    )
    .map_err(|error| ApiError::bad_request(error.to_string()))?;
    session
        .send(Destination::Group(ga), ApplicationService::GroupValueRead)
        .await
        .map_err(session_error_to_api_error)?;
    Ok(Json(match session.transport() {
        Some(transport) => serde_json::json!({
            "service": "GroupValueRead",
            "transport": transport.as_str(),
        }),
        None => serde_json::json!({ "service": "GroupValueRead" }),
    }))
}

async fn write_bound_value(
    State(state): State<SharedState>,
    body: Result<Json<BoundWriteRequest>, axum::extract::rejection::JsonRejection>,
) -> Result<Json<WriteResponse>, ApiError> {
    let Json(body) = body.map_err(|_| ApiError::bad_request("bound group write requires an exact session identity, destination and value; nothing was sent"))?;
    validate_group_identity(body.session_id, &body.server_incarnation)?;
    let guard = state
        .bus_session
        .try_lock()
        .map_err(|_| session_lock_busy(&state, system2_running(&state)))?;
    if system2_running(&state) {
        return Err(session_lock_busy(&state, true));
    }
    let session =
        matching_group_session(&state, &guard, body.session_id, &body.server_incarnation)?;
    send_write_value(
        session,
        WriteRequest {
            destination: body.destination,
            dpt: body.dpt,
            input_format: body.input_format,
            value: body.value,
        },
    )
    .await
}

async fn write_value(
    State(state): State<SharedState>,
    Json(body): Json<WriteRequest>,
) -> Result<Json<WriteResponse>, ApiError> {
    // One lock for the whole handler: resolving the group-address style,
    // resolving the DPT (when not given explicitly) and sending all need
    // the same active session, and `tokio::sync::Mutex` lets this guard stay
    // held across `send`'s own `.await` below — see this module's doc
    // comment.
    let guard = state.bus_session.lock().await;

    let session = guard
        .as_ref()
        .ok_or_else(|| session_error_to_api_error(BusSessionError::NoActiveSession))?;
    if session.serial().is_some() || session.is_tcp() {
        return Err(ApiError::conflict(
            "busSessionBindingRequired",
            "serial and TCP writes require the distinct bound write route; nothing was sent",
        ));
    }
    send_write_value(session, body).await
}

async fn send_write_value(
    session: &BusSession,
    body: WriteRequest,
) -> Result<Json<WriteResponse>, ApiError> {
    let style = session
        .group_address_style()
        .unwrap_or(GroupAddressStyle::ThreeLevel);
    let ga = GroupAddress::parse(&body.destination, style)
        .map_err(|error| ApiError::bad_request(error.to_string()))?;
    let dpt = if let Some(dpt_str) = body.dpt.as_deref() {
        knx_core::DptRef::parse(dpt_str).map_err(|e| ApiError::bad_request(e.to_string()))?
    } else {
        match session.resolve_write_dpt(ga) {
            GroupAddressDpt::Single(dpt) => dpt,
            GroupAddressDpt::None => {
                // Fix 3 (Task 5 review): worded identically to
                // `BusComposeForm.tsx`'s `NO_DPT_RESOLVED_MESSAGE`, the
                // client-side check for the same fact — this route is the
                // backstop for a request the UI's own check did not
                // generate (or was bypassed), not a second, differently
                // phrased opinion about it. The client's message never
                // names the destination either (it only ever fires for the
                // address already sitting in the form), so there is
                // nothing lost by dropping it here too.
                return Err(ApiError::bad_request(
                    "No DPT resolved for this group address — enter one explicitly.",
                ));
            }
            GroupAddressDpt::Conflict(dpts) => {
                // Same alignment, for `conflictingDptsMessage()`.
                let names: Vec<String> = dpts.iter().map(|d| d.to_string()).collect();
                return Err(ApiError::bad_request(format!(
                    "Conflicting DPTs for this group address: {} — enter one explicitly.",
                    names.join(", ")
                )));
            }
        }
    };

    knx_core::validate_group_write_dpt(dpt).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let value = match body.input_format.as_deref() {
        Some(name) => {
            let format = knx_core::DptInputFormat::parse_name(name).ok_or_else(|| {
                ApiError::bad_request(format!(
                    "unknown input format {name:?}; expected canonical, decimal, hexadecimal, binary, or text"
                ))
            })?;
            knx_core::encode(dpt, &body.value, format)
        }
        None => knx_core::encode_inferred_format(dpt, &body.value),
    }
    .map_err(|e| ApiError::bad_request(e.to_string()))?;

    session
        .send(
            Destination::Group(ga),
            ApplicationService::GroupValueWrite(value.clone()),
        )
        .await
        .map_err(session_error_to_api_error)?;

    Ok(Json(WriteResponse {
        encoded_payload: bus::format_group_value_payload(&value),
        service: "GroupValueWrite",
        decoded_echo: DecodedValueDto::from(&decode_single(dpt, &value)),
        transport: session.transport().map(knx_net::TunnelTransport::as_str),
    }))
}

// ---------------------------------------------------------------------------
// POST /api/bus/discover
// ---------------------------------------------------------------------------

/// One entry per KNX-compatible interface that answered the multicast
/// `SEARCH_REQUEST`. Existing endpoint/name/tunnelling fields remain intact;
/// the additive nullable `deviceInfo` carries the decoded raw DIB metadata.
/// Unknown medium/status bits are preserved, never interpreted as capabilities
/// or used as proof of identity or permission to connect/write.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveredInterfaceDto {
    /// `"<ip>:<port>"` — the same `host:port` shape
    /// `POST /api/bus/monitor/start` parses back out of its `gateway`
    /// field, so a found interface can be handed straight to Connect
    /// without the client reassembling anything.
    control_endpoint: String,
    individual_address: String,
    friendly_name: String,
    /// From the Supported Service Families DIB, when the interface sends
    /// one. `false` means "it did not say it supports tunnelling" — which
    /// covers both a routing-only interface and one that sent no families
    /// DIB at all; `knx-net` collapses those two cases before this route
    /// sees them.
    supports_tunnelling: bool,
    device_info: Option<DiscoveredDeviceInfoDto>,
}

/// Raw decoded fields, not interpreted capabilities or device identity proof.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoveredDeviceInfoDto {
    medium: u8,
    status: u8,
    project_installation_id: u16,
    serial_number: [u8; 6],
    routing_multicast: String,
    mac_address: [u8; 6],
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DiscoverResponse {
    interfaces: Vec<DiscoveredInterfaceDto>,
}

/// `POST`, not `GET`, even though a search changes nothing on this server
/// and nothing on any device. A `GET` is fair game for prefetching,
/// speculative revalidation and link-preview crawlers; every one of those
/// would put an unasked-for multicast datagram on somebody's installation
/// network. `POST` is the shape that says "only when a human asked".
///
/// An empty `interfaces` array is a `200`. It is the ordinary answer on a
/// network with no KNX-compatible interface, and the ordinary answer on a
/// host whose multicast never leaves its container — neither is a failure
/// this server can distinguish from the other, and neither is something
/// the caller can fix by retrying differently. Only a search that could
/// not be *performed* is an error, and it maps through
/// [`session_error_to_api_error`] like every other transport failure on
/// this API: `502`.
///
/// Read-only, and this is the boundary that lets it exist at all: one
/// multicast `SEARCH_REQUEST` out, `SEARCH_RESPONSE`s in. No connection is
/// opened, no individual address is addressed, nothing is written to any
/// device.
async fn discover_interfaces(
    State(state): State<SharedState>,
) -> Result<Json<DiscoverResponse>, ApiError> {
    let gateways = state
        .connector
        .discover()
        .await
        .map_err(session_error_to_api_error)?;

    Ok(Json(DiscoverResponse {
        interfaces: gateways
            .into_iter()
            .map(|g| DiscoveredInterfaceDto {
                control_endpoint: g.control_endpoint.to_string(),
                individual_address: g.individual_address.to_string(),
                friendly_name: g.friendly_name,
                supports_tunnelling: g.supports_tunnelling,
                device_info: g.device_info.map(|info| DiscoveredDeviceInfoDto {
                    medium: info.medium,
                    status: info.status,
                    project_installation_id: info.project_installation_id,
                    serial_number: info.serial_number,
                    routing_multicast: info.routing_multicast.to_string(),
                    mac_address: info.mac_address,
                }),
            })
            .collect(),
    }))
}

#[cfg(test)]
#[path = "bus_routes_admission_tests.rs"]
mod admission_tests;
