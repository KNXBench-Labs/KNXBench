//! Explicit experimental System-2 source rules produce complete ordered property plans.
//!
//! These are candidate interpretations, not generic download admission. The
//! caller must bind its source/profile and disclose each hypothesis; the normal
//! memory planner and strict System-2 source-image entry remain unchanged.

use std::collections::BTreeSet;
use std::fmt;

use knx_core::commissioning::absolute_load_control::{
    absolute_data, task_segment, AbsolutePropertySegment,
};
use knx_core::commissioning::load_control::{event_payload, task_control_2, TaskControl2};
use knx_core::commissioning::load_control_memory::{SegmentMemoryType, TaskSegment};
use knx_core::commissioning::load_state::{LoadEvent, MaskVersion};
use knx_core::commissioning::property_download::{
    PropertyDownloadPlan, PropertyDownloadStep, PropertyOperation,
};
use knx_core::IndividualAddress;

use crate::code::{LoadStep, ProgramCode};
use crate::image::DownloadImage;
use crate::system2::{project_tool_metadata, ProjectedStep};

/// Explicitly selected candidate rules; there is deliberately no Default.
///
/// This policy hypothesizes that the XML count already represents the complete
/// allocation, zero XML flags map to checksum-disabled property attributes and
/// table task records use zero identity. MP v02.01.02 defines the wire grammar,
/// not these XML mappings. A version-bound caller profile must disclose that
/// difference. See docs/research/2026-10-20-system2-validation-rules.md.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct System2ProcedurePolicy {
    declared_counts_zero_flags_zero_table_identity: (),
}

impl System2ProcedurePolicy {
    /// Select all three named hypotheses explicitly, never through live admission.
    pub fn declared_counts_zero_flags_zero_table_identity() -> Self {
        Self {
            declared_counts_zero_flags_zero_table_identity: (),
        }
    }
}

/// No partial plan is returned when a declaration cannot be represented.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct System2ProcedureError {
    /// Source index, when a particular declaration is responsible.
    pub source_index: Option<usize>,
    /// The missing or contradictory contract.
    pub reason: String,
}

impl fmt::Display for System2ProcedureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "System-2 candidate at source {:?}: {}",
            self.source_index, self.reason
        )
    }
}
impl std::error::Error for System2ProcedureError {}

fn failure(source_index: Option<usize>, reason: impl Into<String>) -> System2ProcedureError {
    System2ProcedureError {
        source_index,
        reason: reason.into(),
    }
}

fn attribute<T: std::str::FromStr>(
    code: &ProgramCode,
    key: &str,
) -> Result<T, System2ProcedureError> {
    code.program_attributes
        .get(key)
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| failure(None, format!("missing or invalid {key}")))
}

