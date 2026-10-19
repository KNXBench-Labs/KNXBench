//! Drives the built `knx` binary to test `bus write --dpt/--project/--dry-run`
//! (T29 — `docs/superpowers/specs/2026-09-11-dpt-codec-design.md` E4-D7).
//! Builds a tiny `.knxdb` directly through `knx-core`/`knx-store`, the same
//! pattern `cli_group_address_csv.rs` uses — but this fixture needs devices
//! with communication objects linked to group addresses, because that is
//! what `knx_core::resolve_group_address_dpt` reads (`crates/knx-core/src/
//! dpt/resolve.rs`'s own test module builds exactly that shape).
//!
//! None of these tests open a network socket: every case here either
//! fails during argument/DPT/value validation (which spec E4-D7 requires
//! to happen before a socket is ever opened) or passes `--dry-run`, which
//! encodes and prints without connecting.

use std::path::Path;
use std::process::{Command, Output};

use knx_core::{
    ComObjectInstance, CommissioningState, CompletionStatus, DeviceInstance, Direction, DptRef,
    GroupAddress, GroupAddressEntry, GroupLink, GroupValue, Installation, InstallationId, Language,
    Layer, Override, Project, Resolved, ResolvedFlags, SourceRef, Topology,
};

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

fn stated_dpt(main: u16, sub: u16) -> Override<DptRef> {
    Override::Value(Resolved {
        value: DptRef {
            main,
            sub: Some(sub),
        },
        layer: Layer::Instance,
    })
}

/// A project with one installation and one group address at `ga_addr`
/// (`GroupAddressStyle::ThreeLevel`), plus one device and one communication
/// object per entry of `dpts` — each com object `Send`-links to the group
/// address and states the given `(main, sub)` DPT. An empty `dpts` leaves
/// the group address with nothing linked to it at all, which
/// `resolve_group_address_dpt` reports as `GroupAddressDpt::None`.
fn project_with_group_address(ga_addr: &str, dpts: &[(u16, u16)]) -> Project {
    let mut project = Project::new(Language("en".into()));
    let ga_id = project.ids.next_group_address_id().unwrap();
    let mut unassigned = Vec::new();

    for &(main, sub) in dpts {
        let device_id = project.ids.next_device_id().unwrap();
        let com_id = project.ids.next_com_object_instance_id().unwrap();
        project.devices.insert_com_object(ComObjectInstance {
            id: com_id,
            source: source("t"),
            device: device_id,
            number: 0,
            text: Override::Absent,
            description: Override::Absent,
            dpt: stated_dpt(main, sub),
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
            source: source("t"),
            name: format!("D{}", device_id.0),
            description: None,
            address: None,
            product_ref: "P".into(),
            program_ref: "H".into(),
            commissioning: CommissioningState::default(),
            visibility_calculated: true,
            com_objects: vec![com_id],
            binary_data: vec![],
        });
        unassigned.push(device_id);
    }

    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned,
        },
        buildings: vec![],
        group_ranges: vec![],
        group_addresses: vec![GroupAddressEntry {
            id: ga_id,
            source: source("t"),
            name: "GA".into(),
            address: GroupAddress::parse(ga_addr, project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: None,
            declared_dpt: Default::default(),
        }],
        parameters: vec![],
    });
    project
}

fn write_store(path: &Path, project: &Project) {
    let conn = knx_store::open_and_migrate(path).expect("open/migrate store");
    knx_store::save_project(&conn, project).expect("save project");
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("failed to run the knx binary")
}

#[test]
fn usage_names_every_explicit_dpt_input_format() {
    let out = run_cli(&[]);
    assert!(!out.status.success());
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--input-format <canonical|decimal|hexadecimal|binary|text>"));
}

