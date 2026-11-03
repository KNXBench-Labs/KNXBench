//! Tests old policy grammars, current-request refusals and immutable evidence.
use super::super::caption_fixture as f;
use super::per_part::{observed, parameter, parts};
use super::*;
use knx_app::system2_validation_preparation::current_project_hardware_parts;
use knx_core::commissioning::pending::ProgrammingParts;

#[tokio::test]
async fn review_caption_historical_policy_revisions_and_recovery_tracks_stay_separate() {
    for retained in [false, true] {
        let input = f::inputs(20, retained);
        let original = observed(&input.conn, &input.project, &input.profile).await;
        let changed = f::caption(&input.package);
        let input = input.replace(&changed, false);
        for revision in [1, 2, 3] {
            for track in [
                RecoveryTrack::ExternalEts,
                RecoveryTrack::KnxBenchBestEffort,
            ] {
                let mut bundle = original.clone();
                let hardware = bundle.hardware_preparation.as_mut().unwrap();
                hardware.revision = revision;
                hardware.recovery_track = track;
                if revision == 1 {
                    hardware.pointer_width_policy = None;
                }
                if revision < 3 {
                    hardware.post_load_octet_policy = None;
                }
                bundle.binding.project_preparation_sha256 = Some(hardware.sha256().unwrap());
                bundle.plan().unwrap();
                bundle.preconditions().unwrap();
                let frozen = serde_json::to_vec(&bundle).unwrap();
                assert!(admit_current_project_hardware(
                    &input.conn,
                    &input.project,
                    f::DEVICE,
                    &bundle
                )
                .is_err());
                assert_eq!(
                    parts(&input.conn, &input.project, &bundle),
                    Some(ProgrammingParts::ALL),
                    "{retained} {revision} {track:?}"
                );
                assert_eq!(serde_json::to_vec(&bundle).unwrap(), frozen);
            }
        }
    }
}

#[tokio::test]
async fn review_caption_invalid_current_parameter_does_not_use_original_request() {
    for retained in [false, true] {
        let input = f::inputs(23, retained);
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        let changed = f::caption(&input.package);
        let input = input.replace(&changed, false);
        let mut current = input.project.clone();
        parameter(&mut current, "65536");
        let frozen_project = knx_store::project_history::NativeSnapshot {
            project: current.clone(),
            opaque: vec![],
            manufacturer_refs: vec![],
        }
        .encode()
        .unwrap();
        let frozen_bundle = serde_json::to_vec(&bundle).unwrap();
        let changes = input.conn.total_changes();
        assert!(current_project_hardware_parts(&input.conn, &current, f::DEVICE, &bundle).is_err());
        assert_eq!(
            knx_store::project_history::NativeSnapshot {
                project: current.clone(),
                opaque: vec![],
                manufacturer_refs: vec![],
            }
            .encode()
            .unwrap(),
            frozen_project
        );
        assert_eq!(serde_json::to_vec(&bundle).unwrap(), frozen_bundle);
        assert_eq!(input.conn.total_changes(), changes);
        assert_eq!(
            parts(&input.conn, &input.project, &bundle),
            Some(ProgrammingParts::ALL)
        );
    }
}

#[tokio::test]
async fn review_caption_missing_current_device_and_wrong_argument_never_credit() {
    for retained in [false, true] {
        let input = f::inputs(20, retained);
        let bundle = observed(&input.conn, &input.project, &input.profile).await;
        let changed = f::caption(&input.package);
        let input = input.replace(&changed, false);
        assert!(current_project_hardware_parts(
            &input.conn,
            &input.project,
            knx_core::DeviceId(2),
            &bundle
        )
        .is_err());
        let mut missing = input.project.clone();
        assert!(missing.devices.remove(f::DEVICE).is_some());
        assert!(current_project_hardware_parts(&input.conn, &missing, f::DEVICE, &bundle).is_err());
        let mut current = input.project.clone();
        current
            .devices
            .get_mut(f::DEVICE)
            .unwrap()
            .source
            .path
            .push_str("-same-program");
        // This edit leaves catalogue program resolution intact, so reference refusal is not masked.
        assert_eq!(
            knx_productdb::image_request::image_request_for_device(
                &input.conn,
                &current,
                f::DEVICE
            )
            .unwrap()
            .program_id,
            f::PROGRAM
        );
        assert!(current_project_hardware_parts(&input.conn, &current, f::DEVICE, &bundle).is_err());
    }
}
