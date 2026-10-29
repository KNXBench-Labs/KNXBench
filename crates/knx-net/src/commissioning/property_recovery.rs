//! Operation-bound recovery evidence for experimental same-application reinstallation.
//!
//! These values are not a device image or allocator rollback certificate. The
//! caller owns durable serialization, sync/readback and journal integrity. No
//! filesystem or proprietary source interpretation belongs in this module.

use std::collections::BTreeMap;

use super::final_step::{FinalStepError, FinalStepStage};
use super::final_step_evidence::AllocationMismatchEvidence;
use super::pointer_width::{
    PointerWidthAdmission, PointerWidthRefusal, PointerWidthRefusalReason, PropertyPointerValue,
};
use super::property_download::PropertySimulationFailure;
use super::{ManagementSession, SessionError};
use crate::management::ManagementTransport;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::memory::{allocation_service, chunks, MemoryService, WriteLimit};
use knx_core::commissioning::properties::{ObjectIndex, PID_PROGRAM_VERSION, PID_TABLE_REFERENCE};
use knx_core::commissioning::property_download::{PropertyDownloadPlan, PropertyOperation};

/// Explicit unresolved physical premise, never a default admission policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyRecoveryHypothesis {
    /// Reinstall the same exact application/procedure/allocation shape; not raw
    /// allocator/task-state rollback or verification of all physical effects.
    SameApplicationSameProcedureSameAllocationShape,
}

/// A source-applicable PID_TABLE_REFERENCE, preserving its full four bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyResourcePointer {
    pub object: u8,
    pub value: [u8; 4],
}

/// Exact source/identity binding and explicit experimental safety premise.
/// Public fields support App-owned persistence; every entry point revalidates them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentalPropertyPreconditions {
    pub hypothesis: PropertyRecoveryHypothesis,
    pub original_plan: PropertyDownloadPlan,
    /// Serial from preparation. An expectation, not download admission: acquisition
    /// records a different device serial as an observation (owner ruling 2026-10-25).
    pub serial: [u8; 6],
    pub program_version: [u8; 5],
    /// Experimental last absolute/task-segment reference model, four octets
    /// only. This explicit source expectation is not a physical profile proof.
    pub source_pointers: Vec<PropertyResourcePointer>,
    /// Which PID 7 answer widths the reads accept. `same_application_reinstallation`
    /// is always `Legacy`; a wider rule is an explicit, validated opt-in.
    pub pointer_widths: PointerWidthAdmission,
}
impl ExperimentalPropertyPreconditions {
    /// Acknowledge strong-hypothesis same-shape reinstallation, not compatibility.
    /// Source controls remain exact. PID5 states are never interpreted as commands.
    pub fn same_application_reinstallation(
        plan: &PropertyDownloadPlan,
        serial: [u8; 6],
        program_version: [u8; 5],
    ) -> Result<Self, PropertySimulationFailure> {
        Ok(Self {
            hypothesis: PropertyRecoveryHypothesis::SameApplicationSameProcedureSameAllocationShape,
            original_plan: plan.clone(),
            serial,
            program_version,
            source_pointers: source_pointers(plan)?,
            pointer_widths: PointerWidthAdmission::Legacy,
        })
    }
    /// Replace the width rule with a checked one bound to this plan's touched objects.
    pub fn with_pointer_width_admission(
        mut self,
        admission: PointerWidthAdmission,
    ) -> Result<Self, PropertySimulationFailure> {
        admission.validate(&self.source_pointers)?;
        self.pointer_widths = admission;
        Ok(self)
    }
    pub(crate) fn require_full_hardware_write_coverage(
        &self,
    ) -> Result<(), PropertySimulationFailure> {
        for allocation in allocation_shapes(&self.original_plan) {
            let start = u32::from(allocation.address);
            let end = start + allocation.data.len() as u32;
            let written: usize = self
                .original_plan
                .steps()
                .iter()
                .filter_map(|step| match &step.operation {
                    PropertyOperation::WriteMemory {
                        object,
                        address,
                        data,
                    } if *object == allocation.object
                        && u32::from(*address) >= start
                        && u32::from(*address) + data.len() as u32 <= end =>
                    {
                        Some(data.len())
                    }
                    _ => None,
                })
                .sum();
            if written != allocation.data.len() {
                return Err(invalid("hardware recovery for preserved/unwritten allocation bytes is unresolved; simulator retention is not proof"));
            }
        }
        Ok(())
    }

