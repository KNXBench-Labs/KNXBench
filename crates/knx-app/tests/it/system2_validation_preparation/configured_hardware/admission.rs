//! Configured admission refuses stale inputs and never upgrades simulator candidates.
use super::*;

#[tokio::test]
async fn retained_preview_and_freshness_are_source_not_default_bound() {
    let (_dir, conn, project, profile) = inputs();
    let device = old_device(&conn, &project, &profile);
    let source = knx_app::system2_validation_preparation::prepare_retained_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        "retained-test",
        RecoveryTrack::ExternalEts,
    )
    .unwrap();
    let preview = source.preview_summary().unwrap();
    assert_eq!(
        (
            preview.parameter_values,
            preview.group_links,
            preview.flag_overrides
        ),
        (2, 2, 1)
    );
    assert_eq!(
        (
            preview.data_octets,
            preview.allocation_octets,
            preview.not_written_octets
        ),
        (31, 48, 17)
    );
    assert!(device.seen().is_empty(), "offline preview must not connect");
    source
        .check_current(&conn, &project, knx_core::DeviceId(1))
        .unwrap();
    let bundle = observe_project_hardware(
        &Hardware(&device),
        source.clone(),
        integrated_timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    assert_eq!(bundle.consent_phrase(), preview.confirmation_phrase);
    assert_eq!(
        bundle.hardware_preparation.as_ref().unwrap().recovery_track,
        RecoveryTrack::ExternalEts
    );
    assert!(matches!(
        bundle.project_preparation.as_ref().unwrap().product_input,
        knx_app::system2_validation_preparation::ProjectProductInput::RetainedSourceMembers { .. }
    ));
    let mut changed = project.clone();
    changed
        .devices
        .get_mut(knx_core::DeviceId(1))
        .unwrap()
        .name
        .push_str(" changed");
    assert!(
        source
            .check_current(&conn, &changed, knx_core::DeviceId(1))
            .is_err(),
        "even equal-plan project change must invalidate preparation"
    );
    assert!(
        admit_current_project_hardware(&conn, &changed, knx_core::DeviceId(1), &bundle).is_err()
    );
    for mutation in 0..4 {
        let mut bad = bundle.clone();
        match mutation {
            0 => bad.binding.serial[5] ^= 1,
            1 => bad
                .hardware_preparation
                .as_mut()
                .unwrap()
                .operation
                .push('x'),
            2 => bad
                .project_preparation
                .as_mut()
                .unwrap()
                .request
                .links
                .clear(),
            _ => {
                bad.normalized_plan.steps.pop();
            }
        }
        assert!(
            bad.plan().is_err(),
            "mismatched retained hardware input must refuse"
        );
    }
    let mut relabelled = source.preview().clone();
    relabelled.binding.simulated = false;
    relabelled.binding.serial = bundle.binding.serial;
    assert!(relabelled.plan().is_err());
    assert!(source
        .preview()
        .rebind_hardware(bundle.binding.serial)
        .is_err());
    let (_dir, other_conn, other_project, other_profile) = synthetic_inputs(20);
    assert!(prepare_project_hardware_source(
        &other_conn,
        &other_project,
        knx_core::DeviceId(1),
        &other_profile,
        "wrong-app",
        RecoveryTrack::ExternalEts
    )
    .is_err());
}

#[tokio::test]
async fn wrong_phrases_and_identity_refuse_without_mutation_or_retry() {
    let (dir, conn, project, profile) = inputs();
    let device = old_device(&conn, &project, &profile);
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "refusals",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let bundle = observe_project_hardware(
        &Hardware(&device),
        source,
        integrated_timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    let admission =
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &bundle).unwrap();
    let mut store =
        OperationStore::create(&dir.path().join("operation"), bundle.binding.clone()).unwrap();
    let frames = device.seen().len();
    let phrase = bundle.consent_phrase();
    let hypothesis = bundle.acceptance_phrase();
    let download = required_confirmation_phrase(device.address(), WriteScope::Download);
    let risk = knx_net::commissioning::hardware_release::required_release_phrase(device.address());
    for which in 0..4 {
        let mut c = HardwareConfirmations {
            operation: &phrase,
            hypothesis: &hypothesis,
            download: &download,
            risk: &risk,
        };
        match which {
            0 => c.operation = "wrong",
            1 => c.hypothesis = "wrong",
            2 => c.download = "wrong",
            _ => c.risk = "wrong",
        }
        assert!(execute_project_hardware(
            &Hardware(&device),
            &admission,
            &mut store,
            integrated_timing(),
            &c,
            &AtomicBool::new(false)
        )
        .await
        .is_err());
        assert_eq!(device.seen().len(), frames);
    }
    let c = HardwareConfirmations {
        operation: &phrase,
        hypothesis: &hypothesis,
        download: &download,
        risk: &risk,
    };
    // Serial and program version are observations now (owner rulings
    // 2026-10-25); a different device TYPE still refuses before any write.
    device.preset_property(
        0,
        knx_core::commissioning::properties::PID_HARDWARE_TYPE,
        &[0, 1, 0, 0, 0, 92],
    );
    let before = device.system2_snapshot();
    assert!(execute_project_hardware(
        &Hardware(&device),
        &admission,
        &mut store,
        integrated_timing(),
        &c,
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    assert_eq!(device.system2_snapshot(), before);
    assert!(!store.mutation_possible());
    assert!(store.recovery().is_none());
    let frames = device.seen().len();
    assert!(execute_project_hardware(
        &Hardware(&device),
        &admission,
        &mut store,
        integrated_timing(),
        &c,
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    assert_eq!(device.seen().len(), frames, "same operation must not retry");
}

#[tokio::test]
async fn placeholder_serials_are_observed_not_refused() {
    // Amendment 3: the serial is no admission check, even all-zero or all-FF.
    for serial in [[0; 6], [0xFF; 6]] {
        let (_dir, conn, project, profile) = inputs();
        let device = old_device(&conn, &project, &profile);
        device.preset_property(
            0,
            knx_core::commissioning::properties::PID_SERIAL_NUMBER,
            &serial,
        );
        let source = prepare_project_hardware_source(
            &conn,
            &project,
            knx_core::DeviceId(1),
            &profile,
            "placeholder-serial",
            RecoveryTrack::KnxBenchBestEffort,
        )
        .unwrap();
        let bundle = observe_project_hardware(
            &Hardware(&device),
            source,
            integrated_timing(),
            &AtomicBool::new(false),
        )
        .await
        .expect("placeholder serial is recorded, not refused");
        assert_eq!(bundle.binding.serial, serial);
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &bundle).unwrap();
    }
}
