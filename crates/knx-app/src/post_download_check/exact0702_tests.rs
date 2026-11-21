//! Pins exact0702 provenance-marker admission, refusal and per-mask marker parity.

use super::*;

/// A matching record with the given run-state record and optional mask marker.
fn with_marker(record: Value, marker: Option<Value>) -> Value {
    let mut value = matched_with_run_state(
        vec![
            machine("addressTable", json!("loaded"), loaded()),
            machine("applicationProgram", json!("loaded"), loaded()),
        ],
        "matchesPlan",
        record,
    );
    if let Some(marker) = marker {
        value["observation"]["reach"]["identity"]["runStateMask"] = marker;
    }
    value
}

#[test]
fn exact0702_marker_admits_the_same_description_vocabulary_as_0700() {
    let admit = |value: Value| {
        let check: PostDownloadCheck = serde_json::from_value(value.clone()).unwrap();
        assert_eq!(serde_json::to_value(check).unwrap(), value);
    };
    let refuse = |value: Value| {
        assert!(
            serde_json::from_value::<PostDownloadCheck>(value.clone()).is_err(),
            "{value}"
        );
    };
    for marker in [json!("0700"), json!("0702")] {
        for record in [
            json!({"status":"notQualified", "reason":"descriptionProblem"}),
            json!({"status":"notQualified", "reason":"unsupportedDatatype", "pdt":2}),
            json!({"status":"unavailable", "step":"description", "cause":"noCorrelatedAnswer"}),
            json!({"status":"undefinedValue"}),
        ] {
            admit(with_marker(record.clone(), Some(marker.clone())));
        }
        // Description vocabulary without any marker stays refused.
        for record in [
            json!({"status":"notQualified", "reason":"descriptionProblem"}),
            json!({"status":"unavailable", "step":"description", "cause":"readFailed"}),
        ] {
            refuse(with_marker(record, None));
        }
    }
    // Marker plus unsupportedMask is contradictory; wrong marker values refuse.
    refuse(with_marker(
        json!({"status":"notRead", "reason":"unsupportedMask"}),
        Some(json!("0702")),
    ));
    for marker in [
        Value::Null,
        json!(702),
        json!("0701"),
        json!("0705"),
        json!("702"),
    ] {
        refuse(with_marker(
            json!({"status":"undefinedValue"}),
            Some(marker),
        ));
    }
}

#[tokio::test]
async fn exact0702_producer_marks_matched_identity_and_observes_when_qualified() {
    let d = SimulatedDevice::with_config(SimulatorConfig {
        mask_version: 0x0702,
        ..SimulatorConfig::default()
    });
    d.preset_property(0, PID_MANUFACTURER_ID, &[0, 0x83]);
    d.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
    d.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
    d.preset_property(3, 1, &[0, 3]);
    d.preset_property(3, 6, &[1]);
    d.preset_property_description(3, 6, 0x37, [0, 0, 1, 0]);
    for m in MemoryLoadStateMachine::ALL {
        d.preset_load_state(ObjectIndex::new(m.type_number()), LoadState::Loaded);
    }
    let mut p = plan();
    p.mask = MaskVersion(0x0702);
    p.steps.retain(|step| {
        !matches!(step, MemoryDownloadStep::LoadRecord(record)
        if record.machine() == AssociationTable)
    });
    let observation = observe_after_download(
        &d,
        ContactableAddress::new(d.address()).unwrap(),
        fast(),
        &p,
    )
    .await;
    let check = PostDownloadCheck::from_observation(&observation);
    assert_eq!(check.runtime, RuntimeState::Running);
    let value = serde_json::to_value(&check).unwrap();
    let m = &value["observation"]["reach"]["identity"];
    assert_eq!(m["runStateMask"], "0702");
    assert_eq!(
        m["runState"],
        json!({"status":"observed", "objectIndex":3, "state":"running"})
    );
    assert_eq!(
        serde_json::from_value::<PostDownloadCheck>(value.clone()).unwrap(),
        check
    );
    assert!(d.seen().iter().any(|s| matches!(
        s,
        knx_net::commissioning::simulator::Seen::PropertyDescriptionRead { .. }
    )));
}

#[tokio::test]
async fn exact0700_and_0702_producers_emit_distinct_markers_and_0701_stays_marker_free() {
    for (mask, marker) in [
        (0x0700u16, Some("0700")),
        (0x0702, Some("0702")),
        (0x0701, None),
    ] {
        let d = SimulatedDevice::with_config(SimulatorConfig {
            mask_version: mask,
            ..SimulatorConfig::default()
        });
        d.preset_property(0, PID_MANUFACTURER_ID, &[0, 0x83]);
        d.preset_property(0, PID_HARDWARE_TYPE, &HARDWARE);
        d.preset_property(3, PID_PROGRAM_VERSION, &APPLICATION);
        d.preset_property(3, 1, &[0, 3]);
        d.preset_property(3, 6, &[1]);
        if marker.is_some() {
            d.preset_property_description(3, 6, 0x37, [0, 0, 1, 0]);
        }
        for m in MemoryLoadStateMachine::ALL {
            d.preset_load_state(ObjectIndex::new(m.type_number()), LoadState::Loaded);
        }
        let mut p = plan();
        p.mask = MaskVersion(mask);
        p.steps.retain(|step| {
            !matches!(step, MemoryDownloadStep::LoadRecord(record)
            if record.machine() == AssociationTable)
        });
        let observation = observe_after_download(
            &d,
            ContactableAddress::new(d.address()).unwrap(),
            fast(),
            &p,
        )
        .await;
        let value =
            serde_json::to_value(PostDownloadCheck::from_observation(&observation)).unwrap();
        let identity = &value["observation"]["reach"]["identity"];
        match marker {
            Some(marker) => assert_eq!(identity["runStateMask"], json!(marker), "mask {mask:04X}"),
            None => assert!(identity.get("runStateMask").is_none(), "mask {mask:04X}"),
        }
    }
}
