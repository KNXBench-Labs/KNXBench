//! Observes, read-only, what a completed mask-`070nh` memory download left in the device.
//!
//! **Consumers.** The CLI's regular download and the server's download
//! worker call it after a successful transfer; `knx_app::post_download_check`
//! turns its result into the shared, payload-free record.
//!
//! **When.** A caller runs [`observe_after_download`] only after
//! [`run_memory_download`](super::memory_download::run_memory_download) (or
//! one of its siblings) returned `Ok` for the same plan, complete or partial
//! ([`derive_partial_plan`]), including a
//! [`RestartOutcome::Unconfirmed`](super::memory_download::RestartOutcome::Unconfirmed)
//! one. That run already waited out the restart: its `restart_basic_as`
//! disconnects and then waits [`SessionTiming::post_restart_disconnect_wait`]
//! (MP §3.7.3 exception (5)) before returning, and the default of that wait
//! is not shorter than [`SessionTiming::restart_responsive_again`]. This
//! module adds no wait, no timing of its own and no retry: one bounded
//! attempt, whose failure is reported as *unavailable*, never as a final
//! verdict on the download.
//!
//! **What.** A fresh [`ManagementSession::read_only`] session with no key
//! (`AuthorisationPlan::Skip`): no write path exists, no
//! `A_Authorize_Request` and no Verify Mode write goes out, nothing is
//! restarted. It connects, checks mask, manufacturer, every
//! `CompareProperty` the plan declares and the exact five-octet application
//! identity, then reads the load state of every machine the plan loads,
//! unloads or requires `Loaded` ([`MemoryDownloadStep::RequireLoaded`], which
//! a partial download checks instead of loading), and disconnects on every
//! path out after connecting, unless the connection is already gone.
//!
//! **Expected, not assumed.** Each machine's expected state comes from the
//! plan alone: its last load control, or `Loaded` for a machine the plan
//! only requires. `Unload` expects `Unloaded` and `LoadCompleted` expects
//! `Loaded`, the two settled states the download itself demands of those
//! events (its `aimed_at`); any other last control leaves the expectation
//! unresolved, and the verdict [`LoadStateVerdict::Unknown`].
//!
//! **One attempt, as evidence.** Only the first read can say whether the
//! target was reached: `T_Connect` is unconfirmed (TL §3.7). An answer or a
//! `T_ACK` without one ([`SessionError::NoAnswer`]) means reached; every
//! transmission unacknowledged ([`SessionError::ConnectionReleased`]) means
//! not reached in this attempt. A local, tunnel or far-end failure in
//! between proves neither, and stays [`Reach::Unknown`]. None of these is a
//! statement about the device beyond this one attempt.
//!
//! **What not.**
//! - No memory region is read or compared. After a restart the device may
//!   have rewritten volatile octets, so a byte comparison would mislead, and
//!   it would cost a full read of everything the plan wrote.
//! - No serial number is read. A matching identity says the device at the
//!   target has the plan's mask, manufacturer and application, and answers
//!   every `CompareProperty` the plan declares (the hardware type only if
//!   declared); it does not say it is the same physical exemplar.
//! - Whether the application *runs* is not observed: no verified run-state
//!   reader exists for this profile. `Loaded` is a load state, not a run
//!   state, and this result carries no "running" claim of any kind.
//!
//! [`derive_partial_plan`]: knx_core::commissioning::partial_memory_download::derive_partial_plan

use std::fmt;

use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::{LoadEvent, LoadState, MaskVersion};
use knx_core::commissioning::memory_download::{
    property_matches, MemoryDownloadPlan, MemoryDownloadStep,
};
use knx_core::commissioning::partial_memory_download::application_identity;
use knx_core::commissioning::properties::{ObjectIndex, PID_HARDWARE_TYPE, PID_PROGRAM_VERSION};
use knx_core::{ContactableAddress, IndividualAddress};

use super::{ManagementSession, SessionError, SessionTiming};
use crate::management::ManagementTransport;

/// What one post-download observation found.
#[derive(Debug)]
pub struct PostDownloadObservation {
    /// The device the plan was downloaded to.
    pub target: IndividualAddress,
    /// How far the observation got, and what it saw there.
    pub outcome: ObservationOutcome,
}

/// How far an observation got. Each step's result is kept apart: reaching
/// the target, its identity and every load state.
#[derive(Debug)]
pub enum ObservationOutcome {
    /// Nothing was sent.
    NotPerformed(NotPerformed),
    /// The target was not reached in this one attempt: the connection could
    /// not be opened, or every transmission of the first read went
    /// unacknowledged at the Transport Layer. Says nothing about the
    /// download, and nothing about the device beyond this attempt.
    Unreachable(SessionError),
    /// The first read failed in a way that proves neither an answer nor
    /// silence: a local send failure, dropped events, or the connection torn
    /// down by the tunnel or the far end. Whether the target was reached is
    /// unknown.
    FirstReadInconclusive(SessionError),
    /// The target was reached, but an identity read failed, so the identity
    /// is unknown.
    IdentityUnavailable {
        /// The read that failed.
        reading: IdentityReading,
        /// Its failure.
        error: SessionError,
    },
    /// The target was reached and answered something other than the plan's
    /// device. No load state was read.
    IdentityMismatch(IdentityMismatch),
    /// The target was reached, its identity matched, and the load states
    /// were read as far as they could be.
    Observed {
        /// What matched.
        identity: MatchedIdentity,
        /// One entry per machine the plan loads, unloads or requires
        /// `Loaded`, in machine order.
        load_states: Vec<LoadStateReading>,
    },
}

/// Why nothing was sent.
#[derive(Debug)]
pub enum NotPerformed {
    /// The plan's mask has no verified application-identity layout here.
    UnsupportedMask(MaskVersion),
    /// The plan has no single verified five-octet application identity.
    ApplicationUnknown,
    /// No read-only session could be built.
    Session(SessionError),
}

/// Which identity read failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityReading {
    /// The mask version.
    Mask,
    /// The manufacturer.
    Manufacturer,
    /// A `CompareProperty` the plan declares.
    Property {
        /// `ObjIdx`.
        object_index: u8,
        /// `PropId`.
        property_id: u8,
    },
    /// `PID_PROGRAM_VERSION` of the application program object.
    Application,
}

/// What the device answered instead of the plan's identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityMismatch {
    /// Another mask.
    Mask {
        /// The plan's.
        expected: MaskVersion,
        /// The device's.
        found: MaskVersion,
    },
    /// Another manufacturer.
    Manufacturer {
        /// The plan's.
        expected: u16,
        /// The device's.
        found: u16,
    },
    /// A declared property the product's `InlineData` does not accept.
    Property {
        /// `ObjIdx`.
        object_index: u8,
        /// `PropId`.
        property_id: u8,
        /// What the device answered.
        found: Vec<u8>,
    },
    /// `PID_HARDWARE_TYPE` answered with another width than six octets.
    HardwareTypeMalformed {
        /// What the device answered.
        found: Vec<u8>,
    },
    /// The application identity answer was not exactly five octets.
    ApplicationMalformed {
        /// What the device answered.
        found: Vec<u8>,
    },
    /// A complete application identity that differs from the plan's.
    Application {
        /// The plan's.
        expected: [u8; 5],
        /// The device's.
        found: [u8; 5],
    },
}

