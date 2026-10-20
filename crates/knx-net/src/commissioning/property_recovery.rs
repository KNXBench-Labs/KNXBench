//! Operation-bound recovery evidence for experimental same-application reinstallation.
//!
//! These values are not a device image or allocator rollback certificate. The
//! caller owns durable serialization, sync/readback and journal integrity. No
//! filesystem or proprietary source interpretation belongs in this module.

use std::collections::BTreeMap;

use super::property_download::PropertySimulationFailure;
use super::{ManagementSession, SessionError};
use crate::management::ManagementTransport;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::memory::{chunks, WriteLimit};
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
    pub serial: [u8; 6],
    pub program_version: [u8; 5],
    /// Experimental last absolute/task-segment reference model, four octets
    /// only. This explicit source expectation is not a physical profile proof.
    pub source_pointers: Vec<PropertyResourcePointer>,
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
        })
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
        Ok(())
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
/// An observed stable Loaded state and exact source-applicable pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRecoveryResource {
    pub object: u8,
    pub state: LoadState,
    pub pointer: [u8; 4],
}
/// Versioned operation-specific recovery, not a raw persistent-device backup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyRecoverySnapshot {
    pub version: u16,
    pub original_plan: PropertyDownloadPlan,
    pub preconditions: ExperimentalPropertyPreconditions,
    pub old_device_control: u8,
    pub resources: Vec<PropertyRecoveryResource>,
    /// Full declared absolute allocation intervals, including unwritten/masked bytes.
    pub allocations: Vec<PropertyRecoveryRegion>,
    /// Exact WriteMemory prestate, cross-checked against full allocation slices.
    pub regions: Vec<PropertyRecoveryRegion>,
}
impl PropertyRecoverySnapshot {
    /// Rebind persisted evidence to the complete exact original operation list.
    /// Only old WriteMemory data may differ; no commands are recovered from PID5.
    pub fn recovery_plan(
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
                .any(|(resource, expected)| {
                    resource.object != expected.object
                        || resource.state != LoadState::Loaded
                        || resource.pointer != expected.value
                })
        {
            return Err(invalid(
                "snapshot resource coverage differs from original source shape",
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

    pub(crate) async fn verify_outcome<T: ManagementTransport>(
        &self,
        session: &mut ManagementSession<'_, T>,
        executed_plan: &PropertyDownloadPlan,
    ) -> Result<(), PropertySimulationFailure> {
        let limit = session.write_limit(None).await?;
        for allocation in &self.allocations {
            let mut expected = allocation.data.clone();
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
                        }
                    }
                }
            }
            if read_region(session, allocation.address, &expected, limit).await? != expected {
                return Err(invalid(
                    "full allocation verification differs, including preserved/unwritten bytes",
                ));
            }
        }
        for resource in &self.resources {
            let object = ObjectIndex::new(resource.object);
            if session.read_load_state(object).await? != resource.state {
                return Err(invalid(
                    "final resource state differs from captured Loaded state",
                ));
            }
            if session
                .read_property(object, PID_TABLE_REFERENCE, 1, 1)
                .await?
                .as_slice()
                != resource.pointer
            {
                return Err(invalid(
                    "final resource pointer differs from captured source shape",
                ));
            }
        }
        // Interrupted current application identity may be absent at recovery start,
        // but a successful full replay must have reinstated the captured identity.
        // Do not seed/write a guessed identity outside the declared procedure.
        if session
            .read_property(ObjectIndex::new(3), PID_PROGRAM_VERSION, 1, 1)
            .await?
            .as_slice()
            != self.preconditions.program_version
        {
            return Err(invalid("final application identity differs from captured program version; recovery remains unverified"));
        }
        // A bounded setup lifecycle, not an automatic recovery of load state or
        // memory on failure. Preserve unrelated *current* PID14 bits.
        let current = session.read_device_control().await?;
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
            .await?;
        if session.read_device_control().await? != wanted {
            return Err(invalid("prior VerifyMode bit did not restore"));
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
}

pub(crate) fn invalid(reason: &str) -> PropertySimulationFailure {
    PropertySimulationFailure::Recovery(reason.to_string())
}

pub(crate) async fn validate_serial<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    preconditions: &ExperimentalPropertyPreconditions,
) -> Result<(), PropertySimulationFailure> {
    let serial = session.read_serial_number().await?;
    if serial.octets() != preconditions.serial {
        return Err(invalid(
            "serial number differs from exact expected identity",
        ));
    }
    Ok(())
}

pub(crate) async fn validate_identity<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    preconditions: &ExperimentalPropertyPreconditions,
) -> Result<(), PropertySimulationFailure> {
    validate_serial(session, preconditions).await?;
    let version = session
        .read_property(ObjectIndex::APPLICATION_PROGRAM, PID_PROGRAM_VERSION, 1, 1)
        .await?;
    if version.as_slice() != preconditions.program_version {
        return Err(invalid(
            "program version differs from same-application precondition",
        ));
    }
    Ok(())
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

pub(crate) async fn acquire<T: ManagementTransport>(
    session: &mut ManagementSession<'_, T>,
    plan: &PropertyDownloadPlan,
    preconditions: &ExperimentalPropertyPreconditions,
) -> Result<PropertyRecoverySnapshot, PropertySimulationFailure> {
    validate_identity(session, preconditions).await?;
    let mut resources = Vec::new();
    for expected in &preconditions.source_pointers {
        let object = ObjectIndex::new(expected.object);
        let state = session.read_load_state(object).await?;
        if state != LoadState::Loaded {
            return Err(invalid(
                "recovery acquisition requires every touched resource Loaded",
            ));
        }
        let pointer = session
            .read_property(object, PID_TABLE_REFERENCE, 1, 1)
            .await?;
        if pointer.as_slice() != expected.value {
            return Err(invalid(
                "current resource pointer differs from original source allocation shape",
            ));
        }
        resources.push(PropertyRecoveryResource {
            object: expected.object,
            state,
            pointer: expected.value,
        });
    }
    let limit = session.write_limit(None).await?;
    let mut allocations = allocation_shapes(plan);
    for allocation in &mut allocations {
        allocation.data = read_region(session, allocation.address, &allocation.data, limit).await?;
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
            != LoadState::Loaded
        {
            return Err(invalid(
                "resource ceased to be Loaded during recovery acquisition",
            ));
        }
    }
    Ok(PropertyRecoverySnapshot {
        version: 1,
        original_plan: plan.clone(),
        preconditions: preconditions.clone(),
        old_device_control,
        resources,
        allocations,
        regions,
    })
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