/// Mirrors `apps/knx-cli/src/main.rs`'s private `format_group_value_payload`
/// (there is no lib target to import it from — `apps/knx-cli` is
/// binary-only by design, spec E4-D9). Kept to these four lines
/// deliberately, so a drift between the two is a one-glance diff, not a
/// hidden assumption.
fn payload_string(v: &GroupValue) -> String {
    match v {
        GroupValue::Short(bits) => format!("{bits:#04x} (6-bit)"),
        GroupValue::Bytes(bytes) => format!("{bytes:02x?}"),
    }
}

#[test]
fn dry_run_with_explicit_dpt_prints_the_expected_payload_and_exits_0() {
    let dpt = DptRef::parse("DPST-9-1").unwrap();
    let expected = knx_core::encode(dpt, "21.5", knx_core::DptInputFormat::Decimal).unwrap();

    let out = run_cli(&[
        "bus",
        "write",
        "--gateway",
        "127.0.0.1:3671",
        "--dpt",
        "DPST-9-1",
        "--dry-run",
        "1/2/3",
        "21.5",
    ]);

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout,
        format!("1/2/3 DPST-9-1 21.5 -> {}\n", payload_string(&expected))
    );
}

#[test]
fn omitted_format_keeps_the_legacy_fixed_width_binary_bit_set() {
    let dpt = DptRef::parse("DPST-21-1").unwrap();
    let expected = knx_core::encode_inferred_format(dpt, "00000010").unwrap();
    assert_eq!(expected, GroupValue::Bytes(vec![0x02]));

    let out = run_cli(&[
        "bus",
        "write",
        "--gateway",
        "127.0.0.1:3671",
        "--dpt",
        "DPST-21-1",
        "--dry-run",
        "1/2/3",
        "00000010",
    ]);

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!(
            "1/2/3 DPST-21-1 00000010 -> {}\n",
            payload_string(&expected)
        )
    );
}

#[test]
fn dry_run_with_a_project_resolving_to_a_single_dpt_encodes_using_it() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    write_store(&store, &project_with_group_address("1/2/3", &[(1, 1)]));

    let dpt = DptRef::parse("DPST-1-1").unwrap();
    let expected = knx_core::encode(dpt, "on", knx_core::DptInputFormat::Canonical).unwrap();

    let out = run_cli(&[
        "bus",
        "write",
        "--gateway",
        "127.0.0.1:3671",
        "--project",
        store.to_str().unwrap(),
        "--dry-run",
        "1/2/3",
        "on",
    ]);

    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert_eq!(
        stdout,
        format!("1/2/3 DPST-1-1 on -> {}\n", payload_string(&expected))
    );
}

#[test]
fn a_group_address_with_conflicting_dpts_fails_naming_both_and_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    write_store(
        &store,
        &project_with_group_address("1/2/3", &[(1, 1), (5, 1)]),
    );

    let out = run_cli(&[
        "bus",
        "write",
        "--gateway",
        "127.0.0.1:3671",
        "--project",
        store.to_str().unwrap(),
        "--dry-run",
        "1/2/3",
        "1",
    ]);

    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("DPST-1-1"), "{stderr}");
    assert!(stderr.contains("DPST-5-1"), "{stderr}");
    assert!(stderr.contains("--dpt"), "{stderr}");
}

#[test]
fn a_group_address_resolving_to_nothing_fails_asking_for_dpt_and_exits_nonzero() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    write_store(&store, &project_with_group_address("1/2/3", &[]));

    let out = run_cli(&[
        "bus",
        "write",
        "--gateway",
        "127.0.0.1:3671",
        "--project",
        store.to_str().unwrap(),
        "--dry-run",
        "1/2/3",
        "1",
    ]);

    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("1/2/3"), "{stderr}");
    assert!(stderr.contains("--dpt"), "{stderr}");
}

