//! The System2 patient session timing against a synthetic device whose answers come late or never.

use super::*;
use crate::cemi::{LDataMessageKind, Tpci};
use crate::client::{BusError, TunnelEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

/// Forwards the simulated device and holds back its numbered-data answers: delays them by `delay`,
/// or drops them while `mute` is set. T_ACKs and everything else pass at once. In-process only, so
/// the paused Tokio clock is exact here.
struct LateAnswers<'d> {
    device: &'d SimulatedDevice,
    events: broadcast::Sender<TunnelEvent>,
}

impl<'d> LateAnswers<'d> {
    fn new(device: &'d SimulatedDevice, delay: Duration, mute: Arc<AtomicBool>) -> Self {
        Self::with_duplicate_acks(device, delay, mute, false)
    }

    fn with_duplicate_acks(
        device: &'d SimulatedDevice,
        delay: Duration,
        mute: Arc<AtomicBool>,
        duplicate_acks: bool,
    ) -> Self {
        let (events, _) = broadcast::channel(64);
        let mut inner = device.subscribe();
        let forward = events.clone();
        tokio::spawn(async move {
            while let Ok(event) = inner.recv().await {
                let answer = matches!(
                    &event,
                    TunnelEvent::Telegram(frame)
                        if frame.kind == LDataMessageKind::Indication
                            && matches!(frame.transport, Tpci::NumberedData { .. })
                );
                if !answer {
                    if duplicate_acks
                        && mute.load(Ordering::SeqCst)
                        && matches!(&event, TunnelEvent::Telegram(frame) if matches!(frame.transport, Tpci::Ack { .. }))
                    {
                        let forward = forward.clone();
                        let repeated = event.clone();
                        tokio::spawn(async move {
                            for _ in 0..8 {
                                tokio::time::sleep(s(10)).await;
                                let _ = forward.send(repeated.clone());
                            }
                        });
                    }
                    let _ = forward.send(event);
                } else if !mute.load(Ordering::SeqCst) {
                    let forward = forward.clone();
                    tokio::spawn(async move {
                        tokio::time::sleep(delay).await;
                        let _ = forward.send(event);
                    });
                }
            }
        });
        Self { device, events }
    }
}

impl ManagementTransport for LateAnswers<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        self.device.assigned_address()
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.events.subscribe()
    }

    async fn send_frame(
        &self,
        destination: crate::cemi::Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        self.device
            .send_frame(destination, transport, service)
            .await
    }

    fn target_kind(&self) -> TargetKind {
        self.device.target_kind()
    }
}

fn s(whole: u64) -> Duration {
    Duration::from_secs(whole)
}

/// Literals on purpose: the expected figures must not come from the constants under test.
#[test]
fn the_patient_system2_profile_stretches_only_the_answer_and_transition_waits() {
    let patient = SessionTiming::patient_system2();
    assert_eq!(
        patient.response_timeout,
        s(3),
        "the transport ACK timer is not stretched"
    );
    assert_eq!(patient.answer_timeout, Some(s(15)));
    assert_eq!(patient.max_transition, s(120));
    assert_eq!(
        SessionTiming {
            answer_timeout: None,
            max_transition: s(30),
            ..patient
        },
        SessionTiming::default(),
        "every other field is the default"
    );
    let default = SessionTiming::default();
    assert_eq!(default.response_timeout, s(3));
    assert_eq!(default.answer_timeout, None);
    assert_eq!(default.max_transition, s(30));
    assert_eq!(patient.connection_timeout, s(6));
    assert_eq!(patient.poll_interval, s(3));
    assert_eq!(patient.programming_delay, Duration::from_millis(500));
    assert_eq!(patient.restart_basic_t1, s(1));
    assert_eq!(patient.restart_responsive_again, s(5));
    assert_eq!(patient.post_restart_disconnect_wait, s(6));
    assert_eq!(patient.programming_mode_broadcast_timeout, s(1));
}

