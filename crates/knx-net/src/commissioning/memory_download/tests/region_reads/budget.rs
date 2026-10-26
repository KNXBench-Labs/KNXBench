//! A transport that does not say it is a simulator is held to the standard frame; the wire agrees.

use super::*;

mod note;

/// What every real transport looks like: `target_kind` is the trait default, `Hardware`.
struct HardwareLike<'a>(&'a SimulatedDevice);

impl ManagementTransport for HardwareLike<'_> {
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
        let wire = encode_l_data(&frame).map_err(|e| BusError::Protocol(e.to_string()))?;
        // cEMI L_Data: the length octet follows code, add-info length, controls and addresses.
        assert!(
            wire[8] <= 15,
            "a standard frame carries at most 15: {service:?}"
        );
        ManagementTransport::send_frame(self.0, destination, transport, service).await
    }
}

/// Original semantic RED: the synthetic device reports 254, the unproved route used to follow it.
#[tokio::test]
async fn a_hardware_kind_session_over_a_wide_device_negotiates_the_standard_frame() {
    let device = device_with_apdu(254);
    let wire = HardwareLike(&device);
    let mut session =
        ManagementSession::read_only(&wire, device.address(), AuthorisationPlan::Skip, fast())
            .expect("read only");
    session.adopt_mask(MaskVersion(0x0701));
    session.connect().await.expect("connect");
    let limit = session.write_limit(None).await.expect("limit");
    session.disconnect().await;
    assert_eq!(limit.max_octets(), 12);
}

#[tokio::test]
async fn a_hardware_kind_read_of_a_wide_device_uses_twelve_and_eleven_octet_reads() {
    let device = device_with_apdu(254);
    let held = pattern(0x4000, 30);
    device.preset_memory(0x4000, &held);
    let user_held = pattern(0xFFF0, 16);
    device.preset_memory(0xFFF0, &user_held);
    let read = read_memory_runs(
        &HardwareLike(&device),
        contactable(&device),
        fast(),
        MaskVersion(0x0701),
        MANUFACTURER,
        &[(0x4000, 30), (0xFFF0, 16)],
    )
    .await
    .expect("read");
    assert_eq!(read, vec![(0x4000, held), (0xFFF0, user_held)]);
    assert_eq!(
        read_schedule(&device),
        vec![
            standard(0x4000, 12),
            standard(0x400C, 12),
            standard(0x4018, 6),
            user(0xFFF0, 11),
            user(0xFFFB, 5)
        ]
    );
}

/// Encoded frame for one write service from a fixed source, over the public encoder.
fn encoded(service: ApplicationService) -> Vec<u8> {
    encode_l_data(&LDataFrame {
        kind: LDataMessageKind::Request,
        source: knx_core::IndividualAddress::from_raw(0),
        destination: Destination::Individual(knx_core::IndividualAddress::from_raw(0x1101)),
        transport: Tpci::UnnumberedData,
        service,
        control: None,
    })
    .expect("encodes")
}

/// Independent of the production budget helper: both services fill a 15-octet NPDU exactly.
#[test]
fn memory_with_twelve_and_user_memory_with_eleven_octets_both_encode_a_length_field_of_fifteen() {
    let data: Vec<u8> = (1..=12).collect();
    let memory = encoded(ApplicationService::MemoryWrite {
        address: 0x4000,
        data: data.clone(),
    });
    let user = encoded(ApplicationService::UserMemoryWrite {
        address: 0xFFF0,
        data: data[..11].to_vec(),
    });
    // cEMI L_Data: code, add-info length, two control octets, source, destination, then length.
    assert_eq!(memory[8], 15);
    assert_eq!(user[8], 15);
    // A_Memory_Write 0x280 | number, 16-bit address, data.
    assert_eq!(memory[9..11], [0x02, 0x8C]);
    assert_eq!(memory[11..13], [0x40, 0x00]);
    assert_eq!(memory[13..], data[..]);
    // A_UserMemory_Write 0x2C2, then extension/number octet, 16 address bits, data.
    assert_eq!(user[9..11], [0x02, 0xC2]);
    assert_eq!(user[11..14], [0x0B, 0xFF, 0xF0]);
    assert_eq!(user[14..], data[..11]);
}

