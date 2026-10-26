//! Universal support reports remain useful without a catalogue, plan or gateway.
use knx_app::support_report::{build_report, FunctionalCheck, ReportInput};

#[test]
fn a_prior_partial_run_keeps_its_scope_but_never_parameters_or_free_text() {
    use knx_app::commissioning_activity::{Restart, Written};
    use knx_app::download_report::{
        DownloadReport, ReportedOutcome, ReportedPlan, ReportedProgram,
    };
    let prior = DownloadReport {
        kind: "knxbench-download-report",
        format: 1,
        knxbench_version: "old".into(),
        program: ReportedProgram {
            id: "PRIVATE_ID".into(),
            name: Some("PRIVATE_NAME".into()),
            application_version: None,
            source_sha256: None,
            manufacturer: "0001".into(),
            mask: "0025".into(),
        },
        scope: "partial-parameters",
        support: "untested",
        inferences: vec!["PRIVATE_INFERENCE".into()],
        warnings: vec!["PRIVATE_WARNING".into()],
        plan: ReportedPlan {
            steps: 2,
            data_octets: 3,
            group_links: 1,
        },
        outcome: Some(ReportedOutcome::completed(
            Written::Yes,
            Restart::Acknowledged,
        )),
        post_download_check: None,
        failure: None,
        parameter_values: std::collections::BTreeMap::from([(
            "PRIVATE_REF".into(),
            "PRIVATE_VALUE".into(),
        )]),
    };
    let mut report = build_report(
        None,
        None,
        None,
        &ReportInput {
            functional_check: Some(FunctionalCheck::WorksAsConfigured),
            ..Default::default()
        },
        "test",
    );
    report.prior_run(&prior);
    let v = serde_json::to_value(&report).unwrap();
    assert_eq!(v["priorRunScope"], "partial-parameters");
    assert_eq!(v["priorRun"]["completed"], true);
    assert!(!report
        .missing_evidence
        .iter()
        .any(|x| x == "user-functional-check"));
    assert!(!v.to_string().contains("PRIVATE_"));
    assert!(report.summary().contains("partial-parameters"));
}

#[test]
fn unsupported_product_keeps_useful_numeric_context_but_no_original_or_secret_payloads() {
    let id = "M-0001_A-9205-04-9270";
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="{id}" MaskVersion="MV-0025" ApplicationVersion="4" Name="CUSTOMER_SECRET" LoadProcedureStyle="ProductProcedure"><Static><Code><AbsoluteSegment Id="SECRET_SEGMENT" Address="1024" Size="4"><Data>3q2+7w==</Data></AbsoluteSegment></Code><ComObjectTable CodeSegment="SECRET_SEGMENT" Offset="0"/><LoadProcedures><LoadProcedure><LdCtrlConnect/><LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="DEADBEEF"/><LdCtrlMystery SECRET="PRIVATE_KEY"><Nested/></LdCtrlMystery><LdCtrlDisconnect/></LoadProcedure></LoadProcedures></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let master=br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Synthetic"/></Manufacturers></MasterData></KNX>"#;
    let bytes = knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", master),
        ("M-0001/M-0001_A-9205-04-9270.xml", xml.as_bytes()),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    knx_productdb::install_package(&conn, "fixture.knxprod", &bytes).unwrap();
    let input = ReportInput {
        program_id: Some(id.into()),
        include_context: true,
        context_rights: true,
        ..Default::default()
    };
    let report = build_report(Some(&conn), None, None, &input, "test");
    assert_eq!(report.support, "unsupported");
    let context = report.context.as_ref().unwrap();
    assert_eq!(context.steps.len(), 4);
    assert_eq!(context.omitted_steps, 1);
    assert_eq!(context.segments, [(1024, 4)]);
    let s = serde_json::to_string(&report).unwrap();
    for secret in [
        "CUSTOMER_SECRET",
        "SECRET_SEGMENT",
        "PRIVATE_KEY",
        "DEADBEEF",
        "3q2+7w==",
        "LdCtrlMystery",
    ] {
        assert!(!s.contains(secret), "{secret}");
    }
    let no_rights = ReportInput {
        context_rights: false,
        ..input
    };
    assert!(build_report(Some(&conn), None, None, &no_rights, "test")
        .context
        .is_none());
    conn.execute("UPDATE source_file SET bytes=zeroblob(8388609), len=1 WHERE sha256=(SELECT source_sha256 FROM application_program WHERE id=?1)",[id]).unwrap();
    let bounded = build_report(Some(&conn), None, None, &no_rights, "test");
    assert_eq!(bounded.support, "unknown");
    assert!(bounded.context.is_none());
    assert!(bounded
        .missing_evidence
        .iter()
        .any(|s| s == "product-source-over-budget-or-unavailable"));
}

