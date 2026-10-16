//! System-300 S-mode table elements are separate from property-array length values.
//!
//! Resources 03_05_01 v01.10.01 §4.16.7 pp234–236 and §4.17.5 pp248–250:
//! PID_TABLE23 uses element0 for length; elements1 onward contain actual rows.
//! The Type6 association grammar is shared with SystemB. This pure adapter
//! reuses that encoder, not its count-prefixed memory framing or mask admission.
//! Object/format discovery, load state, execution and recovery remain external.

use super::group_tables::TableCapacity;
use super::systemb_group_tables::{self, AssociationFormat, Link, TableError};

/// One complete table's immutable property values, without a property-service header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PropertyTable {
    length: [u8; 2],
    elements: Vec<u8>,
    element_width: usize,
}

impl PropertyTable {
    fn from_count_prefixed(mut image: Vec<u8>, element_width: usize) -> Self {
        let length = [image[0], image[1]];
        image.drain(..2);
        Self {
            length,
            elements: image,
            element_width,
        }
    }

    /// Current length at property-array index0, not a prefix of element1.
    pub fn length_value(&self) -> [u8; 2] {
        self.length
    }

    /// Number of actual elements, not bytes, capacity or an individual-address slot.
    pub fn element_count(&self) -> u16 {
        u16::from_be_bytes(self.length)
    }

    /// Octets in each actual element; element0 always has its own two-octet length.
    pub fn element_width(&self) -> usize {
        self.element_width
    }

    /// Actual elements beginning at property-array index1, without the length value.
    pub fn elements(&self) -> &[u8] {
        &self.elements
    }
}

/// Explicit S-mode address and Type6 association property images, not a load plan.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tables {
    /// PID_TABLE23 on a discovered S-mode Address Table Object (type1).
    pub address_table: PropertyTable,
    /// PID_TABLE23 on a discovered Association Table Object (type2).
    pub association_table: PropertyTable,
}

/// Builds property-array values using the existing Type6 link validation/encoder.
///
/// Caller supplies the discovered association format, one-based object bound and
/// capacities; no object index, profile or LTE format is inferred. Big-endian
/// words and sender-first ordering follow the explicitly selected shared codec.
/// Its conservative no-sender/broadcast refusals remain in force. Empty tables
/// select server acceptance/default-mapping semantics, not disabled communication.
/// No property access, loading, verification or real-device admission occurs.
pub fn build(
    format: AssociationFormat,
    object_count: u16,
    links: &[Link],
    capacity: TableCapacity,
) -> Result<Tables, TableError> {
    let tables = systemb_group_tables::build(format, object_count, links, capacity)?;
    let association_width = match format {
        AssociationFormat::Format0 => 2,
        AssociationFormat::Format1 => 4,
    };
    Ok(Tables {
        address_table: PropertyTable::from_count_prefixed(tables.address_table, 2),
        association_table: PropertyTable::from_count_prefixed(
            tables.association_table,
            association_width,
        ),
    })
}
