//! Closed-schema, replay-refusal and source-preservation tests for the private projection.
use super::*;

fn assert_unavailable(label: &str, path: &Path, binding: &app::Binding) {
    let seen = observe(label, path, binding);
    assert!(
        matches!(&seen.snapshot, PrivateSnapshot::Unavailable(_)),
        "{label}: refusal exposed an available snapshot"
    );
    assert!(seen.snapshot.progress().is_none());
    assert!(seen.snapshot.projection().is_none());
    assert!(seen.json.is_none());
    assert!(seen.text.is_empty());
}

fn checkpoint(path: &Path, sequence: usize) -> PathBuf {
    path.join(format!("checkpoint-{sequence:08}.json"))
}

fn latest_checkpoint(path: &Path) -> PathBuf {
    std::fs::read_dir(path)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .is_some_and(|name| name.to_string_lossy().starts_with("checkpoint-"))
        })
        .max()
        .unwrap()
}

#[tokio::test]
async fn missing_malformed_and_unsupported_chains_are_generically_unavailable() {
    let valid = download(None, false, false).await;
    let missing_parent = tempfile::tempdir().unwrap();
    assert_unavailable(
        "missing",
        &missing_parent.path().join("absent"),
        &valid.bundle.binding,
    );

    let malformed = download(None, false, false).await;
    std::fs::write(latest_checkpoint(&malformed.path), b"{").unwrap();
    assert_unavailable("malformed", &malformed.path, &malformed.bundle.binding);

    let unsupported = download(None, false, false).await;
    let last = latest_checkpoint(&unsupported.path);
    let mut envelope: Value = serde_json::from_slice(&std::fs::read(&last).unwrap()).unwrap();
    envelope["format"] = json!(u16::MAX);
    std::fs::write(last, serde_json::to_vec_pretty(&envelope).unwrap()).unwrap();
    assert_unavailable(
        "unsupported",
        &unsupported.path,
        &unsupported.bundle.binding,
    );
}

#[tokio::test]
async fn binding_mismatch_directory_alias_and_busy_lock_are_generically_unavailable() {
    let binding_case = download(None, false, false).await;
    let mut wrong = binding_case.bundle.binding.clone();
    wrong.source_sha256 = "c".repeat(64);
    assert_unavailable("binding", &binding_case.path, &wrong);

    let alias_case = download(None, false, false).await;
    let alias_parent = tempfile::tempdir().unwrap();
    let alias = alias_parent.path().join("operation-alias");
    std::os::unix::fs::symlink(&alias_case.path, &alias).unwrap();
    assert_unavailable("directory-alias", &alias, &alias_case.bundle.binding);

    let busy = download(None, false, false).await;
    let held = OperationStore::open(&busy.path, &busy.bundle.binding).unwrap();
    assert_unavailable("busy", &busy.path, &busy.bundle.binding);
    drop(held);
}

#[tokio::test]
async fn symlink_and_hardlink_protocol_entries_are_generically_unavailable() {
    let symlink_case = download(None, false, false).await;
    let external = tempfile::tempdir().unwrap();
    let external_checkpoint = external.path().join("checkpoint.json");
    std::fs::copy(checkpoint(&symlink_case.path, 0), &external_checkpoint).unwrap();
    std::fs::remove_file(checkpoint(&symlink_case.path, 0)).unwrap();
    std::os::unix::fs::symlink(&external_checkpoint, checkpoint(&symlink_case.path, 0)).unwrap();
    assert_unavailable(
        "symlink-entry",
        &symlink_case.path,
        &symlink_case.bundle.binding,
    );

    let hardlink_case = download(None, false, false).await;
    std::fs::hard_link(
        checkpoint(&hardlink_case.path, 0),
        hardlink_case.path.join("checkpoint-hardlink-alias.json"),
    )
    .unwrap();
    assert_unavailable(
        "hardlink-entry",
        &hardlink_case.path,
        &hardlink_case.bundle.binding,
    );
}

