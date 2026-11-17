//! #50: reconcile an external CLI download with the server's project state.
//!
//! `POST /api/device/{id}/programming/reconcile` accepts a CLI download
//! report (as `--report` writes it) and clears only the pending parts the
//! report proves: a completed run whose program identity, plan size and
//! scope match the current project state. Anything stale, mismatched or
//! incomplete clears nothing; the run is a bound, undoable, logged command
//! exactly like a manual mark (ADR-0129/ADR-0173), never a blind "all done".

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::commissioning::pending::{ProgrammingChange, ProgrammingParts};
use knx_core::{Command, DeviceId};
use serde::Deserialize;

use crate::domain::{self, BoundCommandError, SnapshotBinding};
use crate::errors::ApiError;
use crate::{AppState, SharedState};

/// One field of the accepted report, bounds-checked like the receipt parser.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
// Fields are parsed for diagnostics, not read by reconciliation.
#[allow(dead_code)]
pub(crate) struct ReportProgram {
    id: String,
    source_sha256: Option<String>,
    // Retained although unread today: the report format forwards these and
    // the reconciliation log prints the raw report; dropping them would
    // silently discard diagnostic data (never silently discard information).
    #[serde(default)]
    application_version: Option<String>,
    #[serde(default)]
    manufacturer: Option<String>,
    #[serde(default)]
    mask: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReportPlan {
    steps: usize,
    data_octets: usize,
    group_links: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReportOutcome {
    completed: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct Report {
    kind: String,
    format: u32,
    scope: String,
    plan: ReportPlan,
    outcome: Option<ReportOutcome>,
    program: ReportProgram,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReconcileRequest {
    report: Report,
    server_incarnation: String,
    project_incarnation: u64,
    snapshot_revision: u64,
}

async fn reconcile(
    State(state): State<SharedState>,
    Path(id): Path<u32>,
    Json(request): Json<ReconcileRequest>,
) -> Result<Json<knx_projection::ProjectTree>, ApiError> {
    crate::routes::off_runtime(move || {
        let binding = SnapshotBinding {
            server_incarnation: request.server_incarnation,
            project_incarnation: request.project_incarnation,
            snapshot_revision: request.snapshot_revision,
        };
        reconcile_device(&state, &binding, DeviceId(id), request.report)
            .map(Json)
            .map_err(|error| match error {
                BoundCommandError::NoProject => ApiError::bad_request("No project open"),
                BoundCommandError::Stale => ApiError::conflict(
                    "programmingStale",
                    "The project changed since this device was shown; refresh and reconcile again. Nothing was changed.",
                ),
                BoundCommandError::Refused(message) => {
                    ApiError::refused("programmingRefused", message)
                }
                BoundCommandError::Failure(message) => ApiError::internal(format!(
                    "The device was not reconciled because the change could not be persisted: {message}"
                )),
            })
    })
    .await?
}

pub fn reconcile_router() -> Router<SharedState> {
    Router::new().route("/api/device/{id}/programming/reconcile", post(reconcile))
}

/// The parts a scope's completed run proves, and the report's bounds.
pub(crate) fn reconcile_parts(scope: &str, plan: &ReportPlan) -> Result<ProgrammingParts, String> {
    if plan.steps == 0 || plan.steps > u32::MAX as usize || plan.data_octets > u32::MAX as usize {
        return Err("the report's plan size is out of range".into());
    }
    Ok(match scope {
        "complete" => ProgrammingParts {
            application: true,
            parameters: true,
            group_addresses: true,
            ..ProgrammingParts::NONE
        },
        "partial-parameters" => ProgrammingParts {
            parameters: true,
            ..ProgrammingParts::NONE
        },
        "partial-group-addresses" => ProgrammingParts {
            group_addresses: true,
            ..ProgrammingParts::NONE
        },
        "partial-both" => ProgrammingParts {
            parameters: true,
            group_addresses: true,
            ..ProgrammingParts::NONE
        },
        other => return Err(format!("unsupported report scope {other:?}")),
    })
}

/// The program id the project resolves for `program_ref`, its hash and the
/// device's target address, in one catalogue lease.
fn project_program(
    state: &AppState,
    project: &knx_core::Project,
    device: DeviceId,
) -> Result<(String, String, knx_core::IndividualAddress), String> {
    let instance = project
        .devices
        .get(device)
        .ok_or_else(|| "the device is not in the open project".to_string())?;
    let Some(target) = instance.address else {
        return Err("the device has no individual address in the project".into());
    };
    let owner = state
        .product_db
        .as_ref()
        .ok_or_else(|| "no product database is available".to_string())?;
    owner
        .with_read(|conn| {
            let program_id = knx_productdb::query::resolve_program(conn, &instance.program_ref)
                .ok()
                .flatten()
                .ok_or_else(|| "the device's program is not in the catalogue".to_string())?;
            let identity = knx_productdb::query::program_identity(conn, &program_id)
                .ok()
                .flatten()
                .ok_or_else(|| "the device's program has no identity".to_string())?;
            Ok((program_id, identity.source_sha256, target))
        })
        .map_err(|_| "the product database is unavailable".to_string())?
}

/// Reconciles one device with one external report: clears exactly the
/// pending parts a completed, matching run proves. Bound to the snapshot.
pub(crate) fn reconcile_device(
    state: &AppState,
    binding: &SnapshotBinding,
    device: DeviceId,
    report: Report,
) -> Result<knx_projection::ProjectTree, BoundCommandError> {
    // Validate the report's shape before touching the project.
    if report.kind != "knxbench-download-report" || report.format != 1 {
        return Err(BoundCommandError::Refused(
            "not a KNXBench download report (format 1)".into(),
        ));
    }
    let Some(outcome) = &report.outcome else {
        return Err(BoundCommandError::Refused(
            "the report has no outcome; nothing was reconciled".into(),
        ));
    };
    if !outcome.completed {
        return Err(BoundCommandError::Refused(
            "the reported run did not complete; nothing was reconciled".into(),
        ));
    }
    let parts = reconcile_parts(&report.scope, &report.plan).map_err(BoundCommandError::Refused)?;
    let program_id = report.program.id.clone();
    let source_sha256 = report.program.source_sha256.clone();

    let command = {
        let mut lock = state.project.lock().expect("state mutex poisoned");
        let project = lock.as_mut().ok_or(BoundCommandError::NoProject)?;
        if !domain::snapshot_is_current(
            state,
            &binding.server_incarnation,
            binding.project_incarnation,
            binding.snapshot_revision,
        ) {
            return Err(BoundCommandError::Stale);
        }
        let program_match =
            project_program(state, project, device).map_err(BoundCommandError::Refused)?;
        let (expected_id, expected_hash, target) = program_match;
        if program_id != expected_id {
            return Err(BoundCommandError::Refused(format!(
                "the report names program {program_id:?}, the project uses {expected_id:?}; nothing was reconciled"
            )));
        }
        if source_sha256.as_deref() != Some(expected_hash.as_str()) {
            return Err(BoundCommandError::Refused(
                "the report's program hash does not match the catalogue; nothing was reconciled"
                    .into(),
            ));
        }
        // Plan shape of the current request, recomputed offline: same counts
        // the CLI's plan had when it ran.
        let expected = plan_shape(state, project, target, &report.scope)
            .map_err(BoundCommandError::Refused)?;
        if expected
            != (
                report.plan.steps,
                report.plan.data_octets,
                report.plan.group_links,
            )
        {
            return Err(BoundCommandError::Refused(
                "the report's plan does not match the current project state; the project changed since the run, nothing was reconciled".into(),
            ));
        }
        // The run proves these parts are on the device: mark them programmed
        // (pending is the inverse of the loaded flags).
        Command::SetDeviceProgramming {
            device,
            change: ProgrammingChange::mark(parts, true),
        }
    };
    // A reconcile is a claim KNXBench did not witness: like a manual mark it
    // forgets the parameter baseline, outside the undoable command (ADR-0173 §3).
    let forget = vec![knx_core::commissioning::parameter_baseline::BaselineEdit::remove(device)];
    domain::apply_bound_prepared_with(
        state,
        binding,
        "ReconcileDeviceProgramming",
        command,
        forget,
    )
}

/// `(steps, data_octets, group_links)` the current request would plan, in
/// the report's scope, computed offline exactly as the CLI plans: a complete
/// preparation with [`PreparedDownload::into_partial`] for a partial scope.
fn plan_shape(
    state: &AppState,
    project: &knx_core::Project,
    target: knx_core::IndividualAddress,
    scope: &str,
) -> Result<(usize, usize, usize), String> {
    let partial = match scope {
        "complete" => None,
        "partial-parameters" => Some(PartialDownloadParts {
            parameters: true,
            group_addresses: false,
        }),
        "partial-group-addresses" => Some(PartialDownloadParts {
            parameters: false,
            group_addresses: true,
        }),
        "partial-both" => Some(PartialDownloadParts {
            parameters: true,
            group_addresses: true,
        }),
        other => return Err(format!("unsupported report scope {other:?}")),
    };
    let owner = state
        .product_db
        .as_ref()
        .ok_or_else(|| "no product database is available".to_string())?;
    owner
        .with_read(|conn| {
            let mut prepared = knx_app::device_download::prepare_device_download_for_reading(
                conn, project, target,
            )
            .map_err(|e| e.to_string())?;
            if let Some(parts) = partial {
                prepared = prepared.into_partial(parts).map_err(|e| e.to_string())?;
            }
            let steps = prepared.plan.steps.len();
            let data_octets = prepared
                .plan
                .steps
                .iter()
                .filter_map(|step| match step {
                    knx_core::commissioning::memory_download::MemoryDownloadStep::WriteMemory {
                        octets,
                        ..
                    } => Some(octets.len()),
                    _ => None,
                })
                .sum();
            Ok((steps, data_octets, prepared.request.links.len()))
        })
        .map_err(|_| "the product database is unavailable".to_string())?
}