    pub(crate) fn validate(
        &self,
        plan: &PropertyDownloadPlan,
    ) -> Result<(), PropertySimulationFailure> {
        if self.original_plan != *plan || self.source_pointers != source_pointers(plan)? {
            return Err(invalid(
                "experimental preconditions do not bind this exact source plan",
            ));
        }
        self.pointer_widths.validate(&self.source_pointers)
    }
}

fn source_pointers(
    plan: &PropertyDownloadPlan,
) -> Result<Vec<PropertyResourcePointer>, PropertySimulationFailure> {
    let mut pointers = BTreeMap::<u8, Option<u16>>::new();
    for step in plan.steps() {
        if let PropertyOperation::LoadControl { object, payload } = &step.operation {
            let pointer = pointers.entry(*object).or_default();
            let b = payload.octets();
            if b[0] == 3 && b[1] <= 2 {
                // Explicit absolute address/task-segment pointer in original
                // source record, not an invented command reconstructed from PID5.
                *pointer = Some(u16::from_be_bytes([b[2], b[3]]));
            }
        }
    }
    pointers
        .into_iter()
        .map(|(object, pointer)| {
            let pointer = pointer.ok_or_else(|| {
                invalid("every touched resource needs an explicit source pointer")
            })?;
            Ok(PropertyResourcePointer {
                object,
                value: u32::from(pointer).to_be_bytes(),
            })
        })
        .collect()
}

/// Full original bytes for one exact normalized WriteMemory operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRecoveryRegion {
    pub operation_index: usize,
    pub source_index: usize,
    pub object: u8,
    pub address: u16,
    pub data: Vec<u8>,
}
/// The stable load state and raw PID_TABLE_REFERENCE actually read before any write
/// (`None`: the device refused the read). Equal to the source expectation unless a
/// prestate difference was observed. The reply width is kept as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRecoveryResource {
    pub object: u8,
    pub state: LoadState,
    pub pointer: Option<PropertyPointerValue>,
}
/// Device serial read before any write that differs from the prepared one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertySerialObservation {
    pub expected: [u8; 6],
    pub observed: [u8; 6],
}
impl std::fmt::Display for PropertySerialObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex = |b: &[u8; 6]| b.iter().map(|o| format!("{o:02x}")).collect::<String>();
        write!(
            f,
            "serial number: expected {}, observed {}",
            hex(&self.expected),
            hex(&self.observed)
        )
    }
}
/// Object 3 PID_PROGRAM_VERSION read before any write that differs from the plan's.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyProgramVersionObservation {
    pub expected: [u8; 5],
    /// Empty when the device refused the read.
    pub observed: Vec<u8>,
}
impl std::fmt::Display for PropertyProgramVersionObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let hex = |b: &[u8]| b.iter().map(|o| format!("{o:02x}")).collect::<String>();
        let observed = if self.observed.is_empty() {
            "refused".to_string()
        } else {
            hex(&self.observed)
        };
        write!(
            f,
            "program version: expected {}, observed {observed}",
            hex(&self.expected)
        )
    }
}
/// A per-object difference between the source pointer model and a device read.
/// An observation, never proof: the source model itself is not physically verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyPointerObservation {
    pub object: u8,
    pub expected: [u8; 4],
    /// `None` when the device refused the read with zero elements.
    pub observed: Option<PropertyPointerValue>,
    pub state: LoadState,
}
impl std::fmt::Display for PropertyPointerObservation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let observed = self
            .observed
            .map_or_else(|| "refused".to_string(), |o| o.to_string());
        write!(
            f,
            "object {}: expected PID_TABLE_REFERENCE {:08x}, observed {observed}, load state {}",
            self.object,
            u32::from_be_bytes(self.expected),
            self.state
        )
    }
}
/// A PID_TABLE_REFERENCE read-back right after an allocation/task record that
/// differs from that record. Observation only (owner amendment 3, 2026-10-25).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyInProcedurePointerObservation {
    pub operation_index: usize,
    pub observation: PropertyPointerObservation,
}
/// Versioned operation-specific recovery, not a raw persistent-device backup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRecoverySnapshot {
    pub version: u16,
    pub original_plan: PropertyDownloadPlan,
    pub preconditions: ExperimentalPropertyPreconditions,
    /// Serial actually read before any write. Recovery replay is bound to it,
    /// because the captured bytes belong to that one physical device.
    pub observed_serial: [u8; 6],
    /// PID_PROGRAM_VERSION of object 3 read before any write; empty when the
    /// device refused the read. Observation only; the download writes the identity.
    pub observed_program_version: Vec<u8>,
    pub old_device_control: u8,
    pub resources: Vec<PropertyRecoveryResource>,
    /// Full declared absolute allocation intervals, including unwritten/masked bytes.
    pub allocations: Vec<PropertyRecoveryRegion>,
    /// Indices into `allocations` whose prestate read was refused or short.
    /// Their data are zero placeholders, never a backup; recovery is unavailable.
    pub unreadable_allocations: Vec<usize>,
    /// Exact WriteMemory prestate, cross-checked against full allocation slices.
    pub regions: Vec<PropertyRecoveryRegion>,
}
impl PropertyRecoverySnapshot {
    /// The device serial differed from the preparation's; observation only.
    pub fn serial_observation(&self) -> Option<PropertySerialObservation> {
        (self.observed_serial != self.preconditions.serial).then_some(PropertySerialObservation {
            expected: self.preconditions.serial,
            observed: self.observed_serial,
        })
    }

