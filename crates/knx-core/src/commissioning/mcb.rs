//! A typed Memory Control Block: RES §4.2.27, Table 12, p. 39, not "unspecified".

use std::fmt;

/// How many octets one Memory Control Block element occupies.
///
/// RES §4.2.27, Table 12, p. 39: Segment Size 1 (4 octets) + CRC Control
/// Byte (1 octet) + Read Access 1 / Write Access 1 (1 octet, as two 4-bit
/// nibbles) + CRC (2 octets) = 8. `PID_MCB_TABLE`'s datatype is
/// `PDT_GENERIC_08[]`, an array of these; this crate allocates and loads
/// one segment per part, so exactly one element is what a load ever
/// produces or compares.
pub const MEMORY_CONTROL_BLOCK_OCTETS: usize = 8;

/// One element of `PID_MCB_TABLE`, RES §4.2.27, Table 12, p. 39.
///
/// `[D]` The table's own PDF rendering prints a merged header cell reading
/// "Nr. of elements" above the whole row; it is the array-level caption
/// ("this property holds *N* elements shaped like this"), not a sixth
/// field, and does not appear here. Verified against the ruled table in
/// the source PDF, not just the extracted text, because the extracted
/// text alone reads as if "Nr. of elements" were a field sharing the
/// access-nibble octet.
///
/// The Standard does not name which nibble is high and which is low; this
/// reads Read Access 1 as the high nibble, matching the table's left-to-
/// right column order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryControlBlock {
    /// Segment Size 1, octets 0-3, big-endian (RES's octets are always
    /// network byte order elsewhere in this crate; Table 12 states no
    /// exception).
    pub segment_size: u32,
    /// CRC Control Byte, octet 4. RES §4.2.27.1.1, Table 13, p. 39.
    pub crc_control_byte: u8,
    /// Read Access 1, the high nibble of octet 5.
    pub read_access: u8,
    /// Write Access 1, the low nibble of octet 5.
    pub write_access: u8,
    /// CRC, octets 6-7, big-endian. RES §4.2.27.1.2, p. 39: a CRC16-CCITT
    /// (width 16, polynomial `1021h`, initial value `FFFFh`, input and
    /// output not reflected, no output XOR), valid only in the load state
    /// 'Loaded'.
    pub crc: u16,
}

impl MemoryControlBlock {
    /// RES §4.2.27.1.1, Table 13, p. 39, bit 0: *"0 : CRC is always valid /
    /// 1 : Contents of protected memory area may change"*. When set, the
    /// device is saying the protected memory may have changed since the
    /// load, so a matching CRC proves nothing.
    pub fn protected_memory_may_change(self) -> bool {
        self.crc_control_byte & 0b0000_0001 != 0
    }

    /// Parses one element, or reports how many octets arrived instead of
    /// [`MEMORY_CONTROL_BLOCK_OCTETS`].
    pub fn parse(octets: &[u8]) -> Result<Self, McbLengthError> {
        let fixed: [u8; MEMORY_CONTROL_BLOCK_OCTETS] = octets
            .try_into()
            .map_err(|_| McbLengthError { got: octets.len() })?;
        let [s0, s1, s2, s3, crc_control_byte, access, c0, c1] = fixed;
        Ok(Self {
            segment_size: u32::from_be_bytes([s0, s1, s2, s3]),
            crc_control_byte,
            read_access: access >> 4,
            write_access: access & 0x0F,
            crc: u16::from_be_bytes([c0, c1]),
        })
    }

    /// The inverse of [`MemoryControlBlock::parse`], for round-trip tests
    /// and for a caller that builds a block rather than reading one.
    pub fn to_octets(self) -> [u8; MEMORY_CONTROL_BLOCK_OCTETS] {
        let [s0, s1, s2, s3] = self.segment_size.to_be_bytes();
        let [c0, c1] = self.crc.to_be_bytes();
        let access = (self.read_access << 4) | (self.write_access & 0x0F);
        [s0, s1, s2, s3, self.crc_control_byte, access, c0, c1]
    }
}

/// [`MemoryControlBlock::parse`] saw something other than
/// [`MEMORY_CONTROL_BLOCK_OCTETS`] octets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct McbLengthError {
    /// How many octets arrived.
    pub got: usize,
}

impl fmt::Display for McbLengthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "a Memory Control Block element is {MEMORY_CONTROL_BLOCK_OCTETS} octets (RES §4.2.27, Table 12, p. 39), got {}",
            self.got
        )
    }
}

impl std::error::Error for McbLengthError {}

#[cfg(test)]
mod tests {
    use super::*;

    /// RES §4.2.27, Table 12, p. 39, applied to made-up but structurally
    /// valid octets: parsing and re-serialising must be the identity.
    #[test]
    fn round_trips_table_12s_layout() {
        let octets: [u8; 8] = [0x00, 0x00, 0x10, 0x00, 0xAB, 0xCD, 0x12, 0x34];
        let mcb = MemoryControlBlock::parse(&octets).expect("valid element");
        assert_eq!(mcb.segment_size, 0x0000_1000);
        assert_eq!(mcb.crc_control_byte, 0xAB);
        assert_eq!(mcb.read_access, 0xC);
        assert_eq!(mcb.write_access, 0xD);
        assert_eq!(mcb.crc, 0x1234);
        assert_eq!(mcb.to_octets(), octets);
    }

    #[test]
    fn crc_control_byte_bit_0_names_whether_memory_may_change() {
        let never_changes =
            MemoryControlBlock::parse(&[0, 0, 0, 0, 0b0000_0000, 0, 0, 0]).expect("valid element");
        assert!(!never_changes.protected_memory_may_change());

        let may_change =
            MemoryControlBlock::parse(&[0, 0, 0, 0, 0b0000_0001, 0, 0, 0]).expect("valid element");
        assert!(may_change.protected_memory_may_change());

        // The other seven bits are reserved (Table 13) and must not leak
        // into the verdict.
        let reserved_bits_set =
            MemoryControlBlock::parse(&[0, 0, 0, 0, 0b1111_1110, 0, 0, 0]).expect("valid element");
        assert!(!reserved_bits_set.protected_memory_may_change());
    }

    #[test]
    fn wrong_length_is_reported_with_the_count() {
        let err = MemoryControlBlock::parse(&[0xDE, 0xAD]).unwrap_err();
        assert_eq!(err.got, 2);
        assert!(err.to_string().contains('2'));
    }
}
