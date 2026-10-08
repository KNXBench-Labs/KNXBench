//! Regression tests for consented community evidence.
use knx_app::contribution::analyze;

#[test]
fn project_analysis_runs_the_real_import_and_offline_device_preparation() {
    let bytes = knx_testsupport::minimal_knxproj_bytes();
    let analysis = analyze(&bytes, "private-house.knxproj").unwrap();
    assert_eq!(analysis.kind, "project");
    assert_eq!(analysis.scheme, Some(11));
    assert!(analysis
        .checks
        .iter()
        .any(|r| r.name == "project-import" && r.status == "measured"));
    assert!(analysis
        .checks
        .iter()
        .any(|r| r.name == "offline-project-plan" && r.status == "measured"));
    assert!(analysis
        .findings
        .iter()
        .any(|r| r.stage == "offline-project-plan" && r.category == "unsupported"));
    assert!(!serde_json::to_string(&analysis)
        .unwrap()
        .contains("private-house.knxproj"));
    let json = serde_json::to_value(&analysis).unwrap();
    assert!(!json["metrics"].as_array().unwrap().is_empty());
}

fn product_entries(namespace: &str, attribute: &str) -> Vec<u8> {
    let master = format!(
        r#"<KNX xmlns="{namespace}"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Synthetic"/></Manufacturers></MasterData></KNX>"#
    );
    let program = format!(
        r#"<KNX xmlns="{namespace}"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" Name="Synthetic" {attribute}><Static/></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", master.as_bytes()),
        ("M-0001/M-0001_A-1.xml", program.as_bytes()),
    ])
}

#[test]
fn product_analysis_uses_existing_offline_program_support() {
    let analysis = analyze(
        &product_entries("http://knx.org/xml/project/11", ""),
        "source.knxprod",
    )
    .unwrap();
    assert!(analysis
        .checks
        .iter()
        .any(|r| r.name == "offline-product-plan" && r.status == "measured"));
    assert!(analysis
        .findings
        .iter()
        .any(|r| r.stage == "offline-product-plan" && r.category == "unsupported"));
}

#[test]
fn a_refused_schema_still_has_expanded_namespace_structure_without_typed_admission() {
    let analysis = analyze(
        &product_entries("http://knx.org/xml/project/24", "FutureFlag=\"value\""),
        "source.knxprod",
    )
    .unwrap();
    assert_eq!(analysis.status, "refused");
    let json = serde_json::to_value(&analysis).unwrap();
    assert_eq!(json["scheme"], 24);
    assert!(json["structure"]
        .as_array()
        .unwrap()
        .iter()
        .any(|r| r["name"] == "FutureFlag" && r["kind"] == "attribute"));
    assert!(!analysis
        .checks
        .iter()
        .any(|r| r.name == "product-import" && r.status == "measured"));
}

#[test]
fn known_key_material_is_not_a_shareable_context_sample_or_original() {
    let analysis = analyze(
        &product_entries(
            "http://knx.org/xml/project/11",
            "BCUKey=\"synthetic-secret\"",
        ),
        "source.knxprod",
    )
    .unwrap();
    let json = serde_json::to_value(&analysis).unwrap();
    assert_eq!(json["originalAllowed"], false);
    let member = json["members"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["path"].as_str().unwrap().ends_with("A-1.xml"))
        .unwrap();
    assert_eq!(member["sampleAllowed"], false);
    assert!(!serde_json::to_string(&analysis)
        .unwrap()
        .contains("synthetic-secret"));
}

#[test]
fn structure_keeps_expanded_names_and_full_ancestry_distinct() {
    let bytes = product_entries(
        "http://knx.org/xml/project/24",
        "xmlns:x=\"urn:foreign\" FutureFlag=\"a\" x:FutureFlag=\"b\"",
    );
    let report = analyze(&bytes, "source.knxprod").unwrap();
    let flags: Vec<_> = report
        .structure
        .iter()
        .filter(|r| r.name == "FutureFlag")
        .collect();
    assert_eq!(flags.len(), 2);
    assert_eq!(flags[0].path.len(), 5);
    assert_eq!(
        flags
            .iter()
            .map(|r| r.namespace.as_str())
            .collect::<std::collections::BTreeSet<_>>(),
        ["", "urn:foreign"].into_iter().collect()
    );
    assert!(flags.iter().all(|r| r.occurrences == 1));
}
#[test]
fn malformed_or_deep_xml_is_partial_not_zero_gaps_and_never_a_sample() {
    for xml in [
        "<KNX><X></KNX>".to_string(),
        format!("{}{}", "<X>".repeat(129), "</X>".repeat(129)),
    ] {
        let bytes = knx_testsupport::zip_with_entries(&[("knx_master.xml", xml.as_bytes())]);
        let report = analyze(&bytes, "source.knxprod").unwrap();
        assert_eq!(report.status, "partial");
        assert!(!report.original_allowed);
        assert!(!report.members[0].sample_allowed);
        assert!(report
            .checks
            .iter()
            .any(|r| r.name == "typed-import" && r.status == "not-examined"));
    }
}
#[test]
fn long_xml_identity_is_budgeted_before_retaining_shape_paths() {
    let xml = format!("<{} />", "a".repeat(2048));
    let bytes = knx_testsupport::zip_with_entries(&[("knx_master.xml", xml.as_bytes())]);
    let report = analyze(&bytes, "source.knxprod").unwrap();
    assert_eq!(report.status, "partial");
    assert!(!report.members[0].sample_allowed);
}
#[test]
fn malformed_zip_and_non_package_file_kinds_are_input_refusals() {
    assert!(analyze(b"not a ZIP", "source.knxprod").is_err());
    assert!(analyze(&knx_testsupport::minimal_knxproj_bytes(), "source.knxkeys").is_err());
    let bytes = knx_testsupport::zip_with_entries(&[("../knx_master.xml", b"<KNX/>")]);
    assert!(analyze(&bytes, "source.knxprod").is_err());
}

#[test]
fn partial_project_retains_its_kind_and_names_unexecuted_stages() {
    let bytes = knx_testsupport::zip_with_entries(&[("knx_master.xml", b"<KNX>")]);
    let analysis = analyze(&bytes, "source.knxproj").unwrap();
    assert_eq!(analysis.kind, "project");
    assert_eq!(analysis.status, "partial");
    for name in ["project-import", "offline-project-plan"] {
        assert!(analysis
            .checks
            .iter()
            .any(|c| c.name == name && c.status == "not-examined"));
    }
}
