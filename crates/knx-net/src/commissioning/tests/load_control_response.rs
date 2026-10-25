//! The state a load-control write is answered with is an observation, and the wait honours it.

use super::*;
use crate::commissioning::simulator::LoadControlResponse;

/// Far apart on purpose: a poll, a ladder and the whole transition are separate magnitudes.
fn timing() -> SessionTiming {
    SessionTiming {
        connection_timeout: Duration::from_millis(100),
        response_timeout: Duration::from_millis(50),
        poll_interval: Duration::from_millis(20),
        max_transition: Duration::from_millis(1500),
        ..fast()
    }
}

fn session<'t>(device: &'t SimulatedDevice) -> ManagementSession<'t, SimulatedDevice> {
    ManagementSession::authorised(
        device,
        AuthorisationPlan::Skip,
        timing(),
        simulator_authorisation(device, WriteScope::Download),
    )
    .expect("a simulator authorisation against a simulator transport is accepted")
}

fn device_with(config: SimulatorConfig, state: LoadState) -> SimulatedDevice {
    let device = SimulatedDevice::with_config(config);
    device.preset_load_state(ObjectIndex::APPLICATION_PROGRAM, state);
    device
}

/// The octets of the device's answer to the load-control write, read off the wire.
///
/// The device's own frames are the evidence, not what the session decoded. Answers to this
/// property come in order: the read before the event, the write's answer, then the polls.
fn write_answer(events: &mut broadcast::Receiver<TunnelEvent>) -> Option<Vec<u8>> {
    let mut answers = Vec::new();
    while let Ok(event) = events.try_recv() {
        if let TunnelEvent::Telegram(LDataFrame {
            service:
                ApplicationService::PropertyValueResponse {
                    property_id: PID_LOAD_STATE_CONTROL,
                    data,
                    ..
                },
            ..
        }) = event
        {
            answers.push(data);
        }
    }
    answers.into_iter().nth(1)
}

fn load_event_writes(device: &SimulatedDevice) -> usize {
    device
        .seen()
        .iter()
        .filter(|entry| {
            matches!(
                entry,
                Seen::PropertyWrite {
                    property_id: PID_LOAD_STATE_CONTROL,
                    ..
                }
            )
        })
        .count()
}

async fn load_completed(
    device: &SimulatedDevice,
) -> (Result<LoadState, SessionError>, Option<Vec<u8>>) {
    let mut events = device.subscribe();
    let mut session = session(device);
    session.connect().await.expect("connect");
    let result = session
        .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::LoadCompleted)
        .await;
    (result, write_answer(&mut events))
}

fn completing_then_quiet(quiet: SimulatorConfig) -> SimulatorConfig {
    SimulatorConfig {
        load_completing_polls: 1,
        load_control_response: LoadControlResponse::CompletingWhilePending,
        ..quiet
    }
}

/// RES §4.23.2.3.2: a device that goes offline in `LoadCompleting` says so in the write answer.
/// The first poll then meets an offline device, and the state is already known.
#[tokio::test]
async fn an_offline_device_after_a_completing_answer_is_waited_for() {
    for (label, window) in [
        ("unacknowledged", (Some(2..6), None)),
        ("unacknowledged, longer", (Some(2..10), None)),
        ("acknowledged, unanswered", (None, Some(2..3))),
    ] {
        let device = device_with(
            completing_then_quiet(SimulatorConfig {
                unacknowledged_load_state_reads: window.0,
                unanswered_load_state_reads: window.1,
                ..SimulatorConfig::default()
            }),
            LoadState::Loading,
        );
        let (result, answers) = load_completed(&device).await;
        assert_eq!(
            answers,
            Some(vec![LoadState::LoadCompleting.octet()]),
            "{label}: precondition, the device's own answer carries LoadCompleting"
        );
        assert_eq!(result.expect(label), LoadState::Loaded, "{label}");
        assert_eq!(load_event_writes(&device), 1, "{label}: sent once");
    }
}

/// The device never comes back: the wait ends at the deadline and reports what it saw.
#[tokio::test]
async fn a_device_that_stays_offline_times_out_with_the_answered_state() {
    let device = device_with(
        completing_then_quiet(SimulatorConfig {
            unacknowledged_load_state_reads: Some(2..u32::MAX),
            unanswered_connects: Some(2..u32::MAX),
            ..SimulatorConfig::default()
        }),
        LoadState::Loading,
    );
    let (result, answers) = load_completed(&device).await;
    assert_eq!(answers, Some(vec![LoadState::LoadCompleting.octet()]));
    match result {
        Err(SessionError::TransitionTimedOut {
            last_state: Some(LoadState::LoadCompleting),
            ..
        }) => {}
        other => panic!("expected a time-out holding LoadCompleting, got {other:?}"),
    }
    assert_eq!(load_event_writes(&device), 1, "the event is never re-sent");
}

