//! Drives the built `knx` binary to test `knx diff` (T14 —
//! `docs/superpowers/specs/2026-09-10-project-diff-design.md`). Builds two
//! tiny `.knxdb` files directly through `knx-core`/`knx-store`, same pattern
//! `cli_documentation_export.rs` uses. This subcommand renders "what changed
//! between two KNXBench project files", never an ETS comparison — no
//! ETS-produced comparison sample exists anywhere in this repository to be
//! compatible with.

use std::path::Path;
use std::process::{Command, Output};

use knx_core::{
    CompletionStatus, GroupAddress, GroupAddressEntry, GroupRange, GroupRangeId, Installation,
    InstallationId, Language, Project, SourceRef, Topology,
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

// `knx-diff`'s generic entity tables render a `changed` row as its
// formatted key plus a `changed_fields` *name* summary, never the old/new
// values themselves (design spec §5: "one changed_fields summary per
// changed line"; `knx-diff::diff.rs`'s `EntityChange::changed_fields` is
// `Vec<&'static str>`, field names only — the values live in
// `EntityChange::left`/`right`, which this renderer does not walk field by
// field for a generic entity). So this asserts the group address's
// formatted address, the changed field's name, and the `~` prefix — not
// the old/new name text, which this render level never prints.
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
        stdout
            .lines()
            .any(|line| line.trim_start() == "~ group address 1/1/1: name"),
        "{stdout}"
    );
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
