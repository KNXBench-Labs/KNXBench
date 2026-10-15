//! Validated normalized property plans remain separate from source mapping and hardware admission.
//!
//! These offline operations use ten-octet property controls, never the 070nh
//! memory grammar. Explicit object indices are already resolved by the caller;
//! this module does not translate XML LsmIdx or SegFlags. See the P3 contract.

use std::collections::BTreeMap;
use std::fmt;

use crate::{ContactableAddress, IndividualAddress};

use super::load_control::LoadControlPayload;
use super::load_control_memory::SegmentMemoryType;
use super::load_state::{LoadState, MaskVersion};

/// One normalized operation; none of these values can open a transport.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropertyOperation {
    /// Open the management connection without yet enabling Verify Mode.
    Connect,
    /// Compare the six octets of Device Object PID_HARDWARE_TYPE exactly.
    HardwareCheck { expected: [u8; 6] },
    /// Write an explicitly resolved object's ten-octet property control.
    LoadControl {
        object: u8,
        payload: LoadControlPayload,
    },
    /// Write data within that object's preceding absolute allocation.
    WriteMemory {
        object: u8,
        address: u16,
        data: Vec<u8>,
    },
    /// Preserve a tool instruction without generating a wire operation.
    ToolMetadata { table_address: u16 },
    /// Close the management connection.
    Disconnect,
}

/// A source position and the operation it produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDownloadStep {
    /// Zero-based original position; allocation data may share its position.
    pub source_index: usize,
    /// Resolved operation, with no implicit skipping of tool metadata.
    pub operation: PropertyOperation,
}

/// Possible mutation classes, not a backup or proof that they are reversible.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MutationEffect {
    /// Connection authorization may change ephemeral access rights.
    ConnectionAuthorization,
    /// Verify Mode setup after identity checks; must be covered by recovery.
    VerifyModeSetup,
    /// A control changes load state, property data or allocator/task state.
    LoadControl {
        source_index: usize,
        object: u8,
        indirect_allocation: bool,
    },
    /// A direct data write, distinct from indirect allocation effects.
    MemoryWrite {
        source_index: usize,
        address: u16,
        length: usize,
    },
}

/// A failed structural/ordering check, located at a normalized operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyPlanError {
    /// Normalized operation offset, not necessarily the source position.
    pub operation: usize,
    /// Explicit reason; no input is truncated or silently discarded.
    pub reason: &'static str,
}
impl fmt::Display for PropertyPlanError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "property plan operation {}: {}",
            self.operation, self.reason
        )
    }
}
impl std::error::Error for PropertyPlanError {}

/// An immutable, structurally complete offline plan for the bounded 0025 slice.
///
/// Success proves neither product-source mapping, per-resource profile support,
/// durable recovery nor physical compatibility. Only the simulator consumes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyDownloadPlan {
    target: IndividualAddress,
    mask: MaskVersion,
    manufacturer: u16,
    steps: Vec<PropertyDownloadStep>,
    source_positions: usize,
    effects: Vec<MutationEffect>,
}

