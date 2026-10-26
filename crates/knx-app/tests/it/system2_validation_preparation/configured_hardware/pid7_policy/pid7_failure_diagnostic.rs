//! Typed PID 7 failure facts from a real refusal through journal, reopen, progress and report.
use super::*;
use knx_app::property_validation::{
    FailureDiagnostic, FailureDiagnosticRun as R, PointerFailureReason as Why,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn diag(run: R, reason: Why, object: u8, elements: u8, octets: u32) -> FailureDiagnostic {
    FailureDiagnostic {
        run,
        reason,
        object,
        property_id: 7,
        elements,
        octets,
    }
}
/// Every checkpoint's envelope format and its checkpoint value, in sequence order.
fn checkpoints(run: &Run) -> Vec<(u64, Value)> {
    run.journal_bytes()
        .into_iter()
        .filter(|(name, _)| name.starts_with("checkpoint-"))
        .map(|(_, bytes)| {
            let value: Value = serde_json::from_slice(&bytes).unwrap();
            (
                value["format"].as_u64().unwrap(),
                value["checkpoint"].clone(),
            )
        })
        .collect()
}
/// Public projections of a reopened store: progress list and the shareable report value.
fn public(run: &Run) -> (Vec<FailureDiagnostic>, Value) {
    let store = run.store();
    let report = serde_json::to_value(store.shareable_report()).unwrap();
    (store.progress().failure_diagnostics, report)
}
fn assert_public_clean(report: &Value, run: &Run, raw: &str) {
    let text = report.to_string();
    for forbidden in [
        raw,
        "local_diagnostic",
        "localDiagnostic",
        "pointer_failure",
        "resource pointer response",
        "checkpoint-",
        "HEAD",
        run.operation.to_str().unwrap(),
        run.bundle.binding.source_sha256.as_str(),
        run.bundle.binding.plan_sha256.as_str(),
        run.bundle.restore_phrase().as_str(),
    ] {
        assert!(!text.contains(forbidden), "public report leaks {forbidden}");
    }
}
/// Characterizes the real create-new writer on a reopened store: its bytes equal the
/// in-memory report, leak nothing, and a second write refuses without touching them.
fn written_report(run: &Run, name: &str, expected: &Value, raws: &[&str]) {
    let store = run.store();
    let path = run.operation.with_file_name(name);
    store.write_shareable_report(&path).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    assert_eq!(&serde_json::from_slice::<Value>(&bytes).unwrap(), expected);
    for raw in raws {
        assert_public_clean(expected, run, raw);
        assert!(!String::from_utf8_lossy(&bytes).contains(raw));
    }
    assert!(
        store.write_shareable_report(&path).is_err(),
        "create-new only"
    );
    assert_eq!(
        std::fs::read(&path).unwrap(),
        bytes,
        "first report unchanged"
    );
}
/// An object with at least three PID 7 reads (prestate, in procedure, final), found by a
/// clean valid-control run: its second read follows a mutation.
async fn in_procedure_object() -> u8 {
    let clean = execute(Policy::Width, None).await;
    finished(&clean);
    let (diagnostics, report) = public(&clean);
    assert!(diagnostics.is_empty(), "valid control has no diagnosis");
    assert_eq!(report["format"], 2);
    assert_eq!(report["failure_diagnostics"], json!([]));
    assert!(checkpoints(&clean).iter().all(|(format, _)| *format == 1));
    clean
        .totals
        .iter()
        .find(|(_, total)| **total >= 3)
        .map(|(object, _)| *object)
        .expect("an object with prestate, in-procedure and final reads")
}

#[tokio::test]
async fn three_closed_reasons_reach_the_reopened_journal_progress_and_report() {
    // Distinct object, count and length per reason; raw markers must stay private.
    let cases: [(u8, u8, &[u8], Why, &str); 3] = [
        (
            2,
            0,
            &[0xAB, 0xCD, 0xEF],
            Why::ZeroElementsWithData,
            "abcdef",
        ),
        (
            3,
            3,
            &[0xC0, 0xFF, 0xEE, 0, 0, 0x5A],
            Why::ElementCount,
            "c0ffee00005a",
        ),
        (
            1,
            1,
            &[0xDE, 0xAD, 0xBE, 0xEF, 0x77],
            Why::UnsupportedWidth,
            "deadbeef77",
        ),
    ];
    for (object, elements, data, reason, raw) in cases {
        let run = execute(Policy::Width, refusal_case(object, elements, data)).await;
        assert!(run.result.is_err(), "{reason:?} refuses");
        assert!(
            !run.writes_after_start(),
            "{reason:?}: prestate refusal, no write"
        );
        assert!(
            run.private_diagnostic().contains(raw),
            "private raw marker kept"
        );
        let expected = diag(R::Download, reason, object, elements, data.len() as u32);
        let (diagnostics, report) = public(&run);
        assert_eq!(diagnostics, [expected], "{reason:?}");
        assert_eq!(report["format"], 2);
        assert_eq!(report["outcome"], "failed");
        assert_eq!(
            report["failure_diagnostics"],
            json!([{
                "run": "download",
                "reason": serde_json::to_value(reason).unwrap(),
                "object": object, "propertyId": 7, "elements": elements, "octets": data.len(),
            }]),
            "{reason:?}"
        );
        assert_public_clean(&report, &run, raw);
        written_report(&run, &format!("report-{object}.json"), &report, &[raw]);
        // Only the populated failure checkpoint moves to envelope version 2.
        let journal = checkpoints(&run);
        let (last, earlier) = journal.split_last().unwrap();
        assert!(earlier
            .iter()
            .all(|(format, c)| *format == 1 && c.get("pointer_failure").is_none()));
        assert_eq!(last.0, 2);
        assert_eq!(last.1["event"], "failed");
        assert_eq!(last.1["code"], "identity_or_recovery");
        assert!(last.1["local_diagnostic"].as_str().unwrap().contains(raw));
        assert_eq!(last.1["pointer_failure"]["octets"], data.len());
    }
}

#[tokio::test]
async fn download_and_restore_failures_are_two_ordered_independent_diagnoses() {
    let object = in_procedure_object().await;
    // Download: the in-procedure read-back after the first mutation is refused.
    let at_second = move |o: u8, index: usize, _: usize, n: u8, d: &[u8]| {
        if o == object && index == 1 {
            (2, vec![0x0B, 0x0C])
        } else {
            (n, d.to_vec())
        }
    };
    let run = execute(Policy::Width, rw(at_second)).await;
    assert!(run.result.is_err());
    let download = diag(R::Download, Why::ElementCount, object, 2, 2);
    let store = run.store();
    assert!(
        store.progress().restore_available,
        "the original backup is kept"
    );
    assert_eq!(store.progress().failure_diagnostics, [download]);
    drop(store);
    // Restore: its first PID 7 read of object 1 is refused with another reason.
    let restore_refusal = refusal_case(1, 1, &[0x9A, 0x9B, 0x9C]);
    assert!(run.restore(restore_refusal).await.is_err());
    let store = run.store();
    assert_eq!(store.outcome(), LocalOutcome::RestoreFailed);
    let restore = diag(R::Restore, Why::UnsupportedWidth, 1, 1, 3);
    assert_eq!(store.progress().failure_diagnostics, [download, restore]);
    let report = serde_json::to_value(store.shareable_report()).unwrap();
    assert_eq!(report["failure_diagnostics"][0]["run"], "download");
    assert_eq!(report["failure_diagnostics"][1]["run"], "restore");
    assert_eq!(
        report["failure_diagnostics"][1]["reason"],
        "unsupportedWidth"
    );
    assert_public_clean(&report, &run, "9a9b9c");
    assert_public_clean(&report, &run, "0b0c");
    drop(store);
    written_report(&run, "report-restore.json", &report, &["9a9b9c", "0b0c"]);
    let store = run.store();
    let formats: Vec<_> = checkpoints(&run)
        .into_iter()
        .filter(|(f, _)| *f == 2)
        .map(|(_, c)| c["event"].as_str().unwrap().to_string())
        .collect();
    assert_eq!(formats, ["failed", "restore_failed"]);
    drop(store);
    // No second restore and no resume: the guard refuses, diagnoses stay as they are.
    assert!(run.restore(None).await.is_err());
    assert_eq!(
        run.store().progress().failure_diagnostics,
        [download, restore]
    );
    // A new operation starts with an empty list, never inheriting the old diagnosis.
    let fresh = OperationStore::create(
        &run.operation.with_file_name("fresh"),
        run.bundle.binding.clone(),
    )
    .unwrap();
    assert!(fresh.progress().failure_diagnostics.is_empty());
}

#[tokio::test]
async fn a_diagnosed_download_is_never_executed_again_and_keeps_its_evidence() {
    let run = execute(Policy::Width, refusal_case(3, 3, &[1, 2, 3, 4, 5, 6])).await;
    assert!(run.result.is_err());
    let diagnosis = [diag(R::Download, Why::ElementCount, 3, 3, 6)];
    // The same deterministic project admits the same bundle again; only the store is reused.
    let (_dir, conn, project, _) = inputs();
    let admission =
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &run.bundle)
            .unwrap();
    let (journal, seen, device) = (
        run.journal_bytes(),
        run.device.seen().len(),
        run.device.system2_snapshot(),
    );
    let mut store = run.store();
    let retry = run_execute(
        &Hardware(&run.device),
        &admission,
        &mut store,
        &run.bundle,
        &run.device,
    )
    .await;
    let refusal = retry.expect_err("a reopened operation is never executed again");
    assert!(
        refusal.to_string().contains("no resume or retry"),
        "{refusal}"
    );
    assert_eq!(store.progress().failure_diagnostics, diagnosis);
    drop(store);
    assert_eq!(run.journal_bytes(), journal, "no checkpoint appended");
    assert_eq!(run.device.seen().len(), seen, "no frame sent");
    assert_eq!(run.device.system2_snapshot(), device);
}

