//! Exercises consuming admission of the closed private projection without reopening its journal.

use super::tests::{
    all_sentinels, assert_no_sentinel, download, restore, run_class, withheld, READ_RESTORE,
    READ_UNWRITTEN, READ_WRITTEN,
};
use super::*;
use serde_json::{json, Value};
use std::path::Path;

fn available(path: &Path, binding: &Binding) -> AvailableSnapshot {
    match read_snapshot(path, binding) {
        PrivateSnapshot::Available(snapshot) => *snapshot,
        PrivateSnapshot::Unavailable(_) => panic!("producer-valid snapshot was unavailable"),
    }
}

fn admit(snapshot: AvailableSnapshot) -> (OperationProgress, AdmittedPrivateProjection) {
    match admit_snapshot(snapshot) {
        Ok(admitted) => admitted,
        Err(_) => panic!("closed projection was not admitted"),
    }
}

fn value(admitted: &AdmittedPrivateProjection) -> Value {
    serde_json::from_slice(admitted.bytes()).expect("admitted projection is JSON")
}

fn reject(snapshot: AvailableSnapshot) {
    assert!(admit_snapshot(snapshot).is_err());
}

fn finished_parts() -> (OperationProgress, PrivateProjection) {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let journal = runtime.block_on(download(None, false, false));
    available(&journal.path, &journal.bundle.binding).into_parts()
}

#[tokio::test]
async fn bundle_admission_closed_projection_owns_bytes_and_preserves_progress() {
    let journal = download(Some(READ_UNWRITTEN), false, false).await;
    let snapshot = available(&journal.path, &journal.bundle.binding);
    let expected_progress = snapshot.progress().clone();
    let expected_bytes = serde_json::to_vec(snapshot.projection()).unwrap();
    let sentinels = all_sentinels(&[]);
    let sentinel_refs: Vec<&[u8]> = sentinels.iter().map(Vec::as_slice).collect();

    std::fs::remove_dir_all(&journal.path).unwrap();
    let (progress, admitted) = admit(snapshot);

    assert_eq!(progress, expected_progress);
    assert_eq!(admitted.bytes(), expected_bytes);
    assert!(admitted.bytes().len() <= 4096);
    assert_no_sentinel("bundle-admission-owned", admitted.bytes(), &sentinel_refs);
    assert_eq!(
        value(&admitted),
        json!({
            "format": PROJECTION_FORMAT,
            "kind": PROJECTION_KIND,
            "runs": [withheld("download")],
        })
    );
}

#[tokio::test]
async fn bundle_admission_serialization_is_deterministic_across_owned_snapshots() {
    let journal = download(None, false, false).await;
    let first = available(&journal.path, &journal.bundle.binding);
    let second = available(&journal.path, &journal.bundle.binding);

    let (_, first) = admit(first);
    let (_, second) = admit(second);

    assert_eq!(first.bytes(), second.bytes());
    assert_eq!(
        value(&first),
        json!({
            "format": PROJECTION_FORMAT,
            "kind": PROJECTION_KIND,
            "runs": [run_class("download", "unclassified")],
        })
    );
}

#[tokio::test]
async fn bundle_admission_restore_is_optional_ordered_and_progress_consistent() {
    let journal = download(Some(READ_WRITTEN), false, false).await;
    restore(&journal, Some(READ_RESTORE)).await;
    let snapshot = available(&journal.path, &journal.bundle.binding);
    let expected_progress = snapshot.progress().clone();
    std::fs::remove_dir_all(&journal.path).unwrap();

    let (progress, admitted) = admit(snapshot);

    assert_eq!(progress, expected_progress);
    assert!(progress.restore_attempted);
    assert_eq!(
        value(&admitted),
        json!({
            "format": PROJECTION_FORMAT,
            "kind": PROJECTION_KIND,
            "runs": [withheld("download"), withheld("restore")],
        })
    );
}

