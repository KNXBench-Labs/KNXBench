//! Schema-23 `GroupAddress/@Description` becomes the address's description (ADR-0164).

use std::io::{Cursor, Write};

use knx_etsproj::import_knxproj_bytes;

const PROJECT_INFO_23: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" />
  </Project>
</KNX>"#;

/// One synthetic schema-23 installation whose group addresses carry
/// `attributes` verbatim after `Id`, `Address` and `Name`.
fn knxproj(addresses: &[(&str, u16, &str)]) -> Vec<u8> {
    let gas: String = addresses
        .iter()
        .map(|(short, raw, attributes)| {
            format!(
                r#"<GroupAddress Id="P-0001-0_{short}" Address="{raw}" Name="{short}" {attributes}/>"#
            )
        })
        .collect();
    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23" CreatedBy="ETS6" ToolVersion="ETS 6.3.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="I0">
        <Topology />
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" RangeStart="0" RangeEnd="65535" Name="all" Description="Range text">{gas}</GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#
    );
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("P-0001.signature", &b"x"[..]),
        ("P-0001/0.xml", xml.as_bytes()),
        ("P-0001/Project.xml", PROJECT_INFO_23),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

#[test]
fn every_description_state_reaches_the_model_and_is_not_also_kept_opaque() {
    let out = import_knxproj_bytes(
        knxproj(&[
            (
                "GA-1",
                1,
                r#"Description="Beschreibung mit; Semikolon und &quot;Anf&#xfc;hrung&quot;""#,
            ),
            ("GA-2", 2, r#"Description="""#),
            ("GA-3", 3, ""),
            ("GA-4", 4, r#"Description="Zeile 1&#10;Zeile 2  ""#),
        ]),
        "described.knxproj",
    )
    .unwrap();
    assert!(out.report.errors.is_empty(), "{:?}", out.report.errors);
    let descriptions: Vec<_> = out.project.installations[0]
        .group_addresses
        .iter()
        .map(|entry| (entry.address.raw(), entry.description.clone()))
        .collect();
    assert_eq!(
        descriptions,
        vec![
            (
                1,
                Some("Beschreibung mit; Semikolon und \"Anf\u{fc}hrung\"".to_string())
            ),
            (2, Some(String::new())),
            (3, None),
            (4, Some("Zeile 1\nZeile 2  ".to_string())),
        ]
    );
    assert!(
        !out.opaque
            .iter()
            .any(|entry| entry.name == "Description" && entry.xpath.contains("/GroupAddress[")),
        "a modelled attribute is stored once, not also as an opaque row"
    );
}

#[test]
fn a_group_range_description_stays_retained_and_is_not_mapped() {
    let out = import_knxproj_bytes(knxproj(&[("GA-1", 1, "")]), "range.knxproj").unwrap();
    let range_rows: Vec<_> = out
        .opaque
        .iter()
        .filter(|entry| entry.name == "Description" && entry.xpath.contains("/GroupRange[@Id="))
        .collect();
    assert_eq!(range_rows.len(), 1, "{:?}", out.opaque);
    assert_eq!(range_rows[0].bytes, b"Range text");
    assert_eq!(
        out.project.installations[0].group_addresses[0].description,
        None
    );
}