#[test]
fn projection_serialization_is_deterministic_and_exactly_allowlisted() {
    let projection = PrivateProjection {
        format: PROJECTION_FORMAT,
        kind: PROJECTION_KIND,
        runs: vec![
            RunProjection {
                run: FailureDiagnosticRun::Download,
                allocation_mismatch: AllocationMismatchClass::RecordedBytesWithheld {
                    reason: WithheldReason::UnclassifiedMemory,
                    loss: ExportedLoss::RecordedBytesNotExported,
                },
            },
            RunProjection {
                run: FailureDiagnosticRun::Restore,
                allocation_mismatch: AllocationMismatchClass::NotRecorded,
            },
            RunProjection {
                run: FailureDiagnosticRun::Restore,
                allocation_mismatch: AllocationMismatchClass::Unclassified,
            },
        ],
    };
    let first = serde_json::to_vec(&projection).unwrap();
    let second = serde_json::to_vec(&projection.clone()).unwrap();
    assert_eq!(first, second);
    assert_eq!(
        String::from_utf8(first).unwrap(),
        concat!(
            r#"{"format":1,"kind":"knxbench-system2-private-projection","runs":["#,
            r#"{"run":"download","allocationMismatch":{"classification":"recordedBytesWithheld","reason":"unclassifiedMemory","loss":"recordedBytesNotExported"}},"#,
            r#"{"run":"restore","allocationMismatch":{"classification":"notRecorded"}},"#,
            r#"{"run":"restore","allocationMismatch":{"classification":"unclassified"}}]}"#
        )
    );
}

#[test]
fn sentinel_scanner_accepts_exact_allowlisted_schema_keys() {
    let value = super::projection(json!([withheld("download")]));
    let sentinels = all_sentinels(&[download_expected()]);
    let refs: Vec<&[u8]> = sentinels.iter().map(Vec::as_slice).collect();

    assert_no_sentinel("exact-schema", value.to_string().as_bytes(), &refs);
}

#[test]
fn sentinel_scanner_rejects_decoded_secret_values_in_representative_encodings() {
    let expected = download_expected();
    let readback = READ_WRITTEN.1.to_vec();
    let raw = b"ASCII-SECRET".to_vec();
    let sentinels = [expected, readback, raw];
    let refs: Vec<&[u8]> = sentinels.iter().map(Vec::as_slice).collect();
    let cases = [
        ("expected-hex", json!({"payload": "5ac31b4d8fe246f0"})),
        ("readback-decimal", json!({"payload": [156, 107]})),
        ("expected-base64", json!({"payload": "WsMbTY/iRvA="})),
        (
            "readback-escaped-hex",
            serde_json::from_str::<Value>(r#"{"payload":"\u0039c6b"}"#).unwrap(),
        ),
        ("raw-ascii", json!({"payload": "ASCII-SECRET"})),
    ];

    for (label, value) in cases {
        let text = value.to_string();
        assert!(
            std::panic::catch_unwind(|| assert_no_sentinel(label, text.as_bytes(), &refs)).is_err(),
            "{label}: inserted secret was not rejected"
        );
    }
}

#[test]
fn sentinel_scanner_does_not_exempt_arbitrary_keys_or_values() {
    let readback = READ_WRITTEN.1;
    let refs = [readback.as_slice()];
    for (label, value) in [
        ("non-schema-key", json!({"custom-9c6b": null})),
        ("non-schema-value", json!({"custom": "9c6b"})),
        (
            "schema-prefix-key",
            json!({"classification-9c6b": "unclassified"}),
        ),
    ] {
        let text = value.to_string();
        assert!(
            std::panic::catch_unwind(|| assert_no_sentinel(label, text.as_bytes(), &refs)).is_err(),
            "{label}: arbitrary key or value was incorrectly exempted"
        );
    }
}

#[tokio::test]
async fn available_snapshot_is_owned_and_preserves_progress_report_and_journal() {
    let journal = download(Some(READ_UNWRITTEN), false, false).await;
    let before = journal_bytes(&journal.path);
    let baseline = OperationStore::open(&journal.path, &journal.bundle.binding).unwrap();
    let expected_progress = baseline.progress();
    let expected_report = serde_json::to_value(baseline.shareable_report()).unwrap();
    drop(baseline);

    let observed = observe("owned", &journal.path, &journal.bundle.binding);
    let (progress, projection) = match observed.snapshot {
        PrivateSnapshot::Available(snapshot) => snapshot.into_parts(),
        PrivateSnapshot::Unavailable(_) => panic!("valid replay stayed unavailable"),
    };
    assert_eq!(progress, expected_progress);
    assert_eq!(
        serde_json::to_value(&projection).unwrap(),
        super::projection(json!([withheld("download")]))
    );

    let reopened = OperationStore::open(&journal.path, &journal.bundle.binding).unwrap();
    assert_eq!(
        serde_json::to_value(reopened.shareable_report()).unwrap(),
        expected_report
    );
    drop(reopened);
    assert_eq!(journal_bytes(&journal.path), before, "journal untouched");

    std::fs::remove_dir_all(&journal.path).unwrap();
    assert_eq!(progress.outcome, LocalOutcome::Failed);
    assert_eq!(projection.runs.len(), 1);
}
