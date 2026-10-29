//! Chunking a memory region into writes: how long a chunk may be, and which service carries it.
//!
//! Pure arithmetic over design spec §6.4 (`PID_MAX_APDU_LENGTH`, and the 12-octet
//! default that is 15 − 3) and §6.5 (`A_Memory_Write` below `FFFFh`,
//! `A_UserMemoryWrite` above it, decided on **base + length**). No I/O:
//! the caller reads the property, this decides what to send.

use std::fmt;

/// The largest `number` `A_Memory_Read`/`A_Memory_Write` can carry, `[D]`
/// AL §3.5.3/§3.5.4: *"between 1 and 63 octets"*.
pub const SERVICE_MAX_OCTETS: u8 = 63;

/// What `A_UserMemory_Write` can carry in one PDU, which is a quarter of
/// what `A_Memory_Write` can.
///
/// `[D]` Application Layer v02.01.01 AS §3.5.6.3: the service writes
/// *"between 1 octet and 15 octets"*, and Figure 81 shows why — its
/// `number` field is four bits, sharing one octet with the four-bit address
/// extension that makes the address 20 bits wide. §6.4 of the design spec
/// derives the chunk size from `PID_MAX_APDU_LENGTH` alone and caps it at
/// 63, which is correct for `A_Memory_Write` and too large by four times
/// for the service §6.5 switches to above `FFFFh`. The spec does not say
/// this; the Application Layer does, and a 63-octet chunk simply does not
/// fit the PDU it would have to travel in.
pub const USER_MEMORY_MAX_OCTETS: u8 = 15;

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

/// The first address CP §3.5.2 routes to the user-memory service.
///
/// Not the first address `A_Memory_Write` cannot reach: `[D]` AL §3.5.4, the
/// clause that defines that very service, says *"The parameter memory_address
/// shall specify the 16 bit start address"*, so `FFFFh` is perfectly reachable
/// by it. The procedure draws the line here anyway. `[D]` CP §3.5.2: *"if BaseAddress
/// plus allocated memory is lower than FFFFh then MaC: MemoryWrite(…); if …
/// higher than FFFFh then MaC: UserMemoryWrite(…)"*.
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
///
/// The fields are private and every constructor in this module yields 12 to
/// 63 octets, so the `max_octets - 1` the user-memory budget takes cannot
/// underflow and no budget is ever zero.
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
    /// Lowered to the standard frame's 12 because the route to the device
    /// is unproved: a policy of this tool, not something the device said.
    CappedToStandardFrame,
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

    /// Data octets one PDU of `service` carries under this limit.
    ///
    /// `[D]` AL v02.01.01 §3.5.6.2/§3.5.6.3: the `A_UserMemory_*` header is
    /// three octets against `A_Memory_*`'s two (the `number` is not inline),
    /// so the same APDU length leaves one data octet less, capped at the
    /// service's own 15. Ordinary memory keeps [`WriteLimit::max_octets`].
    pub fn max_octets_for(self, service: MemoryService) -> u8 {
        match service {
            MemoryService::Memory => self.max_octets,
            MemoryService::UserMemory => (self.max_octets - 1).min(USER_MEMORY_MAX_OCTETS),
        }
    }

    /// This limit held to the standard frame's 12 data octets.
    ///
    /// `[A]` conservative policy for a route whose frame capacity is not
    /// proved, not a measured capacity and not a mandate against extended
    /// frames. Lowers a larger limit and says so in the reason; a limit
    /// already at 12 is returned as it is, reason included.
    pub fn capped_to_standard_frame(self) -> WriteLimit {
        if self.max_octets > STANDARD_FRAME_OCTETS {
            WriteLimit {
                max_octets: STANDARD_FRAME_OCTETS,
                reason: WriteLimitReason::CappedToStandardFrame,
            }
        } else {
            self
        }
    }
}

/// `DEFAULT_APDU_LENGTH` less the three overhead octets: 12 data octets.
const STANDARD_FRAME_OCTETS: u8 = (DEFAULT_APDU_LENGTH - APDU_OVERHEAD_OCTETS) as u8;

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

