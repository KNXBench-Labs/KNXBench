//! Validation rules. These are where correctness lives — not in the UI, not
//! in storage (ARCHITECTURE §6): duplicate individual address, a group
//! address outside its `GroupRange`, and a link to a deleted or missing
//! object.

use std::fmt;

use crate::address::{GroupAddress, IndividualAddress};
use crate::devices::Devices;
use crate::group::GroupRange;
use crate::ids::{AreaId, ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId, LineId};
use crate::installation::Installation;
use crate::topology::{Area, Line, Topology};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    DuplicateIndividualAddress {
        address: IndividualAddress,
        existing: DeviceId,
        new: DeviceId,
    },
    AddressOutsideAssignedLine {
        device: DeviceId,
        address: IndividualAddress,
        area: u8,
        line: u8,
    },
    CouplerAddressRequiresClassification {
        device: DeviceId,
        address: IndividualAddress,
    },
    LineWithoutArea {
        line: LineId,
    },
    LineWithMultipleAreas {
        line: LineId,
    },
    MultipleLineMembership {
        device: DeviceId,
    },
    MultipleTopologyPlacements {
        device: DeviceId,
    },
    GroupAddressOutsideRange {
        address: GroupAddress,
        range: GroupRangeId,
    },
    DanglingGroupLink {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
    },
    DuplicateGroupAddress {
        address: GroupAddress,
        existing: GroupAddressId,
        new: GroupAddressId,
    },
    DuplicateAreaAddress {
        address: u8,
        existing: AreaId,
        new: AreaId,
    },
    DuplicateLineAddress {
        address: u8,
        existing: LineId,
        new: LineId,
    },
    GroupRangeOutsideParent {
        range: GroupRangeId,
        parent: GroupRangeId,
    },
    OverlappingGroupRange {
        range: GroupRangeId,
        existing: GroupRangeId,
    },
    GroupRangeInverted {
        range: GroupRangeId,
        start: GroupAddress,
        end: GroupAddress,
    },
}

impl std::error::Error for ValidationError {}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationError::DuplicateIndividualAddress {
                address,
                existing,
                new,
            } => write!(
                f,
                "individual address {address} already used by device {existing}, cannot assign to device {new}"
            ),
            ValidationError::AddressOutsideAssignedLine { device, address, area, line } => write!(
                f,
                "device {device} address {address} does not match assigned line {area}.{line}; clear the address before moving lines"
            ),
            ValidationError::CouplerAddressRequiresClassification { device, address } => write!(
                f,
                "address {address} ends in 0, reserved for couplers; device {device} cannot be assigned a new coupler address without device classification"
            ),
            ValidationError::LineWithoutArea { line } => write!(
                f, "line {line} has no owning area; cannot safely assign an address"
            ),
            ValidationError::LineWithMultipleAreas { line } => write!(
                f, "line {line} belongs to multiple areas; cannot safely assign an address"
            ),
            ValidationError::MultipleLineMembership { device } => write!(
                f, "device {device} appears on multiple lines; cannot safely assign an address"
            ),
            ValidationError::MultipleTopologyPlacements { device } => write!(
                f,
                "device {device} appears in multiple topology placements; repair the topology before editing its address or line"
            ),
            ValidationError::GroupAddressOutsideRange { address, range } => write!(
                f,
                "group address {} falls outside range {range}",
                address.raw()
            ),
            ValidationError::DanglingGroupLink { com_object, ga } => write!(
                f,
                "communication object {com_object} links to group address {ga}, which does not exist"
            ),
            ValidationError::DuplicateGroupAddress {
                address,
                existing,
                new,
            } => write!(
                f,
                "group address {} already used by group address {existing}, cannot assign to group address {new}",
                address.raw()
            ),
            ValidationError::DuplicateAreaAddress {
                address,
                existing,
                new,
            } => write!(
                f,
                "area address {address} already used by area {existing}, cannot assign to area {new}"
            ),
            ValidationError::DuplicateLineAddress {
                address,
                existing,
                new,
            } => write!(
                f,
                "line address {address} already used by line {existing} in the same area, cannot assign to line {new}"
            ),
            ValidationError::GroupRangeOutsideParent { range, parent } => write!(
                f,
                "group range {range} does not nest inside its parent range {parent}"
            ),
            ValidationError::OverlappingGroupRange { range, existing } => write!(
                f,
                "group range {range} overlaps existing range {existing}"
            ),
            ValidationError::GroupRangeInverted { range, start, end } => write!(
                f,
                "group range {range} has start {} after end {}",
                start.raw(),
                end.raw()
            ),
        }
    }
}

