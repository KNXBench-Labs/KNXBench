//! Explicit System-B address and association bytes without product or download admission.
//!
//! Resources 03_05_01 v01.10.01 §4.16.8.2.4 Table78 (printed239)
//! defines a two-octet address count with no individual-address slot and
//! ascending group addresses. §4.17.5.2.5 Tables81–82 (printed249–250)
//! defines association formats0/1, one-based TSAP/ASAP and first-match sending.
//! This API explicitly emits big-endian words; source/profile binding, format
//! discovery, placement, allocation and recovery are the caller's responsibility.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use super::group_tables::TableCapacity;
use crate::GroupAddress;

/// Explicit association representation, not selected from a mask or table size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationFormat {
    /// Table81: one octet each for TSAP and ASAP; two-octet row count.
    Format0,
    /// Table82: two octets each for TSAP and ASAP; two-octet row count.
    Format1,
}

/// A group link with an explicit one-based source object number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Link {
    /// One-based ASAP, never a zero-based legacy object index.
    pub object: u16,
    /// Engineering group address; zero/broadcast is not admitted by this API.
    pub group_address: GroupAddress,
    /// Exactly one link per represented object must name the intended sender.
    pub sending: bool,
}

/// Complete count-prefixed images, not property-service framing or memory placement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tables {
    /// Two-octet count followed by ascending two-octet group addresses.
    pub address_table: Vec<u8>,
    /// Two-octet row count followed by explicit-format TSAP/ASAP pairs.
    pub association_table: Vec<u8>,
}

/// Why the complete link set cannot be represented without losing intent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TableError {
    /// Zero or a number beyond the explicitly supplied object-table count.
    ObjectOutOfTable { number: u16, count: u16 },
    /// Format0 cannot encode a linked ASAP beyond one octet.
    ObjectNotRepresentable { number: u16 },
    /// This engineering builder does not create broadcast associations.
    BroadcastAddress { object: u16 },
    /// Repeated object/address pair, regardless of the sending marker.
    DuplicateLink {
        object: u16,
        group_address: GroupAddress,
    },
    /// First-match semantics cannot preserve two requested sending addresses.
    SeveralSendingAddresses { object: u16 },
    /// No sender was selected; do not silently make a receiving link send.
    NoSendingAddress { object: u16 },
    /// Unique addresses exceed the caller's capacity or explicit format width.
    TooManyGroupAddresses { count: usize, capacity: usize },
    /// Associations exceed the caller's capacity or the two-octet row count.
    TooManyAssociations { count: usize, capacity: usize },
}

impl fmt::Display for TableError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ObjectOutOfTable { number, count } => write!(f, "System-B object {number} is outside 1..={count}"),
            Self::ObjectNotRepresentable { number } => write!(f, "System-B format0 cannot encode object {number}"),
            Self::BroadcastAddress { object } => write!(f, "System-B object {object} links to broadcast, not an engineering group address"),
            Self::DuplicateLink { object, group_address } => write!(f, "System-B object {object} links to {:04X}h twice", group_address.raw()),
            Self::SeveralSendingAddresses { object } => write!(f, "System-B object {object} requests several sending addresses"),
            Self::NoSendingAddress { object } => write!(f, "System-B object {object} has no explicit sending address; first-match encoding would invent one"),
            Self::TooManyGroupAddresses { count, capacity } => write!(f, "System-B table has {count} group addresses, capacity is {capacity}"),
            Self::TooManyAssociations { count, capacity } => write!(f, "System-B table has {count} associations, capacity is {capacity}"),
        }
    }
}

impl std::error::Error for TableError {}

/// Builds the explicitly selected, count-prefixed big-endian table grammar.
///
/// Addresses are unique and sorted. Rows are deterministic by ASAP, sender
/// first, then ascending group address; the first row for each ASAP therefore
/// preserves its declared sender. Duplicate links and missing/multiple senders
/// refuse instead of being normalized away. A no-sender request could be valid
/// with independent transmit-disabled object evidence, but this bounded API
/// cannot prove that and refuses it. Empty output is literal zero-length tables,
/// not a promise to disable all device communication: System-B zero-length tables
/// have separate management-server default semantics.
pub fn build(
    format: AssociationFormat,
    object_count: u16,
    links: &[Link],
    capacity: TableCapacity,
) -> Result<Tables, TableError> {
    let mut seen = BTreeSet::new();
    let mut senders = BTreeMap::<u16, bool>::new();
    let mut addresses = BTreeSet::new();
    for link in links {
        if link.object == 0 || link.object > object_count {
            return Err(TableError::ObjectOutOfTable {
                number: link.object,
                count: object_count,
            });
        }
        if format == AssociationFormat::Format0 && link.object > u16::from(u8::MAX) {
            return Err(TableError::ObjectNotRepresentable {
                number: link.object,
            });
        }
        if link.group_address.raw() == 0 {
            return Err(TableError::BroadcastAddress {
                object: link.object,
            });
        }
        if !seen.insert((link.object, link.group_address)) {
            return Err(TableError::DuplicateLink {
                object: link.object,
                group_address: link.group_address,
            });
        }
        let sender = senders.entry(link.object).or_default();
        if link.sending {
            if *sender {
                return Err(TableError::SeveralSendingAddresses {
                    object: link.object,
                });
            }
            *sender = true;
        }
        addresses.insert(link.group_address);
    }
    let index_limit = match format {
        AssociationFormat::Format0 => usize::from(u8::MAX),
        AssociationFormat::Format1 => usize::from(u16::MAX),
    };
    let address_room = capacity.group_addresses.min(index_limit);
    if addresses.len() > address_room {
        return Err(TableError::TooManyGroupAddresses {
            count: addresses.len(),
            capacity: address_room,
        });
    }
    let association_room = capacity.associations.min(usize::from(u16::MAX));
    if links.len() > association_room {
        return Err(TableError::TooManyAssociations {
            count: links.len(),
            capacity: association_room,
        });
    }
    for (object, sending) in senders {
        if !sending {
            return Err(TableError::NoSendingAddress { object });
        }
    }
    let addresses: Vec<_> = addresses.into_iter().collect();
    let address_count = u16::try_from(addresses.len()).expect("validated address count");
    let mut address_table = Vec::with_capacity(2 + addresses.len() * 2);
    address_table.extend_from_slice(&address_count.to_be_bytes());
    for address in &addresses {
        address_table.extend_from_slice(&address.raw().to_be_bytes());
    }
    let row_width = if format == AssociationFormat::Format0 {
        2
    } else {
        4
    };
    let mut association_table = Vec::with_capacity(2 + links.len() * row_width);
    association_table.extend_from_slice(
        &u16::try_from(links.len())
            .expect("validated row count")
            .to_be_bytes(),
    );
    let mut rows: Vec<_> = links.iter().collect();
    rows.sort_by_key(|link| (link.object, !link.sending, link.group_address));
    for link in rows {
        let tsap = u16::try_from(
            addresses
                .binary_search(&link.group_address)
                .expect("collected address")
                + 1,
        )
        .expect("validated TSAP range");
        match format {
            AssociationFormat::Format0 => {
                association_table.extend_from_slice(&[
                    u8::try_from(tsap).expect("validated format0 TSAP"),
                    u8::try_from(link.object).expect("validated format0 ASAP"),
                ]);
            }
            AssociationFormat::Format1 => {
                association_table.extend_from_slice(&tsap.to_be_bytes());
                association_table.extend_from_slice(&link.object.to_be_bytes());
            }
        }
    }
    Ok(Tables {
        address_table,
        association_table,
    })
}