#[tokio::test]
async fn untyped_and_legacy_failures_carry_no_invented_diagnosis() {
    // A final-verification memory lie: an executor failure, not a pointer refusal.
    let run = execute_with(
        Policy::Width,
        Setup {
            fault: Some(Fault::Memory),
            ..Setup::default()
        },
        None,
    )
    .await;
    assert!(run.result.is_err());
    let (diagnostics, report) = public(&run);
    assert!(diagnostics.is_empty());
    assert_eq!(report["failure_diagnostics"], json!([]));
    assert!(checkpoints(&run)
        .iter()
        .all(|(format, c)| *format == 1 && c.get("pointer_failure").is_none()));
    // Legacy policy keeps its untyped Recovery(String) refusal: no widening to typed facts.
    let run = execute(Policy::Legacy, refusal_case(1, 1, &[1, 0x16])).await;
    assert!(run.result.is_err());
    assert!(run.private_diagnostic().contains("0116"));
    let (diagnostics, report) = public(&run);
    assert!(diagnostics.is_empty());
    assert_eq!(report["failure_diagnostics"], json!([]));
    assert!(checkpoints(&run).iter().all(|(format, _)| *format == 1));
}

/// Replace the last checkpoint and re-commit HEAD, so only the record grammar decides.
fn rewrite_last(run: &Run, edit: impl FnOnce(&mut Value)) {
    let head_path = run.operation.join("HEAD");
    let mut head: Value = serde_json::from_slice(&std::fs::read(&head_path).unwrap()).unwrap();
    let last = run.operation.join(format!(
        "checkpoint-{:08}.json",
        head["sequence"].as_u64().unwrap()
    ));
    let mut value: Value = serde_json::from_slice(&std::fs::read(&last).unwrap()).unwrap();
    edit(&mut value);
    let bytes = serde_json::to_vec_pretty(&value).unwrap();
    std::fs::write(&last, &bytes).unwrap();
    head["sha256"] = format!("{:x}", Sha256::digest(&bytes)).into();
    std::fs::write(&head_path, serde_json::to_vec_pretty(&head).unwrap()).unwrap();
}

