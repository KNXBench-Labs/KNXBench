//! Schema ≥21 `ComObjectInstanceRef/@Links`: the first address sends, the rest receive.
//!
//! Project Schema23 v01.00.00 §`ComObjectInstanceRef` defines `Links` as "The
//! list of (shortened) group address ids that are linked with this object.
//! The first group address in the list is always the sending one." Mapping
//! every entry to `Send` gave an object with two links two sending addresses,
//! which the download planner refuses as a configuration no device can hold.

use std::io::{Cursor, Write};

use knx_core::{Direction, GroupAddress};
use knx_etsproj::import_knxproj_bytes;

const PROJECT_INFO_23: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/23">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" />
  </Project>
</KNX>"#;

/// One device whose object 6 links `links` (short ids), and three group
/// addresses `GA-1` = 1/0/3, `GA-2` = 1/3/3, `GA-3` = 1/3/4.
fn installation_23(links: &str) -> Vec<u8> {
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
                <DeviceInstance Id="P-0001-0_DI-1" Name="Sensor" ProductRefId="M-0001_H-1_P-1"
                                Hardware2ProgramRefId="M-0001_H-1_HP-1" Address="22">
                  <ComObjectInstanceRefs>
                    <ComObjectInstanceRef RefId="O-6_R-921" Links="{links}" />
                  </ComObjectInstanceRefs>
                  <GroupObjectTree GroupObjectInstances="O-6_R-921" />
                </DeviceInstance>
              </Segment>
            </Line>
          </Area>
        </Topology>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" RangeStart="2048" RangeEnd="4095" Name="1">
              <GroupAddress Id="P-0001-0_GA-1" Address="2051" Name="a" />
              <GroupAddress Id="P-0001-0_GA-2" Address="2819" Name="b" />
              <GroupAddress Id="P-0001-0_GA-3" Address="2820" Name="c" />
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#
    )
    .into_bytes()
}

fn knxproj(links: &str) -> Vec<u8> {
    let installation = installation_23(links);
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("P-0001.signature", &b"x"[..]),
        ("P-0001/0.xml", &installation[..]),
        ("P-0001/Project.xml", PROJECT_INFO_23),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

/// The object's links in document order, as (group address, direction).
fn links_of(links: &str) -> Vec<(GroupAddress, Direction)> {
    let out = import_knxproj_bytes(knxproj(links), "links.knxproj").unwrap();
    let project = &out.project;
    let com = project
        .devices
        .com_objects()
        .next()
        .unwrap_or_else(|| panic!("one com object expected; errors {:?}", out.report.errors));
    com.links
        .iter()
        .map(|l| {
            let ga = project.installations[0]
                .group_addresses
                .iter()
                .find(|g| g.id == l.ga)
                .expect("link resolves");
            (ga.address, l.direction)
        })
        .collect()
}

#[test]
fn the_first_link_sends_and_the_others_receive() {
    assert_eq!(
        links_of("GA-1 GA-2 GA-3"),
        vec![
            (GroupAddress::from_raw(2051), Direction::Send),
            (GroupAddress::from_raw(2819), Direction::Receive),
            (GroupAddress::from_raw(2820), Direction::Receive),
        ]
    );
}

#[test]
fn a_single_link_sends() {
    assert_eq!(
        links_of("GA-2"),
        vec![(GroupAddress::from_raw(2819), Direction::Send)]
    );
}

/// The order is the document's, not the group addresses' numeric order:
/// the higher address listed first is the sending one.
#[test]
fn the_sending_link_is_the_first_listed_not_the_lowest_address() {
    assert_eq!(
        links_of("GA-2 GA-1"),
        vec![
            (GroupAddress::from_raw(2819), Direction::Send),
            (GroupAddress::from_raw(2051), Direction::Receive),
        ]
    );
}

/// A dangling first entry is reported and dropped, as before; it does not
/// promote the next address to sender, because the file named no other.
#[test]
fn a_dangling_first_link_does_not_promote_the_second_to_sender() {
    assert_eq!(
        links_of("GA-9 GA-2"),
        vec![(GroupAddress::from_raw(2819), Direction::Receive)]
    );
}

/// The same house exported twice: the ETS4 file (schema 11) states every
/// direction explicitly with `Send`/`Receive` elements; the ETS 6.3.0 file
/// (schema 23) only lists `Links`. For every object that has exactly one
/// sending address in the ETS4 file and the same address set in both, the
/// schema-23 import must now name that same address as the sender and
/// every other link as receiving.
#[test]
#[ignore = "requires the gitignored OriginalData/ corpus; run with --ignored"]
fn the_ets6_house_agrees_with_the_ets4_export_on_every_sender() {
    use std::collections::{BTreeMap, BTreeSet};

    assert!(
        knx_testsupport::corpus_available(),
        "OriginalData/ corpus not present (gitignored, local-only); this test is #[ignore]d and must be run explicitly on a machine that has it"
    );
    type Key = (String, u16);
    type Directed = BTreeMap<Key, (Vec<GroupAddress>, Vec<GroupAddress>)>;
    fn directed(path: &std::path::Path) -> Directed {
        let out = knx_etsproj::import_knxproj(path).unwrap();
        let project = &out.project;
        let address_of = |id| {
            project
                .installations
                .iter()
                .flat_map(|i| &i.group_addresses)
                .find(|g| g.id == id)
                .map(|g| g.address)
                .expect("link resolves")
        };
        let mut map = Directed::new();
        for device in project.devices.iter() {
            let Some(ia) = device.address else { continue };
            for &com_id in &device.com_objects {
                let com = project.devices.com_object(com_id).unwrap();
                if com.links.is_empty() {
                    continue;
                }
                let entry = map.entry((ia.to_string(), com.number)).or_default();
                for link in &com.links {
                    match link.direction {
                        Direction::Send => entry.0.push(address_of(link.ga)),
                        Direction::Receive => entry.1.push(address_of(link.ga)),
                    }
                }
            }
        }
        map
    }

    let ets4 = directed(&knx_testsupport::reference_ets4_path());
    let ets6 = directed(&knx_testsupport::reference_ets6_path());
    let (mut agree, mut several_links) = (0usize, 0usize);
    for (key, (send4, receive4)) in &ets4 {
        let Some((send6, receive6)) = ets6.get(key) else {
            continue;
        };
        let all4: BTreeSet<_> = send4.iter().chain(receive4).collect();
        let all6: BTreeSet<_> = send6.iter().chain(receive6).collect();
        if all4 != all6 || send4.len() != 1 {
            continue; // the two exports differ here; nothing to compare
        }
        assert_eq!(send6, send4, "{key:?}: the sender");
        agree += 1;
        if all6.len() > 1 {
            several_links += 1;
        }
    }
    // Measured on the two corpus files: 543 comparable objects, 2 of them
    // with more than one link — exactly the two objects (1.1.22 object 6,
    // 1.1.24 object 56) the download planner used to refuse as sending on
    // two group addresses.
    assert_eq!((agree, several_links), (543, 2));
}
