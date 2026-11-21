//! #50: reconcile an external CLI download report with the server's marks (ADR-0179).
//!
//! `POST /api/device/{id}/programming/reconcile` accepts the report a CLI
//! `--report` file holds plus the target address the run was started for.
//! Marks change only when all four matchings hold: the report names the
//! device's catalogue program and its source hash, the canonical content
//! digest of the offline-recomputed plan for that address and scope equals
//! the report's `plan.contentSha256` (counts are only diagnostics; a report
//! without the digest is refused), the target
//! resolves to exactly this device, and the snapshot binding is current —
//! and the run completed and its read-only post-download check matches the
//! plan. Anything stale, mismatched or incomplete clears nothing. The clear
//! is a bound, undoable, logged command like a manual mark (ADR-0129/
//! ADR-0173); no bus access and no extra download ever happens here.

use axum::extract::{Path, State};
use axum::routing::post;
use axum::{Json, Router};
use knx_app::commissioning_activity::{Restart, Written};
use knx_app::download_report::{DownloadReport, REPORT_FORMAT, REPORT_KIND};
use knx_app::post_download_check::{IdentityCheck, Observation, PlanLoadVerdict, ReachCheck};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::commissioning::pending::{ProgrammingChange, ProgrammingParts};
use knx_core::{Command, DeviceId, IndividualAddress};
use serde::Deserialize;

use crate::domain::{self, BoundCommandError, SnapshotBinding};
use crate::errors::ApiError;
use crate::{AppState, SharedState};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ReconcileRequest {
    report: DownloadReport,
    /// The individual address the CLI run was started for
    /// (`knx device download <target>`); the report file itself carries none.
    target: String,
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
        let target = request
            .target
            .parse::<IndividualAddress>()
            .map_err(|error| ApiError::bad_request(error.to_string()))?;
        reconcile_device(&state, &binding, DeviceId(id), target, request.report)
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
pub(crate) fn reconcile_parts(
    scope: &str,
    plan: &knx_app::download_report::ReportedPlan,
) -> Result<ProgrammingParts, String> {
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

/// Whether the report's own read-only check proves the reached device holds
/// the plan — the same rule the server's batch applies (ADR-0172 `doubt`):
/// written yes, restart settled, identity match, load states match the plan.
pub(crate) fn check_proves_run(report: &DownloadReport) -> Result<(), String> {
    let Some(outcome) = &report.outcome else {
        return Err("the report has no outcome; nothing was reconciled".into());
    };
    if !outcome.completed {
        return Err("the reported run did not complete; nothing was reconciled".into());
    }
    if outcome.written != Written::Yes {
        return Err("the reported run did not write everything; nothing was reconciled".into());
    }
    if !matches!(
        outcome.restart,
        Some(Restart::Acknowledged) | Some(Restart::NotInPlan)
    ) {
        return Err("the reported run's restart was not confirmed; nothing was reconciled".into());
    }
    let Some(check) = &report.post_download_check else {
        return Err("the report has no post-download check; an unchecked run is not proof".into());
    };
    match &check.observation {
        Observation::Attempted {
            reach: ReachCheck::Reached {
                identity: IdentityCheck::Match(matched),
            },
        } if matched.plan_load_verdict() == PlanLoadVerdict::MatchesPlan => Ok(()),
        other => Err(format!(
            "the report's post-download check does not prove the plan ({other:?}); nothing was reconciled"
        )),
    }
}

/// The program id the project resolves for the device, its source hash and
/// the device's own address, in one catalogue lease.
fn project_program(
    state: &AppState,
    project: &knx_core::Project,
    device: DeviceId,
) -> Result<(String, String, IndividualAddress), String> {
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
/// pending parts a completed, checked, matching run proves. Bound to the
/// snapshot; every mismatch refuses before any mark is touched.
pub(crate) fn reconcile_device(
    state: &AppState,
    binding: &SnapshotBinding,
    device: DeviceId,
    target: IndividualAddress,
    report: DownloadReport,
) -> Result<knx_projection::ProjectTree, BoundCommandError> {
    // Validate the report's shape before touching the project.
    if report.kind != REPORT_KIND || report.format != REPORT_FORMAT {
        return Err(BoundCommandError::Refused(format!(
            "not a KNXBench download report (format {REPORT_FORMAT})"
        )));
    }
    check_proves_run(&report).map_err(BoundCommandError::Refused)?;
    let Some(reported_digest) = report.plan.content_sha256.clone() else {
        return Err(BoundCommandError::Refused(
            "the report carries no plan content digest (it predates content binding); rerun the download with this KNXBench version. Nothing was reconciled".into(),
        ));
    };
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
        // Device match: the run's target address must resolve to exactly
        // this device — a report from a run against another device (or no
        // device) never reconciles.
        let sole = knx_app::programming_state::sole_device_at(project, target);
        if sole != Some(device) {
            return Err(BoundCommandError::Refused(format!(
                "the run's target address {target} does not resolve to this device in the project; nothing was reconciled"
            )));
        }
        let (expected_id, expected_hash, device_address) =
            project_program(state, project, device).map_err(BoundCommandError::Refused)?;
        if device_address != target {
            return Err(BoundCommandError::Refused(
                "the device's project address differs from the run's target; nothing was reconciled".into(),
            ));
        }
        // Source match: program id and ingested-file hash against the catalogue.
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
        // Plan match: recompute the current request's scoped plan offline, for
        // the run's target, and compare its canonical content digest — the same
        // function the CLI's report used. Counts do not identify content.
        let expected = plan_shape(state, project, target, &report.scope)
            .map_err(BoundCommandError::Refused)?;
        if expected.digest != reported_digest {
            let counts_equal = (expected.steps, expected.data_octets, expected.group_links)
                == (
                    report.plan.steps,
                    report.plan.data_octets,
                    report.plan.group_links,
                );
            return Err(BoundCommandError::Refused(format!(
                "the report's plan content does not match the current project state; the project changed since the run{}, nothing was reconciled",
                if counts_equal {
                    " (same size, different content)"
                } else {
                    ""
                }
            )));
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

/// What the current request would plan, in the report's scope: the content digest
/// (the authority) and the counts (diagnostics only).
struct PlanIdentity {
    digest: String,
    steps: usize,
    data_octets: usize,
    group_links: usize,
}

/// The identity of the plan the current request would run, computed offline exactly as the
/// CLI plans: a complete preparation with [`PreparedDownload::into_partial`] for a partial
/// scope.
fn plan_shape(
    state: &AppState,
    project: &knx_core::Project,
    target: IndividualAddress,
    scope: &str,
) -> Result<PlanIdentity, String> {
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
            Ok(PlanIdentity {
                digest: knx_app::download_plan_digest::plan_content_digest(&prepared),
                steps: prepared.plan.steps.len(),
                data_octets: prepared.plan.data_octets(),
                group_links: prepared.request.links.len(),
            })
        })
        .map_err(|_| "the product database is unavailable".to_string())?
}
