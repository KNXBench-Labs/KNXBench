//! A final allocation mismatch keeps its exact offsets, classes and both byte values.

use super::property_recovery::{auth, timing};
use super::system2_memory_boundary::{one_allocation_plan, system2_device};
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::mutation::TargetKind;
use knx_core::commissioning::properties::PID_PROGRAM_VERSION;
use knx_core::commissioning::property_download::PropertyDownloadPlan;
use knx_core::IndividualAddress;
use knx_net::cemi::{ApplicationService, Destination, Tpci};
use knx_net::client::{BusError, TunnelEvent};
use knx_net::commissioning::final_step::{FinalStepFailure, FinalStepKind, FinalStepStage};
use knx_net::commissioning::final_step_evidence::{
    AllocationMismatchEvidence, DifferenceClass, DifferingRange,
};
use knx_net::commissioning::property_download::{
    run_experimental_property_plan, run_simulated_property_plan, PropertySimulationError,
    PropertySimulationFailure,
};
use knx_net::commissioning::property_recovery::{
    ExperimentalPropertyPreconditions, PropertyJournalEvent, PropertyRecoverySnapshot,
};
use knx_net::commissioning::simulator::SimulatedDevice;
use knx_net::management::ManagementTransport;
use std::sync::atomic::{AtomicBool, Ordering};
use tokio::sync::broadcast;

const FINAL_STEP: usize = 7;
/// Allocation `3C00h`, eight octets; the plan writes four of them from `3C02h`.
const START: u16 = 0x3C00;
const PRESTATE: [u8; 8] = [9, 8, 7, 6, 5, 4, 3, 2];
const WRITE: [u8; 4] = [1, 2, 3, 4];

/// The source allocation shape, then the prestate bytes the recovery captures.
async fn established() -> (
    SimulatedDevice,
    PropertyDownloadPlan,
    ExperimentalPropertyPreconditions,
) {
    let device = system2_device();
    let (plan, preconditions) = one_allocation_plan(&device, START, 8, START + 2, WRITE.to_vec());
    run_simulated_property_plan(
        &device,
        &plan,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
    )
    .await
    .unwrap();
    device.preset_memory(u32::from(START), &PRESTATE);
    (device, plan, preconditions)
}

