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
    assert_known_five_steps(&r, &p, &m);
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
    // Broad output-safety coverage only: these attributed controls exhaust
    // output charge before 2048 roots. The dedicated count test isolates that cap.
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

fn attributes(pairs: &[(&str, &str)]) -> std::collections::BTreeMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).into(), (*v).into()))
        .collect()
}

fn assert_control(
    step: &knx_productdb::procedure_resolution::Step,
    original: &str,
    source: &str,
    location: &str,
    name: &str,
    xml: &str,
    attrs: &[(&str, &str)],
) {
    assert_eq!(step.node.namespace, NS);
    assert_eq!(step.node.name, name);
    assert_eq!(step.node.attributes, attributes(attrs));
    assert!(step.node.children.is_empty());
    assert!(step.node.text.is_empty());
    assert_eq!(step.origin.source, source);
    assert_eq!(step.origin.sha256, sha256_hex(original.as_bytes()));
    assert_eq!(step.origin.location, location);
    let start = usize::try_from(step.node.byte_start).unwrap();
    let end = usize::try_from(step.node.byte_end).unwrap();
    assert!(start > 0 && start < end && end <= original.len());
    assert_eq!(&original[start..end], xml);
    assert_eq!(end - start, xml.len());
}

fn assert_identity_and_sources(r: &Resolution, p: &str, m: &str) {
    assert_eq!(r.format_version, 1);
    assert_eq!(r.program_id, ID);
    assert_eq!(r.mask, "MV-07B0");
    assert_eq!(r.style, "MergedProcedure");
    assert_eq!(r.variant, "ap1");
    assert!(!r.executable);
    assert_eq!(
        r.program_attributes,
        attributes(&[
            ("Id", ID),
            ("MaskVersion", "MV-07B0"),
            ("LoadProcedureStyle", "MergedProcedure"),
        ])
    );
    assert_eq!(
        r.template_attributes,
        attributes(&[
            ("ProcedureType", "Load"),
            ("ProcedureSubType", "ap1"),
            ("Access", "remote local2"),
        ])
    );
    assert_eq!(r.sources.len(), 2);
    for (actual, (kind, original)) in r.sources.iter().zip([("program", p), ("master", m)]) {
        assert_eq!(actual.source, kind);
        assert_eq!(actual.sha256, sha256_hex(original.as_bytes()));
        assert_eq!(actual.namespace, NS);
    }
}

fn assert_clean_expansion(r: &Resolution, p: &str, m: &str) {
    assert_identity_and_sources(r, p, m);
    assert_eq!(r.status, "expanded");
    assert!(r.issues.is_empty());
    assert!(r.unplaced_declarations.is_empty());
    assert_eq!(r.omitted_issue_count, 0);
}

type ControlExpectation<'a> = (&'a str, &'a str, &'a str, &'a str, &'a [(&'a str, &'a str)]);

