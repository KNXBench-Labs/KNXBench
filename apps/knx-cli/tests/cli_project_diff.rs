//! Drives the built `knx` binary to test `knx diff` (T14 —
//! `docs/superpowers/specs/2026-09-10-project-diff-design.md`). Builds two
//! tiny `.knxdb` files directly through `knx-core`/`knx-store`, same pattern
//! `cli_documentation_export.rs` uses. This subcommand renders "what changed
//! between two KNXBench project files", never an ETS comparison — no
//! ETS-produced comparison sample exists anywhere in this repository to be
//! compatible with.

use std::path::Path;
use std::process::{Command, Output};
use std::{fs, io};

use knx_core::{
    ComObjectInstance, ComObjectInstanceId, CommissioningState, CompletionStatus, DeviceId,
    DeviceInstance, GroupAddress, GroupAddressEntry, GroupRange, GroupRangeId, IndividualAddress,
    Installation, InstallationId, Language, Layer, Override, Project, Resolved, ResolvedFlags,
    SourceRef, Topology,
};

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

/// A project named "Test Villa" with one installation, one group range
/// spanning the whole address space, and one group address inside it
/// (`1/1/1`) named `ga_name` — the baseline "nothing structurally odd"
/// fixture, parameterized on the one field the rename test needs to change.
fn project_with_ga_name(ga_name: &str) -> Project {
    let mut project = Project::new(Language("en".into()));
    project.info.name = "Test Villa".into();
    project.installations.push(Installation {
        id: InstallationId(0),
        name: "I".into(),
        default_line: None,
        multicast_address: None,
        completion: CompletionStatus::FinishedDesign,
        topology: Topology {
            areas: vec![],
            lines: vec![],
            unassigned: vec![],
        },
        buildings: vec![],
        group_ranges: vec![GroupRange {
            id: GroupRangeId(1),
            source: source("gr1"),
            name: "Everything".into(),
            start: GroupAddress::from_raw(0),
            end: GroupAddress::from_raw(u16::MAX),
            parent: None,
            children: vec![],
        }],
        group_addresses: vec![],
        parameters: vec![],
    });

    let id = project.ids.next_group_address_id();
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id,
            source: source("t1"),
            name: ga_name.into(),
            address: GroupAddress::parse("1/1/1", project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(1)),
        });

    project
}

/// A project like [`project_with_ga_name`] (group address name held
/// constant) plus one device (individual address `1.1.1`, unassigned) with
/// one communication object numbered 1. `com_read_flag_present` toggles
/// that object's own `ReadFlag` between absent and present — the only
/// field this fixture ever varies, so the device's own fields
/// (`DeviceFields`) never differ between two calls while its nested
/// `com_objects` table does. This is the fixture finding 2 needs: a
/// `~ device ...` line whose own `changed_fields` is empty because only a
/// nested communication object changed.
fn project_with_device_com_object_read_flag(com_read_flag_present: bool) -> Project {
    let mut project = project_with_ga_name("Living Room Light");

    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source("D-1"),
        name: "Dimmer".into(),
        description: None,
        address: Some(IndividualAddress::new(1, 1, 1).unwrap()),
        product_ref: "P-0".into(),
        program_ref: "H-0".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![ComObjectInstanceId(1)],
        binary_data: vec![],
    });
    project.installations[0]
        .topology
        .unassigned
        .push(DeviceId(1));

    // `Layer::Instance` is deliberate, not `Layer::Program`: only
    // `Layer::Instance`/`Layer::UserEdit` are "exported"
    // (`knx_core::provenance::Layer::is_exported`), and `semantic_flag`
    // (`crates/knx-diff/src/semantic.rs`) collapses a non-exported layer's
    // value back to `None` — a `Program`-layer flag here would make both
    // sides look identical and defeat the whole fixture.
    let read = if com_read_flag_present {
        Override::Value(Resolved {
            value: true,
            layer: Layer::Instance,
        })
    } else {
        Override::Absent
    };
    project.devices.insert_com_object(ComObjectInstance {
        id: ComObjectInstanceId(1),
        source: source("CO-1"),
        device: DeviceId(1),
        number: 1,
        text: Override::Absent,
        description: Override::Absent,
        dpt: Override::Absent,
        flags: ResolvedFlags {
            read,
            write: Override::Absent,
            transmit: Override::Absent,
            update: Override::Absent,
            communication: Override::Absent,
            read_on_init: Override::Absent,
        },
        size: None,
        is_active: true,
        links: vec![],
        module_instance: None,
    });

    project
}

/// Writes a fresh `.knxdb` at `path` holding `project`.
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

fn write_knxproj_with_recoverable_duplicate_id(dir: &Path) -> std::path::PathBuf {
    use io::{Cursor, Read, Write};

    let source = knx_testsupport::minimal_knxproj_bytes();
    let mut archive = zip::ZipArchive::new(Cursor::new(source)).unwrap();
    let target = dir.join("duplicate-id.knxproj");
    let file = fs::File::create(&target).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).unwrap();
        let name = entry.name().to_string();
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        if name == "P-0001/0.xml" {
            let xml = String::from_utf8(bytes).unwrap();
            bytes = xml
                .replace(
                    "<GroupAddress Id=\"P-0001-0_GA-1\" Address=\"1\" Name=\"GA\" />",
                    "<GroupAddress Id=\"P-0001-0_GA-1\" Address=\"1\" Name=\"GA\" />\n<GroupAddress Id=\"P-0001-0_GA-1\" Address=\"2\" Name=\"Duplicate\" />",
                )
                .into_bytes();
        }
        writer.start_file(name, options).unwrap();
        writer.write_all(&bytes).unwrap();
    }
    writer.finish().unwrap();
    target
}

