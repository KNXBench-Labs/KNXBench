//! A final allocation mismatch keeps its bytes through journal and reopen, never publicly.
use super::final_step_diagnostic::failing;
use super::pid7_failure_diagnostic::{checkpoints, rewrite_last};
use super::*;
use knx_app::property_validation::{
    AllocationEvidenceAvailability as Availability, AllocationMismatchEvidenceView,
    DifferenceClassification, ExecutionBuild, FailureDiagnosticRun as R, System2Report,
};
use serde_json::{json, Value};

fn decode(hex: &str) -> Vec<u8> {
    (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect()
}
fn recorded(availability: &Availability) -> &AllocationMismatchEvidenceView {
    match availability {
        Availability::Recorded(view) => view,
        other => panic!("evidence must be recorded: {other:?}"),
    }
}
/// The `Fault::Memory` lie flips bit 0 of the first octet of every final memory answer, so
/// the first allocation differs exactly at each read's first offset, by exactly one bit.
fn assert_flipped_chunk_starts(view: &AllocationMismatchEvidenceView) {
    assert_eq!(view.allocation_index, 0);
    let (expected, actual) = (decode(&view.expected), decode(&view.actual));
    assert_eq!((expected.len(), actual.len()), (view.length, view.length));
    assert!(view.differing_ranges.len() > 1, "{view:?}");
    let step = view.differing_ranges[1].offset;
    assert!(step > 1);
    assert_eq!(view.differing_ranges.len(), view.length.div_ceil(step));
    for (i, range) in view.differing_ranges.iter().enumerate() {
        assert_eq!((range.offset, range.length), (i * step, 1), "{view:?}");
        let (e, a) = (expected[range.offset], actual[range.offset]);
        assert_eq!(a, e ^ 1);
        assert_eq!(
            (range.expected.as_str(), range.actual.as_str()),
            (format!("{e:02x}").as_str(), format!("{a:02x}").as_str())
        );
        let written = view
            .written
            .iter()
            .any(|w| w.offset <= range.offset && range.offset < w.offset + w.length);
        assert_eq!(
            range.classification,
            if written {
                DifferenceClassification::Written
            } else {
                DifferenceClassification::Unwritten
            }
        );
    }
    for (offset, (e, a)) in expected.iter().zip(&actual).enumerate() {
        assert_eq!(e == a, offset % step != 0, "only the chunk starts differ");
    }
}

#[tokio::test]
async fn an_allocation_mismatch_journals_its_bytes_and_reopens_with_them() {
    let run = failing(Fault::Memory).await;
    let (format, last) = checkpoints(&run).pop().unwrap();
    assert_eq!(format, 4);
    assert_eq!(last["event"], "failed");
    let record = &last["allocation_evidence"];
    assert_eq!(record["allocation_index"], 0);
    assert_eq!(record["prestate_unreadable"], false);
    // Reopened from disk alone: the same bytes, typed.
    let store = run.store();
    let entries = store.allocation_mismatch_evidence();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].run, R::Download);
    let view = recorded(&entries[0].evidence);
    assert_flipped_chunk_starts(view);
    assert_eq!(record["expected"], view.expected.as_str());
    assert_eq!(record["actual"], view.actual.as_str());
    assert_eq!(record["length"], view.length);
    let bundle_form = serde_json::to_value(&entries[0]).unwrap();
    assert_eq!(bundle_form["run"], "download");
    assert_eq!(bundle_form["evidence"]["availability"], "recorded");
    assert_eq!(bundle_form["evidence"]["allocationIndex"], 0);
    assert_eq!(
        bundle_form["evidence"]["differingRanges"][0]["classification"],
        json!(view.differing_ranges[0].classification)
    );
    // Progress and both shareable reports stay exactly as minimised as before.
    let progress = serde_json::to_string(&store.progress()).unwrap();
    let summary = serde_json::to_string(&store.shareable_report()).unwrap();
    let route = serde_json::to_string(
        &System2Report::from_progress(&store.progress(), ExecutionBuild::Unknown, "0.1.0").unwrap(),
    )
    .unwrap();
    for text in [progress, summary, route] {
        for leak in [
            "allocation_evidence",
            "allocationEvidence",
            "differing",
            view.actual.as_str(),
        ] {
            assert!(!text.contains(leak), "public projection leaks {leak}");
        }
    }
}

#[tokio::test]
async fn a_restore_mismatch_is_replayed_against_the_restore_plan_as_its_own_run() {
    let run = failing(Fault::Memory).await;
    assert!(run
        .restore_with_fault(None, Some(Fault::Memory))
        .await
        .is_err());
    assert_eq!(checkpoints(&run).pop().unwrap().0, 4);
    let entries = run.store().allocation_mismatch_evidence();
    assert_eq!(
        entries.iter().map(|e| e.run).collect::<Vec<_>>(),
        [R::Download, R::Restore]
    );
    let (download, restore) = (
        recorded(&entries[0].evidence),
        recorded(&entries[1].evidence),
    );
    assert_flipped_chunk_starts(download);
    assert_flipped_chunk_starts(restore);
    assert_ne!(
        download.expected, restore.expected,
        "the restore expects the old bytes, not the download's"
    );
}

#[tokio::test]
async fn an_older_journal_reopens_with_the_evidence_marked_unavailable() {
    let run = failing(Fault::Memory).await;
    rewrite_last(&run, |v| {
        v["format"] = 3.into();
        v["checkpoint"]
            .as_object_mut()
            .unwrap()
            .remove("allocation_evidence");
    });
    let store = run.store();
    assert_eq!(store.progress().final_step_failures.len(), 1);
    let entries = store.allocation_mismatch_evidence();
    assert_eq!(entries.len(), 1);
    assert_eq!(
        serde_json::to_value(&entries[0]).unwrap(),
        json!({"run": "download", "evidence": {"availability": "unavailable", "reason": "notRecorded"}})
    );
}

