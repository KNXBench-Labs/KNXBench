//! A restore is derived only for a backup whose prior application the plan can vouch for.
//!
//! Synthetic product data only (`knx_testsupport::vendor_plugin`). The plan's own
//! identity comes from the product; the backup's from the pre-write observation.

use knx_app::device_backup::{prepare_restore, RestorePrepareError, StoredBackup};
use knx_app::download_support::offline_plan;
use knx_core::commissioning::device_backup::{
    written_regions, BackupRegion, DeviceBackup, PriorApplication, RestoreError,
};
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::memory_download::{machines, MemoryDownloadPlan};
use knx_core::commissioning::partial_memory_download::{
    application_identity, derive_partial_plan, PartialDownloadParts,
};
use knx_testsupport::vendor_plugin::{program_xml, HARDWARE_XML, PROGRAM_ID, TARGET};

fn products(dir: &std::path::Path) -> knx_productdb::Connection {
    let products = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", program_xml("").as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    products
}

fn stored(
    plan: &MemoryDownloadPlan,
    partial: Option<PartialDownloadParts>,
    prior: PriorApplication,
) -> StoredBackup {
    StoredBackup {
        backup: DeviceBackup {
            target: TARGET.parse().unwrap(),
            mask: plan.mask,
            manufacturer: plan.manufacturer,
            load_states: machines(plan)
                .into_iter()
                .map(|machine| (machine, LoadState::Loaded))
                .collect(),
            regions: written_regions(plan)
                .into_iter()
                .map(|(address, length)| BackupRegion {
                    address,
                    octets: vec![0xA5; length],
                })
                .collect(),
            prior_application: prior,
        },
        application: PROGRAM_ID.into(),
        partial,
        taken: "2026-10-25T12:00:00Z".into(),
        plan_steps: plan.steps.iter().map(ToString::to_string).collect(),
    }
}

fn shape(error: RestorePrepareError) -> RestoreError {
    match error {
        RestorePrepareError::Shape(error) => error,
        other => panic!("expected a shape refusal, got {other:?}"),
    }
}

#[test]
fn a_complete_restore_needs_the_plans_own_application_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let products = products(dir.path());
    let complete = offline_plan(&products, PROGRAM_ID).unwrap();
    let identity = application_identity(&complete).expect("the program names its application");

    prepare_restore(
        &products,
        &stored(&complete, None, PriorApplication::Recorded(identity)),
    )
    .expect("the same application restores");

    let mut other = identity;
    other[4] ^= 1;
    let error = prepare_restore(
        &products,
        &stored(&complete, None, PriorApplication::Recorded(other)),
    )
    .unwrap_err();
    assert_eq!(
        shape(error),
        RestoreError::OtherApplication {
            plan: identity,
            backup: other
        }
    );

    let error = prepare_restore(
        &products,
        &stored(&complete, None, PriorApplication::NotRecorded),
    )
    .unwrap_err();
    assert_eq!(
        shape(error),
        RestoreError::PriorApplicationNotRecorded { plan: identity }
    );
}

#[test]
fn a_legacy_partial_backup_restores_only_behind_the_plans_identity_precondition() {
    let dir = tempfile::tempdir().unwrap();
    let products = products(dir.path());
    let complete = offline_plan(&products, PROGRAM_ID).unwrap();
    let identity = application_identity(&complete).unwrap();
    let parts = PartialDownloadParts {
        parameters: false,
        group_addresses: true,
    };
    let partial = derive_partial_plan(&complete, parts).unwrap().plan;

    let plan = prepare_restore(
        &products,
        &stored(&partial, Some(parts), PriorApplication::NotRecorded),
    )
    .expect("a legacy partial backup keeps its route");
    assert!(
        plan.steps
            .iter()
            .any(|step| step.to_string().contains("check")),
        "the plan keeps its executable checks"
    );

    let mut other = identity;
    other[0] ^= 1;
    let error = prepare_restore(
        &products,
        &stored(&partial, Some(parts), PriorApplication::Recorded(other)),
    )
    .unwrap_err();
    assert!(matches!(
        shape(error),
        RestoreError::OtherApplication { .. }
    ));

    // A complete plan's steps with a forged selection are another
    // procedure: the app-level procedure guard refuses before the identity
    // guard is reached. The core's own identity guard, which never reads
    // `partial`, is covered directly in `knx-core` (an unrecorded backup is
    // refused without a precondition) and in the executor tests.
    let forged = stored(&complete, Some(parts), PriorApplication::NotRecorded);
    assert!(matches!(
        prepare_restore(&products, &forged),
        Err(RestorePrepareError::DifferentProcedure)
    ));
}
