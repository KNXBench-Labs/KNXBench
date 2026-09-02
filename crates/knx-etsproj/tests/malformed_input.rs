//! The rule under test: bad input produces a named error or a report
//! entry, never a panic, never an unwrap on absent data, and never
//! silently wrong data.

use std::io::{Cursor, Write};

use knx_etsproj::detect::DetectError;
use knx_etsproj::parse::ParseError;
use knx_etsproj::{import_knxproj_bytes, ContainerError, ImportFailure};

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
