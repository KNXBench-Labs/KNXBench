//! A two-octet object 1 PID 7 reply must finish the download and recover with the raw bytes.
use super::*;
use std::sync::{Arc, Mutex};

/// Field report (issue 30): object 1 answers PID 7 with one element of two octets.
const FIELD_OBJECT: u8 = 1;

/// Hardware wrapper that cuts every object 1 PID 7 answer to its last two octets and keeps
/// the element count; the untouched four-octet answers are recorded for the pointer check.
struct TwoOctetPointer<'a> {
    device: &'a SimulatedDevice,
    full: Arc<Mutex<Vec<Vec<u8>>>>,
}
impl ManagementTransport for TwoOctetPointer<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        let mut upstream = ManagementTransport::subscribe(self.device);
        let (sender, receiver) = broadcast::channel(256);
        let full = Arc::clone(&self.full);
        tokio::spawn(async move {
            loop {
                match upstream.recv().await {
                    Ok(TunnelEvent::Telegram(mut frame)) => {
                        if let ApplicationService::PropertyValueResponse {
                            object_index: FIELD_OBJECT,
                            property_id: 7,
                            nr_of_elem,
                            data,
                            ..
                        } = &mut frame.service
                        {
                            if *nr_of_elem == 1 && data.len() == 4 {
                                full.lock().unwrap().push(data.clone());
                                *data = data[2..].to_vec();
                            }
                        }
                        if sender.send(TunnelEvent::Telegram(frame)).is_err() {
                            break;
                        }
                    }
                    Ok(TunnelEvent::Closed) => {
                        let _ = sender.send(TunnelEvent::Closed);
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        receiver
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
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

#[tokio::test]
async fn two_octet_object1_pointer_finishes_and_reopened_recovery_restores_without_padding() {
    let (dir, conn, project, profile) = inputs();
    let device = old_device(&conn, &project, &profile);
    let old = device.system2_snapshot();
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "pid7-two-octet-test",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let bundle = observe_project_hardware(&Hardware(&device), source, integrated_timing(), &cancel)
        .await
        .unwrap();
    let admission =
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &bundle).unwrap();
    let operation = dir.path().join("pid7-two-octet-operation");
    let mut store = OperationStore::create(&operation, bundle.binding.clone()).unwrap();
    let operation_phrase = bundle.consent_phrase();
    let download = required_confirmation_phrase(device.address(), WriteScope::Download);
    let release =
        knx_net::commissioning::hardware_release::required_release_phrase(device.address());
    let hypothesis = bundle.acceptance_phrase();
    let confirmations = HardwareConfirmations {
        operation: &operation_phrase,
        hypothesis: &hypothesis,
        download: &download,
        risk: &release,
    };
    let full = Arc::new(Mutex::new(Vec::new()));
    let transport = TwoOctetPointer {
        device: &device,
        full: Arc::clone(&full),
    };
    let result = execute_project_hardware(
        &transport,
        &admission,
        &mut store,
        integrated_timing(),
        &confirmations,
        &cancel,
    )
    .await;
    // The wrapper only shortens a four-octet answer whose source pointer ends in 0116.
    let seen = full.lock().unwrap().clone();
    assert!(!seen.is_empty(), "object 1 PID 7 must have been read");
    assert!(
        seen.iter().all(|pointer| pointer[2..] == [0x01, 0x16]),
        "fixture object 1 pointer must end in 0116: {seen:02x?}"
    );
    assert!(
        result.is_ok(),
        "two-octet object 1 pointer must execute: {}",
        result.err().map(|e| e.to_string()).unwrap_or_default()
    );
    assert_eq!(store.outcome(), LocalOutcome::Finished);
    assert!(store.progress().backup_available);
    let recovery = store.recovery().unwrap().clone();
    drop(store);

    let mut reopened = OperationStore::open(&operation, &bundle.binding).unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Finished);
    assert_eq!(reopened.recovery(), Some(&recovery));
    let recovery_phrase = bundle.restore_phrase();
    let recovery_confirmations = HardwareRecoveryConfirmations {
        recovery: &recovery_phrase,
        download: &download,
        risk: &release,
    };
    restore_project_hardware(
        &transport,
        &bundle,
        &mut reopened,
        integrated_timing(),
        &recovery_confirmations,
        &cancel,
    )
    .await
    .unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Restored);
    assert_eq!(device.system2_snapshot(), old);
}