use knx_core::commissioning::memory::WriteLimitReason;
use knx_core::commissioning::properties::PID_MAX_APDU_LENGTH;

/// The session limit over `wire`, after a real connect and the property read.
async fn session_limit<T: ManagementTransport>(
    wire: &T,
    device: &SimulatedDevice,
) -> Result<WriteLimit, SessionError> {
    session_limit_with_router(wire, device, None).await
}

async fn session_limit_with_router<T: ManagementTransport>(
    wire: &T,
    device: &SimulatedDevice,
    router: Option<ObjectIndex>,
) -> Result<WriteLimit, SessionError> {
    let mut session =
        ManagementSession::read_only(wire, device.address(), AuthorisationPlan::Skip, fast())
            .expect("read only");
    session.adopt_mask(MaskVersion(0x0701));
    session.connect().await.expect("connect");
    let limit = session.write_limit(router).await;
    session.disconnect().await;
    limit
}

#[tokio::test]
async fn the_cap_names_itself_only_where_it_lowers_the_value() {
    for (apdu, reported, reason) in [
        (Some(15), 12, WriteLimitReason::FromDeviceObject),
        (Some(16), 12, WriteLimitReason::CappedToStandardFrame),
        (Some(17), 12, WriteLimitReason::CappedToStandardFrame),
        (Some(18), 12, WriteLimitReason::CappedToStandardFrame),
        (Some(19), 12, WriteLimitReason::CappedToStandardFrame),
        (Some(254), 12, WriteLimitReason::CappedToStandardFrame),
        (Some(255), 12, WriteLimitReason::EscapeCode),
        (Some(3), 12, WriteLimitReason::BelowStandardRange),
        (None, 12, WriteLimitReason::PropertyAbsent),
    ] {
        let device = device(SimulatorConfig {
            max_apdu_length: apdu,
            ..SimulatorConfig::default()
        });
        let limit = session_limit(&HardwareLike(&device), &device)
            .await
            .expect("limit");
        assert_eq!(
            (limit.max_octets(), limit.reason()),
            (reported, reason),
            "{apdu:?}"
        );
    }
}

fn apdu_length_reads(device: &SimulatedDevice) -> Vec<u8> {
    device
        .seen()
        .into_iter()
        .filter_map(|seen| match seen {
            Seen::PropertyRead {
                object_index,
                property_id: PID_MAX_APDU_LENGTH,
            } => Some(object_index),
            _ => None,
        })
        .collect()
}

#[tokio::test]
async fn a_router_only_length_is_read_from_the_modelled_router_object_and_not_followed() {
    let make = || {
        device(SimulatorConfig {
            max_apdu_length: None,
            router_max_apdu_length: Some(254),
            ..SimulatorConfig::default()
        })
    };
    // Control: without a router object the property is simply absent.
    let none = make();
    let limit = session_limit(&HardwareLike(&none), &none)
        .await
        .expect("limit");
    assert_eq!(
        (limit.max_octets(), limit.reason()),
        (12, WriteLimitReason::PropertyAbsent)
    );
    assert_eq!(apdu_length_reads(&none), vec![0]);
    // The simulator models its Router Object at index 6.
    let routed = make();
    let limit =
        session_limit_with_router(&HardwareLike(&routed), &routed, Some(ObjectIndex::new(6)))
            .await
            .expect("limit");
    assert_eq!(
        (limit.max_octets(), limit.reason()),
        (12, WriteLimitReason::RouterObjectOnly)
    );
    assert_eq!(apdu_length_reads(&routed), vec![0, 6]);
}

