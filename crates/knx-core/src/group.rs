//! Group ranges and group address entities.
//!
//! `GroupAddress` (in `address.rs`) is the typed 16-bit value with parse and
//! format; `GroupAddressEntry` here is the project entity that carries one
//! as a field (DATA_MODEL §4/§9). The two names are deliberate and distinct.

use crate::address::GroupAddress;
use crate::ids::{GroupAddressId, GroupRangeId, SourceRef};

/// A named span of group addresses, e.g. "Lighting".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRange {
    pub id: GroupRangeId,
    pub source: SourceRef,
    pub name: String,
    pub start: GroupAddress,
    pub end: GroupAddress,
}

impl GroupRange {
    /// Whether `ga` falls within this range, inclusive on both ends.
    pub fn contains(&self, ga: GroupAddress) -> bool {
        self.start.raw() <= ga.raw() && ga.raw() <= self.end.raw()
    }
}

/// A group address as a project entity: id, name, the address value, and
/// ETS's `Central`/`Unfiltered` flags.
///
/// A group address without a datapoint type is normal, not an error — 194
/// of 514 in the reference project (DATA_MODEL §9); the datapoint type is
/// not a field here at all, since it is a property of the linked
/// communication objects, not of the address.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupAddressEntry {
    pub id: GroupAddressId,
    pub source: SourceRef,
    pub name: String,
    pub address: GroupAddress,
    pub central: bool,
    pub unfiltered: bool,
    /// The range it was imported under, if any.
    pub range: Option<GroupRangeId>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn range_contains_is_inclusive_on_both_ends() {
        let r = GroupRange {
            id: GroupRangeId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "Lighting".into(),
            start: GroupAddress::from_raw(100),
            end: GroupAddress::from_raw(200),
        };
        assert!(r.contains(GroupAddress::from_raw(100)));
        assert!(r.contains(GroupAddress::from_raw(200)));
        assert!(!r.contains(GroupAddress::from_raw(201)));
    }
}
