//! Exercise inactive-byte selection through imported product data and the real CLI writer.

use std::sync::Arc;
use std::time::Duration;

use knx_app::access_key::{download_keying, KeySource};
use knx_app::commissioning_activity::{DownloadResult, OneShotLog, Restart};
use knx_app::device_backup::read_backup;
use knx_app::download_scope::{prepare_scoped_download, ScopeRequest};
use knx_core::commissioning::device_backup::{ExemplarEvidence, PriorApplication};
use knx_core::commissioning::load_control_memory::MemoryLoadStateMachine;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::memory::MemoryService;
use knx_core::commissioning::memory_download::ExemplarSerial;
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
const OLD_APPLICATION: [u8; 6] = [0xD3, 0xD4, 0xD5, 0xD6, 0xD7, 0xD8];

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

/// No plan editing: public preparation feeds the CLI's actual regular executor unchanged.
fn run_source(default_mode: u8) -> SimulatedDevice {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", source(default_mode).as_bytes())
        .expect("the synthetic program imports through the real product parser");
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes())
        .expect("the hardware-to-program identity imports");
    let scoped = prepare_scoped_download(
        &products,
        &project(),
        TARGET.parse().unwrap(),
        ScopeRequest::Complete,
    )
    .expect("valid kind, no plug-in or CustomerAdjustable declaration, and a plannable image");
    let prepared = scoped.prepared.expect("Complete prepares a real download");
    assert_eq!(prepared.request.program_id, PROGRAM_ID);
    assert!(
        prepared.request.values.is_empty(),
        "source defaults select the branch"
    );
    assert_eq!(prepared.plan.mask.0, 0x0701);
    assert_eq!(prepared.plan.manufacturer, 0xFFF6);
    assert!(
        prepared.partial.is_none(),
        "this remains a full load procedure"
    );

    let device = SimulatedDevice::with_config_at(
        prepared.target,
        SimulatorConfig {
            mask_version: 0x0701,
            serial_number: Some(SERIAL),
            ..SimulatorConfig::default()
        },
    );
    assert_eq!(device.address(), prepared.target);
    device.preset_property(0, PID_MANUFACTURER_ID, &[0xFF, 0xF6]);
    // Explicit precondition: the prior application already has this identity.
    // The simulator does not infer it from the task record after writing.
    device.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
    for machine in MemoryLoadStateMachine::ALL {
        device.preset_load_state(ObjectIndex::new(machine.type_number()), LoadState::Unloaded);
    }
    device.preset_memory(0x4000, &[0xB4; 8]);
    device.preset_memory(0x4400, &OLD_APPLICATION);
    let log = Arc::new(OneShotLog::for_run(dir.path().join("history.sqlite")).unwrap());
    let mut activity = log.start_download(1, device.address()).unwrap();
    let backups = dir.path().join("backups");
    let backup = BackupTarget {
        dir: &backups,
        application: PROGRAM_ID,
        write_selection: prepared.write_selection,
        partial: None,
        taken: "2026-10-30T06:00:00Z".into(),
    };
    let mut out = Vec::new();
    let result = tokio::runtime::Builder::new_current_thread()
        .enable_time()
        .build()
        .unwrap()
        .block_on(execute_regular_download(
            &device,
            WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).unwrap(),
            download_keying(prepared.plan.mask, None, KeySource::None),
            fast(),
            DownloadRun {
                plan: &prepared.plan,
                backup: &backup,
            },
            &mut activity,
            &mut out,
        ));
    let exit = finish_regular_download(&result, true, activity, None, &mut out);
    let out = String::from_utf8(out).unwrap();
    assert_eq!(
        result.result,
        DownloadResult::Finished {
            written: Written::Yes,
            restart: Restart::Acknowledged,
        },
        "writer setup must succeed before the selection assertion: {out}"
    );
    assert_eq!(exit, std::process::ExitCode::SUCCESS, "{out}");
    assert!(result.recorded && result.check_recorded, "{out}");

    let files: Vec<_> = std::fs::read_dir(&backups).unwrap().collect();
    assert_eq!(
        files.len(),
        1,
        "one fresh backup, not a fabricated prestate"
    );
    let stored = read_backup(&files[0].as_ref().unwrap().path()).unwrap();
    assert_eq!(stored.application, PROGRAM_ID);
    assert_eq!(stored.backup.target, device.address());
    assert_eq!(
        stored.backup.prior_application,
        PriorApplication::Recorded(APPLICATION)
    );
    assert_eq!(
        stored.backup.exemplar_serial,
        ExemplarEvidence::Recorded(ExemplarSerial::new(SERIAL))
    );
    assert_eq!(stored.backup.load_states.len(), 2);
    assert!(stored
        .backup
        .load_states
        .iter()
        .all(|(_, state)| *state == LoadState::Unloaded));
    assert!(!stored.backup.regions.is_empty());
    for region in &stored.backup.regions {
        let start = usize::from(region.address);
        let expected = match region.address {
            0x4000 => &[0xB4; 8][..],
            0x4400..=0x4405 => {
                let offset = start - 0x4400;
                &OLD_APPLICATION[offset..offset + region.octets.len()]
            }
            other => panic!("unexpected backup region {other:04X}h"),
        };
        assert_eq!(region.octets, expected, "fresh pre-write observation");
    }
    assert!(out.find("backup kept in ").unwrap() < out.find("-> 4000h").unwrap());

    // These are 0104h load controls, not application data writes.
    let controls: Vec<_> = device
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            Seen::MemoryWrite {
                address: 0x0104,
                data,
                ..
            } => Some(data),
            _ => None,
        })
        .collect();
    assert_eq!(
        controls,
        vec![
            vec![0x14, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0x34, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0x11, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0x13, 0, 0, 0x40, 0, 0, 8, 0xFF, 3, 0x80, 0],
            vec![0x12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0x31, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
            vec![0x33, 0, 0, 0x44, 0, 0, 6, 0xFF, 3, 0x80, 0],
            vec![0x33, 2, 0, 0x44, 0, 0, 0xFF, 0xF6, 0, 1, 1],
            vec![0x32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        ],
        "the complete source procedure still reaches the real writer"
    );
    let table_writes: Vec<_> = device
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            Seen::MemoryWrite {
                address,
                data,
                service,
            } if address < 0x4008 && address + data.len() as u32 > 0x4000 => {
                assert_eq!(service, MemoryService::Memory);
                Some((address, data))
            }
            _ => None,
        })
        .collect();
    assert_eq!(table_writes, vec![(0x4000, vec![0; 8])]);
    assert_eq!(device.memory(0x4000, 8), vec![Some(0); 8]);
    device
}

