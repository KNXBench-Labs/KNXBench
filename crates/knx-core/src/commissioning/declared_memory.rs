//! Validated, source-declared memory ranges for a bounded read-only evidence profile.
//!
//! The ranges are *candidate* locations a product's load procedure declares
//! (ADR-0143). Nothing here claims they are runtime addresses, and nothing
//! here can describe a write: the type only plans `A_Memory_Read` chunks.

use std::fmt;

use super::memory::{
    service_for, MemoryService, APDU_OVERHEAD_OCTETS, DEFAULT_APDU_LENGTH, MEMORY_SERVICE_LIMIT,
};

/// At most this many ranges per profile. ADR-0143: the bounded evidence
/// read is for one product procedure, not a memory dump.
pub const MAX_RANGES: usize = 8;

/// At most this many octets per profile (ADR-0143). The UP 258D12 declares
/// 1514; the ceiling leaves room for a sibling application without
/// admitting a generic dump.
pub const MAX_TOTAL_OCTETS: u32 = 2048;

/// Octets per `A_Memory_Read`: the standard-frame limit, `15 − 3`.
///
/// `[D]` RES §4.3.7.2.1 manages a device without `PID_MAX_APDU_LENGTH` with
/// standard frames of at most 15 APDU octets, and AL §3.5.3 subtracts three
/// for APCI and address. The session's negotiated
/// [`super::memory::write_limit`] is never below this value, so capping it
/// at a standard frame (the only frame this workspace's cEMI encoder sends)
/// always yields exactly this constant; no `PID_MAX_APDU_LENGTH` read can
/// change it.
pub const STANDARD_FRAME_CHUNK_OCTETS: u8 = (DEFAULT_APDU_LENGTH - APDU_OVERHEAD_OCTETS) as u8;

/// One declared range, as the source states it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeclaredRange {
    /// First address.
    pub address: u16,
    /// Length in octets.
    pub length: u16,
}

impl DeclaredRange {
    fn end(self) -> u32 {
        u32::from(self.address) + u32::from(self.length)
    }
}

/// Why a set of ranges is not a bounded read-only profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeError {
    /// No range at all.
    Empty,
    /// More than [`MAX_RANGES`].
    TooManyRanges {
        /// Ranges given.
        count: usize,
    },
    /// A zero-length range.
    ZeroLength {
        /// Zero-based source order.
        index: usize,
    },
    /// The range reaches `FFFFh`, where CP §3.5.2 switches to the
    /// user-memory service.
    OutsideMemoryService {
        /// Zero-based source order.
        index: usize,
    },
    /// Two ranges share an octet.
    Overlap {
        /// The range declared at the lower address.
        first: usize,
        /// The other range.
        second: usize,
    },
    /// More than [`MAX_TOTAL_OCTETS`].
    TooManyOctets {
        /// Octets declared.
        total: u32,
    },
}

impl fmt::Display for RangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => write!(f, "no memory range is declared"),
            Self::TooManyRanges { count } => {
                write!(f, "{count} ranges exceed the limit of {MAX_RANGES}")
            }
            Self::ZeroLength { index } => write!(f, "range {} has zero length", index + 1),
            Self::OutsideMemoryService { index } => write!(
                f,
                "range {} reaches {MEMORY_SERVICE_LIMIT:04X}h, outside A_Memory_Read",
                index + 1
            ),
            Self::Overlap { first, second } => {
                write!(f, "ranges {} and {} overlap", first + 1, second + 1)
            }
            Self::TooManyOctets { total } => {
                write!(f, "{total} octets exceed the limit of {MAX_TOTAL_OCTETS}")
            }
        }
    }
}

impl std::error::Error for RangeError {}

/// Non-empty, non-overlapping, bounded ranges in source order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeclaredRanges(Vec<DeclaredRange>);

/// One planned `A_Memory_Read`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Chunk {
    /// Zero-based index of the range it belongs to.
    pub range: usize,
    /// First address.
    pub address: u16,
    /// Octets requested.
    pub length: u8,
}

impl DeclaredRanges {
    /// Validates `ranges`; refuses rather than trimming anything.
    pub fn new(ranges: Vec<DeclaredRange>) -> Result<Self, RangeError> {
        if ranges.is_empty() {
            return Err(RangeError::Empty);
        }
        if ranges.len() > MAX_RANGES {
            return Err(RangeError::TooManyRanges {
                count: ranges.len(),
            });
        }
        let mut total: u32 = 0;
        for (index, range) in ranges.iter().enumerate() {
            if range.length == 0 {
                return Err(RangeError::ZeroLength { index });
            }
            // Every chunk ends at or before the range's end, so a range whose
            // end stays below the boundary keeps every chunk on A_Memory_Read.
            if range.end() >= MEMORY_SERVICE_LIMIT {
                return Err(RangeError::OutsideMemoryService { index });
            }
            total += u32::from(range.length);
        }
        let mut order: Vec<usize> = (0..ranges.len()).collect();
        order.sort_by_key(|&index| ranges[index].address);
        for pair in order.windows(2) {
            if ranges[pair[0]].end() > u32::from(ranges[pair[1]].address) {
                return Err(RangeError::Overlap {
                    first: pair[0],
                    second: pair[1],
                });
            }
        }
        if total > MAX_TOTAL_OCTETS {
            return Err(RangeError::TooManyOctets { total });
        }
        Ok(Self(ranges))
    }

