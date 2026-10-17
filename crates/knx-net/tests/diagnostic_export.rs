//! Synthetic devices only: an exhaustive outgoing-service allowlist is the safety oracle.
use knx_core::ContactableAddress;
use knx_net::commissioning::diagnostic_export::{collect, Identity, Query, Status};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice, SimulatorConfig};
use knx_net::SessionTiming;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

fn target() -> ContactableAddress {
    ContactableAddress::new("1.1.25".parse().unwrap()).unwrap()
}
fn timing() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(10),
        response_timeout: Duration::from_millis(10),
        ..Default::default()
    }
}
fn assert_read_only(d: &SimulatedDevice) {
    assert_eq!(d.authorize_requests(), 0);
    assert!(!d.memory_was_written());
    assert!(!d.verify_mode());
    assert!(
        d.seen().iter().all(|s| matches!(
            s,
            Seen::Connect
                | Seen::Disconnect
                | Seen::PropertyRead {
                    object_index: 0,
                    property_id: 12 | 78
                }
                | Seen::Other("DeviceDescriptorRead")
        )),
        "unexpected services: {:?}",
        d.seen()
    );
}
#[tokio::test]
async fn multiple_manufacturers_and_unknown_masks_use_the_same_bounded_recipe() {
    for (mask, manufacturer) in [(0x0025, 1u16), (0x0701, 0x83), (0xffff, 0x1234)] {
        let d = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: mask,
            ..Default::default()
        });
        d.preset_property(0, 12, &manufacturer.to_be_bytes());
        d.preset_property(0, 78, &[9, 8, 7, 6, 5, 4]);
        let r = collect(
            &d,
            target(),
            timing(),
            Identity::default(),
            &AtomicBool::new(false),
            |_| {},
        )
        .await;
        assert_eq!(r.items.len(), 3);
        assert!(r.items.iter().all(|i| i.status == Status::Read));
        assert_eq!(r.items[0].bytes, mask.to_be_bytes());
        assert_eq!(r.items[1].bytes, manufacturer.to_be_bytes());
        assert_eq!(r.items[2].bytes, [9, 8, 7, 6, 5, 4]);
        assert_eq!(r.items[2].returned_elements, Some(1));
        assert!(matches!(d.seen().last(), Some(Seen::Disconnect)));
        assert_read_only(&d);
    }
}
#[tokio::test]
async fn identity_conflict_preserves_observation_and_stops_before_further_reads() {
    let d = SimulatedDevice::new();
    let r = collect(
        &d,
        target(),
        timing(),
        Identity {
            mask: Some(0xffff),
            manufacturer: None,
        },
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    assert_eq!(r.items[0].status, Status::Read);
    assert_eq!(r.stop_reason, Some("identity-mismatch"));
    assert_eq!(r.items[1].status, Status::NotRead);
    assert!(!d
        .seen()
        .iter()
        .any(|s| matches!(s, Seen::PropertyRead { .. } | Seen::MemoryRead { .. })));
    assert_read_only(&d);
}
#[tokio::test]
async fn unknown_property_width_is_preserved_not_replaced_by_zeroes() {
    let d = SimulatedDevice::new();
    d.preset_property(0, 78, &[0xde, 0xad, 0xbe]);
    let r = collect(
        &d,
        target(),
        timing(),
        Identity::default(),
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    assert_eq!(r.items[2].query, Query::HardwareType);
    assert_eq!(r.items[2].status, Status::Partial);
    assert_eq!(r.items[2].bytes, [0xde, 0xad, 0xbe]);
    assert_eq!(r.items[2].error, Some("unexpected-length"));
    assert_eq!(r.items[0].status, Status::Read);
    assert_read_only(&d);
}
#[tokio::test]
async fn property_refusal_is_honestly_ambiguous_and_previous_bytes_survive() {
    let d = SimulatedDevice::new();
    let r = collect(
        &d,
        target(),
        timing(),
        Identity::default(),
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    assert_eq!(r.items[0].status, Status::Read);
    // Zero elements cannot distinguish denied access from a missing property.
    assert_eq!(r.items[2].status, Status::Refused);
    assert!(r.items[2].bytes.is_empty());
    assert_eq!(r.items[2].returned_elements, Some(0));
    assert_read_only(&d);
}
#[tokio::test]
async fn cancellation_after_first_read_keeps_bytes_and_disconnects() {
    let d = SimulatedDevice::new();
    let cancel = AtomicBool::new(false);
    let r = collect(
        &d,
        target(),
        timing(),
        Identity::default(),
        &cancel,
        |items| {
            if items[0].status == Status::Read {
                cancel.store(true, Ordering::SeqCst);
            }
        },
    )
    .await;
    assert_eq!(r.items[0].status, Status::Read);
    assert_eq!(r.items[1].status, Status::NotRead);
    assert_eq!(r.stop_reason, Some("cancelled"));
    assert!(matches!(d.seen().last(), Some(Seen::Disconnect)));
    assert_read_only(&d);
}
#[tokio::test]
async fn precancel_does_not_even_connect_and_silent_target_is_bounded() {
    let d = SimulatedDevice::new();
    let r = collect(
        &d,
        target(),
        timing(),
        Identity::default(),
        &AtomicBool::new(true),
        |_| {},
    )
    .await;
    assert!(d.seen().is_empty());
    assert!(r.items.iter().all(|i| i.status == Status::NotRead));
    let silent = SimulatedDevice::with_config(SimulatorConfig {
        silent: true,
        ..Default::default()
    });
    let r = collect(
        &silent,
        target(),
        timing(),
        Identity::default(),
        &AtomicBool::new(false),
        |_| {},
    )
    .await;
    assert_eq!(r.stop_reason, Some("connection-released"));
    assert_eq!(r.items[0].status, Status::Error);
    assert_eq!(r.items[1].status, Status::NotRead);
    assert_read_only(&silent);
}