    /// The prior application identity differed from the plan's; observation only.
    pub fn program_version_observation(&self) -> Option<PropertyProgramVersionObservation> {
        (self.observed_program_version != self.preconditions.program_version).then(|| {
            PropertyProgramVersionObservation {
                expected: self.preconditions.program_version,
                observed: self.observed_program_version.clone(),
            }
        })
    }

    /// Objects whose captured prestate was not Loaded at the source pointer.
    /// Empty for every snapshot that matches the source allocation shape.
    pub fn prestate_observations(&self) -> Vec<PropertyPointerObservation> {
        self.resources
            .iter()
            .zip(&self.preconditions.source_pointers)
            .filter(|(resource, expected)| {
                resource.state != LoadState::Loaded
                    || !resource
                        .pointer
                        .is_some_and(|pointer| pointer.matches_source(&expected.value))
            })
            .map(|(resource, expected)| PropertyPointerObservation {
                object: resource.object,
                expected: expected.value,
                observed: resource.pointer,
                state: resource.state,
            })
            .collect()
    }

    /// Rebind persisted evidence to the complete exact original operation list.
    /// Only old WriteMemory data may differ; no commands are recovered from PID5.
    /// Refused after a prestate difference: the captured bytes then do not
    /// describe the device's actual allocation, so no backup is fabricated.
    pub fn recovery_plan(
        &self,
        original: &PropertyDownloadPlan,
    ) -> Result<PropertyDownloadPlan, PropertySimulationFailure> {
        let plan = self.checked_plan(original)?;
        if let Some(observation) = self.program_version_observation() {
            return Err(PropertySimulationFailure::Recovery(format!(
                "original-state recovery unavailable: captured prestate belongs to a different application ({observation})"
            )));
        }
        if !self.unreadable_allocations.is_empty() {
            return Err(PropertySimulationFailure::Recovery(format!(
                "original-state recovery unavailable: prestate memory unreadable for allocation(s) {:?}",
                self.unreadable_allocations
            )));
        }
        if self.observed_serial == [0; 6] || self.observed_serial == [0xFF; 6] {
            return Err(PropertySimulationFailure::Recovery(format!(
                "original-state recovery unavailable: captured serial number {} cannot bind replay to one physical device",
                self.observed_serial.iter().map(|o| format!("{o:02x}")).collect::<String>()
            )));
        }
        let observations = self.prestate_observations();
        if !observations.is_empty() {
            let details: Vec<String> = observations.iter().map(ToString::to_string).collect();
            return Err(PropertySimulationFailure::Recovery(format!(
                "original-state recovery unavailable: device prestate differed from source allocation shape ({})",
                details.join("; ")
            )));
        }
        Ok(plan)
    }

    /// Check persisted evidence binding and coverage without requiring that the
    /// prestate matched the source shape. Loading evidence is not recovery admission.
    pub fn validate_evidence(
        &self,
        original: &PropertyDownloadPlan,
    ) -> Result<(), PropertySimulationFailure> {
        self.checked_plan(original).map(|_| ())
    }

    /// A pointer read is an observation when it misses the source model or when its
    /// width differs from the original prestate's, even if the value is equal.
    /// The prestate is the width reference for the download and, durably, for restore.
    pub fn pointer_needs_observation(
        &self,
        object: u8,
        expected: &[u8; 4],
        observed: Option<PropertyPointerValue>,
    ) -> bool {
        let Some(observed) = observed else {
            return true;
        };
        if !observed.matches_source(expected) {
            return true;
        }
        self.resources
            .iter()
            .find(|resource| resource.object == object)
            .and_then(|resource| resource.pointer)
            .is_some_and(|prestate| !prestate.same_width(&observed))
    }

