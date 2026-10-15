//! Three-octet legacy group-object descriptors, with source pointers preserved.
//!
//! Resources v01.10.01 §§4.18.3.1–4.18.4.1, printed pp261–266: the
//! Type-1/Type-2 table has a two-octet header and three-octet descriptors.
//! Type 1 requires config bit7=1; Type 2 uses it for Update Enable.
//! This codec selects an explicit format, not a device mask: assigning it
//! to a product (notably 0025h) needs separate source/profile evidence.

use std::collections::BTreeSet;
use std::fmt;

use super::group_object_table::{ActiveObject, GroupObjectTableError};

/// Exact descriptor grammar, independent of product and mask admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LegacyObjectFormat {
    /// Resources §4.18.3; config bit 7 is always one, not Update Enable.
    Type1,
    /// Resources §4.18.4; config bit 7 is Update Enable.
    Type2,
}

/// Why a source table cannot be rewritten safely.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyObjectError {
    /// A legacy table requires a two-octet header, not the Easy-3 header.
    NoHeader { octets: usize },
    /// Structural bounds or duplicate-object error shared with the existing codec.
    Table(GroupObjectTableError),
    /// Type 1 has no Update Enable field; do not silently drop an enabled flag.
    UpdateNotSupported {
        /// Active group-object number.
        number: u8,
    },
}

impl fmt::Display for LegacyObjectError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoHeader { octets } => write!(
                f,
                "legacy table has {octets} octets, shorter than its 2-octet header"
            ),
            Self::Table(error) => write!(f, "{error}"),
            Self::UpdateNotSupported { number } => {
                write!(f, "legacy Type-1 object {number} cannot enable Update")
            }
        }
    }
}
impl std::error::Error for LegacyObjectError {}
impl From<GroupObjectTableError> for LegacyObjectError {
    fn from(error: GroupObjectTableError) -> Self {
        Self::Table(error)
    }
}

/// Rewrites active descriptor flags/types, clearing only communication on inactive slots.
///
/// Preserves the entire source header, data pointers, segment selector bits,
/// inactive type bytes and trailing bytes. Does not grow, relocate or allocate
/// a table, interpret its pointers, or admit a download for any family.
pub fn rewrite(
    format: LegacyObjectFormat,
    base: &[u8],
    active: &[ActiveObject],
) -> Result<Vec<u8>, LegacyObjectError> {
    const HEADER: usize = 2;
    const DESCRIPTOR: usize = 3;
    if base.len() < HEADER {
        return Err(LegacyObjectError::NoHeader { octets: base.len() });
    }
    let size = base[0];
    let needed = HEADER + usize::from(size) * DESCRIPTOR;
    if base.len() < needed {
        return Err(GroupObjectTableError::Truncated {
            needed,
            octets: base.len(),
        }
        .into());
    }
    let mut seen = BTreeSet::new();
    for object in active {
        if object.number >= size {
            return Err(GroupObjectTableError::ObjectOutOfTable {
                number: object.number,
                size,
            }
            .into());
        }
        if !seen.insert(object.number) {
            return Err(GroupObjectTableError::DuplicateObject {
                number: object.number,
            }
            .into());
        }
        if format == LegacyObjectFormat::Type1 && object.flags.update {
            return Err(LegacyObjectError::UpdateNotSupported {
                number: object.number,
            });
        }
    }
    let mut table = base.to_vec();
    for number in 0..usize::from(size) {
        table[HEADER + number * DESCRIPTOR + 1] &= !0x04;
    }
    for object in active {
        let at = HEADER + usize::from(object.number) * DESCRIPTOR;
        let type_bit = if format == LegacyObjectFormat::Type1 {
            0x80
        } else {
            0
        };
        table[at + 1] = object.flags.config_bits() | (base[at + 1] & 0x20) | type_bit;
        table[at + 2] = object.value_type.code();
    }
    Ok(table)
}
