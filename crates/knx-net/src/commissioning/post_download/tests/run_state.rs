//! Tests the run-state continuation: exact PID 1, 13, 6 schedule, states and failures.

use knx_core::commissioning::properties::{PID_OBJECT_TYPE, PID_RUN_STATE_CONTROL};

use super::*;
use crate::commissioning::simulator::{PropertyReadFault, ReadFault};

const APPLICATION_TYPE: [u8; 2] = [0x00, 0x03];

/// [`loaded_device`] under another mask, with the candidate object's type and
/// a run state the way a BIM M112 reports them.
fn device(mask: u16, state: u8, config: SimulatorConfig) -> SimulatedDevice {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: mask,
        ..config
    });
    device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
    device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
    device.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
    for machine in MemoryLoadStateMachine::ALL {
        preset(&device, machine, LoadState::Unloaded);
    }
    preset(&device, AddressTable, LoadState::Loaded);
    preset(&device, ApplicationProgram, LoadState::Loaded);
    device.preset_property(3, PID_OBJECT_TYPE, &APPLICATION_TYPE);
    device.preset_property(3, PID_RUN_STATE_CONTROL, &[state]);
    device
}

fn plan_for(mask: u16) -> MemoryDownloadPlan {
    MemoryDownloadPlan {
        mask: MaskVersion(mask),
        ..plan()
    }
}

fn run_state_of(observation: &PostDownloadObservation) -> RunStateObservation {
    match &observation.outcome {
        ObservationOutcome::Observed { run_state, .. } => *run_state,
        other => panic!("not observed: {other:?}"),
    }
}

/// The frames after the last load-state memory read: what the continuation sent.
fn runtime_frames(device: &SimulatedDevice) -> Vec<Seen> {
    let seen = device.seen();
    let last_load_read = seen
        .iter()
        .rposition(|seen| matches!(seen, Seen::MemoryRead { .. }))
        .expect("the load states were read");
    seen[last_load_read + 1..].to_vec()
}

fn read(object_index: u8, property_id: u8) -> Seen {
    Seen::PropertyRead {
        object_index,
        property_id,
    }
}

#[tokio::test]
async fn both_masks_read_pid_1_then_13_then_6_after_the_loads_and_nothing_else() {
    for mask in [0x0701, 0x0705] {
        let device = device(mask, 0x01, SimulatorConfig::default());
        let plan = plan_for(mask);

        let observation = observe(&device, &plan).await;

        assert_eq!(
            run_state_of(&observation),
            RunStateObservation::Observed(RunState::Running),
            "mask {mask:04X}: {observation:?}"
        );
        assert_eq!(
            runtime_frames(&device),
            vec![
                read(3, PID_OBJECT_TYPE),
                read(3, PID_PROGRAM_VERSION),
                read(3, PID_RUN_STATE_CONTROL),
                Seen::Disconnect,
            ],
            "mask {mask:04X}: exactly the three reads, then the ordinary cleanup"
        );
        let requests = device.property_read_requests();
        assert_eq!(
            requests[requests.len() - 3..],
            [
                (3, PID_OBJECT_TYPE, 1, 1),
                (3, PID_PROGRAM_VERSION, 1, 1),
                (3, PID_RUN_STATE_CONTROL, 1, 1),
            ],
            "mask {mask:04X}: literal start and count"
        );
        assert_read_only_and_closed(&device, 0);
    }
}

#[tokio::test]
async fn each_defined_octet_is_its_own_state_and_only_01_is_running() {
    for mask in [0x0701, 0x0705] {
        for (octet, expected) in [
            (0x00, RunState::Halted),
            (0x01, RunState::Running),
            (0x02, RunState::Ready),
            (0x03, RunState::Terminated),
            (0x04, RunState::Starting),
            (0x05, RunState::ShuttingDown),
        ] {
            let device = device(mask, octet, SimulatorConfig::default());
            let observation = observe(&device, &plan_for(mask)).await;
            assert_eq!(
                run_state_of(&observation),
                RunStateObservation::Observed(expected),
                "mask {mask:04X} octet {octet:#04x}"
            );
        }
    }
}

#[tokio::test]
async fn undefined_octets_are_never_masked_into_a_state() {
    for octet in [0x06, 0x80, 0x81, 0xFF] {
        let device = device(0x0701, octet, SimulatorConfig::default());
        let observation = observe(&device, &plan_for(0x0701)).await;
        assert_eq!(
            run_state_of(&observation),
            RunStateObservation::UndefinedValue,
            "octet {octet:#04x}"
        );
    }
}

