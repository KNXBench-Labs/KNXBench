//! The built probe CLI previews System-2 fields without opening a gateway.

use std::net::UdpSocket;
use std::process::Command;
use std::time::Duration;

use knx_core::{
    CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};

const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0000">
<ApplicationPrograms><ApplicationProgram Id="M-0000_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="OBJ" Address="1024" Size="8" />
<AbsoluteSegment Id="PAR" Address="15360" Size="256" /></Code>
<ParameterTypes><ParameterType Id="U16" Name="Time"><TypeNumber SizeInBit="16" Type="unsignedInt" minInclusive="0" maxInclusive="65535" /></ParameterType></ParameterTypes>
<Parameters><Parameter Id="P" Name="Time" ParameterType="U16" Value="300" LegacyPatchAlways="1"><Memory CodeSegment="PAR" Offset="248" BitOffset="0" /></Parameter></Parameters>
<ParameterRefs><ParameterRef Id="P_R" RefId="P" /></ParameterRefs>
<ComObjectTable CodeSegment="OBJ" Offset="0" />
<LoadProcedures><LoadProcedure><LdCtrlConnect /><LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" />
<LdCtrlTaskSegment LsmIdx="5" Address="1024" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static><Dynamic><Channel Id="CH" Text="Synthetic"><ParameterBlock Id="PB" Text="Block"><ParameterRefRef RefId="P_R" /></ParameterBlock></Channel></Dynamic>
</ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn fixture(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let products = dir.join("products.sqlite");
    let db = knx_productdb::open_and_migrate(&products).unwrap();
    knx_productdb::ingest_file(&db, "synthetic.xml", PROGRAM.as_bytes()).unwrap();
    let hardware = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0000">
<Hardware><Hardware Id="H" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1">
<Products><Product Id="M-0000_H-1_P-1" /></Products><Hardware2Programs><Hardware2Program Id="H_HP" MediumTypes="MT-0">
<ApplicationProgramRef RefId="M-0000_A-0001-01-0000" /></Hardware2Program></Hardware2Programs>
</Hardware></Hardware></Manufacturer></ManufacturerData></KNX>"#;
    knx_productdb::ingest_file(&db, "Hardware.xml", hardware).unwrap();
    let mut project = Project::new(Language("en".into()));
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "Synthetic".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![DeviceId(1)],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: SourceRef {
            path: "synthetic".into(),
            ets_id: "DEV-1".into(),
        },
        name: "Synthetic".into(),
        description: None,
        address: Some("1.1.67".parse().unwrap()),
        product_ref: "M-0000_H-1_P-1".into(),
        program_ref: "H_HP".into(),
        commissioning: Default::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
    });
    let path = dir.join("project.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &project).unwrap();
    (path, products)
}

#[test]
fn source_preview_reports_three_boundary_facts_and_never_sends_to_the_supplied_gateway() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = fixture(dir.path());
    let before = std::fs::read(&project).unwrap();
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "byte-order-probe", "1.1.67", "--project"])
        .arg(&project)
        .arg("--product-db")
        .arg(&products)
        .args([
            "--preview",
            "--system2-source",
            "--parameter-ref",
            "P_R",
            "--gateway",
        ])
        .arg(peer.local_addr().unwrap().to_string())
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&result.stderr);
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(result.status.success(), "{stderr}");
    for expected in [
        "Preview only",
        "3CF8",
        "300",
        "LegacyPatchAlways",
        "not a backup",
        "no bus contact",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
    let error = peer.recv_from(&mut [0; 1024]).unwrap_err();
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
    assert_eq!(std::fs::read(&project).unwrap(), before);
}

#[test]
fn source_override_without_preview_refuses_before_opening_the_project_or_gateway() {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_millis(100)))
        .unwrap();
    let result = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "byte-order-probe",
            "1.1.67",
            "--project",
            "does-not-exist.knxdb",
            "--system2-source",
            "--parameter-ref",
            "P_R",
            "--gateway",
        ])
        .arg(peer.local_addr().unwrap().to_string())
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--preview"));
    assert!(peer.recv_from(&mut [0; 1024]).is_err());
}
