//! A firmware-changed written octet passes final verification only where declared, and is seen.

use super::final_step_evidence::{
    established, PrestateShortRead, FINAL_STEP, PRESTATE, START, WRITE,
};
use super::property_recovery::{auth, timing};
use knx_core::commissioning::authorisation::AuthorisationPlan;
use knx_core::commissioning::property_download::PropertyDownloadPlan;
use knx_net::commissioning::final_step::{FinalStepFailure, FinalStepKind, FinalStepStage};
use knx_net::commissioning::final_step_evidence::{
    AllocationMismatchEvidence, DifferenceClass, DifferingRange,
};
use knx_net::commissioning::post_load_octets::{
    PostLoadOctet, PostLoadOctetAdmission, PostLoadOctetObservation,
};
use knx_net::commissioning::property_download::{
    restore_experimental_property_plan, run_experimental_property_plan, ExperimentalPropertyReport,
    PropertySimulationError, PropertySimulationFailure,
};
use knx_net::commissioning::property_recovery::{
    ExperimentalPropertyPreconditions, PropertyJournalEvent, PropertyRecoverySnapshot,
};
use knx_net::commissioning::simulator::{Seen, SimulatedDevice};
use knx_net::management::ManagementTransport;
use std::sync::atomic::AtomicBool;

/// The plan writes `WRITE[1]` = `02h` here (offset 3); the simulated firmware clears it.
const OFFSET: usize = 3;
const FLAG: u16 = START + OFFSET as u16;

type Journaled = Vec<(Vec<PostLoadOctetObservation>, usize)>;

fn declared(octets: Vec<PostLoadOctet>) -> PostLoadOctetAdmission {
    PostLoadOctetAdmission::Declared(octets)
}

fn flag(accepted: &[u8]) -> PostLoadOctet {
    PostLoadOctet {
        object: 3,
        address: FLAG,
        accepted: accepted.to_vec(),
    }
}

fn flag_may_clear(
    preconditions: ExperimentalPropertyPreconditions,
) -> ExperimentalPropertyPreconditions {
    preconditions
        .with_post_load_octet_admission(declared(vec![flag(&[0])]))
        .unwrap()
}

fn cleared(prestate: Option<u8>) -> PostLoadOctetObservation {
    PostLoadOctetObservation {
        object: 3,
        address: FLAG,
        written: WRITE[1],
        observed: 0,
        prestate,
    }
}

struct Run {
    result: Result<ExperimentalPropertyReport, PropertySimulationError>,
    snapshot: Option<PropertyRecoverySnapshot>,
    /// Post-load observations the journal received, and how many device-control
    /// writes the device had seen by then.
    journaled: Journaled,
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

/// Journals like Net's caller; lets "firmware" change `change` just before the final step.
fn journal<'a>(
    device: &'a SimulatedDevice,
    change: &'a [(u16, &'a [u8])],
    refuse_post_load: bool,
    snapshot: &'a mut Option<PropertyRecoverySnapshot>,
    journaled: &'a mut Journaled,
) -> impl FnMut(PropertyJournalEvent<'_>) -> Result<(), String> + 'a {
    move |event| {
        match event {
            PropertyJournalEvent::RecoveryAcquired(captured) => *snapshot = Some(captured.clone()),
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
                if refuse_post_load {
                    return Err("durable store failed".to_string());
                }
            }
            _ => {}
        }
        Ok(())
    }
}

async fn run_on(
    transport: &impl ManagementTransport,
    device: &SimulatedDevice,
    plan: &PropertyDownloadPlan,
    preconditions: &ExperimentalPropertyPreconditions,
    change: &[(u16, &[u8])],
    refuse_post_load: bool,
) -> Run {
    let mut snapshot = None;
    let mut journaled = Vec::new();
    let result = run_experimental_property_plan(
        transport,
        plan,
        auth(device),
        AuthorisationPlan::Skip,
        timing(),
        preconditions,
        &mut journal(
            device,
            change,
            refuse_post_load,
            &mut snapshot,
            &mut journaled,
        ),
        &|| false,
    )
    .await;
    Run {
        result,
        snapshot,
        journaled,
    }
}

async fn run(
    device: &SimulatedDevice,
    plan: &PropertyDownloadPlan,
    preconditions: &ExperimentalPropertyPreconditions,
    change: &[(u16, &[u8])],
) -> Run {
    run_on(device, device, plan, preconditions, change, false).await
}

fn mismatch(error: PropertySimulationError) -> AllocationMismatchEvidence {
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

#[tokio::test]
async fn without_a_declaration_a_firmware_cleared_written_octet_fails_verification() {
    let (device, plan, preconditions) = established().await;
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0])]).await;
    let evidence = mismatch(run.result.unwrap_err());
    assert_eq!(evidence.differing, [written(OFFSET)]);
    assert_eq!(evidence.actual[OFFSET], 0);
    assert!(run.journaled.is_empty());
}

