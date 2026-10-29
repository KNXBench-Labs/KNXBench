//! Pins producer provenance before description and byte-exact legacy serialization.

use super::*;

fn candidate(mask: u16, config: SimulatorConfig) -> SimulatedDevice {
    let d = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: mask,
        ..config
    });
    d.preset_property(0, PID_MANUFACTURER_ID, &[0, 0x83]);
    d.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
    d.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
    d.preset_property(3, 1, &[0, 3]);
    d.preset_property(3, 6, &[1]);
    for m in MemoryLoadStateMachine::ALL {
        d.preset_load_state(ObjectIndex::new(m.type_number()), LoadState::Loaded);
    }
    d
}

async fn checked(d: &SimulatedDevice, mask: u16) -> PostDownloadCheck {
    let mut p = plan();
    p.mask = MaskVersion(mask);
    // This parity specimen concerns the two loaded machines, not the helper's extra unload.
    p.steps.retain(|step| {
        !matches!(step, MemoryDownloadStep::LoadRecord(record)
        if record.machine() == AssociationTable)
    });
    PostDownloadCheck::from_observation(
        &observe_after_download(d, ContactableAddress::new(d.address()).unwrap(), fast(), &p).await,
    )
}

#[tokio::test]
async fn legacy_0701_and_0705_producers_keep_exact_serialized_bytes_without_mask_inference() {
    // Literal pre-existing record grammar and field order, not generated from the producer.
    let expected = concat!(
        "{\"observation\":{\"status\":\"attempted\",\"reach\":{\"result\":\"reached\",\"identity\":{",
        "\"result\":\"match\",\"exemplar\":\"notVerified\",\"loadStates\":[",
        "{\"machine\":\"addressTable\",\"expected\":\"loaded\",\"observed\":{\"result\":\"observed\",\"loadState\":\"loaded\"}},",
        "{\"machine\":\"applicationProgram\",\"expected\":\"loaded\",\"observed\":{\"result\":\"observed\",\"loadState\":\"loaded\"}}],",
        "\"planLoadVerdict\":\"matchesPlan\",\"runState\":{\"status\":\"observed\",\"objectIndex\":3,\"state\":\"running\"}}}},",
        "\"runtime\":\"running\"}"
    );
    for mask in [0x0701, 0x0705] {
        let d = candidate(mask, SimulatorConfig::default());
        // Independent description evidence must not cause any description traffic on these masks.
        d.preset_property_description(3, 6, 0x27, [2, 0, 0, 0xff]);
        let check = checked(&d, mask).await;
        assert_eq!(serde_json::to_string(&check).unwrap(), expected);
        let back: PostDownloadCheck = serde_json::from_str(expected).unwrap();
        assert_eq!(serde_json::to_string(&back).unwrap(), expected);
        assert!(!d.seen().iter().any(|s| matches!(
            s,
            knx_net::commissioning::simulator::Seen::PropertyDescriptionRead { .. }
        )));
    }
}

#[tokio::test]
async fn exact0700_producer_marks_load_and_candidate_failure_before_any_description() {
    for load_failure in [true, false] {
        let d = candidate(
            0x0700,
            SimulatorConfig {
                protected_memory: load_failure.then_some((0xb6ec, 0xb6ed)),
                ..SimulatorConfig::default()
            },
        );
        d.preset_property(3, 1, &[0, 2]);
        let check = checked(&d, 0x0700).await;
        assert_eq!(check.runtime, RuntimeState::Unknown);
        let value = serde_json::to_value(&check).unwrap();
        let m = &value["observation"]["reach"]["identity"];
        assert_eq!(m["runStateMask"], "0700");
        assert_eq!(
            m["runState"],
            if load_failure {
                json!({"status":"notRead", "reason":"loadReadFailed"})
            } else {
                json!({"status":"candidateMismatch", "kind":"objectType"})
            }
        );
        assert_eq!(
            serde_json::from_value::<PostDownloadCheck>(value).unwrap(),
            check
        );
        assert!(!d.seen().iter().any(|s| matches!(
            s,
            knx_net::commissioning::simulator::Seen::PropertyDescriptionRead { .. }
        )));
    }
}
