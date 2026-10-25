//! Normalized property execution reuses one management-session operation loop.
//!
//! The concrete simulator entry remains unchanged. Separate experimental entries
//! require exact operation-bound recovery and explicit same-shape preconditions.
//! STRONG HYPOTHESIS / SIMULATOR TESTED is not physical-device compatibility,
//! allocator rollback or verification of every indirect side effect. Failure
//! preserves its source/operation position and never automatically restores.

use std::fmt;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE};
use knx_core::commissioning::property_download::{PropertyDownloadPlan, PropertyOperation};
use knx_core::IndividualAddress;

use super::hardware_release::HardwareRelease;
use super::property_recovery::{
    self, ExperimentalPropertyPreconditions, PropertyInProcedurePointerObservation,
    PropertyJournalEvent, PropertyPointerObservation, PropertyRecoverySnapshot,
};
use super::simulator::SimulatedDevice;
use super::{ManagementSession, SessionError, SessionTiming};
use crate::management::ManagementTransport;

/// One completed normalized operation, including explicit zero-wire metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletedPropertyOperation {
    /// Original source position; a data write may share its allocation's index.
    pub source_index: usize,
    /// False for tool metadata; this is not a telegram counter.
    pub wire_operation: bool,
}

/// A complete simulation; no physical-device or restore claim follows.
#[derive(Debug)]
pub struct PropertySimulationReport {
    /// Completed operations in retained order.
    pub completed: Vec<CompletedPropertyOperation>,
    /// Data octets successfully verified by the existing memory writer.
    pub data_octets: usize,
}

/// Why a simulation stopped, with actual errors instead of a guessed diagnosis.
#[derive(Debug)]
pub enum PropertySimulationFailure {
    /// The authorization points at a different target than the plan.
    TargetMismatch {
        expected: IndividualAddress,
        found: IndividualAddress,
    },
    /// The descriptor disagrees with the normalized plan.
    MaskMismatch {
        expected: MaskVersion,
        found: MaskVersion,
    },
    /// The Device Object manufacturer disagrees with the normalized plan.
    ManufacturerMismatch { expected: u16, found: u16 },
    /// Hardware type is compared as six exact bytes, without padding inference.
    HardwareMismatch { expected: [u8; 6], found: Vec<u8> },
    /// A control did not reach this plan's required state.
    UnexpectedState {
        object: u8,
        expected: LoadState,
        found: LoadState,
    },
    /// Cooperative abort, checked before each normalized operation; no restore.
    Cancelled,
    /// A synchronous caller-owned persistence barrier failed.
    Persistence(String),
    /// Operation-specific recovery acquisition or binding refused.
    Recovery(String),
    /// The existing session refused or failed an operation.
    Session(SessionError),
}