/// A device that answers 20 s after the acknowledged request: beyond the default's four 3 s slices
/// (12 s), inside the patient profile's second 15 s slice.
#[tokio::test(start_paused = true)]
async fn an_answer_beyond_the_default_full_wait_is_accepted_with_the_patient_profile() {
    let device = SimulatedDevice::new();
    let late = LateAnswers::new(&device, s(20), Arc::new(AtomicBool::new(false)));
    let mut patient = ManagementSession::read_only(
        &late,
        device.address(),
        AuthorisationPlan::Skip,
        SessionTiming::patient_system2(),
    )
    .expect("contactable");
    patient.connect().await.expect("the 20 s answer is heard");
    let state = patient
        .read_load_state(ObjectIndex::APPLICATION_PROGRAM)
        .await
        .expect("the 20 s answer is heard");
    assert_eq!(state, LoadState::Unloaded);

    let other = SimulatedDevice::new();
    let late = LateAnswers::new(&other, s(20), Arc::new(AtomicBool::new(false)));
    let mut standard = ManagementSession::read_only(
        &late,
        other.address(),
        AuthorisationPlan::Skip,
        SessionTiming::default(),
    )
    .expect("contactable");
    standard
        .connect()
        .await
        .expect("connect tolerates the late verify answer");
    let error = standard
        .read_load_state(ObjectIndex::APPLICATION_PROGRAM)
        .await
        .expect_err("default patience is 12 s");
    assert!(matches!(error, SessionError::NoAnswer { .. }), "{error:?}");
}

/// After the device's positive T_ACK the write is never sent again, however long the answer wait.
#[tokio::test(start_paused = true)]
async fn an_acknowledged_write_is_not_retransmitted_during_the_extended_answer_wait() {
    let device = SimulatedDevice::new();
    let mute = Arc::new(AtomicBool::new(false));
    let late = LateAnswers::new(&device, Duration::ZERO, mute.clone());
    let mut session = ManagementSession::authorised(
        &late,
        AuthorisationPlan::Skip,
        SessionTiming::patient_system2(),
        simulator_authorisation(&device, WriteScope::Download),
    )
    .expect("a simulator authorisation");
    session.connect().await.expect("connect");
    let before = device_control_writes(&device);
    mute.store(true, Ordering::SeqCst);
    let started = tokio::time::Instant::now();
    let error = session
        .write_property(
            ObjectIndex::DEVICE,
            PID_DEVICE_CONTROL,
            vec![0x04],
            WriteScope::Download,
        )
        .await
        .expect_err("the answer never comes");
    let waited = started.elapsed();
    match error {
        SessionError::NoAnswer { attempts, each, .. } => {
            assert_eq!(attempts, 1, "one transmission");
            assert_eq!(each, s(60), "four 15 s answer slices after the T_ACK");
        }
        other => panic!("expected NoAnswer, got {other:?}"),
    }
    assert!(waited >= s(60) && waited < s(61), "waited {waited:?}");
    assert_eq!(
        device_control_writes(&device) - before,
        1,
        "the acknowledged write reached the device exactly once"
    );
}

/// Duplicate T_ACKs cannot renew the bounded answer deadline or repeat the applied write.
#[tokio::test(start_paused = true)]
async fn duplicate_transport_acks_do_not_extend_the_answer_deadline() {
    let device = SimulatedDevice::new();
    let mute = Arc::new(AtomicBool::new(false));
    let late = LateAnswers::with_duplicate_acks(&device, Duration::ZERO, mute.clone(), true);
    let mut session = ManagementSession::authorised(
        &late,
        AuthorisationPlan::Skip,
        SessionTiming::patient_system2(),
        simulator_authorisation(&device, WriteScope::Download),
    )
    .expect("a simulator authorisation");
    session.connect().await.expect("connect");
    let before = device_control_writes(&device);
    mute.store(true, Ordering::SeqCst);
    let started = tokio::time::Instant::now();
    let error = session
        .write_property(
            ObjectIndex::DEVICE,
            PID_DEVICE_CONTROL,
            vec![0x04],
            WriteScope::Download,
        )
        .await
        .expect_err("duplicate ACKs are not answers");
    assert!(
        matches!(error, SessionError::NoAnswer { attempts: 1, each, .. } if each == s(60)),
        "{error:?}"
    );
    let elapsed = started.elapsed();
    assert!(elapsed >= s(60) && elapsed < s(61), "waited {elapsed:?}");
    assert_eq!(device_control_writes(&device) - before, 1);
}

