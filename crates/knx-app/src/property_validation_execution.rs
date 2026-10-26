//! Concrete Net orchestration: simulator entry plus a consent-gated hardware path.
//!
//! Physical admission is available only through `execute_hardware` /
//! `restore_hardware`, which require a device- and operation-specific
//! [`HardwareRelease`] consent (ADR-0152); the simulator entry is unchanged.
use crate::{
    property_validation::{Checkpoint, FailureCode, OperationStore},
    property_validation_bundle::{
        invalid, PointerObservationRecord, PreparedBundle, RecoveryRecord,
    },
};
use knx_core::commissioning::{
    authorisation::AuthorisationPlan,
    mutation::{WriteAuthorisation, WriteScope},
    property_download::{PropertyDownloadPlan, PropertyOperation},
};
use knx_net::commissioning::{
    hardware_release::HardwareRelease,
    property_download::{
        restore_experimental_property_plan, restore_experimental_property_plan_on_hardware,
        run_experimental_property_plan, run_experimental_property_plan_on_hardware,
        PropertySimulationFailure,
    },
    property_recovery::{ExperimentalPropertyPreconditions, PropertyJournalEvent},
    simulator::SimulatedDevice,
    SessionTiming,
};
use knx_net::management::ManagementTransport;
use std::{
    io,
    sync::atomic::{AtomicBool, Ordering},
};

/// Fresh-execution-only ownership: the shared bundle borrow forbids mutation,
/// and the exclusive store borrow binds reuse to this exact unused operation.
/// Never persisted, cloned, or used instead of current-project admission.
/// Recovery serialization and replay retain their independent validators.
pub(crate) struct ValidatedHardwareExecution<'a> {
    bundle: &'a PreparedBundle,
    store: &'a mut OperationStore,
    plan: PropertyDownloadPlan,
    preconditions: ExperimentalPropertyPreconditions,
}
impl<'a> ValidatedHardwareExecution<'a> {
    pub(crate) fn new(
        bundle: &'a PreparedBundle,
        store: &'a mut OperationStore,
    ) -> io::Result<Self> {
        refuse_superseded_defaults_profile(bundle)?;
        // This remains the full retained-source reconstruction/equivalence
        // validator, not a hash-only or normalized-plan-only admission.
        let plan = bundle.plan()?;
        let preconditions = ExperimentalPropertyPreconditions::same_application_reinstallation(
            &plan,
            bundle.binding.serial,
            bundle.binding.application,
        )
        .map_err(|_| invalid("same-application recovery premise cannot represent this plan"))?;
        if bundle.binding.simulated || store.binding() != &bundle.binding {
            return Err(invalid("operation-store binding differs from bundle kind"));
        }
        store.require_fresh_execution()?;
        Ok(Self {
            bundle,
            store,
            plan,
            preconditions,
        })
    }
    pub(crate) fn bundle(&self) -> &PreparedBundle {
        self.bundle
    }
    pub(crate) fn plan(&self) -> &PropertyDownloadPlan {
        &self.plan
    }
}

/// Hardware executors supply the exact bundle/store and its validated original
/// plan, borrowed together for this invocation only (never the old-byte replay).
/// Everything except step classification follows the original callback,
/// including independent recovery validation and the durable pre-mutation barrier.
fn hardware_event(
    store: &mut OperationStore,
    bundle: &PreparedBundle,
    plan: &PropertyDownloadPlan,
    journal_event: PropertyJournalEvent<'_>,
    restoring: bool,
) -> Result<(), String> {
    if let PropertyJournalEvent::OperationStarting {
        operation_index,
        source_index,
        mutation_may_have_started,
    } = journal_event
    {
        let step = plan
            .steps()
            .get(operation_index)
            .ok_or("journal operation is outside exact plan")?;
        if step.source_index != source_index {
            return Err("journal source position differs".into());
        }
        // HardwareCheck's RecoveryAcquired barrier precedes VerifyMode. For
        // later operations record the *upcoming* possibility, not merely a
        // previous operation's result; loss of response is never harmless.
        let mutation = mutation_may_have_started
            || matches!(
                step.operation,
                PropertyOperation::LoadControl { .. }
                    | PropertyOperation::WriteMemory { .. }
                    | PropertyOperation::Disconnect
            );
        store
            .checkpoint(Checkpoint::StepStarted {
                index: operation_index,
                mutation,
            })
            .map_err(|e| e.to_string())
    } else {
        event(store, bundle, journal_event, restoring)
    }
}

