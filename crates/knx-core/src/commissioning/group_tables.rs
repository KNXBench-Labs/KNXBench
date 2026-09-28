//! The two tables that tie a mask-`0701h` device's group objects to group addresses.
//!
//! `[D]` Resources §4.16.11 (*GrAT – Easy 2*, "used by … mask 0701h in
//! E-Mode") takes its format from §4.16.3.1, and §4.17.9 (*GrOAT – Easy 3*,
//! "mask 0701h in E-Mode") takes its format from §4.17.3.1:
//!
//! ```text
//! Group Address Table            Group Object Association Table
//!   1 octet   Length               1 octet   Current Size
//!   2 octets  Individual Address   2 octets  TSAP | ASAP   (association 0)
//!   2 octets  Group Address Nr. 1  2 octets  TSAP | ASAP   (association 1)
//!   …                              …
//! ```
//!
//! *Length* counts the individual address too (§4.16.3.3.1's EXAMPLE: five
//! group addresses give length 6). The group addresses are sorted ascending
//! (§4.16.3.1), and a TSAP is the 1-based position of a group address in that
//! table (§4.16.3.3.2). An ASAP is a group object number.
//!
//! `[A]` Each two-octet address is stored high octet first. No clause of
//! Resources states the byte order for these tables, so this is an
//! assumption, resting on three indirect sources:
//!
//! - `[D]` Resources §4.16.3.4.2 writes the individual address as
//!   `DMP_MemWrite_LEmi1(0117h, 0118h, PPPPh)`: one 16-bit value across the
//!   two ascending addresses, in the notation that writes high first.
//! - `[D]` API §1.2.1 (`03_06_01`): the BCU's two-octet EEPROM pointers
//!   (`CommsTabPtr2` and its siblings) are *"Big Endian"*. That is the same
//!   memory family, but it does not name these tables.
//! - `[V]` The product data agrees without deciding it. MDT's
//!   `A-0027-15-0BAC` ships `AS-4000` as `03 0000 1900 1901`. High first
//!   that is `3/1/0` and `3/1/1`; low first it is `0/0/25` and `0/1/25`,
//!   equally valid and equally sorted.
//!
//! One read of a real device's table, with a known group address, would
//! settle it. Until then a download built on this must not be claimed
//! verified.
//!
//! These functions build the table octets only. Where they go, and what
//! fills the rest of their segment, is the segment image's business.

use std::collections::BTreeSet;
use std::fmt;

use crate::{GroupAddress, IndividualAddress};

/// One link to put into the tables: group object `object` uses
/// `group_address`, and `sending` says whether it is the address the object
/// transmits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TableLink {
    /// The group object number, which is the ASAP.
    pub object: u8,
    /// The group address.
    pub group_address: GroupAddress,
    /// Whether this is the object's sending address.
    pub sending: bool,
}

/// How many entries the device has room for, from the product data
/// (`AddressTable/@MaxEntries`, `AssociationTable/@MaxEntries`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableCapacity {
    /// Group addresses, not counting the individual address.
    pub group_addresses: usize,
    /// Associations.
    pub associations: usize,
}

/// The octets of both tables, ready to be placed at their table addresses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupTables {
    /// The Group Address Table, from its Length octet.
    pub address_table: Vec<u8>,
    /// The Group Object Association Table, from its Current Size octet.
    pub association_table: Vec<u8>,
}

/// Why the tables could not be built. Nothing is truncated or dropped to
/// make a table fit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupTableError {
    /// Group address `0/0/0` is the broadcast destination, not a group
    /// address a table entry can hold.
    BroadcastAddress {
        /// The object linked to it.
        object: u8,
    },
    /// The same object is linked to the same group address twice.
    DuplicateLink {
        /// The object.
        object: u8,
        /// The group address.
        group_address: GroupAddress,
    },
    /// An object has more than one sending address. A group object
    /// transmits on exactly one (§4.17.9.4.1's *"sending association"*).
    SeveralSendingAddresses {
        /// The object.
        object: u8,
    },
    /// More group addresses than the device has room for, or than the one
    /// Length octet can count.
    TooManyGroupAddresses {
        /// How many there are.
        count: usize,
        /// How many fit.
        capacity: usize,
    },
    /// More associations than the device has room for, or than the one
    /// Current Size octet can count.
    TooManyAssociations {
        /// How many there are.
        count: usize,
        /// How many fit.
        capacity: usize,
    },
}

