//! Complete synthetic configuration and opaque-media persistence witnesses.
use knx_testsupport::zip_with_entries;

#[test]
fn modern_node_and_root_styles_keep_unassigned_configuration_across_resave() {
    for version in [21, 23] {
        for tree in [
            r#"<GroupObjectTree GroupObjectInstances="O-7_R-1 O-7_R-1"/>"#,
            r#"<GroupObjectTree><Nodes><Node Type="Channel" RefId="CH-1"><Nodes><Node Type="Folder" GroupObjectInstances="O-7_R-1 O-7_R-1"/></Nodes></Node></Nodes></GroupObjectTree>"#,
        ] {
            // Schema21's documented declaration is node-based; root-attribute
            // declaration is tested only under Schema23.
            if version == 21 && tree.starts_with("<GroupObjectTree GroupObjectInstances") {
                continue;
            }
            let dir = tempfile::tempdir().unwrap();
            let native = dir.path().join("native.knxdb");
            let input = dir.path().join("synthetic.knxproj");
            let topology = format!(
                r#"<KNX xmlns="http://knx.org/xml/project/{version}"><Project Id="P-0001"><Installations><Installation InstallationId="0"><Topology><UnassignedDevices><DeviceInstance Id="P-0001-0_DI-1" ProductRefId="M-0001_P-1" Hardware2ProgramRefId="M-0001_H-1_HP-1" CompletionStatus="Accepted" InstallationHints="synthetic hint"><ParameterInstanceRefs><ParameterInstanceRef RefId="P-1_R-1" Value="37"/></ParameterInstanceRefs><ModuleInstances><ModuleInstance Id="MI-1" RefId="MD-1" RepeatIndex="1"><Arguments><Argument RefId="A-1" Value="13"/></Arguments></ModuleInstance></ModuleInstances><ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-7_R-1" Links="GA-1" ReadFlag="Enabled" DatapointType="DPST-1-1" Text="synthetic text"/></ComObjectInstanceRefs>{tree}</DeviceInstance></UnassignedDevices></Topology><Locations><Space Id="P-0001-0_BP-1" Name="Synthetic" Type="Room"><DeviceInstanceRef RefId="P-0001-0_DI-1"/></Space></Locations><GroupAddresses><GroupRanges><GroupRange Id="P-0001-0_GR-1" RangeStart="0" RangeEnd="65535"><GroupAddress Id="P-0001-0_GA-1" Address="1"/></GroupRange></GroupRanges></GroupAddresses></Installation></Installations></Project></KNX>"#
            );
            let info = format!(
                r#"<KNX xmlns="http://knx.org/xml/project/{version}"><Project Id="P-0001"><ProjectInformation Name="Synthetic" CompletionStatus="Accepted" SameName="metadata"><ToDoItems><ToDoItem Description="synthetic-marker"/></ToDoItems></ProjectInformation></Project></KNX>"#
            );
            std::fs::write(
                &input,
                zip_with_entries(&[
                    ("P-0001.signature", b""),
                    ("P-0001/0.xml", topology.as_bytes()),
                    ("P-0001/project.xml", info.as_bytes()),
                ]),
            )
            .unwrap();
            let conn = knx_store::open_and_migrate(&native).unwrap();
            let imported = knx_app::import_ets_project(&input, &conn).unwrap();
            assert_eq!(imported.report.error_count(), 0);
            let p = &imported.project;
            let d = p.devices.iter().next().unwrap();
            assert_eq!(p.devices.iter().count(), 1);
            assert_eq!(d.address, None);
            assert_eq!(p.installations[0].topology.unassigned, vec![d.id]);
            assert_eq!(p.installations[0].parameters.len(), 1);
            assert_eq!(p.devices.module_instances().count(), 1);
            let m = p.devices.module_instances().next().unwrap();
            assert_eq!(m.instance_ets_id, "MI-1");
            assert_eq!(m.repeat_index, "1");
            assert_eq!(m.arguments[0].1, "13");
            let o = p.devices.com_objects().next().unwrap();
            assert_eq!(p.devices.com_objects().count(), 1);
            assert_eq!(o.links.len(), 1);
            assert!(!matches!(o.dpt, knx_core::Override::Absent));
            assert!(!matches!(o.flags.read, knx_core::Override::Absent));
            assert!(!matches!(o.text, knx_core::Override::Absent));
            let observations = imported.report.source_observations.as_ref().unwrap();
            assert_eq!(observations.topology["ParameterInstanceRef"], 1);
            assert_eq!(observations.topology["ModuleInstance"], 1);
            let opaque = knx_store::load_opaque(&conn).unwrap();
            assert!(opaque.iter().any(|e| e.source_path == "P-0001/project.xml"
                && e.name == "SameName"
                && e.bytes == b"metadata"));
            assert!(opaque.iter().any(|e| e.source_path == "P-0001/project.xml"
                && e.xpath.is_empty()
                && e.bytes == info.as_bytes()));
            assert!(opaque.iter().any(|e| e.source_path == "P-0001/0.xml"
                && e.xpath.is_empty()
                && e.bytes == topology.as_bytes()));
            knx_store::save_project(&conn, p).unwrap();
            drop(conn);
            let conn = knx_store::open_existing_and_migrate(&native).unwrap();
            assert_eq!(knx_store::load_project(&conn).unwrap(), *p);
            knx_store::save_project(&conn, p).unwrap();
            assert_eq!(knx_store::load_opaque(&conn).unwrap(), opaque);
            assert_eq!(knx_store::load_project(&conn).unwrap(), *p);
        }
    }
}

#[test]
fn baggage_magic_descriptions_and_original_payloads_survive_without_execution() {
    for (payload, class) in [
        (&b"\x89PNG\r\n\x1a\nsynthetic"[..], "png"),
        (&b"\xff\xd8\xffsynthetic"[..], "jpeg"),
        (
            &b"BM\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"[..],
            "bmp",
        ),
        (&b"%PDF-1.4 synthetic"[..], "pdf"),
        (&b"PK\x03\x04synthetic"[..], "zip"),
        (&b"synthetic opaque bytes"[..], "unknown"),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let mut template =
            knx_etsproj::Container::open(knx_testsupport::minimal_knxproj_bytes()).unwrap();
        let topology = template.read("P-0001/0.xml").unwrap();
        let info = template.read("P-0001/Project.xml").unwrap();
        let bytes = zip_with_entries(&[
            ("P-0001.signature", b""),
            ("P-0001/0.xml", &topology),
            ("P-0001/Project.xml", &info),
            ("M-0001/Baggages/synthetic.dat", payload),
        ]);
        let conn = knx_store::open_and_migrate(&dir.path().join("native.knxdb")).unwrap();
        let result = knx_app::import_ets_project_bytes(
            bytes,
            "synthetic.knxproj",
            &conn,
            knx_app::ImportOptions::default(),
        )
        .unwrap();
        let summary = result
            .report
            .opaque
            .iter()
            .find(|e| e.kind == "Baggage")
            .unwrap();
        assert!(
            summary.reason.contains(class),
            "{class}: {}",
            summary.reason
        );
        assert!(!summary.reason.contains("plugin binary"));
        assert!(knx_store::load_opaque(&conn)
            .unwrap()
            .iter()
            .any(|e| e.source_path.ends_with("synthetic.dat") && e.bytes == payload));
    }
}
