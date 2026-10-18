//! Real built CLI, using only its explicit synthetic transport.
use std::net::UdpSocket;
use std::process::Command;
use std::time::Duration;

use knx_core::{
    CompletionStatus, DeviceId, DeviceInstance, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};

const MEMORY_PROFILE: &str = "system2-declared-segments-v1";

/// A synthetic System-2 program: one PID78 comparison, two declared segments.
const PROGRAM: &str = r#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">
<ApplicationPrograms><ApplicationProgram Id="M-0001_A-0001-01-0000" Name="Synthetic" ApplicationNumber="1"
ApplicationVersion="1" MaskVersion="MV-0025" LoadProcedureStyle="ProductProcedure"><Static>
<Code><AbsoluteSegment Id="OBJ" Address="1024" Size="8" /></Code>
<ComObjectTable CodeSegment="OBJ" Offset="0" />
<LoadProcedures><LoadProcedure><LdCtrlConnect />
<LdCtrlCompareProp ObjIdx="0" PropId="78" InlineData="00010000005B00000000" />
<LdCtrlAbsSegment LsmIdx="1" SegType="0" Address="278" Size="30" Access="51" MemType="3" SegFlags="0" />
<LdCtrlAbsSegment LsmIdx="3" SegType="0" Address="15360" Size="12" Access="51" MemType="3" SegFlags="0" />
<LdCtrlTaskCtrl2 LsmIdx="3" Callback="0" Address="1024" Seg0="0" Seg1="0" />
<LdCtrlTaskSegment LsmIdx="5" Address="1024" /><LdCtrlDisconnect /></LoadProcedure></LoadProcedures>
</Static></ApplicationProgram></ApplicationPrograms></Manufacturer></ManufacturerData></KNX>"#;

fn fixture(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let products = dir.join("products.sqlite");
    let db = knx_productdb::open_and_migrate(&products).unwrap();
    knx_productdb::ingest_file(&db, "synthetic.xml", PROGRAM.as_bytes()).unwrap();
    let hardware = br#"<KNX xmlns="http://knx.org/xml/project/20"><ManufacturerData><Manufacturer RefId="M-0001">
<Hardware><Hardware Id="H" Name="Synthetic" SerialNumber="synthetic" VersionNumber="1">
<Products><Product Id="M-0001_H-1_P-1" /></Products><Hardware2Programs><Hardware2Program Id="H_HP" MediumTypes="MT-0">
<ApplicationProgramRef RefId="M-0001_A-0001-01-0000" /></Hardware2Program></Hardware2Programs>
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
        product_ref: "M-0001_H-1_P-1".into(),
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

fn silent_peer() -> UdpSocket {
    let peer = UdpSocket::bind("127.0.0.1:0").unwrap();
    peer.set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    peer
}

fn assert_nothing_received(peer: &UdpSocket) {
    let error = peer.recv_from(&mut [0; 1024]).unwrap_err();
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
}
#[test]
fn simulator_export_is_real_reimportable_and_create_only() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("diagnostic.zip");
    let result = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "diagnostics", "--simulate", "--output"])
        .arg(&output)
        .arg("--confirm-read")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report =
        knx_app::device_diagnostics::read_bundle(&std::fs::read(&output).unwrap()).unwrap();
    assert!(report.simulated);
    assert_eq!(report.items[0].bytes, [0x07, 0x01]);
    assert_eq!(report.items[1].bytes, [0x12, 0x34]);
    assert_eq!(report.items[2].bytes, [1, 2, 3, 4, 5, 6]);
    assert!(!report.written);
    assert_eq!(report.memory_status, "not-requested");
    let original = std::fs::read(&output).unwrap();
    let duplicate = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "diagnostics", "--simulate", "--output"])
        .arg(&output)
        .arg("--confirm-read")
        .output()
        .unwrap();
    assert!(!duplicate.status.success());
    assert_eq!(std::fs::read(&output).unwrap(), original);
}
#[test]
fn preview_without_confirmation_neither_acquires_nor_creates_a_file_and_write_flags_refuse() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("diagnostic.zip");
    let result = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "diagnostics", "--simulate", "--output"])
        .arg(&output)
        .output()
        .unwrap();
    assert!(result.status.success());
    assert!(String::from_utf8_lossy(&result.stdout).contains("Preview only"));
    assert!(!output.exists());
    for flag in ["--key-file", "--memory", "--restart", "--authorise"] {
        let result = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args(["device", "diagnostics", "--simulate", flag])
            .output()
            .unwrap();
        assert!(!result.status.success());
    }
}

