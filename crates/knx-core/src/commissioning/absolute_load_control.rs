//! Absolute property-load payloads, independent of mask admission and transport.
//!
//! Management Procedures v02.01.02 §3.31.3.4, printed pp139–141: ten-octet
//! property records. Unlike §3.31.2's memory records, these have no machine
//! nibble or segment-ID octet. Memory-attribute bit7 = 0 disables checksum
//! control (p140). Keep the property grammar separate from memory records;
//! this codec does not interpret product XML SegFlags.

use std::fmt;

use super::load_control::{additional_load_control, LoadControlPayload, LoadControlSubtype};
use super::load_control_memory::SegmentMemoryType;

/// Absolute data/stack allocation fields of the property-control grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AbsolutePropertySegment {
    /// First 16-bit address.
    pub start: u16,
    /// Nonzero length, ending no later than FFFFh.
    pub length: u16,
    /// Low nibble write access, high nibble read access.
    pub access: u8,
    /// Zero-page RAM, RAM or EEPROM; reserved codes are not constructible.
    pub memory_type: SegmentMemoryType,
    /// Clear property-record bit7 to disable checksum control (MP p140).
    pub checksum_disabled: bool,
}

/// An allocation that cannot be represented without truncation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AbsolutePropertyError {
    /// No zero-length segment is defined by EEEE-SSSS+1.
    Empty,
    /// The segment exceeds the 16-bit address space.
    PastAddressSpace { start: u16, length: u16 },
}
impl fmt::Display for AbsolutePropertyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid absolute property allocation: {self:?}")
    }
}
impl std::error::Error for AbsolutePropertyError {}

/// Builds subtype 00h. Encoding does not establish per-profile support or recovery.
pub fn absolute_data(
    segment: AbsolutePropertySegment,
) -> Result<LoadControlPayload, AbsolutePropertyError> {
    absolute(LoadControlSubtype::AllocAbsDataSeg, segment)
}

/// Builds subtype 01h; the caller still needs profile/resource admission.
pub fn absolute_stack(
    segment: AbsolutePropertySegment,
) -> Result<LoadControlPayload, AbsolutePropertyError> {
    absolute(LoadControlSubtype::AllocAbsStackSeg, segment)
}

/// Builds subtype 02h from the same task-identity words as the memory grammar.
/// This reuses fields, never the eleven-octet memory encoding.
pub fn task_segment(task: super::load_control_memory::TaskSegment) -> LoadControlPayload {
    let start = task.start.to_be_bytes();
    let manufacturer = task.manufacturer.to_be_bytes();
    let application = task.application.to_be_bytes();
    additional_load_control(
        LoadControlSubtype::AllocAbsTaskSeg,
        [
            start[0],
            start[1],
            task.pei_type,
            manufacturer[0],
            manufacturer[1],
            application[0],
            application[1],
            task.version,
        ],
    )
}

fn absolute(
    subtype: LoadControlSubtype,
    segment: AbsolutePropertySegment,
) -> Result<LoadControlPayload, AbsolutePropertyError> {
    if segment.length == 0 {
        return Err(AbsolutePropertyError::Empty);
    }
    if u32::from(segment.start) + u32::from(segment.length) > 0x1_0000 {
        return Err(AbsolutePropertyError::PastAddressSpace {
            start: segment.start,
            length: segment.length,
        });
    }
    let start = segment.start.to_be_bytes();
    let length = segment.length.to_be_bytes();
    Ok(additional_load_control(
        subtype,
        [
            start[0],
            start[1],
            length[0],
            length[1],
            segment.access,
            segment.memory_type.octet(),
            if segment.checksum_disabled { 0 } else { 0x80 },
            0,
        ],
    ))
}
