//! Retained RTF-like values stay full raw evidence; reports only preview them.

#[test]
fn retained_rtf_candidate_is_marked_without_changing_full_source_or_native_bytes() {
    let raw = r"{\rtf1\ansi <b>literal</b> \u20320?}";
    let xml = format!(
        r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0"><Topology><Area Id="A-TEST" Address="9"><Line Id="L-TEST" Address="7"><Segment Id="S-TEST" Number="1"><DeviceInstance Id="D-TEST" Address="231" Name="Synthetic RTF witness" ProductRefId="M-TEST_P-TEST" Hardware2ProgramRefId="M-TEST_HP-TEST" InstallationHints="{}"><GroupObjectTree/></DeviceInstance></Segment></Line></Area></Topology><GroupAddresses/></Installation></Installations></Project></KNX>"#,
        raw.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    );
    let info = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><ProjectInformation Name="Synthetic RTF project"/></Project></KNX>"#;
    let dir = tempfile::tempdir().unwrap();
    let native = dir.path().join("native.knxdb");
    let conn = knx_store::open_and_migrate(&native).unwrap();
    let imported = knx_app::import_ets_project_bytes(
        knx_testsupport::zip_with_entries(&[
            ("P-TEST.signature", b""),
            ("P-TEST/0.xml", xml.as_bytes()),
            ("P-TEST/project.xml", info),
        ]),
        "synthetic.knxproj",
        &conn,
        knx_app::ImportOptions::default(),
    )
    .unwrap();
    assert_eq!(imported.report.error_count(), 0);
    assert!(
        imported
            .report
            .opaque
            .iter()
            .any(|e| e.name == "InstallationHints"),
        "synthetic retained names: {:?}",
        imported
            .report
            .opaque
            .iter()
            .map(|e| &e.name)
            .collect::<Vec<_>>()
    );
    let report: serde_json::Value = serde_json::from_str(&imported.report.to_json()).unwrap();
    let summary = report["opaque"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["name"] == "InstallationHints")
        .unwrap();
    assert_eq!(summary["raw_text_preview"]["format"], "rtf-like");
    assert_eq!(summary["raw_text_preview"]["value"], raw);
    assert_eq!(summary["raw_text_preview"]["truncated"], false);
    let before = knx_store::load_opaque(&conn).unwrap();
    assert!(before
        .iter()
        .any(|e| e.name == "InstallationHints" && e.bytes == raw.as_bytes()));
    assert!(before.iter().any(|e| e.source_path == "P-TEST/0.xml"
        && e.xpath.is_empty()
        && e.bytes == xml.as_bytes()));
    knx_store::save_project(&conn, &imported.project).unwrap();
    drop(conn);
    let conn = knx_store::open_existing_and_migrate(&native).unwrap();
    assert_eq!(knx_store::load_opaque(&conn).unwrap(), before);
}
