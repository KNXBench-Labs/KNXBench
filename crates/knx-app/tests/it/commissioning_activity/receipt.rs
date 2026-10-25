//! The run-scoped download receipts through the public writer, the strict reader and tampering.
use std::path::Path;
use std::sync::Arc;

use knx_app::commissioning_activity::{
    CurrentRequest, DownloadPreparation, DownloadResult, DownloadSettlement, Format1, OneShotLog,
    ProgramReceipt, ProjectOutcome, Restart, Scope, SourceVerdict, Written,
};
use knx_app::programming_state::Settlement;
use knx_core::commissioning::pending::ProgrammingParts;
use knx_store::activity_history::ActivityHistory;
use serde_json::{json, Value};

const SHA: &str = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
const AT: &str = "2026-10-25T07:08:09.123Z";

fn log(path: &Path) -> Arc<OneShotLog> {
    Arc::new(OneShotLog::persistent(
        path.to_owned(),
        "synthetic-cli".into(),
    ))
}

fn preparation(scope: Scope) -> DownloadPreparation {
    DownloadPreparation::Available {
        format: Format1,
        scope,
        program: ProgramReceipt::Identified {
            id: "M-0001_A-0001-01-0000".into(),
            application_version: Some("1".into()),
            source_sha256: SHA.into(),
        },
        planned_mask: "0701".into(),
        planned_manufacturer: "0001".into(),
        steps: 12,
        data_octets: 34,
    }
}

fn at() -> chrono::DateTime<chrono::Utc> {
    AT.parse().unwrap()
}

fn settlement(
    outcome: ProjectOutcome,
    source: SourceVerdict,
    request: CurrentRequest,
    programmed: ProgrammingParts,
    pending: ProgrammingParts,
) -> DownloadSettlement {
    DownloadSettlement::computed(
        outcome,
        source,
        request,
        &Settlement {
            programmed,
            pending,
            touched: programmed.union(pending),
        },
        at(),
    )
}

fn complete_settlement() -> DownloadSettlement {
    settlement(
        ProjectOutcome::Persisted,
        SourceVerdict::Unchanged,
        CurrentRequest::Readable,
        ProgrammingParts::ALL,
        ProgrammingParts::NONE,
    )
}

fn parameters() -> ProgrammingParts {
    ProgrammingParts {
        parameters: true,
        ..ProgrammingParts::NONE
    }
}

fn finished() -> DownloadResult {
    DownloadResult::Finished {
        written: Written::Yes,
        restart: Restart::Acknowledged,
    }
}

/// The stored document of the only row, exactly as the writer serialised it.
fn stored(path: &Path) -> Value {
    let (rows, _) = ActivityHistory::open_existing(path)
        .unwrap()
        .page(0, 10)
        .unwrap();
    assert_eq!(rows.len(), 1);
    serde_json::from_str(&rows[0].document).unwrap()
}

fn run(
    scope: Scope,
    result: Option<DownloadResult>,
    settlement: Option<DownloadSettlement>,
) -> (tempfile::TempDir, std::path::PathBuf, Value) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let mut guard = log
        .start_download_prepared(7, "1.1.67".parse().unwrap(), preparation(scope))
        .unwrap();
    guard.mark_send_possible().unwrap();
    if let Some(result) = result {
        guard.record_result(&result).unwrap();
        if let Some(settlement) = settlement {
            guard.record_settlement(&settlement).unwrap();
        }
        guard.record_cleanup(true).unwrap();
        let document = stored(&path);
        return (dir, path, document);
    }
    // Still running: the row as the first persist wrote it, then dropped.
    let document = stored(&path);
    drop(guard);
    (dir, path, document)
}

#[test]
fn both_receipts_round_trip_through_the_writer_and_the_strict_reader_with_exact_fields() {
    let (_dir, path, document) = run(
        Scope::Complete,
        Some(finished()),
        Some(complete_settlement()),
    );
    let evidence = &document["downloadEvidence"];
    assert_eq!(
        evidence["preparation"],
        json!({
            "state": "available", "format": 1, "scope": "complete",
            "program": {"state": "identified", "id": "M-0001_A-0001-01-0000",
                        "applicationVersion": "1", "sourceSha256": SHA},
            "plannedMask": "0701", "plannedManufacturer": "0001",
            "steps": 12, "dataOctets": 34
        })
    );
    let all =
        json!({"address": true, "application": true, "parameters": true, "groupAddresses": true});
    let none = json!({"address": false, "application": false, "parameters": false, "groupAddresses": false});
    assert_eq!(
        evidence["settlement"],
        json!({
            "project": "persisted", "format": 1, "source": "unchanged",
            "currentRequest": "readable", "settledAt": AT,
            "applied": {"programmed": all, "pending": none, "touched": all}
        })
    );
    assert_eq!(evidence["written"], "yes");
    assert_eq!(evidence["cleanup"], "returnedOk");
    // A fresh run reads it back unchanged through the public reader.
    let reader = Arc::new(OneShotLog::for_run(path).unwrap());
    let (rows, _) = reader.history_page(0, 10).unwrap();
    let read = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(
        read["downloadEvidence"]["preparation"],
        evidence["preparation"]
    );
    assert_eq!(
        read["downloadEvidence"]["settlement"],
        evidence["settlement"]
    );
}

