//! KL-8 witness: a secure-capable program in a project without Secure activated.
//!
//! Synthetic throughout: manufacturer `M-FFF7`, one memory-mapped program
//! with a `ProductProcedure`, one device without a `Security` element and no
//! secure group addresses. The same project is imported twice, once with
//! `IsSecureEnabled="true"` and once with `"false"`, each into its own
//! product database. Whatever import, readiness and the planner do with the
//! one, they must do with the other: KNXBench implements no Secure semantics
//! (KNOWN_LIMITATIONS §8), so the flag is catalogue text and nothing more.
//! This pins today's behaviour; it is not a claim about what a secure-capable
//! device needs on the bus.

use std::collections::BTreeMap;

use knx_app::device_download::prepare_device_download;
use knx_app::project_readiness::{project_readiness, DeviceRow};
use knx_core::commissioning::memory_download::MemoryDownloadStep;
use knx_core::IndividualAddress;

const PROGRAM_ID: &str = "M-FFF7_A-0001-01-0000";

/// The secure variant also declares security table sizes, as secure-capable
/// programs do; they are catalogue text exactly like the flag.
const SECURE_ATTRIBUTES: &[&str] = &[
    "IsSecureEnabled",
    "MaxSecurityGroupKeyTableEntries",
    "MaxSecurityIndividualAddressEntries",
];

fn program(secure: bool) -> String {
    let tables = if secure {
        r#" MaxSecurityGroupKeyTableEntries="8" MaxSecurityIndividualAddressEntries="8""#
    } else {
        ""
    };
    format!(
        r#"<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-FFF7">
<ApplicationPrograms><ApplicationProgram Id="{PROGRAM_ID}" ApplicationNumber="1" ApplicationVersion="1"
  ProgramType="ApplicationProgram" MaskVersion="MV-0701" Name="Synthetic secure-capable"
  LoadProcedureStyle="ProductProcedure" PeiType="0" DefaultLanguage="en-US"
  DynamicTableManagement="false" Linkable="false" MinEtsVersion="5.7" IsSecureEnabled="{secure}"{tables}>
<Static>
  <Code><AbsoluteSegment Id="{PROGRAM_ID}_AS-4000" Address="16384" Size="4"><Data>AAAAAA==</Data></AbsoluteSegment></Code>
  <LoadProcedures><LoadProcedure><LdCtrlConnect/><LdCtrlUnload LsmIdx="1"/><LdCtrlLoad LsmIdx="1"/>
    <LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="16384" Size="4" Access="255" MemType="3" SegFlags="128"/>
    <LdCtrlLoadCompleted LsmIdx="1"/><LdCtrlRestart/><LdCtrlDisconnect/></LoadProcedure></LoadProcedures>
</Static>
<Dynamic/>
</ApplicationProgram></ApplicationPrograms>
</Manufacturer></ManufacturerData></KNX>"#
    )
}

const HARDWARE: &str = r#"<KNX xmlns="http://knx.org/xml/project/21"><ManufacturerData><Manufacturer RefId="M-FFF7">
<Hardware><Hardware Id="M-FFF7_H-1" Name="Synthetic" SerialNumber="SYN-1" VersionNumber="1"
  BusCurrent="10" HasIndividualAddress="true" HasApplicationProgram="true">
<Products><Product Id="M-FFF7_H-1_P-1" Text="Synthetic" OrderNumber="SYN-1" IsRailMounted="false" DefaultLanguage="en-US"/></Products>
<Hardware2Programs><Hardware2Program Id="M-FFF7_H-1_HP-0001-01-0000" MediumTypes="MT-0">
  <ApplicationProgramRef RefId="M-FFF7_A-0001-01-0000"/>
</Hardware2Program></Hardware2Programs>
</Hardware></Hardware>
</Manufacturer></ManufacturerData></KNX>"#;

const MASTER: &str = r#"<KNX xmlns="http://knx.org/xml/project/21"><MasterData>
<Manufacturers><Manufacturer Id="M-FFF7" Name="Synthetic"/></Manufacturers>
</MasterData></KNX>"#;

/// No `Security` element on the device, no secure group address: Secure is
/// not activated in this project.
const TOPOLOGY: &str = r#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001"><Installations>
<Installation InstallationId="0"><Topology><Area Id="P-0001-0_A-1" Address="1">
<Line Id="P-0001-0_L-1" Address="1"><Segment Id="P-0001-0_L-1_S-1" Number="0" MediumTypeRefId="MT-0">
<DeviceInstance Id="P-0001-0_DI-1" Name="Secure-capable, not activated" Address="1"
  ProductRefId="M-FFF7_H-1_P-1" Hardware2ProgramRefId="M-FFF7_H-1_HP-0001-01-0000"/>
</Segment></Line></Area></Topology>
<GroupAddresses><GroupRanges><GroupRange Id="P-0001-0_GR-1" RangeStart="1" RangeEnd="100" Name="Plain">
<GroupAddress Id="P-0001-0_GA-1" Address="1" Name="Plain group"/>
</GroupRange></GroupRanges></GroupAddresses>
</Installation></Installations></Project></KNX>"#;