impl std::error::Error for GroupTableError {}

impl fmt::Display for GroupTableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GroupTableError::BroadcastAddress { object } => write!(
                f,
                "object {object} is linked to 0/0/0, the broadcast destination, which no \
                 group address table entry can hold"
            ),
            GroupTableError::DuplicateLink {
                object,
                group_address,
            } => write!(
                f,
                "object {object} is linked to group address {:04X}h twice",
                group_address.raw()
            ),
            GroupTableError::SeveralSendingAddresses { object } => {
                write!(f, "object {object} has more than one sending group address")
            }
            GroupTableError::TooManyGroupAddresses { count, capacity } => write!(
                f,
                "{count} group addresses, but the group address table holds {capacity}"
            ),
            GroupTableError::TooManyAssociations { count, capacity } => write!(
                f,
                "{count} associations, but the association table holds {capacity}"
            ),
        }
    }
}

/// Builds the Group Address Table and the Group Object Association Table
/// for `individual` and `links`.
pub fn build_group_tables(
    individual: IndividualAddress,
    links: &[TableLink],
    capacity: TableCapacity,
) -> Result<GroupTables, GroupTableError> {
    let mut seen = BTreeSet::new();
    let mut senders = BTreeSet::new();
    for link in links {
        if link.group_address.raw() == 0 {
            return Err(GroupTableError::BroadcastAddress {
                object: link.object,
            });
        }
        if !seen.insert((link.object, link.group_address)) {
            return Err(GroupTableError::DuplicateLink {
                object: link.object,
                group_address: link.group_address,
            });
        }
        if link.sending && !senders.insert(link.object) {
            return Err(GroupTableError::SeveralSendingAddresses {
                object: link.object,
            });
        }
    }

    // Ascending, one entry per address (§4.16.3.1).
    let addresses: Vec<GroupAddress> = links
        .iter()
        .map(|link| link.group_address)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let address_room = capacity.group_addresses.min(MAX_GROUP_ADDRESSES);
    if addresses.len() > address_room {
        return Err(GroupTableError::TooManyGroupAddresses {
            count: addresses.len(),
            capacity: address_room,
        });
    }
    let association_room = capacity.associations.min(MAX_ASSOCIATIONS);
    if links.len() > association_room {
        return Err(GroupTableError::TooManyAssociations {
            count: links.len(),
            capacity: association_room,
        });
    }

    let length = u8::try_from(addresses.len() + 1).expect("bounded by MAX_GROUP_ADDRESSES above");
    let mut address_table = Vec::with_capacity(1 + 2 * (addresses.len() + 1));
    address_table.push(length);
    address_table.extend_from_slice(&individual.raw().to_be_bytes());
    for address in &addresses {
        address_table.extend_from_slice(&address.raw().to_be_bytes());
    }

    // TSAP: 1-based position in the sorted table (§4.16.3.3.2).
    let tsap = |address: GroupAddress| -> u8 {
        let position = addresses
            .binary_search(&address)
            .expect("every linked address is in the table");
        u8::try_from(position + 1).expect("bounded by MAX_GROUP_ADDRESSES above")
    };
    let mut associations: Vec<(u8, bool, u8)> = links
        .iter()
        .map(|link| (link.object, !link.sending, tsap(link.group_address)))
        .collect();
    // By object, and within an object the sending association first
    // (§4.17.9.5), then by TSAP so the order is deterministic.
    associations.sort_unstable();
    let size = u8::try_from(associations.len()).expect("bounded by MAX_ASSOCIATIONS above");
    let mut association_table = Vec::with_capacity(1 + 2 * associations.len());
    association_table.push(size);
    for (object, _, tsap) in associations {
        association_table.push(tsap);
        association_table.push(object);
    }

    Ok(GroupTables {
        address_table,
        association_table,
    })
}

/// One Length octet counts the individual address and the group addresses,
/// so it leaves room for 254 group addresses.
const MAX_GROUP_ADDRESSES: usize = u8::MAX as usize - 1;

