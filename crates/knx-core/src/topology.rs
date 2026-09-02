//! Topology: Area → Line → devices. Holds references only — `Devices` is the
//! sole owner (DATA_MODEL §5). A device without a line is valid and lives in
//! `Topology::unassigned`; the reference project contains exactly one, and
//! it is precisely the device `xknxproject` loses.

use std::net::Ipv4Addr;

use crate::ids::{AreaId, DeviceId, LineId, SourceRef};
use crate::CompletionStatus;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Area {
    pub id: AreaId,
    pub source: SourceRef,
    pub name: String,
    pub address: u8,
    pub completion: CompletionStatus,
    pub lines: Vec<LineId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub id: LineId,
    pub source: SourceRef,
    pub name: String,
    pub address: u8,
    /// `MediumTypeRefId` — an opaque product reference, not interpreted
    /// here.
    pub medium_ref: String,
    /// `DomainAddress` — powerline and RF domain address, uninterpreted:
    /// its width and encoding depend on the medium.
    pub domain_address: Option<String>,
    pub domain_address_is_checked: Option<bool>,
    pub ip_routing_multicast_address: Option<Ipv4Addr>,
    pub multicast_ttl: Option<u8>,
    pub completion: CompletionStatus,
    pub devices: Vec<DeviceId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Topology {
    pub areas: Vec<Area>,
    pub lines: Vec<Line>,
    /// Devices with no line. Valid project state, not an error.
    pub unassigned: Vec<DeviceId>,
}

impl Topology {
    pub fn line(&self, id: LineId) -> Option<&Line> {
        self.lines.iter().find(|l| l.id == id)
    }

    pub fn area_of(&self, line: LineId) -> Option<&Area> {
        self.areas.iter().find(|a| a.lines.contains(&line))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source() -> SourceRef {
        SourceRef {
            path: "t".into(),
            ets_id: "t".into(),
        }
    }

    #[test]
    fn line_lookup_finds_line_inside_its_area() {
        let line = Line {
            id: LineId(1),
            source: source(),
            name: "L1".into(),
            address: 1,
            medium_ref: "TP".into(),
            domain_address: None,
            domain_address_is_checked: None,
            ip_routing_multicast_address: None,
            multicast_ttl: None,
            completion: CompletionStatus::FinishedDesign,
            devices: vec![],
        };
        let area = Area {
            id: AreaId(1),
            source: source(),
            name: "A1".into(),
            address: 1,
            completion: CompletionStatus::FinishedDesign,
            lines: vec![line.id],
        };
        let topo = Topology {
            areas: vec![area.clone()],
            lines: vec![line.clone()],
            unassigned: vec![],
        };
        assert_eq!(topo.line(LineId(1)), Some(&line));
        assert_eq!(topo.area_of(LineId(1)), Some(&area));
    }

    #[test]
    fn unassigned_device_is_valid_topology_state() {
        let topo = Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(7)],
        };
        assert_eq!(topo.unassigned, vec![DeviceId(7)]);
    }

    #[test]
    fn a_line_carries_its_medium_configuration() {
        let line = Line {
            id: LineId(1),
            source: source(),
            name: "Hauptlinie".into(),
            address: 1,
            medium_ref: "MT-0".into(),
            domain_address: Some("0".into()),
            domain_address_is_checked: Some(false),
            ip_routing_multicast_address: Some(Ipv4Addr::new(224, 0, 23, 12)),
            multicast_ttl: Some(16),
            completion: CompletionStatus::Accepted,
            devices: vec![],
        };
        assert_eq!(line.multicast_ttl, Some(16));
    }
}
