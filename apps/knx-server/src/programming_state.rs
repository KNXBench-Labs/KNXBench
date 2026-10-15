//! Marks devices programmed by hand, and settles the belief after a bus write ends (P3, ADR-0129).
//!
//! `POST /api/device/{id}/programming` is the user's "Mark as programmed /
//! not programmed": a bound, undoable, logged command over all four parts.
//! The settlers are handed to a download or an individual-address
//! programming when it starts and run once with its final status, before
//! that status becomes readable, so a client that sees the end and reloads
//! the project sees the settled belief.

use std::sync::atomic::Ordering;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use knx_app::device_download::PreparedDownload;
use knx_app::programming_state::{
    apply_settlement, attempted_parts, settle_address, settle_download, sole_device_at, AddressEnd,
    DownloadEnd, Settlement,
};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::commissioning::pending::{ProgrammingChange, ProgrammingParts};
use knx_core::{Command, DeviceId, IndividualAddress};
use knx_productdb::image::ImageRequest;
use knx_projection::ProjectTree;
use serde::Deserialize;

use crate::address_programming::{AddressProgrammingStatus, AddressWritten, OnProgrammingEnd};
use crate::device_download::{DownloadStatus, OnDownloadEnd, Written};
use crate::domain::{self, BoundCommandError, Settled, SnapshotBinding};
use crate::errors::ApiError;
use crate::log_text::{device_label, parts_text};
use crate::{AppState, SharedState};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MarkRequest {
    /// `true`: every part programmed; `false`: every part pending.
    programmed: bool,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
}

async fn mark(
    State(state): State<SharedState>,
    Path(id): Path<u32>,
    Json(request): Json<MarkRequest>,
) -> Result<Json<ProjectTree>, ApiError> {
    let binding = SnapshotBinding {
        server_incarnation: request.server_incarnation,
        project_incarnation: request.project_incarnation,
        snapshot_revision: request.snapshot_revision,
    };
    let command = Command::SetDeviceProgramming {
        device: DeviceId(id),
        change: ProgrammingChange::mark(ProgrammingParts::ALL, request.programmed),
    };
    domain::apply_bound_prepared(&state, &binding, "MarkDeviceProgramming", command)
        .map(Json)
        .map_err(|error| match error {
            BoundCommandError::NoProject => ApiError::bad_request("No project open"),
            BoundCommandError::Stale => ApiError::conflict(
                "programmingStale",
                "The project changed since this device was shown; refresh and mark again. Nothing was changed.",
            ),
            BoundCommandError::Refused(message) => {
                ApiError::refused("programmingRefused", message)
            }
            BoundCommandError::Failure(message) => ApiError::internal(format!(
                "The device was not marked because the change could not be persisted: {message}"
            )),
        })
}

pub fn router() -> Router<SharedState> {
    Router::new().route("/api/device/{id}/programming", post(mark))
}

/// How a download ended, for the belief. `None` while it runs.
pub(crate) fn download_end(
    status: &DownloadStatus,
    attempted: ProgrammingParts,
) -> Option<DownloadEnd> {
    Some(match status {
        DownloadStatus::Running => return None,
        DownloadStatus::Finished {
            written: Written::Yes,
            ..
        } => DownloadEnd::Completed,
        DownloadStatus::Finished {
            written: Written::Partially,
            ..
        }
        | DownloadStatus::Failed {
            written: Written::Partially | Written::Yes,
            ..
        } => DownloadEnd::Interrupted(attempted),
        DownloadStatus::Finished {
            written: Written::No,
            ..
        }
        | DownloadStatus::Failed {
            written: Written::No,
            ..
        } => DownloadEnd::NothingWritten,
    })
}

/// How an individual-address programming ended, for the belief. `None`
/// while it runs.
pub(crate) fn address_end(status: &AddressProgrammingStatus) -> Option<AddressEnd> {
    Some(match status {
        AddressProgrammingStatus::Waiting { .. } | AddressProgrammingStatus::Programming { .. } => {
            return None
        }
        AddressProgrammingStatus::Finished {
            written: AddressWritten::Yes | AddressWritten::NoNeed,
            ..
        } => AddressEnd::Programmed,
        AddressProgrammingStatus::Finished {
            written: AddressWritten::Unconfirmed,
            ..
        }
        | AddressProgrammingStatus::Failed {
            written: AddressWritten::Yes | AddressWritten::Unconfirmed,
            ..
        } => AddressEnd::Uncertain,
        AddressProgrammingStatus::Finished {
            written: AddressWritten::No,
            ..
        }
        | AddressProgrammingStatus::Failed {
            written: AddressWritten::No | AddressWritten::NoNeed,
            ..
        }
        | AddressProgrammingStatus::Stopped { .. } => AddressEnd::NothingWritten,
    })
}

