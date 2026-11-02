//! A firmware-changed written octet passes final verification only where declared, and is seen.

use super::property_recovery::{auth, timing};
use super::system2_memory_boundary::{one_allocation_plan, system2_device};
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::property_download::PropertyDownloadPlan;
use knx_net::commissioning::final_step::{FinalStepFailure, FinalStepKind, FinalStepStage};
use knx_net::commissioning::final_step_evidence::{DifferenceClass, DifferingRange};
use knx_net::commissioning::post_load_octets::{
    PostLoadOctet, PostLoadOctetAdmission, PostLoadOctetObservation,
};
use knx_net::commissioning::property_download::{
    run_experimental_property_plan, run_simulated_property_plan, ExperimentalPropertyReport,
    PropertySimulationError, PropertySimulationFailure,
};
use knx_net::commissioning::property_recovery::{
    ExperimentalPropertyPreconditions, PropertyJournalEvent, PropertyRecoverySnapshot,
};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice};

const FINAL_STEP: usize = 7;
/// Allocation `3C00h`, eight octets; the plan writes `01 02 03 04` from `3C02h`.
const START: u16 = 0x3C00;
const PRESTATE: [u8; 8] = [9, 8, 7, 6, 5, 4, 3, 2];
const WRITE: [u8; 4] = [1, 2, 3, 4];
/// The plan writes `02h` here; the simulated firmware clears it after load.
const FLAG: u16 = 0x3C03;

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

fn flag_may_clear(
    preconditions: ExperimentalPropertyPreconditions,
) -> ExperimentalPropertyPreconditions {
    preconditions
        .with_post_load_octet_admission(PostLoadOctetAdmission::Declared(vec![PostLoadOctet {
            object: 3,
            address: FLAG,
            accepted: vec![0],
        }]))
        .unwrap()
}

struct Run {
    result: Result<ExperimentalPropertyReport, PropertySimulationError>,
    snapshot: PropertyRecoverySnapshot,
    /// Post-load observations the journal received, and how many device-control
    /// writes the device had seen by then.
    journaled: Vec<(Vec<PostLoadOctetObservation>, usize)>,
}

fn control_writes(device: &SimulatedDevice) -> usize {
    device
        .seen()
        .iter()
        .filter(|seen| {
            matches!(
                seen,
                Seen::PropertyWrite {
                    object_index: 0,
                    property_id: 14,
                    ..
                }
            )
        })
        .count()
}

/// Runs the plan, letting "firmware" change `change` after load, just before the final step.
async fn run(
    device: &SimulatedDevice,
    plan: &PropertyDownloadPlan,
    preconditions: &ExperimentalPropertyPreconditions,
    change: &[(u16, &[u8])],
    refuse_post_load_journal: bool,
) -> Run {
    let mut snapshot = None;
    let mut journaled = Vec::new();
    let result = run_experimental_property_plan(
        device,
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
                PropertyJournalEvent::PostLoadOctetObservations(observations) => {
                    journaled.push((observations.to_vec(), control_writes(device)));
                    if refuse_post_load_journal {
                        return Err("durable store failed".to_string());
                    }
                }
                _ => {}
            }
            Ok(())
        },
        &|| false,
    )
    .await;
    Run {
        result,
        snapshot: snapshot.expect("recovery acquired before any write"),
        journaled,
    }
}

fn mismatch(
    error: PropertySimulationError,
) -> knx_net::commissioning::final_step_evidence::AllocationMismatchEvidence {
    assert_eq!(error.operation_index, Some(FINAL_STEP));
    assert_eq!(
        error.final_step,
        Some(FinalStepFailure {
            kind: FinalStepKind::VerificationMismatch,
            stage: FinalStepStage::Allocation,
        })
    );
    *error
        .final_step_evidence
        .expect("allocation mismatch evidence")
}

fn written(offset: usize) -> DifferingRange {
    DifferingRange {
        offset,
        length: 1,
        class: DifferenceClass::Written,
    }
}

const CLEARED: PostLoadOctetObservation = PostLoadOctetObservation {
    object: 3,
    address: FLAG,
    written: 2,
    observed: 0,
};

#[tokio::test]
async fn without_a_declaration_a_firmware_cleared_written_octet_fails_verification() {
    let (device, plan, preconditions) = established().await;
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0])], false).await;
    let evidence = mismatch(run.result.unwrap_err());
    assert_eq!(evidence.differing, [written(3)]);
    assert_eq!(evidence.actual[3], 0);
    assert!(run.journaled.is_empty());
}