/// The identity the device answered and the plan accepted. Not an exemplar
/// match: no serial number is part of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedIdentity {
    /// The mask version.
    pub mask: MaskVersion,
    /// The manufacturer.
    pub manufacturer: u16,
    /// Every declared `CompareProperty` as `(ObjIdx, PropId, answer)`. Empty
    /// when the plan declares none: then nothing, not even the hardware
    /// type, was compared.
    pub properties: Vec<(u8, u8, Vec<u8>)>,
    /// `PID_PROGRAM_VERSION` of the application program object.
    pub application: [u8; 5],
}

/// One machine's load state: what the plan leaves it in, and what was read.
#[derive(Debug)]
pub struct LoadStateReading {
    /// The machine.
    pub machine: MemoryLoadStateMachine,
    /// The state the plan leaves the machine in, or `None` when the plan's
    /// last control on it does not settle it (module header).
    pub expected: Option<LoadState>,
    /// What the read found.
    pub state: LoadStateObservation,
}

/// A load-state read's result.
#[derive(Debug)]
pub enum LoadStateObservation {
    /// The device answered this state.
    Observed(LoadState),
    /// The read failed, was refused or could not be classified: unknown.
    Unavailable(SessionError),
    /// Not read, because an earlier load-state read failed.
    NotRead,
}

/// The observation's verdict on the load states against the plan, and only
/// on those.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadStateVerdict {
    /// Identity matched and every machine was read in the state the plan
    /// leaves it in. That state may be `Unloaded` for a machine the plan
    /// unloads, so this is no claim that anything is `Loaded`, and none
    /// that the application runs.
    MatchesPlan,
    /// Identity matched and at least one machine was read in another state
    /// than the plan leaves it in.
    DiffersFromPlan,
    /// Not enough was observed or expected to say either.
    Unknown,
}

/// Whether the observation reached its target, on the evidence of its one
/// attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reach {
    /// Nothing was sent.
    NotAttempted,
    /// The connection did not open, or no transmission of the first read
    /// was acknowledged.
    NotReached,
    /// The first read failed without evidence either way.
    Unknown,
    /// The target answered or acknowledged the first read.
    Reached,
}

impl PostDownloadObservation {
    /// Whether the target was reached, and only as far as this one attempt
    /// shows it.
    pub fn reach(&self) -> Reach {
        match self.outcome {
            ObservationOutcome::NotPerformed(_) => Reach::NotAttempted,
            ObservationOutcome::Unreachable(_) => Reach::NotReached,
            ObservationOutcome::FirstReadInconclusive(_) => Reach::Unknown,
            ObservationOutcome::IdentityUnavailable { .. }
            | ObservationOutcome::IdentityMismatch(_)
            | ObservationOutcome::Observed { .. } => Reach::Reached,
        }
    }

    /// The verdict on the load states. A mismatch, a failed read, an unread
    /// machine or an unresolved expectation never matches the plan.
    pub fn load_verdict(&self) -> LoadStateVerdict {
        let ObservationOutcome::Observed { load_states, .. } = &self.outcome else {
            return LoadStateVerdict::Unknown;
        };
        let compared = |reading: &LoadStateReading| match (reading.expected, &reading.state) {
            (Some(expected), LoadStateObservation::Observed(found)) => Some(expected == *found),
            _ => None,
        };
        if load_states
            .iter()
            .any(|reading| compared(reading) == Some(false))
        {
            LoadStateVerdict::DiffersFromPlan
        } else if !load_states.is_empty()
            && load_states
                .iter()
                .all(|reading| compared(reading) == Some(true))
        {
            LoadStateVerdict::MatchesPlan
        } else {
            LoadStateVerdict::Unknown
        }
    }
}

/// `PID_HARDWARE_TYPE` is `PDT_GENERIC_06` (*Resources* §4.3.28): six octets.
const HARDWARE_TYPE_OCTETS: usize = 6;

/// Observes what a completed download of `plan` left on `target`. See the
/// module header for when to call it and what it does not observe.
///
/// The mask family and the application identity are checked before any
/// session is built: the identity layout is verified for mask `070nh` only
/// (the same restriction as `compare_with_plan`), and without the plan's
/// own five-octet identity there is nothing to match.
pub async fn observe_after_download<T: ManagementTransport>(
    transport: &T,
    target: ContactableAddress,
    timing: SessionTiming,
    plan: &MemoryDownloadPlan,
) -> PostDownloadObservation {
    let target = target.address();
    PostDownloadObservation {
        target,
        outcome: observe(transport, target, timing, plan).await,
    }
}

async fn observe<T: ManagementTransport>(
    transport: &T,
    target: IndividualAddress,
    timing: SessionTiming,
    plan: &MemoryDownloadPlan,
) -> ObservationOutcome {
    if plan.mask.0 & 0xFFF0 != 0x0700 {
        return ObservationOutcome::NotPerformed(NotPerformed::UnsupportedMask(plan.mask));
    }
    let Some(application) = application_identity(plan) else {
        return ObservationOutcome::NotPerformed(NotPerformed::ApplicationUnknown);
    };
    let mut session =
        match ManagementSession::read_only(transport, target, AuthorisationPlan::Skip, timing) {
            Ok(session) => session,
            Err(error) => return ObservationOutcome::NotPerformed(NotPerformed::Session(error)),
        };
    // A read-only session sends no Verify Mode write; the mask only tells
    // it the device is memory-mapped (MP §3.31.2).
    session.adopt_mask(plan.mask);
    if let Err(error) = session.connect().await {
        if session.connection().is_some() {
            session.disconnect().await;
        }
        return ObservationOutcome::Unreachable(error);
    }
    let outcome = match check_identity(&mut session, plan, application).await {
        Ok(identity) => ObservationOutcome::Observed {
            identity,
            load_states: read_load_states(&mut session, plan).await,
        },
        Err(ObservationOutcome::IdentityUnavailable {
            reading: IdentityReading::Mask,
            error,
        }) => first_read_failed(error),
        Err(outcome) => outcome,
    };
    // Best effort on every path out, as everywhere else in this crate. A
    // connection the session already released (TL's A6) or lost has no
    // second `T_Disconnect` coming, as in `run_memory_download`.
    if session.connection().is_some() {
        session.disconnect().await;
    }
    outcome
}

/// What the mask read's failure, the observation's first evidence, says.
///
/// `T_Connect` is unconfirmed (TL §3.7): an open connection says nothing
/// about the device. Only an answer or a `T_ACK` says "reached", and only
/// every transmission unacknowledged says "not reached". Anything else,
/// including [`SessionError::ConnectionLost`], which a device that exists
/// may cause (MP §2.3, p. 15) and which may follow a `T_ACK`, stays unknown.
fn first_read_failed(error: SessionError) -> ObservationOutcome {
    match error {
        SessionError::NoAnswer { .. } | SessionError::MalformedDescriptor { .. } => {
            ObservationOutcome::IdentityUnavailable {
                reading: IdentityReading::Mask,
                error,
            }
        }
        SessionError::ConnectionReleased { .. } => ObservationOutcome::Unreachable(error),
        error => ObservationOutcome::FirstReadInconclusive(error),
    }
}

