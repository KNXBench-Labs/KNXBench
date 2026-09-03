//! Drives the built `knx` binary with `std::process::Command` — this is
//! the CLI's own contract with a user, not something a library-level test
//! against `import_ets_project` directly can stand in for.

use std::io::{Cursor, Write};
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

#[test]
fn a_flag_missing_its_value_is_a_usage_error_not_a_swallowed_flag() {
    // `--store` must not silently consume `--report-json` as its own value.
    let out = run_cli(&[
        "import",
        "--store",
        "--report-json",
        "out.json",
        reference_ets4_path().to_str().unwrap(),
    ]);
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(stderr.contains("--store"));
}

/// A structurally valid schema-11 project whose only defect is a duplicate
/// `GroupAddress/@Id` — validation reports it as an error, but the import
/// itself still produces a project. Written by hand rather than derived
/// from the reference project, so the CLI's exit-code contract is tested
/// against a known, minimal defect.
const DUPLICATE_ID_TOPOLOGY: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11" CreatedBy="ETS4" ToolVersion="ETS 4.1.8">
  <Project Id="P-0001">
    <Installations>
      <Installation InstallationId="0" Name="" CompletionStatus="Undefined">
        <Topology />
        <GroupAddresses>
          <GroupRanges>
            <GroupRange Id="P-0001-0_GR-1" Name="Licht" RangeStart="1" RangeEnd="255">
              <GroupAddress Id="P-0001-0_GA-1" Address="1" Name="GA" />
              <GroupAddress Id="P-0001-0_GA-1" Address="2" Name="GA2" />
            </GroupRange>
          </GroupRanges>
        </GroupAddresses>
      </Installation>
    </Installations>
  </Project>
</KNX>"#;

const PROJECT_INFO: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<KNX xmlns="http://knx.org/xml/project/11">
  <Project Id="P-0001">
    <ProjectInformation Name="T" GroupAddressStyle="ThreeLevel" CompletionStatus="Undefined" />
  </Project>
</KNX>"#;

fn write_knxproj_with_duplicate_id(path: &Path) {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options =
        zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
    for (name, bytes) in [
        ("P-0001.signature", "x"),
        ("P-0001/0.xml", DUPLICATE_ID_TOPOLOGY),
        ("P-0001/Project.xml", PROJECT_INFO),
    ] {
        writer.start_file(name, options).unwrap();
        writer.write_all(bytes.as_bytes()).unwrap();
    }
    std::fs::write(path, writer.finish().unwrap().into_inner()).unwrap();
}

#[test]
fn a_report_with_real_errors_exits_two_not_zero() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("broken.knxproj");
    write_knxproj_with_duplicate_id(&file);

    let out = run_cli(&["import", file.to_str().unwrap()]);

    // 2, not 1: the project did import — a caller can tell "imported with
    // known holes" from "could not import at all".
    assert_eq!(out.status.code(), Some(2));
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.contains("1 error(s)"), "stdout was: {text}");
}
