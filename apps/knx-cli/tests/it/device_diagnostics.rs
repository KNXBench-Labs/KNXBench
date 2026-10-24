//! Exercises the built diagnostics CLI with synthetic fixtures and silent loopback peers.
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
    fixture_with(dir, &[])
}

/// The synthetic device 1 at 1.1.67 plus `extra` devices (id, name, address).
fn fixture_with(
    dir: &std::path::Path,
    extra: &[(u32, &str, Option<&str>)],
) -> (std::path::PathBuf, std::path::PathBuf) {
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
            unassigned: std::iter::once(1)
                .chain(extra.iter().map(|(id, _, _)| *id))
                .map(DeviceId)
                .collect(),
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
    for (id, name, address) in extra {
        project.devices.insert(DeviceInstance {
            id: DeviceId(*id),
            source: SourceRef {
                path: "synthetic".into(),
                ets_id: format!("DEV-{id}"),
            },
            name: (*name).into(),
            description: None,
            address: address.map(|a| a.parse().unwrap()),
            product_ref: "M-0001_H-1_P-1".into(),
            program_ref: "H_HP".into(),
            commissioning: Default::default(),
            visibility_calculated: true,
            com_objects: vec![],
            binary_data: vec![],
        });
    }
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

fn diagnostics(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "diagnostics"])
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn device_address_selects_the_device_for_both_profile_previews_without_contact() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = fixture_with(dir.path(), &[(2, "Unaddressed", None)]);
    let (project, products) = (project.to_str().unwrap(), products.to_str().unwrap());
    let output = dir.path().join("diagnostic.zip");
    let peer = silent_peer();
    let gateway = peer.local_addr().unwrap().to_string();
    let common = [
        "--project",
        project,
        "--device",
        "1.1.67",
        "--product-db",
        products,
        "--gateway",
        &gateway,
        "--output",
        output.to_str().unwrap(),
    ];
    let identity = diagnostics(&common);
    let stdout = String::from_utf8_lossy(&identity.stdout);
    assert!(
        identity.status.success(),
        "{}",
        String::from_utf8_lossy(&identity.stderr)
    );
    assert!(stdout.contains("Device 1.1.67 via"), "{stdout}");
    assert!(stdout.contains("Preview only"), "{stdout}");
    let memory = diagnostics(&[&["--profile", MEMORY_PROFILE][..], &common].concat());
    let stdout = String::from_utf8_lossy(&memory.stdout);
    assert!(
        memory.status.success(),
        "{}",
        String::from_utf8_lossy(&memory.stderr)
    );
    assert!(
        stdout.contains(&format!("Profile {MEMORY_PROFILE} for device 1.1.67")),
        "{stdout}"
    );
    assert!(stdout.contains("Preview only"), "{stdout}");
    assert!(!output.exists());
    assert_nothing_received(&peer);
}

#[test]
fn device_address_drives_a_simulated_memory_export() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = fixture(dir.path());
    let output = dir.path().join("memory.zip");
    let result = diagnostics(&[
        "--simulate",
        "--profile",
        MEMORY_PROFILE,
        "--project",
        project.to_str().unwrap(),
        "--device",
        "1.1.67",
        "--product-db",
        products.to_str().unwrap(),
        "--output",
        output.to_str().unwrap(),
        "--confirm-read",
    ]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let export =
        knx_app::device_diagnostics::read_export(&std::fs::read(&output).unwrap()).unwrap();
    assert!(export.report.simulated);
    assert_eq!(export.memory.unwrap().chunks.len(), 4);
}

#[test]
fn ambiguous_unknown_malformed_or_conflicting_selectors_refuse_before_any_contact() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = fixture_with(
        dir.path(),
        &[(2, "Duplicate", Some("1.1.67")), (3, "Unaddressed", None)],
    );
    let (project, products) = (project.to_str().unwrap(), products.to_str().unwrap());
    let output = dir.path().join("diagnostic.zip");
    let peer = silent_peer();
    let gateway = peer.local_addr().unwrap().to_string();
    let cases: [(&[&str], &[&str]); 6] = [
        (
            &["--device", "1.1.67"],
            &[
                "1.1.67",
                "1 \"Synthetic\"",
                "2 \"Duplicate\"",
                "--device-id",
            ],
        ),
        (
            &["--profile", MEMORY_PROFILE, "--device", "1.1.67"],
            &["1 \"Synthetic\"", "2 \"Duplicate\""],
        ),
        (&["--device", "1.1.99"], &["1.1.99", "no project device"]),
        (
            &["--device", "1.1.256"],
            &["area.line.device", "\"1.1.256\""],
        ),
        (&["--device", "1.1.67", "--device-id", "1"], &["not both"]),
        (&[], &["--device <area.line.device>", "--device-id <id>"]),
    ];
    for (selector, expected) in cases {
        let args = [
            selector,
            &[
                "--project",
                project,
                "--product-db",
                products,
                "--gateway",
                &gateway,
                "--output",
                output.to_str().unwrap(),
                "--confirm-read",
            ],
        ]
        .concat();
        let result = diagnostics(&args);
        let stderr = String::from_utf8_lossy(&result.stderr);
        assert!(!result.status.success(), "{selector:?} accepted");
        for text in expected {
            assert!(
                stderr.contains(text),
                "{selector:?}: missing {text}: {stderr}"
            );
        }
    }
    assert!(!output.exists());
    assert_nothing_received(&peer);
}