#[test]
fn every_project_outcome_and_a_failed_persist_are_distinct_on_the_wire() {
    let parts = (ProgrammingParts::ALL, ProgrammingParts::NONE);
    let failed = settlement(
        ProjectOutcome::PersistenceFailed,
        SourceVerdict::Unchanged,
        CurrentRequest::Readable,
        parts.0,
        parts.1,
    );
    let (_d, _p, document) = run(Scope::Complete, Some(finished()), Some(failed));
    let stored = &document["downloadEvidence"]["settlement"];
    assert_eq!(stored["project"], "persistenceFailed");
    assert!(stored.get("settledAt").is_none() && stored.get("applied").is_none());
    assert!(stored["computed"]["touched"]["address"] == true);
    let session = settlement(
        ProjectOutcome::SessionOnly,
        SourceVerdict::ChangedOrUnknown,
        CurrentRequest::Unavailable,
        ProgrammingParts::NONE,
        ProgrammingParts {
            address: false,
            ..ProgrammingParts::ALL
        },
    );
    let session = match session {
        DownloadSettlement::SessionOnly { .. } => session,
        other => panic!("unexpected {other:?}"),
    };
    // Touched must be ALL for a completed complete scope: this one is not.
    let (dir, path) = (tempfile::tempdir().unwrap(), "h.sqlite");
    let log = log(&dir.path().join(path));
    let mut guard = log
        .start_download_prepared(7, "1.1.67".parse().unwrap(), preparation(Scope::Complete))
        .unwrap();
    guard.mark_send_possible().unwrap();
    guard.record_result(&finished()).unwrap();
    assert!(guard.record_settlement(&session).is_err());
}

#[test]
fn the_legacy_start_and_a_dropped_prepared_run_read_as_absent_and_prepared_only() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let mut legacy = log.start_download(7, "1.1.67".parse().unwrap()).unwrap();
    legacy.mark_send_possible().unwrap();
    legacy.record_result(&finished()).unwrap();
    assert!(
        legacy.record_settlement(&complete_settlement()).is_err(),
        "a settlement needs the preparation it settles"
    );
    legacy.record_cleanup(true).unwrap();
    let document = stored(&path);
    let evidence = document["downloadEvidence"].as_object().unwrap();
    assert!(!evidence.contains_key("preparation") && !evidence.contains_key("settlement"));
    let (_dir, _path, dropped) = run(Scope::Complete, None, None);
    assert_eq!(dropped["state"], "running");
    assert!(dropped["downloadEvidence"].get("preparation").is_some());
    assert!(dropped["downloadEvidence"].get("settlement").is_none());
}

#[test]
fn a_settlement_is_refused_while_running_twice_and_against_its_row() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let mut guard = log
        .start_download_prepared(7, "1.1.67".parse().unwrap(), preparation(Scope::Complete))
        .unwrap();
    guard.mark_send_possible().unwrap();
    let before = stored(&path);
    assert!(
        guard.record_settlement(&complete_settlement()).is_err(),
        "running"
    );
    assert_eq!(stored(&path), before);
    guard.record_result(&finished()).unwrap();
    let not_attempted = DownloadSettlement::NotAttempted { format: Format1 };
    let interrupted = settlement(
        ProjectOutcome::Persisted,
        SourceVerdict::NotChecked,
        CurrentRequest::Readable,
        ProgrammingParts::NONE,
        parameters(),
    );
    let unchanged_with_loss = settlement(
        ProjectOutcome::Persisted,
        SourceVerdict::Unchanged,
        CurrentRequest::Unavailable,
        ProgrammingParts::ALL,
        ProgrammingParts::NONE,
    );
    for wrong in [&not_attempted, &interrupted, &unchanged_with_loss] {
        assert!(guard.record_settlement(wrong).is_err(), "{wrong:?}");
    }
    guard.record_settlement(&complete_settlement()).unwrap();
    assert!(
        guard.record_settlement(&complete_settlement()).is_err(),
        "twice"
    );
    guard.record_cleanup(true).unwrap();
    assert_eq!(
        stored(&path)["downloadEvidence"]["settlement"]["project"],
        "persisted"
    );
}

