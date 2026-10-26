//! Revision-2 PID 7 width policy matrix on hardware-kind simulated devices with disk reopen.
use super::pid7_width::revision1;
use super::*;
use knx_app::property_validation::{PointerObservationPhase, PointerObservationRun};
use knx_app::property_validation_bundle::{
    PointerObservationRecord, PointerRecord, RecoveryRecord,
};
use knx_core::commissioning::load_state::LoadState;
use knx_net::commissioning::pointer_width::PointerWidthAdmission;
use knx_net::commissioning::simulator::Seen;
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

/// (object, read index of that object, total reads of that object, elements, data) -> reply.
type Rewrite = Arc<dyn Fn(u8, usize, usize, u8, &[u8]) -> (u8, Vec<u8>) + Send + Sync>;
type Counts = Arc<Mutex<BTreeMap<u8, usize>>>;

/// Hardware-kind wrapper over the simulated device. Every PID 7 answer is counted per
/// object, and optionally rewritten; the default simulator answer stays four octets.
struct Rewriting<'a> {
    device: &'a SimulatedDevice,
    rewrite: Option<Rewrite>,
    totals: BTreeMap<u8, usize>,
    counts: Counts,
    fault: Option<Fault>,
    objects: usize,
    done: Arc<AtomicUsize>,
    final_phase: Arc<AtomicBool>,
}
/// One lie told only while the final verification reads, never earlier: the final phase
/// starts at the first memory read after the last LoadCompleted write (no clocks involved).
#[derive(Clone, Copy, PartialEq)]
enum Fault {
    Memory,
    LoadState,
    ProgramVersion,
}
impl ManagementTransport for Rewriting<'_> {
    fn assigned_address(&self) -> IndividualAddress {
        ManagementTransport::assigned_address(self.device)
    }
    fn subscribe(&self) -> broadcast::Receiver<TunnelEvent> {
        let mut upstream = ManagementTransport::subscribe(self.device);
        let (sender, receiver) = broadcast::channel(256);
        let (rewrite, totals, counts) = (
            self.rewrite.clone(),
            self.totals.clone(),
            Arc::clone(&self.counts),
        );
        let (fault, final_phase) = (self.fault, Arc::clone(&self.final_phase));
        tokio::spawn(async move {
            loop {
                match upstream.recv().await {
                    Ok(TunnelEvent::Telegram(mut frame)) => {
                        if let ApplicationService::PropertyValueResponse {
                            object_index,
                            property_id: 7,
                            nr_of_elem,
                            data,
                            ..
                        } = &mut frame.service
                        {
                            let mut counts = counts.lock().unwrap();
                            let index = counts.entry(*object_index).or_default();
                            if let Some(rewrite) = &rewrite {
                                let total = totals.get(object_index).copied().unwrap_or(usize::MAX);
                                (*nr_of_elem, *data) =
                                    rewrite(*object_index, *index, total, *nr_of_elem, data);
                            }
                            *index += 1;
                        }
                        if fault.is_some() && final_phase.load(Ordering::SeqCst) {
                            match (&mut frame.service, fault) {
                                (
                                    ApplicationService::PropertyValueResponse {
                                        property_id: 5,
                                        data,
                                        ..
                                    },
                                    Some(Fault::LoadState),
                                ) => *data = vec![LoadState::Loading.octet()],
                                (
                                    ApplicationService::PropertyValueResponse {
                                        object_index: 3,
                                        property_id: 13,
                                        data,
                                        ..
                                    },
                                    Some(Fault::ProgramVersion),
                                ) => *data = vec![0; 5],
                                (
                                    ApplicationService::MemoryResponse { data, .. }
                                    | ApplicationService::UserMemoryResponse { data, .. },
                                    Some(Fault::Memory),
                                ) if !data.is_empty() => data[0] ^= 1,
                                _ => {}
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
        match &service {
            ApplicationService::PropertyValueWrite {
                property_id: 5,
                data,
                ..
            } if data.first() == Some(&2) => {
                self.done.fetch_add(1, Ordering::SeqCst);
            }
            ApplicationService::MemoryRead { .. } | ApplicationService::UserMemoryRead { .. }
                if self.done.load(Ordering::SeqCst) >= self.objects =>
            {
                self.final_phase.store(true, Ordering::SeqCst);
            }
            _ => {}
        }
        ManagementTransport::send_frame(self.device, destination, transport, service).await
    }
}

fn rewriting<'a>(
    device: &'a SimulatedDevice,
    totals: &BTreeMap<u8, usize>,
    rewrite: Option<Rewrite>,
) -> Rewriting<'a> {
    Rewriting {
        device,
        rewrite,
        totals: totals.clone(),
        counts: Counts::default(),
        fault: None,
        objects: usize::MAX,
        done: Arc::default(),
        final_phase: Arc::default(),
    }
}
fn rw(
    f: impl Fn(u8, usize, usize, u8, &[u8]) -> (u8, Vec<u8>) + Send + Sync + 'static,
) -> Option<Rewrite> {
    Some(Arc::new(f))
}
/// Last two octets of a four-octet answer, count kept.
fn low2(elements: u8, data: &[u8]) -> (u8, Vec<u8>) {
    (elements, data[data.len().saturating_sub(2)..].to_vec())
}
const SOURCE_1: [u8; 4] = [0, 0, 1, 0x16];

#[derive(Clone, Copy, PartialEq)]
enum Policy {
    Legacy,
    Width,
}

struct Run {
    _dir: tempfile::TempDir,
    device: SimulatedDevice,
    old: knx_net::commissioning::simulator::SimulatedSystem2Snapshot,
    bundle: PreparedBundle,
    operation: std::path::PathBuf,
    result: std::io::Result<()>,
    seen_before: usize,
    totals: BTreeMap<u8, usize>,
}
impl Run {
    fn store(&self) -> OperationStore {
        OperationStore::open(&self.operation, &self.bundle.binding).unwrap()
    }
    fn journal_bytes(&self) -> Vec<(String, Vec<u8>)> {
        let mut files: Vec<_> = std::fs::read_dir(&self.operation)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_file())
            .map(|p| {
                (
                    p.file_name().unwrap().to_string_lossy().into_owned(),
                    std::fs::read(&p).unwrap(),
                )
            })
            .collect();
        files.sort();
        files
    }
    fn writes_after_start(&self) -> bool {
        self.device.seen()[self.seen_before..]
            .iter()
            .any(|s| matches!(s, Seen::MemoryWrite { .. } | Seen::PropertyWrite { .. }))
    }
    fn private_code(&self) -> String {
        let head: serde_json::Value =
            serde_json::from_slice(&std::fs::read(self.operation.join("HEAD")).unwrap()).unwrap();
        let last = head["sequence"].as_u64().unwrap();
        let checkpoint: serde_json::Value = serde_json::from_slice(
            &std::fs::read(self.operation.join(format!("checkpoint-{last:08}.json"))).unwrap(),
        )
        .unwrap();
        checkpoint["checkpoint"]["code"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }
    fn private_diagnostic(&self) -> String {
        let head: serde_json::Value =
            serde_json::from_slice(&std::fs::read(self.operation.join("HEAD")).unwrap()).unwrap();
        let last = head["sequence"].as_u64().unwrap();
        let checkpoint: serde_json::Value = serde_json::from_slice(
            &std::fs::read(self.operation.join(format!("checkpoint-{last:08}.json"))).unwrap(),
        )
        .unwrap();
        checkpoint["checkpoint"]["local_diagnostic"]
            .as_str()
            .unwrap_or_default()
            .to_string()
    }
    async fn restore(&self, rewrite: Option<Rewrite>) -> std::io::Result<()> {
        let mut store = self.store();
        let phrase = self.bundle.restore_phrase();
        let download = required_confirmation_phrase(self.device.address(), WriteScope::Download);
        let release = knx_net::commissioning::hardware_release::required_release_phrase(
            self.device.address(),
        );
        let confirmations = HardwareRecoveryConfirmations {
            recovery: &phrase,
            download: &download,
            risk: &release,
        };
        let transport = rewriting(&self.device, &BTreeMap::new(), rewrite);
        restore_project_hardware(
            &transport,
            &self.bundle,
            &mut store,
            integrated_timing(),
            &confirmations,
            &AtomicBool::new(false),
        )
        .await
    }
}

/// Prepare an exact UP258 bundle (`Width` is the freshly observed revision 2) and execute it.
async fn execute(policy: Policy, rewrite: Option<Rewrite>) -> Run {
    execute_with(policy, Setup::default(), rewrite).await
}
/// Optional faults around an execution: a device configuration, a final-phase lie, and a
/// journal sequence made unwritable (a directory squats on its file name).
#[derive(Default)]
struct Setup {
    config: Option<SimulatorConfig>,
    fault: Option<Fault>,
    block_checkpoint: Option<usize>,
}
async fn execute_with(policy: Policy, setup: Setup, rewrite: Option<Rewrite>) -> Run {
    let (dir, conn, project, profile) = inputs();
    let device = old_device_with_config(
        &conn,
        &project,
        &profile,
        setup.config.clone().unwrap_or_default(),
    );
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "pid7-policy-test",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let cancel = AtomicBool::new(false);
    let mut bundle =
        observe_project_hardware(&Hardware(&device), source, integrated_timing(), &cancel)
            .await
            .unwrap();
    if policy == Policy::Legacy {
        bundle = revision1(bundle);
    }
    let admission =
        admit_current_project_hardware(&conn, &project, knx_core::DeviceId(1), &bundle).unwrap();
    // A clean pass over an identical fresh device counts the PID 7 reads per object, so a
    // rewrite can tell the prestate (first) from the final (last) read.
    let totals = {
        let probe = old_device(&conn, &project, &profile);
        let counts = Counts::default();
        let mut transport = rewriting(&probe, &BTreeMap::new(), None);
        transport.counts = Arc::clone(&counts);
        let mut store =
            OperationStore::create(&dir.path().join("probe"), bundle.binding.clone()).unwrap();
        run_execute(&transport, &admission, &mut store, &bundle, &probe)
            .await
            .unwrap();
        let totals = counts.lock().unwrap().clone();
        totals
    };
    let operation = dir.path().join("operation");
    let mut store = OperationStore::create(&operation, bundle.binding.clone()).unwrap();
    if let Some(sequence) = setup.block_checkpoint {
        std::fs::create_dir(operation.join(format!("checkpoint-{sequence:08}.json"))).unwrap();
    }
    let old = device.system2_snapshot();
    let seen_before = device.seen().len();
    let mut transport = rewriting(&device, &totals, rewrite);
    transport.fault = setup.fault;
    transport.objects = totals.len();
    let result = run_execute(&transport, &admission, &mut store, &bundle, &device).await;
    drop(store);
    Run {
        _dir: dir,
        device,
        old,
        bundle,
        operation,
        result,
        seen_before,
        totals,
    }
}
async fn run_execute<T: ManagementTransport>(
    transport: &T,
    admission: &knx_app::system2_validation_preparation::CurrentProjectHardware,
    store: &mut OperationStore,
    bundle: &PreparedBundle,
    device: &SimulatedDevice,
) -> std::io::Result<()> {
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
    execute_project_hardware(
        transport,
        admission,
        store,
        integrated_timing(),
        &confirmations,
        &AtomicBool::new(false),
    )
    .await
}
fn observations(
    store: &OperationStore,
    object: u8,
) -> Vec<(
    PointerObservationRun,
    PointerObservationPhase,
    Option<PointerRecord>,
)> {
    store
        .progress()
        .pointer_observations
        .iter()
        .filter(|o| o.record.object == object)
        .map(|o| (o.run, o.phase, o.record.observed))
        .collect()
}
fn resource_pointers(store: &OperationStore) -> Vec<PointerRecord> {
    store
        .recovery()
        .unwrap()
        .resources
        .iter()
        .map(|r| r.pointer)
        .collect()
}
fn finished(run: &Run) {
    assert!(run.result.is_ok(), "must execute: {:?}", run.result);
    assert_eq!(run.store().outcome(), LocalOutcome::Finished);
}

#[tokio::test]
async fn t3_all_two_finishes_reopens_and_restores_the_original_state() {
    let all_two = || rw(|_, _, _, n, d| low2(n, d));
    let run = execute(Policy::Width, all_two()).await;
    finished(&run);
    assert!(run.totals.keys().copied().eq([1, 2, 3]));
    let store = run.store();
    assert!(store.progress().restore_available);
    assert!(resource_pointers(&store)
        .iter()
        .all(|p| matches!(p, PointerRecord::Two(_))));
    assert!(store.progress().pointer_observations.is_empty());
    drop(store);
    run.restore(all_two()).await.unwrap();
    assert_eq!(run.store().outcome(), LocalOutcome::Restored);
    assert_eq!(run.device.system2_snapshot(), run.old);
}

#[tokio::test]
async fn the_policy_objects_are_exactly_the_three_declared_pointer_objects() {
    let run = execute(Policy::Width, None).await;
    assert_eq!(
        run.bundle.preconditions().unwrap().pointer_widths,
        PointerWidthAdmission::one_element_two_or_four(vec![1, 2, 3]).unwrap()
    );
    let legacy = revision1(run.bundle.clone());
    assert_eq!(
        legacy.preconditions().unwrap().pointer_widths,
        PointerWidthAdmission::Legacy
    );
}

#[tokio::test]
async fn t4_all_four_on_revision_two_behaves_like_legacy() {
    let run = execute(Policy::Width, None).await;
    finished(&run);
    let store = run.store();
    assert!(resource_pointers(&store)
        .iter()
        .all(|p| matches!(p, PointerRecord::Four(_))));
    assert!(store.progress().pointer_observations.is_empty());
    drop(store);
    run.restore(None).await.unwrap();
    assert_eq!(run.store().outcome(), LocalOutcome::Restored);
}

fn refusal_case(object: u8, elements: u8, data: &[u8]) -> Option<Rewrite> {
    let data = data.to_vec();
    rw(move |o, index, _, n, d| {
        if o == object && index == 0 {
            (elements, data.clone())
        } else {
            (n, d.to_vec())
        }
    })
}

#[tokio::test]
async fn t5_revision_two_refuses_every_other_shape_before_any_write() {
    let cases: [(u8, &[u8], &str); 6] = [
        (2, &[0, 0, 2, 0x61], "2 elements, 4 octets, 00000261"),
        (2, &[2, 0x61], "2 elements, 2 octets, 0261"),
        (0, &[0xAB], "0 elements, 1 octet, ab"),
        (1, &[1, 2, 3], "1 element, 3 octets, 010203"),
        (1, &[0xAB], "1 element, 1 octet, ab"),
        (1, &[1, 2, 3, 4, 5], "1 element, 5 octets, 0102030405"),
    ];
    for (elements, data, shown) in cases {
        let run = execute(Policy::Width, refusal_case(2, elements, data)).await;
        assert!(run.result.is_err(), "{shown} must refuse");
        let store = run.store();
        assert_eq!(store.outcome(), LocalOutcome::Failed, "{shown}");
        assert!(!store.progress().mutation_possible, "{shown}");
        assert!(!run.writes_after_start(), "{shown}");
        assert_eq!(run.device.system2_snapshot(), run.old, "{shown}");
        assert!(
            run.private_diagnostic().contains(&format!(
                "resource pointer response is not one element of two or four octets: object 2, {shown}"
            )),
            "{shown}: {}",
            run.private_diagnostic()
        );
        let report = serde_json::to_string(&store.shareable_report()).unwrap();
        assert!(!report.contains(&data.iter().map(|b| format!("{b:02x}")).collect::<String>()));
    }
    // Zero elements and no data is a refused observation, not a malformed reply.
    let run = execute(Policy::Width, refusal_case(2, 0, &[])).await;
    finished(&run);
    let store = run.store();
    assert!(store.progress().prestate_differs);
    assert_eq!(observations(&store, 2)[0].2, None);
}

#[tokio::test]
async fn t6_legacy_keeps_its_leniency_text_and_zero_element_discard() {
    let run = execute(Policy::Legacy, refusal_case(2, 2, &[0, 0, 2, 0x61])).await;
    finished(&run);
    assert_eq!(
        observations(&run.store(), 2)[0].2,
        Some(PointerRecord::Four([0, 0, 2, 0x61]))
    );
    let run = execute(Policy::Legacy, refusal_case(2, 0, &[0xAB])).await;
    finished(&run);
    assert_eq!(observations(&run.store(), 2)[0].2, None);
    let run = execute(Policy::Legacy, refusal_case(1, 1, &[1, 0x16])).await;
    assert!(run.result.is_err());
    assert!(!run.writes_after_start());
    assert!(run.private_diagnostic().contains(
        "resource pointer response is not four octets: object 1, 1 element, 2 octets, 0116"
    ));
}

#[tokio::test]
async fn t7_t8_a_width_change_with_an_equal_value_is_a_durable_observation() {
    // Prestate Two, later reads Four with the same value (T7).
    let run = execute(
        Policy::Width,
        rw(|o, index, _, n, d| {
            if o == 1 && index == 0 {
                low2(n, d)
            } else {
                (n, d.to_vec())
            }
        }),
    )
    .await;
    finished(&run);
    let store = run.store();
    let seen = observations(&store, 1);
    assert!(!seen.is_empty());
    assert!(seen.iter().all(
        |(run, phase, observed)| *run == PointerObservationRun::Download
            && *phase != PointerObservationPhase::Prestate
            && *observed == Some(PointerRecord::Four(SOURCE_1))
    ));
    assert!(seen
        .iter()
        .any(|(_, phase, _)| *phase == PointerObservationPhase::Final));
    assert_eq!(resource_pointers(&store)[0], PointerRecord::Two([1, 0x16]));
    assert!(
        store.progress().restore_available,
        "equal values keep recovery available"
    );
    // Prestate Four, later reads Two with the same value (T8).
    let run = execute(
        Policy::Width,
        rw(|o, index, _, n, d| {
            if o == 1 && index > 0 {
                low2(n, d)
            } else {
                (n, d.to_vec())
            }
        }),
    )
    .await;
    finished(&run);
    let seen = observations(&run.store(), 1);
    assert!(seen
        .iter()
        .any(|(_, phase, _)| *phase == PointerObservationPhase::Final));
    assert!(seen
        .iter()
        .all(|(_, _, observed)| *observed == Some(PointerRecord::Two([1, 0x16]))));
}

#[tokio::test]
async fn t9_a_refused_prestate_has_no_width_reference() {
    let run = execute(
        Policy::Width,
        rw(|o, index, _, n, d| match (o, index) {
            (1, 0) => (0, Vec::new()),
            (1, _) => low2(n, d),
            _ => (n, d.to_vec()),
        }),
    )
    .await;
    finished(&run);
    let seen = observations(&run.store(), 1);
    assert_eq!(seen.len(), 1, "only the refused prestate: {seen:?}");
    assert_eq!(seen[0].1, PointerObservationPhase::Prestate);
    assert_eq!(seen[0].2, None);
}

#[tokio::test]
async fn t10_t11_a_differing_two_prestate_is_observed_and_blocks_recovery_without_a_trace() {
    for (object, prestate, shown) in [(2u8, [2u8, 0x61], [2, 0x61]), (1, [0x16, 1], [0x16, 1])] {
        let run = execute(
            Policy::Width,
            rw(move |o, index, _, n, d| {
                if o == object && index == 0 {
                    (1, prestate.to_vec())
                } else {
                    (n, d.to_vec())
                }
            }),
        )
        .await;
        finished(&run);
        let store = run.store();
        let seen = observations(&store, object);
        assert_eq!(
            seen[0],
            (
                PointerObservationRun::Download,
                PointerObservationPhase::Prestate,
                Some(PointerRecord::Two(shown))
            )
        );
        assert!(store.progress().prestate_differs && !store.progress().restore_available);
        drop(store);
        let before = run.journal_bytes();
        assert!(run.restore(None).await.is_err());
        assert_eq!(run.journal_bytes(), before, "refusal appends nothing");
        assert_eq!(run.store().outcome(), LocalOutcome::Finished);
    }
}

#[tokio::test]
async fn t12_restore_compares_against_the_original_prestate_width() {
    let two = || rw(|o, _, _, n, d| if o == 1 { low2(n, d) } else { (n, d.to_vec()) });
    let run = execute(Policy::Width, two()).await;
    finished(&run);
    assert!(observations(&run.store(), 1).is_empty());
    let before: Vec<_> = run.store().progress().pointer_observations.clone();
    // The restore run answers Four with the same value: the width differs from the original.
    run.restore(None).await.unwrap();
    let store = run.store();
    assert_eq!(store.outcome(), LocalOutcome::Restored);
    assert_eq!(
        &store.progress().pointer_observations[..before.len()],
        &before[..]
    );
    let restore = observations(&store, 1);
    assert!(!restore.is_empty());
    assert!(restore
        .iter()
        .all(|(run, _, observed)| *run == PointerObservationRun::Restore
            && *observed == Some(PointerRecord::Four(SOURCE_1))));
    assert_eq!(run.device.system2_snapshot(), run.old);
}

#[tokio::test]
async fn t13_reopened_mixed_recovery_equals_the_durable_original() {
    let run = execute(
        Policy::Width,
        rw(|o, _, _, n, d| if o == 1 { low2(n, d) } else { (n, d.to_vec()) }),
    )
    .await;
    finished(&run);
    let store = run.store();
    let durable = store.recovery().unwrap().clone();
    assert_eq!(
        resource_pointers(&store),
        [
            PointerRecord::Two([1, 0x16]),
            durable.resources[1].pointer,
            durable.resources[2].pointer
        ]
    );
    let snapshot = durable.to_snapshot(&run.bundle).unwrap();
    assert_eq!(
        RecoveryRecord::from_snapshot(&snapshot, &run.bundle).unwrap(),
        durable
    );
}

fn json(bundle: &PreparedBundle) -> serde_json::Value {
    serde_json::to_value(bundle).unwrap()
}

#[tokio::test]
async fn t16_tampered_policy_and_pointer_records_refuse_before_traffic() {
    let run = execute(Policy::Width, rw(|_, _, _, n, d| low2(n, d))).await;
    let bundle = json(&run.bundle);
    let policy = "/hardware_preparation/pointer_width_policy";
    let revision = "/hardware_preparation/revision";
    let load = |v: serde_json::Value| serde_json::from_value::<PreparedBundle>(v);
    let refuses = |v: serde_json::Value| load(v).map_or(true, |b| b.plan().is_err());
    let mut v = bundle.clone();
    v.pointer_mut("/hardware_preparation")
        .unwrap()
        .as_object_mut()
        .unwrap()
        .remove("pointer_width_policy");
    assert!(refuses(v), "revision 2 without its policy");
    let mut v = bundle.clone();
    *v.pointer_mut(policy).unwrap() =
        "up258-object1-pid7-one-two-octet-element-field-observed-v1".into();
    assert!(refuses(v), "wrong policy id");
    let mut v = bundle.clone();
    *v.pointer_mut(revision).unwrap() = 1.into();
    assert!(refuses(v), "revision 1 carrying a policy id");
    let mut v = bundle.clone();
    *v.pointer_mut(revision).unwrap() = 3.into();
    assert!(refuses(v), "unknown revision");
    let mut v = bundle.clone();
    *v.pointer_mut(policy).unwrap() = serde_json::Value::Null;
    assert!(load(v).is_err(), "null policy is not absence");
    let text = serde_json::to_string(&bundle).unwrap();
    let duplicated = text.replacen(
        "\"pointer_width_policy\":",
        "\"pointer_width_policy\":\"x\",\"pointer_width_policy\":",
        1,
    );
    assert!(serde_json::from_str::<PreparedBundle>(&duplicated).is_err());

    let recovery = serde_json::to_value(run.store().recovery().unwrap()).unwrap();
    let from = |v: serde_json::Value| serde_json::from_value::<RecoveryRecord>(v);
    for bad in [
        serde_json::json!([1]),
        serde_json::json!([1, 2, 3]),
        serde_json::json!([1, 2, 3, 4, 5]),
    ] {
        let mut v = recovery.clone();
        v["resources"][0]["pointer"] = bad;
        assert!(from(v).is_err(), "only 2- and 4-arrays are pointers");
    }
    let mut v = recovery.clone();
    v["resources"][0]["pointer_refused"] = true.into();
    assert!(
        from(v).unwrap().to_snapshot(&run.bundle).is_err(),
        "refused placeholder is Four zeros"
    );
    // A two-octet pointer inside a revision-1 operation.
    let legacy = execute(Policy::Legacy, None).await;
    let mut v = serde_json::to_value(legacy.store().recovery().unwrap()).unwrap();
    v["resources"][0]["pointer"] = serde_json::json!([1, 0x16]);
    assert!(from(v).unwrap().to_snapshot(&legacy.bundle).is_err());
    let observation = PointerObservationRecord {
        object: 1,
        expected: SOURCE_1,
        observed: Some(PointerRecord::Two([1, 0x16])),
        state: 2,
    };
    assert!(observation
        .to_observation(&PointerWidthAdmission::Legacy)
        .is_err());
    assert!(observation
        .to_observation(&PointerWidthAdmission::one_element_two_or_four(vec![1]).unwrap())
        .is_ok());
}

#[test]
fn legacy_hardware_record_bytes_and_digest_stay_golden() {
    use knx_app::system2_validation_preparation::{HardwareObservation, ProjectHardwareRecord};
    use sha2::{Digest, Sha256};
    let mut record = ProjectHardwareRecord {
        revision: 1,
        operation: "golden".into(),
        recovery_track: RecoveryTrack::KnxBenchBestEffort,
        recovery_basis: "basis".into(),
        observation: HardwareObservation {
            target: "1.1.7".into(),
            serial: [0, 1, 2, 3, 4, 5],
            mask: 0x25,
            manufacturer: 1,
            hardware: [0, 1, 0, 0, 0, 91],
            application: [0, 1, 0x92, 5, 4],
            observed_application: None,
        },
        source_preparation_sha256: "0".repeat(64),
        pointer_width_policy: None,
    };
    // The pre-width grammar, written out by hand: no key, same order, same hash domain.
    let text = format!(
        concat!(
            "{{\"revision\":1,\"operation\":\"golden\",\"recovery_track\":\"knx-bench-best-effort\",",
            "\"recovery_basis\":\"basis\",\"observation\":{{\"target\":\"1.1.7\",",
            "\"serial\":[0,1,2,3,4,5],\"mask\":37,\"manufacturer\":1,\"hardware\":[0,1,0,0,0,91],",
            "\"application\":[0,1,146,5,4]}},\"source_preparation_sha256\":\"{}\"}}"
        ),
        "0".repeat(64)
    );
    assert_eq!(serde_json::to_string(&record).unwrap(), text);
    let golden = format!(
        "{:x}",
        Sha256::digest(format!(
            "[\"knxbench-configured-system2-hardware-v1\",{text}]"
        ))
    );
    assert_eq!(record.sha256().unwrap(), golden);
    record.revision = 2;
    record.pointer_width_policy = Some("up258-policy".into());
    assert_ne!(record.sha256().unwrap(), golden);
}

#[tokio::test]
async fn t19_preview_labels_object_one_observed_and_objects_two_three_unverified() {
    let run = execute(Policy::Width, None).await;
    let preview =
        knx_app::system2_validation_preparation::configured_hardware_preview(&run.bundle).unwrap();
    let warning = preview
        .warnings
        .iter()
        .find(|w| w.starts_with("PID 7 width"))
        .unwrap();
    assert!(warning.contains("objects 1, 2 and 3") && warning.contains("observed on one device"));
    assert!(warning.contains("objects 2 and 3 rely on the public package declaration only"));
    assert!(warning.contains("unverified") && !warning.contains("support"));
    let legacy = knx_app::system2_validation_preparation::configured_hardware_preview(&revision1(
        run.bundle.clone(),
    ))
    .unwrap();
    assert!(legacy
        .warnings
        .iter()
        .all(|w| !w.starts_with("PID 7 width")));
    // The source-only preview, before any networking, already carries the policy.
    let (_dir, conn, project, profile) = inputs();
    let source = prepare_project_hardware_source(
        &conn,
        &project,
        knx_core::DeviceId(1),
        &profile,
        "preview",
        RecoveryTrack::KnxBenchBestEffort,
    )
    .unwrap();
    let preview = source.preview_summary().unwrap();
    assert!(preview
        .warnings
        .iter()
        .any(|w| w.starts_with("PID 7 width")));
}

fn all_two() -> Option<Rewrite> {
    rw(|_, _, _, n, d| low2(n, d))
}
/// Device-control (PID 14) writes of the run: the VerifyMode setup, then its restoration.
fn pid14_writes(run: &Run) -> usize {
    run.device.seen()[run.seen_before..]
        .iter()
        .filter(|s| {
            matches!(
                s,
                Seen::PropertyWrite {
                    property_id: 14,
                    ..
                }
            )
        })
        .count()
}
fn dir_bytes(path: &std::path::Path) -> Vec<(String, Vec<u8>)> {
    let mut files: Vec<_> = std::fs::read_dir(path)
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.is_file())
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

#[tokio::test]
async fn t14_final_verification_failures_under_two_replies_never_write_pid14() {
    let clean = execute(Policy::Width, all_two()).await;
    finished(&clean);
    assert_eq!(
        pid14_writes(&clean),
        2,
        "VerifyMode setup and its restoration"
    );
    for (fault, text) in [
        (Fault::Memory, "full allocation verification differs"),
        (Fault::LoadState, "final resource state is not Loaded"),
        (
            Fault::ProgramVersion,
            "final application identity differs from captured program version",
        ),
    ] {
        let run = execute_with(
            Policy::Width,
            Setup {
                fault: Some(fault),
                ..Setup::default()
            },
            all_two(),
        )
        .await;
        assert!(run.result.is_err(), "{text}");
        let store = run.store();
        assert_eq!(store.outcome(), LocalOutcome::Failed, "{text}");
        assert!(store.progress().mutation_possible && store.progress().backup_available);
        assert!(
            run.private_diagnostic().contains(text),
            "{}",
            run.private_diagnostic()
        );
        assert!(store.progress().pointer_observations.is_empty(), "{text}");
        assert!(resource_pointers(&store)
            .iter()
            .all(|p| matches!(p, PointerRecord::Two(_))));
        assert_eq!(
            pid14_writes(&run),
            1,
            "{text}: the restoration write must not happen"
        );
        assert!(matches!(run.device.seen().last(), Some(Seen::Disconnect)));
    }
}

fn final_observation_sequence(run: &Run) -> usize {
    dir_bytes(&run.operation)
        .iter()
        .filter(|(name, _)| name.starts_with("checkpoint-"))
        .find_map(|(name, bytes)| {
            let value: serde_json::Value = serde_json::from_slice(bytes).ok()?;
            (value["checkpoint"]["event"] == "final_pointer_observations").then(|| {
                name["checkpoint-".len()..name.len() - ".json".len()]
                    .parse()
                    .unwrap()
            })
        })
        .expect("the clean run journals a final observation")
}

#[tokio::test]
async fn t14_a_failed_final_observation_barrier_leaves_pid14_untouched() {
    // Prestate Two, final Four with the same value: a width-change observation at the end.
    let width_change = || {
        rw(|o, index, _, n, d| {
            if o == 1 && index == 0 {
                low2(n, d)
            } else {
                (n, d.to_vec())
            }
        })
    };
    let clean = execute(Policy::Width, width_change()).await;
    finished(&clean);
    assert_eq!(pid14_writes(&clean), 2);
    let sequence = final_observation_sequence(&clean);
    let run = execute_with(
        Policy::Width,
        Setup {
            block_checkpoint: Some(sequence),
            ..Setup::default()
        },
        width_change(),
    )
    .await;
    assert!(
        run.result.is_err(),
        "the final observation barrier must stop the run"
    );
    assert!(run
        .operation
        .join(format!("checkpoint-{sequence:08}.json"))
        .is_dir());
    assert!(dir_bytes(&run.operation).iter().all(|(_, bytes)| {
        !String::from_utf8_lossy(bytes).contains("final_pointer_observations")
    }));
    assert_eq!(
        pid14_writes(&run),
        1,
        "VerifyMode is not restored before the barrier"
    );
    assert!(matches!(run.device.seen().last(), Some(Seen::Disconnect)));
}

#[tokio::test]
async fn t15_destructive_unload_with_all_two_replies_is_restore_failed_never_restored() {
    let run = execute_with(
        Policy::Width,
        Setup {
            config: Some(SimulatorConfig {
                experimental_destroy_allocation_memory_on_unload: true,
                ..Default::default()
            }),
            ..Setup::default()
        },
        all_two(),
    )
    .await;
    assert!(
        run.result.is_err(),
        "masked bytes are lost, so the final check fails"
    );
    assert_eq!(run.store().outcome(), LocalOutcome::Failed);
    let original = run.store().recovery().unwrap().clone();
    assert!(original
        .resources
        .iter()
        .all(|r| matches!(r.pointer, PointerRecord::Two(_))));
    assert!(run.restore(all_two()).await.is_err());
    let reopened = run.store();
    assert_eq!(reopened.outcome(), LocalOutcome::RestoreFailed);
    assert_ne!(reopened.outcome(), LocalOutcome::Restored);
    assert_eq!(reopened.recovery(), Some(&original));
    assert!(!reopened.progress().restore_available);
    assert_ne!(run.device.system2_snapshot(), run.old);
}

/// A live store driven to the first allocation record's step: recovery durable, step active.
fn store_at_allocation_step(
    path: &std::path::Path,
    run: &Run,
) -> (OperationStore, usize, u8, [u8; 4]) {
    use knx_app::property_validation::Checkpoint;
    use knx_app::property_validation_bundle::OperationRecord;
    let (index, object, expected) = run
        .bundle
        .normalized_plan
        .steps
        .iter()
        .enumerate()
        .find_map(|(i, step)| match &step.operation {
            OperationRecord::LoadControl { object, octets } if octets[0] == 3 && octets[1] <= 2 => {
                Some((i, *object, [0, 0, octets[2], octets[3]]))
            }
            _ => None,
        })
        .unwrap();
    let mut store = OperationStore::create(path, run.bundle.binding.clone()).unwrap();
    let recovery = run.store().recovery().unwrap().clone();
    store
        .checkpoint(Checkpoint::RecoveryAcquired { recovery })
        .unwrap();
    for k in 0..index {
        store
            .checkpoint(Checkpoint::StepStarted {
                index: k,
                mutation: true,
            })
            .unwrap();
        store
            .checkpoint(Checkpoint::StepFinished { index: k })
            .unwrap();
    }
    store
        .checkpoint(Checkpoint::StepStarted {
            index,
            mutation: true,
        })
        .unwrap();
    (store, index, object, expected)
}

#[tokio::test]
async fn forged_pointer_observation_checkpoints_are_refused_on_a_live_store_and_leave_it_intact() {
    use knx_app::property_validation::Checkpoint;
    let run = execute(Policy::Width, all_two()).await;
    finished(&run);
    let path = run._dir.path().join("forged");
    let (mut store, index, object, expected) = store_at_allocation_step(&path, &run);
    let low = [expected[2], expected[3]];
    let record = |object, expected, observed| PointerObservationRecord {
        object,
        expected,
        observed,
        state: 2,
    };
    let source1 = SOURCE_1;
    let forged = [
        // Final: equal value and the prestate's own width is no difference at all.
        Checkpoint::FinalPointerObservations {
            observations: vec![record(1, source1, Some(PointerRecord::Two([1, 0x16])))],
        },
        // Final: a real difference, but the expectation is not the source model's.
        Checkpoint::FinalPointerObservations {
            observations: vec![record(
                1,
                [0, 0, 2, 2],
                Some(PointerRecord::Four([0, 0, 3, 3])),
            )],
        },
        Checkpoint::FinalPointerObservations {
            observations: vec![],
        },
        // In procedure: same width, same value.
        Checkpoint::InProcedurePointerObservation {
            operation_index: index,
            observation: record(object, expected, Some(PointerRecord::Two(low))),
        },
        // In procedure: another object's record, another step, another source address.
        Checkpoint::InProcedurePointerObservation {
            operation_index: index,
            observation: record(
                object + 1,
                expected,
                Some(PointerRecord::Four([9, 9, 9, 9])),
            ),
        },
        Checkpoint::InProcedurePointerObservation {
            operation_index: index + 1,
            observation: record(object, expected, Some(PointerRecord::Four([9, 9, 9, 9]))),
        },
        Checkpoint::InProcedurePointerObservation {
            operation_index: index,
            observation: record(
                object,
                [0, 0, 9, 9],
                Some(PointerRecord::Four([9, 9, 9, 9])),
            ),
        },
    ];
    let before = dir_bytes(&path);
    let progress = store.progress();
    for (position, checkpoint) in forged.into_iter().enumerate() {
        assert!(
            store.checkpoint(checkpoint).is_err(),
            "forgery {position} must be refused"
        );
        assert_eq!(
            dir_bytes(&path),
            before,
            "forgery {position} wrote a checkpoint"
        );
        assert_eq!(
            store.progress(),
            progress,
            "forgery {position} changed the state"
        );
    }
    drop(store);
    let reopened = OperationStore::open(&path, &run.bundle.binding).unwrap();
    assert_eq!(dir_bytes(&path), before);
    assert!(reopened.progress().pointer_observations.is_empty());
    drop(reopened);

    // Positive control on the same live store: a width change with an equal value IS accepted.
    let (mut store, index, object, expected) =
        store_at_allocation_step(&run._dir.path().join("accepted"), &run);
    store
        .checkpoint(Checkpoint::InProcedurePointerObservation {
            operation_index: index,
            observation: record(object, expected, Some(PointerRecord::Four(expected))),
        })
        .unwrap();
    store
        .checkpoint(Checkpoint::FinalPointerObservations {
            observations: vec![record(1, source1, Some(PointerRecord::Four(source1)))],
        })
        .unwrap();
    drop(store);
    let reopened =
        OperationStore::open(&run._dir.path().join("accepted"), &run.bundle.binding).unwrap();
    let kept: Vec<_> = reopened
        .progress()
        .pointer_observations
        .iter()
        .map(|o| (o.phase, o.record.observed))
        .collect();
    assert_eq!(
        kept,
        [
            (
                PointerObservationPhase::InProcedure,
                Some(PointerRecord::Four(expected))
            ),
            (
                PointerObservationPhase::Final,
                Some(PointerRecord::Four(source1))
            ),
        ]
    );
}

#[tokio::test]
async fn legacy_operations_refuse_two_octet_and_no_op_observation_checkpoints() {
    use knx_app::property_validation::Checkpoint;
    let legacy = execute(Policy::Legacy, None).await;
    finished(&legacy);
    let path = legacy._dir.path().join("forged-legacy");
    let (mut store, _, _, _) = store_at_allocation_step(&path, &legacy);
    let before = dir_bytes(&path);
    for observed in [PointerRecord::Two([1, 0x16]), PointerRecord::Four(SOURCE_1)] {
        let forged = Checkpoint::FinalPointerObservations {
            observations: vec![PointerObservationRecord {
                object: 1,
                expected: SOURCE_1,
                observed: Some(observed),
                state: 1,
            }],
        };
        assert!(store.checkpoint(forged).is_err());
        assert_eq!(dir_bytes(&path), before);
    }
}

fn pid7_reads(run: &Run, object: u8) -> usize {
    run.device.seen()[run.seen_before..]
        .iter()
        .filter(|s| {
            matches!(s, Seen::PropertyRead { object_index, property_id: 7 } if *object_index == object)
        })
        .count()
}

/// Which PID 7 read of `object` is replaced: the first read-back after an allocation
/// (in procedure) or the last read (final verification).
#[derive(Clone, Copy, Debug)]
enum Phase {
    InProcedure,
    Final,
}

#[tokio::test]
async fn refusals_after_mutation_in_both_phases_stop_hard_with_the_original_backup_kept() {
    let clean = execute(Policy::Width, None).await;
    finished(&clean);
    let (object, total) = clean
        .totals
        .iter()
        .map(|(o, t)| (*o, *t))
        .find(|(_, t)| *t >= 3)
        .expect("an object with prestate, in-procedure and final reads");
    for phase in [Phase::InProcedure, Phase::Final] {
        let target = match phase {
            Phase::InProcedure => 1,
            Phase::Final => total - 1,
        };
        let at = move |elements: u8, data: Vec<u8>| {
            rw(move |o, index, _, n, d| {
                if o == object && index == target {
                    (elements, data.clone())
                } else {
                    (n, d.to_vec())
                }
            })
        };
        // Control: a matching two-octet reply in this phase completes, nothing is refused.
        let run = execute(Policy::Width, at(1, vec![SOURCE_1[2], SOURCE_1[3]])).await;
        finished(&run);
        assert_eq!(pid14_writes(&run), 2, "{phase:?}");
        for (elements, data, reason) in [
            // right width, wrong count
            (2, vec![0, 0, 0, 0], "ElementCount"),
            (2, vec![0, 0], "ElementCount"),
            // right count, wrong width
            (1, vec![0, 0, 0], "UnsupportedWidth"),
            (1, vec![0, 0, 0, 0, 0], "UnsupportedWidth"),
            (0, vec![0xAB], "ZeroElementsWithData"),
        ] {
            let run = execute(Policy::Width, at(elements, data.clone())).await;
            let label = format!("{phase:?} {elements} elements {data:02x?}");
            assert!(run.result.is_err(), "{label}");
            let store = run.store();
            assert_eq!(store.outcome(), LocalOutcome::Failed, "{label}");
            let progress = store.progress();
            assert!(progress.mutation_possible, "{label}");
            assert!(
                progress.backup_available,
                "{label}: the original backup is kept"
            );
            assert!(
                store.recovery().is_some() && progress.completed_steps > 0,
                "{label}"
            );
            let diagnostic = run.private_diagnostic();
            assert!(
                diagnostic.contains(&format!(
                    "reason: {reason}, object: {object}, property_id: 7, elements: {elements}, octets: {}",
                    data.len()
                )),
                "{label}: {diagnostic}"
            );
            assert_eq!(run.private_code(), "identity_or_recovery", "{label}");
            // No retry of the read, no restoration write, no restore started.
            assert_eq!(pid7_reads(&run, object), target + 1, "{label}");
            assert_eq!(pid14_writes(&run), 1, "{label}: setup only");
            assert!(
                matches!(run.device.seen().last(), Some(Seen::Disconnect)),
                "{label}"
            );
            assert_ne!(store.outcome(), LocalOutcome::Restoring, "{label}");
        }
    }
}

mod pid7_failure_diagnostic;
