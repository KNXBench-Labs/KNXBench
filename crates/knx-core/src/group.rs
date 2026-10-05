//! Group ranges and group address entities.
//!
//! `GroupAddress` (in `address.rs`) is the typed 16-bit value with parse and
//! format; `GroupAddressEntry` here is the project entity that carries one
//! as a field (DATA_MODEL §4/§9). The two names are deliberate and distinct.

use crate::address::GroupAddress;
use crate::dpt::DptRef;
use crate::ids::{GroupAddressId, GroupRangeId, SourceRef};
use crate::provenance::Override;

/// A named span of group addresses, e.g. "Lighting".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupRange {
    pub id: GroupRangeId,
    pub source: SourceRef,
    pub name: String,
    pub start: GroupAddress,
    pub end: GroupAddress,
    /// The main range this one nests under, if this is a middle range.
    pub parent: Option<GroupRangeId>,
    /// The middle ranges nested directly under this one, if this is a main
    /// range. The reference project nests two levels deep — main range,
    /// middle range, address — never three.
    pub children: Vec<GroupRangeId>,
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
/// of 514 in the reference project (DATA_MODEL §9). The type is usually a
/// property of the linked communication objects; schema ≥21 projects may
/// also state one on the address itself, kept in `declared_dpt` and weighed
/// against the linked objects by `resolve_group_address_type` (ADR-0078).
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
    /// `GroupAddress/@DatapointType` (schema ≥21) in its four source states
    /// (ADR-0010, ADR-0078). Never filled by inference.
    pub declared_dpt: Override<DptRef>,
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
            parent: None,
            children: vec![],
        };
        assert!(r.contains(GroupAddress::from_raw(100)));
        assert!(r.contains(GroupAddress::from_raw(200)));
        assert!(!r.contains(GroupAddress::from_raw(201)));
    }

    #[test]
    fn group_ranges_nest_two_levels_deep() {
        let main = GroupRange {
            id: GroupRangeId(1),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "Licht".into(),
            start: GroupAddress::from_raw(2048),
            end: GroupAddress::from_raw(4095),
            parent: None,
            children: vec![GroupRangeId(2)],
        };
        let middle = GroupRange {
            id: GroupRangeId(2),
            source: SourceRef {
                path: "t".into(),
                ets_id: "t".into(),
            },
            name: "Licht - An/Aus".into(),
            start: GroupAddress::from_raw(2048),
            end: GroupAddress::from_raw(2303),
            parent: Some(main.id),
            children: vec![],
        };
        assert!(main.contains(middle.start));
        assert_eq!(middle.parent, Some(GroupRangeId(1)));
    }
}
