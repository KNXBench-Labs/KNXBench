//! A System-2 final-step failure keeps its typed category through journal, reopen and reports.
use super::pid7_failure_diagnostic::{
    checkpoints, diag, in_procedure_object, public, rewrite_last,
};
use super::*;
use knx_app::property_validation::{
    DownloadResult, ExecutionBuild, FailureDiagnosticRun as R, FinalStepCategory as Cat,
    FinalStepDiagnostic, FinalStepStage as Stage, PointerFailureReason as Why, System2Report,
};
use serde_json::{json, Value};

fn final_diag(run: R, category: Cat, stage: Stage) -> FinalStepDiagnostic {
    FinalStepDiagnostic {
        run,
        category,
        stage,
    }
}
pub(super) async fn failing(fault: Fault) -> Run {
    let run = execute_with(
        Policy::Width,
        Setup {
            fault: Some(fault),
            ..Setup::default()
        },
        None,
    )
    .await;
    assert!(
        run.result.is_err(),
        "the final-step fault must fail the run"
    );
    run
}
fn report(run: &Run) -> Value {
    let store = run.store();
    serde_json::to_value(
        System2Report::from_progress(&store.progress(), ExecutionBuild::Unknown, "0.1.0").unwrap(),
    )
    .unwrap()
}
fn last(run: &Run) -> (u64, Value) {
    checkpoints(run).pop().unwrap()
}

#[tokio::test]
async fn each_final_step_category_reaches_the_reopened_progress_journal_and_both_reports() {
    for (fault, category, stage, text) in [
        (
            Fault::Memory,
            Cat::VerificationMismatch,
            Stage::Allocation,
            ("verificationMismatch", "allocation"),
        ),
        (
            Fault::LoadState,
            Cat::VerificationMismatch,
            Stage::LoadedState,
            ("verificationMismatch", "loadedState"),
        ),
        (
            Fault::ProgramVersion,
            Cat::VerificationMismatch,
            Stage::ProgramVersion,
            ("verificationMismatch", "programVersion"),
        ),
        (
            Fault::MemorySilent,
            Cat::ReadFailed,
            Stage::Allocation,
            ("readFailed", "allocation"),
        ),
        (
            Fault::ControlWriteSilent,
            Cat::WriteFailed,
            Stage::VerifyModeWrite,
            ("writeFailed", "verifyModeWrite"),
        ),
    ] {
        let run = failing(fault).await;
        let expected = final_diag(R::Download, category, stage);
        let progress = run.store().progress();
        assert_eq!(progress.final_step_failures, [expected], "{text:?}");
        assert!(
            progress.failure_diagnostics.is_empty(),
            "no pointer fact is invented"
        );
        assert_eq!(
            progress.download.result,
            DownloadResult::TransferCompleteVerificationUnfinished
        );
        let journal = checkpoints(&run);
        let (final_checkpoint, earlier) = journal.split_last().unwrap();
        assert!(earlier
            .iter()
            .all(|(format, c)| *format == 1 && c.get("final_step").is_none()));
        // An allocation mismatch also carries its bytes, which marks envelope version 4.
        let allocation = (category, stage) == (Cat::VerificationMismatch, Stage::Allocation);
        assert_eq!(
            final_checkpoint.0,
            if allocation { 4 } else { 3 },
            "{text:?}"
        );
        assert_eq!(
            final_checkpoint.1.get("allocation_evidence").is_some(),
            allocation,
            "{text:?}"
        );
        assert_eq!(
            final_checkpoint.1["final_step"],
            json!({"category": text.0, "stage": text.1})
        );
        assert_eq!(
            run.private_code(),
            if category == Cat::VerificationMismatch {
                "identity_or_recovery"
            } else {
                "executor"
            },
            "{text:?}"
        );
        let entry = json!({"run": "download", "category": text.0, "stage": text.1});
        let (_, summary) = public(&run);
        assert_eq!(summary["format"], 3);
        assert_eq!(summary["final_step_failures"], json!([entry]));
        let route = report(&run);
        assert_eq!(route["format"], 3);
        assert_eq!(route["finalStepFailures"], json!([entry]));
        assert_eq!(
            route["download"]["result"],
            "transferCompleteVerificationUnfinished"
        );
        let text_all = route.to_string() + &summary.to_string();
        for leak in [
            "local_diagnostic",
            "Recovery(",
            "full allocation",
            "Session(",
            "timeout",
            "checkpoint-",
        ] {
            assert!(!text_all.contains(leak), "public report leaks {leak}");
        }
    }
}