    fn checked_plan(
        &self,
        original: &PropertyDownloadPlan,
    ) -> Result<PropertyDownloadPlan, PropertySimulationFailure> {
        self.preconditions.validate(original)?;
        if self.version != 1 || self.original_plan != *original {
            return Err(invalid(
                "unsupported snapshot version or different exact original plan",
            ));
        }
        if self.resources.len() != self.preconditions.source_pointers.len()
            || self
                .resources
                .iter()
                .zip(&self.preconditions.source_pointers)
                .any(|(resource, expected)| resource.object != expected.object)
        {
            return Err(invalid(
                "snapshot resource coverage differs from original source shape",
            ));
        }
        if self.resources.iter().any(|resource| {
            matches!(resource.pointer, Some(PropertyPointerValue::Two(_)))
                && !self
                    .preconditions
                    .pointer_widths
                    .applies_to(resource.object)
        }) {
            return Err(invalid(
                "snapshot holds a two-octet pointer the width admission does not cover",
            ));
        }
        if self.unreadable_allocations.windows(2).any(|w| w[0] >= w[1])
            || self.unreadable_allocations.iter().any(|&i| {
                self.allocations
                    .get(i)
                    .is_none_or(|a| a.data.iter().any(|&b| b != 0))
            })
        {
            return Err(invalid(
                "unreadable allocation markers are malformed or carry data",
            ));
        }
        let shapes = allocation_shapes(original);
        if self.allocations.len() != shapes.len()
            || self
                .allocations
                .iter()
                .zip(&shapes)
                .any(|(region, shape)| !same_region_shape(region, shape))
        {
            return Err(invalid(
                "full allocation coverage differs from exact original procedure",
            ));
        }
        let mut captured = self.regions.iter();
        let mut payloads = Vec::new();
        for (operation_index, step) in original.steps().iter().enumerate() {
            if let PropertyOperation::WriteMemory {
                object,
                address,
                data,
            } = &step.operation
            {
                let region = captured
                    .next()
                    .ok_or_else(|| invalid("missing original memory region"))?;
                if region.operation_index != operation_index
                    || region.source_index != step.source_index
                    || region.object != *object
                    || region.address != *address
                    || region.data.len() != data.len()
                {
                    return Err(invalid(
                        "snapshot memory region is not bound to exact original operation",
                    ));
                }
                let old = allocation_slice(&self.allocations, *object, *address, data.len())?;
                if region.data != old {
                    return Err(invalid(
                        "write prestate differs from full captured allocation",
                    ));
                }
                payloads.push(old.to_vec());
            }
        }
        if captured.next().is_some() {
            return Err(invalid("excess original memory regions"));
        }
        original
            .with_memory_payloads(&payloads)
            .map_err(|_| invalid("old memory payloads change procedure shape"))
    }