/// Rejects assigning `address` to `candidate` if any other device already
/// has it. A device reusing its own current address is not a duplicate.
pub fn check_no_duplicate_individual_address(
    devices: &Devices,
    candidate: DeviceId,
    address: IndividualAddress,
) -> Result<(), ValidationError> {
    for device in devices.iter() {
        if device.id != candidate && device.address == Some(address) {
            return Err(ValidationError::DuplicateIndividualAddress {
                address,
                existing: device.id,
                new: candidate,
            });
        }
    }
    Ok(())
}

/// Validate a complete physical address against its assigned topology line.
/// `preserve_existing_zero` permits only an unchanged, already imported
/// coupler address: without a device-kind model, a new zero cannot be
/// distinguished from an invalid ordinary-device assignment.
pub fn check_individual_address_on_line(
    device: DeviceId,
    address: IndividualAddress,
    area: u8,
    line: u8,
    preserve_existing_zero: bool,
) -> Result<(), ValidationError> {
    if address.area() != area || address.line() != line {
        return Err(ValidationError::AddressOutsideAssignedLine {
            device,
            address,
            area,
            line,
        });
    }
    if address.device() == 0 && !preserve_existing_zero {
        return Err(ValidationError::CouplerAddressRequiresClassification { device, address });
    }
    Ok(())
}

/// Rejects assigning `address` to `candidate` if any other group address
/// entry in `installation` already has it. An entry keeping its own
/// current address is not a duplicate.
pub fn check_no_duplicate_group_address(
    installation: &Installation,
    candidate: GroupAddressId,
    address: GroupAddress,
) -> Result<(), ValidationError> {
    for entry in &installation.group_addresses {
        if entry.id != candidate && entry.address == address {
            return Err(ValidationError::DuplicateGroupAddress {
                address,
                existing: entry.id,
                new: candidate,
            });
        }
    }
    Ok(())
}

/// Rejects assigning `address` to `candidate` if any other area in
/// `topology` already has it. An area reusing its own current address is
/// not a duplicate.
pub fn check_no_duplicate_area_address(
    topology: &Topology,
    candidate: AreaId,
    address: u8,
) -> Result<(), ValidationError> {
    for area in &topology.areas {
        if area.id != candidate && area.address == address {
            return Err(ValidationError::DuplicateAreaAddress {
                address,
                existing: area.id,
                new: candidate,
            });
        }
    }
    Ok(())
}

/// Rejects assigning `address` to `candidate` if any other line *owned by
/// `area`* already has it — line addresses are unique per area
/// (Area.Line.Device numbering), not project-wide, so this only ever
/// looks at `area`'s own line list, resolved against `lines`.
pub fn check_no_duplicate_line_address(
    area: &Area,
    lines: &[Line],
    candidate: LineId,
    address: u8,
) -> Result<(), ValidationError> {
    for &line_id in &area.lines {
        if line_id == candidate {
            continue;
        }
        if let Some(line) = lines.iter().find(|l| l.id == line_id) {
            if line.address == address {
                return Err(ValidationError::DuplicateLineAddress {
                    address,
                    existing: line.id,
                    new: candidate,
                });
            }
        }
    }
    Ok(())
}

