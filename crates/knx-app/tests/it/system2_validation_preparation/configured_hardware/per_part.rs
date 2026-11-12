//! Terminal per-part credit: original source proven first, then the current request compared.
use super::*;
use knx_app::system2_validation_preparation::current_project_hardware_parts;
use knx_core::commissioning::pending::ProgrammingParts;
use knx_core::{flags::Direction, ComObjectInstanceId, DeviceId};

const DEVICE: DeviceId = DeviceId(1);
pub(super) const ADR_APP: ProgrammingParts = ProgrammingParts {
    address: true,
    application: true,
    parameters: false,
    group_addresses: false,
};
pub(super) const ADR_APP_PAR: ProgrammingParts = ProgrammingParts {
    parameters: true,
    ..ADR_APP
};

/// The observed, start-admitted bundle a verified run settles with.
pub(super) async fn observed(
    conn: &knx_productdb::Connection,
    project: &knx_core::Project,
    profile: &[u8],
) -> PreparedBundle {
    let device = old_device(conn, project, profile);
    let source = prepare_project_hardware_source(
        conn,
        project,
        DEVICE,
        profile,
        "per-part",
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
    admit_current_project_hardware(conn, project, DEVICE, &bundle).unwrap();
    bundle
}
pub(super) fn parts(
    conn: &knx_productdb::Connection,
    project: &knx_core::Project,
    bundle: &PreparedBundle,
) -> Option<ProgrammingParts> {
    current_project_hardware_parts(conn, project, DEVICE, bundle).ok()
}
pub(super) fn parameter(project: &mut knx_core::Project, raw: &str) {
    let id = format!("{}_R", super::super::fixture::PROGRAM);
    project.installations[0]
        .parameters
        .iter_mut()
        .find(|p| p.source.ets_id == id)
        .unwrap()
        .raw = raw.into();
}

#[tokio::test]
async fn s2_per_part_unchanged_rename_and_unrelated_edits_keep_every_part() {
    let (_dir, conn, project, profile) = inputs();
    let bundle = observed(&conn, &project, &profile).await;
    assert_eq!(parts(&conn, &project, &bundle), Some(ProgrammingParts::ALL));
    let mut renamed = project.clone();
    renamed
        .devices
        .get_mut(DEVICE)
        .unwrap()
        .name
        .push_str(" renamed");
    // The whole-preparation admission refuses this rename; the per-part comparison does not.
    assert!(admit_current_project_hardware(&conn, &renamed, DEVICE, &bundle).is_err());
    assert_eq!(parts(&conn, &renamed, &bundle), Some(ProgrammingParts::ALL));
    let mut other = project.clone();
    other.installations[0].group_addresses[0]
        .name
        .push_str(" renamed");
    let id = other.ids.next_device_id().unwrap();
    other.devices.insert(knx_core::DeviceInstance {
        id,
        source: syn_source("KB-DEV-2"),
        name: "Another device".into(),
        description: None,
        address: Some(IndividualAddress::from_raw(0x1235)),
        product_ref: String::new(),
        program_ref: String::new(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    other.installations[0].topology.unassigned.push(id);
    assert_eq!(parts(&conn, &other, &bundle), Some(ProgrammingParts::ALL));
}

#[tokio::test]
async fn s2_per_part_links_leave_only_the_group_tables_pending_and_flags_the_parameters() {
    let (_dir, conn, project, profile) = inputs();
    let bundle = observed(&conn, &project, &profile).await;
    let mut links = project.clone();
    links
        .devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .links[0]
        .direction = Direction::Receive;
    assert_eq!(parts(&conn, &links, &bundle), Some(ADR_APP_PAR));
    let mut flags = project.clone();
    flags
        .devices
        .com_object_mut(ComObjectInstanceId(1))
        .unwrap()
        .flags
        .write = exported(false);
    // The group object table lies in the application program's memory: a
    // flag is part of Parameters (ADR-0129 amendment, 12 November 2026).
    assert_eq!(parts(&conn, &flags, &bundle), Some(ADR_APP));
}

#[tokio::test]
async fn s2_per_part_a_parameter_edit_leaves_parameters_and_tables_pending() {
    let (_dir, conn, project, profile) = inputs();
    let bundle = observed(&conn, &project, &profile).await;
    let mut edited = project.clone();
    parameter(&mut edited, "772");
    assert_eq!(parts(&conn, &edited, &bundle), Some(ADR_APP));
}

#[tokio::test]
async fn s2_per_part_a_changed_or_missing_source_earns_no_credit() {
    let (_dir, conn, project, profile) = inputs();
    let bundle = observed(&conn, &project, &profile).await;
    let mut renamed = project.clone();
    renamed.devices.get_mut(DEVICE).unwrap().name.push('!');
    // Another catalogue whose winner for the same program carries other source bytes.
    let (_other_dir, changed, _, _) = super::super::fixture::configured_flag_inputs();
    for current in [&project, &renamed] {
        assert_eq!(parts(&changed, current, &bundle), None);
    }
    // The winner is gone.
    conn.execute_batch("PRAGMA foreign_keys=OFF; DELETE FROM hardware2program")
        .unwrap();
    for current in [&project, &renamed] {
        assert_eq!(parts(&conn, current, &bundle), None);
    }
}

#[tokio::test]
async fn s2_per_part_a_reference_switch_is_not_hidden_behind_the_program_id() {
    let (_dir, conn, project, profile) = inputs();
    let bundle = observed(&conn, &project, &profile).await;
    for switch in 0..5 {
        let mut switched = project.clone();
        let device = switched.devices.get_mut(DEVICE).unwrap();
        match switch {
            0 => device.product_ref.push_str("-other"),
            1 => device.program_ref.push_str("-other"),
            2 => device.source.path.push_str("-other"),
            3 => device.source.ets_id.push_str("-other"),
            _ => {
                device.source.path.push_str("-other");
                device.source.ets_id.push_str("-other");
            }
        }
        let refused = match current_project_hardware_parts(&conn, &switched, DEVICE, &bundle) {
            Err(refused) => refused,
            Ok(parts) => panic!("reference switch {switch} must earn nothing, got {parts:?}"),
        };
        assert!(
            refused.to_string().contains("reference"),
            "the reference guard refuses before any request is read (switch {switch}): {refused}"
        );
    }
}

#[tokio::test]
async fn s2_per_part_a_tampered_unbound_or_simulator_bundle_earns_no_credit() {
    let (_dir, conn, project, profile) = inputs();
    let bundle = observed(&conn, &project, &profile).await;
    let mut renamed = project.clone();
    renamed.devices.get_mut(DEVICE).unwrap().name.push('!');
    let mut tampered = bundle.clone();
    tampered
        .project_preparation
        .as_mut()
        .unwrap()
        .request
        .values
        .insert("forged".into(), "1".into());
    // The request stays intact; only the retained hardware record and the plan are forged.
    let mut forged_source_hash = bundle.clone();
    forged_source_hash
        .hardware_preparation
        .as_mut()
        .unwrap()
        .source_preparation_sha256 = "0".repeat(64);
    let mut forged_plan = bundle.clone();
    forged_plan.normalized_plan.mask ^= 1;
    assert_eq!(
        forged_source_hash.project_preparation,
        bundle.project_preparation
    );
    assert_eq!(forged_plan.project_preparation, bundle.project_preparation);
    let simulator = prepare_project_hardware_source(
        &conn,
        &project,
        DEVICE,
        &profile,
        "per-part",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap()
    .preview()
    .clone();
    for current in [&project, &renamed] {
        assert_eq!(parts(&conn, current, &tampered), None);
        assert_eq!(parts(&conn, current, &forged_source_hash), None);
        assert_eq!(parts(&conn, current, &forged_plan), None);
        assert_eq!(parts(&conn, current, &simulator), None);
        assert!(
            current_project_hardware_parts(&conn, current, DeviceId(2), &bundle).is_err(),
            "another device id is not the bound device"
        );
    }
}

/// Opt-in: `cargo test -p knx-app --test it -- s2_per_part_large_project_measurement
/// --ignored --nocapture`. Synthetic project (unrelated bare devices), dev profile, no
/// hardware or network: the timings only compare the old and the new call on one machine.
#[tokio::test]
#[ignore = "opt-in measurement: prints debug-build timings, asserts correctness only"]
async fn s2_per_part_large_project_measurement() {
    const EXTRA_DEVICES: u16 = 5000;
    const SAMPLES: usize = 3;
    let (_dir, conn, mut project, profile) = inputs();
    for n in 0..EXTRA_DEVICES {
        let id = project.ids.next_device_id().unwrap();
        project.devices.insert(knx_core::DeviceInstance {
            id,
            source: syn_source(&format!("KB-LARGE-{n}")),
            name: format!("Unrelated device {n}"),
            description: None,
            address: Some(IndividualAddress::from_raw(0x2000 + n)),
            product_ref: String::new(),
            program_ref: String::new(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
        project.installations[0].topology.unassigned.push(id);
    }
    // The bundle is prepared AFTER the project is built, so it retains the large snapshot.
    let bundle = observed(&conn, &project, &profile).await;
    let snapshot = bundle
        .project_preparation
        .as_ref()
        .unwrap()
        .project_snapshot
        .len();
    let devices = project.devices.iter().count();
    assert_eq!(devices, usize::from(EXTRA_DEVICES) + 1);
    let time = |f: &dyn Fn() -> bool| {
        let start = std::time::Instant::now();
        assert!(f(), "unchanged project must be admitted");
        start.elapsed()
    };
    for sample in 1..=SAMPLES {
        let old =
            time(&|| admit_current_project_hardware(&conn, &project, DEVICE, &bundle).is_ok());
        let new = time(&|| parts(&conn, &project, &bundle) == Some(ProgrammingParts::ALL));
        println!(
            "s2_per_part_large_project_measurement debug-build synthetic sample {sample}: \
             devices={devices} snapshot_bytes={snapshot} old_admit={old:?} new_helper={new:?}"
        );
    }
}
