//! Hardware-kind failure/barrier tests run only against a persistent simulator.
use super::*;

struct BackupBarrier<'a> {
    device: &'a SimulatedDevice,
    operation: &'a std::path::Path,
    cancel: &'a AtomicBool,
}
impl ManagementTransport for BackupBarrier<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device)
    }
    fn target_kind(&self) -> TargetKind {
        TargetKind::Hardware
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        service: ApplicationService,
    ) -> Result<(), BusError> {
        let mut unload = false;
        if let ApplicationService::PropertyValueWrite {
            property_id, data, ..
        } = &service
        {
            let head: serde_json::Value =
                serde_json::from_slice(&std::fs::read(self.operation.join("HEAD")).unwrap())
                    .unwrap();
            let sequence = head["sequence"].as_u64().unwrap();
            let recovery = (0..=sequence)
                .find_map(|n| {
                    let bytes =
                        std::fs::read(self.operation.join(format!("checkpoint-{n:08}.json")))
                            .unwrap();
                    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
                    (value["checkpoint"]["event"] == "recovery_acquired").then_some(value)
                })
                .expect(
                    "full actual-state recovery must be readable BEFORE ANY setup or load mutation",
                );
            let allocations = recovery["checkpoint"]["recovery"]["allocations"]
                .as_array()
                .unwrap();
            assert_eq!(
                allocations
                    .iter()
                    .map(|r| r["data"].as_array().unwrap().len())
                    .sum::<usize>(),
                48
            );
            assert!(allocations.iter().all(|r| r["data"]
                .as_array()
                .unwrap()
                .iter()
                .all(|b| b == 0xa5)));
            unload = *property_id == 5 && data.first() == Some(&4);
        }
        let result =
            ManagementTransport::send_frame(self.device, destination, transport, service).await;
        if unload {
            self.cancel.store(true, std::sync::atomic::Ordering::SeqCst);
        }
        result
    }
}

#[tokio::test]
async fn cancel_after_unload_keeps_backup_and_restore_is_explicit_once() {
    let (dir, conn, project, profile) = inputs();
    let device = old_device(&conn, &project, &profile);
    let old = device.system2_snapshot();
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "cancel-test",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let bundle = observe_project_hardware(
        &Hardware(&device),
        source,
        integrated_timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    let admission =
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &bundle).unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    let cancel = AtomicBool::new(false);
    let phrase = bundle.consent_phrase();
    let hypothesis = bundle.acceptance_phrase();
    let download = required_confirmation_phrase(device.address(), WriteScope::Download);
    let risk = knx_net::commissioning::hardware_release::required_release_phrase(device.address());
    let c = HardwareConfirmations {
        operation: &phrase,
        hypothesis: &hypothesis,
        download: &download,
        risk: &risk,
    };
    assert!(execute_project_hardware(
        &BackupBarrier {
            device: &device,
            operation: &path,
            cancel: &cancel
        },
        &admission,
        &mut store,
        integrated_timing(),
        &c,
        &cancel
    )
    .await
    .is_err());
    assert_eq!(store.outcome(), LocalOutcome::Cancelled);
    assert!(store.mutation_possible());
    let original = store.recovery().unwrap().clone();
    assert_ne!(device.system2_snapshot(), old);
    drop(store);
    let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
    let phrase = bundle.restore_phrase();
    let wrong = HardwareRecoveryConfirmations {
        recovery: &phrase,
        download: &download,
        risk: "wrong",
    };
    let frames = device.seen().len();
    assert!(restore_project_hardware(
        &Hardware(&device),
        &bundle,
        &mut reopened,
        integrated_timing(),
        &wrong,
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    assert_eq!(device.seen().len(), frames);
    assert!(reopened.progress().restore_available);
    let c = HardwareRecoveryConfirmations {
        recovery: &phrase,
        download: &download,
        risk: &risk,
    };
    restore_project_hardware(
        &Hardware(&device),
        &bundle,
        &mut reopened,
        integrated_timing(),
        &c,
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    assert_eq!(device.system2_snapshot(), old);
    assert_eq!(reopened.recovery(), Some(&original));
    assert!(restore_project_hardware(
        &Hardware(&device),
        &bundle,
        &mut reopened,
        integrated_timing(),
        &c,
        &AtomicBool::new(false)
    )
    .await
    .is_err());
}

#[tokio::test]
async fn destructive_unload_cannot_claim_successful_hardware_recovery() {
    let (dir, conn, project, profile) = inputs();
    let device = old_device_with_config(
        &conn,
        &project,
        &profile,
        SimulatorConfig {
            experimental_destroy_allocation_memory_on_unload: true,
            ..Default::default()
        },
    );
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "destructive",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let bundle = observe_project_hardware(
        &Hardware(&device),
        source,
        integrated_timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    let admission =
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &bundle).unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    let phrase = bundle.consent_phrase();
    let hypothesis = bundle.acceptance_phrase();
    let download = required_confirmation_phrase(device.address(), WriteScope::Download);
    let risk = knx_net::commissioning::hardware_release::required_release_phrase(device.address());
    let c = HardwareConfirmations {
        operation: &phrase,
        hypothesis: &hypothesis,
        download: &download,
        risk: &risk,
    };
    assert!(execute_project_hardware(
        &Hardware(&device),
        &admission,
        &mut store,
        integrated_timing(),
        &c,
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    let original = store.recovery().unwrap().clone();
    drop(store);
    let mut reopened = OperationStore::open(&path, &bundle.binding).unwrap();
    let phrase = bundle.restore_phrase();
    let c = HardwareRecoveryConfirmations {
        recovery: &phrase,
        download: &download,
        risk: &risk,
    };
    assert!(restore_project_hardware(
        &Hardware(&device),
        &bundle,
        &mut reopened,
        integrated_timing(),
        &c,
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    assert_eq!(reopened.outcome(), LocalOutcome::RestoreFailed);
    assert_eq!(reopened.recovery(), Some(&original));
    assert!(!reopened.progress().restore_available);
}

#[tokio::test]
async fn persistence_failure_at_actual_backup_barrier_forbids_implicit_setup_write() {
    let (dir, conn, project, profile) = inputs();
    let device = old_device(&conn, &project, &profile);
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "disk-failure",
        RecoveryTrack::ExternalEts,
    )
    .unwrap();
    let bundle = observe_project_hardware(
        &Hardware(&device),
        source,
        integrated_timing(),
        &AtomicBool::new(false),
    )
    .await
    .unwrap();
    let admission =
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &bundle).unwrap();
    let path = dir.path().join("operation");
    let mut store = OperationStore::create(&path, bundle.binding.clone()).unwrap();
    // Connect started/finished + HardwareCheck started precede RecoveryAcquired.
    std::fs::create_dir(path.join("checkpoint-00000004.json")).unwrap();
    let before = device.system2_snapshot();
    let phrase = bundle.consent_phrase();
    let hypothesis = bundle.acceptance_phrase();
    let download = required_confirmation_phrase(device.address(), WriteScope::Download);
    let risk = knx_net::commissioning::hardware_release::required_release_phrase(device.address());
    let c = HardwareConfirmations {
        operation: &phrase,
        hypothesis: &hypothesis,
        download: &download,
        risk: &risk,
    };
    assert!(execute_project_hardware(
        &Hardware(&device),
        &admission,
        &mut store,
        integrated_timing(),
        &c,
        &AtomicBool::new(false)
    )
    .await
    .is_err());
    assert_eq!(device.system2_snapshot(), before);
    assert!(store.recovery().is_none());
    assert!(!store.mutation_possible());
}