/// Rejects a group range whose `start` comes after its `end` — an inverted
/// span that `GroupRange::contains` (which assumes `start <= x <= end`)
/// could never match, and that would also confuse the overlap check below
/// (its interval test assumes canonical ordering). `start == end` (a
/// single-address range) is well-ordered and accepted.
pub fn check_group_range_is_well_ordered(
    candidate: GroupRangeId,
    start: GroupAddress,
    end: GroupAddress,
) -> Result<(), ValidationError> {
    if start > end {
        Err(ValidationError::GroupRangeInverted {
            range: candidate,
            start,
            end,
        })
    } else {
        Ok(())
    }
}

/// Rejects a group range whose `[start, end]` span is not entirely
/// contained by `parent`'s own span.
pub fn check_group_range_nests_in_parent(
    parent: &GroupRange,
    candidate: GroupRangeId,
    start: GroupAddress,
    end: GroupAddress,
) -> Result<(), ValidationError> {
    if parent.contains(start) && parent.contains(end) {
        Ok(())
    } else {
        Err(ValidationError::GroupRangeOutsideParent {
            range: candidate,
            parent: parent.id,
        })
    }
}

/// Rejects a group range whose `[start, end]` span overlaps any sibling's
/// (ranges at the same nesting level — all main ranges, or all middle
/// ranges under the same parent). A range checked against its own current
/// span is not an overlap with itself.
pub fn check_no_overlapping_group_range<'a>(
    siblings: impl Iterator<Item = &'a GroupRange>,
    candidate: GroupRangeId,
    start: GroupAddress,
    end: GroupAddress,
) -> Result<(), ValidationError> {
    for sibling in siblings {
        if sibling.id == candidate {
            continue;
        }
        let overlaps = sibling.start.raw() <= end.raw() && start.raw() <= sibling.end.raw();
        if overlaps {
            return Err(ValidationError::OverlappingGroupRange {
                range: candidate,
                existing: sibling.id,
            });
        }
    }
    Ok(())
}

/// Rejects a group address that falls outside its range.
pub fn check_group_address_in_range(
    range: &GroupRange,
    address: GroupAddress,
) -> Result<(), ValidationError> {
    if range.contains(address) {
        Ok(())
    } else {
        Err(ValidationError::GroupAddressOutsideRange {
            address,
            range: range.id,
        })
    }
}

