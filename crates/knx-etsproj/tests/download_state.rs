//! C14: ETS's differential-download state (`LoadedImage`, `CheckSums`,
//! `DownloadCounter` on `DeviceInstance`, `Project Schema23 v01.00.00.pdf`
//! p. 44) survives import byte-exact, is named in the import report rather
//! than only counted, and is correctly absent when the source project never
//! carries it (the common case, since all three are optional).

use std::io::{Cursor, Write};

use knx_etsproj::{import_knxproj_bytes, Container};

const PROJECT_INFO_21: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" />
  </Project>
</KNX>"#;

/// A schema-21-shaped `0.xml` (`Segment` between `Line` and `DeviceInstance`,
/// per `crate::known::SCHEMA_21`) with one device. `with_download_state`
/// controls whether that device carries `LoadedImage`/`CheckSums`/
/// `DownloadCounter` — the two fixtures this file needs differ only there.
fn installation_21(with_download_state: bool) -> Vec<u8> {
    let download_state_attrs = if with_download_state {
        r#" LoadedImage="QUJD" CheckSums="RUZH" DownloadCounter="7""#
    } else {
        ""
    };
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/21" CreatedBy="ETS6" ToolVersion="ETS 6.1.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="" DefaultLine="P-0001-0_L-2">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1">
            <Line Id="P-0001-0_L-2" Address="1">
              <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0">
                <DeviceInstance Id="P-0001-0_DI-1" Name="D" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1"
                                LastModified="2023-07-14T11:55:33.0000000Z"
                                IndividualAddressLoaded="true" ApplicationProgramLoaded="true"
                                ParametersLoaded="true" CommunicationPartLoaded="true"
                                MediumConfigLoaded="true" IsActivityCalculated="true"{download_state_attrs} />
              </Segment>
            </Line>
          </Area>
        </Topology>
      </Installation>
    </Installations>
  </Project>
</KNX>"#
    )
    .into_bytes()
}

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

/// Same shape as `malformed_input.rs`'s `knxproj_with_installation`, kept as
/// its own copy here rather than pulled into `tests/support/mod.rs`: this is
/// the only test file that needs a schema-21 container, and duplicating six
/// lines is cheaper than a shared helper two other schema-11-only test files
/// would have to keep ignoring.
fn knxproj_with_installation_21(installation_xml: &[u8]) -> Vec<u8> {
    zip_with_entries(&[
        ("P-0001.signature", b"x"),
        ("P-0001/0.xml", installation_xml),
        ("P-0001/Project.xml", PROJECT_INFO_21),
    ])
}

/// The fixture carrying all three attributes: not reported as a loss, and
/// preserved byte-exact — matching `a.value.into_bytes()`
/// ([`knx_etsproj::opaque::from_retained_attribute`]), i.e. the base64 text
/// itself, not decoded. `LoadedImage` is opaque base64 and stays opaque.
#[test]
fn download_state_attributes_are_preserved_and_reported_by_name() {
    let bytes = knxproj_with_installation_21(&installation_21(true));
    let outcome = import_knxproj_bytes(bytes, "download-state.knxproj").unwrap();

    // Not a "loss": these are documented, optional ETS state, not something
    // this importer failed to understand.
    assert!(
        outcome
            .report
            .unknown
            .iter()
            .all(|u| !["LoadedImage", "CheckSums", "DownloadCounter"].contains(&u.name.as_str())),
        "download-state attributes must not appear in the unknown/loss report: {:?}",
        outcome.report.unknown
    );
    assert!(
        !outcome.report.has_losses(),
        "a documented, optional ETS attribute must not trip has_losses(): {:?}",
        outcome.report
    );

    // Preserved byte-exact in the opaque entry list `export_knxproj` reads.
    let device_xpath =
        "/KNX/Project/Installations/Installation/Topology/Area/Line/Segment/DeviceInstance";
    let find = |name: &str| {
        outcome
            .opaque
            .iter()
            .find(|e| e.xpath == device_xpath && e.name == name)
            .unwrap_or_else(|| panic!("no opaque entry for {name} at {device_xpath}"))
    };
    let loaded_image = find("LoadedImage");
    let checksums = find("CheckSums");
    let download_counter = find("DownloadCounter");
    assert_eq!(loaded_image.bytes, b"QUJD");
    assert_eq!(checksums.bytes, b"RUZH");
    assert_eq!(download_counter.bytes, b"7");
    assert_eq!(
        loaded_image.kind,
        knx_etsproj::opaque::OpaqueKind::RetainedAttribute
    );

    // Named, not just counted, in the report's preserved-data section —
    // "A field preserved in the store but invisible in the report only
    // half-satisfies the brief."
    for name in ["LoadedImage", "CheckSums", "DownloadCounter"] {
        assert!(
            outcome
                .report
                .opaque
                .iter()
                .any(|s| s.xpath == device_xpath && s.name == name),
            "{name} must be named in report.opaque, not just present in the opaque store: {:?}",
            outcome.report.opaque
        );
    }
}

