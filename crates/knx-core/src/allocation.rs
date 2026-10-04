//! Lowest free individual addresses on one line, for opt-in device allocation.
//!
//! MODEL-04: the catalog can place a batch of new devices on a line and, only
//! when asked, give each one an address. The allocator answers *which*
//! addresses; the caller still assigns them through `SetIndividualAddress`,
//! so the core's line-prefix and uniqueness checks run on every result.
//!
//! Policy: device octets 1–255 in ascending order. Octet 0 is the coupler's
//! (MODEL-03) and never allocated. An address already used by any device in
//! the project, in any installation, is skipped, and so is an address on the
//! project exclusion list ([`crate::EXCLUDED_INDIVIDUAL_ADDRESSES`]), which no
//! address iteration may contain. The allocator knows only the project, not
//! the real bus.

use std::collections::HashSet;
use std::fmt;

use crate::ids::LineId;
use crate::project::Project;
use crate::{is_project_excluded, IndividualAddress};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AddressAllocationError {
    LineNotFound(LineId),
    /// The line id appears more than once, in several installations, or the
    /// line is not owned by exactly one area: no prefix can be chosen safely.
    LinePlacementAmbiguous(LineId),
    NotEnoughFreeAddresses {
        area: u8,
        line: u8,
        free: usize,
        requested: usize,
    },
}

impl fmt::Display for AddressAllocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LineNotFound(line) => write!(f, "line {line} not found"),
            Self::LinePlacementAmbiguous(line) => write!(
                f,
                "line {line} is not owned by exactly one area; cannot allocate addresses"
            ),
            Self::NotEnoughFreeAddresses {
                area,
                line,
                free,
                requested,
            } => write!(
                f,
                "line {area}.{line} has {free} free device addresses, {requested} requested"
            ),
        }
    }
}

impl std::error::Error for AddressAllocationError {}

/// The `count` lowest free addresses on `line`, or an error that names why
/// none are returned. Never returns fewer than `count`.
pub fn free_line_addresses(
    project: &Project,
    line: LineId,
    count: usize,
) -> Result<Vec<IndividualAddress>, AddressAllocationError> {
    let (area, line_address) = line_prefix(project, line)?;
    let used: HashSet<IndividualAddress> =
        project.devices.iter().filter_map(|d| d.address).collect();
    let free: Vec<IndividualAddress> = (1..=255u8)
        .filter_map(|device| IndividualAddress::new(area, line_address, device).ok())
        .filter(|address| !used.contains(address) && !is_project_excluded(*address))
        .collect();
    if free.len() < count {
        return Err(AddressAllocationError::NotEnoughFreeAddresses {
            area,
            line: line_address,
            free: free.len(),
            requested: count,
        });
    }
    Ok(free.into_iter().take(count).collect())
}

