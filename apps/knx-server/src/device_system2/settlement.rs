//! Fresh project-belief settlement of a System-2 run, outside undo, never replayed on reopen.
use super::dto::Metadata;
use crate::{
    domain::{self, SettleOutcome, Settled},
    log_text::{device_label, parts_text},
    AppState,
};
use knx_app::{
    programming_state::{apply_settlement, Settlement},
    property_validation::{DownloadResult, LocalOutcome, OperationProgress},
};
use knx_core::{commissioning::pending::ProgrammingParts, DeviceId, Project};
use serde::Serialize;

/// What the terminal settlement did, as the domain reported it. Volatile: a restart, a select
/// or a reopen shows `notRecorded`, never a replay or a guess.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) enum Receipt {
    NotRecorded,
    NoMutation,
    Persisted,
    SessionOnly,
    PersistenceFailed,
    NoProject,
    OtherProject,
    DeviceMissing,
    Unavailable,
}

/// The parts a download writes into device memory; the address is the project's own.
const MEMORY: ProgrammingParts = ProgrammingParts {
    address: false,
    application: true,
    parameters: true,
    group_addresses: true,
};

/// Which end is settled: the download itself or an explicit recovery of the original state.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Run {
    Download,
    Recovery,
}

/// The pure belief decision. Only a verified, current download earns "programmed"; any
/// possible mutation otherwise invalidates the memory parts; nothing else is touched.
pub(super) fn decide(run: Run, progress: &OperationProgress, current: bool) -> Option<Settlement> {
    let mutated =
        progress.mutation_possible && (run == Run::Download || progress.restore_attempted);
    if !mutated {
        return None;
    }
    let verified = run == Run::Download
        && current
        && !progress.restore_attempted
        && progress.outcome == LocalOutcome::Finished
        && progress.download.outcome == LocalOutcome::Finished
        && progress.download.result == DownloadResult::Verified;
    Some(if verified {
        // The device answered at the project's address for the whole run.
        Settlement {
            programmed: ProgrammingParts::ALL,
            pending: ProgrammingParts::NONE,
            touched: ProgrammingParts::ALL,
        }
    } else {
        Settlement {
            programmed: ProgrammingParts::NONE,
            pending: MEMORY,
            touched: MEMORY,
        }
    })
}

/// Applies the settlement once to the project the run was bound to. `current` is evaluated
/// under the project lock against the model that will be changed: the exact bound device,
/// at the recorded address, and (download only) the current retained-source plan.
pub(super) fn settle(
    state: &AppState,
    meta: &Metadata,
    progress: &OperationProgress,
    run: Run,
    current: impl FnOnce(&Project, DeviceId) -> bool,
) -> Receipt {
    if decide(run, progress, false).is_none() {
        return Receipt::NoMutation;
    }
    // A historical operation (recovery after a restart) may act on its own acknowledged
    // hardware, but a coincident numeric project id in a new server session is not its project.
    if meta.binding.server_incarnation != state.server_incarnation {
        return Receipt::OtherProject;
    }
    let Ok(incarnation) = meta.binding.project_incarnation.parse::<u64>() else {
        return Receipt::Unavailable;
    };
    let device = DeviceId(meta.binding.device_id);
    let outcome =
        domain::settle_programming(state, incarnation, "System2Download", |project, stack| {
            let instance = project.devices.get(device)?;
            if instance.address.map(|a| a.to_string()).as_deref() != Some(meta.address.as_str()) {
                return None;
            }
            let label = device_label(instance);
            // The costly source re-check runs only for a run that could earn "programmed".
            let earns = decide(run, progress, true).is_some_and(|s| !s.programmed.is_empty());
            let is_current = earns && current(project, device);
            let settlement = decide(run, progress, is_current)?;
            let at = chrono::Utc::now();
            apply_settlement(project, stack, device, &settlement, Some(at));
            let subject = match run {
                Run::Download if !settlement.programmed.is_empty() => {
                    format!("Experimental System-2 download to {label} verified")
                }
                Run::Download => format!("Experimental System-2 download to {label} not verified"),
                Run::Recovery => format!("Experimental System-2 recovery of {label} attempted"),
            };
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
            Some(Settled {
                message: format!("{subject}: {}", parts.join("; ")),
                detail: None,
                changed: true,
            })
        });
    match outcome {
        SettleOutcome::NoProject => Receipt::NoProject,
        SettleOutcome::OtherProject => Receipt::OtherProject,
        SettleOutcome::Declined => Receipt::DeviceMissing,
        SettleOutcome::Persisted => Receipt::Persisted,
        SettleOutcome::SessionOnly => Receipt::SessionOnly,
        SettleOutcome::PersistFailed => Receipt::PersistenceFailed,
        SettleOutcome::LoggedOnly => Receipt::Unavailable,
    }
}