#[test]
fn an_unparsable_value_for_the_given_dpt_fails_before_connecting() {
    let out = run_cli(&[
        "bus",
        "write",
        "--gateway",
        "127.0.0.1:3671",
        "--dpt",
        "DPST-9-1",
        "--dry-run",
        "1/2/3",
        "not-a-number",
    ]);

    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("DPST-9-1"), "{stderr}");
    assert!(stderr.contains("not-a-number"), "{stderr}");
}

#[test]
fn dry_run_uses_the_declared_hexadecimal_format_without_prefix_inference() {
    let out = run_cli(&[
        "bus",
        "write",
        "--gateway",
        "127.0.0.1:3671",
        "--dpt",
        "DPST-21-1",
        "--input-format",
        "hexadecimal",
        "--dry-run",
        "1/2/3",
        "07",
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(String::from_utf8(out.stdout).unwrap().contains("[07]"));
}

#[test]
fn declared_decimal_format_rejects_a_hexadecimal_prefix_before_connecting() {
    let out = run_cli(&[
        "bus",
        "write",
        "--gateway",
        "127.0.0.1:3671",
        "--dpt",
        "DPST-21-1",
        "--input-format",
        "decimal",
        "--dry-run",
        "1/2/3",
        "0x07",
    ]);
    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("declared format decimal"), "{stderr}");
}

#[test]
fn generic_group_writes_refuse_parameter_only_dpts_in_dry_run() {
    for dpt in [
        "DPST-7-3",
        "DPST-7-4",
        "DPST-7-6",
        "DPST-7-13",
        "DPST-8-3",
        "DPST-8-4",
        "DPST-8-6",
        "DPST-8-12",
        "DPST-20-22",
    ] {
        for explicit in [false, true] {
            let mut args = vec![
                "bus",
                "write",
                "--gateway",
                "127.0.0.1:3671",
                "--dpt",
                dpt,
                "--dry-run",
            ];
            if explicit {
                args.extend(["--input-format", "decimal"]);
            }
            args.extend(["1/2/3", "1"]);
            let out = run_cli(&args);
            assert!(
                !out.status.success(),
                "{dpt} was accepted as a runtime write"
            );
            let error = String::from_utf8(out.stderr).unwrap();
            assert!(
                error.contains("not allowed for generic runtime group writes"),
                "{error}"
            );
            assert!(error.contains(dpt), "{error}");
            assert!(out.stdout.is_empty());
        }
    }
}

#[test]
fn project_resolved_parameter_only_dpt_is_refused_in_dry_run() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("parameter.knxdb");
    write_store(&path, &project_with_group_address("1/2/3", &[(7, 13)]));
    for explicit in [false, true] {
        let mut args = vec![
            "bus",
            "write",
            "--gateway",
            "127.0.0.1:3671",
            "--project",
            path.to_str().unwrap(),
            "--dry-run",
        ];
        if explicit {
            args.extend(["--input-format", "decimal"]);
        }
        args.extend(["1/2/3", "1"]);
        let out = run_cli(&args);
        assert!(!out.status.success());
        assert!(String::from_utf8(out.stderr)
            .unwrap()
            .contains("not allowed for generic runtime group writes"));
        assert!(out.stdout.is_empty());
    }
}

#[test]
fn a_dpt_naming_an_unimplemented_main_type_fails_with_the_unsupported_error() {
    // This used to name DPST-20-102, which stopped being unimplemented when
    // main types 20-30 landed. DPST-31-101 replaces it, and stays
    // unimplemented on purpose: DPT-AS §4.7.1 says of
    // DPT_PB_Action_HVAC_Extended "This DPT shall not be used for runtime
    // communication", so `bus write` has nothing legitimate to send with it.
    let out = run_cli(&[
        "bus",
        "write",
        "--gateway",
        "127.0.0.1:3671",
        "--dpt",
        "DPST-31-101",
        "--dry-run",
        "1/2/3",
        "1",
    ]);

    assert_ne!(out.status.code(), Some(0));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("unsupported"), "{stderr}");
    assert!(stderr.contains("DPST-31-101"), "{stderr}");
}
