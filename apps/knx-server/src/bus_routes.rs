// apps/knx-server/src/bus_routes.rs
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
//! Mutex` (see `domain.rs`'s doc comment on that field for why) — every
//! handler below locks it with `.lock().await` and, for `/start` and
//! `/write`, holds the guard across `BusSession::start`/`BusSession::send`'s
//! own `.await` on purpose: at most one session exists, so serializing
//! concurrent attempts through this one lock is correct, not a cost to
//! avoid.

use std::net::SocketAddrV4;
use std::time::Duration;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

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
use crate::bus_scan::{LineScanResult, LineScanSession, LineScanStatus};
use crate::domain;
use crate::errors::ApiError;
use crate::SharedState;

pub fn bus_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/bus/monitor/start", post(start_monitor))
        .route("/api/bus/monitor/stop", post(stop_monitor))
        .route("/api/bus/monitor/telegrams", get(poll_telegrams))
        .route("/api/bus/write", post(write_value))
        .route("/api/bus/discover", post(discover_interfaces))
        .route("/api/bus/scan/estimate", post(estimate_scan))
        .route("/api/bus/scan/start", post(start_scan))
        .route("/api/bus/scan/results", get(poll_scan))
        .route("/api/bus/scan/cancel", post(cancel_scan))
        .route("/api/bus/scan/comparison", get(scan_comparison))
        .route("/api/bus/scan/reconcile", post(reconcile_scan))
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
    estimate: ScanEstimateResponse,
}

