//! Verifies descriptor-qualified exact0700 observation, stops and unchanged legacy wire schedules.

use super::*;
use crate::cemi::{decode_l_data, encode_l_data, LDataFrame, LDataMessageKind};

fn qualified(state: u8) -> SimulatedDevice {
    let d = device(0x0700, state, SimulatorConfig::default());
    d.preset_property_description(3, 6, 0x37, [0, 0, 1, 0]);
    d
}

#[tokio::test]
async fn exact0700_net_display_names_optional_description_problem_without_absence_claim() {
    let d = qualified(1);
    d.preset_property_description(3, 6, 9, [0x40, 0, 0, 0]);
    let text = observe(&d, &plan_for(0x0700)).await.to_string();
    assert!(
        text.contains("optional run-state property not qualified"),
        "{text}"
    );
    assert!(text.contains("unspecified description problem"), "{text}");
    assert!(!text.contains("property absent"));
    assert!(!text.contains("run state running"));
}

fn description() -> Seen {
    Seen::PropertyDescriptionRead {
        object_index: 3,
        property_id: 6,
        property_index: 0,
    }
}

fn schedule(value: bool) -> Vec<Seen> {
    let mut frames = vec![read(3, 1), read(3, 13), description()];
    if value {
        frames.push(read(3, 6));
    }
    frames.push(Seen::Disconnect);
    frames
}

#[tokio::test]
async fn exact0700_six_states_and_complete_undefined_octets_follow_qualified_description() {
    for (octet, expected) in [
        (0, RunStateObservation::Observed(RunState::Halted)),
        (1, RunStateObservation::Observed(RunState::Running)),
        (2, RunStateObservation::Observed(RunState::Ready)),
        (3, RunStateObservation::Observed(RunState::Terminated)),
        (4, RunStateObservation::Observed(RunState::Starting)),
        (5, RunStateObservation::Observed(RunState::ShuttingDown)),
        (6, RunStateObservation::UndefinedValue),
        (0x80, RunStateObservation::UndefinedValue),
        (0x81, RunStateObservation::UndefinedValue),
        (0xff, RunStateObservation::UndefinedValue),
    ] {
        let d = qualified(octet);
        assert_eq!(
            run_state_of(&observe(&d, &plan_for(0x0700)).await),
            expected
        );
        assert_eq!(runtime_frames(&d), schedule(true));
        assert_eq!(d.property_read_requests().last(), Some(&(3, 6, 1, 1)));
        assert_eq!(memory_reads(&d.seen()), vec![0xb6ea, 0xb6ec]);
        assert_read_only_and_closed(&d, 0);
    }
}

#[tokio::test]
async fn exact0700_qualification_precedence_and_legacy_bit_never_grant_access() {
    use RunStateNotQualified::*;
    let mut cases = vec![
        ([0, 0, 0, 0], Some(DescriptionProblem)),
        ([0x80, 0, 0, 0], Some(IndeterminateMetadata)),
        ([0, 0, 0, 1], Some(IndeterminateMetadata)),
        ([2, 0, 0, 0], Some(IndeterminateMetadata)),
        ([0, 0, 1, 0], None),
        ([0x80, 0x0f, 0xff, 0xff], None),
    ];
    for pdt in 1..=63 {
        cases.push(([pdt, 0, 1, 0], Some(UnsupportedDatatype { pdt })));
    }
    for upper in 1..=15 {
        for lower in [0, 1] {
            for pdt in [0, 2, 63] {
                cases.push(([pdt, upper << 4, lower, 0], Some(UnqualifiedElements)));
            }
        }
    }
    for (raw, reason) in cases {
        for legacy in [0, 0x40] {
            let d = qualified(1);
            let mut raw = raw;
            raw[0] |= legacy;
            d.preset_property_description(3, 6, 0x29, raw);
            let observation = observe(&d, &plan_for(0x0700)).await;
            assert_eq!(
                run_state_of(&observation),
                reason
                    .map(RunStateObservation::NotQualified)
                    .unwrap_or(RunStateObservation::Observed(RunState::Running)),
                "{raw:02x?}"
            );
            assert_eq!(runtime_frames(&d), schedule(reason.is_none()), "{raw:02x?}");
            assert_read_only_and_closed(&d, 0);
        }
    }
}