    /// Read-only. Memory, final Loaded state and program version are verification;
    /// a final PID_TABLE_REFERENCE that differs from the source model is returned
    /// as an observation, because that model is itself unverified. Allocations whose
    /// prestate was unreadable are verified on their written bytes only.
    pub(crate) async fn verify_outcome<T: ManagementTransport>(
        &self,
        session: &mut ManagementSession<'_, T>,
        executed_plan: &PropertyDownloadPlan,
    ) -> Result<Vec<PropertyPointerObservation>, FinalStepError> {
        let limit = session
            .write_limit(None)
            .await
            .map_err(FinalStepError::read(FinalStepStage::Limit))?;
        for (index, allocation) in self.allocations.iter().enumerate() {
            let (expected, written) = expected_allocation(allocation, executed_plan);
            let actual = read_region(session, allocation.address, &expected, limit)
                .await
                .map_err(FinalStepError::read(FinalStepStage::Allocation))?;
            // No captured prestate predicts an unreadable allocation's unwritten bytes;
            // `compare` never lets them decide, but records them once a written byte differs.
            if let Some(evidence) = AllocationMismatchEvidence::compare(
                index,
                allocation,
                self.unreadable_allocations.contains(&index),
                expected,
                written,
                actual,
            ) {
                return Err(FinalStepError::mismatch(
                    FinalStepStage::Allocation,
                    "full allocation verification differs, including preserved/unwritten bytes",
                )
                .with_evidence(evidence));
            }
        }
        let mut observations = Vec::new();
        for expected in &self.preconditions.source_pointers {
            let object = ObjectIndex::new(expected.object);
            let state = session
                .read_load_state(object)
                .await
                .map_err(FinalStepError::read(FinalStepStage::LoadedState))?;
            if state != LoadState::Loaded {
                return Err(FinalStepError::mismatch(
                    FinalStepStage::LoadedState,
                    "final resource state is not Loaded",
                ));
            }
            let observed = read_pointer(session, object, &self.preconditions.pointer_widths)
                .await
                .map_err(FinalStepError::read(FinalStepStage::Pointer))?;
            if self.pointer_needs_observation(expected.object, &expected.value, observed) {
                observations.push(PropertyPointerObservation {
                    object: expected.object,
                    expected: expected.value,
                    observed,
                    state: LoadState::Loaded,
                });
            }
        }
        // Interrupted current application identity may be absent at recovery start,
        // but a successful full replay must have reinstated the captured identity.
        // Do not seed/write a guessed identity outside the declared procedure.
        let version = session
            .read_property(ObjectIndex::new(3), PID_PROGRAM_VERSION, 1, 1)
            .await
            .map_err(FinalStepError::read(FinalStepStage::ProgramVersion))?;
        if version.as_slice() != self.preconditions.program_version {
            return Err(FinalStepError::mismatch(
                FinalStepStage::ProgramVersion,
                "final application identity differs from captured program version; recovery remains unverified",
            ));
        }
        Ok(observations)
    }

    /// Replays the final comparison of allocation `index` against recorded `actual` bytes:
    /// `Some` exactly when verification of `executed_plan` fails there, with the evidence
    /// Net records. Lets a journal reader prove a stored record instead of trusting it.
    /// `None` for an unknown index or a length other than the allocation's.
    pub fn allocation_mismatch(
        &self,
        index: usize,
        executed_plan: &PropertyDownloadPlan,
        actual: Vec<u8>,
    ) -> Option<AllocationMismatchEvidence> {
        let allocation = self.allocations.get(index)?;
        if actual.len() != allocation.data.len() {
            return None;
        }
        let (expected, written) = expected_allocation(allocation, executed_plan);
        AllocationMismatchEvidence::compare(
            index,
            allocation,
            self.unreadable_allocations.contains(&index),
            expected,
            written,
            actual,
        )
    }

    /// The only write after verification. The caller journals the verification's
    /// observations first, so they are durable before this device write.
    pub(crate) async fn restore_verify_mode<T: ManagementTransport>(
        &self,
        session: &mut ManagementSession<'_, T>,
    ) -> Result<(), FinalStepError> {
        // A bounded setup lifecycle, not an automatic recovery of load state or
        // memory on failure. Preserve unrelated *current* PID14 bits.
        let current = session
            .read_device_control()
            .await
            .map_err(FinalStepError::read(FinalStepStage::VerifyModeRead))?;
        let wanted = knx_core::commissioning::properties::with_verify_mode(
            current,
            knx_core::commissioning::properties::verify_mode_active(self.old_device_control),
        );
        session
            .write_property(
                ObjectIndex::DEVICE,
                knx_core::commissioning::properties::PID_DEVICE_CONTROL,
                vec![wanted],
                knx_core::commissioning::mutation::WriteScope::Download,
            )
            .await
            .map_err(FinalStepError::write(FinalStepStage::VerifyModeWrite))?;
        let readback = session
            .read_device_control()
            .await
            .map_err(FinalStepError::read(FinalStepStage::VerifyModeReadback))?;
        if readback != wanted {
            return Err(FinalStepError::mismatch(
                FinalStepStage::VerifyModeReadback,
                "prior VerifyMode bit did not restore",
            ));
        }
        Ok(())
    }
}

