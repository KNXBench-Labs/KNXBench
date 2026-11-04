//! Exercise datasheet wire vectors and serial actor lifecycle using only in-memory duplex links.

use super::*;
use crate::cemi::{FrameControl, GroupValue};
use knx_core::GroupAddress;
use tokio::io::DuplexStream;

// Independent NCN5130 Table 12/13 + standard TP1 layout vectors. FCS is one's
// complement XOR; no encoder-under-test builds the oracle or fake responses.
const TP: &[u8] = &[0xbc, 0x11, 0x01, 0x09, 0x01, 0xe1, 0x00, 0x81, 0x3b];
const WIRE: &[u8] = &[
    0x80, 0xbc, 0x81, 0x11, 0x82, 0x01, 0x83, 0x09, 0x84, 0x01, 0x85, 0xe1, 0x86, 0x00, 0x87, 0x81,
    0x48, 0x3b,
];
fn source() -> IndividualAddress {
    IndividualAddress::from_raw(0x1101)
}
fn destination() -> Destination {
    Destination::Group(GroupAddress::from_raw(0x0901))
}
fn service() -> ApplicationService {
    ApplicationService::GroupValueWrite(GroupValue::Short(1))
}
fn frame() -> LDataFrame {
    LDataFrame {
        kind: LDataMessageKind::Request,
        source: source(),
        destination: destination(),
        transport: Tpci::UnnumberedData,
        service: service(),
        control: None,
    }
}
fn timing() -> Timing {
    Timing {
        initialize: Duration::from_millis(100),
        send: Duration::from_millis(100),
        frame: Duration::from_millis(40),
        shutdown: Duration::from_millis(50),
    }
}
async fn bytes(peer: &mut DuplexStream, expected: &[u8]) {
    let mut got = vec![0; expected.len()];
    peer.read_exact(&mut got).await.unwrap();
    assert_eq!(got, expected);
}
async fn initialize(peer: &mut DuplexStream, mode: TpuartMode) {
    bytes(peer, &[1]).await;
    peer.write_all(&[3]).await.unwrap();
    bytes(peer, &[2]).await;
    peer.write_all(&[7]).await.unwrap();
    match mode {
        TpuartMode::BusMonitor => bytes(peer, &[5]).await,
        TpuartMode::Active => {
            bytes(peer, &[0xf1, 0x11, 0x01, 0]).await;
            peer.write_all(&[0x21]).await.unwrap();
        }
    }
}
async fn setup(mode: TpuartMode) -> (TpuartClient, DuplexStream) {
    let (stream, mut peer) = tokio::io::duplex(512);
    let (client, ()) = tokio::join!(
        TpuartClient::connect_stream(stream, source(), mode, timing()),
        initialize(&mut peer, mode)
    );
    (client.unwrap(), peer)
}
async fn close(client: &TpuartClient, peer: &mut DuplexStream) {
    let (result, ()) = tokio::join!(client.disconnect(), async {
        bytes(peer, &[1]).await;
        peer.write_all(&[3]).await.unwrap();
    });
    result.unwrap();
}

#[test]
fn independent_golden_standard_write_and_control_vectors() {
    assert_eq!(codec::encode(&frame()).unwrap(), WIRE);
    let mut control = frame();
    control.destination = Destination::Individual(IndividualAddress::from_raw(0x1102));
    control.transport = Tpci::Connect;
    control.service = ApplicationService::NoApplicationPdu;
    assert_eq!(
        codec::encode(&control).unwrap(),
        [
            0x80, 0xb0, 0x81, 0x11, 0x82, 0x01, 0x83, 0x11, 0x84, 0x02, 0x85, 0x60, 0x86, 0x80,
            0x47, 0xac
        ]
    );
    let mut parser = codec::Parser::default();
    for b in &TP[..8] {
        assert_eq!(parser.push(*b).unwrap(), None);
    }
    let Some(Service::Telegram(decoded, tp)) = parser.push(TP[8]).unwrap() else {
        panic!("missing telegram")
    };
    assert_eq!(tp, TP);
    assert_eq!(decoded.kind, LDataMessageKind::Indication);
    assert_eq!(decoded.source, source());
    assert_eq!(decoded.destination, destination());
    assert_eq!(decoded.service, service());
    assert_eq!(decoded.effective_control().hop_count, 6);
}

