//! Fault-injection tests for the diagnostic recipe with an exhaustive service allowlist.
//!
//! Fault injection around the regular simulator. Every outgoing application
//! service, including setup and error/cancel cleanup, is checked exhaustively.
use knx_core::commissioning::declared_memory::{DeclaredRange, DeclaredRanges};
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::diagnostic_export::{
    collect, collect_declared_memory, Identity, MemoryRequest, Status, MEMORY_DEADLINE,
};
use knx_net::commissioning::simulator::{SimulatedDevice, SimulatorConfig};
use knx_net::{
    ApplicationService, BusError, Destination, ManagementTransport, SessionTiming, Tpci,
    TunnelEvent,
};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};
use std::time::Duration;
use tokio::sync::broadcast;
#[derive(Clone, Copy)]
enum Fault {
    Descriptor,
    Count,
    Silent,
    Cancel,
    WrongIndex,
    None,
}
/// Test-only memory faults, injected between the simulator and the client.
#[derive(Clone, Copy, PartialEq)]
enum MemoryFault {
    None,
    /// Drop the response to the read at this address.
    SilentAt(u16),
    /// Drop it and request cancellation.
    CancelAt(u16),
    /// Answer one octet short at this address.
    ShortAt(u16),
}
struct Wire {
    device: SimulatedDevice,
    tx: broadcast::Sender<TunnelEvent>,
    fault: Fault,
    memory_fault: MemoryFault,
    seen: Mutex<Vec<(Tpci, ApplicationService)>>,
    cancel: std::sync::Arc<AtomicBool>,
}
impl Wire {
    fn new(fault: Fault) -> Self {
        let device = SimulatedDevice::new();
        device.preset_property(0, 78, &[1, 2, 3, 4, 5, 6]);
        Self {
            device,
            tx: broadcast::channel(256).0,
            fault,
            memory_fault: MemoryFault::None,
            seen: Mutex::new(vec![]),
            cancel: Default::default(),
        }
    }
    /// A System-2 device as the profile asserts it, unless `adjust` differs.
    fn system2(adjust: impl FnOnce(&mut SimulatorConfig), memory_fault: MemoryFault) -> Self {
        let mut config = SimulatorConfig {
            mask_version: 0x0025,
            ..Default::default()
        };
        adjust(&mut config);
        let device = SimulatedDevice::with_config(config);
        device.preset_property(0, 12, &[0x00, 0x01]);
        device.preset_property(0, 78, &HARDWARE_TYPE[..6]);
        for range in RANGES {
            let pattern: Vec<u8> = (0..range.1)
                .map(|i| (range.0 as u32 + i as u32) as u8 ^ 0xA5)
                .collect();
            device.preset_memory(u32::from(range.0), &pattern);
        }
        Self {
            device,
            tx: broadcast::channel(256).0,
            fault: Fault::None,
            memory_fault,
            seen: Mutex::new(vec![]),
            cancel: Default::default(),
        }
    }
    fn memory_reads(&self) -> Vec<(u16, u8)> {
        self.seen
            .lock()
            .unwrap()
            .iter()
            .filter_map(|(_, s)| match s {
                ApplicationService::MemoryRead { number, address } => Some((*address, *number)),
                _ => None,
            })
            .collect()
    }
    /// The memory profile's exhaustive allowlist: the identity recipe plus
    /// `A_Memory_Read` of at most a standard frame inside a declared range.
    fn memory_safety(&self) {
        for (tpci, s) in self.seen.lock().unwrap().iter() {
            let allowed = match s {
                ApplicationService::NoApplicationPdu
                | ApplicationService::DeviceDescriptorRead { descriptor_type: 0 }
                | ApplicationService::PropertyValueRead {
                    object_index: 0,
                    property_id: 12 | 78,
                    nr_of_elem: 1,
                    start_index: 1,
                } => true,
                ApplicationService::MemoryRead { number, address } => {
                    (1..=12).contains(number)
                        && RANGES.iter().any(|(base, length)| {
                            *address >= *base
                                && u32::from(*address) + u32::from(*number)
                                    <= u32::from(*base) + u32::from(*length)
                        })
                }
                _ => false,
            };
            assert!(allowed, "forbidden service: {s:?}");
            assert!(matches!(
                tpci,
                Tpci::Connect | Tpci::Disconnect | Tpci::NumberedData { .. } | Tpci::Ack { .. }
            ));
        }
        assert_eq!(self.device.authorize_requests(), 0);
        assert!(!self.device.memory_was_written());
        assert!(!self.device.verify_mode());
        assert!(matches!(
            self.seen.lock().unwrap().last(),
            Some((Tpci::Disconnect, _))
        ));
    }
    fn safety(&self) {
        for (tpci, s) in self.seen.lock().unwrap().iter() {
            assert!(
                matches!(
                    s,
                    ApplicationService::NoApplicationPdu
                        | ApplicationService::DeviceDescriptorRead { descriptor_type: 0 }
                        | ApplicationService::PropertyValueRead {
                            object_index: 0,
                            property_id: 12 | 78,
                            nr_of_elem: 1,
                            start_index: 1
                        }
                ),
                "forbidden service: {s:?}"
            );
            assert!(matches!(
                tpci,
                Tpci::Connect | Tpci::Disconnect | Tpci::NumberedData { .. } | Tpci::Ack { .. }
            ));
        }
        assert_eq!(self.device.authorize_requests(), 0);
        assert!(!self.device.memory_was_written());
        assert!(!self.device.verify_mode());
    }
}
impl ManagementTransport for Wire {
    fn assigned_address(&self) -> IndividualAddress {
        self.device.assigned_address()
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.tx.subscribe()
    }
    async fn send_frame(
        &self,
        d: Destination,
        t: Tpci,
        s: ApplicationService,
    ) -> Result<(), BusError> {
        self.seen.lock().unwrap().push((t, s.clone()));
        let mut rx = self.device.subscribe();
        self.device.send_frame(d, t, s).await?;
        while let Ok(mut e) = rx.try_recv() {
            if let TunnelEvent::Telegram(ref mut f) = e {
                match (&mut f.service, self.fault) {
                    (
                        ApplicationService::DeviceDescriptorResponse { data, .. },
                        Fault::Descriptor,
                    ) => *data = vec![1, 2, 3],
                    (
                        ApplicationService::PropertyValueResponse {
                            property_id: 78,
                            nr_of_elem,
                            data,
                            ..
                        },
                        Fault::Count,
                    ) => {
                        *nr_of_elem = 2;
                        *data = vec![0xde, 0xad, 0xbe, 0xef];
                    }
                    (
                        ApplicationService::PropertyValueResponse {
                            property_id: 78, ..
                        },
                        Fault::Silent | Fault::Cancel,
                    ) => {
                        if matches!(self.fault, Fault::Cancel) {
                            let flag = self.cancel.clone();
                            tokio::spawn(async move {
                                tokio::time::sleep(Duration::from_millis(1)).await;
                                flag.store(true, Ordering::SeqCst);
                            });
                        }
                        continue;
                    }
                    (
                        ApplicationService::PropertyValueResponse {
                            property_id: 78,
                            start_index,
                            ..
                        },
                        Fault::WrongIndex,
                    ) => *start_index = 2,
                    _ => {}
                }
                if let ApplicationService::MemoryResponse { address, data } = &mut f.service {
                    match self.memory_fault {
                        MemoryFault::SilentAt(at) if *address == at => continue,
                        MemoryFault::CancelAt(at) if *address == at => {
                            self.cancel.store(true, Ordering::SeqCst);
                            continue;
                        }
                        MemoryFault::ShortAt(at) if *address == at => {
                            data.pop();
                        }
                        _ => {}
                    }
                }
            }
            let _ = self.tx.send(e);
        }
        Ok(())
    }
}
fn timing() -> SessionTiming {
    SessionTiming {
        response_timeout: Duration::from_millis(10),
        connection_timeout: Duration::from_millis(10),
        ..Default::default()
    }
}
fn target() -> ContactableAddress {
    ContactableAddress::new("1.1.25".parse().unwrap()).unwrap()
}
#[tokio::test]
async fn malformed_descriptor_bytes_survive_and_block_all_followups() {
    let w = Wire::new(Fault::Descriptor);
    let r = collect(
        &w,
        target(),
        timing(),
        Identity::default(),
        &w.cancel,
        |_| {},
    )
    .await;
    assert_eq!(r.items[0].bytes, [1, 2, 3]);
    assert_eq!(r.items[0].status, Status::Partial);
    assert_eq!(r.items[1].status, Status::NotRead);
    w.safety();
}
#[tokio::test]
async fn returned_element_count_and_unexpected_octets_are_preserved() {
    let w = Wire::new(Fault::Count);
    let r = collect(
        &w,
        target(),
        timing(),
        Identity::default(),
        &w.cancel,
        |_| {},
    )
    .await;
    assert_eq!(r.items[2].bytes, [0xde, 0xad, 0xbe, 0xef]);
    assert_eq!(r.items[2].returned_elements, Some(2));
    assert_eq!(r.items[2].status, Status::Partial);
    w.safety();
}
#[tokio::test]
async fn timeout_preserves_earlier_raw_data_and_disconnects() {
    let w = Wire::new(Fault::Silent);
    let r = collect(
        &w,
        target(),
        timing(),
        Identity::default(),
        &w.cancel,
        |_| {},
    )
    .await;
    assert_eq!(r.items[0].status, Status::Read);
    assert_eq!(r.items[1].status, Status::Read);
    assert_eq!(r.items[2].error, Some("timeout"));
    assert!(matches!(
        w.seen.lock().unwrap().last(),
        Some((Tpci::Disconnect, _))
    ));
    w.safety();
}
#[tokio::test]
async fn cancellation_during_unanswered_read_keeps_completed_items_and_tears_down() {
    let w = Wire::new(Fault::Cancel);
    let r = collect(
        &w,
        target(),
        SessionTiming {
            response_timeout: Duration::from_secs(1),
            ..timing()
        },
        Identity::default(),
        &w.cancel,
        |_| {},
    )
    .await;
    assert_eq!(r.stop_reason, Some("cancelled"));
    assert_eq!(r.items[0].status, Status::Read);
    assert_eq!(r.items[1].status, Status::Read);
    assert_eq!(r.items[2].error, Some("cancelled"));
    assert!(matches!(
        w.seen.lock().unwrap().last(),
        Some((Tpci::Disconnect, _))
    ));
    w.safety();
}
#[tokio::test]
async fn wrong_property_start_index_is_not_accepted_as_requested_data() {
    let w = Wire::new(Fault::WrongIndex);
    let r = collect(
        &w,
        target(),
        timing(),
        Identity::default(),
        &w.cancel,
        |_| {},
    )
    .await;
    assert_eq!(r.items[2].error, Some("timeout"));
    assert!(r.items[2].bytes.is_empty());
    w.safety();
}

