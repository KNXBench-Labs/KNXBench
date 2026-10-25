//! Region reads keep one memory service for the whole region and fit every request on the wire.

use super::*;
use knx_core::commissioning::memory::MemoryService;
use knx_core::commissioning::mutation::TargetKind;
use tokio::sync::broadcast;

use crate::cemi::{
    encode_l_data, ApplicationService, Destination, LDataFrame, LDataMessageKind, Tpci,
};
use crate::client::{BusError, TunnelEvent};

use MemoryService::{Memory, UserMemory};

/// Forwards to the simulator, but first runs the real cEMI encoder over every
/// request, as `TunnelClient::send_frame` does; the simulator alone never encodes.
struct Encoding<'a>(&'a SimulatedDevice);

impl ManagementTransport for Encoding<'_> {
    fn assigned_address(&self) -> knx_core::IndividualAddress {
        ManagementTransport::assigned_address(self.0)
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.0)
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: knx_core::IndividualAddress::from_raw(0),
            destination,
            transport,
            service: service.clone(),
            control: None,
        };
        encode_l_data(&frame).map_err(|e| BusError::Protocol(e.to_string()))?;
        ManagementTransport::send_frame(self.0, destination, transport, service).await
    }

    fn target_kind(&self) -> TargetKind {
        ManagementTransport::target_kind(self.0)
    }
}

/// A device with the given APDU length: 254 negotiates 63 octets per read, 15 negotiates 12.
fn device_with_apdu(apdu: u16) -> SimulatedDevice {
    let device = device(SimulatorConfig {
        max_apdu_length: Some(apdu),
        ..SimulatorConfig::default()
    });
    device.preset_property(3, PID_PROGRAM_VERSION, &IDENTITY);
    device
}

/// Distinct, never all-equal octets for `length` addresses from `base`.
fn pattern(base: u32, length: usize) -> Vec<u8> {
    (0..length as u32)
        .map(|i| ((base + i).wrapping_mul(37) % 251) as u8 | 0x01)
        .collect()
}

/// Every read the device saw, in order, with the service that carried it.
fn read_schedule(device: &SimulatedDevice) -> Vec<(u32, u8, MemoryService)> {
    device
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            Seen::MemoryRead {
                address,
                number,
                service,
            } => Some((address, number, service)),
            _ => None,
        })
        .collect()
}

fn write_schedule(device: &SimulatedDevice) -> Vec<(u32, usize, MemoryService)> {
    device
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            Seen::MemoryWrite {
                address,
                data,
                service,
            } => Some((address, data.len(), service)),
            _ => None,
        })
        .collect()
}

/// A synthetic plan (not a product build) that identifies the application and
/// writes `octets` at `address`.
fn writing(address: u16, octets: Vec<u8>) -> MemoryDownloadPlan {
    MemoryDownloadPlan {
        mask: MaskVersion(0x0701),
        manufacturer: MANUFACTURER,
        steps: vec![
            MemoryDownloadStep::Connect,
            MemoryDownloadStep::CompareProperty {
                object_index: 3,
                property_id: PID_PROGRAM_VERSION,
                inline_data: IDENTITY.to_vec(),
            },
            MemoryDownloadStep::WriteMemory { address, octets },
            MemoryDownloadStep::Disconnect,
        ],
    }
}

async fn probe(
    device: &SimulatedDevice,
    runs: &[(u32, usize)],
) -> Result<Vec<(u32, Vec<u8>)>, ReadBackError> {
    read_memory_runs(
        &Encoding(device),
        contactable(device),
        fast(),
        MaskVersion(0x0701),
        MANUFACTURER,
        runs,
    )
    .await
}

fn user(address: u32, number: u8) -> (u32, u8, MemoryService) {
    (address, number, UserMemory)
}

fn standard(address: u32, number: u8) -> (u32, u8, MemoryService) {
    (address, number, Memory)
}

#[tokio::test]
async fn compare_reads_a_region_that_ends_at_the_service_limit_in_user_memory_chunks_of_fifteen() {
    let device = device_with_apdu(254);
    let held = pattern(0xFFF0, 16);
    device.preset_memory(0xFFF0, &held);
    let plan = writing(0xFFF0, vec![0xEE; 16]);
    let compared = compare_with_plan(&Encoding(&device), contactable(&device), fast(), &plan)
        .await
        .expect("a 16-octet region fits fifteen plus one");
    assert_eq!(compared.held.regions.len(), 1);
    assert_eq!(compared.held.regions[0].address, 0xFFF0);
    assert_eq!(compared.held.regions[0].octets, held);
    assert_eq!(
        read_schedule(&device),
        vec![user(0xFFF0, 15), user(0xFFFF, 1)]
    );
    assert_eq!(writes_of_any_kind(&device), vec![], "read only");
    assert_eq!(device.seen().last(), Some(&Seen::Disconnect));
}

