//! Fault-injection tests for the diagnostic recipe with an exhaustive service allowlist.
//!
//! Fault injection around the regular simulator. Every outgoing application
//! service, including setup and error/cancel cleanup, is checked exhaustively.
use knx_core::{ContactableAddress, IndividualAddress};
use knx_net::commissioning::diagnostic_export::{collect, Identity, Status};
use knx_net::commissioning::simulator::SimulatedDevice;
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
}
struct Wire {
    device: SimulatedDevice,
    tx: broadcast::Sender<TunnelEvent>,
    fault: Fault,
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
            seen: Mutex::new(vec![]),
            cancel: Default::default(),
        }
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
