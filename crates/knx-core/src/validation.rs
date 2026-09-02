//! Validation rules. These are where correctness lives — not in the UI, not
//! in storage (ARCHITECTURE §6): duplicate individual address, a group
//! address outside its `GroupRange`, and a link to a deleted or missing
//! object.

use std::fmt;

use crate::address::{GroupAddress, IndividualAddress};
use crate::devices::Devices;
use crate::group::GroupRange;
use crate::ids::{ComObjectInstanceId, DeviceId, GroupAddressId, GroupRangeId};
use crate::installation::Installation;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    DuplicateIndividualAddress {
        address: IndividualAddress,
        existing: DeviceId,
        new: DeviceId,
    },
    GroupAddressOutsideRange {
        address: GroupAddress,
        range: GroupRangeId,
    },
    DanglingGroupLink {
        com_object: ComObjectInstanceId,
        ga: GroupAddressId,
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
            ValidationError::GroupAddressOutsideRange { address, range } => write!(
                f,
                "group address {} falls outside range {range}",
                address.raw()
            ),
            ValidationError::DanglingGroupLink { com_object, ga } => write!(
                f,
                "communication object {com_object} links to group address {ga}, which does not exist"
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
    use crate::ids::SourceRef;
    use crate::topology::Topology;

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
}
