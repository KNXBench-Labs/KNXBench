//! A two-device project with links, flags and parameters for the P3 tests.

use crate::building::BuildingPart;
use crate::commissioning::{CommissioningState, CompletionStatus};
use crate::device::{ComObjectInstance, DeviceInstance};
use crate::flags::{Direction, GroupLink, ResolvedFlags};
use crate::group::{GroupAddressEntry, GroupRange};
use crate::ids::{
    ComObjectInstanceId, DeviceId, GroupAddressId, InstallationId, ParameterInstanceId, SourceRef,
};
use crate::installation::Installation;
use crate::parameter::ParameterInstance;
use crate::provenance::{Layer, Override, Resolved};
use crate::topology::Topology;
use crate::{GroupAddress, IndividualAddress, Language, Project};

pub(crate) const D1: DeviceId = DeviceId(1);
pub(crate) const D2: DeviceId = DeviceId(2);
/// Device 1's object, linked to GA 1 (send).
pub(crate) const CO1: ComObjectInstanceId = ComObjectInstanceId(11);
/// Device 2's object, linked to GA 1 (receive).
pub(crate) const CO2: ComObjectInstanceId = ComObjectInstanceId(21);
pub(crate) const GA1: GroupAddressId = GroupAddressId(1);
pub(crate) const GA2: GroupAddressId = GroupAddressId(2);

fn source(id: &str) -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: id.into(),
    }
}

fn ga(id: GroupAddressId, raw: u16) -> GroupAddressEntry {
    GroupAddressEntry {
        id,
        source: source("ga"),
        name: format!("GA {raw}"),
        address: GroupAddress::from_raw(raw),
        central: false,
        unfiltered: false,
        range: None,
        declared_dpt: Override::Absent,
    }
}

fn com_object(id: ComObjectInstanceId, device: DeviceId, link: Direction) -> ComObjectInstance {
    ComObjectInstance {
        id,
        source: source("co"),
        device,
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags {
            write: Override::Value(Resolved {
                value: true,
                layer: Layer::Program,
            }),
            ..ResolvedFlags::default()
        },
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga: GA1,
            direction: link,
        }],
        module_instance: None,
    }
}

/// Every one of the four parts programmed on both devices.
pub(crate) fn programmed() -> CommissioningState {
    CommissioningState {
        individual_address_loaded: true,
        application_program_loaded: true,
        parameters_loaded: true,
        communication_part_loaded: true,
        ..CommissioningState::default()
    }
}

fn device(id: DeviceId, octet: u8, com_object: ComObjectInstanceId) -> DeviceInstance {
    DeviceInstance {
        id,
        source: source("d"),
        name: format!("D{}", id.0),
        description: None,
        address: Some(IndividualAddress::new(1, 1, octet).unwrap()),
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: programmed(),
        visibility_calculated: true,
        com_objects: vec![com_object],
        binary_data: vec![],
    }
}

/// Two programmed devices sharing GA 1, one parameter each, GA 2 unused.
pub(crate) fn project() -> Project {
    let mut p = Project::new(Language("en".into()));
    p.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![D1, D2],
        },
        buildings: Vec::<BuildingPart>::new(),
        group_ranges: Vec::<GroupRange>::new(),
        group_addresses: vec![ga(GA1, 0x0801), ga(GA2, 0x0802)],
        parameters: vec![
            ParameterInstance {
                id: ParameterInstanceId(1),
                device: D1,
                source: source("P-1"),
                raw: "1".into(),
            },
            ParameterInstance {
                id: ParameterInstanceId(2),
                device: D2,
                source: source("P-1"),
                raw: "1".into(),
            },
        ],
    });
    p.devices.insert(device(D1, 1, CO1));
    p.devices.insert(device(D2, 2, CO2));
    p.devices
        .insert_com_object(com_object(CO1, D1, Direction::Send));
    p.devices
        .insert_com_object(com_object(CO2, D2, Direction::Receive));
    p
}
