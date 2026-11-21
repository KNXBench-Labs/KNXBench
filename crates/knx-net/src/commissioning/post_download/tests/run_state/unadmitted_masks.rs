//! Pins that unadmitted masks get no run-state continuation, only the identity read (PRG-01-N4).

use super::*;

/// Every `070nh` low nibble the sources neither approve nor profile, plus masks outside the family.
/// `0700h`/`0701h`/`0705h` are admitted; `0702h`/`070Fh` are pinned by their own tests.
const UNADMITTED: [u16; 11] = [
    0x0703, 0x0704, 0x0706, 0x0707, 0x0708, 0x0709, 0x070A, 0x070B, 0x070C, 0x070D, 0x070E,
];
const OUTSIDE_FAMILY: [u16; 5] = [0x0020, 0x0101, 0x0102, 0x5705, 0x0710];

#[tokio::test]
async fn unadmitted_masks_send_no_object_3_frame_beyond_identity_and_never_scan() {
    for mask in UNADMITTED {
        // A plausible-looking PID 6 value must not tempt a decoder: the state is never read.
        let device = device(mask, 0x01, SimulatorConfig::default());
        let observation = observe(&device, &plan_for(mask)).await;
        assert_eq!(
            run_state_of(&observation),
            RunStateObservation::NotRead(RunStateNotRead::UnsupportedMask),
            "mask {mask:04X}"
        );
        let requests = device.property_read_requests();
        assert_eq!(
            requests.iter().filter(|r| r.0 == 3).count(),
            1,
            "mask {mask:04X}: identity only: {requests:?}"
        );
        assert!(
            requests.iter().all(|r| r.1 != PID_RUN_STATE_CONTROL),
            "mask {mask:04X}: PID 6 never read: {requests:?}"
        );
        assert_read_only_and_closed(&device, 0);
    }
}

#[tokio::test]
async fn masks_outside_the_070n_family_are_not_performed_and_send_no_property_read() {
    for mask in OUTSIDE_FAMILY {
        let device = device(mask, 0x01, SimulatorConfig::default());
        let observation = observe(&device, &plan_for(mask)).await;
        assert!(
            matches!(
                observation.outcome,
                ObservationOutcome::NotPerformed(NotPerformed::UnsupportedMask(_))
            ),
            "mask {mask:04X}: {:?}",
            observation.outcome
        );
        assert!(
            device.property_read_requests().is_empty(),
            "mask {mask:04X}"
        );
    }
}

#[tokio::test]
async fn an_unadmitted_mask_with_a_failed_load_read_stays_unsupported_not_load_failed() {
    // Malformed/failing input: the mask refusal outranks the load-read classification,
    // so a refused mask can never be reported as a transient read problem.
    let device = device(
        0x0703,
        0x01,
        SimulatorConfig {
            protected_memory: Some((0xB6EC, 0xB6ED)),
            ..SimulatorConfig::default()
        },
    );
    let observation = observe(&device, &plan_for(0x0703)).await;
    assert_eq!(
        run_state_of(&observation),
        RunStateObservation::NotRead(RunStateNotRead::UnsupportedMask)
    );
    assert_read_only_and_closed(&device, 0);
}