#[tokio::test]
async fn compare_at_the_default_read_size_keeps_the_region_in_user_memory() {
    let device = device_with_apdu(15);
    let held = pattern(0xFFF0, 16);
    device.preset_memory(0xFFF0, &held);
    let plan = writing(0xFFF0, vec![0xEE; 16]);
    let compared = compare_with_plan(&Encoding(&device), contactable(&device), fast(), &plan)
        .await
        .expect("compared");
    assert_eq!(compared.held.regions[0].octets, held);
    assert_eq!(
        read_schedule(&device),
        vec![user(0xFFF0, 12), user(0xFFFC, 4)]
    );
    assert_eq!(writes_of_any_kind(&device), vec![], "read only");
}

#[tokio::test]
async fn a_region_ending_exactly_at_ffff_is_user_memory_all_the_way() {
    let wide = device_with_apdu(254);
    let held = pattern(0xFFC0, 63);
    wide.preset_memory(0xFFC0, &held);
    let read = probe(&wide, &[(0xFFC0, 63)]).await.expect("read");
    assert_eq!(read, vec![(0xFFC0, held.clone())]);
    assert_eq!(
        read_schedule(&wide),
        vec![
            user(0xFFC0, 15),
            user(0xFFCF, 15),
            user(0xFFDE, 15),
            user(0xFFED, 15),
            user(0xFFFC, 3)
        ]
    );
    assert_eq!(writes_of_any_kind(&wide), vec![], "read only");

    let narrow = device_with_apdu(15);
    narrow.preset_memory(0xFFC0, &held);
    let read = probe(&narrow, &[(0xFFC0, 63)]).await.expect("read");
    assert_eq!(read, vec![(0xFFC0, held)]);
    assert_eq!(
        read_schedule(&narrow),
        vec![
            user(0xFFC0, 12),
            user(0xFFCC, 12),
            user(0xFFD8, 12),
            user(0xFFE4, 12),
            user(0xFFF0, 12),
            user(0xFFFC, 3)
        ]
    );
}

#[tokio::test]
async fn a_region_ending_at_fffe_stays_in_the_memory_service() {
    let wide = device_with_apdu(254);
    let held = pattern(0xFFC0, 62);
    wide.preset_memory(0xFFC0, &held);
    let read = probe(&wide, &[(0xFFC0, 62)]).await.expect("read");
    assert_eq!(read, vec![(0xFFC0, held.clone())]);
    assert_eq!(read_schedule(&wide), vec![standard(0xFFC0, 62)]);

    let narrow = device_with_apdu(15);
    narrow.preset_memory(0xFFC0, &held);
    let read = probe(&narrow, &[(0xFFC0, 62)]).await.expect("read");
    assert_eq!(read, vec![(0xFFC0, held)]);
    assert_eq!(
        read_schedule(&narrow),
        vec![
            standard(0xFFC0, 12),
            standard(0xFFCC, 12),
            standard(0xFFD8, 12),
            standard(0xFFE4, 12),
            standard(0xFFF0, 12),
            standard(0xFFFC, 2)
        ]
    );
}

#[tokio::test]
async fn a_region_below_the_limit_reads_in_full_sixty_three_and_twelve_octet_reads() {
    let wide = device_with_apdu(254);
    let held = pattern(0x4000, 130);
    wide.preset_memory(0x4000, &held);
    let read = probe(&wide, &[(0x4000, 130)]).await.expect("read");
    assert_eq!(read, vec![(0x4000, held.clone())]);
    assert_eq!(
        read_schedule(&wide),
        vec![
            standard(0x4000, 63),
            standard(0x403F, 63),
            standard(0x407E, 4)
        ]
    );

    let narrow = device_with_apdu(15);
    narrow.preset_memory(0x4000, &held[..30]);
    let read = probe(&narrow, &[(0x4000, 30)]).await.expect("read");
    assert_eq!(read, vec![(0x4000, held[..30].to_vec())]);
    assert_eq!(
        read_schedule(&narrow),
        vec![
            standard(0x4000, 12),
            standard(0x400C, 12),
            standard(0x4018, 6)
        ]
    );
}

#[tokio::test]
async fn a_region_past_ffff_reads_in_user_memory_and_a_single_octet_at_ffff_too() {
    let wide = device_with_apdu(254);
    let held = pattern(0xFFF8, 16);
    wide.preset_memory(0xFFF8, &held);
    let read = probe(&wide, &[(0xFFF8, 16), (0xFFFF, 1), (0x10000, 4)])
        .await
        .expect("read");
    assert_eq!(
        read,
        vec![
            (0xFFF8, held.clone()),
            (0xFFFF, held[7..8].to_vec()),
            (0x10000, held[8..12].to_vec())
        ]
    );
    assert_eq!(
        read_schedule(&wide),
        vec![
            user(0xFFF8, 15),
            user(0x10007, 1),
            user(0xFFFF, 1),
            user(0x10000, 4)
        ]
    );

    let narrow = device_with_apdu(15);
    narrow.preset_memory(0xFFF8, &held);
    let read = probe(&narrow, &[(0xFFF8, 16)]).await.expect("read");
    assert_eq!(read, vec![(0xFFF8, held)]);
    assert_eq!(
        read_schedule(&narrow),
        vec![user(0xFFF8, 12), user(0x10004, 4)]
    );
}

