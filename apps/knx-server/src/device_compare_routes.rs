//! `/api/device-compare`: what a download would change on a device, read only.
//!
//! The same plan `/api/device-download/plan` shows (the open project, the
//! product database, an optional CP §3.9.2.4 partial selection), then
//! `knx_net`'s `compare_with_plan`: a read-only management session that
//! reads exactly the regions the plan would write and the load states of
//! the machines it touches. It is `knx device compare` over HTTP.
//!
//! It cannot write: the session has no authorisation, the request has no
//! phrase and no key field, and no access key is sent. A device that
//! protects its memory against reading at the free level refuses, and the
//! error says so; no key is ever guessed.
//!
//! It holds the gateway's one tunnel while it reads, so it takes the same
//! locks as `serial_address_routes::find`: the address programming by
//! `try_lock`, then the monitor, then the scan, after checking that no
//! download is starting or running. It never waits on the download's lock,
//! and it holds nothing a download start holds while that start waits, so
//! no wait can form a cycle (ADR-0045 §4).

use std::net::SocketAddrV4;
use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use axum::extract::{DefaultBodyLimit, Extension, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_net::commissioning::memory_download::{CompareError, ReadBackError};
use knx_net::SessionError;

use crate::device_download::TunnelTransport;
use crate::device_download_routes::{parse_target, PartialDto};
use crate::errors::ApiError;
use crate::SharedState;

mod adoption;
mod read;

pub fn device_compare_routes() -> Router<SharedState> {
    Router::new()
        .route("/api/device-compare", post(compare))
        .route("/api/device-compare/preview", post(adoption::preview))
        .route("/api/device-compare/apply", post(adoption::apply))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .layer(Extension(Arc::new(Mutex::new(
            adoption::Retained::default(),
        ))))
        .layer(axum::middleware::map_response(
            |mut response: Response| async move {
                response
                    .headers_mut()
                    .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
                response
            },
        ))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CompareRequest {
    address: String,
    gateway: String,
    #[serde(default)]
    partial: Option<PartialDto>,
    #[serde(default)]
    binding: Option<ComparisonBinding>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ComparisonBinding {
    server_incarnation: String,
    project_incarnation: u64,
    device_id: u32,
    comparison_revision: u64,
}

fn binding_at(
    state: &SharedState,
    target: knx_core::IndividualAddress,
) -> Option<ComparisonBinding> {
    let guard = state.project.lock().expect("state mutex poisoned");
    let project = guard.as_ref()?;
    let device = knx_app::programming_state::sole_device_at(project, target)?;
    let stack = state.command_stack.lock().expect("state mutex poisoned");
    Some(ComparisonBinding {
        server_incarnation: state.server_incarnation.clone(),
        project_incarnation: state.project_incarnation.load(Ordering::Relaxed),
        device_id: device.0,
        comparison_revision: stack.comparison_revision(device),
    })
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct DecodedDto {
    index: usize,
    /// Presentation hint only; App validates the entire selection again.
    adoptable: bool,
    kind: &'static str,
    label: String,
    address: u32,
    project: Option<String>,
    device: Option<String>,
    reason: Option<String>,
}

fn decoded(rows: &[knx_productdb::image_comparison::ComparisonRow]) -> Vec<DecodedDto> {
    rows.iter()
        .enumerate()
        .map(|(index, row)| DecodedDto {
            index,
            adoptable: row.kind == "parameter"
                && row.parameter.is_some()
                && row.reason.is_none()
                && row.project.is_some()
                && row.device.is_some(),
            kind: row.kind,
            label: row.label.clone(),
            address: row.address,
            project: row.project.clone(),
            device: row.device.clone(),
            reason: row.reason.clone(),
        })
        .collect()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LoadStateDto {
    machine: String,
    state: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ChangeDto {
    /// The first address of the run.
    address: u16,
    /// The segment the run lies in, when the image names one.
    segment: Option<String>,
    /// What the device holds there now.
    device: Vec<u8>,
    /// What a download would write there.
    project: Vec<u8>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CompareResponse {
    address: String,
    device_name: String,
    program_id: String,
    /// Always `false`: this route reads only. Stated, not implied.
    written: bool,
    /// `true` when this compared a CP §3.9.2.4 partial download.
    partial: bool,
    mask: u16,
    manufacturer: u16,
    load_states: Vec<LoadStateDto>,
    /// Octets read, every one a download would write.
    octets: usize,
    /// Octets a download would change.
    differing_octets: usize,
    /// `true` when the device holds every octet a download would write.
    same: bool,
    changes: Vec<ChangeDto>,
    #[serde(flatten)]
    binding: ComparisonBinding,
    /// Content changed during the read: do not present an old plan as current.
    outdated: bool,
    decoded: Vec<DecodedDto>,
    observation_token: Option<String>,
    observation_expires_in_seconds: Option<u64>,
}

async fn compare(
    State(state): State<SharedState>,
    Extension(sessions): Extension<adoption::Sessions>,
    headers: HeaderMap,
    Json(body): Json<CompareRequest>,
) -> Result<Response, ApiError> {
    // Goal A3: no new tunnel once a stop has begun (KL-163).
    state.stop_admission.admit()?;
    let gateway: SocketAddrV4 = body
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request("gateway is not a host:port IPv4 address"))?;
    let target = parse_target(&body.address)?;
    let partial = body.partial.map(PartialDownloadParts::from);
    let prepared = read::Read::prepare(&state, target, partial, body.binding.as_ref())?;
    let binding = prepared.binding.clone();
    let device_name = prepared.name.clone();
    let (browser, cookie) = adoption::browser(&headers)?;

    let Ok(programming) = state.address_programming.try_lock() else {
        return Err(crate::tunnel_holder::gateway_busy(
            crate::tunnel_holder::TunnelHolder::Connecting {
                session: "managementOperation",
            },
            "the gateway is in use by an individual-address programming",
        ));
    };
    if let Some(holder) = crate::tunnel_holder::download_holder(&state) {
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
            "an individual-address programming is running: the gateway serves one tunnel",
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

    if !prepared.is_current(&state) {
        return Err(read::stale());
    }
    // Register only after validation and gateway-conflict checks. A dropped
    // request leaves `unknown`, never a fabricated successful comparison.
    let activity = state
        .one_shot_activity
        .start("deviceCompare", Some(target.to_string()));
    let tunnel = match state.connector.connect_tunnel(gateway).await {
        Ok(tunnel) => tunnel,
        Err(e) => {
            activity.finish("failed");
            return Err(crate::tunnel_holder::gateway_connect_failed(gateway, &e));
        }
    };
    let compared = prepared
        .observe(
            &TunnelTransport(tunnel.as_ref()),
            state.device_download_timing,
        )
        .await;
    let _ = tunnel.disconnect().await;
    activity.finish(if compared.is_ok() {
        "finished"
    } else {
        "failed"
    });
    drop(scan);
    drop(monitor);
    drop(programming);
    let observed = compared?;
    let outdated = !observed.is_current(&state, target, &binding);
    let decoded = decoded(observed.rows());
    let segments = &observed.image().segments;
    let compared = observed.comparison();
    let changes = compared
        .changes
        .iter()
        .map(|change| ChangeDto {
            address: change.address,
            segment: segments
                .iter()
                .find(|segment| {
                    let start = segment.address;
                    let end = start + segment.octets.len() as u32;
                    (start..end).contains(&u32::from(change.address))
                })
                .map(|segment| segment.id.clone()),
            device: change.device.clone(),
            project: change.planned.clone(),
        })
        .collect();
    let mut response = CompareResponse {
        address: target.to_string(),
        device_name,
        program_id: observed.program_id().to_owned(),
        written: false,
        partial: matches!(&observed, read::Observation::Partial { .. }),
        mask: compared.held.mask.0,
        manufacturer: compared.held.manufacturer,
        load_states: compared
            .held
            .load_states
            .iter()
            .map(|(machine, state)| LoadStateDto {
                machine: machine.to_string(),
                state: state.to_string(),
            })
            .collect(),
        octets: compared.held.octets(),
        differing_octets: compared.differing_octets(),
        same: compared.is_same(),
        changes,
        binding,
        outdated,
        decoded,
        observation_token: None,
        observation_expires_in_seconds: None,
    };
    // Retain only genuine full reads, under current project ownership. A late
    // result cannot evict the new project's evidence or acquire a fresh token.
    if let read::Observation::Full(observation) = observed {
        let project = state.project.lock().expect("state mutex poisoned");
        let stack = state.command_stack.lock().expect("state mutex poisoned");
        if project
            .as_ref()
            .is_some_and(|p| observation.is_current(p, &stack, &adoption::scope(&state)))
        {
            response.observation_token = Some(
                sessions
                    .lock()
                    .expect("comparison mutex poisoned")
                    .observe(browser, *observation)?,
            );
            response.observation_expires_in_seconds = Some(600);
        } else {
            response.outdated = true;
        }
    }
    let mut response = Json(response).into_response();
    if let Some(cookie) = cookie {
        response.headers_mut().insert(header::SET_COOKIE, cookie);
    }
    Ok(response)
}

fn compare_error(error: CompareError) -> ApiError {
    let kind = match &error {
        CompareError::ApplicationMismatch { .. } => "deviceCompareWrongApplication",
        CompareError::Session(
            SessionError::NoAnswer { .. } | SessionError::ConnectionReleased { .. },
        )
        | CompareError::Read(ReadBackError::Session(
            SessionError::NoAnswer { .. } | SessionError::ConnectionReleased { .. },
        )) => "deviceCompareNoAnswer",
        _ => "deviceCompareNotComparable",
    };
    ApiError::with_kind(StatusCode::BAD_GATEWAY, kind, error.to_string())
}