/// A transition whose first four state reads go unanswered (T_ACK only): under the patient profile
/// that is 60 s of waiting plus polls, past the default's 30 s, and the fifth read still settles it.
/// The same session with the 30 s transition budget gives up.
#[tokio::test(start_paused = true)]
async fn a_transition_beyond_thirty_seconds_is_waited_out_with_the_patient_profile() {
    let wait = |timing: SessionTiming| async move {
        let device = SimulatedDevice::with_config(SimulatorConfig {
            unanswered_load_state_reads: Some(1..5),
            ..SimulatorConfig::default()
        });
        device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, LoadState::Loaded);
        let mut session = ManagementSession::read_only(
            &device,
            device.address(),
            AuthorisationPlan::Skip,
            timing,
        )
        .expect("contactable");
        session.connect().await.expect("connect");
        let outcomes = permitted_outcomes(
            LoadState::Loading,
            Stimulus::Event(LoadEvent::LoadCompleted),
            None,
        );
        let started = tokio::time::Instant::now();
        let result = session
            .wait_for_load_state(
                ObjectIndex::APPLICATION_PROGRAM,
                LoadEvent::LoadCompleted,
                LoadState::Loading,
                Some(LoadState::LoadCompleting),
                &outcomes,
            )
            .await;
        (result, started.elapsed(), device.load_state_reads())
    };
    let (result, elapsed, reads) = wait(SessionTiming::patient_system2()).await;
    assert_eq!(
        result.expect("settled on the fifth read"),
        LoadState::Loaded
    );
    assert_eq!(reads, 5);
    assert!(elapsed >= s(60) && elapsed < s(120), "{elapsed:?}");

    let (result, elapsed, reads) = wait(SessionTiming {
        max_transition: s(30),
        ..SessionTiming::patient_system2()
    })
    .await;
    assert!(
        matches!(result, Err(SessionError::TransitionTimedOut { .. })),
        "{result:?}"
    );
    assert_eq!(
        reads, 3,
        "two polls before the deadline and the one extra attempt"
    );
    assert!(elapsed >= s(30), "{elapsed:?}");
}

/// The device's transport ACK is a 3 s timer even under the patient profile: a request nobody
/// acknowledged is repeated at 3 s, 6 s and 9 s and the connection is released at 12 s. The 15 s
/// patience only starts once a T_ACK has arrived.
#[tokio::test(start_paused = true)]
async fn an_unacknowledged_request_still_repeats_every_three_seconds_under_the_patient_profile() {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        // No T_ACK and no answer for any read.
        unacknowledged_load_state_reads: Some(1..u32::MAX),
        ..SimulatorConfig::default()
    });
    let mut session = ManagementSession::read_only(
        &device,
        device.address(),
        AuthorisationPlan::Skip,
        SessionTiming::patient_system2(),
    )
    .expect("contactable");
    session.connect().await.expect("connect");
    let started = tokio::time::Instant::now();
    let probe = async {
        let mut seen = Vec::new();
        for ms in [2_900u64, 3_200, 5_900, 6_200, 8_900, 9_200] {
            tokio::time::sleep(Duration::from_millis(ms) - started.elapsed()).await;
            seen.push(device.load_state_reads());
        }
        seen
    };
    let (result, seen) = tokio::join!(
        session.read_load_state(ObjectIndex::APPLICATION_PROGRAM),
        probe
    );
    assert_eq!(
        seen,
        [1, 2, 2, 3, 3, 4],
        "reads just before and after each 3 s mark"
    );
    match result.expect_err("nothing acknowledges the request") {
        SessionError::ConnectionReleased { attempts, .. } => assert_eq!(attempts, 4),
        other => panic!("expected the connection to be released, got {other:?}"),
    }
    let waited = started.elapsed();
    assert!(waited >= s(12) && waited < s(13), "{waited:?}");
    assert_eq!(
        device.load_state_reads(),
        4,
        "four transmissions and no more"
    );
}