async fn start_scan(
    State(state): State<SharedState>,
    Json(body): Json<ScanRequest>,
) -> Result<Json<ScanStartResponse>, ApiError> {
    let gateway = body
        .gateway
        .as_deref()
        .ok_or_else(|| ApiError::bad_request("gateway is required"))?
        .parse::<SocketAddrV4>()
        .map_err(|_| ApiError::bad_request("gateway is not a host:port IPv4 address"))?;
    let (plan, policy, estimate, range) = build_scan(&body)?;
    let response_estimate = estimate_response(&plan, &policy, estimate);
    let mut guard = state.line_scan_session.lock().await;
    // Checked while holding this session's lock, never before it: a
    // download start holds its own lock and then waits for this one, so
    // this `try_lock` sees it (ADR-0045 §4).
    if crate::device_download_routes::download_in_progress(&state) {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "a download to a device is running: the gateway serves one tunnel",
        ));
    }
    if crate::address_programming_routes::address_programming_in_progress(&state) {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "an individual-address programming is running: the gateway serves one tunnel",
        ));
    }
    if let Some(existing) = guard.as_mut() {
        if existing.status() == LineScanStatus::Running {
            return Err(ApiError::with_status(
                StatusCode::CONFLICT,
                format!("line scan session {} is already running", existing.id()),
            ));
        }
        existing.cancel().await;
    }
    let id = state
        .next_line_scan_session_id
        .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let session =
        LineScanSession::start(id, range, gateway, state.connector.as_ref(), plan, policy)
            .await
            .map_err(session_error_to_api_error)?;
    *guard = Some(session);
    Ok(Json(ScanStartResponse {
        session_id: id,
        estimate: response_estimate,
    }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ScanPollQuery {
    #[serde(default)]
    since: usize,
    session_id: Option<u64>,
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

fn scan_response(session: &LineScanSession, since: usize) -> ScanResultsResponse {
    let (snapshot_status, next_since, results) = session.snapshot_since(since);
    let (status, error) = scan_status(snapshot_status);
    ScanResultsResponse {
        session_id: session.id(),
        status,
        error,
        next_since,
        completed_count: next_since,
        total_count: session.total_count(),
        omitted_addresses: session.omitted().iter().map(ToString::to_string).collect(),
        excluded_addresses: session.excluded().iter().map(ToString::to_string).collect(),
        results: results.into_iter().map(Into::into).collect(),
    }
}

async fn poll_scan(
    State(state): State<SharedState>,
    Query(query): Query<ScanPollQuery>,
) -> Result<Json<ScanResultsResponse>, ApiError> {
    let guard = state.line_scan_session.lock().await;
    let session = guard.as_ref().ok_or_else(|| {
        ApiError::with_status(StatusCode::NOT_FOUND, "no line scan session exists")
    })?;
    if query.session_id.is_some_and(|id| id != session.id()) {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            format!(
                "line scan session {} is no longer active",
                query.session_id.unwrap()
            ),
        ));
    }
    Ok(Json(scan_response(session, query.since)))
}

async fn cancel_scan(
    State(state): State<SharedState>,
    Query(query): Query<ScanSessionQuery>,
) -> Result<Json<ScanResultsResponse>, ApiError> {
    let mut guard = state.line_scan_session.lock().await;
    let session = guard.as_mut().ok_or_else(|| {
        ApiError::with_status(StatusCode::CONFLICT, "no line scan session exists")
    })?;
    if query.session_id != session.id() {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            format!("line scan session {} is no longer active", query.session_id),
        ));
    }
    session.cancel().await;
    Ok(Json(scan_response(session, 0)))
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
    let guard = state.line_scan_session.lock().await;
    let session = guard.as_ref().ok_or_else(|| {
        ApiError::with_status(StatusCode::CONFLICT, "no line scan session exists")
    })?;
    if query.session_id != session.id() {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            format!("line scan session {} is no longer active", query.session_id),
        ));
    }

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
    let guard = state.line_scan_session.lock().await;
    let session = guard.as_ref().ok_or_else(|| {
        ApiError::with_status(StatusCode::CONFLICT, "no line scan session exists")
    })?;
    if body.session_id != session.id() {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            format!("line scan session {} is no longer active", body.session_id),
        ));
    }
    let (status, _, results) = session.snapshot_since(0);
    if status != LineScanStatus::Completed {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "only a completed line scan can be reconciled",
        ));
    }

    let unexpected = parse_scan_selection(body.unexpected, "unexpected")?;
    let missing = parse_scan_selection(body.missing, "missing")?;
    let evidence: Vec<_> = results
        .into_iter()
        .map(|result| (result.address, result.outcome))
        .collect();
    let excluded = session.excluded().iter().copied().collect();
    let tree = domain::reconcile_scan_impl(
        &state,
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
fn session_error_to_api_error(error: BusSessionError) -> ApiError {
    match error {
        BusSessionError::Transport(e) => {
            ApiError::with_status(StatusCode::BAD_GATEWAY, e.to_string())
        }
        BusSessionError::AlreadyActive | BusSessionError::NoActiveSession => {
            ApiError::with_status(StatusCode::CONFLICT, error.to_string())
        }
    }
}

// ---------------------------------------------------------------------------
// POST /api/bus/monitor/start
// ---------------------------------------------------------------------------

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
}

