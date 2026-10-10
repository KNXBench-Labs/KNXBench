//! Synthetic import-integrity witnesses; no private project data.
use knx_etsproj::import_knxproj_bytes;
use std::io::{Cursor, Write};
use zip::write::SimpleFileOptions;

fn fixture(version: u32, topology: &str, metadata: &str) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let wrap = |s: &str| {
        format!("<KNX xmlns=\"http://knx.org/xml/project/{version}\"><Project Id=\"P-0001\">{s}</Project></KNX>")
    };
    for (name, bytes) in [
        ("P-0001.signature", Vec::new()),
        ("P-0001/0.xml", wrap(&format!("<Installations><Installation InstallationId=\"0\">{topology}</Installation></Installations>")).into_bytes()),
        ("P-0001/project.xml", wrap(metadata).into_bytes()),
    ] {
        zip.start_file(name, SimpleFileOptions::default()).unwrap();
        zip.write_all(&bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn device(tree: &str) -> String {
    format!(
        r#"<DeviceInstance Id="P-0001-0_DI-1" ProductRefId="M-0001_P-1" Hardware2ProgramRefId="M-0001_H-1_HP-1"><ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-7_R-1" Links="GA-1" ReadFlag="Enabled"/></ComObjectInstanceRefs>{tree}</DeviceInstance>"#
    )
}
fn topology(dev: &str) -> String {
    format!(
        r#"<Topology><Area Id="P-0001-0_A-1" Address="1"><Line Id="P-0001-0_L-1" Address="1"><Segment Id="P-0001-0_S-1" Number="1">{dev}</Segment></Line></Area></Topology><GroupAddresses><GroupRanges><GroupRange Id="P-0001-0_GR-1" RangeStart="0" RangeEnd="65535"><GroupAddress Id="P-0001-0_GA-1" Address="1"/></GroupRange></GroupRanges></GroupAddresses>"#
    )
}
#[test]
fn conflicting_object_overrides_are_not_resolved_by_last_winner() {
    let tree = r#"<GroupObjectTree GroupObjectInstances="O-7_R-1"/>"#;
    let dev = device(tree).replace(
        "</ComObjectInstanceRefs>",
        r#"<ComObjectInstanceRef RefId="O-7_R-1" ReadFlag="Disabled"/></ComObjectInstanceRefs>"#,
    );
    let result = import_knxproj_bytes(
        fixture(23, &topology(&dev), "<ProjectInformation/>"),
        "synthetic.knxproj",
    )
    .unwrap();
    assert!(result
        .report
        .errors
        .iter()
        .any(|e| e.detail.contains("AmbiguousObjectOverride")
            && e.severity == knx_etsproj::report::Severity::Error));
    assert!(matches!(
        result
            .project
            .devices
            .com_objects()
            .next()
            .unwrap()
            .flags
            .read,
        knx_core::Override::Absent
    ));
}

#[test]
fn extension_nodes_cannot_impersonate_project_object_declarations() {
    for tree in [
        r#"<GroupObjectTree><Node xmlns="urn:synthetic-extension" GroupObjectInstances="O-7_R-1"/></GroupObjectTree>"#,
        r#"<GroupObjectTree><xml:Node xmlns:xml="http://knx.org/xml/project/23" GroupObjectInstances="O-7_R-1"/></GroupObjectTree>"#,
    ] {
        let result = import_knxproj_bytes(
            fixture(23, &topology(&device(tree)), "<ProjectInformation/>"),
            "synthetic.knxproj",
        )
        .unwrap();
        assert_eq!(result.project.devices.com_objects().count(), 0);
        assert!(result.report.unknown.iter().any(|u| u.name == "Node"));
    }
    let tree = r#"<GroupObjectTree><Node xmlns="urn:synthetic-extension"/><k:Node xmlns:k="http://knx.org/xml/project/23" Type="Channel" RefId="CH-1" GroupObjectInstances="O-7_R-1"/></GroupObjectTree>"#;
    let result = import_knxproj_bytes(
        fixture(23, &topology(&device(tree)), "<ProjectInformation/>"),
        "synthetic.knxproj",
    )
    .unwrap();
    assert_eq!(result.project.devices.com_objects().count(), 1);
}

#[test]
fn schema23_reads_nested_nodes_and_root_objects_in_encounter_order() {
    for tree in [
        r#"<GroupObjectTree><Nodes><Node Type="Channel" RefId="CH-1" GroupObjectInstances="O-7_R-1"><Nodes><Node Type="Folder" GroupObjectInstances="O-9_R-1 O-7_R-1"/></Nodes></Node></Nodes></GroupObjectTree>"#,
        r#"<GroupObjectTree GroupObjectInstances="O-7_R-1 O-7_R-1"><Node Type="Folder" GroupObjectInstances="O-9_R-1"/></GroupObjectTree>"#,
    ] {
        let result = import_knxproj_bytes(
            fixture(23, &topology(&device(tree)), "<ProjectInformation/>"),
            "synthetic.knxproj",
        )
        .unwrap();
        assert!(
            result.report.errors.is_empty(),
            "{:?}",
            result.report.errors
        );
        let objects: Vec<_> = result.project.devices.com_objects().collect();
        assert_eq!(
            objects
                .iter()
                .map(|o| o.source.ets_id.as_str())
                .collect::<Vec<_>>(),
            ["O-7_R-1", "O-9_R-1"]
        );
        assert_eq!(objects[0].links.len(), 1);
        assert!(!result
            .report
            .unknown
            .iter()
            .any(|u| matches!(u.name.as_str(), "Node" | "Nodes")));
    }
}
#[test]
fn modern_unassigned_devices_keep_configuration_and_location_refs() {
    for version in [21, 23] {
        let dev = device(
            r#"<GroupObjectTree><Nodes><Node Type="Channel" RefId="CH-1" GroupObjectInstances="O-7_R-1"/></Nodes></GroupObjectTree>"#,
        );
        let input = format!(
            r#"<Topology><UnassignedDevices>{dev}</UnassignedDevices></Topology><Locations><Space Id="P-0001-0_BP-1" Type="Room" Name="Synthetic"><DeviceInstanceRef RefId="P-0001-0_DI-1"/></Space></Locations><GroupAddresses><GroupRanges><GroupRange Id="P-0001-0_GR-1" RangeStart="0" RangeEnd="65535"><GroupAddress Id="P-0001-0_GA-1" Address="1"/></GroupRange></GroupRanges></GroupAddresses>"#
        );
        let result = import_knxproj_bytes(
            fixture(version, &input, "<ProjectInformation/>"),
            "synthetic.knxproj",
        )
        .unwrap();
        assert_eq!(result.project.devices.iter().count(), 1);
        let d = result.project.devices.iter().next().unwrap();
        assert_eq!(d.address, None);
        assert_eq!(result.project.installations[0].topology.unassigned, [d.id]);
        assert!(
            result.report.errors.is_empty(),
            "{:?}",
            result.report.errors
        );
    }
}
#[test]
fn foreign_metadata_cannot_impersonate_known_project_information_or_traces() {
    let metadata = r#"<ProjectInformation Name="Synthetic" CompletionStatus="Editing"><ProjectTraces><ProjectTrace Comment="synthetic audit"/></ProjectTraces><x:ProjectTraces xmlns:x="urn:synthetic-metadata"><x:ProjectTrace Comment="foreign direct audit"/></x:ProjectTraces></ProjectInformation><x:ProjectInformation xmlns:x="urn:synthetic-metadata" Name="foreign-metadata-marker" CompletionStatus="Locked"><x:ProjectTraces><x:ProjectTrace Comment="foreign audit"/></x:ProjectTraces></x:ProjectInformation>"#;
    let result =
        import_knxproj_bytes(fixture(23, "<Topology/>", metadata), "synthetic.knxproj").unwrap();
    assert_eq!(result.project.info.name, "Synthetic");
    assert_eq!(
        result.project.info.completion,
        knx_core::CompletionStatus::Editing
    );
    assert!(result
        .report
        .unknown
        .iter()
        .any(|u| u.name == "ProjectInformation" && u.kind == knx_etsproj::UnknownKind::Element));
    assert!(result.opaque.iter().any(|e| e.name == "ProjectInformation"
        && String::from_utf8_lossy(&e.bytes).contains("foreign-metadata-marker")));
    let traces: Vec<_> = result
        .opaque
        .iter()
        .filter(|e| e.name == "ProjectTraces")
        .collect();
    assert_eq!(traces.len(), 2);
    assert!(traces
        .iter()
        .any(|e| String::from_utf8_lossy(&e.bytes).contains("synthetic audit")));
    assert!(traces
        .iter()
        .any(|e| String::from_utf8_lossy(&e.bytes).contains("foreign direct audit")));
    assert!(result
        .report
        .unknown
        .iter()
        .any(|u| u.name == "ProjectTraces" && u.kind == knx_etsproj::UnknownKind::Element));
}

#[test]
fn metadata_unknown_subtrees_and_original_documents_are_retained() {
    let metadata = r#"<ProjectInformation><ToDoItems><ToDoItem Description="synthetic-marker"/></ToDoItems></ProjectInformation><AddinData><Extension xmlns="urn:synthetic"/></AddinData>"#;
    let result =
        import_knxproj_bytes(fixture(23, "<Topology/>", metadata), "synthetic.knxproj").unwrap();
    for name in ["ToDoItems", "AddinData"] {
        assert!(result
            .opaque
            .iter()
            .any(|e| e.name == name && e.source_path.eq_ignore_ascii_case("P-0001/project.xml")));
    }
    for path in ["P-0001/0.xml", "P-0001/project.xml"] {
        assert!(result
            .opaque
            .iter()
            .any(|e| e.xpath.is_empty() && e.source_path.eq_ignore_ascii_case(path)));
    }
}
#[test]
fn source_observations_include_elements_inside_retained_subtrees() {
    let input = "<Topology><FutureWrapper><DeviceInstance Id=\"synthetic-uninterpreted\"/></FutureWrapper></Topology>";
    let result = import_knxproj_bytes(
        fixture(23, input, "<ProjectInformation/>"),
        "synthetic.knxproj",
    )
    .unwrap();
    assert_eq!(result.project.devices.iter().count(), 0);
    let json: serde_json::Value = serde_json::from_str(&result.report.to_json()).unwrap();
    assert_eq!(json["source_observations"]["topology"]["DeviceInstance"], 1);
    assert_eq!(
        json["source_observations"]["meaning"],
        "xml-element-occurrences-not-semantic-acceptance"
    );
}

#[test]
fn orphan_object_overrides_are_reported_not_silently_lost() {
    let result = import_knxproj_bytes(
        fixture(
            23,
            &topology(&device("<GroupObjectTree/>")),
            "<ProjectInformation/>",
        ),
        "synthetic.knxproj",
    )
    .unwrap();
    assert!(result
        .report
        .errors
        .iter()
        .any(|e| e.detail.contains("UnmappedObjectOverride")));
    assert!(result
        .opaque
        .iter()
        .any(|e| e.xpath.is_empty() && e.source_path == "P-0001/0.xml"));
}

#[test]
fn documented_group_description_is_retained_without_being_misclassified() {
    let input = topology("").replace(
        "Address=\"1\"/>",
        "Address=\"1\" Description=\"synthetic description\"/>",
    );
    let result = import_knxproj_bytes(
        fixture(23, &input, "<ProjectInformation/>"),
        "synthetic.knxproj",
    )
    .unwrap();
    assert!(!result
        .report
        .unknown
        .iter()
        .any(|u| u.name == "Description"));
    assert!(result
        .opaque
        .iter()
        .any(|e| e.name == "Description" && e.bytes == b"synthetic description"));
}

#[test]
fn completion_presence_is_not_confused_with_unknown_or_empty_values() {
    for (attribute, expected_error) in [
        ("", false),
        (" CompletionStatus=\"\"", true),
        (" CompletionStatus=\"synthetic-future-status\"", true),
        (" CompletionStatus=\"Locked\"", false),
    ] {
        let metadata = format!("<ProjectInformation{attribute}/>");
        let result =
            import_knxproj_bytes(fixture(23, "<Topology/>", &metadata), "synthetic.knxproj")
                .unwrap();
        assert_eq!(result.report.error_count() > 0, expected_error);
        assert!(result
            .opaque
            .iter()
            .any(|e| e.source_path == "P-0001/project.xml"
                && e.xpath.is_empty()
                && String::from_utf8_lossy(&e.bytes).contains(&metadata)));
        if expected_error {
            assert!(result.opaque.iter().any(|e| e.name == "CompletionStatus"));
        }
        if attribute.contains("Locked") {
            assert_eq!(
                result.project.info.completion,
                knx_core::CompletionStatus::Locked
            );
        }
    }
}

#[test]
fn modern_completion_status_is_not_replaced_by_a_default() {
    let dev = device("<GroupObjectTree/>").replace(
        "<DeviceInstance ",
        "<DeviceInstance CompletionStatus=\"Accepted\" InstallationHints=\"synthetic hint\" ",
    );
    let result = import_knxproj_bytes(
        fixture(
            23,
            &topology(&dev),
            "<ProjectInformation CompletionStatus=\"Tested\"/>",
        ),
        "synthetic.knxproj",
    )
    .unwrap();
    assert_eq!(
        result
            .project
            .devices
            .iter()
            .next()
            .unwrap()
            .commissioning
            .completion,
        knx_core::CompletionStatus::Accepted
    );
    assert_eq!(format!("{:?}", result.project.info.completion), "Tested");
    assert!(!result
        .report
        .errors
        .iter()
        .any(|e| e.detail.contains("Tested")));
    assert!(!result
        .report
        .unknown
        .iter()
        .any(|u| matches!(u.name.as_str(), "CompletionStatus" | "InstallationHints")));
}

#[test]
fn metadata_attributes_have_their_actual_source_file() {
    let result = import_knxproj_bytes(
        fixture(
            23,
            "<Topology/>",
            r#"<ProjectInformation Comment="synthetic-comment" Extra="synthetic-extra"/>"#,
        ),
        "synthetic.knxproj",
    )
    .unwrap();
    for name in ["Comment", "Extra"] {
        let e = result.opaque.iter().find(|e| e.name == name).unwrap();
        assert_eq!(e.source_path, "P-0001/project.xml");
    }
}
