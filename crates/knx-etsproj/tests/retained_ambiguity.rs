//! An ambiguous retained attribute is dropped and warned about, never guessed at (§34).

use std::io::{Cursor, Write};

use knx_etsproj::export::{export_knxproj, ExportWarning};
use knx_etsproj::opaque::OpaqueEntry;
use knx_etsproj::{import_knxproj_bytes, Container};

const PROJECT_INFO_21: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" />
  </Project>
</KNX>"#;

/// One line with *two* `Segment`s, each carrying its own `Puid`. A segment
/// has no identity in the domain model — `knx_core` has lines, not segments
/// — so both retained `Puid`s key on the same line, and which one belongs
/// to which segment is no longer recoverable. This is the shape the rule
/// exists for; the ETS files measured so far carry one segment per line.
const TWO_SEGMENTS: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21" CreatedBy="ETS6" ToolVersion="ETS 6.1.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="T" DefaultLine="P-0001-0_L-2">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1">
            <Line Id="P-0001-0_L-2" Address="1" Puid="11">
              <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0" Puid="101">
                <DeviceInstance Id="P-0001-0_DI-1" Name="D1" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1" Puid="201" />
              </Segment>
              <Segment Id="P-0001-0_L-2_S-2" Number="1" MediumTypeRefId="MT-0" Puid="102">
                <DeviceInstance Id="P-0001-0_DI-2" Name="D2" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="2" Puid="202" />
              </Segment>
            </Line>
          </Area>
        </Topology>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

fn knxproj(installation_xml: &[u8]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("P-0001.signature", b"x" as &[u8]),
        ("P-0001/0.xml", installation_xml),
        ("P-0001/Project.xml", PROJECT_INFO_21),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn round_trip(installation_xml: &[u8]) -> (String, Vec<ExportWarning>) {
    let outcome = import_knxproj_bytes(knxproj(installation_xml), "ambiguity.knxproj").unwrap();
    let entries: Vec<OpaqueEntry> = outcome.opaque.to_vec();
    let exported = export_knxproj(&outcome.project, &entries).unwrap();
    let mut written = Container::open(exported.bytes).unwrap();
    let part = written.project_part().unwrap().to_string();
    let xml = String::from_utf8(written.read(&format!("{part}/0.xml")).unwrap()).unwrap();
    (xml, exported.warnings)
}

/// The ambiguous value is dropped, not guessed at, and the drop is
/// announced. §34: writing one segment's `Puid` onto both would be silent
/// corruption, and corruption is worse than loss.
#[test]
fn an_ambiguous_retained_attribute_is_omitted_and_warned_about() {
    let (xml, warnings) = round_trip(TWO_SEGMENTS);

    assert!(
        !xml.contains("Puid=\"101\"") && !xml.contains("Puid=\"102\""),
        "neither segment's Puid may be written when the two cannot be told \
         apart:\n{xml}"
    );

    let warning = warnings
        .iter()
        .find(|w| {
            matches!(
                w,
                ExportWarning::RetainedAttributeNotExported { element, attribute, .. }
                    if element == "Segment" && attribute == "Puid"
            )
        })
        .unwrap_or_else(|| panic!("no warning named the dropped Segment/@Puid: {warnings:?}"));
    let ExportWarning::RetainedAttributeNotExported {
        instances, detail, ..
    } = warning
    else {
        unreachable!("matched above")
    };
    assert_eq!(*instances, 2, "both segments should be counted");
    assert!(
        detail.contains("shared one `Segment` identity"),
        "the warning must say why, not merely that: {detail}"
    );
}

/// The unambiguous values on the same document are unaffected: each device
/// keeps its own `Puid`, and the line keeps its own. A rule that dropped
/// everything nearby would be safe and useless.
#[test]
fn unambiguous_neighbours_still_come_back() {
    let (xml, warnings) = round_trip(TWO_SEGMENTS);

    for puid in ["\"201\"", "\"202\"", "\"11\""] {
        assert!(
            xml.contains(&format!("Puid={puid}")),
            "Puid={puid} has an unambiguous owner and must be written back:\n{xml}"
        );
    }
    assert!(
        !warnings.iter().any(|w| matches!(
            w,
            ExportWarning::RetainedAttributeNotExported { element, .. }
                if element == "DeviceInstance"
        )),
        "no device attribute was dropped, so nothing should warn about one: {warnings:?}"
    );
}
