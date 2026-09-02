//! One KNX installation inside a project: topology, buildings, group
//! addresses and parameters. A project may hold more than one — ETS numbers
//! installations from 0 — even though the reference project has only one.

use std::net::Ipv4Addr;

use crate::building::BuildingPart;
use crate::commissioning::CompletionStatus;
use crate::group::{GroupAddressEntry, GroupRange};
use crate::ids::{InstallationId, LineId};
use crate::parameter::ParameterInstance;
use crate::topology::Topology;

#[derive(Debug, Clone, PartialEq)]
pub struct Installation {
    pub id: InstallationId,
    pub name: String,
    pub default_line: Option<LineId>,
    /// `IPRoutingMulticastAddress`.
    pub multicast_address: Option<Ipv4Addr>,
    pub completion: CompletionStatus,
    pub topology: Topology,
    pub buildings: Vec<BuildingPart>,
    pub group_ranges: Vec<GroupRange>,
    pub group_addresses: Vec<GroupAddressEntry>,
    pub parameters: Vec<ParameterInstance>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_installation_is_constructible() {
        let installation = Installation {
            id: InstallationId(0),
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
        assert_eq!(installation.id, InstallationId(0));
    }
}