/// Synchronous durability barriers. Returning Err stops before the next send.
/// Operation indices distinguish an allocation and memory write sharing a source.
#[derive(Debug)]
pub enum PropertyJournalEvent<'a> {
    RecoveryAcquired(&'a PropertyRecoverySnapshot),
    OperationStarting {
        operation_index: usize,
        source_index: usize,
        mutation_may_have_started: bool,
    },
    OperationCompleted {
        operation_index: usize,
        source_index: usize,
        mutation_may_have_started: bool,
    },
    /// Final PID_TABLE_REFERENCE values that differ from the source model after
    /// an otherwise verified outcome. Sent only when non-empty, before disconnect.
    FinalPointerObservations(&'a [PropertyPointerObservation]),
    /// An allocation/task read-back that differed from its record; sent before
    /// this operation completes, so before the next mutation.
    InProcedurePointerObservation(&'a PropertyInProcedurePointerObservation),
}

pub(crate) fn invalid(reason: &str) -> PropertySimulationFailure {
    PropertySimulationFailure::Recovery(reason.to_string())
}

/// Recovery only: captured bytes belong to the device read at acquisition, so
/// replaying them onto another serial would write foreign memory. Not download admission.
pub(crate) async fn validate_recovery_serial<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    snapshot: &PropertyRecoverySnapshot,
) -> Result<(), PropertySimulationFailure> {
    let serial = session.read_serial_number().await?;
    if serial.octets() != snapshot.observed_serial {
        return Err(invalid(
            "recovery refused: device serial number differs from the device whose prestate was captured",
        ));
    }
    Ok(())
}

/// Serial and current program version are read for the snapshot and never
/// compared: the download itself writes the application identity (owner rulings
/// 2026-10-25). Hardware type, mask and manufacturer remain the hard pre-write checks.
pub(crate) async fn read_identity<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
) -> Result<([u8; 6], Vec<u8>), PropertySimulationFailure> {
    let serial = session.read_serial_number().await?.octets();
    let version = match session
        .read_property(ObjectIndex::APPLICATION_PROGRAM, PID_PROGRAM_VERSION, 1, 1)
        .await
    {
        Ok(version) => version,
        // An unloaded device may refuse the read; that is an observation too.
        Err(SessionError::PropertyRefused { .. }) => Vec::new(),
        Err(error) => return Err(error.into()),
    };
    Ok((serial, version))
}

pub(crate) async fn read_region<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    address: u16,
    shape: &[u8],
    limit: WriteLimit,
) -> Result<Vec<u8>, PropertySimulationFailure> {
    let mut bytes = Vec::with_capacity(shape.len());
    for chunk in chunks(u32::from(address), shape, limit).map_err(SessionError::Chunk)? {
        let read = session
            .read_memory_as(chunk.service, chunk.address, chunk.length)
            .await?;
        if read.len() != usize::from(chunk.length) {
            return Err(invalid(
                "incomplete memory response cannot form a recovery snapshot",
            ));
        }
        bytes.extend(read);
    }
    Ok(bytes)
}

/// Longest answer shown byte for byte in a width refusal; the true length is always stated.
const WIDTH_REFUSAL_SHOWN_OCTETS: usize = 16;

/// The refusal for a PID 7 answer that is not four octets: the observed object, element
/// count and raw bytes, so the field fact (issue #30) survives into the journal.
fn pointer_width_refusal(object: ObjectIndex, elements: u8, data: &[u8]) -> String {
    pointer_width_text(
        "resource pointer response is not four octets",
        object,
        elements,
        data,
    )
}

fn pointer_width_text(prefix: &str, object: ObjectIndex, elements: u8, data: &[u8]) -> String {
    let shown = if data.is_empty() {
        "(none)".into()
    } else {
        data.iter()
            .take(WIDTH_REFUSAL_SHOWN_OCTETS)
            .map(|octet| format!("{octet:02x}"))
            .collect::<String>()
    };
    let more = if data.len() > WIDTH_REFUSAL_SHOWN_OCTETS {
        "..."
    } else {
        ""
    };
    format!(
        "{prefix}: object {}, {elements} element{}, {} octet{}, {shown}{more}",
        object.octet(),
        if elements == 1 { "" } else { "s" },
        data.len(),
        if data.len() == 1 { "" } else { "s" },
    )
}

