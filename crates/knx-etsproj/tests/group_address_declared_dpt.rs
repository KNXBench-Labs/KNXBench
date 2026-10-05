//! Schema ≥21 `GroupAddress/@DatapointType` becomes the address's declared DPT (ADR-0078).

use std::io::{Cursor, Write};

use knx_core::dpt::DptRef;
use knx_core::provenance::{Layer, Override, Resolved};
use knx_etsproj::import_knxproj_bytes;

const PROJECT_INFO_23: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" />
  </Project>
</KNX>"#;

/// One schema-23 installation whose group addresses carry `attributes`
/// verbatim after `Id`, `Address` and `Name`.
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
            <GroupRange Id="P-0001-0_GR-1" RangeStart="0" RangeEnd="65535" Name="all">{gas}</GroupRange>
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
fn every_source_state_of_the_attribute_reaches_the_model_and_nothing_stays_opaque() {
    let out = import_knxproj_bytes(
        knxproj(&[
            ("GA-1", 2049, r#"DatapointType="DPST-9-1""#),
            ("GA-2", 2050, r#"DatapointType="""#),
            ("GA-3", 2051, r#"DatapointType="DPST-1-1 DPST-1-2""#),
            ("GA-4", 2052, ""),
        ]),
        "declared.knxproj",
    )
    .unwrap();
    let declared: Vec<_> = out.project.installations[0]
        .group_addresses
        .iter()
        .map(|entry| (entry.address.raw(), entry.declared_dpt.clone()))
        .collect();
    assert_eq!(
        declared,
        vec![
            (
                2049,
                Override::Value(Resolved {
                    value: DptRef {
                        main: 9,
                        sub: Some(1)
                    },
                    layer: Layer::Instance,
                })
            ),
            (2050, Override::Empty),
            (2051, Override::Malformed("DPST-1-1 DPST-1-2".into())),
            (2052, Override::Absent),
        ]
    );
    assert!(
        !out.opaque.iter().any(|entry| entry.name == "DatapointType"),
        "a modelled attribute is stored once, not also as an opaque row"
    );
    let reported: Vec<_> = out
        .report
        .errors
        .iter()
        .filter(|error| error.detail.contains("GroupAddress/@DatapointType"))
        .collect();
    assert_eq!(reported.len(), 1, "only the malformed value is reported");
    assert!(reported[0].detail.contains("DPST-1-1 DPST-1-2"));
    assert_eq!(out.project.info.unlifted_group_address_dpt_declarations, 0);
}

/// The store's v10 migration spells this key itself
/// (`GROUP_ADDRESS_XPATH_PREFIX` in `knx-store`); the malformed value's
/// report entry carries the importer's key, so both are pinned to one text.
#[test]
fn the_reported_key_is_the_keyed_group_address_path_the_store_migration_matches() {
    let out = import_knxproj_bytes(
        knxproj(&[("GA-7", 2049, r#"DatapointType="DPT-x""#)]),
        "declared.knxproj",
    )
    .unwrap();
    let error = out
        .report
        .errors
        .iter()
        .find(|error| error.detail.contains("GroupAddress/@DatapointType"))
        .unwrap();
    assert_eq!(
        error.xpath,
        "/KNX/Project/Installations/Installation/GroupAddresses/GroupRanges/GroupRange/GroupAddress[@Id='P-0001-0_GA-7']"
    );
}