/// `(area address, line address)` for a line that exists exactly once in the
/// project and has exactly one owning area.
fn line_prefix(project: &Project, line: LineId) -> Result<(u8, u8), AddressAllocationError> {
    let mut found = None;
    for installation in &project.installations {
        let topology = &installation.topology;
        for candidate in topology.lines.iter().filter(|l| l.id == line) {
            let mut owners = topology.areas.iter().filter(|a| a.lines.contains(&line));
            let (Some(area), None) = (owners.next(), owners.next()) else {
                return Err(AddressAllocationError::LinePlacementAmbiguous(line));
            };
            if found.replace((area.address, candidate.address)).is_some() {
                return Err(AddressAllocationError::LinePlacementAmbiguous(line));
            }
        }
    }
    found.ok_or(AddressAllocationError::LineNotFound(line))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commissioning::{CommissioningState, CompletionStatus};
    use crate::device::DeviceInstance;
    use crate::ids::{AreaId, DeviceId, InstallationId, SourceRef};
    use crate::installation::Installation;
    use crate::string_table::Language;
    use crate::topology::{Area, Line, Topology};

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    fn line(id: u32, address: u8) -> Line {
        Line {
            id: LineId(id),
            source: source(),
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

    fn installation(id: u8, area: u8, lines: Vec<Line>) -> Installation {
        Installation {
            id: InstallationId(id),
            name: "I".into(),
            default_line: None,
            multicast_address: None,
            completion: CompletionStatus::FinishedDesign,
            topology: Topology {
                areas: vec![Area {
                    id: AreaId(u32::from(id)),
                    source: source(),
                    name: "A".into(),
                    address: area,
                    completion: CompletionStatus::FinishedDesign,
                    lines: lines.iter().map(|l| l.id).collect(),
                }],
                lines,
                unassigned: vec![],
            },
            buildings: vec![],
            group_ranges: vec![],
            group_addresses: vec![],
            parameters: vec![],
        }
    }

    fn device(id: u32, address: &str) -> DeviceInstance {
        DeviceInstance {
            id: DeviceId(id),
            source: source(),
            name: "D".into(),
            description: None,
            address: Some(address.parse().unwrap()),
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        }
    }

    fn project() -> Project {
        let mut project = Project::new(Language("en".into()));
        project
            .installations
            .push(installation(0, 1, vec![line(1, 1)]));
        project
    }

    fn ia(text: &str) -> IndividualAddress {
        text.parse().unwrap()
    }

    #[test]
    fn lowest_free_octets_skip_zero_and_every_used_address() {
        let mut project = project();
        // Used anywhere in the project counts, placed on the line or not.
        project.devices.insert(device(1, "1.1.1"));
        project.devices.insert(device(2, "1.1.3"));
        let free = free_line_addresses(&project, LineId(1), 3).unwrap();
        assert_eq!(free, [ia("1.1.2"), ia("1.1.4"), ia("1.1.5")]);
    }

    #[test]
    fn the_excluded_address_is_never_allocated() {
        let mut project = project();
        for octet in 1..=219u32 {
            project
                .devices
                .insert(device(octet, &format!("1.1.{octet}")));
        }
        let free = free_line_addresses(&project, LineId(1), 2).unwrap();
        assert_eq!(free, [ia("1.1.221"), ia("1.1.222")]);
    }

    #[test]
    fn a_full_line_refuses_the_whole_request() {
        let mut project = project();
        for octet in 1..=253u32 {
            project
                .devices
                .insert(device(octet, &format!("1.1.{octet}")));
        }
        // 254 and 255 remain (220 is excluded and also used here).
        assert_eq!(
            free_line_addresses(&project, LineId(1), 2).unwrap().len(),
            2
        );
        assert_eq!(
            free_line_addresses(&project, LineId(1), 3).unwrap_err(),
            AddressAllocationError::NotEnoughFreeAddresses {
                area: 1,
                line: 1,
                free: 2,
                requested: 3
            }
        );
    }

    #[test]
    fn unknown_or_ambiguous_lines_are_refused() {
        let project = project();
        assert_eq!(
            free_line_addresses(&project, LineId(9), 1).unwrap_err(),
            AddressAllocationError::LineNotFound(LineId(9))
        );
        let mut twice = project.clone();
        twice
            .installations
            .push(installation(1, 2, vec![line(1, 1)]));
        assert_eq!(
            free_line_addresses(&twice, LineId(1), 1).unwrap_err(),
            AddressAllocationError::LinePlacementAmbiguous(LineId(1))
        );
        let mut orphan = project.clone();
        orphan.installations[0].topology.areas.clear();
        assert_eq!(
            free_line_addresses(&orphan, LineId(1), 1).unwrap_err(),
            AddressAllocationError::LinePlacementAmbiguous(LineId(1))
        );
        let mut two_owners = project;
        let mut second_area = two_owners.installations[0].topology.areas[0].clone();
        second_area.id = AreaId(7);
        second_area.address = 2;
        two_owners.installations[0].topology.areas.push(second_area);
        assert_eq!(
            free_line_addresses(&two_owners, LineId(1), 1).unwrap_err(),
            AddressAllocationError::LinePlacementAmbiguous(LineId(1))
        );
    }

    #[test]
    fn a_line_in_a_later_installation_uses_its_own_prefix() {
        let mut project = project();
        project
            .installations
            .push(installation(1, 3, vec![line(2, 4)]));
        let free = free_line_addresses(&project, LineId(2), 1).unwrap();
        assert_eq!(free, [ia("3.4.1")]);
    }
}
