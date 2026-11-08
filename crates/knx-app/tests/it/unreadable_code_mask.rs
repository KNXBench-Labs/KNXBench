//! Unreadable program code never hides a mask no download loads (RESEARCH §19.22).
//!
//! Synthetic: the KL-006 witness program (`M-FFF6`, one device at `1.1.7`)
//! gains a `RelativeSegment` whose `Data` decodes to fewer octets than its
//! `Size`, so its code cannot be read. With a mask that does not load through
//! memory, coverage and download preparation must name the mask, as they do
//! for readable code; with a memory-mapped mask the unreadable code stays the
//! refusal (`product-data`). Neither case plans or writes anything.

use std::collections::BTreeMap;

use knx_app::device_download::prepare_device_download;
use knx_app::download_support::{coverage, SupportLevel};
use knx_app::project_readiness::prepare_category;
use knx_core::{
    CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};
use knx_productdb::code::load_program_code;
use knx_testsupport::vendor_plugin::{
    program_xml, HARDWARE2PROGRAM_REF, HARDWARE_XML, PRODUCT_REF, PROGRAM_ID, TARGET,
};

/// Two declared octets, one present: the reader refuses the whole code.
fn unreadable(mask: &str) -> String {
    let xml = program_xml("")
        .replace(
            "MaskVersion=\"MV-0701\"",
            &format!("MaskVersion=\"{mask}\""),
        )
        .replace(
            "<Code>",
            &format!(
                "<Code><RelativeSegment Id=\"{PROGRAM_ID}_RS-1\" Offset=\"0\" Size=\"2\" \
                 LoadStateMachine=\"4\"><Data>AQ==</Data></RelativeSegment>"
            ),
        );
    assert!(xml.contains(mask) && xml.contains("RelativeSegment"));
    xml
}

/// The coverage category and the download preparation's category.
fn categories(mask: &str) -> (String, String) {
    let dir = tempfile::tempdir().unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/A.xml", unreadable(mask).as_bytes()).unwrap();
    knx_productdb::ingest_file(&products, "M-FFF6/Hardware.xml", HARDWARE_XML.as_bytes()).unwrap();
    let error = load_program_code(&products, PROGRAM_ID).expect_err("the code is unreadable");
    assert!(error.to_string().contains("decodes to 1 octets"), "{error}");

    let rows = coverage(&products, None, &BTreeMap::new()).unwrap();
    assert_eq!(rows.len(), 1);
    let coverage_category = match &rows[0].level {
        SupportLevel::Unsupported { category, .. } => category.code().to_string(),
        other => panic!("an unreadable program was not refused: {other:?}"),
    };
    let prepare = match prepare_device_download(&products, &project(), TARGET.parse().unwrap()) {
        Err(error) => prepare_category(&error).code().to_string(),
        Ok(prepared) => panic!(
            "an unreadable program prepared {} octets",
            prepared.plan.data_octets()
        ),
    };
    (coverage_category, prepare)
}

#[test]
fn a_mask_no_download_loads_is_named_even_when_the_code_is_unreadable() {
    for mask in ["MV-07B0", "MV-0300-01000001000000000000"] {
        let (coverage, prepare) = categories(mask);
        assert_eq!(coverage, "not-memory-mapped", "{mask}");
        assert_eq!(prepare, "not-memory-mapped", "{mask}");
    }
}

#[test]
fn a_memory_mask_keeps_the_unreadable_code_as_its_refusal() {
    let (coverage, prepare) = categories("MV-0701");
    assert_eq!(coverage, "product-data");
    assert_eq!(prepare, "product-data");
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
        name: "Synthetic System-B device".into(),
        description: None,
        address: Some(TARGET.parse().unwrap()),
        product_ref: PRODUCT_REF.into(),
        program_ref: HARDWARE2PROGRAM_REF.into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    project
}
