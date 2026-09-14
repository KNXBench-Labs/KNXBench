//! Chunking a memory region into writes: how long a chunk may be, and which service carries it.
//!
//! Pure arithmetic over spec §6.4 (`PID_MAX_APDU_LENGTH`, and the 12-octet
//! default that is 15 − 3) and §6.5 (`A_Memory_Write` below `FFFFh`,
//! `A_UserMemoryWrite` above it, decided on **base + length**). No I/O:
//! the caller reads the property, this decides what to send.

use std::fmt;

/// The largest `number` `A_Memory_Read`/`A_Memory_Write` can carry, `[D]`
/// AL §3.5.3/§3.5.4: *"between 1 and 63 octets"*.
pub const SERVICE_MAX_OCTETS: u8 = 63;

/// The APDU length a device without `PID_MAX_APDU_LENGTH` is managed
/// with. `[D]` RES §4.3.7.2.1: *"then the Management Client shall manage
/// the device with L_Data_Standard-frames with an APDU-length of maximal
/// 15 octets."*
pub const DEFAULT_APDU_LENGTH: u16 = 15;

/// The three octets AL §3.5.4's *"number is greater than Maximum APDU
/// Length – 3"* subtracts: the APCI/number octet and the two address
/// octets.
pub const APDU_OVERHEAD_OCTETS: u16 = 3;

/// The largest value `PID_MAX_APDU_LENGTH` may carry as a length. `[D]`
/// RES §4.3.7: *"The value of PID_MAX_APDU_LENGTH may be in the range
/// between 15 and 254"*.
pub const MAX_APDU_LENGTH: u16 = 254;

/// `[D]` RES §4.3.7 footnote 4: *"255 as value for PID_MAX_APDU_LENGTH is
/// an ESCape Code"* — not a length, and not a licence to write 252 octets.
pub const APDU_ESCAPE_CODE: u16 = 255;

/// The first address `A_Memory_Write` cannot reach. `[D]` CP §3.5.2:
/// *"if BaseAddress plus allocated memory is lower than FFFFh then MaC:
/// MemoryWrite(…); if … higher than FFFFh then MaC: UserMemoryWrite(…)"*.
pub const MEMORY_SERVICE_LIMIT: u32 = 0xFFFF;

/// Where a device's `PID_MAX_APDU_LENGTH` was found, which changes what
/// it means.
///
/// `[D]` RES §4.3.7.2.2: *"If PID_MAX_APDU_LENGTH is solely in the Router
/// Object …, then the device shall only support L_Data_Extended-frames
/// for Routing and only L_Data_Standard-frames for Management."* So a
/// value read from the wrong object says the opposite of what a careless
/// read would conclude, and this enum makes the caller say which it read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ApduLengthSource {
    /// The property is absent from the Device Object.
    Absent,
    /// Read from the Device Object (PID 56 there), the only place that
    /// governs management.
    DeviceObject(u16),
    /// Present only in the Router Object. Governs routing, not
    /// management: treated exactly as [`ApduLengthSource::Absent`].
    RouterObjectOnly(u16),
}

/// The negotiated write length for one device, and why it is that value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WriteLimit {
    max_octets: u8,
    reason: WriteLimitReason,
}

/// Why a [`WriteLimit`] came out the way it did, so a report can say so
/// instead of presenting 12 as folklore.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WriteLimitReason {
    /// `PID_MAX_APDU_LENGTH` absent from the Device Object: 15 − 3.
    PropertyAbsent,
    /// Present only in the Router Object, which does not govern
    /// management: 15 − 3.
    RouterObjectOnly,
    /// The value read was the ESCape Code 255, which is not a length:
    /// 15 − 3.
    EscapeCode,
    /// The value read was below the 15 the Standard's range starts at.
    /// Treated as 15, never as something smaller, because AL's own
    /// subtraction would otherwise underflow.
    BelowStandardRange,
    /// Derived from the value read: `min(v, 254) − 3`.
    FromDeviceObject,
    /// Derived from the value read, then capped at the 63 the service
    /// itself allows.
    CappedByServiceLimit,
}

impl WriteLimit {
    /// The chunk size to use, in octets of data. Never zero.
    pub fn max_octets(self) -> u8 {
        self.max_octets
    }

    /// Why it is that value.
    pub fn reason(self) -> WriteLimitReason {
        self.reason
    }
}

impl fmt::Display for WriteLimit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} octets", self.max_octets)
    }
}