#[test]
fn duplicate_addresses_across_installations_refuse_address_then_each_internal_id() {
    let dir = tempfile::tempdir().unwrap();
    let (project, products) = fixture_with(dir.path(), &[(2, "Duplicate", Some("1.1.67"))]);
    let conn = knx_store::open_and_migrate(&project).unwrap();
    let mut p = knx_store::load_project(&conn).unwrap();
    let mut second = p.installations[0].clone();
    second.id = InstallationId(1);
    second.name = "Second installation".into();
    second.topology.unassigned = vec![DeviceId(2)];
    p.installations[0].topology.unassigned = vec![DeviceId(1)];
    p.installations.push(second);
    knx_store::save_project(&conn, &p).unwrap();
    drop(conn);

    let peer = silent_peer();
    let gateway = peer.local_addr().unwrap().to_string();
    for profile in ["generic-identity-v1", MEMORY_PROFILE] {
        let output = dir.path().join(format!("{profile}.zip"));
        let common = [
            "--profile",
            profile,
            "--project",
            project.to_str().unwrap(),
            "--product-db",
            products.to_str().unwrap(),
            "--gateway",
            &gateway,
            "--output",
            output.to_str().unwrap(),
            "--confirm-read",
        ];
        let address = diagnostics(&[&common[..], &["--device", "1.1.67"]].concat());
        assert!(!address.status.success());
        assert!(address.stdout.is_empty());
        assert!(!output.exists());
        assert_nothing_received(&peer);
        let stderr = String::from_utf8_lossy(&address.stderr);
        // Keep the identification list, then try every listed internal id.
        let candidates = [("1", "Synthetic"), ("2", "Duplicate")];
        let mut id_errors = Vec::new();
        for (id, name) in candidates {
            assert!(stderr.contains(&format!("{id} {name:?}")), "{stderr}");
            let by_id = diagnostics(&[&common[..], &["--device-id", id]].concat());
            assert!(!by_id.status.success(), "{profile}: id {id} accepted");
            assert!(by_id.stdout.is_empty());
            assert!(!output.exists());
            assert_nothing_received(&peer);
            let id_error = String::from_utf8_lossy(&by_id.stderr);
            assert!(
                id_error.contains("diagnostics-address-ambiguous"),
                "{id_error}"
            );
            id_errors.push(id_error.into_owned());
        }
        assert!(
            stderr.contains(
                "individual address must uniquely identify one device across the project"
            ),
            "{stderr}"
        );
        assert!(stderr.contains("--device-id does not bypass"), "{stderr}");
        assert!(!stderr.contains("choose one with"), "{stderr}");
        for id_error in id_errors {
            assert!(
                id_error.contains(
                    "individual address must uniquely identify one device across the project"
                ),
                "{id_error}"
            );
            assert!(
                id_error.contains("--device-id does not bypass"),
                "{id_error}"
            );
        }
    }
}

#[test]
fn diagnostics_help_requires_project_wide_unique_addresses_for_both_selectors() {
    let result = Command::new(env!("CARGO_BIN_EXE_knx")).output().unwrap();
    assert!(!result.status.success());
    let stderr = String::from_utf8_lossy(&result.stderr);
    assert!(stderr.contains("uniquely identify one device across the project"));
    assert!(stderr.contains("--device-id does not bypass"));
    assert!(!stderr.contains("internal --device-id the refusal lists"));
}

#[test]
fn simulated_identity_still_refuses_a_device_selector() {
    let dir = tempfile::tempdir().unwrap();
    let output = dir.path().join("diagnostic.zip");
    let result = diagnostics(&[
        "--simulate",
        "--device",
        "1.1.25",
        "--output",
        output.to_str().unwrap(),
    ]);
    assert!(!result.status.success());
    assert!(
        String::from_utf8_lossy(&result.stderr).contains("--simulate cannot be combined"),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(!output.exists());
}