/// The common case: none of the three attributes present (most projects
/// never had a differential download recorded). No phantom entries, no
/// phantom report rows, no loss.
#[test]
fn download_state_attributes_absent_is_unremarkable() {
    let bytes = knxproj_with_installation_21(&installation_21(false));
    let outcome = import_knxproj_bytes(bytes, "no-download-state.knxproj").unwrap();

    assert!(!outcome.report.has_losses(), "{:?}", outcome.report);
    for name in ["LoadedImage", "CheckSums", "DownloadCounter"] {
        assert!(
            !outcome.opaque.iter().any(|e| e.name == name),
            "no {name} entry should exist when the source never carried it"
        );
        assert!(
            !outcome.report.opaque.iter().any(|s| s.name == name),
            "no {name} row should exist in the report when the source never carried it"
        );
    }
}

/// No export/round-trip-through-a-file path exists for these three
/// attributes today, same as every other per-`DeviceInstance` known-but-
/// unmapped attribute (`KNOWN_LIMITATIONS.md` #34): `schema21.rs` never
/// calls `fill_retained()` for `DeviceInstance`, because `RetainedAttrs` is
/// keyed only by `(xpath, name)`, project-wide, not per device — writing
/// one device's `DownloadCounter` back onto every device the moment a
/// project has more than one would be silent corruption, not preservation.
/// This test documents that today's `export_knxproj` output for this
/// fixture does *not* contain the attribute text, rather than silently
/// relying on the two tests above to imply it.
#[test]
fn download_state_attributes_do_not_survive_export_yet() {
    let bytes = knxproj_with_installation_21(&installation_21(true));
    let outcome = import_knxproj_bytes(bytes, "download-state.knxproj").unwrap();

    let entries: Vec<_> = outcome
        .opaque
        .iter()
        .cloned()
        .chain(
            outcome
                .manufacturer
                .iter()
                .map(|m| knx_etsproj::opaque::OpaqueEntry {
                    source_path: m.source_path.clone(),
                    xpath: String::new(),
                    kind: m.kind,
                    name: String::new(),
                    bytes: m.bytes.clone(),
                    sha256: m.sha256.clone(),
                }),
        )
        .collect();
    let exported = knx_etsproj::export::export_knxproj(&outcome.project, &entries).unwrap();
    // Read the exported archive's own `0.xml` back out rather than
    // substring-scanning `exported.bytes` directly — that is a ZIP
    // container, not text, and may or may not compress its entries.
    let mut written = Container::open(exported.bytes).unwrap();
    let part = written.project_part().unwrap().to_string();
    let topology = written.read(&format!("{part}/0.xml")).unwrap();
    let xml = String::from_utf8_lossy(&topology);
    assert!(
        !xml.contains("DownloadCounter")
            && !xml.contains("LoadedImage")
            && !xml.contains("CheckSums"),
        "export_knxproj unexpectedly reconstructed a per-device known-but-unmapped \
         attribute — if this now passes, KNOWN_LIMITATIONS.md #34 and this test's own \
         doc comment are both stale and need updating together"
    );
}
