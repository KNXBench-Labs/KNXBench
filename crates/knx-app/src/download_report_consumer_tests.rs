//! Pins that a report file reads a post-download check exactly like the other consumers do.

use super::tests::report;
use super::*;
use crate::post_download_check::RuntimeState;
use serde_json::{json, Value};

const FIXTURES: &str = include_str!("../../../apps/knx-web/src/postDownloadCheck.fixtures.json");
const CASES: &str = include_str!("../../../apps/knx-web/src/postDownloadCheck.cases.json");

/// A finished report file whose check field carries `check` verbatim.
fn file_with(check: Value) -> Value {
    let mut file: Value = serde_json::from_str(
        &report()
            .with_outcome(ReportedOutcome::completed(
                Written::Yes,
                Restart::Acknowledged,
            ))
            .to_json(),
    )
    .unwrap();
    file["postDownloadCheck"] = check;
    file
}

/// One shared-table case: the named fixture with its `[pointer, value]` edits applied.
fn edited(fixtures: &Value, case: &Value) -> Value {
    let mut value = fixtures[case["fixture"].as_str().unwrap()].clone();
    for edit in case["set"].as_array().unwrap() {
        let (parent, key) = edit[0].as_str().unwrap().rsplit_once('/').unwrap();
        match value.pointer_mut(parent).unwrap() {
            Value::Object(object) => {
                object.insert(key.into(), edit[1].clone());
            }
            Value::Array(array) => array[key.parse::<usize>().unwrap()] = edit[1].clone(),
            _ => panic!("case {}: no parent", case["name"]),
        }
    }
    value
}

/// The report file is a third reader of the shared table: same verdicts as the check itself.
#[test]
fn a_report_file_admits_and_refuses_the_shared_case_table_like_the_check_does() {
    let fixtures: Value = serde_json::from_str(FIXTURES).unwrap();
    let cases: Value = serde_json::from_str(CASES).unwrap();
    for case in cases["accepted"].as_array().unwrap() {
        let check = edited(&fixtures, case);
        let parsed: DownloadReport = serde_json::from_value(file_with(check.clone()))
            .unwrap_or_else(|error| panic!("case {}: {error}", case["name"]));
        let stored = serde_json::to_value(parsed.post_download_check.unwrap()).unwrap();
        assert_eq!(stored, check, "case {}", case["name"]);
    }
    for case in cases["refused"].as_array().unwrap() {
        let check = edited(&fixtures, case);
        // A null check is the one refused value a report reads as absent; see the next test.
        if check.is_null() {
            continue;
        }
        assert!(
            serde_json::from_value::<DownloadReport>(file_with(check)).is_err(),
            "case {} was admitted by the report file",
            case["name"]
        );
    }
}

/// Characterisation: a report's `null` equals an absent check (not checked, never proof).
#[test]
fn a_null_or_absent_check_in_a_report_file_reads_as_not_checked_and_prints_no_result() {
    let absent = {
        let mut file = file_with(Value::Null);
        file.as_object_mut().unwrap().remove("postDownloadCheck");
        file
    };
    for file in [file_with(Value::Null), absent] {
        let parsed: DownloadReport = serde_json::from_value(file).unwrap();
        assert!(parsed.post_download_check.is_none());
        assert!(!parsed
            .summary()
            .contains("Read-only check after the download"));
    }
    // An explicit not-checked record is stated, not hidden.
    let stated =
        file_with(json!({ "observation": { "status": "notChecked" }, "runtime": "unknown" }));
    let parsed: DownloadReport = serde_json::from_value(stated).unwrap();
    assert!(parsed
        .summary()
        .contains("Read-only check after the download: not checked"));
}

/// The report summary and the CLI lines state the same run-state facts for every fixture.
#[test]
fn the_report_summary_and_the_cli_lines_agree_on_every_fixture() {
    let fixtures: Value = serde_json::from_str(FIXTURES).unwrap();
    for (name, value) in fixtures.as_object().unwrap() {
        let check: PostDownloadCheck = serde_json::from_value(value.clone()).unwrap();
        let summary = report().with_check(check.clone()).summary();
        let lines = check.lines();
        let record = lines
            .iter()
            .find_map(|line| line.strip_prefix("  run-state read: "));
        match (check.runtime, record) {
            (RuntimeState::Unknown, Some(record)) => assert!(
                summary.contains(&format!(
                    "application running: unknown (no defined run state observed; run-state read: {record})"
                )),
                "{name}: {summary}"
            ),
            (RuntimeState::Unknown, None) => {
                assert!(!summary.contains("run-state read"), "{name}: {summary}");
                assert!(summary.contains("application running: unknown"), "{name}: {summary}");
            }
            (known, _) => {
                assert!(summary.contains(&format!("run state: {known} (")), "{name}: {summary}");
                assert!(lines.iter().any(|l| l.starts_with(&format!("run state: {known} ("))), "{name}");
                assert!(!summary.contains("application running: unknown"), "{name}: {summary}");
            }
        }
    }
}