struct Failure {
    label: String,
    /// The property the fault strikes, and how many continuation reads are sent.
    pid: u8,
    from_read: u32,
    sent: usize,
    fault: ReadFault,
    expected: RunStateObservation,
}

fn unavailable(step: RunStateStep, cause: RunStateCause) -> RunStateObservation {
    RunStateObservation::Unavailable { step, cause }
}

fn failures() -> Vec<Failure> {
    use RunStateCause::{ConnectionLost, Malformed, NoCorrelatedAnswer, Refused};
    use RunStateStep::{Application, ObjectType, RunState};
    let mut cases = Vec::new();
    let mut step = |name: &'static str,
                    step: RunStateStep,
                    pid: u8,
                    from_read: u32,
                    sent: usize,
                    widths: [&'static [u8]; 3]| {
        let mut case = |label: &'static str, fault: ReadFault, expected: RunStateObservation| {
            cases.push(Failure {
                label: format!("{name} {label}"),
                pid,
                from_read,
                sent,
                fault,
                expected,
            });
        };
        case(
            "count 0",
            ReadFault::ElementCount(0),
            unavailable(step, Refused),
        );
        case(
            "count 2",
            ReadFault::ElementCount(2),
            unavailable(step, Malformed),
        );
        for octets in widths {
            case(
                "width",
                ReadFault::Data(octets),
                unavailable(step, Malformed),
            );
        }
        case(
            "echoed object",
            ReadFault::EchoObject(5),
            unavailable(step, NoCorrelatedAnswer),
        );
        case(
            "echoed property",
            ReadFault::EchoProperty(pid + 1),
            unavailable(step, NoCorrelatedAnswer),
        );
        case(
            "echoed start",
            ReadFault::EchoStart(2),
            unavailable(step, NoCorrelatedAnswer),
        );
        case(
            "no answer",
            ReadFault::Unanswered,
            unavailable(step, NoCorrelatedAnswer),
        );
        case(
            "disconnect",
            ReadFault::Disconnect,
            unavailable(step, ConnectionLost),
        );
    };
    step(
        "type",
        ObjectType,
        PID_OBJECT_TYPE,
        1,
        1,
        [&[], &[0x00], &[0x00, 0x03, 0x00]],
    );
    step(
        "application",
        Application,
        PID_PROGRAM_VERSION,
        2,
        2,
        [
            &[],
            &[0x00, 0x83, 0x00, 0x27],
            &[0x00, 0x83, 0x00, 0x27, 0x15, 0x00],
        ],
    );
    step(
        "state",
        RunState,
        PID_RUN_STATE_CONTROL,
        1,
        3,
        [&[], &[0x01, 0x01], &[0x00, 0x01]],
    );
    cases.push(Failure {
        label: "valid other type".to_string(),
        pid: PID_OBJECT_TYPE,
        from_read: 1,
        sent: 1,
        fault: ReadFault::Data(&[0x00, 0x01]),
        expected: RunStateObservation::CandidateMismatch(CandidateMismatch::ObjectType),
    });
    cases.push(Failure {
        label: "valid other application".to_string(),
        pid: PID_PROGRAM_VERSION,
        from_read: 2,
        sent: 2,
        fault: ReadFault::Data(&[0x00, 0x83, 0x00, 0x28, 0x15]),
        expected: RunStateObservation::CandidateMismatch(CandidateMismatch::Application),
    });
    cases
}