#[test]
fn source_and_standard_frame_preflight_is_strict() {
    for raw in [0, 0xffff] {
        assert!(TpuartClient::validate_source(IndividualAddress::from_raw(raw)).is_err());
    }
    let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0];
    assert!(knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES.contains(&excluded));
    assert!(TpuartClient::validate_source(excluded).is_err());
    let mut f = frame();
    f.destination = Destination::Individual(excluded);
    assert!(TpuartClient::validate_frame(&f).is_err());
    f = frame();
    f.service = ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0; 15]));
    assert!(TpuartClient::validate_frame(&f)
        .unwrap_err()
        .to_string()
        .contains("extended"));
    f.service = ApplicationService::GroupValueWrite(GroupValue::Bytes(vec![0; 14]));
    assert!(TpuartClient::validate_frame(&f).is_ok());
    f.control = Some(FrameControl {
        hop_count: 8,
        ..f.effective_control()
    });
    assert!(TpuartClient::validate_frame(&f).is_err());
    f = frame();
    f.destination = Destination::SystemBroadcast;
    assert!(TpuartClient::validate_frame(&f).is_err());
    f = frame();
    f.kind = LDataMessageKind::Indication;
    assert!(TpuartClient::validate_frame(&f).is_err());
}

#[test]
fn parser_preserves_service_bytes_inside_frames_and_unknown_apci() {
    let mut tp = vec![
        0xbc, 0x03, 0x8b, 0x0b, 0x07, 0xe5, 0x03, 0xff, 0x03, 0x8b, 0x0b, 0x07,
    ];
    tp.push(tp.iter().fold(0xff, |xor, b| xor ^ b));
    let mut parser = codec::Parser::default();
    let mut services = Vec::new();
    for b in tp.into_iter().chain([0x13, 0x8b]) {
        if let Some(s) = parser.push(b).unwrap() {
            services.push(s);
        }
    }
    let Service::Telegram(decoded, _) = &services[0] else {
        panic!("telegram expected")
    };
    assert_eq!(
        decoded.service,
        ApplicationService::Other {
            apci: 0x3ff,
            data: vec![3, 0x8b, 0x0b, 7]
        }
    );
    assert_eq!(
        &services[1..],
        &[Service::FrameState, Service::Confirmation(true)]
    );
}

#[test]
fn parser_rejects_bad_checksum_unknown_extended_and_frame_state() {
    let mut bad = TP.to_vec();
    bad[8] ^= 1;
    let mut parser = codec::Parser::default();
    for b in &bad[..8] {
        parser.push(*b).unwrap();
    }
    assert!(parser
        .push(bad[8])
        .unwrap_err()
        .to_string()
        .contains("checksum"));
    for byte in [0x02, 0xff, 0xf0, 0xcb, 0x10, 0x30, 0x53, 0x93, 0x33] {
        assert!(codec::Parser::default().push(byte).is_err(), "{byte:#x}");
    }
}

#[tokio::test(start_paused = true)]
async fn fragmented_and_coalesced_reception_and_passive_ack_diagnostics() {
    let (client, mut peer) = setup(TpuartMode::BusMonitor).await;
    let mut events = client.subscribe();
    for chunk in TP.chunks(2) {
        peer.write_all(chunk).await.unwrap();
        tokio::task::yield_now().await;
    }
    peer.write_all(&[TP, &[0xcc, 0xc0, 0x0c]].concat())
        .await
        .unwrap();
    for _ in 0..2 {
        let TunnelEvent::Telegram(f) = events.recv().await.unwrap() else {
            panic!("telegram expected")
        };
        assert_eq!(f.service, service());
    }
    tokio::task::yield_now().await;
    assert_eq!(client.diagnostics().acknowledgements, 3);
    assert_eq!(client.diagnostics().last_acknowledgement, Some(0x0c));
    close(&client, &mut peer).await;
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert_eq!(
        peer.read_u8().await.unwrap_err().kind(),
        std::io::ErrorKind::UnexpectedEof
    );
}

