//! Drives the built `knx` binary with `std::process::Command` — this is
//! the CLI's own contract with a user, not something a library-level test
//! against `import_ets_project` directly can stand in for.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn reference_ets4_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .join("Unser Zuhause ets4 - 2025-12-15.knxproj")
}

fn run_cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .expect("failed to run the knx binary")
}

#[test]
fn the_cli_reports_counts_and_exits_zero() {
    let out = run_cli(&["import", reference_ets4_path().to_str().unwrap()]);
    assert_eq!(out.status.code(), Some(0));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("36 devices"));
    assert!(text.contains("514 group addresses"));
    assert!(text.contains("unsupported"));
}

#[test]
fn the_cli_writes_a_machine_readable_report() {
    let dir = tempfile::tempdir().unwrap();
    let report = dir.path().join("report.json");
    let out = run_cli(&[
        "import",
        reference_ets4_path().to_str().unwrap(),
        "--report-json",
        report.to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(0));
    let json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&report).unwrap()).unwrap();
    assert_eq!(json["source"]["schema_version"], 11);
    assert!(!json["unsupported"].as_array().unwrap().is_empty());
}

#[test]
fn the_cli_exits_nonzero_on_a_file_it_cannot_read() {
    let out = run_cli(&["import", "/nonexistent.knxproj"]);
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("nonexistent"));
}
