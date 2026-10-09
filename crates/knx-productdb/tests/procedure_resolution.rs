//! Adversarial tests for source-bound, non-executable AP1 expansion.
use knx_productdb::{
    procedure_resolution::{resolve_package_ap1, Resolution},
    *,
};

const NS: &str = "http://knx.org/xml/project/20";
const ID: &str = "M-0001_A-1";
fn master(sequence: &str) -> String {
    format!(
        r#"<KNX xmlns="{NS}"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Synthetic"/></Manufacturers><MaskVersions><MaskVersion Id="MV-07B0"><HawkConfigurationData><Procedures><Procedure ProcedureType="Load" ProcedureSubType="ap1" Access="remote local2">{sequence}</Procedure></Procedures></HawkConfigurationData></MaskVersion></MaskVersions></MasterData></KNX>"#
    )
}
fn program(fragments: &str) -> String {
    format!(
        r#"<KNX xmlns="{NS}"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="{ID}" MaskVersion="MV-07B0" LoadProcedureStyle="MergedProcedure"><Static><LoadProcedures>{fragments}</LoadProcedures></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    )
}
fn sequence() -> &'static str {
    r#"<LdCtrlConnect/><LdCtrlMerge MergeId="1"/><LdCtrlLoad LsmIdx="4"/><LdCtrlMerge MergeId="2"/><LdCtrlMerge MergeId="4"/><LdCtrlMerge MergeId="6"/><LdCtrlMerge MergeId="7"/><LdCtrlRestart/>"#
}
fn fragments() -> &'static str {
    r#"<LoadProcedure MergeId="2"><LdCtrlRelSegment LsmIdx="4" Size="16"/></LoadProcedure><LoadProcedure MergeId="4"><LdCtrlWriteRelMem ObjIdx="4" Size="16"/></LoadProcedure>"#
}
fn db(p: &str, m: &str) -> (tempfile::TempDir, Connection, String) {
    let temp = tempfile::tempdir().unwrap();
    let conn = open_and_migrate(&temp.path().join("p.sqlite")).unwrap();
    let bytes = knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", m.as_bytes()),
        ("M-0001/A.xml", p.as_bytes()),
    ]);
    let report = install_package(&conn, "synthetic.knxprod", &bytes).unwrap();
    (temp, conn, report.sha256)
}
fn result(f: &str, s: &str) -> Resolution {
    let (_t, c, sha) = db(&program(f), &master(s));
    resolve_package_ap1(&c, &sha, ID).unwrap().unwrap()
}
fn issue(r: &Resolution, c: &str) -> bool {
    r.issues.iter().any(|i| i.code == c)
}
#[test]
fn optional_omissions_are_accounted_for_and_offsets_identify_exact_source_nodes() {
    let p = program(fragments());
    let m = master(sequence());
    let (_t, c, sha) = db(&p, &m);
    let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
    assert_eq!(r.status, "expanded");
    assert!(!r.executable);
    assert_eq!(
        r.merges
            .iter()
            .map(|m| (m.merge_id.as_str(), m.disposition.as_str()))
            .collect::<Vec<_>>(),
        [
            ("1", "optional-omitted"),
            ("2", "expanded"),
            ("4", "expanded"),
            ("6", "optional-omitted"),
            ("7", "optional-omitted")
        ]
    );
    for step in &r.steps {
        let original = if step.origin.source == "program" {
            &p
        } else {
            &m
        };
        let slice = &original[step.node.byte_start as usize..step.node.byte_end as usize];
        assert!(slice.starts_with(&format!("<{}", step.node.name)));
        assert_eq!(sha256_hex(original.as_bytes()), step.origin.sha256);
    }
    assert_eq!(r, resolve_package_ap1(&c, &sha, ID).unwrap().unwrap());
}
#[test]
fn duplicate_fragments_are_never_first_or_last_wins() {
    let r = result(&format!("{}{}", fragments(), fragments()), sequence());
    assert_eq!(r.status, "partial");
    assert!(issue(&r, "duplicate-fragment"));
    assert!(r
        .unplaced_declarations
        .iter()
        .any(|s| s.node.name == "LoadProcedure"));
}
#[test]
fn duplicate_merge_points_and_missing_points_stay_partial() {
    let duplicate = format!("{}<LdCtrlMerge MergeId=\"2\"/>", sequence());
    assert!(issue(
        &result(fragments(), &duplicate),
        "invalid-or-duplicate-merge-point"
    ));
    let missing = sequence().replace("<LdCtrlMerge MergeId=\"4\"/>", "");
    assert!(issue(
        &result(fragments(), &missing),
        "missing-mandatory-point"
    ));
}
#[test]
fn nested_and_cyclic_fragments_are_preserved_not_recursively_executed() {
    for nested in [
        r#"<LdCtrlMerge MergeId="2"/>"#,
        r#"<LdCtrlMerge MergeId="4"/>"#,
    ] {
        let f = fragments().replace(r#"<LdCtrlRelSegment LsmIdx="4" Size="16"/>"#, nested);
        let r = result(&f, sequence());
        assert_eq!(r.status, "partial");
        assert!(issue(&r, "nested-or-unresolved-merge"));
        assert!(r.steps.len() < 20);
    }
}
#[test]
fn unknown_conditions_and_children_retain_the_entire_subtree() {
    let f=fragments().replace(r#"<LdCtrlRelSegment LsmIdx="4" Size="16"/>"#,r#"<choose ParamRefId="source-private"><when test="1"><LdCtrlRelSegment LsmIdx="4" Size="16"/></when></choose>"#);
    let r = result(&f, sequence());
    assert_eq!(r.status, "partial");
    assert!(issue(&r, "uninterpreted-step"));
    let choose = &r
        .steps
        .iter()
        .find(|s| s.node.name == "choose")
        .unwrap()
        .node;
    assert_eq!(choose.attributes["ParamRefId"], "source-private");
    assert_eq!(choose.children[0].children[0].name, "LdCtrlRelSegment");
}
#[test]
fn unused_fragments_and_unknown_merge_ids_are_explicit() {
    let f = format!(
        "{}<LoadProcedure MergeId=\"99\"><Unfamiliar/></LoadProcedure>",
        fragments()
    );
    let r = result(&f, sequence());
    assert_eq!(r.status, "partial");
    assert!(issue(&r, "unused-fragment"));
    assert!(r
        .unplaced_declarations
        .iter()
        .any(|s| s.node.name == "LoadProcedure" && s.node.attributes["MergeId"] == "99"));
}
#[test]
fn unplaced_declarations_keep_source_order_outside_the_effective_sequence() {
    let f=format!("{}<LoadProcedure MergeId=\"9\"><UnknownA/></LoadProcedure><LoadProcedure MergeId=\"8\"><UnknownB/></LoadProcedure>",fragments());
    let r = result(&f, sequence());
    let json = serde_json::to_value(&r).unwrap();
    let extra = json["unplacedDeclarations"]
        .as_array()
        .expect("unplaced declarations must be separate");
    assert_eq!(
        extra
            .iter()
            .map(|s| s["node"]["attributes"]["MergeId"].as_str().unwrap())
            .collect::<Vec<_>>(),
        ["9", "8"]
    );
    assert!(r.steps.iter().all(|s| s.node.name != "LoadProcedure"));
}
#[test]
fn unknown_control_attributes_are_reported_not_silently_recognized() {
    let f = fragments().replace("Size=\"16\"", "Size=\"16\" FutureBehavior=\"vendor-data\"");
    let r = result(&f, sequence());
    assert_eq!(r.status, "partial");
    assert!(issue(&r, "uninterpreted-step"));
}
#[test]
fn master_from_another_package_is_never_used_as_a_fallback() {
    let (_t, c, sha) = db(&program(fragments()), &master(sequence()));
    c.execute(
        "DELETE FROM package_member WHERE package_sha256=?1 AND role='Master'",
        [&sha],
    )
    .unwrap();
    let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
    assert_eq!(r.status, "unavailable");
    assert!(issue(&r, "missing-or-ambiguous-master"));
    assert!(r.steps.is_empty());
}
fn replace_program(c: &Connection, package: &str, xml: &str) {
    let old: String = c
        .query_row(
            "SELECT source_sha256 FROM application_program WHERE id=?1",
            [ID],
            |r| r.get(0),
        )
        .unwrap();
    store_source_file(
        c,
        &SourceFile {
            source_path: "M-0001/A.xml".into(),
            manufacturer_id: Some("M-0001".into()),
            bytes: xml.as_bytes().to_vec(),
        },
    )
    .unwrap();
    let sha = sha256_hex(xml.as_bytes());
    c.execute(
        "UPDATE package_member SET source_sha256=?1 WHERE package_sha256=?2 AND source_sha256=?3",
        [&sha, package, &old],
    )
    .unwrap();
    c.execute(
        "UPDATE application_program SET source_sha256=?1 WHERE id=?2",
        [&sha, ID],
    )
    .unwrap();
}
#[test]
fn duplicate_program_ids_and_templates_are_refused_instead_of_selected() {
    let p = program(fragments());
    let start = p.find("<ApplicationProgram Id=").unwrap();
    let end = p.find("</ApplicationProgram>").unwrap() + "</ApplicationProgram>".len();
    let duplicate = p.replacen(
        "</ApplicationPrograms>",
        &format!("{}</ApplicationPrograms>", &p[start..end]),
        1,
    );
    let (_t, c, sha) = db(&p, &master(sequence()));
    replace_program(&c, &sha, &duplicate);
    let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
    assert_eq!(r.status, "unavailable");
    assert!(issue(&r, "missing-or-duplicate-program"));
}
#[test]
fn foreign_and_qualified_identity_names_never_impersonate_knx_declarations() {
    let p = program(fragments());
    for xml in [
        p.replace(
            "<LoadProcedure MergeId=\"2\">",
            "<LoadProcedure xmlns=\"urn:foreign\" MergeId=\"2\">",
        ),
        p.replace("MergeId=\"2\"", "xmlns:x=\"urn:foreign\" x:MergeId=\"2\""),
    ] {
        let (_t, c, sha) = db(&p, &master(sequence()));
        replace_program(&c, &sha, &xml);
        let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
        assert_eq!(r.status, "partial");
        assert!(issue(&r, "missing-mandatory-merge"));
        assert!(issue(&r, "uninterpreted-fragment"));
    }
}
#[test]
fn malformed_xml_missing_and_namespace_conflicting_sources_are_refused() {
    for (xml, expected) in [
        (
            program(fragments()).replace("</Static>", "</Wrong>"),
            "malformed-xml",
        ),
        (
            program(fragments()).replace(NS, "http://knx.org/xml/project/21"),
            "source-namespace-conflict",
        ),
    ] {
        let p = program(fragments());
        let (_t, c, sha) = db(&p, &master(sequence()));
        replace_program(&c, &sha, &xml);
        let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
        assert_eq!(r.status, "unavailable");
        assert!(issue(&r, expected));
    }
}
#[test]
fn depth_and_output_budgets_report_incompleteness_without_promotion() {
    let deep = format!("{}{}", "<Unknown>".repeat(65), "</Unknown>".repeat(65));
    let p = program(&format!(
        "<LoadProcedure MergeId=\"2\">{deep}</LoadProcedure>"
    ));
    let (_t, c, sha) = db(&program(fragments()), &master(sequence()));
    replace_program(&c, &sha, &p);
    let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
    assert!(issue(&r, "xml-work-limit"));
    assert!(!r.executable);
    let f = fragments().replace(
        "<LdCtrlRelSegment LsmIdx=\"4\" Size=\"16\"/>",
        &"<LdCtrlLoad LsmIdx=\"4\"/>".repeat(2049),
    );
    let r = result(&f, sequence());
    assert!(issue(&r, "resolution-output-limit"));
    assert_ne!(r.status, "expanded");
    assert!(!r.executable);
}
#[test]
fn source_resolution_is_read_only_and_does_not_change_existing_mask_refusal() {
    let (_t, c, sha) = db(&program(fragments()), &master(sequence()));
    let before = c.total_changes();
    let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
    assert_eq!(r.status, "expanded");
    assert_eq!(before, c.total_changes());
    let code = knx_productdb::code::load_program_code(&c, ID)
        .unwrap()
        .unwrap();
    assert!(matches!(
        knx_productdb::download_plan::check_program_kind(&code),
        Err(knx_productdb::download_plan::DownloadPlanError::NotMemoryMapped(_))
    ));
}

#[test]
fn invalid_fragment_sharing_a_used_id_is_not_lost() {
    let f = format!(
        "{}<LoadProcedure MergeId=\"2\" Future=\"opaque\"><Unfamiliar/></LoadProcedure>",
        fragments()
    );
    let r = result(&f, sequence());
    assert_eq!(r.unplaced_declarations.len(), 1);
    assert_eq!(
        r.unplaced_declarations[0].node.attributes["Future"],
        "opaque"
    );
}
#[test]
fn conflicting_program_sources_in_one_package_are_not_first_winner_resolution() {
    let temp = tempfile::tempdir().unwrap();
    let c = open_and_migrate(&temp.path().join("p.sqlite")).unwrap();
    let m = master(sequence());
    let p = program(fragments());
    let other = p.replace("Size=\"16\"", "Size=\"32\"");
    let bytes = knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", m.as_bytes()),
        ("M-0001/A.xml", p.as_bytes()),
        ("M-0001/B.xml", other.as_bytes()),
    ]);
    let installed = install_package(&c, "synthetic.knxprod", &bytes).unwrap();
    let r = resolve_package_ap1(&c, &installed.sha256, ID)
        .unwrap()
        .unwrap();
    assert_eq!(r.status, "unavailable");
    assert!(issue(&r, "ambiguous-program-source"));
}
#[test]
fn empty_optional_fragment_is_not_misreported_as_a_missing_mandatory_merge() {
    let r = result(
        &format!("{}<LoadProcedure MergeId=\"1\"/>", fragments()),
        sequence(),
    );
    assert_eq!(r.status, "partial");
    assert!(issue(&r, "empty-fragment"));
    assert!(!issue(&r, "missing-mandatory-merge"));
    assert_eq!(r.unplaced_declarations.len(), 1);
}
#[test]
fn duplicate_ap1_templates_across_wrappers_are_not_selected() {
    let m = master(sequence());
    let wrapper_start = m.find("<HawkConfigurationData>").unwrap();
    let wrapper_end =
        m.find("</HawkConfigurationData>").unwrap() + "</HawkConfigurationData>".len();
    let duplicate = m.replacen(
        "</MaskVersion>",
        &format!("{}</MaskVersion>", &m[wrapper_start..wrapper_end]),
        1,
    );
    let (_t, c, sha) = db(&program(fragments()), &duplicate);
    let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
    assert_eq!(r.status, "unavailable");
    assert!(issue(&r, "missing-or-duplicate-template"));
    assert!(r.steps.is_empty());
}
#[test]
fn qualified_template_containers_are_reported_without_inferred_selection() {
    for m in [
        master(sequence()).replace(
            "<HawkConfigurationData>",
            "<HawkConfigurationData LegacyVersion=\"1\">",
        ),
        master(sequence()).replace("<Procedures>", "<Procedures Future=\"opaque\">"),
    ] {
        let (_t, c, sha) = db(&program(fragments()), &m);
        let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
        assert_eq!(r.status, "unavailable");
        assert!(issue(&r, "uninterpreted-template-container"));
        assert!(r.steps.is_empty());
    }
}
#[test]
fn fabricated_direct_mask_templates_are_not_a_fallback() {
    let m = master(sequence())
        .replace("<HawkConfigurationData>", "")
        .replace("</HawkConfigurationData>", "");
    let (_t, c, sha) = db(&program(fragments()), &m);
    let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
    assert_eq!(r.status, "unavailable");
    assert!(issue(&r, "missing-or-duplicate-template"));
}
#[test]
fn default_procedure_never_silently_consumes_product_fragments() {
    let p = program(fragments()).replace("MergedProcedure", "DefaultProcedure");
    let (_t, c, sha) = db(&p, &master(sequence()));
    let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
    assert_eq!(r.status, "partial");
    assert!(issue(&r, "fragments-on-default-procedure"));
    assert_eq!(r.unplaced_declarations.len(), 2);
    assert!(r.steps.iter().all(|s| s.origin.source == "master"));
}
#[test]
fn source_integrity_is_checked_before_resolution() {
    let (_t, c, sha) = db(&program(fragments()), &master(sequence()));
    c.execute("UPDATE source_file SET bytes=CAST(replace(CAST(bytes AS TEXT),'Size=\"16\"','Size=\"32\"') AS BLOB) WHERE source_path='M-0001/A.xml'",[]).unwrap();
    let r = resolve_package_ap1(&c, &sha, ID).unwrap().unwrap();
    assert!(issue(&r, "source-hash-mismatch"));
    assert!(r.steps.is_empty());
}