const INFO: &str = r#"<KNX xmlns="http://knx.org/xml/project/21"><Project Id="P-0001">
<ProjectInformation Name="Synthetic Secure witness" GroupAddressStyle="ThreeLevel"/>
</Project></KNX>"#;

fn project_bytes(secure: bool) -> Vec<u8> {
    let program = program(secure);
    knx_testsupport::zip_with_entries(&[
        ("P-0001.signature", b"synthetic signature"),
        ("P-0001/0.xml", TOPOLOGY.as_bytes()),
        ("P-0001/Project.xml", INFO.as_bytes()),
        ("knx_master.xml", MASTER.as_bytes()),
        ("M-FFF7/Hardware.xml", HARDWARE.as_bytes()),
        ("M-FFF7/M-FFF7_A-0001-01-0000.xml", program.as_bytes()),
    ])
}

struct Witness {
    _dir: tempfile::TempDir,
    products: knx_productdb::Connection,
    imported: knx_app::ImportedProject,
}

fn witness(secure: bool) -> Witness {
    let dir = tempfile::tempdir().unwrap();
    let store = knx_store::open_and_migrate(&dir.path().join("p.knxdb")).unwrap();
    let products = knx_productdb::open_and_migrate(&dir.path().join("products.sqlite")).unwrap();
    let imported = knx_app::import::import_ets_project_bytes(
        project_bytes(secure),
        "synthetic-secure.knxproj",
        &store,
        knx_app::ImportOptions {
            product_db: Some(&products),
        },
    )
    .expect("the synthetic witness imports");
    Witness {
        _dir: dir,
        products,
        imported,
    }
}

fn target() -> IndividualAddress {
    "1.1.1".parse().unwrap()
}

/// Exhaustive on purpose: a future step kind that writes access keys has to
/// be classified here before this test compiles again.
fn writes_an_access_key(step: &MemoryDownloadStep) -> bool {
    match step {
        MemoryDownloadStep::Connect
        | MemoryDownloadStep::CompareProperty { .. }
        | MemoryDownloadStep::LoadRecord(_)
        | MemoryDownloadStep::WriteMemory { .. }
        | MemoryDownloadStep::RequireLoaded(_)
        | MemoryDownloadStep::Restart
        | MemoryDownloadStep::Disconnect => false,
    }
}

#[test]
fn the_secure_flag_is_stored_as_catalogue_text_and_nothing_else() {
    let secure = witness(true);
    let plain = witness(false);
    for (w, expected) in [(&secure, "true"), (&plain, "false")] {
        assert!(
            w.imported.report.errors.is_empty(),
            "{:?}",
            w.imported.report.errors
        );
        assert!(w.imported.manufacturer_ingested > 0);
        let programs = knx_productdb::query::programs(&w.products, None).unwrap();
        let program = programs
            .iter()
            .find(|p| p.id == PROGRAM_ID)
            .expect("the embedded program is in the catalogue");
        assert_eq!(program.is_secure_enabled.as_deref(), Some(expected));
        let tables = (expected == "true").then_some("8");
        assert_eq!(
            program.max_security_group_key_table_entries.as_deref(),
            tables
        );
        // Nothing about Secure is reported as unknown or retained.
        let report = serde_json::to_string(&w.imported.report).unwrap();
        assert!(!report.contains("Security"), "{report}");
    }
    // The imported project model does not depend on the flag at all.
    assert_eq!(secure.imported.project, plain.imported.project);
}

#[test]
fn readiness_and_the_planner_treat_both_programs_identically() {
    let secure = witness(true);
    let plain = witness(false);
    let rows = |w: &Witness| -> Vec<DeviceRow> {
        project_readiness(&w.products, &w.imported.project, &BTreeMap::new())
    };
    assert_eq!(rows(&secure), rows(&plain));
    // Graded like any other plannable program without recorded evidence.
    assert_eq!(rows(&secure)[0].readiness.code(), "untested");
    // The flag reaches the planner's program input only as one verbatim
    // entry of the generic attribute map, which planning reads by name
    // (`PeiType`, `ApplicationNumber`, `ApplicationVersion`), never this one.
    let code = |w: &Witness| {
        knx_productdb::code::load_program_code(&w.products, PROGRAM_ID)
            .unwrap()
            .expect("the program has code")
    };
    let (mut secure_code, mut plain_code) = (code(&secure), code(&plain));
    assert_eq!(
        secure_code
            .program_attributes
            .get("IsSecureEnabled")
            .map(String::as_str),
        Some("true")
    );
    for attribute in SECURE_ATTRIBUTES {
        secure_code.program_attributes.remove(*attribute);
        plain_code.program_attributes.remove(*attribute);
    }
    assert_eq!(secure_code, plain_code);
    let prepared =
        |w: &Witness| prepare_device_download(&w.products, &w.imported.project, target());
    let (secure_plan, plain_plan) = (
        prepared(&secure)
            .expect("the secure-capable program plans")
            .plan,
        prepared(&plain).expect("the plain program plans").plan,
    );
    assert_eq!(secure_plan, plain_plan);
    assert_eq!(secure_plan.steps.len(), 8);
    assert!(!secure_plan.steps.iter().any(writes_an_access_key));
}