/// Spec §6.4's algorithm, once, so nothing else implements it again.
///
/// ```text
/// read PID_MAX_APDU_LENGTH from the Device Object (PID 56)
///   absent          -> max write = 12 octets, L_Data_Standard frames only
///   present, value v-> max write = min(v, 254) - 3, capped at 63
///   value 255       -> ESCape Code, not a length; treat as "do not use", i.e. 12
/// ```
///
/// The default is 12 and is raised only from a value actually read from
/// the Device Object of the device in front of the caller — never from a
/// product database, a mask version or an assumption.
pub fn write_limit(source: ApduLengthSource) -> WriteLimit {
    let fallback = |reason| WriteLimit {
        max_octets: (DEFAULT_APDU_LENGTH - APDU_OVERHEAD_OCTETS) as u8,
        reason,
    };
    let value = match source {
        ApduLengthSource::Absent => return fallback(WriteLimitReason::PropertyAbsent),
        ApduLengthSource::RouterObjectOnly(_) => {
            return fallback(WriteLimitReason::RouterObjectOnly)
        }
        ApduLengthSource::DeviceObject(value) => value,
    };
    if value == APDU_ESCAPE_CODE {
        return fallback(WriteLimitReason::EscapeCode);
    }
    if value < DEFAULT_APDU_LENGTH {
        return fallback(WriteLimitReason::BelowStandardRange);
    }
    let usable = value.min(MAX_APDU_LENGTH) - APDU_OVERHEAD_OCTETS;
    if usable > SERVICE_MAX_OCTETS as u16 {
        WriteLimit {
            max_octets: SERVICE_MAX_OCTETS,
            reason: WriteLimitReason::CappedByServiceLimit,
        }
    } else {
        WriteLimit {
            max_octets: usable as u8,
            reason: WriteLimitReason::FromDeviceObject,
        }
    }
}

/// Which memory service carries a given write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MemoryService {
    /// `A_Memory_Read`/`A_Memory_Write`, 16-bit addresses, up to 64 kB.
    Memory,
    /// `A_UserMemory_Read`/`A_UserMemory_Write`, 20-bit addresses, up to
    /// 1 MB (CP §3.5.1.4 Table 5).
    UserMemory,
}

impl fmt::Display for MemoryService {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MemoryService::Memory => "A_Memory_Write",
            MemoryService::UserMemory => "A_UserMemoryWrite",
        })
    }
}

/// Picks the service for a region, per CP §3.5.2's straight comparison.
///
/// The condition is on **base + length**, not on the base alone: a region
/// that starts below `FFFFh` and ends above it uses the user-memory
/// service for the whole region. An implementation that switches on the
/// start address writes the tail of a segment to the wrong service.
pub fn service_for(base: u32, length: u32) -> MemoryService {
    if base.saturating_add(length) < MEMORY_SERVICE_LIMIT {
        MemoryService::Memory
    } else {
        MemoryService::UserMemory
    }
}

/// One write in a chunked region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chunk {
    /// Absolute address of this chunk's first octet.
    pub address: u32,
    /// Offset into the region's data, so the caller can slice.
    pub offset: usize,
    /// How many octets this chunk carries. Never zero, never above the
    /// [`WriteLimit`] and never above [`SERVICE_MAX_OCTETS`].
    pub length: u8,
    /// The service that carries it, chosen once for the whole region.
    pub service: MemoryService,
}

/// Why a region could not be chunked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChunkError {
    /// There is nothing to write. An empty write is not a no-op worth
    /// sending — and an allocation of size 0 is `PID_ERROR_CODE` 8.
    EmptyRegion,
    /// `base + length` would leave the 20-bit space
    /// `A_UserMemory_*`/`PID_TABLE_REFERENCE` can address at all.
    RegionOutOfAddressSpace { base: u32, length: u32 },
    /// The base address is the failure value `PID_REFERENCE` returns.
    /// `[D]` CP §3.5.2: *"if it is zero then allocation was not
    /// successful"*. Zero is not an address, so it never becomes one.
    AllocationFailed,
}

impl std::error::Error for ChunkError {}

impl fmt::Display for ChunkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ChunkError::EmptyRegion => write!(f, "nothing to write: the region is empty"),
            ChunkError::RegionOutOfAddressSpace { base, length } => write!(
                f,
                "region at {base:#x} of {length} octets leaves the 20-bit \
                 addressable space"
            ),
            ChunkError::AllocationFailed => write!(
                f,
                "base address is 0, which CP §3.5.2 defines as allocation failure, \
                 not as an address"
            ),
        }
    }
}

