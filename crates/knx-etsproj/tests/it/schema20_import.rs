//! ETS 5.7 projects (schema 20) import as the ETS6 model without `Segment` (issue #7).
//!
//! The always-on tests use a hand-written project shaped after the measured
//! inventory of three public ETS 5.7 sample projects (xknxproject's GPL-2.0
//! test resources, inspected, not copied — docs/research/project-format.md
//! §3.6). The `#[ignore]`d test reads those samples and xknxproject's own
//! expected-output stubs from a checkout named by
//! `KNXBENCH_XKNXPROJECT_RESOURCES` and compares against them.

use std::collections::BTreeSet;
use std::io::{Cursor, Write};
use std::path::PathBuf;

use knx_etsproj::{import_knxproj, import_knxproj_bytes, import_knxproj_with, ImportFailure};
use knx_etsproj::{ImportOutcome, ProjectPassword};

const TOPOLOGY_20: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" CreatedBy="ETS5" ToolVersion="5.7.0.0" xmlns="http://knx.org/xml/project/20">
  <Project Id="P-0001">
    <Installations>
      <Installation Name="" BCUKey="4294967295" DefaultLine="P-0001-0_L-2" IPRoutingLatencyTolerance="2000">
        <Topology>
          <Area Id="P-0001-0_A-1" Name="Ground floor" Address="1" Puid="1">
            <Line Id="P-0001-0_L-2" Name="Lighting line" Address="1" MediumTypeRefId="MT-0" Puid="2">
              <DeviceInstance Id="P-0001-0_DI-1" Address="5" Name="Switch actuator" ProductRefId="M-0001_H-1_P-1" Hardware2ProgramRefId="M-0001_H-1_HP-1" Comment="" Description="" CompletionStatus="Accepted" IsActivityCalculated="true" LastModified="2022-04-25T14:58:32.9818931Z" Puid="3">
                <ParameterInstanceRefs>
                  <ParameterInstanceRef RefId="P-1_R-1" Value="1" />
                </ParameterInstanceRefs>
                <ComObjectInstanceRefs>
                  <ComObjectInstanceRef RefId="O-1_R-1" Links="GA-1" DatapointType="DPST-1-1" />
                  <ComObjectInstanceRef RefId="O-2_R-2" Text="Status" UpdateFlag="Enabled" />
                </ComObjectInstanceRefs>
                <GroupObjectTree GroupObjectInstances="O-1_R-1 O-2_R-2" />
              </DeviceInstance>
            </Line>
          </Area>
        </Topology>
        <Locations>
          <Space Type="Building" Id="P-0001-0_BP-1" Name="House" Puid="4">
            <Space Type="Room" Id="P-0001-0_BP-2" Name="Kitchen" Number="1" Puid="5" />
          </Space>
        </Locations>
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" Name="Lighting" RangeStart="1" RangeEnd="2047" Description="" Puid="6">
              <GroupRange Id="P-0001-0_GR-2" Name="Switching" RangeStart="1" RangeEnd="255" Puid="7">
                <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="Kitchen light" DatapointType="DPST-1-1" Puid="8" />
              </GroupRange>
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

const PROJECT_INFO_20: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance" xmlns:xsd="http://www.w3.org/2001/XMLSchema" CreatedBy="ETS5" ToolVersion="5.7.0.0" xmlns="http://knx.org/xml/project/20">
  <Project Id="P-0001">
    <ProjectInformation Name="Schema 20 sample" GroupAddressStyle="ThreeLevel" LastModified="2022-04-25T14:59:02.0234098Z" ProjectStart="2022-04-25T14:58:07.0541219Z" Comment="" CompletionStatus="Editing" LastUsedPuid="8" Guid="00000000-0000-4000-8000-000000000020" />
  </Project>
</KNX>"#;

fn knxproj(topology: &[u8]) -> Vec<u8> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("P-0001.signature", &b"x"[..]),
        ("P-0001/0.xml", topology),
        ("P-0001/project.xml", PROJECT_INFO_20),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes).unwrap();
    }
    writer.finish().unwrap().into_inner()
}

fn import_sample() -> ImportOutcome {
    import_knxproj_bytes(knxproj(TOPOLOGY_20), "schema20.knxproj").expect("schema 20 imports")
}