#[tokio::test]
async fn exact0700_one_byte_value_without_description_does_not_qualify() {
    let d = device(0x0700, 1, SimulatorConfig::default());
    assert_eq!(
        run_state_of(&observe(&d, &plan_for(0x0700)).await),
        unavailable(RunStateStep::Description, RunStateCause::NoCorrelatedAnswer)
    );
    assert_eq!(runtime_frames(&d), schedule(false));
    assert_read_only_and_closed(&d, 0);
}

#[tokio::test]
async fn exact0700_candidate_failures_block_description_and_every_later_read() {
    for case in failures().into_iter().filter(|c| c.pid != 6) {
        let d = device(
            0x0700,
            1,
            SimulatorConfig {
                property_read_fault: Some(PropertyReadFault {
                    object_index: 3,
                    property_id: case.pid,
                    from_read: case.from_read,
                    fault: case.fault,
                }),
                ..SimulatorConfig::default()
            },
        );
        d.preset_property_description(3, 6, 0, [0, 0, 1, 0]);
        let observation = observe(&d, &plan_for(0x0700)).await;
        assert_eq!(run_state_of(&observation), case.expected, "{}", case.label);
        assert!(!d.seen().contains(&description()), "{}", case.label);
        assert!(!d.property_read_requests().iter().any(|r| r.1 == 6));
        assert_read_only(&d, 0);
    }
}

#[tokio::test]
async fn exact0700_value_failures_after_description_keep_loads_and_stop() {
    for case in failures().into_iter().filter(|c| c.pid == 6) {
        let d = device(
            0x0700,
            1,
            SimulatorConfig {
                property_read_fault: Some(PropertyReadFault {
                    object_index: 3,
                    property_id: case.pid,
                    from_read: case.from_read,
                    fault: case.fault,
                }),
                ..SimulatorConfig::default()
            },
        );
        d.preset_property_description(3, 6, 0x37, [0, 0, 1, 0]);
        let o = observe(&d, &plan_for(0x0700)).await;
        assert_eq!(run_state_of(&o), case.expected, "{}", case.label);
        assert_eq!(o.load_verdict(), LoadStateVerdict::MatchesPlan);
        assert_eq!(d.property_read_requests().last(), Some(&(3, 6, 1, 1)));
        let frames = runtime_frames(&d);
        assert_eq!(&frames[..3], &schedule(true)[..3], "{}", case.label);
        assert_read_only(&d, 0);
    }
}

#[tokio::test]
async fn exact0700_identity_failure_blocks_load_candidate_description_and_value() {
    for (object, pid, value) in [
        (0, PID_MANUFACTURER_ID, vec![0, 2]),
        (0, PID_HARDWARE_TYPE, vec![0; 7]),
        (3, PID_PROGRAM_VERSION, vec![0; 5]),
        (3, PID_PROGRAM_VERSION, APPLICATION[..4].to_vec()),
    ] {
        let d = qualified(1);
        d.preset_property(object, pid, &value);
        let o = observe(&d, &plan_for(0x0700)).await;
        assert!(matches!(o.outcome, ObservationOutcome::IdentityMismatch(_)));
        assert_eq!(memory_reads(&d.seen()), Vec::<u32>::new());
        assert!(!d.seen().contains(&description()));
        assert!(!d
            .property_read_requests()
            .iter()
            .any(|r| r.1 == 1 || r.1 == 6));
        assert_read_only_and_closed(&d, 0);
    }
}