/// Rejects a link to a group address that does not exist in `installation`.
pub fn check_group_link_target_exists(
    installation: &Installation,
    com_object: ComObjectInstanceId,
    ga: GroupAddressId,
) -> Result<(), ValidationError> {
    if installation
        .group_addresses
        .iter()
        .any(|entry| entry.id == ga)
    {
        Ok(())
    } else {
        Err(ValidationError::DanglingGroupLink { com_object, ga })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commissioning::{CommissioningState, CompletionStatus};
    use crate::device::DeviceInstance;
    use crate::group::GroupRange;
    use crate::ids::{AreaId, GroupRangeId, LineId, SourceRef};
    use crate::topology::{Area, Line, Topology};

    fn test_source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn make_device(id: DeviceId, address: Option<IndividualAddress>) -> DeviceInstance {
        DeviceInstance {
            id,
            source: test_source(),
            name: "D".into(),
            description: None,
            address,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    #[test]
    fn duplicate_individual_address_is_rejected() {
        let mut devices = Devices::new();
        devices.insert(make_device(
            DeviceId(1),
            Some(IndividualAddress::new(1, 1, 1).unwrap()),
        ));
        let err = check_no_duplicate_individual_address(
            &devices,
            DeviceId(2),
            IndividualAddress::new(1, 1, 1).unwrap(),
        );
        assert!(matches!(
            err,
            Err(ValidationError::DuplicateIndividualAddress { .. })
        ));
    }

    #[test]
    fn same_device_reusing_its_own_address_is_not_a_duplicate() {
        let mut devices = Devices::new();
        let addr = IndividualAddress::new(1, 1, 1).unwrap();
        devices.insert(make_device(DeviceId(1), Some(addr)));
        assert!(check_no_duplicate_individual_address(&devices, DeviceId(1), addr).is_ok());
    }

    #[test]
    fn group_address_outside_range_is_rejected() {
        let range = GroupRange {
            id: GroupRangeId(1),
            source: test_source(),
            name: "R".into(),
            start: GroupAddress::from_raw(0),
            end: GroupAddress::from_raw(10),
            parent: None,
            children: vec![],
        };
        assert!(check_group_address_in_range(&range, GroupAddress::from_raw(11)).is_err());
        assert!(check_group_address_in_range(&range, GroupAddress::from_raw(5)).is_ok());
    }

    #[test]
    fn group_link_to_nonexistent_address_is_dangling() {
        let installation = Installation {
            id: crate::ids::InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        };
        let err = check_group_link_target_exists(
            &installation,
            ComObjectInstanceId(1),
            GroupAddressId(99),
        );
        assert!(matches!(
            err,
            Err(ValidationError::DanglingGroupLink { .. })
        ));
    }

    #[test]
    fn check_no_duplicate_group_address_rejects_a_second_entry_with_the_same_address() {
        let installation = Installation {
            id: crate::ids::InstallationId(0),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: crate::commissioning::CompletionStatus::FinishedDesign,
            topology: crate::topology::Topology {
                areas: vec![],
                lines: vec![],
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![crate::group::GroupAddressEntry {
                id: GroupAddressId(1),
                source: test_source(),
                name: "Existing".into(),
                address: GroupAddress::from_raw(5),
                central: false,
                unfiltered: false,
                range: None,
            }],
            parameters: vec![],
        };
        let result = check_no_duplicate_group_address(
            &installation,
            GroupAddressId(2),
            GroupAddress::from_raw(5),
        );
        assert!(matches!(
            result,
            Err(ValidationError::DuplicateGroupAddress { existing, new, .. })
                if existing == GroupAddressId(1) && new == GroupAddressId(2)
        ));
        // An entry keeping its own current address is not a duplicate of itself.
        assert!(check_no_duplicate_group_address(
            &installation,
            GroupAddressId(1),
            GroupAddress::from_raw(5)
        )
        .is_ok());
    }

    #[test]
    fn duplicate_area_address_is_rejected() {
        let topology = Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: test_source(),
                name: "A1".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![],
            }],
            lines: vec![],
            unassigned: vec![],
        };
        let err = check_no_duplicate_area_address(&topology, AreaId(2), 1);
        assert!(matches!(
            err,
            Err(ValidationError::DuplicateAreaAddress { .. })
        ));
    }

    #[test]
    fn an_area_reusing_its_own_address_is_not_a_duplicate() {
        let topology = Topology {
            areas: vec![Area {
                id: AreaId(1),
                source: test_source(),
                name: "A1".into(),
                address: 1,
                completion: CompletionStatus::FinishedDesign,
                lines: vec![],
            }],
            lines: vec![],
            unassigned: vec![],
        };
        assert!(check_no_duplicate_area_address(&topology, AreaId(1), 1).is_ok());
    }

    fn test_line(id: LineId, address: u8) -> Line {
        Line {
            id,
            source: test_source(),
            name: "L".into(),
            address,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices: vec![],
        }
    }

    #[test]
    fn duplicate_line_address_within_the_same_area_is_rejected() {
        let area = Area {
            id: AreaId(1),
            source: test_source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        };
        let lines = vec![test_line(LineId(1), 1)];
        let err = check_no_duplicate_line_address(&area, &lines, LineId(2), 1);
        assert!(matches!(
            err,
            Err(ValidationError::DuplicateLineAddress { .. })
        ));
    }

    #[test]
    fn a_line_reusing_its_own_address_is_not_a_duplicate() {
        let area = Area {
            id: AreaId(1),
            source: test_source(),
            name: "A".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![LineId(1)],
        };
        let lines = vec![test_line(LineId(1), 1)];
        assert!(check_no_duplicate_line_address(&area, &lines, LineId(1), 1).is_ok());
    }

    #[test]
    fn the_same_address_in_a_different_area_is_not_a_duplicate() {
        // check_no_duplicate_line_address only ever sees one area's own
        // line list, so a different area's line sharing the same address
        // number never reaches it — this documents that scoping choice.
        let area = Area {
            id: AreaId(2),
            source: test_source(),
            name: "A2".into(),
            address: 2,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![],
        };
        let other_areas_line = vec![test_line(LineId(1), 1)];
        assert!(check_no_duplicate_line_address(&area, &other_areas_line, LineId(2), 1).is_ok());
    }

    fn test_range(
        id: GroupRangeId,
        start: u16,
        end: u16,
        parent: Option<GroupRangeId>,
    ) -> GroupRange {
        GroupRange {
            id,
            source: test_source(),
            name: "R".into(),
            start: GroupAddress::from_raw(start),
            end: GroupAddress::from_raw(end),
            parent,
            children: vec![],
        }
    }

    #[test]
    fn an_inverted_range_is_rejected() {
        let err = check_group_range_is_well_ordered(
            GroupRangeId(1),
            GroupAddress::from_raw(255),
            GroupAddress::from_raw(0),
        );
        assert!(matches!(
            err,
            Err(ValidationError::GroupRangeInverted { .. })
        ));
    }

    #[test]
    fn a_well_ordered_range_is_accepted() {
        assert!(check_group_range_is_well_ordered(
            GroupRangeId(1),
            GroupAddress::from_raw(0),
            GroupAddress::from_raw(255)
        )
        .is_ok());
        // The degenerate single-address case (start == end) is well-ordered.
        assert!(check_group_range_is_well_ordered(
            GroupRangeId(1),
            GroupAddress::from_raw(100),
            GroupAddress::from_raw(100)
        )
        .is_ok());
    }

    #[test]
    fn a_range_nesting_inside_its_parent_is_accepted() {
        let parent = test_range(GroupRangeId(1), 0, 2047, None);
        assert!(check_group_range_nests_in_parent(
            &parent,
            GroupRangeId(2),
            GroupAddress::from_raw(0),
            GroupAddress::from_raw(255)
        )
        .is_ok());
    }

    #[test]
    fn a_range_extending_past_its_parent_is_rejected() {
        let parent = test_range(GroupRangeId(1), 0, 255, None);
        let err = check_group_range_nests_in_parent(
            &parent,
            GroupRangeId(2),
            GroupAddress::from_raw(0),
            GroupAddress::from_raw(2047),
        );
        assert!(matches!(
            err,
            Err(ValidationError::GroupRangeOutsideParent { .. })
        ));
    }

    #[test]
    fn overlapping_sibling_ranges_are_rejected() {
        let existing = test_range(GroupRangeId(1), 0, 255, None);
        let siblings = [existing];
        let err = check_no_overlapping_group_range(
            siblings.iter(),
            GroupRangeId(2),
            GroupAddress::from_raw(200),
            GroupAddress::from_raw(500),
        );
        assert!(matches!(
            err,
            Err(ValidationError::OverlappingGroupRange { .. })
        ));
    }

    #[test]
    fn non_overlapping_sibling_ranges_are_accepted() {
        let existing = test_range(GroupRangeId(1), 0, 255, None);
        let siblings = [existing];
        assert!(check_no_overlapping_group_range(
            siblings.iter(),
            GroupRangeId(2),
            GroupAddress::from_raw(256),
            GroupAddress::from_raw(500)
        )
        .is_ok());
    }

    #[test]
    fn a_range_checked_against_its_own_current_span_is_not_an_overlap() {
        let existing = test_range(GroupRangeId(1), 0, 255, None);
        let siblings = [existing];
        assert!(check_no_overlapping_group_range(
            siblings.iter(),
            GroupRangeId(1),
            GroupAddress::from_raw(0),
            GroupAddress::from_raw(255)
        )
        .is_ok());
    }
}