#[tokio::test]
async fn the_first_runtime_failure_stops_later_reads_and_keeps_prior_facts() {
    for differs in [false, true] {
        for case in failures() {
            let device = device(
                0x0701,
                0x01,
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
            if differs {
                preset(&device, ApplicationProgram, LoadState::Unloaded);
            }

            let observation = observe(&device, &plan_for(0x0701)).await;

            let label = format!("{} (differs: {differs})", case.label);
            assert_eq!(run_state_of(&observation), case.expected, "{label}");
            let requests = device.property_read_requests();
            let first_identity = requests
                .iter()
                .position(|request| *request == (3, PID_PROGRAM_VERSION, 1, 1))
                .expect("the identity read");
            let schedule = [
                (3, PID_OBJECT_TYPE, 1, 1),
                (3, PID_PROGRAM_VERSION, 1, 1),
                (3, PID_RUN_STATE_CONTROL, 1, 1),
            ];
            assert_eq!(
                requests[first_identity + 1..],
                schedule[..case.sent],
                "{label}"
            );
            let expected_observed = if differs {
                LoadState::Unloaded
            } else {
                LoadState::Loaded
            };
            assert_eq!(
                summary(&observation),
                vec![
                    (
                        AddressTable,
                        Some(LoadState::Loaded),
                        Some(LoadState::Loaded)
                    ),
                    (
                        ApplicationProgram,
                        Some(LoadState::Loaded),
                        Some(expected_observed)
                    ),
                ],
                "{label}"
            );
            assert_eq!(
                observation.load_verdict(),
                if differs {
                    LoadStateVerdict::DiffersFromPlan
                } else {
                    LoadStateVerdict::MatchesPlan
                },
                "{label}"
            );
            let ObservationOutcome::Observed { identity, .. } = &observation.outcome else {
                panic!("{label}: {observation:?}");
            };
            assert_eq!(identity.application, APPLICATION, "{label}");
            assert_read_only(&device, 0);
            if case.fault != ReadFault::Disconnect {
                assert_eq!(device.seen().last(), Some(&Seen::Disconnect), "{label}");
            }
        }
    }
}

#[tokio::test]
async fn an_earlier_load_failure_sends_zero_runtime_frames() {
    let device = device(
        0x0701,
        0x01,
        SimulatorConfig {
            protected_memory: Some((0xB6EC, 0xB6ED)),
            ..SimulatorConfig::default()
        },
    );

    let observation = observe(&device, &plan_for(0x0701)).await;

    assert_eq!(
        run_state_of(&observation),
        RunStateObservation::NotRead(RunStateNotRead::LoadReadFailed)
    );
    let requests = device.property_read_requests();
    assert_eq!(
        requests.iter().filter(|request| request.0 == 3).count(),
        1,
        "only the identity read of object 3: {requests:?}"
    );
    assert_read_only_and_closed(&device, 0);
}

#[tokio::test]
async fn other_070n_masks_read_the_loads_but_no_runtime_frame() {
    for mask in [0x0700, 0x0702, 0x070F] {
        let device = device(mask, 0x01, SimulatorConfig::default());

        let observation = observe(&device, &plan_for(mask)).await;

        assert_eq!(
            run_state_of(&observation),
            RunStateObservation::NotRead(RunStateNotRead::UnsupportedMask),
            "mask {mask:04X}"
        );
        assert_eq!(
            device
                .property_read_requests()
                .iter()
                .filter(|r| r.0 == 3)
                .count(),
            1,
            "mask {mask:04X}: identity only"
        );
        assert_read_only_and_closed(&device, 0);
    }
}

#[tokio::test]
async fn a_candidate_only_at_index_5_is_unsupported_not_discovered() {
    let device = device(0x0701, 0x01, SimulatorConfig::default());
    // The type moves away from the candidate; a full object sits at 5.
    device.preset_property(3, PID_OBJECT_TYPE, &[]);
    device.preset_property(5, PID_OBJECT_TYPE, &APPLICATION_TYPE);
    device.preset_property(5, PID_PROGRAM_VERSION, &APPLICATION);
    device.preset_property(5, PID_RUN_STATE_CONTROL, &[0x01]);

    let observation = observe(&device, &plan_for(0x0701)).await;

    assert_eq!(
        run_state_of(&observation),
        unavailable(RunStateStep::ObjectType, RunStateCause::Malformed),
        "an empty type answer at the candidate"
    );
    assert!(
        device.property_read_requests().iter().all(|r| r.0 != 5),
        "no other object is scanned or guessed"
    );
}

#[tokio::test]
async fn an_absent_type_property_at_the_candidate_is_refused_with_no_search() {
    let device = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0701,
        ..SimulatorConfig::default()
    });
    device.preset_property(0, PID_MANUFACTURER_ID, &MANUFACTURER.to_be_bytes());
    device.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
    device.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
    for machine in MemoryLoadStateMachine::ALL {
        preset(&device, machine, LoadState::Loaded);
    }
    device.preset_property(5, PID_OBJECT_TYPE, &APPLICATION_TYPE);
    device.preset_property(5, PID_PROGRAM_VERSION, &APPLICATION);
    device.preset_property(5, PID_RUN_STATE_CONTROL, &[0x01]);

    let observation = observe(&device, &plan_for(0x0701)).await;

    assert_eq!(
        run_state_of(&observation),
        unavailable(RunStateStep::ObjectType, RunStateCause::Refused)
    );
    assert!(device.property_read_requests().iter().all(|r| r.0 != 5));
}