#[tokio::test]
async fn exact0700_local_read_failure_at_each_continuation_step_blocks_every_later_read() {
    let d = qualified(1);
    let baseline = LagAt {
        device: &d,
        nth: 0,
        subscriptions: Mutex::new(0),
    };
    let o = observe_after_download(&baseline, contactable(&d), fast(), &plan_for(0x0700)).await;
    assert_eq!(
        run_state_of(&o),
        RunStateObservation::Observed(RunState::Running)
    );
    let total = *baseline.subscriptions.lock().unwrap();
    for (offset, step, sent) in [
        (3, RunStateStep::ObjectType, 1),
        (2, RunStateStep::Application, 2),
        (1, RunStateStep::Description, 3),
        (0, RunStateStep::RunState, 4),
    ] {
        let d = qualified(1);
        let lag = LagAt {
            device: &d,
            nth: total - offset,
            subscriptions: Mutex::new(0),
        };
        let o = observe_after_download(&lag, contactable(&d), fast(), &plan_for(0x0700)).await;
        assert_eq!(
            run_state_of(&o),
            unavailable(step, RunStateCause::ReadFailed)
        );
        assert_eq!(o.load_verdict(), LoadStateVerdict::MatchesPlan);
        let mut expected = schedule(true)[..sent].to_vec();
        expected.push(Seen::Disconnect);
        assert_eq!(runtime_frames(&d), expected);
        assert_read_only_and_closed(&d, 0);
    }
}

#[tokio::test]
async fn exact0700_load_failure_blocks_candidate_but_differing_success_continues() {
    let d = device(
        0x0700,
        1,
        SimulatorConfig {
            protected_memory: Some((0xb6ec, 0xb6ed)),
            ..SimulatorConfig::default()
        },
    );
    assert_eq!(
        run_state_of(&observe(&d, &plan_for(0x0700)).await),
        RunStateObservation::NotRead(RunStateNotRead::LoadReadFailed)
    );
    assert_eq!(runtime_frames(&d), vec![Seen::Disconnect]);
    assert_read_only_and_closed(&d, 0);
    for load in [LoadState::Unloaded, LoadState::Loading, LoadState::Error] {
        let d = qualified(1);
        preset(&d, ApplicationProgram, load);
        let o = observe(&d, &plan_for(0x0700)).await;
        assert_eq!(o.load_verdict(), LoadStateVerdict::DiffersFromPlan);
        assert_eq!(
            run_state_of(&o),
            RunStateObservation::Observed(RunState::Running)
        );
        assert_eq!(runtime_frames(&d), schedule(true));
    }
}

#[derive(Clone, Copy)]
enum Fault {
    Pass,
    Send,
    Release,
    Disconnect,
    WrongObject,
    PidZero,
    Width,
}

/// Serializes both directions and forwards synchronously: no task or detached receiver.
struct Wire<'a> {
    d: &'a SimulatedDevice,
    fault: Fault,
    events: broadcast::Sender<TunnelEvent>,
    frames: Mutex<Vec<LDataFrame>>,
}
impl<'a> Wire<'a> {
    fn new(d: &'a SimulatedDevice, fault: Fault) -> Self {
        Self {
            d,
            fault,
            events: broadcast::channel(64).0,
            frames: Mutex::new(vec![]),
        }
    }
}
impl ManagementTransport for Wire<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        self.d.assigned_address()
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.events.subscribe()
    }
    fn target_kind(&self) -> TargetKind {
        TargetKind::Simulator
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let frame = LDataFrame {
            kind: LDataMessageKind::Request,
            source: self.assigned_address(),
            destination,
            transport,
            service,
            control: None,
        };
        let frame = decode_l_data(&encode_l_data(&frame).unwrap()).unwrap();
        self.frames.lock().unwrap().push(frame.clone());
        let desc = matches!(
            frame.service,
            ApplicationService::PropertyDescriptionRead { .. }
        );
        if desc && matches!(self.fault, Fault::Send) {
            return Err(BusError::Timeout);
        }
        if desc && matches!(self.fault, Fault::Release) {
            return Ok(());
        }
        let mut received = self.d.subscribe();
        self.d
            .send_frame(destination, transport, frame.service)
            .await?;
        while let Ok(event) = received.try_recv() {
            let event = match event {
                TunnelEvent::Telegram(mut f) => {
                    if desc
                        && matches!(
                            f.service,
                            ApplicationService::PropertyDescriptionResponse { .. }
                        )
                    {
                        match self.fault {
                            Fault::Disconnect => {
                                f.transport = Tpci::Disconnect;
                                f.service = ApplicationService::NoApplicationPdu;
                            }
                            Fault::WrongObject | Fault::PidZero | Fault::Width => {
                                let mut data = vec![3, 6, 0x37, 0, 0, 1, 0];
                                match self.fault {
                                    Fault::WrongObject => data[0] = 5,
                                    Fault::PidZero => data[1] = 0,
                                    Fault::Width => {
                                        data.pop();
                                    }
                                    _ => {}
                                }
                                f.service = ApplicationService::Other { apci: 0x3d9, data };
                            }
                            _ => {}
                        }
                    }
                    TunnelEvent::Telegram(decode_l_data(&encode_l_data(&f).unwrap()).unwrap())
                }
                event => event,
            };
            let _ = self.events.send(event);
        }
        Ok(())
    }
}