/// `Ok(None)` for a zero-element refusal, an observation; transport errors,
/// timeouts and malformed lengths stay errors. A malformed length keeps its element
/// count and bytes in the refusal; it is never padded, truncated or read as a pointer.
///
/// `Legacy` keeps the historical leniency byte for byte (any element count with four
/// octets; zero elements discard their data). On an object the admission names, only
/// exactly one element of two or four octets is a pointer and everything else is a typed
/// refusal.
pub(crate) async fn read_pointer<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    object: ObjectIndex,
    admission: &PointerWidthAdmission,
) -> Result<Option<PropertyPointerValue>, PropertySimulationFailure> {
    let (elements, bytes) = session
        .read_property_counted(object, PID_TABLE_REFERENCE, 1, 1)
        .await?;
    if !admission.applies_to(object.octet()) {
        return match (elements, bytes.as_slice()) {
            (0, _) => Ok(None),
            _ => <[u8; 4]>::try_from(bytes.as_slice())
                .map(|octets| Some(PropertyPointerValue::Four(octets)))
                .map_err(|_| invalid(&pointer_width_refusal(object, elements, &bytes))),
        };
    }
    let refuse = |reason| {
        PropertySimulationFailure::PointerWidth(PointerWidthRefusal {
            reason,
            object: object.octet(),
            property_id: PID_TABLE_REFERENCE,
            elements,
            octets: bytes.len(),
            diagnostic: pointer_width_text(
                "resource pointer response is not one element of two or four octets",
                object,
                elements,
                &bytes,
            ),
        })
    };
    match (elements, bytes.as_slice()) {
        (0, []) => Ok(None),
        (0, _) => Err(refuse(PointerWidthRefusalReason::ZeroElementsWithData)),
        (1, [a, b, c, d]) => Ok(Some(PropertyPointerValue::Four([*a, *b, *c, *d]))),
        (1, [a, b]) => Ok(Some(PropertyPointerValue::Two([*a, *b]))),
        (1, _) => Err(refuse(PointerWidthRefusalReason::UnsupportedWidth)),
        _ => Err(refuse(PointerWidthRefusalReason::ElementCount)),
    }
}

pub(crate) async fn acquire<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &PropertyDownloadPlan,
    preconditions: &ExperimentalPropertyPreconditions,
) -> Result<PropertyRecoverySnapshot, PropertySimulationFailure> {
    let (observed_serial, observed_program_version) = read_identity(session).await?;
    // A prestate that differs from the source shape is recorded, not refused
    // (issue #30); recovery_plan() then refuses original-state recovery.
    let mut resources = Vec::new();
    for expected in &preconditions.source_pointers {
        let object = ObjectIndex::new(expected.object);
        let state = session.read_load_state(object).await?;
        let pointer = read_pointer(session, object, &preconditions.pointer_widths).await?;
        resources.push(PropertyRecoveryResource {
            object: expected.object,
            state,
            pointer,
        });
    }
    let limit = session.write_limit(None).await?;
    let mut allocations = allocation_shapes(plan);
    let mut unreadable_allocations = Vec::new();
    for (index, allocation) in allocations.iter_mut().enumerate() {
        // A refused or short prestate answer is an observation (review fix 5):
        // keep zero placeholders, flag them and make recovery unavailable.
        match read_region(session, allocation.address, &allocation.data, limit).await {
            Ok(bytes) => allocation.data = bytes,
            Err(PropertySimulationFailure::Session(error))
                if matches!(error.as_ref(), SessionError::MemoryRefused { .. }) =>
            {
                unreadable_allocations.push(index)
            }
            Err(PropertySimulationFailure::Recovery(_)) => unreadable_allocations.push(index),
            Err(error) => return Err(error),
        }
    }
    let mut regions = Vec::new();
    for (operation_index, step) in plan.steps().iter().enumerate() {
        if let PropertyOperation::WriteMemory {
            object,
            address,
            data,
        } = &step.operation
        {
            regions.push(PropertyRecoveryRegion {
                operation_index,
                source_index: step.source_index,
                object: *object,
                address: *address,
                data: allocation_slice(&allocations, *object, *address, data.len())?.to_vec(),
            });
        }
    }
    let old_device_control = session.read_device_control().await?;
    // Reject a load-state transition during acquisition rather than backing up
    // an unstable resource and guessing an old control record from its status.
    for resource in &resources {
        if session
            .read_load_state(ObjectIndex::new(resource.object))
            .await?
            != resource.state
        {
            return Err(invalid(
                "resource load state changed during recovery acquisition",
            ));
        }
    }
    Ok(PropertyRecoverySnapshot {
        version: 1,
        original_plan: plan.clone(),
        preconditions: preconditions.clone(),
        observed_serial,
        observed_program_version,
        old_device_control,
        resources,
        allocations,
        unreadable_allocations,
        regions,
    })
}