#[tokio::test]
async fn an_empty_run_reads_nothing() {
    let device = device_with_apdu(254);
    let read = probe(&device, &[(0xFFF0, 0)]).await.expect("read");
    assert_eq!(read, vec![(0xFFF0, vec![])]);
    assert_eq!(read_schedule(&device), vec![]);
}

#[tokio::test]
async fn a_short_answer_in_the_user_memory_chunks_is_a_structured_short_read() {
    let device = device(SimulatorConfig {
        max_apdu_length: Some(254),
        short_memory_answers: true,
        ..SimulatorConfig::default()
    });
    device.preset_memory(0xFFF0, &pattern(0xFFF0, 16));
    let error = probe(&device, &[(0xFFF0, 16)]).await.expect_err("short");
    assert!(
        matches!(
            error,
            ReadBackError::ShortRead {
                address: 0xFFF0,
                asked: 15,
                got: 14
            }
        ),
        "{error}"
    );
    assert_eq!(read_schedule(&device), vec![user(0xFFF0, 15)]);
    assert_eq!(writes_of_any_kind(&device), vec![], "read only");
}

#[tokio::test]
async fn a_refused_user_memory_read_is_a_structured_session_error() {
    let device = device(SimulatorConfig {
        max_apdu_length: Some(254),
        protected_memory: Some((0xFFF0, 0x1_0000)),
        ..SimulatorConfig::default()
    });
    let error = probe(&device, &[(0xFFF0, 16)]).await.expect_err("refused");
    assert!(
        matches!(
            error,
            ReadBackError::Session(SessionError::MemoryRefused { address: 0xFFF0 })
        ),
        "{error}"
    );
    assert_eq!(read_schedule(&device), vec![user(0xFFF0, 15)]);
}

#[tokio::test]
async fn the_backup_of_a_boundary_region_is_kept_before_any_write_and_the_download_runs() {
    let device = device_with_apdu(254);
    let original = pattern(0xFFF0, 16);
    device.preset_memory(0xFFF0, &original);
    let new = vec![0xA5; 16];
    let plan = writing(0xFFF0, new.clone());
    let transport = Encoding(&device);
    let authorisation =
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).expect("ok");
    let mut session =
        ManagementSession::authorised(&transport, AuthorisationPlan::Skip, fast(), authorisation)
            .expect("a simulator session");

    let mut before_keep = None;
    let mut kept = None;
    let report = run_memory_download_with_backup(
        &mut session,
        &plan,
        |_| {},
        &mut |backup: &DeviceBackup| {
            before_keep = Some((write_schedule(&device), read_schedule(&device)));
            kept = Some(backup.clone());
            Ok(())
        },
    )
    .await
    .expect("the download runs after the backup");

    let (writes, reads) = before_keep.expect("keep was called");
    assert_eq!(writes, vec![], "no write before the backup was kept");
    assert_eq!(reads, vec![user(0xFFF0, 15), user(0xFFFF, 1)]);
    let backup = kept.expect("kept");
    assert_eq!(backup.regions.len(), 1);
    assert_eq!(backup.regions[0].address, 0xFFF0);
    assert_eq!(backup.regions[0].octets, original);
    assert_eq!(report.backup.as_ref(), Some(&backup));
    // The unchanged executor writes the region in the same two chunks.
    assert_eq!(
        write_schedule(&device),
        vec![(0xFFF0, 15, UserMemory), (0xFFFF, 1, UserMemory),]
    );
    assert_eq!(
        device.memory(0xFFF0, 16),
        new.iter().map(|b| Some(*b)).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn a_backup_that_cannot_be_kept_stops_a_boundary_download_before_any_write() {
    let device = device_with_apdu(254);
    device.preset_memory(0xFFF0, &pattern(0xFFF0, 16));
    let plan = writing(0xFFF0, vec![0xA5; 16]);
    let transport = Encoding(&device);
    let authorisation =
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).expect("ok");
    let mut session =
        ManagementSession::authorised(&transport, AuthorisationPlan::Skip, fast(), authorisation)
            .expect("a simulator session");

    let error =
        run_memory_download_with_backup(&mut session, &plan, |_| {}, &mut |_: &DeviceBackup| {
            Err("disk full".to_string())
        })
        .await
        .expect_err("the keeper failed");

    assert!(
        matches!(error, MemoryDownloadError::BackupNotKept(ref why) if why == "disk full"),
        "{error}"
    );
    assert_eq!(
        read_schedule(&device),
        vec![user(0xFFF0, 15), user(0xFFFF, 1)]
    );
    assert_eq!(writes_of_any_kind(&device), vec![], "nothing mutated");
    assert_eq!(
        device.memory(0xFFF0, 16),
        pattern(0xFFF0, 16)
            .into_iter()
            .map(Some)
            .collect::<Vec<_>>()
    );
}