#[tokio::test]
async fn a_restore_final_step_failure_is_its_own_run_and_never_the_downloads() {
    let run = failing(Fault::Memory).await;
    assert!(run
        .restore_with_fault(None, Some(Fault::MemorySilent))
        .await
        .is_err());
    let store = run.store();
    assert_eq!(store.outcome(), LocalOutcome::RestoreFailed);
    let progress = store.progress();
    assert_eq!(
        progress.final_step_failures,
        [
            final_diag(R::Download, Cat::VerificationMismatch, Stage::Allocation),
            final_diag(R::Restore, Cat::ReadFailed, Stage::Allocation),
        ]
    );
    // The frozen download facts survive the restore.
    assert_eq!(
        progress.download.result,
        DownloadResult::TransferCompleteVerificationUnfinished
    );
    assert_eq!(progress.download.outcome, LocalOutcome::Failed);
    drop(store);
    let route = report(&run);
    assert_eq!(route["download"]["outcome"], "failed");
    assert_eq!(route["recovery"]["outcome"], "restoreFailed");
    assert_eq!(route["finalStepFailures"][0]["run"], "download");
    assert_eq!(route["finalStepFailures"][1]["run"], "restore");
    assert_eq!(last(&run).0, 3);
    assert_eq!(last(&run).1["event"], "restore_failed");
}

#[tokio::test]
async fn a_restore_final_step_failure_after_an_earlier_download_failure_is_not_attributed_to_it() {
    let object = in_procedure_object().await;
    let at_second = move |o: u8, index: usize, _: usize, n: u8, d: &[u8]| {
        if o == object && index == 1 {
            (2, vec![0x0B, 0x0C])
        } else {
            (n, d.to_vec())
        }
    };
    let run = execute(Policy::Width, rw(at_second)).await;
    assert!(run.result.is_err());
    assert!(
        run.store().progress().final_step_failures.is_empty(),
        "an in-procedure refusal is not a final-step failure"
    );
    assert!(run
        .restore_with_fault(None, Some(Fault::Memory))
        .await
        .is_err());
    let progress = run.store().progress();
    assert_eq!(
        progress.final_step_failures,
        [final_diag(
            R::Restore,
            Cat::VerificationMismatch,
            Stage::Allocation
        )]
    );
    assert_eq!(
        progress.failure_diagnostics,
        [diag(R::Download, Why::ElementCount, object, 2, 2)]
    );
}

#[tokio::test]
async fn a_refused_final_pointer_answer_is_a_final_read_failure_and_keeps_its_pointer_fact() {
    let object = in_procedure_object().await;
    let final_read = move |o: u8, index: usize, total: usize, n: u8, d: &[u8]| {
        if o == object && index + 1 == total {
            (2, vec![0x0B, 0x0C])
        } else {
            (n, d.to_vec())
        }
    };
    let run = execute(Policy::Width, rw(final_read)).await;
    assert!(run.result.is_err());
    let progress = run.store().progress();
    assert_eq!(
        progress.final_step_failures,
        [final_diag(R::Download, Cat::ReadFailed, Stage::Pointer)]
    );
    assert_eq!(
        progress.failure_diagnostics,
        [diag(R::Download, Why::ElementCount, object, 2, 2)]
    );
    assert_eq!(last(&run).0, 3);
    assert_eq!(last(&run).1["pointer_failure"]["reason"], "elementCount");
    assert_eq!(
        last(&run).1["final_step"],
        json!({"category": "readFailed", "stage": "pointer"})
    );
}

#[tokio::test]
async fn failures_outside_the_final_step_stay_without_a_final_step_record() {
    let run = execute(Policy::Width, refusal_case(2, 0, &[0xAB, 0xCD, 0xEF])).await;
    assert!(run.result.is_err());
    let progress = run.store().progress();
    assert!(progress.final_step_failures.is_empty());
    assert_eq!(progress.failure_diagnostics.len(), 1);
    assert_eq!(
        last(&run).0,
        2,
        "a pointer-only failure keeps envelope version 2"
    );
    assert!(last(&run).1.get("final_step").is_none());
    assert_eq!(report(&run)["finalStepFailures"], json!([]));
}

#[tokio::test]
async fn a_journal_without_the_final_step_record_replays_as_unavailable_not_guessed() {
    let run = failing(Fault::Memory).await;
    rewrite_last(&run, |v| {
        v["format"] = 1.into();
        let checkpoint = v["checkpoint"].as_object_mut().unwrap();
        checkpoint.remove("final_step");
        checkpoint.remove("allocation_evidence");
    });
    let progress = run.store().progress();
    assert!(progress.final_step_failures.is_empty());
    assert_eq!(
        progress.download.result,
        DownloadResult::TransferCompleteVerificationUnfinished
    );
    assert_eq!(report(&run)["finalStepFailures"], json!([]));
}

/// The reader before this change: envelope versions 1 and 2 only, closed failure keys.
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
#[allow(dead_code)]
struct PreviousEnvelope {
    format: u16,
    kind: String,
    sequence: usize,
    previous_sha256: String,
    checkpoint: PreviousCheckpoint,
}
#[derive(serde::Deserialize)]
#[serde(tag = "event", rename_all = "snake_case", deny_unknown_fields)]
#[allow(dead_code)]
enum PreviousCheckpoint {
    Failed {
        code: String,
        local_diagnostic: String,
        #[serde(default)]
        pointer_failure: Option<Value>,
    },
}