/// Mask, manufacturer, every declared `CompareProperty`, then the exact
/// application identity. The first answer that does not match ends it.
async fn check_identity<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
    application: [u8; 5],
) -> Result<MatchedIdentity, ObservationOutcome> {
    let unavailable =
        |reading| move |error| ObservationOutcome::IdentityUnavailable { reading, error };
    let mismatch = |mismatch| Err(ObservationOutcome::IdentityMismatch(mismatch));
    let mask = session
        .read_mask_version()
        .await
        .map_err(unavailable(IdentityReading::Mask))?;
    if mask != plan.mask {
        return mismatch(IdentityMismatch::Mask {
            expected: plan.mask,
            found: mask,
        });
    }
    let manufacturer = session
        .read_manufacturer_id()
        .await
        .map_err(unavailable(IdentityReading::Manufacturer))?;
    if manufacturer != plan.manufacturer {
        return mismatch(IdentityMismatch::Manufacturer {
            expected: plan.manufacturer,
            found: manufacturer,
        });
    }
    let mut properties = Vec::new();
    for step in &plan.steps {
        let MemoryDownloadStep::CompareProperty {
            object_index,
            property_id,
            inline_data,
        } = step
        else {
            continue;
        };
        let (object_index, property_id) = (*object_index, *property_id);
        if is_application_identity(object_index, property_id) {
            // Checked below, exactly rather than by the padding rule.
            continue;
        }
        let found = session
            .read_property(ObjectIndex::new(object_index), property_id, 1, 1)
            .await
            .map_err(unavailable(IdentityReading::Property {
                object_index,
                property_id,
            }))?;
        if object_index == ObjectIndex::DEVICE.octet()
            && property_id == PID_HARDWARE_TYPE
            && found.len() != HARDWARE_TYPE_OCTETS
        {
            return mismatch(IdentityMismatch::HardwareTypeMalformed { found });
        }
        if !property_matches(&found, inline_data) {
            return mismatch(IdentityMismatch::Property {
                object_index,
                property_id,
                found,
            });
        }
        properties.push((object_index, property_id, found));
    }
    let found = session
        .read_property(ObjectIndex::APPLICATION_PROGRAM, PID_PROGRAM_VERSION, 1, 1)
        .await
        .map_err(unavailable(IdentityReading::Application))?;
    let found: [u8; 5] = match found.try_into() {
        Ok(found) => found,
        Err(found) => return mismatch(IdentityMismatch::ApplicationMalformed { found }),
    };
    if found != application {
        return mismatch(IdentityMismatch::Application {
            expected: application,
            found,
        });
    }
    Ok(MatchedIdentity {
        mask,
        manufacturer,
        properties,
        application: found,
    })
}

fn is_application_identity(object_index: u8, property_id: u8) -> bool {
    object_index == ObjectIndex::APPLICATION_PROGRAM.octet() && property_id == PID_PROGRAM_VERSION
}

/// One read per machine, in machine order. The first failure ends the
/// reads: the machines after it are `NotRead`, the ones before it kept.
async fn read_load_states<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &MemoryDownloadPlan,
) -> Vec<LoadStateReading> {
    let mut failed = false;
    let mut readings = Vec::new();
    for (machine, expected) in expected_states(plan) {
        let state = if failed {
            LoadStateObservation::NotRead
        } else {
            match session.read_memory_load_state(machine).await {
                Ok(state) => LoadStateObservation::Observed(state),
                Err(error) => {
                    failed = true;
                    LoadStateObservation::Unavailable(error)
                }
            }
        };
        readings.push(LoadStateReading {
            machine,
            expected,
            state,
        });
    }
    readings
}

/// Every machine the plan loads, unloads or requires `Loaded`, in machine
/// order, with the state the plan leaves it in. The last step on a machine
/// decides, as it decides the download's own `final_states`.
fn expected_states(plan: &MemoryDownloadPlan) -> Vec<(MemoryLoadStateMachine, Option<LoadState>)> {
    let mut expected: Vec<(MemoryLoadStateMachine, Option<LoadState>)> = Vec::new();
    for step in &plan.steps {
        let (machine, state) = match step {
            // The download stops unless the machine is `Loaded`, and a
            // later load control on it overrides this below.
            MemoryDownloadStep::RequireLoaded(machine) => (*machine, Some(LoadState::Loaded)),
            MemoryDownloadStep::LoadRecord(record) => {
                (record.machine(), settled_by(record.event()))
            }
            _ => continue,
        };
        expected.retain(|(earlier, _)| *earlier != machine);
        expected.push((machine, state));
    }
    expected.sort_by_key(|(machine, _)| *machine);
    expected
}

/// The settled state the download demands after `event` (its `aimed_at`),
/// or `None` for a control whose target, `Loading`, is a state passed
/// through: what a restart makes of it is not the plan's to say.
fn settled_by(event: LoadEvent) -> Option<LoadState> {
    match event {
        LoadEvent::Unload => Some(LoadState::Unloaded),
        LoadEvent::LoadCompleted => Some(LoadState::Loaded),
        LoadEvent::StartLoading | LoadEvent::AdditionalLoadControls | LoadEvent::NoOperation => {
            None
        }
    }
}

impl fmt::Display for PostDownloadObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let target = self.target;
        match &self.outcome {
            ObservationOutcome::NotPerformed(why) => write!(
                f,
                "{target}: post-download observation not performed: {why}"
            )?,
            ObservationOutcome::Unreachable(error) => write!(
                f,
                "{target}: not reached in one read-only attempt ({error}); the load is unknown"
            )?,
            ObservationOutcome::FirstReadInconclusive(error) => write!(
                f,
                "{target}: the first read failed without evidence either way ({error}); \
                 whether the target was reached, its identity and the load are unknown"
            )?,
            ObservationOutcome::IdentityUnavailable { reading, error } => write!(
                f,
                "{target}: reached, but the {reading} could not be read ({error}); \
                 identity and load unknown"
            )?,
            ObservationOutcome::IdentityMismatch(mismatch) => write!(
                f,
                "{target}: reached, but {mismatch}; no load state was read"
            )?,
            ObservationOutcome::Observed {
                identity,
                load_states,
            } => {
                write!(
                    f,
                    "{target}: mask {:04X}h, manufacturer {:04X}h, ",
                    identity.mask.0, identity.manufacturer
                )?;
                let hardware = identity.properties.iter().find(|(object, property, _)| {
                    *object == ObjectIndex::DEVICE.octet() && *property == PID_HARDWARE_TYPE
                });
                match hardware {
                    Some((_, _, found)) => write!(f, "hardware type {found:02X?}, ")?,
                    None => f.write_str("hardware type not compared, ")?,
                }
                write!(
                    f,
                    "application {:02X?} match the plan (no serial number read, so not \
                     proven the same exemplar)",
                    identity.application
                )?;
                for reading in load_states {
                    match reading.expected {
                        Some(expected) => {
                            write!(f, "; {}: expected {expected:?}", reading.machine)?
                        }
                        None => write!(f, "; {}: expected state unresolved", reading.machine)?,
                    }
                    write!(f, ", observed {}", reading.state)?;
                }
            }
        }
        f.write_str("; whether the application runs was not observed")
    }
}