/// Translate every source instruction and every segment under explicit candidate rules.
///
/// Unknown steps, flags, memory types, non-binary masks, duplicate or missing
/// images and unallocated data refuse the entire plan. Protected bytes generate
/// no writes. Each emitted data interval retains its allocation's source index.
/// A successful result is not evidence of physical compatibility or recovery.
pub fn plan_system2_candidate(
    image: &DownloadImage,
    target: IndividualAddress,
    _policy: &System2ProcedurePolicy,
) -> Result<PropertyDownloadPlan, System2ProcedureError> {
    let code = &image.code;
    if code.has_fixup_list || !code.relative_segments.is_empty() {
        return Err(failure(
            None,
            "uninterpreted source transformations or relative segments",
        ));
    }
    if code.mask_version.as_deref() != Some("MV-0025") {
        return Err(failure(
            None,
            "only the explicit 0025 candidate grammar is represented",
        ));
    }
    let metadata = project_tool_metadata(code).map_err(|e| failure(None, e.to_string()))?;
    let manufacturer = code
        .program_id
        .split('_')
        .next()
        .and_then(|p| p.strip_prefix("M-"))
        .filter(|p| p.len() == 4)
        .and_then(|p| u16::from_str_radix(p, 16).ok())
        .ok_or_else(|| failure(None, "invalid manufacturer identity"))?;
    let application = TaskSegment {
        start: 0,
        manufacturer,
        pei_type: attribute(code, "PeiType")?,
        application: attribute(code, "ApplicationNumber")?,
        version: attribute(code, "ApplicationVersion")?,
    };
    let mut images = BTreeSet::new();
    for segment in &image.segments {
        if !images.insert(segment.id.as_str()) {
            return Err(failure(None, "duplicate image segment"));
        }
        let source = code
            .segment(&segment.id)
            .ok_or_else(|| failure(None, "image has no source segment"))?;
        if source.address != segment.address
            || source.size as usize != segment.octets.len()
            || source.data.is_none()
            || source.mask != segment.mask
            || segment.mask.as_ref().is_some_and(|mask| {
                mask.len() != segment.octets.len() || mask.iter().any(|b| !matches!(b, 0 | 0xFF))
            })
        {
            return Err(failure(
                None,
                "image length, placement or mask differs from complete source",
            ));
        }
    }
    let mut source_ids = BTreeSet::new();
    for segment in &code.segments {
        if !source_ids.insert(segment.id.as_str())
            || segment
                .data
                .as_ref()
                .is_none_or(|d| d.len() != segment.size as usize)
            || !images.contains(segment.id.as_str())
        {
            return Err(failure(
                None,
                "source segments must have unique complete images",
            ));
        }
    }
    let mut allocated = BTreeSet::new();
    let mut steps = Vec::new();
    let mut task_objects = BTreeSet::new();
    for (source_index, projected) in metadata.steps.iter().enumerate() {
        let fail = |why| failure(Some(source_index), why);
        let mut emit = |operation| {
            steps.push(PropertyDownloadStep {
                source_index,
                operation,
            })
        };
        let source = match projected {
            ProjectedStep::ToolTableReference { address } => {
                emit(PropertyOperation::ToolMetadata {
                    table_address: *address,
                });
                continue;
            }
            ProjectedStep::Retained(step) => *step,
        };
        let object = |lsm: u8| match lsm {
            1..=3 => Ok(lsm),
            _ => Err(fail("no explicit resource mapping for this LSM")),
        };
        match source {
            LoadStep::Connect => emit(PropertyOperation::Connect),
            LoadStep::Disconnect => emit(PropertyOperation::Disconnect),
            LoadStep::CompareProp {
                object_index: 0,
                property_id: 78,
                data,
            } if data.len() >= 6 && data.len() <= 10 && data[6..].iter().all(|b| *b == 0) => {
                let mut expected = [0; 6];
                expected.copy_from_slice(&data[..6]);
                emit(PropertyOperation::HardwareCheck { expected });
            }
            LoadStep::Unload { lsm } | LoadStep::Load { lsm } | LoadStep::LoadCompleted { lsm } => {
                let event = match source {
                    LoadStep::Unload { .. } => LoadEvent::Unload,
                    LoadStep::Load { .. } => LoadEvent::StartLoading,
                    _ => LoadEvent::LoadCompleted,
                };
                emit(PropertyOperation::LoadControl {
                    object: object(*lsm)?,
                    payload: event_payload(event),
                });
            }
            LoadStep::AbsSegment {
                lsm,
                segment_type: 0,
                address,
                size,
                access,
                memory_type,
                flags: 0,
            } => {
                let object = object(*lsm)?;
                let mut candidates = image.segments.iter().filter(|s| {
                    s.address == u32::from(*address) && s.octets.len() == usize::from(*size)
                });
                let segment = candidates
                    .next()
                    .ok_or_else(|| fail("allocation has no exact complete image"))?;
                if candidates.next().is_some() || !allocated.insert(segment.id.as_str()) {
                    return Err(fail("allocation image is ambiguous or repeated"));
                }
                let memory_type = SegmentMemoryType::from_octet(*memory_type)
                    .ok_or_else(|| fail("unsupported memory type"))?;
                let payload = absolute_data(AbsolutePropertySegment {
                    start: *address,
                    length: *size,
                    access: *access,
                    memory_type,
                    checksum_disabled: true,
                })
                .map_err(|e| failure(Some(source_index), e.to_string()))?;
                emit(PropertyOperation::LoadControl { object, payload });
                let mut offset = 0;
                while offset < segment.octets.len() {
                    let writable = |i: usize| segment.mask.as_ref().is_none_or(|m| m[i] == 0xFF);
                    if !writable(offset) {
                        offset += 1;
                        continue;
                    }
                    let start = offset;
                    while offset < segment.octets.len() && writable(offset) {
                        offset += 1;
                    }
                    let address = segment
                        .address
                        .checked_add(start as u32)
                        .and_then(|a| u16::try_from(a).ok())
                        .ok_or_else(|| fail("write exceeds address space"))?;
                    emit(PropertyOperation::WriteMemory {
                        object,
                        address,
                        data: segment.octets[start..offset].to_vec(),
                    });
                }
            }
            LoadStep::TaskSegment { lsm, address } => {
                let object = object(*lsm)?;
                if !task_objects.insert(object) {
                    return Err(fail("repeated task identity"));
                }
                if object != 3 {
                    let placement = if object == 1 {
                        code.address_table.as_ref()
                    } else {
                        code.association_table.as_ref()
                    };
                    let pointer = placement.and_then(|p| {
                        let segment = code.segment(p.code_segment.as_deref()?)?;
                        let offset = p.offset.filter(|offset| *offset < segment.size)?;
                        segment.address.checked_add(offset)
                    });
                    if pointer != Some(u32::from(*address)) {
                        return Err(fail("table task pointer differs from its source resource"));
                    }
                }
                let task = if object == 3 {
                    TaskSegment {
                        start: *address,
                        ..application
                    }
                } else {
                    TaskSegment {
                        start: *address,
                        pei_type: 0,
                        manufacturer: 0,
                        application: 0,
                        version: 0,
                    }
                };
                emit(PropertyOperation::LoadControl {
                    object,
                    payload: task_segment(task),
                });
            }
            LoadStep::TaskCtrl2 {
                lsm: 3,
                callback,
                address,
                segment_0,
                segment_1,
            } => {
                emit(PropertyOperation::LoadControl {
                    object: 3,
                    payload: task_control_2(TaskControl2 {
                        callback: *callback,
                        communication_objects: *address,
                        segment_1: *segment_0,
                        segment_2: *segment_1,
                    }),
                });
            }
            _ => {
                return Err(fail(
                    "unsupported source instruction or candidate policy value",
                ))
            }
        }
    }
    if allocated != images || task_objects != BTreeSet::from([1, 2, 3]) {
        return Err(failure(
            None,
            "complete image coverage and all three task identities are required",
        ));
    }
    PropertyDownloadPlan::new(target, MaskVersion(0x0025), manufacturer, steps)
        .map_err(|e| failure(None, e.to_string()))
}
