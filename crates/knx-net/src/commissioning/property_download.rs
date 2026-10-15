//! Normalized property plans execute only against the concrete offline device simulator.
//!
//! No generic transport, public command or hardware admission is introduced.
//! A successful run is a software probe, not a source-translation or recovery
//! certificate. Failure preserves its source position and never auto-restores.

use std::fmt;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::mutation::{WriteAuthorisation, WriteScope};
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE};
use knx_core::commissioning::property_download::{PropertyDownloadPlan, PropertyOperation};
use knx_core::IndividualAddress;

use super::simulator::SimulatedDevice;
use super::{ManagementSession, SessionError, SessionTiming};

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
    /// The existing session refused or failed an operation.
    Session(SessionError),
}

/// An incomplete simulation, not a successful download or recovered device.
#[derive(Debug)]
pub struct PropertySimulationError {
    /// None for a pre-connection authorization/target refusal.
    pub source_index: Option<usize>,
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
            "property simulation stopped at source {:?}: {:?}",
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
    let mut session = ManagementSession::authorised(device, key, timing, authorization)
        .map_err(|error| early(error.into()))?;
    session
        .authorise_write(WriteScope::Download)
        .map_err(|error| early(error.into()))?;
    session.adopt_mask(plan.mask());
    let mut completed = Vec::new();
    let mut data_octets = 0;
    let mut mutation_may_have_started = false;
    for step in plan.steps() {
        let result: Result<(), PropertySimulationFailure> = async {
            match &step.operation {
                PropertyOperation::Connect => {
                    // The simulator-only entry defers setup writes until the
                    // descriptor, manufacturer and hardware bytes are checked.
                    session.connect_inner(false).await?;
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
                }
                PropertyOperation::HardwareCheck { expected } => {
                    let found = session
                        .read_property(ObjectIndex::DEVICE, PID_HARDWARE_TYPE, 1, 1)
                        .await?;
                    if found.as_slice() != expected {
                        return Err(PropertySimulationFailure::HardwareMismatch {
                            expected: *expected,
                            found,
                        });
                    }
                    mutation_may_have_started = true;
                    session.assert_verify_mode().await?;
                }
                PropertyOperation::LoadControl { object, payload } => {
                    mutation_may_have_started = true;
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
                }
                PropertyOperation::WriteMemory { address, data, .. } => {
                    let limit = session.write_limit(None).await?;
                    mutation_may_have_started = true;
                    session
                        .write_memory_region(u32::from(*address), data, limit)
                        .await?;
                    data_octets += data.len();
                }
                PropertyOperation::ToolMetadata { .. } => {}
                PropertyOperation::Disconnect => session.disconnect().await,
            }
            Ok(())
        }
        .await;
        if let Err(failure) = result {
            session.disconnect().await;
            return Err(PropertySimulationError {
                source_index: Some(step.source_index),
                mutation_may_have_started,
                completed,
                failure,
            });
        }
        completed.push(CompletedPropertyOperation {
            source_index: step.source_index,
            wire_operation: !matches!(step.operation, PropertyOperation::ToolMetadata { .. }),
        });
    }
    Ok(PropertySimulationReport {
        completed,
        data_octets,
    })
}
