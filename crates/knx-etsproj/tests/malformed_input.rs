//! The rule under test: bad input produces a named error or a report
//! entry, never a panic, never an unwrap on absent data, and never
//! silently wrong data.

use std::io::{Cursor, Write};

use knx_etsproj::detect::DetectError;
use knx_etsproj::parse::ParseError;
use knx_etsproj::{detect, import_knxproj_bytes, Container, ContainerError, ImportFailure};

const MINIMAL: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation InstallationId="0" Name="" DefaultLine="P-0001-0_L-2" CompletionStatus="Undefined">
        <Topology>
          <Area Id="P-0001-0_A-1" Name="A" Address="1" CompletionStatus="Undefined">
            <Line Id="P-0001-0_L-2" Name="L" Address="1" MediumTypeRefId="MT-0" CompletionStatus="Accepted">
              <DeviceInstance Id="P-0001-0_DI-1" Name="D" ProductRefId="M-0001_H-1_P-1"
                              Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1"
                              LastModified="2023-07-14T11:55:33" CompletionStatus="FinishedDesign"
                              IndividualAddressLoaded="1" ApplicationProgramLoaded="1"
                              ParametersLoaded="1" CommunicationPartLoaded="1"
                              MediumConfigLoaded="1" IsCommunicationObjectVisibilityCalculated="1"
                              Broken="0">
                <ComObjectInstanceRefs>
                  <ComObjectInstanceRef RefId="M-0001_A-1_O-0_R-1" DatapointType="" IsActive="1">
                    <Connectors><Send GroupAddressRefId="P-0001-0_GA-1" /></Connectors>
                  </ComObjectInstanceRef>
                </ComObjectInstanceRefs>
              </DeviceInstance>
            </Line>
          </Area>
        </Topology>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" Name="Licht" RangeStart="1" RangeEnd="255">
              <GroupRange Id="P-0001-0_GR-2" Name="An/Aus" RangeStart="1" RangeEnd="127">
                <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
              </GroupRange>
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

const PROJECT_INFO: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" CompletionStatus="Undefined" />
  </Project>
</KNX>"#;

