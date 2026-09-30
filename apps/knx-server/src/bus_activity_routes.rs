//! Read-only snapshot of the server's retained bus sessions.
//!
//! This is deliberately not a universal bus-activity ledger: one-shot
//! commands have no retained session, and a successful poll cannot reconstruct
//! an operation that began and ended between polls. `coverage: partial` and
//! `untracked` are part of the response contract, not UI decoration.
use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::bus::SessionStatus;
use crate::bus_scan::LineScanStatus;
use crate::device_download::{DownloadStatus, ProgressEvent};
use crate::{AddressProgrammingStatus, SharedState};

pub fn bus_activity_routes() -> Router<SharedState> {
    Router::new().route("/api/bus/activity", get(snapshot))
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ActivitySnapshot {
    server_incarnation: String,
    coverage: &'static str,
    /// At most one session per kind, in kind order rather than time order.
    sessions: Vec<SessionActivity>,
    /// A lock is held; the underlying route may be connecting, writing, or
    /// stopping. No target or operation type can safely be inferred from it.
    busy_locks: Vec<&'static str>,
    /// One-shot operations are never retained; their past activity is absent.
    untracked: [&'static str; 5],
}

#[derive(Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
enum SessionActivity {
    DeviceDownload {
        id: u64,
        address: String,
        state: &'static str,
        completed_steps: usize,
        total_steps: usize,
        written_octets: usize,
        total_octets: usize,
    },
    AddressProgramming {
        id: u64,
        address: String,
        state: &'static str,
        #[serde(skip_serializing_if = "Option::is_none")]
        rounds: Option<u32>,
    },
    BusMonitor {
        id: u64,
        state: &'static str,
    },
    LineScan {
        id: u64,
        state: &'static str,
        probed: usize,
        total: usize,
    },
}

async fn snapshot(State(state): State<SharedState>) -> Json<ActivitySnapshot> {
    // Never await one of the session locks: starts hold them across a gateway
    // connection attempt, and the status bar must remain responsive. A locked
    // holder is reported as unknown/busy rather than silently treated as idle.
    let mut sessions = Vec::new();
    let mut busy_locks = Vec::new();
    match state.device_download.try_lock() {
        Ok(guard) => {
            if let Some(session) = guard.as_ref() {
                let (status, _, events) = session.snapshot_since(0);
                let completed_steps = events
                    .iter()
                    .filter_map(|event| match event {
                        ProgressEvent::StepDone { number, .. } => Some(*number),
                        _ => None,
                    })
                    .max()
                    .unwrap_or(0);
                let written_octets = events
                    .iter()
                    .filter_map(|event| match event {
                        ProgressEvent::DataWritten { written, .. } => Some(*written),
                        _ => None,
                    })
                    .max()
                    .unwrap_or(0);
                let state = match status {
                    DownloadStatus::Running => "running",
                    DownloadStatus::Finished { .. } => "finished",
                    DownloadStatus::Failed { .. } => "failed",
                };
                sessions.push(SessionActivity::DeviceDownload {
                    id: session.id(),
                    address: session.target().to_string(),
                    state,
                    completed_steps,
                    total_steps: session.steps(),
                    written_octets,
                    total_octets: session.data_octets(),
                });
            }
        }
        Err(_) => busy_locks.push("deviceDownload"),
    }
    match state.address_programming.try_lock() {
        Ok(guard) => {
            if let Some(session) = guard.as_ref() {
                let (status, _, _) = session.snapshot_since(usize::MAX);
                let (state, rounds) = match status {
                    AddressProgrammingStatus::Waiting { rounds, .. } => ("waiting", Some(rounds)),
                    AddressProgrammingStatus::Programming { .. } => ("programming", None),
                    AddressProgrammingStatus::Finished { .. } => ("finished", None),
                    AddressProgrammingStatus::Stopped { rounds } => ("stopped", Some(rounds)),
                    AddressProgrammingStatus::Failed { .. } => ("failed", None),
                };
                sessions.push(SessionActivity::AddressProgramming {
                    id: session.id(),
                    address: session.new_address().to_string(),
                    state,
                    rounds,
                });
            }
        }
        Err(_) => busy_locks.push("managementOperation"),
    }
    match state.bus_session.try_lock() {
        Ok(guard) => {
            if let Some(session) = guard.as_ref() {
                sessions.push(SessionActivity::BusMonitor {
                    id: session.id(),
                    state: match session.status() {
                        SessionStatus::Active => "active",
                        SessionStatus::Closed => "closed",
                    },
                });
            }
        }
        Err(_) => busy_locks.push("busMonitorOrGroupWrite"),
    }
    match state.line_scan_session.try_lock() {
        Ok(guard) => {
            if let Some(session) = guard.as_ref() {
                let (status, probed, _) = session.snapshot_since(usize::MAX);
                let state = match status {
                    LineScanStatus::Running => "running",
                    LineScanStatus::Completed => "completed",
                    LineScanStatus::Cancelled => "cancelled",
                    LineScanStatus::Failed(_) => "failed",
                };
                sessions.push(SessionActivity::LineScan {
                    id: session.id(),
                    state,
                    probed,
                    total: session.total_count(),
                });
            }
        }
        Err(_) => busy_locks.push("lineScan"),
    }
    Json(ActivitySnapshot {
        server_incarnation: state.server_incarnation.clone(),
        coverage: "partial",
        sessions,
        busy_locks,
        untracked: [
            "deviceCompare",
            "groupWrite",
            "serialAddress",
            "serialLookup",
            "serviceControl",
        ],
    })
}
