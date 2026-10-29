//! Keeps property-description metadata in its lossless four-octet wire representation.

/// AL v02.01.01 §3.4.3.1: flags/type, maximum-elements pair, access pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PropertyDescription([u8; 4]);

impl PropertyDescription {
    pub const fn from_raw(raw: [u8; 4]) -> Self {
        Self(raw)
    }
    pub const fn raw(self) -> [u8; 4] {
        self.0
    }
    pub const fn write_enabled(self) -> bool {
        self.0[0] & 0x80 != 0
    }
    pub const fn legacy_bit(self) -> bool {
        self.0[0] & 0x40 != 0
    }
    pub const fn pdt(self) -> u8 {
        self.0[0] & 0x3f
    }
    pub const fn upper_max_elements(self) -> u8 {
        self.0[1] >> 4
    }
    pub const fn max_elements(self) -> u16 {
        u16::from_be_bytes([self.0[1] & 0x0f, self.0[2]])
    }
    pub const fn read_access(self) -> u8 {
        self.0[3] >> 4
    }
    pub const fn write_access(self) -> u8 {
        self.0[3] & 0x0f
    }
}
