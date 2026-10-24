//! KL-006: a program that declares an ETS download plug-in is never downloaded.
//!
//! Synthetic throughout: manufacturer `M-FFF6`, one memory-mapped program
//! with a complete `ProductProcedure` (address table and application
//! program), one project device at `1.1.7`. The same program is installed
//! with and without `Static/Extension/@EtsDownloadPlugin`. A plug-in writes
//! what KNXBench cannot reproduce from the product's base data, so every
//! download scope of the declaring program is refused with its own category;
//! the program without the declaration plans exactly as before.

use std::collections::BTreeMap;

use knx_app::comparison_adoption::{prepare_comparison, ComparisonScope};
use knx_app::device_backup::{prepare_restore, RestorePrepareError, StoredBackup};
use knx_app::device_download::{prepare_device_download, prepare_device_download_for_reading};
use knx_app::download_support::{coverage, offline_plan, SupportLevel};
use knx_app::project_readiness::{prepare_category, project_readiness, DeviceReadiness};
use knx_core::commissioning::device_backup::{written_regions, BackupRegion, DeviceBackup};
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::memory_download::{machines, MemoryDownloadPlan};
use knx_core::commissioning::partial_memory_download::PartialDownloadParts;
use knx_core::{
    CommandStack, CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId,
    Language, Project, SourceRef, Topology,
};

use knx_testsupport::vendor_plugin::{
    declaration, program_xml, HARDWARE2PROGRAM_REF, HARDWARE_XML, PLUGIN_GUID as PLUGIN,
    PRODUCT_REF, PROGRAM_ID, TARGET,
};

const CATEGORY: &str = "vendor-download-plugin";

/// A product database holding the program with `extension`, and a project
/// with one device using it at `1.1.7`.
fn installed(dir: &std::path::Path, extension: &str) -> (knx_productdb::Connection, Project) {
    let products = knx_productdb::open_and_migrate(&dir.join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", program_xml(extension).as_bytes())
        .unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    (products, project())
}

fn project() -> Project {
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-1".into(),
        },
        name: "Synthetic actuator".into(),
        description: None,
        address: Some(target()),
        product_ref: PRODUCT_REF.into(),
        program_ref: HARDWARE2PROGRAM_REF.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}

fn target() -> knx_core::IndividualAddress {
    TARGET.parse().unwrap()
}

const SCOPES: [PartialDownloadParts; 3] = [
    PartialDownloadParts {
        parameters: true,
        group_addresses: false,
    },
    PartialDownloadParts {
        parameters: false,
        group_addresses: true,
    },
    PartialDownloadParts {
        parameters: true,
        group_addresses: true,
    },
];

fn assert_kl_006_detail(detail: &str) {
    assert!(detail.contains("KL-006"), "{detail}");
    assert!(detail.contains("ETS download plug-in"), "{detail}");
    assert!(detail.contains(PLUGIN), "{detail}");
    assert!(detail.contains("no download scope is offered"), "{detail}");
}

fn assert_kl_006(category: &str, detail: &str) {
    assert_eq!(category, CATEGORY, "{detail}");
    assert_kl_006_detail(detail);
}

/// A structurally valid backup for the synthetic program. Its shape comes
/// from the same program without the declaration; KL-006 is the only reason
/// the declaring program cannot prepare it for restoration.
fn stored_backup(plan: &MemoryDownloadPlan) -> StoredBackup {
    StoredBackup {
        backup: DeviceBackup {
            target: target(),
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
        },
        application: PROGRAM_ID.into(),
        partial: None,
        taken: "2026-10-24T12:00:00Z".into(),
        plan_steps: plan.steps.iter().map(ToString::to_string).collect(),
    }
}

#[test]
fn readiness_refuses_a_declared_download_plugin_naming_kl_006() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), &declaration());
    let rows = project_readiness(&products, &project, &BTreeMap::new());
    match &rows[0].readiness {
        DeviceReadiness::Graded(SupportLevel::Unsupported { category, detail }) => {
            assert_kl_006(category.code(), detail)
        }
        other => panic!("a plug-in program must not grade {other:?}"),
    }
}

#[test]
fn every_download_scope_is_refused_including_group_addresses_only() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), &declaration());
    // The complete download and every partial one start from this
    // preparation (CLI and server alike), so its refusal refuses them all.
    let error = prepare_device_download(&products, &project, target())
        .expect_err("a download of a plug-in program must not be prepared");
    assert_kl_006(prepare_category(&error).code(), &error.to_string());

    // The same fixture without the declaration derives every partial scope,
    // the group-address-only one included: the refusal above is the
    // declaration's, not the fixture's.
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), "");
    for parts in SCOPES {
        let prepared = prepare_device_download(&products, &project, target()).unwrap();
        prepared
            .into_partial(parts)
            .unwrap_or_else(|e| panic!("{parts:?}: {e}"));
    }
}