/// One Current Size octet counts the associations.
const MAX_ASSOCIATIONS: usize = u8::MAX as usize;

#[cfg(test)]
mod tests {
    use super::*;

    const ROOMY: TableCapacity = TableCapacity {
        group_addresses: 255,
        associations: 255,
    };

    fn ga(main: u16, middle: u16, sub: u16) -> GroupAddress {
        GroupAddress::from_raw((main << 11) | (middle << 8) | sub)
    }

    fn link(object: u8, group_address: GroupAddress, sending: bool) -> TableLink {
        TableLink {
            object,
            group_address,
            sending,
        }
    }

    /// The product data's own defaults, rebuilt: `AS-4000` starts
    /// `03 0000 1900 1901` and `AS-4201` starts `02 0100 0205` in
    /// `M-0083_A-0027-15-0BAC.xml` (MDT_KP_BE_01_Push_Button_V15a).
    #[test]
    fn the_mdt_defaults_come_out_octet_for_octet() {
        let tables = build_group_tables(
            IndividualAddress::from_raw(0x0000),
            &[link(0, ga(3, 1, 0), true), link(5, ga(3, 1, 1), true)],
            ROOMY,
        )
        .unwrap();
        assert_eq!(
            tables.address_table,
            vec![0x03, 0x00, 0x00, 0x19, 0x00, 0x19, 0x01]
        );
        assert_eq!(tables.association_table, vec![0x02, 0x01, 0x00, 0x02, 0x05]);
    }

    /// The request that started this: button 1 (object 0) toggles `2/0/53`
    /// on the device at `1.1.67`.
    #[test]
    fn one_toggle_on_2_0_53_at_1_1_67() {
        let tables = build_group_tables(
            IndividualAddress::new(1, 1, 67).unwrap(),
            &[link(0, ga(2, 0, 53), true)],
            ROOMY,
        )
        .unwrap();
        // 1.1.67 = 1143h; 2/0/53 = (2 << 11) | 53 = 1035h.
        assert_eq!(tables.address_table, vec![0x02, 0x11, 0x43, 0x10, 0x35]);
        assert_eq!(tables.association_table, vec![0x01, 0x01, 0x00]);
    }

    /// §4.16.3.1: ascending, whatever order the links came in. §4.16.3.3.2:
    /// a TSAP is the position in that sorted table.
    #[test]
    fn group_addresses_are_sorted_and_tsaps_follow_the_sort() {
        let tables = build_group_tables(
            IndividualAddress::from_raw(0x1143),
            &[
                link(3, ga(5, 0, 1), false),
                link(1, ga(1, 0, 9), true),
                link(2, ga(1, 0, 2), true),
            ],
            ROOMY,
        )
        .unwrap();
        assert_eq!(
            tables.address_table,
            vec![0x04, 0x11, 0x43, 0x08, 0x02, 0x08, 0x09, 0x28, 0x01]
        );
        // 1/0/2 is TSAP 1, 1/0/9 TSAP 2, 5/0/1 TSAP 3.
        assert_eq!(
            tables.association_table,
            vec![0x03, 0x02, 0x01, 0x01, 0x02, 0x03, 0x03]
        );
    }

    /// One group address used by several objects is one table entry, and
    /// one TSAP in several associations.
    #[test]
    fn a_shared_group_address_is_one_entry() {
        let tables = build_group_tables(
            IndividualAddress::from_raw(0x1143),
            &[link(0, ga(2, 0, 53), true), link(16, ga(2, 0, 53), false)],
            ROOMY,
        )
        .unwrap();
        assert_eq!(tables.address_table, vec![0x02, 0x11, 0x43, 0x10, 0x35]);
        assert_eq!(tables.association_table, vec![0x02, 0x01, 0x00, 0x01, 0x10]);
    }

