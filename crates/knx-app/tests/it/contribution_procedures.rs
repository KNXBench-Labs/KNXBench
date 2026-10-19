//! Offline AP1 resolution stays separate from download readiness and public disclosure.
use knx_app::contribution::analyze;
use knx_app::contribution_bundle::{build_bundle, preview_bundle, Audience, BundleOptions};

fn package(fragments: &str, extra: &str) -> Vec<u8> {
    let master = r#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Synthetic"/></Manufacturers><MaskVersions><MaskVersion Id="MV-07B0"><HawkConfigurationData><Procedures><Procedure ProcedureType="Load" ProcedureSubType="ap1" Access="remote local2"><LdCtrlConnect/><LdCtrlMerge MergeId="1"/><LdCtrlLoad LsmIdx="4"/><LdCtrlMerge MergeId="2"/><LdCtrlMerge MergeId="4"/><LdCtrlLoadCompleted LsmIdx="4"/><LdCtrlMerge MergeId="6"/><LdCtrlMerge MergeId="7"/><LdCtrlRestart/></Procedure></Procedures></HawkConfigurationData></MaskVersion></MaskVersions></MasterData></KNX>"#;
    let program = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="M-0001_A-1" Name="Synthetic" MaskVersion="MV-07B0" LoadProcedureStyle="MergedProcedure" ApplicationNumber="1" ApplicationVersion="2" {extra}><Static><LoadProcedures>{fragments}</LoadProcedures></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", master.as_bytes()),
        ("M-0001/A.xml", program.as_bytes()),
    ])
}
fn fragments() -> &'static str {
    r#"<LoadProcedure MergeId="2"><LdCtrlRelSegment LsmIdx="4" Size="16" Mode="0" Fill="0"/></LoadProcedure><LoadProcedure MergeId="4"><LdCtrlWriteRelMem ObjIdx="4" Offset="0" Size="16" Verify="true"/></LoadProcedure>"#
}
fn options() -> BundleOptions {
    BundleOptions {
        audience: Audience::Public,
        sample_ids: vec![],
        include_original: false,
        consent: true,
        original_consent: false,
        expected_manifest_sha256: None,
    }
}
#[test]
fn merged_ap1_is_expanded_in_source_order_without_new_live_support() {
    let report = analyze(&package(fragments(), ""), "source.knxprod").unwrap();
    let json = serde_json::to_value(&report).unwrap();
    let r = &json["procedureResolutions"][0];
    assert_eq!(r["status"], "expanded");
    assert_eq!(r["variant"], "ap1");
    assert_eq!(r["executable"], false);
    assert_eq!(r["programAttributes"]["ApplicationVersion"], "2");
    assert_eq!(r["sources"].as_array().unwrap().len(), 2);
    let names: Vec<_> = r["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["node"]["name"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "LdCtrlConnect",
            "LdCtrlLoad",
            "LdCtrlRelSegment",
            "LdCtrlWriteRelMem",
            "LdCtrlLoadCompleted",
            "LdCtrlRestart"
        ]
    );
    assert_eq!(r["steps"][2]["origin"]["source"], "program");
    assert!(r["steps"][2]["origin"]["sha256"].as_str().unwrap().len() == 64);
    assert_eq!(r["steps"][2]["node"]["attributes"]["Size"], "16");
    assert!(report
        .findings
        .iter()
        .any(|f| f.stage == "offline-product-plan" && f.category == "unsupported"));
    assert!(!report.findings.iter().any(|f| f.category == "verified"));
    assert!(report
        .findings
        .iter()
        .any(|f| f.stage == "offline-procedure-resolution" && f.category == "observation"));
}
#[test]
fn missing_mandatory_fragment_stays_visible_in_a_partial_sequence() {
    let report = analyze(
        &package(
            r#"<LoadProcedure MergeId="2"><LdCtrlRelSegment LsmIdx="4"/></LoadProcedure>"#,
            "",
        ),
        "source.knxprod",
    )
    .unwrap();
    let json = serde_json::to_value(report).unwrap();
    let r = &json["procedureResolutions"][0];
    assert_eq!(r["status"], "partial");
    assert!(r["steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["node"]["name"] == "LdCtrlMerge" && s["node"]["attributes"]["MergeId"] == "4"));
    assert!(r["issues"]
        .as_array()
        .unwrap()
        .iter()
        .any(|i| i["code"] == "missing-mandatory-merge"));
}
#[test]
fn reduced_preview_and_export_withhold_all_detailed_resolution_provenance() {
    let bytes = package(fragments(), "");
    let local = serde_json::to_value(analyze(&bytes, "source.knxprod").unwrap()).unwrap();
    let sha = local["procedureResolutions"][0]["sources"][0]["sha256"]
        .as_str()
        .unwrap();
    let mut opts = options();
    let preview = preview_bundle(&bytes, "source.knxprod", &opts).unwrap();
    let text = preview
        .files
        .iter()
        .find(|f| f.path == "findings.json")
        .unwrap()
        .text
        .as_ref()
        .unwrap();
    assert!(!text.contains(sha));
    assert!(!text.contains("M-0001_A-1"));
    assert!(!text.contains("procedureResolutions"));
    assert!(text.contains("offline-procedure-resolution"));
    opts.expected_manifest_sha256 = Some(preview.manifest_sha256);
    let bundle = build_bundle(&bytes, "source.knxprod", &opts).unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bundle.bytes)).unwrap();
    let mut exported = String::new();
    std::io::Read::read_to_string(&mut zip.by_name("findings.json").unwrap(), &mut exported)
        .unwrap();
    assert_eq!(&exported, text);
}
#[test]
fn a_resolution_budget_stop_is_partial_analysis_not_measured_complete() {
    let many = fragments().replace(
        r#"<LdCtrlRelSegment LsmIdx="4" Size="16" Mode="0" Fill="0"/>"#,
        &r#"<LdCtrlLoad LsmIdx="4"/>"#.repeat(2049),
    );
    let report = analyze(&package(&many, ""), "source.knxprod").unwrap();
    assert_eq!(report.status, "partial");
    let check = report
        .checks
        .iter()
        .find(|c| c.name == "offline-procedure-resolution")
        .unwrap();
    assert_eq!(check.status, "partial");
}
#[test]
fn issue_projection_is_aggregated_without_repeating_source_identifiers() {
    let f = fragments().replace(
        r#"<LdCtrlRelSegment LsmIdx="4" Size="16" Mode="0" Fill="0"/>"#,
        &"<Unfamiliar/>".repeat(100),
    );
    let report = analyze(&package(&f, ""), "source.knxprod").unwrap();
    let findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.stage == "offline-procedure-resolution" && f.category == "unknown")
        .collect();
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].occurrences, 100);
    assert!(findings[0].name.is_none());
    assert!(report
        .metrics
        .iter()
        .any(|m| m.stage == "offline-procedure-resolution"
            && m.entity == "uninterpreted-step"
            && m.disposition == "issue"
            && m.count == 100));
    let local = serde_json::to_value(&report).unwrap();
    assert_eq!(
        local["procedureResolutions"][0]["issues"]
            .as_array()
            .unwrap()
            .len(),
        100
    );
    let preview = preview_bundle(&package(&f, ""), "source.knxprod", &options()).unwrap();
    let text = preview
        .files
        .iter()
        .find(|f| f.path == "findings.json")
        .unwrap()
        .text
        .as_ref()
        .unwrap();
    assert!(text.contains("uninterpreted-step"));
    assert!(!text.contains("M-0001_A-1"));
}
#[test]
fn known_secret_material_withholds_resolution_details_on_the_local_analysis_too() {
    let report = analyze(
        &package(fragments(), r#"BCUKey="synthetic-secret""#),
        "source.knxprod",
    )
    .unwrap();
    let text = serde_json::to_string(&report).unwrap();
    assert!(!text.contains("synthetic-secret"));
    assert!(!text.contains("procedureResolutions"));
}
