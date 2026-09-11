// apps/knx-server/src/bus_routes.rs
//! `/api/bus/*` — T15 task 3: the HTTP layer over `bus.rs`'s session/buffer/
//! drain-task machinery (design spec `docs/superpowers/specs/
//! 2026-09-11-group-monitor-design.md` §4.3). Four routes, registered by
//! [`bus_routes`] and merged into `app()` in `lib.rs` next to
//! `routes::project_routes()`/`fs_routes::fs_routes()`.
//!
//! A new module, not a section of `routes.rs`: `routes.rs` is already large
//! (1500+ lines) and entirely project/device/catalog-shaped; these four
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

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use knx_core::{GroupAddress, GroupAddressDpt, GroupAddressStyle};
use knx_net::{ApplicationService, Destination};

use crate::bus::{
    self, BusSession, BusSessionError, DecodedValue, GroupAddressContext, SessionStatus,
    TelegramRow,
};
use crate::errors::ApiError;
use crate::SharedState;

pub fn bus_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/bus/monitor/start", post(start_monitor))
        .route("/api/bus/monitor/stop", post(stop_monitor))
        .route("/api/bus/monitor/telegrams", get(poll_telegrams))
        .route("/api/bus/write", post(write_value))
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
    telegram_count: usize,
    dropped_count: u64,
    /// `Some(message)` only if the drain task's own `JoinHandle` reported a
    /// panic (`BusSessionSummary::drain_panic`, carried Task 2 review
    /// finding) — omitted from the wire entirely in the ordinary case
    /// (`skip_serializing_if`), so an ordinary stop's JSON shape is exactly
    /// design spec §4.3's `{ sessionId, telegramCount, droppedCount }`, byte
    /// for byte. `200`, not `500`: the stop itself genuinely succeeded (the
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
        telegram_count: summary.telegram_count,
        dropped_count: summary.dropped_count,
        warning: summary.drain_panic,
    }))
}

// ---------------------------------------------------------------------------
// GET /api/bus/monitor/telegrams?since=<seq>
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct TelegramsQuery {
    since: Option<u64>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecodedValueDto {
    kind: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    dpt: Option<String>,
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
                text: text.clone(),
                error: None,
            },
            DecodedValue::Unresolved { text } => Self {
                kind: "unresolved",
                dpt: None,
                text: text.clone(),
                error: None,
            },
            DecodedValue::Conflict { text } => Self {
                kind: "conflict",
                dpt: None,
                text: text.clone(),
                error: None,
            },
            DecodedValue::Error { text, error } => Self {
                kind: "error",
                dpt: None,
                text: text.clone(),
                error: Some(error.clone()),
            },
        }
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
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TelegramsResponse {
    session_id: u64,
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
        status,
        next_since: buffer.next_seq(),
        dropped_before: buffer.dropped_before(),
        telegrams: buffer
            .telegrams_since(since)
            .iter()
            .map(TelegramRowDto::from)
            .collect(),
    };
    Ok(Json(response))
}

// ---------------------------------------------------------------------------
// POST /api/bus/write
// ---------------------------------------------------------------------------

/// `destination` is parsed using the active session's own cached
/// `GroupAddressStyle` (design spec §4.4's snapshot; see
/// [`BusSession::group_address_style`]) — Free, TwoLevel or ThreeLevel,
/// whichever the project open at session-start time was actually configured
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
    value: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct WriteResponse {
    encoded_payload: String,
    service: &'static str,
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

    // The active session's own cached style (Free/TwoLevel/ThreeLevel), or
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
                return Err(ApiError::bad_request(format!(
                    "no DPT resolved for {} — supply \"dpt\" explicitly",
                    body.destination
                )));
            }
            GroupAddressDpt::Conflict(dpts) => {
                let names: Vec<String> = dpts.iter().map(|d| d.to_string()).collect();
                return Err(ApiError::bad_request(format!(
                    "conflicting DPTs for {}: {} — supply \"dpt\" explicitly",
                    body.destination,
                    names.join(", ")
                )));
            }
        }
    };

    let value =
        knx_core::encode(dpt, &body.value).map_err(|e| ApiError::bad_request(e.to_string()))?;

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
    }))
}
