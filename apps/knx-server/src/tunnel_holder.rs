//! Who holds the gateway's one tunnel, named in a structured `409` (issue #10).
//!
//! Every route that opens a tunnel refuses while another KNXBench session
//! holds one. The refusal keeps its `error` text and adds `kind:
//! "gatewayBusy"` and a `holder` the client words per kind ("Bus monitor
//! connected to …", "Line scan running (37/120)"), never a generic "busy".
//! Only KNXBench's own sessions can be named here: another client (ETS, a
//! visualisation) holding the gateway's tunnels shows up as the gateway's
//! own CONNECT_RESPONSE refusal instead (`gateway_refusal`).

use axum::http::StatusCode;
use serde::Serialize;

use std::net::SocketAddrV4;

use knx_net::BusError;

use crate::bus::{BusSession, BusSessionError, SessionStatus};
use crate::bus_scan::{LineScanSession, LineScanStatus};
use crate::errors::ApiError;
use crate::{
    AddressProgrammingSession, AddressProgrammingStatus, DeviceDownloadSession, SharedState,
};

/// The `kind` of every refusal built here.
pub const GATEWAY_BUSY: &str = "gatewayBusy";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TunnelHolder {
    DeviceDownload {
        address: String,
        state: &'static str,
    },
    /// `address` is the new individual address being programmed.
    AddressProgramming {
        address: String,
        state: &'static str,
    },
    BusMonitor {
        session_id: u64,
        gateway: String,
        state: &'static str,
    },
    LineScan {
        session_id: u64,
        probed: usize,
        total: usize,
    },
    /// A session's lock is held, so its start is still connecting (or, for
    /// `busMonitorOrGroupWrite`, a group write is being sent): nothing about
    /// its target can be read, so nothing is guessed. Same names as the
    /// activity snapshot's `busyLocks`.
    Connecting { session: &'static str },
}

/// The structured `409`: `{ error, kind: "gatewayBusy", holder }`.
pub fn gateway_busy(holder: TunnelHolder, message: impl Into<String>) -> ApiError {
    let mut details = serde_json::Map::new();
    details.insert(
        "holder".into(),
        serde_json::to_value(holder).expect("a holder always serialises"),
    );
    ApiError::with_details(StatusCode::CONFLICT, GATEWAY_BUSY, message, details)
}

/// A failed tunnel connect, structured for the client (issue #10). A
/// gateway's CONNECT_RESPONSE refusal is `gatewayRefused` with the raw
/// `gatewayStatus` and its specification name `gatewayError` (`null` for an
/// undefined code, never guessed); no answer is `gatewayTimeout`. Both carry
/// the `gateway` asked. Anything else stays a plain `502`. `error` keeps
/// `BusError`'s text, which names the refusal too.
pub fn gateway_connect_failed(gateway: SocketAddrV4, error: &BusSessionError) -> ApiError {
    let mut details = serde_json::Map::new();
    details.insert("gateway".into(), gateway.to_string().into());
    match error {
        BusSessionError::Transport(BusError::ConnectionRefused(status)) => {
            details.insert("gatewayStatus".into(), (*status).into());
            details.insert(
                "gatewayError".into(),
                knx_net::core::services::connect_refusal(*status)
                    .map(|refusal| refusal.constant)
                    .into(),
            );
            ApiError::with_details(
                StatusCode::BAD_GATEWAY,
                "gatewayRefused",
                error.to_string(),
                details,
            )
        }
        BusSessionError::Transport(BusError::Timeout) => ApiError::with_details(
            StatusCode::BAD_GATEWAY,
            "gatewayTimeout",
            error.to_string(),
            details,
        ),
        _ => ApiError::with_status(StatusCode::BAD_GATEWAY, error.to_string()),
    }
}

/// The download holding (or about to hold) the tunnel, by `try_lock` only:
/// a held lock is a start connecting right now.
pub fn download_holder(state: &SharedState) -> Option<TunnelHolder> {
    match state.device_download.try_lock() {
        Err(_) => Some(TunnelHolder::Connecting {
            session: "deviceDownload",
        }),
        Ok(guard) => guard.as_ref().and_then(download_holder_of),
    }
}

pub fn download_holder_of(session: &DeviceDownloadSession) -> Option<TunnelHolder> {
    session.is_running().then(|| TunnelHolder::DeviceDownload {
        address: session.target().to_string(),
        state: "running",
    })
}

/// The address programming holding the tunnel, by `try_lock` only.
pub fn programming_holder(state: &SharedState) -> Option<TunnelHolder> {
    match state.address_programming.try_lock() {
        Err(_) => Some(TunnelHolder::Connecting {
            session: "managementOperation",
        }),
        Ok(guard) => guard.as_ref().and_then(programming_holder_of),
    }
}

pub fn programming_holder_of(session: &AddressProgrammingSession) -> Option<TunnelHolder> {
    if !session.is_active() {
        return None;
    }
    let state = match session.status() {
        AddressProgrammingStatus::Programming { .. } => "programming",
        _ => "waiting",
    };
    Some(TunnelHolder::AddressProgramming {
        address: session.new_address().to_string(),
        state,
    })
}

/// A retained monitor session blocks a new tunnel in either state: the
/// routes refuse on `is_some()`, so the holder says which state it is in.
pub fn monitor_holder_of(session: &BusSession) -> TunnelHolder {
    TunnelHolder::BusMonitor {
        session_id: session.id(),
        gateway: session.gateway().to_string(),
        state: match session.status() {
            SessionStatus::Active => "active",
            SessionStatus::Closed => "closed",
        },
    }
}

pub fn scan_holder_of(session: &LineScanSession) -> Option<TunnelHolder> {
    let (status, probed, _) = session.snapshot_since(usize::MAX);
    (status == LineScanStatus::Running).then(|| TunnelHolder::LineScan {
        session_id: session.id(),
        probed,
        total: session.total_count(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::response::IntoResponse;

    #[tokio::test]
    async fn the_body_keeps_the_text_and_names_the_holder_by_kind() {
        let response = gateway_busy(
            TunnelHolder::LineScan {
                session_id: 4,
                probed: 37,
                total: 120,
            },
            "a line scan is running: the gateway serves one tunnel",
        )
        .into_response();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        let bytes = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            body,
            serde_json::json!({
                "error": "a line scan is running: the gateway serves one tunnel",
                "kind": "gatewayBusy",
                "holder": { "kind": "lineScan", "sessionId": 4, "probed": 37, "total": 120 }
            })
        );
    }

    #[test]
    fn a_held_lock_names_only_the_session_kind() {
        let holder = TunnelHolder::Connecting {
            session: "deviceDownload",
        };
        assert_eq!(
            serde_json::to_value(holder).unwrap(),
            serde_json::json!({ "kind": "connecting", "session": "deviceDownload" })
        );
    }
}
