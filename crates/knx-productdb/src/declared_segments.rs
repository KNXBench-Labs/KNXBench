//! The read-only `system2-declared-segments-v1` profile, derived from one ProgramCode.
//!
//! ADR-0143. The ranges are the `LdCtrlAbsSegment` declarations of the one
//! product procedure `system2::project_tool_metadata` already accepts, in
//! source order, plus the procedure's Device Object `PID_HARDWARE_TYPE`
//! comparison. They are source-declared *candidate* locations, not proven
//! runtime addresses; nothing here admits a write or a download.

use std::fmt;

use knx_core::commissioning::declared_memory::{DeclaredRange, DeclaredRanges, RangeError};
use knx_core::commissioning::properties::PID_HARDWARE_TYPE;

use crate::code::{LoadStep, ProgramCode};
use crate::system2::{project_tool_metadata, MetadataError};

/// The profile's name in previews and archives.
pub const PROFILE: &str = "system2-declared-segments-v1";

/// What the profile reads and what the device must prove first.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredSegments {
    /// `ApplicationProgram/@Id`.
    pub program_id: String,
    /// The declared mask (`MV-xxxx`), the identity the device must show.
    pub mask: u16,
    /// The ranges, in source order.
    pub ranges: DeclaredRanges,
    /// Zero-based procedure position of each range's `LdCtrlAbsSegment`.
    pub range_positions: Vec<usize>,
    /// Zero-based procedure position of the `LdCtrlCompareProp`.
    pub compare_position: usize,
    /// Its `@InlineData`, compared with `property_matches`.
    pub hardware_type: Vec<u8>,
}

/// Why no profile exists for a program; never a partial profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeclaredSegmentsError {
    /// Not the exact System-2 shape the tool-metadata witness accepts.
    Shape(MetadataError),
    /// Not exactly one Device Object `PID_HARDWARE_TYPE` comparison.
    HardwareTypeComparison {
        /// Comparisons found.
        count: usize,
    },
    /// The comparison has no inline data, so nothing can match it.
    EmptyHardwareType,
    /// The declared ranges are not a bounded read-only profile.
    Ranges(RangeError),
}

impl fmt::Display for DeclaredSegmentsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Shape(error) => write!(f, "{error}"),
            Self::HardwareTypeComparison { count } => write!(
                f,
                "the load procedure has {count} PID_HARDWARE_TYPE comparisons, not exactly one"
            ),
            Self::EmptyHardwareType => {
                write!(f, "the PID_HARDWARE_TYPE comparison has no inline data")
            }
            Self::Ranges(error) => write!(f, "declared segments refused: {error}"),
        }
    }
}

impl std::error::Error for DeclaredSegmentsError {}

/// Derives the profile, or refuses it whole.
pub fn derive_declared_segments(
    code: &ProgramCode,
) -> Result<DeclaredSegments, DeclaredSegmentsError> {
    // The precondition is the existing witness, not a second copy of its checks.
    project_tool_metadata(code).map_err(DeclaredSegmentsError::Shape)?;
    let mask = code
        .mask_version
        .as_deref()
        .and_then(|mask| mask.strip_prefix("MV-"))
        .and_then(|hex| u16::from_str_radix(hex, 16).ok())
        .ok_or(DeclaredSegmentsError::Shape(
            MetadataError::UnsupportedProgram,
        ))?;
    let steps = &code.load_procedures[0].steps;
    let comparisons: Vec<(usize, &Vec<u8>)> = steps
        .iter()
        .enumerate()
        .filter_map(|(position, step)| match step {
            LoadStep::CompareProp {
                object_index: 0,
                property_id: PID_HARDWARE_TYPE,
                data,
            } => Some((position, data)),
            _ => None,
        })
        .collect();
    let [(compare_position, hardware_type)] = comparisons[..] else {
        return Err(DeclaredSegmentsError::HardwareTypeComparison {
            count: comparisons.len(),
        });
    };
    if hardware_type.is_empty() {
        return Err(DeclaredSegmentsError::EmptyHardwareType);
    }
    let (range_positions, ranges): (Vec<usize>, Vec<DeclaredRange>) = steps
        .iter()
        .enumerate()
        .filter_map(|(position, step)| match step {
            LoadStep::AbsSegment { address, size, .. } => Some((
                position,
                DeclaredRange {
                    address: *address,
                    length: *size,
                },
            )),
            _ => None,
        })
        .unzip();
    let ranges = DeclaredRanges::new(ranges).map_err(DeclaredSegmentsError::Ranges)?;
    Ok(DeclaredSegments {
        program_id: code.program_id.clone(),
        mask,
        ranges,
        range_positions,
        compare_position,
        hardware_type: hardware_type.clone(),
    })
}
