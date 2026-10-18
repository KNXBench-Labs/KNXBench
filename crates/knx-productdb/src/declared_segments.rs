//! The read-only `system2-declared-segments-v1` profile, derived from one ProgramCode.
//!
//! ADR-0143. The ranges are the `LdCtrlAbsSegment` declarations of the one
//! product procedure `system2::project_tool_metadata` already accepts, in
//! source order, plus the procedure's Device Object `PID_HARDWARE_TYPE`
//! comparison. They are source-declared *candidate* locations, not proven
//! runtime addresses; nothing here admits a write or a download.

use std::fmt;

use knx_core::commissioning::declared_memory::{
    DeclaredRange, DeclaredRanges, RangeError, MAX_COMPARISON_OCTETS,
};
use knx_core::commissioning::properties::PID_HARDWARE_TYPE;

use crate::code::{LoadStep, ProgramCode};
use crate::system2::{project_tool_metadata, MetadataError};

/// The profile's name in previews and archives.
pub const PROFILE: &str = "system2-declared-segments-v1";

/// What the profile reads and what the device must prove first.
///
/// Only [`derive_declared_segments`] makes one, from a ProgramCode; outside
/// code reads it through accessors and can neither build nor edit it:
///
/// ```compile_fail,E0451
/// use knx_core::commissioning::declared_memory::{DeclaredRange, DeclaredRanges};
/// use knx_productdb::declared_segments::DeclaredSegments;
/// let ranges = DeclaredRanges::new(vec![DeclaredRange { address: 0x4000, length: 16 }]).unwrap();
/// let _forged = DeclaredSegments {
///     program_id: "M-0001_A-0001-01-0000".into(),
///     mask: 0x0025,
///     ranges,
///     range_positions: vec![1],
///     compare_position: 0,
///     hardware_type: vec![1],
/// };
/// ```
///
/// ```compile_fail,E0616
/// use knx_core::commissioning::declared_memory::DeclaredRanges;
/// use knx_productdb::declared_segments::DeclaredSegments;
/// fn widen(profile: &mut DeclaredSegments, ranges: DeclaredRanges) {
///     profile.ranges = ranges;
/// }
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredSegments {
    program_id: String,
    mask: u16,
    ranges: DeclaredRanges,
    range_positions: Vec<usize>,
    compare_position: usize,
    hardware_type: Vec<u8>,
}

impl DeclaredSegments {
    /// `ApplicationProgram/@Id`.
    pub fn program_id(&self) -> &str {
        &self.program_id
    }
    /// The declared mask (`MV-xxxx`), the identity the device must show.
    pub fn mask(&self) -> u16 {
        self.mask
    }
    /// The ranges, in source order.
    pub fn ranges(&self) -> &DeclaredRanges {
        &self.ranges
    }
    /// Zero-based procedure position of each range's `LdCtrlAbsSegment`.
    pub fn range_positions(&self) -> &[usize] {
        &self.range_positions
    }
    /// Zero-based procedure position of the `LdCtrlCompareProp`.
    pub fn compare_position(&self) -> usize {
        self.compare_position
    }
    /// Its `@InlineData`, compared with `property_matches`.
    pub fn hardware_type(&self) -> &[u8] {
        &self.hardware_type
    }
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
    /// The comparison is longer than [`MAX_COMPARISON_OCTETS`], so the
    /// evidence archive could not record it.
    HardwareTypeTooLong {
        /// Octets declared.
        octets: usize,
    },
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
            Self::HardwareTypeTooLong { octets } => write!(
                f,
                "the PID_HARDWARE_TYPE comparison has {octets} octets, more than {MAX_COMPARISON_OCTETS}"
            ),
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
    if hardware_type.len() > MAX_COMPARISON_OCTETS {
        return Err(DeclaredSegmentsError::HardwareTypeTooLong {
            octets: hardware_type.len(),
        });
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