#[tokio::test]
async fn a_genuine_simulator_keeps_its_modeled_capacity() {
    for (apdu, octets, reason) in [
        (15, 12, WriteLimitReason::FromDeviceObject),
        (19, 16, WriteLimitReason::FromDeviceObject),
        (254, 63, WriteLimitReason::CappedByServiceLimit),
    ] {
        let device = device_with_apdu(apdu);
        let limit = session_limit(&Encoding(&device), &device)
            .await
            .expect("limit");
        assert_eq!((limit.max_octets(), limit.reason()), (octets, reason));
    }
}

#[tokio::test]
async fn a_malformed_apdu_length_reply_is_still_reported_over_the_cap() {
    let device = device_with_apdu(254);
    device.preset_property(0, PID_MAX_APDU_LENGTH, &[1, 2, 3]);
    let error = session_limit(&HardwareLike(&device), &device)
        .await
        .expect_err("malformed");
    assert!(
        matches!(error, SessionError::MalformedProperty { got: 3, .. }),
        "{error}"
    );
}

#[tokio::test]
async fn every_hardware_kind_value_reads_a_boundary_region_in_eleven_plus_five() {
    for apdu in [15u16, 16, 17, 18, 19, 254] {
        let device = device_with_apdu(apdu);
        let held = pattern(0xFFF0, 16);
        device.preset_memory(0xFFF0, &held);
        let read = read_memory_runs(
            &HardwareLike(&device),
            contactable(&device),
            fast(),
            MaskVersion(0x0701),
            MANUFACTURER,
            &[(0xFFF0, 16)],
        )
        .await
        .expect("read");
        assert_eq!(read, vec![(0xFFF0, held)], "{apdu}");
        assert_eq!(
            read_schedule(&device),
            vec![user(0xFFF0, 11), user(0xFFFB, 5)],
            "{apdu}"
        );
    }
}

#[tokio::test]
async fn the_simulated_executor_writes_and_backs_up_a_boundary_region_in_eleven_plus_five() {
    let device = device_with_apdu(15);
    let original = pattern(0xFFF0, 16);
    device.preset_memory(0xFFF0, &original);
    let new: Vec<u8> = (0..16).map(|i| 0xA0 + i).collect();
    let plan = writing(0xFFF0, new.clone());
    let transport = Encoding(&device);
    let authorisation =
        WriteAuthorisation::for_simulator(device.address(), WriteScope::Download).expect("ok");
    let mut session =
        ManagementSession::authorised(&transport, AuthorisationPlan::Skip, fast(), authorisation)
            .expect("a simulator session");
    let mut kept = None;
    run_memory_download_with_backup(&mut session, &plan, |_| {}, &mut |b: &DeviceBackup| {
        kept = Some(b.clone());
        Ok(())
    })
    .await
    .expect("download");
    assert_eq!(kept.expect("kept").regions[0].octets, original);
    // Backup first, then the write's own read-back of each chunk; nothing else.
    assert_eq!(
        read_schedule(&device),
        vec![
            user(0xFFF0, 11),
            user(0xFFFB, 5),
            user(0xFFF0, 11),
            user(0xFFFB, 5)
        ]
    );
    assert_eq!(
        write_schedule(&device),
        vec![(0xFFF0, 11, UserMemory), (0xFFFB, 5, UserMemory)]
    );
    assert_eq!(
        device.memory(0xFFF0, 16),
        new.iter().map(|b| Some(*b)).collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn a_genuine_simulator_at_apdu_nineteen_reads_a_boundary_region_in_fifteen_plus_one() {
    let device = device_with_apdu(19);
    let held = pattern(0xFFF0, 16);
    device.preset_memory(0xFFF0, &held);
    let read = probe(&device, &[(0xFFF0, 16)]).await.expect("read");
    assert_eq!(read, vec![(0xFFF0, held)]);
    assert_eq!(
        read_schedule(&device),
        vec![user(0xFFF0, 15), user(0xFFFF, 1)]
    );
}