#[tokio::test]
async fn a_declared_post_load_value_verifies_and_is_journaled_before_the_final_write() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let before = control_writes(&device);
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0])], false).await;
    let report = run.result.expect("the declared value is tolerated");
    assert_eq!(report.post_load_octet_observations, [CLEARED]);
    // Durable before the only remaining write, the VerifyMode restore.
    assert_eq!(run.journaled, [(vec![CLEARED], before + 1)]);
    assert_eq!(control_writes(&device), before + 2);
    assert_eq!(
        CLEARED.to_string(),
        "object 3 octet 3C03h: wrote 02h, device holds declared post-load value 00h"
    );
}

#[tokio::test]
async fn a_refused_post_load_journal_stops_before_the_final_write() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let before = control_writes(&device);
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0])], true).await;
    let error = run.result.unwrap_err();
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Persistence(_)
    ));
    assert_eq!(control_writes(&device), before + 1, "no VerifyMode restore");
}

#[tokio::test]
async fn an_undeclared_value_at_a_declared_octet_still_fails_with_its_real_byte() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0x7F])], false).await;
    let evidence = mismatch(run.result.unwrap_err());
    assert_eq!(evidence.differing, [written(3)]);
    assert_eq!(evidence.actual[3], 0x7F);
    assert!(run.journaled.is_empty());
}

#[tokio::test]
async fn an_unchanged_declared_octet_is_neither_tolerated_nor_observed() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let run = run(&device, &plan, &preconditions, &[], false).await;
    let report = run.result.unwrap();
    assert!(report.post_load_octet_observations.is_empty());
    assert!(run.journaled.is_empty());
}

#[tokio::test]
async fn any_other_difference_still_fails_and_the_replay_agrees() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0, 0xEE])], false).await;
    let evidence = mismatch(run.result.unwrap_err());
    assert_eq!(
        evidence.differing,
        [written(4)],
        "the tolerated octet is no difference"
    );
    assert_eq!(
        evidence.actual,
        [9, 8, 1, 0, 0xEE, 4, 3, 2],
        "real readback kept"
    );
    assert_eq!(
        run.snapshot
            .allocation_mismatch(0, &plan, evidence.actual.clone()),
        Some(evidence)
    );
    // The journal replay tolerates exactly what the live verification tolerated.
    assert_eq!(
        run.snapshot
            .allocation_mismatch(0, &plan, vec![9, 8, 1, 0, 3, 4, 3, 2]),
        None
    );
    assert!(run
        .snapshot
        .allocation_mismatch(0, &plan, vec![9, 8, 1, 0x7F, 3, 4, 3, 2])
        .is_some());
}

#[tokio::test]
async fn a_declaration_must_name_distinct_written_octets_and_new_values() {
    let (_device, _plan, preconditions) = established().await;
    let refused = |octets: Vec<PostLoadOctet>| match preconditions
        .clone()
        .with_post_load_octet_admission(PostLoadOctetAdmission::Declared(octets))
    {
        Err(PropertySimulationFailure::Recovery(reason)) => reason,
        other => panic!("accepted or wrong failure: {other:?}"),
    };
    let octet = |object, address, accepted: &[u8]| PostLoadOctet {
        object,
        address,
        accepted: accepted.to_vec(),
    };
    assert_eq!(
        refused(vec![]),
        "post-load octet declaration names no octet"
    );
    assert_eq!(
        refused(vec![octet(3, START, &[0])]),
        "post-load octet is not written by this plan",
        "a preserved octet"
    );
    assert_eq!(
        refused(vec![octet(1, FLAG, &[0])]),
        "post-load octet is not written by this plan",
        "another object"
    );
    assert_eq!(
        refused(vec![octet(3, FLAG, &[])]),
        "post-load octet accepts no other value"
    );
    for accepted in [&[2u8][..], &[0, 0]] {
        assert_eq!(
            refused(vec![octet(3, FLAG, accepted)]),
            "post-load values must be distinct and differ from the written value"
        );
    }
    assert_eq!(
        refused(vec![octet(3, FLAG, &[0]), octet(3, FLAG, &[1])]),
        "post-load octet declared twice"
    );
    assert_eq!(
        preconditions.post_load_octets,
        PostLoadOctetAdmission::Legacy,
        "the default stays strict"
    );
}