fn assert_known_five_steps(r: &Resolution, p: &str, m: &str) {
    assert_clean_expansion(r, p, m);
    assert_eq!(r.steps.len(), 5);
    let expected: [ControlExpectation<'_>; 5] = [
        (
            "master",
            "master/template/step-1",
            "LdCtrlConnect",
            "<LdCtrlConnect/>",
            &[],
        ),
        (
            "master",
            "master/template/step-3",
            "LdCtrlLoad",
            r#"<LdCtrlLoad LsmIdx="4"/>"#,
            &[("LsmIdx", "4")],
        ),
        (
            "program",
            "program/merge-2/step-1",
            "LdCtrlRelSegment",
            r#"<LdCtrlRelSegment LsmIdx="4" Size="16"/>"#,
            &[("LsmIdx", "4"), ("Size", "16")],
        ),
        (
            "program",
            "program/merge-4/step-1",
            "LdCtrlWriteRelMem",
            r#"<LdCtrlWriteRelMem ObjIdx="4" Size="16"/>"#,
            &[("ObjIdx", "4"), ("Size", "16")],
        ),
        (
            "master",
            "master/template/step-8",
            "LdCtrlRestart",
            "<LdCtrlRestart/>",
            &[],
        ),
    ];
    let mut program_end = 0;
    let mut master_end = 0;
    for (step, (source, location, name, xml, attrs)) in r.steps.iter().zip(expected) {
        let original = if source == "program" { p } else { m };
        assert_control(step, original, source, location, name, xml, attrs);
        assert_eq!(step.node.byte_start as usize, original.find(xml).unwrap());
        let previous_end = if source == "program" {
            &mut program_end
        } else {
            &mut master_end
        };
        assert!(step.node.byte_start >= *previous_end);
        *previous_end = step.node.byte_end;
    }
    assert_eq!(r.merges.len(), 5);
    for (actual, (id, disposition, location)) in r.merges.iter().zip([
        ("1", "optional-omitted", "master/template/step-2"),
        ("2", "expanded", "master/template/step-4"),
        ("4", "expanded", "master/template/step-5"),
        ("6", "optional-omitted", "master/template/step-6"),
        ("7", "optional-omitted", "master/template/step-7"),
    ]) {
        assert_eq!(actual.merge_id, id);
        assert_eq!(actual.disposition, disposition);
        assert_eq!(actual.origin.source, "master");
        assert_eq!(actual.origin.sha256, sha256_hex(m.as_bytes()));
        assert_eq!(actual.origin.location, location);
    }
}

type TableSnapshot = Vec<(String, Vec<Vec<rusqlite::types::Value>>)>;
type FileSnapshot = Vec<(std::ffi::OsString, Vec<u8>, String)>;

// Snapshot every logical table, schema and all actual database/sidecar bytes.
// This is deliberately stronger than the connection-local total_changes check.
fn catalogue_snapshot(c: &Connection, root: &std::path::Path) -> (TableSnapshot, FileSnapshot) {
    let names: Vec<String> = c
        .prepare("SELECT name FROM sqlite_schema WHERE type='table' ORDER BY name")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .collect::<Result<_, _>>()
        .unwrap();
    let rows = std::iter::once("sqlite_schema".to_owned())
        .chain(names)
        .map(|name| {
            let query = format!("SELECT * FROM \"{}\"", name.replace('"', "\"\""));
            let mut statement = c.prepare(&query).unwrap();
            let columns = statement.column_count();
            let rows = statement
                .query_map([], |row| {
                    (0..columns)
                        .map(|i| row.get(i))
                        .collect::<Result<Vec<rusqlite::types::Value>, _>>()
                })
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            (name, rows)
        })
        .collect();
    let mut files: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let bytes = std::fs::read(entry.path()).unwrap();
            let sha = sha256_hex(&bytes);
            (entry.file_name(), bytes, sha)
        })
        .collect();
    files.sort_by(|a, b| a.0.cmp(&b.0));
    (rows, files)
}

