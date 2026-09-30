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

use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};

use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::ContactableAddress;
use knx_net::commissioning::memory_download::compare_with_plan;

use crate::bus_scan::LineScanStatus;
use crate::device_download::TunnelTransport;
use crate::device_download_routes::{parse_target, prepare, PartialDto};
use crate::errors::ApiError;
use crate::SharedState;

pub fn device_compare_routes() -> Router<SharedState> {
    Router::new().route("/api/device-compare", post(compare))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompareRequest {
    address: String,
    gateway: String,
    #[serde(default)]
    partial: Option<PartialDto>,
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
}

fn conflict(message: impl Into<String>) -> ApiError {
    ApiError::with_status(StatusCode::CONFLICT, message)
}

async fn compare(
    State(state): State<SharedState>,
    Json(body): Json<CompareRequest>,
) -> Result<Json<CompareResponse>, ApiError> {
    let gateway: SocketAddrV4 = body
        .gateway
        .parse()
        .map_err(|_| ApiError::bad_request("gateway is not a host:port IPv4 address"))?;
    let target = parse_target(&body.address)?;
    let contactable =
        ContactableAddress::new(target).map_err(|e| ApiError::bad_request(e.to_string()))?;
    let partial = body.partial.map(PartialDownloadParts::from);
    // Both of `prepare`'s locks are released before it returns.
    let prepared = prepare(&state, target, partial)?;

    let Ok(programming) = state.address_programming.try_lock() else {
        return Err(conflict(
            "the gateway is in use by an individual-address programming",
        ));
    };
    if crate::device_download_routes::download_in_progress(&state) {
        return Err(conflict(
            "a download to a device is running: the gateway serves one tunnel",
        ));
    }
    if programming.as_ref().is_some_and(|p| p.is_active()) {
        return Err(conflict(
            "an individual-address programming is running: the gateway serves one tunnel",
        ));
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

    // Register only after validation and gateway-conflict checks. A dropped
    // request leaves `unknown`, never a fabricated successful comparison.
    let activity = state
        .one_shot_activity
        .start("deviceCompare", target.to_string());
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
    let compared = compare_with_plan(
        &TunnelTransport(tunnel.as_ref()),
        contactable,
        state.device_download_timing,
        &prepared.plan,
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
    let compared =
        compared.map_err(|e| ApiError::with_status(StatusCode::BAD_GATEWAY, e.to_string()))?;

    let segments = &prepared.image.segments;
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
    Ok(Json(CompareResponse {
        address: target.to_string(),
        device_name: prepared.device_name.clone(),
        program_id: prepared.request.program_id.clone(),
        written: false,
        partial: prepared.partial.is_some(),
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
    }))
}