#[tokio::test]
async fn a_declared_post_load_value_verifies_and_is_journaled_before_the_final_write() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let before = control_writes(&device);
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0])]).await;
    let report = run.result.expect("the declared value is tolerated");
    let observed = cleared(Some(PRESTATE[OFFSET]));
    assert_eq!(report.post_load_octet_observations, [observed]);
    // Durable before the only remaining write, the VerifyMode restore.
    assert_eq!(run.journaled, [(vec![observed], before + 1)]);
    assert_eq!(control_writes(&device), before + 2);
    assert_eq!(
        observed.to_string(),
        "object 3 octet 3C03h: wrote 02h, device holds declared post-load value 00h \
         (held 06h before the download)"
    );
}

#[tokio::test]
async fn a_prestate_equal_to_the_observed_value_is_reported_not_hidden() {
    // A write that never took effect looks like a firmware change; the prestate shows it.
    let (device, plan, preconditions) = established().await;
    device.preset_memory(u32::from(FLAG), &[0]);
    let preconditions = flag_may_clear(preconditions);
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0])]).await;
    assert_eq!(
        run.result.unwrap().post_load_octet_observations,
        [cleared(Some(0))]
    );
}

#[tokio::test]
async fn an_unreadable_prestate_still_tolerates_only_the_declared_value() {
    for (value, verified) in [(0u8, true), (0x7F, false)] {
        let (device, plan, preconditions) = established().await;
        let preconditions = flag_may_clear(preconditions);
        let transport = PrestateShortRead {
            device: &device,
            written: AtomicBool::new(false),
        };
        let change: [(u16, &[u8]); 1] = [(FLAG, std::slice::from_ref(&value))];
        let run = run_on(&transport, &device, &plan, &preconditions, &change, false).await;
        assert_eq!(run.snapshot.unwrap().unreadable_allocations, [0]);
        if verified {
            assert_eq!(
                run.result.unwrap().post_load_octet_observations,
                [cleared(None)]
            );
            assert_eq!(
                cleared(None).to_string(),
                "object 3 octet 3C03h: wrote 02h, device holds declared post-load value 00h \
                 (prestate unreadable)"
            );
        } else {
            assert!(mismatch(run.result.unwrap_err())
                .differing
                .contains(&written(OFFSET)));
            assert!(run.journaled.is_empty());
        }
    }
}

#[tokio::test]
async fn a_refused_post_load_journal_stops_before_the_final_write() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let before = control_writes(&device);
    let run = run_on(
        &device,
        &device,
        &plan,
        &preconditions,
        &[(FLAG, &[0])],
        true,
    )
    .await;
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
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0x7F])]).await;
    let evidence = mismatch(run.result.unwrap_err());
    assert_eq!(evidence.differing, [written(OFFSET)]);
    assert_eq!(evidence.actual[OFFSET], 0x7F);
    assert!(run.journaled.is_empty());
}

#[tokio::test]
async fn an_unchanged_declared_octet_is_neither_tolerated_nor_observed() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let run = run(&device, &plan, &preconditions, &[]).await;
    assert!(run.result.unwrap().post_load_octet_observations.is_empty());
    assert!(run.journaled.is_empty());
}

#[tokio::test]
async fn any_other_difference_still_fails_and_the_replay_agrees() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let run = run(&device, &plan, &preconditions, &[(FLAG, &[0, 0xEE])]).await;
    let snapshot = run.snapshot.unwrap();
    let evidence = mismatch(run.result.unwrap_err());
    assert_eq!(
        evidence.differing,
        [written(OFFSET + 1)],
        "the tolerated octet is no difference"
    );
    assert_eq!(
        evidence.actual,
        [9, 8, 1, 0, 0xEE, 4, 3, 2],
        "real readback kept"
    );
    assert_eq!(
        snapshot.allocation_mismatch(0, &plan, evidence.actual.clone()),
        Some(evidence)
    );
    // The journal replay tolerates exactly what the live verification tolerated.
    assert_eq!(
        snapshot.allocation_mismatch(0, &plan, vec![9, 8, 1, 0, 3, 4, 3, 2]),
        None
    );
    assert!(snapshot
        .allocation_mismatch(0, &plan, vec![9, 8, 1, 0x7F, 3, 4, 3, 2])
        .is_some());
}