/// The project's incarnation now. Read before a write is prepared, so a
/// project opened in between is never mistaken for the one written from.
pub(crate) fn incarnation(state: &AppState) -> u64 {
    state.project_incarnation.load(Ordering::Relaxed)
}

fn sentence(subject: &str, settlement: &Settlement) -> String {
    let mut parts = Vec::new();
    if !settlement.programmed.is_empty() {
        parts.push(format!(
            "{} marked programmed",
            parts_text(settlement.programmed)
        ));
    }
    if !settlement.pending.is_empty() {
        parts.push(format!(
            "{} marked not programmed",
            parts_text(settlement.pending)
        ));
    }
    if parts.is_empty() {
        parts.push("programming state left as it was".to_string());
    }
    format!("{subject}: {}", parts.join("; "))
}

/// The settler for one download, prepared from `prepared` at `incarnation`.
pub(crate) fn download_settler(
    state: &SharedState,
    incarnation: u64,
    prepared: &PreparedDownload,
) -> OnDownloadEnd {
    let state = Arc::downgrade(state);
    let device = prepared.device;
    let target = prepared.target;
    let partial = prepared.partial.as_ref().map(|(parts, _)| *parts);
    let written = prepared.request.clone();
    let plan = prepared.plan.clone();
    Box::new(move |status| {
        let Some(state) = state.upgrade() else {
            return;
        };
        after_download(
            &state,
            incarnation,
            device,
            target,
            partial,
            &written,
            &plan,
            status,
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn after_download(
    state: &AppState,
    incarnation: u64,
    device: DeviceId,
    target: IndividualAddress,
    partial: Option<PartialDownloadParts>,
    written: &ImageRequest,
    plan: &knx_core::commissioning::memory_download::MemoryDownloadPlan,
    status: &DownloadStatus,
) {
    let through = match status {
        DownloadStatus::Failed {
            stopped_in_step, ..
        } => *stopped_in_step,
        _ => Some(plan.steps.len()),
    };
    let attempted = attempted_parts(plan, partial, through);
    let Some(end) = download_end(status, attempted) else {
        return;
    };
    if end == DownloadEnd::NothingWritten {
        return;
    }
    domain::settle_programming(state, incarnation, "DeviceDownload", |project, stack| {
        let instance = project.devices.get(device)?;
        let label = device_label(instance);
        // Read under the project lock, in the same order `prepare` takes them.
        let now = state.product_db.as_ref().and_then(|products| {
            let products = products.lock().expect("state mutex poisoned");
            knx_productdb::image_request::image_request_for_device(&products, project, device).ok()
        });
        let settlement = settle_download(partial, end, written, now.as_ref());
        let subject = match end {
            DownloadEnd::Completed => format!("Download to {label} finished"),
            DownloadEnd::Interrupted(_) => {
                format!("Download to {target} stopped partway ({label})")
            }
            DownloadEnd::NothingWritten => unreachable!("returned above"),
        };
        let at = chrono::Utc::now();
        apply_settlement(project, stack, device, &settlement, Some(at));
        Some(Settled {
            message: sentence(&subject, &settlement),
            // The history changed even where the flags did not.
            changed: true,
        })
    });
}

/// The settler for one individual-address programming to `address`.
pub fn address_programming_settler(
    state: &SharedState,
    address: IndividualAddress,
) -> OnProgrammingEnd {
    let state = Arc::clone(state);
    let (incarnation, device) = {
        let project = state.project.lock().expect("state mutex poisoned");
        (
            incarnation(&state),
            project
                .as_ref()
                .and_then(|project| sole_device_at(project, address)),
        )
    };
    Box::new(move |status| after_address_programming(&state, incarnation, address, device, status))
}

fn after_address_programming(
    state: &AppState,
    incarnation: u64,
    address: IndividualAddress,
    device: Option<DeviceId>,
    status: &AddressProgrammingStatus,
) {
    let Some(end) = address_end(status) else {
        return;
    };
    if end == AddressEnd::NothingWritten {
        return;
    }
    domain::settle_programming(
        state,
        incarnation,
        "AddressProgramming",
        |project, stack| {
            let Some(device) = device.filter(|id| sole_device_at(project, address) == Some(*id))
            else {
                return Some(Settled {
                    message: format!(
                        "Individual address {address}: no unchanged, uniquely identified project device, so its programming state was left as it was"
                    ),
                    changed: false,
                });
            };
            let label = device_label(project.devices.get(device)?);
            let settlement = settle_address(end);
            apply_settlement(project, stack, device, &settlement, None);
            let subject = match end {
                AddressEnd::Programmed => {
                    format!("Individual address {address} programmed into {label}")
                }
                AddressEnd::Uncertain => {
                    format!("Individual address {address} not confirmed in {label}")
                }
                AddressEnd::NothingWritten => unreachable!("returned above"),
            };
            Some(Settled {
                message: sentence(&subject, &settlement),
                changed: true,
            })
        },
    );
}

#[cfg(test)]
mod tests;
