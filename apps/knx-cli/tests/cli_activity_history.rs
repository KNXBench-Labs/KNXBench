//! CLI activity admission and plan-only refusal boundaries against a loopback fake.

#[cfg(unix)]
#[test]
fn existing_history_aliases_are_refused_without_changing_the_nonempty_store() {
    use knx_app::commissioning_activity::{OneShotLog, WriteOutcome};
    use std::sync::Arc;

    for caller in ["download", "restore"] {
        for input_kind in ["primary", "products", "key"] {
            for alias in ["same", "symlink", "hardlink"] {
                let dir = tempfile::tempdir().unwrap();
                let history = dir.path().join("history.sqlite");
                let log = Arc::new(OneShotLog::for_run(history.clone()).unwrap());
                log.start_write("serviceControlWrite", Some("1.1.67".into()))
                    .unwrap()
                    .finish(WriteOutcome::NotSent);
                let (rows, _) = log.history_page(0, 100).unwrap();
                assert_eq!(rows.len(), 1, "seed must contain a durable original row");
                let before_rows = serde_json::to_value(rows).unwrap();
                let before_bytes = std::fs::read(&history).unwrap();
                let input = if alias == "same" {
                    history.clone()
                } else {
                    let input = dir.path().join("input");
                    if alias == "symlink" {
                        std::os::unix::fs::symlink(&history, &input).unwrap();
                    } else {
                        std::fs::hard_link(&history, &input).unwrap();
                    }
                    input
                };
                let primary = if input_kind == "primary" {
                    input.clone()
                } else {
                    let primary = dir.path().join("primary-input");
                    std::fs::write(&primary, b"synthetic input must remain unopened").unwrap();
                    primary
                };
                let primary_bytes = std::fs::read(&primary).unwrap();
                let unused_products = dir.path().join("unused-products.sqlite");
                let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
                gateway
                    .set_read_timeout(Some(Duration::from_millis(150)))
                    .unwrap();
                let phrase =
                    required_confirmation_phrase("1.1.67".parse().unwrap(), WriteScope::Download);
                let mut cli = Command::new(env!("CARGO_BIN_EXE_knx"));
                cli.args(["device", caller]);
                if caller == "download" {
                    cli.args(["1.1.67", "--project"]);
                }
                cli.arg(&primary)
                    .arg("--product-db")
                    .arg(if input_kind == "products" {
                        &input
                    } else {
                        &unused_products
                    });
                if input_kind == "key" {
                    cli.arg("--key-file").arg(&input);
                }
                let output = cli
                    .args(["--activity-history"])
                    .arg(&history)
                    .args([
                        "--gateway",
                        &gateway.local_addr().unwrap().to_string(),
                        "--confirm",
                        &phrase,
                    ])
                    .output()
                    .unwrap();
                assert!(!output.status.success());
                assert_eq!(
                    std::fs::read(&history).unwrap(),
                    before_bytes,
                    "{caller}/{input_kind}/{alias} changed an aliased nonempty history input"
                );
                assert_eq!(
                    std::fs::read(&primary).unwrap(),
                    primary_bytes,
                    "{caller}/{input_kind}/{alias} touched the primary before admission"
                );
                assert!(
                    !unused_products.exists(),
                    "an unused product input was created"
                );
                let (after_rows, _) = log.history_page(0, 100).unwrap();
                assert_eq!(serde_json::to_value(after_rows).unwrap(), before_rows);
                let stderr = String::from_utf8(output.stderr).unwrap();
                assert!(
                stderr.contains("activity history")
                    && stderr.contains("input")
                    && stderr.contains("not sent"),
                "{caller}/{input_kind}/{alias} did not refuse the history/input alias before reading: {stderr}"
            );
                let mut packet = [0; 64];
                let error = gateway
                    .recv_from(&mut packet)
                    .expect_err("aliased input reached the connector");
                assert!(matches!(
                    error.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ));
            }
        }
    }
}