/// Without a completing answer nothing licenses silence: the first poll's failure stands.
#[tokio::test]
async fn an_unchanged_answer_creates_no_permission_to_be_silent() {
    for response in [
        LoadControlResponse::ResultingState,
        LoadControlResponse::Octets(vec![LoadState::Loading.octet()]),
    ] {
        let device = device_with(
            SimulatorConfig {
                load_control_response: response.clone(),
                unacknowledged_load_state_reads: Some(2..u32::MAX),
                drop_load_state_writes: true,
                ..SimulatorConfig::default()
            },
            LoadState::Loading,
        );
        let (result, _) = load_completed(&device).await;
        assert!(
            matches!(result, Err(SessionError::ConnectionReleased { .. })),
            "{response:?}: {result:?}"
        );
    }
}

/// An unchanged answer is not a verdict either way: the machine may still move.
#[tokio::test]
async fn an_unchanged_answer_stays_pending_until_a_poll_settles_it() {
    let device = device_with(
        SimulatorConfig {
            load_control_response: LoadControlResponse::Octets(vec![LoadState::Loading.octet()]),
            ..SimulatorConfig::default()
        },
        LoadState::Loading,
    );
    let (result, _) = load_completed(&device).await;
    assert_eq!(result.expect("settles on the poll"), LoadState::Loaded);
}

/// A settled answer is still checked by the verification poll.
#[tokio::test]
async fn a_settled_answer_keeps_its_verification_poll() {
    let device = device_with(SimulatorConfig::default(), LoadState::Loading);
    let (result, _) = load_completed(&device).await;
    assert_eq!(result.expect("loaded"), LoadState::Loaded);
    assert_eq!(device.load_state_reads(), 2, "the read before and one poll");
}

/// A wrong answer is the error, before a healthy poll can paper over it.
#[tokio::test]
async fn an_invalid_answer_is_rejected_before_any_poll() {
    let cases: [(&str, Vec<u8>, LoadState); 4] = [
        ("empty", vec![], LoadState::Loading),
        ("two octets", vec![5, 5], LoadState::Loading),
        ("unknown state", vec![0x77], LoadState::Loading),
        (
            "wrong transition",
            vec![LoadState::Unloaded.octet()],
            LoadState::Loading,
        ),
    ];
    for (label, octets, before) in cases {
        let device = device_with(
            SimulatorConfig {
                load_control_response: LoadControlResponse::Octets(octets),
                ..SimulatorConfig::default()
            },
            before,
        );
        let (result, _) = load_completed(&device).await;
        match (label, result) {
            ("empty" | "two octets", Err(SessionError::MalformedProperty { .. }))
            | ("unknown state", Err(SessionError::UnknownLoadState(_)))
            | ("wrong transition", Err(SessionError::IllegalTransition { .. })) => {}
            (_, other) => panic!("{label}: {other:?}"),
        }
        assert_eq!(device.load_state_reads(), 1, "{label}: no poll followed");
    }
}

/// The mask profile narrows Table 94 for the answer exactly as it does for a poll.
#[tokio::test]
async fn the_mask_restriction_applies_to_the_answer() {
    for (mask, rejected) in [(Some(MaskVersion::COUPLER_0912), true), (None, false)] {
        let device = device_with(
            SimulatorConfig {
                load_control_response: LoadControlResponse::Octets(vec![LoadState::Error.octet()]),
                ..SimulatorConfig::default()
            },
            LoadState::Loaded,
        );
        let mut session = session(&device);
        if let Some(mask) = mask {
            session.adopt_mask(mask);
        }
        session.connect().await.expect("connect");
        let result = session
            .write_load_event(ObjectIndex::APPLICATION_PROGRAM, LoadEvent::LoadCompleted)
            .await;
        match (rejected, &result) {
            (true, Err(SessionError::IllegalTransition { .. })) => {
                assert_eq!(device.load_state_reads(), 1);
            }
            (false, Ok(observed)) => {
                assert_eq!(
                    *observed,
                    LoadState::Error,
                    "the allowed Error response is confirmed by the verification poll"
                );
                assert!(device.load_state_reads() > 1);
            }
            _ => panic!("{mask:?}: {result:?}"),
        }
    }
}