async fn start_monitor(
    State(state): State<SharedState>,
    Json(body): Json<StartRequest>,
) -> Result<Json<StartResponse>, ApiError> {
    let gateway: SocketAddrV4 = body
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request(format!("not a gateway address: {:?}", body.gateway)))?;

    let mut guard = state.bus_session.lock().await;
    // Checked while holding this session's lock, never before it: a
    // download start holds its own lock and then waits for this one, so
    // this `try_lock` sees it (ADR-0045 §4).
    if crate::device_download_routes::download_in_progress(&state) {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "a download to a device is running: the gateway serves one tunnel",
        ));
    }
    if crate::address_programming_routes::address_programming_in_progress(&state) {
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            "an individual-address programming is running: the gateway serves one tunnel",
        ));
    }
    if let Some(existing) = guard.as_ref() {
        // Design spec §4.3 / brief item 2: the `409` body must *name* the
        // existing session, not just say "one exists" — `ApiError` only
        // carries a plain string, so "naming it" means putting the id and
        // gateway into that string's prose, the same way every other
        // `ApiError` in this codebase communicates detail.
        return Err(ApiError::with_status(
            StatusCode::CONFLICT,
            format!(
                "a monitor session is already active (id {}, gateway {})",
                existing.id(),
                existing.gateway()
            ),
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

    let session = BusSession::start(id, gateway, state.connector.as_ref(), ctx)
        .await
        .map_err(session_error_to_api_error)?;

    let assigned_address = session.assigned_address().to_string();
    *guard = Some(session);

    Ok(Json(StartResponse {
        session_id: id,
        server_incarnation: state.server_incarnation.clone(),
        assigned_address,
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
    /// `Some(message)` only if the drain task's own `JoinHandle` reported a
    /// panic (`BusSessionSummary::drain_panic`, carried Task 2 review
    /// finding) — omitted from the wire entirely in the ordinary case
    /// (`skip_serializing_if`). The additive `serverIncarnation` binds the
    /// reusable numeric id to this process lifetime. `200`, not `500`: the
    /// stop itself genuinely succeeded (the
    /// session is gone, the tally is accurate) — only the teardown that
    /// followed misbehaved. See `BusSession::stop`'s doc comment for the
    /// full reasoning and the task report for why `500` was rejected.
    #[serde(skip_serializing_if = "Option::is_none")]
    warning: Option<String>,
}

async fn stop_monitor(State(state): State<SharedState>) -> Result<Json<StopResponse>, ApiError> {
    let session = {
        let mut guard = state.bus_session.lock().await;
        guard.take()
    };
    let Some(session) = session else {
        return Err(session_error_to_api_error(BusSessionError::NoActiveSession));
    };
    let session_id = session.id();
    let summary = session.stop().await;
    Ok(Json(StopResponse {
        session_id,
        server_incarnation: state.server_incarnation.clone(),
        telegram_count: summary.telegram_count,
        dropped_count: summary.dropped_count,
        warning: summary.drain_panic,
    }))
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
                text: text.clone(),
                error: None,
            },
            DecodedValue::Unresolved { text } => Self {
                kind: "unresolved",
                dpt: None,
                reason: None,
                text: text.clone(),
                error: None,
            },
            DecodedValue::Conflict { text } => Self {
                kind: "conflict",
                dpt: None,
                reason: None,
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

impl From<&TelegramRow> for TelegramRowDto {
    fn from(row: &TelegramRow) -> Self {
        Self {
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
    session_id: u64,
    server_incarnation: String,
    context_status: &'static str,
    project_open: Option<bool>,
    status: &'static str,
    next_since: u64,
    dropped_before: u64,
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
    let buffer = buffer.lock().expect("bus session buffer poisoned");
    // `status`/`droppedBefore`/`telegrams` are all read from the same
    // locked `buffer` above, in one snapshot — never observed from two
    // different instants (design spec §4.3, `TelegramBuffer`'s own doc
    // comment on why this matters).
    let status = match buffer.status() {
        SessionStatus::Active => "active",
        SessionStatus::Closed => "closed",
    };
    let response = TelegramsResponse {
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
        telegrams: if q.context_only {
            Vec::new()
        } else {
            buffer
                .telegrams_since(since)
                .iter()
                .map(TelegramRowDto::from)
                .collect()
        },
    };
    Ok(Json(response))
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

    // The active session's current shared style (Free/TwoLevel/ThreeLevel), or
    // the three-level fallback if no session is active yet or its project
    // snapshot has none — see `WriteRequest`'s doc comment for why this
    // must match what `/telegrams` rendered.
    let style = guard
        .as_ref()
        .and_then(BusSession::group_address_style)
        .unwrap_or(GroupAddressStyle::ThreeLevel);
    let ga = GroupAddress::parse(&body.destination, style)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;

    let Some(session) = guard.as_ref() else {
        return Err(session_error_to_api_error(BusSessionError::NoActiveSession));
    };

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