#[test]
fn usage_names_the_required_history_for_each_confirmed_write_caller() {
    let output = Command::new(env!("CARGO_BIN_EXE_knx")).output().unwrap();
    assert!(!output.status.success());
    let usage = String::from_utf8(output.stderr).unwrap();
    for caller in ["download", "restore", "service-control"] {
        let prefix = format!("{caller} ");
        let section = usage
            .split("knx device ")
            .find(|section| section.starts_with(&prefix))
            .expect("caller is documented in usage");
        assert!(
            section.contains("--activity-history <path>")
                && section.contains("required for confirmed writes"),
            "{caller} usage hides its mandatory persistent history"
        );
    }
}

#[test]
fn download_write_requires_persistent_history_before_project_io_or_connector() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("missing.knxdb");
    let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
    gateway
        .set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let phrase = required_confirmation_phrase("1.1.67".parse().unwrap(), WriteScope::Download);
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "download", "1.1.67", "--project"])
        .arg(&project)
        .args([
            "--confirm",
            &phrase,
            "--gateway",
            &gateway.local_addr().unwrap().to_string(),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("activity history") && stderr.contains("not sent"),
        "missing persistent history was not refused first: {stderr}"
    );
    assert!(!project.exists(), "a missing input project was created");
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    let mut packet = [0; 64];
    let error = gateway
        .recv_from(&mut packet)
        .expect_err("connector ran without persistent history");
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
}
use std::net::UdpSocket;
use std::process::Command;
use std::time::Duration;

use knx_core::commissioning::mutation::{required_confirmation_phrase, WriteScope};

#[test]
fn history_admission_never_creates_a_missing_download_or_restore_input() {
    for command in ["download", "restore"] {
        let dir = tempfile::tempdir().unwrap();
        let input = dir.path().join("missing-input");
        let products = dir.path().join("unused-products.sqlite");
        let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
        gateway
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        let phrase = required_confirmation_phrase("1.1.67".parse().unwrap(), WriteScope::Download);
        let mut cli = Command::new(env!("CARGO_BIN_EXE_knx"));
        cli.args(["device", command]);
        if command == "download" {
            cli.args(["1.1.67", "--project"]);
        }
        let output = cli
            .arg(&input)
            .args([
                "--gateway",
                &gateway.local_addr().unwrap().to_string(),
                "--confirm",
                &phrase,
                "--activity-history",
            ])
            .arg(&input)
            .arg("--product-db")
            .arg(&products)
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(
            !input.exists(),
            "{command} history admission created its missing input"
        );
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
        let mut packet = [0; 64];
        let error = gateway
            .recv_from(&mut packet)
            .expect_err("connector ran for a missing input");
        assert!(matches!(
            error.kind(),
            std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
        ));
    }
}

#[test]
fn restore_write_requires_history_before_backup_io_or_connector() {
    let dir = tempfile::tempdir().unwrap();
    let backup = dir.path().join("missing-backup.json");
    let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
    gateway
        .set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let phrase = required_confirmation_phrase("1.1.67".parse().unwrap(), WriteScope::Download);
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "restore"])
        .arg(&backup)
        .args([
            "--gateway",
            &gateway.local_addr().unwrap().to_string(),
            "--confirm",
            &phrase,
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("activity history") && stderr.contains("not sent"),
        "missing restore history was not refused before backup IO: {stderr}"
    );
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 0);
    let mut packet = [0; 64];
    let error = gateway
        .recv_from(&mut packet)
        .expect_err("restore connector ran without history");
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
}

