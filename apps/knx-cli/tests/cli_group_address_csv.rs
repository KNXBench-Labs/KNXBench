//! Drives the built `knx` binary to test `ga-export`/`ga-import` (T12 —
//! `docs/superpowers/specs/2026-09-10-csv-group-address-exchange-design.md`
//! §7). Builds a tiny `.knxdb` directly through `knx-core`/`knx-store`
//! (same pattern `apps/knx-server/tests/http_group_address_csv.rs` uses)
//! rather than depending on the `OriginalData/` corpus — these subcommands
//! have nothing to do with `.knxproj` import.

use std::path::Path;
use std::process::{Command, Output};

use knx_core::{
    CompletionStatus, GroupAddress, GroupAddressEntry, Installation, InstallationId, Language,
    Project, SourceRef, Topology,
};

fn source(tag: &str) -> SourceRef {
    SourceRef {
        path: tag.into(),
        ets_id: tag.into(),
    }
}

/// A project with one installation and one group address (`1/1/1`/"Living
/// Room Light"), the project's default `GroupAddressStyle::ThreeLevel`
/// style, and no group ranges.
fn tiny_project() -> Project {
    let mut project = Project::new(Language("en".into()));
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
        group_ranges: vec![],
        group_addresses: vec![],
        parameters: vec![],
    });

    let id = project.ids.next_group_address_id();
    project.installations[0]
        .group_addresses
        .push(GroupAddressEntry {
            id,
            source: source("t1"),
            name: "Living Room Light".into(),
            address: GroupAddress::parse("1/1/1", project.info.group_address_style).unwrap(),
            central: false,
            unfiltered: false,
            range: None,
        });

    project
}

/// Writes a fresh `.knxdb` at `path` holding [`tiny_project`].
fn write_store(path: &Path) {
    let conn = knx_store::open_and_migrate(path).expect("open/migrate store");
    knx_store::save_project(&conn, &tiny_project()).expect("save tiny project");
}

/// Every group address currently in the store at `path`, as
/// `(address, name)` pairs in installation order — the cheapest way to
/// assert "the store is unchanged" without depending on internal ids.
fn store_addresses(path: &Path) -> Vec<(String, String)> {
    let conn = knx_store::open_and_migrate(path).expect("open/migrate store");
    let project = knx_store::load_project(&conn).expect("load project");
    project.installations[0]
        .group_addresses
        .iter()
        .map(|ga| {
            (
                ga.address.format(project.info.group_address_style),
                ga.name.clone(),
            )
        })
        .collect()
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("failed to run the knx binary")
}

/// Splits `ga-import` stdout into the report body (everything but the last
/// line, trailing newline kept) and the trailing `store written: ...` line
/// (no trailing newline) — the report body is what T12's byte-identical
/// dry-run/real-run guarantee covers; the trailing line is deliberately
/// outside that guarantee (review finding, task-5-review.md Ruling (a)).
fn split_trailing_status_line(stdout: &str) -> (&str, &str) {
    let trimmed = stdout.strip_suffix('\n').unwrap_or(stdout);
    let idx = trimmed.rfind('\n').map(|i| i + 1).unwrap_or(0);
    (&trimmed[..idx], &trimmed[idx..])
}

#[test]
fn ga_export_writes_the_header_and_one_data_row() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let csv_path = dir.path().join("export.csv");
    write_store(&store);

    let out = run_cli(&[
        "ga-export",
        store.to_str().unwrap(),
        csv_path.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(0),
        "ga-export failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let bytes = std::fs::read(&csv_path).unwrap();
    let text = String::from_utf8(bytes).unwrap();
    let text = text.strip_prefix('\u{FEFF}').unwrap_or(&text);
    let lines: Vec<&str> = text.trim_end_matches("\r\n").split("\r\n").collect();
    assert_eq!(
        lines[0],
        "Address,Name,Central,Unfiltered,DatapointType,MainGroup,MiddleGroup"
    );
    assert_eq!(lines[1], "1/1/1,Living Room Light,false,false,,,");
    assert_eq!(lines.len(), 2, "{lines:?}");
}

#[test]
fn ga_import_of_a_freshly_exported_file_reports_nothing_to_do() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    let csv_path = dir.path().join("export.csv");
    write_store(&store);

    let export_out = run_cli(&[
        "ga-export",
        store.to_str().unwrap(),
        csv_path.to_str().unwrap(),
    ]);
    assert_eq!(export_out.status.code(), Some(0));

    let import_out = run_cli(&[
        "ga-import",
        store.to_str().unwrap(),
        csv_path.to_str().unwrap(),
    ]);
    assert_eq!(
        import_out.status.code(),
        Some(0),
        "ga-import failed: {}",
        String::from_utf8_lossy(&import_out.stderr)
    );
    let stdout = String::from_utf8(import_out.stdout).unwrap();
    assert!(
        stdout.contains("1 row(s) read, 0 created, 0 updated, 1 unchanged"),
        "{stdout}"
    );
    assert!(stdout.contains("nothing to do"), "{stdout}");
    let (_, status_line) = split_trailing_status_line(&stdout);
    assert_eq!(status_line, "store written: no (nothing to do)", "{stdout}");

    // Re-importing an unchanged file must leave the store exactly as it
    // was.
    assert_eq!(
        store_addresses(&store),
        vec![("1/1/1".to_string(), "Living Room Light".to_string())]
    );
}