#[test]
fn rejected_manual_identifiers_and_absent_sources_never_become_success() {
    let input = ReportInput {
        program_id: Some("CUSTOMER /home/private 1.1.5".into()),
        firmware: Some("secret value with spaces".into()),
        ..Default::default()
    };
    let r = build_report(None, None, None, &input, "test");
    let s = serde_json::to_string(&r).unwrap();
    assert!(!s.contains("CUSTOMER"));
    assert!(!s.contains("/home/private"));
    assert!(!s.contains("secret value"));
    assert_eq!(r.support, "unknown");
    assert!(r
        .withheld
        .iter()
        .any(|x| x == "unrecognised-application-id-withheld"));
}

#[test]
fn manual_canonical_identity_and_firmware_keep_their_user_origin() {
    let i = ReportInput {
        program_id: Some("M-0001_A-9205-04-9270".into()),
        firmware: Some("25N03-V04".into()),
        mask: Some(0x0025),
        ..Default::default()
    };
    let r = build_report(None, None, None, &i, "test");
    assert_eq!(r.support, "unknown");
    assert!(r.facts.iter().any(|f| f.field == "firmware"
        && f.origin == "user"
        && f.value.as_deref() == Some("25N03-V04")));
    assert!(r
        .missing_evidence
        .iter()
        .any(|f| f == "matching-product-package"));
    let bytes = r.bundle().unwrap();
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).unwrap();
    assert_eq!(zip.len(), 2);
    assert!(zip.by_name("report.json").is_ok());
    assert!(zip.by_name("README.txt").is_ok());
}

#[test]
fn observed_application_reference_suffixes_are_preserved_not_repaired() {
    let id = "M-006A_A-0001-22-617E-O0079";
    let r = build_report(
        None,
        None,
        None,
        &ReportInput {
            program_id: Some(id.into()),
            ..Default::default()
        },
        "test",
    );
    assert!(r.facts.iter().any(|f| f.field == "applicationId"
        && f.origin == "user"
        && f.value.as_deref() == Some(id)));
}