#[test]
fn restore_foreign_history_refuses_before_backup_io_and_connector() {
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("foreign.sqlite");
    let backup = dir.path().join("missing-backup.json");
    let original = b"synthetic foreign restore history";
    std::fs::write(&history, original).unwrap();
    let input_bytes = b"synthetic input must remain unparsed and unchanged";
    std::fs::write(&backup, input_bytes).unwrap();
    let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
    gateway
        .set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let phrase = required_confirmation_phrase("1.1.67".parse().unwrap(), WriteScope::Download);
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "restore"])
        .arg(&backup)
        .args([
            "--gateway",
            &gateway.local_addr().unwrap().to_string(),
            "--confirm",
            &phrase,
            "--activity-history",
        ])
        .arg(&history)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("activity history") && stderr.contains("not sent"),
        "foreign restore history was not refused before backup IO: {stderr}"
    );
    assert_eq!(std::fs::read(&history).unwrap(), original);
    assert_eq!(std::fs::read(&backup).unwrap(), input_bytes);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    let mut packet = [0; 64];
    let error = gateway
        .recv_from(&mut packet)
        .expect_err("restore connector ran before history admission");
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
}

#[test]
fn download_foreign_history_refuses_before_project_io_and_connector() {
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("foreign.sqlite");
    let project = dir.path().join("missing.knxdb");
    let original = b"synthetic foreign download history";
    std::fs::write(&history, original).unwrap();
    let input_bytes = b"synthetic input must remain unparsed and unchanged";
    std::fs::write(&project, input_bytes).unwrap();
    let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
    gateway
        .set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let phrase = required_confirmation_phrase("1.1.67".parse().unwrap(), WriteScope::Download);
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args(["device", "download", "1.1.67", "--project"])
        .arg(&project)
        .args([
            "--gateway",
            &gateway.local_addr().unwrap().to_string(),
            "--confirm",
            &phrase,
            "--activity-history",
        ])
        .arg(&history)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("activity history") && stderr.contains("not sent"),
        "foreign download history was not refused first: {stderr}"
    );
    assert_eq!(std::fs::read(&history).unwrap(), original);
    assert_eq!(std::fs::read(&project).unwrap(), input_bytes);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    let mut packet = [0; 64];
    let error = gateway
        .recv_from(&mut packet)
        .expect_err("connector ran before history admission");
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
}

#[test]
fn service_control_foreign_history_refuses_before_connector_and_preserves_input() {
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("foreign.sqlite");
    let original = b"synthetic foreign history, never recovery data";
    std::fs::write(&history, original).unwrap();
    let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
    gateway
        .set_read_timeout(Some(Duration::from_millis(150)))
        .unwrap();
    let target = "1.1.67".parse().unwrap();
    let phrase = required_confirmation_phrase(target, WriteScope::IndividualAddressWriteEnable);
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "service-control",
            "1.1.67",
            "--enable",
            "--confirm",
            &phrase,
            "--gateway",
            &gateway.local_addr().unwrap().to_string(),
            "--activity-history",
        ])
        .arg(&history)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(
        stderr.contains("activity history") && stderr.contains("not sent"),
        "write did not reach explicit history-admission refusal: {stderr}"
    );
    assert_eq!(std::fs::read(&history).unwrap(), original);
    let mut packet = [0; 64];
    let error = gateway
        .recv_from(&mut packet)
        .expect_err("connector sent before history admission");
    assert!(matches!(
        error.kind(),
        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
    ));
    assert!(!dir.path().join("foreign.sqlite-journal").exists());
    assert!(!dir.path().join("foreign.sqlite-wal").exists());
    assert!(!dir.path().join("foreign.sqlite-shm").exists());
}

#[test]
fn service_control_plan_does_not_open_or_modify_configured_history() {
    let dir = tempfile::tempdir().unwrap();
    let history = dir.path().join("foreign.sqlite");
    let original = b"synthetic foreign history untouched by plan";
    std::fs::write(&history, original).unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_knx"))
        .args([
            "device",
            "service-control",
            "1.1.67",
            "--enable",
            "--activity-history",
        ])
        .arg(&history)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "plan-only command unexpectedly refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("plan (nothing sent yet)"));
    assert_eq!(std::fs::read(&history).unwrap(), original);
    assert!(!dir.path().join("foreign.sqlite-journal").exists());
    assert!(!dir.path().join("foreign.sqlite-wal").exists());
    assert!(!dir.path().join("foreign.sqlite-shm").exists());
}