fn application_writes(device: &SimulatedDevice) -> Vec<(u32, Vec<u8>)> {
    device
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            Seen::MemoryWrite {
                address,
                data,
                service,
            } if address < 0x4406 && address + data.len() as u32 > 0x4400 => {
                assert_eq!(service, MemoryService::Memory);
                Some((address, data))
            }
            _ => None,
        })
        .collect()
}

#[derive(Debug, PartialEq, Eq)]
enum ApplicationIo {
    Read(u32, u8),
    Write(u32, Vec<u8>),
}

fn application_io(device: &SimulatedDevice) -> Vec<ApplicationIo> {
    device
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            Seen::MemoryRead {
                address,
                number,
                service,
            } if address < 0x4406 && address + u32::from(number) > 0x4400 => {
                assert_eq!(service, MemoryService::Memory);
                Some(ApplicationIo::Read(address, number))
            }
            Seen::MemoryWrite { address, data, .. }
                if address < 0x4406 && address + data.len() as u32 > 0x4400 =>
            {
                Some(ApplicationIo::Write(address, data))
            }
            _ => None,
        })
        .collect()
}

#[test]
fn source_default_zero_control_reaches_the_real_writer() {
    let device = run_source(0);
    assert_eq!(
        application_writes(&device),
        vec![(0x4400, vec![0x00, 0xE2, 0xC7, 0xA6]), (0x4405, vec![0x68]),]
    );
    use ApplicationIo::{Read, Write};
    assert_eq!(
        application_io(&device),
        vec![
            Read(0x4400, 4),
            Read(0x4405, 1),
            Write(0x4400, vec![0x00, 0xE2, 0xC7, 0xA6]),
            Read(0x4400, 4),
            Write(0x4405, vec![0x68]),
            Read(0x4405, 1),
        ],
        "backup reads precede mutation; every data frame is read back"
    );
    assert_eq!(
        device.memory(0x4400, 4),
        vec![Some(0x00), Some(0xE2), Some(0xC7), Some(0xA6)]
    );
    assert_eq!(device.memory(0x4405, 1), vec![Some(0x68)]);
}

#[test]
fn source_default_one_omits_exclusive_inactive_byte_at_the_real_writer() {
    let device = run_source(1);
    // Establish active/literal landing before the intended omission assertion.
    assert_eq!(device.memory(0x4400, 1), vec![Some(0x01)]);
    assert_eq!(device.memory(0x4402, 2), vec![Some(0xC7), Some(0x53)]);
    assert_eq!(device.memory(0x4405, 1), vec![Some(0x68)]);
    assert_eq!(application_writes(&device), vec![
        (0x4400, vec![0x01]),
        (0x4402, vec![0xC7, 0x53]),
        (0x4405, vec![0x68]),
    ], "4401h is exclusively inactive; 4404h is source-blocked; active Union U1 and literal C7 must still write");
    use ApplicationIo::{Read, Write};
    assert_eq!(
        application_io(&device),
        vec![
            Read(0x4400, 1),
            Read(0x4402, 2),
            Read(0x4405, 1),
            Write(0x4400, vec![0x01]),
            Read(0x4400, 1),
            Write(0x4402, vec![0xC7, 0x53]),
            Read(0x4402, 2),
            Write(0x4405, vec![0x68]),
            Read(0x4405, 1),
        ],
        "neither backup nor writer readback may bridge the omitted bytes"
    );
    // No assertion about 4401h/4404h storage surviving Unload/Allocation/Restart:
    // this simulator does not model those 0701 lifecycle side effects.
}