#[tokio::test(start_paused = true)]
async fn send_waits_for_positive_confirmation_and_receives_echo() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    let mut sent = Box::pin(client.send(destination(), service()));
    let (held, ()) = tokio::join!(timeout(Duration::from_millis(1), &mut sent), async {
        bytes(&mut peer, WIRE).await;
    });
    assert!(
        held.is_err(),
        "send completed before any echo or 8B confirmation"
    );
    peer.write_all(TP).await.unwrap();
    let TunnelEvent::Telegram(_) = events.recv().await.unwrap() else {
        panic!("echo missing")
    };
    // Poll the SAME future without cancelling it on the observation timeout.
    assert!(
        timeout(Duration::from_millis(1), &mut sent).await.is_err(),
        "send completed after echo but before any 8B confirmation"
    );
    peer.write_all(&[0x8b]).await.unwrap();
    sent.await.unwrap();
    close(&client, &mut peer).await;
}

#[tokio::test(start_paused = true)]
async fn concurrent_sends_are_serialized_until_each_confirmation() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let (a, b, ()) = tokio::join!(
        client.send(destination(), service()),
        client.send(destination(), service()),
        async {
            bytes(&mut peer, WIRE).await;
            assert!(timeout(Duration::from_millis(1), peer.read_u8())
                .await
                .is_err());
            peer.write_all(&[TP, &[0x8b]].concat()).await.unwrap();
            bytes(&mut peer, WIRE).await;
            peer.write_all(&[TP, &[0x8b]].concat()).await.unwrap();
        }
    );
    a.unwrap();
    b.unwrap();
    close(&client, &mut peer).await;
}

#[tokio::test(start_paused = true)]
async fn passive_send_refused_without_transmit_or_ack_commands() {
    let (client, mut peer) = setup(TpuartMode::BusMonitor).await;
    assert!(client
        .send(destination(), service())
        .await
        .unwrap_err()
        .to_string()
        .contains("passive"));
    assert!(timeout(Duration::from_millis(1), peer.read_u8())
        .await
        .is_err());
    close(&client, &mut peer).await;
}

#[tokio::test(start_paused = true)]
async fn negative_confirmation_closes_without_success_or_retry() {
    for echo in [false, true] {
        let (client, mut peer) = setup(TpuartMode::Active).await;
        let mut events = client.subscribe();
        let (sent, ()) = tokio::join!(client.send(destination(), service()), async {
            bytes(&mut peer, WIRE).await;
            if echo {
                peer.write_all(TP).await.unwrap();
            }
            peer.write_all(&[0x0b]).await.unwrap();
            bytes(&mut peer, &[1]).await;
            peer.write_all(&[3]).await.unwrap();
        });
        assert!(sent.unwrap_err().to_string().contains("negative"));
        if echo {
            assert!(matches!(
                events.recv().await.unwrap(),
                TunnelEvent::Telegram(_)
            ));
        }
        assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
        assert!(client.send(destination(), service()).await.is_err());
    }
}