#[tokio::test]
async fn malformed_typed_checkpoints_are_refused_on_reopen() {
    type Edit = fn(&mut Value);
    let cases: [(&str, Edit); 17] = [
        ("control: identical record", |_| {}),
        // Absent stays the legacy shape: version 1 without the key replays, with no diagnosis.
        ("control: absent key in version 1", |v| {
            v["format"] = 1.into();
            v["checkpoint"]
                .as_object_mut()
                .unwrap()
                .remove("pointer_failure");
        }),
        ("explicit null in version 1", |v| {
            v["format"] = 1.into();
            v["checkpoint"]["pointer_failure"] = Value::Null;
        }),
        ("explicit null in version 2", |v| {
            v["checkpoint"]["pointer_failure"] = Value::Null
        }),
        ("populated record in version 1", |v| v["format"] = 1.into()),
        ("version 2 without a record", |v| {
            v["checkpoint"]
                .as_object_mut()
                .unwrap()
                .remove("pointer_failure");
        }),
        ("unknown envelope version", |v| v["format"] = 3.into()),
        ("count contradicts reason", |v| {
            v["checkpoint"]["pointer_failure"]["elements"] = 1.into()
        }),
        ("zero elements without data", |v| {
            v["checkpoint"]["pointer_failure"]["octets"] = 0.into()
        }),
        ("element-count reason with one element", |v| {
            v["checkpoint"]["pointer_failure"]["reason"] = "elementCount".into();
            v["checkpoint"]["pointer_failure"]["elements"] = 1.into();
        }),
        ("element-count reason with zero elements", |v| {
            v["checkpoint"]["pointer_failure"]["reason"] = "elementCount".into();
        }),
        ("unsupported-width reason with two octets", |v| {
            v["checkpoint"]["pointer_failure"]["reason"] = "unsupportedWidth".into();
            v["checkpoint"]["pointer_failure"]["elements"] = 1.into();
            v["checkpoint"]["pointer_failure"]["octets"] = 2.into();
        }),
        ("unsupported-width reason with four octets", |v| {
            v["checkpoint"]["pointer_failure"]["reason"] = "unsupportedWidth".into();
            v["checkpoint"]["pointer_failure"]["elements"] = 1.into();
            v["checkpoint"]["pointer_failure"]["octets"] = 4.into();
        }),
        ("other property", |v| {
            v["checkpoint"]["pointer_failure"]["property_id"] = 8.into()
        }),
        ("unknown reason", |v| {
            v["checkpoint"]["pointer_failure"]["reason"] = "rawText".into()
        }),
        ("unknown record key", |v| {
            v["checkpoint"]["pointer_failure"]["raw"] = "abcdef".into()
        }),
        ("record under an executor code", |v| {
            v["checkpoint"]["code"] = "executor".into()
        }),
    ];
    for (label, edit) in cases {
        let run = execute(Policy::Width, refusal_case(2, 0, &[0xAB, 0xCD, 0xEF])).await;
        rewrite_last(&run, edit);
        let opened = OperationStore::open(&run.operation, &run.bundle.binding);
        if label.starts_with("control") {
            let store = opened.expect("a control reopens");
            let typed = usize::from(label.contains("identical"));
            assert_eq!(store.progress().failure_diagnostics.len(), typed, "{label}");
        } else {
            assert!(opened.is_err(), "{label} must be refused");
        }
    }
}

