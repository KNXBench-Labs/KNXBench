//! `knx bus write --project` reads and prints group addresses in the project's own style.
//!
//! AR14 / KNOWN_LIMITATIONS §29 and §62 item 13: the CLI parsed every
//! destination as three-level even when the project uses two-level or free
//! addresses, so a free-style project's `2049` was refused before its DPT was
//! ever looked up. Without `--project` there is no style to read, and the
//! three-level default stays. Every case uses `--dry-run` or fails during
//! validation, so no socket is opened (the CLI validates before connecting).

use std::path::Path;
use std::process::{Command, Output};

use knx_core::{
    ComObjectInstance, CommissioningState, CompletionStatus, DeviceInstance, Direction, DptRef,
    GroupAddress, GroupAddressEntry, GroupAddressStyle, GroupLink, GroupValue, Installation,
    InstallationId, Language, Layer, Override, Project, Resolved, ResolvedFlags, SourceRef,
    Topology,
};

fn source() -> SourceRef {
    SourceRef {
        path: "t".into(),
        ets_id: "t".into(),
    }
}

/// One group address (typed in `style`) linked to one DPT-1.001 object.
fn project_in_style(style: GroupAddressStyle, ga: &str) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.group_address_style = style;
    let ga_id = project.ids.next_group_address_id().unwrap();
    let device_id = project.ids.next_device_id().unwrap();
    let com_id = project.ids.next_com_object_instance_id().unwrap();
    project.devices.insert_com_object(ComObjectInstance {
        id: com_id,
        source: source(),
        device: device_id,
        number: 0,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Value(Resolved {
            value: DptRef {
                main: 1,
                sub: Some(1),
            },
            layer: Layer::Instance,
        }),
        flags: ResolvedFlags::none(),
        size: None,
        is_active: true,
        links: vec![GroupLink {
            ga: ga_id,
            direction: Direction::Send,
        }],
        module_instance: None,
    });
    project.devices.insert(DeviceInstance {
        id: device_id,
        source: source(),
        name: "D".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![com_id],
        binary_data: vec![],
    });
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![device_id],
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![GroupAddressEntry {
            id: ga_id,
            source: source(),
            name: "GA".into(),
            address: GroupAddress::parse(ga, style).unwrap(),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        }],
        parameters: vec![],
    });
    project
}

fn store_with(dir: &Path, style: GroupAddressStyle, ga: &str) -> String {
    let path = dir.join("p.knxdb");
    let conn = knx_store::open_and_migrate(&path).unwrap();
    knx_store::save_project(&conn, &project_in_style(style, ga)).unwrap();
    path.to_str().unwrap().to_string()
}

fn dry_run(project: Option<&str>, ga: &str) -> Output {
    let mut args = vec!["bus", "write", "--gateway", "127.0.0.1:3671", "--dry-run"];
    if let Some(project) = project {
        args.extend(["--project", project]);
    }
    if project.is_none() {
        args.extend(["--dpt", "DPST-1-1"]);
    }
    args.extend([ga, "on"]);
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(&args)
        .output()
        .expect("failed to run the knx binary")
}

fn on_payload() -> String {
    let dpt = DptRef::parse("DPST-1-1").unwrap();
    match knx_core::encode(dpt, "on", knx_core::DptInputFormat::Canonical).unwrap() {
        GroupValue::Short(bits) => format!("{bits:#04x} (6-bit)"),
        GroupValue::Bytes(bytes) => format!("{bytes:02x?}"),
    }
}

fn assert_dry_run(out: &Output, expected_address: &str) {
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&out.stdout),
        format!("{expected_address} DPST-1-1 on -> {}\n", on_payload())
    );
}

#[test]
fn a_free_style_project_takes_and_prints_free_addresses() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_with(dir.path(), GroupAddressStyle::Free, "2049");
    assert_dry_run(&dry_run(Some(&store), "2049"), "2049");
}

#[test]
fn a_two_level_project_takes_and_prints_two_level_addresses() {
    let dir = tempfile::tempdir().unwrap();
    // Raw 2049 is 1/1 two-level and 1/0/1 three-level.
    let store = store_with(dir.path(), GroupAddressStyle::TwoLevel, "1/1");
    assert_dry_run(&dry_run(Some(&store), "1/1"), "1/1");
}

#[test]
fn a_three_level_project_is_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_with(dir.path(), GroupAddressStyle::ThreeLevel, "1/2/3");
    assert_dry_run(&dry_run(Some(&store), "1/2/3"), "1/2/3");
}

/// The project's style is the only accepted spelling, like the server's
/// `/api/bus/write`: a three-level string is not silently reinterpreted.
#[test]
fn a_free_style_project_refuses_a_three_level_spelling() {
    let dir = tempfile::tempdir().unwrap();
    let store = store_with(dir.path(), GroupAddressStyle::Free, "2049");
    let out = dry_run(Some(&store), "1/0/1");
    assert_ne!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("invalid group address 1/0/1"), "{stderr}");
}

/// No project, no style to read: the documented three-level default stays.
#[test]
fn without_a_project_three_level_stays_the_default() {
    assert_dry_run(&dry_run(None, "1/0/1"), "1/0/1");
    let free = dry_run(None, "2049");
    assert_ne!(free.status.code(), Some(0));
    assert!(free.stdout.is_empty());
}
