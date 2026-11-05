//! Local event loss is fatal while waiting through LoadCompleting, including re-establishment.

use super::*;
use std::cell::Cell;

const EVENT_CAPACITY: usize = 8;

#[derive(Clone, Copy)]
enum Boundary {
    QuietPoll,
    Reconnect,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Counts {
    connects: u32,
    polls: u32,
    disconnects: u32,
    subscriptions: u32,
    bursts: u32,
}

/// The configured simulator answers synchronously. Drain each send's real events before
/// returning, then inject harmless confirmations at exactly read #2 or T_Connect #2.
/// No forwarding task, timer or scheduler race separates subscribe, send and overflow.
struct BoundedEvents<'d> {
    device: &'d SimulatedDevice,
    events: broadcast::Sender<TunnelEvent>,
    boundary: Boundary,
    overflow: bool,
    counts: Cell<Counts>,
}

impl ManagementTransport for BoundedEvents<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        self.device.assigned_address()
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        let mut counts = self.counts.get();
        counts.subscriptions += 1;
        self.counts.set(counts);
        self.events.subscribe()
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let mut counts = self.counts.get();
        let connecting = transport == Tpci::Connect;
        let polling = matches!(transport, Tpci::NumberedData { .. })
            && matches!(
                &service,
                ApplicationService::PropertyValueRead {
                    object_index,
                    property_id: PID_LOAD_STATE_CONTROL,
                    ..
                } if *object_index == ObjectIndex::APPLICATION_PROGRAM.octet()
            );
        counts.connects += u32::from(connecting);
        counts.polls += u32::from(polling);
        counts.disconnects += u32::from(transport == Tpci::Disconnect);
        let inject = counts.bursts == 0
            && match self.boundary {
                Boundary::QuietPoll => polling && counts.polls == 2,
                Boundary::Reconnect => connecting && counts.connects == 2,
            };
        self.counts.set(counts);

        let mut inner = self.device.subscribe();
        self.device
            .send_frame(destination, transport, service)
            .await?;
        loop {
            match inner.try_recv() {
                Ok(event) => {
                    self.events
                        .send(event)
                        .expect("the session subscribed before sending");
                }
                Err(broadcast::error::TryRecvError::Empty) => break,
                other => panic!("the synchronous simulator stream must stay intact: {other:?}"),
            }
        }

        if inject {
            counts.bursts += 1;
            self.counts.set(counts);
            // Neither a disconnect nor an answer: without loss these are ignored by both
            // receiving paths. Keep the sender alive, so Closed cannot explain the result.
            let noise = TunnelEvent::Telegram(LDataFrame {
                kind: LDataMessageKind::Confirmation { error: false },
                source: self.assigned_address(),
                destination,
                transport: Tpci::UnnumberedData,
                service: ApplicationService::NoApplicationPdu,
                control: None,
            });
            let burst = if self.overflow { EVENT_CAPACITY + 1 } else { 1 };
            for _ in 0..burst {
                self.events
                    .send(noise.clone())
                    .expect("the active session receives the burst");
            }
        }
        Ok(())
    }

    fn target_kind(&self) -> TargetKind {
        self.device.target_kind()
    }
}

struct Outcome {
    result: Result<LoadState, SessionError>,
    counts: Counts,
    reconnects: u32,
    device_reads: u32,
    device_connects: usize,
}