/// The pre-change reader grammar, copied: exact envelope version 1, closed failure keys.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct OldEnvelope {
    format: u16,
    kind: String,
    sequence: usize,
    previous_sha256: String,
    checkpoint: OldCheckpoint,
}
#[derive(serde::Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
#[allow(dead_code)]
enum OldCheckpoint {
    Failed {
        code: String,
        local_diagnostic: String,
    },
}
fn old_reader_admits(bytes: &[u8]) -> bool {
    serde_json::from_slice::<OldEnvelope>(bytes).is_ok_and(|e| e.format == 1)
}

#[tokio::test]
async fn an_older_strict_reader_rejects_the_populated_record_but_reads_legacy_failures() {
    let typed = execute(Policy::Width, refusal_case(3, 3, &[1, 2, 3, 4, 5, 6])).await;
    let legacy = execute(Policy::Legacy, refusal_case(1, 1, &[1, 0x16])).await;
    let last = |run: &Run| {
        run.journal_bytes()
            .into_iter()
            .rfind(|(n, _)| n.starts_with("checkpoint-"))
            .unwrap()
            .1
    };
    assert!(
        !old_reader_admits(&last(&typed)),
        "version 2 and its key refuse"
    );
    assert!(
        old_reader_admits(&last(&legacy)),
        "legacy failure stays readable"
    );
    // Even with the version forged back to 1, the closed old grammar rejects the new key.
    let mut forged: Value = serde_json::from_slice(&last(&typed)).unwrap();
    forged["format"] = 1.into();
    assert!(!old_reader_admits(&serde_json::to_vec(&forged).unwrap()));
}
