//! Wrong-width PID 7 replies stay prewrite refusals with private, durable diagnostics.
use super::*;

struct ShortPointer<'a>(&'a SimulatedDevice);
impl ManagementTransport for ShortPointer<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.0)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        let mut upstream = ManagementTransport::subscribe(self.0);
        let (sender, receiver) = broadcast::channel(256);
        tokio::spawn(async move {
            loop {
                match upstream.recv().await {
                    Ok(TunnelEvent::Telegram(mut frame)) => {
                        if let ApplicationService::PropertyValueResponse {
                            object_index: 3,
                            property_id: 7,
                            nr_of_elem,
                            data,
                            ..
                        } = &mut frame.service
                        {
                            *nr_of_elem = 2;
                            *data = vec![0x3c, 0x00];
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
        ManagementTransport::send_frame(self.0, destination, transport, service).await
    }
}

#[tokio::test]
async fn wrong_width_pointer_is_retained_in_private_journal_not_shareable_report() {
    let (dir, conn, project, profile) = inputs();
    let device = old_device(&conn, &project, &profile);
    let old = device.system2_snapshot();
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "pid7-width-test",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let bundle = observe_project_hardware(&Hardware(&device), source, integrated_timing(), &cancel)
        .await
        .unwrap();
    let admission =
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &bundle).unwrap();
    let operation = dir.path().join("pid7-width-operation");
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
    let before = device.seen().len();
    let result = execute_project_hardware(
        &ShortPointer(&device),
        &admission,
        &mut store,
        integrated_timing(),
        &confirmations,
        &cancel,
    )
    .await;
    assert!(
        result.is_err(),
        "a short pointer must refuse before any write"
    );
    assert_eq!(store.outcome(), LocalOutcome::Failed);
    assert!(!store.progress().mutation_possible);
    assert!(!store.progress().backup_available);
    assert!(store.recovery().is_none());
    assert_eq!(device.system2_snapshot(), old);
    use knx_net::commissioning::simulator::Seen;
    assert!(device.seen()[before..]
        .iter()
        .all(|s| !matches!(s, Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. })));
    assert!(matches!(device.seen().last(), Some(Seen::Disconnect)));
    drop(store);

    let reopened = OperationStore::open(&operation, &bundle.binding).unwrap();
    assert_eq!(reopened.outcome(), LocalOutcome::Failed);
    let head: serde_json::Value =
        serde_json::from_slice(&std::fs::read(operation.join("HEAD")).unwrap()).unwrap();
    let last = head["sequence"].as_u64().unwrap();
    let checkpoint_path = operation.join(format!("checkpoint-{last:08}.json"));
    let checkpoint: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&checkpoint_path).unwrap()).unwrap();
    assert_eq!(checkpoint["checkpoint"]["event"], "failed");
    assert_eq!(checkpoint["checkpoint"]["code"], "identity_or_recovery");
    let diagnostic = checkpoint["checkpoint"]["local_diagnostic"]
        .as_str()
        .unwrap();
    assert!(diagnostic.contains(
        "resource pointer response is not four octets: object 3, 2 elements, 2 octets, 3c00"
    ));
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(checkpoint_path)
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
    }
    let report = serde_json::to_value(reopened.shareable_report()).unwrap();
    assert_eq!(report["outcome"], "failed");
    assert_eq!(report["mutation_possible"], false);
    assert!(report.get("local_diagnostic").is_none());
    assert!(!report.to_string().contains("3c00"));
}
