//! Shared commissioning lifecycle contracts without a transport, server or UI dependency.
use std::sync::Arc;

use knx_app::commissioning_activity::{DownloadResult, OneShotLog, Restart, WriteOutcome, Written};

fn log(path: &std::path::Path) -> Arc<OneShotLog> {
    Arc::new(OneShotLog::persistent(
        path.to_owned(),
        "synthetic-cli".into(),
    ))
}

#[test]
fn independent_run_namespaces_keep_prior_running_rows_uncertain() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let first = Arc::new(OneShotLog::for_run(path.clone()).unwrap());
    let second = Arc::new(OneShotLog::for_run(path.clone()).unwrap());
    assert!(
        !path.exists(),
        "constructing a run must not admit or create history"
    );
    let abandoned = first
        .start_write("serviceControlWrite", Some("1.1.67".into()))
        .unwrap();
    let current = second
        .start_write("serviceControlWrite", Some("1.1.67".into()))
        .unwrap();
    let (rows, _) = second.history_page(0, 100).unwrap();
    assert_eq!(rows.len(), 2);
    assert_ne!(rows[0].server_incarnation, rows[1].server_incarnation);
    assert!(rows[0].interrupted);
    assert_eq!(rows[0].activity.state, "unknown");
    assert!(!rows[1].interrupted);
    assert_eq!(rows[1].activity.state, "running");
    current.finish(WriteOutcome::NotSent);
    drop(abandoned);
}

#[test]
fn foreign_history_refuses_every_write_admission_and_preserves_input() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("foreign.sqlite");
    let original = b"synthetic foreign input, not a journal or recovery record";
    std::fs::write(&path, original).unwrap();
    let log = log(&path);
    assert!(log
        .start_write("serviceControlWrite", Some("1.1.67".into()))
        .is_err());
    assert!(log.start_download(7, "1.1.67".parse().unwrap()).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), original);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn a_volatile_log_never_admits_a_write() {
    let log = Arc::new(OneShotLog::default());
    assert!(log
        .start_write("serviceControlWrite", Some("1.1.67".into()))
        .is_err());
    assert!(log.start_download(7, "1.1.67".parse().unwrap()).is_err());
}

#[test]
fn durable_start_precedes_any_possible_send() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let guard = log
        .start_write("serviceControlWrite", Some("1.1.67".into()))
        .unwrap();
    let history = knx_store::activity_history::ActivityHistory::open_existing(&path).unwrap();
    let (rows, more) = history.page(0, 100).unwrap();
    assert!(!more);
    assert_eq!(rows.len(), 1);
    let value: serde_json::Value = serde_json::from_str(&rows[0].document).unwrap();
    assert_eq!(value["state"], "running");
    assert_eq!(value["writeEvidence"]["backupRecorded"], false);
    assert_eq!(value["writeEvidence"]["sendPossible"], false);
    guard.finish(WriteOutcome::NotSent);
}

#[test]
fn dropped_write_preserves_uncertainty_and_does_not_claim_not_sent() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let guard = log
        .start_write("serviceControlWrite", Some("1.1.67".into()))
        .unwrap();
    guard.mark_send_possible().unwrap();
    drop(guard);
    let (rows, _) = log.history_page(0, 100).unwrap();
    let value = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(value["state"], "unknown");
    assert_eq!(value["writeEvidence"]["sendPossible"], true);
    assert_eq!(value["writeEvidence"]["backupRecorded"], true);
    assert!(!value.to_string().contains("octets"));
}

#[test]
fn vanished_admitted_store_refuses_intent_without_claiming_a_send() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let guard = log
        .start_write("serviceControlWrite", Some("1.1.67".into()))
        .unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(guard.mark_send_possible().is_err());
    assert!(!guard.send_possible());
    assert_eq!(log.history_state(), "unavailable");
    assert!(!path.exists());
    drop(guard);
}

#[test]
fn download_result_restart_and_cleanup_remain_distinct() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let mut guard = log.start_download(7, "1.1.67".parse().unwrap()).unwrap();
    guard.mark_send_possible().unwrap();
    guard
        .record_result(&DownloadResult::Finished {
            written: Written::Yes,
            restart: Restart::Unconfirmed,
        })
        .unwrap();
    guard.record_cleanup(false).unwrap();
    let (rows, _) = log.history_page(0, 100).unwrap();
    let value = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(value["state"], "finished");
    assert_eq!(value["downloadEvidence"]["written"], "yes");
    assert_eq!(value["downloadEvidence"]["restart"], "unconfirmed");
    assert_eq!(value["downloadEvidence"]["cleanup"], "returnedError");
    assert_eq!(value["downloadEvidence"]["sessionId"], 7);
}