/// An incomplete simulation, not a successful download or recovered device.
#[derive(Debug)]
pub struct PropertySimulationError {
    /// None for a pre-connection authorization/target refusal.
    pub source_index: Option<usize>,
    /// Normalized offset distinguishes data from its same-source allocation.
    pub operation_index: Option<usize>,
    /// A property/memory write may have occurred, including Verify Mode setup.
    /// Connection authorization can separately change ephemeral access rights.
    pub mutation_may_have_started: bool,
    /// Every successfully completed operation before the failure.
    pub completed: Vec<CompletedPropertyOperation>,
    /// Actual cause retained for diagnosis.
    pub failure: PropertySimulationFailure,
}
impl fmt::Display for PropertySimulationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "property execution stopped at source {:?}: {:?}",
            self.source_index, self.failure
        )
    }
}
impl std::error::Error for PropertySimulationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match &self.failure {
            PropertySimulationFailure::Session(error) => Some(error),
            _ => None,
        }
    }
}
impl From<SessionError> for PropertySimulationFailure {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

/// Exercise a validated normalized plan using the existing simulated session.
///
/// Hardware transports cannot be passed, even with an operator confirmation:
/// ```compile_fail
/// use knx_core::commissioning::{authorisation::AuthorisationPlan,
///     mutation::WriteAuthorisation, property_download::PropertyDownloadPlan};
/// use knx_net::{commissioning::{property_download::run_simulated_property_plan,
///     SessionTiming}, management::ManagementTransport};
/// fn hardware_is_not_a_simulator<T: ManagementTransport>(transport: &T,
///     plan: &PropertyDownloadPlan, authorization: WriteAuthorisation) {
///     let _ = run_simulated_property_plan(transport, plan, authorization,
///         AuthorisationPlan::Skip, SessionTiming::default());
/// }
/// ```
/// No XML step, source encoding or whole-mutation recovery is inferred here.
pub async fn run_simulated_property_plan(
    device: &SimulatedDevice,
    plan: &PropertyDownloadPlan,
    authorization: WriteAuthorisation,
    key: AuthorisationPlan,
    timing: SessionTiming,
) -> Result<PropertySimulationReport, PropertySimulationError> {
    let early = |failure| PropertySimulationError {
        source_index: None,
        operation_index: None,
        mutation_may_have_started: false,
        completed: Vec::new(),
        failure,
    };
    if device.address() != plan.target() {
        return Err(early(PropertySimulationFailure::TargetMismatch {
            expected: plan.target(),
            found: device.address(),
        }));
    }
    if authorization.target().address() != plan.target() {
        return Err(early(PropertySimulationFailure::TargetMismatch {
            expected: plan.target(),
            found: authorization.target().address(),
        }));
    }
    let report = run_property_plan_inner(
        device,
        plan,
        authorization,
        key,
        timing,
        None,
        None,
        None,
        &mut |_| Ok(()),
        &|| false,
    )
    .await?;
    Ok(report.execution)
}

/// The shared mutation dispatcher for simulator, experimental execute and restore.
/// Returns written data octets and, for an allocation/task record whose pointer
/// read-back differs, that observation. The load-state result stays hard.
async fn execute_property_mutation<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    operation: &PropertyOperation,
) -> Result<(usize, Option<PropertyPointerObservation>), PropertySimulationFailure> {
    match operation {
        PropertyOperation::LoadControl { object, payload } => {
            let (found, expected) = match payload.event_octet() {
                3 => (
                    session
                        .write_allocation(ObjectIndex::new(*object), *payload)
                        .await?,
                    LoadState::Loading,
                ),
                event => {
                    let (event, expected) = match event {
                        4 => (LoadEvent::Unload, LoadState::Unloaded),
                        1 => (LoadEvent::StartLoading, LoadState::Loading),
                        2 => (LoadEvent::LoadCompleted, LoadState::Loaded),
                        _ => unreachable!("immutable validated property plan event"),
                    };
                    (
                        session
                            .write_load_event(ObjectIndex::new(*object), event)
                            .await?,
                        expected,
                    )
                }
            };
            if found != expected {
                return Err(PropertySimulationFailure::UnexpectedState {
                    object: *object,
                    expected,
                    found,
                });
            }
            let bytes = payload.octets();
            if bytes[0] == 3 && bytes[1] <= 2 {
                // An observation (owner amendment 3): ETS-programmed devices report
                // pointers the source model does not predict. Writes use the plan's
                // absolute addresses and the complete allocations are verified at the end.
                let expected = u32::from(u16::from_be_bytes([bytes[2], bytes[3]])).to_be_bytes();
                let observed =
                    property_recovery::read_pointer(session, ObjectIndex::new(*object)).await?;
                if observed != Some(expected) {
                    return Ok((
                        0,
                        Some(PropertyPointerObservation {
                            object: *object,
                            expected,
                            observed,
                            state: found,
                        }),
                    ));
                }
            }
            Ok((0, None))
        }
        PropertyOperation::WriteMemory { address, data, .. } => {
            let limit = session.write_limit(None).await?;
            session
                .write_memory_region(u32::from(*address), data, limit)
                .await?;
            Ok((data.len(), None))
        }
        _ => unreachable!("only mutating plan operations are dispatched"),
    }
}

/// Successful software execution with caller-persisted recovery evidence.
/// No physical allocator/task side-effect verification is implied.
#[derive(Debug)]
pub struct ExperimentalPropertyReport {
    pub execution: PropertySimulationReport,
    pub recovery: PropertyRecoverySnapshot,
    /// Final pointer reads that differed from the source model; observations only.
    pub final_pointer_observations: Vec<PropertyPointerObservation>,
    /// Allocation/task pointer read-backs that differed from their record.
    pub in_procedure_pointer_observations: Vec<PropertyInProcedurePointerObservation>,
}

