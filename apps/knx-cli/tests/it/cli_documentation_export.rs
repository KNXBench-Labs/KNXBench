//! Drives the built `knx` binary to test `doc-export` (T13 —
//! `docs/superpowers/specs/2026-09-10-project-documentation-export-design.md`).
//! Builds a tiny `.knxdb` directly through `knx-core`/`knx-store`, same
//! pattern `cli_group_address_csv.rs` uses. This subcommand exports
//! "project documentation", nothing more, nothing less — it makes no claim
//! of ETS-report parity, since no ETS-produced report sample exists
//! anywhere in this repository to be compatible with.

use std::path::Path;
use std::process::{Command, Output};

use knx_core::{
    CommissioningState, CompletionStatus, DeviceId, DeviceInstance, GroupAddress,
    GroupAddressEntry, GroupRange, GroupRangeId, Installation, InstallationId, Language, Project,
    SourceRef, Topology,
};

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

/// A project named "Test Villa" with one installation, one group range
/// spanning the whole address space (so the address never also triggers
/// the unrelated "not inside any group range" warning), one group address
/// inside it (`1/1/1`/"Living Room Light"), and no devices at all — the
/// baseline "nothing structurally odd" fixture (same shape as
/// `apps/knx-server/tests/http_documentation_export.rs`'s
/// `state_with_one_group_address`).
fn tiny_project() -> Project {
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

    let id = project.ids.next_group_address_id().unwrap();
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id,
            source: source("t1"),
            name: "Living Room Light".into(),
            address: GroupAddress::parse("1/1/1", project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: Some(GroupRangeId(1)),
            declared_dpt: Default::default(),
        });

    project
}

/// Same as [`tiny_project`], plus one device (`DeviceId(1)`) assigned to
/// no line (`Topology::unassigned`) — the structural oddity the
/// "every warning individually" test exercises.
fn project_with_a_device_in_no_line() -> Project {
    let mut project = tiny_project();
    project.installations[0].topology.unassigned = vec![DeviceId(1)];
    project.devices.insert(DeviceInstance {
        id: DeviceId(1),
        source: source("d1"),
        name: "Stray Device".into(),
        description: None,
        address: None,
        product_ref: "P".into(),
        program_ref: "H".into(),
        commissioning: CommissioningState::default(),
        visibility_calculated: true,
        com_objects: vec![],
        binary_data: vec![],
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

#[test]
fn doc_export_writes_a_self_contained_html_document_and_names_output_and_warning_count() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let html_path = dir.path().join("documentation.html");
    write_store(&store, &tiny_project());

    let out = run_cli(&[
        "doc-export",
        store.to_str().unwrap(),
        html_path.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "doc-export failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let text = std::fs::read_to_string(&html_path).unwrap();
    assert!(text.starts_with("<!DOCTYPE html>"), "{text}");
    assert!(text.contains("Test Villa"), "{text}");
    assert!(text.contains("1/1/1"), "{text}");

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(
        stdout.contains(html_path.to_str().unwrap()),
        "stdout must name the output path: {stdout}"
    );
    assert!(
        stdout.contains("0 warning(s)"),
        "stdout must name the warning count: {stdout}"
    );
}

#[test]
fn doc_export_prints_every_warning_individually_not_just_a_count() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let html_path = dir.path().join("documentation.html");
    write_store(&store, &project_with_a_device_in_no_line());

    let out = run_cli(&[
        "doc-export",
        store.to_str().unwrap(),
        html_path.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "doc-export failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("2 warning(s)"), "{stdout}");
    assert!(
        stdout.contains("device 1"),
        "each warning's location must be printed individually: {stdout}"
    );
    assert!(stdout.contains("product database"), "{stdout}");
}

#[test]
fn doc_export_of_a_missing_store_exits_1_and_writes_no_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("does-not-exist.knxdb");
    let html_path = dir.path().join("documentation.html");

    let out = run_cli(&[
        "doc-export",
        store.to_str().unwrap(),
        html_path.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !html_path.exists(),
        "a failed doc-export must not produce an output file"
    );
}

#[test]
fn doc_export_reports_the_actual_default_product_database_open_error() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let html_path = dir.path().join("documentation.html");
    let data_home = dir.path().join("data");
    let products_path = data_home.join("knx/products.sqlite");
    write_store(&store, &tiny_project());
    let products = knx_productdb::open_and_migrate(&products_path).unwrap();
    products
        .pragma_update(
            None,
            "user_version",
            knx_productdb::CURRENT_PRODUCTDB_VERSION + 1,
        )
        .unwrap();
    drop(products);

    let out = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "doc-export",
            store.to_str().unwrap(),
            html_path.to_str().unwrap(),
        ])
        .env("XDG_DATA_HOME", &data_home)
        .env_remove("HOME")
        .output()
        .expect("failed to run the knx binary");

    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        html_path.exists(),
        "the report still degrades to raw references"
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("failed to open product database"),
        "{stderr}"
    );
    // Derived from the build's own schema version: hard-coded numbers would
    // pass for the wrong reason after every ProductDB migration.
    let unsupported = knx_productdb::CURRENT_PRODUCTDB_VERSION + 1;
    let supported = knx_productdb::CURRENT_PRODUCTDB_VERSION;
    assert!(
        stderr.contains(&format!("product database is version {unsupported}")),
        "{stderr}"
    );
    assert!(
        stderr.contains(&format!("this build supports up to {supported}")),
        "{stderr}"
    );
}

#[test]
fn doc_export_with_wrong_argument_count_prints_usage_and_exits_1() {
    let out = run_cli(&["doc-export", "only-one-argument"]);
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