impl fmt::Display for NotPerformed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedMask(mask) => write!(
                f,
                "no verified application identity layout for mask {:04X}h",
                mask.0
            ),
            Self::ApplicationUnknown => {
                f.write_str("the plan has no verified application identity")
            }
            Self::Session(error) => write!(f, "{error}"),
        }
    }
}

impl fmt::Display for IdentityReading {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mask => f.write_str("mask version"),
            Self::Manufacturer => f.write_str("manufacturer"),
            Self::Property {
                object_index,
                property_id,
            } => write!(f, "property {object_index}/{property_id}"),
            Self::Application => f.write_str("application identity"),
        }
    }
}

impl fmt::Display for IdentityMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Mask { expected, found } => write!(
                f,
                "the device has mask {:04X}h, the plan is for {:04X}h",
                found.0, expected.0
            ),
            Self::Manufacturer { expected, found } => write!(
                f,
                "the device is from manufacturer {found:04X}h, the plan is for {expected:04X}h"
            ),
            Self::Property {
                object_index,
                property_id,
                found,
            } => write!(
                f,
                "property {object_index}/{property_id} is {found:02X?}, which the product does \
                 not accept"
            ),
            Self::HardwareTypeMalformed { found } => write!(
                f,
                "the hardware type has {} octets instead of {HARDWARE_TYPE_OCTETS}",
                found.len()
            ),
            Self::ApplicationMalformed { found } => write!(
                f,
                "the application identity has {} octets instead of five",
                found.len()
            ),
            Self::Application { expected, found } => write!(
                f,
                "another application: expected {expected:02X?}, found {found:02X?}"
            ),
        }
    }
}