#[test]
fn memory_profile_preview_lists_exact_ranges_and_never_contacts_the_gateway() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = fixture(dir.path());
    let output = dir.path().join("memory.zip");
    let peer = silent_peer();
    let result = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "diagnostics",
            "--profile",
            MEMORY_PROFILE,
            "--project",
        ])
        .arg(&project)
        .args(["--device-id", "1", "--product-db"])
        .arg(&products)
        .arg("--gateway")
        .arg(peer.local_addr().unwrap().to_string())
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for expected in [
        MEMORY_PROFILE,
        "M-0001_A-0001-01-0000",
        "0116h, 30 octets",
        "3C00h, 12 octets",
        "42 octets in 4 A_Memory_Read",
        "mask 0025h",
        "manufacturer 0001h",
        "00010000005b00000000",
        "Nothing will be written",
        "not proven runtime addresses",
        "Preview only",
    ] {
        assert!(stdout.contains(expected), "missing {expected}: {stdout}");
    }
    assert!(!output.exists());
    assert_nothing_received(&peer);
}

#[test]
fn memory_profile_simulation_runs_end_to_end_into_a_format_2_archive() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = fixture(dir.path());
    let output = dir.path().join("memory.zip");
    let result = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "diagnostics",
            "--simulate",
            "--profile",
            MEMORY_PROFILE,
        ])
        .arg("--project")
        .arg(&project)
        .args(["--device-id", "1", "--product-db"])
        .arg(&products)
        .arg("--output")
        .arg(&output)
        .arg("--confirm-read")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let export =
        knx_app::device_diagnostics::read_export(&std::fs::read(&output).unwrap()).unwrap();
    assert!(export.report.simulated);
    assert!(!export.report.written);
    assert_eq!(export.report.format, 2);
    assert_eq!(export.report.stop_reason, None);
    let memory = export.memory.unwrap();
    assert_eq!(memory.chunks.len(), 4);
    assert!(memory.chunks.iter().all(|c| c.status == "read"));
    assert_eq!(memory.ranges[0].coverage.read, 30);
    assert!(String::from_utf8_lossy(&result.stdout).contains("SIMULATED"));
}

#[test]
fn memory_profile_refuses_without_product_data_and_refuses_write_flags() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = fixture(dir.path());
    let output = dir.path().join("memory.zip");
    let peer = silent_peer();
    let gateway = peer.local_addr().unwrap().to_string();
    let result = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "diagnostics",
            "--profile",
            MEMORY_PROFILE,
            "--project",
        ])
        .arg(&project)
        .args(["--device-id", "1", "--gateway", &gateway, "--output"])
        .arg(&output)
        .arg("--confirm-read")
        .output()
        .unwrap();
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("--product-db"));
    for flag in ["--write", "--key", "--force", "--restart", "--memory-write"] {
        let result = Command::new(env!("CARGO_BIN_EXE_knx"))
            .args([
                "device",
                "diagnostics",
                "--profile",
                MEMORY_PROFILE,
                "--project",
            ])
            .arg(&project)
            .args(["--device-id", "1", "--product-db"])
            .arg(&products)
            .args(["--gateway", &gateway, "--output"])
            .arg(&output)
            .args([flag, "--confirm-read"])
            .output()
            .unwrap();
        assert!(!result.status.success(), "accepted {flag}");
    }
    let unknown = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "diagnostics",
            "--simulate",
            "--profile",
            "dump-everything",
        ])
        .arg("--output")
        .arg(&output)
        .output()
        .unwrap();
    assert!(!unknown.status.success());
    assert!(!output.exists());
    assert_nothing_received(&peer);
}
