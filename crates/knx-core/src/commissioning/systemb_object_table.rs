//! Pure System-B Format-1 group-object rewriting without product or download admission.
//!
//! Resources 03_05_01 v01.10.01 §4.18.6.2.4.1, Tables 86–88,
//! printed/PDF pp. 271–272: a two-octet length followed by two-octet
//! descriptors, with ASAPs 1 through length. The descriptor's upper octet
//! holds update, transmit, initial read, write, read, communication and
//! priority; its lower octet is the value-field type. These are not Easy-3
//! descriptors and have no source data pointer or segment-selector field.
//!
//! The caller must select this exact grammar using separate product evidence.
//! This codec neither selects a mask nor builds an executable download plan.

use std::collections::BTreeSet;
use std::fmt;

use super::group_object_table::{ObjectFlags, ValueType as LegacyValueType};

/// Table 88's value-field code; every octet is defined, including 255 (252 octets).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ValueType(u8);

impl ValueType {
    /// An explicit Table 88 code, not a DPT identifier or a bit length.
    pub const fn from_code(code: u8) -> Self {
        Self(code)
    }

    /// The exact type for a supported bit length; never rounds or clamps.
    pub fn for_bits(bits: u32) -> Option<Self> {
        if let Some(value) = LegacyValueType::for_bits(bits) {
            return Some(Self(value.code()));
        }
        let code = match bits {
            40 => 15,
            56 => 16,
            72 => 17,
            88 => 18,
            96 => 19,
            104 => 20,
            120..=1984 if bits.is_multiple_of(8) => {
                return u8::try_from(bits / 8 + 6).ok().map(Self);
            }
            2016 => 255,
            _ => return None,
        };
        Some(Self(code))
    }

    /// The lower descriptor octet.
    pub const fn code(self) -> u8 {
        self.0
    }
}

/// An explicitly active System-B object; numbers start at one, not zero.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ActiveObject {
    /// One-based ASAP, bounded by the source table's two-octet length.
    pub number: u16,
    /// Shared communication flags and transmission priority.
    pub flags: ObjectFlags,
    /// Descriptor bit 13; this is not the legacy segment-selector bit.
    pub read_on_initialisation: bool,
    /// Table 88 value-field type.
    pub value_type: ValueType,
}

/// Why the entire rewrite was refused; no object is silently discarded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// The two-octet length is absent.
    NoHeader {
        /// Available source octets.
        octets: usize,
    },
    /// The source length declares descriptors that are not present.
    Truncated {
        /// Required source octets, including the length.
        needed: usize,
        /// Available source octets.
        octets: usize,
    },
    /// Zero is not an ASAP, or the number exceeds the source table length.
    ObjectOutOfTable {
        /// Refused active object's number.
        number: u16,
        /// Number of source descriptors.
        size: u16,
    },
    /// The active input names one object twice.
    DuplicateObject {
        /// Repeated one-based object number.
        number: u16,
    },
}

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHeader { octets } => write!(
                f,
                "System-B table has {octets} octets, shorter than its 2-octet length"
            ),
            Self::Truncated { needed, octets } => write!(
                f,
                "System-B table needs {needed} octets for its descriptors, but has {octets}"
            ),
            Self::ObjectOutOfTable { number, size } => write!(
                f,
                "System-B object {number} is outside the source table's 1..={size} ASAPs"
            ),
            Self::DuplicateObject { number } => {
                write!(f, "System-B object {number} is listed twice")
            }
        }
    }
}

impl std::error::Error for TableError {}

/// Rewrites explicit active descriptors in a big-endian Format-1 source table.
///
/// Preserves the source length and all trailing octets, even an odd trailing
/// length. Inactive descriptors retain every bit except communication enable
/// (bit 10), which is cleared. Active descriptors are replaced completely with
/// the explicitly supplied flags/type. Does not grow, allocate, relocate or
/// select a source table, evaluate product activation, or change the input.
pub fn rewrite(base: &[u8], active: &[ActiveObject]) -> Result<Vec<u8>, TableError> {
    if base.len() < 2 {
        return Err(TableError::NoHeader { octets: base.len() });
    }
    let size = u16::from_be_bytes([base[0], base[1]]);
    let needed = 2 + usize::from(size) * 2;
    if base.len() < needed {
        return Err(TableError::Truncated {
            needed,
            octets: base.len(),
        });
    }
    let mut seen = BTreeSet::new();
    for object in active {
        if object.number == 0 || object.number > size {
            return Err(TableError::ObjectOutOfTable {
                number: object.number,
                size,
            });
        }
        if !seen.insert(object.number) {
            return Err(TableError::DuplicateObject {
                number: object.number,
            });
        }
    }
    let mut result = base.to_vec();
    for descriptor in result[2..needed].as_chunks_mut::<2>().0 {
        descriptor[0] &= !0x04;
    }
    for object in active {
        // Header + (ASAP - 1) * two octets = ASAP * two octets.
        let offset = usize::from(object.number) * 2;
        result[offset] = object.flags.config_bits()
            | if object.read_on_initialisation {
                0x20
            } else {
                0
            };
        result[offset + 1] = object.value_type.code();
    }
    Ok(result)
}
