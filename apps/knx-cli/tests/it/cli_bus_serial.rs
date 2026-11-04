//! Drive real CLI serial selection, offline preflight and missing-path errors without hardware.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(args)
        .output()
        .unwrap()
}
fn text(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn serial_dry_run_validates_and_encodes_without_opening_a_path() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent-serial");
    let path = path.to_str().unwrap();
    for extra in [vec![], vec!["--dpt", "DPST-1-1"]] {
        let mut args = vec![
            "bus",
            "write",
            "--serial",
            path,
            "--source",
            "1.1.1",
            "--dry-run",
        ];
        args.extend(extra);
        args.extend(["1/1/1", "1"]);
        let result = run(&args);
        assert!(result.status.success(), "{}", text(&result));
        assert!(String::from_utf8_lossy(&result.stdout).contains("0x01"));
        assert!(!std::path::Path::new(path).exists());
    }
}

#[test]
fn both_commands_reject_conflicting_duplicate_and_invalid_selection_before_open() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonexistent-serial");
    let path = path.to_str().unwrap();
    let excluded = knx_core::EXCLUDED_INDIVIDUAL_ADDRESSES[0].to_string();
    let cases = [
        (vec!["--serial", path], "requires --source"),
        (vec!["--serial", path, "--source", "0.0.0"], "nonzero"),
        (
            vec!["--serial", path, "--source", "15.15.255"],
            "non-broadcast",
        ),
        (
            vec!["--serial", path, "--source", "16.1.1"],
            "area.line.device",
        ),
        (vec!["--serial", path, "--source", &excluded], "exclusion"),
        (
            vec![
                "--serial",
                path,
                "--source",
                "1.1.1",
                "--gateway",
                "192.0.2.1:3671",
            ],
            "mutually exclusive",
        ),
        (
            vec!["--serial", path, "--serial", path, "--source", "1.1.1"],
            "duplicate --serial",
        ),
        (
            vec!["--serial", path, "--source", "1.1.1", "--source", "1.1.2"],
            "duplicate --source",
        ),
        (
            vec!["--gateway", "192.0.2.1:3671", "--gateway", "192.0.2.2:3671"],
            "duplicate --gateway",
        ),
        (
            vec!["--gateway", "192.0.2.1:3671", "--source", "1.1.1"],
            "only valid with --serial",
        ),
        (vec!["--gateway", "192.0.2.1:0"], "nonzero port"),
        (vec!["--gateway", "0.0.0.0:3671"], "unicast"),
    ];
    for command in ["monitor", "write"] {
        for (flags, expected) in &cases {
            let mut args = vec!["bus", command];
            args.extend(flags.iter().copied());
            if command == "write" {
                args.extend(["1/1/1", "1"]);
            }
            let output = run(&args);
            let error = text(&output);
            assert!(!output.status.success(), "{args:?}");
            assert!(error.contains(expected), "{args:?}: {error}");
            assert!(
                !error.contains("I/O error"),
                "validation must precede opening: {error}"
            );
            assert!(!error.contains("panicked"));
        }
    }
}

#[test]
fn serial_bad_group_payload_and_extended_frames_are_refused_before_open() {
    for (ga, value, dry_run, expected) in [
        ("32/0/0", "1", false, "invalid group address"),
        ("1/1/1", "gg", false, "invalid hex value"),
        ("1/1/1", "€€", false, "value must be"),
        ("1/1/1", "000102030405060708090a0b0c0d0e", false, "extended"),
        ("1/1/1", "000102030405060708090a0b0c0d0e", true, "extended"),
    ] {
        let mut args = vec![
            "bus",
            "write",
            "--serial",
            "nonexistent-serial-test",
            "--source",
            "1.1.1",
            ga,
            value,
        ];
        if dry_run {
            args.push("--dry-run");
        }
        let output = run(&args);
        let error = text(&output);
        assert!(!output.status.success());
        assert!(error.contains(expected), "{args:?}: {error}");
        assert!(!error.contains("I/O error"));
        assert!(!error.contains("panicked"));
    }
}

#[test]
fn missing_serial_path_returns_failure_not_panic_for_both_commands() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("absent");
    for command in ["monitor", "write"] {
        let mut args = vec![
            "bus",
            command,
            "--serial",
            path.to_str().unwrap(),
            "--source",
            "1.1.1",
        ];
        if command == "write" {
            args.extend(["1/1/1", "1"]);
        }
        let output = run(&args);
        let error = text(&output);
        assert!(!output.status.success());
        assert!(error.contains("could not connect to serial"), "{error}");
        assert!(!error.contains("panicked"));
    }
}

#[test]
fn help_exposes_firmware_mode_and_serial_boundary() {
    let output = run(&["--help"]);
    let all = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        text(&output)
    );
    for phrase in [
        "--serial <tty>",
        "--source <area.line.device>",
        "19200 8E1",
        "transparent NCN5130",
        "monitor is passive",
        "ALL groups",
        "modem lines",
    ] {
        assert!(all.contains(phrase), "missing {phrase}: {all}");
    }
}