fn assert_closed_classification(class: AllocationMismatchClass, expected: &str) {
    let (progress, projection) = finished_parts();
    let snapshot = AvailableSnapshot {
        progress,
        projection: PrivateProjection {
            format: projection.format,
            kind: projection.kind,
            runs: vec![RunProjection {
                run: FailureDiagnosticRun::Download,
                allocation_mismatch: class,
            }],
        },
    };
    let (_, admitted) = admit(snapshot);
    assert_eq!(
        value(&admitted)["runs"][0]["allocationMismatch"]["classification"].as_str(),
        Some(expected)
    );
}

#[test]
fn bundle_admission_accepts_recorded_bytes_withheld_classification() {
    assert_closed_classification(
        AllocationMismatchClass::RecordedBytesWithheld {
            reason: WithheldReason::UnclassifiedMemory,
            loss: ExportedLoss::RecordedBytesNotExported,
        },
        "recordedBytesWithheld",
    );
}

#[test]
fn bundle_admission_accepts_not_recorded_classification() {
    assert_closed_classification(AllocationMismatchClass::NotRecorded, "notRecorded");
}

#[test]
fn bundle_admission_accepts_unclassified_classification() {
    assert_closed_classification(AllocationMismatchClass::Unclassified, "unclassified");
}

#[test]
fn bundle_admission_refuses_unknown_projection_format() {
    let (progress, mut projection) = finished_parts();
    projection.format = PROJECTION_FORMAT + 1;
    reject(AvailableSnapshot {
        progress,
        projection,
    });
}

#[test]
fn bundle_admission_refuses_unknown_projection_kind() {
    let (progress, mut projection) = finished_parts();
    projection.kind = "not-a-private-projection";
    reject(AvailableSnapshot {
        progress,
        projection,
    });
}

#[test]
fn bundle_admission_refuses_missing_download_run() {
    let (progress, mut projection) = finished_parts();
    projection.runs.clear();
    reject(AvailableSnapshot {
        progress,
        projection,
    });
}

#[test]
fn bundle_admission_refuses_duplicate_download_runs() {
    let (progress, mut projection) = finished_parts();
    projection.runs.push(projection.runs[0]);
    reject(AvailableSnapshot {
        progress,
        projection,
    });
}

#[test]
fn bundle_admission_refuses_restore_before_download() {
    let (progress, mut projection) = finished_parts();
    projection.runs.insert(
        0,
        RunProjection {
            run: FailureDiagnosticRun::Restore,
            allocation_mismatch: AllocationMismatchClass::Unclassified,
        },
    );
    reject(AvailableSnapshot {
        progress,
        projection,
    });
}

#[test]
fn bundle_admission_refuses_multiple_restore_runs() {
    let (progress, mut projection) = finished_parts();
    let restore = RunProjection {
        run: FailureDiagnosticRun::Restore,
        allocation_mismatch: AllocationMismatchClass::Unclassified,
    };
    projection.runs.extend([restore, restore]);
    reject(AvailableSnapshot {
        progress,
        projection,
    });
}

#[test]
fn bundle_admission_refuses_restore_projection_without_restore_progress() {
    let (progress, mut projection) = finished_parts();
    projection.runs.push(RunProjection {
        run: FailureDiagnosticRun::Restore,
        allocation_mismatch: AllocationMismatchClass::Unclassified,
    });
    assert!(!progress.restore_attempted);
    reject(AvailableSnapshot {
        progress,
        projection,
    });
}

#[tokio::test]
async fn bundle_admission_refuses_restore_progress_without_restore_projection() {
    let journal = download(Some(READ_WRITTEN), false, false).await;
    restore(&journal, None).await;
    let (progress, mut projection) = available(&journal.path, &journal.bundle.binding).into_parts();
    assert!(progress.restore_attempted);
    projection.runs.truncate(1);
    reject(AvailableSnapshot {
        progress,
        projection,
    });
}