#[test]
fn a_run_that_wrote_nothing_takes_only_not_attempted() {
    let result = DownloadResult::Failed {
        written: Written::No,
    };
    let (_d, _p, document) = run(
        Scope::Complete,
        Some(result),
        Some(DownloadSettlement::NotAttempted { format: Format1 }),
    );
    assert_eq!(
        document["downloadEvidence"]["settlement"],
        json!({"project": "notAttempted", "format": 1})
    );
    let dir = tempfile::tempdir().unwrap();
    let log = log(&dir.path().join("h.sqlite"));
    let mut guard = log
        .start_download_prepared(7, "1.1.67".parse().unwrap(), preparation(Scope::Complete))
        .unwrap();
    guard.mark_send_possible().unwrap();
    guard.record_result(&result).unwrap();
    for wrong in [
        DownloadSettlement::NoProject { format: Format1 },
        complete_settlement(),
    ] {
        assert!(guard.record_settlement(&wrong).is_err(), "{wrong:?}");
    }
}

#[test]
fn a_failed_history_never_gains_a_settlement_and_the_witness_stays_in_memory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let mut guard = log
        .start_download_prepared(7, "1.1.67".parse().unwrap(), preparation(Scope::Complete))
        .unwrap();
    guard.mark_send_possible().unwrap();
    std::fs::remove_file(&path).unwrap();
    assert!(guard.record_result(&finished()).is_err(), "latches");
    assert_eq!(log.history_state(), "unavailable");
    assert!(guard.record_settlement(&complete_settlement()).is_err());
    assert!(!path.exists(), "nothing is written after a latch");
}

// ---- tampering ----------------------------------------------------------

/// Every file of the directory, name and bytes, for before/after equality.
fn files(dir: &Path) -> Vec<(String, Vec<u8>)> {
    let mut all: Vec<_> = std::fs::read_dir(dir)
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                std::fs::read(entry.path()).unwrap(),
            )
        })
        .collect();
    all.sort();
    all
}

fn plant(document: &Value) -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let history = ActivityHistory::open(&path).unwrap();
    history
        .record("synthetic-cli", 1, &document.to_string())
        .unwrap();
    drop(history);
    (dir, path)
}

const FIXTURES: &str =
    include_str!("../../../../../apps/knx-web/src/downloadReceipt.fixtures.json");
const CASES: &str = include_str!("../../../../../apps/knx-web/src/downloadReceipt.cases.json");

/// One case: its fixture document with the listed edits, as the web reader's table applies them.
fn edited(fixtures: &Value, case: &Value) -> Value {
    let mut value = fixtures[case["fixture"].as_str().unwrap()].clone();
    assert!(value.is_object(), "fixture {}", case["fixture"]);
    for edit in case["set"].as_array().unwrap() {
        let pointer = edit[0].as_str().unwrap();
        if pointer.is_empty() {
            value = edit[1].clone();
            continue;
        }
        let (parent, key) = pointer.rsplit_once('/').unwrap();
        value
            .pointer_mut(parent)
            .unwrap_or_else(|| panic!("case {}: no parent for {pointer}", case["name"]))
            .as_object_mut()
            .unwrap()
            .insert(key.into(), edit[1].clone());
    }
    for pointer in case["remove"].as_array().unwrap() {
        let (parent, key) = pointer.as_str().unwrap().rsplit_once('/').unwrap();
        let parent = if parent.is_empty() {
            &mut value
        } else {
            value.pointer_mut(parent).unwrap()
        };
        parent
            .as_object_mut()
            .unwrap()
            .remove(key)
            .unwrap_or_else(|| panic!("case {}: no {pointer}", case["name"]));
    }
    value
}

fn assert_refused(name: &str, document: &Value) {
    let (dir, path) = plant(document);
    let before = files(dir.path());
    let log = log(&path);
    assert!(
        log.history_page(0, 10).is_err(),
        "{name}: the page must fail"
    );
    assert_eq!(log.history_state(), "unavailable", "{name}");
    assert!(
        log.start_download(8, "1.1.67".parse().unwrap()).is_err(),
        "{name}: a latched history admits no write"
    );
    assert_eq!(files(dir.path()), before, "{name}: bytes must not change");
}

#[test]
fn the_shared_table_is_admitted_or_refused_exactly_and_refusal_keeps_the_bytes() {
    let fixtures: Value = serde_json::from_str(FIXTURES).unwrap();
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let accepted = cases["accepted"].as_array().unwrap();
    let refused = cases["refused"].as_array().unwrap();
    assert!(accepted.len() >= fixtures.as_object().unwrap().len());
    assert!(
        refused.len() >= 60,
        "the table is the contract: {}",
        refused.len()
    );
    for case in accepted {
        let name = case["name"].as_str().unwrap();
        let document = edited(&fixtures, case);
        let (_dir, path) = plant(&document);
        let (rows, _) = log(&path)
            .history_page(0, 10)
            .unwrap_or_else(|_| panic!("{name}: must be admitted"));
        // Re-serialised unchanged: the fixture is the serializer's vocabulary.
        let mut read = serde_json::to_value(&rows[0]).unwrap();
        for added in ["sequence", "serverIncarnation", "interrupted"] {
            read.as_object_mut().unwrap().remove(added);
        }
        assert_eq!(read, document, "{name}");
    }
    for case in refused {
        assert_refused(case["name"].as_str().unwrap(), &edited(&fixtures, case));
    }
}
