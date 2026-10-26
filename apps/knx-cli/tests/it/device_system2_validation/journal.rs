//! The gated command's activity journal: a refusal before any send is `notSent`, not a panic.
use std::{net::UdpSocket, path::PathBuf, sync::Arc};

use knx_app::commissioning_activity::OneShotLog;

use super::{bundle, cli};

#[test]
fn unreachable_gateway_is_journalled_not_sent_without_a_panic() {
    let dir = tempfile::tempdir().unwrap();
    let (path, _) = bundle(dir.path());
    let preview = cli()
        .args(["--bundle", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(preview.status.success());
    let preview = String::from_utf8(preview.stdout).unwrap();
    let phrase = |label: &str| {
        preview
            .lines()
            .find_map(|line| line.strip_prefix(label))
            .unwrap_or_else(|| panic!("{label} missing from preview"))
            .to_owned()
    };
    // Bound and released: nothing answers on this loopback port.
    let gateway = UdpSocket::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .to_string();
    let operation = dir.path().join("operation");
    let history: PathBuf = dir.path().join("activity.sqlite");
    let out = cli()
        .env("XDG_RUNTIME_DIR", dir.path().join("runtime"))
        .env("XDG_DATA_HOME", dir.path().join("data"))
        .env("XDG_CONFIG_HOME", dir.path().join("config"))
        .args([
            "--bundle",
            path.to_str().unwrap(),
            "--gateway",
            &gateway,
            "--operation",
            operation.to_str().unwrap(),
            "--activity-history",
            history.to_str().unwrap(),
            "--confirm",
            &phrase("Confirmation: "),
            "--accept-hypothesis",
            &phrase("Hypothesis acceptance: "),
            "--download-confirm",
            &phrase("Ordinary download: "),
            "--release-confirm",
            &phrase("Best-effort release: "),
        ])
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert_eq!(out.status.code(), Some(1), "{stderr}");
    assert!(!stderr.contains("panicked"), "{stderr}");
    assert!(stderr.contains("gateway"), "{stderr}");
    assert!(!operation.exists(), "no recovery store before a tunnel");

    let log = Arc::new(OneShotLog::for_run(history).unwrap());
    let (rows, more) = log.history_page(0, 10).unwrap();
    assert!(!more);
    assert_eq!(rows.len(), 1);
    let row = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(row["kind"], "system2Validation");
    assert_eq!(row["address"], "1.1.25");
    assert_eq!(row["state"], "notSent");
    assert_eq!(row["interrupted"], false);
    assert!(row["finishedAt"].is_string());
    assert_eq!(
        row["writeEvidence"],
        serde_json::json!({"backupRecorded": false, "sendPossible": false})
    );
}