#[tokio::test(start_paused = true)]
async fn confirmation_timeout_then_stale_confirmation_cannot_settle_next_send() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    let (first, ()) = tokio::join!(client.send(destination(), service()), async {
        bytes(&mut peer, WIRE).await;
        // Only a reset, never an application retry, may follow the deadline.
        bytes(&mut peer, &[1]).await;
        peer.write_all(&[0x8b, 3]).await.unwrap();
    });
    assert!(matches!(first, Err(BusError::Timeout)));
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(client.send(destination(), service()).await.is_err());
    assert!(peer.read_u8().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn cancelling_send_closes_and_releases_stream() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    let (result, ()) = tokio::join!(
        timeout(
            Duration::from_millis(1),
            client.send(destination(), service())
        ),
        async {
            bytes(&mut peer, WIRE).await;
            bytes(&mut peer, &[1]).await;
            peer.write_all(&[0x8b, 3]).await.unwrap();
        }
    );
    assert!(result.is_err());
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(client.send(destination(), service()).await.is_err());
    assert!(peer.read_u8().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn drop_resets_and_announces_closed_after_resource_release() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    drop(client);
    bytes(&mut peer, &[1]).await;
    peer.write_all(&[3]).await.unwrap();
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(peer.read_u8().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn eof_closes_and_retains_io_diagnostic() {
    let (client, peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    drop(peer);
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(client.diagnostics().closure.unwrap().contains("I/O"));
    assert!(client.disconnect().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn truncated_and_oversized_receive_frames_have_bounded_failure() {
    for data in [
        vec![0xbc, 0x11],
        vec![0xbc, 0x11, 1, 9, 1, 0xef, 0],
        vec![0x10],
        vec![0x01],
        vec![0x97],
    ] {
        let (client, mut peer) = setup(TpuartMode::BusMonitor).await;
        let mut events = client.subscribe();
        peer.write_all(&data).await.unwrap();
        assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
        assert!(client.diagnostics().closure.is_some());
        assert!(client.disconnect().await.is_err());
    }
}

#[tokio::test(start_paused = true)]
async fn initialization_reset_state_config_refusals_and_deadlines() {
    for stage in 0..6 {
        let (stream, mut peer) = tokio::io::duplex(256);
        let (client, ()) = tokio::join!(
            TpuartClient::connect_stream(stream, source(), TpuartMode::Active, timing()),
            async {
                bytes(&mut peer, &[1]).await;
                if stage == 0 {
                    return;
                }
                peer.write_all(&[if stage == 1 { 0xff } else { 3 }])
                    .await
                    .unwrap();
                if stage == 1 {
                    return;
                }
                bytes(&mut peer, &[2]).await;
                if stage == 2 {
                    return;
                }
                peer.write_all(&[if stage == 3 { 0x17 } else { 7 }])
                    .await
                    .unwrap();
                if stage == 3 {
                    return;
                }
                bytes(&mut peer, &[0xf1, 0x11, 1, 0]).await;
                if stage == 4 {
                    return;
                }
                peer.write_all(&[1]).await.unwrap(); // Configure.ind without auto-ACK.
            }
        );
        assert!(client.is_err(), "stage {stage}");
        // Allow the failed-init actor to finish bounded reset and release stream.
        tokio::time::sleep(timing().shutdown * 2).await;
        let mut data = Vec::new();
        peer.read_to_end(&mut data).await.unwrap();
        assert_eq!(data, [1]);
    }
}

#[tokio::test(start_paused = true)]
async fn initialization_cancellation_resets_then_closes() {
    let (stream, mut peer) = tokio::io::duplex(256);
    let result = timeout(
        Duration::from_millis(1),
        TpuartClient::connect_stream(stream, source(), TpuartMode::Active, timing()),
    )
    .await;
    assert!(result.is_err());
    bytes(&mut peer, &[1, 1]).await;
    peer.write_all(&[3]).await.unwrap();
    let mut rest = Vec::new();
    peer.read_to_end(&mut rest).await.unwrap();
    assert!(rest.is_empty());
}

#[tokio::test(start_paused = true)]
async fn unsolicited_and_duplicate_confirmations_close_without_next_transmission() {
    for confirmation in [vec![0x8b], vec![0x0b]] {
        let (client, mut peer) = setup(TpuartMode::Active).await;
        let mut events = client.subscribe();
        peer.write_all(&confirmation).await.unwrap();
        bytes(&mut peer, &[1]).await;
        peer.write_all(&[3]).await.unwrap();
        assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
        assert!(client.send(destination(), service()).await.is_err());
        assert!(peer.read_u8().await.is_err());
    }
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    let (result, ()) = tokio::join!(client.send(destination(), service()), async {
        bytes(&mut peer, WIRE).await;
        peer.write_all(&[TP, &[0x8b, 0x8b]].concat()).await.unwrap();
        bytes(&mut peer, &[1]).await;
        peer.write_all(&[3]).await.unwrap();
    });
    result.unwrap();
    assert!(matches!(
        events.recv().await.unwrap(),
        TunnelEvent::Telegram(_)
    ));
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(client.send(destination(), service()).await.is_err());
}

#[tokio::test(start_paused = true)]
async fn bad_receive_checksum_never_publishes_a_successful_telegram() {
    let (client, mut peer) = setup(TpuartMode::BusMonitor).await;
    let mut events = client.subscribe();
    let mut bad = TP.to_vec();
    bad[8] ^= 1;
    peer.write_all(&bad).await.unwrap();
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(client.diagnostics().closure.unwrap().contains("checksum"));
}

#[tokio::test(start_paused = true)]
async fn backpressured_write_has_deadline_and_releases_stream() {
    let (stream, mut peer) = tokio::io::duplex(1);
    let (client, ()) = tokio::join!(
        TpuartClient::connect_stream(stream, source(), TpuartMode::Active, timing()),
        initialize(&mut peer, TpuartMode::Active)
    );
    let client = client.unwrap();
    let mut events = client.subscribe();
    let (result, ()) = tokio::join!(client.send(destination(), service()), async {
        assert_eq!(peer.read_u8().await.unwrap(), 0x80);
        // Stop draining: write_all must time out, not hang or claim delivery.
    });
    assert!(matches!(result, Err(BusError::Timeout)));
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    let mut rest = Vec::new();
    peer.read_to_end(&mut rest).await.unwrap();
    assert_eq!(rest, [0xbc]);
    assert!(client.send(destination(), service()).await.is_err());
}

#[tokio::test]
async fn public_open_validates_source_and_empty_path_before_io() {
    for address in [
        IndividualAddress::from_raw(0),
        IndividualAddress::from_raw(0xffff),
        knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0],
    ] {
        let error = TpuartClient::open("nonexistent-serial-test", address, TpuartMode::Active)
            .await
            .err()
            .unwrap();
        assert!(matches!(error, BusError::Protocol(_)));
    }
    assert!(matches!(
        TpuartClient::open("", source(), TpuartMode::BusMonitor)
            .await
            .err()
            .unwrap(),
        BusError::Protocol(_)
    ));
}

#[test]
fn parser_rejects_incomplete_tpdu_even_with_valid_checksum() {
    // Length zero is legal only for a transport control PDU; 00 needs an APCI byte.
    let mut parser = codec::Parser::default();
    for byte in [0xbc, 0x11, 0x01, 0x09, 0x01, 0xe0, 0x00] {
        parser.push(byte).unwrap();
    }
    assert!(parser.push(0xbb).is_err());
}

#[tokio::test(start_paused = true)]
async fn timeout_rejects_queued_send_despite_late_positive_confirmation() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    let (first, second, ()) = tokio::join!(
        client.send(destination(), service()),
        async {
            tokio::time::sleep(timing().send / 2).await;
            client.send(destination(), service()).await
        },
        async {
            bytes(&mut peer, WIRE).await;
            bytes(&mut peer, &[1]).await; // Timeout closes, never transmits the queued frame.
            peer.write_all(&[0x8b, 3]).await.unwrap();
        }
    );
    assert!(matches!(first, Err(BusError::Timeout)));
    assert!(second.is_err());
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(peer.read_u8().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn cancelling_queued_send_invalidates_the_inflight_session() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    let (first, second, ()) = tokio::join!(
        client.send(destination(), service()),
        async {
            tokio::time::sleep(Duration::from_millis(1)).await;
            timeout(
                Duration::from_millis(1),
                client.send(destination(), service()),
            )
            .await
        },
        async {
            bytes(&mut peer, WIRE).await;
            bytes(&mut peer, &[1]).await;
            peer.write_all(&[0x8b, 3]).await.unwrap();
        }
    );
    assert!(first.is_err());
    assert!(second.is_err());
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(peer.read_u8().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn shutdown_deadline_is_reported_and_releases_resource() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    assert!(client
        .disconnect()
        .await
        .unwrap_err()
        .to_string()
        .contains("reset on close timed out"));
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    bytes(&mut peer, &[1]).await;
    assert!(peer.read_u8().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn subscriptions_before_during_and_after_cleanup_observe_closed_after_release() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut before = client.subscribe();
    let mut closing = Box::pin(client.disconnect());
    let (held, ()) = tokio::join!(
        timeout(Duration::from_millis(1), &mut closing),
        bytes(&mut peer, &[1])
    );
    assert!(held.is_err());
    let mut during = client.subscribe();
    // Reset is still unconfirmed: neither terminal event nor done may escape.
    assert!(!*client.done.borrow());
    assert!(timeout(Duration::from_millis(1), before.recv())
        .await
        .is_err());
    assert!(timeout(Duration::from_millis(1), during.recv())
        .await
        .is_err());
    peer.write_all(&[3]).await.unwrap();
    closing.await.unwrap();
    assert!(*client.done.borrow());
    let mut after = client.subscribe();
    for receiver in [&mut before, &mut during, &mut after] {
        assert_eq!(
            timeout(Duration::from_millis(1), receiver.recv())
                .await
                .unwrap()
                .unwrap(),
            TunnelEvent::Closed
        );
    }
    assert!(
        peer.read_u8().await.is_err(),
        "Closed/done must follow resource release"
    );
}

#[tokio::test(start_paused = true)]
async fn failure_between_open_and_subscribe_never_leaves_a_silent_live_receiver() {
    for wait_for_close in [false, true] {
        let (client, peer) = setup(TpuartMode::BusMonitor).await;
        drop(peer);
        if wait_for_close {
            assert!(client.disconnect().await.is_err());
        }
        let mut receiver = client.subscribe();
        assert_eq!(
            timeout(Duration::from_secs(1), receiver.recv())
                .await
                .unwrap()
                .unwrap(),
            TunnelEvent::Closed
        );
    }
}

#[test]
fn subscribe_racing_terminal_publication_observes_closed() {
    let (sender, _) = broadcast::channel(256);
    let events = Arc::new(Mutex::new(Events {
        sender,
        closed: false,
    }));
    let barrier = Arc::new(std::sync::Barrier::new(17));
    let subscribers: Vec<_> = (0..16)
        .map(|_| {
            let events = events.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                events.lock().unwrap().subscribe()
            })
        })
        .collect();
    barrier.wait();
    events.lock().unwrap().close();
    for subscriber in subscribers {
        assert_eq!(
            subscriber.join().unwrap().try_recv().unwrap(),
            TunnelEvent::Closed
        );
    }
    assert_eq!(
        events.lock().unwrap().subscribe().try_recv().unwrap(),
        TunnelEvent::Closed
    );
}

fn changed_tp(index: usize, mask: u8) -> Vec<u8> {
    let mut tp = TP.to_vec();
    tp[index] ^= mask;
    let last = tp.len() - 1;
    tp[last] = tp[..last].iter().fold(0xff, |xor, b| xor ^ b);
    tp
}

#[test]
fn echo_matches_exact_tp1_except_native_repeat_and_not_cemi_ack_bits() {
    let mut f = frame();
    f.control = Some(FrameControl {
        ack_request: true,
        ..f.effective_control()
    });
    assert_eq!(
        codec::encode(&f).unwrap(),
        WIRE,
        "strip cEMI A/C before forming expected TP1"
    );
    assert!(codec::matches_echo(TP, TP));
    assert!(codec::matches_echo(TP, &changed_tp(0, 0x20)));
    // Priority, source, destination, hop count, TPCI and APCI/payload are exact.
    for (index, mask) in [
        (0, 4),
        (1, 1),
        (2, 1),
        (3, 1),
        (4, 1),
        (5, 0x10),
        (6, 4),
        (7, 1),
    ] {
        assert!(
            !codec::matches_echo(TP, &changed_tp(index, mask)),
            "index {index}"
        );
    }
    assert!(!codec::matches_echo(TP, &TP[..TP.len() - 1]));
}

#[tokio::test(start_paused = true)]
async fn first_and_second_send_reject_missing_or_wrong_echo_before_positive_con() {
    for second_send in [false, true] {
        // None: missing; the rest: unrelated source/destination/payload/priority/hop.
        for wrong in [
            None,
            Some((1, 1)),
            Some((4, 1)),
            Some((7, 1)),
            Some((0, 4)),
            Some((5, 0x10)),
        ] {
            let (client, mut peer) = setup(TpuartMode::Active).await;
            if second_send {
                let (result, ()) = tokio::join!(client.send(destination(), service()), async {
                    bytes(&mut peer, WIRE).await;
                    peer.write_all(&[TP, &[0x8b]].concat()).await.unwrap();
                });
                result.unwrap();
            }
            let mut events = client.subscribe();
            let mut sent = Box::pin(client.send(destination(), service()));
            let (held, ()) = tokio::join!(
                timeout(Duration::from_millis(1), &mut sent),
                bytes(&mut peer, WIRE)
            );
            assert!(held.is_err(), "send must actually be pending");
            if let Some((index, mask)) = wrong {
                peer.write_all(&changed_tp(index, mask)).await.unwrap();
                assert!(matches!(
                    events.recv().await.unwrap(),
                    TunnelEvent::Telegram(_)
                ));
            }
            peer.write_all(&[0x8b]).await.unwrap();
            let error = sent.await.unwrap_err().to_string();
            assert!(
                error.contains("without complete matching transmission echo"),
                "{error}"
            );
            bytes(&mut peer, &[1]).await;
            peer.write_all(&[3]).await.unwrap();
            assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
            assert!(client.send(destination(), service()).await.is_err());
            assert!(peer.read_u8().await.is_err(), "no application retry");
        }
    }
}

#[tokio::test(start_paused = true)]
async fn delayed_old_positive_con_before_second_echo_is_terminal_not_success() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let (first, ()) = tokio::join!(client.send(destination(), service()), async {
        bytes(&mut peer, WIRE).await;
        peer.write_all(&[TP, &[0x8b]].concat()).await.unwrap();
    });
    first.unwrap();
    let mut events = client.subscribe();
    let mut second = Box::pin(client.send(destination(), service()));
    let (held, ()) = tokio::join!(
        timeout(Duration::from_millis(1), &mut second),
        bytes(&mut peer, WIRE)
    );
    assert!(held.is_err());
    // An old 8B arrives first, followed even by a matching echo and another con.
    peer.write_all(&[&[0x8b], TP, &[0x8b]].concat())
        .await
        .unwrap();
    assert!(second
        .await
        .unwrap_err()
        .to_string()
        .contains("without complete matching"));
    bytes(&mut peer, &[1]).await;
    peer.write_all(&[3]).await.unwrap();
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(client.send(destination(), service()).await.is_err());
    assert!(peer.read_u8().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn conforming_echo_repeat_then_fresh_con_confirms_both_sends() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    for echo in [TP.to_vec(), changed_tp(0, 0x20)] {
        let mut events = client.subscribe();
        let mut sent = Box::pin(client.send(destination(), service()));
        let (held, ()) = tokio::join!(
            timeout(Duration::from_millis(1), &mut sent),
            bytes(&mut peer, WIRE)
        );
        assert!(held.is_err());
        // Unrelated valid traffic must not prevent a subsequent genuine echo.
        peer.write_all(&changed_tp(4, 1)).await.unwrap();
        assert!(matches!(
            events.recv().await.unwrap(),
            TunnelEvent::Telegram(_)
        ));
        peer.write_all(&echo[..4]).await.unwrap();
        assert!(timeout(Duration::from_millis(1), &mut sent).await.is_err());
        peer.write_all(&echo[4..]).await.unwrap();
        assert!(matches!(
            events.recv().await.unwrap(),
            TunnelEvent::Telegram(_)
        ));
        assert!(timeout(Duration::from_millis(1), &mut sent).await.is_err());
        peer.write_all(&[0x8b]).await.unwrap();
        sent.await.unwrap();
    }
    close(&client, &mut peer).await;
}

#[tokio::test(start_paused = true)]
async fn confirmed_delivery_is_preserved_when_cleanup_reset_times_out() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let (sent, ()) = tokio::join!(client.send(destination(), service()), async {
        bytes(&mut peer, WIRE).await;
        peer.write_all(&[TP, &[0x8b]].concat()).await.unwrap();
    });
    sent.unwrap();
    let mut events = client.subscribe();
    let error = client.disconnect().await.unwrap_err().to_string();
    assert!(error.contains("auto-ACK state unknown"));
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    bytes(&mut peer, &[1]).await;
    assert!(peer.read_u8().await.is_err());
}

#[tokio::test(start_paused = true)]
async fn complete_matching_echo_without_confirmation_still_times_out_and_closes() {
    let (client, mut peer) = setup(TpuartMode::Active).await;
    let mut events = client.subscribe();
    let (sent, ()) = tokio::join!(client.send(destination(), service()), async {
        bytes(&mut peer, WIRE).await;
        peer.write_all(TP).await.unwrap();
        bytes(&mut peer, &[1]).await;
        peer.write_all(&[3]).await.unwrap();
    });
    assert!(matches!(sent, Err(BusError::Timeout)));
    assert!(matches!(
        events.recv().await.unwrap(),
        TunnelEvent::Telegram(_)
    ));
    assert_eq!(events.recv().await.unwrap(), TunnelEvent::Closed);
    assert!(client.send(destination(), service()).await.is_err());
    assert!(peer.read_u8().await.is_err());
}
