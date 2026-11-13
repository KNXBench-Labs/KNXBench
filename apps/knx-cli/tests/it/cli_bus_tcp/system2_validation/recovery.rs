//! Restore the same preseeded device from the original immutable operation in a fresh CLI process.

use super::*;

#[test]
fn tcp_failed_execute_then_fresh_process_recovery_restores_whole_old_and_original_evidence() {
    let f = Fixture::new();
    // Detached whole-model OLD, captured before NEW and the deliberate fault.
    let old = f.inputs.device.system2_snapshot();
    let binding = f.bundle.binding.clone();
    let (failed, original_frames) = f.run(false, true, true);
    exit(&failed, 1);
    // These owned bytes were detached at the first wire mutation, not reread
    // from a mutable owner after execution/recovery could have corrupted them.
    let before_first_mutation = f.first_mutation_evidence.lock().unwrap().clone().unwrap();
    assert!(!before_first_mutation.is_empty());
    literal_new(&original_frames);
    assert_ne!(f.inputs.device.system2_snapshot(), old);
    let store = f.store();
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(store.mutation_possible());
    assert!(store.progress().restore_available);
    let backup = store.recovery().unwrap().clone();
    let phrases = store.recovery_phrases().unwrap();
    let facts = serde_json::to_value(store.progress().download).unwrap();
    drop(store);
    let immutable: Vec<_> = tree(&f.operation())
        .into_iter()
        .filter(|(p, _)| p != "HEAD")
        .collect();
    let prepared_bytes = fs::read(f.inputs.output()).unwrap();
    // A new public CLI child, no in-process replay or replacement SimulatedDevice.
    let (restored, frames) = f.run(true, true, false);
    exit(&restored, 0);
    assert_eq!(
        f.inputs.device.system2_snapshot(),
        old,
        "whole modeled OLD restoration; no post-run normalization"
    );
    assert!(
        frames.iter().any(|frame| is_mutation(&frame.service)),
        "real recovery writes witnessed"
    );
    let store = f.store();
    assert_eq!(store.outcome(), LocalOutcome::Restored);
    assert_eq!(store.recovery(), Some(&backup));
    assert_eq!(
        serde_json::to_value(store.progress().download).unwrap(),
        facts
    );
    assert!(!store.progress().restore_available);
    assert!(
        store.recovery_phrases().is_none(),
        "admission consumed only by explicit restoration"
    );
    assert_eq!(phrases.recovery_phrase, f.bundle.restore_phrase());
    assert_eq!(f.bundle.binding, binding);
    assert_eq!(fs::read(f.inputs.output()).unwrap(), prepared_bytes);
    for (name, bytes) in before_first_mutation.into_iter().chain(immutable) {
        assert_eq!(
            fs::read(f.operation().join(&name)).unwrap(),
            bytes,
            "original immutable evidence {name}"
        );
    }
    let rows = f.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0]["state"], "effectUnverified");
    assert_eq!(rows[1]["state"], "verified");
    for row in rows {
        assert_eq!(
            row["writeEvidence"],
            serde_json::json!({"backupRecorded": true, "sendPossible": true})
        );
    }
}

#[test]
fn tcp_recovery_wrong_device_reads_identity_but_sends_no_mutation_suffix() {
    let f = Fixture::new();
    let (failed, _) = f.run(false, false, true);
    exit(&failed, 1);
    let backup = f.store().recovery().unwrap().clone();
    let immutable: Vec<_> = tree(&f.operation())
        .into_iter()
        .filter(|(p, _)| p != "HEAD")
        .collect();
    // Change ONLY the actual device observation, not the bundle/journal/confirmation facts.
    f.inputs.device.preset_property(0, 78, &[9; 6]);
    let immediate = f.inputs.device.system2_snapshot();
    let (out, suffix) = f.run(true, false, false);
    exit(&out, 1);
    assert_eq!(f.inputs.device.system2_snapshot(), immediate);
    use knx_net::Tpci;
    let property = |property_id| A::PropertyValueRead {
        object_index: 0,
        property_id,
        nr_of_elem: 1,
        start_index: 1,
    };
    assert_eq!(
        suffix
            .iter()
            .map(|f| (f.transport, f.service.clone()))
            .collect::<Vec<_>>(),
        vec![
            (Tpci::Connect, A::NoApplicationPdu),
            (
                Tpci::NumberedData { seq: 0 },
                A::DeviceDescriptorRead { descriptor_type: 0 }
            ),
            (Tpci::Ack { seq: 0 }, A::NoApplicationPdu),
            (Tpci::NumberedData { seq: 1 }, property(12)),
            (Tpci::Ack { seq: 1 }, A::NoApplicationPdu),
            (Tpci::NumberedData { seq: 2 }, property(78)),
            (Tpci::Ack { seq: 2 }, A::NoApplicationPdu),
            (Tpci::Disconnect, A::NoApplicationPdu),
        ],
        "exact outgoing recovery-only suffix; no setup, writes, key, restart or replay"
    );
    assert!(
        suffix.iter().any(|frame| matches!(
            frame.service,
            A::PropertyValueRead {
                object_index: 0,
                property_id: 78,
                ..
            }
        )),
        "actual identity read reached refusal"
    );
    assert!(
        suffix.iter().all(|frame| !is_mutation(&frame.service)),
        "no idempotent or setup writes hidden by snapshot: {suffix:?}"
    );
    assert!(
        suffix.iter().all(|frame| matches!(
            frame.service,
            A::NoApplicationPdu
                | A::DeviceDescriptorRead { .. }
                | A::PropertyValueRead { .. }
                | A::MemoryRead { .. }
        )),
        "exact allowed read/control-only recovery suffix: {suffix:?}"
    );
    assert_eq!(f.store().recovery(), Some(&backup));
    for (name, bytes) in immutable {
        assert_eq!(fs::read(f.operation().join(name)).unwrap(), bytes);
    }
    let rows = f.rows();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[1]["state"], "notSent");
}