#[test]
fn diff_of_two_identical_stores_reports_no_changes() {
    let dir = tempfile::tempdir().unwrap();
    let store_a = dir.path().join("a.knxdb");
    let store_b = dir.path().join("b.knxdb");
    write_store(&store_a, &project_with_ga_name("Living Room Light"));
    write_store(&store_b, &project_with_ga_name("Living Room Light"));

    let out = run_cli(&["diff", store_a.to_str().unwrap(), store_b.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.to_lowercase().contains("no differences"), "{stdout}");
    assert!(
        !stdout.lines().any(|line| {
            let t = line.trim_start();
            t.starts_with(['+', '-', '~', '?'])
        }),
        "an identical comparison must print no change lines: {stdout}"
    );
}

#[test]
fn diff_of_two_stores_with_one_changed_group_address_name_prints_it() {
    let dir = tempfile::tempdir().unwrap();
    let store_a = dir.path().join("a.knxdb");
    let store_b = dir.path().join("b.knxdb");
    write_store(&store_a, &project_with_ga_name("Living Room Light"));
    write_store(&store_b, &project_with_ga_name("Living Room Light V2"));

    let out = run_cli(&["diff", store_a.to_str().unwrap(), store_b.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("1/1/1"), "{stdout}");
    assert!(
        stdout.lines().any(|line| line.trim_start()
            == "~ group address 1/1/1: name: Living Room Light -> Living Room Light V2"),
        "{stdout}"
    );
}

#[test]
fn diff_exit_code_mode_returns_zero_for_equal_one_for_different_and_two_for_input_errors() {
    let dir = tempfile::tempdir().unwrap();
    let store_a = dir.path().join("a.knxdb");
    let store_b = dir.path().join("b.knxdb");
    write_store(&store_a, &project_with_ga_name("A"));
    write_store(&store_b, &project_with_ga_name("A"));

    let equal = run_cli(&[
        "diff",
        "--exit-code",
        store_a.to_str().unwrap(),
        store_b.to_str().unwrap(),
    ]);
    assert_eq!(equal.status.code(), Some(0));

    write_store(&store_b, &project_with_ga_name("B"));
    let different = run_cli(&[
        "diff",
        "--exit-code",
        store_a.to_str().unwrap(),
        store_b.to_str().unwrap(),
    ]);
    assert_eq!(different.status.code(), Some(1));

    let missing = dir.path().join("missing.knxdb");
    let invalid = run_cli(&[
        "diff",
        "--exit-code",
        missing.to_str().unwrap(),
        store_b.to_str().unwrap(),
    ]);
    assert_eq!(invalid.status.code(), Some(2));
}

#[test]
fn diff_accepts_raw_knxproj_inputs_and_exposes_their_import_reports() {
    let dir = tempfile::tempdir().unwrap();
    let project = knx_testsupport::write_minimal_knxproj(dir.path());

    let output = run_cli(&[
        "diff",
        "--exit-code",
        project.to_str().unwrap(),
        project.to_str().unwrap(),
    ]);

    assert_eq!(
        output.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("comparison import report"),
        "raw ETS normalization must not hide its compatibility report"
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("no differences found"));
}

#[test]
fn diff_exit_code_mode_rejects_a_partial_import_with_error_diagnostics() {
    let dir = tempfile::tempdir().unwrap();
    let project = write_knxproj_with_recoverable_duplicate_id(dir.path());

    let output = run_cli(&[
        "diff",
        "--exit-code",
        project.to_str().unwrap(),
        project.to_str().unwrap(),
    ]);

    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("comparison import report"), "{stderr}");
    assert!(stderr.contains("error diagnostics"), "{stderr}");
}

#[test]
fn diff_with_a_missing_first_store_exits_one_and_prints_no_stray_knxdb() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("does-not-exist.knxdb");
    let store_b = dir.path().join("b.knxdb");
    write_store(&store_b, &project_with_ga_name("Living Room Light"));

    let out = run_cli(&["diff", missing.to_str().unwrap(), store_b.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !missing.exists(),
        "a failed diff must never create the store it could not find"
    );
}

/// A device whose own fields are identical between left and right must
/// still surface as `~ device ...: (own fields unchanged)` when only a
/// nested communication object differs (`print_device_table`,
/// `apps/knx-cli/src/main.rs`) — otherwise the change the domain layer
/// went out of its way to keep would be silently dropped at render time.
#[test]
fn diff_of_two_stores_with_only_a_com_object_flag_changed_prints_own_fields_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let store_a = dir.path().join("a.knxdb");
    let store_b = dir.path().join("b.knxdb");
    write_store(&store_a, &project_with_device_com_object_read_flag(false));
    write_store(&store_b, &project_with_device_com_object_read_flag(true));

    let out = run_cli(&["diff", store_a.to_str().unwrap(), store_b.to_str().unwrap()]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().map(str::trim_start).collect();
    let device_line_idx = lines
        .iter()
        .position(|line| *line == "~ device 1.1.1: (own fields unchanged)")
        .unwrap_or_else(|| panic!("expected the own-fields-unchanged device line: {stdout}"));
    assert!(
        lines[device_line_idx + 1..]
            .iter()
            .any(|line| line.starts_with("~ communication object 1: read")),
        "expected the changed communication object beneath the device line: {stdout}"
    );
}

#[test]
fn diff_with_wrong_argument_count_prints_usage_and_exits_one() {
    let out = run_cli(&["diff", "only-one-argument"]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("usage:"), "{stderr}");
}
