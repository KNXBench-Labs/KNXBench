//! Defaults profile v1 refuses new physical execution; recovery of its existing operations stays.
use std::sync::atomic::AtomicBool;

use knx_app::property_validation::{self as app, Checkpoint, LocalOutcome, OperationStore};
use knx_app::property_validation_bundle::PreparedBundle;
use knx_core::commissioning::mutation::{required_confirmation_phrase, WriteScope};
use knx_net::commissioning::hardware_release::required_release_phrase;

use super::{establish, fixture, timing, SERIAL};

fn phrases(bundle: &PreparedBundle) -> (String, String) {
    let target = bundle.plan().unwrap().target();
    (
        required_confirmation_phrase(target, WriteScope::Download),
        required_release_phrase(target),
    )
}

fn journal_files(path: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|e| {
            let e = e.unwrap();
            (
                e.file_name().to_string_lossy().into_owned(),
                std::fs::read(e.path()).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

#[tokio::test]
async fn v1_defaults_hardware_execution_is_refused_before_any_frame_or_checkpoint() {
    let (device, bundle) = fixture();
    assert_eq!(bundle.binding.profile_version, 1);
    let rebound = bundle.rebind_hardware(SERIAL).unwrap();
    let (download, release) = phrases(&rebound);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, rebound.binding.clone()).unwrap();
    let before = journal_files(&path);
    let error = app::execute_hardware(
        &device,
        &rebound,
        &mut store,
        timing(),
        &download,
        &release,
        &AtomicBool::new(false),
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(error.contains("profile v1"), "{error}");
    assert!(error.contains("v2"), "{error}");
    assert!(device.seen().is_empty(), "refused before any frame");
    assert_eq!(journal_files(&path), before, "no checkpoint written");
    assert!(store.recovery().is_none());
    assert!(!store.mutation_possible());

    // The same synthetic plan under profile version 2 passes this rule: the executor
    // starts, journals, and only the in-process simulator's hardware refusal stops it.
    let (device, mut bundle) = fixture();
    bundle.binding.profile_version = 2;
    bundle.binding.profile_id = "synthetic-defaults-v2".into();
    let bundle = PreparedBundle::new(bundle.binding, bundle.normalized_plan).unwrap();
    let rebound = bundle.rebind_hardware(SERIAL).unwrap();
    let path = temp.path().join("v2");
    let mut store = OperationStore::create(&path, rebound.binding.clone()).unwrap();
    let error = app::execute_hardware(
        &device,
        &rebound,
        &mut store,
        timing(),
        &download,
        &release,
        &AtomicBool::new(false),
    )
    .await
    .unwrap_err()
    .to_string();
    assert!(!error.contains("profile v1"), "{error}");
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(journal_text(&path).contains("NotASimulator"));
}

fn journal_text(path: &std::path::Path) -> String {
    journal_files(path)
        .into_iter()
        .map(|(_, bytes)| String::from_utf8_lossy(&bytes).into_owned())
        .collect()
}

#[tokio::test]
async fn v1_defaults_hardware_recovery_of_an_existing_operation_is_still_admitted() {
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    // An existing v1 operation: durable original recovery, then a failed download.
    let temp = tempfile::tempdir().unwrap();
    let mut simulated =
        OperationStore::create(&temp.path().join("sim"), bundle.binding.clone()).unwrap();
    app::execute_simulated(
        &device,
        &bundle,
        &mut simulated,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    assert_eq!(
        device.memory(0x3c00, 4),
        [Some(1), Some(2), Some(3), Some(4)]
    );
    let rebound = bundle.rebind_hardware(SERIAL).unwrap();
    assert_eq!(rebound.binding.profile_version, 1);
    let mut recovery = simulated.recovery().unwrap().clone();
    recovery.original_bundle = rebound.clone();
    recovery.to_snapshot(&rebound).unwrap();
    let path = temp.path().join("hardware");
    let mut store = OperationStore::create(&path, rebound.binding.clone()).unwrap();
    store
        .checkpoint(Checkpoint::RecoveryAcquired { recovery })
        .unwrap();
    store
        .checkpoint(Checkpoint::Failed {
            code: app::FailureCode::Executor,
            local_diagnostic: "synthetic existing v1 operation".into(),
        })
        .unwrap();
    drop(store);
    let mut store = OperationStore::open(&path, &rebound.binding).unwrap();
    let (download, release) = phrases(&rebound);
    let error = app::restore_hardware(
        &device,
        &rebound,
        &mut store,
        timing(),
        &rebound.restore_phrase(),
        &download,
        &release,
        &AtomicBool::new(false),
    )
    .await
    .unwrap_err()
    .to_string();
    // Admission passed: restore started and reached the transport, which (an
    // in-process simulator) refuses hardware authority. The CLI gateway test
    // completes a v1 recovery end to end over loopback.
    assert!(!error.contains("profile v1"), "{error}");
    assert_eq!(store.outcome(), LocalOutcome::RestoreFailed);
    let journal = journal_text(&path);
    assert!(journal.contains("restore_started"), "{journal}");
    assert!(journal.contains("NotASimulator"), "{journal}");
    assert!(store.recovery().is_some(), "original recovery retained");
}
