//! The final Disconnect step names which of its own operations failed, never an earlier one.

use super::property_recovery::{auth, establish_shape, fixture, timing, FinalReference};
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::load_state::LoadState;
use knx_core::commissioning::mutation::TargetKind;
use knx_core::commissioning::properties::{
    ObjectIndex, PID_MANUFACTURER_ID, PID_MAX_APDU_LENGTH, PID_PROGRAM_VERSION,
};
use knx_core::IndividualAddress;
use knx_net::cemi::{ApplicationService, Destination, Tpci};
use knx_net::client::{BusError, TunnelEvent};
use knx_net::commissioning::final_step::{FinalStepFailure, FinalStepKind, FinalStepStage};
use knx_net::commissioning::property_download::{
    restore_experimental_property_plan, run_experimental_property_plan, PropertySimulationError,
    PropertySimulationFailure,
};
use knx_net::commissioning::property_recovery::PropertyJournalEvent;
use knx_net::commissioning::simulator::SimulatedDevice;
use knx_net::management::ManagementTransport;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::broadcast;

const FINAL_STEP: usize = 7;
const PID_DEVICE_CONTROL: u8 = 14;
const PID_LOAD_STATE_CONTROL: u8 = 5;
const PID_TABLE_REFERENCE: u8 = 7;

