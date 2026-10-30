//! Share independent synthetic fixtures for writer selection and versioned capture acceptance.

use std::sync::Arc;
use std::time::Duration;

use knx_app::access_key::{download_keying, KeySource};
use knx_app::commissioning_activity::{DownloadResult, OneShotLog};

use knx_app::download_scope::{prepare_scoped_download, ScopeRequest};

use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::memory::MemoryService;

use knx_core::commissioning::properties::{ObjectIndex, PID_MANUFACTURER_ID, PID_PROGRAM_VERSION};
use knx_core::{
    CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_testsupport::vendor_plugin::{
    HARDWARE2PROGRAM_REF, HARDWARE_XML, PRODUCT_REF, PROGRAM_ID, TARGET,
};

use super::*;

const APPLICATION: [u8; 5] = [0xFF, 0xF6, 0x00, 0x01, 0x01];
const SERIAL: [u8; 6] = [0xFF, 0xF6, 0x12, 0x34, 0x56, 0x78];

/// The existing synthetic 0701 ProductProcedure, with a six-byte application allocation.
/// Mode is a source default, not a branch override on a different default topology.
fn source(default_mode: u8) -> String {
    assert!(default_mode <= 1);
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-FFF6"><ApplicationPrograms>
<ApplicationProgram Id="{PROGRAM_ID}" Name="Synthetic inactive-byte writer" ApplicationNumber="1" ApplicationVersion="1" PeiType="0"
  MaskVersion="MV-0701" LoadProcedureStyle="ProductProcedure"><Static>
<Code>
  <AbsoluteSegment Id="{PROGRAM_ID}_AS-4000" Address="16384" Size="8"><Data>AAAAAAAAAAA=</Data></AbsoluteSegment>
  <AbsoluteSegment Id="{PROGRAM_ID}_AS-4400" Address="17408" Size="6"><Data>AZrHRG0i</Data><Mask>/////wD/</Mask></AbsoluteSegment>
</Code>
<AddressTable CodeSegment="{PROGRAM_ID}_AS-4000" Offset="0" MaxEntries="2" />
<ParameterTypes>
  <ParameterType Id="{PROGRAM_ID}_PT-Mode" Name="Mode"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="1" /></ParameterType>
  <ParameterType Id="{PROGRAM_ID}_PT-Octet" Name="Octet"><TypeNumber SizeInBit="8" Type="unsignedInt" minInclusive="0" maxInclusive="255" /></ParameterType>
</ParameterTypes>
<Parameters>
  <Parameter Id="{PROGRAM_ID}_P-Mode" Name="Mode" ParameterType="{PROGRAM_ID}_PT-Mode" Value="{default_mode}"><Memory CodeSegment="{PROGRAM_ID}_AS-4400" Offset="0" BitOffset="0" /></Parameter>
  <Parameter Id="{PROGRAM_ID}_P-I" Name="Exclusive mode zero" ParameterType="{PROGRAM_ID}_PT-Octet" Value="226"><Memory CodeSegment="{PROGRAM_ID}_AS-4400" Offset="1" BitOffset="0" /></Parameter>
  <Union SizeInBit="8"><Memory CodeSegment="{PROGRAM_ID}_AS-4400" Offset="3" BitOffset="0" />
    <Parameter Id="{PROGRAM_ID}_P-U0" Name="Union zero" ParameterType="{PROGRAM_ID}_PT-Octet" Value="166" Offset="0" BitOffset="0" />
    <Parameter Id="{PROGRAM_ID}_P-U1" Name="Union one" ParameterType="{PROGRAM_ID}_PT-Octet" Value="83" Offset="0" BitOffset="0" />
  </Union>
  <Parameter Id="{PROGRAM_ID}_P-Tail" Name="Tail" ParameterType="{PROGRAM_ID}_PT-Octet" Value="104"><Memory CodeSegment="{PROGRAM_ID}_AS-4400" Offset="5" BitOffset="0" /></Parameter>
</Parameters>
<ParameterRefs>
  <ParameterRef Id="{PROGRAM_ID}_P-Mode_R-Mode" RefId="{PROGRAM_ID}_P-Mode" />
  <ParameterRef Id="{PROGRAM_ID}_P-I_R-I" RefId="{PROGRAM_ID}_P-I" />
  <ParameterRef Id="{PROGRAM_ID}_P-U0_R-U0" RefId="{PROGRAM_ID}_P-U0" />
  <ParameterRef Id="{PROGRAM_ID}_P-U1_R-U1" RefId="{PROGRAM_ID}_P-U1" />
  <ParameterRef Id="{PROGRAM_ID}_P-Tail_R-Tail" RefId="{PROGRAM_ID}_P-Tail" />
</ParameterRefs>
<LoadProcedures><LoadProcedure>
  <LdCtrlConnect /><LdCtrlUnload LsmIdx="1" /><LdCtrlUnload LsmIdx="3" />
  <LdCtrlLoad LsmIdx="1" />
  <LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="16384" Size="8" Access="255" MemType="3" SegFlags="128" />
  <LdCtrlLoadCompleted LsmIdx="1" />
  <LdCtrlLoad LsmIdx="3" />
  <LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="17408" Size="6" Access="255" MemType="3" SegFlags="128" />
  <LdCtrlTaskSegment LsmIdx="3" Address="17408" />
  <LdCtrlLoadCompleted LsmIdx="3" />
  <LdCtrlRestart /><LdCtrlDisconnect />
</LoadProcedure></LoadProcedures>
</Static><Dynamic><Channel Id="{PROGRAM_ID}_CH-1" Text="Synthetic"><ParameterBlock Id="{PROGRAM_ID}_PB-1" Text="Selection">
  <ParameterRefRef RefId="{PROGRAM_ID}_P-Mode_R-Mode" />
  <choose ParamRefId="{PROGRAM_ID}_P-Mode_R-Mode">
    <when test="0"><ParameterRefRef RefId="{PROGRAM_ID}_P-I_R-I" /><ParameterRefRef RefId="{PROGRAM_ID}_P-U0_R-U0" /></when>
    <when test="1"><ParameterRefRef RefId="{PROGRAM_ID}_P-U1_R-U1" /></when>
  </choose>
  <ParameterRefRef RefId="{PROGRAM_ID}_P-Tail_R-Tail" />
</ParameterBlock></Channel></Dynamic></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Synthetic installation".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-1".into(),
        },
        name: "Synthetic inactive-byte writer".into(),
        description: None,
        address: Some(TARGET.parse().unwrap()),
        product_ref: PRODUCT_REF.into(),
        program_ref: HARDWARE2PROGRAM_REF.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

fn fast() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(50),
        response_timeout: Duration::from_millis(50),
        answer_timeout: None,
        poll_interval: Duration::from_millis(1),
        max_transition: Duration::from_millis(40),
        programming_delay: Duration::ZERO,
        restart_basic_t1: Duration::from_millis(1),
        restart_responsive_again: Duration::from_millis(5),
        post_restart_disconnect_wait: Duration::from_millis(5),
        programming_mode_broadcast_timeout: Duration::from_millis(20),
    }
}

#[path = "device_download_inactive_capture_tests.rs"]
mod capture_tests;
#[path = "device_download_inactive_policy_tests.rs"]
mod policy_tests;

fn application_writes(device: &SimulatedDevice) -> Vec<(u32, Vec<u8>)> {
    device
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            Seen::MemoryWrite {
                address,
                data,
                service,
            } if (0x4400..0x4406).contains(&address) => {
                assert_eq!(service, MemoryService::Memory);
                Some((address, data))
            }
            _ => None,
        })
        .collect()
}