impl fmt::Display for LoadStateObservation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Observed(state) => write!(f, "{state:?}"),
            Self::Unavailable(error) => write!(f, "unknown ({error})"),
            Self::NotRead => f.write_str("unknown (not read after an earlier failure)"),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use knx_core::commissioning::load_control_memory::{
        abs_data_segment, abs_task_segment, event_record, AbsoluteSegment, SegmentMemoryType,
        TaskSegment,
    };
    use knx_core::commissioning::load_state::LoadEvent;
    use knx_core::commissioning::memory_download::{machines, MemoryDownloadStep};
    use knx_core::commissioning::mutation::{TargetKind, WriteAuthorisation, WriteScope};
    use knx_core::commissioning::partial_memory_download::{
        derive_partial_plan, PartialDownloadParts,
    };
    use knx_core::commissioning::properties::{
        ObjectIndex, PID_HARDWARE_TYPE, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION,
    };
    use tokio::sync::broadcast;

    use super::*;
    use crate::cemi::{ApplicationService, Destination, Tpci};
    use crate::client::{BusError, TunnelEvent};
    use crate::commissioning::memory_download::{
        run_memory_download, MemoryDownloadReport, RestartOutcome,
    };
    use crate::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};

    use MemoryLoadStateMachine::{AddressTable, ApplicationProgram, AssociationTable};

    const MANUFACTURER: u16 = 0x0083;
    const HARDWARE: [u8; 6] = [0, 0, 0, 0, 0x01, 0x27];
    const APPLICATION: [u8; 5] = [0x00, 0x83, 0x00, 0x27, 0x15];

    fn fast() -> SessionTiming {
        SessionTiming {
            connection_timeout: Duration::from_millis(50),
            response_timeout: Duration::from_millis(50),
            answer_timeout: None,
            poll_interval: Duration::from_millis(1),
            max_transition: Duration::from_millis(40),
            programming_delay: Duration::from_millis(0),
            restart_basic_t1: Duration::from_millis(1),
            restart_responsive_again: Duration::from_millis(5),
            post_restart_disconnect_wait: Duration::from_millis(5),
            programming_mode_broadcast_timeout: Duration::from_millis(20),
        }
    }

    /// A BIM M112-like device with the plan's identity and every machine
    /// still `Unloaded`: the state before any download.
    fn unloaded_device(config: SimulatorConfig) -> SimulatedDevice {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0701,
            ..config
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
        device.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
        for machine in MemoryLoadStateMachine::ALL {
            preset(&device, machine, LoadState::Unloaded);
        }
        device
    }

    /// [`unloaded_device`] in the state [`plan`] leaves it in: the address
    /// table and the application `Loaded`, the other machines untouched.
    fn loaded_device(config: SimulatorConfig) -> SimulatedDevice {
        let device = unloaded_device(config);
        preset(&device, AddressTable, LoadState::Loaded);
        preset(&device, ApplicationProgram, LoadState::Loaded);
        device
    }

    fn preset(device: &SimulatedDevice, machine: MemoryLoadStateMachine, state: LoadState) {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), state);
    }

    fn state_of(device: &SimulatedDevice, machine: MemoryLoadStateMachine) -> LoadState {
        device.load_state(ObjectIndex::new(machine.type_number()))
    }

    fn contactable(device: &SimulatedDevice) -> ContactableAddress {
        ContactableAddress::new(device.address()).expect("not excluded")
    }

    fn record(machine: MemoryLoadStateMachine, event: LoadEvent) -> MemoryDownloadStep {
        MemoryDownloadStep::LoadRecord(event_record(machine, event).expect("an event record"))
    }

    fn segment(start: u16, length: u16) -> AbsoluteSegment {
        AbsoluteSegment {
            start,
            length,
            access: 0xFF,
            memory_type: SegmentMemoryType::Eeprom,
            checksum_control: true,
        }
    }

    /// A complete download of the address table and the application, with
    /// the hardware check the product declares (ten octets, zero padded)
    /// and the application identity in the task segment.
    fn plan() -> MemoryDownloadPlan {
        MemoryDownloadPlan {
            mask: MaskVersion(0x0701),
            manufacturer: MANUFACTURER,
            steps: vec![
                MemoryDownloadStep::Connect,
                MemoryDownloadStep::CompareProperty {
                    object_index: 0,
                    property_id: PID_HARDWARE_TYPE,
                    inline_data: [&HARDWARE[..], &[0, 0, 0, 0]].concat(),
                },
                record(AddressTable, LoadEvent::Unload),
                record(ApplicationProgram, LoadEvent::Unload),
                record(AddressTable, LoadEvent::StartLoading),
                MemoryDownloadStep::LoadRecord(
                    abs_data_segment(AddressTable, segment(0x4000, 4)).expect("a segment record"),
                ),
                MemoryDownloadStep::WriteMemory {
                    address: 0x4000,
                    octets: vec![0x01, 0x11, 0x43, 0x00],
                },
                record(AddressTable, LoadEvent::LoadCompleted),
                record(ApplicationProgram, LoadEvent::StartLoading),
                MemoryDownloadStep::LoadRecord(
                    abs_data_segment(ApplicationProgram, segment(0x4400, 8))
                        .expect("a segment record"),
                ),
                MemoryDownloadStep::WriteMemory {
                    address: 0x4400,
                    octets: vec![0xA5; 8],
                },
                MemoryDownloadStep::LoadRecord(abs_task_segment(
                    ApplicationProgram,
                    TaskSegment {
                        start: 0x4400,
                        pei_type: 1,
                        manufacturer: MANUFACTURER,
                        application: 0x0027,
                        version: 0x15,
                    },
                )),
                record(ApplicationProgram, LoadEvent::LoadCompleted),
                MemoryDownloadStep::Restart,
                MemoryDownloadStep::Disconnect,
            ],
        }
    }

    /// [`plan`] that also unloads the association table and leaves it
    /// unloaded: three machines, one of them expected `Unloaded`.
    fn three_machine_plan() -> MemoryDownloadPlan {
        let mut plan = plan();
        plan.steps
            .insert(4, record(AssociationTable, LoadEvent::Unload));
        plan
    }

    /// The real partial download [`derive_partial_plan`] makes of [`plan`].
    fn partial_plan(parameters: bool, group_addresses: bool) -> MemoryDownloadPlan {
        derive_partial_plan(
            &plan(),
            PartialDownloadParts {
                parameters,
                group_addresses,
            },
        )
        .expect("a partial plan")
        .plan
    }

    async fn download(device: &SimulatedDevice, plan: &MemoryDownloadPlan) -> MemoryDownloadReport {
        let authorisation =
            WriteAuthorisation::for_simulator(device.address(), WriteScope::Download)
                .expect("not excluded");
        let mut session =
            ManagementSession::authorised(device, AuthorisationPlan::Skip, fast(), authorisation)
                .expect("a simulator session");
        run_memory_download(&mut session, plan)
            .await
            .expect("the download runs")
    }

    async fn observe(
        device: &SimulatedDevice,
        plan: &MemoryDownloadPlan,
    ) -> PostDownloadObservation {
        observe_after_download(device, contactable(device), fast(), plan).await
    }

    fn mutations(seen: &[Seen]) -> Vec<Seen> {
        seen.iter()
            .filter(|seen| {
                matches!(
                    seen,
                    Seen::MemoryWrite { .. }
                        | Seen::PropertyWrite { .. }
                        | Seen::Restart { .. }
                        | Seen::Authorize(_)
                )
            })
            .cloned()
            .collect()
    }

    fn memory_reads(seen: &[Seen]) -> Vec<u32> {
        seen.iter()
            .filter_map(|seen| match seen {
                Seen::MemoryRead { address, .. } => Some(*address),
                _ => None,
            })
            .collect()
    }

    fn property_reads(seen: &[Seen]) -> Vec<(u8, u8)> {
        seen.iter()
            .filter_map(|seen| match seen {
                Seen::PropertyRead {
                    object_index,
                    property_id,
                } => Some((*object_index, *property_id)),
                _ => None,
            })
            .collect()
    }

    fn load_states(observation: &PostDownloadObservation) -> &[LoadStateReading] {
        match &observation.outcome {
            ObservationOutcome::Observed { load_states, .. } => load_states,
            other => panic!("not observed: {other:?}"),
        }
    }

    /// `(machine, expected, observed)` of every reading that was observed.
    fn summary(
        observation: &PostDownloadObservation,
    ) -> Vec<(MemoryLoadStateMachine, Option<LoadState>, Option<LoadState>)> {
        load_states(observation)
            .iter()
            .map(|reading| {
                let observed = match reading.state {
                    LoadStateObservation::Observed(state) => Some(state),
                    _ => None,
                };
                (reading.machine, reading.expected, observed)
            })
            .collect()
    }

    /// Every state the download's own report recorded is the expectation
    /// the observation derived from the plan alone.
    fn assert_expected_agrees_with(
        observation: &PostDownloadObservation,
        report: &MemoryDownloadReport,
    ) {
        for (machine, state) in &report.final_states {
            let reading = load_states(observation)
                .iter()
                .find(|reading| reading.machine == *machine)
                .unwrap_or_else(|| panic!("the {machine} was not observed"));
            assert_eq!(reading.expected, Some(*state), "{machine}");
        }
    }

    /// Asserts the frames from `before` on only read: no write, restart,
    /// key or Verify Mode.
    fn assert_read_only(device: &SimulatedDevice, before: usize) {
        let seen = device.seen()[before..].to_vec();
        assert_eq!(mutations(&seen), vec![], "the observation only reads");
        assert_eq!(device.authorize_requests(), 0, "no key is ever sent");
        assert!(!device.verify_mode(), "no Verify Mode");
    }

    /// [`assert_read_only`], and the observation opened its own connection
    /// and closed it on the way out.
    fn assert_read_only_and_closed(device: &SimulatedDevice, before: usize) {
        assert_read_only(device, before);
        let seen = device.seen()[before..].to_vec();
        assert_eq!(seen.first(), Some(&Seen::Connect), "a fresh connection");
        assert_eq!(
            seen.last(),
            Some(&Seen::Disconnect),
            "closed on the way out"
        );
    }

    /// What a [`Recording`] transport does to the observation's first read.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Fault {
        /// Nothing: frames pass through, and are counted.
        None,
        /// The first `T_Data_Connected` never leaves: the local send fails.
        SendFails,
        /// The first read's event receiver has lagged before anything arrives.
        EventsLag,
        /// The tunnel reports itself closed during the first read.
        TunnelCloses,
    }

    /// Passes everything to a simulated device, counts the client's
    /// `T_Disconnect` frames, and injects one [`Fault`] into the first read.
    struct Recording<'a> {
        device: &'a SimulatedDevice,
        fault: Fault,
        subscriptions: Mutex<u32>,
        numbered_sends: Mutex<u32>,
        disconnects: Mutex<u32>,
    }

    impl<'a> Recording<'a> {
        fn new(device: &'a SimulatedDevice, fault: Fault) -> Self {
            Self {
                device,
                fault,
                subscriptions: Mutex::new(0),
                numbered_sends: Mutex::new(0),
                disconnects: Mutex::new(0),
            }
        }

        fn disconnects(&self) -> u32 {
            *self.disconnects.lock().expect("not poisoned")
        }

        async fn observe(&self, plan: &MemoryDownloadPlan) -> PostDownloadObservation {
            observe_after_download(self, contactable(self.device), fast(), plan).await
        }
    }

    impl ManagementTransport for Recording<'_> {
        fn assigned_address(&self) -> IndividualAddress {
            self.device.assigned_address()
        }

        fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
            let nth = {
                let mut count = self.subscriptions.lock().expect("not poisoned");
                *count += 1;
                *count
            };
            // The first subscription is `T_Connect`'s, the second the
            // first read's.
            let queued = match (nth, self.fault) {
                (2, Fault::EventsLag) => 2,
                (2, Fault::TunnelCloses) => 1,
                _ => return self.device.subscribe(),
            };
            let (sender, receiver) = broadcast::channel(1);
            for _ in 0..queued {
                let _ = sender.send(TunnelEvent::Closed);
            }
            receiver
        }

        async fn send_frame(
            &self,
            destination: Destination,
            transport: Tpci,
            service: ApplicationService,
        ) -> Result<(), BusError> {
            match transport {
                Tpci::Disconnect => *self.disconnects.lock().expect("not poisoned") += 1,
                Tpci::NumberedData { .. } => {
                    let first = {
                        let mut count = self.numbered_sends.lock().expect("not poisoned");
                        *count += 1;
                        *count == 1
                    };
                    if first && self.fault == Fault::SendFails {
                        return Err(BusError::Timeout);
                    }
                }
                _ => {}
            }
            self.device
                .send_frame(destination, transport, service)
                .await
        }

        fn target_kind(&self) -> TargetKind {
            self.device.target_kind()
        }
    }

    #[tokio::test]
    async fn a_download_then_an_observation_of_the_same_device_reads_it_loaded() {
        // Seeded with the old state, so that only the download can produce
        // what the observation reads.
        let device = unloaded_device(SimulatorConfig::default());
        let report = download(&device, &plan()).await;
        assert!(report.restart.is_confirmed(), "{}", report.restart);
        assert!(device.memory_was_written(), "the download wrote");
        assert_eq!(state_of(&device, AddressTable), LoadState::Loaded);
        assert_eq!(state_of(&device, ApplicationProgram), LoadState::Loaded);
        let before = device.seen().len();
        let memory_before = device.memory(0x4000, 0x500);

        let observation = observe(&device, &plan()).await;

        assert_eq!(observation.target, device.address());
        assert_eq!(observation.reach(), Reach::Reached);
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::MatchesPlan,
            "{observation}"
        );
        let ObservationOutcome::Observed { identity, .. } = &observation.outcome else {
            panic!("not observed: {observation:?}");
        };
        assert_eq!(
            identity,
            &MatchedIdentity {
                mask: MaskVersion(0x0701),
                manufacturer: MANUFACTURER,
                properties: vec![(0, PID_HARDWARE_TYPE, HARDWARE.to_vec())],
                application: APPLICATION,
            }
        );
        assert_eq!(
            summary(&observation),
            vec![
                (
                    AddressTable,
                    Some(LoadState::Loaded),
                    Some(LoadState::Loaded)
                ),
                (
                    ApplicationProgram,
                    Some(LoadState::Loaded),
                    Some(LoadState::Loaded)
                ),
            ]
        );
        assert_expected_agrees_with(&observation, &report);
        assert_read_only_and_closed(&device, before);
        let seen = device.seen()[before..].to_vec();
        assert_eq!(
            memory_reads(&seen),
            vec![0xB6EA, 0xB6EC],
            "load states only, no region read"
        );
        assert_eq!(device.memory(0x4000, 0x500), memory_before);
        let text = observation.to_string();
        assert!(text.contains("not observed"), "{text}");
        assert!(text.contains("no serial number"), "{text}");
        assert!(!text.contains("running"), "no run-state claim: {text}");
    }

    #[tokio::test]
    async fn an_unconfirmed_restart_then_an_observation_of_the_same_device() {
        let device = unloaded_device(SimulatorConfig {
            restart_unanswered: true,
            ..SimulatorConfig::default()
        });
        let report = download(&device, &plan()).await;
        assert!(
            matches!(report.restart, RestartOutcome::Unconfirmed { .. }),
            "{}",
            report.restart
        );
        let before = device.seen().len();

        let observation = observe(&device, &plan()).await;

        assert_eq!(observation.reach(), Reach::Reached);
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::MatchesPlan,
            "{observation}"
        );
        assert_expected_agrees_with(&observation, &report);
        assert_read_only_and_closed(&device, before);
        assert_eq!(memory_reads(&device.seen()[before..]), vec![0xB6EA, 0xB6EC]);
        assert!(observation.to_string().contains("not observed"));
    }

    #[tokio::test]
    async fn a_parameters_only_partial_download_is_observed_with_the_tables_it_requires() {
        let device = unloaded_device(SimulatorConfig::default());
        download(&device, &plan()).await;
        let partial = partial_plan(true, false);
        assert!(
            !machines(&partial).contains(&AddressTable),
            "the fixture's point: the address table is only required, not loaded"
        );
        let report = download(&device, &partial).await;
        let before = device.seen().len();

        let observation = observe(&device, &partial).await;

        assert_eq!(
            summary(&observation),
            vec![
                (
                    AddressTable,
                    Some(LoadState::Loaded),
                    Some(LoadState::Loaded)
                ),
                (
                    ApplicationProgram,
                    Some(LoadState::Loaded),
                    Some(LoadState::Loaded)
                ),
            ]
        );
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::MatchesPlan,
            "{observation}"
        );
        assert_expected_agrees_with(&observation, &report);
        assert_read_only_and_closed(&device, before);
        assert_eq!(memory_reads(&device.seen()[before..]), vec![0xB6EA, 0xB6EC]);
    }

    #[tokio::test]
    async fn a_group_address_only_partial_download_still_reads_the_application_it_requires() {
        let device = unloaded_device(SimulatorConfig::default());
        download(&device, &plan()).await;
        let partial = partial_plan(false, true);
        assert!(
            !machines(&partial).contains(&ApplicationProgram),
            "the fixture's point: the application is only required, not loaded"
        );
        let report = download(&device, &partial).await;
        assert_eq!(state_of(&device, ApplicationProgram), LoadState::Loaded);
        // The application the partial download relied on is gone by the
        // time the observation reads.
        preset(&device, ApplicationProgram, LoadState::Unloaded);
        let before = device.seen().len();

        let observation = observe(&device, &partial).await;

        assert_eq!(
            summary(&observation),
            vec![
                (
                    AddressTable,
                    Some(LoadState::Loaded),
                    Some(LoadState::Loaded)
                ),
                (
                    ApplicationProgram,
                    Some(LoadState::Loaded),
                    Some(LoadState::Unloaded)
                ),
            ]
        );
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::DiffersFromPlan,
            "{observation}"
        );
        assert_expected_agrees_with(&observation, &report);
        assert_read_only_and_closed(&device, before);
    }

    #[tokio::test]
    async fn a_machine_the_plan_only_unloads_matches_the_plan_when_read_unloaded() {
        let device = unloaded_device(SimulatorConfig::default());
        // The old association table, which the plan unloads for good.
        preset(&device, AssociationTable, LoadState::Loaded);
        let report = download(&device, &three_machine_plan()).await;
        assert_eq!(state_of(&device, AssociationTable), LoadState::Unloaded);
        let before = device.seen().len();

        let observation = observe(&device, &three_machine_plan()).await;

        assert_eq!(
            summary(&observation),
            vec![
                (
                    AddressTable,
                    Some(LoadState::Loaded),
                    Some(LoadState::Loaded)
                ),
                (
                    AssociationTable,
                    Some(LoadState::Unloaded),
                    Some(LoadState::Unloaded)
                ),
                (
                    ApplicationProgram,
                    Some(LoadState::Loaded),
                    Some(LoadState::Loaded)
                ),
            ]
        );
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::MatchesPlan,
            "{observation}"
        );
        assert_expected_agrees_with(&observation, &report);
        let text = observation.to_string();
        assert!(
            text.contains("association table: expected Unloaded, observed Unloaded"),
            "{text}"
        );
        assert_read_only_and_closed(&device, before);
    }

    #[tokio::test]
    async fn a_machine_whose_last_control_is_not_settled_has_no_expected_state() {
        let device = loaded_device(SimulatorConfig::default());
        preset(&device, AssociationTable, LoadState::Loaded);
        let mut plan = plan();
        plan.steps
            .insert(4, record(AssociationTable, LoadEvent::StartLoading));

        let observation = observe(&device, &plan).await;

        assert_eq!(
            summary(&observation)[1],
            (AssociationTable, None, Some(LoadState::Loaded))
        );
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::Unknown,
            "an unresolved expectation is never filled in as Loaded"
        );
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn an_unsupported_mask_or_unknown_application_sends_nothing() {
        let device = loaded_device(SimulatorConfig::default());
        let mut other_mask = plan();
        other_mask.mask = MaskVersion(0x0025);
        let observation = observe(&device, &other_mask).await;
        assert!(matches!(
            observation.outcome,
            ObservationOutcome::NotPerformed(NotPerformed::UnsupportedMask(MaskVersion(0x0025)))
        ));

        let mut no_identity = plan();
        // Without the task segment the plan names no application.
        no_identity.steps.retain(|step| {
            !matches!(step, MemoryDownloadStep::LoadRecord(record)
                if record.event() == LoadEvent::AdditionalLoadControls
                    && record.machine() == ApplicationProgram
                    && record.octets()[3..5] == [0x44, 0x00]
                    && record.octets()[6..8] == MANUFACTURER.to_be_bytes())
        });
        let observation = observe(&device, &no_identity).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::NotPerformed(NotPerformed::ApplicationUnknown)
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::NotAttempted);
        assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
        assert!(device.seen().is_empty(), "no session opened");
    }

    #[tokio::test]
    async fn another_mask_is_a_mismatch_and_reads_no_load_state() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0705,
            ..SimulatorConfig::default()
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        let observation = observe(&device, &plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::IdentityMismatch(IdentityMismatch::Mask {
                    expected: MaskVersion(0x0701),
                    found: MaskVersion(0x0705)
                })
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::Reached);
        assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
        assert!(memory_reads(&device.seen()).is_empty());
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn another_manufacturer_is_a_mismatch() {
        let device = loaded_device(SimulatorConfig::default());
        device.preset_property(0, PID_MANUFACTURER_ID, &[0x00, 0x02]);
        let observation = observe(&device, &plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::IdentityMismatch(IdentityMismatch::Manufacturer {
                    expected: MANUFACTURER,
                    found: 0x0002
                })
            ),
            "{observation:?}"
        );
        assert!(memory_reads(&device.seen()).is_empty());
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn another_hardware_type_is_a_mismatch_before_the_application_is_read() {
        let device = loaded_device(SimulatorConfig::default());
        device.preset_property(0, PID_HARDWARE_TYPE, &[0, 0, 0, 0, 0x02, 0x39]);
        let observation = observe(&device, &plan()).await;
        assert!(
            matches!(
                &observation.outcome,
                ObservationOutcome::IdentityMismatch(IdentityMismatch::Property {
                    object_index: 0,
                    property_id: PID_HARDWARE_TYPE,
                    found,
                }) if found == &[0, 0, 0, 0, 0x02, 0x39]
            ),
            "{observation:?}"
        );
        assert!(!property_reads(&device.seen()).contains(&(3, PID_PROGRAM_VERSION)));
        assert!(memory_reads(&device.seen()).is_empty());
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn a_hardware_type_of_another_width_is_malformed_even_when_the_padding_rule_accepts_it() {
        let device = loaded_device(SimulatorConfig::default());
        // Seven octets: `property_matches` accepts them against the ten
        // zero-padded octets, the six-octet width guard does not.
        device.preset_property(0, PID_HARDWARE_TYPE, &[0, 0, 0, 0, 0x01, 0x27, 0x00]);
        let observation = observe(&device, &plan()).await;
        assert!(
            matches!(
                &observation.outcome,
                ObservationOutcome::IdentityMismatch(IdentityMismatch::HardwareTypeMalformed {
                    found
                }) if found.len() == 7
            ),
            "{observation:?}"
        );
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn a_plan_without_a_hardware_compare_makes_no_hardware_claim() {
        let device = loaded_device(SimulatorConfig::default());
        let mut plan = plan();
        plan.steps
            .retain(|step| !matches!(step, MemoryDownloadStep::CompareProperty { .. }));

        let observation = observe(&device, &plan).await;

        let ObservationOutcome::Observed { identity, .. } = &observation.outcome else {
            panic!("not observed: {observation:?}");
        };
        assert_eq!(
            identity.properties,
            vec![],
            "nothing declared, nothing matched"
        );
        assert!(!property_reads(&device.seen()).contains(&(0, PID_HARDWARE_TYPE)));
        let text = observation.to_string();
        assert!(text.contains("hardware type not compared"), "{text}");
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn a_refused_declared_property_leaves_the_identity_unavailable() {
        // No PID_HARDWARE_TYPE: the device refuses the declared read.
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0701,
            ..SimulatorConfig::default()
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        device.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
        let observation = observe(&device, &plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::IdentityUnavailable {
                    reading: IdentityReading::Property {
                        object_index: 0,
                        property_id: PID_HARDWARE_TYPE,
                    },
                    error: SessionError::PropertyRefused { .. },
                }
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::Reached);
        assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
        assert!(!property_reads(&device.seen()).contains(&(3, PID_PROGRAM_VERSION)));
        assert!(memory_reads(&device.seen()).is_empty());
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn a_truncated_application_identity_is_malformed_not_a_prefix_match() {
        let device = loaded_device(SimulatorConfig::default());
        device.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION[..4]);
        let observation = observe(&device, &plan()).await;
        assert!(
            matches!(
                &observation.outcome,
                ObservationOutcome::IdentityMismatch(IdentityMismatch::ApplicationMalformed {
                    found
                }) if found == &APPLICATION[..4]
            ),
            "{observation:?}"
        );
        assert!(memory_reads(&device.seen()).is_empty());
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn another_application_is_a_mismatch_and_reads_no_load_state() {
        let device = loaded_device(SimulatorConfig::default());
        device.preset_property(3, PID_PROGRAM_VERSION, &[0x00, 0x83, 0x00, 0x28, 0x15]);
        let observation = observe(&device, &plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::IdentityMismatch(IdentityMismatch::Application {
                    expected: APPLICATION,
                    found: [0x00, 0x83, 0x00, 0x28, 0x15],
                })
            ),
            "{observation:?}"
        );
        assert!(memory_reads(&device.seen()).is_empty());
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn a_refused_application_read_leaves_the_identity_unavailable() {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: 0x0701,
            ..SimulatorConfig::default()
        });
        device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
        device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
        // No PID_PROGRAM_VERSION: the device refuses the read.
        let observation = observe(&device, &plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::IdentityUnavailable {
                    reading: IdentityReading::Application,
                    error: SessionError::PropertyRefused { .. },
                }
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::Reached);
        assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn a_machine_that_is_not_loaded_differs_from_the_plan() {
        let device = loaded_device(SimulatorConfig::default());
        preset(&device, ApplicationProgram, LoadState::Unloaded);
        let observation = observe(&device, &plan()).await;
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::DiffersFromPlan
        );
        assert_eq!(
            summary(&observation),
            vec![
                (
                    AddressTable,
                    Some(LoadState::Loaded),
                    Some(LoadState::Loaded)
                ),
                (
                    ApplicationProgram,
                    Some(LoadState::Loaded),
                    Some(LoadState::Unloaded)
                ),
            ]
        );
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn an_error_or_loading_machine_differs_from_the_plan() {
        for state in [LoadState::Error, LoadState::Loading] {
            let device = loaded_device(SimulatorConfig::default());
            preset(&device, ApplicationProgram, state);
            let observation = observe(&device, &plan()).await;
            assert_eq!(
                observation.load_verdict(),
                LoadStateVerdict::DiffersFromPlan,
                "{state:?}: {observation}"
            );
            assert_eq!(
                summary(&observation)[1],
                (ApplicationProgram, Some(LoadState::Loaded), Some(state))
            );
            let text = observation.to_string();
            assert!(
                text.contains(&format!("expected Loaded, observed {state:?}")),
                "{text}"
            );
            assert_read_only_and_closed(&device, 0);
        }
    }

    #[tokio::test]
    async fn a_silent_device_is_unreachable_and_released_once() {
        let device = loaded_device(SimulatorConfig {
            silent: true,
            ..SimulatorConfig::default()
        });
        let transport = Recording::new(&device, Fault::None);
        let observation = transport.observe(&plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::Unreachable(SessionError::ConnectionReleased { .. })
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::NotReached);
        assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
        assert_eq!(
            transport.disconnects(),
            1,
            "TL's own release, and no second T_Disconnect after it"
        );
        assert_read_only(&device, 0);
        assert_eq!(state_of(&device, AddressTable), LoadState::Loaded);
    }

    #[tokio::test]
    async fn an_acknowledged_but_unanswered_mask_read_is_reached_with_the_identity_unavailable() {
        let device = loaded_device(SimulatorConfig {
            device_descriptor_read_unanswered: true,
            ..SimulatorConfig::default()
        });
        let transport = Recording::new(&device, Fault::None);
        let observation = transport.observe(&plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::IdentityUnavailable {
                    reading: IdentityReading::Mask,
                    error: SessionError::NoAnswer { .. },
                }
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::Reached, "the T_ACK is evidence");
        assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
        assert_eq!(transport.disconnects(), 1, "the open connection is closed");
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn a_first_read_that_fails_locally_is_inconclusive_not_reached() {
        let device = loaded_device(SimulatorConfig::default());
        let transport = Recording::new(&device, Fault::SendFails);
        let observation = transport.observe(&plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::FirstReadInconclusive(SessionError::Transport(_))
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::Unknown);
        assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
        assert_eq!(transport.disconnects(), 1, "the open connection is closed");
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn a_first_read_whose_events_were_dropped_is_inconclusive() {
        let device = loaded_device(SimulatorConfig::default());
        let transport = Recording::new(&device, Fault::EventsLag);
        let observation = transport.observe(&plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::FirstReadInconclusive(SessionError::Lagged { .. })
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::Unknown);
        assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn a_tunnel_closing_on_the_first_read_is_inconclusive_and_not_disconnected_again() {
        let device = loaded_device(SimulatorConfig::default());
        let transport = Recording::new(&device, Fault::TunnelCloses);
        let observation = transport.observe(&plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::FirstReadInconclusive(SessionError::ConnectionLost { .. })
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::Unknown);
        assert_eq!(
            transport.disconnects(),
            0,
            "the session holds no connection"
        );
        assert_read_only(&device, 0);
    }

    #[tokio::test]
    async fn a_disconnect_instead_of_the_mask_is_inconclusive_and_not_disconnected_again() {
        // A device that exists answers with `T_Disconnect` (MP §2.3, p. 15),
        // so `ConnectionLost` is no evidence that nothing is there.
        let device = loaded_device(SimulatorConfig {
            device_descriptor_read_gets_disconnect: true,
            ..SimulatorConfig::default()
        });
        let transport = Recording::new(&device, Fault::None);
        let observation = transport.observe(&plan()).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::FirstReadInconclusive(SessionError::ConnectionLost { .. })
            ),
            "{observation:?}"
        );
        assert_eq!(observation.reach(), Reach::Unknown);
        assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
        assert_eq!(transport.disconnects(), 0, "the device already released it");
        assert_read_only(&device, 0);
    }

    #[tokio::test]
    async fn a_connect_that_fails_is_unreachable_and_sends_no_disconnect() {
        for config in [
            SimulatorConfig {
                rejected_connects: Some(1..2),
                ..SimulatorConfig::default()
            },
            SimulatorConfig {
                unanswered_connects: Some(1..2),
                ..SimulatorConfig::default()
            },
        ] {
            let device = loaded_device(config);
            let transport = Recording::new(&device, Fault::None);
            let observation = transport.observe(&plan()).await;
            assert!(
                matches!(
                    observation.outcome,
                    ObservationOutcome::Unreachable(
                        SessionError::ConnectRejected { .. } | SessionError::NoAnswer { .. }
                    )
                ),
                "{observation:?}"
            );
            assert_eq!(observation.reach(), Reach::NotReached);
            assert_eq!(observation.load_verdict(), LoadStateVerdict::Unknown);
            assert_eq!(transport.disconnects(), 0, "no connection was opened");
            assert!(device.seen().is_empty(), "{:?}", device.seen());
        }
    }

    #[tokio::test]
    async fn a_later_failed_load_state_read_keeps_the_earlier_ones() {
        // The association table's load state (B6EBh) refuses to be read.
        let device = loaded_device(SimulatorConfig {
            protected_memory: Some((0xB6EB, 0xB6EC)),
            ..SimulatorConfig::default()
        });
        let observation = observe(&device, &three_machine_plan()).await;
        let states = load_states(&observation);
        assert_eq!(states.len(), 3);
        assert_eq!(states[0].machine, AddressTable);
        assert!(matches!(
            states[0].state,
            LoadStateObservation::Observed(LoadState::Loaded)
        ));
        assert_eq!(states[1].machine, AssociationTable);
        assert!(
            matches!(states[1].state, LoadStateObservation::Unavailable(_)),
            "{:?}",
            states[1]
        );
        assert_eq!(states[2].machine, ApplicationProgram);
        assert!(matches!(states[2].state, LoadStateObservation::NotRead));
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::Unknown,
            "a missing reading never matches the plan"
        );
        assert_eq!(memory_reads(&device.seen()), vec![0xB6EA, 0xB6EB]);
        assert_read_only_and_closed(&device, 0);
    }

    #[tokio::test]
    async fn an_observed_difference_wins_over_a_missing_reading() {
        let device = loaded_device(SimulatorConfig {
            protected_memory: Some((0xB6EB, 0xB6EC)),
            ..SimulatorConfig::default()
        });
        preset(&device, AddressTable, LoadState::Loading);
        let observation = observe(&device, &three_machine_plan()).await;
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::DiffersFromPlan
        );
        assert_read_only_and_closed(&device, 0);
    }
}
