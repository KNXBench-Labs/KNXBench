//! Regression tests for consented community evidence.
use std::{fs, process::Command};
fn knx(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .unwrap()
}
#[test]
fn real_cli_analyzes_offline_and_exports_without_overwriting() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.knxproj");
    fs::write(&input, knx_testsupport::minimal_knxproj_bytes()).unwrap();
    let result = knx(&["contribution", "analyze", input.to_str().unwrap()]);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&result.stdout).unwrap();
    assert_eq!(report["kind"], "project");
    assert!(report["checks"]
        .as_array()
        .unwrap()
        .iter()
        .any(|c| c["name"] == "hardware" && c["status"] == "not-examined"));
    let opts = dir.path().join("options.json");
    fs::write(
        &opts,
        r#"{"audience":"public","sampleIds":[],"consent":true}"#,
    )
    .unwrap();
    let out = dir.path().join("evidence.zip");
    let args = [
        "contribution",
        "export",
        input.to_str().unwrap(),
        out.to_str().unwrap(),
        "--options",
        opts.to_str().unwrap(),
    ];
    let result = knx(&args);
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    let original = fs::read(&out).unwrap();
    assert!(original.starts_with(b"PK"));
    let result = knx(&args);
    assert!(!result.status.success());
    assert_eq!(fs::read(&out).unwrap(), original);
}
#[test]
fn real_cli_refuses_bad_inputs_before_creating_output() {
    let dir = tempfile::tempdir().unwrap();
    let input = dir.path().join("source.knxprod");
    fs::write(&input, b"not a ZIP").unwrap();
    let opts = dir.path().join("options.json");
    fs::write(&opts, r#"{"audience":"public","consent":true}"#).unwrap();
    let out = dir.path().join("evidence.zip");
    assert!(!knx(&[
        "contribution",
        "export",
        input.to_str().unwrap(),
        out.to_str().unwrap(),
        "--options",
        opts.to_str().unwrap()
    ])
    .status
    .success());
    assert!(!out.exists());
}