#[tokio::test]
async fn a_second_same_type_object_creates_no_uniqueness_claim_and_is_not_read() {
    let device = device(0x0701, 0x02, SimulatorConfig::default());
    device.preset_property(4, PID_OBJECT_TYPE, &APPLICATION_TYPE);
    device.preset_property(4, PID_PROGRAM_VERSION, &APPLICATION);
    device.preset_property(4, PID_RUN_STATE_CONTROL, &[0x00]);

    let observation = observe(&device, &plan_for(0x0701)).await;

    assert_eq!(
        run_state_of(&observation),
        RunStateObservation::Observed(RunState::Ready),
        "the validated candidate 3 speaks for itself only"
    );
    assert!(device.property_read_requests().iter().all(|r| r.0 != 4));
}

#[tokio::test]
async fn the_display_states_a_reported_state_with_its_scope_and_nothing_else_as_running() {
    let device = device(0x0701, 0x01, SimulatorConfig::default());
    let text = observe(&device, &plan_for(0x0701)).await.to_string();
    assert!(text.contains("run state running"), "{text}");
    assert!(text.contains("candidate object 3"), "{text}");
    assert!(text.contains("not device health"), "{text}");
    assert!(!text.contains("was not observed"), "{text}");
    assert!(!text.contains("no defined run state"), "{text}");

    let device = device_with_undefined();
    let text = observe(&device, &plan_for(0x0701)).await.to_string();
    assert!(text.contains("no defined run state observed"), "{text}");
    assert!(!text.contains("was not observed"), "{text}");
    assert!(!text.contains("run state running"), "{text}");
}

#[tokio::test]
async fn the_display_names_every_state_in_words_never_as_a_debug_variant() {
    for (octet, words) in [
        (0x00, "halted"),
        (0x01, "running"),
        (0x02, "ready"),
        (0x03, "terminated"),
        (0x04, "starting"),
        (0x05, "shutting down"),
    ] {
        let device = device(0x0701, octet, SimulatorConfig::default());
        let text = observe(&device, &plan_for(0x0701)).await.to_string();
        assert!(
            text.contains(&format!("candidate object 3 reports run state {words} (")),
            "octet {octet:#04x}: {text}"
        );
        for debug in [
            "Halted",
            "Running",
            "Ready",
            "Terminated",
            "Starting",
            "ShuttingDown",
        ] {
            assert!(!text.contains(debug), "octet {octet:#04x}: {text}");
        }
    }
}

#[tokio::test]
async fn a_read_that_found_no_defined_state_says_so_but_no_read_at_all_stays_not_observed() {
    // Evidence of a runtime read, none of it a defined state.
    let other_type = device(0x0701, 0x01, SimulatorConfig::default());
    other_type.preset_property(3, PID_OBJECT_TYPE, &[0x00, 0x01]);
    let text = observe(&other_type, &plan_for(0x0701)).await.to_string();
    assert!(text.contains("no defined run state observed"), "{text}");
    assert!(!text.contains("was not observed"), "{text}");

    // No runtime evidence at all: the identity did not match, no load was read.
    let other_application = device(0x0701, 0x01, SimulatorConfig::default());
    other_application.preset_property(3, PID_PROGRAM_VERSION, &[0x00, 0x83, 0x00, 0x28, 0x15]);
    let text = observe(&other_application, &plan_for(0x0701))
        .await
        .to_string();
    assert!(
        text.ends_with("; whether the application runs was not observed"),
        "{text}"
    );
    assert!(!text.contains("no defined run state"), "{text}");
}

fn device_with_undefined() -> SimulatedDevice {
    device(0x0701, 0x81, SimulatorConfig::default())
}

/// Passes everything to the device, but the `nth` subscription (counting from
/// one) lags before anything arrives: a local read failure, not a device answer.
struct LagAt<'a> {
    device: &'a SimulatedDevice,
    nth: u32,
    subscriptions: Mutex<u32>,
}