#[tokio::test]
async fn the_previous_strict_reader_refuses_a_final_step_record_instead_of_dropping_it() {
    let run = failing(Fault::Memory).await;
    let bytes = run
        .journal_bytes()
        .into_iter()
        .rfind(|(n, _)| n.starts_with("checkpoint-"))
        .unwrap()
        .1;
    let admits =
        |b: &[u8]| serde_json::from_slice::<PreviousEnvelope>(b).is_ok_and(|e| e.format <= 2);
    assert!(!admits(&bytes));
    let mut forged: Value = serde_json::from_slice(&bytes).unwrap();
    forged["format"] = 2.into();
    assert!(
        !admits(&serde_json::to_vec(&forged).unwrap()),
        "the key itself is unknown to it"
    );
}

#[tokio::test]
async fn contradictory_or_malformed_final_step_checkpoints_are_refused_on_reopen() {
    type Edit = fn(&mut Value);
    let cases: Vec<(&str, Edit)> = vec![
        ("control: identical record", |_| {}),
        ("explicit null", |v| {
            v["checkpoint"]["final_step"] = Value::Null
        }),
        ("explicit null under version 1", |v| {
            v["format"] = 1.into();
            v["checkpoint"]["final_step"] = Value::Null
        }),
        ("record under version 1", |v| v["format"] = 1.into()),
        ("record under version 2", |v| v["format"] = 2.into()),
        ("version 3 without a record", |v| {
            v["checkpoint"]
                .as_object_mut()
                .unwrap()
                .remove("final_step");
        }),
        ("unknown envelope version", |v| v["format"] = 5.into()),
        ("unknown category", |v| {
            v["checkpoint"]["final_step"]["category"] = "rawText".into()
        }),
        ("unknown stage", |v| {
            v["checkpoint"]["final_step"]["stage"] = "disconnect".into()
        }),
        ("unknown record key", |v| {
            v["checkpoint"]["final_step"]["raw"] = "ab".into()
        }),
        ("missing stage", |v| {
            v["checkpoint"]["final_step"]
                .as_object_mut()
                .unwrap()
                .remove("stage");
        }),
        ("write failure in the allocation stage", |v| {
            v["checkpoint"]["final_step"] =
                json!({"category": "writeFailed", "stage": "allocation"});
            v["checkpoint"]["code"] = "executor".into();
        }),
        ("mismatch in the write stage", |v| {
            v["checkpoint"]["final_step"]["stage"] = "verifyModeWrite".into()
        }),
        ("mismatch in the limit stage", |v| {
            v["checkpoint"]["final_step"]["stage"] = "limit".into()
        }),
        ("mismatch in the pointer stage", |v| {
            v["checkpoint"]["final_step"]["stage"] = "pointer".into()
        }),
        ("read failure in the write stage", |v| {
            v["checkpoint"]["final_step"] =
                json!({"category": "readFailed", "stage": "verifyModeWrite"});
            v["checkpoint"]["code"] = "executor".into();
        }),
        ("mismatch under an executor code", |v| {
            v["checkpoint"]["code"] = "executor".into()
        }),
        ("mismatch under a persistence code", |v| {
            v["checkpoint"]["code"] = "persistence".into()
        }),
        ("write failure under the recovery code", |v| {
            v["checkpoint"]["final_step"] =
                json!({"category": "writeFailed", "stage": "verifyModeWrite"});
        }),
        ("mismatch beside a pointer record", |v| {
            v["checkpoint"]["pointer_failure"] = json!({"reason": "elementCount", "object": 3, "property_id": 7, "elements": 2, "octets": 2});
        }),
        ("read failure in memory beside a pointer record", |v| {
            v["checkpoint"]["final_step"] =
                json!({"category": "readFailed", "stage": "allocation"});
            v["checkpoint"]["pointer_failure"] = json!({"reason": "elementCount", "object": 3, "property_id": 7, "elements": 2, "octets": 2});
        }),
    ];
    // One real failed run; every case starts from its exact original bytes.
    let run = failing(Fault::Memory).await;
    let original = run.journal_bytes();
    for (label, edit) in cases {
        for (name, bytes) in &original {
            std::fs::write(run.operation.join(name), bytes).unwrap();
        }
        rewrite_last(&run, edit);
        let opened = OperationStore::open(&run.operation, &run.bundle.binding);
        if label.starts_with("control") {
            assert_eq!(
                opened
                    .expect("a control reopens")
                    .progress()
                    .final_step_failures
                    .len(),
                1,
                "{label}"
            );
        } else {
            assert!(opened.is_err(), "{label} must be refused");
        }
    }
}

#[tokio::test]
async fn a_final_step_record_on_a_failure_outside_the_final_step_is_refused() {
    // A prestate pointer refusal ended the run at an early step, not the final one.
    let run = execute(Policy::Width, refusal_case(2, 0, &[0xAB, 0xCD, 0xEF])).await;
    rewrite_last(&run, |v| {
        v["format"] = 3.into();
        v["checkpoint"]["final_step"] = json!({"category": "readFailed", "stage": "pointer"});
    });
    assert!(OperationStore::open(&run.operation, &run.bundle.binding).is_err());
}