/// Short-answers memory reads before the first property write, so the prestate is
/// unreadable while the final verification reads normally.
struct PrestateShortRead<'a> {
    device: &'a SimulatedDevice,
    written: AtomicBool,
}
impl ManagementTransport for PrestateShortRead<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        ManagementTransport::subscribe(self.device)
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
        match &mut service {
            ApplicationService::PropertyValueWrite { .. } => {
                self.written.store(true, Ordering::SeqCst)
            }
            ApplicationService::MemoryRead { number, .. }
                if *number > 1 && !self.written.load(Ordering::SeqCst) =>
            {
                *number -= 1
            }
            _ => {}
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

/// Runs the plan, changing the honest device just before the final step; returns the
/// outcome and the recovery snapshot Net journaled.
async fn run_changed(
    transport: &impl ManagementTransport,
    device: &SimulatedDevice,
    plan: &PropertyDownloadPlan,
    preconditions: &ExperimentalPropertyPreconditions,
    change: &[(u16, &[u8])],
) -> (
    Result<(), PropertySimulationError>,
    PropertyRecoverySnapshot,
) {
    let mut snapshot = None;
    let result = run_experimental_property_plan(
        transport,
        plan,
        auth(device),
        AuthorisationPlan::Skip,
        timing(),
        preconditions,
        &mut |event| {
            match event {
                PropertyJournalEvent::RecoveryAcquired(captured) => {
                    snapshot = Some(captured.clone())
                }
                PropertyJournalEvent::OperationStarting {
                    operation_index: FINAL_STEP,
                    ..
                } => {
                    for (address, bytes) in change {
                        device.preset_memory(u32::from(*address), bytes);
                    }
                }
                _ => {}
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .map(|_| ());
    (
        result,
        snapshot.expect("recovery acquired before any write"),
    )
}

fn range(offset: usize, length: usize, class: DifferenceClass) -> DifferingRange {
    DifferingRange {
        offset,
        length,
        class,
    }
}

/// The strict outcome is exactly the pre-evidence one; returns the evidence it carried.
fn allocation_mismatch(error: PropertySimulationError) -> AllocationMismatchEvidence {
    assert_eq!(error.operation_index, Some(FINAL_STEP));
    assert_eq!(
        error.final_step,
        Some(FinalStepFailure {
            kind: FinalStepKind::VerificationMismatch,
            stage: FinalStepStage::Allocation,
        })
    );
    match &error.failure {
        PropertySimulationFailure::Recovery(reason) => assert_eq!(
            reason,
            "full allocation verification differs, including preserved/unwritten bytes"
        ),
        other => panic!("unexpected failure {other:?}"),
    }
    let debug = format!("{error:?}");
    assert!(
        !debug.contains("final_step_evidence") && !debug.contains("expected:"),
        "diagnostic text unchanged: {debug}"
    );
    *error
        .final_step_evidence
        .expect("an allocation mismatch carries its bytes")
}

const WRITTEN_MASK: [bool; 8] = [false, false, true, true, true, true, false, false];

#[tokio::test]
async fn a_differing_written_byte_is_recorded_with_its_offset_and_both_values() {
    let (device, plan, preconditions) = established().await;
    let (result, snapshot) = run_changed(
        &device,
        &device,
        &plan,
        &preconditions,
        &[(0x3C03, &[0xEE])],
    )
    .await;
    let evidence = allocation_mismatch(result.unwrap_err());
    assert_eq!(evidence.allocation_index, 0);
    assert_eq!((evidence.object, evidence.address), (3, START));
    assert_eq!(evidence.length(), 8);
    assert!(!evidence.prestate_unreadable);
    assert_eq!(evidence.expected, [9, 8, 1, 2, 3, 4, 3, 2]);
    assert_eq!(evidence.actual, [9, 8, 1, 0xEE, 3, 4, 3, 2]);
    assert_eq!(evidence.written, WRITTEN_MASK);
    assert_eq!(evidence.differing, [range(3, 1, DifferenceClass::Written)]);
    // A journal reader can replay the same comparison from the snapshot alone.
    assert_eq!(
        snapshot.allocation_mismatch(0, &plan, evidence.actual.clone()),
        Some(evidence.clone())
    );
    assert_eq!(
        snapshot.allocation_mismatch(0, &plan, evidence.expected.clone()),
        None
    );
    assert_eq!(
        snapshot.allocation_mismatch(1, &plan, evidence.actual.clone()),
        None
    );
    assert_eq!(snapshot.allocation_mismatch(0, &plan, vec![0xEE; 7]), None);
}

#[tokio::test]
async fn a_differing_preserved_byte_is_recorded_as_unwritten() {
    let (device, plan, preconditions) = established().await;
    let (result, _) = run_changed(
        &device,
        &device,
        &plan,
        &preconditions,
        &[(0x3C06, &[0xEE])],
    )
    .await;
    let evidence = allocation_mismatch(result.unwrap_err());
    assert_eq!(evidence.expected[6], 3);
    assert_eq!(evidence.actual[6], 0xEE);
    assert_eq!(
        evidence.differing,
        [range(6, 1, DifferenceClass::Unwritten)]
    );
}

#[tokio::test]
async fn one_contiguous_difference_splits_where_written_and_preserved_bytes_meet() {
    let (device, plan, preconditions) = established().await;
    let (result, _) = run_changed(
        &device,
        &device,
        &plan,
        &preconditions,
        &[(0x3C01, &[0xA1, 0xA2, 0xA3])],
    )
    .await;
    let evidence = allocation_mismatch(result.unwrap_err());
    assert_eq!(
        evidence.differing,
        [
            range(1, 1, DifferenceClass::Unwritten),
            range(2, 2, DifferenceClass::Written)
        ]
    );
    assert_eq!(&evidence.expected[1..4], [8, 1, 2]);
    assert_eq!(&evidence.actual[1..4], [0xA1, 0xA2, 0xA3]);
}

#[tokio::test]
async fn an_unreadable_prestate_records_every_difference_once_a_written_byte_differs() {
    let (device, plan, preconditions) = established().await;
    let transport = PrestateShortRead {
        device: &device,
        written: AtomicBool::new(false),
    };
    let (result, snapshot) = run_changed(
        &transport,
        &device,
        &plan,
        &preconditions,
        &[(0x3C03, &[0xEE])],
    )
    .await;
    assert_eq!(snapshot.unreadable_allocations, [0]);
    let evidence = allocation_mismatch(result.unwrap_err());
    assert!(evidence.prestate_unreadable);
    assert_eq!(
        evidence.expected,
        [0, 0, 1, 2, 3, 4, 0, 0],
        "placeholders, never a prediction"
    );
    assert_eq!(evidence.actual, [9, 8, 1, 0xEE, 3, 4, 3, 2]);
    assert_eq!(
        evidence.differing,
        [
            range(0, 2, DifferenceClass::UnwrittenUnpredicted),
            range(3, 1, DifferenceClass::Written),
            range(6, 2, DifferenceClass::UnwrittenUnpredicted),
        ]
    );
    assert_eq!(
        snapshot.allocation_mismatch(0, &plan, evidence.actual.clone()),
        Some(evidence)
    );
}

#[tokio::test]
async fn an_unreadable_prestate_with_only_unpredicted_differences_still_verifies() {
    let (device, plan, preconditions) = established().await;
    let transport = PrestateShortRead {
        device: &device,
        written: AtomicBool::new(false),
    };
    let (result, snapshot) = run_changed(&transport, &device, &plan, &preconditions, &[]).await;
    assert_eq!(snapshot.unreadable_allocations, [0]);
    result.expect("unwritten bytes of an unreadable prestate never decide verification");
}

#[tokio::test]
async fn other_final_step_failures_carry_no_evidence() {
    let (device, plan, preconditions) = established().await;
    let mut changed = false;
    let error = run_experimental_property_plan(
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
                device.preset_property(3, PID_PROGRAM_VERSION, &[0, 9, 9, 9, 9]);
                changed = true;
            }
            Ok(())
        },
        &|| false,
    )
    .await
    .unwrap_err();
    assert!(changed);
    assert_eq!(
        error.final_step.map(|f| f.stage),
        Some(FinalStepStage::ProgramVersion)
    );
    assert!(error.final_step_evidence.is_none());
}