/// Explicit opt-in experimental execution; all hardware-kind transports refuse.
///
/// Ordinary Download authorization or full write coverage never grants physical
/// source/recovery admission. A future hardware route needs a separate contract.
///
/// Requires same exact application/procedure/allocation-shape preconditions,
/// target-specific Download authorization, synchronous durable journal callbacks,
/// and cooperative cancellation. A callback's Ok promises durable sync/readback;
/// Net has no filesystem policy. Dropping this future is NOT explicit abort and
/// has no asynchronous disconnect/restore guarantee. All returned errors attempt
/// disconnect; no failure path automatically restores resources.
#[allow(clippy::too_many_arguments)]
pub async fn run_experimental_property_plan<T: ManagementTransport>(
    transport: &T,
    plan: &PropertyDownloadPlan,
    authorization: WriteAuthorisation,
    key: AuthorisationPlan,
    timing: SessionTiming,
    preconditions: &ExperimentalPropertyPreconditions,
    journal: &mut impl FnMut(PropertyJournalEvent<'_>) -> Result<(), String>,
    abort_requested: &impl Fn() -> bool,
) -> Result<ExperimentalPropertyReport, PropertySimulationError> {
    let report = run_property_plan_inner(
        transport,
        plan,
        authorization,
        key,
        timing,
        Some(preconditions),
        None,
        None,
        journal,
        abort_requested,
    )
    .await?;
    Ok(report.experimental())
}

/// Explicit opt-in experimental execution on a physical device, released by a
/// typed best-effort consent ([`HardwareRelease`]). Unwritten (masked)
/// allocation bytes are left untouched and recovery is best-effort; see
/// ADR-0152. All other requirements — same-shape preconditions, target-specific
/// Download authorization, durable journal, cooperative cancellation — are
/// identical to [`run_experimental_property_plan`], which stays simulator-only.
#[allow(clippy::too_many_arguments)]
pub async fn run_experimental_property_plan_on_hardware<T: ManagementTransport>(
    transport: &T,
    plan: &PropertyDownloadPlan,
    authorization: WriteAuthorisation,
    key: AuthorisationPlan,
    timing: SessionTiming,
    preconditions: &ExperimentalPropertyPreconditions,
    release: &HardwareRelease,
    journal: &mut impl FnMut(PropertyJournalEvent<'_>) -> Result<(), String>,
    abort_requested: &impl Fn() -> bool,
) -> Result<ExperimentalPropertyReport, PropertySimulationError> {
    let report = run_property_plan_inner(
        transport,
        plan,
        authorization,
        key,
        timing,
        Some(preconditions),
        None,
        Some(release),
        journal,
        abort_requested,
    )
    .await?;
    Ok(report.experimental())
}

/// Explicit recovery replay: complete original procedure with old memory bytes.
/// Validates persisted operation coverage before any connection and current exact
/// identity before setup. Interrupted current load states are not mistaken for old
/// command records. This remains experimental same-shape reinstallation only.
#[allow(clippy::too_many_arguments)]
pub async fn restore_experimental_property_plan<T: ManagementTransport>(
    transport: &T,
    original: &PropertyDownloadPlan,
    snapshot: &PropertyRecoverySnapshot,
    authorization: WriteAuthorisation,
    key: AuthorisationPlan,
    timing: SessionTiming,
    journal: &mut impl FnMut(PropertyJournalEvent<'_>) -> Result<(), String>,
    abort_requested: &impl Fn() -> bool,
) -> Result<ExperimentalPropertyReport, PropertySimulationError> {
    let plan = snapshot
        .recovery_plan(original)
        .map_err(|failure| PropertySimulationError {
            source_index: None,
            operation_index: None,
            mutation_may_have_started: false,
            completed: Vec::new(),
            failure,
        })?;
    let report = run_property_plan_inner(
        transport,
        &plan,
        authorization,
        key,
        timing,
        Some(&snapshot.preconditions),
        Some(snapshot),
        None,
        journal,
        abort_requested,
    )
    .await?;
    Ok(report.experimental())
}

/// Explicit recovery replay on a physical device, released by a typed
/// best-effort consent ([`HardwareRelease`]). Identical to
/// [`restore_experimental_property_plan`] except that the hardware transport is
/// admitted; unwritten (masked) bytes still rely on the captured pre-dump and
/// best-effort recovery (ADR-0152).
#[allow(clippy::too_many_arguments)]
pub async fn restore_experimental_property_plan_on_hardware<T: ManagementTransport>(
    transport: &T,
    original: &PropertyDownloadPlan,
    snapshot: &PropertyRecoverySnapshot,
    authorization: WriteAuthorisation,
    key: AuthorisationPlan,
    timing: SessionTiming,
    release: &HardwareRelease,
    journal: &mut impl FnMut(PropertyJournalEvent<'_>) -> Result<(), String>,
    abort_requested: &impl Fn() -> bool,
) -> Result<ExperimentalPropertyReport, PropertySimulationError> {
    let plan = snapshot
        .recovery_plan(original)
        .map_err(|failure| PropertySimulationError {
            source_index: None,
            operation_index: None,
            mutation_may_have_started: false,
            completed: Vec::new(),
            failure,
        })?;
    let report = run_property_plan_inner(
        transport,
        &plan,
        authorization,
        key,
        timing,
        Some(&snapshot.preconditions),
        Some(snapshot),
        Some(release),
        journal,
        abort_requested,
    )
    .await?;
    Ok(report.experimental())
}

struct PropertyPlanExecution {
    execution: PropertySimulationReport,
    recovery: Option<PropertyRecoverySnapshot>,
    final_pointer_observations: Vec<PropertyPointerObservation>,
    in_procedure_pointer_observations: Vec<PropertyInProcedurePointerObservation>,
}
impl PropertyPlanExecution {
    fn experimental(self) -> ExperimentalPropertyReport {
        ExperimentalPropertyReport {
            execution: self.execution,
            recovery: self
                .recovery
                .expect("experimental validated hardware guard captured recovery"),
            final_pointer_observations: self.final_pointer_observations,
            in_procedure_pointer_observations: self.in_procedure_pointer_observations,
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_property_plan_inner<T: ManagementTransport>(
    transport: &T,
    plan: &PropertyDownloadPlan,
    authorization: WriteAuthorisation,
    key: AuthorisationPlan,
    timing: SessionTiming,
    preconditions: Option<&ExperimentalPropertyPreconditions>,
    restore: Option<&PropertyRecoverySnapshot>,
    release: Option<&HardwareRelease>,
    journal: &mut impl FnMut(PropertyJournalEvent<'_>) -> Result<(), String>,
    abort_requested: &impl Fn() -> bool,
) -> Result<PropertyPlanExecution, PropertySimulationError> {
    let early = |failure| PropertySimulationError {
        source_index: None,
        operation_index: None,
        mutation_may_have_started: false,
        completed: Vec::new(),
        failure,
    };
    if let Some(preconditions) = preconditions {
        preconditions
            .validate(
                restore
                    .map(|snapshot| &snapshot.original_plan)
                    .unwrap_or(plan),
            )
            .map_err(early)?;
        if transport.target_kind() == knx_core::commissioning::mutation::TargetKind::Hardware {
            let Some(release) = release else {
                preconditions
                    .require_full_hardware_write_coverage()
                    .map_err(early)?;
                return Err(early(property_recovery::invalid("physical source/recovery admission is unavailable; a complete-write plan and ordinary Download authorization are not admission")));
            };
            if release.target().address() != plan.target() {
                return Err(early(PropertySimulationFailure::TargetMismatch {
                    expected: plan.target(),
                    found: release.target().address(),
                }));
            }
        }
    }
    if authorization.target().address() != plan.target() {
        return Err(early(PropertySimulationFailure::TargetMismatch {
            expected: plan.target(),
            found: authorization.target().address(),
        }));
    }
    let mut session = ManagementSession::authorised(transport, key, timing, authorization)
        .map_err(|error| early(error.into()))?;
    session
        .authorise_write(WriteScope::Download)
        .map_err(|error| early(error.into()))?;
    session.adopt_mask(plan.mask());
    let mut completed = Vec::new();
    let mut data_octets = 0;
    let mut mutation_may_have_started = false;
    let mut recovery = None;
    let mut final_pointer_observations = Vec::new();
    let mut in_procedure_pointer_observations = Vec::new();
    for (operation_index, step) in plan.steps().iter().enumerate() {
        let result: Result<(), PropertySimulationFailure> = async {
            if abort_requested() {
                return Err(PropertySimulationFailure::Cancelled);
            }
            journal(PropertyJournalEvent::OperationStarting {
                operation_index,
                source_index: step.source_index,
                mutation_may_have_started,
            })
            .map_err(PropertySimulationFailure::Persistence)?;
            if abort_requested() {
                return Err(PropertySimulationFailure::Cancelled);
            }
            match &step.operation {
                PropertyOperation::Connect => {
                    session.connect_inner(false).await?;
                    validate_plan_identity(&mut session, plan).await?;
                }
                PropertyOperation::HardwareCheck { expected } => {
                    validate_hardware(&mut session, *expected).await?;
                    if let Some(preconditions) = preconditions {
                        let snapshot = if let Some(snapshot) = restore {
                            property_recovery::validate_recovery_serial(&mut session, snapshot)
                                .await?;
                            snapshot.clone()
                        } else {
                            property_recovery::acquire(&mut session, plan, preconditions).await?
                        };
                        journal(PropertyJournalEvent::RecoveryAcquired(&snapshot))
                            .map_err(PropertySimulationFailure::Persistence)?;
                        if abort_requested() {
                            return Err(PropertySimulationFailure::Cancelled);
                        }
                        let setup_control = if restore.is_some() {
                            session.read_device_control().await?
                        } else {
                            snapshot.old_device_control
                        };
                        mutation_may_have_started = true;
                        session
                            .assert_verify_mode_from(setup_control, WriteScope::Download)
                            .await?;
                        recovery = Some(snapshot);
                    } else {
                        mutation_may_have_started = true;
                        session.assert_verify_mode().await?;
                    }
                }
                PropertyOperation::LoadControl { .. } | PropertyOperation::WriteMemory { .. } => {
                    mutation_may_have_started = true;
                    let (octets, observation) =
                        execute_property_mutation(&mut session, &step.operation).await?;
                    data_octets += octets;
                    if let Some(observation) = observation {
                        let observation = PropertyInProcedurePointerObservation {
                            operation_index,
                            observation,
                        };
                        journal(PropertyJournalEvent::InProcedurePointerObservation(
                            &observation,
                        ))
                        .map_err(PropertySimulationFailure::Persistence)?;
                        in_procedure_pointer_observations.push(observation);
                    }
                }
                PropertyOperation::ToolMetadata { .. } => {}
                PropertyOperation::Disconnect => {
                    if let Some(snapshot) = &recovery {
                        final_pointer_observations =
                            snapshot.verify_outcome(&mut session, plan).await?;
                        if !final_pointer_observations.is_empty() {
                            journal(PropertyJournalEvent::FinalPointerObservations(
                                &final_pointer_observations,
                            ))
                            .map_err(PropertySimulationFailure::Persistence)?;
                        }
                    }
                    session.disconnect().await;
                }
            }
            completed.push(CompletedPropertyOperation {
                source_index: step.source_index,
                wire_operation: !matches!(step.operation, PropertyOperation::ToolMetadata { .. }),
            });
            journal(PropertyJournalEvent::OperationCompleted {
                operation_index,
                source_index: step.source_index,
                mutation_may_have_started,
            })
            .map_err(PropertySimulationFailure::Persistence)?;
            Ok(())
        }
        .await;
        if let Err(failure) = result {
            session.disconnect().await;
            return Err(PropertySimulationError {
                source_index: Some(step.source_index),
                operation_index: Some(operation_index),
                mutation_may_have_started,
                completed,
                failure,
            });
        }
    }
    Ok(PropertyPlanExecution {
        execution: PropertySimulationReport {
            completed,
            data_octets,
        },
        recovery,
        final_pointer_observations,
        in_procedure_pointer_observations,
    })
}

async fn validate_plan_identity<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &PropertyDownloadPlan,
) -> Result<(), PropertySimulationFailure> {
    let found = session.read_mask_version().await?;
    if found != plan.mask() {
        return Err(PropertySimulationFailure::MaskMismatch {
            expected: plan.mask(),
            found,
        });
    }
    let found = session.read_manufacturer_id().await?;
    if found != plan.manufacturer() {
        return Err(PropertySimulationFailure::ManufacturerMismatch {
            expected: plan.manufacturer(),
            found,
        });
    }
    Ok(())
}
async fn validate_hardware<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    expected: [u8; 6],
) -> Result<(), PropertySimulationFailure> {
    let found = session
        .read_property(ObjectIndex::DEVICE, PID_HARDWARE_TYPE, 1, 1)
        .await?;
    if found.as_slice() != expected {
        return Err(PropertySimulationFailure::HardwareMismatch { expected, found });
    }
    Ok(())
}
