//! Schema ≥21 short `Links` ids resolve inside the device's own installation.
//!
//! `Links` names group addresses by short id (`GA-1`), which embeds no
//! installation; validation already checks dangling targets per installation
//! (IMPORT_EXPORT §validation). The mapper used one document-wide table in
//! which the last full id won, so a device in installation 0 could be linked
//! to installation 1's `GA-1` — a link between two separate infrastructures
//! (ADR-0070). Each installation now resolves against its own group
//! addresses, and a short id repeated inside one installation is reported
//! as ambiguous instead of being guessed.

use std::io::{Cursor, Write};

use knx_etsproj::import_knxproj_bytes;
use knx_etsproj::map::MapProblemDetail;

const PROJECT_INFO_23: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" />
  </Project>
</KNX>"#;

/// Installation `index` with one device whose object links `GA-1`, and the
/// group addresses `addresses` as `(short id, raw address)`.
fn installation(index: u8, addresses: &[(&str, u16)]) -> String {
    let p = format!("P-0001-{index}");
    let gas: String = addresses
        .iter()
        .map(|(short, raw)| {
            format!(r#"<GroupAddress Id="{p}_{short}" Address="{raw}" Name="{short}" />"#)
        })
        .collect();
    format!(
        r#"<Installation Name="I{index}" DefaultLine="{p}_L-2">
        <Topology>
          <Area Id="{p}_A-1" Address="{area}">
            <Line Id="{p}_L-2" Address="1">
              <Segment Id="{p}_L-2_S-1" Number="0" MediumTypeRefId="MT-0">
                <DeviceInstance Id="{p}_DI-1" Name="Sensor" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="22">
                  <ComObjectInstanceRefs>
                    <ComObjectInstanceRef RefId="O-6_R-921" Links="GA-1" />
                  </ComObjectInstanceRefs>
                  <GroupObjectTree GroupObjectInstances="O-6_R-921" />
                </DeviceInstance>
              </Segment>
            </Line>
          </Area>
        </Topology>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="{p}_GR-1" RangeStart="0" RangeEnd="65535" Name="all">{gas}</GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>"#,
        area = index + 1,
    )
}

fn knxproj(installations: &[String]) -> Vec<u8> {
    let xml = format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23" CreatedBy="ETS6" ToolVersion="ETS 6.3.0">
  <Project Id="P-0001">
    <Installations>{}</Installations>
  </Project>
</KNX>"#,
        installations.concat()
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

/// For each installation: the raw addresses its devices' links point to,
/// looked up in that same installation (`None` = the link points elsewhere).
fn links_per_installation(bytes: Vec<u8>) -> (Vec<Vec<Option<u16>>>, Vec<String>) {
    let out = import_knxproj_bytes(bytes, "scope.knxproj").unwrap();
    let project = &out.project;
    let per_installation = project
        .installations
        .iter()
        .map(|installation| {
            let devices = installation
                .topology
                .lines
                .iter()
                .flat_map(|line| &line.devices);
            devices
                .flat_map(|&device| project.devices.get(device).unwrap().com_objects.clone())
                .flat_map(|com| project.devices.com_object(com).unwrap().links.clone())
                .map(|link| {
                    installation
                        .group_addresses
                        .iter()
                        .find(|entry| entry.id == link.ga)
                        .map(|entry| entry.address.raw())
                })
                .collect()
        })
        .collect();
    let problems = out.report.errors.iter().map(|e| e.detail.clone()).collect();
    (per_installation, problems)
}

#[test]
fn the_same_short_id_in_two_installations_links_within_each_installation() {
    let (links, problems) = links_per_installation(knxproj(&[
        installation(0, &[("GA-1", 2051)]),
        installation(1, &[("GA-1", 4000)]),
    ]));
    assert_eq!(
        links,
        vec![vec![Some(2051)], vec![Some(4000)]],
        "each device links its own installation's GA-1; problems: {problems:?}"
    );
}

#[test]
fn a_short_id_repeated_inside_one_installation_is_ambiguous_not_guessed() {
    let (links, problems) = links_per_installation(knxproj(&[installation(
        0,
        &[("GA-1", 2051), ("GA-1", 2052)],
    )]));
    assert_eq!(links, vec![Vec::<Option<u16>>::new()]);
    let expected = format!(
        "{:?}",
        MapProblemDetail::AmbiguousReference {
            kind: "GroupAddressRef",
            target: "GA-1".into(),
        }
    );
    assert!(problems.contains(&expected), "{problems:?}");
}