fn zip_with_entries(entries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in entries {
        writer.start_file(*name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// A `.knxproj`-shaped archive with `installation_xml` as `P-0001/0.xml`,
/// a valid `Project.xml`, and the signature entry `Container::project_part`
/// needs to find the project part at all.
fn knxproj_with_installation(installation_xml: &[u8]) -> Vec<u8> {
    zip_with_entries(&[
        ("P-0001.signature", b"x"),
        ("P-0001/0.xml", installation_xml),
        ("P-0001/Project.xml", PROJECT_INFO),
    ])
}

fn assert_default_line_boundaries(version: u32) {
    use knx_etsproj::map::MapProblemDetail;
    use knx_etsproj::report::{ImportError, Severity};

    let present = Some("P-0001-0_L-3");
    let missing = Some("P-0001-0_L-99");
    let ref_id = if version == 11 {
        "M-0001_A-1_O-7_R-1"
    } else {
        "O-7_R-1"
    };
    for (installation_ref, space_ref) in [
        (None, None),
        (present, present),
        (Some(""), Some("")),
        (missing, missing),
        (present, missing),
        (missing, present),
    ] {
        let imported = import_knxproj_bytes(
            knx_testsupport::mapping_boundary_knxproj_bytes(
                version,
                installation_ref,
                space_ref,
                ref_id,
            ),
            "synthetic-mapping.knxproj",
        )
        .unwrap();
        assert_eq!(imported.report.source.schema_version, version);
        let installation = &imported.project.installations[0];
        assert_eq!(installation.topology.lines.len(), 2);
        let stated_line = installation.topology.lines[1].id;
        assert_ne!(stated_line, installation.topology.lines[0].id);
        assert_eq!(
            installation.default_line,
            (installation_ref == present).then_some(stated_line),
        );
        assert_eq!(installation.buildings.len(), 1);
        let space = &installation.buildings[0];
        assert_eq!(
            space.default_line,
            (space_ref == present).then_some(stated_line)
        );

        let base = "/KNX/Project/Installations/Installation";
        let space_xpath = if version == 11 {
            format!("{base}/Buildings/BuildingPart[@Id='P-0001-0_BP-1']")
        } else {
            format!("{base}/Locations/Space[@Id='P-0001-0_BP-1']")
        };
        let mut expected = Vec::new();
        for (value, kind, xpath) in [
            (installation_ref, "Installation/@DefaultLine", base),
            (space_ref, "BuildingPart/@DefaultLine", space_xpath.as_str()),
        ] {
            if let Some(target) = value.filter(|value| Some(*value) != present) {
                expected.push(ImportError {
                    stage: "map",
                    severity: Severity::Error,
                    xpath: xpath.to_string(),
                    detail: format!(
                        "{:?}",
                        MapProblemDetail::UnresolvedReference {
                            kind,
                            target: target.to_string(),
                        }
                    ),
                });
            }
        }
        // Exact stage/path/detail/count, not a broad "some error" assertion.
        assert_eq!(imported.report.errors, expected);
        assert_eq!(imported.project.devices.iter().count(), 2);
        assert_eq!(imported.project.devices.com_objects().count(), 4);
        assert_eq!(
            space.devices,
            vec![installation.topology.lines[0].devices[0]]
        );
    }
}

#[test]
fn schema11_default_lines_resolve_or_report_without_a_fallback() {
    assert_default_line_boundaries(11);
}

#[test]
fn schema21_default_lines_resolve_or_report_without_a_fallback() {
    assert_default_line_boundaries(21);
}

#[test]
fn schema23_default_lines_resolve_or_report_without_a_fallback() {
    assert_default_line_boundaries(23);
}

#[test]
fn legacy_named_inputs_are_refused_before_modern_archive_parsing() {
    let bytes = knxproj_with_installation(MINIMAL);
    for extension in [
        "vd2", "vd3", "vd4", "vd5", "pr3", "pr4", "pr5", "VD3", "Pr5",
    ] {
        let error = import_knxproj_bytes(bytes.clone(), &format!("synthetic.{extension}"))
            .expect_err("a readable modern archive must not enable a legacy filename extension");
        assert_eq!(error.to_string(), format!(
            "legacy ETS filename extension .{extension} is unsupported; legacy import is not implemented"
        ));
    }
}

#[test]
fn legacy_filename_refusal_does_not_echo_source_bytes_or_classify_other_suffixes() {
    let payload = b"SYNTHETIC-SENSITIVE-PAYLOAD";
    let error = import_knxproj_bytes(payload.to_vec(), "input.PR4").unwrap_err();
    assert!(
        matches!(error, ImportFailure::UnsupportedLegacyFormat { ref extension } if extension == "PR4")
    );
    assert!(!error.to_string().contains("SYNTHETIC-SENSITIVE-PAYLOAD"));
    for name in ["input.pr4.backup", "input.vd30", "input.knxproj"] {
        let error = import_knxproj_bytes(payload.to_vec(), name).unwrap_err();
        assert!(matches!(error, ImportFailure::Container(_)));
    }
    assert!(import_knxproj_bytes(knxproj_with_installation(MINIMAL), "input.KNXPROJ").is_ok());
}

#[test]
fn unsupported_master_root_metadata_is_reported_without_exposing_its_values() {
    let master =
        br#"<KNX xmlns="urn:MASTER_METADATA_SENTINEL/11" CreatedBy="MASTER_METADATA_SENTINEL"/>"#;
    let outcome = import_knxproj_bytes(
        zip_with_entries(&[
            ("P-0001.signature", b"x"),
            ("P-0001/0.xml", MINIMAL),
            ("P-0001/Project.xml", PROJECT_INFO),
            ("knx_master.xml", master),
        ]),
        "unsupported-master.knxproj",
    )
    .expect("unsupported master metadata must not replace the supported project schema");
    assert_eq!(outcome.report.source.schema_version, 11);
    assert_eq!(outcome.report.source.namespace_disagreement, None);
    assert_eq!(
        outcome.report.unsupported,
        vec![knx_etsproj::report::UnsupportedFeature {
            what: "knx_master.xml root metadata".to_string(),
            consequence: "unsupported KNX project namespace; master namespace comparison unavailable; original master bytes retained without interpreting root metadata".to_string(),
        }]
    );
    let retained: Vec<_> = outcome
        .opaque
        .iter()
        .filter(|entry| entry.source_path == "knx_master.xml")
        .collect();
    assert_eq!(retained.len(), 1);
    assert_eq!(
        retained[0].kind,
        knx_etsproj::opaque::OpaqueKind::MasterData
    );
    assert_eq!(retained[0].bytes, master);
    assert_eq!(retained[0].sha256, knx_etsproj::opaque::sha256_hex(master));
    assert!(!outcome
        .report
        .to_json()
        .contains("MASTER_METADATA_SENTINEL"));
}

#[test]
fn unreadable_master_root_boundaries_retain_bytes_and_name_the_failed_comparison() {
    let cases: &[(&[u8], &str)] = &[
        (b"", "missing root namespace"),
        (br#"<KNX CreatedBy="MASTER_METADATA_SENTINEL"/>"#, "missing root namespace"),
        (br#"<MASTER_METADATA_SENTINEL xmlns="http://knx.org/xml/project/11"/>"#, "unexpected XML root"),
        (br#"<KNX xmlns="http://knx.org/xml/project/MASTER_METADATA_SENTINEL"/>"#, "unparsable schema namespace"),
        (br#"<KNX xmlns="http://knx.org/xml/project/11" ToolVersion="&MASTER_METADATA_SENTINEL;"/>"#, "malformed XML root attributes or encoding"),
        (br#"<KNX xmlns="http://knx.org/xml/project/11" ToolVersion="a" ToolVersion="MASTER_METADATA_SENTINEL"/>"#, "malformed XML root attributes or encoding"),
        (b"<KNX xmlns=\"http://knx.org/xml/project/11\" CreatedBy=\"\xff\"/>", "malformed XML root attributes or encoding"),
        (br#"<KNX xmlns="http://knx.org/xml/project/11" xmlns:xml="MASTER_METADATA_SENTINEL"/>"#, "malformed XML root attributes or encoding"),
    ];
    for (master, boundary) in cases {
        let bytes = zip_with_entries(&[
            ("P-0001.signature", b"x"),
            ("P-0001/0.xml", MINIMAL),
            ("P-0001/Project.xml", PROJECT_INFO),
            ("knx_master.xml", master),
        ]);
        let mut container = Container::open(bytes.clone()).unwrap();
        let detected = detect(&mut container).unwrap();
        assert_eq!(detected.namespace_disagreement, None);
        assert!(detected.master_metadata_error.is_some());
        let outcome = import_knxproj_bytes(bytes, "unreadable-master.knxproj").unwrap();
        assert_eq!(outcome.report.source.schema_version, 11);
        assert_eq!(
            outcome.report.unsupported,
            vec![knx_etsproj::report::UnsupportedFeature {
                what: "knx_master.xml root metadata".to_string(),
                consequence: format!("{boundary}; master namespace comparison unavailable; original master bytes retained without interpreting root metadata"),
            }]
        );
        let retained = outcome
            .opaque
            .iter()
            .find(|entry| entry.source_path == "knx_master.xml")
            .unwrap();
        assert_eq!(retained.bytes, *master);
        assert_eq!(retained.sha256, knx_etsproj::opaque::sha256_hex(master));
        assert!(!outcome
            .report
            .to_json()
            .contains("MASTER_METADATA_SENTINEL"));
    }
}

#[test]
fn unreadable_master_container_bytes_are_fatal_not_a_retention_claim() {
    let master = br#"<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="CRC_MASTER_SENTINEL"/>"#;
    let mut bytes = zip_with_entries(&[
        ("P-0001.signature", b"x"),
        ("P-0001/0.xml", MINIMAL),
        ("P-0001/Project.xml", PROJECT_INFO),
        ("knx_master.xml", master),
    ]);
    // Stored ZIP members expose the fixture bytes verbatim. Corrupt only the
    // master payload, leaving its central/local CRC and all other entries intact.
    let positions: Vec<_> = bytes
        .windows(master.len())
        .enumerate()
        .filter_map(|(position, window)| (window == master).then_some(position))
        .collect();
    assert_eq!(positions.len(), 1);
    bytes[positions[0]] = b'!';
    let mut container = Container::open(bytes.clone()).unwrap();
    assert!(matches!(detect(&mut container),
        Err(DetectError::Container(ContainerError::Read { path, .. })) if path == "knx_master.xml"
    ));
    assert!(
        matches!(import_knxproj_bytes(bytes, "corrupt-master.knxproj"),
            Err(ImportFailure::Detect(DetectError::Container(ContainerError::Read { path, .. }))) if path == "knx_master.xml"
        )
    );
}

#[test]
fn absent_agreeing_and_disagreeing_masters_are_not_unreadable_metadata() {
    for (master, disagreement) in [
        (None, None),
        (
            Some(br#"<KNX xmlns="http://knx.org/xml/project/11"/>"#.as_slice()),
            None,
        ),
        (
            Some(br#"<KNX xmlns="http://knx.org/xml/project/21"/>"#.as_slice()),
            Some(21),
        ),
    ] {
        let mut entries = vec![
            ("P-0001.signature", b"x".as_slice()),
            ("P-0001/0.xml", MINIMAL),
            ("P-0001/Project.xml", PROJECT_INFO),
        ];
        if let Some(master) = master {
            entries.push(("knx_master.xml", master));
        }
        let bytes = zip_with_entries(&entries);
        let mut container = Container::open(bytes.clone()).unwrap();
        let detected = detect(&mut container).unwrap();
        assert_eq!(detected.master_metadata_error, None);
        assert_eq!(
            detected.namespace_disagreement.map(|version| version.0),
            disagreement
        );
        let outcome = import_knxproj_bytes(bytes, "optional-master.knxproj").unwrap();
        assert_eq!(outcome.report.unsupported, vec![]);
        assert_eq!(
            outcome.report.source.namespace_disagreement,
            disagreement.map(|version| format!(
                "knx_master.xml declares schema {version}, the project part declares schema 11"
            ))
        );
    }
}

/// `MINIMAL` with its `<DeviceInstance>...</DeviceInstance>` subtree
/// replaced wholesale by a single self-closed element built from
/// `device_open_tag` (no closing `/>` needed in the argument).
fn minimal_xml_with(device_open_tag: &str) -> String {
    let minimal = std::str::from_utf8(MINIMAL).unwrap();
    let start = minimal.find("<DeviceInstance").unwrap();
    let end = minimal.find("</DeviceInstance>").unwrap() + "</DeviceInstance>".len();
    format!(
        "{}{} />{}",
        &minimal[..start],
        device_open_tag,
        &minimal[end..]
    )
}

fn knxproj_with_duplicate_ga_id() -> Vec<u8> {
    let minimal = std::str::from_utf8(MINIMAL).unwrap();
    let doubled = minimal.replace(
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />"#,
        r#"<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
                <GroupAddress Id="P-0001-0_GA-1" Address="2" Name="GA2" />"#,
    );
    knxproj_with_installation(doubled.as_bytes())
}

fn knxproj_with_dangling_link() -> Vec<u8> {
    let minimal = std::str::from_utf8(MINIMAL).unwrap();
    let dangling = minimal.replace(
        r#"GroupAddressRefId="P-0001-0_GA-1""#,
        r#"GroupAddressRefId="P-0001-0_GA-999""#,
    );
    knxproj_with_installation(dangling.as_bytes())
}

/// Writes a tiny, structurally valid archive, then patches its 4-byte
/// little-endian size fields (both the local file header's and the
/// central directory's — `zip`'s writer always sets compressed size equal
/// to uncompressed size for a `Stored` one-byte entry, so this pattern
/// only ever matches those two fields) from the real size (1 byte) to a
/// declared `size` — a zip-bomb-shaped entry with real content nowhere
/// near what it claims to hold.
fn zip_with_declared_size(name: &str, size: u64) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    writer.start_file(name, options).unwrap();
    writer.write_all(b"x").unwrap();
    let mut bytes = writer.finish().unwrap().into_inner();

    let declared = u32::try_from(size)
        .expect("test sizes fit in a 32-bit field")
        .to_le_bytes();
    let real = 1u32.to_le_bytes();
    let mut i = 0;
    while i + 4 <= bytes.len() {
        if bytes[i..i + 4] == real {
            bytes[i..i + 4].copy_from_slice(&declared);
        }
        i += 1;
    }
    bytes
}

#[test]
fn an_empty_file_is_not_a_zip() {
    assert!(matches!(
        import_knxproj_bytes(vec![], "empty.knxproj"),
        Err(ImportFailure::Container(ContainerError::NotAZip(_)))
    ));
}

#[test]
fn a_zip_with_no_project_part_is_named_as_such() {
    let bytes = zip_with_entries(&[("knx_master.xml", b"<KNX/>")]);
    assert!(matches!(
        import_knxproj_bytes(bytes, "no-project.knxproj"),
        Err(ImportFailure::Container(ContainerError::NoProjectPart))
    ));
}

#[test]
fn a_truncated_installation_document_reports_its_position() {
    let bytes = knxproj_with_installation(&MINIMAL[..MINIMAL.len() / 2]);
    match import_knxproj_bytes(bytes, "truncated.knxproj") {
        Err(ImportFailure::Parse(ParseError::Xml { position, .. })) => assert!(position > 0),
        other => panic!("expected a parse error with a position, got {other:?}"),
    }
}

#[test]
fn a_document_with_no_namespace_is_rejected_rather_than_assumed_to_be_schema_eleven() {
    let bytes = knxproj_with_installation(br#"<KNX CreatedBy="ETS4"><Project Id="P-0001"/></KNX>"#);
    assert!(matches!(
        import_knxproj_bytes(bytes, "no-ns.knxproj"),
        Err(ImportFailure::Detect(
            DetectError::NoDefaultNamespace { .. }
        ))
    ));
}

#[test]
fn an_unsupported_schema_version_is_named_and_not_guessed_at() {
    let bytes = knxproj_with_installation(
        br#"<KNX xmlns="http://knx.org/xml/project/14"><Project Id="P-0001"/></KNX>"#,
    );
    assert!(matches!(
        import_knxproj_bytes(bytes, "v14.knxproj"),
        Err(ImportFailure::NoKnownSchemaTable { version: 14 })
    ));
}

#[test]
fn a_foreign_namespace_cannot_borrow_a_supported_project_version() {
    for namespace in [
        "https://example.invalid/project/11",
        "urn:foreign/21",
        "http://knx.org/xml/project/011",
        "http://knx.org/xml/project/+11",
        "http://knx.org/xml/project/23/11",
        "https://knx.org/xml/project/11",
    ] {
        let xml = std::str::from_utf8(MINIMAL)
            .unwrap()
            .replace("http://knx.org/xml/project/11", namespace);
        let result = import_knxproj_bytes(
            knxproj_with_installation(xml.as_bytes()),
            "foreign-namespace.knxproj",
        );
        match result {
            Err(ImportFailure::Detect(DetectError::UnsupportedNamespace {
                entry,
                namespace: rejected,
            })) => {
                assert_eq!(entry, "P-0001/0.xml");
                assert_eq!(rejected, namespace);
            }
            _ => panic!(
                "foreign namespace {namespace:?} reached KNX parsing instead of detection refusal"
            ),
        }
    }
}

#[test]
fn namespace_and_producer_fields_use_xml_values_without_version_guessing() {
    let xml = std::str::from_utf8(MINIMAL)
        .unwrap()
        .replace(
            "http://knx.org/xml/project/11",
            "http://knx.org/xml/project/1&#49;",
        )
        .replace("CreatedBy=\"ETS4\"", "CreatedBy=\"Independent &amp; Tool\"")
        .replace(
            "<KNX ",
            "<KNX xmlns:xml=\"http://www.w3.org/XML/1998/namesp&#97;ce\" \
             xmlns:xmlVendor=\"urn:synthetic:vendor\" ",
        )
        .replace(
            "ToolVersion=\"ETS 4.1.8\"",
            "ToolVersion=\"opaque&#x2B;version\"",
        );
    let imported = import_knxproj_bytes(
        knxproj_with_installation(xml.as_bytes()),
        "xml-values.knxproj",
    )
    .expect("XML character references must be decoded before namespace comparison");
    assert_eq!(
        imported.report.source.namespace,
        "http://knx.org/xml/project/11"
    );
    assert_eq!(
        imported.report.source.created_by.as_deref(),
        Some("Independent & Tool")
    );
    assert_eq!(
        imported.report.source.tool_version.as_deref(),
        Some("opaque+version")
    );
    assert_eq!(imported.report.source.schema_version, 11);
}

#[test]
fn an_invalid_individual_address_is_reported_and_the_device_still_imports() {
    let xml = minimal_xml_with(r#"<DeviceInstance Id="P-0001-0_DI-1" Name="D" Address="999""#);
    let out = import_knxproj_bytes(
        knxproj_with_installation(xml.as_bytes()),
        "bad-addr.knxproj",
    )
    .unwrap();
    assert!(out
        .report
        .errors
        .iter()
        .any(|e| e.detail.contains("Address")));
    assert_eq!(out.project.devices.iter().count(), 1);
    assert!(out.project.devices.iter().next().unwrap().address.is_none());
}

#[test]
fn unreadable_root_attributes_cannot_be_silently_erased_by_detection() {
    let mut invalid_utf8 = br#"<KNX xmlns="http://knx.org/xml/project/11" CreatedBy=""#.to_vec();
    invalid_utf8.push(0xff);
    invalid_utf8.extend_from_slice(br#""/>"#);
    for xml in [
        br#"<KNX xmlns="http://knx.org/xml/project/11" ToolVersion="a" ToolVersion="b"/>"#.to_vec(),
        br#"<KNX xmlns="http://knx.org/xml/project/11" Future=unquoted/>"#.to_vec(),
        invalid_utf8,
    ] {
        let mut container = Container::open(knxproj_with_installation(&xml)).unwrap();
        assert!(
            detect(&mut container).is_err(),
            "a malformed root is not a successful detection with absent/replacement attributes"
        );
    }
}

#[test]
fn a_default_namespace_does_not_override_the_actual_root_identity() {
    for (root, declaration) in [
        ("NotKNX", ""),
        ("foreign:KNX", " xmlns:foreign=\"urn:foreign\""),
        ("unbound:KNX", ""),
    ] {
        let xml = std::str::from_utf8(MINIMAL)
            .unwrap()
            .replace("<KNX ", &format!("<{root}{declaration} "))
            .replace("</KNX>", &format!("</{root}>"));
        assert!(
            matches!(
                import_knxproj_bytes(
                    knxproj_with_installation(xml.as_bytes()),
                    "wrong-root.knxproj"
                ),
                Err(ImportFailure::Detect(_))
            ),
            "root {root:?} must be refused before applying a KNX known table"
        );
    }
}

#[test]
fn reserved_namespace_bindings_cannot_impersonate_knx() {
    for declaration in [
        "xmlns:xml=\"http://knx.org/xml/project/11\"",
        "xmlns:xmlns=\"http://knx.org/xml/project/11\"",
        "xmlns:other=\"http://www.w3.org/XML/1998/namespace\"",
        "xmlns:other=\"http://www.w3.org/2000/xmlns/\"",
        "xmlns:other=\"http://www.w3.org/XML/1998/namesp&#97;ce\"",
        "xmlns:=\"http://knx.org/xml/project/11\"",
        "xmlns:other=\"\"",
    ] {
        let xml = format!("<KNX xmlns=\"http://knx.org/xml/project/11\" {declaration}/>");
        let mut container = Container::open(knxproj_with_installation(xml.as_bytes())).unwrap();
        assert!(
            matches!(
                detect(&mut container),
                Err(DetectError::MalformedRoot { .. })
            ),
            "reserved namespace misuse must not be accepted as a valid KNX root"
        );
    }
}

#[test]
fn a_bound_knx_prefix_is_not_mistaken_for_a_missing_namespace() {
    // Every element is qualified; no default namespace exists.
    let mut xml = std::str::from_utf8(MINIMAL).unwrap().to_string();
    xml = xml.replace("xmlns=", "xmlns:knx=");
    for element in [
        "KNX",
        "Project",
        "Installations",
        "Installation",
        "Topology",
        "Area",
        "Line",
        "DeviceInstance",
        "ComObjectInstanceRefs",
        "ComObjectInstanceRef",
        "Connectors",
        "Send",
        "GroupAddresses",
        "GroupRanges",
        "GroupRange",
        "GroupAddress",
    ] {
        xml = xml
            .replace(&format!("<{element} "), &format!("<knx:{element} "))
            .replace(&format!("<{element}>"), &format!("<knx:{element}>"))
            .replace(&format!("</{element}>"), &format!("</knx:{element}>"));
    }
    let imported = import_knxproj_bytes(
        knxproj_with_installation(xml.as_bytes()),
        "prefixed.knxproj",
    )
    .expect("the root QName is bound to the supported KNX project namespace");
    assert_eq!(imported.project.devices.iter().count(), 1);
    assert_eq!(imported.project.installations[0].group_addresses.len(), 1);
}

#[test]
fn a_duplicate_group_address_id_is_reported_and_both_entries_survive() {
    let out = import_knxproj_bytes(knxproj_with_duplicate_ga_id(), "dupe.knxproj").unwrap();
    // `ProblemDetail::DuplicateId`'s Debug output capitalizes "Duplicate"
    // (it is the variant name); the plan's own draft checked lowercase
    // "duplicate", which no `Debug`-formatted `ProblemDetail` ever
    // produces — corrected here and in the plan document to match reality.
    assert!(out
        .report
        .errors
        .iter()
        .any(|e| e.detail.contains("Duplicate")));
    assert_eq!(out.project.installations[0].group_addresses.len(), 2);
}

/// Surviving in memory is not enough: both entries shared one internal id,
/// so the native save kept only the last one. Each element now has its own
/// id, and the device's `Send` to the repeated ETS id is reported as
/// ambiguous instead of being linked to one of the two by guesswork.
#[test]
fn a_duplicate_group_address_id_keeps_two_internal_ids_and_does_not_guess_links() {
    use knx_etsproj::map::MapProblemDetail;
    let out = import_knxproj_bytes(knxproj_with_duplicate_ga_id(), "dupe.knxproj").unwrap();
    let entries = &out.project.installations[0].group_addresses;
    assert_eq!(entries.len(), 2);
    assert_ne!(entries[0].id, entries[1].id, "one id per element");
    assert_eq!(entries[0].source.ets_id, entries[1].source.ets_id);
    let links: Vec<_> = out
        .project
        .devices
        .com_objects()
        .flat_map(|com| com.links.iter())
        .collect();
    assert!(links.is_empty(), "the link target is ambiguous: {links:?}");
    assert!(out.report.errors.iter().any(|e| e.stage == "map"
        && e.detail
            == format!(
                "{:?}",
                MapProblemDetail::AmbiguousReference {
                    kind: "GroupAddressRef",
                    target: "P-0001-0_GA-1".into(),
                }
            )));
}

#[test]
fn project_metadata_must_have_the_same_knx_root_namespace_as_topology() {
    let original = std::str::from_utf8(PROJECT_INFO).unwrap();
    for info in [
        original.replace("http://knx.org/xml/project/11", "urn:foreign/11"),
        original.replace(
            "http://knx.org/xml/project/11",
            "http://knx.org/xml/project/23",
        ),
        original
            .replace("<KNX ", "<NotKNX ")
            .replace("</KNX>", "</NotKNX>"),
    ] {
        let bytes = zip_with_entries(&[
            ("P-0001.signature", b"x"),
            ("P-0001/0.xml", MINIMAL),
            ("P-0001/Project.xml", info.as_bytes()),
        ]);
        assert!(
            matches!(
                import_knxproj_bytes(bytes, "wrong-metadata-namespace.knxproj"),
                Err(ImportFailure::Detect(_))
            ),
            "metadata with a different root identity must not use topology's known table"
        );
    }
}

#[test]
fn a_dangling_group_address_reference_drops_the_link_and_reports_it() {
    let out = import_knxproj_bytes(knxproj_with_dangling_link(), "dangling.knxproj").unwrap();
    assert!(out
        .report
        .errors
        .iter()
        .any(|e| e.detail.contains("GroupAddressRefId")));
    let coms: Vec<_> = out
        .project
        .devices
        .iter()
        .flat_map(|d| d.com_objects.clone())
        .filter_map(|id| out.project.devices.com_object(id))
        .collect();
    assert!(coms.iter().all(|c| c.links.is_empty()));
}

#[test]
fn a_zip_bomb_shaped_entry_does_not_exhaust_memory() {
    // A single entry declaring a huge uncompressed size must be refused before
    // it is read into a Vec, not after. (500 MB, not the 4 GB a literal
    // reading of the plan would suggest: 4 GiB is one past `u32::MAX`, the
    // width of a plain—non-ZIP64—size field this test patches by hand; 500
    // MB is already far past the 64 MB limit the guard enforces, so it
    // exercises exactly the same guard without hitting that unrelated
    // 32-bit boundary.)
    let bytes = zip_with_declared_size("P-0001/0.xml", 500 * 1024 * 1024);
    assert!(import_knxproj_bytes(bytes, "bomb.knxproj").is_err());
}

#[test]
fn a_deeply_nested_document_does_not_overflow_the_stack() {
    // The parser is iterative with an explicit stack; 10000 levels must be an
    // error, not a crash.
    let mut xml = String::from(r#"<KNX xmlns="http://knx.org/xml/project/11">"#);
    for _ in 0..10_000 {
        xml.push_str("<GroupRange>");
    }
    let bytes = knxproj_with_installation(xml.as_bytes());
    assert!(import_knxproj_bytes(bytes, "deep.knxproj").is_err());
}
