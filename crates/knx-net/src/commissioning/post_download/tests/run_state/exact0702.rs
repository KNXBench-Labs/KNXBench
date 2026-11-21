//! Verifies descriptor-qualified exact0702 observation and unchanged neighbouring masks.

use super::exact0700::{Fault, Wire};
use super::*;
use crate::cemi::encode_l_data;

/// [`device`] at exact 0702 with a qualifying PID 6 description preset.
fn qualified(state: u8) -> SimulatedDevice {
    let d = device(0x0702, state, SimulatorConfig::default());
    d.preset_property_description(3, 6, 0x37, [0, 0, 1, 0]);
    d
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
async fn exact0702_six_states_and_complete_undefined_octets_follow_qualified_description() {
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
            run_state_of(&observe(&d, &plan_for(0x0702)).await),
            expected
        );
        assert_eq!(runtime_frames(&d), schedule(true));
        assert_eq!(d.property_read_requests().last(), Some(&(3, 6, 1, 1)));
        assert_eq!(memory_reads(&d.seen()), vec![0xb6ea, 0xb6ec]);
        assert_read_only_and_closed(&d, 0);
    }
}

#[tokio::test]
async fn exact0702_qualification_precedence_and_legacy_bit_never_grant_access() {
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
            let observation = observe(&d, &plan_for(0x0702)).await;
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
async fn exact0702_one_byte_value_without_description_does_not_qualify() {
    // Footnote 94 names only 0020h/0701h for PDT_UNSIGNED_CHAR, so 0702 is
    // PDT_CONTROL by source; without independent description evidence the
    // one-byte value still never qualifies the property.
    let d = device(0x0702, 1, SimulatorConfig::default());
    assert_eq!(
        run_state_of(&observe(&d, &plan_for(0x0702)).await),
        unavailable(RunStateStep::Description, RunStateCause::NoCorrelatedAnswer)
    );
    assert_eq!(runtime_frames(&d), schedule(false));
    assert_read_only_and_closed(&d, 0);
}

#[tokio::test]
async fn exact0702_candidate_failures_block_description_and_every_later_read() {
    for case in failures().into_iter().filter(|c| c.pid != 6) {
        let d = device(
            0x0702,
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
        let observation = observe(&d, &plan_for(0x0702)).await;
        assert_eq!(run_state_of(&observation), case.expected, "{}", case.label);
        assert!(!d.seen().contains(&description()), "{}", case.label);
        assert!(!d.property_read_requests().iter().any(|r| r.1 == 6));
        assert_read_only(&d, 0);
    }
}

#[tokio::test]
async fn exact0702_value_failures_after_description_keep_loads_and_stop() {
    for case in failures().into_iter().filter(|c| c.pid == 6) {
        let d = device(
            0x0702,
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
        let o = observe(&d, &plan_for(0x0702)).await;
        assert_eq!(run_state_of(&o), case.expected, "{}", case.label);
        assert_eq!(o.load_verdict(), LoadStateVerdict::MatchesPlan);
        assert_eq!(d.property_read_requests().last(), Some(&(3, 6, 1, 1)));
        let frames = runtime_frames(&d);
        assert_eq!(&frames[..3], &schedule(true)[..3], "{}", case.label);
        assert_read_only(&d, 0);
    }
}

#[tokio::test]
async fn exact0702_load_failure_blocks_candidate_but_differing_success_continues() {
    let d = device(
        0x0702,
        1,
        SimulatorConfig {
            protected_memory: Some((0xb6ec, 0xb6ed)),
            ..SimulatorConfig::default()
        },
    );
    assert_eq!(
        run_state_of(&observe(&d, &plan_for(0x0702)).await),
        RunStateObservation::NotRead(RunStateNotRead::LoadReadFailed)
    );
    assert_eq!(runtime_frames(&d), vec![Seen::Disconnect]);
    assert_read_only_and_closed(&d, 0);
    for load in [LoadState::Unloaded, LoadState::Loading, LoadState::Error] {
        let d = qualified(1);
        preset(&d, ApplicationProgram, load);
        let o = observe(&d, &plan_for(0x0702)).await;
        assert_eq!(o.load_verdict(), LoadStateVerdict::DiffersFromPlan);
        assert_eq!(
            run_state_of(&o),
            RunStateObservation::Observed(RunState::Running)
        );
        assert_eq!(runtime_frames(&d), schedule(true));
    }
}

/// The 0702 schedule is the 0700 schedule: PID 1, PID 13, description, then
/// the value read, on one connection, with nothing else after.
#[tokio::test]
async fn exact0702_wire_continuation_is_literal_and_0701_stays_description_free() {
    for mask in [0x0701, 0x0705] {
        let d = device(mask, 1, SimulatorConfig::default());
        let wire = Wire::new(&d, Fault::Pass);
        let o = observe_after_download(&wire, contactable(&d), fast(), &plan_for(mask)).await;
        assert_eq!(
            run_state_of(&o),
            RunStateObservation::Observed(RunState::Running)
        );
        assert!(!d.seen().contains(&description()), "mask {mask:04X}");
    }
    let d = qualified(1);
    let wire = Wire::new(&d, Fault::Pass);
    let o = observe_after_download(&wire, contactable(&d), fast(), &plan_for(0x0702)).await;
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
            vec![0x67, 0xd5, 3, 6, 0x10, 1]
        ]
    );
    assert!(d.seen().contains(&description()));
}