/// The top of the 20-bit space CP §3.5.1.4 Table 5 gives
/// `A_UserMemory_Write` and `PID_TABLE_REFERENCE`.
const USER_MEMORY_LIMIT: u32 = 1 << 20;

/// Splits `data` starting at `base` into writes no longer than `limit`.
///
/// Refuses a zero base outright: spec §7.2 step 3 and §9.2 both say
/// `PID_REFERENCE` = 0 means allocation failed, and the one thing the
/// implementation must never do with that value is compute a write offset
/// from it.
pub fn chunks(base: u32, data: &[u8], limit: WriteLimit) -> Result<Vec<Chunk>, ChunkError> {
    if base == 0 {
        return Err(ChunkError::AllocationFailed);
    }
    if data.is_empty() {
        return Err(ChunkError::EmptyRegion);
    }
    let length = data.len() as u32;
    if base.saturating_add(length) > USER_MEMORY_LIMIT {
        return Err(ChunkError::RegionOutOfAddressSpace { base, length });
    }
    // Chosen once for the region, from base + length, so a region that
    // straddles FFFFh does not change service half way through.
    let service = service_for(base, length);
    let step = limit.max_octets() as usize;
    let mut out = Vec::with_capacity(data.len().div_ceil(step));
    let mut offset = 0usize;
    while offset < data.len() {
        let length = step.min(data.len() - offset) as u8;
        out.push(Chunk {
            address: base + offset as u32,
            offset,
            length,
            service,
        });
        offset += length as usize;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn limit_of(n: u8) -> WriteLimit {
        WriteLimit {
            max_octets: n,
            reason: WriteLimitReason::FromDeviceObject,
        }
    }

    /// §14 item 13, every clause of it.
    #[test]
    fn the_apdu_length_rule_of_section_6_4() {
        // property absent => 12
        let absent = write_limit(ApduLengthSource::Absent);
        assert_eq!(absent.max_octets(), 12);
        assert_eq!(absent.reason(), WriteLimitReason::PropertyAbsent);

        // present with 15 => 12
        let fifteen = write_limit(ApduLengthSource::DeviceObject(15));
        assert_eq!(fifteen.max_octets(), 12);
        assert_eq!(fifteen.reason(), WriteLimitReason::FromDeviceObject);

        // with 254 => capped at 63 by the service limit, not 251
        let max = write_limit(ApduLengthSource::DeviceObject(254));
        assert_eq!(max.max_octets(), 63);
        assert_eq!(max.reason(), WriteLimitReason::CappedByServiceLimit);

        // with 255 => ESCape Code, not a length
        let escape = write_limit(ApduLengthSource::DeviceObject(255));
        assert_eq!(escape.max_octets(), 12);
        assert_eq!(escape.reason(), WriteLimitReason::EscapeCode);

        // only in the Router Object => still 12, per RES §4.3.7.2.2
        let router = write_limit(ApduLengthSource::RouterObjectOnly(254));
        assert_eq!(router.max_octets(), 12);
        assert_eq!(router.reason(), WriteLimitReason::RouterObjectOnly);
    }

    #[test]
    fn the_twelve_is_fifteen_minus_three_and_not_a_folklore_constant() {
        assert_eq!(DEFAULT_APDU_LENGTH - APDU_OVERHEAD_OCTETS, 12);
    }

    #[test]
    fn a_value_above_254_is_clamped_before_the_subtraction() {
        // 255 is the ESCape Code and handled above; anything larger than
        // the stated range is clamped to 254 rather than trusted.
        assert_eq!(
            write_limit(ApduLengthSource::DeviceObject(1000)).max_octets(),
            63
        );
    }

    #[test]
    fn a_value_below_the_standard_range_falls_back_rather_than_underflowing() {
        for value in 0..15u16 {
            let limit = write_limit(ApduLengthSource::DeviceObject(value));
            assert_eq!(limit.max_octets(), 12);
            assert_eq!(limit.reason(), WriteLimitReason::BelowStandardRange);
        }
    }

    #[test]
    fn no_negotiated_limit_ever_exceeds_the_service_limit() {
        for value in 0..=1000u16 {
            assert!(write_limit(ApduLengthSource::DeviceObject(value)).max_octets() <= 63);
        }
    }

    /// §14 item 3: chunking at 12 and at 63.
    #[test]
    fn chunking_at_the_twelve_octet_default() {
        let data = vec![0xAA; 30];
        let chunks = chunks(0x4000, &data, write_limit(ApduLengthSource::Absent)).unwrap();
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].length, 12);
        assert_eq!(chunks[0].address, 0x4000);
        assert_eq!(chunks[1].length, 12);
        assert_eq!(chunks[1].address, 0x400C);
        assert_eq!(chunks[2].length, 6);
        assert_eq!(chunks[2].address, 0x4018);
        assert_eq!(chunks.iter().map(|c| c.length as usize).sum::<usize>(), 30);
    }

    #[test]
    fn chunking_at_sixty_three() {
        let data = vec![0x5A; 200];
        let chunks = chunks(
            0x1000,
            &data,
            write_limit(ApduLengthSource::DeviceObject(254)),
        )
        .unwrap();
        assert_eq!(chunks.len(), 4);
        assert!(chunks.iter().all(|c| c.length <= 63));
        assert_eq!(chunks.last().unwrap().length, 11);
        // Contiguous, in order, no gaps and no overlaps.
        let mut expected = 0x1000u32;
        for chunk in &chunks {
            assert_eq!(chunk.address, expected);
            expected += chunk.length as u32;
        }
    }

    /// §14 item 3's second half: the §6.5 rule is exercised on
    /// `base + length`, not on `base`.
    #[test]
    fn a_region_straddling_ffff_uses_the_user_memory_service_for_all_of_it() {
        // Starts below FFFFh, ends above it.
        let base = 0xFFF0;
        let data = vec![0u8; 64];
        assert_eq!(
            service_for(base, data.len() as u32),
            MemoryService::UserMemory
        );
        let chunks = chunks(base, &data, limit_of(12)).unwrap();
        assert!(
            chunks
                .iter()
                .all(|c| c.service == MemoryService::UserMemory),
            "a region must not change service half way through"
        );
        // And the naive rule — switching on the base alone — would have
        // said otherwise, which is the bug this test exists for.
        assert_eq!(service_for(base, 1), MemoryService::Memory);
    }

    #[test]
    fn a_region_wholly_below_ffff_uses_the_memory_service() {
        assert_eq!(service_for(0x4000, 0x100), MemoryService::Memory);
        let chunks = chunks(0x4000, &[0u8; 0x100], limit_of(63)).unwrap();
        assert!(chunks.iter().all(|c| c.service == MemoryService::Memory));
    }

    #[test]
    fn a_region_wholly_above_ffff_uses_the_user_memory_service() {
        assert_eq!(service_for(0x1_0000, 0x10), MemoryService::UserMemory);
    }

    #[test]
    fn the_boundary_is_ffff_exclusive_as_cp_3_5_2_words_it() {
        // "lower than FFFFh" => Memory; anything reaching FFFFh => user.
        assert_eq!(service_for(0xFFFE, 0), MemoryService::Memory);
        assert_eq!(service_for(0xFFFE, 1), MemoryService::UserMemory);
    }

    /// §14 item 9, the domain half: `PID_REFERENCE` = 0 never becomes an
    /// address.
    #[test]
    fn a_base_of_zero_is_allocation_failure_and_not_an_address() {
        assert_eq!(
            chunks(0, &[1, 2, 3], limit_of(12)),
            Err(ChunkError::AllocationFailed)
        );
    }

    #[test]
    fn an_empty_region_is_refused_rather_than_producing_no_writes() {
        assert_eq!(
            chunks(0x100, &[], limit_of(12)),
            Err(ChunkError::EmptyRegion)
        );
    }

    #[test]
    fn a_region_leaving_the_addressable_space_is_refused() {
        assert_eq!(
            chunks(0xF_FFF0, &[0u8; 64], limit_of(12)),
            Err(ChunkError::RegionOutOfAddressSpace {
                base: 0xF_FFF0,
                length: 64
            })
        );
    }

    #[test]
    fn chunk_offsets_slice_the_data_back_together_exactly() {
        let data: Vec<u8> = (0..=200u8).collect();
        let chunks = chunks(0x2000, &data, limit_of(12)).unwrap();
        let mut rebuilt = Vec::new();
        for chunk in &chunks {
            rebuilt.extend_from_slice(&data[chunk.offset..chunk.offset + chunk.length as usize]);
        }
        assert_eq!(rebuilt, data);
    }

    #[test]
    fn a_region_shorter_than_one_chunk_is_a_single_write() {
        let chunks = chunks(0x2000, &[1, 2, 3], limit_of(12)).unwrap();
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].length, 3);
        assert_eq!(chunks[0].offset, 0);
    }
}