/// New physical execution refuses defaults profile v1, which omits independent source data.
///
/// Fresh execution only: recovery of an existing v1 operation and every
/// simulator path stay available, so historical evidence remains reproducible.
pub fn refuse_superseded_defaults_profile(bundle: &PreparedBundle) -> io::Result<()> {
    if bundle.format == 1 && bundle.binding.profile_version == 1 {
        return Err(invalid(
            "defaults profile v1 omits independent source data outside the declared table reservation; prepare the bundle again with defaults profile v2 before a physical execution (recovery of an existing v1 operation stays available)",
        ));
    }
    Ok(())
}
/// Refuse before RestoreStarted, so an unavailable recovery appends nothing.
pub(crate) fn refuse_unavailable_recovery(
    snapshot: &knx_net::commissioning::property_recovery::PropertyRecoverySnapshot,
    bundle: &PreparedBundle,
) -> io::Result<()> {
    match snapshot.recovery_plan(&bundle.plan()?) {
        Ok(_) => Ok(()),
        Err(PropertySimulationFailure::Recovery(reason)) => {
            Err(io::Error::new(io::ErrorKind::InvalidData, reason))
        }
        Err(_) => Err(invalid("original-state recovery evidence is unusable")),
    }
}
fn check_bundle(
    bundle: &PreparedBundle,
    store: &OperationStore,
    simulated: bool,
) -> io::Result<()> {
    bundle.plan()?;
    bundle.preconditions()?;
    if bundle.binding.simulated != simulated || store.binding() != &bundle.binding {
        return Err(invalid("operation-store binding differs from bundle kind"));
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
            // VerifyMode, the first possible mutation, follows this barrier;
            // every hardware path supplies preconditions, so it always occurs.
            if restoring {
                if store.recovery() != Some(&recovery) {
                    return Err("restore snapshot differs from durable original".into());
                }
                store.reach_send_boundary()
            } else {
                store
                    .checkpoint(Checkpoint::RecoveryAcquired { recovery })
                    .and_then(|()| store.reach_send_boundary())
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
        PropertyJournalEvent::InProcedurePointerObservation(observed) => {
            store.checkpoint(Checkpoint::InProcedurePointerObservation {
                operation_index: observed.operation_index,
                observation: PointerObservationRecord::from_observation(&observed.observation),
            })
        }
        PropertyJournalEvent::FinalPointerObservations(observations) => {
            store.checkpoint(Checkpoint::FinalPointerObservations {
                observations: observations
                    .iter()
                    .map(PointerObservationRecord::from_observation)
                    .collect(),
            })
        }
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
    check_bundle(bundle, store, true)?;
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
    check_bundle(bundle, store, true)?;
    if confirmation != bundle.restore_phrase() {
        return Err(invalid("exact target/plan recovery confirmation required"));
    }
    let snapshot = store
        .recovery()
        .ok_or_else(|| invalid("no durable prewrite recovery exists"))?
        .to_snapshot(bundle)?;
    refuse_unavailable_recovery(&snapshot, bundle)?;
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
/// Consent-gated best-effort execution on a physical device (ADR-0152).
///
/// The bundle must be a [`PreparedBundle::rebind_hardware`] result carrying the
/// real device serial; the caller supplies both typed phrases — the ordinary
/// download confirmation and the best-effort release consent. The durable
/// journal and recovery acquisition are identical to the simulator path.
#[allow(clippy::too_many_arguments)]
pub async fn execute_hardware<T: ManagementTransport>(
    transport: &T,
    bundle: &PreparedBundle,
    store: &mut OperationStore,
    timing: SessionTiming,
    download_confirmation: &str,
    release_confirmation: &str,
    cancel: &AtomicBool,
) -> io::Result<()> {
    if bundle.hardware_preparation.is_some() {
        return Err(invalid(
            "configured preparation requires execute_project_hardware with fresh project admission",
        ));
    }
    let execution = ValidatedHardwareExecution::new(bundle, store)?;
    execute_hardware_inner(
        transport,
        execution,
        timing,
        download_confirmation,
        release_confirmation,
        cancel,
    )
    .await
}
pub(crate) async fn execute_hardware_inner<T: ManagementTransport>(
    transport: &T,
    execution: ValidatedHardwareExecution<'_>,
    timing: SessionTiming,
    download_confirmation: &str,
    release_confirmation: &str,
    cancel: &AtomicBool,
) -> io::Result<()> {
    let ValidatedHardwareExecution {
        bundle,
        store,
        plan,
        preconditions,
    } = execution;
    if cancel.load(Ordering::SeqCst) {
        store.checkpoint(Checkpoint::Cancelled)?;
        return Err(invalid("property validation cancelled before connection"));
    }
    let authorization = WriteAuthorisation::for_hardware(
        plan.target(),
        WriteScope::Download,
        download_confirmation,
    )
    .map_err(|_| invalid("hardware download confirmation refused"))?;
    let release = HardwareRelease::for_hardware(plan.target(), release_confirmation)
        .map_err(|_| invalid("hardware best-effort release refused"))?;
    let result = run_experimental_property_plan_on_hardware(
        transport,
        &plan,
        authorization,
        AuthorisationPlan::Skip,
        timing,
        &preconditions,
        &release,
        &mut |e| hardware_event(store, bundle, &plan, e, false),
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
            Err(io::Error::other(format!(
                "property validation failed: {error}; private protocol retained, no automatic retry or restore"
            )))
        }
    }
}
/// Consent-gated best-effort recovery replay on a physical device (ADR-0152).
/// The recovery phrase binds the exact target/plan; the download and release
/// phrases admit the physical write exactly as in `execute_hardware`.
#[allow(clippy::too_many_arguments)]
pub async fn restore_hardware<T: ManagementTransport>(
    transport: &T,
    bundle: &PreparedBundle,
    store: &mut OperationStore,
    timing: SessionTiming,
    recovery_confirmation: &str,
    download_confirmation: &str,
    release_confirmation: &str,
    cancel: &AtomicBool,
) -> io::Result<()> {
    if bundle.hardware_preparation.is_some() {
        return Err(invalid(
            "configured preparation requires restore_project_hardware",
        ));
    }
    restore_hardware_inner(
        transport,
        bundle,
        store,
        timing,
        recovery_confirmation,
        download_confirmation,
        release_confirmation,
        cancel,
    )
    .await
}
#[allow(clippy::too_many_arguments)]
pub(crate) async fn restore_hardware_inner<T: ManagementTransport>(
    transport: &T,
    bundle: &PreparedBundle,
    store: &mut OperationStore,
    timing: SessionTiming,
    recovery_confirmation: &str,
    download_confirmation: &str,
    release_confirmation: &str,
    cancel: &AtomicBool,
) -> io::Result<()> {
    check_bundle(bundle, store, false)?;
    if recovery_confirmation != bundle.restore_phrase() {
        return Err(invalid("exact target/plan recovery confirmation required"));
    }
    let snapshot = store
        .recovery()
        .ok_or_else(|| invalid("no durable prewrite recovery exists"))?
        .to_snapshot(bundle)?;
    refuse_unavailable_recovery(&snapshot, bundle)?;
    let plan = bundle.plan()?;
    let authorization = WriteAuthorisation::for_hardware(
        plan.target(),
        WriteScope::Download,
        download_confirmation,
    )
    .map_err(|_| invalid("hardware download confirmation refused"))?;
    let release = HardwareRelease::for_hardware(plan.target(), release_confirmation)
        .map_err(|_| invalid("hardware best-effort release refused"))?;
    if cancel.load(Ordering::SeqCst) {
        return Err(invalid("hardware recovery cancelled before connection"));
    }
    store.checkpoint(Checkpoint::RestoreStarted)?;
    let result = restore_experimental_property_plan_on_hardware(
        transport,
        &plan,
        &snapshot,
        authorization,
        AuthorisationPlan::Skip,
        timing,
        &release,
        // The original bundle is shared immutably and this exact store is
        // exclusively borrowed throughout restore. Its already validated plan
        // suffices for step/source/mutation classification; snapshot validation
        // and comparison against the durable original still use event(..., true).
        &mut |e| hardware_event(store, bundle, &plan, e, true),
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

#[cfg(test)]
#[path = "property_validation_execution_tests.rs"]
mod tests;