#[test]
fn ga_import_of_a_bad_row_exits_2_and_leaves_the_store_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let store = dir.path().join("project.knxdb");
    write_store(&store);
    let before = store_addresses(&store);

    let bad_path = dir.path().join("bad.csv");
    // Row 2 (line 3, counting the header) has no name at all — a row-level
    // error (design §4), so the whole file must be rejected.
    std::fs::write(&bad_path, "Address,Name\n1/1/1,Still Fine\n1/1/9,\n").unwrap();

    let out = run_cli(&[
        "ga-import",
        store.to_str().unwrap(),
        bad_path.to_str().unwrap(),
    ]);
    assert_eq!(
        out.status.code(),
        Some(2),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8(out.stdout).unwrap();
    assert!(stdout.contains("error: row 3"), "{stdout}");
    let (_, status_line) = split_trailing_status_line(&stdout);
    assert_eq!(status_line, "store written: no (rejected)", "{stdout}");

    assert_eq!(
        store_addresses(&store),
        before,
        "a rejected import must leave the store exactly as it was"
    );
}

#[test]
fn ga_import_dry_run_matches_the_real_imports_report_and_leaves_the_store_untouched() {
    // One shared directory and one shared input file (identical path,
    // identical bytes) for both runs — only `parsed.input` (the CSV path)
    // ever reaches the printed report, so keeping it identical between the
    // two invocations is what makes "byte-identical output" a meaningful
    // assertion rather than a coincidence of two different temp paths.
    let dir = tempfile::tempdir().unwrap();
    let edits_csv = dir.path().join("edits.csv");
    // Renames the existing address and adds a new one — exercises both
    // "updated" and "created" so the report has real content to compare,
    // not just zeros.
    std::fs::write(
        &edits_csv,
        "Address,Name\n1/1/1,Living Room Light (renamed)\n1/1/9,Hallway Light\n",
    )
    .unwrap();

    // Two separately-seeded but identical stores: one for the real run,
    // one for `--dry-run`.
    let real_store = dir.path().join("real.knxdb");
    write_store(&real_store);
    let dry_store = dir.path().join("dry.knxdb");
    write_store(&dry_store);
    let dry_before = store_addresses(&dry_store);

    let real_out = run_cli(&[
        "ga-import",
        real_store.to_str().unwrap(),
        edits_csv.to_str().unwrap(),
    ]);
    assert_eq!(
        real_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&real_out.stderr)
    );
    let real_stdout = String::from_utf8(real_out.stdout).unwrap();
    let real_addresses_after = store_addresses(&real_store);
    assert_eq!(
        real_addresses_after.len(),
        2,
        "the real run must have applied the create and the update: {real_addresses_after:?}"
    );
    let (_, real_status_line) = split_trailing_status_line(&real_stdout);
    assert_eq!(
        real_status_line, "store written: yes",
        "a real import that actually wrote must say so: {real_stdout}"
    );

    let dry_out = run_cli(&[
        "ga-import",
        dry_store.to_str().unwrap(),
        edits_csv.to_str().unwrap(),
        "--dry-run",
    ]);
    assert_eq!(
        dry_out.status.code(),
        Some(0),
        "stderr: {}",
        String::from_utf8_lossy(&dry_out.stderr)
    );
    let dry_stdout = String::from_utf8(dry_out.stdout).unwrap();
    let (dry_body, dry_status_line) = split_trailing_status_line(&dry_stdout);
    let (real_body, _) = split_trailing_status_line(&real_stdout);

    assert_eq!(
        dry_body, real_body,
        "--dry-run must print a byte-identical report body to a real import"
    );
    assert_eq!(
        dry_status_line, "store written: no (dry run)",
        "a dry run must never claim to have written the store: {dry_stdout}"
    );
    assert_eq!(
        store_addresses(&dry_store),
        dry_before,
        "--dry-run must leave the store untouched"
    );
}
