//! Concrete offline Net orchestration; physical admission is deliberately absent.
use crate::{
    property_validation::{Checkpoint, FailureCode, OperationStore},
    property_validation_bundle::{invalid, PreparedBundle, RecoveryRecord},
};
use knx_core::commissioning::{
    authorisation::AuthorisationPlan,
    mutation::{WriteAuthorisation, WriteScope},
    property_download::PropertyOperation,
};
use knx_net::commissioning::{
    property_download::{
        restore_experimental_property_plan, run_experimental_property_plan,
        PropertySimulationFailure,
    },
    property_recovery::PropertyJournalEvent,
    simulator::SimulatedDevice,
    SessionTiming,
};
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

fn check_bundle(bundle: &PreparedBundle, store: &OperationStore) -> io::Result<()> {
    bundle.plan()?;
    bundle.preconditions()?;
    if !bundle.binding.simulated || store.binding() != &bundle.binding {
        return Err(invalid("simulator binding differs from operation store"));
    }
    Ok(())
}
fn event(
    store: &mut OperationStore,
    bundle: &PreparedBundle,
    event: PropertyJournalEvent<'_>,
    restoring: bool,
) -> Result<(), String> {
    let result = match event {
        PropertyJournalEvent::RecoveryAcquired(snapshot) => {
            let recovery =
                RecoveryRecord::from_snapshot(snapshot, bundle).map_err(|e| e.to_string())?;
            if restoring {
                if store.recovery() != Some(&recovery) {
                    return Err("restore snapshot differs from durable original".into());
                }
                Ok(())
            } else {
                store.checkpoint(Checkpoint::RecoveryAcquired { recovery })
            }
        }
        PropertyJournalEvent::OperationStarting {
            operation_index,
            source_index,
            mutation_may_have_started,
        } => {
            let step = bundle
                .normalized_plan
                .steps
                .get(operation_index)
                .ok_or("journal operation is outside exact plan")?;
            if step.source_index != source_index {
                return Err("journal source position differs".into());
            }
            // HardwareCheck's RecoveryAcquired barrier precedes VerifyMode. For
            // later operations record the *upcoming* possibility, not merely a
            // previous operation's result; loss of response is never harmless.
            let plan = bundle.plan().map_err(|e| e.to_string())?;
            let mutation = mutation_may_have_started
                || matches!(
                    plan.steps()[operation_index].operation,
                    PropertyOperation::LoadControl { .. }
                        | PropertyOperation::WriteMemory { .. }
                        | PropertyOperation::Disconnect
                );
            store.checkpoint(Checkpoint::StepStarted {
                index: operation_index,
                mutation,
            })
        }
        PropertyJournalEvent::OperationCompleted {
            operation_index, ..
        } => store.checkpoint(Checkpoint::StepFinished {
            index: operation_index,
        }),
    };
    result.map_err(|e| e.to_string())
}
/// Runs only a concrete SimulatedDevice, never a gateway or generic transport.
/// The caller supplies the already-established simulated source allocation shape;
/// this function does not fabricate prior source agreement by programming it.
pub async fn execute_simulated(
    device: &SimulatedDevice,
    bundle: &PreparedBundle,
    store: &mut OperationStore,
    timing: SessionTiming,
    cancel: &AtomicBool,
) -> io::Result<()> {
    check_bundle(bundle, store)?;
    if cancel.load(Ordering::SeqCst) {
        store.checkpoint(Checkpoint::Cancelled)?;
        return Err(invalid("property validation cancelled before connection"));
    }
    let plan = bundle.plan()?;
    let preconditions = bundle.preconditions()?;
    let authorization = WriteAuthorisation::for_simulator(plan.target(), WriteScope::Download)
        .map_err(|_| invalid("simulator authorization refused"))?;
    let result = run_experimental_property_plan(
        device,
        &plan,
        authorization,
        AuthorisationPlan::Skip,
        timing,
        &preconditions,
        &mut |e| event(store, bundle, e, false),
        &|| cancel.load(Ordering::SeqCst),
    )
    .await;
    match result {
        Ok(_) => store.checkpoint(Checkpoint::Finished),
        Err(error) => {
            let checkpoint = if matches!(error.failure, PropertySimulationFailure::Cancelled) {
                Checkpoint::Cancelled
            } else {
                Checkpoint::Failed {
                    code: failure_code(&error.failure),
                    local_diagnostic: format!("{error:?}"),
                }
            };
            store.checkpoint(checkpoint)?;
            Err(io::Error::other(format!("property validation failed: {error}; private protocol retained, no automatic retry or restore")))
        }
    }
}
/// Explicit once-only recovery of a *freshly reopened* operation. The restore
/// phrase cannot be used as ordinary execution consent. Disk binding/coverage is
/// checked again before Net reads the current target's exact identity.
pub async fn restore_simulated(
    device: &SimulatedDevice,
    bundle: &PreparedBundle,
    store: &mut OperationStore,
    timing: SessionTiming,
    cancel: &AtomicBool,
    confirmation: &str,
) -> io::Result<()> {
    check_bundle(bundle, store)?;
    if confirmation != bundle.restore_phrase() {
        return Err(invalid("exact target/plan recovery confirmation required"));
    }
    let snapshot = store
        .recovery()
        .ok_or_else(|| invalid("no durable prewrite recovery exists"))?
        .to_snapshot(bundle)?;
    store.checkpoint(Checkpoint::RestoreStarted)?;
    let plan = bundle.plan()?;
    let authorization = WriteAuthorisation::for_simulator(plan.target(), WriteScope::Download)
        .map_err(|_| invalid("simulator authorization refused"))?;
    let result = restore_experimental_property_plan(
        device,
        &plan,
        &snapshot,
        authorization,
        AuthorisationPlan::Skip,
        timing,
        &mut |e| event(store, bundle, e, true),
        &|| cancel.load(Ordering::SeqCst),
    )
    .await;
    match result {
        Ok(_) => store.checkpoint(Checkpoint::Restored),
        Err(error) => {
            store.checkpoint(Checkpoint::RestoreFailed {
                code: failure_code(&error.failure),
                local_diagnostic: format!("{error:?}"),
            })?;
            Err(invalid(
                "explicit restore failed; original recovery retained, no automatic retry",
            ))
        }
    }
}
fn failure_code(error: &PropertySimulationFailure) -> FailureCode {
    match error {
        PropertySimulationFailure::Cancelled => FailureCode::Cancelled,
        PropertySimulationFailure::Persistence(_) => FailureCode::Persistence,
        PropertySimulationFailure::Recovery(_) => FailureCode::IdentityOrRecovery,
        _ => FailureCode::Executor,
    }
}