impl ManagementTransport for LagAt<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        self.device.assigned_address()
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        let count = {
            let mut count = self.subscriptions.lock().expect("not poisoned");
            *count += 1;
            *count
        };
        if count != self.nth {
            return self.device.subscribe();
        }
        let (sender, receiver) = broadcast::channel(1);
        for _ in 0..2 {
            let _ = sender.send(TunnelEvent::Closed);
        }
        receiver
    }

    async fn send_frame(
        &self,
        destination: Destination,
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

async fn observe_lagging(device: &SimulatedDevice, nth: u32) -> (PostDownloadObservation, u32) {
    let transport = LagAt {
        device,
        nth,
        subscriptions: Mutex::new(0),
    };
    let observation =
        observe_after_download(&transport, contactable(device), fast(), &plan_for(0x0701)).await;
    let count = *transport.subscriptions.lock().expect("not poisoned");
    (observation, count)
}

#[tokio::test]
async fn a_local_read_failure_at_each_step_is_read_failed_and_stops_the_sequence() {
    let (_, total) = observe_lagging(&device(0x0701, 0x01, SimulatorConfig::default()), 0).await;
    for (offset, step, sent) in [
        (2, RunStateStep::ObjectType, 1),
        (1, RunStateStep::Application, 2),
        (0, RunStateStep::RunState, 3),
    ] {
        let device = device(0x0701, 0x01, SimulatorConfig::default());

        let (observation, _) = observe_lagging(&device, total - offset).await;

        assert_eq!(
            run_state_of(&observation),
            unavailable(step, RunStateCause::ReadFailed),
            "{step:?}"
        );
        let requests = device.property_read_requests();
        let identity = requests
            .iter()
            .position(|request| *request == (3, PID_PROGRAM_VERSION, 1, 1))
            .expect("the identity read");
        assert_eq!(
            requests.len() - identity - 1,
            sent,
            "{step:?}: {requests:?}"
        );
        assert_read_only(&device, 0);
    }
}

#[tokio::test]
async fn the_legacy_path_sends_no_description_lookup_only_the_three_reads() {
    let device = device(0x0701, 0x01, SimulatorConfig::default());
    observe(&device, &plan_for(0x0701)).await;
    let others: Vec<Seen> = device
        .seen()
        .into_iter()
        .filter(|seen| matches!(seen, Seen::Other(_)))
        .collect();
    assert_eq!(others, vec![Seen::Other("DeviceDescriptorRead")]);
}

/// Characterization: a differing machine that was read successfully does not
/// gate the continuation, and the observed state does not contradict it.
#[tokio::test]
async fn a_differing_load_reading_still_allows_an_observed_run_state_on_both_masks() {
    for mask in [0x0701, 0x0705] {
        let device = device(mask, 0x01, SimulatorConfig::default());
        preset(&device, ApplicationProgram, LoadState::Unloaded);

        let observation = observe(&device, &plan_for(mask)).await;

        assert_eq!(
            run_state_of(&observation),
            RunStateObservation::Observed(RunState::Running),
            "mask {mask:04X}: {observation:?}"
        );
        assert_eq!(
            observation.load_verdict(),
            LoadStateVerdict::DiffersFromPlan,
            "mask {mask:04X}"
        );
        assert_eq!(
            summary(&observation),
            vec![
                (
                    AddressTable,
                    Some(LoadState::Loaded),
                    Some(LoadState::Loaded)
                ),
                (
                    ApplicationProgram,
                    Some(LoadState::Loaded),
                    Some(LoadState::Unloaded)
                ),
            ],
            "mask {mask:04X}: the differing machine is the application"
        );
        let ObservationOutcome::Observed { identity, .. } = &observation.outcome else {
            panic!("mask {mask:04X}: {observation:?}");
        };
        assert_eq!(identity.mask, MaskVersion(mask));
        assert_eq!(identity.application, APPLICATION);
        let requests = device.property_read_requests();
        assert_eq!(
            requests[requests.len() - 3..],
            [
                (3, PID_OBJECT_TYPE, 1, 1),
                (3, PID_PROGRAM_VERSION, 1, 1),
                (3, PID_RUN_STATE_CONTROL, 1, 1),
            ],
            "mask {mask:04X}: the same literal start and count as for a match"
        );
        assert_eq!(
            runtime_frames(&device),
            vec![
                read(3, PID_OBJECT_TYPE),
                read(3, PID_PROGRAM_VERSION),
                read(3, PID_RUN_STATE_CONTROL),
                Seen::Disconnect,
            ],
            "mask {mask:04X}"
        );
        assert_read_only_and_closed(&device, 0);
    }
}

/// What a [`ReleasingReads`] transport saw after the first dropped read.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Afterwards {
    DroppedRead((u8, u8, u8, u16)),
    Disconnect,
    Other,
}