#[test]
fn product_coverage_and_the_offline_planner_refuse_it_too() {
    let dir = tempfile::tempdir().unwrap();
    let (products, _) = installed(dir.path(), &declaration());
    let rows = coverage(&products, None, &BTreeMap::new()).unwrap();
    let row = rows.iter().find(|r| r.program_id == PROGRAM_ID).unwrap();
    match &row.level {
        SupportLevel::Unsupported { category, detail } => assert_kl_006(category.code(), detail),
        other => panic!("coverage must not grade a plug-in program {other:?}"),
    }
    // A backup restore plans from the product offline; it is a download too.
    let (category, detail) = offline_plan(&products, PROGRAM_ID).unwrap_err();
    assert_kl_006(category.code(), &detail);
}

#[test]
fn public_restore_preparation_refuses_a_declared_download_plugin() {
    let plain_dir = tempfile::tempdir().unwrap();
    let (plain_products, _) = installed(plain_dir.path(), "");
    let plain_plan = offline_plan(&plain_products, PROGRAM_ID).unwrap();
    let stored = stored_backup(&plain_plan);
    prepare_restore(&plain_products, &stored)
        .expect("the synthetic backup must fit the same program without the declaration");

    let plugin_dir = tempfile::tempdir().unwrap();
    let (plugin_products, _) = installed(plugin_dir.path(), &declaration());
    match prepare_restore(&plugin_products, &stored) {
        Err(RestorePrepareError::Plan(detail)) => assert_kl_006_detail(&detail),
        other => panic!("a plug-in restore must be refused as a plan: {other:?}"),
    }
}

#[test]
fn a_program_without_the_declaration_is_unchanged() {
    // No Extension at all, an Extension without a plug-in, and an empty
    // attribute (which names no plug-in) all plan and grade as before.
    for extension in [
        "",
        r#"<Extension RequiresExternalSoftware="0" />"#,
        r#"<Extension EtsDownloadPlugin="" />"#,
    ] {
        let dir = tempfile::tempdir().unwrap();
        let (products, project) = installed(dir.path(), extension);
        let rows = project_readiness(&products, &project, &BTreeMap::new());
        assert_eq!(rows[0].readiness.code(), "untested", "{extension}");
        let rows = coverage(&products, None, &BTreeMap::new()).unwrap();
        assert_eq!(rows[0].level.code(), "untested", "{extension}");
        let prepared = prepare_device_download(&products, &project, target()).unwrap();
        assert_eq!(prepared.plan.data_octets(), 12, "{extension}");
    }
}

#[test]
fn reading_the_device_stays_possible_with_the_same_plan() {
    // `knx device compare` and comparison adoption read what the plan names;
    // a declared plug-in must not stop that, and the plan read is exactly
    // the one the program would have without the declaration.
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), &declaration());
    let read = prepare_device_download_for_reading(&products, &project, target())
        .expect("a read-only preparation is not a download");
    let dir = tempfile::tempdir().unwrap();
    let (plain_products, plain_project) = installed(dir.path(), "");
    let plain = prepare_device_download(&plain_products, &plain_project, target()).unwrap();
    assert_eq!(read.plan, plain.plan);
    assert_eq!(read.image.segments, plain.image.segments);
}

#[test]
fn comparison_adoption_preparation_stays_read_only_for_a_declared_plugin() {
    let dir = tempfile::tempdir().unwrap();
    let (products, project) = installed(dir.path(), &declaration());
    let scope = ComparisonScope {
        server_incarnation: "synthetic-plugin-test".into(),
        project_incarnation: 1,
    };
    let prepared = prepare_comparison(
        &products,
        &project,
        &CommandStack::new(),
        &scope,
        DeviceId(1),
    )
    .expect("comparison preparation reads a plug-in program; it does not download it");

    let plain_dir = tempfile::tempdir().unwrap();
    let (plain_products, plain_project) = installed(plain_dir.path(), "");
    let plain = prepare_device_download(&plain_products, &plain_project, target()).unwrap();
    assert_eq!(prepared.target(), target());
    assert_eq!(prepared.program_id(), PROGRAM_ID);
    assert_eq!(prepared.plan(), &plain.plan);
    assert_eq!(prepared.image().segments, plain.image.segments);
}

/// Precedence is deterministic: when a program is also of a kind no
/// download translates, that more fundamental refusal is reported, in
/// readiness, preparation and coverage alike; the plug-in refuses only what
/// would otherwise plan.
#[test]
fn a_kind_refusal_wins_over_a_declared_plugin() {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let xml =
        program_xml(&declaration()).replace(r#"MaskVersion="MV-0701""#, r#"MaskVersion="MV-0012""#);
    assert!(xml.contains("MV-0012"));
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", xml.as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    let project = project();
    let rows = project_readiness(&products, &project, &BTreeMap::new());
    match &rows[0].readiness {
        DeviceReadiness::Graded(SupportLevel::Unsupported { category, detail }) => {
            assert_eq!(category.code(), "not-memory-mapped", "{detail}");
            assert!(!detail.contains("KL-006"), "{detail}");
        }
        other => panic!("{other:?}"),
    }
    let error = prepare_device_download(&products, &project, target()).unwrap_err();
    assert_eq!(
        prepare_category(&error).code(),
        "not-memory-mapped",
        "{error}"
    );
    let (category, _) = offline_plan(&products, PROGRAM_ID).unwrap_err();
    assert_eq!(category.code(), "not-memory-mapped");
}
