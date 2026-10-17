//! Real built CLI, using only its explicit synthetic transport.
use std::process::Command;
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
