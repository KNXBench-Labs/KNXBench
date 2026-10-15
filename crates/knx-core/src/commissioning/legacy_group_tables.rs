//! Legacy association tables reserve the sending entry at its group-object index.
//!
//! Resources v01.10.01 §§4.17.3.3.1–4.17.4.3.1, printed pp246–248:
//! sending uses association index = ASAP; Type2 additionally checks the stored
//! ASAP. FEh TSAP denotes an unused association. Receive-only associations may
//! follow the indexed slots. This explicit format codec does not select a mask,
//! place the table or admit hardware. Address words use the existing explicit
//! big-endian table convention; product-specific evidence stays separate.

use std::fmt;

use super::group_tables::{
    build_group_tables, GroupTableError, GroupTables, TableCapacity, TableLink,
};
use crate::IndividualAddress;

/// Why indexed legacy tables cannot represent the supplied links.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LegacyGroupTableError {
    /// Existing common validation: duplicates, broadcast, sending or capacities.
    Links(GroupTableError),
    /// A link names a descriptor not present in the source object table.
    ObjectOutOfTable { number: u8, count: u8 },
}
impl fmt::Display for LegacyGroupTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Links(error) => write!(f, "{error}"),
            Self::ObjectOutOfTable { number, count } => write!(
                f,
                "legacy group object {number} is outside {count} descriptors"
            ),
        }
    }
}
impl std::error::Error for LegacyGroupTableError {}
impl From<GroupTableError> for LegacyGroupTableError {
    fn from(error: GroupTableError) -> Self {
        Self::Links(error)
    }
}

/// Builds explicit Type1/Type2 byte grammar with one reserved sending slot per object.
///
/// Never picks a receive-only address as sending, never truncates high ASAPs,
/// and accounts for unused reserved rows in the association capacity. The caller
/// owns profile-specific placement, byte-order evidence and segment protection.
pub fn build(
    individual: IndividualAddress,
    object_count: u8,
    links: &[TableLink],
    capacity: TableCapacity,
) -> Result<GroupTables, LegacyGroupTableError> {
    for link in links {
        if link.object >= object_count {
            return Err(LegacyGroupTableError::ObjectOutOfTable {
                number: link.object,
                count: object_count,
            });
        }
    }
    // FEh cannot be a real TSAP in this grammar. Common validation preserves
    // duplicates, broadcast and multiple-sender errors instead of repairing them.
    let common = build_group_tables(
        individual,
        links,
        TableCapacity {
            group_addresses: capacity.group_addresses.min(253),
            associations: capacity.associations,
        },
    )?;
    let count = usize::from(object_count) + links.iter().filter(|link| !link.sending).count();
    let room = capacity.associations.min(usize::from(u8::MAX));
    if count > room {
        return Err(GroupTableError::TooManyAssociations {
            count,
            capacity: room,
        }
        .into());
    }
    let addresses: Vec<u16> = common.address_table[3..]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| u16::from_be_bytes([bytes[0], bytes[1]]))
        .collect();
    let tsap = |link: &TableLink| -> u8 {
        // Already checked by the common builder and limited to 253 positions.
        u8::try_from(
            addresses
                .binary_search(&link.group_address.raw())
                .expect("validated linked address")
                + 1,
        )
        .expect("bounded legacy TSAP")
    };
    let mut associations = Vec::with_capacity(1 + count * 2);
    associations.push(u8::try_from(count).expect("association count validated"));
    for number in 0..object_count {
        let sending = links
            .iter()
            .find(|link| link.object == number && link.sending);
        associations.extend_from_slice(&[sending.map_or(0xFE, tsap), number]);
    }
    let mut receivers: Vec<_> = links.iter().filter(|link| !link.sending).collect();
    receivers.sort_by_key(|link| (link.object, link.group_address));
    for link in receivers {
        associations.extend_from_slice(&[tsap(link), link.object]);
    }
    Ok(GroupTables {
        address_table: common.address_table,
        association_table: associations,
    })
}