/// The service for one whole allocation, decided once from its base plus its *allocated*
/// size, so its writes, prestate reads and verification reads all use the same one.
///
/// `[D]` CP v02.01.01 §3.5.2 (printed p43, repeated in §3.5.3): *lower* than `FFFFh` is
/// `A_Memory_*`, *higher* is `A_UserMemory_*`; exactly `FFFFh` is not defined there, so
/// `None` is returned for it instead of either guess. [`service_for`] keeps its historical
/// answer for that case for read-only paths that are not allocations.
pub fn allocation_service(base: u32, allocated: u32) -> Option<MemoryService> {
    match base.saturating_add(allocated) {
        MEMORY_SERVICE_LIMIT => None,
        _ => Some(service_for(base, allocated)),
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
    /// [`WriteLimit`], never above [`SERVICE_MAX_OCTETS`] — and never above
    /// [`USER_MEMORY_MAX_OCTETS`] when `service` is
    /// [`MemoryService::UserMemory`].
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
/// Refuses a zero base outright: design spec §7.2 step 3 and §9.2 both say
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
    // The negotiated limit is an upper bound, not the only one: the chosen
    // service has a header and a hard limit of its own (see
    // `WriteLimit::max_octets_for`). Using its budget is the difference
    // between a chunk plan that can be sent and one that cannot.
    let step = limit.max_octets_for(service) as usize;
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

    /// Only what a production constructor can yield: 12 to 63 octets.
    fn limit_of(n: u8) -> WriteLimit {
        assert!((12..=63).contains(&n), "not a production limit: {n}");
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
        // A 12-octet negotiated limit leaves the user-memory service 11.
        assert!(chunks.iter().all(|c| c.length <= 11));
        // And the naive rule — switching on the base alone — would have
        // said otherwise, which is the bug this test exists for.
        assert_eq!(service_for(base, 1), MemoryService::Memory);
    }

    /// `A_UserMemory_Write`'s `number` field is four bits (AL §3.5.6.3,
    /// Figure 81), so a negotiated limit of 63 octets — which §6.4's
    /// arithmetic happily produces — cannot be used above `FFFFh`. The
    /// design spec does not mention this cap; the Application Layer does.
    #[test]
    fn the_user_memory_service_caps_chunks_at_fifteen_octets_whatever_the_apdu_allows() {
        let above = chunks(0x8_0000, &[0u8; 200], limit_of(63)).unwrap();
        assert!(
            above.iter().all(|c| c.service == MemoryService::UserMemory),
            "a region entirely above FFFFh uses the user-memory service"
        );
        assert_eq!(
            above.iter().map(|c| c.length).max(),
            Some(USER_MEMORY_MAX_OCTETS),
            "no chunk may exceed what the service's 4-bit number field can name"
        );
        // And the same data below FFFFh still uses the full 63, so the cap
        // is scoped to the service and is not a new global ceiling.
        let below = chunks(0x1000, &[0u8; 200], limit_of(63)).unwrap();
        assert_eq!(below.iter().map(|c| c.length).max(), Some(63));
    }

    /// The cap narrows, never widens: a device that negotiated 12 octets
    /// gets 11-octet chunks above `FFFFh` (one header octet more), not 15.
    #[test]
    fn the_user_memory_cap_never_raises_a_smaller_negotiated_limit() {
        let above = chunks(0x8_0000, &[0u8; 100], limit_of(12)).unwrap();
        assert_eq!(above.iter().map(|c| c.length).max(), Some(11));
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

    /// CP §3.5.2 defines "lower" and "higher" than `FFFFh` only; an allocation whose base
    /// plus allocated size equals it has no service, on both sides of which the rule holds.
    #[test]
    fn an_allocation_ending_exactly_at_ffff_has_no_defined_service() {
        assert_eq!(
            allocation_service(0xFF00, 0xFE),
            Some(MemoryService::Memory)
        );
        assert_eq!(allocation_service(0xFF00, 0xFF), None);
        assert_eq!(allocation_service(0xFFFE, 1), None);
        assert_eq!(
            allocation_service(0xFF00, 0x100),
            Some(MemoryService::UserMemory)
        );
        // The decision quantity is the allocation, not a write inside it.
        assert_eq!(service_for(0xFF00, 0x10), MemoryService::Memory);
        assert_eq!(
            allocation_service(0xFF00, 0x100),
            Some(service_for(0xFF00, 0x100))
        );
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

    // Service budgets: UserMemory's header is one octet longer than Memory's.

    fn sources() -> Vec<ApduLengthSource> {
        let mut all = vec![ApduLengthSource::Absent];
        for v in 0..=u16::MAX {
            all.push(ApduLengthSource::DeviceObject(v));
            all.push(ApduLengthSource::RouterObjectOnly(v));
        }
        all
    }

    fn plan(base: u32, len: usize, apdu: u16) -> Vec<(u32, u8, MemoryService)> {
        let data = vec![0x5A; len];
        chunks(
            base,
            &data,
            write_limit(ApduLengthSource::DeviceObject(apdu)),
        )
        .expect("chunks")
        .into_iter()
        .map(|c| (c.address, c.length, c.service))
        .collect()
    }

    #[test]
    fn a_sixteen_octet_boundary_region_at_the_standard_frame_is_eleven_plus_five() {
        assert_eq!(
            plan(0xFFF0, 16, 15),
            vec![
                (0xFFF0, 11, MemoryService::UserMemory),
                (0xFFFB, 5, MemoryService::UserMemory)
            ]
        );
    }

    #[test]
    fn the_user_memory_budget_follows_the_apdu_length_by_the_extra_header_octet() {
        let user = |apdu| {
            write_limit(ApduLengthSource::DeviceObject(apdu))
                .max_octets_for(MemoryService::UserMemory)
        };
        assert_eq!(user(15), 11);
        assert_eq!(user(16), 12);
        assert_eq!(user(18), 14);
        assert_eq!(user(19), 15);
        assert_eq!(user(20), 15);
        assert_eq!(user(254), 15);
    }

    #[test]
    fn every_reported_value_yields_a_nonzero_budget_without_underflow() {
        for source in sources() {
            let limit = write_limit(source);
            let m = limit.max_octets();
            assert!((12..=63).contains(&m), "{source:?}: {m}");
            assert_eq!(limit.max_octets_for(MemoryService::Memory), m);
            let user = limit.max_octets_for(MemoryService::UserMemory);
            assert_eq!(user, (m - 1).min(15), "{source:?}");
            assert!(user >= 11);
            let capped = limit.capped_to_standard_frame();
            assert_eq!(capped.max_octets_for(MemoryService::UserMemory), 11);
            assert_eq!(capped.max_octets_for(MemoryService::Memory), 12);
        }
    }

    #[test]
    fn the_standard_frame_cap_names_itself_only_when_it_lowers_something() {
        for source in sources() {
            let limit = write_limit(source);
            let capped = limit.capped_to_standard_frame();
            if limit.max_octets() > 12 {
                assert_eq!(capped.max_octets(), 12, "{source:?}");
                assert_eq!(capped.reason(), WriteLimitReason::CappedToStandardFrame);
            } else {
                assert_eq!(capped, limit, "{source:?}: value and reason stay");
            }
            assert_eq!(capped.capped_to_standard_frame(), capped, "idempotent");
        }
        // The twelve that was already 12 keeps the reason that explains it.
        for (source, reason) in [
            (ApduLengthSource::Absent, WriteLimitReason::PropertyAbsent),
            (
                ApduLengthSource::RouterObjectOnly(254),
                WriteLimitReason::RouterObjectOnly,
            ),
            (
                ApduLengthSource::DeviceObject(255),
                WriteLimitReason::EscapeCode,
            ),
            (
                ApduLengthSource::DeviceObject(3),
                WriteLimitReason::BelowStandardRange,
            ),
            (
                ApduLengthSource::DeviceObject(15),
                WriteLimitReason::FromDeviceObject,
            ),
        ] {
            assert_eq!(
                write_limit(source).capped_to_standard_frame().reason(),
                reason
            );
        }
        assert_eq!(
            write_limit(ApduLengthSource::DeviceObject(254)).reason(),
            WriteLimitReason::CappedByServiceLimit,
            "write_limit itself is not changed"
        );
    }

    #[test]
    fn chunk_plans_tile_the_region_exactly_for_every_source_and_both_services() {
        let patterns = [
            (0x4000u32, 200usize),
            (0xFFF0, 90),
            (0xFF80, 0x7F),
            (0x1_0000, 70),
        ];
        for apdu in 0..=300u16 {
            let limit = write_limit(ApduLengthSource::DeviceObject(apdu));
            for (base, len) in patterns {
                let data: Vec<u8> = (0..len).map(|i| (i % 251) as u8 | 1).collect();
                let plan = chunks(base, &data, limit).expect("chunks");
                let service = service_for(base, len as u32);
                let mut expected = 0usize;
                for c in &plan {
                    assert_eq!(c.service, service, "one service for the whole region");
                    assert!(c.length >= 1 && c.length <= limit.max_octets_for(service));
                    assert_eq!(c.offset, expected);
                    assert_eq!(c.address, base + expected as u32);
                    expected += c.length as usize;
                }
                assert_eq!(expected, len, "no gap, no duplicate");
            }
        }
    }

    #[test]
    fn a_region_crossing_ffff_is_held_in_one_service_at_the_user_memory_budget() {
        assert_eq!(
            plan(0xFFF0, 16, 254),
            vec![
                (0xFFF0, 15, MemoryService::UserMemory),
                (0xFFFF, 1, MemoryService::UserMemory)
            ]
        );
        assert_eq!(
            plan(0xFFF0, 16, 18),
            vec![
                (0xFFF0, 14, MemoryService::UserMemory),
                (0xFFFE, 2, MemoryService::UserMemory)
            ]
        );
    }

    #[test]
    fn ordinary_memory_keeps_its_data_budget() {
        let low: Vec<_> = plan(0x4000, 30, 15).iter().map(|c| c.1).collect();
        assert_eq!(low, vec![12, 12, 6]);
        let wide: Vec<_> = plan(0x4000, 130, 254).iter().map(|c| c.1).collect();
        assert_eq!(wide, vec![63, 63, 4]);
    }
}