#[tokio::test]
async fn failures_without_an_allocation_mismatch_list_no_evidence() {
    for fault in [Fault::ProgramVersion, Fault::MemorySilent] {
        let run = failing(fault).await;
        assert!(checkpoints(&run)
            .iter()
            .all(|(_, c)| c.get("allocation_evidence").is_none()));
        assert_eq!(run.store().allocation_mismatch_evidence(), []);
    }
}

#[tokio::test]
async fn malformed_or_inconsistent_evidence_is_refused_on_reopen() {
    type Edit = fn(&mut Value);
    fn evidence(v: &mut Value) -> &mut Value {
        &mut v["checkpoint"]["allocation_evidence"]
    }
    fn flip_hex(text: &str, at: usize) -> String {
        let mut bytes = decode(text);
        bytes[at] ^= 0x80;
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }
    let cases: Vec<(&str, Edit)> = vec![
        ("control: identical record", |_| {}),
        ("explicit null", |v| *evidence(v) = Value::Null),
        ("record under version 3", |v| v["format"] = 3.into()),
        ("missing entire evidence under version 4", |v| {
            v["checkpoint"]
                .as_object_mut()
                .unwrap()
                .remove("allocation_evidence");
        }),
        ("unknown difference class", |v| {
            evidence(v)["differing"][0]["classification"] = "guessed".into();
        }),
        ("wrong address", |v| evidence(v)["address"] = 0.into()),
        ("unknown envelope version", |v| v["format"] = 5.into()),
        ("unknown record key", |v| evidence(v)["raw"] = "ab".into()),
        ("missing key", |v| {
            evidence(v).as_object_mut().unwrap().remove("written");
        }),
        ("uppercase hex", |v| {
            let upper = evidence(v)["actual"].as_str().unwrap().to_uppercase();
            evidence(v)["actual"] = upper.into();
        }),
        ("odd hex", |v| {
            let short = evidence(v)["actual"].as_str().unwrap()[1..].to_owned();
            evidence(v)["actual"] = short.into();
        }),
        ("length differs from bytes", |v| {
            let length = evidence(v)["length"].as_u64().unwrap();
            evidence(v)["length"] = (length + 1).into();
        }),
        ("actual equal to expected", |v| {
            let expected = evidence(v)["expected"].clone();
            evidence(v)["actual"] = expected;
        }),
        ("edited expected byte", |v| {
            let edited = flip_hex(evidence(v)["expected"].as_str().unwrap(), 1);
            evidence(v)["expected"] = edited.into();
        }),
        ("edited actual byte outside the ranges", |v| {
            let edited = flip_hex(evidence(v)["actual"].as_str().unwrap(), 1);
            evidence(v)["actual"] = edited.into();
        }),
        ("reclassified range", |v| {
            let class = &mut evidence(v)["differing"][0]["classification"];
            *class = if *class == "written" {
                "unwritten"
            } else {
                "written"
            }
            .into();
        }),
        ("dropped range", |v| {
            evidence(v)["differing"].as_array_mut().unwrap().pop();
        }),
        ("edited written mask", |v| {
            evidence(v)["written"] = json!([]);
        }),
        ("another allocation", |v| {
            evidence(v)["allocation_index"] = 1.into()
        }),
        ("unknown allocation", |v| {
            evidence(v)["allocation_index"] = 99.into()
        }),
        ("another object", |v| evidence(v)["object"] = 9.into()),
        ("claimed unreadable prestate", |v| {
            evidence(v)["prestate_unreadable"] = true.into()
        }),
        ("beside another final stage", |v| {
            v["checkpoint"]["final_step"]["stage"] = "loadedState".into()
        }),
        ("without a final-step record", |v| {
            v["checkpoint"]
                .as_object_mut()
                .unwrap()
                .remove("final_step");
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
            let store = opened.expect("a control reopens");
            assert!(matches!(
                store.allocation_mismatch_evidence()[0].evidence,
                Availability::Recorded(_)
            ));
        } else {
            assert!(opened.is_err(), "{label} must be refused");
        }
    }
}

/// The reader before this change: envelope versions 1 to 3, closed failure keys.
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
        #[serde(default)]
        final_step: Option<Value>,
    },
}

#[tokio::test]
async fn the_previous_strict_reader_refuses_an_evidence_record_instead_of_dropping_it() {
    let run = failing(Fault::Memory).await;
    let bytes = run
        .journal_bytes()
        .into_iter()
        .rfind(|(n, _)| n.starts_with("checkpoint-"))
        .unwrap()
        .1;
    let admits =
        |b: &[u8]| serde_json::from_slice::<PreviousEnvelope>(b).is_ok_and(|e| e.format <= 3);
    assert!(!admits(&bytes));
    let mut forged: Value = serde_json::from_slice(&bytes).unwrap();
    forged["format"] = 3.into();
    assert!(
        !admits(&serde_json::to_vec(&forged).unwrap()),
        "the key itself is unknown to it"
    );
    forged["checkpoint"]
        .as_object_mut()
        .unwrap()
        .remove("allocation_evidence");
    assert!(
        admits(&serde_json::to_vec(&forged).unwrap()),
        "control: the rest is the version 3 grammar"
    );
}