/// One synthetic bus fault, applied only once armed; the device itself stays honest.
#[derive(Clone, Copy)]
enum Fault {
    /// Swallow the request: the session sees silence and times out.
    DropPropertyRead {
        object: u8,
        property: u8,
    },
    DropMemoryRead,
    /// Ask for one octet fewer, so the answer is shorter than the region.
    ShortMemoryRead,
    DropDeviceControlWrite,
    /// Answer the Device Control read-back (the third exchange) with a flipped Verify Mode bit.
    FlipDeviceControlReadback,
}
struct Faulty<'a> {
    device: &'a SimulatedDevice,
    armed: Arc<AtomicBool>,
    fault: Fault,
    /// Device Control requests sent while armed; the third one is the read-back.
    control_requests: Arc<AtomicUsize>,
    readback_pending: Arc<AtomicBool>,
}
impl<'a> Faulty<'a> {
    fn new(device: &'a SimulatedDevice, fault: Fault, armed: bool) -> Self {
        Self {
            device,
            armed: Arc::new(AtomicBool::new(armed)),
            fault,
            control_requests: Arc::default(),
            readback_pending: Arc::default(),
        }
    }
}
impl ManagementTransport for Faulty<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        let mut upstream = ManagementTransport::subscribe(self.device);
        let (sender, receiver) = broadcast::channel(256);
        let (pending, fault) = (self.readback_pending.clone(), self.fault);
        tokio::spawn(async move {
            loop {
                match upstream.recv().await {
                    Ok(TunnelEvent::Telegram(mut frame)) => {
                        if let (
                            Fault::FlipDeviceControlReadback,
                            true,
                            ApplicationService::PropertyValueResponse {
                                object_index: 0,
                                property_id: PID_DEVICE_CONTROL,
                                data,
                                ..
                            },
                        ) = (fault, pending.load(Ordering::SeqCst), &mut frame.service)
                        {
                            data[0] ^= 0x04;
                        }
                        let _ = sender.send(TunnelEvent::Telegram(frame));
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
        TargetKind::Simulator
    }
    async fn send_frame(
        &self,
        destination: Destination,
        transport: Tpci,
        mut service: ApplicationService,
    ) -> Result<(), BusError> {
        if self.armed.load(Ordering::SeqCst) {
            if matches!(
                &service,
                ApplicationService::PropertyValueRead {
                    object_index: 0,
                    property_id: PID_DEVICE_CONTROL,
                    ..
                } | ApplicationService::PropertyValueWrite {
                    object_index: 0,
                    property_id: PID_DEVICE_CONTROL,
                    ..
                }
            ) && self.control_requests.fetch_add(1, Ordering::SeqCst) == 2
            {
                // Setup is over: read, write, now the read-back.
                self.readback_pending.store(true, Ordering::SeqCst);
            }
            let swallow = match (&mut service, self.fault) {
                (
                    ApplicationService::PropertyValueRead {
                        object_index,
                        property_id,
                        ..
                    },
                    Fault::DropPropertyRead { object, property },
                ) => *object_index == object && *property_id == property,
                (
                    ApplicationService::MemoryRead { .. }
                    | ApplicationService::UserMemoryRead { .. },
                    Fault::DropMemoryRead,
                ) => true,
                (
                    ApplicationService::PropertyValueWrite {
                        object_index: 0,
                        property_id: PID_DEVICE_CONTROL,
                        ..
                    },
                    Fault::DropDeviceControlWrite,
                ) => true,
                (
                    ApplicationService::MemoryRead { number, .. }
                    | ApplicationService::UserMemoryRead { number, .. },
                    Fault::ShortMemoryRead,
                ) => {
                    if *number > 1 {
                        *number -= 1;
                    }
                    false
                }
                _ => false,
            };
            if swallow {
                return Ok(());
            }
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

fn arm_at_final_step(
    armed: Arc<AtomicBool>,
) -> impl FnMut(PropertyJournalEvent<'_>) -> Result<(), String> {
    move |event| {
        if let PropertyJournalEvent::OperationStarting {
            operation_index: FINAL_STEP,
            ..
        } = event
        {
            armed.store(true, Ordering::SeqCst);
        }
        Ok(())
    }
}

async fn run_with(fault: Fault, armed_from_start: bool) -> PropertySimulationError {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = Faulty::new(&device, fault, armed_from_start);
    let armed = transport.armed.clone();
    run_experimental_property_plan(
        &transport,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut arm_at_final_step(armed),
        &|| false,
    )
    .await
    .expect_err("the injected fault must stop the run")
}

fn failure(kind: FinalStepKind, stage: FinalStepStage) -> Option<FinalStepFailure> {
    Some(FinalStepFailure { kind, stage })
}

#[tokio::test]
async fn a_read_that_times_out_in_each_final_substage_is_a_final_read_failure() {
    let silent = |object, property| Fault::DropPropertyRead { object, property };
    for (fault, stage) in [
        (silent(0, PID_MAX_APDU_LENGTH), FinalStepStage::Limit),
        (Fault::DropMemoryRead, FinalStepStage::Allocation),
        (
            silent(3, PID_LOAD_STATE_CONTROL),
            FinalStepStage::LoadedState,
        ),
        (silent(3, PID_TABLE_REFERENCE), FinalStepStage::Pointer),
        (
            silent(3, PID_PROGRAM_VERSION),
            FinalStepStage::ProgramVersion,
        ),
        (
            silent(0, PID_DEVICE_CONTROL),
            FinalStepStage::VerifyModeRead,
        ),
    ] {
        let error = run_with(fault, false).await;
        assert_eq!(error.operation_index, Some(FINAL_STEP));
        assert!(
            matches!(error.failure, PropertySimulationFailure::Session(_)),
            "{stage:?}: {:?}",
            error.failure
        );
        assert_eq!(
            error.final_step,
            failure(FinalStepKind::ReadFailed, stage),
            "{stage:?}"
        );
    }
}

#[tokio::test]
async fn a_short_final_memory_answer_is_a_final_read_failure_not_a_mismatch() {
    let error = run_with(Fault::ShortMemoryRead, false).await;
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Recovery(_)
    ));
    assert_eq!(
        error.final_step,
        failure(FinalStepKind::ReadFailed, FinalStepStage::Allocation)
    );
}

#[tokio::test]
async fn a_lost_verify_mode_write_is_the_final_write_failure() {
    let error = run_with(Fault::DropDeviceControlWrite, false).await;
    assert_eq!(error.operation_index, Some(FINAL_STEP));
    assert_eq!(
        error.final_step,
        failure(FinalStepKind::WriteFailed, FinalStepStage::VerifyModeWrite)
    );
}

#[tokio::test]
async fn a_verify_mode_bit_that_did_not_restore_is_a_verification_mismatch() {
    let error = run_with(Fault::FlipDeviceControlReadback, false).await;
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Recovery(_)
    ));
    assert_eq!(
        error.final_step,
        failure(
            FinalStepKind::VerificationMismatch,
            FinalStepStage::VerifyModeReadback
        )
    );
}

/// The device itself answers differently at the final step; every read is honest.
async fn mismatch_after(change: impl Fn(&SimulatedDevice)) -> PropertySimulationError {
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if let PropertyJournalEvent::OperationStarting {
                operation_index: FINAL_STEP,
                ..
            } = event
            {
                change(&device);
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .expect_err("the changed device must fail final verification")
}

#[tokio::test]
async fn honest_reads_that_disagree_are_a_verification_mismatch_per_substage() {
    let error = mismatch_after(|device| device.preset_memory(0x3C02, &[0xEE])).await;
    assert_eq!(
        error.final_step,
        failure(
            FinalStepKind::VerificationMismatch,
            FinalStepStage::Allocation
        )
    );
    let error =
        mismatch_after(|device| device.preset_load_state(ObjectIndex::new(3), LoadState::Unloaded))
            .await;
    assert_eq!(
        error.final_step,
        failure(
            FinalStepKind::VerificationMismatch,
            FinalStepStage::LoadedState
        )
    );
    let error = mismatch_after(|device| {
        device.preset_property(3, PID_PROGRAM_VERSION, &[0, 9, 9, 9, 9]);
    })
    .await;
    assert_eq!(
        error.final_step,
        failure(
            FinalStepKind::VerificationMismatch,
            FinalStepStage::ProgramVersion
        )
    );
}

#[tokio::test]
async fn a_journal_failure_in_the_middle_of_the_final_step_is_never_classified() {
    // The verification passed; the pointer observation cannot be made durable before the
    // Verify Mode write. That is a store failure, not a read, write or mismatch.
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let transport = FinalReference {
        device: &device,
        armed: Default::default(),
        property: 7,
        elements: 1,
        replacement: Some(vec![0, 0, 0x3D, 0]),
        arm_on: 2,
    };
    let mut observed = false;
    let error = run_experimental_property_plan(
        &transport,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| match event {
            PropertyJournalEvent::FinalPointerObservations(_) => {
                observed = true;
                Err("durable store failed".into())
            }
            _ => Ok(()),
        },
        &|| false,
    )
    .await
    .unwrap_err();
    assert!(observed, "the failing journal write was reached");
    assert!(
        matches!(error.failure, PropertySimulationFailure::Persistence(_)),
        "{:?}",
        error.failure
    );
    assert_eq!(error.operation_index, Some(FINAL_STEP));
    assert_eq!(error.final_step, None);
}

#[tokio::test]
async fn errors_before_or_beside_the_final_operations_are_never_classified() {
    // A read timeout in the identity check of the first step.
    let error = run_with(
        Fault::DropPropertyRead {
            object: 0,
            property: PID_MANUFACTURER_ID,
        },
        true,
    )
    .await;
    assert_eq!(error.operation_index, Some(0));
    assert_eq!(error.final_step, None);
    // A lost Device Control write at setup is not the final write.
    let error = run_with(Fault::DropDeviceControlWrite, true).await;
    assert_eq!(error.operation_index, Some(1));
    assert_eq!(error.final_step, None);
    // A durable-journal failure at the start or end of the final step is not a verification
    // (the one inside it has its own test below).
    for stop_at in ["start", "end"] {
        let (device, plan, preconditions) = fixture();
        establish_shape(&device, &plan).await;
        let error = run_experimental_property_plan(
            &device,
            &plan,
            auth(&device),
            AuthorisationPlan::Skip,
            timing(),
            &preconditions,
            &mut |event| match (event, stop_at) {
                (
                    PropertyJournalEvent::OperationStarting {
                        operation_index: FINAL_STEP,
                        ..
                    },
                    "start",
                )
                | (
                    PropertyJournalEvent::OperationCompleted {
                        operation_index: FINAL_STEP,
                        ..
                    },
                    "end",
                ) => Err("durable store failed".into()),
                _ => Ok(()),
            },
            &|| false,
        )
        .await
        .unwrap_err();
        assert!(
            matches!(error.failure, PropertySimulationFailure::Persistence(_)),
            "{stop_at}: {:?}",
            error.failure
        );
        assert_eq!(error.operation_index, Some(FINAL_STEP), "{stop_at}");
        assert_eq!(error.final_step, None, "{stop_at}");
    }
    // Cancellation requested just before the final step starts.
    let cancel = std::cell::Cell::new(false);
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let error = run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            if let PropertyJournalEvent::OperationCompleted {
                operation_index: 6, ..
            } = event
            {
                cancel.set(true);
            }
            Ok(())
        },
        &|| cancel.get(),
    )
    .await
    .unwrap_err();
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Cancelled
    ));
    assert_eq!(error.final_step, None);
}

#[tokio::test]
async fn a_failed_explicit_restore_classifies_its_own_final_step() {
    use std::cell::Cell;
    let (device, plan, preconditions) = fixture();
    establish_shape(&device, &plan).await;
    let abort = Cell::new(false);
    let mut saved = None;
    run_experimental_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &preconditions,
        &mut |event| {
            match event {
                PropertyJournalEvent::RecoveryAcquired(snapshot) => saved = Some(snapshot.clone()),
                PropertyJournalEvent::OperationCompleted {
                    operation_index: 5, ..
                } => abort.set(true),
                _ => {}
            }
            Ok(())
        },
        &|| abort.get(),
    )
    .await
    .unwrap_err();
    let snapshot = saved.unwrap();
    let transport = Faulty::new(&device, Fault::DropMemoryRead, false);
    let armed = transport.armed.clone();
    let error = restore_experimental_property_plan(
        &transport,
        &plan,
        &snapshot,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &mut arm_at_final_step(armed),
        &|| false,
    )
    .await
    .unwrap_err();
    assert_eq!(error.operation_index, Some(FINAL_STEP));
    assert_eq!(
        error.final_step,
        failure(FinalStepKind::ReadFailed, FinalStepStage::Allocation)
    );
}