async fn wait(boundary: Boundary, overflow: bool) -> Outcome {
    let config = match boundary {
        Boundary::QuietPoll => SimulatorConfig {
            // One ACK-only poll on the still-live connection.
            unanswered_load_state_reads: Some(2..3),
            ..SimulatorConfig::default()
        },
        Boundary::Reconnect => SimulatorConfig {
            // Four unacknowledged transmissions cause the real session's TL release.
            unacknowledged_load_state_reads: Some(one_unacknowledged_poll()),
            ..SimulatorConfig::default()
        },
    };
    let device = SimulatedDevice::with_config(config);
    completing_once(&device);
    let (events, _) = broadcast::channel(EVENT_CAPACITY);
    let transport = BoundedEvents {
        device: &device,
        events,
        boundary,
        overflow,
        counts: Cell::new(Counts::default()),
    };
    let mut session = ManagementSession::read_only(
        &transport,
        device.address(),
        AuthorisationPlan::Skip,
        SessionTiming {
            max_transition: Duration::from_secs(30),
            ..fast()
        },
    )
    .expect("the simulated device is contactable");
    session
        .connect()
        .await
        .expect("the initial connection is healthy");
    let outcomes = permitted_outcomes(
        LoadState::Loading,
        Stimulus::Event(LoadEvent::LoadCompleted),
        None,
    );
    // No supplied last-state seed: the first real poll must earn silence permission by
    // reading LoadCompleting. Paused time bounds legitimate quiet waits without real sleeps.
    let result = session
        .wait_for_load_state(
            ObjectIndex::APPLICATION_PROGRAM,
            LoadEvent::LoadCompleted,
            LoadState::Loading,
            None,
            &outcomes,
        )
        .await;
    Outcome {
        result,
        counts: transport.counts.get(),
        reconnects: session.reconnects(),
        device_reads: device.load_state_reads(),
        device_connects: connects(&device),
    }
}

fn assert_counts(
    outcome: &Outcome,
    polls: u32,
    connects: u32,
    subscriptions: u32,
    disconnects: u32,
) {
    assert_eq!(
        outcome.counts,
        Counts {
            polls,
            connects,
            subscriptions,
            disconnects,
            bursts: 1
        },
        "every outgoing poll/reconnect and subscription is counted, including failed attempts"
    );
    assert_eq!(
        outcome.device_reads, polls,
        "the real simulator received every poll"
    );
    assert_eq!(outcome.device_connects, connects as usize);
    assert_eq!(outcome.reconnects, connects - 1);
}

#[tokio::test(start_paused = true)]
async fn lagged_during_a_quiet_poll_is_fatal_without_another_poll_or_reconnect() {
    let outcome = wait(Boundary::QuietPoll, true).await;
    assert!(
        matches!(
            outcome.result,
            Err(SessionError::Lagged {
                waiting_for: "A_PropertyValue_Response"
            })
        ),
        "local event loss must not become device silence or a transition timeout: {:?}",
        outcome.result
    );
    // Read #1 reported LoadCompleting; read #2 was ACK-only and overflowed before recv.
    assert_counts(&outcome, 2, 1, 3, 0);
}

#[tokio::test(start_paused = true)]
async fn a_quiet_poll_without_event_loss_keeps_polling_on_the_same_connection() {
    let outcome = wait(Boundary::QuietPoll, false).await;
    assert!(
        matches!(outcome.result, Ok(LoadState::Loaded)),
        "{:?}",
        outcome.result
    );
    // The same adapter/boundary, one harmless event: the ACK-only read is tolerated.
    assert_counts(&outcome, 3, 1, 4, 0);
}

#[tokio::test(start_paused = true)]
async fn lagged_during_reestablishment_is_fatal_without_another_poll_or_reconnect() {
    let outcome = wait(Boundary::Reconnect, true).await;
    assert!(
        matches!(
            outcome.result,
            Err(SessionError::Lagged {
                waiting_for: "positive L_Data.con for T_Connect"
            })
        ),
        "re-establishment must not retry local event loss: {:?}",
        outcome.result
    );
    // Read #1 reported LoadCompleting; #2-#5 exhausted TL; only T_Connect #2 overflows.
    assert_counts(&outcome, 5, 2, 4, 1);
}

#[tokio::test(start_paused = true)]
async fn reestablishment_without_event_loss_resumes_polling_after_a_quiet_release() {
    let outcome = wait(Boundary::Reconnect, false).await;
    assert!(
        matches!(outcome.result, Ok(LoadState::Loaded)),
        "{:?}",
        outcome.result
    );
    // The same release and adapter/boundary, no overflow: read #6 settles the transition.
    assert_counts(&outcome, 6, 2, 5, 1);
}