#[tokio::test]
async fn a_recovery_replay_stays_strict_at_a_declared_octet() {
    let (device, plan, preconditions) = established().await;
    let preconditions = flag_may_clear(preconditions);
    let download = run(&device, &plan, &preconditions, &[(FLAG, &[0])]).await;
    let snapshot = download.snapshot.unwrap();
    download.result.unwrap();
    let recovery = snapshot.recovery_plan(&plan).unwrap();
    for (label, at_flag) in [
        ("firmware clears the restored octet", Some(0u8)),
        ("the download value is left behind", Some(WRITE[1])),
        ("the prestate is restored", None),
    ] {
        let change: Vec<(u16, &[u8])> = at_flag
            .as_ref()
            .map(|value| (FLAG, std::slice::from_ref(value)))
            .into_iter()
            .collect();
        let mut unused = None;
        let mut journaled = Vec::new();
        let result = restore_experimental_property_plan(
            &device,
            &plan,
            &snapshot,
            auth(&device),
            AuthorisationPlan::Skip,
            timing(),
            &mut journal(&device, &change, false, &mut unused, &mut journaled),
            &|| false,
        )
        .await;
        assert!(journaled.is_empty(), "{label}");
        let Some(value) = at_flag else {
            let report = result.expect(label);
            assert!(report.post_load_octet_observations.is_empty(), "{label}");
            continue;
        };
        let evidence = mismatch(result.unwrap_err());
        assert_eq!(evidence.differing, [written(OFFSET)], "{label}");
        let mut actual = PRESTATE.to_vec();
        actual[OFFSET] = value;
        assert_eq!(evidence.actual, actual, "{label}");
        assert_eq!(
            snapshot.allocation_mismatch(0, &recovery, actual),
            Some(evidence),
            "{label}: the replay of the recovery plan is strict too"
        );
    }
}

#[tokio::test]
async fn a_restore_identical_to_the_download_tolerates_and_reports_like_it() {
    // The prestate already held every written value, so the recovery plan is the download.
    let (device, plan, preconditions) = established().await;
    device.preset_memory(u32::from(START) + 2, &WRITE);
    let preconditions = flag_may_clear(preconditions);
    let download = run(&device, &plan, &preconditions, &[]).await;
    let snapshot = download.snapshot.unwrap();
    assert!(download
        .result
        .unwrap()
        .post_load_octet_observations
        .is_empty());
    let recovery = snapshot.recovery_plan(&plan).unwrap();
    assert_eq!(recovery, plan);
    let change: [(u16, &[u8]); 1] = [(FLAG, &[0])];
    let mut unused = None;
    let mut journaled = Vec::new();
    let report = restore_experimental_property_plan(
        &device,
        &plan,
        &snapshot,
        auth(&device),
        AuthorisationPlan::Skip,
        timing(),
        &mut journal(&device, &change, false, &mut unused, &mut journaled),
        &|| false,
    )
    .await
    .unwrap();
    let observed = cleared(Some(WRITE[1]));
    assert_eq!(report.post_load_octet_observations, [observed]);
    assert_eq!(journaled.len(), 1, "journaled, never silent");
    let mut actual = PRESTATE.to_vec();
    actual[2..6].copy_from_slice(&WRITE);
    actual[OFFSET] = 0;
    assert_eq!(snapshot.allocation_mismatch(0, &recovery, actual), None);
}

#[tokio::test]
async fn an_undeclared_octet_set_on_the_struct_is_refused_before_any_frame() {
    let (device, plan, mut preconditions) = established().await;
    preconditions.post_load_octets = declared(vec![PostLoadOctet {
        object: 3,
        address: START,
        accepted: vec![0],
    }]);
    let before = device.seen().len();
    let error = run(&device, &plan, &preconditions, &[])
        .await
        .result
        .unwrap_err();
    assert_eq!(error.operation_index, None);
    assert!(matches!(
        error.failure,
        PropertySimulationFailure::Recovery(ref reason)
            if reason == "post-load octet is not written by this plan"
    ));
    assert_eq!(device.seen().len(), before, "no frame at all");
}

#[tokio::test]
async fn a_declaration_must_name_distinct_written_octets_and_new_values() {
    let (_device, _plan, preconditions) = established().await;
    let refused = |octets: Vec<PostLoadOctet>| match preconditions
        .clone()
        .with_post_load_octet_admission(declared(octets))
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
        refused(vec![flag(&[])]),
        "post-load octet accepts no other value"
    );
    for accepted in [&[WRITE[1]][..], &[0, 0]] {
        assert_eq!(
            refused(vec![flag(accepted)]),
            "post-load values must be distinct and differ from the written value"
        );
    }
    assert_eq!(
        refused(vec![flag(&[0]), flag(&[1])]),
        "post-load octet declared twice"
    );
    assert_eq!(
        preconditions.post_load_octets,
        PostLoadOctetAdmission::Legacy,
        "the default stays strict"
    );
}
