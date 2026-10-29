//! DIAG-02 failure diagnostic through the history writer, the strict reader and shared cases.
use std::path::Path;
use std::sync::Arc;

use knx_app::commissioning_activity::{DownloadResult, OneShotLog, Restart, Written};
use knx_app::download_failure::{
    code_token, DownloadFailure, FailureCategory, FailureCode, FailurePhase, FailureRestart,
};
use knx_store::activity_history::ActivityHistory;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../apps/knx-web/src/downloadFailure.cases.json");
const WEB_READER: &str = include_str!("../../../../../apps/knx-web/src/downloadFailure.ts");

fn log(path: &Path) -> Arc<OneShotLog> {
    Arc::new(OneShotLog::persistent(
        path.to_owned(),
        "synthetic-cli".into(),
    ))
}

fn stored(path: &Path) -> Value {
    let (rows, _) = ActivityHistory::open_existing(path)
        .unwrap()
        .page(0, 10)
        .unwrap();
    assert_eq!(rows.len(), 1);
    serde_json::from_str(&rows[0].document).unwrap()
}

fn base_failure() -> DownloadFailure {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    serde_json::from_value(cases["base"]["downloadEvidence"]["failure"].clone()).unwrap()
}

#[test]
fn a_recorded_failure_round_trips_through_the_writer_and_the_strict_reader() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let mut guard = log.start_download(7, "1.1.67".parse().unwrap()).unwrap();
    guard.mark_send_possible().unwrap();
    let failure = base_failure();
    guard.record_failure(Written::Partially, &failure).unwrap();
    guard.record_cleanup(true).unwrap();
    let document = stored(&path);
    assert_eq!(document["state"], "failed");
    assert_eq!(document["downloadEvidence"]["restart"], Value::Null);
    assert_eq!(
        document["downloadEvidence"]["failure"],
        serde_json::to_value(&failure).unwrap()
    );
    let (rows, _) = log.history_page(0, 10).unwrap();
    let row = serde_json::to_value(&rows[0]).unwrap();
    assert_eq!(row["downloadEvidence"], document["downloadEvidence"]);
}

#[test]
fn an_inconsistent_or_late_failure_is_refused_and_leaves_the_row_unchanged() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let mut guard = log.start_download(7, "1.1.67".parse().unwrap()).unwrap();
    guard.mark_send_possible().unwrap();
    let before = stored(&path);
    let mut wrong = base_failure();
    wrong.category = FailureCategory::Internal;
    assert!(guard.record_failure(Written::Partially, &wrong).is_err());
    let mut stop = base_failure();
    stop.code = FailureCode::Stopped;
    stop.category = FailureCategory::Stopped;
    stop.phase = FailurePhase::StoppedBeforeStep;
    assert!(
        guard.record_failure(Written::Partially, &stop).is_err(),
        "a stop lands before the first change"
    );
    assert_eq!(stored(&path), before, "nothing was written");
    // A recorded result cannot be followed by a failure.
    guard
        .record_result(&DownloadResult::Finished {
            written: Written::Yes,
            restart: Restart::Acknowledged,
        })
        .unwrap();
    assert!(guard
        .record_failure(Written::Partially, &base_failure())
        .is_err());
    assert_eq!(stored(&path)["state"], "finished");
    assert!(stored(&path)["downloadEvidence"].get("failure").is_none());
}

fn edited(base: &Value, case: &Value) -> Value {
    let mut value = base.clone();
    for edit in case["set"].as_array().unwrap() {
        let pointer = edit[0].as_str().unwrap();
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
        value
            .pointer_mut(parent)
            .unwrap()
            .as_object_mut()
            .unwrap()
            .remove(key)
            .expect("removed key exists");
    }
    value
}

#[test]
fn the_shared_cases_are_admitted_exactly_as_the_web_reader_admits_them() {
    let cases: Value = serde_json::from_str(CASES).unwrap();
    let all = cases["cases"].as_array().unwrap();
    assert!(all.len() >= 20);
    for case in all {
        let document = edited(&cases["base"], case);
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        let history = ActivityHistory::open(&path).unwrap();
        history
            .record("synthetic-cli", 1, &document.to_string())
            .unwrap();
        drop(history);
        let log = log(&path);
        let admitted = log.history_page(0, 10).is_ok();
        assert_eq!(
            admitted,
            case["admitted"].as_bool().unwrap(),
            "{}: {document}",
            case["name"]
        );
    }
}