    /// §4.17.9.4.1 sends on the first association matching the ASAP, and
    /// §4.17.9.5 requires *"the sending connection number shall be before
    /// all other connection numbers for a certain SAP"* — even when the
    /// sending address sorts last in the address table.
    #[test]
    fn the_sending_association_comes_first_for_its_object() {
        let tables = build_group_tables(
            IndividualAddress::from_raw(0x1143),
            &[
                link(0, ga(1, 0, 1), false),
                link(0, ga(9, 0, 9), true),
                link(0, ga(4, 0, 4), false),
            ],
            ROOMY,
        )
        .unwrap();
        // TSAPs: 1/0/1 = 1, 4/0/4 = 2, 9/0/9 = 3.
        assert_eq!(
            tables.association_table,
            vec![0x03, 0x03, 0x00, 0x01, 0x00, 0x02, 0x00]
        );
    }

    #[test]
    fn no_links_is_an_empty_but_valid_pair_of_tables() {
        let tables = build_group_tables(IndividualAddress::from_raw(0x1143), &[], ROOMY).unwrap();
        // §4.16.3.3.1: Length 1 is "no group frame shall be passed".
        assert_eq!(tables.address_table, vec![0x01, 0x11, 0x43]);
        assert_eq!(tables.association_table, vec![0x00]);
    }

    #[test]
    fn the_broadcast_address_is_refused() {
        let error = build_group_tables(
            IndividualAddress::from_raw(0x1143),
            &[link(4, GroupAddress::from_raw(0), true)],
            ROOMY,
        )
        .unwrap_err();
        assert_eq!(error, GroupTableError::BroadcastAddress { object: 4 });
    }

    #[test]
    fn a_duplicate_link_is_refused_not_merged() {
        let error = build_group_tables(
            IndividualAddress::from_raw(0x1143),
            &[link(0, ga(2, 0, 53), true), link(0, ga(2, 0, 53), false)],
            ROOMY,
        )
        .unwrap_err();
        assert_eq!(
            error,
            GroupTableError::DuplicateLink {
                object: 0,
                group_address: ga(2, 0, 53)
            }
        );
    }

    #[test]
    fn two_sending_addresses_on_one_object_are_refused() {
        let error = build_group_tables(
            IndividualAddress::from_raw(0x1143),
            &[link(0, ga(2, 0, 53), true), link(0, ga(2, 0, 54), true)],
            ROOMY,
        )
        .unwrap_err();
        assert_eq!(
            error,
            GroupTableError::SeveralSendingAddresses { object: 0 }
        );
    }

    /// The device's `MaxEntries` bounds the tables, and nothing is dropped
    /// to fit.
    #[test]
    fn the_capacity_is_enforced() {
        let links: Vec<TableLink> = (1..=3)
            .map(|sub| link(sub as u8, ga(1, 0, sub), true))
            .collect();
        let small = TableCapacity {
            group_addresses: 2,
            associations: 255,
        };
        assert_eq!(
            build_group_tables(IndividualAddress::from_raw(0x1143), &links, small).unwrap_err(),
            GroupTableError::TooManyGroupAddresses {
                count: 3,
                capacity: 2
            }
        );
        let small = TableCapacity {
            group_addresses: 255,
            associations: 2,
        };
        assert_eq!(
            build_group_tables(IndividualAddress::from_raw(0x1143), &links, small).unwrap_err(),
            GroupTableError::TooManyAssociations {
                count: 3,
                capacity: 2
            }
        );
    }

    /// One Length octet counts at most 255 entries, the individual address
    /// among them, so at most 254 group addresses — whatever the product
    /// data claims. Likewise one Current Size octet counts at most 255.
    #[test]
    fn the_count_octets_bound_the_tables_too() {
        let links: Vec<TableLink> = (0..255u16)
            .map(|n| link((n % 200) as u8, GroupAddress::from_raw(n + 1), false))
            .collect();
        assert_eq!(
            build_group_tables(IndividualAddress::from_raw(0x1143), &links, ROOMY).unwrap_err(),
            GroupTableError::TooManyGroupAddresses {
                count: 255,
                capacity: 254
            }
        );
        let links: Vec<TableLink> = (0..256u16)
            .map(|n| link((n % 128) as u8, GroupAddress::from_raw(n / 2 + 1), false))
            .collect();
        let huge = TableCapacity {
            group_addresses: 1000,
            associations: 1000,
        };
        assert_eq!(
            build_group_tables(IndividualAddress::from_raw(0x1143), &links, huge).unwrap_err(),
            GroupTableError::TooManyAssociations {
                count: 256,
                capacity: 255
            }
        );
    }
}
