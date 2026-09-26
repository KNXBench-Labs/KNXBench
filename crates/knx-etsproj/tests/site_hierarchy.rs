//! ADR-0038: a site above several buildings is an ordinary `Ground` space, not a new kind.

use std::io::{Cursor, Write};

use knx_core::{BuildingPart, BuildingPartType, DeviceId};
use knx_etsproj::import_knxproj_bytes;

const PROJECT_INFO_23: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" />
  </Project>
</KNX>"#;

/// Schema 23 §1.2.6.3: "Space elements directly below Locations_t will
/// normally have Type "Area" or "Building" or "Ground"". One installation,
/// one line carrying both devices, and a `Ground` root holding two
/// `Building`s, each of which references one of the two devices.
/// `root_type` lets the same fixture probe a type the standard does not
/// document.
fn installation_23(root_type: &str) -> Vec<u8> {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23" CreatedBy="ETS6" ToolVersion="ETS 6.3.0">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="" DefaultLine="P-0001-0_L-2">
        <Topology>
          <Area Id="P-0001-0_A-1" Address="1">
            <Line Id="P-0001-0_L-2" Address="1">
              <Segment Id="P-0001-0_L-2_S-1" Number="0" MediumTypeRefId="MT-0">
                <DeviceInstance Id="P-0001-0_DI-1" Name="North actuator" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="1" />
                <DeviceInstance Id="P-0001-0_DI-2" Name="South actuator" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="2" />
              </Segment>
            </Line>
          </Area>
        </Topology>
        <Locations>
          <Space Id="P-0001-0_BP-1" Name="Campus" Type="{root_type}" Puid="1">
            <Space Id="P-0001-0_BP-2" Name="North house" Type="Building" Puid="2">
              <DeviceInstanceRef RefId="P-0001-0_DI-1" />
            </Space>
            <Space Id="P-0001-0_BP-3" Name="South house" Type="Building" Puid="3">
              <DeviceInstanceRef RefId="P-0001-0_DI-2" />
            </Space>
          </Space>
        </Locations>
      </Installation>
    </Installations>
  </Project>
</KNX>"#
    )
    .into_bytes()
}

fn knxproj(root_type: &str) -> Vec<u8> {
    let installation = installation_23(root_type);
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("P-0001.signature", b"x".as_slice()),
        ("P-0001/0.xml", installation.as_slice()),
        ("P-0001/Project.xml", PROJECT_INFO_23),
    ] {
        writer.start_file(name, options).expect("zip entry");
        writer.write_all(bytes).expect("zip entry body");
    }
    writer.finish().expect("zip central directory").into_inner()
}

fn by_name<'a>(parts: &'a [BuildingPart], name: &str) -> &'a BuildingPart {
    parts
        .iter()
        .find(|part| part.name == name)
        .unwrap_or_else(|| panic!("no building part named {name}"))
}

#[test]
fn a_ground_root_groups_two_buildings_of_one_installation() {
    let imported = import_knxproj_bytes(knxproj("Ground"), "site.knxproj").unwrap();
    assert!(
        imported.report.errors.is_empty(),
        "{:?}",
        imported.report.errors
    );
    let project = &imported.project;

    // One installation, one line: the two buildings share the KNX
    // infrastructure rather than each getting their own.
    assert_eq!(project.installations.len(), 1);
    let installation = &project.installations[0];
    assert_eq!(installation.topology.lines.len(), 1);
    assert_eq!(installation.topology.lines[0].devices.len(), 2);

    let parts = &installation.buildings;
    assert_eq!(parts.len(), 3);
    let site = by_name(parts, "Campus");
    let north = by_name(parts, "North house");
    let south = by_name(parts, "South house");

    assert_eq!(site.kind, BuildingPartType::Ground);
    assert_eq!(site.parent, None);
    assert_eq!(site.children, vec![north.id, south.id]);
    assert!(site.devices.is_empty());
    for building in [north, south] {
        assert_eq!(building.kind, BuildingPartType::Building);
        assert_eq!(building.parent, Some(site.id));
    }

    // Devices stay owned once, by `Devices`; each building only references
    // its own (DATA_MODEL §5). Grouping under a site duplicates nothing.
    assert_eq!(project.devices.iter().count(), 2);
    assert_eq!(north.devices.len(), 1);
    assert_eq!(south.devices.len(), 1);
    assert_ne!(north.devices[0], south.devices[0]);
    let referenced: Vec<DeviceId> = parts.iter().flat_map(|p| p.devices.clone()).collect();
    assert_eq!(referenced.len(), 2);
}

/// The issue's interface rule: a type the standard does not document must
/// stay reported, never silently become a site or a building.
#[test]
fn an_undocumented_root_type_is_reported_not_read_as_a_site() {
    let imported = import_knxproj_bytes(knxproj("Site"), "site.knxproj").unwrap();
    let type_errors: Vec<_> = imported
        .report
        .errors
        .iter()
        .filter(|error| error.stage == "map" && error.detail.contains("BuildingPart/@Type"))
        .collect();
    assert_eq!(type_errors.len(), 1, "{:?}", imported.report.errors);
    assert!(type_errors[0].detail.contains("Site"));

    let parts = &imported.project.installations[0].buildings;
    let root = by_name(parts, "Campus");
    assert_eq!(root.kind, BuildingPartType::BuildingPart);
    assert_ne!(root.kind, BuildingPartType::Ground);
    // The hierarchy under it is still intact.
    assert_eq!(root.children.len(), 2);
}
