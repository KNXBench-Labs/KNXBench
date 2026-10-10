//! Synthetic service-level persistence witness for inactive-tree override evidence.

#[test]
fn outside_tree_overrides_are_reported_and_preserved_without_activation() {
    let base_topology = r#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><Installations><Installation InstallationId="0"><Topology><Area Id="P-TEST-0_A-91" Address="9"><Line Id="P-TEST-0_L-71" Address="7"><Segment Id="P-TEST-0_S-41" Number="1"><DeviceInstance Id="P-TEST-0_DI-271" Address="53" ProductRefId="M-TEST_P-UNUSED" Hardware2ProgramRefId="M-TEST_H-UNUSED_HP-UNUSED"><ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-19_R-271" Links="GA-321" ReadFlag="Enabled" WriteFlag="Disabled" Text="Synthetic outside-tree override"/></ComObjectInstanceRefs><GroupObjectTree/></DeviceInstance></Segment></Line></Area></Topology><GroupAddresses><GroupRanges><GroupRange Id="P-TEST-0_GR-31" RangeStart="0" RangeEnd="65535"><GroupAddress Id="P-TEST-0_GA-321" Address="23011"/></GroupRange></GroupRanges></GroupAddresses></Installation></Installations></Project></KNX>"#;
    let info = br#"<KNX xmlns="http://knx.org/xml/project/23"><Project Id="P-TEST"><ProjectInformation Name="Synthetic override retention"/></Project></KNX>"#;
    for tree in [
        "<GroupObjectTree/>",
        "<GroupObjectTree GroupObjectInstances=\"O-41_R-173\"/>",
    ] {
        let topology = base_topology.replace("<GroupObjectTree/>", tree);
        let archive = knx_testsupport::zip_with_entries(&[
            ("P-TEST.signature", b""),
            ("P-TEST/0.xml", topology.as_bytes()),
            ("P-TEST/project.xml", info),
        ]);
        let dir = tempfile::tempdir().unwrap();
        let native = dir.path().join("synthetic.knxdb");
        let conn = knx_store::open_and_migrate(&native).unwrap();
        let imported = knx_app::import_ets_project_bytes(
            archive,
            "synthetic.knxproj",
            &conn,
            knx_app::ImportOptions::default(),
        )
        .unwrap();
        assert_eq!(imported.report.error_count(), 0);
        assert_eq!(imported.project.devices.iter().count(), 1);
        let expected_objects = usize::from(tree.contains("O-41_R-173"));
        assert_eq!(
            imported.project.devices.com_objects().count(),
            expected_objects
        );

        if expected_objects == 1 {
            let object = imported.project.devices.com_objects().next().unwrap();
            assert_eq!(object.source.ets_id, "O-41_R-173");
            assert!(object.links.is_empty());
            assert!(matches!(object.text, knx_core::Override::Absent));
        }
        let diagnostics: Vec<_> = imported
            .report
            .errors
            .iter()
            .filter(|row| row.detail.contains("UnmappedObjectOverride"))
            .collect();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].severity,
            knx_etsproj::report::Severity::Warning
        );
        assert!(diagnostics[0].detail.contains("O-19_R-271"));
        let opaque = knx_store::load_opaque(&conn).unwrap();
        let retained = opaque
            .iter()
            .find(|entry| entry.xpath.is_empty() && entry.source_path == "P-TEST/0.xml")
            .expect("outside-tree source must have a persisted owner");
        assert_eq!(retained.bytes, topology.as_bytes());
        assert_eq!(
            retained.sha256,
            knx_etsproj::opaque::sha256_hex(topology.as_bytes())
        );
        // Persist/reopen the actual imported model, not a hand-built approximation.
        knx_store::save_project(&conn, &imported.project).unwrap();
        drop(conn);
        let conn = knx_store::open_existing_and_migrate(&native).unwrap();
        let reopened = knx_store::load_project(&conn).unwrap();
        assert_eq!(reopened, imported.project);
        assert_eq!(reopened.devices.com_objects().count(), expected_objects);
        assert_eq!(knx_store::load_opaque(&conn).unwrap(), opaque);
        knx_store::save_project(&conn, &reopened).unwrap();
        assert_eq!(knx_store::load_opaque(&conn).unwrap(), opaque);
        assert_eq!(knx_store::load_project(&conn).unwrap(), reopened);
    }
}