#[test]
fn a_schema_20_project_imports_with_its_topology_devices_and_links() {
    let out = import_sample();
    assert_eq!(out.report.source.schema_version, 20);

    let installation = &out.project.installations[0];
    let line = &installation.topology.lines[0];
    assert_eq!(line.address, 1);
    assert_eq!(
        line.medium_ref, "MT-0",
        "the medium sits on Line at schema 20"
    );
    assert_eq!(line.devices.len(), 1, "DeviceInstance directly under Line");

    let device = out.project.devices.iter().next().unwrap();
    assert_eq!(device.address.unwrap().to_string(), "1.1.5");
    assert_eq!(device.com_objects.len(), 2);

    let ga = &installation.group_addresses[0];
    assert_eq!(ga.name, "Kitchen light");
    assert_eq!(
        ga.address.format(out.project.info.group_address_style),
        "0/0/1"
    );
    let linked: Vec<_> = device
        .com_objects
        .iter()
        .filter_map(|id| out.project.devices.com_object(*id))
        .flat_map(|c| c.links.iter().map(|l| l.ga))
        .collect();
    assert_eq!(
        linked,
        vec![ga.id],
        "Links=\"GA-1\" resolves to the address"
    );

    let part = |name: &str| {
        installation
            .buildings
            .iter()
            .find(|b| b.name == name)
            .unwrap_or_else(|| panic!("no building part {name}"))
    };
    let (house, kitchen) = (part("House"), part("Kitchen"));
    assert_eq!(installation.buildings.len(), 2);
    assert_eq!(
        kitchen.parent,
        Some(house.id),
        "nested Space keeps its parent"
    );
    assert_eq!(kitchen.number.as_deref(), Some("1"));
    assert_eq!(out.project.info.name, "Schema 20 sample");
}

#[test]
fn the_measured_schema_20_vocabulary_is_known_not_reported_as_unknown() {
    let out = import_sample();
    assert!(
        out.report.unknown.is_empty(),
        "every construct of the measured schema-20 shape must be known: {:?}",
        out.report.unknown
    );
}

#[test]
fn schema_20_keys_devices_without_a_segment_level() {
    let out = import_sample();
    let device = "/KNX/Project/Installations/Installation/Topology/Area/Line/DeviceInstance[@Id='P-0001-0_DI-1']";
    assert!(
        out.opaque
            .iter()
            .any(|e| e.xpath == format!("{device}/GroupObjectTree")),
        "the GroupObjectTree is retained under the device's real path: {:?}",
        out.opaque.iter().map(|e| &e.xpath).collect::<Vec<_>>()
    );
    assert!(
        out.opaque.iter().all(|e| !e.xpath.contains("/Segment")),
        "schema 20 has no Segment element, so no key may invent one"
    );
}

#[test]
fn a_schema_20_import_says_what_its_support_rests_on() {
    let out = import_sample();
    assert!(
        out.report
            .unsupported
            .iter()
            .any(|u| u.what.contains("schema 20") && u.consequence.contains("public")),
        "the evidence boundary is reported, not implied: {:?}",
        out.report.unsupported
    );
}

#[test]
fn a_schema_20_unassigned_device_is_known_too() {
    let topology = String::from_utf8(TOPOLOGY_20.to_vec()).unwrap().replace(
        "</Topology>",
        r#"<UnassignedDevices>
            <DeviceInstance Id="P-0001-0_DI-9" Name="Spare" ProductRefId="M-0001_H-1_P-1" Hardware2ProgramRefId="M-0001_H-1_HP-1" Puid="9">
              <ComObjectInstanceRefs><ComObjectInstanceRef RefId="O-1_R-1" /></ComObjectInstanceRefs>
            </DeviceInstance>
          </UnassignedDevices>
        </Topology>"#,
    );
    let out = import_knxproj_bytes(knxproj(topology.as_bytes()), "unassigned.knxproj").unwrap();
    assert_eq!(out.project.devices.iter().count(), 2);
    assert!(
        out.report.unknown.is_empty(),
        "an unassigned schema-20 device uses the same DeviceInstance vocabulary: {:?}",
        out.report.unknown
    );
}