#[tokio::test]
async fn exact0700_description_session_errors_stop_without_value_or_reconnect() {
    for (fault, cause) in [
        (Fault::Send, RunStateCause::ReadFailed),
        (Fault::Release, RunStateCause::ConnectionLost),
        (Fault::Disconnect, RunStateCause::ConnectionLost),
        (Fault::WrongObject, RunStateCause::NoCorrelatedAnswer),
        (Fault::PidZero, RunStateCause::NoCorrelatedAnswer),
        (Fault::Width, RunStateCause::NoCorrelatedAnswer),
    ] {
        let d = qualified(1);
        let wire = Wire::new(&d, fault);
        let o = observe_after_download(&wire, contactable(&d), fast(), &plan_for(0x0700)).await;
        assert_eq!(
            run_state_of(&o),
            unavailable(RunStateStep::Description, cause)
        );
        let frames = wire.frames.lock().unwrap();
        assert_eq!(
            frames
                .iter()
                .filter(|f| f.transport == Tpci::Connect)
                .count(),
            1
        );
        assert!(!frames.iter().any(|f| matches!(
            f.service,
            ApplicationService::PropertyValueRead { property_id: 6, .. }
        )));
        assert_read_only(&d, 0);
    }
}

#[tokio::test]
async fn legacy_0701_0705_wire_continuation_stays_literal_without_description() {
    for mask in [0x0701, 0x0705] {
        let d = device(mask, 1, SimulatorConfig::default());
        let wire = Wire::new(&d, Fault::Pass);
        let o = observe_after_download(&wire, contactable(&d), fast(), &plan_for(mask)).await;
        assert_eq!(
            run_state_of(&o),
            RunStateObservation::Observed(RunState::Running)
        );
        let frames = wire.frames.lock().unwrap();
        let requests: Vec<Vec<u8>> = frames
            .iter()
            .filter(|f| {
                matches!(
                    f.service,
                    ApplicationService::PropertyValueRead {
                        object_index: 3,
                        ..
                    }
                )
            })
            .map(|f| encode_l_data(f).unwrap()[9..].to_vec())
            .collect();
        // Mask, manufacturer, hardware, application, two loads occupy send sequences 0..5.
        assert_eq!(
            requests,
            vec![
                vec![0x4f, 0xd5, 3, 13, 0x10, 1],
                vec![0x5b, 0xd5, 3, 1, 0x10, 1],
                vec![0x5f, 0xd5, 3, 13, 0x10, 1],
                vec![0x63, 0xd5, 3, 6, 0x10, 1]
            ]
        );
        assert!(!d.seen().contains(&description()));
    }
}
