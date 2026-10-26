//! The store's send-boundary hook runs once at the durable recovery barrier, before VerifyMode.
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

use knx_app::property_validation::{self as app, LocalOutcome, OperationStore};
use knx_core::commissioning::{load_state::LoadState, properties::ObjectIndex};

use super::{establish, fixture, timing};

#[tokio::test]
async fn send_boundary_runs_once_after_the_recovery_barrier_and_before_any_mutation() {
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let device = Arc::new(device);
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    let calls = Arc::new(AtomicUsize::new(0));
    let (seen, observed) = (Arc::clone(&calls), Arc::clone(&device));
    let journal = path.clone();
    store.on_send_boundary(Box::new(move || {
        seen.fetch_add(1, Ordering::SeqCst);
        // Untouched device: no VerifyMode, no load control, no memory write.
        assert!(!observed.verify_mode());
        assert_eq!(observed.load_state(ObjectIndex::new(3)), LoadState::Loaded);
        assert_eq!(
            observed.memory(0x3c00, 4),
            [Some(9), Some(8), Some(7), Some(6)]
        );
        // The recovery checkpoint is already on disk.
        assert!(std::fs::read_dir(&journal).unwrap().count() >= 2);
        Ok(())
    }));
    app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn a_failing_send_boundary_stops_the_operation_before_any_mutation() {
    let (device, bundle) = fixture();
    establish(&device, &bundle).await;
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("op");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    store.on_send_boundary(Box::new(|| Err(std::io::Error::other("journal refused"))));
    let result = app::execute_simulated(
        &device,
        &bundle,
        &mut store,
        timing(),
        &AtomicBool::new(false),
    )
    .await;
    assert!(result.is_err());
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(store.recovery().is_some());
    assert!(!device.verify_mode());
    assert_eq!(device.load_state(ObjectIndex::new(3)), LoadState::Loaded);
    assert_eq!(
        device.memory(0x3c00, 4),
        [Some(9), Some(8), Some(7), Some(6)]
    );
}
