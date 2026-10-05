//! AR10 / KNOWN_LIMITATIONS §14: an imported project has no source-backed
//! language, and nothing pretends it does.
//!
//! *Project Schema23* `ProjectInformation` declares no language attribute;
//! the only language-bearing project attribute is the optional, per-device
//! `DeviceInstance/@InitialValueLanguage` ("the product language in which
//! the initial values have been set upon device creation"), absent from all
//! three local reference projects. These tests pin the boundary: that
//! attribute is reported and retained, not promoted to a project language,
//! and the project's string table stays empty, so the placeholder default
//! language is never consulted by any resolution.

use std::io::{Cursor, Write};

use knx_etsproj::import_knxproj_bytes;

const PROJECT_INFO_23: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" />
  </Project>
</KNX>"#;

const INSTALLATION_23: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23" CreatedBy="ETS6" ToolVersion="ETS 6.3.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="" DefaultLine="P-0001-0_L-2">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1">
            <Line Id="P-0001-0_L-2" Address="1">
              <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0">
                <DeviceInstance Id="P-0001-0_DI-1" Name="Actuator" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1"
                                InitialValueLanguage="de-DE" />
              </Segment>
            </Line>
          </Area>
        </Topology>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

fn knxproj() -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("P-0001.signature", b"x".as_slice()),
        ("P-0001/0.xml", INSTALLATION_23),
        ("P-0001/Project.xml", PROJECT_INFO_23),
    ] {
        writer.start_file(name, options).expect("zip entry");
        writer.write_all(bytes).expect("zip entry body");
    }
    writer.finish().expect("zip central directory").into_inner()
}

#[test]
fn a_device_initial_value_language_is_reported_not_promoted_to_the_project() {
    let imported = import_knxproj_bytes(knxproj(), "language.knxproj").unwrap();
    // Reported (and therefore retained as evidence), not silently dropped.
    assert!(
        imported
            .report
            .unknown
            .iter()
            .any(|u| u.xpath.ends_with("/DeviceInstance") && u.name == "InitialValueLanguage"),
        "{:?}",
        imported.report.unknown
    );
    // Not promoted: the project default stays the documented placeholder,
    // never the one device's value.
    assert_eq!(imported.project.strings.default_language().0, "en");
}

#[test]
fn an_imported_project_has_no_string_table_entry_for_the_placeholder_to_answer() {
    let imported = import_knxproj_bytes(knxproj(), "language.knxproj").unwrap();
    assert_eq!(imported.project.strings.iter().count(), 0);
}