#[test]
fn the_web_reader_maps_every_code_to_the_same_category() {
    use FailureCode as F;
    let all = [
        F::DoesNotConnectFirst,
        F::CheckAfterWrite,
        F::StopRefused,
        F::Excluded,
        F::SelfAddressed,
        F::NoAuthorisation,
        F::NotASimulator,
        F::AuthorisationRefused,
        F::LoadNotMemoryMapped,
        F::VerifyModeWithMemoryLoad,
        F::MaskMismatch,
        F::ManufacturerMismatch,
        F::PropertyMismatch,
        F::BackupRead,
        F::BackupShortRead,
        F::BackupNotKept,
        F::NoAnswer,
        F::TransitionTimedOut,
        F::Transport,
        F::NotConnected,
        F::ConnectionLost,
        F::ConnectRejected,
        F::ConnectionReleased,
        F::PropertyRefused,
        F::MemoryRefused,
        F::ReadBackMismatch,
        F::PropertyReadBackMismatch,
        F::NotLoaded,
        F::UnexpectedLoadState,
        F::UnknownLoadState,
        F::LoadErrorCode,
        F::LoadStateUnchanged,
        F::IllegalLoadTransition,
        F::AllocationFailed,
        F::MemoryLoadNotSettled,
        F::MemoryLoadIllegalTransition,
        F::MalformedProperty,
        F::MalformedDescriptor,
        F::MalformedRestartResponse,
        F::MalformedMemoryLoadState,
        F::Encode,
        F::Chunk,
        F::Lagged,
        F::Stopped,
        F::TaskEnded,
    ];
    let table = &WEB_READER[WEB_READER.find("FAILURE_CODES = {").unwrap()..];
    let table = &table[..table.find("} as const").unwrap()];
    assert_eq!(
        table.matches(": \"").count(),
        all.len(),
        "no extra web code"
    );
    for code in all {
        let category = serde_json::to_value(code.category()).unwrap();
        let line = format!("  {}: {},", code_token(code), category);
        assert!(table.contains(&line), "web reader lacks `{line}`");
    }
    // The other closed vocabularies, by their serialized tokens.
    for token in [
        FailurePhase::BeforeSteps,
        FailurePhase::InStep,
        FailurePhase::BackupBeforeStep,
        FailurePhase::StoppedBeforeStep,
    ]
    .map(|phase| serde_json::to_value(phase).unwrap())
    .into_iter()
    .chain(
        [
            FailureRestart::NotInPlan,
            FailureRestart::NotSent,
            FailureRestart::Unknown,
        ]
        .map(|restart| serde_json::to_value(restart).unwrap()),
    ) {
        assert!(WEB_READER.contains(&token.to_string()), "{token}");
    }
}

/// The worker's fallback: a refused diagnostic still lets the row end failed.
#[test]
fn a_refused_failure_leaves_the_row_open_for_a_plain_failed_result() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("history.sqlite");
    let log = log(&path);
    let mut guard = log.start_download(7, "1.1.67".parse().unwrap()).unwrap();
    guard.mark_send_possible().unwrap();
    let mut wrong = base_failure();
    wrong.read_back_octets = usize::MAX;
    assert!(guard.record_failure(Written::Partially, &wrong).is_err());
    guard
        .record_result(&DownloadResult::Failed {
            written: Written::Partially,
        })
        .unwrap();
    let document = stored(&path);
    assert_eq!(document["state"], "failed");
    assert!(document["downloadEvidence"].get("failure").is_none());
}

/// The writer and the strict reader the server's history route also uses (`history_page`).
#[test]
fn exemplar_failures_in_the_backup_phase_are_recorded_and_read_back_with_their_cause() {
    for (code, category) in [
        (
            FailureCode::PropertyMismatch,
            FailureCategory::IdentityMismatch,
        ),
        (
            FailureCode::MalformedProperty,
            FailureCategory::MalformedResponse,
        ),
    ] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("history.sqlite");
        let log = log(&path);
        let mut guard = log.start_download(7, "1.1.67".parse().unwrap()).unwrap();
        let mut failure = base_failure();
        failure.code = code;
        failure.category = category;
        failure.phase = FailurePhase::BackupBeforeStep;
        guard
            .record_failure(Written::No, &failure)
            .expect("admitted");
        let (rows, _) = log.history_page(0, 10).unwrap();
        let row = serde_json::to_value(&rows[0]).unwrap();
        assert_eq!(
            row["downloadEvidence"]["failure"],
            serde_json::to_value(&failure).unwrap()
        );
        // Anything read back before the first change, or any other code, stays refused.
        let mut late = failure.clone();
        late.read_back_octets = 1;
        assert!(!late.is_consistent());
        let mut other = failure.clone();
        other.code = FailureCode::MaskMismatch;
        other.category = FailureCategory::IdentityMismatch;
        assert!(!other.is_consistent());
    }
}
