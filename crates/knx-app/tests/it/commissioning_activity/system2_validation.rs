//! System-2 validation write activity: durable lifecycle, reopen and refused inconsistent rows.
use std::path::Path;
use std::sync::Arc;

use knx_app::commissioning_activity::{OneShotLog, WriteOutcome};
use knx_store::activity_history::ActivityHistory;
use serde_json::{json, Value};

/// The kind the gated `knx device system2-validation` hardware path records.
const KIND: &str = "system2Validation";
const INCARNATION: &str = "AAAAAAAAAAAAAAAAAAAAAA";

fn stored(path: &Path) -> Vec<Value> {
    let history = ActivityHistory::open_existing(path).unwrap();
    let (rows, more) = history.page(0, 100).unwrap();
    assert!(!more);
    rows.iter()
        .map(|row| serde_json::from_str(&row.document).unwrap())
        .collect()
}

#[test]
fn system2_validation_write_lifecycle_is_durable_and_survives_a_reopen() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = Arc::new(OneShotLog::for_run(path.clone()).unwrap());
    log.ensure_write_available().unwrap();

    // Durable start precedes any possible send.
    let verified = log.start_write(KIND, Some("1.1.1".into())).unwrap();
    let first = stored(&path);
    assert_eq!(first.len(), 1);
    assert_eq!(first[0]["kind"], KIND);
    assert_eq!(first[0]["state"], "running");
    assert_eq!(
        first[0]["writeEvidence"],
        json!({"backupRecorded": false, "sendPossible": false})
    );
    verified.mark_send_possible().unwrap();
    assert_eq!(stored(&path)[0]["writeEvidence"]["sendPossible"], true);
    verified.finish(WriteOutcome::Verified);

    let unverified = log.start_write(KIND, Some("1.1.2".into())).unwrap();
    unverified.mark_send_possible().unwrap();
    unverified.finish(WriteOutcome::EffectUnverified);

    let refused = log.start_write(KIND, Some("1.1.3".into())).unwrap();
    refused.finish(WriteOutcome::NotSent);

    let dropped = log.start_write(KIND, Some("1.1.4".into())).unwrap();
    dropped.mark_send_possible().unwrap();
    drop(dropped);

    let in_flight = log.start_write(KIND, Some("1.1.5".into())).unwrap();
    assert_eq!(log.history_state(), "configured");

    // A fresh run reopens the persisted rows through the strict reader.
    let reopened = Arc::new(OneShotLog::for_run(path.clone()).unwrap());
    let (rows, more) = reopened.history_page(0, 100).unwrap();
    assert!(!more);
    let rows: Vec<Value> = rows
        .iter()
        .map(|r| serde_json::to_value(r).unwrap())
        .collect();
    let expected = [
        ("1.1.1", "verified", true, false),
        ("1.1.2", "effectUnverified", true, false),
        ("1.1.3", "notSent", false, false),
        ("1.1.4", "unknown", true, false),
        ("1.1.5", "unknown", false, true),
    ];
    assert_eq!(rows.len(), expected.len());
    for (row, (address, state, send, interrupted)) in rows.iter().zip(expected) {
        assert_eq!(row["kind"], KIND, "{address}");
        assert_eq!(row["address"], address);
        assert_eq!(row["state"], state, "{address}");
        assert_eq!(row["writeEvidence"]["sendPossible"], send, "{address}");
        assert_eq!(row["writeEvidence"]["backupRecorded"], send, "{address}");
        assert_eq!(row["interrupted"], interrupted, "{address}");
        assert_eq!(row["finishedAt"].is_null(), interrupted, "{address}");
        assert!(row.get("downloadEvidence").is_none());
    }
    assert_eq!(reopened.history_state(), "configured");
    reopened.ensure_write_available().unwrap();
    // The stored in-flight row is projected, not rewritten.
    assert_eq!(stored(&path)[4]["state"], "running");
    in_flight.finish(WriteOutcome::NotSent);
    assert_eq!(stored(&path)[4]["state"], "notSent");
}

#[test]
fn inconsistent_system2_validation_rows_are_still_refused() {
    let valid = json!({
        "id": 1, "kind": KIND, "address": "1.1.1", "state": "verified",
        "startedAt": "2026-10-26T00:00:00Z", "finishedAt": "2026-10-26T00:00:01Z",
        "writeEvidence": {"backupRecorded": true, "sendPossible": true},
    });
    let mut cases = vec![("consistent", valid.clone(), true)];
    let mut unsent_verified = valid.clone();
    unsent_verified["writeEvidence"] = json!({"backupRecorded": true, "sendPossible": false});
    cases.push(("verified without send", unsent_verified, false));
    let mut sent_not_sent = valid.clone();
    sent_not_sent["state"] = "notSent".into();
    cases.push(("notSent after send", sent_not_sent, false));
    let mut send_without_backup = valid.clone();
    send_without_backup["state"] = "running".into();
    send_without_backup["finishedAt"] = Value::Null;
    send_without_backup["writeEvidence"]["backupRecorded"] = false.into();
    cases.push(("send without backup", send_without_backup, false));
    let mut read_state = valid.clone();
    read_state["state"] = "finished".into();
    cases.push(("read-only state", read_state, false));
    let mut missing_evidence = valid.clone();
    missing_evidence
        .as_object_mut()
        .unwrap()
        .remove("writeEvidence");
    cases.push(("missing evidence", missing_evidence, false));
    let mut no_address = valid.clone();
    no_address["address"] = Value::Null;
    cases.push(("no address", no_address, false));
    let mut download = valid.clone();
    download["downloadEvidence"] = json!({"sessionId": 1, "written": "yes", "restart": "acknowledged", "cleanup": "returnedOk"});
    cases.push(("download evidence", download, false));
    let mut near_kind = valid.clone();
    near_kind["kind"] = "system2validation".into();
    cases.push(("unknown kind", near_kind, false));
    for (name, document, admitted) in cases {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        ActivityHistory::open(&path)
            .unwrap()
            .record(INCARNATION, 1, &document.to_string())
            .unwrap();
        let before = std::fs::read(&path).unwrap();
        let log = Arc::new(OneShotLog::persistent(path.clone(), INCARNATION.into()));
        assert_eq!(log.history_page(0, 10).is_ok(), admitted, "{name}");
        assert_eq!(log.ensure_write_available().is_ok(), admitted, "{name}");
        if !admitted {
            assert_eq!(log.history_state(), "unavailable", "{name}");
            assert!(
                log.start_write(KIND, Some("1.1.1".into())).is_err(),
                "{name}"
            );
            assert_eq!(std::fs::read(&path).unwrap(), before, "{name}");
        }
    }
}