impl PropertyDownloadPlan {
    /// Validate every operation before a caller can execute any of them.
    pub fn new(
        target: IndividualAddress,
        mask: MaskVersion,
        manufacturer: u16,
        steps: Vec<PropertyDownloadStep>,
    ) -> Result<Self, PropertyPlanError> {
        let fail = |operation, reason| PropertyPlanError { operation, reason };
        if mask != MaskVersion(0x0025) {
            return Err(fail(0, "mask is outside this bounded offline slice"));
        }
        if ContactableAddress::new(target).is_err() {
            return Err(fail(0, "target is excluded"));
        }
        if !matches!(
            steps.first().map(|s| &s.operation),
            Some(PropertyOperation::Connect)
        ) || !matches!(
            steps.last().map(|s| &s.operation),
            Some(PropertyOperation::Disconnect)
        ) {
            return Err(fail(
                0,
                "plan needs one opening Connect and a final Disconnect",
            ));
        }
        let mut previous: Option<usize> = None;
        let mut checked = false;
        let mut states = BTreeMap::new();
        let mut allocations = Vec::<(u8, u32, u32, usize)>::new();
        let mut writes = Vec::<(u32, u32)>::new();
        let mut effects = vec![
            MutationEffect::ConnectionAuthorization,
            MutationEffect::VerifyModeSetup,
        ];
        for (index, step) in steps.iter().enumerate() {
            let source = step.source_index;
            let continuation = previous == Some(source)
                && matches!(step.operation, PropertyOperation::WriteMemory { .. });
            let ordered = match previous {
                None => source == 0,
                Some(last) => last.checked_add(1) == Some(source) || continuation,
            };
            if !ordered {
                return Err(fail(
                    index,
                    "source positions are missing, repeated or reordered",
                ));
            }
            previous = Some(source);
            match &step.operation {
                PropertyOperation::Connect if index == 0 => {}
                PropertyOperation::Disconnect if index == steps.len() - 1 => {
                    if !checked
                        || states.is_empty()
                        || states.values().any(|s| *s != LoadState::Loaded)
                    {
                        return Err(fail(index, "all touched resources must complete loading"));
                    }
                }
                PropertyOperation::Connect | PropertyOperation::Disconnect => {
                    return Err(fail(index, "connection boundary is not outermost"));
                }
                PropertyOperation::HardwareCheck { .. } => {
                    if checked || !states.is_empty() {
                        return Err(fail(
                            index,
                            "hardware guard is repeated or follows mutation",
                        ));
                    }
                    checked = true;
                }
                PropertyOperation::ToolMetadata { .. } => {}
                PropertyOperation::LoadControl { object, payload } => {
                    if !checked {
                        return Err(fail(index, "hardware comparison must precede mutation"));
                    }
                    if !matches!(object, 1..=3) {
                        return Err(fail(
                            index,
                            "object is not resolved in the bounded resource set",
                        ));
                    }
                    let bytes = payload.octets();
                    let loading = states.get(object) == Some(&LoadState::Loading);
                    match bytes[0] {
                        4 => {
                            states.insert(*object, LoadState::Unloaded);
                        }
                        1 if states.get(object) == Some(&LoadState::Unloaded) => {
                            states.insert(*object, LoadState::Loading);
                        }
                        2 if loading => {
                            states.insert(*object, LoadState::Loaded);
                        }
                        3 if loading => match bytes[1] {
                            0 | 1 => {
                                let start = u32::from(u16::from_be_bytes([bytes[2], bytes[3]]));
                                let length = u32::from(u16::from_be_bytes([bytes[4], bytes[5]]));
                                let end = start + length;
                                if length == 0
                                    || end > 0x1_0000
                                    || SegmentMemoryType::from_octet(bytes[7]).is_none()
                                    || bytes[8] & 0x7F != 0
                                    || bytes[9] != 0
                                {
                                    return Err(fail(index, "malformed absolute allocation"));
                                }
                                if allocations
                                    .iter()
                                    .any(|(_, a, b, _)| start < *b && *a < end)
                                {
                                    return Err(fail(index, "absolute allocations overlap"));
                                }
                                allocations.push((*object, start, end, source));
                            }
                            2 | 5 if *object == 3 => {}
                            _ => {
                                return Err(fail(
                                    index,
                                    "unsupported additional control/resource pair",
                                ))
                            }
                        },
                        _ => {
                            return Err(fail(index, "load lifecycle is incomplete or out of order"))
                        }
                    }
                    effects.push(MutationEffect::LoadControl {
                        source_index: source,
                        object: *object,
                        indirect_allocation: bytes[0] == 3 && bytes[1] <= 2,
                    });
                }
                PropertyOperation::WriteMemory {
                    object,
                    address,
                    data,
                } => {
                    let start = u32::from(*address);
                    let length = u32::try_from(data.len()).ok();
                    let end = length.and_then(|length| start.checked_add(length));
                    let Some(end) = end.filter(|end| *end > start && *end <= 0x1_0000) else {
                        return Err(fail(index, "empty or overflowing memory write"));
                    };
                    if !continuation
                        || states.get(object) != Some(&LoadState::Loading)
                        || !allocations.iter().any(|(o, a, b, s)| {
                            o == object && *s == source && *a <= start && end <= *b
                        })
                    {
                        return Err(fail(
                            index,
                            "memory write has no matching active source allocation",
                        ));
                    }
                    if writes.iter().any(|(a, b)| start < *b && *a < end) {
                        return Err(fail(index, "memory writes overlap"));
                    }
                    writes.push((start, end));
                    effects.push(MutationEffect::MemoryWrite {
                        source_index: source,
                        address: *address,
                        length: data.len(),
                    });
                }
            }
        }
        let source_positions = previous
            .and_then(|p| p.checked_add(1))
            .ok_or_else(|| fail(steps.len(), "source position overflow"))?;
        Ok(Self {
            target,
            mask,
            manufacturer,
            steps,
            source_positions,
            effects,
        })
    }

    /// The immutable operations in their retained order.
    pub fn steps(&self) -> &[PropertyDownloadStep] {
        &self.steps
    }
    /// Explicit target, not permission to contact hardware.
    pub fn target(&self) -> IndividualAddress {
        self.target
    }
    /// Expected mask for identity checking, not profile admission.
    pub fn mask(&self) -> MaskVersion {
        self.mask
    }
    /// Expected manufacturer identity.
    pub fn manufacturer(&self) -> u16 {
        self.manufacturer
    }
    /// Number of completely represented source positions.
    pub fn source_positions(&self) -> usize {
        self.source_positions
    }
    /// All modeled mutation classes; no recovery completeness claim follows.
    pub fn effects(&self) -> &[MutationEffect] {
        &self.effects
    }
}