fn assert_retained_sources(c: &Connection, package: &str, p: &str, m: &str) {
    use std::io::{Cursor, Read};
    let bytes: Vec<u8> = c
        .query_row(
            "SELECT bytes FROM package WHERE sha256=?1",
            [package],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(sha256_hex(&bytes), package);
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    assert_eq!(archive.len(), 2);
    for (path, role, original) in [
        ("M-0001/A.xml", "ApplicationProgram", p),
        ("knx_master.xml", "Master", m),
    ] {
        let sha = sha256_hex(original.as_bytes());
        let (stored_path, declared, actual): (String, i64, i64) = c
            .query_row(
                "SELECT source_path,len,length(bytes) FROM source_file WHERE sha256=?1",
                [&sha],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(stored_path, path);
        assert_eq!(declared, original.len() as i64);
        assert_eq!(actual, declared);
        let retained = load_source_file(c, &sha).unwrap().unwrap();
        assert!(
            retained == original.as_bytes(),
            "installed source bytes changed"
        );
        assert_eq!(sha256_hex(&retained), sha);
        let owned: bool = c.query_row(
            "SELECT EXISTS(SELECT 1 FROM package_member WHERE package_sha256=?1 AND source_sha256=?2 AND role=?3)",
            [package, &sha, role], |row| row.get(0)
        ).unwrap();
        assert!(owned, "source must retain its actual package role");
        let mut materialized = Vec::new();
        archive
            .by_name(path)
            .unwrap()
            .read_to_end(&mut materialized)
            .unwrap();
        assert!(
            materialized == retained,
            "package member differs from installed source"
        );
        assert_eq!(sha256_hex(&materialized), sha);
    }
    let program_sha: String = c
        .query_row(
            "SELECT source_sha256 FROM application_program WHERE id=?1",
            [ID],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(program_sha, sha256_hex(p.as_bytes()));
}

fn resolve_unchanged(
    temp: &tempfile::TempDir,
    c: &Connection,
    sha: &str,
    p: &str,
    m: &str,
) -> Resolution {
    assert_retained_sources(c, sha, p, m);
    let before = catalogue_snapshot(c, temp.path());
    let changes = c.total_changes();
    let r = resolve_package_ap1(c, sha, ID).unwrap().unwrap();
    assert_eq!(changes, c.total_changes());
    assert_retained_sources(c, sha, p, m);
    assert!(
        catalogue_snapshot(c, temp.path()) == before,
        "resolution changed catalogue rows/schema or file bytes/hashes"
    );
    r
}

fn pad_source(xml: &str, bytes: usize) -> String {
    // A single valid comment after the relevant subtree, inside the KNX root.
    // No extra retained nodes, text, or output is created by the padding.
    let padding = bytes.checked_sub(xml.len() + "<!---->".len()).unwrap();
    let padded = xml.replacen(
        "</KNX>",
        &format!("<!--{}--></KNX>", "x".repeat(padding)),
        1,
    );
    assert_eq!(padded.len(), bytes);
    padded
}

#[test]
fn exact_source_byte_bound_preserves_expansion_and_refusal() {
    assert_eq!(
        knx_productdb::procedure_resolution::MAX_XML_BYTES,
        8_388_608
    );
    for source in ["program", "master"] {
        for length in [8_388_608, 8_388_609] {
            let p = program(fragments());
            let m = master(sequence());
            let p = if source == "program" {
                pad_source(&p, length)
            } else {
                p
            };
            let m = if source == "master" {
                pad_source(&m, length)
            } else {
                m
            };
            assert_eq!(
                if source == "program" {
                    p.len()
                } else {
                    m.len()
                },
                length
            );
            assert!(
                if source == "program" {
                    m.len()
                } else {
                    p.len()
                } < 8_388_608
            );
            // Install the actual intended sources, retaining authentic candidate provenance.
            let (temp, c, sha) = db(&p, &m);
            let r = resolve_unchanged(&temp, &c, &sha, &p, &m);
            if length == 8_388_608 {
                assert_known_five_steps(&r, &p, &m);
            } else {
                assert_eq!(r.status, "unavailable");
                assert!(!r.executable);
                assert!(r.steps.is_empty());
                assert!(r.unplaced_declarations.is_empty());
                assert!(r.merges.is_empty());
                assert_eq!(r.omitted_issue_count, 0);
                assert_eq!(
                    r.issues
                        .iter()
                        .map(|i| (i.code.as_str(), i.location.as_str()))
                        .collect::<Vec<_>>(),
                    [("source-byte-limit", source)]
                );
            }
            eprintln!(
                "source-byte-bound: source={source} bytes={length} status={} steps={}",
                r.status,
                r.steps.len()
            );
        }
    }
}

// Independently check the documented charge using the actual public result.
// Literal boundary assertions below pin policy; this helper does not choose fixtures.
fn output_charge(r: &Resolution) -> usize {
    fn node_charge(n: &knx_productdb::procedure_resolution::Node) -> usize {
        256 + 6 * (n.namespace.len() + n.name.len() + n.text.len())
            + n.attributes
                .iter()
                .map(|(k, v)| 32 + 6 * (k.len() + v.len()))
                .sum::<usize>()
            + n.children.iter().map(node_charge).sum::<usize>()
    }
    let metadata = 512
        + 6 * r
            .program_attributes
            .iter()
            .chain(r.template_attributes.iter())
            .map(|(k, v)| k.len() + v.len())
            .sum::<usize>();
    assert_eq!(metadata, 1_220);
    metadata
        + r.steps
            .iter()
            .chain(&r.unplaced_declarations)
            .map(|s| node_charge(&s.node))
            .sum::<usize>()
}

#[test]
fn step_count_limit_is_independent_of_output_charge() {
    for repeated in [2_044, 2_045] {
        let f = fragments().replace(
            r#"<LdCtrlRelSegment LsmIdx="4" Size="16"/>"#,
            &"<LdCtrlLoad/>".repeat(repeated),
        );
        let p = program(&f);
        let m = master(sequence());
        // Four fixed roots + these light roots; even the requested 2049th root
        // fits the byte allowance, so output charge cannot be the first refusal.
        let requested_roots = repeated + 4;
        let requested_charge = 1_220 + 508 + 564 + repeated * 490 + 674 + 508;
        assert_eq!(
            requested_roots,
            if repeated == 2_044 { 2_048 } else { 2_049 }
        );
        assert_eq!(
            requested_charge,
            if repeated == 2_044 {
                1_005_034
            } else {
                1_005_524
            }
        );
        assert!(requested_charge < 1_048_576);
        let (temp, c, sha) = db(&p, &m);
        let r = resolve_unchanged(&temp, &c, &sha, &p, &m);
        assert_identity_and_sources(&r, &p, &m);
        assert_eq!(r.steps.len(), 2_048);
        assert!(r.unplaced_declarations.is_empty());
        assert_eq!(r.omitted_issue_count, 0);
        assert_control(
            &r.steps[0],
            &m,
            "master",
            "master/template/step-1",
            "LdCtrlConnect",
            "<LdCtrlConnect/>",
            &[],
        );
        assert_control(
            &r.steps[1],
            &m,
            "master",
            "master/template/step-3",
            "LdCtrlLoad",
            r#"<LdCtrlLoad LsmIdx="4"/>"#,
            &[("LsmIdx", "4")],
        );
        let start = p.find("<LdCtrlLoad/>").unwrap();
        for (i, step) in r.steps[2..2 + repeated].iter().enumerate() {
            assert_control(
                step,
                &p,
                "program",
                &format!("program/merge-2/step-{}", i + 1),
                "LdCtrlLoad",
                "<LdCtrlLoad/>",
                &[],
            );
            assert_eq!(
                step.node.byte_start as usize,
                start + i * "<LdCtrlLoad/>".len()
            );
        }
        assert_control(
            &r.steps[2 + repeated],
            &p,
            "program",
            "program/merge-4/step-1",
            "LdCtrlWriteRelMem",
            r#"<LdCtrlWriteRelMem ObjIdx="4" Size="16"/>"#,
            &[("ObjIdx", "4"), ("Size", "16")],
        );
        if repeated == 2_044 {
            assert_clean_expansion(&r, &p, &m);
            assert_control(
                &r.steps[2_047],
                &m,
                "master",
                "master/template/step-8",
                "LdCtrlRestart",
                "<LdCtrlRestart/>",
                &[],
            );
            assert_eq!(output_charge(&r), 1_005_034);
        } else {
            assert_eq!(r.status, "partial");
            assert_eq!(
                r.issues
                    .iter()
                    .map(|i| (i.code.as_str(), i.location.as_str()))
                    .collect::<Vec<_>>(),
                [("resolution-output-limit", "source")]
            );
            assert_eq!(output_charge(&r), 1_005_016);
            assert_eq!(output_charge(&r) + 508, requested_charge);
            assert!(r.steps.iter().all(|s| s.node.name != "LdCtrlRestart"));
        }
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
                ("7", "optional-omitted"),
            ]
        );
        eprintln!("root-count-bound: requested={requested_roots} requested_charge={requested_charge} retained={} retained_charge={} status={}", r.steps.len(), output_charge(&r), r.status);
    }
}

#[test]
fn output_charge_preserves_exact_prefix_at_its_boundary() {
    // The charged node must be LAST: with the standard five-step fixture, +6
    // would refuse Restart, not RelSegment, because two suffix roots still fit.
    // These three roots preserve mandatory merges 2/4 while isolating a clone
    // refusal at the third root and keeping the exact Connect/Load prefix.
    let s = r#"<LdCtrlConnect/><LdCtrlMerge MergeId="1"/><LdCtrlMerge MergeId="2"/><LdCtrlMerge MergeId="4"/><LdCtrlMerge MergeId="6"/><LdCtrlMerge MergeId="7"/>"#;
    for extra in [0, 1] {
        let size = format!("16{}", "x".repeat(174_260 + extra));
        let xml = format!(r#"<LdCtrlRelSegment LsmIdx="4" Size="{size}" Mode=""/>"#);
        let f = format!(
            r#"<LoadProcedure MergeId="2"><LdCtrlLoad LsmIdx="4"/></LoadProcedure><LoadProcedure MergeId="4">{xml}</LoadProcedure>"#
        );
        let p = program(&f);
        let m = master(s);
        // 1220 metadata + Connect 508 + Load 564 + RelSegment 668 +
        // empty Mode 56 + six charged bytes per additional ASCII Size byte.
        assert_eq!(size.len(), 174_262 + extra);
        let requested_charge = 1_220 + 508 + 564 + 668 + 56 + 6 * (174_260 + extra);
        assert_eq!(
            requested_charge,
            if extra == 0 { 1_048_576 } else { 1_048_582 }
        );
        assert!(p.len() < 8_388_608 && m.len() < 8_388_608);
        let (temp, c, sha) = db(&p, &m);
        let r = resolve_unchanged(&temp, &c, &sha, &p, &m);
        assert_identity_and_sources(&r, &p, &m);
        assert!(r.unplaced_declarations.is_empty());
        assert_eq!(r.omitted_issue_count, 0);
        assert_control(
            &r.steps[0],
            &m,
            "master",
            "master/template/step-1",
            "LdCtrlConnect",
            "<LdCtrlConnect/>",
            &[],
        );
        assert_control(
            &r.steps[1],
            &p,
            "program",
            "program/merge-2/step-1",
            "LdCtrlLoad",
            r#"<LdCtrlLoad LsmIdx="4"/>"#,
            &[("LsmIdx", "4")],
        );
        if extra == 0 {
            assert_clean_expansion(&r, &p, &m);
            assert_eq!(r.steps.len(), 3);
            assert_control(
                &r.steps[2],
                &p,
                "program",
                "program/merge-4/step-1",
                "LdCtrlRelSegment",
                &xml,
                &[("LsmIdx", "4"), ("Size", &size), ("Mode", "")],
            );
            assert_eq!(output_charge(&r), 1_048_576);
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
                    ("7", "optional-omitted"),
                ]
            );
        } else {
            assert_eq!(r.status, "partial");
            assert_eq!(r.steps.len(), 2);
            assert_eq!(output_charge(&r), 2_292);
            assert_eq!(
                r.issues
                    .iter()
                    .map(|i| (i.code.as_str(), i.location.as_str()))
                    .collect::<Vec<_>>(),
                [("resolution-output-limit", "source")]
            );
            assert_eq!(
                r.merges
                    .iter()
                    .map(|m| (m.merge_id.as_str(), m.disposition.as_str()))
                    .collect::<Vec<_>>(),
                [
                    ("1", "optional-omitted"),
                    ("2", "expanded"),
                    ("4", "expanded"),
                ]
            );
        }
        eprintln!("output-charge-bound: requested={requested_charge} retained_charge={} roots={} status={}", output_charge(&r), r.steps.len(), r.status);
    }
}