#[test]
fn conflicting_user_identity_is_preserved_as_an_assertion_not_the_selected_source() {
    use knx_core::{DeviceId, DeviceInstance, Language, Project, SourceRef};
    let mut project = Project::new(Language("en".into()));
    project.devices.insert(DeviceInstance {
        id: DeviceId(7),
        source: SourceRef {
            path: "PRIVATE_PATH".into(),
            ets_id: "PRIVATE_SOURCE".into(),
        },
        name: "PRIVATE_NAME".into(),
        description: None,
        address: None,
        product_ref: "PRIVATE_PRODUCT".into(),
        program_ref: "M-0001_A-9205-04-9270".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let input = ReportInput {
        program_id: Some("M-0002_A-9205-04-9270".into()),
        ..Default::default()
    };
    let report = build_report(None, Some(&project), Some(DeviceId(7)), &input, "test");
    assert!(report.facts.iter().any(|f| f.field == "applicationId"
        && f.origin == "project"
        && f.value.as_deref() == Some("M-0001_A-9205-04-9270")));
    assert!(report
        .facts
        .iter()
        .any(|f| f.field == "assertedApplicationId"
            && f.origin == "user"
            && f.value == input.program_id));
    assert!(report
        .missing_evidence
        .iter()
        .any(|m| m == "application-identity-conflict"));
    assert!(!serde_json::to_string(&report).unwrap().contains("PRIVATE_"));
}

#[test]
fn declared_text_buffers_are_budgeted_before_optional_plan_assessment() {
    let id = "M-0001_A-9205-04-9270";
    let bits = (8u64 * 1024 * 1024 + 1) * 8;
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001"><ApplicationPrograms><ApplicationProgram Id="{id}" MaskVersion="MV-0025" ApplicationVersion="4" LoadProcedureStyle="ProductProcedure"><Static><Code><AbsoluteSegment Id="S" Address="1024" Size="4"><Data>AAAAAA==</Data></AbsoluteSegment></Code><ParameterTypes><ParameterType Id="T"><TypeText SizeInBit="{bits}"/></ParameterType></ParameterTypes><Parameters><Parameter Id="P" ParameterType="T" Value=""><Memory CodeSegment="S" Offset="0" BitOffset="0"/></Parameter></Parameters><ParameterRefs><ParameterRef Id="PR" RefId="P"/><ParameterRef Id="PR2" RefId="P"/></ParameterRefs><LoadProcedures><LoadProcedure><LdCtrlConnect/><LdCtrlDisconnect/></LoadProcedure></LoadProcedures></Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#
    );
    let master=br#"<KNX xmlns="http://knx.org/xml/project/20"><MasterData><Manufacturers><Manufacturer Id="M-0001" Name="Synthetic"/></Manufacturers></MasterData></KNX>"#;
    let bytes = knx_testsupport::zip_with_entries(&[
        ("knx_master.xml", master),
        ("M-0001/M-0001_A-9205-04-9270.xml", xml.as_bytes()),
    ]);
    let dir = tempfile::tempdir().unwrap();
    let conn = knx_productdb::open_and_migrate(&dir.path().join("p.sqlite")).unwrap();
    knx_productdb::install_package(&conn, "fixture.knxprod", &bytes).unwrap();
    let code = knx_productdb::code::load_program_code(&conn, id)
        .unwrap()
        .unwrap();
    let declared_total = (bits / 8) * 2;
    assert!(
        knx_productdb::image::text_parameter_buffers_fit(&conn, id, &code, declared_total).unwrap()
    );
    assert!(!knx_productdb::image::text_parameter_buffers_fit(
        &conn,
        id,
        &code,
        declared_total - 1
    )
    .unwrap());
    let input = ReportInput {
        program_id: Some(id.into()),
        include_context: true,
        context_rights: true,
        ..Default::default()
    };
    let r = build_report(Some(&conn), None, None, &input, "test");
    assert!(r.context.is_some(), "source context stays available");
    assert!(r
        .missing_evidence
        .iter()
        .any(|m| m == "plan-assessment-unavailable-or-over-budget"));
    assert_eq!(
        r.support, "unknown",
        "omitted assessment must not invent compatibility"
    );
    conn.execute(
        "UPDATE parameter_type SET size_in_bit=0 WHERE program_id=?1",
        [id],
    )
    .unwrap();
    assert!(
        knx_productdb::image::text_parameter_buffers_fit(&conn, id, &code, declared_total).is_err()
    );
    let malformed = build_report(Some(&conn), None, None, &input, "test");
    assert!(malformed
        .missing_evidence
        .iter()
        .any(|m| m == "plan-assessment-unavailable-or-over-budget"));
}

#[test]
fn completely_unknown_device_still_has_a_shareable_offline_report() {
    let report = build_report(None, None, None, &ReportInput::default(), "test-build");
    let value = serde_json::to_value(&report).unwrap();
    assert_eq!(value["kind"], "knxbench-support-report");
    assert_eq!(value["format"], 1);
    assert_eq!(value["support"], "unknown");
    assert_eq!(value["written"], false);
    assert!(value["missingEvidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "product-identity"));
    assert!(value["missingEvidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|x| x == "hardware-verification"));
}