#[test]
fn schema_20_validation_names_locations_in_its_own_vocabulary() {
    // Two `Space`s with one id: the duplicate is reported at the path the
    // file really has (`Locations/Space`), not schema 11's `Buildings`.
    let topology = String::from_utf8(TOPOLOGY_20.to_vec())
        .unwrap()
        .replace(r#"Id="P-0001-0_BP-2""#, r#"Id="P-0001-0_BP-1""#);
    let out = import_knxproj_bytes(knxproj(topology.as_bytes()), "duplicate.knxproj").unwrap();
    let duplicate = out
        .report
        .errors
        .iter()
        .find(|e| e.detail.contains("DuplicateId"))
        .expect("the duplicate Space id is reported");
    assert!(
        duplicate
            .xpath
            .contains("/Locations/Space[@Id='P-0001-0_BP-1']"),
        "{}",
        duplicate.xpath
    );
}

#[test]
fn an_unsupported_schema_names_itself_and_what_is_supported() {
    let topology = String::from_utf8(TOPOLOGY_20.to_vec())
        .unwrap()
        .replace("project/20", "project/22");
    let error = match import_knxproj_bytes(knxproj(topology.as_bytes()), "v22.knxproj") {
        Err(error @ ImportFailure::NoKnownSchemaTable { version: 22 }) => error,
        other => panic!("expected a refusal for schema 22, got {other:?}"),
    };
    let message = error.to_string();
    assert!(message.contains("schema 22"), "{message}");
    assert!(message.contains("cannot import"), "{message}");
    for supported in ["11 (ETS4)", "20 (ETS 5.7)", "21", "23"] {
        assert!(
            message.contains(supported),
            "{supported} missing: {message}"
        );
    }
}

// ---------------------------------------------------------------------------
// Opt-in comparison with the public ETS 5.7 samples and xknxproject's stubs.

fn resources() -> PathBuf {
    let dir = std::env::var_os("KNXBENCH_XKNXPROJECT_RESOURCES").expect(
        "set KNXBENCH_XKNXPROJECT_RESOURCES to xknxproject's test/resources directory \
         (this test is #[ignore]d and must be run explicitly)",
    );
    let dir = PathBuf::from(dir);
    assert!(
        dir.join("stubs").is_dir(),
        "{} has no stubs/",
        dir.display()
    );
    dir
}

/// `(address, name)` per group address and the set of linked
/// `individual address/RefId` objects, from xknxproject's expected output.
struct Oracle {
    group_addresses: BTreeSet<(String, String)>,
    devices: BTreeSet<String>,
    links: BTreeSet<(String, String)>,
}

fn oracle(stub: &str) -> Oracle {
    let text = std::fs::read_to_string(resources().join("stubs").join(stub)).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(json["info"]["schema_version"], "20");
    let mut group_addresses = BTreeSet::new();
    let mut links = BTreeSet::new();
    for ga in json["group_addresses"].as_object().unwrap().values() {
        let address = ga["address"].as_str().unwrap().to_string();
        group_addresses.insert((address.clone(), ga["name"].as_str().unwrap().to_string()));
        for object in ga["communication_object_ids"].as_array().unwrap() {
            links.insert((address.clone(), object.as_str().unwrap().to_string()));
        }
    }
    let devices = json["devices"]
        .as_object()
        .unwrap()
        .values()
        .map(|d| d["individual_address"].as_str().unwrap().to_string())
        .collect();
    Oracle {
        group_addresses,
        devices,
        links,
    }
}

fn ours(out: &ImportOutcome) -> Oracle {
    let style = out.project.info.group_address_style;
    let installation = &out.project.installations[0];
    let address_of = |id| {
        installation
            .group_addresses
            .iter()
            .find(|g| g.id == id)
            .unwrap()
            .address
            .format(style)
    };
    let mut links = BTreeSet::new();
    for device in out.project.devices.iter() {
        let Some(ia) = device.address else { continue };
        for object in device
            .com_objects
            .iter()
            .filter_map(|id| out.project.devices.com_object(*id))
        {
            for link in &object.links {
                links.insert((
                    address_of(link.ga),
                    format!("{ia}/{}", object.source.ets_id),
                ));
            }
        }
    }
    Oracle {
        group_addresses: installation
            .group_addresses
            .iter()
            .map(|g| (g.address.format(style), g.name.clone()))
            .collect(),
        devices: out
            .project
            .devices
            .iter()
            .filter_map(|d| d.address.map(|a| a.to_string()))
            .collect(),
        links,
    }
}

/// Error-severity report entries, counted by the two documented refusals:
/// `(sub-module objects, DPT lists)`. Objects inside nested (sub-)modules are
/// refused by `RefId` (`SubModule` ids are not implemented,
/// KNOWN_LIMITATIONS §177); an instance `DatapointType` that lists several
/// DPTs is kept verbatim as `Malformed` and reported (`override_dpt`). Any
/// other error fails the comparison outright.
fn documented_refusals(out: &ImportOutcome, stub: &str) -> (usize, usize) {
    let (mut sub_modules, mut dpt_lists) = (0, 0);
    for e in out
        .report
        .errors
        .iter()
        .filter(|e| e.severity == knx_etsproj::report::Severity::Error)
    {
        if let Some(id) = e.detail.strip_prefix("Value(MalformedRefId(\"") {
            assert!(
                id.contains("_SM-"),
                "{stub}: {id} is not a sub-module object"
            );
            sub_modules += 1;
        } else if e.detail.starts_with(
            "Value(UnknownEnumValue { kind: \"ComObjectInstanceRef/@DatapointType\", value: \"DPT-",
        ) && e.detail.contains(' ')
        {
            dpt_lists += 1;
        } else {
            panic!("{stub}: unexpected import error {e:?}");
        }
    }
    (sub_modules, dpt_lists)
}

fn assert_agrees(out: &ImportOutcome, stub: &str, refusals: (usize, usize)) {
    assert_eq!(out.report.source.schema_version, 20);
    assert_eq!(
        documented_refusals(out, stub),
        refusals,
        "{stub}: (sub-module objects, DPT lists)"
    );

    let theirs = oracle(stub);
    let mine = ours(out);
    assert!(!theirs.group_addresses.is_empty() && !theirs.links.is_empty());
    assert_eq!(
        mine.group_addresses, theirs.group_addresses,
        "{stub}: group addresses"
    );
    assert_eq!(mine.devices, theirs.devices, "{stub}: device addresses");
    assert_eq!(mine.links, theirs.links, "{stub}: object-to-address links");
}

#[test]
#[ignore = "requires KNXBENCH_XKNXPROJECT_RESOURCES (xknxproject test/resources, GPL-2.0, not vendored)"]
fn the_public_ets57_samples_agree_with_xknxproject() {
    let dir = resources();
    let plain = import_knxproj(&dir.join("module-definition-test.knxproj")).unwrap();
    assert!(
        plain.project.devices.module_instances().next().is_some(),
        "the module sample carries module instances"
    );
    assert_agrees(&plain, "module-definition-test.json", (8, 0));

    // ETS5 ZipCrypto (schema < 21) with the published test password.
    let password = ProjectPassword::new("test");
    let protected =
        import_knxproj_with(&dir.join("xknx_test_project.knxproj"), Some(&password), &()).unwrap();
    assert_agrees(&protected, "xknx_test_project.json", (0, 1));

    let small = import_knxproj(&dir.join("xknx_test_project_no_password.knxproj")).unwrap();
    assert_eq!(small.report.source.schema_version, 20);
    assert_eq!(small.project.devices.iter().count(), 1);
    assert_eq!(small.project.installations[0].group_addresses.len(), 7);
}

/// The nested-layout fix is not schema-20 specific: the public ETS4
/// (schema 11) protected sample stores bare `0.xml`/`Project.xml` too.
#[test]
#[ignore = "requires KNXBENCH_XKNXPROJECT_RESOURCES (xknxproject test/resources, GPL-2.0, not vendored)"]
fn the_public_protected_ets4_sample_decrypts_and_agrees_with_xknxproject() {
    let dir = resources();
    let password = ProjectPassword::new("test");
    let out =
        import_knxproj_with(&dir.join("test_project-ets4.knxproj"), Some(&password), &()).unwrap();
    assert_eq!(out.report.source.schema_version, 11);
    let text = std::fs::read_to_string(dir.join("stubs/test_project-ets4.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&text).unwrap();
    let theirs: BTreeSet<(String, String)> = json["group_addresses"]
        .as_object()
        .unwrap()
        .values()
        .map(|g| {
            (
                g["address"].as_str().unwrap().to_string(),
                g["name"].as_str().unwrap().to_string(),
            )
        })
        .collect();
    let mine = ours(&out);
    assert!(!theirs.is_empty());
    assert_eq!(mine.group_addresses, theirs);
    let devices: BTreeSet<String> = json["devices"]
        .as_object()
        .unwrap()
        .values()
        .map(|d| d["individual_address"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(mine.devices, devices);
}