/// Passes everything to the device except the numbered transmissions of one
/// property read, which never arrive: no `T_ACK` returns, so the client's own
/// repetitions run out and TL releases the connection
/// ([`SessionError::ConnectionReleased`]). The device is not told a thing; the
/// simulator's own faults cannot withhold the `T_ACK` for a single property.
/// The first `pass_first` matching reads go through untouched.
struct ReleasingReads<'a> {
    device: &'a SimulatedDevice,
    object_index: u8,
    property_id: u8,
    pass_first: u32,
    matching: Mutex<u32>,
    afterwards: Mutex<Option<Vec<Afterwards>>>,
}

impl ManagementTransport for ReleasingReads<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        self.device.assigned_address()
    }

    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        self.device.subscribe()
    }

    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let dropped = match (&transport, &service) {
            (
                Tpci::NumberedData { .. },
                ApplicationService::PropertyValueRead {
                    object_index,
                    property_id,
                    nr_of_elem,
                    start_index,
                },
            ) if (*object_index, *property_id) == (self.object_index, self.property_id) => {
                let mut matching = self.matching.lock().expect("not poisoned");
                *matching += 1;
                (*matching > self.pass_first).then_some((
                    *object_index,
                    *property_id,
                    *nr_of_elem,
                    *start_index,
                ))
            }
            _ => None,
        };
        {
            let mut afterwards = self.afterwards.lock().expect("not poisoned");
            if afterwards.is_none() && dropped.is_some() {
                *afterwards = Some(Vec::new());
            }
            if let Some(frames) = afterwards.as_mut() {
                frames.push(match (dropped, &transport) {
                    (Some(read), _) => Afterwards::DroppedRead(read),
                    (None, Tpci::Disconnect) => Afterwards::Disconnect,
                    (None, _) => Afterwards::Other,
                });
            }
        }
        if dropped.is_some() {
            return Ok(());
        }
        self.device
            .send_frame(destination, transport, service)
            .await
    }

    fn target_kind(&self) -> TargetKind {
        self.device.target_kind()
    }
}

/// Characterization of the `SessionError::ConnectionReleased` arm of the
/// strict candidate read, which device-side `Disconnect` faults do not reach.
#[tokio::test]
async fn a_released_connection_at_each_step_is_connection_lost_with_no_further_frames() {
    for (step, pid, pass_first, before) in [
        (RunStateStep::ObjectType, PID_OBJECT_TYPE, 0, 0),
        // The identity check's own read of PID 13 passes; the strict reread
        // is the second one.
        (RunStateStep::Application, PID_PROGRAM_VERSION, 1, 1),
        (RunStateStep::RunState, PID_RUN_STATE_CONTROL, 0, 2),
    ] {
        let device = device(0x0701, 0x01, SimulatorConfig::default());
        let transport = ReleasingReads {
            device: &device,
            object_index: 3,
            property_id: pid,
            pass_first,
            matching: Mutex::new(0),
            afterwards: Mutex::new(None),
        };
        let observation =
            observe_after_download(&transport, contactable(&device), fast(), &plan_for(0x0701))
                .await;

        assert_eq!(
            run_state_of(&observation),
            unavailable(step, RunStateCause::ConnectionLost),
            "{step:?}"
        );
        let tuple = (3, pid, 1, 1);
        let afterwards = transport
            .afterwards
            .lock()
            .expect("not poisoned")
            .clone()
            .expect("a read was dropped");
        assert_eq!(
            afterwards,
            vec![
                Afterwards::DroppedRead(tuple),
                Afterwards::DroppedRead(tuple),
                Afterwards::DroppedRead(tuple),
                Afterwards::DroppedRead(tuple),
                Afterwards::Disconnect,
            ],
            "{step:?}: four unacknowledged transmissions, then TL's own release and nothing else"
        );
        let requests = device.property_read_requests();
        let identity = requests
            .iter()
            .position(|request| *request == (3, PID_PROGRAM_VERSION, 1, 1))
            .expect("the identity read");
        let schedule = [
            (3, PID_OBJECT_TYPE, 1, 1),
            (3, PID_PROGRAM_VERSION, 1, 1),
            (3, PID_RUN_STATE_CONTROL, 1, 1),
        ];
        assert_eq!(
            requests[identity + 1..],
            schedule[..before],
            "{step:?}: the device saw only the reads before the dropped one"
        );
        assert_read_only(&device, 0);
    }
}