/// Two declared ranges: 30 octets (three chunks) and 12 (one chunk).
const RANGES: [(u16, u16); 2] = [(0x0116, 30), (0x3C00, 12)];
/// The source's comparison value; the device answers its first six octets.
const HARDWARE_TYPE: [u8; 10] = [0, 1, 0, 0, 0, 0x5B, 0, 0, 0, 0];
fn ranges() -> DeclaredRanges {
    DeclaredRanges::new(
        RANGES
            .iter()
            .map(|&(address, length)| DeclaredRange { address, length })
            .collect(),
    )
    .unwrap()
}
fn asserted() -> Identity {
    Identity {
        mask: Some(0x0025),
        manufacturer: Some(0x0001),
    }
}
async fn run_memory(
    w: &Wire,
    expected: Identity,
    timing: SessionTiming,
) -> knx_net::commissioning::diagnostic_export::MemoryAcquisition {
    let ranges = ranges();
    collect_declared_memory(
        w,
        target(),
        timing,
        MemoryRequest {
            expected,
            hardware_type: &HARDWARE_TYPE,
            ranges: &ranges,
        },
        &w.cancel,
        |_, _| {},
    )
    .await
}
#[tokio::test]
async fn verified_identity_reads_every_declared_octet_once_in_source_order() {
    let w = Wire::system2(|_| {}, MemoryFault::None);
    let r = run_memory(&w, asserted(), timing()).await;
    assert_eq!(r.acquisition.stop_reason, None);
    assert!(r.acquisition.items.iter().all(|i| i.status == Status::Read));
    assert_eq!(
        w.memory_reads(),
        [(0x0116, 12), (0x0122, 12), (0x012E, 6), (0x3C00, 12)]
    );
    assert!(r.memory.iter().all(|m| m.status == Status::Read));
    for m in &r.memory {
        let expected: Vec<u8> = w
            .device
            .memory(u32::from(m.address), usize::from(m.requested))
            .into_iter()
            .map(Option::unwrap)
            .collect();
        assert_eq!(m.bytes, expected);
        assert!(m.attempted_at.is_some() && m.completed_at.is_some());
    }
    assert!(r.acquisition.disconnect_attempt_completed);
    w.memory_safety();
}
#[tokio::test]
async fn each_identity_mismatch_sends_no_memory_read() {
    // (case, device mask, PID12, PID78): exactly one differs from the assertion.
    let cases: [(&str, u16, [u8; 2], [u8; 6]); 3] = [
        ("mask", 0x0705, [0x00, 0x01], [0, 1, 0, 0, 0, 0x5B]),
        ("manufacturer", 0x0025, [0x00, 0x02], [0, 1, 0, 0, 0, 0x5B]),
        ("hardware type", 0x0025, [0x00, 0x01], [0, 1, 0, 0, 0, 0x5C]),
    ];
    for (name, mask, manufacturer, hardware) in cases {
        let w = Wire::system2(|c| c.mask_version = mask, MemoryFault::None);
        w.device.preset_property(0, 12, &manufacturer);
        w.device.preset_property(0, 78, &hardware);
        let r = run_memory(&w, asserted(), timing()).await;
        assert_eq!(
            r.acquisition.stop_reason,
            Some("identity-mismatch"),
            "{name}"
        );
        assert!(w.memory_reads().is_empty(), "{name}");
        assert!(
            r.memory.iter().all(|m| m.status == Status::NotRead),
            "{name}"
        );
        w.memory_safety();
    }
}
#[tokio::test]
async fn a_hardware_type_of_another_width_reads_no_memory() {
    // Seven to ten octets are all prefixes of the zero-padded comparison
    // value; PID78 is one six-octet value, so none of them is a match.
    for width in (0..6).chain(7..=HARDWARE_TYPE.len()) {
        let w = Wire::system2(|_| {}, MemoryFault::None);
        w.device.preset_property(0, 78, &HARDWARE_TYPE[..width]);
        let r = run_memory(&w, asserted(), timing()).await;
        let item = &r.acquisition.items[2];
        assert_eq!(item.bytes, HARDWARE_TYPE[..width], "{width} octets");
        assert_ne!(item.status, Status::Read, "{width} octets");
        assert!(
            matches!(
                r.acquisition.stop_reason,
                Some("identity-unverified" | "identity-mismatch")
            ),
            "{width} octets: {:?}",
            r.acquisition.stop_reason
        );
        assert!(w.memory_reads().is_empty(), "{width} octets");
        w.memory_safety();
    }
}
#[tokio::test]
async fn an_identity_property_that_is_not_exactly_read_reads_no_memory() {
    // PID78 answered with a different element count: partial, not a match.
    let w = Wire {
        fault: Fault::Count,
        ..Wire::system2(|_| {}, MemoryFault::None)
    };
    let r = run_memory(&w, asserted(), timing()).await;
    assert_eq!(r.acquisition.items[2].status, Status::Partial);
    assert_eq!(r.acquisition.stop_reason, Some("identity-unverified"));
    assert!(w.memory_reads().is_empty());
    w.memory_safety();
    // An unanswered identity read is an error and stops everything.
    let w = Wire {
        fault: Fault::Silent,
        ..Wire::system2(|_| {}, MemoryFault::None)
    };
    let r = run_memory(&w, asserted(), timing()).await;
    assert_eq!(r.acquisition.stop_reason, Some("timeout"));
    assert!(w.memory_reads().is_empty());
    w.memory_safety();
}
#[tokio::test]
async fn a_missing_assertion_refuses_before_any_frame() {
    for expected in [
        Identity {
            mask: None,
            ..asserted()
        },
        Identity {
            manufacturer: None,
            ..asserted()
        },
    ] {
        let w = Wire::system2(|_| {}, MemoryFault::None);
        let r = run_memory(&w, expected, timing()).await;
        assert_eq!(
            r.acquisition.stop_reason,
            Some("identity-assertion-missing")
        );
        assert!(w.seen.lock().unwrap().is_empty());
    }
    let w = Wire::system2(|_| {}, MemoryFault::None);
    let ranges = ranges();
    let r = collect_declared_memory(
        &w,
        target(),
        timing(),
        MemoryRequest {
            expected: asserted(),
            hardware_type: &[],
            ranges: &ranges,
        },
        &w.cancel,
        |_, _| {},
    )
    .await;
    assert_eq!(
        r.acquisition.stop_reason,
        Some("identity-assertion-missing")
    );
    assert!(w.seen.lock().unwrap().is_empty());
}
#[tokio::test]
async fn a_refused_chunk_marks_its_range_and_the_next_range_is_still_read() {
    let w = Wire::system2(
        |c| c.protected_memory = Some((0x0122, 0x0123)),
        MemoryFault::None,
    );
    let r = run_memory(&w, asserted(), timing()).await;
    assert_eq!(r.acquisition.stop_reason, None);
    let status: Vec<Status> = r.memory.iter().map(|m| m.status).collect();
    assert_eq!(
        status,
        [Status::Read, Status::Refused, Status::NotRead, Status::Read]
    );
    assert!(r.memory[1].bytes.is_empty(), "a refusal is never zeros");
    assert_eq!(w.memory_reads(), [(0x0116, 12), (0x0122, 12), (0x3C00, 12)]);
    w.memory_safety();
}
#[tokio::test]
async fn a_short_answer_is_kept_as_partial_and_stops_only_its_range() {
    let w = Wire::system2(|_| {}, MemoryFault::ShortAt(0x0116));
    let r = run_memory(&w, asserted(), timing()).await;
    assert_eq!(r.memory[0].status, Status::Partial);
    assert_eq!(r.memory[0].bytes.len(), 11);
    assert_eq!(r.memory[0].error, Some("unexpected-length"));
    assert_eq!(r.memory[1].status, Status::NotRead);
    assert_eq!(r.memory[3].status, Status::Read);
    assert_eq!(w.memory_reads(), [(0x0116, 12), (0x3C00, 12)]);
    w.memory_safety();
}
#[tokio::test]
async fn a_timeout_mid_range_keeps_earlier_chunks_and_stops_everything() {
    let w = Wire::system2(|_| {}, MemoryFault::SilentAt(0x0122));
    let r = run_memory(&w, asserted(), timing()).await;
    assert_eq!(r.acquisition.stop_reason, Some("timeout"));
    assert_eq!(r.memory[0].status, Status::Read);
    assert_eq!(r.memory[0].bytes.len(), 12);
    assert_eq!(r.memory[1].status, Status::Error);
    assert_eq!(r.memory[1].error, Some("timeout"));
    assert_eq!(r.memory[2].status, Status::NotRead);
    assert_eq!(r.memory[3].status, Status::NotRead);
    assert!(!w.memory_reads().contains(&(0x3C00, 12)));
    w.memory_safety();
}
#[tokio::test]
async fn cancellation_mid_range_keeps_earlier_chunks_and_still_disconnects() {
    let w = Wire::system2(|_| {}, MemoryFault::CancelAt(0x0122));
    let r = run_memory(
        &w,
        asserted(),
        SessionTiming {
            response_timeout: Duration::from_secs(5),
            ..timing()
        },
    )
    .await;
    assert_eq!(r.acquisition.stop_reason, Some("cancelled"));
    assert_eq!(r.memory[0].status, Status::Read);
    assert_eq!(r.memory[1].error, Some("cancelled"));
    assert_eq!(r.memory[2].status, Status::NotRead);
    w.memory_safety();
}
#[tokio::test(start_paused = true)]
async fn the_profile_deadline_stops_a_stalled_read_and_keeps_earlier_chunks() {
    let w = Wire::system2(|_| {}, MemoryFault::SilentAt(0x3C00));
    let started = tokio::time::Instant::now();
    let r = run_memory(
        &w,
        asserted(),
        SessionTiming {
            response_timeout: MEMORY_DEADLINE * 2,
            ..timing()
        },
    )
    .await;
    assert_eq!(r.acquisition.stop_reason, Some("deadline"));
    assert_eq!(r.memory[3].error, Some("deadline"));
    assert!(r.memory[..3].iter().all(|m| m.status == Status::Read));
    assert!(started.elapsed() >= MEMORY_DEADLINE);
    assert!(started.elapsed() < MEMORY_DEADLINE + Duration::from_secs(10));
    w.memory_safety();
}
