//! Durable operation receipts are exercised through real files and fresh openers.
use knx_app::property_validation::{Binding, Checkpoint, LocalOutcome, OperationStore};
use std::fs;

fn binding() -> Binding {
    Binding {
        target: "1.1.7".into(),
        serial: [1, 2, 3, 4, 5, 6],
        mask: 0x0025,
        manufacturer: 2,
        hardware: [7; 6],
        application: [0, 2, 0x12, 0x34, 1],
        profile_id: "synthetic-v1".into(),
        profile_version: 1,
        profile_sha256: "a".repeat(64),
        source_sha256: "b".repeat(64),
        plan_sha256: "c".repeat(64),
        simulated: true,
        project_preparation_sha256: None,
    }
}
#[test]
fn property_validation_fresh_open_preserves_unfinished_step_as_interrupted() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("operation");
    let mut store = OperationStore::create(&path, binding()).unwrap();
    store
        .checkpoint(Checkpoint::StepStarted {
            index: 0,
            mutation: false,
        })
        .unwrap();
    store
        .checkpoint(Checkpoint::StepFinished { index: 0 })
        .unwrap();
    assert!(
        store
            .checkpoint(Checkpoint::StepStarted {
                index: 1,
                mutation: true
            })
            .is_err(),
        "no recovery must refuse mutation admission"
    );
    store
        .checkpoint(Checkpoint::StepStarted {
            index: 1,
            mutation: false,
        })
        .unwrap();
    drop(store);
    let reopened = OperationStore::open(&path, &binding()).unwrap();
    assert!(!reopened.mutation_possible());
    assert_eq!(reopened.outcome(), LocalOutcome::Interrupted);
    assert!(reopened.recovery().is_none());
}
#[test]
fn property_validation_exclusive_and_create_new() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("operation");
    let store = OperationStore::create(&path, binding()).unwrap();
    assert!(OperationStore::open(&path, &binding()).is_err());
    assert!(OperationStore::create(&path, binding()).is_err());
    drop(store);
    assert!(OperationStore::open(&path, &binding()).is_ok());
}
#[test]
fn property_validation_binding_changes_are_refused_on_reopen() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("operation");
    drop(OperationStore::create(&path, binding()).unwrap());
    for field in [
        "target",
        "application",
        "profile",
        "source",
        "plan",
        "serial",
    ] {
        let mut wrong = binding();
        match field {
            "target" => wrong.target = "1.1.8".into(),
            "application" => wrong.application[4] += 1,
            "profile" => wrong.profile_sha256 = "d".repeat(64),
            "source" => wrong.source_sha256 = "d".repeat(64),
            "plan" => wrong.plan_sha256 = "d".repeat(64),
            _ => wrong.serial[5] += 1,
        }
        assert!(
            OperationStore::open(&path, &wrong).is_err(),
            "wrong binding admitted"
        );
    }
}
#[test]
fn property_validation_corrupt_missing_or_unknown_records_refuse() {
    for fault in ["corrupt", "gap", "tail", "extra"] {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("operation");
        let mut store = OperationStore::create(&path, binding()).unwrap();
        store.checkpoint(Checkpoint::Cancelled).unwrap();
        drop(store);
        match fault {
            "corrupt" => fs::write(path.join("checkpoint-00000001.json"), b"{}").unwrap(),
            "gap" => fs::remove_file(path.join("checkpoint-00000000.json")).unwrap(),
            "tail" => fs::remove_file(path.join("checkpoint-00000001.json")).unwrap(),
            _ => fs::write(path.join("checkpoint-unknown.json"), b"{}").unwrap(),
        }
        assert!(OperationStore::open(&path, &binding()).is_err());
    }
}
#[test]
fn property_validation_summary_is_an_allowlist_not_private_receipt_serialization() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("operation");
    let mut store = OperationStore::create(&path, binding()).unwrap();
    store.checkpoint(Checkpoint::Cancelled).unwrap();
    let summary = serde_json::to_value(store.shareable_report()).unwrap();
    let object = summary.as_object().unwrap();
    let mut keys: Vec<_> = object.keys().map(String::as_str).collect();
    keys.sort();
    assert_eq!(
        keys,
        [
            "completed_operations",
            "failure_diagnostics",
            "final_step_failures",
            "format",
            "kind",
            "mutation_possible",
            "outcome",
            "simulated",
            "support"
        ]
    );
    assert_eq!(summary["support"], "STRONG HYPOTHESIS SIMULATOR TESTED");
    assert_eq!(summary["outcome"], "cancelled");
    assert_eq!(summary["format"], 3);
    assert_eq!(summary["failure_diagnostics"], serde_json::json!([]));
    assert_eq!(summary["final_step_failures"], serde_json::json!([]));
}
#[test]
fn property_validation_owner_only_files_and_no_invalid_finish() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("nested/operation");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut store = OperationStore::create(&path, binding()).unwrap();
    assert!(store
        .checkpoint(Checkpoint::StepFinished { index: 0 })
        .is_err());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for entry in fs::read_dir(&path).unwrap() {
            assert_eq!(
                entry.unwrap().metadata().unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o700
        );
    }
}