/// One allocation's captured prestate overlaid with `executed_plan`'s writes, and which
/// bytes those writes cover.
fn expected_allocation(
    allocation: &PropertyRecoveryRegion,
    executed_plan: &PropertyDownloadPlan,
) -> (Vec<u8>, Vec<bool>) {
    let mut expected = allocation.data.clone();
    let mut written = vec![false; expected.len()];
    for step in executed_plan.steps() {
        if let PropertyOperation::WriteMemory {
            object,
            address,
            data,
        } = &step.operation
        {
            if *object == allocation.object && *address >= allocation.address {
                let offset = usize::from(*address - allocation.address);
                if let Some(bytes) = expected.get_mut(offset..offset + data.len()) {
                    bytes.copy_from_slice(data);
                    written[offset..offset + data.len()].fill(true);
                }
            }
        }
    }
    (expected, written)
}

/// Mask `0025h` has no "DMA on User Memory" (06 Profiles v02.01.01 §4.2, printed p37), and
/// CP §3.5.2 (printed p43) names `A_Memory_*` only for base plus allocated size below
/// `FFFFh`. So every absolute allocation must end below it, and then each write, prestate
/// read and verification read inside it uses `A_Memory_*`. Checked before any frame.
pub(crate) fn refuse_beyond_memory_service(
    plan: &PropertyDownloadPlan,
) -> Result<(), PropertySimulationFailure> {
    for allocation in allocation_shapes(plan) {
        let length = allocation.data.len();
        if allocation_service(u32::from(allocation.address), length as u32)
            != Some(MemoryService::Memory)
        {
            return Err(PropertySimulationFailure::MemoryServiceBoundary {
                object: allocation.object,
                address: allocation.address,
                length,
            });
        }
    }
    Ok(())
}

fn allocation_shapes(plan: &PropertyDownloadPlan) -> Vec<PropertyRecoveryRegion> {
    plan.steps()
        .iter()
        .enumerate()
        .filter_map(|(operation_index, step)| {
            let PropertyOperation::LoadControl { object, payload } = &step.operation else {
                return None;
            };
            let bytes = payload.octets();
            if bytes[0] != 3 || bytes[1] > 1 {
                return None;
            }
            Some(PropertyRecoveryRegion {
                operation_index,
                source_index: step.source_index,
                object: *object,
                address: u16::from_be_bytes([bytes[2], bytes[3]]),
                data: vec![0; usize::from(u16::from_be_bytes([bytes[4], bytes[5]]))],
            })
        })
        .collect()
}
fn same_region_shape(a: &PropertyRecoveryRegion, b: &PropertyRecoveryRegion) -> bool {
    a.operation_index == b.operation_index
        && a.source_index == b.source_index
        && a.object == b.object
        && a.address == b.address
        && a.data.len() == b.data.len()
}
fn allocation_slice(
    allocations: &[PropertyRecoveryRegion],
    object: u8,
    address: u16,
    length: usize,
) -> Result<&[u8], PropertySimulationFailure> {
    allocations
        .iter()
        .filter(|allocation| allocation.object == object && address >= allocation.address)
        .find_map(|allocation| {
            let offset = usize::from(address - allocation.address);
            allocation.data.get(offset..offset + length)
        })
        .ok_or_else(|| invalid("write prestate is outside full captured allocation"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_refusal_names_object_elements_octets_and_bytes() {
        let reason = pointer_width_refusal(ObjectIndex::new(3), 2, &[0x3C, 0x00]);
        assert_eq!(
            reason,
            "resource pointer response is not four octets: object 3, 2 elements, 2 octets, 3c00"
        );
        let one = pointer_width_refusal(ObjectIndex::new(1), 1, &[0xAB]);
        assert!(one.ends_with("object 1, 1 element, 1 octet, ab"), "{one}");
    }

    #[test]
    fn width_refusal_has_an_exact_hex_prefix_boundary() {
        let exact = pointer_width_refusal(ObjectIndex::new(3), 1, &[0x11; 16]);
        assert!(exact.ends_with(&"11".repeat(16)));
        assert!(!exact.contains("..."));
        let longer = pointer_width_refusal(ObjectIndex::new(3), 1, &[0x11; 17]);
        assert!(longer.contains("17 octets"));
        assert!(longer.ends_with(&format!("{}...", "11".repeat(16))));
    }

    #[test]
    fn width_refusal_bounds_a_long_answer_and_keeps_its_true_length() {
        let reason = pointer_width_refusal(ObjectIndex::new(3), 1, &[0x11; 40]);
        assert!(reason.contains("40 octets"), "{reason}");
        assert!(reason.contains(&"11".repeat(16)), "{reason}");
        assert!(!reason.contains(&"11".repeat(17)), "{reason}");
        assert!(reason.ends_with("..."), "{reason}");
    }
}