    /// The ranges, in source order.
    pub fn ranges(&self) -> &[DeclaredRange] {
        &self.0
    }

    /// Octets over all ranges.
    pub fn total_octets(&self) -> u32 {
        self.0.iter().map(|range| u32::from(range.length)).sum()
    }

    /// The reads, [`STANDARD_FRAME_CHUNK_OCTETS`] at a time, range by range.
    pub fn chunks(&self) -> Vec<Chunk> {
        let mut chunks = Vec::new();
        for (index, range) in self.0.iter().enumerate() {
            let mut offset: u16 = 0;
            while offset < range.length {
                let length = (range.length - offset).min(u16::from(STANDARD_FRAME_CHUNK_OCTETS));
                let address = range.address + offset;
                debug_assert_eq!(
                    service_for(u32::from(address), u32::from(length)),
                    MemoryService::Memory
                );
                chunks.push(Chunk {
                    range: index,
                    address,
                    length: length as u8,
                });
                offset += length;
            }
        }
        chunks
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commissioning::memory::{write_limit, ApduLengthSource};

    fn range(address: u16, length: u16) -> DeclaredRange {
        DeclaredRange { address, length }
    }

    #[test]
    fn ranges_keep_source_order_and_total() {
        let ranges = DeclaredRanges::new(vec![range(0x0400, 512), range(0x0116, 330)]).unwrap();
        assert_eq!(ranges.ranges(), [range(0x0400, 512), range(0x0116, 330)]);
        assert_eq!(ranges.total_octets(), 842);
    }

    #[test]
    fn every_unsafe_shape_is_refused_without_a_partial_result() {
        assert_eq!(DeclaredRanges::new(vec![]), Err(RangeError::Empty));
        assert_eq!(
            DeclaredRanges::new(vec![range(0x100, 1); MAX_RANGES + 1]),
            Err(RangeError::TooManyRanges {
                count: MAX_RANGES + 1
            })
        );
        assert_eq!(
            DeclaredRanges::new(vec![range(0x100, 4), range(0x200, 0)]),
            Err(RangeError::ZeroLength { index: 1 })
        );
        // The last octet must stay below the CP §3.5.2 user-memory boundary,
        // otherwise a chunk would be sent as A_UserMemory_Read.
        assert_eq!(
            DeclaredRanges::new(vec![range(0xFFF0, 15)]),
            Err(RangeError::OutsideMemoryService { index: 0 })
        );
        assert_eq!(
            DeclaredRanges::new(vec![range(0xFFFF, 0xFFFF)]),
            Err(RangeError::OutsideMemoryService { index: 0 })
        );
        assert!(DeclaredRanges::new(vec![range(0xFFF0, 14)]).is_ok());
        assert_eq!(
            DeclaredRanges::new(vec![range(0x100, 16), range(0x10F, 1)]),
            Err(RangeError::Overlap {
                first: 0,
                second: 1
            })
        );
        assert_eq!(
            DeclaredRanges::new(vec![range(0x200, 8), range(0x100, 0x101)]),
            Err(RangeError::Overlap {
                first: 1,
                second: 0
            })
        );
        assert!(DeclaredRanges::new(vec![range(0x100, 16), range(0x110, 1)]).is_ok());
        assert_eq!(
            DeclaredRanges::new(vec![range(0x100, 1024), range(0x1000, 1025)]),
            Err(RangeError::TooManyOctets { total: 2049 })
        );
        assert!(DeclaredRanges::new(vec![range(0x100, 1024), range(0x1000, 1024)]).is_ok());
    }

    #[test]
    fn the_negotiated_limit_capped_at_a_standard_frame_is_always_twelve() {
        for source in [
            ApduLengthSource::Absent,
            ApduLengthSource::RouterObjectOnly(254),
            ApduLengthSource::DeviceObject(15),
            ApduLengthSource::DeviceObject(55),
            ApduLengthSource::DeviceObject(254),
            ApduLengthSource::DeviceObject(255),
            ApduLengthSource::DeviceObject(3),
        ] {
            assert_eq!(
                write_limit(source)
                    .max_octets()
                    .min(STANDARD_FRAME_CHUNK_OCTETS),
                STANDARD_FRAME_CHUNK_OCTETS
            );
        }
        assert_eq!(STANDARD_FRAME_CHUNK_OCTETS, 12);
    }

    #[test]
    fn chunks_cover_each_range_exactly_once_in_order() {
        let ranges = DeclaredRanges::new(vec![range(0x0116, 30), range(0x3C00, 12)]).unwrap();
        let chunks = ranges.chunks();
        assert_eq!(
            chunks,
            [
                Chunk {
                    range: 0,
                    address: 0x0116,
                    length: 12
                },
                Chunk {
                    range: 0,
                    address: 0x0122,
                    length: 12
                },
                Chunk {
                    range: 0,
                    address: 0x012E,
                    length: 6
                },
                Chunk {
                    range: 1,
                    address: 0x3C00,
                    length: 12
                },
            ]
        );
    }

    #[test]
    fn the_up258_shape_needs_128_chunks() {
        let ranges = DeclaredRanges::new(vec![
            range(0x0116, 330),
            range(0x0260, 416),
            range(0x0400, 512),
            range(0x3C00, 256),
        ])
        .unwrap();
        assert_eq!(ranges.total_octets(), 1514);
        assert_eq!(ranges.chunks().len(), 28 + 35 + 43 + 22);
    }
}
