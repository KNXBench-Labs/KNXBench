//! Defaults profiles v1/v2 refuse new physical execution; recovery of existing operations stays.
use std::sync::atomic::AtomicBool;

use knx_app::property_validation::{self as app, Checkpoint, LocalOutcome, OperationStore};
use knx_app::property_validation_bundle::PreparedBundle;
use knx_core::commissioning::mutation::{required_confirmation_phrase, WriteScope};
use knx_net::commissioning::hardware_release::required_release_phrase;
use knx_net::commissioning::simulator::SimulatedDevice;

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

/// The synthetic fixture plan, bound to the named defaults profile version.
fn fixture_version(version: u32) -> (SimulatedDevice, PreparedBundle) {
    let (device, mut bundle) = fixture();
    if version != 1 {
        bundle.binding.profile_version = version;
        bundle.binding.profile_id = format!("synthetic-defaults-v{version}");
    }
    let bundle = PreparedBundle::new(bundle.binding, bundle.normalized_plan).unwrap();
    (device, bundle)
}

#[tokio::test]
async fn superseded_defaults_hardware_execution_is_refused_before_any_frame_or_checkpoint() {
    let temp = tempfile::tempdir().unwrap();
    for version in [1, 2] {
        let (device, bundle) = fixture_version(version);
        assert_eq!(bundle.binding.profile_version, version);
        let rebound = bundle.rebind_hardware(SERIAL).unwrap();
        let (download, release) = phrases(&rebound);
        let path = temp.path().join(format!("v{version}"));
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
        assert!(error.contains(&format!("profile v{version}")), "{error}");
        assert!(error.contains("v3"), "{error}");
        assert!(device.seen().is_empty(), "refused before any frame");
        assert_eq!(journal_files(&path), before, "no checkpoint written");
        assert!(store.recovery().is_none());
        assert!(!store.mutation_possible());
    }

    // The same synthetic plan under profile version 3 passes this rule: the executor
    // starts, journals, and only the in-process simulator's hardware refusal stops it.
    let (device, bundle) = fixture_version(3);
    let rebound = bundle.rebind_hardware(SERIAL).unwrap();
    let (download, release) = phrases(&rebound);
    let path = temp.path().join("v3");
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
    assert!(!error.contains("profile v"), "{error}");
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
async fn superseded_defaults_hardware_recovery_of_an_existing_operation_is_still_admitted() {
    for version in [1, 2] {
        recovery_is_admitted(version).await;
    }
}

async fn recovery_is_admitted(version: u32) {
    let (device, bundle) = fixture_version(version);
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
    assert_eq!(rebound.binding.profile_version, version);
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
            local_diagnostic: format!("synthetic existing v{version} operation"),
            pointer_failure: None,
            final_step: None,
            allocation_evidence: None,
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
    assert!(!error.contains("profile v"), "{error}");
    assert_eq!(store.outcome(), LocalOutcome::RestoreFailed);
    let journal = journal_text(&path);
    assert!(journal.contains("restore_started"), "{journal}");
    assert!(journal.contains("NotASimulator"), "{journal}");
    assert!(store.recovery().is_some(), "original recovery retained");
}

/// Format-1 admission binds each defaults version to its own id suffix, also after a JSON roundtrip.
#[test]
fn format1_bundles_admit_only_known_defaults_versions_with_matching_id_suffix() {
    for (version, id) in [(1, "anything"), (2, "x-defaults-v2"), (3, "x-defaults-v3")] {
        let mut bundle = fixture_version(version).1;
        bundle.binding.profile_id = id.into();
        let bundle = PreparedBundle::new(bundle.binding, bundle.normalized_plan).unwrap();
        bundle.rebind_hardware(SERIAL).unwrap();
    }
    for (version, id) in [
        (3, "x-defaults-v2"),
        (3, "up258d12-configured-v3"),
        (2, "x-defaults-v3"),
        (4, "x-defaults-v4"),
        (0, "x-defaults-v3"),
    ] {
        let mut binding = fixture_version(3).1.binding;
        binding.profile_version = version;
        binding.profile_id = id.into();
        let plan = fixture_version(3).1.normalized_plan;
        assert!(
            PreparedBundle::new(binding.clone(), plan.clone()).is_err(),
            "direct v{version} {id}"
        );
        // A deserialized bundle bypasses the constructor; plan and rebind still refuse.
        let mut value = serde_json::to_value(fixture_version(3).1).unwrap();
        value["binding"] = serde_json::to_value(&binding).unwrap();
        let read: PreparedBundle = serde_json::from_value(value).unwrap();
        assert!(read.plan().is_err(), "deserialized v{version} {id}");
        assert!(
            read.rebind_hardware(SERIAL).is_err(),
            "rebind v{version} {id}"
        );
    }
}
